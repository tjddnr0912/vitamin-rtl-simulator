//! R2 intermediate (round-39) — `run.json`'s `subroutines` object: which user
//! functions/tasks the elaborator turned into FRAME calls and which it inlined.
//!
//! SPEC = `docs/preview/19-ai-agent-observability.md` §4.10.
//!
//! WHY THIS TABLE EXISTS, and why the assertions below are exact. An INLINED
//! subroutine leaves no call node behind, so it contributes zero rows to
//! `processes`/`builtins` — and zero reads as "free". An external report
//! measured one frame call at 5x the cost of the same expression written without
//! it while the profile was silent about both halves. This object is the half
//! that is knowable WITHOUT a profiling run, so it is emitted unconditionally
//! (no `--obs-procs`) and it is deterministic.
//!
//! Every `sites` number below is derived by hand from the design and written
//! beside the assertion. `sites` counts call sites LOWERED — after generate and
//! instance expansion — so a call written once inside a module instantiated
//! twice is 2, and a call inside a `for` loop body is 1. If a change moves one of
//! these numbers, redo the derivation; do not relax the assertion.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run vita on `src` with `args` in a per-test directory (`obs_builtins.rs`'s
/// recipe — `n` restarts at 0 in every test PROCESS and the OS recycles PIDs, so
/// two runs can otherwise land on one directory). Returns `run.json`'s text.
fn run_json(src: &str, args: &[&str]) -> (String, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d: PathBuf = std::env::temp_dir().join(format!("vita_obssub_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let obs = d.join("obsout");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .arg("--obs-dir")
        .arg(obs.to_str().unwrap())
        .args(args)
        .current_dir(&d)
        .output()
        .expect("run vita");
    let json = std::fs::read_to_string(obs.join("run.json")).unwrap_or_default();
    (
        json,
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

/// The `subroutines` object's text alone, so a field assertion cannot
/// accidentally match `codegen`/`processes` around it.
fn subs_obj(json: &str) -> String {
    let i = json.find("\"subroutines\": ").expect("no subroutines key");
    let rest = &json[i..];
    let end = rest.find("\"processes\":").unwrap_or(rest.len());
    rest[..end].to_string()
}

/// One row, as `(kind, route, sites)`. A hand parser and not a JSON crate, for
/// `obs_builtins.rs`'s reason: the CLI hand-serializes this file with no JSON
/// dependency, so asserting through a lenient reader would let a formatting bug
/// hide.
fn row(json: &str, module: &str, name: &str) -> Option<(String, String, u64)> {
    let pat = format!("{{\"module\": \"{module}\", \"name\": \"{name}\", \"kind\": \"");
    let i = json.find(&pat)? + pat.len();
    let rest = &json[i..];
    let kind = rest[..rest.find('"')?].to_string();
    let rp = "\", \"route\": \"";
    let j = rest.find(rp)? + rp.len();
    let rest = &rest[j..];
    let route = rest[..rest.find('"')?].to_string();
    let sp = "\", \"sites\": ";
    let k = rest.find(sp)? + sp.len();
    let rest = &rest[k..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    Some((kind, route, rest[..end].parse().ok()?))
}

/// One design carrying every route the table can report: an inlined function, a
/// framed function, a framed function that is never called, an inlined task, a
/// framed task, and a package function reached by its scoped spelling — through
/// TWO instances of the module that declares them, which is what makes the
/// "lowered, not written" half of `sites` observable.
const ROUTES: &str = r#"package p;
  function automatic int dbl(input int x); dbl = x*2; endfunction
endpackage
module leaf;
  function [7:0] pure8(input [7:0] c); pure8 = c ^ 8'h5a; endfunction
  function automatic int aut(input [7:0] c); aut = c + 8'h02; endfunction
  function automatic int never_called(input [7:0] c); never_called = c; endfunction
  task automatic tk(input int a, output int b); b = a + 1; endtask
  task st(input int a); $display("st %0d", a); endtask
  logic [7:0] r;
  int q, a;
  initial begin
    r = pure8(8'h10) ^ pure8(8'h20);
    a = aut(8'h07);
    tk(5, q);
    st(q);
    $display("leaf r=%h q=%0d a=%0d", r, q, a);
  end
endmodule
module top;
  leaf u1(); leaf u2();
  int z;
  initial begin
    #1; z = p::dbl(21) + u1.aut(8'h00);
    $display("top z=%0d", z);
    $finish;
  end
endmodule
"#;

/// The headline: every route appears, with the hand-derived site counts.
#[test]
fn every_route_is_reported_with_its_lowered_site_count() {
    let (json, out, code) = run_json(ROUTES, &[]);
    assert_eq!(code, 0, "expected exit 0, stdout:\n{out}");
    let s = subs_obj(&json);

    // `pure8` is a STATIC straight-line 4-state function ⇒ the inline fold takes
    // it. Two calls in one statement x two `leaf` instances = 4.
    assert_eq!(
        row(&s, "leaf", "pure8"),
        Some(("function".into(), "inlined".into(), 4))
    );
    // `aut` is `automatic` ⇒ framed. One call per instance (2). The THIRD call —
    // the hierarchical `u1.aut(...)` in `top` — is NOT counted: its target is not
    // bound until the deferred-hier resolve, which runs after this seam. That is
    // the documented `uncounted` set, and this row is what pins it.
    assert_eq!(
        row(&s, "leaf", "aut"),
        Some(("function".into(), "frame".into(), 2))
    );
    // Declared, never called: a row with the route it WOULD take and 0 sites.
    // This is the case a seam-only census cannot produce, and the reason the
    // table is seeded from the same two sets the lowering reserves from.
    assert_eq!(
        row(&s, "leaf", "never_called"),
        Some(("function".into(), "frame".into(), 0))
    );
    // A static non-recursive task still inlines (frame ⊇ inline, but the inline
    // path is byte-identical and stays the default).
    assert_eq!(
        row(&s, "leaf", "st"),
        Some(("task".into(), "inlined".into(), 2))
    );
    // `automatic` task ⇒ framed.
    assert_eq!(
        row(&s, "leaf", "tk"),
        Some(("task".into(), "frame".into(), 2))
    );
    // The scoped package spelling is unconditionally framed, and its row is filed
    // under the module that CALLS it (a package has no instance of its own).
    assert_eq!(
        row(&s, "top", "p::dbl"),
        Some(("function".into(), "frame".into(), 1))
    );
}

/// The counts header agrees with the rows it summarizes, and the object states
/// what `sites` means rather than leaving a reader to guess (doc-19 §3: a log a
/// harness can misread is a silent-wrong).
#[test]
fn the_counts_header_and_the_semantics_notes_are_present() {
    let (json, _, code) = run_json(ROUTES, &[]);
    assert_eq!(code, 0);
    let s = subs_obj(&json);
    // 6 rows: pure8, aut, never_called, st, tk (leaf) + p::dbl (top). Framed =
    // aut, never_called, tk, p::dbl (4); inlined = pure8, st (2).
    assert!(
        s.contains("\"counts\": {\"total\": 6, \"frame\": 4, \"inlined\": 2}"),
        "counts header wrong in:\n{s}"
    );
    assert!(
        s.contains("\"sites_semantics\": \"call sites LOWERED"),
        "{s}"
    );
    assert!(
        s.contains("\"uncounted\": \"class methods and hierarchical calls\""),
        "{s}"
    );
}

/// The object is STATIC, so it must not need `--obs-procs` (which `processes`
/// and `builtins` do) and must not move when profiling is switched on. This is
/// the property that makes it usable before anyone has profiled anything.
#[test]
fn it_is_emitted_without_obs_procs_and_does_not_move_with_it() {
    let (plain, _, c1) = run_json(ROUTES, &[]);
    let (timed, _, c2) = run_json(ROUTES, &["--obs-procs", "--obs-procs-time"]);
    assert_eq!((c1, c2), (0, 0));
    assert!(plain.contains("\"processes\": null"), "{plain}");
    assert_eq!(subs_obj(&plain), subs_obj(&timed));
}

/// A design with no user subroutine emits an EMPTY table, not a missing key — a
/// consumer must be able to tell "no functions" from "this build has no census".
#[test]
fn a_design_with_no_subroutine_emits_an_empty_table() {
    let (json, _, code) = run_json(
        "module top;\n  initial begin $display(\"x\"); $finish; end\nendmodule\n",
        &[],
    );
    assert_eq!(code, 0);
    let s = subs_obj(&json);
    assert!(
        s.contains("\"counts\": {\"total\": 0, \"frame\": 0, \"inlined\": 0}"),
        "{s}"
    );
    assert!(s.contains("\"items\": []}"), "{s}");
}

/// The row is the route the design ACTUALLY took, not a re-derived prediction.
/// `int` is 2-state, so a plain static `function int f` is framed (the frame
/// return slot is what coerces x/z→0) while its 4-state `logic` twin inlines —
/// a difference no reader would guess from the source, and exactly the kind of
/// thing this table is for.
#[test]
fn a_two_state_return_is_framed_and_its_four_state_twin_is_not() {
    let (json, out, code) = run_json(
        "module top;\n\
           function int two_state(input [7:0] c); two_state = c + 1; endfunction\n\
           function logic [31:0] four_state(input [7:0] c); four_state = c + 1; endfunction\n\
           int a; logic [31:0] b;\n\
           initial begin a = two_state(8'h10); b = four_state(8'h10);\n\
             $display(\"a=%0d b=%0d\", a, b); $finish; end\n\
         endmodule\n",
        &[],
    );
    assert_eq!(code, 0, "{out}");
    let s = subs_obj(&json);
    assert_eq!(
        row(&s, "top", "two_state"),
        Some(("function".into(), "frame".into(), 1))
    );
    assert_eq!(
        row(&s, "top", "four_state"),
        Some(("function".into(), "inlined".into(), 1))
    );
}

/// The seam a caller-side census MISSED. A function with an `output` formal is a
/// frame call that never reaches `inline_function`: the hoist rewrites the
/// assignment form into a temp + statement call, and statement position has its
/// own arm. Recording at the callers reported **`sites: 0` for two real call
/// sites** in the first version of this slice; the census now lives inside the
/// three frame EMITTERS, so both spellings count.
#[test]
fn an_output_formal_call_counts_in_both_spellings() {
    const F: &str = "  function automatic int addout(input int a, output int dbl);\n\
                     \x20   dbl = a * 2; addout = a + 1;\n\
                     \x20 endfunction\n";
    // assignment form — hoisted to a temp before `inline_function` would see it
    let (json, out, code) = run_json(
        &format!(
            "module top;\n{F}  int r, d;\n\
               initial begin r = addout(5, d); r = addout(7, d);\n\
                 $display(\"r=%0d d=%0d\", r, d); $finish; end\n\
             endmodule\n"
        ),
        &[],
    );
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        row(&subs_obj(&json), "top", "addout"),
        Some(("function".into(), "frame".into(), 2))
    );
    // statement form — `void'(f(...))` and the bare `f(...);`
    let (json, out, code) = run_json(
        &format!(
            "module top;\n  int d;\n{F}\
               initial begin void'(addout(5, d)); addout(7, d);\n\
                 $display(\"d=%0d\", d); $finish; end\n\
             endmodule\n"
        ),
        &[],
    );
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        row(&subs_obj(&json), "top", "addout"),
        Some(("function".into(), "frame".into(), 2))
    );
}
