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
    run_json_in(&d, src, args)
}

/// The same, in a CALLER-CHOSEN directory and without wiping it.
///
/// ⚠️ Load-bearing for every cross-run comparison below, on the same ground
/// `obs_subroutine_calls.rs` records it: a row's `decl_file` is the source path
/// AS GIVEN, deliberately not a basename (doc-19 §4.6 — a 19-file design needs
/// the directory to be actionable), so two runs from two temp directories are two
/// different INPUTS. A test asserting "this object does not move" across them
/// would be asserting the opposite of what it says.
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

/// The runtime `subroutine_calls` object's text alone — the object the static
/// rows below must JOIN to.
fn calls_obj(json: &str) -> String {
    let i = json
        .find("\"subroutine_calls\": ")
        .expect("no subroutine_calls key");
    let rest = &json[i..];
    rest[..rest.find("\"utc_unix_s\":").unwrap_or(rest.len())].to_string()
}

/// The `decl_file` / `decl_line` / `decl_col` triple that follows `rest`'s
/// start. ONE parser for both objects on purpose: the join is only executable
/// while the two writers spell the three fields identically, so a drift in
/// either must stop the join rather than be papered over by a second parser.
fn triple_at(rest: &str) -> Option<(String, u32, u32)> {
    let fp = "\"decl_file\": \"";
    let i = rest.find(fp)? + fp.len();
    let rest = &rest[i..];
    let file = rest[..rest.find('"')?].to_string();
    let lp = "\"decl_line\": ";
    let j = rest.find(lp)? + lp.len();
    let rest = &rest[j..];
    let line: u32 = rest[..rest.find(|c: char| !c.is_ascii_digit())?]
        .parse()
        .ok()?;
    let cp = "\"decl_col\": ";
    let k = rest.find(cp)? + cp.len();
    let rest = &rest[k..];
    let col: u32 = rest[..rest.find(|c: char| !c.is_ascii_digit())?]
        .parse()
        .ok()?;
    Some((file, line, col))
}

/// The STATIC row `(module, name)`'s declaration triple.
fn decl(s: &str, module: &str, name: &str) -> Option<(String, u32, u32)> {
    let pat = format!("{{\"module\": \"{module}\", \"name\": \"{name}\", \"kind\": \"");
    triple_at(&s[s.find(&pat)?..])
}

/// The RUNTIME row labelled `name`'s declaration triple.
fn rt_decl(o: &str, name: &str) -> Option<(String, u32, u32)> {
    let pat = format!("\"name\": \"{name}\", \"decl_file\": ");
    triple_at(&o[o.find(&pat)?..])
}

