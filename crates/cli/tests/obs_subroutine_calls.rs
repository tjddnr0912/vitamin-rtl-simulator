//! R2 ⓑ/ⓒ — `run.json`'s `subroutine_calls` object: how many times each
//! subroutine was actually ENTERED at runtime, and where it was declared.
//!
//! SPEC = `docs/preview/19-ai-agent-observability.md` §4.10.
//!
//! WHY IT IS A SEPARATE OBJECT FROM `subroutines`. That one is static — it says
//! which functions and tasks the elaborator turned into frame calls and how many
//! call SITES it lowered under that route, which is knowable without running
//! anything. It cannot say how often a site ran, and it deliberately files no row
//! for a class method or a hierarchical call. This object is per-INSTANCE FuncId
//! and covers both. The two must not be added; `decl_file`/`decl_line` (ⓒ) is the
//! join, because a declaration is written once no matter how many FuncIds it
//! mints.
//!
//! Every `calls` number below is derived by hand from the design beside it. If a
//! change moves one, redo the derivation — do not relax the assertion.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run vita on `src` with `args` in a per-test directory (`obs_subroutines.rs`'s
/// recipe — `n` restarts at 0 in every test PROCESS and the OS recycles PIDs, so
/// two runs can otherwise land on one directory). Returns `run.json`'s text.
fn run_json(src: &str, args: &[&str]) -> (String, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d: PathBuf = std::env::temp_dir().join(format!("vita_obssubc_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    run_json_in(&d, src, args)
}

/// The same, in a CALLER-CHOSEN directory and without wiping it.
///
/// ⚠️ Load-bearing for every cross-run comparison below. A row's `decl_file` is
/// the source path AS GIVEN, deliberately not a basename (doc-19 §4.6: a 19-file
/// design needs the directory to be actionable), so two runs from two temp
/// directories are two different INPUTS. R-F1's claim is "same input ⇒
/// byte-identical file"; comparing across directories would test the opposite of
/// what it says, exactly as `obs_procs.rs`'s determinism test records.
fn run_json_in(d: &std::path::Path, src: &str, args: &[&str]) -> (String, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::fs::create_dir_all(d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let obs = d.join(format!("obsout{n}"));
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .arg("--obs-dir")
        .arg(obs.to_str().unwrap())
        .args(args)
        .current_dir(d)
        .output()
        .expect("run vita");
    let json = std::fs::read_to_string(obs.join("run.json")).unwrap_or_default();
    (
        json,
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

/// The `subroutine_calls` object's text alone, so a field assertion cannot
/// accidentally match `builtins`/`processes` around it.
fn obj(json: &str) -> String {
    let i = json
        .find("\"subroutine_calls\": ")
        .expect("no subroutine_calls key");
    let rest = &json[i..];
    let end = rest.find("\"utc_unix_s\":").unwrap_or(rest.len());
    rest[..end].to_string()
}

/// One row, as `(decl_line, calls)`. A hand parser and not a JSON crate, for
/// `obs_subroutines.rs`'s reason: the CLI hand-serializes this file with no JSON
/// dependency, so asserting through a lenient reader would let a formatting bug
/// hide. `name` is `%m`'s path with the engine's leading `.` already trimmed.
fn row(json: &str, name: &str) -> Option<(u32, u64)> {
    let pat = format!("\"name\": \"{name}\", \"decl_file\": ");
    let i = json.find(&pat)? + pat.len();
    let rest = &json[i..];
    let lp = "\"decl_line\": ";
    let j = rest.find(lp)? + lp.len();
    let rest2 = &rest[j..];
    let line: u32 = rest2[..rest2.find(|c: char| !c.is_ascii_digit())?]
        .parse()
        .ok()?;
    let cp = "\"calls\": ";
    let k = rest2.find(cp)? + cp.len();
    let rest3 = &rest2[k..];
    let calls: u64 = rest3[..rest3.find(|c: char| !c.is_ascii_digit())?]
        .parse()
        .ok()?;
    Some((line, calls))
}

/// Two instances of one module, so the per-INSTANCE key and the shared
/// declaration are both observable, plus a class whose methods `subroutines`
/// cannot report at all.
///
/// ⚠️ `#1 $finish`, not a bare one: a `$finish` at time 0 in `top` can end the
/// run before the two `leaf` instances' own `initial` blocks have called
/// anything, and the profile would then be measuring the scheduler's process
/// order rather than the design. Both oracles print `leaf q=6` twice and
/// `z=10`.
const D: &str = r#"module leaf;
  int q;
  function automatic int aut(input int c); aut = c + 2; endfunction
  task automatic tk(input int a, output int b); b = a + 1; endtask
  initial begin q = aut(1); tk(5, q); $display("leaf q=%0d", q); end
endmodule
module top;
  class C;
    int n;
    function int bump(input int x); n = n + x; bump = n; endfunction
  endclass
  leaf u1(); leaf u2();
  C c; int z;
  initial begin
    c = new();
    z = c.bump(3) + c.bump(4);
    $display("z=%0d", z);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn calls_are_counted_per_instance_and_the_declaration_is_the_join() {
    let (json, out, code) = run_json(D, &["--obs-procs"]);
    assert_eq!(code, 0, "{out}");
    let o = obj(&json);
    // `aut` runs once in each of the two `leaf` instances → two FuncIds, one
    // call each, and BOTH name line 3 — the declaration is written once.
    assert_eq!(row(&o, "top.u1.aut"), Some((3, 1)), "{o}");
    assert_eq!(row(&o, "top.u2.aut"), Some((3, 1)), "{o}");
    assert_eq!(row(&o, "top.u1.tk"), Some((4, 1)), "{o}");
    assert_eq!(row(&o, "top.u2.tk"), Some((4, 1)), "{o}");
    // The class method: two calls, and a row that `subroutines` documents itself
    // as NOT having. This is the half only the runtime seam can report.
    assert_eq!(row(&o, "C.bump"), Some((10, 2)), "{o}");
    assert!(
        !json.contains("\"name\": \"bump\", \"kind\""),
        "a class method must not appear in the static `subroutines` object"
    );
    // 1 + 1 + 1 + 1 + 2
    assert!(o.contains("\"total_calls\": 6"), "{o}");
    assert!(o.contains("\"distinct\": 5"), "{o}");
}

/// The counts must not depend on which executor ran the process bodies. This is
/// what pins the bumps to the three `&self` `SimState` funnels: the interpreter,
/// the VM and the native kernel all reach a subroutine through them, so a bump
/// placed at any CALLER instead would have to be proved invariant rather than
/// being invariant by construction.
#[test]
fn the_counts_are_backend_invariant() {
    let d = std::env::temp_dir().join(format!("vita_obssubc_inv_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let mut seen: Option<String> = None;
    for b in ["native", "vm", "interp"] {
        let (json, out, code) = run_json_in(&d, D, &["--obs-procs", "--backend", b]);
        assert_eq!(code, 0, "{b}: {out}");
        let o = obj(&json);
        match &seen {
            None => seen = Some(o),
            Some(first) => assert_eq!(first, &o, "backend {b} reported different rows"),
        }
    }
}

/// Two runs of one design produce byte-identical rows — the property the whole
/// OBS rail promises. Counts are a function of (design, options); the row order
/// is `calls` then FuncId, both deterministic, which is why timing is not the
/// sort key.
#[test]
fn two_runs_are_byte_identical() {
    let d = std::env::temp_dir().join(format!("vita_obssubc_det_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let (a, _, ca) = run_json_in(&d, D, &["--obs-procs"]);
    let (b, _, cb) = run_json_in(&d, D, &["--obs-procs"]);
    assert_eq!((ca, cb), (0, 0));
    assert_eq!(obj(&a), obj(&b));
}

/// `null`, not an empty object, without the opt-in — a consumer must be able to
/// tell "not measured" from "measured, nothing ran". The static `subroutines`
/// object beside it is unconditional and stays present either way.
#[test]
fn it_is_null_without_obs_procs() {
    let (json, out, code) = run_json(D, &[]);
    assert_eq!(code, 0, "{out}");
    assert!(
        json.contains("\"subroutine_calls\": null"),
        "expected null without --obs-procs"
    );
    assert!(json.contains("\"subroutines\": {\"counts\""));
}

/// An INLINED subroutine has NO row here and that is correct, not a gap — there
/// is no call node to count. It is exactly the case the static object exists to
/// cover, and reading the two together is what stops `0` from being read as
/// "free": `subroutines` says `inlined` with its site count.
#[test]
fn an_inlined_subroutine_has_no_runtime_row_but_a_static_one() {
    const SRC: &str = "module top;\n\
                       \x20 int n;\n\
                       \x20 task waiter(input int k); #5 n = n + k; endtask\n\
                       \x20 initial begin waiter(1); waiter(2);\n\
                       \x20   $display(\"n=%0d\", n); #1 $finish; end\n\
                       endmodule\n";
    let (json, out, code) = run_json(SRC, &["--obs-procs"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("n=3"), "{out}"); // both oracles: 1 + 2 at t=10
    assert!(
        json.contains(
            "\"name\": \"waiter\", \"kind\": \"task\", \"route\": \"inlined\", \
                       \"sites\": 2"
        ),
        "static row missing: {json}"
    );
    assert_eq!(row(&obj(&json), "top.waiter"), None, "{}", obj(&json));
    assert!(obj(&json).contains("\"total_calls\": 0"), "{}", obj(&json));
}

/// A SUSPENDABLE task frame is counted and never timed, and the file says so in
/// the row rather than leaving a reader to infer it.
///
/// ⚠️ This column exists because the alternative is a wrong number. That seam
/// (`enter_task_frame`) returns as soon as the frame is open; the task then sits
/// on its `#5` for the rest of the timestep, so wall time to its `Return` is
/// mostly time the task was not running. `timed_calls: 0` beside `calls: 2` is
/// what distinguishes "not measured" from "costs nothing".
#[test]
fn a_suspendable_task_frame_is_counted_and_not_timed() {
    const SRC: &str = "module top;\n\
                       \x20 int n;\n\
                       \x20 task automatic waiter(input int k); #5 n = n + k; endtask\n\
                       \x20 initial begin waiter(1); waiter(2);\n\
                       \x20   $display(\"n=%0d\", n); #1 $finish; end\n\
                       endmodule\n";
    let (json, out, code) = run_json(SRC, &["--obs-procs", "--obs-procs-time"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("n=3"), "{out}"); // both oracles
    let o = obj(&json);
    // `automatic` forces the frame the inlined twin above does not take.
    assert!(
        json.contains("\"name\": \"waiter\", \"kind\": \"task\", \"route\": \"frame\""),
        "{json}"
    );
    assert_eq!(row(&o, "top.waiter"), Some((3, 2)), "{o}");
    assert!(o.contains("\"timed\": true"), "{o}");
    assert!(
        o.contains("\"calls\": 2, \"timed_calls\": 0, \"time_s\": 0.0"),
        "a suspendable frame must be counted and NOT timed: {o}"
    );
}

/// A synchronous call IS timed, so `timed_calls` is not a constant 0 — the
/// column has to distinguish two real cases or it is measuring nothing.
#[test]
fn a_synchronous_call_is_timed() {
    const SRC: &str = "module top;\n\
                       \x20 function automatic int f(input int x); f = x + 1; endfunction\n\
                       \x20 int z;\n\
                       \x20 initial begin z = f(1); z = f(z); $display(\"z=%0d\", z); $finish; end\n\
                       endmodule\n";
    let (json, out, code) = run_json(SRC, &["--obs-procs", "--obs-procs-time"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("z=3"), "{out}"); // both oracles
    let o = obj(&json);
    assert_eq!(row(&o, "top.f"), Some((2, 2)), "{o}");
    assert!(
        o.contains("\"calls\": 2, \"timed_calls\": 2, "),
        "a synchronous call must be timed: {o}"
    );
}

/// The FACTS a reader needs before adding a column to another object's, pinned as
/// facts rather than as phrasing.
///
/// ⚠️ Each assertion below is a claim the emitted text makes, and each is checked
/// against the file in the SAME run — because a self-describing rail that
/// describes itself wrongly is a silent-wrong of its own kind. Round 1 of this
/// slice's review caught exactly that: the text said "join on decl_file:decl_line"
/// while the object it named carries neither column, and an earlier version of
/// this test pinned the sentence rather than the property, so it passed.
#[test]
fn the_object_describes_itself_truthfully() {
    let (json, _, code) = run_json(D, &["--obs-procs"]);
    assert_eq!(code, 0);
    let o = obj(&json);
    // It must warn against adding the two objects' columns…
    assert!(o.contains("must not be added"), "{o}");
    // …and the reason must be stated in both directions it is true in.
    assert!(o.contains("INCLUDES class methods"), "{o}");
    assert!(o.contains("EXCLUDES every INLINED subroutine"), "{o}");
    // The identifying columns it names must exist on its own rows.
    for f in ["\"decl_file\": ", "\"decl_line\": ", "\"decl_col\": "] {
        assert!(
            o.contains(f),
            "row column {f} named by `key` is missing: {o}"
        );
    }
    // A join it does NOT offer must not be claimed. The static object carries no
    // decl location, so the text must say so rather than instruct the join.
    let statics = {
        let i = json.find("\"subroutines\": ").unwrap();
        let rest = &json[i..];
        rest[..rest.find("\"processes\":").unwrap_or(rest.len())].to_string()
    };
    assert!(
        !statics.contains("decl_"),
        "the static object gained a decl column — update `key`, which says it has none"
    );
    assert!(o.contains("does not carry that yet"), "{o}");
    // And the timing column's coverage must be explained.
    assert!(o.contains("SUSPENDABLE task frames"), "{o}");
    assert!(
        o.contains("an inlined one\n                     contributes none")
            || o.contains("an inlined one contributes none"),
        "{o}"
    );
}
