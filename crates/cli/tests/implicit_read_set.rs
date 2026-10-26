//! ENG-2: the implicit read set at variable granularity (`elaborate/src/read_set.rs`) — the
//! nets an `always_comb` / `always_latch` / static deferred assertion (IEEE 1800-2017
//! §9.2.2.2.1, §16.4.3) and an `always @*` / in-body `@(*)` (§9.4.2.2) wait on:
//!
//! * R2.1 every function called within an `always_comb` adds its body's reads — a void
//!   function (inlined or framed), a call through an output formal, a function writing a
//!   module net, a call nested in a walked body, a call in an lvalue index — minus its own
//!   variables (exception a);
//! * R2.2 a task adds its argument expressions only;
//! * R2.3 expressions in assertion action blocks add nothing (LRM over iverilog and
//!   verilator, ENG-2 O2); the condition adds as an `if` condition;
//! * R2.5 an `@*` reads a call's arguments, not an inlined static function / void
//!   function / task body; an inlined function's arguments are read in `always_comb` too;
//! * R2.6 exception b at variable granularity: a read whose constant prefix is covered by
//!   what the block writes (its statements, its action arms, a task's or function's output
//!   actuals, a called function's body — the last LRM over all three tools, ENG-2 OP1) is
//!   dropped; a write whose index is not provably constant covers nothing;
//! * R2.7 an `@*` leaves out the implicit lvalue read of `op=` / `++` / `--`;
//! * C8 the recompute after hierarchical resolution applies the same rule, armed by the
//!   hierarchical function-call and task-enable lanes too, and covers every in-body `@(*)`;
//! * round 2: an exclusion is relative to the read set's root (B1: an in-body `@(*)` inside
//!   a static task body reads its statement); a `ref` / `const ref` actual is not written by
//!   the block unless the callee writes the formal (B2); the in-body recompute reads only the
//!   controlled statement (B3); a rebuilt hierarchical select keeps its mark (B4); an NBA
//!   write covers like any write (N-NBA, LRM-literal = iv, owner flag);
//! * round 3: a builder that runs after lowering appends under the mark of the node it
//!   replaces or the call site's (R-SITE); every read set is computed once after the last
//!   post-lowering builder (R-SETTLE); an in-body `@(*)` keeps its statements by membership
//!   (R-MEMBER); a `ref` actual covers exactly the bits the subroutine writes of it, a
//!   system task / function output argument is a write (R1 / R3, owner flag OP-1); W3056 is
//!   decided from the settled set;
//! * round 4 (the frame part parked, R4): an in-body `@(*)` in a frame task body (an automatic
//!   or recursive task) keeps the set taken when that body is lowered — no nested call's
//!   arguments, no hierarchical operand (resolved later) — the frame classify reads it and
//!   W3056 is decided from it there; the settle leaves it alone;
//! * round 5: that set is taken by the walk the site used before ENG-2
//!   (`frame_star_read_set`: an inlined body's reads, not the call's arguments; a compound
//!   assignment's lvalue read), for every frame body — a frame task, a frame function, a class
//!   method — and an empty one warns W3056 there, as at PRE.
//!
//! The controls the rule must not move are here and in `implicit_sensitivity_controls.rs`
//! (ENG-0). Oracles: Icarus Verilog 13.0 `-g2012` (iv), sv2v 0.0.13 -> iverilog (s2v),
//! Verilator 5.052 `--binary --timing` (vl); vl runs every header block at time 0 and
//! prints a woken block twice, so its lines bear only on values. Each test's raw line
//! quotes PRE (73d22f5e) and every tool as captured (`|` joins lines). Every cell runs on
//! native, vm and interp; the C07b, S01 and N30 cells and six round-3 cells also run staged.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn scratch(tag: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_irs_{tag}_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn vita(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run vita")
}

/// Stdout lines without the `simulation ended` line, after a clean exit.
fn lines_of(o: &std::process::Output, what: &str) -> Vec<String> {
    let stdout = String::from_utf8_lossy(&o.stdout);
    let code = o.status.code().unwrap_or(-1);
    assert_eq!(
        code,
        0,
        "{what}: exit {code}\n{stdout}{}",
        String::from_utf8_lossy(&o.stderr)
    );
    stdout
        .lines()
        .filter(|l| !l.starts_with("simulation ended"))
        .map(str::to_string)
        .collect()
}

/// Every backend prints exactly `want`.
fn pin(src: &str, want: &[&str]) {
    pin_files(src, &[], want);
}

/// `pin` with `files` (name, contents) written next to `t.sv`, in vita's working directory.
fn pin_files(src: &str, files: &[(&str, &str)], want: &[&str]) {
    for be in ["native", "vm", "interp"] {
        let d = scratch(be);
        std::fs::write(d.join("t.sv"), src).unwrap();
        for (name, text) in files {
            std::fs::write(d.join(name), text).unwrap();
        }
        let got = lines_of(&vita(&d, &["--backend", be, "t.sv"]), be);
        let _ = std::fs::remove_dir_all(&d);
        assert_eq!(got, want, "backend {be}");
    }
}

/// The staged flow prints exactly `want`: `vcmp` (once to a work library and once again,
/// the unit recompiled in place), `velab -L`, `vrun`.
fn staged(src: &str, want: &[&str]) {
    let d = scratch("staged");
    std::fs::write(d.join("t.sv"), src).unwrap();
    for pass in ["a", "b"] {
        let o = vita(&d, &["vcmp", "--work", "lib", "t.sv"]);
        assert_eq!(
            o.status.code(),
            Some(0),
            "vcmp {pass}: {}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
    let o = vita(&d, &["velab", "-L", "lib", "--top", "top", "-o", "t.velab"]);
    assert_eq!(
        o.status.code(),
        Some(0),
        "velab: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let got = lines_of(&vita(&d, &["vrun", "t.velab"]), "vrun");
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(got, want, "staged");
}

/// The diagnostic codes `o` printed on stderr, in order (the default-timescale W1017 left out).
fn diag_codes(o: &std::process::Output) -> Vec<String> {
    String::from_utf8_lossy(&o.stderr)
        .lines()
        .filter_map(|l| {
            let i = l.find("[VITA-")?;
            let code = &l[i + 6..];
            Some(code[..code.find(']')?].to_string())
        })
        .filter(|c| c != "W1017")
        .collect()
}

/// Every backend prints exactly `want` and exactly the diagnostics `codes` (ENG-2 R4: a process
/// body's in-body `@(*)` decides W3056 from its settled set, a frame task body's when the body is
/// lowered).
fn pin_diag(src: &str, want: &[&str], codes: &[&str]) {
    for be in ["native", "vm", "interp"] {
        let d = scratch(be);
        std::fs::write(d.join("t.sv"), src).unwrap();
        let o = vita(&d, &["--backend", be, "t.sv"]);
        let _ = std::fs::remove_dir_all(&d);
        assert_eq!(diag_codes(&o), codes, "backend {be} diagnostics");
        assert_eq!(lines_of(&o, be), want, "backend {be}");
    }
}

/// Every backend stops at elaboration (exit 1) with the error `code` whose text holds `needle`.
fn pin_loud(src: &str, code: &str, needle: &str) {
    for be in ["native", "vm", "interp"] {
        let d = scratch(be);
        std::fs::write(d.join("t.sv"), src).unwrap();
        let o = vita(&d, &["--backend", be, "t.sv"]);
        let _ = std::fs::remove_dir_all(&d);
        let err = String::from_utf8_lossy(&o.stderr);
        assert_eq!(o.status.code(), Some(1), "backend {be}: {err}");
        let tag = format!("error[VITA-{code}]");
        assert!(
            err.lines().any(|l| l.contains(&tag) && l.contains(needle)),
            "backend {be}: no `{tag}` with `{needle}`:\n{err}"
        );
    }
}

/// A `.velab` carries the read sets as process edges and `Wait` nets, with no SimIr shape
/// change: format_version 38 is what keeps a format-37 `.velab` (old read sets inside)
/// from running. Forged from a fresh one with only the version changed.
#[test]
fn a_format_37_velab_is_stale() {
    let d = scratch("v37");
    std::fs::write(
        d.join("t.sv"),
        "module top; initial begin $display(\"ok\"); $finish; end endmodule\n",
    )
    .unwrap();
    assert_eq!(
        vita(&d, &["vcmp", "-o", "t.vu", "t.sv"]).status.code(),
        Some(0)
    );
    assert_eq!(
        vita(&d, &["velab", "-o", "t.velab", "t.vu"]).status.code(),
        Some(0)
    );
    let bytes = std::fs::read(d.join("t.velab")).unwrap();
    let (mut h, body) = vita_artifact::read_velab(&bytes).unwrap();
    assert_eq!(h.format_version, 38);
    h.format_version = 37;
    std::fs::write(d.join("old.velab"), vita_artifact::write_velab(&h, body)).unwrap();
    let o = vita(&d, &["vrun", "old.velab"]);
    let err = String::from_utf8_lossy(&o.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(o.status.code(), Some(2), "stderr:\n{err}");
    assert!(err.contains("E-ART-FORMAT-MISMATCH"), "stderr:\n{err}");
}

/// A format-37 `.vu` (the PRE parser's AST, its schema hash and version) is stale too.
#[test]
fn a_format_37_vu_is_stale() {
    let d = scratch("vu37");
    std::fs::write(
        d.join("t.sv"),
        "module top; initial begin $display(\"ok\"); $finish; end endmodule\n",
    )
    .unwrap();
    assert_eq!(
        vita(&d, &["vcmp", "-o", "t.vu", "t.sv"]).status.code(),
        Some(0)
    );
    let bytes = std::fs::read(d.join("t.vu")).unwrap();
    let (mut h, body) = vita_artifact::read_vu(&bytes).unwrap();
    h.format_version = 37;
    // `hdl_ast::SourceUnit`'s hash before ENG-2 (`hdl-ast/tests/schema_hash.rs`).
    h.schema_hash = [
        164, 72, 10, 182, 206, 128, 192, 91, 255, 27, 189, 168, 113, 129, 151, 126, 140, 254, 213,
        51, 124, 52, 250, 13, 148, 14, 210, 91, 254, 156, 234, 221,
    ];
    std::fs::write(d.join("old.vu"), vita_artifact::write_vu(&h, body)).unwrap();
    let o = vita(&d, &["velab", "-o", "old.velab", "old.vu"]);
    let err = String::from_utf8_lossy(&o.stderr).into_owned();
    let made = d.join("old.velab").exists();
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(o.status.code(), Some(2), "stderr:\n{err}");
    assert!(!made, "a stale .vu must not produce a .velab");
}

/// R2.1: `always_comb` reads the body of the automatic void function it calls (IEEE 1800-2017
/// :9045-9049 "The implicit sensitivity list of an always_comb includes the expansions of the
/// longest static prefix of each variable or select expression that is read within the block or
/// within any function called within the block" (exception a: :9049 variables declared within any
/// function called)); PRE never woke on `g`. iv = s2v = vl. Cell `C07b_comb_voidfn_auto_read`.
///
/// Raw lines: PRE `C t=0 z=0 | C t=5 z=0 | A end z=0`; iv `C t=0 z=0 | C t=5 z=0 | C t=6 z=1 | A
/// end z=1`; s2v `C t=0 z=0 | C t=5 z=0 | C t=6 z=1 | A end z=1`; vl `C t=0 z=0 | C t=5 z=0 | C
/// t=5 z=0 | C t=6 z=1 | C t=6 z=1 | A end z=1 | C t=8 z=1 | C t=8 z=1`.
#[test]
fn c07b_comb_reads_an_automatic_void_function_body() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y, z;\n\
  function automatic void upd(); z = g; endfunction\n\
  always_comb begin y = a; upd(); $display(\"C t=%0t z=%0d\", $time, z); end\n\
  initial begin #5 a = 1; #1 g = 1; #2 $display(\"A end z=%0d\", z); $finish; end\n\
endmodule\n\
",
        &["C t=0 z=0", "C t=5 z=0", "C t=6 z=1", "A end z=1"],
    );
}

/// Staged twin of `c07b_comb_reads_an_automatic_void_function_body`: the marks the rule reads are
/// in the `.vu` (vcmp a; vcmp b is the same unit compiled again; velab -L; vrun).
#[test]
fn c07b_staged_flow_matches_the_one_shot_run() {
    staged(
        "module top;\n\
  logic a = 0, g = 0, y, z;\n\
  function automatic void upd(); z = g; endfunction\n\
  always_comb begin y = a; upd(); $display(\"C t=%0t z=%0d\", $time, z); end\n\
  initial begin #5 a = 1; #1 g = 1; #2 $display(\"A end z=%0d\", z); $finish; end\n\
endmodule\n\
",
        &["C t=0 z=0", "C t=5 z=0", "C t=6 z=1", "A end z=1"],
    );
}

/// R2.1: `always_latch` is an `always_comb` for its read set; the automatic void function's body
/// read wakes it. iv = s2v = vl. Cell `G05_latch_voidfn_auto_read`.
///
/// Raw lines: PRE `L t=0 z=0 | E end z=0`; iv `L t=0 z=0 | L t=5 z=1 | E end z=1`; s2v `L t=0 z=0
/// | L t=5 z=1 | E end z=1`; vl `L t=0 z=0 | L t=5 z=1 | L t=5 z=1 | E end z=1 | L t=7 z=1 | L t=7
/// z=1`.
#[test]
fn g05_latch_reads_an_automatic_void_function_body() {
    pin(
        "module top;\n\
  logic en = 1, g = 0, z;\n\
  function automatic void upd(); z = g; endfunction\n\
  always_latch if (en) begin upd(); $display(\"L t=%0t z=%0d\", $time, z); end\n\
  initial begin #5 g = 1; #2 $display(\"E end z=%0d\", z); $finish; end\n\
endmodule\n\
",
        &["L t=0 z=0", "L t=5 z=1", "E end z=1"],
    );
}

/// R2.1: the walk is transitive: a value function called inside a void function's body adds its
/// reads. iv = s2v = vl. Cell `G06_comb_voidfn_nested_valfn`.
///
/// Raw lines: PRE `C t=0 z=0 | E end z=0`; iv `C t=0 z=0 | C t=5 z=1 | E end z=1`; s2v `C t=0 z=0
/// | C t=5 z=1 | E end z=1`; vl `C t=0 z=0 | C t=5 z=1 | C t=5 z=1 | E end z=1 | C t=7 z=1 | C t=7
/// z=1`.
#[test]
fn g06_comb_reads_a_value_function_called_by_a_void_function() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y, z;\n\
  function automatic logic rd(); return g; endfunction\n\
  function automatic void upd(); z = rd(); endfunction\n\
  always_comb begin y = a; upd(); $display(\"C t=%0t z=%0d\", $time, z); end\n\
  initial begin #5 g = 1; #2 $display(\"E end z=%0d\", z); $finish; end\n\
endmodule\n\
",
        &["C t=0 z=0", "C t=5 z=1", "E end z=1"],
    );
}

/// R2.1: a void function called under an `if` adds its body's reads. iv = s2v = vl. Cell
/// `G08_comb_voidfn_in_if`.
///
/// Raw lines: PRE `S t=6 z=0 | E end`; iv `S t=6 z=1 | E end`; s2v `S t=6 z=1 | E end`; vl `S t=6
/// z=1 | E end`.
#[test]
fn g08_comb_reads_a_void_function_called_in_an_if_arm() {
    pin(
        "module top;\n\
  logic a = 1, g = 0, z;\n\
  function automatic void upd(); z = g; endfunction\n\
  always_comb begin z = 0; if (a) upd(); end\n\
  initial begin #5 g = 1; #1 $display(\"S t=%0t z=%0d\", $time, z); #1 $display(\"E end\"); $finish; end\n\
endmodule\n\
",
        &["S t=6 z=1", "E end"],
    );
}

/// R2.1 + R2.3: a void function holding only an assertion adds the assertion's condition (an `if`
/// condition, :9061-9063). iv "F t=5", vl F t=5. Cell `C07e_comb_voidfn_assert_only`.
///
/// Raw lines: PRE `A end`; iv `F t=5 | A end`; s2v `A end`; vl `F t=5 | F t=5 | A end | F t=7 | F
/// t=7`.
#[test]
fn c07e_comb_reads_a_void_function_assertion_condition() {
    pin(
        "module top;\n\
  logic a = 0, b = 0, y;\n\
  function automatic void chk(); assert (a == 0) else $display(\"F t=%0t\", $time); endfunction\n\
  always_comb begin y = b; chk(); end\n\
  initial begin #5 a = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["F t=5", "A end"],
    );
}

/// R2.1 + R2.3: the void function's assertion condition `a` wakes the block (t=5); its action's
/// `k` does not (t=6). Hand-IEEE: IEEE 1800-2017 :9061-9063 "An expression used in an immediate
/// assertion (see 16.3) within the procedure, or in any function called within the procedure,
/// contributes to the implicit sensitivity list of an always_comb as if that expression were used
/// as a condition of an if statement. Expressions used in assertion action blocks do not
/// contribute to the implicit sensitivity list of an always_comb." (example :9064-9068: `A1:assert
/// (a != e) else if (!disable_error) $error("failed");` triggers on b, c, e only). iv adds the
/// action read (wakes at 6). Cell `C07_comb_fn_assert_action`.
///
/// Raw lines: PRE `C t=0 | A end`; iv `C t=0 | F t=5 k=0 | C t=5 | F t=6 k=1 | C t=6 | A end`; s2v
/// `C t=0 | C t=5 | C t=6 | A end`; vl `C t=0 | F t=5 k=0 | C t=5 | F t=5 k=0 | ...(14 lines)`.
#[test]
fn c07_comb_reads_a_void_function_condition_not_its_action() {
    pin(
        "module top;\n\
  logic a = 0, b = 0, y; int k = 0;\n\
  function automatic void chk(); assert (a == 0) else $display(\"F t=%0t k=%0d\", $time, k); endfunction\n\
  always_comb begin y = b; chk(); $display(\"C t=%0t\", $time); end\n\
  initial begin #5 a = 1; #1 k = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0", "F t=5 k=0", "C t=5", "A end"],
    );
}

/// R2.1: a void function calling a void function with argument `g` — the nested call's binds are
/// read in the walked body. iv = s2v = vl. Cell `N04_comb_voidfn_calls_voidfn_arg`.
///
/// Raw lines: PRE `C t=0 z=0 | A end z=0`; iv `C t=0 z=0 | C t=5 z=1 | A end z=1`; s2v `C t=0 z=0
/// | C t=5 z=1 | A end z=1`; vl `C t=0 z=0 | C t=5 z=1 | C t=5 z=1 | A end z=1 | C t=7 z=1 | C t=7
/// z=1`.
#[test]
fn n04_comb_reads_a_nested_void_call_argument() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y, z;\n\
  function automatic void put(input logic v); z = v; endfunction\n\
  function automatic void upd(); put(g); endfunction\n\
  always_comb begin y = a; upd(); $display(\"C t=%0t z=%0d\", $time, z); end\n\
  initial begin #5 g = 1; #2 $display(\"A end z=%0d\", z); $finish; end\n\
endmodule\n\
",
        &["C t=0 z=0", "C t=5 z=1", "A end z=1"],
    );
}

/// R2.1: a function with an output formal is lowered to a `Terminator::Call`; its body (`o = g`)
/// is walked. vl z=1 (iv and s2v refuse the output formal of a function); hand-IEEE IEEE 1800-2017
/// :9045-9049 "The implicit sensitivity list of an always_comb includes the expansions of the
/// longest static prefix of each variable or select expression that is read within the block or
/// within any function called within the block" (exception a: :9049 variables declared within any
/// function called). Cell `N05_comb_outformal_fn_body`.
///
/// Raw lines: PRE `C t=0 z=0 | A end z=0`; iv `(no iverilog binary)`; s2v `(no sv2v->iverilog
/// binary)`; vl `C t=0 z=0 | C t=5 z=1 | C t=5 z=1 | A end z=1 | C t=7 z=1 | C t=7 z=1`.
#[test]
fn n05_comb_reads_a_function_called_through_an_output_formal() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y, z;\n\
  function automatic logic f(input logic x, output logic o); o = g; return x; endfunction\n\
  always_comb begin y = f(a, z); $display(\"C t=%0t z=%0d\", $time, z); end\n\
  initial begin #5 g = 1; #2 $display(\"A end z=%0d\", z); $finish; end\n\
endmodule\n\
",
        &["C t=0 z=0", "C t=5 z=1", "A end z=1"],
    );
}

/// R2.1: a function whose body writes a module net is routed to a `Terminator::Call`; its read of
/// `g` wakes the block. iv = s2v = vl. (native prints the W4030 fallback on stderr.) Cell
/// `N06_comb_bodywrite_fn`.
///
/// Raw lines: PRE `C t=0 z=0 | A end z=0`; iv `C t=0 z=0 | C t=5 z=1 | A end z=1`; s2v `C t=0 z=0
/// | C t=5 z=1 | A end z=1`; vl `C t=0 z=0 | C t=5 z=1 | C t=5 z=1 | A end z=1 | C t=7 z=1 | C t=7
/// z=1`.
#[test]
fn n06_comb_reads_a_function_that_writes_a_module_net() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y, z;\n\
  function automatic logic f(input logic x); z = g; return x; endfunction\n\
  always_comb begin y = f(a); $display(\"C t=%0t z=%0d\", $time, z); end\n\
  initial begin #5 g = 1; #2 $display(\"A end z=%0d\", z); $finish; end\n\
endmodule\n\
",
        &["C t=0 z=0", "C t=5 z=1", "A end z=1"],
    );
}

/// R2.1: `y[idx()] = 1` — the function called in the lvalue index adds its body's reads. iv = s2v
/// = vl. Cell `N33_comb_lval_index_fn`.
///
/// Raw lines: PRE `C t=0 y=0010 | A end y=0010`; iv `C t=0 y=0010 | C t=5 y=0100 | A end y=0100`;
/// s2v `C t=0 y=0010 | C t=5 y=0100 | A end y=0100`; vl `C t=0 y=0010 | C t=5 y=0100 | C t=5
/// y=0100 | A end y=0100 | C t=7 y=0100 | C t=7 y=0100`.
#[test]
fn n33_comb_reads_a_function_called_in_an_lvalue_index() {
    pin(
        "module top;\n\
  logic [3:0] y; logic a = 0, g = 0;\n\
  function automatic int idx(); return g ? 2 : 1; endfunction\n\
  always_comb begin y = 4'b0; y[idx()] = 1'b1; $display(\"C t=%0t y=%b\", $time, y); end\n\
  initial begin #5 g = 1; #2 $display(\"A end y=%b\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0010", "C t=5 y=0100", "A end y=0100"],
    );
}

/// R2.1 vs R2.2 (the `TaskDef.is_void_fn` mark): the same body as a static void function wakes on
/// `g`, as a static task does not. iv = s2v. PRE woke both. Cell
/// `N30_comb_static_voidfn_vs_task_body`.
///
/// Raw lines: PRE `F t=0 z=0 | T t=0 w=0 | F t=5 z=1 | T t=5 w=1 | A end z=1 w=1`; iv `T t=0 w=0 |
/// F t=0 z=0 | F t=5 z=1 | A end z=1 w=0`; s2v `F t=0 z=0 | T t=0 w=0 | F t=5 z=1 | A end z=1
/// w=0`; vl `F t=0 z=0 | T t=0 w=0 | F t=5 z=1 | T t=5 w=1 | ...(11 lines)`.
#[test]
fn n30_comb_reads_a_static_void_function_but_not_a_static_task() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, ya, yb, z, w;\n\
  function void fv(); z = g; endfunction\n\
  task tt(); w = g; endtask\n\
  always_comb begin ya = a; fv(); $display(\"F t=%0t z=%0d\", $time, z); end\n\
  always_comb begin yb = a; tt(); $display(\"T t=%0t w=%0d\", $time, w); end\n\
  initial begin #5 g = 1; #2 $display(\"A end z=%0d w=%0d\", z, w); $finish; end\n\
endmodule\n\
",
        &["F t=0 z=0", "T t=0 w=0", "F t=5 z=1", "A end z=1 w=0"],
    );
}

/// Staged twin of `n30_comb_reads_a_static_void_function_but_not_a_static_task`: the marks the
/// rule reads are in the `.vu` (vcmp a; vcmp b is the same unit compiled again; velab -L; vrun).
#[test]
fn n30_staged_flow_matches_the_one_shot_run() {
    staged(
        "module top;\n\
  logic a = 0, g = 0, ya, yb, z, w;\n\
  function void fv(); z = g; endfunction\n\
  task tt(); w = g; endtask\n\
  always_comb begin ya = a; fv(); $display(\"F t=%0t z=%0d\", $time, z); end\n\
  always_comb begin yb = a; tt(); $display(\"T t=%0t w=%0d\", $time, w); end\n\
  initial begin #5 g = 1; #2 $display(\"A end z=%0d w=%0d\", z, w); $finish; end\n\
endmodule\n\
",
        &["F t=0 z=0", "T t=0 w=0", "F t=5 z=1", "A end z=1 w=0"],
    );
}

/// R2.1 exception a: two `always_comb` call one STATIC void function whose formal net is shared;
/// the formal is the function's own variable, so neither block reads it (PRE ping-ponged to F4016
/// at t=5). vl no loop; s2v loops; iv refuses the output formal; hand "A t=0|A t=8|E end". Cell
/// `N32_comb_two_static_voidfn_input_formal`.
///
/// Raw lines: PRE `A t=0 | A t=5 | A t=5 | A t=5 | ...(500002 lines)`; iv `(no iverilog binary)`;
/// s2v `A t=0 | A t=5 | A t=5 | A t=5 | ...(15658326 lines)`; vl `A t=0 | E end`.
#[test]
fn n32_comb_drops_a_static_void_function_shared_formal() {
    pin(
        "module top;\n\
  logic a = 0, b = 0, ya, yb;\n\
  function void put(input logic v, output logic o); o = ~v; endfunction\n\
  always_comb begin put(a, ya); $display(\"A t=%0t\", $time); end\n\
  always_comb begin put(b, yb); end\n\
  initial begin #5 b = 1; #3 a = 1; #2 $display(\"E end\"); $finish; end\n\
endmodule\n\
",
        &["A t=0", "A t=8", "E end"],
    );
}

/// R2.2: a static task adds its argument expressions only (IEEE 1800-2017 :9056-9057 "Task calls
/// are allowed in an always_comb, but the contents of the tasks do not add anything to the
/// sensitivity list."). iv = s2v. Cell `C22_comb_task_args`.
///
/// Raw lines: PRE `C t=0 | C t=5 | C t=6 | A end`; iv `C t=0 | C t=5 | A end`; s2v `C t=0 | C t=5
/// | A end`; vl `C t=0 | A end`.
#[test]
fn c22_comb_reads_a_static_task_arguments_not_its_body() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, o;\n\
  task t(input logic x); o = x & g; endtask\n\
  always_comb begin t(a); $display(\"C t=%0t\", $time); end\n\
  initial begin #5 a = 1; #1 g = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0", "C t=5", "A end"],
    );
}

/// R2.2: the static task's body read does not wake the block. iv = s2v. Cell
/// `X04s_comb_task_read_sens`.
///
/// Raw lines: PRE `C t=0 a=0 | C t=5 a=1 | C t=6 a=1 | A end`; iv `C t=0 a=0 | C t=5 a=1 | A end`;
/// s2v `C t=0 a=0 | C t=5 a=1 | A end`; vl `C t=0 a=0 | C t=5 a=1 | C t=5 a=1 | C t=6 a=1 | C t=6
/// a=1 | A end | C t=20 a=1 | C t=20 a=1`.
#[test]
fn x04s_comb_does_not_read_a_static_task_body() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, o;\n\
  task t; o = g; endtask\n\
  always_comb begin t(); $display(\"C t=%0t a=%0d\", $time, a); end\n\
  initial begin #5 a = 1; #1 g = 1; end\n\
  initial #20 $display(\"A end\");\n\
endmodule\n\
",
        &["C t=0 a=0", "C t=5 a=1", "A end"],
    );
}

/// R2.2: same rule, a second shape. iv = s2v. Cell `T1s_comb_task_sens`.
///
/// Raw lines: PRE `C t=0 g=0 a=0 | C t=5 g=1 a=0 | C t=8 g=1 a=1 | A end`; iv `C t=0 g=0 a=0 | C
/// t=8 g=1 a=1 | A end`; s2v `C t=0 g=0 a=0 | C t=8 g=1 a=1 | A end`; vl `C t=0 g=0 a=0 | C t=5
/// g=1 a=0 | C t=5 g=1 a=0 | C t=8 g=1 a=1 | C t=8 g=1 a=1 | A end | C t=20 g=1 a=1 | C t=20 g=1
/// a=1`.
#[test]
fn t1s_comb_does_not_read_a_static_task_body() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y;\n\
  task t; $display(\"C t=%0t g=%b a=%b\", $time, g, a); endtask\n\
  always_comb begin y = a; t(); end\n\
  initial begin #5 g = 1; #3 a = 1; end\n\
  initial #20 $display(\"A end\");\n\
endmodule\n\
",
        &["C t=0 g=0 a=0", "C t=8 g=1 a=1", "A end"],
    );
}

/// R2.2: an event triggered from the `always_comb` fires on the argument change only. iv = s2v.
/// Cell `V30s_ev_from_comb_task_read_sens`.
///
/// Raw lines: PRE `W t=5 | A end`; iv `W t=8 | A end`; s2v `W t=8 | A end`; vl `A end`.
#[test]
fn v30s_comb_task_body_read_does_not_drive_an_event() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y, z; event e;\n\
  task chk; z = g; endtask\n\
  always_comb begin y = a; chk(); ->e; end\n\
  initial begin\n\
    #1 @(e);\n\
    $display(\"W t=%0t\", $time);\n\
  end\n\
  initial begin #5 g = 1; #3 a = 1; end\n\
  initial #20 $display(\"A end\");\n\
endmodule\n\
",
        &["W t=8", "A end"],
    );
}

/// R2.2: same rule through an `int`. iv = s2v. Cell `V31s_int_from_comb_task_read_sens`.
///
/// Raw lines: PRE `W t=5 n=1 | A end`; iv `W t=8 n=1 | A end`; s2v `W t=8 n=1 | A end`; vl `W t=5
/// n=1 | A end`.
#[test]
fn v31s_comb_task_body_read_does_not_wake_an_int_chain() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y; int n = 0;\n\
  task chk; if (g) n = 1; endtask\n\
  always_comb begin y = a; chk(); end\n\
  initial begin\n\
    @(n);\n\
    $display(\"W t=%0t n=%0d\", $time, n);\n\
  end\n\
  initial begin #5 g = 1; #3 a = 1; end\n\
  initial #20 $display(\"A end\");\n\
endmodule\n\
",
        &["W t=8 n=1", "A end"],
    );
}

/// R2.2: two `always_comb` call one static task; its shared formal and copy-out read are the
/// task's contents and add nothing (PRE F4016 at t=5). iv and s2v loop; vl "A t=0|E end";
/// hand-IEEE IEEE 1800-2017 :9056-9057 "Task calls are allowed in an always_comb, but the contents
/// of the tasks do not add anything to the sensitivity list.". Cell
/// `G03_comb_stask_shared_formal`.
///
/// Raw lines: PRE `A t=0 | A t=5 | A t=5 | A t=5 | ...(500002 lines)`; iv `A t=0 | A t=5 | A t=5 |
/// A t=5 | ...(11560960 lines)`; s2v `A t=0 | A t=5 | A t=5 | A t=5 | ...(10452992 lines)`; vl `A
/// t=0 | E end`.
#[test]
fn g03_comb_drops_a_static_task_shared_formal() {
    pin(
        "module top;\n\
  logic a = 0, b = 0, ya, yb;\n\
  task t(input logic x, output logic y); y = ~x; endtask\n\
  always_comb begin t(a, ya); $display(\"A t=%0t\", $time); end\n\
  always_comb begin t(b, yb); end\n\
  initial begin #5 b = 1; #3 a = 1; #2 $display(\"E end\"); $finish; end\n\
endmodule\n\
",
        &["A t=0", "A t=8", "E end"],
    );
}

/// R2.6 (ENG-2 OP2): a task's OUTPUT actual `v[0]` is written within the block, so the block's
/// read of `v[0]` is covered (exception b, IEEE 1800-2017 :9047-9050 "... read within the block or
/// within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"). = iv (no wake); PRE = s2v woke. Cell `N27_comb_task_outactual_covers`.
///
/// Raw lines: PRE `C t=0 y=0 | C t=5 y=0 | C t=8 y=1 | A end`; iv `C t=0 y=0 | C t=8 y=1 | A end`;
/// s2v `C t=0 y=0 | C t=5 y=0 | C t=8 y=1 | A end`; vl `C t=0 y=0 | C t=5 y=0 | C t=5 y=0 | C t=8
/// y=1 | C t=8 y=1 | A end | C t=10 y=1 | C t=10 y=1`.
#[test]
fn n27_comb_task_output_actual_covers_its_read() {
    pin(
        "module top;\n\
  logic [1:0] v; logic b = 0, y;\n\
  task automatic tk(output logic o); o = 0; endtask\n\
  always_comb begin tk(v[0]); y = v[0] | b; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 v[1] = 1; #3 b = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0", "C t=8 y=1", "A end"],
    );
}

/// R2.3 (ENG-2 O2, LRM over both tools): the action's `k` does not wake the block; the condition
/// `a` does. IEEE 1800-2017 :9061-9063 "An expression used in an immediate assertion (see 16.3)
/// within the procedure, or in any function called within the procedure, contributes to the
/// implicit sensitivity list of an always_comb as if that expression were used as a condition of
/// an if statement. Expressions used in assertion action blocks do not contribute to the implicit
/// sensitivity list of an always_comb." (example :9064-9068: `A1:assert (a != e) else if
/// (!disable_error) $error("failed");` triggers on b, c, e only). Cell `S01_comb_imm_action_sens`.
///
/// Raw lines: PRE `C t=5 k=0 | C t=6 k=1 | A end`; iv `C t=5 k=0 | C t=6 k=1 | A end`; s2v `A
/// end`; vl `C t=5 k=0 | C t=5 k=0 | C t=6 k=1 | C t=6 k=1 | A end | C t=20 k=1 | C t=20 k=1`.
#[test]
fn s01_comb_does_not_read_an_immediate_assertion_action() {
    pin(
        "module top;\n\
  logic a = 0; int k = 0;\n\
  always_comb assert (a == 0) else $display(\"C t=%0t k=%0d\", $time, k);\n\
  initial begin #5 a = 1; #1 k = 1; end\n\
  initial #20 $display(\"A end\");\n\
endmodule\n\
",
        &["C t=5 k=0", "A end"],
    );
}

/// Staged twin of `s01_comb_does_not_read_an_immediate_assertion_action`: the marks the rule reads
/// are in the `.vu` (vcmp a; vcmp b is the same unit compiled again; velab -L; vrun).
#[test]
fn s01_staged_flow_matches_the_one_shot_run() {
    staged(
        "module top;\n\
  logic a = 0; int k = 0;\n\
  always_comb assert (a == 0) else $display(\"C t=%0t k=%0d\", $time, k);\n\
  initial begin #5 a = 1; #1 k = 1; end\n\
  initial #20 $display(\"A end\");\n\
endmodule\n\
",
        &["C t=5 k=0", "A end"],
    );
}

/// R2.3 (O2): a function called inside an assertion action adds nothing. IEEE 1800-2017 :9061-9063
/// "An expression used in an immediate assertion (see 16.3) within the procedure, or in any
/// function called within the procedure, contributes to the implicit sensitivity list of an
/// always_comb as if that expression were used as a condition of an if statement. Expressions used
/// in assertion action blocks do not contribute to the implicit sensitivity list of an
/// always_comb." (example :9064-9068: `A1:assert (a != e) else if (!disable_error)
/// $error("failed");` triggers on b, c, e only). Cell `C23_comb_action_fncall`.
///
/// Raw lines: PRE `C t=5 f=0 | C t=6 f=1 | A end`; iv `C t=5 f=0 | C t=6 f=1 | A end`; s2v `A
/// end`; vl `C t=5 f=0 | C t=5 f=0 | C t=6 f=1 | C t=6 f=1 | A end | C t=8 f=1 | C t=8 f=1`.
#[test]
fn c23_comb_does_not_read_a_function_called_in_an_action() {
    pin(
        "module top;\n\
  logic a = 0; int k = 0;\n\
  function automatic int f(); return k; endfunction\n\
  always_comb assert (a == 0) else $display(\"C t=%0t f=%0d\", $time, f());\n\
  initial begin #5 a = 1; #1 k = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=5 f=0", "A end"],
    );
}

/// R2.3 (O2): a static deferred assertion is an `always_comb` (§16.4.3); its action's reads add
/// nothing. IEEE 1800-2017 :9061-9063 "An expression used in an immediate assertion (see 16.3)
/// within the procedure, or in any function called within the procedure, contributes to the
/// implicit sensitivity list of an always_comb as if that expression were used as a condition of
/// an if statement. Expressions used in assertion action blocks do not contribute to the implicit
/// sensitivity list of an always_comb." (example :9064-9068: `A1:assert (a != e) else if
/// (!disable_error) $error("failed");` triggers on b, c, e only). Cell `C24_item_action_k`.
///
/// Raw lines: PRE `F t=5 k=0 | F t=6 k=1 | A end`; iv `(no iverilog binary)`; s2v `A end`; vl `F
/// t=5 k=0 | F t=5 k=0 | F t=6 k=1 | F t=6 k=1 | A end | F t=8 k=1 | F t=8 k=1`.
#[test]
fn c24_static_deferred_assertion_does_not_read_its_action() {
    pin(
        "module top;\n\
  logic a = 0; int k = 0;\n\
  a1: assert #0 (a == 0) else $display(\"F t=%0t k=%0d\", $time, k);\n\
  initial begin #5 a = 1; #1 k = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["F t=5 k=0", "A end"],
    );
}

/// R2.3 (O2): the PASS action is an action block too. IEEE 1800-2017 :9061-9063 "An expression
/// used in an immediate assertion (see 16.3) within the procedure, or in any function called
/// within the procedure, contributes to the implicit sensitivity list of an always_comb as if that
/// expression were used as a condition of an if statement. Expressions used in assertion action
/// blocks do not contribute to the implicit sensitivity list of an always_comb." (example
/// :9064-9068: `A1:assert (a != e) else if (!disable_error) $error("failed");` triggers on b, c, e
/// only). Cell `G09_comb_pass_action_k`.
///
/// Raw lines: PRE `P t=0 k=0 | P t=5 k=1 | E end`; iv `P t=0 k=0 | P t=5 k=1 | E end`; s2v `E
/// end`; vl `P t=0 k=0 | P t=5 k=1 | P t=5 k=1 | E end | P t=7 k=1 | P t=7 k=1`.
#[test]
fn g09_comb_does_not_read_a_pass_action() {
    pin(
        "module top;\n\
  logic a = 0; int k = 0;\n\
  always_comb assert (a == 0) $display(\"P t=%0t k=%0d\", $time, k);\n\
  initial begin #5 k = 1; #2 $display(\"E end\"); $finish; end\n\
endmodule\n\
",
        &["P t=0 k=0", "E end"],
    );
}

/// R2.3 (O2): a task called in an action adds nothing (its arguments neither). IEEE 1800-2017
/// :9061-9063 "An expression used in an immediate assertion (see 16.3) within the procedure, or in
/// any function called within the procedure, contributes to the implicit sensitivity list of an
/// always_comb as if that expression were used as a condition of an if statement. Expressions used
/// in assertion action blocks do not contribute to the implicit sensitivity list of an
/// always_comb." (example :9064-9068: `A1:assert (a != e) else if (!disable_error)
/// $error("failed");` triggers on b, c, e only). Cell `N11_comb_action_taskcall`.
///
/// Raw lines: PRE `C t=5 k=0 | C t=6 k=1 | A end`; iv `C t=5 k=0 | C t=6 k=1 | A end`; s2v `A
/// end`; vl `C t=5 k=0 | C t=5 k=0 | C t=6 k=1 | C t=6 k=1 | A end | C t=8 k=1 | C t=8 k=1`.
#[test]
fn n11_comb_does_not_read_a_task_called_in_an_action() {
    pin(
        "module top;\n\
  logic a = 0; int k = 0;\n\
  task automatic rep(input int v); $display(\"C t=%0t k=%0d\", $time, v); endtask\n\
  always_comb assert (a == 0) else rep(k);\n\
  initial begin #5 a = 1; #1 k = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=5 k=0", "A end"],
    );
}

/// R2.3 (O2): `assume` is an immediate assertion (§16.3). IEEE 1800-2017 :9061-9063 "An expression
/// used in an immediate assertion (see 16.3) within the procedure, or in any function called
/// within the procedure, contributes to the implicit sensitivity list of an always_comb as if that
/// expression were used as a condition of an if statement. Expressions used in assertion action
/// blocks do not contribute to the implicit sensitivity list of an always_comb." (example
/// :9064-9068: `A1:assert (a != e) else if (!disable_error) $error("failed");` triggers on b, c, e
/// only). Cell `N12_comb_assume_action`.
///
/// Raw lines: PRE `C t=5 k=0 | C t=6 k=1 | A end`; iv `C t=5 k=0 | C t=6 k=1 | A end`; s2v `A
/// end`; vl `C t=5 k=0 | C t=5 k=0 | C t=6 k=1 | C t=6 k=1 | A end | C t=8 k=1 | C t=8 k=1`.
#[test]
fn n12_comb_does_not_read_an_assume_action() {
    pin(
        "module top;\n\
  logic a = 0; int k = 0;\n\
  always_comb assume (a == 0) else $display(\"C t=%0t k=%0d\", $time, k);\n\
  initial begin #5 a = 1; #1 k = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=5 k=0", "A end"],
    );
}

/// R2.3 (O2): `assert final` action. IEEE 1800-2017 :9061-9063 "An expression used in an immediate
/// assertion (see 16.3) within the procedure, or in any function called within the procedure,
/// contributes to the implicit sensitivity list of an always_comb as if that expression were used
/// as a condition of an if statement. Expressions used in assertion action blocks do not
/// contribute to the implicit sensitivity list of an always_comb." (example :9064-9068: `A1:assert
/// (a != e) else if (!disable_error) $error("failed");` triggers on b, c, e only). Cell
/// `N13_comb_final_action`.
///
/// Raw lines: PRE `C t=5 k=0 | C t=6 k=1 | A end`; iv `(no iverilog binary)`; s2v `A end`; vl `C
/// t=5 k=0 | C t=5 k=0 | C t=6 k=1 | C t=6 k=1 | A end | C t=8 k=1 | C t=8 k=1`.
#[test]
fn n13_comb_does_not_read_a_final_deferred_action() {
    pin(
        "module top;\n\
  logic a = 0; int k = 0;\n\
  always_comb assert final (a == 0) else $display(\"C t=%0t k=%0d\", $time, k);\n\
  initial begin #5 a = 1; #1 k = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=5 k=0", "A end"],
    );
}

/// R2.3 (O2): `assert #0` action. IEEE 1800-2017 :9061-9063 "An expression used in an immediate
/// assertion (see 16.3) within the procedure, or in any function called within the procedure,
/// contributes to the implicit sensitivity list of an always_comb as if that expression were used
/// as a condition of an if statement. Expressions used in assertion action blocks do not
/// contribute to the implicit sensitivity list of an always_comb." (example :9064-9068: `A1:assert
/// (a != e) else if (!disable_error) $error("failed");` triggers on b, c, e only). Cell
/// `N14_comb_defer0_action`.
///
/// Raw lines: PRE `C t=5 k=0 | C t=6 k=1 | A end`; iv `(no iverilog binary)`; s2v `A end`; vl `C
/// t=5 k=0 | C t=5 k=0 | C t=6 k=1 | C t=6 k=1 | A end | C t=8 k=1 | C t=8 k=1`.
#[test]
fn n14_comb_does_not_read_a_deferred_action() {
    pin(
        "module top;\n\
  logic a = 0; int k = 0;\n\
  always_comb assert #0 (a == 0) else $display(\"C t=%0t k=%0d\", $time, k);\n\
  initial begin #5 a = 1; #1 k = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=5 k=0", "A end"],
    );
}

/// R2.3 (O2): an action inside a called static void function adds nothing (:9061 "or in any
/// function called within the procedure"). IEEE 1800-2017 :9061-9063 "An expression used in an
/// immediate assertion (see 16.3) within the procedure, or in any function called within the
/// procedure, contributes to the implicit sensitivity list of an always_comb as if that expression
/// were used as a condition of an if statement. Expressions used in assertion action blocks do not
/// contribute to the implicit sensitivity list of an always_comb." (example :9064-9068: `A1:assert
/// (a != e) else if (!disable_error) $error("failed");` triggers on b, c, e only). Cell
/// `N15_comb_static_voidfn_assert_action`.
///
/// Raw lines: PRE `C t=0 | F t=5 k=0 | C t=5 | F t=6 k=1 | C t=6 | A end`; iv `C t=0 | F t=5 k=0 |
/// C t=5 | F t=6 k=1 | C t=6 | A end`; s2v `C t=0 | C t=5 | C t=6 | A end`; vl `C t=0 | F t=5 k=0
/// | C t=5 | F t=5 k=0 | ...(14 lines)`.
#[test]
fn n15_comb_does_not_read_an_action_in_a_called_void_function() {
    pin(
        "module top;\n\
  logic a = 0, b = 0, y; int k = 0;\n\
  function void chk(); assert (a == 0) else $display(\"F t=%0t k=%0d\", $time, k); endfunction\n\
  always_comb begin y = b; chk(); $display(\"C t=%0t\", $time); end\n\
  initial begin #5 a = 1; #1 k = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0", "F t=5 k=0", "C t=5", "A end"],
    );
}

/// R2.5: `always @*` does not read the inlined body of a static task. iv = s2v. Cell
/// `C02_star_task`.
///
/// Raw lines: PRE `C t=5 | C t=6 | A end`; iv `C t=5 | A end`; s2v `C t=5 | A end`; vl `C t=0 | A
/// end`.
#[test]
fn c02_star_does_not_read_a_static_task_body() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, o, y;\n\
  task t; o = g; endtask\n\
  always @* begin y = a; t(); $display(\"C t=%0t\", $time); end\n\
  initial begin #5 a = 1; #1 g = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=5", "A end"],
    );
}

/// R2.5: `always @*` does not read the inlined body of a static function (IEEE 1800-2017
/// §9.2.2.2.2 :9077-9078 "always_comb is sensitive to changes within the contents of a function,
/// whereas always @* is only sensitive to changes to the arguments of a function."). iv = s2v.
/// Cell `C03_star_sfn`.
///
/// Raw lines: PRE `C t=5 | C t=6 | A end`; iv `C t=5 | A end`; s2v `C t=5 | A end`; vl `C t=0 | A
/// end`.
#[test]
fn c03_star_does_not_read_a_static_function_body() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y;\n\
  function logic f(); f = g; endfunction\n\
  always @* begin y = a ^ f(); $display(\"C t=%0t\", $time); end\n\
  initial begin #5 a = 1; #1 g = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=5", "A end"],
    );
}

/// R2.5: `y = f(a)` where static `f` ignores its formal and returns `g`: the argument `a` wakes,
/// `g` does not (value: y=0). iv = s2v. Cell `N03_star_sfn_unused_formal`.
///
/// Raw lines: PRE `C t=6 y=1 | A end y=1`; iv `C t=5 y=0 | A end y=0`; s2v `C t=5 y=0 | A end
/// y=0`; vl `C t=0 y=0 | C t=5 y=0 | C t=5 y=0 | C t=6 y=1 | C t=6 y=1 | A end y=1 | C t=8 y=1 | C
/// t=8 y=1`.
#[test]
fn n03_star_reads_a_static_function_argument_not_its_body() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y;\n\
  function logic f(input logic x); f = g; endfunction\n\
  always @* begin y = f(a); $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 a = 1; #1 g = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=5 y=0", "A end y=0"],
    );
}

/// R2.5 / R2.1: an `always_comb` reads the argument expression of an inlined static function even
/// where the body ignores the formal. iv = s2v. Cell `N03c_comb_sfn_unused_formal`.
///
/// Raw lines: PRE `C t=0 y=0 | C t=6 y=1 | A end y=1`; iv `C t=0 y=0 | C t=5 y=0 | C t=6 y=1 | A
/// end y=1`; s2v `C t=0 y=0 | C t=5 y=0 | C t=6 y=1 | A end y=1`; vl `C t=0 y=0 | C t=5 y=0 | C
/// t=5 y=0 | C t=6 y=1 | C t=6 y=1 | A end y=1 | C t=8 y=1 | C t=8 y=1`.
#[test]
fn n03c_comb_reads_a_static_function_argument_it_ignores() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y;\n\
  function logic f(input logic x); f = g; endfunction\n\
  always_comb begin y = f(a); $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 a = 1; #1 g = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0", "C t=5 y=0", "C t=6 y=1", "A end y=1"],
    );
}

/// R2.5: `always @*` does not read the inlined body of a static void function. iv = s2v. Cell
/// `N34_star_static_voidfn_read`.
///
/// Raw lines: PRE `C t=5 z=0 | C t=6 z=1 | A end z=1`; iv `C t=5 z=0 | A end z=0`; s2v `C t=5 z=0
/// | A end z=0`; vl `C t=0 z=0 | C t=5 z=0 | C t=5 z=0 | C t=6 z=1 | C t=6 z=1 | A end z=1 | C t=8
/// z=1 | C t=8 z=1`.
#[test]
fn n34_star_does_not_read_a_static_void_function_body() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y, z;\n\
  function void upd(); z = g; endfunction\n\
  always @* begin y = a; upd(); $display(\"C t=%0t z=%0d\", $time, z); end\n\
  initial begin #5 a = 1; #1 g = 1; #2 $display(\"A end z=%0d\", z); $finish; end\n\
endmodule\n\
",
        &["C t=5 z=0", "A end z=0"],
    );
}

/// R2.5: an in-body `@(*)` follows the same rule. iv = s2v. Cell `N23_inbody_star_sfn`.
///
/// Raw lines: PRE `C t=5 | C t=6 | A end`; iv `C t=5 | A end`; s2v `C t=5 | A end`; vl `(no
/// verilator binary) | %Error-UNSUPPORTED: t.sv:4:25: Unsupported: Event control with implicit
/// sensitivity (@*) in this context; use an explicit sensitivity list instead | %Error: Exiting
/// due to 1 error(s)`.
#[test]
fn n23_inbody_star_does_not_read_a_static_function_body() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y;\n\
  function logic f(); f = g; endfunction\n\
  initial forever begin @* y = a ^ f(); $display(\"C t=%0t\", $time); end\n\
  initial begin #5 a = 1; #1 g = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=5", "A end"],
    );
}

/// R2.6: a read of what the block itself writes is exception b. = iv; hand-IEEE IEEE 1800-2017
/// :9047-9050 "... read within the block or within any function called within the block with the
/// following exceptions: ... b) Any expression that is also written within the block or within any
/// function called within the block". Cell `C30_comb_write_then_read_same`.
///
/// Raw lines: PRE `C t=0 y=0 | C t=5 y=0 | C t=8 y=0 | A end`; iv `C t=0 y=0 | C t=8 y=0 | A end`;
/// s2v `C t=0 y=0 | C t=5 y=0 | C t=8 y=0 | A end`; vl `C t=0 y=0 | C t=5 y=0 | C t=5 y=0 | C t=8
/// y=0 | C t=8 y=0 | A end | C t=10 y=0 | C t=10 y=0`.
#[test]
fn c30_comb_drops_a_read_the_block_writes_first() {
    pin(
        "module top;\n\
  logic [1:0] v; logic a = 0, b = 0, y;\n\
  always_comb begin v[0] = a; y = v[0] & b; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 v[1] = 1; #3 b = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0", "C t=8 y=0", "A end"],
    );
}

/// R2.6 (ENG-2 OP1, LRM over all three tools): `v[0]` written only inside the called void function
/// covers the block's read of `v[0]`, so the t=5 write of `v[1]` does not wake it. IEEE 1800-2017
/// :9047-9050 "... read within the block or within any function called within the block with the
/// following exceptions: ... b) Any expression that is also written within the block or within any
/// function called within the block". iv, s2v and vl all wake at 5. Cell
/// `N25_comb_voidfn_write_covers_read`.
///
/// Raw lines: PRE `C t=0 y=0 | C t=5 y=0 | C t=8 y=1 | A end`; iv `C t=0 y=0 | C t=5 y=0 | C t=8
/// y=1 | A end`; s2v `C t=0 y=0 | C t=5 y=0 | C t=8 y=1 | A end`; vl `C t=0 y=0 | C t=5 y=0 | C
/// t=5 y=0 | C t=8 y=1 | C t=8 y=1 | A end | C t=10 y=1 | C t=10 y=1`.
#[test]
fn n25_comb_drops_a_read_written_by_a_called_function() {
    pin(
        "module top;\n\
  logic [1:0] v; logic b = 0, y;\n\
  function automatic void clr(); v[0] = 0; endfunction\n\
  always_comb begin clr(); y = v[0] | b; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 v[1] = 1; #3 b = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0", "C t=8 y=1", "A end"],
    );
}

/// R2.6, §5.2 row 30 (O16): the genvar-indexed `any[k] = '0; any[k] |= ...` pair per element —
/// each block's own element is covered, so the blocks stop waking each other (PRE F4016 at t=0).
/// iv = vl "any=11"; s2v loops. Cell `PP_t`.
///
/// Raw lines: PRE `fatal[VITA-F4016] F-RUN-NO-CONVERGE: did not converge: delta limit (1000000)
/// exceeded at time 0 (zero-delay loop / combinational oscillation) [at time 0]`; iv `any=11`; s2v
/// `(no output)`; vl `any=11`.
#[test]
fn pp_t_row30_pingpong_converges() {
    pin(
        "module top;\n\
  logic [1:0]      any;        // each generate iteration's always_comb writes its own bit\n\
  logic [1:0][1:0] m;          // never assigned before #1: x\n\
  for (genvar k = 0; k < 2; k++) begin : BANKS\n\
    always_comb begin\n\
      any[k] = '0;\n\
      for (int l = 0; l < 2; l++) any[k] |= m[k][l];\n\
    end\n\
  end\n\
  initial begin\n\
    #1 m = 4'b0110;\n\
    #1 $display(\"any=%b\", any);\n\
    $finish;\n\
  end\n\
endmodule\n\
",
        &["any=11"],
    );
}

/// R2.7, row 30: the same shape in `always @*` — the compound's implicit lvalue read is not an
/// identifier of the statement (IEEE 1800-2017 §9.4.2.2 :9590-9597 "All net and variable
/// identifiers that appear in the statement will be automatically added to the event expression
/// with the following exceptions: ... Identifiers that only appear as a
/// hierarchical_variable_identifier in the variable_lvalue of the left-hand side of assignments.";
/// PRE F4016 at t=1). iv = vl "any=11"; s2v loops (it rewrites `|=` as a read). Cell `PP_star`.
///
/// Raw lines: PRE `fatal[VITA-F4016] F-RUN-NO-CONVERGE: did not converge: delta limit (1000000)
/// exceeded at time 1 (zero-delay loop / combinational oscillation) [at time 1]`; iv `any=11`; s2v
/// `(no output)`; vl `any=11`.
#[test]
fn pp_star_row30_pingpong_converges() {
    pin(
        "module top;\n\
  logic [1:0]      any;        // each generate iteration's always_comb writes its own bit\n\
  logic [1:0][1:0] m;          // never assigned before #1: x\n\
  for (genvar k = 0; k < 2; k++) begin : BANKS\n\
    always @* begin\n\
      any[k] = '0;\n\
      for (int l = 0; l < 2; l++) any[k] |= m[k][l];\n\
    end\n\
  end\n\
  initial begin\n\
    #1 m = 4'b0110;\n\
    #1 $display(\"any=%b\", any);\n\
    $finish;\n\
  end\n\
endmodule\n\
",
        &["any=11"],
    );
}

/// R2.6 (C3): the row-30 shape on a packed 2-D vector — the element offset carries elaborate's
/// bounds guard `(k >= 0 && k <= 1) ? k*4 : 'h80000000`, which the exact fold reads as constant
/// (PRE F4016 at t=0). iv = vl "any=3c"; s2v loops. Cell `PP_t_2d`.
///
/// Raw lines: PRE `fatal[VITA-F4016] F-RUN-NO-CONVERGE: did not converge: delta limit (1000000)
/// exceeded at time 0 (zero-delay loop / combinational oscillation) [at time 0]`; iv `any=3c`; s2v
/// `(no output)`; vl `any=3c`.
#[test]
fn pp_t_2d_row30_pingpong_on_a_packed_2d_vector() {
    pin(
        "module top;\n\
  logic [1:0][3:0] any;        // each generate iteration's always_comb writes its own 4-bit element\n\
  logic [1:0][1:0][3:0] m;     // never assigned before #1: x\n\
  for (genvar k = 0; k < 2; k++) begin : BANKS\n\
    always_comb begin\n\
      any[k] = '0;\n\
      for (int l = 0; l < 2; l++) any[k] |= m[k][l];\n\
    end\n\
  end\n\
  initial begin\n\
    #1 m = 16'h1248;\n\
    #1 $display(\"any=%h\", any);\n\
    $finish;\n\
  end\n\
endmodule\n\
",
        &["any=3c"],
    );
}

/// R2.7: two `@*` blocks with `|=` on `v[0]` / `v[1]` — neither reads its own compound lvalue. =
/// iv "v=11 n0=1 n1=1" (PRE F4016; s2v loops; vl DIDNOTCONVERGE). Cell `PP_two_compound_count`.
///
/// Raw lines: PRE `fatal[VITA-F4016] F-RUN-NO-CONVERGE: did not converge: delta limit (1000000)
/// exceeded at time 1 (zero-delay loop / combinational oscillation) [at time 1]`; iv `v=11 n0=1
/// n1=1`; s2v `(no output)`; vl `%Error-DIDNOTCONVERGE: Settle region did not converge after
/// '--converge-limit' of 10000 tries | Aborting...`.
#[test]
fn pp_two_compound_blocks_wake_once_each() {
    pin(
        "module top;\n\
  logic [1:0] v; logic m0, m1; int n0 = 0, n1 = 0;\n\
  always @* begin v[0] = 1'b0; v[0] |= m0; n0 = n0 + 1; end\n\
  always @* begin v[1] = 1'b0; v[1] |= m1; n1 = n1 + 1; end\n\
  initial begin #1 m0 = 1; m1 = 1; #1 $display(\"v=%b n0=%0d n1=%0d\", v, n0, n1); $finish; end\n\
endmodule\n\
",
        &["v=11 n0=1 n1=1"],
    );
}

/// R2.7: `v[0] |= m` in `@*` with another writer of `v[1]`: the block wakes once. = iv (n=1); PRE
/// = s2v n=2. Cell `N24n_star_compound_count`.
///
/// Raw lines: PRE `A end n=2 v=11`; iv `A end n=1 v=11`; s2v `A end n=2 v=11`; vl
/// `%Error-DIDNOTCONVERGE: Settle region did not converge after '--converge-limit' of 10000 tries
/// | Aborting...`.
#[test]
fn n24n_star_compound_lvalue_is_not_read() {
    pin(
        "module top;\n\
  logic [1:0] v; logic m = 0; int n = 0;\n\
  always @* begin v[0] = 1'b0; v[0] |= m; n = n + 1; end\n\
  initial begin #5 v[1] = 1; #1 m = 1; #2 $display(\"A end n=%0d v=%b\", n, v); $finish; end\n\
endmodule\n\
",
        &["A end n=1 v=11"],
    );
}

/// R2.7: `c++` covers `++`/`--` too. = iv (n=1); PRE = s2v n=2. Cell `N24ni_star_incr_count`.
///
/// Raw lines: PRE `A end n=2 c=11`; iv `A end n=1 c=11`; s2v `A end n=2 c=11`; vl
/// `%Error-DIDNOTCONVERGE: Settle region did not converge after '--converge-limit' of 10000 tries
/// | Aborting...`.
#[test]
fn n24ni_star_increment_lvalue_is_not_read() {
    pin(
        "module top;\n\
  int c = 0; logic m = 0; int n = 0;\n\
  always @* begin if (m) c++; n = n + 1; end\n\
  initial begin #5 c = 10; #1 m = 1; #2 $display(\"A end n=%0d c=%0d\", n, c); $finish; end\n\
endmodule\n\
",
        &["A end n=1 c=11"],
    );
}

/// R2.7: value cell `always @* v[0] |= m;` with `v[0] = 0` written at t=5: the write does not
/// re-run the block. = iv "v=00"; PRE = s2v = vl "v=01". Cell `N24nv_star_compound_value`.
///
/// Raw lines: PRE `A end v=01`; iv `A end v=00`; s2v `A end v=01`; vl `A end v=01`.
#[test]
fn n24nv_star_compound_value() {
    pin(
        "module top;\n\
  logic [1:0] v = 0; logic m = 1;\n\
  always @* v[0] |= m;\n\
  initial begin #1 m = 0; #1 m = 1; #3 v[0] = 0; #2 $display(\"A end v=%b\", v); $finish; end\n\
endmodule\n\
",
        &["A end v=00"],
    );
}

/// C8: S01 plus a hierarchical read elsewhere in the block (the recompute runs): the action rule
/// holds through it. IEEE 1800-2017 :9061-9063 "An expression used in an immediate assertion (see
/// 16.3) within the procedure, or in any function called within the procedure, contributes to the
/// implicit sensitivity list of an always_comb as if that expression were used as a condition of
/// an if statement. Expressions used in assertion action blocks do not contribute to the implicit
/// sensitivity list of an always_comb." (example :9064-9068: `A1:assert (a != e) else if
/// (!disable_error) $error("failed");` triggers on b, c, e only). Cell
/// `N19_comb_action_hier_recompute`.
///
/// Raw lines: PRE `C t=5 k=0 | C t=6 k=1 | A end`; iv `C t=5 k=0 | C t=6 k=1 | A end`; s2v `A
/// end`; vl `C t=5 k=0 | C t=5 k=0 | C t=6 k=1 | C t=6 k=1 | A end | C t=8 k=1 | C t=8 k=1`.
#[test]
fn n19_recompute_keeps_the_action_rule() {
    pin(
        "module sub; logic q = 0; endmodule\n\
module top;\n\
  sub u(); logic a = 0, y; int k = 0;\n\
  always_comb begin y = u.q; assert (a == 0) else $display(\"C t=%0t k=%0d\", $time, k); end\n\
  initial begin #5 a = 1; #1 k = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=5 k=0", "A end"],
    );
}

/// C8: C22 plus a hierarchical read (recompute armed): = C22. iv = s2v. Cell
/// `N20_comb_task_hier_recompute`.
///
/// Raw lines: PRE `C t=0 | C t=5 | C t=6 | A end`; iv `C t=0 | C t=5 | A end`; s2v `C t=0 | C t=5
/// | A end`; vl `C t=0 | A end`.
#[test]
fn n20_recompute_keeps_the_task_rule() {
    pin(
        "module sub; logic q = 0; endmodule\n\
module top;\n\
  sub u(); logic a = 0, g = 0, o, y;\n\
  task t(input logic x); o = x & g; endtask\n\
  always_comb begin y = u.q; t(a); $display(\"C t=%0t\", $time); end\n\
  initial begin #5 a = 1; #1 g = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0", "C t=5", "A end"],
    );
}

/// C8: C03 plus a hierarchical read: = C03. iv = s2v. Cell `N21_star_sfn_hier_recompute`.
///
/// Raw lines: PRE `C t=5 | C t=6 | A end`; iv `C t=5 | A end`; s2v `C t=5 | A end`; vl `C t=0 | A
/// end`.
#[test]
fn n21_recompute_keeps_the_star_rule() {
    pin(
        "module sub; logic q = 0; endmodule\n\
module top;\n\
  sub u(); logic a = 0, g = 0, y, w;\n\
  function logic f(); f = g; endfunction\n\
  always @* begin w = u.q; y = a ^ f(); $display(\"C t=%0t\", $time); end\n\
  initial begin #5 a = 1; #1 g = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=5", "A end"],
    );
}

/// C8: PP_t plus an unrelated hierarchical read: = PP_t. iv = vl. Cell
/// `N22_pingpong_t_hier_elsewhere`.
///
/// Raw lines: PRE `fatal[VITA-F4016] F-RUN-NO-CONVERGE: did not converge: delta limit (1000000)
/// exceeded at time 0 (zero-delay loop / combinational oscillation) [at time 0]`; iv `any=11`; s2v
/// `(no output)`; vl `any=11`.
#[test]
fn n22_recompute_keeps_exception_b() {
    pin(
        "module sub; logic q = 0; endmodule\n\
module top;\n\
  sub u(); logic yq;\n\
  logic [1:0]      any;\n\
  logic [1:0][1:0] m;\n\
  for (genvar k = 0; k < 2; k++) begin : BANKS\n\
    always_comb begin\n\
      any[k] = '0;\n\
      for (int l = 0; l < 2; l++) any[k] |= m[k][l];\n\
    end\n\
  end\n\
  always_comb yq = u.q;\n\
  initial begin\n\
    #1 m = 4'b0110;\n\
    #1 $display(\"any=%b\", any);\n\
    $finish;\n\
  end\n\
endmodule\n\
",
        &["any=11"],
    );
}

/// C8: `y = u.f(1'b0)` — the callee is patched after lowering; the recompute (armed by the
/// hierarchical call lane) walks its body's read of `u.g`. iv = vl. Cell
/// `N01e_comb_hier_fn_body_const_arg`.
///
/// Raw lines: PRE `C t=0 y=0 | A end y=0`; iv `C t=0 y=0 | C t=5 y=1 | A end y=1`; s2v `C t=0 y=0
/// | A end y=0`; vl `C t=0 y=0 | C t=5 y=1 | C t=5 y=1 | A end y=1 | C t=8 y=1 | C t=8 y=1`.
#[test]
fn n01e_comb_reads_a_hierarchical_function_body() {
    pin(
        "module sub;\n\
  logic g = 0;\n\
  function automatic logic f(input logic x); return x ^ g; endfunction\n\
  initial #5 g = 1;\n\
endmodule\n\
module top;\n\
  logic y;\n\
  sub u();\n\
  always_comb begin y = u.f(1'b0); $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #8 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0", "C t=5 y=1", "A end y=1"],
    );
}

/// C8: `u.tk(a)` — the task enable's binds are installed after lowering; the recompute (armed by
/// the task-enable lane) reads `a`. iv = s2v = vl. Cell `N31c_comb_hier_task_var_arg_noinit`.
///
/// Raw lines: PRE `T t=0 x=x | A end`; iv `T t=0 x=x | T t=3 x=0 | T t=5 x=1 | A end`; s2v `T t=0
/// x=x | T t=3 x=0 | T t=5 x=1 | A end`; vl `T t=0 x=0 | T t=3 x=0 | T t=3 x=0 | T t=5 x=1 | T t=5
/// x=1 | A end | T t=7 x=1 | T t=7 x=1`.
#[test]
fn n31c_comb_reads_a_hierarchical_task_argument() {
    pin(
        "module sub;\n\
  task automatic tk(input logic x); $display(\"T t=%0t x=%0d\", $time, x); endtask\n\
endmodule\n\
module top;\n\
  logic a;\n\
  sub u();\n\
  always_comb begin u.tk(a); end\n\
  initial begin #3 a = 0; #2 a = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["T t=0 x=x", "T t=3 x=0", "T t=5 x=1", "A end"],
    );
}

/// C8: an in-body `@* y = u.q;` — its `Wait` held the unpatched placeholder (PRE panicked, index
/// 4294967295 on every backend); the in-body sites are recomputed. iv = s2v "W t=5 y=1". Cell
/// `N02_inbody_star_hier_read`.
///
/// Raw lines: PRE `thread 'vita-main' panicked at crates/sim-engine/src/native/run.rs:1644:43: |
/// index out of bounds: the len is 2 but the index is 4294967295 | note: run with
/// `RUST_BACKTRACE=1` environment variable to display a backtrace`; iv `W t=5 y=1 | A end`; s2v `W
/// t=5 y=1 | A end`; vl `(no verilator binary) | %Error-UNSUPPORTED: t.sv:4:25: Unsupported: Event
/// control with implicit sensitivity (@*) in this context; use an explicit sensitivity list
/// instead | %Error: Exiting due to 1 error(s)`.
#[test]
fn n02_inbody_star_reads_a_hierarchical_net() {
    pin(
        "module sub; logic q = 0; initial #5 q = 1; endmodule\n\
module top;\n\
  sub u(); logic y;\n\
  initial forever begin @* y = u.q; $display(\"W t=%0t y=%0d\", $time, y); end\n\
  initial #10 begin $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1", "A end"],
    );
}

/// control (R2.6): a read the block's writes do not cover keeps the net. Cell
/// `C21_comb_written_read_whole`.
///
/// Raw lines: PRE `C t=0 | C t=5 | A end`; iv `C t=0 | C t=5 | A end`; s2v `C t=0 | C t=5 | A
/// end`; vl `C t=0 | A end`.
#[test]
fn c21_control_a_read_not_covered_stays() {
    pin(
        "module top;\n\
  logic a = 0, t, y;\n\
  always_comb begin t = a; y = t; $display(\"C t=%0t\", $time); end\n\
  initial begin #5 a = 1; #3 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0", "C t=5", "A end"],
    );
}

/// control: §9.4.2.2 Example 5 — `y[a] = !en` reads `a` (an index on the left-hand side). Cell
/// `N08_star_lhs_index_ex5`.
///
/// Raw lines: PRE `C t=5 y=1111 | C t=6 y=1101 | A end`; iv `C t=5 y=1111 | C t=6 y=1101 | A end`;
/// s2v `C t=5 y=1111 | C t=6 y=1101 | A end`; vl `C t=0 y=1111 | C t=5 y=1111 | C t=5 y=1111 | C
/// t=6 y=1101 | C t=6 y=1101 | A end | C t=8 y=1101 | C t=8 y=1101`.
#[test]
fn n08_control_star_reads_an_lhs_index() {
    pin(
        "module top;\n\
  logic [3:0] y; logic [1:0] a = 0; logic en = 0;\n\
  always @* begin y = 4'hf; y[a] = !en; $display(\"C t=%0t y=%b\", $time, y); end\n\
  initial begin #5 a = 1; #1 en = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=5 y=1111", "C t=6 y=1101", "A end"],
    );
}

/// control: §9.4.2.2 Example 2 — a temporary the `@*` writes and then reads still wakes it
/// (exception b is `always_comb` only). Cell `N09_star_written_temp_ext`.
///
/// Raw lines: PRE `C t=5 y=0 | A end y=0`; iv `C t=5 y=0 | A end y=0`; s2v `C t=5 y=0 | A end
/// y=0`; vl `C t=0 y=0 | A end y=0`.
#[test]
fn n09_control_star_reads_a_written_temporary() {
    pin(
        "module top;\n\
  logic a = 0, tmp, y;\n\
  always @* begin tmp = a; y = tmp; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 tmp = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=5 y=0", "A end y=0"],
    );
}

/// control: a package function reading a package variable (all three tools agree). Cell
/// `N18b_comb_pkg_fn_import`.
///
/// Raw lines: PRE `C t=0 y=0 | C t=5 y=1 | A end y=1`; iv `C t=0 y=0 | C t=5 y=1 | A end y=1`; s2v
/// `C t=0 y=0 | C t=5 y=1 | A end y=1`; vl `C t=0 y=0 | C t=5 y=1 | C t=5 y=1 | A end y=1 | C t=7
/// y=1 | C t=7 y=1`.
#[test]
fn n18b_control_comb_reads_a_package_function() {
    pin(
        "package p; int pv = 0; function automatic int f(); return pv; endfunction endpackage\n\
module top;\n\
  import p::*;\n\
  int y;\n\
  always_comb begin y = f(); $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 pv = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0", "C t=5 y=1", "A end y=1"],
    );
}

/// control (R2.7): the written-out `v[0] = v[0] | m` reads `v[0]`: all wake twice. Cell
/// `N24ne_star_explicit_count`.
///
/// Raw lines: PRE `A end n=2 v=11`; iv `A end n=2 v=11`; s2v `A end n=2 v=11`; vl
/// `%Error-DIDNOTCONVERGE: Settle region did not converge after '--converge-limit' of 10000 tries
/// | Aborting...`.
#[test]
fn n24ne_control_star_explicit_rhs_read_counts() {
    pin(
        "module top;\n\
  logic [1:0] v; logic m = 0; int n = 0;\n\
  always @* begin v[0] = 1'b0; v[0] = v[0] | m; n = n + 1; end\n\
  initial begin #5 v[1] = 1; #1 m = 1; #2 $display(\"A end n=%0d v=%b\", n, v); $finish; end\n\
endmodule\n\
",
        &["A end n=2 v=11"],
    );
}

/// control (R2.7): explicit twin of N24nv: all "v=01". Cell `N24nve_star_explicit_value`.
///
/// Raw lines: PRE `A end v=01`; iv `A end v=01`; s2v `A end v=01`; vl `A end v=01`.
#[test]
fn n24nve_control_star_explicit_value() {
    pin(
        "module top;\n\
  logic [1:0] v = 0; logic m = 1;\n\
  always @* v[0] = v[0] | m;\n\
  initial begin #1 m = 0; #1 m = 1; #3 v[0] = 0; #2 $display(\"A end v=%b\", v); $finish; end\n\
endmodule\n\
",
        &["A end v=01"],
    );
}

/// control (R2.6): a write inside a task body is the task's content, not the block's: the read
/// stays (all wake). Cell `N26_comb_task_write_not_cover`.
///
/// Raw lines: PRE `C t=0 y=0 | C t=5 y=0 | C t=8 y=1 | A end`; iv `C t=0 y=0 | C t=5 y=0 | C t=8
/// y=1 | A end`; s2v `C t=0 y=0 | C t=5 y=0 | C t=8 y=1 | A end`; vl `C t=0 y=0 | C t=5 y=0 | C
/// t=5 y=0 | C t=8 y=1 | C t=8 y=1 | A end | C t=10 y=1 | C t=10 y=1`.
#[test]
fn n26_control_task_body_write_does_not_cover() {
    pin(
        "module top;\n\
  logic [1:0] v; logic b = 0, y;\n\
  task automatic clr(); v[0] = 0; endtask\n\
  always_comb begin clr(); y = v[0] | b; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 v[1] = 1; #3 b = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0", "C t=5 y=0", "C t=8 y=1", "A end"],
    );
}

/// control (O7, ENG-3's pin): `v[1:0] = a; y = v[2:0];` keeps the whole `v` (PRE = s2v). Cell
/// `N28_O7_partial_cover`.
///
/// Raw lines: PRE `C t=0 y=x00 | C t=5 y=x00 | C t=6 y=100 | A end`; iv `C t=0 y=x00 | A end`; s2v
/// `C t=0 y=x00 | C t=5 y=x00 | C t=6 y=100 | A end`; vl `C t=0 y=000 | A end`.
#[test]
fn n28_control_partially_covered_read_stays_whole() {
    pin(
        "module top;\n\
  logic [3:0] v; logic [1:0] a = 0; logic [2:0] y;\n\
  always_comb begin v[1:0] = a; y = v[2:0]; $display(\"C t=%0t y=%b\", $time, y); end\n\
  initial begin #5 v[3] = 1; #1 v[2] = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=x00", "C t=5 y=x00", "C t=6 y=100", "A end"],
    );
}

/// control: a static task with an output formal in two blocks. Cell
/// `N29_comb_static_task_shared_formal_out`.
///
/// Raw lines: PRE `C t=0 y=1 | C t=5 y=0 | A end y=0`; iv `C t=0 y=1 | C t=5 y=0 | A end y=0`; s2v
/// `C t=0 y=1 | C t=5 y=0 | A end y=0`; vl `C t=0 y=1 | C t=5 y=0 | C t=5 y=0 | A end y=0 | C t=7
/// y=0 | C t=7 y=0`.
#[test]
fn n29_control_static_task_output_formal() {
    pin(
        "module top;\n\
  logic a = 0, y;\n\
  task t(input logic x, output logic o); o = ~x; endtask\n\
  always_comb begin t(a, y); $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 a = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=1", "C t=5 y=0", "A end y=0"],
    );
}

/// control (C3): `always_comb ds[r+1] = ds[r] ^ k;` per genvar `r`: the write's offset `(r+1)*W`
/// folds exactly, so the read `ds[r]` is not covered (the plan's prefix wording dropped it:
/// opentitan-prims digest, this cell "t=6 ds=11101120"). PRE = s2v = vl. Cell
/// `N36d_comb_genvar_chain_packed2d_src`.
///
/// Raw lines: PRE `t=1 ds=11101110 | t=6 ds=21202120`; iv `(no iverilog binary)`; s2v `t=1
/// ds=xxxxxxxx | t=6 ds=21202120`; vl `t=1 ds=11101110 | t=6 ds=21202120`.
#[test]
fn n36d_control_genvar_chain_on_a_packed_2d_vector() {
    pin(
        "module top;\n\
  logic [3:0][7:0] ds; logic [7:0] k = 8'h01, src = 8'h10;\n\
  assign ds[0] = src;\n\
  for (genvar r = 0; r < 3; r++) begin : g\n\
    always_comb ds[r+1] = ds[r] ^ k;\n\
  end\n\
  initial begin #1 $display(\"t=%0t ds=%h\", $time, ds); #4 src = 8'h20; #1 $display(\"t=%0t ds=%h\", $time, ds); $finish; end\n\
endmodule\n\
",
        &["t=1 ds=11101110", "t=6 ds=21202120"],
    );
}

/// control: the r9 concat-port pingpong `t` (unchanged). Cell `PP_concat_t`.
///
/// Raw lines: PRE `st=a`; iv `st=a`; s2v `st=a`; vl `st=a`.
#[test]
fn pp_concat_t_control() {
    pin(
        "module dffs #(parameter W = 2) (input logic clk, input logic rst_l, input logic en, input logic [W-1:0] din, output logic [W-1:0] dout);\n\
  always_ff @(posedge clk or negedge rst_l) if (!rst_l) dout <= '0; else if (en) dout <= din;\n\
endmodule\n\
module top;\n\
  logic clk = 0, rst_l = 1;\n\
  logic [1:0][1:0] st, nxt;    // per-entry state, one element per generate iteration\n\
  logic [1:0]      en;\n\
  logic            go = 0;\n\
  for (genvar i = 0; i < 2; i++) begin : genblock\n\
    always_comb begin\n\
      nxt[i] = 2'd0;\n\
      en[i]  = 1'b0;\n\
      case (st[i])\n\
        2'd0: begin nxt[i] = 2'd1; en[i] = go; end\n\
        2'd1: begin nxt[i] = 2'd2; en[i] = 1'b1; end\n\
        default: begin nxt[i] = st[i]; en[i] = 1'b0; end\n\
      endcase\n\
    end\n\
    dffs #(2) ff (.clk(clk), .rst_l(rst_l), .en(en[i]), .din(nxt[i]), .dout({st[i]}));   // concatenation as the output actual\n\
  end\n\
  initial begin\n\
    #1 rst_l = 0; #1 rst_l = 1;\n\
    #1 go = 1;\n\
    repeat (4) begin #5 clk = 1; #5 clk = 0; end\n\
    $display(\"st=%h\", st);\n\
    $finish;\n\
  end\n\
endmodule\n\
",
        &["st=a"],
    );
}

/// control: N01e plus a hierarchical net read (PRE already armed the recompute). Cell
/// `N01f_comb_hier_fn_body_const_arg_plus_hier_net`.
///
/// Raw lines: PRE `C t=0 y=0 | C t=5 y=1 | A end y=1`; iv `C t=0 y=0 | C t=5 y=1 | A end y=1`; s2v
/// `C t=0 y=0 | A end y=0`; vl `C t=0 y=0 | C t=5 y=1 | C t=5 y=1 | A end y=1 | C t=8 y=1 | C t=8
/// y=1`.
#[test]
fn n01f_control_hier_function_with_a_hier_net_read() {
    pin(
        "module sub;\n\
  logic g = 0, h = 0;\n\
  function automatic logic f(input logic x); return x ^ g; endfunction\n\
  initial #5 g = 1;\n\
endmodule\n\
module top;\n\
  logic y, z;\n\
  sub u();\n\
  always_comb begin y = u.f(1'b0); z = u.h; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #8 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0", "C t=5 y=1", "A end y=1"],
    );
}

/// control: N31c with a local read beside. Cell `N31b_comb_hier_task_const_arg_plus_read`.
///
/// Raw lines: PRE `T t=0 x=1 | T t=5 x=1 | A end`; iv `T t=0 x=1 | T t=5 x=1 | A end`; s2v `T t=0
/// x=1 | T t=5 x=1 | A end`; vl `T t=0 x=1 | A end`.
#[test]
fn n31b_control_hier_task_with_a_local_read() {
    pin(
        "module sub;\n\
  task automatic tk(input logic x); $display(\"T t=%0t x=%0d\", $time, x); endtask\n\
endmodule\n\
module top;\n\
  logic a = 0, y;\n\
  sub u();\n\
  always_comb begin y = a; u.tk(1'b1); end\n\
  initial begin #5 a = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["T t=0 x=1", "T t=5 x=1", "A end"],
    );
}

/// control (O3): ENG-2 changes no `wait`. Cell `R23_wait_recip_negzero`.
///
/// Raw lines: PRE `W t=5 | A end`; iv `W t=8 | A end`; s2v `W t=8 | A end`; vl `W t=5 | A end`.
#[test]
fn r23_control_wait_is_not_touched() {
    pin(
        "module top;\n\
  real r = 0.0;\n\
  initial begin #1; wait ((1.0 / r) < 0.0); $display(\"W t=%0t\", $time); end\n\
  initial begin #5 r = -0.0; #3 r = -1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W t=5", "A end"],
    );
}

/// control (ENG-2 OP5): IEEE 1800-2017 :9052-9055 "References to class objects and method calls of
/// class objects do not add anything to the sensitivity list of an always_comb, except for any
/// contributions from argument expressions passed to these method calls." = iv; vl wakes at 6 (it
/// reads the method body). Cell `P1_class_method`.
///
/// Raw lines: PRE `C t=0 y=0 | C t=5 y=1 | A end y=1`; iv `C t=0 y=0 | C t=5 y=1 | A end y=1`; s2v
/// `(no sv2v->iverilog binary)`; vl `C t=0 y=0 | C t=5 y=1 | C t=6 y=0 | A end y=0`.
#[test]
fn p1_control_comb_does_not_read_a_class_method_body() {
    pin(
        "class C;\n\
  logic g;\n\
  function logic get(input logic a);\n\
    return a ^ g;\n\
  endfunction\n\
endclass\n\
module top;\n\
  C c;\n\
  logic a, y;\n\
  initial begin c = new; c.g = 0; a = 0; end\n\
  always_comb y = c.get(a);\n\
  initial begin\n\
    #5 a = 1;\n\
    #1 c.g = 1;\n\
    #1 $display(\"A end y=%b\", y);\n\
    $finish;\n\
  end\n\
  always @(y) $display(\"C t=%0t y=%b\", $time, y);\n\
endmodule\n\
",
        &["C t=0 y=0", "C t=5 y=1", "A end y=1"],
    );
}

/// control (ENG-2 OP5): an in-body `@(*)` inside an automatic task body (its site is rebased with
/// the frame body). iv = s2v. Cell `P3_frame_task_inbody_star`.
///
/// Raw lines: PRE `W t=0 y=0 | W t=5 y=1 | W t=6 y=0 | A end y=0`; iv `W t=0 y=0 | W t=5 y=1 | W
/// t=6 y=0 | A end y=0`; s2v `W t=0 y=0 | W t=5 y=1 | W t=6 y=0 | A end y=0`; vl `(no verilator
/// binary) | %Error-UNSUPPORTED: t.sv:4:13: Unsupported: Event control with implicit sensitivity
/// (@*) in this context; use an explicit sensitivity list instead | %Error: Exiting due to 1
/// error(s)`.
#[test]
fn p3_control_inbody_star_inside_an_automatic_task() {
    pin(
        "module top;\n\
  logic a, b, y;\n\
  task automatic watch();\n\
    forever @(*) begin\n\
      y = a & b;\n\
      $display(\"W t=%0t y=%b\", $time, y);\n\
    end\n\
  endtask\n\
  initial watch();\n\
  initial begin\n\
    a = 0; b = 1;\n\
    #5 a = 1;\n\
    #1 b = 0;\n\
    #1 $display(\"A end y=%b\", y);\n\
    $finish;\n\
  end\n\
endmodule\n\
",
        &["W t=0 y=0", "W t=5 y=1", "W t=6 y=0", "A end y=0"],
    );
}

/// control (ENG-2 OP5): `always @* y = f(da)` with an input dynamic-array formal aliased to `da`
/// (no copy): `da` appears in the statement (IEEE 1800-2017 §9.4.2.2 :9590-9597 "All net and
/// variable identifiers that appear in the statement will be automatically added to the event
/// expression with the following exceptions: ... Identifiers that only appear as a
/// hierarchical_variable_identifier in the variable_lvalue of the left-hand side of
/// assignments."), so it stays in the set. PRE = vl; iv does not wake. Cell `P2f_dyn_fn_star`.
///
/// Raw lines: PRE `C t=0 y=1 | C t=5 y=7 | A end y=7`; iv `C t=0 y=1 | A end y=1`; s2v `(no
/// sv2v->iverilog binary)`; vl `C t=0 y=1 | C t=5 y=7 | A end y=7`.
#[test]
fn p2f_control_star_reads_an_aliased_dynamic_array_argument() {
    pin(
        "module top;\n\
  int da[];\n\
  int y;\n\
  function int f(input int d[]);\n\
    return d[0];\n\
  endfunction\n\
  always @* y = f(da);\n\
  initial begin\n\
    da = new[1]; da[0] = 1;\n\
    #5 da[0] = 7;\n\
    #1 $display(\"A end y=%0d\", y);\n\
    $finish;\n\
  end\n\
  always @(y) $display(\"C t=%0t y=%0d\", $time, y);\n\
endmodule\n\
",
        &["C t=0 y=1", "C t=5 y=7", "A end y=7"],
    );
}

/// R2.2 (sweep move, §4.5.585 cell): `always_comb begin t(); y = r; end` with a static task
/// reading `a`, `b` — the t=2 write of `b` does not re-run the block, so `y` keeps 2 (PRE re-ran
/// it: "t=3 y=0", and its `_H` twin reported a W4031 violation at time 2). = s2v; iv cannot parse
/// `unique if`; vl reads task bodies. Cell
/// `unique-if-chain/s585__r1__sound__cells__q4t2_comb_task_modvars.sv`.
///
/// Raw lines: PRE `t=1 y=2 | t=3 y=0`; iv `(no iverilog binary)`; s2v `t=1 y=2 | t=3 y=2`; vl `t=1
/// y=2 | %Error: t.sv:6: Verilog $stop | Aborting...`.
#[test]
fn q4t2_comb_static_task_body_read_does_not_wake() {
    pin(
        include_str!("../../testdata/cells/unique-if-chain/s585__r1__sound__cells__q4t2_comb_task_modvars.sv"),
        &["t=1 y=2", "t=3 y=2"],
    );
}

/// R2.2: the `_H` spelling of q4t2 (aux twin). = s2v. Cell
/// `unique-if-chain/s585__r1__sound__cells__q4t2_comb_task_modvars.aux/r1/sound/cells/q4t2_comb_task_modvars_H.sv`.
///
/// Raw lines: PRE `t=1 y=2 | t=3 y=0`; iv `(no iverilog binary)`; s2v `t=1 y=2 | t=3 y=2`; vl `t=1
/// y=2 | %Error: t.sv:6: Verilog $stop | Aborting...`.
#[test]
fn q4t2h_comb_static_task_body_read_does_not_wake() {
    pin(
        include_str!("../../testdata/cells/unique-if-chain/s585__r1__sound__cells__q4t2_comb_task_modvars.aux/r1/sound/cells/q4t2_comb_task_modvars_H.sv"),
        &["t=1 y=2", "t=3 y=2"],
    );
}

/// R2.2 / R2.5 (sweep move): the `always @*` twin — the task body's reads are not identifiers of
/// the statement and `r` is written only inside the task, so the block never runs and `y` stays x.
/// = s2v (PRE "t=1 y=2|t=3 y=0"). Cell
/// `unique-if-chain/s585__r1__sound__cells__q4t3_star_task_modvars.sv`.
///
/// Raw lines: PRE `t=1 y=2 | t=3 y=0`; iv `(no iverilog binary)`; s2v `t=1 y=x | t=3 y=x`; vl `t=1
/// y=2 | %Error: t.sv:6: Verilog $stop | Aborting...`.
#[test]
fn q4t3_star_static_task_body_read_does_not_wake() {
    pin(
        include_str!("../../testdata/cells/unique-if-chain/s585__r1__sound__cells__q4t3_star_task_modvars.sv"),
        &["t=1 y=x", "t=3 y=x"],
    );
}

/// R2.2 / R2.5: the `_H` spelling of q4t3 (aux twin). = s2v. Cell
/// `unique-if-chain/s585__r1__sound__cells__q4t3_star_task_modvars.aux/r1/sound/cells/q4t3_star_task_modvars_H.sv`.
///
/// Raw lines: PRE `t=1 y=2 | t=3 y=0`; iv `(no iverilog binary)`; s2v `t=1 y=x | t=3 y=x`; vl `t=1
/// y=2 | %Error: t.sv:6: Verilog $stop | Aborting...`.
#[test]
fn q4t3h_star_static_task_body_read_does_not_wake() {
    pin(
        include_str!("../../testdata/cells/unique-if-chain/s585__r1__sound__cells__q4t3_star_task_modvars.aux/r1/sound/cells/q4t3_star_task_modvars_H.sv"),
        &["t=1 y=x", "t=3 y=x"],
    );
}

// ---- ENG-2 round 2 ----

/// B1 (ENG-2 r2): an in-body `@(*)` inside an inlined STATIC task body reads its own controlled
/// statement — the task-body exclusion is relative to the read set's root (IEEE 1800-2017 §9.4.2.2
/// :9580-9582 "@* ... adding all nets and variables that are read by the statement (which can be a
/// statement group) of a procedural_timing_control_statement" and :9590-9597 (the identifiers in
/// the statement; "in subroutine calls" = a call's arguments)); r1 excluded the whole body and
/// never woke. PRE = iv = s2v. Cell `Y01_stask_inbody_star_plain`.
///
/// Raw lines: PRE `W t=5 y=1 | A end y=1`; iv `W t=5 y=1 | A end y=1`; s2v `W t=5 y=1 | A end
/// y=1`; vl `(verilator refused)`.
#[test]
fn y01_inbody_star_in_a_static_task_reads_its_statement() {
    pin(
        "module top;\n\
  logic a = 0, y;\n\
  task t(); @* y = a; $display(\"W t=%0t y=%0d\", $time, y); endtask\n\
  initial t();\n\
  initial begin #5 a = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1", "A end y=1"],
    );
}

/// B1 (ENG-2 r2): an in-body `@(*)` inside an inlined STATIC task body reads its own controlled
/// statement — the task-body exclusion is relative to the read set's root (IEEE 1800-2017 §9.4.2.2
/// :9580-9582 "@* ... adding all nets and variables that are read by the statement (which can be a
/// statement group) of a procedural_timing_control_statement" and :9590-9597 (the identifiers in
/// the statement; "in subroutine calls" = a call's arguments)); r1 excluded the whole body and
/// never woke. The statement reads the formal and a module variable; r1 was silent. PRE = iv =
/// s2v. Cell `Y04_stask_inbody_star_arg`.
///
/// Raw lines: PRE `W t=5 y=1 | A end y=1`; iv `W t=5 y=1 | A end y=1`; s2v `W t=5 y=1 | A end
/// y=1`; vl `(verilator refused)`.
#[test]
fn y04_inbody_star_in_a_static_task_reads_a_formal() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y;\n\
  task t(input logic x); @* y = x ^ g; $display(\"W t=%0t y=%0d\", $time, y); endtask\n\
  initial t(a);\n\
  initial begin #5 g = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1", "A end y=1"],
    );
}

/// Staged twin of `y04_inbody_star_in_a_static_task_reads_a_formal`: the site's root mark and
/// range reach the `.vu` / `.velab` flow (vcmp a; vcmp b; velab -L; vrun).
#[test]
fn y04_staged_flow_matches_the_one_shot_run() {
    staged(
        "module top;\n\
  logic a = 0, g = 0, y;\n\
  task t(input logic x); @* y = x ^ g; $display(\"W t=%0t y=%0d\", $time, y); endtask\n\
  initial t(a);\n\
  initial begin #5 g = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1", "A end y=1"],
    );
}

/// B1 (ENG-2 r2): an in-body `@(*)` inside an inlined STATIC task body reads its own controlled
/// statement — the task-body exclusion is relative to the read set's root (IEEE 1800-2017 §9.4.2.2
/// :9580-9582 "@* ... adding all nets and variables that are read by the statement (which can be a
/// statement group) of a procedural_timing_control_statement" and :9590-9597 (the identifiers in
/// the statement; "in subroutine calls" = a call's arguments)); r1 excluded the whole body and
/// never woke. The header `always @*` still leaves the task body out (its set is `a`), the in-body
/// site reads `b`. PRE = iv = s2v. Cell `Y05_star_hdr_calls_stask_with_inbody_star`.
///
/// Raw lines: PRE `W t=7 y=1 | A end y=1`; iv `W t=7 y=1 | A end y=1`; s2v `W t=7 y=1 | A end
/// y=1`; vl `(verilator refused)`.
#[test]
fn y05_inbody_star_in_a_task_called_from_always_star() {
    pin(
        "module top;\n\
  logic a = 0, b = 0, y;\n\
  task t(); @* y = b; $display(\"W t=%0t y=%0d\", $time, y); endtask\n\
  always @* begin if (a) t(); end\n\
  initial begin #5 a = 1; #2 b = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=7 y=1", "A end y=1"],
    );
}

/// B1 (ENG-2 r2): an in-body `@(*)` inside an inlined STATIC task body reads its own controlled
/// statement — the task-body exclusion is relative to the read set's root (IEEE 1800-2017 §9.4.2.2
/// :9580-9582 "@* ... adding all nets and variables that are read by the statement (which can be a
/// statement group) of a procedural_timing_control_statement" and :9590-9597 (the identifiers in
/// the statement; "in subroutine calls" = a call's arguments)); r1 excluded the whole body and
/// never woke. Y04 plus an unrelated hierarchical read (the recompute runs with the site's root).
/// PRE = iv = s2v. Cell `H07_y04_plus_hier`.
///
/// Raw lines: PRE `W t=5 y=1 | A end y=1`; iv `W t=5 y=1 | A end y=1`; s2v `W t=5 y=1 | A end
/// y=1`; vl `(verilator refused)`.
#[test]
fn h07_inbody_star_in_a_static_task_survives_the_recompute() {
    pin(
        "module sub; logic q = 0; endmodule\n\
module top;\n\
  sub u(); logic hz;\n\
  always_comb hz = u.q;\n\
  logic a = 0, g = 0, y;\n\
  task t(input logic x); @* y = x ^ g; $display(\"W t=%0t y=%0d\", $time, y); endtask\n\
  initial t(a);\n\
  initial begin #5 g = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1", "A end y=1"],
    );
}

/// B1 (ENG-2 r2): an in-body `@(*)` inside an inlined STATIC task body reads its own controlled
/// statement — the task-body exclusion is relative to the read set's root (IEEE 1800-2017 §9.4.2.2
/// :9580-9582 "@* ... adding all nets and variables that are read by the statement (which can be a
/// statement group) of a procedural_timing_control_statement" and :9590-9597 (the identifiers in
/// the statement; "in subroutine calls" = a call's arguments)); r1 excluded the whole body and
/// never woke. The statement reads `u.q` (patched after lowering). PRE panicked; = iv = s2v. Cell
/// `I01_stask_inbody_star_hier`.
///
/// Raw lines: PRE `thread 'vita-main' panicked at crates/sim-engine/src/native/run.rs:1644:43: |
/// index out of bounds: the len is 2 but the index is 4294967295 | note: run with
/// `RUST_BACKTRACE=1` environment variable to display a backtrace`; iv `W t=5 y=1 | A end y=1`;
/// s2v `W t=5 y=1 | A end y=1`; vl `(verilator refused)`.
#[test]
fn i01_inbody_star_in_a_static_task_reads_a_hierarchical_net() {
    pin(
        "module sub; logic q = 0; endmodule\n\
module top;\n\
  sub u(); logic y;\n\
  task t(); @* y = u.q; $display(\"W t=%0t y=%0d\", $time, y); endtask\n\
  initial t();\n\
  initial begin #5 u.q = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1", "A end y=1"],
    );
}

/// B1 (ENG-2 r2): an in-body `@(*)` inside an inlined STATIC task body reads its own controlled
/// statement — the task-body exclusion is relative to the read set's root (IEEE 1800-2017 §9.4.2.2
/// :9580-9582 "@* ... adding all nets and variables that are read by the statement (which can be a
/// statement group) of a procedural_timing_control_statement" and :9590-9597 (the identifiers in
/// the statement; "in subroutine calls" = a call's arguments)); r1 excluded the whole body and
/// never woke. PRE = iv = s2v. Cell `S1_inbody_star_in_static_task`.
///
/// Raw lines: PRE `W1 t=5 y=1 | W2 t=10 y=0 | A end`; iv `W1 t=5 y=1 | W2 t=10 y=0 | A end`; s2v
/// `W1 t=5 y=1 | W2 t=10 y=0 | A end`; vl `%Error-UNSUPPORTED: t.sv:3:11: Event control with
/// implicit sensitivity (@*) in this context; use an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn s1_inbody_star_in_a_static_task_called_twice() {
    pin(
        "module top;\n\
  logic a = 0, y;\n\
  task t; @(*) y = a; endtask\n\
  initial begin t(); $display(\"W1 t=%0t y=%0d\", $time, y); t(); $display(\"W2 t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 a = 1; #5 a = 0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W1 t=5 y=1", "W2 t=10 y=0", "A end"],
    );
}

/// B1 (ENG-2 r2): an in-body `@(*)` inside an inlined STATIC task body reads its own controlled
/// statement — the task-body exclusion is relative to the read set's root (IEEE 1800-2017 §9.4.2.2
/// :9580-9582 "@* ... adding all nets and variables that are read by the statement (which can be a
/// statement group) of a procedural_timing_control_statement" and :9590-9597 (the identifiers in
/// the statement; "in subroutine calls" = a call's arguments)); r1 excluded the whole body and
/// never woke. `@(*) y = f(a)`: the argument is read. PRE = iv = s2v. Cell
/// `S1c_inbody_star_in_static_task_calls_fn`.
///
/// Raw lines: PRE `W1 t=5 y=1 | A end`; iv `W1 t=5 y=1 | A end`; s2v `W1 t=5 y=1 | A end`; vl
/// `%Error-UNSUPPORTED: t.sv:4:11: Event control with implicit sensitivity (@*) in this context;
/// use an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn s1c_inbody_star_in_a_static_task_reads_a_function_argument() {
    pin(
        "module top;\n\
  logic a = 0, b = 0, y;\n\
  function logic f(input logic x); return x ^ b; endfunction\n\
  task t; @(*) y = f(a); endtask\n\
  initial begin t(); $display(\"W1 t=%0t y=%0d\", $time, y); t(); $display(\"W2 t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 a = 1; #5 b = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W1 t=5 y=1", "A end"],
    );
}

/// B1 (ENG-2 r2): an in-body `@(*)` inside an inlined STATIC task body reads its own controlled
/// statement — the task-body exclusion is relative to the read set's root (IEEE 1800-2017 §9.4.2.2
/// :9580-9582 "@* ... adding all nets and variables that are read by the statement (which can be a
/// statement group) of a procedural_timing_control_statement" and :9590-9597 (the identifiers in
/// the statement; "in subroutine calls" = a call's arguments)); r1 excluded the whole body and
/// never woke. `@(*) v[0] |= a`: `a` is read, the compound lvalue copy is not. PRE = iv = s2v.
/// Cell `S1d_inbody_star_compound_in_static_task`.
///
/// Raw lines: PRE `W1 t=5 v=01 | A end v=01`; iv `W1 t=5 v=01 | A end v=01`; s2v `W1 t=5 v=01 | A
/// end v=01`; vl `%Error-UNSUPPORTED: t.sv:3:11: Event control with implicit sensitivity (@*) in
/// this context; use an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn s1d_inbody_star_in_a_static_task_compound_assignment() {
    pin(
        "module top;\n\
  logic [1:0] v = 0; logic a = 0;\n\
  task t; @(*) v[0] |= a; endtask\n\
  initial begin t(); $display(\"W1 t=%0t v=%b\", $time, v); end\n\
  initial begin #5 a = 1; #2 $display(\"A end v=%b\", v); $finish; end\n\
endmodule\n\
",
        &["W1 t=5 v=01", "A end v=01"],
    );
}

/// B1 (ENG-2 r2): an in-body `@(*)` inside an inlined STATIC task body reads its own controlled
/// statement — the task-body exclusion is relative to the read set's root (IEEE 1800-2017 §9.4.2.2
/// :9580-9582 "@* ... adding all nets and variables that are read by the statement (which can be a
/// statement group) of a procedural_timing_control_statement" and :9590-9597 (the identifiers in
/// the statement; "in subroutine calls" = a call's arguments)); r1 excluded the whole body and
/// never woke. The statement calls another static task reading `q`: that body is one level deeper
/// than the site and still adds nothing (a root-relative bit mask would read it). PRE woke on `q`
/// at 3; = iv = s2v. Cell `B1n_inbody_star_in_stask_calls_stask`.
///
/// Raw lines: PRE `W t=3 y=0 z=1 | A end y=0 z=1`; iv `W t=5 y=1 z=1 | A end y=1 z=1`; s2v `W t=5
/// y=1 z=1 | A end y=1 z=1`; vl `(verilator refused)`.
#[test]
fn b1n_inbody_star_in_a_static_task_leaves_a_called_task_body_out() {
    pin(
        "module top;\n\
  logic a = 0, q = 0, y, z;\n\
  task t2(); z = q; endtask\n\
  task t(); @* begin t2(); y = a; end $display(\"W t=%0t y=%0d z=%0d\", $time, y, z); endtask\n\
  initial t();\n\
  initial begin #3 q = 1; #2 a = 1; #2 $display(\"A end y=%0d z=%0d\", y, z); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1 z=1", "A end y=1 z=1"],
    );
}

/// B1 (ENG-2 r2): an in-body `@(*)` inside an inlined STATIC task body reads its own controlled
/// statement — the task-body exclusion is relative to the read set's root (IEEE 1800-2017 §9.4.2.2
/// :9580-9582 "@* ... adding all nets and variables that are read by the statement (which can be a
/// statement group) of a procedural_timing_control_statement" and :9590-9597 (the identifiers in
/// the statement; "in subroutine calls" = a call's arguments)); r1 excluded the whole body and
/// never woke. The statement calls a static function reading `q`: its arguments only. PRE woke on
/// `q` at 3; = iv = s2v. Cell `B1v_inbody_star_in_stask_calls_sfn`.
///
/// Raw lines: PRE `W t=3 y=1 | A end y=1`; iv `W t=5 y=0 | A end y=0`; s2v `W t=5 y=0 | A end
/// y=0`; vl `(verilator refused)`.
#[test]
fn b1v_inbody_star_in_a_static_task_leaves_a_static_function_body_out() {
    pin(
        "module top;\n\
  logic a = 0, q = 0, y;\n\
  function logic f(input logic x); f = x ^ q; endfunction\n\
  task t(); @* y = f(a); $display(\"W t=%0t y=%0d\", $time, y); endtask\n\
  initial t();\n\
  initial begin #3 q = 1; #2 a = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=0", "A end y=0"],
    );
}

/// control (B1): the automatic-task twin of Y01 (a frame body, lowered unmarked). PRE = r1 = iv =
/// s2v. Cell `Y03_atask_inbody_star_plain`.
///
/// Raw lines: PRE `W t=5 y=1 | A end y=1`; iv `W t=5 y=1 | A end y=1`; s2v `W t=5 y=1 | A end
/// y=1`; vl `(verilator refused)`.
#[test]
fn y03_control_inbody_star_in_an_automatic_task() {
    pin(
        "module top;\n\
  logic a = 0, y;\n\
  task automatic t(); @* y = a; $display(\"W t=%0t y=%0d\", $time, y); endtask\n\
  initial t();\n\
  initial begin #5 a = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1", "A end y=1"],
    );
}

/// control (B1): the automatic-task twin of S1. PRE = r1 = iv = s2v. Cell
/// `S1b_inbody_star_in_auto_task`.
///
/// Raw lines: PRE `W1 t=5 y=1 | W2 t=10 y=0 | A end`; iv `(not run)`; s2v `(not run)`; vl `(not
/// run)`.
#[test]
fn s1b_control_inbody_star_in_an_automatic_task_called_twice() {
    pin(
        "module top;\n\
  logic a = 0, y;\n\
  task automatic t; @(*) y = a; endtask\n\
  initial begin t(); $display(\"W1 t=%0t y=%0d\", $time, y); t(); $display(\"W2 t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 a = 1; #5 a = 0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W1 t=5 y=1", "W2 t=10 y=0", "A end"],
    );
}

/// B2 (ENG-2 r2): a `ref` / `const ref` actual (desugared to an `inout` copy-out) is not a write
/// of the block unless the callee writes the formal — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied into the subroutine
/// area, rather, a reference to the original argument is passed", :14840-14841 "When the formal
/// argument is declared as a const ref, the subroutine cannot alter the variable". The task only
/// reads its `ref` formal. PRE = vl; iv / s2v refuse `ref`. Cell `T06b_comb_atask_ref_arg`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=0 o=0 | C t=5 o=1 | A end o=1`; iv `(iverilog refused)`; s2v
/// `(sv2v/iverilog refused)`; vl `C t=0 o=0 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C
/// t=7 o=1`.
#[test]
fn t06b_comb_reads_an_automatic_task_ref_actual() {
    pin(
        "module top;\n\
  logic g, o;\n\
  task automatic t(ref logic x); o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=0 o=0", "C t=5 o=1", "A end o=1"],
    );
}

/// B2 (ENG-2 r2): a `ref` / `const ref` actual (desugared to an `inout` copy-out) is not a write
/// of the block unless the callee writes the formal — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied into the subroutine
/// area, rather, a reference to the original argument is passed", :14840-14841 "When the formal
/// argument is declared as a const ref, the subroutine cannot alter the variable". PRE = vl; iv /
/// s2v refuse. Cell `T08_comb_fn_ref_arg`.
///
/// Raw lines: PRE `C t=0 y=0 | C t=0 y=0 | C t=5 y=1 | A end y=1`; iv `(iverilog refused)`; s2v
/// `(sv2v/iverilog refused)`; vl `C t=0 y=0 | C t=5 y=1 | C t=5 y=1 | A end y=1 | C t=7 y=1 | C
/// t=7 y=1`.
#[test]
fn t08_comb_reads_a_function_ref_actual() {
    pin(
        "module top;\n\
  logic g, y;\n\
  function automatic logic f(ref logic x); return x; endfunction\n\
  always_comb begin y = f(g); $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0", "C t=0 y=0", "C t=5 y=1", "A end y=1"],
    );
}

/// B2 (ENG-2 r2): a `ref` / `const ref` actual (desugared to an `inout` copy-out) is not a write
/// of the block unless the callee writes the formal — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied into the subroutine
/// area, rather, a reference to the original argument is passed", :14840-14841 "When the formal
/// argument is declared as a const ref, the subroutine cannot alter the variable". A `const ref`
/// never covers. PRE = vl; iv / s2v refuse. Cell `T09_comb_fn_constref_arg`.
///
/// Raw lines: PRE `C t=0 y=0 | C t=0 y=0 | C t=5 y=1 | A end y=1`; iv `(iverilog refused)`; s2v
/// `(sv2v/iverilog refused)`; vl `C t=0 y=0 | C t=5 y=1 | C t=5 y=1 | A end y=1 | C t=7 y=1 | C
/// t=7 y=1`.
#[test]
fn t09_comb_reads_a_function_const_ref_actual() {
    pin(
        "module top;\n\
  logic g, y;\n\
  function automatic logic f(const ref logic x); return x; endfunction\n\
  always_comb begin y = f(g); $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0", "C t=0 y=0", "C t=5 y=1", "A end y=1"],
    );
}

/// B2 (ENG-2 r2): a `ref` / `const ref` actual (desugared to an `inout` copy-out) is not a write
/// of the block unless the callee writes the formal — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied into the subroutine
/// area, rather, a reference to the original argument is passed", :14840-14841 "When the formal
/// argument is declared as a const ref, the subroutine cannot alter the variable". The `output`
/// actual beside it still covers (OP2). PRE = vl; iv / s2v refuse. Cell `T12_comb_voidfn_ref_arg`.
///
/// Raw lines: PRE `C t=0 y=0 | C t=0 y=0 | C t=5 y=1 | A end y=1`; iv `(iverilog refused)`; s2v
/// `(sv2v/iverilog refused)`; vl `C t=0 y=0 | C t=5 y=1 | C t=5 y=1 | A end y=1 | C t=7 y=1 | C
/// t=7 y=1`.
#[test]
fn t12_comb_reads_a_void_function_ref_actual() {
    pin(
        "module top;\n\
  logic g, y;\n\
  function automatic void get(ref logic x, output logic o); o = x; endfunction\n\
  always_comb begin get(g, y); $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0", "C t=0 y=0", "C t=5 y=1", "A end y=1"],
    );
}

/// B2 (ENG-2 r2): a `ref` / `const ref` actual (desugared to an `inout` copy-out) is not a write
/// of the block unless the callee writes the formal — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied into the subroutine
/// area, rather, a reference to the original argument is passed", :14840-14841 "When the formal
/// argument is declared as a const ref, the subroutine cannot alter the variable". The actual is
/// `v[1]`. PRE; iv / s2v / vl refuse. Cell `T13_comb_atask_ref_elem`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=0 o=0 | C t=5 o=1 | A end o=1`; iv `(iverilog refused)`; s2v
/// `(sv2v/iverilog refused)`; vl `(verilator refused)`.
#[test]
fn t13_comb_reads_a_ref_actual_element() {
    pin(
        "module top;\n\
  logic [1:0] v; logic o;\n\
  task automatic t(ref logic x); o = x; endtask\n\
  always_comb begin t(v[1]); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin v = 0; #5 v[1] = 1; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=0 o=0", "C t=5 o=1", "A end o=1"],
    );
}

/// B2 (ENG-2 r2): a `ref` / `const ref` actual (desugared to an `inout` copy-out) is not a write
/// of the block unless the callee writes the formal — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied into the subroutine
/// area, rather, a reference to the original argument is passed", :14840-14841 "When the formal
/// argument is declared as a const ref, the subroutine cannot alter the variable". PRE = vl; iv /
/// s2v refuse. Cell `T14_comb_atask_ref_plus_input`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=0 o=0 | C t=5 o=1 | A end o=1`; iv `(iverilog refused)`; s2v
/// `(sv2v/iverilog refused)`; vl `C t=0 o=0 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C
/// t=7 o=1`.
#[test]
fn t14_comb_reads_a_ref_actual_beside_an_input() {
    pin(
        "module top;\n\
  logic g, a, o;\n\
  task automatic t(ref logic x, input logic b); o = x ^ b; endtask\n\
  always_comb begin t(g, a); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin g = 0; a = 0; #5 g = 1; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=0 o=0", "C t=5 o=1", "A end o=1"],
    );
}

/// B2 (ENG-2 r2): a `ref` / `const ref` actual (desugared to an `inout` copy-out) is not a write
/// of the block unless the callee writes the formal — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied into the subroutine
/// area, rather, a reference to the original argument is passed", :14840-14841 "When the formal
/// argument is declared as a const ref, the subroutine cannot alter the variable". An
/// unpacked-array `ref` (the framed copy-out unpacks into the elements). PRE = vl; iv / s2v
/// refuse. Cell `T15_comb_atask_ref_arr`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=0 o=0 | C t=5 o=3 | A end o=3`; iv `(iverilog refused)`; s2v
/// `(sv2v/iverilog refused)`; vl `C t=0 o=0 | C t=5 o=3 | C t=5 o=3 | A end o=3 | C t=7 o=3 | C
/// t=7 o=3`.
#[test]
fn t15_comb_reads_a_ref_unpacked_array_actual() {
    pin(
        "module top;\n\
  logic [7:0] m [0:1]; logic [7:0] o;\n\
  task automatic t(ref logic [7:0] x [0:1]); o = x[1]; endtask\n\
  always_comb begin t(m); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin m[0] = 0; m[1] = 0; #5 m[1] = 3; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=0 o=0", "C t=5 o=3", "A end o=3"],
    );
}

/// B2 (ENG-2 r2): a `ref` / `const ref` actual (desugared to an `inout` copy-out) is not a write
/// of the block unless the callee writes the formal — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied into the subroutine
/// area, rather, a reference to the original argument is passed", :14840-14841 "When the formal
/// argument is declared as a const ref, the subroutine cannot alter the variable". The task writes
/// its `ref` formal, so the actual is written within the block: no wake at 5 (g stays 1 after the
/// write). = vl; PRE woke; iv / s2v refuse. Cell `R11_comb_atask_ref_written_const`.
///
/// Raw lines: PRE `C t=0 o=9 | C t=0 o=9 | C t=5 o=9 | A end o=9 g=9`; iv `(iverilog refused)`;
/// s2v `(sv2v/iverilog refused)`; vl `C t=0 o=9 | A end o=9 g=1`.
#[test]
fn r11_comb_ref_actual_the_task_writes_covers() {
    pin(
        "module top;\n\
  logic [3:0] g, o;\n\
  task automatic t(ref logic [3:0] x); x = 4'd9; o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=9", "A end o=9 g=1"],
    );
}

/// B2 (ENG-2 r2): a `ref` / `const ref` actual (desugared to an `inout` copy-out) is not a write
/// of the block unless the callee writes the formal — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied into the subroutine
/// area, rather, a reference to the original argument is passed", :14840-14841 "When the formal
/// argument is declared as a const ref, the subroutine cannot alter the variable". The task passes
/// its `ref` on to another task's `ref` that is only read. PRE = vl; iv / s2v refuse. Cell
/// `R06_comb_atask_ref_passes_to_reader`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=0 o=0 | C t=5 o=1 | A end o=1`; iv `(iverilog refused)`; s2v
/// `(sv2v/iverilog refused)`; vl `C t=0 o=0 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C
/// t=7 o=1`.
#[test]
fn r06_comb_ref_passed_on_to_a_reader_does_not_cover() {
    pin(
        "module top;\n\
  logic [3:0] g, o;\n\
  task automatic r(ref logic [3:0] z, output logic [3:0] q); q = z; endtask\n\
  task automatic t(ref logic [3:0] x); r(x, o); endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=0 o=0", "C t=5 o=1", "A end o=1"],
    );
}

/// B2 (ENG-2 r2): a `ref` / `const ref` actual (desugared to an `inout` copy-out) is not a write
/// of the block unless the callee writes the formal — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied into the subroutine
/// area, rather, a reference to the original argument is passed", :14840-14841 "When the formal
/// argument is declared as a const ref, the subroutine cannot alter the variable". The task passes
/// its `ref` on to a task that writes it: covered. = vl; PRE woke; iv / s2v refuse. Cell
/// `R05_comb_atask_ref_passes_to_writer`.
///
/// Raw lines: PRE `C t=0 o=9 | C t=0 o=9 | C t=5 o=9 | A end o=9 g=9`; iv `(iverilog refused)`;
/// s2v `(sv2v/iverilog refused)`; vl `C t=0 o=9 | A end o=9 g=1`.
#[test]
fn r05_comb_ref_passed_on_to_a_writer_covers() {
    pin(
        "module top;\n\
  logic [3:0] g, o;\n\
  task automatic w(ref logic [3:0] z); z = 4'd9; endtask\n\
  task automatic t(ref logic [3:0] x); w(x); o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=9", "A end o=9 g=1"],
    );
}

/// B2 (ENG-2 r2): a `ref` / `const ref` actual (desugared to an `inout` copy-out) is not a write
/// of the block unless the callee writes the formal — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied into the subroutine
/// area, rather, a reference to the original argument is passed", :14840-14841 "When the formal
/// argument is declared as a const ref, the subroutine cannot alter the variable". PRE = vl; iv /
/// s2v refuse. Cell `R09_comb_atask_constref_arr`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=0 o=0 | C t=5 o=3 | A end o=3`; iv `(iverilog refused)`; s2v
/// `(sv2v/iverilog refused)`; vl `C t=0 o=0 | C t=5 o=3 | C t=5 o=3 | A end o=3 | C t=7 o=3 | C
/// t=7 o=3`.
#[test]
fn r09_comb_reads_a_const_ref_unpacked_array_actual() {
    pin(
        "module top;\n\
  logic [7:0] m [0:1]; logic [7:0] o;\n\
  task automatic t(const ref logic [7:0] x [0:1]); o = x[1]; endtask\n\
  always_comb begin t(m); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin m[0] = 0; m[1] = 0; #5 m[1] = 3; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=0 o=0", "C t=5 o=3", "A end o=3"],
    );
}

/// B2 (ENG-2 r2): a `ref` / `const ref` actual (desugared to an `inout` copy-out) is not a write
/// of the block unless the callee writes the formal — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied into the subroutine
/// area, rather, a reference to the original argument is passed", :14840-14841 "When the formal
/// argument is declared as a const ref, the subroutine cannot alter the variable". ENG-2 r3 R1: a
/// `ref` actual covers what the same write spelled in the block covers — the task writes element 0
/// of its array `ref`, so `m[0]` is covered and the block's read of `m[1]` (through `o = x[1]`)
/// stays: it wakes at 5. = PRE = the local twin `m[0] = 8'd7; o = m[1];` on every vita (= iv = s2v
/// on the twin); vl `C t=0 o=0 m0=7 | A end o=0` (it excludes a written variable whole). Cell
/// `R10_comb_atask_ref_arr_written`.
///
/// Raw lines: PRE `C t=0 o=0 m0=7 | C t=0 o=0 m0=7 | C t=5 o=3 m0=7 | A end o=3`; iv `(iverilog
/// refused)`; s2v `(sv2v/iverilog refused)`; vl `C t=0 o=0 m0=7 | A end o=0`.
#[test]
fn r10_comb_ref_unpacked_array_covers_only_the_element_the_task_writes() {
    pin(
        "module top;\n\
  logic [7:0] m [0:1]; logic [7:0] o;\n\
  task automatic t(ref logic [7:0] x [0:1]); x[0] = 8'd7; o = x[1]; endtask\n\
  always_comb begin t(m); $display(\"C t=%0t o=%0d m0=%0d\", $time, o, m[0]); end\n\
  initial begin m[0] = 0; m[1] = 0; #5 m[1] = 3; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 o=0 m0=7",
            "C t=0 o=0 m0=7",
            "C t=5 o=3 m0=7",
            "A end o=3",
        ],
    );
}

/// B2 (ENG-2 r2): a `ref` / `const ref` actual (desugared to an `inout` copy-out) is not a write
/// of the block unless the callee writes the formal — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied into the subroutine
/// area, rather, a reference to the original argument is passed", :14840-14841 "When the formal
/// argument is declared as a const ref, the subroutine cannot alter the variable". vita accepts
/// `ref` on a static task (an extension: :14778 "It shall be illegal to use argument passing by
/// reference for subroutines with a lifetime of static"; vl accepts it too); this pins the inlined
/// copy-out path. PRE = vl; iv / s2v refuse. Cell `R02_comb_stask_ref_read`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=0 o=0 | C t=5 o=1 | A end o=1`; iv `(iverilog refused)`; s2v
/// `(sv2v/iverilog refused)`; vl `C t=0 o=0 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C
/// t=7 o=1`.
#[test]
fn r02_comb_reads_a_static_task_ref_actual() {
    pin(
        "module top;\n\
  logic g, o;\n\
  task t(ref logic x); o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=0 o=0", "C t=5 o=1", "A end o=1"],
    );
}

/// B2 (ENG-2 r2): a `ref` / `const ref` actual (desugared to an `inout` copy-out) is not a write
/// of the block unless the callee writes the formal — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied into the subroutine
/// area, rather, a reference to the original argument is passed", :14840-14841 "When the formal
/// argument is declared as a const ref, the subroutine cannot alter the variable". vita accepts
/// `ref` on a static task (an extension: :14778 "It shall be illegal to use argument passing by
/// reference for subroutines with a lifetime of static"; vl accepts it too); this pins the inlined
/// copy-out path. = vl; PRE woke; iv / s2v refuse. Cell `R03_comb_stask_ref_written`.
///
/// Raw lines: PRE `C t=0 o=9 | C t=0 o=9 | C t=5 o=9 | A end o=9 g=9`; iv `(iverilog refused)`;
/// s2v `(sv2v/iverilog refused)`; vl `C t=0 o=9 | A end o=9 g=1`.
#[test]
fn r03_comb_static_task_ref_actual_it_writes_covers() {
    pin(
        "module top;\n\
  logic [3:0] g; logic [3:0] o;\n\
  task t(ref logic [3:0] x); x = 4'd9; o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=9", "A end o=9 g=1"],
    );
}

/// B2 (ENG-2 r2): a `ref` / `const ref` actual (desugared to an `inout` copy-out) is not a write
/// of the block unless the callee writes the formal — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied into the subroutine
/// area, rather, a reference to the original argument is passed", :14840-14841 "When the formal
/// argument is declared as a const ref, the subroutine cannot alter the variable". vita accepts
/// `ref` on a static task (an extension: :14778 "It shall be illegal to use argument passing by
/// reference for subroutines with a lifetime of static"; vl accepts it too); this pins the inlined
/// copy-out path. PRE = vl; iv / s2v refuse. Cell `R04_comb_stask_constref_read`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=0 o=0 | C t=5 o=1 | A end o=1`; iv `(iverilog refused)`; s2v
/// `(sv2v/iverilog refused)`; vl `C t=0 o=0 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C
/// t=7 o=1`.
#[test]
fn r04_comb_reads_a_static_task_const_ref_actual() {
    pin(
        "module top;\n\
  logic g, o;\n\
  task t(const ref logic x); o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=0 o=0", "C t=5 o=1", "A end o=1"],
    );
}

/// control (B2): `always @*` records no writes; the `ref` actual is an argument read. PRE = r1 =
/// vl. Cell `T06c_star_atask_ref_arg`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=5 o=1 | A end o=1`; iv `(iverilog refused)`; s2v
/// `(sv2v/iverilog refused)`; vl `C t=0 o=0 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C
/// t=7 o=1`.
#[test]
fn t06c_control_star_reads_a_ref_actual() {
    pin(
        "module top;\n\
  logic g, o;\n\
  task automatic t(ref logic x); o = x; endtask\n\
  always @* begin t(g); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=5 o=1", "A end o=1"],
    );
}

/// control (B2): the `always @*` twin of T08. PRE = r1 = vl. Cell `T08s_star_fn_ref_arg`.
///
/// Raw lines: PRE `C t=0 y=0 | C t=5 y=1 | A end y=1`; iv `(iverilog refused)`; s2v
/// `(sv2v/iverilog refused)`; vl `C t=0 y=0 | C t=5 y=1 | C t=5 y=1 | A end y=1 | C t=7 y=1 | C
/// t=7 y=1`.
#[test]
fn t08s_control_star_reads_a_function_ref_actual() {
    pin(
        "module top;\n\
  logic g, y;\n\
  function automatic logic f(ref logic x); return x; endfunction\n\
  always @* begin y = f(g); $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0", "C t=5 y=1", "A end y=1"],
    );
}

/// control (B2 / OP2): an `inout` actual is copied out, a write of the block: it covers (= iv; s2v
/// prints x, vl refuses). Cell `R07_comb_inout_control`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=0 o=0 | C t=5 o=1 | A end o=1`; iv `C t=0 o=0 | A end o=0`; s2v
/// `C t=0 o=x | A end o=x`; vl `(verilator refused)`.
#[test]
fn r07_control_inout_actual_covers() {
    pin(
        "module top;\n\
  logic g, o;\n\
  task automatic t(inout logic x); o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "A end o=0"],
    );
}

/// B3 (ENG-2 r2): the recompute after hierarchical resolution re-reads only the `@(*)`'s
/// controlled statement (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and variables
/// that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)), not the statements appended after it to the same
/// block; r1 woke on a later statement's read (only with a deferred hierarchical reference in the
/// design). PRE = iv = s2v. Cell `CONT1_inbody_star_cont_same_block_hier`.
///
/// Raw lines: PRE `W t=5 y=1 z=1 | A end`; iv `W t=5 y=1 z=1 | A end`; s2v `W t=5 y=1 z=1 | A
/// end`; vl `%Error-UNSUPPORTED: t.sv:6:5: Event control with implicit sensitivity (@*) in this
/// context; use an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn cont1_inbody_star_recompute_reads_only_its_statement() {
    pin(
        "module sub; logic q = 0; endmodule\n\
module top;\n\
  sub u();\n\
  logic a = 0, b = 0, y, z, w;\n\
  initial begin\n\
    @(*) y = a; z = b;\n\
    $display(\"W t=%0t y=%0d z=%0d\", $time, y, z);\n\
  end\n\
  always @* w = u.q;\n\
  initial begin #3 b = 1; #2 a = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1 z=1", "A end"],
    );
}

/// Staged twin of `cont1_inbody_star_recompute_reads_only_its_statement`: the site's root mark and
/// range reach the `.vu` / `.velab` flow (vcmp a; vcmp b; velab -L; vrun).
#[test]
fn cont1_staged_flow_matches_the_one_shot_run() {
    staged(
        "module sub; logic q = 0; endmodule\n\
module top;\n\
  sub u();\n\
  logic a = 0, b = 0, y, z, w;\n\
  initial begin\n\
    @(*) y = a; z = b;\n\
    $display(\"W t=%0t y=%0d z=%0d\", $time, y, z);\n\
  end\n\
  always @* w = u.q;\n\
  initial begin #3 b = 1; #2 a = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1 z=1", "A end"],
    );
}

/// B3 (ENG-2 r2): the recompute after hierarchical resolution re-reads only the `@(*)`'s
/// controlled statement (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and variables
/// that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)), not the statements appended after it to the same
/// block; r1 woke on a later statement's read (only with a deferred hierarchical reference in the
/// design). PRE = iv = s2v. Cell `CONT2_inbody_star_loop_hier`.
///
/// Raw lines: PRE `W t=5 y=1 z=1 | W t=10 y=0 z=0 | A end`; iv `W t=5 y=1 z=1 | W t=10 y=0 z=0 | A
/// end`; s2v `W t=5 y=1 z=1 | W t=10 y=0 z=0 | A end`; vl `%Error-UNSUPPORTED: t.sv:6:22: Event
/// control with implicit sensitivity (@*) in this context; use an e... | %Error: Exiting due to 1
/// error(s)`.
#[test]
fn cont2_inbody_star_in_a_loop_recompute_reads_only_its_statement() {
    pin(
        "module sub; logic q = 0; endmodule\n\
module top;\n\
  sub u();\n\
  logic a = 0, b = 0, y, z, w;\n\
  initial begin\n\
    repeat (2) begin @(*) y = a; z = b; $display(\"W t=%0t y=%0d z=%0d\", $time, y, z); end\n\
  end\n\
  always @* w = u.q;\n\
  initial begin #3 b = 1; #2 a = 1; #3 b = 0; #2 a = 0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1 z=1", "W t=10 y=0 z=0", "A end"],
    );
}

/// B3 (ENG-2 r2): the recompute after hierarchical resolution re-reads only the `@(*)`'s
/// controlled statement (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and variables
/// that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)), not the statements appended after it to the same
/// block; r1 woke on a later statement's read (only with a deferred hierarchical reference in the
/// design). The site is in a frame task body (function-arena blocks). PRE = iv = s2v. Cell
/// `CONT3_autotask_inbody_star_cont_hier`.
///
/// Raw lines: PRE `W t=5 y=1 z=1 | A end`; iv `W t=5 y=1 z=1 | A end`; s2v `W t=5 y=1 z=1 | A
/// end`; vl `%Error-UNSUPPORTED: t.sv:5:21: Event control with implicit sensitivity (@*) in this
/// context; use an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn cont3_inbody_star_in_an_automatic_task_recompute_reads_only_its_statement() {
    pin(
        "module sub; logic q = 0; endmodule\n\
module top;\n\
  sub u();\n\
  logic a = 0, b = 0, y, z, w;\n\
  task automatic t; @(*) y = a; z = b; endtask\n\
  initial begin t(); $display(\"W t=%0t y=%0d z=%0d\", $time, y, z); end\n\
  always @* w = u.q;\n\
  initial begin #3 b = 1; #2 a = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1 z=1", "A end"],
    );
}

/// control (B3): CONT1 with no hierarchical reference (no recompute). PRE = r1 = iv = s2v. Cell
/// `CONT1n_inbody_star_cont_same_block_nohier`.
///
/// Raw lines: PRE `W t=5 y=1 z=1 | A end`; iv `W t=5 y=1 z=1 | A end`; s2v `W t=5 y=1 z=1 | A
/// end`; vl `%Error-UNSUPPORTED: t.sv:4:5: Event control with implicit sensitivity (@*) in this
/// context; use an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn cont1n_control_inbody_star_without_a_recompute() {
    pin(
        "module top;\n\
  logic a = 0, b = 0, y, z;\n\
  initial begin\n\
    @(*) y = a; z = b;\n\
    $display(\"W t=%0t y=%0d z=%0d\", $time, y, z);\n\
  end\n\
  initial begin #3 b = 1; #2 a = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1 z=1", "A end"],
    );
}

/// B4 (ENG-2 r2): a rebuilt hierarchical indexed select keeps its placeholder's provenance mark,
/// so its read inside a static task body (IEEE 1800-2017 :9056-9057 "Task calls are allowed in an
/// always_comb, but the contents of the tasks do not add anything to the sensitivity list.") / an
/// action block (IEEE 1800-2017 :9062-9063 "Expressions used in assertion action blocks do not
/// contribute to the implicit sensitivity list of an always_comb.") stays out after the recompute.
/// = iv = s2v = vl; PRE = r1 woke at 5. Cell `H2_comb_hier_sel_in_static_task`.
///
/// Raw lines: PRE `C t=0 | C t=5 | A end`; iv `C t=0 | A end`; s2v `C t=0 | A end`; vl `C t=0 | A
/// end`.
#[test]
fn h2_comb_does_not_read_a_hierarchical_select_in_a_static_task() {
    pin(
        "module sub; logic [3:0] mem = 0; endmodule\n\
module top;\n\
  sub u();\n\
  logic a = 0; logic [1:0] k = 0; logic z;\n\
  task t; z = u.mem[k]; endtask\n\
  always_comb begin t(); $display(\"C t=%0t\", $time); end\n\
  initial begin #5 u.mem = 4'hf; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0", "A end"],
    );
}

/// B4 (ENG-2 r2): a rebuilt hierarchical indexed select keeps its placeholder's provenance mark,
/// so its read inside a static task body (IEEE 1800-2017 :9056-9057 "Task calls are allowed in an
/// always_comb, but the contents of the tasks do not add anything to the sensitivity list.") / an
/// action block (IEEE 1800-2017 :9062-9063 "Expressions used in assertion action blocks do not
/// contribute to the implicit sensitivity list of an always_comb.") stays out after the recompute.
/// = s2v = vl; iv wakes (it adds action-block reads: the O2 decision); PRE = r1 woke. Cell
/// `H1_comb_hier_sel_in_action`.
///
/// Raw lines: PRE `C t=0 | C t=5 | A end`; iv `C t=0 | C t=5 | A end`; s2v `C t=0 | A end`; vl `C
/// t=0 | A end`.
#[test]
fn h1_comb_does_not_read_a_hierarchical_select_in_an_action_block() {
    pin(
        "module sub; logic [3:0] mem = 0; endmodule\n\
module top;\n\
  sub u();\n\
  logic a = 0; logic [1:0] k = 0; logic z;\n\
  always_comb begin assert (a == 0) else z = u.mem[k]; $display(\"C t=%0t\", $time); end\n\
  initial begin #5 u.mem = 4'hf; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0", "A end"],
    );
}

/// N-NBA (ENG-2 r2, owner flag): an `always_comb` / `always_latch` reading back a variable it
/// writes with `<=` does not wake on the NBA update — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block" names no assignment kind; POST = iv = the literal reading (main decision, the O2 / OP1
/// class), s2v and vl wake. Cell `Z01_comb_nba_then_read`.
///
/// Raw lines: PRE `C t=0 y=x | C t=0 y=x | C t=0 y=0 | C t=5 y=0 | C t=5 y=1 | A end y=1 v=01`; iv
/// `C t=0 y=x | C t=5 y=0 | A end y=0 v=01`; s2v `C t=0 y=x | C t=0 y=0 | C t=5 y=0 | C t=5 y=1 |
/// A end y=1 v=01`; vl `C t=0 y=0 | C t=5 y=1 | C t=5 y=1 | A end y=1 v=01 | C t=7 y=1 | C t=7
/// y=1`.
#[test]
fn z01_comb_nba_element_read_back_does_not_wake() {
    pin(
        "module top;\n\
  logic [1:0] v; logic a = 0, y;\n\
  always_comb begin v[0] <= a; y = v[0]; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin v[1] = 0; #5 a = 1; #2 $display(\"A end y=%0d v=%b\", y, v); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=x", "C t=5 y=0", "A end y=0 v=01"],
    );
}

/// N-NBA (ENG-2 r2, owner flag): an `always_comb` / `always_latch` reading back a variable it
/// writes with `<=` does not wake on the NBA update — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block" names no assignment kind; POST = iv = the literal reading (main decision, the O2 / OP1
/// class), s2v and vl wake. Cell `Z02_comb_nba_whole_then_read`.
///
/// Raw lines: PRE `C t=0 y=x | C t=0 y=0 | C t=5 y=0 | C t=5 y=1 | A end y=1 v=1`; iv `C t=0 y=x |
/// C t=5 y=0 | A end y=0 v=1`; s2v `C t=0 y=x | C t=0 y=0 | C t=5 y=0 | C t=5 y=1 | A end y=1
/// v=1`; vl `C t=0 y=0 | C t=5 y=1 | C t=5 y=1 | A end y=1 v=1 | C t=7 y=1 | C t=7 y=1`.
#[test]
fn z02_comb_nba_read_back_does_not_wake() {
    pin(
        "module top;\n\
  logic v; logic a = 0, y;\n\
  always_comb begin v <= a; y = v; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 a = 1; #2 $display(\"A end y=%0d v=%b\", y, v); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=x", "C t=5 y=0", "A end y=0 v=1"],
    );
}

/// N-NBA (ENG-2 r2, owner flag): an `always_comb` / `always_latch` reading back a variable it
/// writes with `<=` does not wake on the NBA update — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block" names no assignment kind; POST = iv = the literal reading (main decision, the O2 / OP1
/// class), s2v and vl wake. Cell `Z03_latch_nba_then_read`.
///
/// Raw lines: PRE `L t=0 y=x | L t=0 y=0 | L t=5 y=0 | L t=5 y=1 | A end y=1 v=1`; iv `L t=0 y=x |
/// L t=5 y=0 | A end y=0 v=1`; s2v `L t=0 y=x | L t=0 y=0 | L t=5 y=0 | L t=5 y=1 | A end y=1
/// v=1`; vl `L t=0 y=0 | L t=5 y=1 | L t=5 y=1 | A end y=1 v=1 | L t=7 y=1 | L t=7 y=1`.
#[test]
fn z03_latch_nba_read_back_does_not_wake() {
    pin(
        "module top;\n\
  logic v; logic en = 1, a = 0, y;\n\
  always_latch begin if (en) v <= a; y = v; $display(\"L t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 a = 1; #2 $display(\"A end y=%0d v=%b\", y, v); $finish; end\n\
endmodule\n\
",
        &["L t=0 y=x", "L t=5 y=0", "A end y=0 v=1"],
    );
}

/// N-NBA (ENG-2 r2, owner flag): an `always_comb` / `always_latch` reading back a variable it
/// writes with `<=` does not wake on the NBA update — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block" names no assignment kind; POST = iv = the literal reading (main decision, the O2 / OP1
/// class), s2v and vl wake. Cell `K30cn_comb_nba_whole`.
///
/// Raw lines: PRE `C t=0 y=x | C t=0 y=0 | C t=5 y=0 | C t=5 y=1 | A end y=1`; iv `C t=0 y=x | C
/// t=5 y=0 | A end y=0`; s2v `C t=0 y=x | C t=0 y=0 | C t=5 y=0 | C t=5 y=1 | A end y=1`; vl `C
/// t=0 y=0 | C t=5 y=1 | C t=5 y=1 | A end y=1 | C t=7 y=1 | C t=7 y=1`.
#[test]
fn k30cn_comb_nba_read_back_does_not_wake() {
    pin(
        "module top;\n\
  logic q; logic a = 0, y;\n\
  always_comb begin q <= a; y = q; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 a = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=x", "C t=5 y=0", "A end y=0"],
    );
}

/// N-NBA (ENG-2 r2, owner flag): an `always_comb` / `always_latch` reading back a variable it
/// writes with `<=` does not wake on the NBA update — IEEE 1800-2017 :9047-9050 "... read within
/// the block or within any function called within the block with the following exceptions: ... b)
/// Any expression that is also written within the block or within any function called within the
/// block" names no assignment kind; POST = iv = the literal reading (main decision, the O2 / OP1
/// class), s2v and vl wake. Cell `K30b_latch_nba_constidx`.
///
/// Raw lines: PRE `L t=0 y=0 | L t=5 y=0 | L t=5 y=1 | A end y=1`; iv `L t=0 y=0 | L t=5 y=0 | A
/// end y=0`; s2v `L t=0 y=0 | L t=5 y=0 | L t=5 y=1 | A end y=1`; vl `L t=0 y=0 | L t=5 y=1 | L
/// t=5 y=1 | A end y=1 | L t=7 y=1 | L t=7 y=1`.
#[test]
fn k30b_latch_nba_element_read_back_does_not_wake() {
    pin(
        "module top;\n\
  logic [1:0] v = 0; logic a = 0, y;\n\
  always_latch begin if (a) v[0] <= 1; y = v[0]; $display(\"L t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 a = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["L t=0 y=0", "L t=5 y=0", "A end y=0"],
    );
}

/// M22's killer (exception a for an inlined static void function's locals): the local `s` written
/// at a variable index is the function's own variable, not a read of either caller. = iv = s2v;
/// PRE looped at 5; vl misses the change at 8. Cell `K22_comb_static_voidfn_local_varidx`.
///
/// Raw lines: PRE `A t=0 | A t=5 | A t=5 | A t=5 | ...(500002 lines)`; iv `(not run)`; s2v `(not
/// run)`; vl `(not run)`.
#[test]
fn k22_comb_static_void_function_local_is_its_own() {
    pin(
        "module top;\n\
  logic a = 0, b = 0; logic [1:0] i = 0;\n\
  function void put(input logic x, input logic [1:0] j);\n\
    logic [3:0] s; logic z;\n\
    s[j] = x; z = s[0];\n\
  endfunction\n\
  always_comb begin put(a, i); $display(\"A t=%0t\", $time); end\n\
  always_comb begin put(b, i); end\n\
  initial begin #5 b = 1; #3 a = 1; #2 $display(\"E end\"); $finish; end\n\
endmodule\n\
",
        &["A t=0", "A t=8", "E end"],
    );
}

/// M22's killer, the part-select twin of K22 (`s[1:0]` written, `^s` read). = iv = s2v; PRE
/// looped. Cell `K22p_comb_static_voidfn_local_partial`.
///
/// Raw lines: PRE `A t=0 | A t=5 | A t=5 | A t=5 | ...(500002 lines)`; iv `A t=0 | A t=8 | E end`;
/// s2v `A t=0 | A t=8 | E end`; vl `A t=0 | E end`.
#[test]
fn k22p_comb_static_void_function_partially_written_local_is_its_own() {
    pin(
        "module top;\n\
  logic a = 0, b = 0;\n\
  function void put(input logic x);\n\
    logic [3:0] s; logic z;\n\
    s[1:0] = {x, x}; z = ^s;\n\
  endfunction\n\
  always_comb begin put(a); $display(\"A t=%0t\", $time); end\n\
  always_comb begin put(b); end\n\
  initial begin #5 b = 1; #3 a = 1; #2 $display(\"E end\"); $finish; end\n\
endmodule\n\
",
        &["A t=0", "A t=8", "E end"],
    );
}

/// control, M27's killer: a write inside a STATIC task body is the task's content, not the block's
/// (IEEE 1800-2017 :9056-9057 "Task calls are allowed in an always_comb, but the contents of the
/// tasks do not add anything to the sensitivity list."): the read stays. PRE = r1 = iv = s2v. Cell
/// `K27_comb_static_task_body_write`.
///
/// Raw lines: PRE `C t=0 y=0 | C t=5 y=0 | C t=8 y=1 | A end`; iv `(not run)`; s2v `(not run)`; vl
/// `(not run)`.
#[test]
fn k27_control_static_task_body_write_does_not_cover() {
    pin(
        "module top;\n\
  logic [1:0] v; logic b = 0, y;\n\
  task clr(); v[0] = 0; endtask\n\
  always_comb begin clr(); y = v[0] | b; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 v[1] = 1; #3 b = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0", "C t=5 y=0", "C t=8 y=1", "A end"],
    );
}

/// control, M27's killer: the static-task twin of N26 (N26's automatic task is a frame body M27
/// cannot reach). PRE = r1 = iv = s2v. Cell `N26s_static_task_body_write`.
///
/// Raw lines: PRE `C t=0 y=0 | C t=5 y=0 | C t=8 y=1 | A end`; iv `C t=0 y=0 | C t=5 y=0 | C t=8
/// y=1 | A end`; s2v `C t=0 y=0 | C t=5 y=0 | C t=8 y=1 | A end`; vl `C t=0 y=0 | C t=5 y=0 | C
/// t=5 y=0 | C t=8 y=1 | C t=8 y=1 | A end | C t=10 y=1 | C t=10 y=1`.
#[test]
fn n26s_control_static_task_body_write_does_not_cover() {
    pin(
        "module top;\n\
  logic [1:0] v; logic b = 0, y;\n\
  task clr(); v[0] = 0; endtask\n\
  always_comb begin clr(); y = v[0] | b; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 v[1] = 1; #3 b = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0", "C t=5 y=0", "C t=8 y=1", "A end"],
    );
}

/// control, M30's killer (C3): a write at a variable index covers nothing, so the read of `v[1]`
/// stays. PRE = r1 = s2v = vl; iv drops it (whole-variable exclusion). Cell
/// `K30n_comb_nba_varidx`.
///
/// Raw lines: PRE `C t=0 y=x v=xxxx | C t=0 y=x v=xxx0 | C t=5 y=x v=xxx0 | C t=5 y=0 v=xx00 | C
/// t=6 y=0 v=xx00 | C t=6 y=1 v=xx10 | A end y=1 v=xx10`; iv `C t=0 y=x v=xxxx | C t=5 y=x v=xxx0
/// | C t=6 y=0 v=xx00 | A end y=0 v=xx10`; s2v `C t=0 y=x v=xxxx | C t=0 y=x v=xxx0 | C t=5 y=x
/// v=xxx0 | C t=5 y=0 v=xx00 | C t=6 y=0 v=xx00 | C t=6 y=1 v=xx10 | A end y=1 v=xx10`; vl `C t=0
/// y=0 v=0000 | C t=5 y=0 v=0000 | C t=5 y=0 v=0000 | C t=6 y=1 v=0010 | C t=6 y=1 v=0010 | A end
/// y=1 v=0010 | C t=8 y=1 v=0010 | C t=8 y=1 v=0010`.
#[test]
fn k30n_control_variable_index_write_covers_nothing() {
    pin(
        "module top;\n\
  logic [3:0] v; logic [1:0] i = 0; logic x = 0, y;\n\
  always_comb begin v[i] <= x; y = v[1]; $display(\"C t=%0t y=%0d v=%b\", $time, y, v); end\n\
  initial begin #5 i = 1; #1 x = 1; #2 $display(\"A end y=%0d v=%b\", y, v); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 y=x v=xxxx",
            "C t=0 y=x v=xxx0",
            "C t=5 y=x v=xxx0",
            "C t=5 y=0 v=xx00",
            "C t=6 y=0 v=xx00",
            "C t=6 y=1 v=xx10",
            "A end y=1 v=xx10",
        ],
    );
}

/// control, M30's killer (C3): `ds[r/2 + 1] = ds[r/2] ^ k` per genvar: `/` is not folded, so the
/// write covers nothing. PRE = r1 = vl; iv refuses. Cell `N36k_chain_div_index`.
///
/// Raw lines: PRE `t=1 ds=11101110 | t=6 ds=21202120`; iv `(iverilog refused)`; s2v `t=1
/// ds=xxxxxxxx | t=6 ds=21202120`; vl `t=1 ds=11101110 | t=6 ds=21202120`.
#[test]
fn n36k_control_unfolded_write_index_covers_nothing() {
    pin(
        "module top;\n\
  logic [3:0][7:0] ds; logic [7:0] k = 8'h01, src = 8'h10;\n\
  assign ds[0] = src;\n\
  for (genvar r = 0; r < 6; r += 2) begin : g\n\
    always_comb ds[r/2 + 1] = ds[r/2] ^ k;\n\
  end\n\
  initial begin #1 $display(\"t=%0t ds=%h\", $time, ds); #4 src = 8'h20; #1 $display(\"t=%0t ds=%h\", $time, ds); $finish; end\n\
endmodule\n\
",
        &["t=1 ds=11101110", "t=6 ds=21202120"],
    );
}

/// control, M30's killer (C3): `ds[(2*r+2)/2]`. PRE = r1 = vl = s2v. Cell
/// `N36p_chain_partsel_div`.
///
/// Raw lines: PRE `t=1 ds=11101110 | t=6 ds=21202120`; iv `(iverilog refused)`; s2v `t=1
/// ds=xxxxxxxx | t=6 ds=21202120`; vl `t=1 ds=11101110 | t=6 ds=21202120`.
#[test]
fn n36p_control_unfolded_part_select_write_covers_nothing() {
    pin(
        "module top;\n\
  logic [3:0][7:0] ds; logic [7:0] k = 8'h01, src = 8'h10;\n\
  assign ds[0] = src;\n\
  for (genvar r = 0; r < 3; r++) begin : g\n\
    always_comb ds[(2*r+2)/2] = ds[r] ^ k;\n\
  end\n\
  initial begin #1 $display(\"t=%0t ds=%h\", $time, ds); #4 src = 8'h20; #1 $display(\"t=%0t ds=%h\", $time, ds); $finish; end\n\
endmodule\n\
",
        &["t=1 ds=11101110", "t=6 ds=21202120"],
    );
}

/// control, M30's killer (C3): the chain on `logic [2:-1][7:0] ds`. PRE = r1 = vl = s2v. Cell
/// `N36n_chain_negative_lsb`.
///
/// Raw lines: PRE `t=1 ds=11101110 | t=6 ds=21202120`; iv `(iverilog refused)`; s2v `t=1
/// ds=xxxxxxxx | t=6 ds=21202120`; vl `t=1 ds=11101110 | t=6 ds=21202120`.
#[test]
fn n36n_control_negative_lsb_chain() {
    pin(
        "module top;\n\
  logic [2:-1][7:0] ds; logic [7:0] k = 8'h01, src = 8'h10;\n\
  assign ds[-1] = src;\n\
  for (genvar r = -1; r < 2; r++) begin : g\n\
    always_comb ds[r+1] = ds[r] ^ k;\n\
  end\n\
  initial begin #1 $display(\"t=%0t ds=%h\", $time, ds); #4 src = 8'h20; #1 $display(\"t=%0t ds=%h\", $time, ds); $finish; end\n\
endmodule\n\
",
        &["t=1 ds=11101110", "t=6 ds=21202120"],
    );
}

/// control, MASK0's killer: a package frame function lowered on demand inside an action block is
/// lowered unmarked, so the other `always_comb` calling it still reads its body. PRE = r1 = iv.
/// Cell `MASK5b_pkgfn_first_in_action`.
///
/// Raw lines: PRE `C t=0 y=0 | C t=0 y=0 | C t=5 y=1 | A end y=1`; iv `C t=0 y=0 | C t=0 y=0 | C
/// t=5 y=1 | A end y=1`; s2v `C t=0 y=0 | C t=5 y=1 | A end y=1`; vl `C t=0 y=0 | C t=5 y=1 | C
/// t=5 y=1 | A end y=1 | C t=7 y=1 | C t=7 y=1`.
#[test]
fn mask5b_control_package_function_first_lowered_in_an_action() {
    pin(
        "package p;\n\
  logic g = 0;\n\
  function automatic logic f(input logic x); return x ^ g; endfunction\n\
endpackage\n\
module top;\n\
  import p::*;\n\
  logic a, k, y, z;\n\
  always_comb begin assert (a == a) else z = p::f(k); end\n\
  always_comb begin y = p::f(a); $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin a = 0; k = 0; #5 g = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0", "C t=0 y=0", "C t=5 y=1", "A end y=1"],
    );
}

/// control, MASK0's killer: the same, first lowered inside a static task body. PRE = r1 = iv. Cell
/// `MASK6b_pkgfn_first_in_static_task`.
///
/// Raw lines: PRE `C t=0 y=0 | C t=0 y=0 | C t=5 y=1 | A end y=1`; iv `C t=0 y=0 | C t=0 y=0 | C
/// t=5 y=1 | A end y=1`; s2v `C t=0 y=0 | C t=5 y=1 | A end y=1`; vl `C t=0 y=0 | C t=5 y=1 | C
/// t=5 y=1 | A end y=1 | C t=7 y=1 | C t=7 y=1`.
#[test]
fn mask6b_control_package_function_first_lowered_in_a_static_task() {
    pin(
        "package p;\n\
  logic g = 0;\n\
  function automatic logic f(input logic x); return x ^ g; endfunction\n\
endpackage\n\
module top;\n\
  import p::*;\n\
  logic a, k, y, z;\n\
  task t; z = p::f(k); endtask\n\
  always_comb begin t(); end\n\
  always_comb begin y = p::f(a); $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin a = 0; k = 0; #5 g = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0", "C t=0 y=0", "C t=5 y=1", "A end y=1"],
    );
}

// ---- ENG-2 round 3 ----

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). A hierarchical task's output unpacked-array actual in a static task
/// called from always_comb: the unpack statements are the call's copy-out. r3 = PRE = s2v. Cell
/// `F1_min`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:2: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:7: warning: user task
/// (t) must be automatic to be synthesized in an always_comb process. | t.sv:8: warning: System
/// task ($display) cannot be synthesized in an always_comb process.`; s2v `C t=0 o=x | C t=5 o=7 |
/// A end o=7`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C t=5 o=2 | A end o=2`.
#[test]
fn f1_min_hier_task_output_array_unpack_keeps_the_call_site_mark() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o;\n\
  task t; u.w(m); endtask\n\
  always_comb begin o = m[1]; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "C t=5 o=7", "A end o=7"],
    );
}

/// Staged twin of `f1_min_hier_task_output_array_unpack_keeps_the_call_site_mark`: the settled
/// read set reaches the `.vu` / `.velab` flow (vcmp a; vcmp b; velab -L; vrun).
#[test]
fn f1_min_staged_flow_matches_the_one_shot_run() {
    staged(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o;\n\
  task t; u.w(m); endtask\n\
  always_comb begin o = m[1]; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "C t=5 o=7", "A end o=7"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). The always_latch twin of F1_min. r3 = PRE = s2v. Cell `F1_latch`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:2: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:7: warning: user task
/// (t) must be automatic to be synthesized in an always_latch process. | t.sv:8: warning: System
/// task ($display) cannot be synthesized in an always_latch process.`; s2v `C t=0 o=x | C t=5 o=7
/// | A end o=7`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C t=5 o=2 | A end o=2`.
#[test]
fn f1_latch_hier_task_output_array_unpack_keeps_the_call_site_mark() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o;\n\
  task t; u.w(m); endtask\n\
  always_latch begin o = m[1]; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "C t=5 o=7", "A end o=7"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). F1_min two static task levels deep. r3 = PRE = s2v. Cell `F1_lvl2`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:2: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:10: warning: user task
/// (t) must be automatic to be synthesized in an always_comb process. | t.sv:11: warning: System
/// task ($display) cannot be synthesized in an always_comb process.`; s2v `C t=0 o=x | C t=5 o=7 |
/// A end o=7`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C t=5 o=2 | A end o=2`.
#[test]
fn f1_lvl2_hier_task_output_array_two_task_levels() {
    pin(
        "module sub2;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module sub;\n\
  sub2 v();\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o;\n\
  task t; u.v.w(m); endtask\n\
  always_comb begin o = m[1]; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "C t=5 o=7", "A end o=7"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). F1_min with an inout array formal. r3 = PRE = s2v. Cell `F1_inout`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:2: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:7: error: Array m needs
/// an array index here. | t.sv:7: warning: user task (t) must be automatic to be synthesized in an
/// always_comb process.`; s2v `C t=0 o=x | C t=5 o=7 | A end o=7`; vl `(verilator refused) |
/// %Error: Internal Error: t.sv:7:15: ../V3DfgSynthesize.cpp:358: Non-ReadOnly reference`.
#[test]
fn f1_inout_hier_task_inout_array_unpack_keeps_the_call_site_mark() {
    pin(
        "module sub;\n\
  task automatic w(inout logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o;\n\
  task t; u.w(m); endtask\n\
  always_comb begin o = m[1]; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "C t=5 o=7", "A end o=7"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). F1_min with a static hierarchical callee. r3 = PRE = s2v. Cell
/// `F1_static_callee`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:2: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:7: warning: user task
/// (t) must be automatic to be synthesized in an always_comb process. | t.sv:2: warning: user task
/// (w) must be automatic to be synthesized in an always_comb process.`; s2v `C t=0 o=x | C t=5 o=7
/// | A end o=7`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C t=5 o=2 | A end o=2`.
#[test]
fn f1_static_callee_hier_task_output_array() {
    pin(
        "module sub;\n\
  task w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o;\n\
  task t; u.w(m); endtask\n\
  always_comb begin o = m[1]; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "C t=5 o=7", "A end o=7"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). F1_min through a hierarchical void function. r3 = PRE = s2v. Cell
/// `F1_fncallee`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:2: error:
/// Function top.u.w port a is not an input port. | t.sv:2:      : Function arguments must be input
/// ports. | t.sv:2: sorry: Subroutine ports with unpacked dimensions are not yet supported.`; s2v
/// `C t=0 o=x | C t=5 o=7 | A end o=7`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C t=5
/// o=2 | A end o=2`.
#[test]
fn f1_fncallee_hier_function_output_array() {
    pin(
        "module sub;\n\
  function automatic void w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endfunction\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o;\n\
  task t; u.w(m); endtask\n\
  always_comb begin o = m[1]; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "C t=5 o=7", "A end o=7"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). F1_min with a second value change. r3 = PRE = s2v. Cell `F1_val`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | C t=6 o=3 | A end o=3`; iv `(iverilog refused) |
/// t.sv:2: sorry: Subroutine ports with unpacked dimensions are not yet supported. | t.sv:7:
/// warning: user task (t) must be automatic to be synthesized in an always_comb process. | t.sv:8:
/// warning: System task ($display) cannot be synthesized in an always_comb process.`; s2v `C t=0
/// o=x | C t=5 o=7 | C t=6 o=3 | A end o=3`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C
/// t=6 o=3 | A end o=3 | C t=8 o=3`.
#[test]
fn f1_val_hier_task_output_array_value() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; u.w(m); endtask\n\
  always_comb begin o = m[1] + g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #1 g = 4'd1; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "C t=5 o=7", "C t=6 o=3", "A end o=3"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). lens-diff r2 HT1l. r3 = PRE = s2v. Cell
/// `HT1l_hier_task_out_arr_in_stask_latch`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:5: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:2: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | t.sv:10: warning: user task (t) must be
/// automatic to be synthesized in an always_latch process.`; s2v `C t=0 o=x | C t=5 o=7 | A end
/// o=7`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C t=5 o=2 | A end o=2`.
#[test]
fn ht1l_hier_task_output_array_in_a_static_task_latch() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
  task automatic ws(output logic [3:0] a); a = 4'd2; endtask\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o; logic en = 1;\n\
  task t; u.w(m); endtask\n\
  always_latch begin if (en) begin o = m[1]; t(); $display(\"C t=%0t o=%0d\", $time, o); end end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "C t=5 o=7", "A end o=7"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). lens-diff r2 HT1nest. r3 = PRE = s2v. Cell
/// `HT1nest_hier_task_out_arr_in_nested_stask`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:4: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:2: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | t.sv:10: warning: user task (t) must be
/// automatic to be synthesized in an always_comb process.`; s2v `C t=0 o=x | C t=5 o=7 | A end
/// o=7`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C t=5 o=2 | A end o=2`.
#[test]
fn ht1nest_hier_task_output_array_in_a_nested_static_task() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o;\n\
  task t2; u.w(m); endtask\n\
  task t; t2(); endtask\n\
  always_comb begin o = m[1]; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "C t=5 o=7", "A end o=7"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). A hierarchical task's input unpacked-array actual in a static task
/// called from always_comb: the pack reads are the call's argument, inside the task content. r3 =
/// PRE = s2v = vl. Cell `F2_min`.
///
/// Raw lines: PRE `C t=0 o=0 | A end o=0`; iv `(iverilog refused) | t.sv:3: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | t.sv:8: error: Array m needs an array
/// index here. | t.sv:8: warning: user task (t) must be automatic to be synthesized in an
/// always_comb process.`; s2v `C t=0 o=0 | A end o=0`; vl `C t=0 o=0 | A end o=0`.
#[test]
fn f2_min_hier_task_input_array_pack_keeps_the_call_site_mark() {
    pin(
        "module sub;\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2] = '{4'd0, 4'd0}; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; u.r(m); endtask\n\
  always_comb begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "A end o=0"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering appends its IR under the mark of the node
/// it replaces or of the call site, so the `@*` / in-body `@(*)` set is its local twin's (IEEE
/// 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and variables that are read by the
/// statement (which can be a statement group) of a procedural_timing_control_statement" and
/// :9590-9597 (the identifiers in the statement; "in subroutine calls" = a call's arguments)). The
/// always @* twin of F2_min. r3 = PRE = s2v. Cell `F2_star`.
///
/// Raw lines: PRE `C t=1 o=1 | A end o=1`; iv `(iverilog refused) | t.sv:3: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | t.sv:8: error: Array m needs an array
/// index here. | 2 error(s) during elaboration.`; s2v `C t=1 o=1 | A end o=1`; vl `C t=0 o=0 | C
/// t=1 o=1 | C t=1 o=1 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C t=7 o=1`.
#[test]
fn f2_star_hier_task_input_array_pack_in_always_star() {
    pin(
        "module sub;\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2] = '{4'd0, 4'd0}; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; u.r(m); endtask\n\
  always @* begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=1 o=1", "A end o=1"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering appends its IR under the mark of the node
/// it replaces or of the call site, so the `@*` / in-body `@(*)` set is its local twin's (IEEE
/// 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and variables that are read by the
/// statement (which can be a statement group) of a procedural_timing_control_statement" and
/// :9590-9597 (the identifiers in the statement; "in subroutine calls" = a call's arguments)). F2
/// under an in-body @(*). r3 = PRE = s2v. Cell `F2_inbody`.
///
/// Raw lines: PRE `C t=1 o=1 | A end o=1`; iv `(iverilog refused) | t.sv:3: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | t.sv:8: error: Array m needs an array
/// index here. | 2 error(s) during elaboration.`; s2v `C t=1 o=1 | A end o=1`; vl `(verilator
/// refused) | %Error-UNSUPPORTED: t.sv:9:16: Event control with implicit sensitivity (@*) in this
/// context; use an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn f2_inbody_hier_task_input_array_pack_in_an_inbody_star() {
    pin(
        "module sub;\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2] = '{4'd0, 4'd0}; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; u.r(m); endtask\n\
  always begin @* begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end end\n\
  initial begin #1 g = 4'd1; #4 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=1 o=1", "A end o=1"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering appends its IR under the mark of the node
/// it replaces or of the call site, so the `@*` / in-body `@(*)` set is its local twin's (IEEE
/// 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and variables that are read by the
/// statement (which can be a statement group) of a procedural_timing_control_statement" and
/// :9590-9597 (the identifiers in the statement; "in subroutine calls" = a call's arguments)).
/// F2_star with an inout formal. r3 = PRE = s2v. Cell `F2_inout_star`.
///
/// Raw lines: PRE `C t=1 o=1 | A end o=1`; iv `(iverilog refused) | t.sv:3: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | t.sv:8: error: Array m needs an array
/// index here. | 2 error(s) during elaboration.`; s2v `C t=1 o=1 | A end o=1`; vl `(verilator
/// refused) | %Error: Internal Error: t.sv:8:15: ../V3DfgSynthesize.cpp:358: Non-ReadOnly
/// reference`.
#[test]
fn f2_inout_star_hier_task_inout_array_pack_in_always_star() {
    pin(
        "module sub;\n\
  logic [3:0] d;\n\
  task automatic r(inout logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2] = '{4'd0, 4'd0}; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; u.r(m); endtask\n\
  always @* begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=1 o=1", "A end o=1"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering appends its IR under the mark of the node
/// it replaces or of the call site, so the `@*` / in-body `@(*)` set is its local twin's (IEEE
/// 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and variables that are read by the
/// statement (which can be a statement group) of a procedural_timing_control_statement" and
/// :9590-9597 (the identifiers in the statement; "in subroutine calls" = a call's arguments)).
/// F2_star with the call inside a static task. r3 = PRE = s2v. Cell `F2_star_in_task`.
///
/// Raw lines: PRE `C t=1 o=1 | A end o=1`; iv `(iverilog refused) | t.sv:3: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | t.sv:8: error: Array m needs an array
/// index here. | 2 error(s) during elaboration.`; s2v `C t=1 o=1 | A end o=1`; vl `(verilator
/// refused) | %Error-UNSUPPORTED: t.sv:9:20: Event control with implicit sensitivity (@*) in this
/// context; use an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn f2_star_in_task_hier_task_input_array_pack_in_a_static_task_star() {
    pin(
        "module sub;\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2] = '{4'd0, 4'd0}; logic [3:0] o; logic [3:0] g = 0;\n\
  task t2; u.r(m); endtask\n\
  task ts; forever @* begin o = g; t2(); $display(\"C t=%0t o=%0d\", $time, o); end endtask\n\
  initial ts();\n\
  initial begin #1 g = 4'd1; #4 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=1 o=1", "A end o=1"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering appends its IR under the mark of the node
/// it replaces or of the call site, so the `@*` / in-body `@(*)` set is its local twin's (IEEE
/// 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and variables that are read by the
/// statement (which can be a statement group) of a procedural_timing_control_statement" and
/// :9590-9597 (the identifiers in the statement; "in subroutine calls" = a call's arguments)).
/// F2_star with the call inside an automatic (frame) task: the site mark is keyed in the function
/// arena. r3 = PRE = s2v. Cell `F2_star_in_frame`.
///
/// Raw lines: PRE `C t=1 o=1 | A end o=1`; iv `(iverilog refused) | t.sv:3: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | t.sv:9: error: Array m needs an array
/// index here. | 2 error(s) during elaboration.`; s2v `C t=1 o=1 | A end o=1`; vl `(verilator
/// refused) | %Error-UNSUPPORTED: t.sv:10:22: Event control with implicit sensitivity (@*) in this
/// context; use an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn f2_star_in_frame_hier_task_input_array_pack_in_a_frame_task_star() {
    pin(
        "module sub;\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2] = '{4'd0, 4'd0};\n\
  logic [3:0] o; logic [3:0] g = 0;\n\
  task t2; u.r(m); endtask\n\
  task automatic ts; @(*) begin o = g; t2(); end $display(\"C t=%0t o=%0d\", $time, o); endtask\n\
  initial begin ts(); ts(); end\n\
  initial begin #1 g = 4'd1; #4 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=1 o=1", "A end o=1"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). The always_latch twin of F2_min. r3 = PRE = s2v. Cell `F2_latch`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=1 o=1 | A end o=1`; iv `(iverilog refused) | t.sv:3: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:8: error: Array m needs
/// an array index here. | t.sv:8: warning: user task (t) must be automatic to be synthesized in an
/// always_latch process.`; s2v `C t=0 o=0 | C t=1 o=1 | A end o=1`; vl `C t=0 o=0 | C t=1 o=1 | C
/// t=1 o=1 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C t=7 o=1`.
#[test]
fn f2_latch_hier_task_input_array_pack_in_always_latch() {
    pin(
        "module sub;\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2] = '{4'd0, 4'd0}; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; u.r(m); endtask\n\
  always_latch begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=1 o=1", "A end o=1"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). F2_min two static task levels deep. r3 = PRE = s2v. Cell `F2_lvl2`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=1 o=1 | A end o=1`; iv `(iverilog refused) | t.sv:3: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:11: error: Array m
/// needs an array index here. | t.sv:11: warning: user task (t) must be automatic to be
/// synthesized in an always_comb process.`; s2v `C t=0 o=0 | C t=1 o=1 | A end o=1`; vl `C t=0 o=0
/// | C t=1 o=1 | C t=1 o=1 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C t=7 o=1`.
#[test]
fn f2_lvl2_hier_task_input_array_two_task_levels() {
    pin(
        "module sub2;\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module sub;\n\
  sub2 v();\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2] = '{4'd0, 4'd0}; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; u.v.r(m); endtask\n\
  always_comb begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=1 o=1", "A end o=1"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). F2_min with a static hierarchical callee. r3 = PRE = s2v. Cell
/// `F2_static_callee`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=1 o=1 | A end o=1`; iv `(iverilog refused) | t.sv:3: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:8: error: Array m needs
/// an array index here. | t.sv:8: warning: user task (t) must be automatic to be synthesized in an
/// always_comb process.`; s2v `C t=0 o=0 | C t=1 o=1 | A end o=1`; vl `C t=0 o=0 | C t=1 o=1 | C
/// t=1 o=1 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C t=7 o=1`.
#[test]
fn f2_static_callee_hier_task_input_array() {
    pin(
        "module sub;\n\
  logic [3:0] d;\n\
  task r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2] = '{4'd0, 4'd0}; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; u.r(m); endtask\n\
  always_comb begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=1 o=1", "A end o=1"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). F2_min through a hierarchical void function. r3 = PRE = s2v. Cell
/// `F2_fncallee`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=1 o=1 | A end o=1`; iv `(iverilog refused) | t.sv:3: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:8: error: Array m needs
/// an array index here. | t.sv:8: warning: user task (t) must be automatic to be synthesized in an
/// always_comb process.`; s2v `C t=0 o=0 | C t=1 o=1 | A end o=1`; vl `C t=0 o=0 | C t=1 o=1 | C
/// t=1 o=1 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C t=7 o=1`.
#[test]
fn f2_fncallee_hier_function_input_array() {
    pin(
        "module sub;\n\
  logic [3:0] d;\n\
  function automatic void r(input logic [3:0] a [2]); d = a[0]; endfunction\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2] = '{4'd0, 4'd0}; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; u.r(m); endtask\n\
  always_comb begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=1 o=1", "A end o=1"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). F2 with the call in an assertion action arm. r3 = PRE = s2v. Cell
/// `F2_action_noinit`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=1 o=1 | A end o=1`; iv `(iverilog refused) | t.sv:3: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:8: error: Array m needs
/// an array index here. | t.sv:8: warning: System task ($display) cannot be synthesized in an
/// always_comb process.`; s2v `C t=0 o=0 | C t=1 o=1 | A end o=1`; vl `C t=0 o=0 | C t=1 o=1 | C
/// t=1 o=1 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C t=7 o=1`.
#[test]
fn f2_action_noinit_hier_task_input_array_in_an_action() {
    pin(
        "module sub;\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o; logic [3:0] g = 0;\n\
  always_comb begin o = g; assert (g < 4'd15) else u.r(m); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=1 o=1", "A end o=1"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). F2 printing only the final value. r3 = PRE = s2v. Cell `F2_time`.
///
/// Raw lines: PRE `A end o=1`; iv `(iverilog refused) | t.sv:3: sorry: Subroutine ports with
/// unpacked dimensions are not yet supported. | t.sv:8: error: Array m needs an array index here.
/// | t.sv:8: warning: user task (t) must be automatic to be synthesized in an always_comb
/// process.`; s2v `A end o=1`; vl `A end o=0`.
#[test]
fn f2_time_hier_task_input_array_value_at_the_end() {
    pin(
        "module sub;\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2] = '{4'd0, 4'd0}; logic [7:0] o; logic [3:0] y; logic [3:0] g = 0;\n\
  task t; u.r(m); o = $time; endtask\n\
  always_comb begin y = g; t(); end\n\
  initial begin #1 g = 4'd1; #4 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["A end o=1"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). F2_min with a second value change (PRE was wrong too). r3 = s2v. Cell
/// `F2_val`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=3 o=5 | A end o=5`; iv `(iverilog refused) | t.sv:3: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:8: error: Array m needs
/// an array index here. | t.sv:8: warning: user task (t) must be automatic to be synthesized in an
/// always_comb process.`; s2v `C t=0 o=0 | A end o=0`; vl `C t=0 o=0 | C t=3 o=5 | C t=3 o=5 | C
/// t=5 o=5 | C t=5 o=5 | A end o=5 | C t=7 o=5 | C t=7 o=5`.
#[test]
fn f2_val_hier_task_input_array_value() {
    pin(
        "module sub;\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2] = '{4'd0, 4'd0}; logic [3:0] o, y, q = 0, g = 0;\n\
  task t; u.r(m); o = q; endtask\n\
  always_comb begin y = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #3 q = 4'd5; #2 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "A end o=0"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). A hierarchical fill (`'1`) whose width is decided at resolve time: the
/// re-lowered rhs keeps the old rhs's mark. r3 = iv. Cell `P_fill`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=1 o=1 | C t=5 o=1 | A end o=1`; iv `C t=0 o=0 | C t=1 o=1 | A
/// end o=1`; s2v `(sv2v/iverilog refused) | s2v.v:11: syntax error | s2v.v:11: error: Malformed
/// statement`; vl `C t=0 o=0 | C t=1 o=1 | C t=1 o=1 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7
/// o=1 | C t=7 o=1`.
#[test]
fn p_fill_hier_fill_rhs_relowered_under_its_mark() {
    pin(
        "module sub;\n\
  logic [7:0] x;\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] k = 0; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; u.x = k | '0; endtask\n\
  always_comb begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 k = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=1 o=1", "A end o=1"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering appends its IR under the mark of the node
/// it replaces or of the call site, so the `@*` / in-body `@(*)` set is its local twin's (IEEE
/// 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and variables that are read by the
/// statement (which can be a statement group) of a procedural_timing_control_statement" and
/// :9590-9597 (the identifiers in the statement; "in subroutine calls" = a call's arguments)). The
/// always @* twin of P_fill. r3 = iv. Cell `P_fill_star`.
///
/// Raw lines: PRE `C t=1 o=1 | C t=5 o=1 | A end o=1`; iv `C t=1 o=1 | A end o=1`; s2v
/// `(sv2v/iverilog refused) | s2v.v:10: syntax error | s2v.v:10: error: Malformed statement`; vl
/// `C t=0 o=0 | C t=1 o=1 | C t=1 o=1 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C t=7
/// o=1`.
#[test]
fn p_fill_star_hier_fill_rhs_in_always_star() {
    pin(
        "module sub;\n\
  logic [7:0] x;\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] k = 0; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; u.x = k | '0; endtask\n\
  always @* begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 k = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=1 o=1", "A end o=1"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering appends its IR under the mark of the node
/// it replaces or of the call site, so the `@*` / in-body `@(*)` set is its local twin's (IEEE
/// 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and variables that are read by the
/// statement (which can be a statement group) of a procedural_timing_control_statement" and
/// :9590-9597 (the identifiers in the statement; "in subroutine calls" = a call's arguments)).
/// P_fill under an in-body @(*). r3 = iv. Cell `P_fill_inbody`.
///
/// Raw lines: PRE `C t=1 o=1 | C t=5 o=1 | A end o=1`; iv `C t=1 o=1 | A end o=1`; s2v
/// `(sv2v/iverilog refused) | s2v.v:10: syntax error | s2v.v:10: error: Malformed statement`; vl
/// `(verilator refused) | %Error-UNSUPPORTED: t.sv:8:16: Event control with implicit sensitivity
/// (@*) in this context; use an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn p_fill_inbody_hier_fill_rhs_in_an_inbody_star() {
    pin(
        "module sub;\n\
  logic [7:0] x;\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] k = 0; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; u.x = k | '0; endtask\n\
  always begin @* begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end end\n\
  initial begin #1 g = 4'd1; #4 k = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=1 o=1", "A end o=1"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). P_fill with a value check. r3 = iv. Cell `P_fill_val`.
///
/// Raw lines: PRE `A end o=5 x=7`; iv `A end o=1 x=0`; s2v `(sv2v/iverilog refused) | s2v.v:13:
/// syntax error | s2v.v:13: error: Malformed statement`; vl `A end o=0 x=7`.
#[test]
fn p_fill_val_hier_fill_rhs_value() {
    pin(
        "module sub;\n\
  logic [7:0] x;\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] k = 0; logic [7:0] o; logic [3:0] y; logic [3:0] g = 0;\n\
  task t; u.x = k | '0; o = $time; endtask\n\
  always_comb begin y = g; t(); end\n\
  initial begin #1 g = 4'd1; #4 k = 4'd7; #2 $display(\"A end o=%0d x=%0d\", o, u.x); $finish; end\n\
endmodule\n\
",
        &["A end o=1 x=0"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). P_fill in an assertion action arm (O2: action reads add nothing; iv
/// wakes). r3 = s2v. Cell `P_fill_action`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=1 o=1 | C t=5 o=1 | A end o=1`; iv `C t=0 o=0 | C t=1 o=1 | C
/// t=5 o=1 | A end o=1`; s2v `C t=0 o=0 | C t=1 o=1 | A end o=1`; vl `C t=0 o=0 | C t=1 o=1 | C
/// t=1 o=1 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C t=7 o=1`.
#[test]
fn p_fill_action_hier_fill_rhs_in_an_action() {
    pin(
        "module sub;\n\
  logic [7:0] x;\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] k = 0; logic [3:0] o; logic [3:0] g = 0;\n\
  always_comb begin o = g; assert (g < 4'd15) else u.x = k | '0; $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 k = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=1 o=1", "A end o=1"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). `$readmemh` into an associative array through a hierarchical path: the
/// resolve-time marker keeps the placeholder's mark. Cell `P_readmem_assoc`.
/// The memory file `mem.hex` (`1`, `2`: words before any `@address`, so W4023 and nothing loaded,
/// as in the cell) is written next to `t.sv`.
///
/// Raw lines: PRE `W4023: $readmem('mem.hex'): a word before any `@address`, into an a... | C t=0
/// o=0 | W4023: $readmem('mem.hex'): a word before any `@address`, into an a... | C t=1 o=1 |
/// W4023: $readmem('mem.hex'): a word before any `@address`, into an a... | C t=5 o=1 | A end
/// o=1`; iv `(iverilog refused) | t.sv:2: error: Type names are not valid expressions here. |
/// t.sv:2: internal error: I do not know how to elaborate this expression. | t.sv:2:
/// : Expression is: <type>`; s2v `(sv2v/iverilog refused) | t.sv:2:14: Parse error: unexpected
/// token ']' (Sym_brack_r)`; vl `C t=0 o=0 | C t=1 o=1 | C t=1 o=1 | C t=5 o=1 | C t=5 o=1 | A end
/// o=1 | C t=7 o=1 | C t=7 o=1`.
#[test]
fn p_readmem_assoc_hier_assoc_marker_keeps_its_mark() {
    pin_files(
        "module sub;\n\
  int aa [int];\n\
  initial #5 aa[3] = 9;\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] o; logic [3:0] g = 0;\n\
  task t; $readmemh(\"mem.hex\", u.aa); endtask\n\
  always_comb begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #6 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &[("mem.hex", "1\n2\n")],
        &["C t=0 o=0", "C t=1 o=1", "A end o=1"],
    );
}

/// R-MEMBER (ENG-2 r3): an in-body `@(*)`'s controlled statement is the statements its open block
/// held when the statement's lowering ended plus what a call inside it splices there after
/// lowering, never a position count (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets
/// and variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)). A hierarchical task call inside the @(*) statement
/// splices its unpack statements at the FRONT of the ret block after lowering. r3 = PRE. Cell
/// `INB_SPLICE`.
///
/// Raw lines: PRE `W t=5 y=5 m1=2 | A end y=1`; iv `(iverilog refused) | t.sv:2: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | 1 error(s) during elaboration.`; s2v
/// `(sv2v/iverilog refused) | s2v.v:18: error: 'm' is not a valid l-value for a procedural
/// assignment. | s2v.v:12:      : 'm' is declared here as a wire.`; vl `(verilator refused) |
/// %Error-UNSUPPORTED: t.sv:9:5: Event control with implicit sensitivity (@*) in this context; use
/// an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn inb_splice_front_splice_stays_in_the_controlled_statement() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2];\n\
  logic [3:0] a = 0, b = 0, y;\n\
  initial begin\n\
    @(*) begin u.w(m); y = a; end\n\
    $display(\"W t=%0t y=%0d m1=%0d\", $time, y, m[1]);\n\
    y = b;\n\
  end\n\
  initial begin #3 b = 4'd1; #2 a = 4'd5; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=5 m1=2", "A end y=1"],
    );
}

/// Staged twin of `inb_splice_front_splice_stays_in_the_controlled_statement`: the settled read
/// set reaches the `.vu` / `.velab` flow (vcmp a; vcmp b; velab -L; vrun).
#[test]
fn inb_splice_staged_flow_matches_the_one_shot_run() {
    staged(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2];\n\
  logic [3:0] a = 0, b = 0, y;\n\
  initial begin\n\
    @(*) begin u.w(m); y = a; end\n\
    $display(\"W t=%0t y=%0d m1=%0d\", $time, y, m[1]);\n\
    y = b;\n\
  end\n\
  initial begin #3 b = 4'd1; #2 a = 4'd5; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=5 m1=2", "A end y=1"],
    );
}

/// R-MEMBER (ENG-2 r3): an in-body `@(*)`'s controlled statement is the statements its open block
/// held when the statement's lowering ended plus what a call inside it splices there after
/// lowering, never a position count (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets
/// and variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)). INB_SPLICE inside an automatic task. r3 = PRE. Cell
/// `INB_SPLICE_frame`.
///
/// Raw lines: PRE `W t=5 y=5 m1=2 | A end y=1`; iv `(iverilog refused) | t.sv:2: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | 1 error(s) during elaboration.`; s2v
/// `(sv2v/iverilog refused) | s2v.v:19: error: 'm' is not a valid l-value for a procedural
/// assignment. | s2v.v:12:      : 'm' is declared here as a wire.`; vl `(verilator refused) |
/// %Error-UNSUPPORTED: t.sv:9:5: Event control with implicit sensitivity (@*) in this context; use
/// an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn inb_splice_frame_front_splice_in_a_frame_task() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2];\n\
  logic [3:0] a = 0, b = 0, y;\n\
  task automatic tf;\n\
    @(*) begin u.w(m); y = a; end\n\
    $display(\"W t=%0t y=%0d m1=%0d\", $time, y, m[1]);\n\
    y = b;\n\
  endtask\n\
  initial tf();\n\
  initial begin #3 b = 4'd1; #2 a = 4'd5; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=5 m1=2", "A end y=1"],
    );
}

/// R-SETTLE (ENG-2 r3): every implicit read set is computed once after the last post-lowering
/// builder, for every design — a `$bits(u.x)` width is resolved before any write region is folded
/// from it (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"). `v[$bits(u.x)-1:0] = a` with
/// no other hierarchical reference: the set is built after `$bits(u.x)` resolves. r3 = PRE. Cell
/// `BITS_LATE`.
///
/// Raw lines: PRE `C t=0 y=x | C t=0 y=0 | C t=5 y=9 | A end y=9`; iv `C t=0 y=0 | A end y=0`; s2v
/// `(sv2v/iverilog refused) | s2v.v:14: syntax error | s2v.v:14: Syntax in assignment statement
/// l-value.`; vl `C t=0 y=0 | C t=5 y=9 | C t=5 y=9 | A end y=9 | C t=7 y=9 | C t=7 y=9`.
#[test]
fn bits_late_hier_bits_width_resolved_before_the_set() {
    pin(
        "module sub;\n\
  logic [3:0] x;\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [7:0] v; logic [3:0] a = 0, b = 0; logic [3:0] y;\n\
  always_comb begin v[$bits(u.x)-1:0] = a; y = v[7:4]; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  always_comb v[7:4] = b;\n\
  initial begin #5 b = 4'd9; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=x", "C t=0 y=0", "C t=5 y=9", "A end y=9"],
    );
}

/// Staged twin of `bits_late_hier_bits_width_resolved_before_the_set`: the settled read set
/// reaches the `.vu` / `.velab` flow (vcmp a; vcmp b; velab -L; vrun).
#[test]
fn bits_late_staged_flow_matches_the_one_shot_run() {
    staged(
        "module sub;\n\
  logic [3:0] x;\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [7:0] v; logic [3:0] a = 0, b = 0; logic [3:0] y;\n\
  always_comb begin v[$bits(u.x)-1:0] = a; y = v[7:4]; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  always_comb v[7:4] = b;\n\
  initial begin #5 b = 4'd9; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=x", "C t=0 y=0", "C t=5 y=9", "A end y=9"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). lens-diff r2 HT1. r3 = PRE = s2v. Cell
/// `HT1_hier_task_out_arr_in_stask_comb`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:5: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:2: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | t.sv:10: warning: user task (t) must be
/// automatic to be synthesized in an always_comb process.`; s2v `C t=0 o=x | C t=5 o=7 | A end
/// o=7`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C t=5 o=2 | A end o=2`.
#[test]
fn ht1_hier_task_output_array_in_a_static_task() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
  task automatic ws(output logic [3:0] a); a = 4'd2; endtask\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o;\n\
  task t; u.w(m); endtask\n\
  always_comb begin o = m[1]; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "C t=5 o=7", "A end o=7"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). lens-diff r2 HT2. r3 = PRE = s2v = vl. Cell
/// `HT2_hier_task_in_arr_in_stask_comb`.
///
/// Raw lines: PRE `C t=0 o=0 | A end o=0`; iv `(iverilog refused) | t.sv:5: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | t.sv:2: sorry: Subroutine ports with
/// unpacked dimensions are not yet supported. | t.sv:10: error: Array m needs an array index
/// here.`; s2v `C t=0 o=0 | A end o=0`; vl `C t=0 o=0 | A end o=0`.
#[test]
fn ht2_hier_task_input_array_in_a_static_task() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
  task automatic ws(output logic [3:0] a); a = 4'd2; endtask\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; u.r(m); endtask\n\
  always_comb begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "A end o=0"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). lens-diff r2 HT2a. r3 = PRE = s2v = vl. Cell
/// `HT2a_hier_task_in_arr_in_action_comb`.
///
/// Raw lines: PRE `C t=0 o=0 | A end o=0`; iv `(iverilog refused) | t.sv:5: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | t.sv:2: sorry: Subroutine ports with
/// unpacked dimensions are not yet supported. | t.sv:10: error: Array m needs an array index
/// here.`; s2v `C t=0 o=0 | A end o=0`; vl `C t=0 o=0 | A end o=0`.
#[test]
fn ht2a_hier_task_input_array_in_an_action() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
  task automatic ws(output logic [3:0] a); a = 4'd2; endtask\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o; logic [3:0] g = 0; logic a = 0;\n\
  always_comb begin o = g; assert (a == 1) else u.r(m); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "A end o=0"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). lens-diff r2 HT1rd. r3 = PRE = s2v. Cell
/// `HT1rd_hier_task_out_arr_in_stask_other_reader`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | C t=6 o=3 | A end o=3`; iv `(iverilog refused) |
/// t.sv:4: sorry: Subroutine ports with unpacked dimensions are not yet supported. | t.sv:2:
/// sorry: Subroutine ports with unpacked dimensions are not yet supported. | t.sv:9: warning: user
/// task (t) must be automatic to be synthesized in an always_comb process.`; s2v `C t=0 o=x | C
/// t=5 o=7 | C t=6 o=3 | A end o=3`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C t=6 o=3
/// | A end o=3 | C t=8 o=3`.
#[test]
fn ht1rd_hier_task_output_array_with_another_reader() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; u.w(m); endtask\n\
  always_comb begin o = m[1] + g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #1 g = 4'd1; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "C t=5 o=7", "C t=6 o=3", "A end o=3"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). lens-diff r2 HT2l. r3 = PRE = s2v. Cell
/// `HT2l_hier_task_in_arr_in_stask_latch`.
///
/// Raw lines: PRE `C t=0 o=0 | A end o=0`; iv `(iverilog refused) | t.sv:4: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | t.sv:2: sorry: Subroutine ports with
/// unpacked dimensions are not yet supported. | t.sv:9: error: Array m needs an array index
/// here.`; s2v `C t=0 o=0 | A end o=0`; vl `C t=0 o=0 | C t=5 o=0 | C t=5 o=0 | A end o=0 | C t=7
/// o=0 | C t=7 o=0`.
#[test]
fn ht2l_hier_task_input_array_in_a_static_task_latch() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o; logic [3:0] g = 0; logic en = 1;\n\
  task t; u.r(m); endtask\n\
  always_latch begin if (en) begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "A end o=0"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering appends its IR under the mark of the node
/// it replaces or of the call site, so the `@*` / in-body `@(*)` set is its local twin's (IEEE
/// 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and variables that are read by the
/// statement (which can be a statement group) of a procedural_timing_control_statement" and
/// :9590-9597 (the identifiers in the statement; "in subroutine calls" = a call's arguments)).
/// lens-diff r2 HT2s. r3 = PRE = s2v. Cell `HT2s_hier_task_in_arr_in_stask_star`.
///
/// Raw lines: PRE `A end o=x`; iv `(iverilog refused) | t.sv:4: sorry: Subroutine ports with
/// unpacked dimensions are not yet supported. | t.sv:2: sorry: Subroutine ports with unpacked
/// dimensions are not yet supported. | t.sv:9: error: Array m needs an array index here.`; s2v `A
/// end o=x`; vl `C t=0 o=0 | A end o=0`.
#[test]
fn ht2s_hier_task_input_array_in_a_static_task_star() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; u.r(m); endtask\n\
  always @* begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["A end o=x"],
    );
}

/// R-MEMBER (ENG-2 r3): an in-body `@(*)`'s controlled statement is the statements its open block
/// held when the statement's lowering ended plus what a call inside it splices there after
/// lowering, never a position count (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets
/// and variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)). lens-sound r2 S1 (X1). r3 = PRE. Cell `X1`.
///
/// Raw lines: PRE `W t=5 y=1 m0=1 | A end y=1`; iv `t.sv:2: sorry: Subroutine ports with unpacked
/// dimensions are not yet supported. | 1 error(s) during elaboration. | (iverilog refused)`; s2v
/// `s2v.v:18: error: 'm' is not a valid l-value for a procedural assignment. | s2v.v:12:      :
/// 'm' is declared here as a wire. | s2v.v:18: error: I give up on task port 1 expression: m | 1
/// error(s) during elaboration. | (sv2v/iverilog refused)`; vl `%Error-UNSUPPORTED: t.sv:9:5:
/// Event control with implicit sensitivity (@*) in this context; use an e... |
/// ... For error description see https://verilator.org/warn/UNSUPPORTED?v=5.052 | %Error: Exiting
/// due to 1 error(s) | (verilator refused)`.
#[test]
fn x1_inbody_star_front_splice_from_a_hier_task() {
    pin(
        "module child;\n\
  task automatic tk(output logic [3:0] o [0:1]); o[0] = 4'd1; o[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  child u();\n\
  logic [3:0] m [0:1]; logic [3:0] a, y;\n\
  initial begin\n\
    a = 0;\n\
    @(*) begin u.tk(m); y = a; end\n\
    $display(\"W t=%0t y=%0d m0=%0d\", $time, y, m[0]);\n\
  end\n\
  initial begin #5 a = 1; #5 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1 m0=1", "A end y=1"],
    );
}

/// R-MEMBER (ENG-2 r3): an in-body `@(*)`'s controlled statement is the statements its open block
/// held when the statement's lowering ended plus what a call inside it splices there after
/// lowering, never a position count (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets
/// and variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)). lens-sound r2 S1 (X1b). r3 = PRE. Cell `X1b`.
///
/// Raw lines: PRE `W t=5 y=0 z=0 w=1 | A end y=0 w=1`; iv `t.sv:2: sorry: Subroutine ports with
/// unpacked dimensions are not yet supported. | 1 error(s) during elaboration. | (iverilog
/// refused)`; s2v `s2v.v:25: error: 'm' is not a valid l-value for a procedural assignment. |
/// s2v.v:13:      : 'm' is declared here as a wire. | s2v.v:25: error: I give up on task port 1
/// expression: m | 1 error(s) during elaboration. | (sv2v/iverilog refused)`; vl
/// `%Error-UNSUPPORTED: t.sv:9:5: Event control with implicit sensitivity (@*) in this context;
/// use an e... |                     ... For error description see
/// https://verilator.org/warn/UNSUPPORTED?v=5.052 | %Error: Exiting due to 1 error(s) | (verilator
/// refused)`.
#[test]
fn x1b_inbody_star_front_splice_two_outputs() {
    pin(
        "module child;\n\
  task automatic tk(output logic [3:0] o [0:2]); o[0] = 4'd1; o[1] = 4'd2; o[2] = 4'd3; endtask\n\
endmodule\n\
module top;\n\
  child u();\n\
  logic [3:0] m [0:2]; logic [3:0] a, b, c, y, z, w;\n\
  initial begin\n\
    a = 0; b = 0; c = 0;\n\
    @(*) begin u.tk(m); y = a; z = b; w = c; end\n\
    $display(\"W t=%0t y=%0d z=%0d w=%0d\", $time, y, z, w);\n\
  end\n\
  initial begin #5 c = 1; #5 a = 1; #5 $display(\"A end y=%0d w=%0d\", y, w); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=0 z=0 w=1", "A end y=0 w=1"],
    );
}

/// R-MEMBER (ENG-2 r3): an in-body `@(*)`'s controlled statement is the statements its open block
/// held when the statement's lowering ended plus what a call inside it splices there after
/// lowering, never a position count (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets
/// and variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)). lens-sound r2 S1 (X1i). r3 = PRE. Cell `X1i`.
///
/// Raw lines: PRE `W t=5 y=1 m0=1 | A end y=1`; iv `t.sv:2: sorry: Subroutine ports with unpacked
/// dimensions are not yet supported. | t.sv:9: error: Array m needs an array index here. | 2
/// error(s) during elaboration. | (iverilog refused)`; s2v `W t=5 y=1 m0=x | A end y=1`; vl
/// `%Error-UNSUPPORTED: t.sv:9:5: Event control with implicit sensitivity (@*) in this context;
/// use an e... |                     ... For error description see
/// https://verilator.org/warn/UNSUPPORTED?v=5.052 | %Error: Exiting due to 1 error(s) | (verilator
/// refused)`.
#[test]
fn x1i_inbody_star_front_splice_inout() {
    pin(
        "module child;\n\
  task automatic tk(inout logic [3:0] o [0:1]); o[0] = o[0] + 4'd1; endtask\n\
endmodule\n\
module top;\n\
  child u();\n\
  logic [3:0] m [0:1]; logic [3:0] a, y;\n\
  initial begin\n\
    a = 0; m[0] = 0; m[1] = 0;\n\
    @(*) begin u.tk(m); y = a; end\n\
    $display(\"W t=%0t y=%0d m0=%0d\", $time, y, m[0]);\n\
  end\n\
  initial begin #5 a = 1; #5 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1 m0=1", "A end y=1"],
    );
}

/// R-MEMBER (ENG-2 r3): an in-body `@(*)`'s controlled statement is the statements its open block
/// held when the statement's lowering ended plus what a call inside it splices there after
/// lowering, never a position count (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets
/// and variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)). lens-sound r2 S1 (X1f). r3 = PRE. Cell `X1f`.
///
/// Raw lines: PRE `W t=5 y=1 m0=1 | A end y=1`; iv `t.sv:2: sorry: Subroutine ports with unpacked
/// dimensions are not yet supported. | 1 error(s) during elaboration. | (iverilog refused)`; s2v
/// `s2v.v:18: error: 'm' is not a valid l-value for a procedural assignment. | s2v.v:12:      :
/// 'm' is declared here as a wire. | s2v.v:18: error: I give up on task port 1 expression: m | 1
/// error(s) during elaboration. | (sv2v/iverilog refused)`; vl `%Error-UNSUPPORTED: t.sv:8:5:
/// Event control with implicit sensitivity (@*) in this context; use an e... |
/// ... For error description see https://verilator.org/warn/UNSUPPORTED?v=5.052 | %Error: Exiting
/// due to 1 error(s) | (verilator refused)`.
#[test]
fn x1f_inbody_star_front_splice_function() {
    pin(
        "module child;\n\
  task automatic tk(output logic [3:0] o [0:1]); o[0] = 4'd1; o[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  child u();\n\
  logic [3:0] m [0:1]; logic [3:0] a, y;\n\
  task automatic w;\n\
    @(*) begin u.tk(m); y = a; end\n\
    $display(\"W t=%0t y=%0d m0=%0d\", $time, y, m[0]);\n\
  endtask\n\
  initial begin a = 0; w(); end\n\
  initial begin #5 a = 1; #5 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1 m0=1", "A end y=1"],
    );
}

/// R-MEMBER (ENG-2 r3): an in-body `@(*)`'s controlled statement is the statements its open block
/// held when the statement's lowering ended plus what a call inside it splices there after
/// lowering, never a position count (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets
/// and variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)). lens-sound r2 S1 (X1t). r3 = PRE. Cell `X1t`.
///
/// Raw lines: PRE `W t=5 y=1 m0=1 | A end y=1`; iv `t.sv:2: sorry: Subroutine ports with unpacked
/// dimensions are not yet supported. | 1 error(s) during elaboration. | (iverilog refused)`; s2v
/// `s2v.v:18: error: 'm' is not a valid l-value for a procedural assignment. | s2v.v:12:      :
/// 'm' is declared here as a wire. | s2v.v:18: error: I give up on task port 1 expression: m | 1
/// error(s) during elaboration. | (sv2v/iverilog refused)`; vl `%Error-UNSUPPORTED: t.sv:8:5:
/// Event control with implicit sensitivity (@*) in this context; use an e... |
/// ... For error description see https://verilator.org/warn/UNSUPPORTED?v=5.052 | %Error: Exiting
/// due to 1 error(s) | (verilator refused)`.
#[test]
fn x1t_inbody_star_front_splice_in_a_static_task() {
    pin(
        "module child;\n\
  task automatic tk(output logic [3:0] o [0:1]); o[0] = 4'd1; o[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  child u();\n\
  logic [3:0] m [0:1]; logic [3:0] a, y;\n\
  task w;\n\
    @(*) begin u.tk(m); y = a; end\n\
    $display(\"W t=%0t y=%0d m0=%0d\", $time, y, m[0]);\n\
  endtask\n\
  initial begin a = 0; w(); end\n\
  initial begin #5 a = 1; #5 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1 m0=1", "A end y=1"],
    );
}

/// R-MEMBER (ENG-2 r3): an in-body `@(*)`'s controlled statement is the statements its open block
/// held when the statement's lowering ended plus what a call inside it splices there after
/// lowering, never a position count (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets
/// and variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)). lens-sound r2 S1 (X1l). r3 = PRE. Cell `X1l`.
///
/// Raw lines: PRE `W t=0 y=0 | W t=5 y=1 | W t=10 y=2 | A end y=2`; iv `t.sv:2: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | 1 error(s) during elaboration. |
/// (iverilog refused)`; s2v `s2v.v:17: error: 'm' is not a valid l-value for a procedural
/// assignment. | s2v.v:12:      : 'm' is declared here as a wire. | s2v.v:17: error: I give up on
/// task port 1 expression: m | 1 error(s) during elaboration. | (sv2v/iverilog refused)`; vl
/// `%Error-UNSUPPORTED: t.sv:8:5: Event control with implicit sensitivity (@*) in this context;
/// use an e... |                     ... For error description see
/// https://verilator.org/warn/UNSUPPORTED?v=5.052 | %Error: Exiting due to 1 error(s) | (verilator
/// refused)`.
#[test]
fn x1l_inbody_star_front_splice_in_a_loop() {
    pin(
        "module child;\n\
  task automatic tk(output logic [3:0] o [0:1]); o[0] = 4'd1; o[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  child u();\n\
  logic [3:0] m [0:1]; logic [3:0] a, y;\n\
  always begin\n\
    @(*) begin u.tk(m); y = a; end\n\
    $display(\"W t=%0t y=%0d\", $time, y);\n\
  end\n\
  initial begin a = 0; #5 a = 1; #5 a = 2; #5 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=0 y=0", "W t=5 y=1", "W t=10 y=2", "A end y=2"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). lens-sound r2 S2 (X3v). r3 = PRE = s2v. Cell `X3v`.
///
/// Raw lines: PRE `C t=0 y=x | C t=5 y=7 | A end y=7`; iv `t.sv:2: sorry: Subroutine ports with
/// unpacked dimensions are not yet supported. | t.sv:7: warning: user task (t) must be automatic
/// to be synthesized in an always_comb process. | t.sv:8: warning: System task ($display) cannot
/// be synthesized in an always_comb process. | 1 error(s) during elaboration. | (iverilog
/// refused)`; s2v `C t=0 y=x | C t=5 y=7 | A end y=7`; vl `C t=0 y=0 | C t=0 y=1 | C t=0 y=1 | C
/// t=5 y=7 | C t=5 y=1 | A end y=1`.
#[test]
fn x3v_hier_task_output_array_unpack_in_a_static_task() {
    pin(
        "module child;\n\
  task automatic tk(output logic [3:0] o [0:1]); o[0] = 4'd1; o[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  child u();\n\
  logic [3:0] m [0:1]; logic [3:0] y;\n\
  task t; u.tk(m); endtask\n\
  always_comb begin y = m[0]; t(); $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 m[0] = 7; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=x", "C t=5 y=7", "A end y=7"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). lens-sound r2 S2 (X3e). r3 = PRE = s2v. Cell `X3e`.
///
/// Raw lines: PRE `C t=0 y=1 | C t=5 y=1 | A end y=1`; iv `t.sv:2: sorry: Subroutine ports with
/// unpacked dimensions are not yet supported. | t.sv:7: warning: user task (t) must be automatic
/// to be synthesized in an always_comb process. | t.sv:8: warning: System task ($display) cannot
/// be synthesized in an always_comb process. | 1 error(s) during elaboration. | (iverilog
/// refused)`; s2v `C t=0 y=1 | C t=5 y=1 | A end y=1`; vl `C t=0 y=1 | A end y=1`.
#[test]
fn x3e_hier_task_output_array_read_after_the_call() {
    pin(
        "module child;\n\
  task automatic tk(output logic [3:0] o [0:1]); o[0] = 4'd1; o[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  child u();\n\
  logic [3:0] m [0:1]; logic [3:0] y;\n\
  task t; u.tk(m); endtask\n\
  always_comb begin t(); y = m[0]; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 m[0] = 7; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=1", "C t=5 y=1", "A end y=1"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). lens-sound r2 N-PACK (X2a); PRE was wrong. Cell `X2a`.
///
/// Raw lines: PRE `C t=0 | C t=0 | C t=5 | A end s=x`; iv `t.sv:3: sorry: Subroutine ports with
/// unpacked dimensions are not yet supported. | t.sv:8: error: Array m needs an array index here.
/// | t.sv:8: warning: System task ($display) cannot be synthesized in an always_comb process. | 2
/// error(s) during elaboration. | (iverilog refused)`; s2v `C t=0 | A end s=x`; vl `C t=0 | A end
/// s=0`.
#[test]
fn x2a_hier_task_input_array_pack_reads_are_the_argument() {
    pin(
        "module child;\n\
  logic [3:0] s;\n\
  task automatic tk(input logic [3:0] i [0:1]); s = i[0] + i[1]; endtask\n\
endmodule\n\
module top;\n\
  child u();\n\
  logic [3:0] m [0:1]; logic c;\n\
  always_comb begin assert (c) else u.tk(m); $display(\"C t=%0t\", $time); end\n\
  initial begin c = 1; m[0] = 0; m[1] = 0; #5 m[0] = 1; #2 $display(\"A end s=%0d\", u.s); $finish; end\n\
endmodule\n\
",
        &["C t=0", "C t=0", "A end s=x"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). lens-sound r2 N-PACK (X2c); PRE was wrong. r3 = s2v. Cell `X2c`.
///
/// Raw lines: PRE `C t=0 | C t=0 | C t=5 | A end s=1`; iv `t.sv:3: sorry: Subroutine ports with
/// unpacked dimensions are not yet supported. | t.sv:8: error: Array m needs an array index here.
/// | t.sv:9: warning: always_comb process has no sensitivities. | t.sv:8: warning: user task (t)
/// must be automatic to be synthesized in an always_comb process. | t.sv:9: warning: System task
/// ($display) cannot be synthesized in an always_comb process. | 2 error(s) during elaboration. |
/// (iverilog refused)`; s2v `C t=0 | A end s=0`; vl `C t=0 | A end s=1`.
#[test]
fn x2c_hier_task_input_array_pack_comb() {
    pin(
        "module child;\n\
  logic [3:0] s;\n\
  task automatic tk(input logic [3:0] i [0:1]); s = i[0] + i[1]; endtask\n\
endmodule\n\
module top;\n\
  child u();\n\
  logic [3:0] m [0:1];\n\
  task t; u.tk(m); endtask\n\
  always_comb begin t(); $display(\"C t=%0t\", $time); end\n\
  initial begin m[0] = 0; m[1] = 0; #5 m[0] = 1; #2 $display(\"A end s=%0d\", u.s); $finish; end\n\
endmodule\n\
",
        &["C t=0", "A end s=0"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering appends its IR under the mark of the node
/// it replaces or of the call site, so the `@*` / in-body `@(*)` set is its local twin's (IEEE
/// 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and variables that are read by the
/// statement (which can be a statement group) of a procedural_timing_control_statement" and
/// :9590-9597 (the identifiers in the statement; "in subroutine calls" = a call's arguments)).
/// lens-sound r2 N-PACK (X2s); PRE was wrong. r3 = s2v. Cell `X2s`.
///
/// Raw lines: PRE `S t=0 k=0 | S t=5 k=0 | A end s=1`; iv `t.sv:3: sorry: Subroutine ports with
/// unpacked dimensions are not yet supported. | t.sv:8: error: Array m needs an array index here.
/// | 2 error(s) during elaboration. | (iverilog refused)`; s2v `S t=0 k=0 | A end s=0`; vl `S t=0
/// k=0 | S t=5 k=0 | S t=5 k=0 | A end s=1 | S t=7 k=0 | S t=7 k=0`.
#[test]
fn x2s_hier_task_input_array_pack_star() {
    pin(
        "module child;\n\
  logic [3:0] s;\n\
  task automatic tk(input logic [3:0] i [0:1]); s = i[0] + i[1]; endtask\n\
endmodule\n\
module top;\n\
  child u();\n\
  logic [3:0] m [0:1]; logic [3:0] k;\n\
  task t; u.tk(m); endtask\n\
  always @* begin t(); $display(\"S t=%0t k=%0d\", $time, k); end\n\
  initial begin k = 0; m[0] = 0; m[1] = 0; #5 m[0] = 1; #2 $display(\"A end s=%0d\", u.s); $finish; end\n\
endmodule\n\
",
        &["S t=0 k=0", "A end s=0"],
    );
}

/// R-MEMBER (ENG-2 r3): an in-body `@(*)`'s controlled statement is the statements its open block
/// held when the statement's lowering ended plus what a call inside it splices there after
/// lowering, never a position count (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets
/// and variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)). Two hierarchical calls splicing into one controlled
/// statement. r3 = PRE. Cell `KP_INB2`.
///
/// Raw lines: PRE `W t=5 y=5 m1=2 n1=2 | A end y=1`; iv `(iverilog refused) | t.sv:2: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | 1 error(s) during
/// elaboration.`; s2v `(sv2v/iverilog refused) | s2v.v:19: error: 'm' is not a valid l-value for a
/// procedural assignment. | s2v.v:12:      : 'm' is declared here as a wire.`; vl `(verilator
/// refused) | %Error-UNSUPPORTED: t.sv:9:5: Event control with implicit sensitivity (@*) in this
/// context; use an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn kp_inb2_two_front_splices() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] n [2];\n\
  logic [3:0] a = 0, b = 0, y;\n\
  initial begin\n\
    @(*) begin u.w(m); u.w(n); y = a; end\n\
    $display(\"W t=%0t y=%0d m1=%0d n1=%0d\", $time, y, m[1], n[1]);\n\
    y = b;\n\
  end\n\
  initial begin #3 b = 4'd1; #2 a = 4'd5; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=5 m1=2 n1=2", "A end y=1"],
    );
}

/// R-MEMBER (ENG-2 r3): an in-body `@(*)`'s controlled statement is the statements its open block
/// held when the statement's lowering ended plus what a call inside it splices there after
/// lowering, never a position count (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets
/// and variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)). A splice from a hierarchical call inside an inlined
/// static task. r3 = PRE. Cell `KP_INB_STASK`.
///
/// Raw lines: PRE `W t=5 y=5 m1=2 | A end y=1`; iv `(iverilog refused) | t.sv:2: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | 1 error(s) during elaboration.`; s2v
/// `(sv2v/iverilog refused) | s2v.v:17: error: 'm' is not a valid l-value for a procedural
/// assignment. | s2v.v:12:      : 'm' is declared here as a wire.`; vl `(verilator refused) |
/// %Error-UNSUPPORTED: t.sv:10:5: Event control with implicit sensitivity (@*) in this context;
/// use an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn kp_inb_stask_front_splice_inside_an_inlined_static_task() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2];\n\
  logic [3:0] a = 0, b = 0, y;\n\
  task t; u.w(m); endtask\n\
  initial begin\n\
    @(*) begin t(); y = a; end\n\
    $display(\"W t=%0t y=%0d m1=%0d\", $time, y, m[1]);\n\
    y = b;\n\
  end\n\
  initial begin #3 b = 4'd1; #2 a = 4'd5; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=5 m1=2", "A end y=1"],
    );
}

/// R-SETTLE (ENG-2 r3): every implicit read set is computed once after the last post-lowering
/// builder, for every design — a `$bits(u.x)` width is resolved before any write region is folded
/// from it (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"). The always_latch twin of
/// BITS_LATE. r3 = PRE. Cell `KP_BITS_LATCH`.
///
/// Raw lines: PRE `L t=0 y=x | L t=0 y=0 | L t=5 y=9 | A end y=9`; iv `L t=0 y=0 | A end y=0`; s2v
/// `(sv2v/iverilog refused) | s2v.v:14: syntax error | s2v.v:14: Syntax in assignment statement
/// l-value.`; vl `L t=0 y=0 | L t=5 y=9 | L t=5 y=9 | A end y=9 | L t=7 y=9 | L t=7 y=9`.
#[test]
fn kp_bits_latch_hier_bits_width_in_always_latch() {
    pin(
        "module sub; logic [3:0] x; endmodule\n\
module top;\n\
  sub u();\n\
  logic [7:0] v; logic [3:0] a = 0, b = 0; logic [3:0] y;\n\
  always_latch begin v[$bits(u.x)-1:0] = a; y = v[7:4]; $display(\"L t=%0t y=%0d\", $time, y); end\n\
  always_comb v[7:4] = b;\n\
  initial begin #5 b = 4'd9; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["L t=0 y=x", "L t=0 y=0", "L t=5 y=9", "A end y=9"],
    );
}

/// R-SETTLE (ENG-2 r3): every implicit read set is computed once after the last post-lowering
/// builder, for every design — a `$bits(u.x)` width is resolved before any write region is folded
/// from it (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"). A READ region over
/// `$bits(u.x)` (r2 = PRE woke at 5). r3 = iv = vl. Cell `KP_BITS_READ`.
///
/// Raw lines: PRE `C t=0 y=0 | C t=0 y=0 | C t=5 y=0 | A end y=0`; iv `C t=0 y=0 | A end y=0`; s2v
/// `(sv2v/iverilog refused) | s2v.v:15: syntax error | s2v.v:15: error: Malformed statement`; vl
/// `C t=0 y=0 | A end y=0`.
#[test]
fn kp_bits_read_read_region_over_a_hier_bits_width() {
    pin(
        "module sub; logic [3:0] x; endmodule\n\
module top;\n\
  sub u();\n\
  logic [7:0] v; logic [3:0] a = 0, b = 0; logic [3:0] y;\n\
  always_comb begin v[3:0] = a; y = v[$bits(u.x)-1 -: 4]; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  always_comb v[7:4] = b;\n\
  initial begin #5 b = 4'd9; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0", "A end y=0"],
    );
}

/// R-SITE (ENG-2 r3): a builder that runs after lowering (hierarchical select / task-call pack and
/// unpack / fill re-lowering / `$readmem` assoc marker / late edge) appends its IR under the mark
/// the lowering gives the node it replaces, or the call site's, so the read set is its local
/// twin's (IEEE 1800-2017 :9047-9050 "... read within the block or within any function called
/// within the block with the following exceptions: ... b) Any expression that is also written
/// within the block or within any function called within the block"; IEEE 1800-2017 :9056-9057
/// "Task calls are allowed in an always_comb, but the contents of the tasks do not add anything to
/// the sensitivity list."). F1 in an action arm inside a static task. r3 = PRE = s2v. Cell
/// `KP_F1_ACTION_STASK`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:2: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:5: warning: user task
/// (t) must be automatic to be synthesized in an always_comb process. | t.sv:6: warning: System
/// task ($display) cannot be synthesized in an always_comb process.`; s2v `C t=0 o=x | C t=5 o=7 |
/// A end o=7`; vl `C t=0 o=0 | C t=0 o=0 | C t=5 o=7 | A end o=7`.
#[test]
fn kp_f1_action_stask_hier_task_output_array_in_an_action_in_a_static_task() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top; sub u(); logic [3:0] m [2]; logic [3:0] o;\n\
  task t; u.w(m); endtask\n\
  always_comb begin o = m[1]; assert (o != 4'd15) else t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "C t=5 o=7", "A end o=7"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. F1 in an automatic task. r3 =
/// PRE = s2v. Cell `F1_frame`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:2: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:8: warning: System task
/// ($display) cannot be synthesized in an always_comb process. | 1 error(s) during elaboration.`;
/// s2v `C t=0 o=x | C t=5 o=7 | A end o=7`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C
/// t=5 o=2 | A end o=2`.
#[test]
fn f1_frame_control_hier_task_output_array_in_a_frame_task() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o;\n\
  task automatic t; u.w(m); endtask\n\
  always_comb begin o = m[1]; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "C t=5 o=7", "A end o=7"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. F1's local twin. r3 = PRE = s2v.
/// Cell `F1_local_control`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:3: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:4: warning: user task
/// (t) must be automatic to be synthesized in an always_comb process. | t.sv:5: warning: System
/// task ($display) cannot be synthesized in an always_comb process.`; s2v `C t=0 o=x | C t=5 o=7 |
/// A end o=7`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C t=5 o=2 | A end o=2`.
#[test]
fn f1_local_control_local_task_output_array() {
    pin(
        "module top;\n\
  logic [3:0] m [2]; logic [3:0] o;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
  task t; w(m); endtask\n\
  always_comb begin o = m[1]; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "C t=5 o=7", "A end o=7"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. F1 with a scalar output. r3 =
/// PRE = iv = s2v. Cell `F1_scalar`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `C t=0 o=x | C t=5 o=7 | A end o=7`; s2v
/// `C t=0 o=x | C t=5 o=7 | A end o=7`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C t=5
/// o=2 | A end o=2`.
#[test]
fn f1_scalar_control_hier_task_scalar_output() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a); a = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m; logic [3:0] o;\n\
  task t; u.w(m); endtask\n\
  always_comb begin o = m; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "C t=5 o=7", "A end o=7"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. F1 with a packed 2-D output. r3
/// = PRE = iv = s2v. Cell `F1_packed2d`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `C t=0 o=x | C t=5 o=7 | A end o=7`; s2v
/// `C t=0 o=x | C t=5 o=7 | A end o=7`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C t=5
/// o=2 | A end o=2`.
#[test]
fn f1_packed2d_control_hier_task_packed_output() {
    pin(
        "module sub;\n\
  task automatic w(output logic [1:0][3:0] a); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [1:0][3:0] m; logic [3:0] o;\n\
  task t; u.w(m); endtask\n\
  always_comb begin o = m[1]; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "C t=5 o=7", "A end o=7"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. F1 under always @*. r3 = PRE =
/// s2v. Cell `F1_star`.
///
/// Raw lines: PRE `C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:2: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | 1 error(s) during elaboration.`; s2v `C
/// t=5 o=7 | A end o=7`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C t=5 o=2 | A end
/// o=2`.
#[test]
fn f1_star_control_hier_task_output_array_in_always_star() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o;\n\
  task t; u.w(m); endtask\n\
  always @* begin o = m[1]; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=5 o=7", "A end o=7"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. F1 under an in-body @(*). r3 =
/// PRE = s2v. Cell `F1_inbody`.
///
/// Raw lines: PRE `C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:2: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | 1 error(s) during elaboration.`; s2v `C
/// t=5 o=7 | A end o=7`; vl `(verilator refused) | %Error-UNSUPPORTED: t.sv:8:16: Event control
/// with implicit sensitivity (@*) in this context; use an e... | %Error: Exiting due to 1
/// error(s)`.
#[test]
fn f1_inbody_control_hier_task_output_array_in_an_inbody_star() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o;\n\
  task t; u.w(m); endtask\n\
  always begin @* begin o = m[1]; t(); $display(\"C t=%0t o=%0d\", $time, o); end end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=5 o=7", "A end o=7"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. F1 under @* inside an automatic
/// task. r3 = PRE = s2v. Cell `F1_auto_star`.
///
/// Raw lines: PRE `C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:2: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | 1 error(s) during elaboration.`; s2v `C
/// t=5 o=7 | A end o=7`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C t=5 o=2 | A end
/// o=2`.
#[test]
fn f1_auto_star_control_hier_task_output_array_in_an_automatic_task_star() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o;\n\
  task automatic t; u.w(m); endtask\n\
  always @* begin o = m[1]; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=5 o=7", "A end o=7"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. F2 in an automatic task. r3 =
/// PRE = s2v. Cell `F2_auto`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=1 o=1 | A end o=1`; iv `(iverilog refused) | t.sv:3: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:8: error: Array m needs
/// an array index here. | t.sv:9: warning: System task ($display) cannot be synthesized in an
/// always_comb process.`; s2v `C t=0 o=0 | C t=1 o=1 | A end o=1`; vl `C t=0 o=0 | C t=1 o=1 | C
/// t=1 o=1 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C t=7 o=1`.
#[test]
fn f2_auto_control_hier_task_input_array_in_a_frame_task() {
    pin(
        "module sub;\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2] = '{4'd0, 4'd0}; logic [3:0] o; logic [3:0] g = 0;\n\
  task automatic t; u.r(m); endtask\n\
  always_comb begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=1 o=1", "A end o=1"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. F2 with an inout formal (now by
/// the right reason). r3 = PRE = s2v. Cell `F2_inout`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=1 o=1 | A end o=1`; iv `(iverilog refused) | t.sv:3: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:8: error: Array m needs
/// an array index here. | t.sv:8: warning: user task (t) must be automatic to be synthesized in an
/// always_comb process.`; s2v `C t=0 o=0 | C t=1 o=1 | A end o=1`; vl `(verilator refused) |
/// %Error: Internal Error: t.sv:8:15: ../V3DfgSynthesize.cpp:358: Non-ReadOnly reference`.
#[test]
fn f2_inout_control_hier_task_inout_array() {
    pin(
        "module sub;\n\
  logic [3:0] d;\n\
  task automatic r(inout logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2] = '{4'd0, 4'd0}; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; u.r(m); endtask\n\
  always_comb begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=1 o=1", "A end o=1"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. F2 with a scalar input. r3 = PRE
/// = iv = s2v. Cell `F2_scalar`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=1 o=1 | A end o=1`; iv `C t=0 o=0 | C t=1 o=1 | A end o=1`; s2v
/// `C t=0 o=0 | C t=1 o=1 | A end o=1`; vl `C t=0 o=0 | C t=1 o=1 | C t=1 o=1 | C t=5 o=1 | C t=5
/// o=1 | A end o=1 | C t=7 o=1 | C t=7 o=1`.
#[test]
fn f2_scalar_control_hier_task_scalar_input() {
    pin(
        "module sub;\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a); d = a; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m = 4'd0; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; u.r(m); endtask\n\
  always_comb begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 m = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=1 o=1", "A end o=1"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. F2 with a packed 2-D input. r3 =
/// PRE = iv = s2v. Cell `F2_packed2d`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=1 o=1 | A end o=1`; iv `C t=0 o=0 | C t=1 o=1 | A end o=1`; s2v
/// `C t=0 o=0 | C t=1 o=1 | A end o=1`; vl `C t=0 o=0 | C t=1 o=1 | C t=1 o=1 | C t=5 o=1 | C t=5
/// o=1 | A end o=1 | C t=7 o=1 | C t=7 o=1`.
#[test]
fn f2_packed2d_control_hier_task_packed_input() {
    pin(
        "module sub;\n\
  logic [3:0] d;\n\
  task automatic r(input logic [1:0][3:0] a); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [1:0][3:0] m = '0; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; u.r(m); endtask\n\
  always_comb begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=1 o=1", "A end o=1"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. F2's local twin. r3 = s2v = vl.
/// Cell `F2_local_control`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=5 o=0 | A end o=0`; iv `(iverilog refused) | t.sv:3: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:4: error: Array m needs
/// an array index here. | t.sv:4: warning: user task (t) must be automatic to be synthesized in an
/// always_comb process.`; s2v `C t=0 o=0 | A end o=0`; vl `C t=0 o=0 | A end o=0`.
#[test]
fn f2_local_control_local_task_input_array() {
    pin(
        "module top;\n\
  logic [3:0] m [2] = '{4'd0, 4'd0}; logic [3:0] o; logic [3:0] g = 0, d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
  task t; r(m); endtask\n\
  always_comb begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "A end o=0"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. F2_val's local twin. r3 = s2v.
/// Cell `F2_val_local`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=3 o=5 | C t=5 o=5 | A end o=5`; iv `(iverilog refused) |
/// t.sv:3: sorry: Subroutine ports with unpacked dimensions are not yet supported. | t.sv:4:
/// error: Array m needs an array index here. | t.sv:4: warning: user task (t) must be automatic to
/// be synthesized in an always_comb process.`; s2v `C t=0 o=0 | A end o=0`; vl `C t=0 o=0 | C t=3
/// o=5 | C t=3 o=5 | C t=5 o=5 | C t=5 o=5 | A end o=5 | C t=7 o=5 | C t=7 o=5`.
#[test]
fn f2_val_local_control_local_task_input_array_value() {
    pin(
        "module top;\n\
  logic [3:0] m [2] = '{4'd0, 4'd0}; logic [3:0] o, y, q = 0, g = 0, d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
  task t; r(m); o = q; endtask\n\
  always_comb begin y = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #3 q = 4'd5; #2 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "A end o=0"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. R2.1: a function called in the
/// block adds its argument reads. r3 = s2v. Cell `F2_fn`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=1 o=1 | A end o=1`; iv `(iverilog refused) | t.sv:3: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:8: error: Array m needs
/// an array index here. | Assertion failed: (net->pin_count() == 1), function func_always_sens,
/// file net_nex_input.cc, line 283.`; s2v `C t=0 o=0 | C t=1 o=1 | C t=5 o=1 | A end o=1`; vl `C
/// t=0 o=0 | C t=1 o=1 | C t=1 o=1 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C t=7 o=1`.
#[test]
fn f2_fn_control_hier_function_input_array_read() {
    pin(
        "module sub;\n\
  logic [3:0] d;\n\
  function automatic void r(input logic [3:0] a [2]); d = a[0]; endfunction\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2] = '{4'd0, 4'd0}; logic [3:0] o; logic [3:0] g = 0;\n\
  function void t; u.r(m); endfunction\n\
  always_comb begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=1 o=1", "C t=5 o=1", "A end o=1"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. F2 called directly from
/// always_comb (the argument is read). r3 = s2v. Cell `F2_direct_noinit`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=1 o=1 | A end o=1`; iv `(iverilog refused) | t.sv:3: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:8: error: Array m needs
/// an array index here. | t.sv:8: warning: System task ($display) cannot be synthesized in an
/// always_comb process.`; s2v `C t=0 o=0 | C t=1 o=1 | C t=5 o=1 | A end o=1`; vl `C t=0 o=0 | C
/// t=1 o=1 | C t=1 o=1 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C t=7 o=1`.
#[test]
fn f2_direct_noinit_control_hier_task_input_array_direct() {
    pin(
        "module sub;\n\
  logic [3:0] d;\n\
  task automatic r(input logic [3:0] a [2]); d = a[0]; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o; logic [3:0] g = 0;\n\
  always_comb begin o = g; u.r(m); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=1 o=1", "C t=5 o=1", "A end o=1"],
    );
}

/// R2 / OP2 (ENG-2 r3, unchanged): an `output` / `inout` actual is written whole by its copy-out
/// on every path (§13.5 :14750-14751 (output / inout arguments are copied out when the subroutine
/// returns)), so it covers the whole actual whatever the callee writes; the pin cites the LRM, not
/// iv (iv covers an output only when the task writes it, OP-7). F1 called directly from
/// always_comb (OP2 decided). Cell `F1_direct`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:2: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:7: warning: System task
/// ($display) cannot be synthesized in an always_comb process. | 1 error(s) during elaboration.`;
/// s2v `C t=0 o=x | C t=5 o=7 | A end o=7`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C
/// t=5 o=2 | A end o=2`.
#[test]
fn f1_direct_control_hier_task_output_array_direct() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o;\n\
  always_comb begin o = m[1]; u.w(m); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "A end o=x"],
    );
}

/// R2 / OP2 (ENG-2 r3, unchanged): an `output` / `inout` actual is written whole by its copy-out
/// on every path (§13.5 :14750-14751 (output / inout arguments are copied out when the subroutine
/// returns)), so it covers the whole actual whatever the callee writes; the pin cites the LRM, not
/// iv (iv covers an output only when the task writes it, OP-7). F1_direct's local twin. Cell
/// `F1_direct_local`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:3: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:4: warning: System task
/// ($display) cannot be synthesized in an always_comb process. | 1 error(s) during elaboration.`;
/// s2v `C t=0 o=x | C t=5 o=7 | A end o=7`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C
/// t=5 o=2 | A end o=2`.
#[test]
fn f1_direct_local_control_local_task_output_array_direct() {
    pin(
        "module top;\n\
  logic [3:0] m [2]; logic [3:0] o;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
  always_comb begin o = m[1]; w(m); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "A end o=x"],
    );
}

/// R2 / OP2 (ENG-2 r3, unchanged): an `output` / `inout` actual is written whole by its copy-out
/// on every path (§13.5 :14750-14751 (output / inout arguments are copied out when the subroutine
/// returns)), so it covers the whole actual whatever the callee writes; the pin cites the LRM, not
/// iv (iv covers an output only when the task writes it, OP-7). F1_direct with a scalar output. r3
/// = iv. Cell `F1_direct_scalar`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `C t=0 o=x | A end o=x`; s2v `C t=0 o=x
/// | C t=5 o=7 | A end o=7`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C t=5 o=2 | A end
/// o=2`.
#[test]
fn f1_direct_scalar_control_hier_task_scalar_output_direct() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a); a = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m; logic [3:0] o;\n\
  always_comb begin o = m; u.w(m); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "A end o=x"],
    );
}

/// R2 / OP2 (ENG-2 r3, unchanged): an `output` / `inout` actual is written whole by its copy-out
/// on every path (§13.5 :14750-14751 (output / inout arguments are copied out when the subroutine
/// returns)), so it covers the whole actual whatever the callee writes; the pin cites the LRM, not
/// iv (iv covers an output only when the task writes it, OP-7). F1 through a function (OP1
/// decided). Cell `F1_fn`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:2: error:
/// Function top.u.w port a is not an input port. | t.sv:2:      : Function arguments must be input
/// ports. | t.sv:2: sorry: Subroutine ports with unpacked dimensions are not yet supported.`; s2v
/// `C t=0 o=x | C t=0 o=2 | C t=5 o=7 | C t=5 o=2 | A end o=2`; vl `C t=0 o=0 | C t=0 o=2 | C t=0
/// o=2 | C t=5 o=7 | C t=5 o=2 | A end o=2`.
#[test]
fn f1_fn_control_hier_function_output_array() {
    pin(
        "module sub;\n\
  function automatic void w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endfunction\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o;\n\
  function void t; u.w(m); endfunction\n\
  always_comb begin o = m[1]; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "A end o=x"],
    );
}

/// R2 / OP2 (ENG-2 r3, unchanged): an `output` / `inout` actual is written whole by its copy-out
/// on every path (§13.5 :14750-14751 (output / inout arguments are copied out when the subroutine
/// returns)), so it covers the whole actual whatever the callee writes; the pin cites the LRM, not
/// iv (iv covers an output only when the task writes it, OP-7). F1_fn's local twin. Cell
/// `F1_fn_local`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:3: error:
/// Function top.w port a is not an input port. | t.sv:3:      : Function arguments must be input
/// ports. | t.sv:3: sorry: Subroutine ports with unpacked dimensions are not yet supported.`; s2v
/// `C t=0 o=x | C t=5 o=7 | A end o=7`; vl `C t=0 o=0 | C t=0 o=2 | C t=0 o=2 | C t=5 o=7 | C t=5
/// o=2 | A end o=2`.
#[test]
fn f1_fn_local_control_local_function_output_array() {
    pin(
        "module top;\n\
  logic [3:0] m [2]; logic [3:0] o;\n\
  function automatic void w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endfunction\n\
  function void t; w(m); endfunction\n\
  always_comb begin o = m[1]; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "A end o=x"],
    );
}

/// C4 (decided, owner flag OP-c): a write in an assertion action arm is a write of the block (IEEE
/// 1800-2017 :9047-9050 "... read within the block or within any function called within the block
/// with the following exceptions: ... b) Any expression that is also written within the block or
/// within any function called within the block"), so the read it covers drops — hierarchical =
/// local twin; PRE = s2v = vl wake. F1 in an assertion action arm. Cell `F1_action`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:2: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:7: warning: System task
/// ($display) cannot be synthesized in an always_comb process. | 1 error(s) during elaboration.`;
/// s2v `C t=0 o=x | C t=5 o=7 | A end o=7`; vl `C t=0 o=0 | C t=5 o=7 | C t=5 o=7 | A end o=7 | C
/// t=7 o=7 | C t=7 o=7`.
#[test]
fn f1_action_control_hier_task_output_array_in_an_action() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2]; logic [3:0] o; logic [3:0] g = 0;\n\
  always_comb begin o = m[1]; assert (g < 4'd15) else u.w(m); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "A end o=x"],
    );
}

/// C4 (decided, owner flag OP-c): a write in an assertion action arm is a write of the block (IEEE
/// 1800-2017 :9047-9050 "... read within the block or within any function called within the block
/// with the following exceptions: ... b) Any expression that is also written within the block or
/// within any function called within the block"), so the read it covers drops — hierarchical =
/// local twin; PRE = s2v = vl wake. F1_action's local twin. Cell `F1_action_local`.
///
/// Raw lines: PRE `C t=0 o=x | C t=5 o=7 | A end o=7`; iv `(iverilog refused) | t.sv:2: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:4: warning: System task
/// ($display) cannot be synthesized in an always_comb process. | 1 error(s) during elaboration.`;
/// s2v `C t=0 o=x | C t=5 o=7 | A end o=7`; vl `C t=0 o=0 | C t=5 o=7 | C t=5 o=7 | A end o=7 | C
/// t=7 o=7 | C t=7 o=7`.
#[test]
fn f1_action_local_control_local_task_output_array_in_an_action() {
    pin(
        "module top;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
  logic [3:0] m [2]; logic [3:0] o; logic [3:0] g = 0;\n\
  always_comb begin o = m[1]; assert (g < 4'd15) else w(m); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "A end o=x"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. A hierarchical fill written
/// directly in the block. r3 = PRE = iv. Cell `P_fill_direct`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=1 o=1 | C t=5 o=1 | A end o=1`; iv `C t=0 o=0 | C t=1 o=1 | C
/// t=5 o=1 | A end o=1`; s2v `(sv2v/iverilog refused) | s2v.v:14: syntax error | s2v.v:14: error:
/// Malformed statement`; vl `C t=0 o=0 | C t=1 o=1 | C t=1 o=1 | C t=5 o=1 | C t=5 o=1 | A end o=1
/// | C t=7 o=1 | C t=7 o=1`.
#[test]
fn p_fill_direct_control_hier_fill_in_always_comb() {
    pin(
        "module sub;\n\
  logic [7:0] x;\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] k = 0; logic [3:0] o; logic [3:0] g = 0;\n\
  always_comb begin o = g; u.x = k | '0; $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 k = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=1 o=1", "C t=5 o=1", "A end o=1"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. P_fill's local twin. r3 = iv =
/// s2v. Cell `P_fill_local`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=1 o=1 | C t=5 o=1 | A end o=1`; iv `C t=0 o=0 | C t=1 o=1 | A
/// end o=1`; s2v `C t=0 o=0 | C t=1 o=1 | A end o=1`; vl `C t=0 o=0 | C t=1 o=1 | C t=1 o=1 | C
/// t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C t=7 o=1`.
#[test]
fn p_fill_local_control_local_fill() {
    pin(
        "module top;\n\
  logic [7:0] x;\n\
  logic [3:0] k = 0; logic [3:0] o; logic [3:0] g = 0;\n\
  task t; x = k | '0; endtask\n\
  always_comb begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 k = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=1 o=1", "A end o=1"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. P_fill_action's local twin. r3 =
/// s2v. Cell `P_action_fill_local`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=1 o=1 | C t=5 o=1 | A end o=1`; iv `C t=0 o=0 | C t=1 o=1 | C
/// t=5 o=1 | A end o=1`; s2v `C t=0 o=0 | C t=1 o=1 | A end o=1`; vl `C t=0 o=0 | C t=1 o=1 | C
/// t=1 o=1 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C t=7 o=1`.
#[test]
fn p_action_fill_local_control_local_fill_in_an_action() {
    pin(
        "module top;\n\
  logic [7:0] x;\n\
  logic [3:0] k = 0; logic [3:0] o; logic [3:0] g = 0;\n\
  always_comb begin o = g; assert (g < 4'd15) else x = k | '0; $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 k = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=1 o=1", "A end o=1"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. P_fill_action without the fill.
/// r3 = s2v. Cell `P_action_nofill`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=1 o=1 | C t=5 o=1 | A end o=1`; iv `C t=0 o=0 | C t=1 o=1 | C
/// t=5 o=1 | A end o=1`; s2v `C t=0 o=0 | C t=1 o=1 | A end o=1`; vl `C t=0 o=0 | C t=1 o=1 | C
/// t=1 o=1 | C t=5 o=1 | C t=5 o=1 | A end o=1 | C t=7 o=1 | C t=7 o=1`.
#[test]
fn p_action_nofill_control_action_without_a_fill() {
    pin(
        "module sub;\n\
  logic [7:0] x;\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] k = 0; logic [3:0] o; logic [3:0] g = 0;\n\
  always_comb begin o = g; assert (g < 4'd15) else u.x = k; $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #1 g = 4'd1; #4 k = 4'd7; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "C t=1 o=1", "A end o=1"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. P_readmem_assoc's local twin.
/// Cell `P_readmem_assoc_local`.
/// The memory file `mem.hex` (`1`, `2`: words before any `@address`, so W4023 and nothing loaded,
/// as in the cell) is written next to `t.sv`.
///
/// Raw lines: PRE `W4023: $readmem('mem.hex'): a word before any `@address`, into an a... | C t=0
/// o=0 | W4023: $readmem('mem.hex'): a word before any `@address`, into an a... | C t=1 o=1 |
/// W4023: $readmem('mem.hex'): a word before any `@address`, into an a... | C t=5 o=1 | A end
/// o=1`; iv `(iverilog refused) | t.sv:2: error: Type names are not valid expressions here. |
/// t.sv:2: internal error: I do not know how to elaborate this expression. | t.sv:2:
/// : Expression is: <type>`; s2v `(sv2v/iverilog refused) | t.sv:2:14: Parse error: unexpected
/// token ']' (Sym_brack_r)`; vl `C t=0 o=0 | C t=1 o=1 | C t=1 o=1 | C t=5 o=1 | C t=5 o=1 | A end
/// o=1 | C t=7 o=1 | C t=7 o=1`.
#[test]
fn p_readmem_assoc_local_control_local_assoc_marker() {
    pin_files(
        "module top;\n\
  int aa [int];\n\
  logic [3:0] o; logic [3:0] g = 0;\n\
  task t; $readmemh(\"mem.hex\", aa); endtask\n\
  always_comb begin o = g; t(); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial #5 aa[3] = 9;\n\
  initial begin #1 g = 4'd1; #6 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &[("mem.hex", "1\n2\n")],
        &["C t=0 o=0", "C t=1 o=1", "A end o=1"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. INB_SPLICE with the call last in
/// the controlled statement. r3 = PRE. Cell `INB_SPLICE_last`.
///
/// Raw lines: PRE `W t=5 y=5 m1=2 | A end y=1`; iv `(iverilog refused) | t.sv:2: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | 1 error(s) during elaboration.`; s2v
/// `(sv2v/iverilog refused) | s2v.v:19: error: 'm' is not a valid l-value for a procedural
/// assignment. | s2v.v:12:      : 'm' is declared here as a wire.`; vl `(verilator refused) |
/// %Error-UNSUPPORTED: t.sv:9:5: Event control with implicit sensitivity (@*) in this context; use
/// an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn inb_splice_last_control_call_last_in_the_statement() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] m [2];\n\
  logic [3:0] a = 0, b = 0, y;\n\
  initial begin\n\
    @(*) begin y = a; u.w(m); end\n\
    $display(\"W t=%0t y=%0d m1=%0d\", $time, y, m[1]);\n\
    y = b;\n\
  end\n\
  initial begin #3 b = 4'd1; #2 a = 4'd5; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=5 m1=2", "A end y=1"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. INB_SPLICE's local twin. r3 =
/// PRE = s2v. Cell `INB_SPLICE_local`.
///
/// Raw lines: PRE `W t=5 y=5 m1=2 | A end y=1`; iv `(iverilog refused) | t.sv:2: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | 1 error(s) during elaboration.`; s2v `W
/// t=5 y=5 m1=2 | A end y=1`; vl `(verilator refused) | %Error-UNSUPPORTED: t.sv:6:5: Event
/// control with implicit sensitivity (@*) in this context; use an e... | %Error: Exiting due to 1
/// error(s)`.
#[test]
fn inb_splice_local_control_local_task() {
    pin(
        "module top;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
  logic [3:0] m [2];\n\
  logic [3:0] a = 0, b = 0, y;\n\
  initial begin\n\
    @(*) begin w(m); y = a; end\n\
    $display(\"W t=%0t y=%0d m1=%0d\", $time, y, m[1]);\n\
    y = b;\n\
  end\n\
  initial begin #3 b = 4'd1; #2 a = 4'd5; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=5 m1=2", "A end y=1"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. BITS_LATE plus another
/// hierarchical read (r2's arming list ran). r3 = PRE. Cell `BITS_LATE_armed`.
///
/// Raw lines: PRE `C t=0 y=x | C t=0 y=0 | C t=5 y=9 | A end y=9`; iv `C t=0 y=0 | A end y=0`; s2v
/// `(sv2v/iverilog refused) | s2v.v:15: syntax error | s2v.v:15: Syntax in assignment statement
/// l-value.`; vl `C t=0 y=0 | C t=5 y=9 | C t=5 y=9 | A end y=9 | C t=7 y=9 | C t=7 y=9`.
#[test]
fn bits_late_armed_control_with_another_hier_read() {
    pin(
        "module sub;\n\
  logic [3:0] x = 0;\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [7:0] v; logic [3:0] a = 0, b = 0; logic [3:0] y; logic q;\n\
  always_comb begin v[$bits(u.x)-1:0] = a; y = v[7:4]; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  always_comb v[7:4] = b;\n\
  always_comb q = u.x[0];\n\
  initial begin #5 b = 4'd9; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=x", "C t=0 y=0", "C t=5 y=9", "A end y=9"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. BITS_LATE's local twin
/// `$bits(x)`. r3 = PRE = s2v. Cell `BITS_LOCAL`.
///
/// Raw lines: PRE `C t=0 y=x | C t=0 y=0 | C t=5 y=9 | A end y=9`; iv `C t=0 y=0 | A end y=0`; s2v
/// `C t=0 y=x | C t=0 y=0 | C t=5 y=9 | A end y=9`; vl `C t=0 y=0 | C t=5 y=9 | C t=5 y=9 | A end
/// y=9 | C t=7 y=9 | C t=7 y=9`.
#[test]
fn bits_local_control_local_bits_width() {
    pin(
        "module top;\n\
  logic [3:0] x;\n\
  logic [7:0] v; logic [3:0] a = 0, b = 0; logic [3:0] y;\n\
  always_comb begin v[$bits(x)-1:0] = a; y = v[7:4]; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  always_comb v[7:4] = b;\n\
  initial begin #5 b = 4'd9; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=x", "C t=0 y=0", "C t=5 y=9", "A end y=9"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. r2 B4 cell H3. r3 = PRE = iv =
/// s2v. Cell `H3_star_hier_sel_in_static_fn`.
///
/// Raw lines: PRE `C t=3 z=1 | A end`; iv `C t=3 z=1 | A end`; s2v `C t=3 z=1 | A end`; vl `C t=0
/// z=0 | C t=3 z=1 | C t=3 z=1 | C t=5 z=0 | C t=5 z=0 | A end | C t=7 z=0 | C t=7 z=0`.
#[test]
fn h3_control_star_hier_select_in_a_static_function() {
    pin(
        "module sub; logic [3:0] mem = 0; endmodule\n\
module top;\n\
  sub u();\n\
  logic a = 0; logic [1:0] k = 0; logic z;\n\
  function logic f(input logic x); return x ^ u.mem[k]; endfunction\n\
  always @* begin z = f(a); $display(\"C t=%0t z=%0d\", $time, z); end\n\
  initial begin #3 a = 1; #2 u.mem = 4'hf; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=3 z=1", "A end"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. r2 B4 cell H4. r3 = iv = s2v =
/// vl. Cell `H4_comb_hier_whole_in_static_task`.
///
/// Raw lines: PRE `C t=0 | C t=5 | A end`; iv `C t=0 | A end`; s2v `C t=0 | A end`; vl `C t=0 | A
/// end`.
#[test]
fn h4_control_comb_hier_whole_net_in_a_static_task() {
    pin(
        "module sub; logic q = 0; endmodule\n\
module top;\n\
  sub u();\n\
  logic z;\n\
  task t; z = u.q; endtask\n\
  always_comb begin t(); $display(\"C t=%0t\", $time); end\n\
  initial begin #5 u.q = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0", "A end"],
    );
}

/// N-STARCOMPOUND: the r1 compound rule (R2.7, IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding
/// all nets and variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments): the implicit lvalue read of `op=` is not part of `@*`)
/// holds in a static task body and at top level. The local twin of HCOMP_star (Q-HIERCOMPOUND: the
/// hierarchical form is not pinned). r3 = iv. Cell `LCOMP_star`.
///
/// Raw lines: PRE `A end v=01`; iv `A end v=00`; s2v `A end v=01`; vl `A end v=01`.
#[test]
fn lcomp_star_control_local_compound_lvalue_in_always_star() {
    pin(
        "module top;\n\
  logic [1:0] v = 2'b00;\n\
  logic m = 0;\n\
  always @* v[0] |= m;\n\
  initial begin #1 m = 1'b1; #4 v[0] = 1'b0; #2 $display(\"A end v=%b\", v); $finish; end\n\
endmodule\n\
",
        &["A end v=00"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. lens-sound r2 X1n. r3 = PRE =
/// s2v. Cell `X1n`.
///
/// Raw lines: PRE `W t=5 y=1 m0=1 | A end y=1`; iv `t.sv:2: sorry: Subroutine ports with unpacked
/// dimensions are not yet supported. | 1 error(s) during elaboration. | (iverilog refused)`; s2v
/// `W t=5 y=1 m0=1 | A end y=1`; vl `%Error-UNSUPPORTED: t.sv:7:5: Event control with implicit
/// sensitivity (@*) in this context; use an e... |                     ... For error description
/// see https://verilator.org/warn/UNSUPPORTED?v=5.052 | %Error: Exiting due to 1 error(s) |
/// (verilator refused)`.
#[test]
fn x1n_control_inbody_star_without_a_splice() {
    pin(
        "module top;\n\
  task automatic tk(output logic [3:0] o [0:1]); o[0] = 4'd1; o[1] = 4'd2; endtask\n\
  logic [3:0] m [0:1]; logic [3:0] a, y;\n\
  wire [3:0] q = a;\n\
  initial begin\n\
    a = 0;\n\
    @(*) begin tk(m); y = a; end\n\
    $display(\"W t=%0t y=%0d m0=%0d\", $time, y, m[0]);\n\
  end\n\
  initial begin #5 a = 1; #5 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1 m0=1", "A end y=1"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. lens-sound r2 X1s. r3 = PRE =
/// iv. Cell `X1s`.
///
/// Raw lines: PRE `W t=5 y=1 m=1 | A end y=1`; iv `W t=5 y=1 m=1 | A end y=1`; s2v `s2v.v:15:
/// error: 'm' is not a valid l-value for a procedural assignment. | s2v.v:9:      : 'm' is
/// declared here as a wire. | s2v.v:15: error: I give up on task port 1 expression: m | 1 error(s)
/// during elaboration. | (sv2v/iverilog refused)`; vl `%Error-UNSUPPORTED: t.sv:9:5: Event control
/// with implicit sensitivity (@*) in this context; use an e... |                     ... For error
/// description see https://verilator.org/warn/UNSUPPORTED?v=5.052 | %Error: Exiting due to 1
/// error(s) | (verilator refused)`.
#[test]
fn x1s_control_inbody_star_scalar_output() {
    pin(
        "module child;\n\
  task automatic tk(output logic [3:0] o); o = 4'd1; endtask\n\
endmodule\n\
module top;\n\
  child u();\n\
  logic [3:0] m; logic [3:0] a, y;\n\
  initial begin\n\
    a = 0;\n\
    @(*) begin u.tk(m); y = a; end\n\
    $display(\"W t=%0t y=%0d m=%0d\", $time, y, m);\n\
  end\n\
  initial begin #5 a = 1; #5 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1 m=1", "A end y=1"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. X3v's local twin. r3 = PRE =
/// s2v. Cell `X3vl`.
///
/// Raw lines: PRE `C t=0 y=x | C t=5 y=7 | A end y=7`; iv `t.sv:2: sorry: Subroutine ports with
/// unpacked dimensions are not yet supported. | t.sv:4: warning: user task (t) must be automatic
/// to be synthesized in an always_comb process. | t.sv:5: warning: System task ($display) cannot
/// be synthesized in an always_comb process. | 1 error(s) during elaboration. | (iverilog
/// refused)`; s2v `C t=0 y=x | C t=5 y=7 | A end y=7`; vl `C t=0 y=0 | C t=0 y=1 | C t=0 y=1 | C
/// t=5 y=7 | C t=5 y=1 | A end y=1`.
#[test]
fn x3vl_control_local_task_output_array_in_a_static_task() {
    pin(
        "module top;\n\
  task automatic tk(output logic [3:0] o [0:1]); o[0] = 4'd1; o[1] = 4'd2; endtask\n\
  logic [3:0] m [0:1]; logic [3:0] y;\n\
  task t; tk(m); endtask\n\
  always_comb begin y = m[0]; t(); $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 m[0] = 7; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=x", "C t=5 y=7", "A end y=7"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. X3v with a scalar output. r3 =
/// PRE = iv = s2v. Cell `X3vs`.
///
/// Raw lines: PRE `C t=0 y=x | C t=5 y=7 | A end y=7`; iv `C t=0 y=x | C t=5 y=7 | A end y=7`; s2v
/// `C t=0 y=x | C t=5 y=7 | A end y=7`; vl `C t=0 y=0 | C t=0 y=1 | C t=0 y=1 | C t=5 y=7 | C t=5
/// y=1 | A end y=1`.
#[test]
fn x3vs_control_hier_task_scalar_output_in_a_static_task() {
    pin(
        "module child;\n\
  task automatic tk(output logic [3:0] o); o = 4'd1; endtask\n\
endmodule\n\
module top;\n\
  child u();\n\
  logic [3:0] m; logic [3:0] y;\n\
  task t; u.tk(m); endtask\n\
  always_comb begin y = m; t(); $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 m = 7; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=x", "C t=5 y=7", "A end y=7"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. X2c's scalar twin. r3 = iv =
/// s2v. Cell `X2cs`.
///
/// Raw lines: PRE `C t=0 | C t=0 | C t=5 | A end s=1`; iv `C t=0 | A end s=0`; s2v `C t=0 | A end
/// s=0`; vl `C t=0 | A end s=1`.
#[test]
fn x2cs_control_hier_task_scalar_input() {
    pin(
        "module child;\n\
  logic [3:0] s;\n\
  task automatic tk(input logic [3:0] i); s = i; endtask\n\
endmodule\n\
module top;\n\
  child u();\n\
  logic [3:0] m;\n\
  task t; u.tk(m); endtask\n\
  always_comb begin t(); $display(\"C t=%0t\", $time); end\n\
  initial begin m = 0; #5 m = 1; #2 $display(\"A end s=%0d\", u.s); $finish; end\n\
endmodule\n\
",
        &["C t=0", "A end s=0"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. KP_INB2's local twin. r3 = PRE =
/// s2v. Cell `KP_INB2_local`.
///
/// Raw lines: PRE `W t=5 y=5 m1=2 n1=2 | A end y=1`; iv `(iverilog refused) | t.sv:2: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | 1 error(s) during
/// elaboration.`; s2v `W t=5 y=5 m1=2 n1=2 | A end y=1`; vl `(verilator refused) |
/// %Error-UNSUPPORTED: t.sv:6:5: Event control with implicit sensitivity (@*) in this context; use
/// an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn kp_inb2_local_control() {
    pin(
        "module top;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
  logic [3:0] m [2]; logic [3:0] n [2];\n\
  logic [3:0] a = 0, b = 0, y;\n\
  initial begin\n\
    @(*) begin w(m); w(n); y = a; end\n\
    $display(\"W t=%0t y=%0d m1=%0d n1=%0d\", $time, y, m[1], n[1]);\n\
    y = b;\n\
  end\n\
  initial begin #3 b = 4'd1; #2 a = 4'd5; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=5 m1=2 n1=2", "A end y=1"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. KP_INB_STASK's local twin. r3 =
/// PRE = s2v. Cell `KP_INB_STASK_local`.
///
/// Raw lines: PRE `W t=5 y=5 m1=2 | A end y=1`; iv `(iverilog refused) | t.sv:2: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | 1 error(s) during elaboration.`; s2v `W
/// t=5 y=5 m1=2 | A end y=1`; vl `(verilator refused) | %Error-UNSUPPORTED: t.sv:7:5: Event
/// control with implicit sensitivity (@*) in this context; use an e... | %Error: Exiting due to 1
/// error(s)`.
#[test]
fn kp_inb_stask_local_control() {
    pin(
        "module top;\n\
  task automatic w(output logic [3:0] a [2]); a[0] = 4'd1; a[1] = 4'd2; endtask\n\
  logic [3:0] m [2];\n\
  logic [3:0] a = 0, b = 0, y;\n\
  task t; w(m); endtask\n\
  initial begin\n\
    @(*) begin t(); y = a; end\n\
    $display(\"W t=%0t y=%0d m1=%0d\", $time, y, m[1]);\n\
    y = b;\n\
  end\n\
  initial begin #3 b = 4'd1; #2 a = 4'd5; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=5 m1=2", "A end y=1"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. KP_BITS_LATCH's local twin. r3 =
/// PRE = s2v. Cell `KP_BITS_LATCH_local`.
///
/// Raw lines: PRE `L t=0 y=x | L t=0 y=0 | L t=5 y=9 | A end y=9`; iv `L t=0 y=0 | A end y=0`; s2v
/// `L t=0 y=x | L t=0 y=0 | L t=5 y=9 | A end y=9`; vl `L t=0 y=0 | L t=5 y=9 | L t=5 y=9 | A end
/// y=9 | L t=7 y=9 | L t=7 y=9`.
#[test]
fn kp_bits_latch_local_control() {
    pin(
        "module top;\n\
  logic [3:0] x;\n\
  logic [7:0] v; logic [3:0] a = 0, b = 0; logic [3:0] y;\n\
  always_latch begin v[$bits(x)-1:0] = a; y = v[7:4]; $display(\"L t=%0t y=%0d\", $time, y); end\n\
  always_comb v[7:4] = b;\n\
  initial begin #5 b = 4'd9; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["L t=0 y=x", "L t=0 y=0", "L t=5 y=9", "A end y=9"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. KP_BITS_READ's local twin. r3 =
/// iv = vl. Cell `KP_BITS_READ_local`.
///
/// Raw lines: PRE `C t=0 y=0 | C t=0 y=0 | C t=5 y=0 | A end y=0`; iv `C t=0 y=0 | A end y=0`; s2v
/// `C t=0 y=0 | C t=0 y=0 | C t=5 y=0 | A end y=0`; vl `C t=0 y=0 | A end y=0`.
#[test]
fn kp_bits_read_local_control() {
    pin(
        "module top;\n\
  logic [3:0] x;\n\
  logic [7:0] v; logic [3:0] a = 0, b = 0; logic [3:0] y;\n\
  always_comb begin v[3:0] = a; y = v[$bits(x)-1 -: 4]; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  always_comb v[7:4] = b;\n\
  initial begin #5 b = 4'd9; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0", "A end y=0"],
    );
}

/// R-MEMBER (ENG-2 r3, lens N-MB3T): the open block's terminator is not part of the controlled
/// statement — the settle ends that block (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all
/// nets and variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)). Lens N-MB3T's cell. r3 = PRE = iv = s2v. Cell `MB3T`.
///
/// Raw lines: PRE `W t=5 y=1 | A end y=1`; iv `W t=5 y=1 | A end y=1`; s2v `W t=5 y=1 | A end
/// y=1`; vl `%Error-UNSUPPORTED: t.sv:8:5: Event control with implicit sensitivity (@*) in this
/// context; use an e... |                     ... For error description see
/// https://verilator.org/warn/UNSUPPORTED?v=5.052 | %Error: Exiting due to 1 error(s) | (verilator
/// refused)`.
#[test]
fn mb3t_inbody_star_open_block_terminator_is_not_read() {
    pin(
        "module child; logic q; endmodule\n\
module top;\n\
  child u();\n\
  logic [3:0] a, b, y, z; logic w;\n\
  always @* w = u.q;\n\
  initial begin\n\
    a = 0; b = 0;\n\
    @(*) y = a;\n\
    if (b == 4'd1) z = 1;\n\
    $display(\"W t=%0t y=%0d\", $time, y);\n\
  end\n\
  initial begin #3 b = 1; #2 a = 1; #5 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1", "A end y=1"],
    );
}

/// R-MEMBER (ENG-2 r3, lens N-MRB): a frame task body's in-body `@(*)` keeps its root mark when
/// the body's blocks are rebased (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and
/// variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)). Lens N-MRB's cell. r3 = PRE = iv = s2v. Cell `MRB`.
///
/// Raw lines: PRE `W t=5 y=1 | A end y=1`; iv `W t=5 y=1 | A end y=1`; s2v `W t=5 y=1 | A end
/// y=1`; vl `%Error-UNSUPPORTED: t.sv:6:11: Event control with implicit sensitivity (@*) in this
/// context; use an e... |                     ... For error description see
/// https://verilator.org/warn/UNSUPPORTED?v=5.052 | %Error: Exiting due to 1 error(s) | (verilator
/// refused)`.
#[test]
fn mrb_inbody_star_in_a_frame_task_keeps_its_root_mark() {
    pin(
        "module child; logic q; endmodule\n\
module top;\n\
  child u();\n\
  logic [3:0] a, y; logic w;\n\
  always @* w = u.q;\n\
  task t; @(*) y = a; $display(\"W t=%0t y=%0d\", $time, y); endtask\n\
  task automatic aw; t(); endtask\n\
  initial begin a = 0; aw(); end\n\
  initial begin #5 a = 1; #5 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1", "A end y=1"],
    );
}

/// N-ONEBRANCH (B2 decided, LRM b literal): a write in one branch is a write of the block (IEEE
/// 1800-2017 :9047-9050 "... read within the block or within any function called within the block
/// with the following exceptions: ... b) Any expression that is also written within the block or
/// within any function called within the block"); vl wakes. lens-diff r2 c2/P06. Cell
/// `P06_one_branch_write`.
///
/// Raw lines: PRE `C t=0 o=0 | C t=0 o=0 | C t=5 o=1 | A end o=1 g=1`; iv `(iverilog refused) |
/// t.sv:3: sorry: Reference ports not supported yet. | t.sv:4: warning: System task ($display)
/// cannot be synthesized in an always_comb process. | 1 error(s) during elaboration.`; s2v
/// `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `C t=0
/// o=0 | C t=0 o=0 | C t=5 o=1 | A end o=1 g=1`.
#[test]
fn p06_one_branch_write_covers() {
    pin(
        "module top;\n\
  logic [3:0] g, o;\n\
  task automatic t(ref logic [3:0] x); if (x == 4'hf) x = 0; o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0", "A end o=0 g=1"],
    );
}

/// N-STARCOMPOUND: the r1 compound rule (R2.7, IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding
/// all nets and variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments): the implicit lvalue read of `op=` is not part of `@*`)
/// holds in a static task body and at top level. lens-diff r2 c1/B1compound_in_stask. r3 = iv.
/// Cell `B1compound_in_stask`.
///
/// Raw lines: PRE `W t=3 y=2 | A end y=2`; iv `W t=5 y=3 | A end y=3`; s2v `W t=3 y=2 | A end
/// y=2`; vl `(verilator refused) | %Error-UNSUPPORTED: t.sv:3:13: Event control with implicit
/// sensitivity (@*) in this context; use an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn b1compound_star_compound_lvalue_in_a_static_task() {
    pin(
        "module top;\n\
  logic [3:0] a = 0, y = 0;\n\
  task t(); @* y += a; $display(\"W t=%0t y=%0d\", $time, y); endtask\n\
  initial t();\n\
  initial begin #3 y = 2; #2 a = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=3", "A end y=3"],
    );
}

/// N-STARCOMPOUND: the r1 compound rule (R2.7, IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding
/// all nets and variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments): the implicit lvalue read of `op=` is not part of `@*`)
/// holds in a static task body and at top level. lens-diff r2 c6/I06t. r3 = iv. Cell
/// `I06t_inbody_star_compound_toplevel`.
///
/// Raw lines: PRE `W t=3 y=2 | A end y=2`; iv `W t=5 y=3 | A end y=3`; s2v `W t=3 y=2 | A end
/// y=2`; vl `(verilator refused) | %Error-UNSUPPORTED: t.sv:3:17: Event control with implicit
/// sensitivity (@*) in this context; use an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn i06t_inbody_star_compound_lvalue_at_top_level() {
    pin(
        "module top;\n\
  logic [3:0] a = 0, y = 0;\n\
  initial begin @* y += a; $display(\"W t=%0t y=%0d\", $time, y); end\n\
  initial begin #3 y = 2; #2 a = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=3", "A end y=3"],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. The task writes x[1] of its ref formal
/// bound to g. r3 = PRE. Cell `RP1`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv
/// `(iverilog refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:4: sorry:
/// constant selects in always_* processes are not fully supported (the process will be sensitive
/// to all bits in 'g[3:0]'). | t.sv:4: warning: always_comb process has no sensitivities.`; s2v
/// `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `C t=0
/// y=0 g=0010 | A end y=0 g=0001`; local twin `RP1_local: C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C
/// t=5 y=1 g=0011 | A end y=1 g=0011`.
#[test]
fn rp1_ref_covers_only_the_bit_the_callee_writes() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  task automatic t(ref logic [3:0] x); x[1] = 1'b1; endtask\n\
  always_comb begin t(g); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 y=0 g=0010",
            "C t=0 y=0 g=0010",
            "C t=5 y=1 g=0011",
            "A end y=1 g=0011",
        ],
    );
}

/// Staged twin of `rp1_ref_covers_only_the_bit_the_callee_writes`: the settled read set reaches
/// the `.vu` / `.velab` flow (vcmp a; vcmp b; velab -L; vrun).
#[test]
fn rp1_staged_flow_matches_the_one_shot_run() {
    staged(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  task automatic t(ref logic [3:0] x); x[1] = 1'b1; endtask\n\
  always_comb begin t(g); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 y=0 g=0010",
            "C t=0 y=0 g=0010",
            "C t=5 y=1 g=0011",
            "A end y=1 g=0011",
        ],
    );
}

/// R3 (ENG-2 r3, OP-1 IEEE-literal): a system task / function output argument is a write (IEEE
/// 1800-2017 :9047-9050 "... read within the block or within any function called within the block
/// with the following exceptions: ... b) Any expression that is also written within the block or
/// within any function called within the block"); here it writes a `ref` formal, so the actual is
/// covered. `$sscanf` writes the ref formal. r3 = vl. Cell `RSYS`.
///
/// Raw lines: PRE `C t=0 o=9 g=9 | C t=0 o=9 g=9 | C t=5 o=9 g=9 | A end o=9 g=9`; iv `(iverilog
/// refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:4: warning: System task
/// ($display) cannot be synthesized in an always_comb process. | 1 error(s) during elaboration.`;
/// s2v `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `C
/// t=0 o=9 g=9 | A end o=9 g=1`; local twin `RSYS_tw: E3001:variable `g` is written by
/// `always_comb` AND by `.
#[test]
fn rsys_ref_written_by_sscanf_covers() {
    pin(
        "module top;\n\
  logic [3:0] g; logic [3:0] o; integer n;\n\
  task automatic t(ref logic [3:0] x); n = $sscanf(\"9\", \"%d\", x); o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d g=%0d\", $time, o, g); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=9 g=9", "A end o=9 g=1"],
    );
}

/// Staged twin of `rsys_ref_written_by_sscanf_covers`: the settled read set reaches the `.vu` /
/// `.velab` flow (vcmp a; vcmp b; velab -L; vrun).
#[test]
fn rsys_staged_flow_matches_the_one_shot_run() {
    staged(
        "module top;\n\
  logic [3:0] g; logic [3:0] o; integer n;\n\
  task automatic t(ref logic [3:0] x); n = $sscanf(\"9\", \"%d\", x); o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d g=%0d\", $time, o, g); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=9 g=9", "A end o=9 g=1"],
    );
}

/// R3 (ENG-2 r3, OP-1 IEEE-literal): a system task / function output argument is a write (IEEE
/// 1800-2017 :9047-9050 "... read within the block or within any function called within the block
/// with the following exceptions: ... b) Any expression that is also written within the block or
/// within any function called within the block"); here it writes a `ref` formal, so the actual is
/// covered. `$cast` (function form) writes the ref formal. r3 = vl. Cell `RCAST`.
///
/// Raw lines: PRE `C t=0 o=9 g=9 | C t=0 o=9 g=9 | C t=5 o=9 g=9 | A end o=9 g=9`; iv `(iverilog
/// refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:4: warning: System task
/// ($display) cannot be synthesized in an always_comb process. | 1 error(s) during elaboration.`;
/// s2v `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `C
/// t=0 o=9 g=9 | A end o=9 g=1`; local twin `RSYS_tw: E3001:variable `g` is written by
/// `always_comb` AND by `.
#[test]
fn rcast_ref_written_by_cast_function_covers() {
    pin(
        "module top;\n\
  logic [3:0] g; logic [3:0] o; int k;\n\
  task automatic t(ref logic [3:0] x); k = $cast(x, 4'd9); o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d g=%0d\", $time, o, g); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=9 g=9", "A end o=9 g=1"],
    );
}

/// R3 (ENG-2 r3, OP-1 IEEE-literal): a system task / function output argument is a write (IEEE
/// 1800-2017 :9047-9050 "... read within the block or within any function called within the block
/// with the following exceptions: ... b) Any expression that is also written within the block or
/// within any function called within the block"); here it writes a `ref` formal, so the actual is
/// covered. `$value$plusargs` writes the ref formal. r3 = vl. Cell `RPLUS`.
///
/// Raw lines: PRE `C t=0 o=0 g=0 | C t=0 o=0 g=0 | C t=5 o=1 g=1 | A end o=1 g=1`; iv `(iverilog
/// refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:4: warning: System task
/// ($display) cannot be synthesized in an always_comb process. | 1 error(s) during elaboration.`;
/// s2v `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `C
/// t=0 o=0 g=0 | A end o=0 g=1`; local twin `RSYS_tw: E3001:variable `g` is written by
/// `always_comb` AND by `.
#[test]
fn rplus_ref_written_by_value_plusargs_covers() {
    pin(
        "module top;\n\
  logic [3:0] g; logic [3:0] o; int k;\n\
  task automatic t(ref logic [3:0] x); k = $value$plusargs(\"N=%d\", x); o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d g=%0d\", $time, o, g); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0 g=0", "A end o=0 g=1"],
    );
}

/// R5 (ENG-2 r3): what a `ref` copy-out covers is computed in the settle, after the hierarchical
/// task enable installed its copy-outs (IEEE 1800-2017 :9047-9050 "... read within the block or
/// within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"); r1 fixed it at lowering and missed them. A static task passes its ref on to a
/// hierarchical task's output. r3 = vl. Cell `RINL`.
///
/// Raw lines: PRE `C t=0 o=9 g=9 | C t=0 o=9 g=9 | C t=5 o=9 g=9 | A end o=9 g=9`; iv `(iverilog
/// refused) | t.sv:7: sorry: Reference ports not supported yet. | t.sv:7: warning: user task (t)
/// must be automatic to be synthesized in an always_comb process. | t.sv:8: warning: System task
/// ($display) cannot be synthesized in an always_comb process.`; s2v `(sv2v/iverilog refused) |
/// t.sv:7:10: Parse error: unexpected token 'ref' (KW_ref)`; vl `C t=0 o=9 g=9 | A end o=9 g=1`.
#[test]
fn rinl_static_task_ref_passed_to_a_hier_output_covers() {
    pin(
        "module child;\n\
  task automatic tk(output logic [3:0] q); q = 4'd9; endtask\n\
endmodule\n\
module top;\n\
  child u();\n\
  logic [3:0] g; logic [3:0] o;\n\
  task t(ref logic [3:0] x); u.tk(x); o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d g=%0d\", $time, o, g); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=9 g=9", "A end o=9 g=1"],
    );
}

/// R4 (ENG-2 r3): an in-body `@(*)`'s W3056 ("reads no nets; it can never wake") is decided in the
/// settle from the settled set (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and
/// variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)): a hierarchical call's arguments count, a resolved
/// parameter does not. `@(*) u.w2(m, a);`: no false W3056, wakes at 5. Cell `KP_INB_CALLONLY`.
///
/// Raw lines: PRE `W3056: in-body @(*) reads no nets; it can never wake [in top]... | A end m0=x`;
/// iv `(iverilog refused) | t.sv:2: sorry: Subroutine ports with unpacked dimensions are not yet
/// supported. | 1 error(s) during elaboration.`; s2v `(sv2v/iverilog refused) | s2v.v:19: error:
/// 'm' is not a valid l-value for a procedural assignment. | s2v.v:13:      : 'm' is declared here
/// as a wire.`; vl `(verilator refused) | %Error-UNSUPPORTED: t.sv:7:5: Event control with
/// implicit sensitivity (@*) in this context; use an e... | %Error: Exiting due to 1 error(s)`;
/// local twin `KP_INB_CALLONLY_local: W t=5 m0=5 | A end m0=5`.
#[test]
fn kp_inb_callonly_hier_call_arguments_settle_without_w3056() {
    pin_diag(
        "module sub;\n\
  task automatic w2(output logic [3:0] o [2], input logic [3:0] x); o[0] = x; o[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u(); logic [3:0] m [2]; logic [3:0] a = 0, b = 0, y;\n\
  initial begin\n\
    @(*) u.w2(m, a);\n\
    $display(\"W t=%0t m0=%0d\", $time, m[0]);\n\
    y = b;\n\
  end\n\
  initial begin #3 b = 4'd1; #2 a = 4'd5; #2 $display(\"A end m0=%0d\", m[0]); $finish; end\n\
endmodule\n\
",
        &["W t=5 m0=5", "A end m0=5"],
        &[],
    );
}

/// Staged twin of `kp_inb_callonly_hier_call_arguments_settle_without_w3056`: the settled read set
/// reaches the `.vu` / `.velab` flow (vcmp a; vcmp b; velab -L; vrun).
#[test]
fn kp_inb_callonly_staged_flow_matches_the_one_shot_run() {
    staged(
        "module sub;\n\
  task automatic w2(output logic [3:0] o [2], input logic [3:0] x); o[0] = x; o[1] = 4'd2; endtask\n\
endmodule\n\
module top;\n\
  sub u(); logic [3:0] m [2]; logic [3:0] a = 0, b = 0, y;\n\
  initial begin\n\
    @(*) u.w2(m, a);\n\
    $display(\"W t=%0t m0=%0d\", $time, m[0]);\n\
    y = b;\n\
  end\n\
  initial begin #3 b = 4'd1; #2 a = 4'd5; #2 $display(\"A end m0=%0d\", m[0]); $finish; end\n\
endmodule\n\
",
        &["W t=5 m0=5", "A end m0=5"],
    );
}

/// R3 (ENG-2 r3, owner flag OP-1): a system output destination in an `always_comb` /
/// `always_latch` is written within the block (IEEE 1800-2017 :9047-9050 "... read within the
/// block or within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"), so it is not read — = vl; iv reads it (its deviation), s2v runs always_comb as always
/// @* (no exception b). Observable only beside a second writer (:9036 makes that illegal).
/// `aa.first(k)` writes k (no oracle runs it). Cell `LASSOC`.
///
/// Raw lines: PRE `C t=0 y=3 | C t=0 y=3 | C t=5 y=3 | A end y=3 k=3`; iv `(iverilog refused) |
/// t.sv:2: error: Type names are not valid expressions here. | t.sv:2: internal error: I do not
/// know how to elaborate this expression. | t.sv:2:               : Expression is: <type>`; s2v
/// `(sv2v/iverilog refused) | t.sv:2:14: Parse error: unexpected token ']' (Sym_brack_r)`; vl
/// `(verilator refused) | %Error: Circular logic when ordering code (non-cutable edge loop) |
/// %Error: Internal Error: ../V3Graph.cpp:211: Loops detected in graph:   VERTEX=0x7aa909c640\n
/// ALWAYS r1 c1`.
#[test]
fn lassoc_assoc_first_key_is_written() {
    pin(
        "module top;\n\
  int aa [int]; int k, y, n;\n\
  always_comb begin n = aa.first(k); y = k; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin aa[3] = 1; #5 k = 7; #2 $display(\"A end y=%0d k=%0d\", y, k); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=3", "C t=0 y=3", "A end y=3 k=7"],
    );
}

/// R3 (ENG-2 r3, owner flag OP-1): a system output destination in an `always_comb` /
/// `always_latch` is written within the block (IEEE 1800-2017 :9047-9050 "... read within the
/// block or within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"), so it is not read — = vl; iv reads it (its deviation), s2v runs always_comb as always
/// @* (no exception b). Observable only beside a second writer (:9036 makes that illegal).
/// `$cast(g, ..)` function form. r3 = vl. Cell `LCAST`.
///
/// Raw lines: PRE `C t=0 o=9 g=9 | C t=5 o=9 g=9 | A end o=9 g=9`; iv `t.sv:3: Error: System
/// task/function $cast() is not defined by any module. | iv.vvp: Program not runnable, 1 errors.`;
/// s2v `s2v.v:9: Error: System task/function $cast() is not defined by any module. | s2v.vvp:
/// Program not runnable, 1 errors.`; vl `C t=0 o=9 g=9 | A end o=9 g=1`.
#[test]
fn lcast_cast_destination_is_written() {
    pin(
        "module top;\n\
  logic [3:0] g; logic [3:0] o; int k;\n\
  always_comb begin k = $cast(g, 4'd9); o = g; $display(\"C t=%0t o=%0d g=%0d\", $time, o, g); end\n\
  initial begin #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=9 g=9", "A end o=9 g=1"],
    );
}

/// R3 (ENG-2 r3, owner flag OP-1): a system output destination in an `always_comb` /
/// `always_latch` is written within the block (IEEE 1800-2017 :9047-9050 "... read within the
/// block or within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"), so it is not read — = vl; iv reads it (its deviation), s2v runs always_comb as always
/// @* (no exception b). Observable only beside a second writer (:9036 makes that illegal).
/// `$cast(g, ..)` task form (the IR's argument is recorded). r3 = vl. Cell `LCASTT`.
///
/// Raw lines: PRE `C t=0 o=9 g=9 | C t=5 o=9 g=9 | A end o=9 g=9`; iv `t.sv:3: Error: System
/// task/function $cast() is not defined by any module. | iv.vvp: Program not runnable, 1 errors.`;
/// s2v `s2v.v:8: Error: System task/function $cast() is not defined by any module. | s2v.vvp:
/// Program not runnable, 1 errors.`; vl `C t=0 o=9 g=9 | A end o=9 g=1`.
#[test]
fn lcastt_cast_task_destination_is_written() {
    pin(
        "module top;\n\
  logic [3:0] g; logic [3:0] o;\n\
  always_comb begin $cast(g, 4'd9); o = g; $display(\"C t=%0t o=%0d g=%0d\", $time, o, g); end\n\
  initial begin #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=9 g=9", "A end o=9 g=1"],
    );
}

/// R3 (ENG-2 r3, owner flag OP-1): a system output destination in an `always_comb` /
/// `always_latch` is written within the block (IEEE 1800-2017 :9047-9050 "... read within the
/// block or within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"), so it is not read — = vl; iv reads it (its deviation), s2v runs always_comb as always
/// @* (no exception b). Observable only beside a second writer (:9036 makes that illegal).
/// `$value$plusargs` into g. r3 = vl. Cell `LPLUS`.
///
/// Raw lines: PRE `C t=0 o=0 g=0 | C t=0 o=0 g=0 | C t=5 o=1 g=1 | A end o=1 g=1`; iv `C t=0 o=0
/// g=0 | C t=0 o=0 g=0 | C t=5 o=1 g=1 | A end o=1 g=1`; s2v `C t=0 o=0 g=0 | C t=5 o=1 g=1 | A
/// end o=1 g=1`; vl `C t=0 o=0 g=0 | A end o=0 g=1`.
#[test]
fn lplus_value_plusargs_destination_is_written() {
    pin(
        "module top;\n\
  logic [3:0] g; logic [3:0] o; int k;\n\
  always_comb begin k = $value$plusargs(\"N=%d\", g); o = g; $display(\"C t=%0t o=%0d g=%0d\", $time, o, g); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0 g=0", "A end o=0 g=1"],
    );
}

/// R3 (ENG-2 r3, owner flag OP-1): a system output destination in an `always_comb` /
/// `always_latch` is written within the block (IEEE 1800-2017 :9047-9050 "... read within the
/// block or within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"), so it is not read — = vl; iv reads it (its deviation), s2v runs always_comb as always
/// @* (no exception b). Observable only beside a second writer (:9036 makes that illegal).
/// `$random(g)`: the seed is written. Cell `LRAND`.
///
/// Raw lines: PRE `C t=0 o=-2147414528 p=69070 | C t=0 o=-1671855048 p=475628535 | C t=5
/// o=-2147345408 p=138139 | A end p=138139 g=138139`; iv `C t=0 o=-2147414528 p=69070 | C t=0
/// o=-1671855048 p=475628535 | C t=5 o=-2147345408 p=138139 | A end p=138139 g=138139`; s2v `C t=0
/// o=-2147414528 p=69070 | C t=5 o=-2147345408 p=138139 | A end p=138139 g=138139`; vl `(verilator
/// refused) | %Error: Circular logic when ordering code (non-cutable edge loop) | %Error: Internal
/// Error: ../V3Graph.cpp:211: Loops detected in graph:   VERTEX=0x7970cb0e60\n ALWAYS r1 c1`.
#[test]
fn lrand_random_seed_is_written() {
    pin(
        "module top;\n\
  integer g, o, p;\n\
  always_comb begin o = $random(g); p = g; $display(\"C t=%0t o=%0d p=%0d\", $time, o, p); end\n\
  initial begin g = 1; #5 g = 2; #2 $display(\"A end p=%0d g=%0d\", p, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=-2147414528 p=69070", "A end p=69070 g=2"],
    );
}

/// R3 (ENG-2 r3, owner flag OP-1): a system output destination in an `always_comb` /
/// `always_latch` is written within the block (IEEE 1800-2017 :9047-9050 "... read within the
/// block or within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"), so it is not read — = vl; iv reads it (its deviation), s2v runs always_comb as always
/// @* (no exception b). Observable only beside a second writer (:9036 makes that illegal).
/// `$readmemh` into m. Cell `LRDM`.
///
/// Raw lines: PRE `C t=0 o=6 | C t=5 o=6 | A end o=6 m1=6`; iv `C t=0 o=6 | C t=5 o=6 | A end o=6
/// m1=6`; s2v `C t=0 o=6 | C t=5 o=6 | A end o=6 m1=6`; vl `C t=0 o=6 | A end o=6 m1=3`.
#[test]
fn lrdm_readmem_memory_is_written() {
    pin(
        "module top;\n\
  logic [7:0] m [0:1]; logic [7:0] o;\n\
  always_comb begin $readmemh(\"m.hex\", m); o = m[1]; $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 8'd3; #2 $display(\"A end o=%0d m1=%0d\", o, m[1]); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=x", "A end o=x m1=3"],
    );
}

/// R3 (ENG-2 r3, owner flag OP-1): a system output destination in an `always_comb` /
/// `always_latch` is written within the block (IEEE 1800-2017 :9047-9050 "... read within the
/// block or within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"), so it is not read — = vl; iv reads it (its deviation), s2v runs always_comb as always
/// @* (no exception b). Observable only beside a second writer (:9036 makes that illegal).
/// `$sformat` into g (the IR's argument is recorded). r3 = vl. Cell `LSFMT`.
///
/// Raw lines: PRE `C t=0 o=0037 g=0037 | C t=5 o=0037 g=0037 | A end o=0037 g=0037`; iv `C t=0
/// o=0037 g=0037 | C t=5 o=0037 g=0037 | A end o=0037 g=0037`; s2v `C t=0 o=0037 g=0037 | C t=5
/// o=0037 g=0037 | A end o=0037 g=0037`; vl `C t=0 o=0037 g=0037 | A end o=0037 g=0001`.
#[test]
fn lsfmt_sformat_destination_is_written() {
    pin(
        "module top;\n\
  logic [15:0] g; logic [15:0] o;\n\
  always_comb begin $sformat(g, \"%0d\", 7); o = g; $display(\"C t=%0t o=%h g=%h\", $time, o, g); end\n\
  initial begin #5 g = 1; #2 $display(\"A end o=%h g=%h\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0037 g=0037", "A end o=0037 g=0001"],
    );
}

/// R3 (ENG-2 r3, owner flag OP-1): a system output destination in an `always_comb` /
/// `always_latch` is written within the block (IEEE 1800-2017 :9047-9050 "... read within the
/// block or within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"), so it is not read — = vl; iv reads it (its deviation), s2v runs always_comb as always
/// @* (no exception b). Observable only beside a second writer (:9036 makes that illegal).
/// `$sscanf` into g. r3 = vl. Cell `LSYS`.
///
/// Raw lines: PRE `C t=0 o=9 g=9 | C t=5 o=9 g=9 | A end o=9 g=9`; iv `C t=0 o=9 g=9 | C t=5 o=9
/// g=9 | A end o=9 g=9`; s2v `C t=0 o=9 g=9 | C t=5 o=9 g=9 | A end o=9 g=9`; vl `C t=0 o=9 g=9 |
/// A end o=9 g=1`.
#[test]
fn lsys_sscanf_destination_is_written() {
    pin(
        "module top;\n\
  logic [3:0] g; logic [3:0] o; integer n;\n\
  always_comb begin n = $sscanf(\"9\", \"%d\", g); o = g; $display(\"C t=%0t o=%0d g=%0d\", $time, o, g); end\n\
  initial begin #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=9 g=9", "A end o=9 g=1"],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. An automatic void function writes x[1] of
/// its ref. r3 = PRE. Cell `RAVFNP`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv
/// `(iverilog refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:3: error:
/// Function top.f port x is not an input port. | t.sv:3:      : Function arguments must be input
/// ports.`; s2v `(sv2v/iverilog refused) | t.sv:3:29: Parse error: unexpected token 'ref'
/// (KW_ref)`; vl `C t=0 y=0 g=0010 | A end y=0 g=0001`; local twin `RP1_local: C t=0 y=0 g=0010 |
/// C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`.
#[test]
fn ravfnp_automatic_void_function_ref_bit_write_covers_that_bit() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  function automatic void f(ref logic [3:0] x); x[1] = 1'b1; endfunction\n\
  always_comb begin f(g); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 y=0 g=0010",
            "C t=0 y=0 g=0010",
            "C t=5 y=1 g=0011",
            "A end y=1 g=0011",
        ],
    );
}

/// R3 (ENG-2 r3, OP-1 IEEE-literal): a system task / function output argument is a write (IEEE
/// 1800-2017 :9047-9050 "... read within the block or within any function called within the block
/// with the following exceptions: ... b) Any expression that is also written within the block or
/// within any function called within the block"); here it writes a `ref` formal, so the actual is
/// covered. `$cast` (task form) writes the ref formal. r3 = vl. Cell `RCASTT`.
///
/// Raw lines: PRE `C t=0 o=9 g=9 | C t=0 o=9 g=9 | C t=5 o=9 g=9 | A end o=9 g=9`; iv `(iverilog
/// refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:3: warning: System task
/// ($cast) cannot be synthesized in an always_comb process. | t.sv:4: warning: System task
/// ($display) cannot be synthesized in an always_comb process.`; s2v `(sv2v/iverilog refused) |
/// t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `C t=0 o=9 g=9 | A end o=9 g=1`;
/// local twin `RSYS_tw: E3001:variable `g` is written by `always_comb` AND by `.
#[test]
fn rcastt_ref_written_by_cast_task_covers() {
    pin(
        "module top;\n\
  logic [3:0] g; logic [3:0] o;\n\
  task automatic t(ref logic [3:0] x); $cast(x, 4'd9); o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d g=%0d\", $time, o, g); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=9 g=9", "A end o=9 g=1"],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. x[1] written under `if (c)`. r3 = PRE.
/// Cell `RCOND`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv
/// `(iverilog refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:4: sorry:
/// constant selects in always_* processes are not fully supported (the process will be sensitive
/// to all bits in 'g[3:0]'). | t.sv:4: warning: always_comb process has no sensitivities.`; s2v
/// `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `C t=0
/// y=0 g=0010 | C t=5 y=1 g=0011 | C t=5 y=1 g=0011 | A end y=1 g=0011 | C t=7 y=1 g=0011 | C t=7
/// y=1 g=0011`; local twin `RCOND_tw: C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A
/// end y=1 g=0011`.
#[test]
fn rcond_ref_bit_written_under_if_covers_that_bit() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y, c;\n\
  task automatic t(ref logic [3:0] x); if (c) x[1] = 1'b1; endtask\n\
  always_comb begin t(g); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin c = 1; g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0 g=0010", "C t=0 y=0 g=0010", "C t=5 y=1 g=0011", "A end y=1 g=0011"],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. A non-packed (dynamic array) ref: an
/// element write covers nothing. r3 = PRE. Cell `RDYN`.
///
/// Raw lines: PRE `C t=0 y=0 q1=5 | C t=0 y=0 q1=5 | C t=5 y=3 q1=5 | A end y=3`; iv `(iverilog
/// refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:4: sorry: constant selects
/// in always_* processes are not fully supported (the process will be sensitive to all bits in
/// '+q'). | t.sv:4: sorry: constant selects in always_* processes are not fully supported (the
/// process will be sensitive to all bits in '+q').`; s2v `(sv2v/iverilog refused) | sv2v:
/// t.sv:2:9: Parse error: expected comma or end of declarations | CallStack (from HasCallStack):`;
/// vl `C t=0 y=0 q1=0 | A end y=0`; local twin `RDYN_tw: C t=0 y=0 q1=5 | C t=0 y=0 q1=5 | C t=5
/// y=3 q1=5 | A end y=3`.
#[test]
fn rdyn_ref_dynamic_array_element_write_covers_nothing() {
    pin(
        "module top;\n\
  int q []; int y;\n\
  task automatic t(ref int x []); x[1] = 5; endtask\n\
  always_comb begin t(q); y = q[0]; $display(\"C t=%0t y=%0d q1=%0d\", $time, y, q[1]); end\n\
  initial begin q = new[2]; #5 q[0] = 3; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 y=0 q1=5",
            "C t=0 y=0 q1=5",
            "C t=5 y=3 q1=5",
            "A end y=3",
        ],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. The ref is bound to m[1]; the callee
/// writes x[2]. r3 = PRE. Cell `RELEMP`.
///
/// Raw lines: PRE `C t=0 y=0 m1=0100 | C t=0 y=0 m1=0100 | C t=5 y=1 m1=0101 | A end y=1 m1=0101`;
/// iv `(iverilog refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:4: sorry:
/// constant selects in always_* processes are not fully supported (the process will be sensitive
/// to all bits in 'm[word='sd1][3: | t.sv:4: warning: always_comb process has no sensitivities.`;
/// s2v `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `C
/// t=0 y=0 m1=0100 | A end y=0 m1=0001`; local twin `RELEMP_tw: C t=0 y=0 m1=0100 | C t=0 y=0
/// m1=0100 | C t=5 y=1 m1=0101 | A end y=1 m1=0101`.
#[test]
fn relemp_ref_bound_to_an_element_covers_the_mapped_bit() {
    pin(
        "module top;\n\
  logic [3:0] m [0:1]; logic y;\n\
  task automatic t(ref logic [3:0] x); x[2] = 1'b1; endtask\n\
  always_comb begin t(m[1]); y = m[1][0]; $display(\"C t=%0t y=%0d m1=%b\", $time, y, m[1]); end\n\
  initial begin m[0] = 0; m[1] = 0; #5 m[1][0] = 1'b1; #2 $display(\"A end y=%0d m1=%b\", y, m[1]); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0 m1=0100", "C t=0 y=0 m1=0100", "C t=5 y=1 m1=0101", "A end y=1 m1=0101"],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. An automatic function writes x[1] of its
/// ref. r3 = PRE. Cell `RFNPART`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv
/// `(iverilog refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:3: error:
/// Function top.f port x is not an input port. | t.sv:3:      : Function arguments must be input
/// ports.`; s2v `(sv2v/iverilog refused) | t.sv:3:30: Parse error: unexpected token 'ref'
/// (KW_ref)`; vl `C t=0 y=0 g=0010 | A end y=0 g=0001`; local twin `RP1_local: C t=0 y=0 g=0010 |
/// C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`.
#[test]
fn rfnpart_automatic_function_ref_bit_write_covers_that_bit() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  function automatic logic f(ref logic [3:0] x); x[1] = 1'b1; return x[0]; endfunction\n\
  always_comb begin y = f(g); $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 y=0 g=0010",
            "C t=0 y=0 g=0010",
            "C t=5 y=1 g=0011",
            "A end y=1 g=0011",
        ],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. A hierarchical callee `u.tk(g)` writes
/// z[1] of its ref. r3 = PRE. Cell `RHCALLEE`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv
/// `(iverilog refused) | t.sv:2: sorry: Reference ports not supported yet. | t.sv:6: sorry:
/// constant selects in always_* processes are not fully supported (the process will be sensitive
/// to all bits in 'g[3:0]'). | t.sv:6: warning: always_comb process has no sensitivities.`; s2v
/// `(sv2v/iverilog refused) | t.sv:2:21: Parse error: unexpected token 'ref' (KW_ref)`; vl `C t=0
/// y=0 g=0010 | A end y=0 g=0001`; local twin `RP1_local: C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C
/// t=5 y=1 g=0011 | A end y=1 g=0011`.
#[test]
fn rhcallee_hier_callee_ref_bit_write_covers_that_bit() {
    pin(
        "module child;\n\
  task automatic tk(ref logic [3:0] z); z[1] = 1'b1; endtask\n\
endmodule\n\
module top;\n\
  child u(); logic [3:0] g; logic y;\n\
  always_comb begin u.tk(g); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 y=0 g=0010",
            "C t=0 y=0 g=0010",
            "C t=5 y=1 g=0011",
            "A end y=1 g=0011",
        ],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. The ref is passed on to a ref that writes
/// z[1]. r3 = PRE. Cell `RNEST`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv
/// `(iverilog refused) | t.sv:4: sorry: Reference ports not supported yet. | t.sv:3: sorry:
/// Reference ports not supported yet. | t.sv:5: sorry: constant selects in always_* processes are
/// not fully supported (the process will be sensitive to all bits in 'g[3:0]').`; s2v
/// `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `C t=0
/// y=0 g=0010 | A end y=0 g=0001`; local twin `RP1_local: C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C
/// t=5 y=1 g=0011 | A end y=1 g=0011`.
#[test]
fn rnest_ref_passed_on_to_a_bit_writer_covers_that_bit() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  task automatic w(ref logic [3:0] z); z[1] = 1'b1; endtask\n\
  task automatic t(ref logic [3:0] x); w(x); endtask\n\
  always_comb begin t(g); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 y=0 g=0010",
            "C t=0 y=0 g=0010",
            "C t=5 y=1 g=0011",
            "A end y=1 g=0011",
        ],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. The callee writes x[i] (non-constant:
/// covers nothing, C3). r3 = PRE. Cell `RNONC`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv
/// `(iverilog refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:4: sorry:
/// constant selects in always_* processes are not fully supported (the process will be sensitive
/// to all bits in 'g[3:0]'). | t.sv:4: warning: System task ($display) cannot be synthesized in an
/// always_comb process.`; s2v `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token
/// 'ref' (KW_ref)`; vl `C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | C t=5 y=1 g=0011 | A end y=1 g=0011
/// | C t=7 y=1 g=0011 | C t=7 y=1 g=0011`; local twin `RNONC_tw: C t=0 y=0 g=0010 | C t=0 y=0
/// g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`.
#[test]
fn rnonc_ref_non_constant_index_write_covers_nothing() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y; int k;\n\
  task automatic t(ref logic [3:0] x, input int i); x[i] = 1'b1; endtask\n\
  always_comb begin t(g, k); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin k = 1; g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0 g=0010", "C t=0 y=0 g=0010", "C t=5 y=1 g=0011", "A end y=1 g=0011"],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. The always_latch twin of RP1. r3 = PRE.
/// Cell `RP1L`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv
/// `(iverilog refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:4: sorry:
/// constant selects in always_* processes are not fully supported (the process will be sensitive
/// to all bits in 'g[3:0]'). | t.sv:4: error: always_latch process has no event control.`; s2v
/// `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `C t=0
/// y=0 g=0010 | A end y=0 g=0001`; local twin `RP1L_tw: C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C
/// t=5 y=1 g=0011 | A end y=1 g=0011`.
#[test]
fn rp1l_latch_ref_bit_write_covers_that_bit() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  task automatic t(ref logic [3:0] x); x[1] = 1'b1; endtask\n\
  always_latch begin t(g); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 y=0 g=0010",
            "C t=0 y=0 g=0010",
            "C t=5 y=1 g=0011",
            "A end y=1 g=0011",
        ],
    );
}

/// R3 (ENG-2 r3, OP-1 IEEE-literal): a system task / function output argument is a write (IEEE
/// 1800-2017 :9047-9050 "... read within the block or within any function called within the block
/// with the following exceptions: ... b) Any expression that is also written within the block or
/// within any function called within the block"); here it writes a `ref` formal, so the actual is
/// covered. The ref is a `$random` seed. Cell `RRAND`.
///
/// Raw lines: PRE `C t=0 o=-2147414528 g=69070 | C t=0 o=-1671855048 g=475628535 | C t=5
/// o=-2147345408 g=138139 | A end o=-2147345408 g=138139`; iv `(iverilog refused) | t.sv:3: sorry:
/// Reference ports not supported yet. | t.sv:4: warning: System task ($display) cannot be
/// synthesized in an always_comb process. | 1 error(s) during elaboration.`; s2v `(sv2v/iverilog
/// refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `(verilator refused) |
/// %Error: Circular logic when ordering code (non-cutable edge loop) | %Error: Internal Error:
/// ../V3Graph.cpp:211: Loops detected in graph:   VERTEX=0x7c26cd0f00\n ALWAYS r1 c1`; local twin
/// `RSYS_tw: E3001:variable `g` is written by `always_comb` AND by `.
#[test]
fn rrand_ref_random_seed_covers() {
    pin(
        "module top;\n\
  integer g, o;\n\
  task automatic t(ref integer s); o = $random(s); endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d g=%0d\", $time, o, g); end\n\
  initial begin g = 1; #5 g = 2; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=-2147414528 g=69070", "A end o=-2147414528 g=2"],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. A recursive task passes its ref on and
/// writes x[1] (least fixed point). r3 = PRE. Cell `RREC`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv
/// `(iverilog refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:4: sorry:
/// constant selects in always_* processes are not fully supported (the process will be sensitive
/// to all bits in 'g[3:0]'). | sh: line 1: 76069 Done
/// /opt/homebrew/Cellar/icarus-verilog/13.0/lib/ivl/ivlpp -L
/// -F"/var/folders/pm/fxr_mw3x7sj0dkh9qqhsv`; s2v `(sv2v/iverilog refused) | t.sv:3:20: Parse
/// error: unexpected token 'ref' (KW_ref)`; vl `(verilator refused) | %Error: Circular logic when
/// ordering code (non-cutable edge loop) | %Error: Internal Error: ../V3Graph.cpp:211: Loops
/// detected in graph:   VERTEX=0x75b308cf00\n ALWAYS r1 c1`; local twin `RP1_local: C t=0 y=0
/// g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`.
#[test]
fn rrec_recursive_task_ref_bit_write_covers_that_bit() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  task automatic t(ref logic [3:0] x, input int n); if (n > 0) t(x, n - 1); else x[1] = 1'b1; endtask\n\
  always_comb begin t(g, 2); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0 g=0010", "C t=0 y=0 g=0010", "C t=5 y=1 g=0011", "A end y=1 g=0011"],
    );
}

/// R3 (ENG-2 r3, OP-1 IEEE-literal): a system task / function output argument is a write (IEEE
/// 1800-2017 :9047-9050 "... read within the block or within any function called within the block
/// with the following exceptions: ... b) Any expression that is also written within the block or
/// within any function called within the block"); here it writes a `ref` formal, so the actual is
/// covered. `$sformat` writes the ref formal. r3 = vl. Cell `RSFMT`.
///
/// Raw lines: PRE `C t=0 o=0037 g=0037 | C t=0 o=0037 g=0037 | C t=5 o=0037 g=0037 | A end o=0037
/// g=0037`; iv `(iverilog refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:3:
/// warning: System task ($sformat) cannot be synthesized in an always_comb process. | t.sv:4:
/// warning: System task ($display) cannot be synthesized in an always_comb process.`; s2v
/// `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `C t=0
/// o=0037 g=0037 | A end o=0037 g=0001`; local twin `RSYS_tw: E3001:variable `g` is written by
/// `always_comb` AND by `.
#[test]
fn rsfmt_ref_written_by_sformat_covers() {
    pin(
        "module top;\n\
  logic [15:0] g; logic [15:0] o;\n\
  task automatic t(ref logic [15:0] x); $sformat(x, \"%0d\", 7); o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%h g=%h\", $time, o, g); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%h g=%h\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0037 g=0037", "A end o=0037 g=0001"],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. A static function's ref (vita's
/// extension: :14778 makes ref on a static subroutine illegal) writes x[1]. r3 = PRE. Cell
/// `RSFNP`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv
/// `(iverilog refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:3: error:
/// Function top.f port x is not an input port. | t.sv:3:      : Function arguments must be input
/// ports.`; s2v `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token 'ref'
/// (KW_ref)`; vl `C t=0 y=0 g=0010 | A end y=0 g=0001`; local twin `RP1_local: C t=0 y=0 g=0010 |
/// C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`.
#[test]
fn rsfnp_static_function_ref_bit_write_covers_that_bit() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  function logic f(ref logic [3:0] x); x[1] = 1'b1; return x[0]; endfunction\n\
  always_comb begin y = f(g); $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 y=0 g=0010",
            "C t=0 y=0 g=0010",
            "C t=5 y=1 g=0011",
            "A end y=1 g=0011",
        ],
    );
}

/// R3 (ENG-2 r3, OP-1 IEEE-literal): a system task / function output argument is a write (IEEE
/// 1800-2017 :9047-9050 "... read within the block or within any function called within the block
/// with the following exceptions: ... b) Any expression that is also written within the block or
/// within any function called within the block"); here it writes a `ref` formal, so the actual is
/// covered. The always_latch twin of RSYS. r3 = vl. Cell `RSYSL`.
///
/// Raw lines: PRE `C t=0 y=1 g=9 | C t=0 y=1 g=9 | C t=5 y=1 g=9 | A end y=1 g=9`; iv `(iverilog
/// refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:4: sorry: constant selects
/// in always_* processes are not fully supported (the process will be sensitive to all bits in
/// 'g[3:0]'). | t.sv:4: error: always_latch process has no event control.`; s2v `(sv2v/iverilog
/// refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `C t=0 y=1 g=9 | A end
/// y=1 g=2`; local twin `RSYS_tw: E3001:variable `g` is written by `always_comb` AND by `.
#[test]
fn rsysl_latch_ref_written_by_sscanf_covers() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y; integer n;\n\
  task automatic t(ref logic [3:0] x); n = $sscanf(\"9\", \"%d\", x); endtask\n\
  always_latch begin t(g); y = g[0]; $display(\"C t=%0t y=%0d g=%0d\", $time, y, g); end\n\
  initial begin g = 0; #5 g = 2; #2 $display(\"A end y=%0d g=%0d\", y, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=1 g=9", "A end y=1 g=2"],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. Two refs bound to g; the callee writes
/// x[1] and z[2] (the value is vita's copy-out alias, Q-REFALIAS). r3 = PRE. Cell `RTWO`.
///
/// Raw lines: PRE `C t=0 y=0 g=0100 | C t=0 y=0 g=0100 | C t=5 y=1 g=0101 | A end y=1 g=0101`; iv
/// `(iverilog refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:3: sorry:
/// Reference ports not supported yet. | t.sv:4: sorry: constant selects in always_* processes are
/// not fully supported (the process will be sensitive to all bits in 'g[3:0]').`; s2v
/// `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `C t=0
/// y=0 g=0110 | A end y=0 g=0001`; local twin `RTWO_tw: C t=0 y=0 g=0110 | C t=0 y=0 g=0110 | C
/// t=5 y=1 g=0111 | A end y=1 g=0111`.
#[test]
fn rtwo_two_refs_on_one_variable_cover_their_bits() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  task automatic t(ref logic [3:0] x, ref logic [3:0] z); x[1] = 1'b1; z[2] = 1'b1; endtask\n\
  always_comb begin t(g, g); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 y=0 g=0100",
            "C t=0 y=0 g=0100",
            "C t=5 y=1 g=0101",
            "A end y=1 g=0101",
        ],
    );
}

/// R4 (ENG-2 r3): an in-body `@(*)`'s W3056 ("reads no nets; it can never wake") is decided in the
/// settle from the settled set (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and
/// variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)): a hierarchical call's arguments count, a resolved
/// parameter does not. `@(*) y = u.P`: the settled set is empty. r3 = PRE = iv = s2v. Cell `WHP`.
///
/// Raw lines: PRE `A end y=x`; iv `A end y=x`; s2v `A end y=x`; vl `(verilator refused) |
/// %Error-UNSUPPORTED: t.sv:4:17: Event control with implicit sensitivity (@*) in this context;
/// use an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn whp_inbody_star_on_a_hier_parameter_warns_w3056() {
    pin_diag(
        "module child #(parameter int P = 3) (); endmodule\n\
module top;\n\
  child u(); logic [3:0] y;\n\
  initial begin @(*) y = u.P; $display(\"W t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["A end y=x"],
        &["W3056"],
    );
}

/// R4 (ENG-2 r3): an in-body `@(*)`'s W3056 ("reads no nets; it can never wake") is decided in the
/// settle from the settled set (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and
/// variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)): a hierarchical call's arguments count, a resolved
/// parameter does not. `@(*) u.w2(m, a);` with a scalar output. r3 = iv. Cell `WHT`.
///
/// Raw lines: PRE `W3056: in-body @(*) reads no nets; it can never wake [in top]... | A end m=x`;
/// iv `W t=5 m=5 | A end m=5`; s2v `(sv2v/iverilog refused) | s2v.v:14: error: 'm' is not a valid
/// l-value for a procedural assignment. | s2v.v:10:      : 'm' is declared here as a wire.`; vl
/// `(verilator refused) | %Error-UNSUPPORTED: t.sv:6:17: Event control with implicit sensitivity
/// (@*) in this context; use an e... | %Error: Exiting due to 1 error(s)`.
#[test]
fn wht_inbody_star_hier_task_scalar_output_settles_without_w3056() {
    pin_diag(
        "module sub;\n\
  task automatic w2(output logic [3:0] o, input logic [3:0] x); o = x; endtask\n\
endmodule\n\
module top;\n\
  sub u(); logic [3:0] m; logic [3:0] a = 0;\n\
  initial begin @(*) u.w2(m, a); $display(\"W t=%0t m=%0d\", $time, m); end\n\
  initial begin #5 a = 4'd5; #2 $display(\"A end m=%0d\", m); $finish; end\n\
endmodule\n\
",
        &["W t=5 m=5", "A end m=5"],
        &[],
    );
}

/// R3 (ENG-2 r3, OP-1 IEEE-literal): a system task / function output argument is a write (IEEE
/// 1800-2017 :9047-9050 "... read within the block or within any function called within the block
/// with the following exceptions: ... b) Any expression that is also written within the block or
/// within any function called within the block"); here it writes a `ref` formal, so the actual is
/// covered. `void'($sscanf(..))` into a ref (automatic task). r3 = vl. Cell `P10`.
///
/// Raw lines: PRE `C t=0 o=9 | C t=0 o=9 | C t=5 o=9 | A end o=9 g=9`; iv `(iverilog refused) |
/// t.sv:3: sorry: Reference ports not supported yet. | t.sv:4: warning: System task ($display)
/// cannot be synthesized in an always_comb process. | 1 error(s) during elaboration.`; s2v
/// `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `C t=0
/// o=9 | A end o=9 g=1`; local twin `RSYS_tw: E3001:variable `g` is written by `always_comb` AND
/// by `.
#[test]
fn p10_void_sscanf_into_a_ref_covers() {
    pin(
        "module top;\n\
  logic [3:0] g, o;\n\
  task automatic t(ref logic [3:0] x); void'($sscanf(\"9\", \"%d\", x)); o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=9", "A end o=9 g=1"],
    );
}

/// R3 (ENG-2 r3, OP-1 IEEE-literal): a system task / function output argument is a write (IEEE
/// 1800-2017 :9047-9050 "... read within the block or within any function called within the block
/// with the following exceptions: ... b) Any expression that is also written within the block or
/// within any function called within the block"); here it writes a `ref` formal, so the actual is
/// covered. P10 in a static task (the inline path). r3 = vl. Cell `P10c`.
///
/// Raw lines: PRE `C t=0 o=9 | C t=0 o=9 | C t=5 o=9 | A end o=9 g=9`; iv `(iverilog refused) |
/// t.sv:3: sorry: Reference ports not supported yet. | t.sv:3: warning: user task (t) must be
/// automatic to be synthesized in an always_comb process. | t.sv:4: warning: System task
/// ($display) cannot be synthesized in an always_comb process.`; s2v `(sv2v/iverilog refused) |
/// t.sv:3:10: Parse error: unexpected token 'ref' (KW_ref)`; vl `C t=0 o=9 | A end o=9 g=1`; local
/// twin `RSYS_tw: E3001:variable `g` is written by `always_comb` AND by `.
#[test]
fn p10c_void_sscanf_into_a_static_task_ref_covers() {
    pin(
        "module top;\n\
  logic [3:0] g, o;\n\
  task t(ref logic [3:0] x); void'($sscanf(\"9\", \"%d\", x)); o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=9", "A end o=9 g=1"],
    );
}

/// R5 (ENG-2 r3): what a `ref` copy-out covers is computed in the settle, after the hierarchical
/// task enable installed its copy-outs (IEEE 1800-2017 :9047-9050 "... read within the block or
/// within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"); r1 fixed it at lowering and missed them. A static task passes its ref to a
/// hierarchical task's output. r3 = vl. Cell `P08b`.
///
/// Raw lines: PRE `C t=0 o=9 | C t=0 o=9 | C t=5 o=9 | A end o=9 g=9`; iv `(iverilog refused) |
/// t.sv:7: sorry: Reference ports not supported yet. | t.sv:7: warning: user task (t) must be
/// automatic to be synthesized in an always_comb process. | t.sv:8: warning: System task
/// ($display) cannot be synthesized in an always_comb process.`; s2v `(sv2v/iverilog refused) |
/// t.sv:7:10: Parse error: unexpected token 'ref' (KW_ref)`; vl `C t=0 o=9 | A end o=9 g=1`.
#[test]
fn p08b_static_task_ref_passed_to_a_hier_output_covers() {
    pin(
        "module sub;\n\
  task automatic w(output logic [3:0] z); z = 4'd9; endtask\n\
endmodule\n\
module top;\n\
  sub u();\n\
  logic [3:0] g, o;\n\
  task t(ref logic [3:0] x); u.w(x); o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=9", "A end o=9 g=1"],
    );
}

/// Local twin (ENG-2 r3 R1's anchor): the callee's write spelled as an assignment in the block —
/// exception b at variable granularity (IEEE 1800-2017 :9047-9050 "... read within the block or
/// within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"). RP1's twin. r3 = PRE. Cell `RP1_local`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv
/// `C t=0 y=0 g=0010 | A end y=0 g=0011`; s2v `C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1
/// g=0011`; vl `C t=0 y=0 g=0010 | A end y=0 g=0001`.
#[test]
fn rp1_local_twin_bit_write_covers_that_bit() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  always_comb begin g[1] = 1'b1; y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 y=0 g=0010",
            "C t=0 y=0 g=0010",
            "C t=5 y=1 g=0011",
            "A end y=1 g=0011",
        ],
    );
}

/// Local twin (ENG-2 r3 R1's anchor): the callee's write spelled as an assignment in the block —
/// exception b at variable granularity (IEEE 1800-2017 :9047-9050 "... read within the block or
/// within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"). RCOND's twin. r3 = PRE. Cell `RCOND_tw`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv
/// `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | A end y=0 g=0011`; s2v `C t=0 y=0 g=0010 | C t=5 y=1
/// g=0011 | A end y=1 g=0011`; vl `C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | C t=5 y=1 g=0011 | A end
/// y=1 g=0011 | C t=7 y=1 g=0011 | C t=7 y=1 g=0011`.
#[test]
fn rcond_local_twin() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y, c;\n\
  always_comb begin if (c) g[1] = 1'b1; y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin c = 1; g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0 g=0010", "C t=0 y=0 g=0010", "C t=5 y=1 g=0011", "A end y=1 g=0011"],
    );
}

/// Local twin (ENG-2 r3 R1's anchor): the callee's write spelled as an assignment in the block —
/// exception b at variable granularity (IEEE 1800-2017 :9047-9050 "... read within the block or
/// within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"). RNONC's twin. r3 = PRE. Cell `RNONC_tw`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv
/// `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | A end y=0 g=0011`; s2v `C t=0 y=0 g=0010 | C t=5 y=1
/// g=0011 | A end y=1 g=0011`; vl `C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | C t=5 y=1 g=0011 | A end
/// y=1 g=0011 | C t=7 y=1 g=0011 | C t=7 y=1 g=0011`.
#[test]
fn rnonc_local_twin() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y; int k;\n\
  always_comb begin g[k] = 1'b1; y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin k = 1; g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0 g=0010", "C t=0 y=0 g=0010", "C t=5 y=1 g=0011", "A end y=1 g=0011"],
    );
}

/// Local twin (ENG-2 r3 R1's anchor): the callee's write spelled as an assignment in the block —
/// exception b at variable granularity (IEEE 1800-2017 :9047-9050 "... read within the block or
/// within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"). RELEMP's twin. r3 = PRE. Cell `RELEMP_tw`.
///
/// Raw lines: PRE `C t=0 y=0 m1=0100 | C t=0 y=0 m1=0100 | C t=5 y=1 m1=0101 | A end y=1 m1=0101`;
/// iv `C t=0 y=0 m1=0100 | A end y=0 m1=0101`; s2v `C t=0 y=0 m1=0100 | C t=5 y=1 m1=0101 | A end
/// y=1 m1=0101`; vl `C t=0 y=0 m1=0100 | A end y=0 m1=0001`.
#[test]
fn relemp_local_twin() {
    pin(
        "module top;\n\
  logic [3:0] m [0:1]; logic y;\n\
  always_comb begin m[1][2] = 1'b1; y = m[1][0]; $display(\"C t=%0t y=%0d m1=%b\", $time, y, m[1]); end\n\
  initial begin m[0] = 0; m[1] = 0; #5 m[1][0] = 1'b1; #2 $display(\"A end y=%0d m1=%b\", y, m[1]); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0 m1=0100", "C t=0 y=0 m1=0100", "C t=5 y=1 m1=0101", "A end y=1 m1=0101"],
    );
}

/// Local twin (ENG-2 r3 R1's anchor): the callee's write spelled as an assignment in the block —
/// exception b at variable granularity (IEEE 1800-2017 :9047-9050 "... read within the block or
/// within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"). RTWO's twin (by-reference value g=0110). r3 = PRE. Cell `RTWO_tw`.
///
/// Raw lines: PRE `C t=0 y=0 g=0110 | C t=0 y=0 g=0110 | C t=5 y=1 g=0111 | A end y=1 g=0111`; iv
/// `C t=0 y=0 g=0110 | A end y=0 g=0111`; s2v `C t=0 y=0 g=0110 | C t=5 y=1 g=0111 | A end y=1
/// g=0111`; vl `C t=0 y=0 g=0110 | A end y=0 g=0001`.
#[test]
fn rtwo_local_twin() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  always_comb begin g[1] = 1'b1; g[2] = 1'b1; y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0 g=0110", "C t=0 y=0 g=0110", "C t=5 y=1 g=0111", "A end y=1 g=0111"],
    );
}

/// Local twin (ENG-2 r3 R1's anchor): the callee's write spelled as an assignment in the block —
/// exception b at variable granularity (IEEE 1800-2017 :9047-9050 "... read within the block or
/// within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"). RP1L's twin. r3 = PRE. Cell `RP1L_tw`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv
/// `(iverilog refused) | t.sv:3: sorry: constant selects in always_* processes are not fully
/// supported (the process will be sensitive to all bits in 'g[3:0]'). | t.sv:3: error:
/// always_latch process has no event control. | t.sv:3: warning: System task ($display) cannot be
/// synthesized in an always_latch process.`; s2v `C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1
/// g=0011`; vl `C t=0 y=0 g=0010 | A end y=0 g=0001`.
#[test]
fn rp1l_local_twin() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  always_latch begin g[1] = 1'b1; y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 y=0 g=0010",
            "C t=0 y=0 g=0010",
            "C t=5 y=1 g=0011",
            "A end y=1 g=0011",
        ],
    );
}

/// Local twin (ENG-2 r3 R1's anchor): the callee's write spelled as an assignment in the block —
/// exception b at variable granularity (IEEE 1800-2017 :9047-9050 "... read within the block or
/// within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"). R10's twin. r3 = PRE = iv. Cell `R10_tw`.
///
/// Raw lines: PRE `C t=0 o=0 m0=7 | C t=0 o=0 m0=7 | C t=5 o=3 m0=7 | A end o=3`; iv `C t=0 o=0
/// m0=7 | C t=0 o=0 m0=7 | C t=5 o=3 m0=7 | A end o=3`; s2v `C t=0 o=0 m0=7 | C t=5 o=3 m0=7 | A
/// end o=3`; vl `C t=0 o=0 m0=7 | A end o=0`.
#[test]
fn r10_local_twin() {
    pin(
        "module top;\n\
  logic [7:0] m [0:1]; logic [7:0] o;\n\
  always_comb begin m[0] = 8'd7; o = m[1]; $display(\"C t=%0t o=%0d m0=%0d\", $time, o, m[0]); end\n\
  initial begin m[0] = 0; m[1] = 0; #5 m[1] = 3; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=0 m0=7", "C t=0 o=0 m0=7", "C t=5 o=3 m0=7", "A end o=3"],
    );
}

/// Local twin (ENG-2 r3 R1's anchor): the callee's write spelled as an assignment in the block —
/// exception b at variable granularity (IEEE 1800-2017 :9047-9050 "... read within the block or
/// within any function called within the block with the following exceptions: ... b) Any
/// expression that is also written within the block or within any function called within the
/// block"). RDYN's twin. r3 = PRE. Cell `RDYN_tw`.
///
/// Raw lines: PRE `C t=0 y=0 q1=5 | C t=0 y=0 q1=5 | C t=5 y=3 q1=5 | A end y=3`; iv `(iverilog
/// refused) | t.sv:2: internal error: pin(1) out of bounds(1) | t.sv:2:               :
/// typeid=6NetNet | t.sv:2: assert: netlist.cc:202: failed assertion idx < npins_`; s2v
/// `(sv2v/iverilog refused) | sv2v: t.sv:2:9: Parse error: expected comma or end of declarations |
/// CallStack (from HasCallStack):`; vl `C t=0 y=0 q1=0 | A end y=0`.
#[test]
fn rdyn_local_twin() {
    pin(
        "module top;\n\
  int q []; int y;\n\
  always_comb begin q[1] = 5; y = q[0]; $display(\"C t=%0t y=%0d q1=%0d\", $time, y, q[1]); end\n\
  initial begin q = new[2]; #5 q[0] = 3; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 y=0 q1=5",
            "C t=0 y=0 q1=5",
            "C t=5 y=3 q1=5",
            "A end y=3",
        ],
    );
}

/// R2 / OP2 (ENG-2 r3, unchanged): an `output` / `inout` actual is written whole by its copy-out
/// on every path (§13.5 :14750-14751 (output / inout arguments are copied out when the subroutine
/// returns)), so it covers the whole actual whatever the callee writes; the pin cites the LRM, not
/// iv (iv covers an output only when the task writes it, OP-7). An inout formal, x[1] written. r3
/// = iv. Cell `RP1o`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv
/// `C t=0 y=0 g=0010 | A end y=0 g=0011`; s2v `C t=0 y=x g=xx1x | C t=5 y=x g=xx1x | A end y=x
/// g=xx1x`; vl `(verilator refused) | %Error: Internal Error: t.sv:4:23:
/// ../V3DfgSynthesize.cpp:358: Non-ReadOnly reference`.
#[test]
fn rp1o_control_inout_actual_covers_whole() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  task automatic t(inout logic [3:0] x); x[1] = 1'b1; endtask\n\
  always_comb begin t(g); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0 g=0010", "A end y=0 g=0011"],
    );
}

/// R2 / OP2 (ENG-2 r3, unchanged): an `output` / `inout` actual is written whole by its copy-out
/// on every path (§13.5 :14750-14751 (output / inout arguments are copied out when the subroutine
/// returns)), so it covers the whole actual whatever the callee writes; the pin cites the LRM, not
/// iv (iv covers an output only when the task writes it, OP-7). An output formal bound to g. r3 =
/// iv = vl. Cell `RP1_out`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=5 y=0 g=0010 | A end y=0 g=0010`; iv `C t=0 y=0 g=0010 |
/// A end y=0 g=0011`; s2v `C t=0 y=0 g=0010 | C t=5 y=0 g=0010 | A end y=0 g=0010`; vl `C t=0 y=0
/// g=0010 | A end y=0 g=0011`.
#[test]
fn rp1_out_control_output_actual_covers_whole() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  task automatic t(output logic [3:0] x); x = 4'b0010; endtask\n\
  always_comb begin t(g); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0 g=0010", "A end y=0 g=0011"],
    );
}

/// R2 / OP2 (ENG-2 r3, unchanged): an `output` / `inout` actual is written whole by its copy-out
/// on every path (§13.5 :14750-14751 (output / inout arguments are copied out when the subroutine
/// returns)), so it covers the whole actual whatever the callee writes; the pin cites the LRM, not
/// iv (iv covers an output only when the task writes it, OP-7). An output formal bound to g[1]. r3
/// = PRE. Cell `RP1_outpart`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv
/// `C t=0 y=0 g=0010 | A end y=0 g=0011`; s2v `C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1
/// g=0011`; vl `C t=0 y=0 g=0010 | A end y=0 g=0001`.
#[test]
fn rp1_outpart_control_output_bound_to_a_bit_covers_that_bit() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  task automatic t(output logic x); x = 1'b1; endtask\n\
  always_comb begin t(g[1]); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 y=0 g=0010",
            "C t=0 y=0 g=0010",
            "C t=5 y=1 g=0011",
            "A end y=1 g=0011",
        ],
    );
}

/// R2 / OP2 (ENG-2 r3, unchanged): an `output` / `inout` actual is written whole by its copy-out
/// on every path (§13.5 :14750-14751 (output / inout arguments are copied out when the subroutine
/// returns)), so it covers the whole actual whatever the callee writes; the pin cites the LRM, not
/// iv (iv covers an output only when the task writes it, OP-7). The output is never written; the
/// copy-out still writes it. Cell `ROUTNW2`.
///
/// Raw lines: PRE `C t=0 y=x g=xxxx | C t=5 y=x g=xxxx | A end y=x g=xxxx`; iv `C t=0 y=x g=xxxx |
/// C t=5 y=1 g=xxx1 | A end y=1 g=xxx1`; s2v `C t=0 y=x g=xxxx | C t=5 y=1 g=xxx1 | A end y=1
/// g=xxx1`; vl `C t=0 y=0 g=0000 | A end y=0 g=0001`.
#[test]
fn routnw2_control_unwritten_output_still_covers() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  task automatic t(output logic [3:0] x); endtask\n\
  always_comb begin t(g); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=x g=xxxx", "A end y=x g=xxx1"],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. The ref is bound to m[1]; whole write. r3
/// = PRE. Cell `RELEM`.
///
/// Raw lines: PRE `C t=0 y=0 m0=0 m1=9 | C t=0 y=0 m0=0 m1=9 | C t=5 y=1 m0=1 m1=9 | A end y=1
/// m1=9`; iv `(iverilog refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:4:
/// sorry: constant selects in always_* processes are not fully supported (the process will be
/// sensitive to all bits in 'm[word='sd0][3: | t.sv:4: warning: System task ($display) cannot be
/// synthesized in an always_comb process.`; s2v `(sv2v/iverilog refused) | t.sv:3:20: Parse error:
/// unexpected token 'ref' (KW_ref)`; vl `C t=0 y=0 m0=0 m1=9 | A end y=0 m1=0`.
#[test]
fn relem_control_ref_bound_to_an_element_whole_write() {
    pin(
        "module top;\n\
  logic [3:0] m [0:1]; logic y;\n\
  task automatic t(ref logic [3:0] x); x = 4'd9; endtask\n\
  always_comb begin t(m[1]); y = m[0][0]; $display(\"C t=%0t y=%0d m0=%0d m1=%0d\", $time, y, m[0], m[1]); end\n\
  initial begin m[0] = 0; m[1] = 0; #5 m[0] = 4'd1; #2 $display(\"A end y=%0d m1=%0d\", y, m[1]); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0 m0=0 m1=9", "C t=0 y=0 m0=0 m1=9", "C t=5 y=1 m0=1 m1=9", "A end y=1 m1=9"],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. The ref is bound to m[k]: a non-constant
/// actual covers nothing. r3 = PRE. Cell `RELEMK`.
///
/// Raw lines: PRE `C t=0 y=0 m0=0 m1=9 | C t=0 y=0 m0=0 m1=9 | C t=5 y=1 m0=1 m1=9 | A end y=1`;
/// iv `(iverilog refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:4: sorry:
/// constant selects in always_* processes are not fully supported (the process will be sensitive
/// to all bits in 'm[word='sd0][3: | t.sv:4: warning: System task ($display) cannot be synthesized
/// in an always_comb process.`; s2v `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected
/// token 'ref' (KW_ref)`; vl `C t=0 y=0 m0=0 m1=9 | C t=5 y=1 m0=1 m1=9 | C t=5 y=1 m0=1 m1=9 | A
/// end y=1 | C t=7 y=1 m0=1 m1=9 | C t=7 y=1 m0=1 m1=9`.
#[test]
fn relemk_control_ref_bound_to_a_variable_element() {
    pin(
        "module top;\n\
  logic [3:0] m [0:1]; logic y; int k;\n\
  task automatic t(ref logic [3:0] x); x = 4'd9; endtask\n\
  always_comb begin t(m[k]); y = m[0][0]; $display(\"C t=%0t y=%0d m0=%0d m1=%0d\", $time, y, m[0], m[1]); end\n\
  initial begin k = 1; m[0] = 0; m[1] = 0; #5 m[0] = 4'd1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0 m0=0 m1=9", "C t=0 y=0 m0=0 m1=9", "C t=5 y=1 m0=1 m1=9", "A end y=1"],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. The ref is bound to g[1] (IEEE-illegal
/// packed select; vita accepts). r3 = PRE. Cell `RPSEL`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv
/// `(iverilog refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:4: sorry:
/// constant selects in always_* processes are not fully supported (the process will be sensitive
/// to all bits in 'g[3:0]'). | t.sv:4: sorry: constant selects in always_* processes are not fully
/// supported (the process will be sensitive to all bits in 'g[3:0]').`; s2v `(sv2v/iverilog
/// refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `(verilator refused) |
/// %Error: t.sv:4:24: Function/task ref argument is not of allowed type | %Error: Exiting due to 1
/// error(s)`.
#[test]
fn rpsel_control_ref_bound_to_a_bit_select() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  task automatic t(ref logic x); x = 1'b1; endtask\n\
  always_comb begin t(g[1]); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 y=0 g=0010",
            "C t=0 y=0 g=0010",
            "C t=5 y=1 g=0011",
            "A end y=1 g=0011",
        ],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. Every element of a ref array is written.
/// r3 = vl. Cell `RARRW`.
///
/// Raw lines: PRE `C t=0 o=8 m0=7 | C t=5 o=8 m0=7 | A end o=8`; iv `(iverilog refused) | t.sv:3:
/// sorry: Reference ports not supported yet. | t.sv:3: sorry: Subroutine ports with unpacked
/// dimensions are not yet supported. | t.sv:4: error: Array m needs an array index here.`; s2v
/// `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `C t=0
/// o=8 m0=7 | A end o=8`.
#[test]
fn rarrw_control_ref_array_every_element_written() {
    pin(
        "module top;\n\
  logic [7:0] m [0:1]; logic [7:0] o;\n\
  task automatic t(ref logic [7:0] x [0:1]); x[0] = 8'd7; x[1] = 8'd8; endtask\n\
  always_comb begin t(m); o = m[1]; $display(\"C t=%0t o=%0d m0=%0d\", $time, o, m[0]); end\n\
  initial begin #5 m[1] = 8'd3; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0 o=8 m0=7", "A end o=8"],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. x[1] under if, x whole else: the union is
/// whole. Cell `RMIX`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv
/// `(iverilog refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:4: sorry:
/// constant selects in always_* processes are not fully supported (the process will be sensitive
/// to all bits in 'g[3:0]'). | t.sv:4: warning: always_comb process has no sensitivities.`; s2v
/// `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl
/// `(verilator refused) | %Error: Internal Error: t.sv:2:15: ../V3DfgSynthesize.cpp:1034:
/// Different default drivers`.
#[test]
fn rmix_control_bit_or_whole_write_is_whole() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y, c;\n\
  task automatic t(ref logic [3:0] x); if (c) x[1] = 1'b1; else x = 4'd2; endtask\n\
  always_comb begin t(g); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin c = 1; g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0 g=0010", "A end y=0 g=0011"],
    );
}

/// R1 (ENG-2 r3): a `ref` actual covers what the same write spelled in the block covers — the
/// callee's written regions of the formal, mapped onto the actual (IEEE 1800-2017 :9047-9050 "...
/// read within the block or within any function called within the block with the following
/// exceptions: ... b) Any expression that is also written within the block or within any function
/// called within the block"; §13.5.2 :14775-14776 "Arguments passed by reference are not copied
/// into the subroutine area, rather, a reference to the original argument is passed", :14840-14841
/// "When the formal argument is declared as a const ref, the subroutine cannot alter the
/// variable"). iv / s2v refuse `ref`; vl loses the callee's write or excludes a written variable
/// whole, so the anchor is the local twin on every vita. The hierarchical callee writes its ref
/// whole. r3 = vl. Cell `RHCALLEEW`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=5 y=0 g=0010 | A end y=0 g=0010`; iv `(iverilog refused)
/// | t.sv:2: sorry: Reference ports not supported yet. | t.sv:6: sorry: constant selects in
/// always_* processes are not fully supported (the process will be sensitive to all bits in
/// 'g[3:0]'). | t.sv:6: warning: always_comb process has no sensitivities.`; s2v `(sv2v/iverilog
/// refused) | t.sv:2:21: Parse error: unexpected token 'ref' (KW_ref)`; vl `C t=0 y=0 g=0010 | A
/// end y=0 g=0011`.
#[test]
fn rhcalleew_control_hier_callee_ref_whole_write() {
    pin(
        "module child;\n\
  task automatic tk(ref logic [3:0] z); z = 4'b0010; endtask\n\
endmodule\n\
module top;\n\
  child u(); logic [3:0] g; logic y;\n\
  always_comb begin u.tk(g); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0 g=0010", "A end y=0 g=0011"],
    );
}

/// B2 (ENG-2 r2, unchanged in r3): a `const ref` actual never covers; a `ref` actual covers what
/// the callee writes of it (IEEE 1800-2017 :9047-9050 "... read within the block or within any
/// function called within the block with the following exceptions: ... b) Any expression that is
/// also written within the block or within any function called within the block"; §13.5.2
/// :14775-14776 "Arguments passed by reference are not copied into the subroutine area, rather, a
/// reference to the original argument is passed", :14840-14841 "When the formal argument is
/// declared as a const ref, the subroutine cannot alter the variable"). A const ref passed on to a
/// whole ref writer (IEEE-illegal; Q-line OP-3). r3 = PRE. Cell `RCR2RW`.
///
/// Raw lines: PRE `C t=0 o=9 g=9 | C t=0 o=9 g=9 | C t=5 o=9 g=9 | A end o=9 g=9`; iv `(iverilog
/// refused) | t.sv:4: syntax error | t.sv:5: error: Syntax error in task/function port
/// declaration. | t.sv:5: syntax error`; s2v `(sv2v/iverilog refused) | t.sv:3:20: Parse error:
/// unexpected token 'ref' (KW_ref)`; vl `(verilator refused) | %Error: t.sv:4:48: Assigning to
/// const ref variable: 'x' | %Error: Exiting due to 1 error(s)`.
#[test]
fn rcr2rw_control_const_ref_passed_to_a_writer_covers_nothing() {
    pin(
        "module top;\n\
  logic [3:0] g; logic [3:0] o;\n\
  task automatic w(ref logic [3:0] z); z = 4'd9; endtask\n\
  task automatic t(const ref logic [3:0] x); w(x); o = x; endtask\n\
  always_comb begin t(g); $display(\"C t=%0t o=%0d g=%0d\", $time, o, g); end\n\
  initial begin g = 0; #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 o=9 g=9",
            "C t=0 o=9 g=9",
            "C t=5 o=9 g=9",
            "A end o=9 g=9",
        ],
    );
}

/// `always @*` records no writes (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and
/// variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)): the destination / actual is read. `$sscanf` in always
/// @*. r3 = PRE = iv = s2v. Cell `SSYS`.
///
/// Raw lines: PRE `C t=5 o=9 g=9 | A end o=9 g=9`; iv `C t=5 o=9 g=9 | A end o=9 g=9`; s2v `C t=5
/// o=9 g=9 | A end o=9 g=9`; vl `C t=0 o=9 g=9 | A end o=9 g=1`.
#[test]
fn ssys_control_star_reads_a_sscanf_destination() {
    pin(
        "module top;\n\
  logic [3:0] g; logic [3:0] o; integer n;\n\
  always @* begin n = $sscanf(\"9\", \"%d\", g); o = g; $display(\"C t=%0t o=%0d g=%0d\", $time, o, g); end\n\
  initial begin #5 g = 1; #2 $display(\"A end o=%0d g=%0d\", o, g); $finish; end\n\
endmodule\n\
",
        &["C t=5 o=9 g=9", "A end o=9 g=9"],
    );
}

/// `always @*` records no writes (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and
/// variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)): the destination / actual is read. RP1 under always
/// @*. r3 = PRE. Cell `RP1STAR`.
///
/// Raw lines: PRE `C t=0 y=0 g=0010 | C t=5 y=1 g=0011 | A end y=1 g=0011`; iv `(iverilog refused)
/// | t.sv:3: sorry: Reference ports not supported yet. | 1 error(s) during elaboration.`; s2v
/// `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `C t=0
/// y=0 g=0010 | A end y=0 g=0001`.
#[test]
fn rp1star_control_star_ref_actual_is_read() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  task automatic t(ref logic [3:0] x); x[1] = 1'b1; endtask\n\
  always @* begin t(g); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=0 g=0010", "C t=5 y=1 g=0011", "A end y=1 g=0011"],
    );
}

/// Control (ENG-2 r3): the shape the round-3 rules must not move. KP_INB_CALLONLY's local twin. r3
/// = PRE = s2v. Cell `KP_INB_CALLONLY_local`.
///
/// Raw lines: PRE `W t=5 m0=5 | A end m0=5`; iv `(iverilog refused) | t.sv:2: sorry: Subroutine
/// ports with unpacked dimensions are not yet supported. | 1 error(s) during elaboration.`; s2v `W
/// t=5 m0=5 | A end m0=5`; vl `(verilator refused) | %Error-UNSUPPORTED: t.sv:5:5: Event control
/// with implicit sensitivity (@*) in this context; use an e... | %Error: Exiting due to 1
/// error(s)`.
#[test]
fn kp_inb_callonly_local_control() {
    pin_diag(
        "module top;\n\
  task automatic w2(output logic [3:0] o [2], input logic [3:0] x); o[0] = x; o[1] = 4'd2; endtask\n\
  logic [3:0] m [2]; logic [3:0] a = 0, b = 0, y;\n\
  initial begin\n\
    @(*) w2(m, a);\n\
    $display(\"W t=%0t m0=%0d\", $time, m[0]);\n\
    y = b;\n\
  end\n\
  initial begin #3 b = 4'd1; #2 a = 4'd5; #2 $display(\"A end m0=%0d\", m[0]); $finish; end\n\
endmodule\n\
",
        &["W t=5 m0=5", "A end m0=5"],
        &[],
    );
}

/// R4 (ENG-2 r3): an in-body `@(*)`'s W3056 ("reads no nets; it can never wake") is decided in the
/// settle from the settled set (IEEE 1800-2017 §9.4.2.2 :9580-9582 "@* ... adding all nets and
/// variables that are read by the statement (which can be a statement group) of a
/// procedural_timing_control_statement" and :9590-9597 (the identifiers in the statement; "in
/// subroutine calls" = a call's arguments)): a hierarchical call's arguments count, a resolved
/// parameter does not. `@(*) y = 4'd5;`: empty on the T0 set and the settled set. r3 = PRE = iv =
/// s2v. Cell `WLOC`.
///
/// Raw lines: PRE `W3056: in-body @(*) reads no nets; it can never wake [in top]... | A end y=x`;
/// iv `A end y=x`; s2v `A end y=x`; vl `(verilator refused) | %Error-UNSUPPORTED: t.sv:3:17: Event
/// control with implicit sensitivity (@*) in this context; use an e... | %Error: Exiting due to 1
/// error(s)`.
#[test]
fn wloc_control_inbody_star_on_a_constant_warns_w3056() {
    pin_diag(
        "module top;\n\
  logic [3:0] y;\n\
  initial begin @(*) y = 4'd5; $display(\"W t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["A end y=x"],
        &["W3056"],
    );
}

/// Premise pin (ENG-2 r3): this shape is refused at elaboration on every vita (PRE = K3 = r3), so
/// no read-set question reaches it. A hierarchical ref actual `u.v`. Cell `RHIER`.
///
/// Raw lines: PRE `E3009: task `t` output/inout arg must be a simple net [in top]...`; iv
/// `(iverilog refused) | t.sv:4: sorry: Reference ports not supported yet. | t.sv:5: sorry:
/// constant selects in always_* processes are not fully supported (the process will be sensitive
/// to all bits in 'v[3:0]'). | t.sv:5: warning: always_comb process has no sensitivities.`; s2v
/// `(sv2v/iverilog refused) | t.sv:4:20: Parse error: unexpected token 'ref' (KW_ref)`; vl `C t=0
/// y=0 v=0010 | A end y=0 v=0001`.
#[test]
fn rhier_premise_hier_ref_actual_is_loud() {
    pin_loud(
        "module child; logic [3:0] v; endmodule\n\
module top;\n\
  child u(); logic y;\n\
  task automatic t(ref logic [3:0] x); x[1] = 1'b1; endtask\n\
  always_comb begin t(u.v); y = u.v[0]; $display(\"C t=%0t y=%0d v=%b\", $time, y, u.v); end\n\
  initial begin u.v = 0; #5 u.v[0] = 1'b1; #2 $display(\"A end y=%0d v=%b\", y, u.v); $finish; end\n\
endmodule\n\
",
        "E3009",
        "output/inout arg must be a simple net",
    );
}

/// Premise pin (ENG-2 r3): this shape is refused at elaboration on every vita (PRE = K3 = r3), so
/// no read-set question reaches it. `$sscanf` into x[1] of a ref. Cell `RSYSPART`.
///
/// Raw lines: PRE `E3009: $fscanf/$sscanf destination arguments must be plain variable...`; iv
/// `(iverilog refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:4: sorry:
/// constant selects in always_* processes are not fully supported (the process will be sensitive
/// to all bits in 'g[3:0]'). | t.sv:4: warning: always_comb process has no sensitivities.`; s2v
/// `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token 'ref' (KW_ref)`; vl
/// `(verilator refused) | %Error: 'make -C obj -f Vtop.mk -j 1' exited with 2 | %Error: Command
/// Failed /opt/homebrew/Cellar/verilator/5.052/bin/verilator_bin --binary --timing -Wno-fatal
/// -Wno-lint -Wno-style -Wno-MULTIDR`.
#[test]
fn rsyspart_premise_sscanf_into_a_ref_bit_is_loud() {
    pin_loud(
        "module top;\n\
  logic [3:0] g; logic y; integer n;\n\
  task automatic t(ref logic [3:0] x); n = $sscanf(\"1\", \"%d\", x[1]); endtask\n\
  always_comb begin t(g); y = g[0]; $display(\"C t=%0t y=%0d g=%b\", $time, y, g); end\n\
  initial begin g = 0; #5 g[0] = 1'b1; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        "E3009",
        "destination arguments must be",
    );
}

/// Premise pin (ENG-2 r3): this shape is refused at elaboration on every vita (PRE = K3 = r3), so
/// no read-set question reaches it. `$readmem` into a ref array formal. Cell `RRDM`.
///
/// Raw lines: PRE `E3009: a whole unpacked-array formal has no value here — index an e...`; iv
/// `(iverilog refused) | t.sv:3: sorry: Reference ports not supported yet. | t.sv:3: sorry:
/// Subroutine ports with unpacked dimensions are not yet supported. | t.sv:4: error: Array m needs
/// an array index here.`; s2v `(sv2v/iverilog refused) | t.sv:3:20: Parse error: unexpected token
/// 'ref' (KW_ref)`; vl `C t=0 o=6 | A end o=6 m0=5`.
#[test]
fn rrdm_premise_readmem_into_a_ref_array_formal_is_loud() {
    pin_loud(
        "module top;\n\
  logic [7:0] m [0:1]; logic [7:0] o;\n\
  task automatic t(ref logic [7:0] x [0:1]); $readmemh(\"m.hex\", x); endtask\n\
  always_comb begin t(m); o = m[1]; $display(\"C t=%0t o=%0d\", $time, o); end\n\
  initial begin #5 m[1] = 8'd3; #2 $display(\"A end o=%0d m0=%0d\", o, m[0]); $finish; end\n\
endmodule\n\
",
        "E3009",
        "a whole unpacked-array formal has no value here",
    );
}

/// Frame task (ENG-2 r3, parked at r4): the `Wait` cause of an in-body `@(*)` in a frame task
/// body is built when the body is lowered and is final, so the frame classify guard
/// (`wait_cond_reads_frame_local`) sees it: a set that reads the task's own variable stays loud
/// E3009 (its level wait would re-evaluate without the frame window: never woken), as at PRE. `@(*) y = l;` on the task's local l, written by a forked arm.
/// Cell `w7`.
///
/// Raw lines: PRE `E3009: frame task `t` body uses a `wait`/`@` whose condition reads ...`; iv
/// `Assertion failed: (child->wt_context==0 || thr->wt_context!=child->wt_context), function
/// of_JOIN_DETACH, file vthread.cc, line 3793.`; s2v `(sv2v/iverilog refused) | t.sv:3:82: Parse
/// error: missing expected `join``; vl `(verilator refused) | %Error-UNSUPPORTED: t.sv:3:92: Event
/// control with implicit sensitivity (@*) in this context; use an e... | %Error: Exiting due to 1
/// error(s)`.
#[test]
fn w7_inbody_star_on_a_frame_local_in_a_forking_task_stays_loud() {
    pin_loud(
        "module top;\n\
  logic [3:0] a, y;\n\
  task automatic t(input logic [3:0] v); logic [3:0] l; l = v; fork #2 l = 4'd7; join_none @(*) y = l; endtask\n\
  initial begin a = 0; t(4'd1); end\n\
  initial begin #5 a = 4'd3; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        "E3009",
        "reads a variable local to this task",
    );
}

/// Frame task (ENG-2 r3, parked at r4): the `Wait` cause of an in-body `@(*)` in a frame task
/// body is built when the body is lowered and is final, so the frame classify guard
/// (`wait_cond_reads_frame_local`) sees it: a set that reads the task's own variable stays loud
/// E3009 (its level wait would re-evaluate without the frame window: never woken), as at PRE. `@(*) y = u.g(l);`: a hierarchical call's frame-local argument.
/// Cell `w9`.
///
/// Raw lines: PRE `E3009: frame task `t` body uses a `wait`/`@` whose condition reads ...`; iv
/// `Assertion failed: (child->wt_context==0 || thr->wt_context!=child->wt_context), function
/// of_JOIN_DETACH, file vthread.cc, line 3793.`; s2v `(sv2v/iverilog refused) | t.sv:5:82: Parse
/// error: missing expected `join``; vl `(verilator refused) | %Error-UNSUPPORTED: t.sv:5:92: Event
/// control with implicit sensitivity (@*) in this context; use an e... | %Error: Exiting due to 1
/// error(s)`.
#[test]
fn w9_inbody_star_hier_call_on_a_frame_local_stays_loud() {
    pin_loud(
        "module sub; function automatic logic [3:0] g(input logic [3:0] x); return x; endfunction endmodule\n\
module top;\n\
  logic [3:0] a, y;\n\
  sub u();\n\
  task automatic t(input logic [3:0] v); logic [3:0] l; l = v; fork #2 l = 4'd7; join_none @(*) y = u.g(l); endtask\n\
  initial begin a = 0; t(4'd1); end\n\
  initial begin #5 a = 4'd3; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        "E3009",
        "reads a variable local to this task",
    );
}

/// Frame task (ENG-2 r3, parked at r4): the `Wait` cause of an in-body `@(*)` in a frame task
/// body is built when the body is lowered and is final, so the frame classify guard
/// (`wait_cond_reads_frame_local`) sees it: a set that reads the task's own variable stays loud
/// E3009 (its level wait would re-evaluate without the frame window: never woken), as at PRE. The control: the wait reads a module net (the fork-in-frame path
/// runs). r3 = PRE = vl. Cell `w7c`.
///
/// Raw lines: PRE `A end y=7`; iv `Assertion failed: (child->wt_context==0 ||
/// thr->wt_context!=child->wt_context), function of_JOIN_DETACH, file vthread.cc, line 3793.`; s2v
/// `(sv2v/iverilog refused) | t.sv:3:67: Parse error: missing expected `join``; vl `A end y=7`.
#[test]
fn w7c_control_inbody_star_on_a_module_net_in_a_forking_task() {
    pin(
        "module top;\n\
  logic [3:0] a, y, m;\n\
  task automatic t(input logic [3:0] v); m = v; fork #2 m = 4'd7; join_none @(m) y = m; endtask\n\
  initial begin a = 0; t(4'd1); end\n\
  initial begin #5 a = 4'd3; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["A end y=7"],
    );
}

/// Open point (ENG-2 r3): `foreach` over an associative array writes its key through the assoc
/// key path (a write within the block, IEEE 1800-2017 :9047-9050 exception b); the block wakes
/// once per change of the array and settles. PRE = r2 = K3 = K3b = r3; no oracle runs it, the
/// values are the hand sum (2*1 + 4*3, + 5*2, then 4*3 -> 1*3). Cell `r3/probe/FASSOC`.
///
/// Raw lines: PRE `C t=0 s=14 | C t=0 s=14 | C t=5 s=24 | C t=10 s=15 | A end s=15`; iv
/// `(iverilog refused) | t.sv:2: error: Type names are not valid expressions here.`; s2v
/// `(sv2v/iverilog refused) | t.sv:2:14: Parse error: unexpected token ']' (Sym_brack_r)`; vl
/// `(verilator refused) | %Error: Circular logic when ordering code (non-cutable edge loop)`.
#[test]
fn fassoc_comb_foreach_over_an_assoc_array_wakes_on_the_array_only() {
    pin(
        "module top;\n\
  int aa [int];\n\
  int s;\n\
  always_comb begin\n\
    s = 0;\n\
    foreach (aa[k]) s += aa[k] * k;\n\
    $display(\"C t=%0t s=%0d\", $time, s);\n\
  end\n\
  initial begin\n\
    aa[1] = 2; aa[3] = 4;\n\
    #5 aa[2] = 5;\n\
    #5 aa[3] = 1;\n\
    #2 $display(\"A end s=%0d\", s); $finish;\n\
  end\n\
endmodule\n\
",
        &[
            "C t=0 s=14",
            "C t=0 s=14",
            "C t=5 s=24",
            "C t=10 s=15",
            "A end s=15",
        ],
    );
}

// ---- ENG-2 round 4 (the frame-task part parked: R4) ----

/// MS9 (ENG-2 r3 lens-sound): a system task output argument inside an inlined static task body
/// called from `always_comb` is a read of the block, not a write (the task body adds its arguments
/// only, IEEE 1800-2017 §9.2.2.2.1 :9056-9057), so the block wakes on `g` = PRE = iv = s2v. Cell
/// `r3/lens-sound/c/LSYST`.
///
/// Raw lines: PRE `C t=0 y=9 g=9 | C t=5 y=9 g=9 | A end y=9 g=9`; iv `C t=0 y=9 g=9 | C t=5 y=9
/// g=9 | A end y=9 g=9`; s2v `C t=0 y=9 g=9 | C t=5 y=9 g=9 | A end y=9 g=9`; vl `C t=0 y=9 g=9 |
/// A end y=9 g=1`.
#[test]
fn lsyst_comb_static_task_sscanf_destination_is_read_not_written() {
    pin(
        "module top;\n\
  logic [3:0] g, y; int n;\n\
  task st; n = $sscanf(\"9\", \"%d\", g); endtask\n\
  always_comb begin st(); y = g; $display(\"C t=%0t y=%0d g=%0d\", $time, y, g); end\n\
  initial begin #5 g = 4'd1; #2 $display(\"A end y=%0d g=%0d\", y, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=9 g=9", "C t=5 y=9 g=9", "A end y=9 g=9"],
    );
}

/// MS7 (ENG-2 r3 lens-sound): a `ref` actual covers what the subroutine writes of it through a
/// three-level `ref` chain (the write-map fixed point reaches the leaf's whole write; IEEE
/// 1800-2017 §9.2.2.2.1 :9047-9050 exception b, §13.5.2), so the block does not wake on `g` = vl.
/// PRE woke (the copy-out). Cell `r3/lens-sound/c/R3W`.
///
/// Raw lines: PRE `C t=0 y=9 | C t=5 y=9 | A end y=9 g=9`; iv `(iverilog refused) | t.sv:5:
/// sorry: Reference ports not supported yet.`; s2v `(sv2v/iverilog refused) | t.sv:3:23: Parse
/// error: unexpected token 'ref' (KW_ref)`; vl `C t=0 y=9 | A end y=9 g=1`.
#[test]
fn r3w_comb_ref_chain_three_levels_covers_the_leaf_write() {
    pin(
        "module top;\n\
  logic [3:0] g, y;\n\
  task automatic c_in(ref logic [3:0] z); z = 4'd9; endtask\n\
  task automatic b_mid(ref logic [3:0] w); c_in(w); endtask\n\
  task automatic a_out(ref logic [3:0] x); b_mid(x); endtask\n\
  always_comb begin a_out(g); y = g; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 g = 4'd1; #2 $display(\"A end y=%0d g=%0d\", y, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=9", "A end y=9 g=1"],
    );
}

/// MS7 (ENG-2 r3 lens-sound): a four-task `ref` recursion cycle writing `x = 4'hF` in its last
/// task covers `g` (the write map is a least fixed point over the cycle; §9.2.2.2.1 exception b),
/// so the block does not wake on `g[3]` — the local twin `RMW_tw` (`g = 4'hF; y = g[3];`) = iv = vl
/// `C t=0 y=1 | A end y=1 g=0111`. PRE woke. No oracle runs the `ref` form. Cell
/// `r3/lens-sound/c/RMW4`.
///
/// Raw lines: PRE `C t=0 y=1 | C t=5 y=1 | A end y=1 g=1111`; twin iv `C t=0 y=1 | A end y=1
/// g=0111`; twin s2v `C t=0 y=1 | C t=5 y=1 | A end y=1 g=1111`; twin vl `C t=0 y=1 | A end y=1
/// g=0111`.
#[test]
fn rmw4_comb_ref_cycle_of_four_tasks_covers_the_actual() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  task automatic t0(ref logic [3:0] x, input int n); if (n > 0) t1(x, n - 1); endtask\n\
  task automatic t1(ref logic [3:0] x, input int n); if (n > 0) t2(x, n - 1); endtask\n\
  task automatic t2(ref logic [3:0] x, input int n); if (n > 0) t3(x, n - 1); endtask\n\
  task automatic t3(ref logic [3:0] x, input int n); x = 4'hF; if (n > 0) t0(x, n - 1); endtask\n\
  always_comb begin t0(g, 5); y = g[3]; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 g[3] = 1'b0; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=1", "A end y=1 g=0111"],
    );
}

/// MS7 (ENG-2 r3 lens-sound): RMW4 with the tasks declared in reverse order (the fixed point does
/// not depend on the walk order). = the local twin `RMW_tw` = iv = vl. Cell `r3/lens-sound/c/RMW4r`.
///
/// Raw lines: PRE `C t=0 y=1 | C t=5 y=1 | A end y=1 g=1111`; twin iv `C t=0 y=1 | A end y=1
/// g=0111`; twin s2v `C t=0 y=1 | C t=5 y=1 | A end y=1 g=1111`; twin vl `C t=0 y=1 | A end y=1
/// g=0111`.
#[test]
fn rmw4r_comb_ref_cycle_declared_in_reverse_covers_the_actual() {
    pin(
        "module top;\n\
  logic [3:0] g; logic y;\n\
  task automatic t3(ref logic [3:0] x, input int n); x = 4'hF; if (n > 0) t0(x, n - 1); endtask\n\
  task automatic t2(ref logic [3:0] x, input int n); if (n > 0) t3(x, n - 1); endtask\n\
  task automatic t1(ref logic [3:0] x, input int n); if (n > 0) t2(x, n - 1); endtask\n\
  task automatic t0(ref logic [3:0] x, input int n); if (n > 0) t1(x, n - 1); endtask\n\
  always_comb begin t0(g, 5); y = g[3]; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 g[3] = 1'b0; #2 $display(\"A end y=%0d g=%b\", y, g); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=1", "A end y=1 g=0111"],
    );
}

/// N-W7C (ENG-2 r3 lens-sound: the `w7c` control spells `@(m)`): the `@(*)` spelling of it — an
/// automatic task forks a writer of the module variable `m` and waits on `@(*) y = m;`, which reads
/// a module variable only (no frame-local read), so the fork-in-frame path runs and the wait wakes
/// on the arm's write (IEEE 1800-2017 §9.4.2.2 :9580-9582; hand `A end y=7`). Every vita binary
/// = PRE. Cell `replan4/c8/W7CS`.
///
/// Raw lines: PRE `A end y=7`; iv `Assertion failed: (child->wt_context==0 ||
/// thr->wt_context!=child->wt_context), function of_JOIN_DETACH, file vthread.cc, line 3793.`; s2v
/// `(sv2v/iverilog refused) | t.sv:3:67: Parse error: missing expected `join``; vl `(verilator
/// refused) | %Error-UNSUPPORTED: t.sv:3:77: Unsupported: Event control with implicit sensitivity
/// (@*) in this context; use an explicit sensitivity list instead`.
#[test]
fn w7cs_inbody_star_on_a_module_net_in_a_forking_task() {
    pin(
        "module top;\n\
  logic [3:0] a, y, m;\n\
  task automatic t(input logic [3:0] v); m = v; fork #2 m = 4'd7; join_none @(*) y = m; endtask\n\
  initial begin a = 0; t(4'd1); end\n\
  initial begin #5 a = 4'd3; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["A end y=7"],
    );
}

/// Park (ENG-2 r4, R4): an in-body `@(*)` in a frame task body keeps the set taken when the body
/// is lowered — its statement's own reads; a nested call's arguments are not resolved there — so
/// the frame classify does not see `v` through `tk3(v, y)` and the static recursive task runs as
/// at PRE (round 1-3 refused it, E3009; the PLAN-r4 finding SK1). The wait wakes on `a` (IEEE
/// 1800-2017 §9.4.2.2 :9580-9582). = PRE = iv = s2v. Cell `replan4/c6/SK1`.
///
/// Raw lines: PRE `W t=5 y=1 z=3 | A end y=1 z=3`; iv `W t=5 y=1 z=3 | A end y=1 z=3`; s2v `W t=5
/// y=1 z=3 | A end y=1 z=3`; vl `(verilator refused) | %Error-UNSUPPORTED: t.sv:4:74: Unsupported:
/// Event control with implicit sensitivity (@*) in this context; use an explicit sensitivity list
/// instead`.
#[test]
fn sk1_park_inbody_star_in_a_static_recursive_task_with_a_nested_call_runs() {
    pin(
        "module top;\n\
  logic [3:0] a, y, z, g;\n\
  task automatic tk3(input logic [3:0] x, output logic [3:0] q); q = x; endtask\n\
  task t(input logic [3:0] v, input int n); if (n > 0) t(v, n - 1); else @(*) begin tk3(v, y); z = a; $display(\"W t=%0t y=%0d z=%0d\", $time, y, z); end endtask\n\
  initial begin #1 t(4'd1, 0); end\n\
  initial begin a = 0; g = 0; #3 g = 4'd5; #2 a = 4'd3; #2 $display(\"A end y=%0d z=%0d\", y, z); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1 z=3", "A end y=1 z=3"],
    );
}

/// Park (ENG-2 r4, R4): SK1's automatic-task form — a nested automatic task's argument `v` (the
/// task's own variable, which nothing else writes) is not in the lowering-time set, the classify
/// lifts the task, and the wait wakes on `a` (§9.4.2.2 :9580-9582: hand `W t=5 y=1 z=3`). = PRE;
/// rounds 1-3 refused it (E3009). Cell `replan4/c5/K1_ftask`.
///
/// Raw lines: PRE `W t=5 y=1 z=3 | A end y=1 z=3 m1=0`; iv `` (prints nothing); s2v `` (prints
/// nothing); vl `(verilator refused) | %Error-UNSUPPORTED: t.sv:7:42: Unsupported: Event control
/// with implicit sensitivity (@*) in this context; use an explicit sensitivity list instead`.
#[test]
fn k1_park_inbody_star_in_an_automatic_task_with_a_nested_call_runs() {
    pin(
        "module top;\n\
  logic [3:0] a, y, z;\n\
  logic [3:0] mm [4];\n\
  function automatic logic [3:0] f(input logic [3:0] x); return x + 4'd1; endfunction\n\
  task automatic tk3(input logic [3:0] x, output logic [3:0] q); q = x; endtask\n\
  task tks(input logic [3:0] x, output logic [3:0] q); q = x; endtask\n\
  task automatic t(input logic [3:0] v); @(*) begin tk3(v, y); z = a; $display(\"W t=%0t y=%0d z=%0d\", $time, y, z); end endtask\n\
  initial begin #1 t(4'd1); end\n\
  initial begin a = 0; mm[0] = 0; mm[1] = 0; #5 a = 4'd3; #2 $display(\"A end y=%0d z=%0d m1=%0d\", y, z, mm[1]); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=1 z=3", "A end y=1 z=3 m1=0"],
    );
}

/// Park (ENG-2 r4, R4): a `fork ... join` writes the task's own `l` and ends before the wait, then
/// `@(*) begin tk3(l, y); z = a; .. end`: nothing writes `l` while the wait pends, so it wakes on
/// `a` with `y=7` (§9.3.2, §9.4.2.2 :9580-9582: hand `W t=5 y=7 z=3`). = PRE = K3b; rounds 1-3 and
/// K3 refused it (E3009). Cell `replan4/review/d5src/DJ_nc`.
///
/// Raw lines: PRE `W t=5 y=7 z=3 | A end y=7 z=3`; iv, s2v, vl not run on this cell (the
/// hierarchical twin `DJ_h`: iv prints nothing, s2v and vl refuse).
#[test]
fn dj_nc_park_join_writer_ends_before_the_inbody_star_runs() {
    pin(
        "module top;\n\
  logic [3:0] a, y, z;\n\
  task automatic tk3(input logic [3:0] x, output logic [3:0] q); q = x; endtask\n\
  task automatic t(input logic [3:0] v0); logic [3:0] l; l = v0; fork l = 4'd7; join @(*) begin tk3(l, y); z = a; $display(\"W t=%0t y=%0d z=%0d\", $time, y, z); end endtask\n\
  initial begin #1 t(4'd1); end\n\
  initial begin a = 0; #5 a = 4'd3; #2 $display(\"A end y=%0d z=%0d\", y, z); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=7 z=3", "A end y=7 z=3"],
    );
}

/// Park (ENG-2 r4, R4): a `join_none` arm only READS the task's own `l`, then
/// `@(*) begin tk3(l, y); z = a; .. end`: `l` cannot change while the wait pends, so it wakes on
/// `a` with `y=1` (§9.3.2, §9.4.2.2: hand `F t=1 l=1 | W t=5 y=1 z=3`). = PRE = K3b; rounds 1-3
/// and K3 refused it (E3009). Cell `replan4/review/d5src/DR_nc`.
///
/// Raw lines: PRE `F t=1 l=1 | W t=5 y=1 z=3 | A end y=1 z=3`; iv, s2v, vl not run on this cell
/// (the hierarchical twin `DR_h`: iv crashes vthread.cc:3793, s2v parse error, vl refuses `@*`).
#[test]
fn dr_nc_park_read_only_arm_beside_the_inbody_star_runs() {
    pin(
        "module top;\n\
  logic [3:0] a, y, z;\n\
  task automatic tk3(input logic [3:0] x, output logic [3:0] q); q = x; endtask\n\
  task automatic t(input logic [3:0] v0); logic [3:0] l; l = v0; fork $display(\"F t=%0t l=%0d\", $time, l); join_none @(*) begin tk3(l, y); z = a; $display(\"W t=%0t y=%0d z=%0d\", $time, y, z); end endtask\n\
  initial begin #1 t(4'd1); end\n\
  initial begin a = 0; #5 a = 4'd3; #2 $display(\"A end y=%0d z=%0d\", y, z); $finish; end\n\
endmodule\n\
",
        &["F t=1 l=1", "W t=5 y=1 z=3", "A end y=1 z=3"],
    );
}

/// Park (ENG-2 r4, R4): the parked part, pinned where it stands. KNOWN-WRONG: the pinned value
/// is PRE's wrong `q=x`, kept only because this pin is mutant MP1's only killer (MP1: the frame
/// site recorded and settled again; it prints `A end q=3`). A hierarchical task enable is
/// resolved after the frame task body is lowered, so the lowering-time set of
/// `@(*) u.tk(v + a);` in an automatic task is empty: `VITA-W3056 ... it can never wake` at
/// lowering and the wait never wakes (`q=x`) — PRE's line. Right: iv = s2v `A end q=3` (wakes on
/// `a`, IEEE 1800-2017 §9.4.2.2 :9590-9597 "in subroutine calls"); round 3 gave `A end q=3` from
/// the settled set. ROADMAP §5.2 Q-FRAMESTAR names this pin: it flips to `A end q=3`
/// when that row lands. Cell `r3/lens-diff/f1/Fhtask_mix`.
///
/// Raw lines: PRE `t.sv:5:42: warning[VITA-W3056] W-ELAB-FEATURE-LIMIT: in-body @(*) reads no
/// nets; it can never wake [in top.$func$t] | A end q=x`; iv `A end q=3`; s2v `A end q=3`; vl
/// `(verilator refused) | %Error-UNSUPPORTED: t.sv:5:42: Unsupported: Event control with implicit
/// sensitivity (@*) in this context; use an explicit sensitivity list instead`.
#[test]
fn fhtask_mix_park_hier_task_enable_in_an_automatic_task_warns_at_lowering() {
    pin_diag(
        "module sub; logic [3:0] q; task automatic tk(input logic [3:0] x); q = x; endtask endmodule\n\
module top;\n\
  logic [3:0] a;\n\
  sub u();\n\
  task automatic t(input logic [3:0] v); @(*) u.tk(v + a); endtask\n\
  initial begin a = 0; #1 t(4'd1); end\n\
  initial begin #5 a = 4'd2; #2 $display(\"A end q=%0d\", u.q); $finish; end\n\
endmodule\n\
",
        &["A end q=x"],
        &["W3056"],
    );
}

/// Every backend stops at elaboration (exit 1) with exactly the diagnostics `codes`, the last an
/// error whose text holds `needle` (PRE warns `VITA-W3056` on an empty in-body `@(*)` before the
/// frame body is refused).
fn pin_loud_codes(src: &str, codes: &[&str], needle: &str) {
    for be in ["native", "vm", "interp"] {
        let d = scratch(be);
        std::fs::write(d.join("t.sv"), src).unwrap();
        let o = vita(&d, &["--backend", be, "t.sv"]);
        let _ = std::fs::remove_dir_all(&d);
        let err = String::from_utf8_lossy(&o.stderr);
        assert_eq!(o.status.code(), Some(1), "backend {be}: {err}");
        assert_eq!(diag_codes(&o), codes, "backend {be} diagnostics:\n{err}");
        let last = err.lines().rfind(|l| l.contains("error[VITA-"));
        assert!(
            last.is_some_and(|l| l.contains(needle)),
            "backend {be}: last error lacks `{needle}`:\n{err}"
        );
    }
}

/// Park (ENG-2 r5): an in-body `@(*)` in a frame task body takes the set its walk took before
/// ENG-2 (`frame_star_read_set`): the reads of the static function `sf2` reduced into the
/// statement, which ignores its second formal, so the task's own `l` passed there is not in the
/// set, the frame classify lifts the task, and the wait wakes on `a` (IEEE 1800-2017 §9.4.2.2:
/// `l` never changes). = PRE = iv = s2v; rounds 1-4 refused it (E3009: the argument counted).
/// Cell `r4/lens-diff/c/W7_sfn_ign_loc`.
///
/// Raw lines: PRE `W t=5 y=4 | A end y=4`; iv `W t=5 y=4 | A end y=4`; s2v `W t=5 y=4 | A end
/// y=4`; vl `(verilator refused) | %Error-UNSUPPORTED: t.sv:7:5: Unsupported: Event control with
/// implicit sensitivity (@*) in this context; use an explicit sensitivity list instead`.
#[test]
fn w7s_park_inbody_star_static_function_ignores_a_frame_local() {
    pin(
        "module top;\n\
  logic [3:0] a, y;\n\
  function [3:0] sf2(input [3:0] x, input [3:0] z); sf2 = x + 4'd1; endfunction\n\
  task automatic t(input logic [3:0] v);\n\
    logic [3:0] l;\n\
    l = v;\n\
    @(*) y = sf2(a, l);\n\
    $display(\"W t=%0t y=%0d\", $time, y);\n\
  endtask\n\
  initial begin a = 1; #1 t(4'd2); end\n\
  initial begin #5 a = 3; end\n\
  initial begin #9 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=4", "A end y=4"],
    );
}

/// Park (ENG-2 r5): `w7s` with the task's own input formal `v` as the ignored argument. = PRE =
/// iv = s2v; rounds 1-4 refused it (E3009). Cell `r4/lens-diff/c/X7g_sfn_ign_formal`.
///
/// Raw lines: PRE `W t=5 y=4 | A end y=4`; iv `W t=5 y=4 | A end y=4`; s2v `W t=5 y=4 | A end
/// y=4`; vl `(verilator refused) | %Error-UNSUPPORTED: t.sv:5:5: Unsupported: Event control with
/// implicit sensitivity (@*) in this context; use an explicit sensitivity list instead`.
#[test]
fn x7g_park_inbody_star_static_function_ignores_a_task_formal() {
    pin(
        "module top;\n\
  logic [3:0] a, y;\n\
  function [3:0] sf2(input [3:0] x, input [3:0] z); sf2 = x + 4'd1; endfunction\n\
  task automatic t(input logic [3:0] v);\n\
    @(*) y = sf2(a, v);\n\
    $display(\"W t=%0t y=%0d\", $time, y);\n\
  endtask\n\
  initial begin a = 1; #1 t(4'd2); end\n\
  initial begin #5 a = 3; end\n\
  initial begin #9 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=4", "A end y=4"],
    );
}

/// Park (ENG-2 r5): `w7s` in a module instantiated twice — each instance's task wakes on its own
/// `a`. = PRE; iv = s2v print the same lines with `u0` first (two wakes in one time step: their
/// order is not defined, IEEE 1800-2017 §4.7). Rounds 1-4 refused it (E3009 per instance). Cell
/// `r4/lens-diff/b/B4_2inst_ign`.
///
/// Raw lines: PRE `W top.u1.t t=5 y=4 | W top.u0.t t=5 y=4 | A end y0=4 y1=4`; iv `W top.u0.t t=5
/// y=4 | W top.u1.t t=5 y=4 | A end y0=4 y1=4`; s2v `W top.u0.t t=5 y=4 | W top.u1.t t=5 y=4 | A
/// end y0=4 y1=4`; vl `(verilator refused) | %Error-UNSUPPORTED: t.sv:4:64: Unsupported: Event
/// control with implicit sensitivity (@*) in this context; use an explicit sensitivity list
/// instead`.
#[test]
fn b4_park_inbody_star_ignored_frame_local_in_two_instances() {
    pin(
        "module m;\n\
  logic [3:0] a, y;\n\
  function [3:0] sf2(input [3:0] x, input [3:0] z); sf2 = x + 4'd1; endfunction\n\
  task automatic t(input logic [3:0] v); logic [3:0] l; l = v; @(*) y = sf2(a, l); $display(\"W %m t=%0t y=%0d\", $time, y); endtask\n\
  initial begin a = 1; #1 t(4'd2); end\n\
  initial begin #5 a = 3; end\n\
endmodule\n\
module top;\n\
  m u0(); m u1();\n\
  initial begin #9 $display(\"A end y0=%0d y1=%0d\", u0.y, u1.y); $finish; end\n\
endmodule\n\
",
        &[
            "W top.u1.t t=5 y=4",
            "W top.u0.t t=5 y=4",
            "A end y0=4 y1=4",
        ],
    );
}

/// Park (ENG-2 r5): `w7s` with the task called from an `always @*`. = PRE = iv = s2v; rounds 1-4
/// refused it (E3009). Cell `r4/lens-diff/b/B11_always_star_calls_auto`.
///
/// Raw lines: PRE `W t=5 y=4 | A end y=4`; iv `W t=5 y=4 | A end y=4`; s2v `W t=5 y=4 | A end
/// y=4`; vl `(verilator refused) | %Error-UNSUPPORTED: t.sv:4:64: Unsupported: Event control with
/// implicit sensitivity (@*) in this context; use an explicit sensitivity list instead`.
#[test]
fn b11_park_inbody_star_task_called_from_always_star() {
    pin(
        "module top;\n\
  logic [3:0] a, b, y;\n\
  function [3:0] sf2(input [3:0] x, input [3:0] z); sf2 = x + 4'd1; endfunction\n\
  task automatic t(input logic [3:0] v); logic [3:0] l; l = v; @(*) y = sf2(a, l); $display(\"W t=%0t y=%0d\", $time, y); endtask\n\
  always @* if (b == 4'd1) t(4'd2);\n\
  initial begin a = 1; b = 0; #1 b = 1; end\n\
  initial begin #5 a = 3; end\n\
  initial begin #9 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W t=5 y=4", "A end y=4"],
    );
}

/// Park (ENG-2 r5): with the ignored argument out of the set, the frame classify refuses the task
/// for its `disable fork`, as at PRE (rounds 1-4 refused it for the frame-local read first).
/// Loud on every tool that runs it. Cell `r4/lens-diff/b/B5b_disable_fork_ign`.
///
/// Raw lines: PRE `t.sv:1:8: error[VITA-E3009] E-ELAB-UNSUPPORTED: frame task `t` body uses a
/// `disable fork`. ...`; iv `Assertion failed: (child->wt_context==0 ||
/// thr->wt_context!=child->wt_context), function of_JOIN_DETACH, file vthread.cc, line 3793.`; s2v
/// `(sv2v/iverilog refused) | t.sv:6:23: Parse error: missing expected `join``; vl `(verilator
/// refused) | %Error-UNSUPPORTED: t.sv:7:5: Unsupported: Event control with implicit sensitivity
/// (@*) in this context; use an explicit sensitivity list instead`.
#[test]
fn b5b_park_disable_fork_refusal_keeps_its_text() {
    pin_loud(
        "module top;\n\
  logic [3:0] a, y, z;\n\
  function [3:0] sf2(input [3:0] x, input [3:0] q); sf2 = x + 4'd1; endfunction\n\
  task automatic t(input logic [3:0] v);\n\
    logic [3:0] l; l = v;\n\
    fork #7 z = 4'd1; join_none\n\
    @(*) y = sf2(a, l);\n\
    disable fork;\n\
    $display(\"W t=%0t y=%0d z=%0d\", $time, y, z);\n\
  endtask\n\
  initial begin a = 1; #1 t(4'd2); end\n\
  initial begin #5 a = 3; end\n\
  initial begin #9 $display(\"A end y=%0d z=%0d\", y, z); $finish; end\n\
endmodule\n\
",
        "E3009",
        "uses a `disable fork`",
    );
}

/// Park (ENG-2 r5): an in-body `@(*)` reading nothing inside a class method warns `VITA-W3056`
/// where it is lowered, then the method is refused for its event control — PRE's two lines
/// (round 4 kept only the refusal). Cell `r4/lens-sound/c/S13`.
///
/// Raw lines: PRE `t.sv:3:22: warning[VITA-W3056] W-ELAB-FEATURE-LIMIT: in-body @(*) reads no
/// nets; it can never wake [in $class$C$m] | error[VITA-E3009] E-ELAB-UNSUPPORTED: frame
/// function/task `m` body uses a timing/suspend/fork control ...`; iv `A end`; s2v `(sv2v/iverilog
/// refused) | t.sv:3:3: Parse error: missing expected `endmodule``; vl `(verilator refused) |
/// %Error-UNSUPPORTED: t.sv:3:22: Unsupported: Event control with implicit sensitivity (@*) in
/// this context; use an explicit sensitivity list instead`.
#[test]
fn s13_park_inbody_star_in_a_class_method_warns_then_refuses() {
    pin_loud_codes(
        "module top;\n\
  logic [3:0] y;\n\
  class C; task m(); @(*) $display(\"x\"); endtask endclass\n\
  C c;\n\
  initial begin c = new; c.m(); end\n\
  initial begin #7 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W3056", "E3009"],
        "uses a timing/suspend/fork control",
    );
}

/// Park (ENG-2 r5): the same in an automatic function body (illegal, IEEE 1800-2017 §13.4: a
/// function holds no timing control). PRE's two lines. Cell `r4/lens-sound/c/S14`.
///
/// Raw lines: PRE `t.sv:3:58: warning[VITA-W3056] W-ELAB-FEATURE-LIMIT: in-body @(*) reads no
/// nets; it can never wake [in top.$func$f] | t.sv:1:8: error[VITA-E3009] E-ELAB-UNSUPPORTED:
/// frame function/task `f` body uses a timing/suspend/fork control ...`; iv `(iverilog refused) |
/// t.sv:3: error: functions cannot have event statements.`; s2v `(sv2v/iverilog refused) |
/// s2v.v:7: error: functions cannot have event statements.`; vl `(verilator refused) | %Error:
/// t.sv:3:58: Event controls are not legal in functions. Suggest use a task (IEEE 1800-2023
/// 13.4.4)`.
#[test]
fn s14_park_inbody_star_in_an_automatic_function_warns_then_refuses() {
    pin_loud_codes(
        "module top;\n\
  logic [3:0] y, a;\n\
  function automatic logic [3:0] f(input logic [3:0] x); @(*) $display(\"x\"); f = x; endfunction\n\
  initial begin a = 1; y = f(a); end\n\
  initial begin #7 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &["W3056", "E3009"],
        "uses a timing/suspend/fork control",
    );
}

/// ENG-2 landing (r5 lens-sound N-MF5L): an `always_comb` calls an automatic `function void`
/// whose `fork st(); join_none` enables a static task that passes `m` to a hierarchical task's
/// output formal. That write of `m` is the task's, not the block's or a called function's (IEEE
/// 1800-2017 §9.2.2.2.1 :9055-9056: the contents of a task add nothing; a task call adds its
/// arguments only, and `st` has none), so `m[0]` stays in the block's set: the block wakes at t=5
/// and again on the task's write. Legal: §13.4.4 :14686-14692 lets a function's `fork ...
/// join_none` hold any statement legal in a task, and the block's statement is the call
/// (§9.2.2.2.2 :9085). Preservation pin, = PRE = round 4; mutant MF5 (a frame-nested call site
/// keyed in `rs.proc_calls`) drops `m[0]`: `C t=0 y=x | C t=0 y=7 | A end y=7 m0=1`. Cell
/// `r5/lens-sound/c/MK3`.
///
/// Raw lines: PRE `C t=0 y=x | C t=0 y=7 | C t=5 y=1 | C t=5 y=7 | A end y=7 m0=7`; iv
/// `(iverilog refused) | t.sv:1: sorry: Subroutine ports with unpacked dimensions are not yet
/// supported. | t.sv:6: error: Functions cannot enable/call tasks. | t.sv:6: error: A
/// fork/join_none statement cannot be synthesized in an always_comb process.`; s2v
/// `(sv2v/iverilog refused) | t.sv:6:43: Parse error: missing expected `join``; vl `C t=0 y=0 |
/// A end y=0 m0=1` (2-state; the forked task's write of 7 never shows).
#[test]
fn mk3_comb_keeps_a_read_a_function_forked_task_writes() {
    pin(
        "module sub; task automatic tko(output logic [3:0] a [2]); a[0] = 7; a[1] = 7; endtask endmodule\n\
module top;\n\
  logic [3:0] m [2]; logic [3:0] y;\n\
  sub u();\n\
  task st(); u.tko(m); endtask\n\
  function automatic void f(); fork st(); join_none endfunction\n\
  always_comb begin f(); y = m[0]; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 m[0] = 1; #2 $display(\"A end y=%0d m0=%0d\", y, m[0]); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 y=x",
            "C t=0 y=7",
            "C t=5 y=1",
            "C t=5 y=7",
            "A end y=7 m0=7",
        ],
    );
}

/// ENG-2 landing (r5 lens-sound N-MF5L): the input twin. The forked static task passes `m` to a
/// hierarchical task's input formal; the block reads only `k`, so a change of `m[1]` at t=5 does
/// not wake it (§9.2.2.2.1 :9055-9056: the task call adds its arguments only, `st` has none) and
/// `u.q` keeps the sum taken at t=0. Preservation pin, = PRE = round 4; mutant MF5 wakes the
/// block on `m`: `C t=0 y=1 | C t=0 y=1 | C t=5 y=1 | A end y=1 q=5`. Cell
/// `r5/lens-sound/c/MK1`.
///
/// Raw lines: PRE `C t=0 y=1 | C t=0 y=1 | A end y=1 q=3`; iv `(iverilog refused) | t.sv:1:
/// sorry: Subroutine ports with unpacked dimensions are not yet supported. | t.sv:6: error:
/// Functions cannot enable/call tasks. | t.sv:5: error: Array m needs an array index here.`; s2v
/// `(sv2v/iverilog refused) | t.sv:6:43: Parse error: missing expected `join``; vl `C t=0 y=1`
/// 10001 times, then `%Error-DIDNOTCONVERGE: Active region did not converge after
/// '--converge-limit' of 10000 tries` (it re-runs the block on the forked task's activity).
#[test]
fn mk1_comb_does_not_read_a_function_forked_task_input() {
    pin(
        "module sub; logic [3:0] q; task automatic tk(input logic [3:0] a [2]); q = a[0] + a[1]; endtask endmodule\n\
module top;\n\
  logic [3:0] m [2]; logic [3:0] y, k;\n\
  sub u();\n\
  task st(); u.tk(m); endtask\n\
  function automatic void f(); fork st(); join_none endfunction\n\
  always_comb begin f(); y = k; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin k = 1; m[0] = 1; m[1] = 2; #5 m[1] = 4; #2 $display(\"A end y=%0d q=%0d\", y, u.q); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=1", "C t=0 y=1", "A end y=1 q=3"],
    );
}