/// Every static row's `(module, name)` key, in emission order.
fn keys(s: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(i) = rest.find("{\"module\": \"") {
        rest = &rest[i + "{\"module\": \"".len()..];
        let Some(e) = rest.find('"') else { break };
        let module = rest[..e].to_string();
        let np = "\", \"name\": \"";
        let Some(j) = rest.find(np) else { break };
        rest = &rest[j + np.len()..];
        let Some(e) = rest.find('"') else { break };
        out.push((module, rest[..e].to_string()));
    }
    out
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
///
/// ⚠️ ONE directory for both runs: since the rows carry `decl_file`, two temp
/// directories are two different inputs and the comparison would fail on the
/// path rather than on the property.
#[test]
fn it_is_emitted_without_obs_procs_and_does_not_move_with_it() {
    let d = std::env::temp_dir().join(format!("vita_obssub_move_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let (plain, _, c1) = run_json_in(&d, ROUTES, &[]);
    let (timed, _, c2) = run_json_in(&d, ROUTES, &["--obs-procs", "--obs-procs-time"]);
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

/// A CLASS in the design must not move any `sites` number.
///
/// ⚠️ This is the asymmetric-mutation teeth for the FuncId alignment, and it is
/// the shape the 7,337-test suite could not see: every design in this file was
/// class-free, and `reserve_class_method` was the one producer that minted a
/// FuncId without pushing its route key. Class methods are reserved FIRST, so
/// each one shifted every module subroutine's key by 1 — measured as two `sites`
/// counts SWAPPED (`aaa` 3 → 0, i.e. "declared and never called", while the
/// never-called `bbb` reported `aaa`'s 3), and as both silently dropped when the
/// shifted index ran off the end.
///
/// A same-input-twice golden cannot catch that: both runs lie identically. The
/// only gate that can is a PAIR whose two members must agree — the mutation is
/// upstream (a class), the observed numbers are downstream (module routines),
/// and a class contributes no row of its own (`uncounted`).
#[test]
fn a_class_does_not_shift_the_module_routine_counts() {
    // `aaa` is called three times, `bbb` once — derived from the source below.
    const BODY: &str = "  function automatic int aaa(input int x); aaa = x + 1; endfunction\n\
                        \x20 function automatic int bbb(input int x); bbb = x + 2; endfunction\n\
                        \x20 int z;\n\
                        \x20 initial begin z = aaa(5); z = aaa(z); z = aaa(z); z = bbb(z);\n\
                        \x20   $display(\"z=%0d\", z); $finish; end\n";
    let control = format!("module top;\n{BODY}endmodule\n");
    // Same design, plus a class whose two methods are never called at all. The
    // class exists only to mint FuncIds ahead of `aaa`/`bbb`.
    let mutated = format!(
        "module top;\n\
         \x20 class C;\n\
         \x20   int n;\n\
         \x20   function int f1(input int x); f1 = x; endfunction\n\
         \x20   function int f2(input int x); f2 = x; endfunction\n\
         \x20 endclass\n{BODY}endmodule\n"
    );
    let (jc, oc, cc) = run_json(&control, &[]);
    let (jm, om, cm) = run_json(&mutated, &[]);
    assert_eq!(cc, 0, "{oc}");
    assert_eq!(cm, 0, "{om}");
    // The design's own answer must not move either — the mutation is inert.
    assert!(oc.contains("z=10"), "control stdout: {oc}");
    assert!(om.contains("z=10"), "mutated stdout: {om}");
    for (name, sites) in [("aaa", 3u64), ("bbb", 1u64)] {
        assert_eq!(
            row(&subs_obj(&jc), "top", name),
            Some(("function".into(), "frame".into(), sites)),
            "control row for {name}"
        );
        assert_eq!(
            row(&subs_obj(&jm), "top", name),
            Some(("function".into(), "frame".into(), sites)),
            "a class in the design changed `{name}`'s site count"
        );
    }
    // And the class itself earns no row, as `uncounted` promises.
    assert!(
        row(&subs_obj(&jm), "top", "f1").is_none() && row(&subs_obj(&jm), "C", "f1").is_none(),
        "a class method must not appear in `subroutines`: {}",
        subs_obj(&jm)
    );
}

/// The same alignment, one class method and one CALL SITE — the minimal shape.
/// Declaring a single class method was enough to drop `plain`'s only call site
/// to `0`, and `0` is the value the object documents as "declared and never
/// called". The class is never even instantiated here.
#[test]
fn one_declared_class_method_does_not_erase_a_call_site() {
    const BODY: &str = "  function automatic int plain(input int x); plain = x + 1; endfunction\n\
                        \x20 int z;\n\
                        \x20 initial begin z = plain(5); $display(\"z=%0d\", z); $finish; end\n";
    let (jc, oc, cc) = run_json(&format!("module top;\n{BODY}endmodule\n"), &[]);
    let (jm, om, cm) = run_json(
        &format!(
            "module top;\n\
             \x20 class C; int n; function int m(input int x); m = x; endfunction endclass\n\
             {BODY}endmodule\n"
        ),
        &[],
    );
    assert_eq!(cc, 0, "{oc}");
    assert_eq!(cm, 0, "{om}");
    let want = Some(("function".into(), "frame".into(), 1u64));
    assert_eq!(row(&subs_obj(&jc), "top", "plain"), want);
    assert_eq!(
        row(&subs_obj(&jm), "top", "plain"),
        want,
        "declaring a class method erased a real call site"
    );
}

/// A design carrying every JOIN shape the two subroutine objects can produce, so
/// the declaration triple is pinned as the join key rather than as a new column.
///
/// Line numbers are load-bearing here — every `decl_line` below is derived by
/// hand from THIS text — so a line must not be inserted without redoing them.
///
/// ⚠️ `#5 $finish` in `top`, not a bare one: a `$finish` at time 0 can end the
/// run before the two `leaf` instances have called anything, and the runtime half
/// of the join would then be measuring process order.
const JOIN: &str = r#"package p;
  function automatic int dbl(input int x); dbl = x*2; endfunction
  function automatic int trp(input int x); trp = x*3; endfunction
  task automatic ptk(input int a); $display("ptk %0d", a); endtask
endpackage
module leaf;
  import p::trp; import p::ptk;
  function [7:0] pure8(input [7:0] c); pure8 = c ^ 8'h5a; endfunction
  function automatic int aut(input [7:0] c); aut = c + 8'h02; endfunction
  function automatic int never_called(input [7:0] c); never_called = c; endfunction
  task automatic tk(input int a, output int b); b = a + 1; endtask
  task st(input int a); $display("st %0d", a); endtask
  logic [7:0] r;
  int q, a, t3;
  initial begin
    r = pure8(8'h10) ^ pure8(8'h20);
    a = aut(8'h07);
    tk(5, q);
    st(q);
    t3 = trp(2);
    ptk(9);
    $display("leaf r=%h q=%0d a=%0d t3=%0d", r, q, a, t3);
  end
endmodule
module other;
  function automatic int aut(input [7:0] c); aut = c - 1; endfunction
  int w;
  initial begin w = aut(8'h05); $display("other w=%0d", w); end
endmodule
class C;
  function int m(input int x); return x+1; endfunction
endclass
module top;
  leaf u1(); leaf u2();
  other o();
  int z, cm;
  C c;
  genvar g;
  generate for (g = 0; g < 2; g++) begin : gb
    function automatic int gf(input int x); gf = x + g; endfunction
    int gv;
    initial gv = gf(10);
  end endgenerate
  initial begin
    c = new;
    #1; z = p::dbl(21) + u1.aut(8'h00);
    cm = c.m(4);
    $display("top z=%0d cm=%0d gv0=%0d gv1=%0d", z, cm, gb[0].gv, gb[1].gv);
    #5 $finish;
  end
endmodule
"#;

/// Every static row carries the declaration site of its routine's NAME, and that
/// triple is the JOIN to `subroutine_calls` — in all four cardinalities the two
/// objects can produce.
///
/// WHY THE JOIN NEEDED PINNING AND NOT JUST THE COLUMN. Before this, `key` told a
/// reader to cross-read "by name and route", which does not work: the static row
/// is `(leaf, aut)` and the runtime rows are `top.u1.aut` / `top.u2.aut`, so the
/// reader has to reconstruct the instance tree to get back to one written
/// function. The declaration is written once, which is exactly why it joins — and
/// why the cardinality is 1:N, not 1:1.
#[test]
fn every_row_carries_its_declaration_site_and_joins_the_runtime_rows() {
    let (json, out, code) = run_json(JOIN, &["--obs-procs"]);
    assert_eq!(code, 0, "expected exit 0, stdout:\n{out}");
    // The design's own answers, so a join over a mis-elaborated design cannot
    // read as a passing join.
    assert!(out.contains("top z=44 cm=5 gv0=10 gv1=11"), "{out}");
    let s = subs_obj(&json);
    let o = calls_obj(&json);

    // The file name is whatever spelling the resolver produced (a path AS GIVEN,
    // see `obs_subroutine_calls.rs`), so it is compared BETWEEN the two objects
    // rather than against a literal.
    let file = decl(&s, "leaf", "aut").expect("no (leaf, aut) row").0;
    assert!(
        file.ends_with("t.sv"),
        "decl_file is not the source: {file}"
    );

    // 1:N — one written function, two instances. The static table folds them.
    assert_eq!(decl(&s, "leaf", "aut"), Some((file.clone(), 9, 26)), "{s}");
    assert_eq!(rt_decl(&o, "top.u1.aut"), decl(&s, "leaf", "aut"), "{o}");
    assert_eq!(rt_decl(&o, "top.u2.aut"), decl(&s, "leaf", "aut"), "{o}");
    // Same shape through an `import p::trp` spelling and through a task.
    assert_eq!(decl(&s, "leaf", "trp"), Some((file.clone(), 3, 26)), "{s}");
    assert_eq!(rt_decl(&o, "top.u1.trp"), decl(&s, "leaf", "trp"), "{o}");
    assert_eq!(rt_decl(&o, "top.u2.trp"), decl(&s, "leaf", "trp"), "{o}");
    assert_eq!(decl(&s, "leaf", "ptk"), Some((file.clone(), 4, 18)), "{s}");
    assert_eq!(rt_decl(&o, "top.u1.ptk"), decl(&s, "leaf", "ptk"), "{o}");
    assert_eq!(decl(&s, "leaf", "tk"), Some((file.clone(), 11, 18)), "{s}");
    assert_eq!(rt_decl(&o, "top.u1.tk"), decl(&s, "leaf", "tk"), "{o}");
    // A same-named function in ANOTHER module is a DIFFERENT declaration — the
    // pair key and the triple must move together or the join is ambiguous.
    assert_eq!(
        decl(&s, "other", "aut"),
        Some((file.clone(), 26, 26)),
        "{s}"
    );
    assert_eq!(rt_decl(&o, "top.o.aut"), decl(&s, "other", "aut"), "{o}");
    assert_ne!(decl(&s, "other", "aut"), decl(&s, "leaf", "aut"));

    // Two GENERATE-scoped keys, ONE declaration: both rows carry 40:28, so the
    // triple joins EACH static row to BOTH runtime rows (many-to-many). The
    // generate index lives in `name` / the `%m` path, not in the triple, and
    // `sites` must not be summed across the two static rows.
    let gf = Some((file.clone(), 40, 28));
    assert_eq!(decl(&s, "top", "gb[0]$gf"), gf, "{s}");
    assert_eq!(decl(&s, "top", "gb[1]$gf"), gf, "{s}");
    assert_eq!(rt_decl(&o, "top.gb[0].gf"), gf, "{o}");
    assert_eq!(rt_decl(&o, "top.gb[1].gf"), gf, "{o}");

    // A PACKAGE routine is filed under the CALLING module and its declaration is
    // in the package — which is the case "read it by name" could never solve
    // (`p::dbl` against `top.dbl`).
    assert_eq!(
        decl(&s, "top", "p::dbl"),
        Some((file.clone(), 2, 26)),
        "{s}"
    );
    assert_eq!(rt_decl(&o, "top.dbl"), decl(&s, "top", "p::dbl"), "{o}");

    // Declared and never called: a row with `sites: 0` still carries its site.
    assert_eq!(
        row(&s, "leaf", "never_called"),
        Some(("function".into(), "frame".into(), 0))
    );
    assert_eq!(
        decl(&s, "leaf", "never_called"),
        Some((file.clone(), 10, 26)),
        "{s}"
    );
    assert!(rt_decl(&o, "top.u1.never_called").is_none(), "{o}");

    // N:0 — an INLINED routine carries its declaration and joins nothing, because
    // there is no call node to count. Both spellings, function and task.
    assert_eq!(
        decl(&s, "leaf", "pure8"),
        Some((file.clone(), 8, 18)),
        "{s}"
    );
    assert_eq!(decl(&s, "leaf", "st"), Some((file.clone(), 12, 8)), "{s}");
    assert!(rt_decl(&o, "top.u1.pure8").is_none(), "{o}");
    assert!(rt_decl(&o, "top.u1.st").is_none(), "{o}");

    // 0:1 — a class method has a runtime row and NO static row, as `uncounted`
    // promises. The join must not invent one.
    assert_eq!(rt_decl(&o, "C.m"), Some((file.clone(), 31, 16)), "{o}");
    assert!(decl(&s, "C", "m").is_none(), "{s}");
    assert!(decl(&s, "top", "m").is_none(), "{s}");
    assert!(decl(&s, "top", "C.m").is_none(), "{s}");

    // The new fields sit AFTER `sites`, so the existing column order — which
    // every hand parser in these tests keys on — is untouched.
    assert!(
        s.contains(
            "{\"module\": \"leaf\", \"name\": \"aut\", \"kind\": \"function\", \
             \"route\": \"frame\", \"sites\": 2, \"decl_file\": \""
        ),
        "column order moved: {s}"
    );
    // And the `(module, name)` sort is unchanged.
    assert_eq!(
        keys(&s),
        [
            ("leaf", "aut"),
            ("leaf", "never_called"),
            ("leaf", "ptk"),
            ("leaf", "pure8"),
            ("leaf", "st"),
            ("leaf", "tk"),
            ("leaf", "trp"),
            ("other", "aut"),
            ("top", "gb[0]$gf"),
            ("top", "gb[1]$gf"),
            ("top", "p::dbl"),
        ]
        .map(|(m, n)| (m.to_string(), n.to_string()))
        .to_vec(),
        "{s}"
    );
}

/// The declaration site must not depend on the profile flag: the static object is
/// emitted without `--obs-procs` and the triple is part of it.
#[test]
fn the_declaration_site_is_emitted_without_obs_procs() {
    // ONE directory, so the two runs are the same INPUT — see `run_json_in`.
    let d = std::env::temp_dir().join(format!("vita_obssub_declflag_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let (plain, _, c1) = run_json_in(&d, JOIN, &[]);
    let (procs, _, c2) = run_json_in(&d, JOIN, &["--obs-procs"]);
    assert_eq!((c1, c2), (0, 0));
    let s = subs_obj(&plain);
    // No runtime half exists here at all, and the triple is still there.
    assert!(plain.contains("\"subroutine_calls\": null"), "{plain}");
    assert_eq!(decl(&s, "leaf", "aut").map(|d| (d.1, d.2)), Some((9, 26)));
    assert_eq!(decl(&s, "top", "p::dbl").map(|d| (d.1, d.2)), Some((2, 26)));
    assert_eq!(
        s,
        subs_obj(&procs),
        "the profile flag moved the static object"
    );
}

/// The join is MANY-TO-MANY, and the triple names a SOURCE, not a row. Three
/// shapes the review measured, in one design: a package function reached by
/// its imported spelling in one module (inlined there) and by `p::mix` in
/// another (framed there) — two static rows under two modules, one of them
/// `inlined`, ONE triple, one runtime row; a hierarchical call to a declared
/// routine — a static row with `sites: 0` (the call SITE is uncounted) beside a
/// runtime row with `calls: 1`, same triple; and the class method that has no
/// static row at all.
const MANY: &str = r#"package p;
  function [7:0] mix(input [7:0] c); mix = c + 8'h01; endfunction
endpackage
module ma;
  import p::mix;
  logic [7:0] a;
  initial begin a = mix(8'h10); $display("ma %h", a); end
endmodule
module mb;
  logic [7:0] b;
  initial begin b = p::mix(8'h20); $display("mb %h", b); end
endmodule
module leaf;
  int acc;
  function automatic int bump(input int x);
    acc = acc + x;
    bump = acc;
  endfunction
  initial acc = 0;
endmodule
class C;
  function int m(input int x); return x + 1; endfunction
endclass
module top;
  ma u1(); mb u2(); leaf u();
  int z, cm;
  C c;
  initial begin
    c = new;
    #1; z = u.bump(7); cm = c.m(4);
    $display("z=%0d acc=%0d cm=%0d", z, u.acc, cm);
    #1 $finish;
  end
endmodule
"#;

#[test]
fn the_join_is_many_to_many_and_the_triple_names_a_source_not_a_row() {
    let (json, out, code) = run_json(MANY, &["--obs-procs"]);
    assert_eq!(code, 0, "expected exit 0, stdout:\n{out}");
    assert!(out.contains("ma 11"), "{out}");
    assert!(out.contains("mb 21"), "{out}");
    assert!(out.contains("z=7 acc=7 cm=5"), "{out}");
    let s = subs_obj(&json);
    let o = calls_obj(&json);

    // One package function, two calling modules, two spellings: TWO static rows
    // that share one triple. One is inlined and the other framed — the route is
    // per (module, name), never per declaration — so the `inlined` row shares
    // its triple with a `frame` row.
    let mix = decl(&s, "ma", "mix").expect("no (ma, mix) row");
    assert_eq!(mix.1, 2, "{s}");
    assert_eq!(mix.2, 18, "{s}");
    assert_eq!(decl(&s, "mb", "p::mix"), Some(mix.clone()), "{s}");
    assert_eq!(
        row(&s, "ma", "mix"),
        Some(("function".into(), "inlined".into(), 1))
    );
    assert_eq!(
        row(&s, "mb", "p::mix"),
        Some(("function".into(), "frame".into(), 1))
    );
    // The one runtime row (the framed call) joins BOTH static rows.
    assert_eq!(rt_decl(&o, "top.u2.mix"), Some(mix.clone()), "{o}");
    assert!(
        rt_decl(&o, "top.u1.mix").is_none(),
        "an inlined call has no runtime row: {o}"
    );

    // A hierarchical callee is a DECLARED routine: its static row exists with
    // `sites: 0` (the call site is what the static object does not count), and
    // the runtime row joins it.
    assert_eq!(
        row(&s, "leaf", "bump"),
        Some(("function".into(), "frame".into(), 0))
    );
    let bump = decl(&s, "leaf", "bump").expect("no (leaf, bump) row");
    assert_eq!((bump.1, bump.2), (15, 26), "{s}");
    assert_eq!(rt_decl(&o, "top.u.bump"), Some(bump), "{o}");

    // A class method has a runtime row and no static row.
    assert!(rt_decl(&o, "C.m").is_some(), "{o}");
    assert!(decl(&s, "C", "m").is_none(), "{s}");
    assert!(decl(&s, "top", "m").is_none(), "{s}");
}
