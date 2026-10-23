//! ENG-0: controls for ENG-2 and ENG-3 (implicit sensitivity) — the implicit-sensitivity shapes vita
//! already runs right, pinned so a change to the read set (ENG-2, variable granularity) or to the
//! wake region (ENG-3, longest-static-prefix granularity) cannot move them unseen. The rule ids
//! (R2.x, R3.x) are the ENG-2 / ENG-3 rules each control guards:
//!
//! * `always @*` (IEEE 1800-2017 §9.4.2.2): the identifiers in the statement, whole (C01); call
//!   arguments, not a function body (C04, C07d) nor a static function's formal written by another
//!   caller (G02); assertion action blocks included (C08);
//! * `always_comb` (§9.2.2.2.1): the body of every function it calls, static, automatic, void or
//!   value (C05, C06, C07c, C07f), minus what the function declares (G01's shared formal); a task
//!   adds its arguments only (C22a); a variable index ends the prefix (C14); exception b) drops only
//!   what the block writes (C26b: `v[0]` written, `v[1]` read);
//! * a static deferred assertion is an `always_comb` (§16.4.3) whose condition reads the function it
//!   calls (G07);
//! * the row-30 pingpong `own` and `single` cells (`crates/testdata/cells/r9/`, not in the cells
//!   manifest, so not run by the cells harness).
//!
//! Oracles: Icarus Verilog 13.0 `-g2012` (iv), sv2v 0.0.13 -> iverilog (s2v), Verilator 5.052
//! `--binary --timing` (vl). vl is no oracle for wake times here: it runs every header block at time
//! 0, prints a woken `always_comb` / `@*` line twice and after `$finish`, and never wakes the `@*` of
//! C01 / C04; its lines are named only where they bear on a value. Each cell's comment names the
//! oracle lines that decide it; "hand-IEEE" cites the clause. Every cell runs on native, vm and
//! interp and pins the printed lines, values and times.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn scratch(tag: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_isc_{tag}_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// `src` on one backend: its stdout lines (no `simulation ended` line), after a clean exit.
fn run_on(src: &str, backend: &str) -> Vec<String> {
    let d = scratch(backend);
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["--backend", backend, "t.sv"])
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let code = out.status.code().unwrap_or(-1);
    assert_eq!(
        code,
        0,
        "{backend}: exit {code}\n{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    stdout
        .lines()
        .filter(|l| !l.starts_with("simulation ended"))
        .map(str::to_string)
        .collect()
}

/// Every backend prints exactly `want`.
fn pin(src: &str, want: &[&str]) {
    for be in ["native", "vm", "interp"] {
        assert_eq!(run_on(src, be), want, "backend {be}");
    }
}

/// R3.3: `always @*` waits on the whole identifier `v`, not the prefix `v[0]` (IEEE 1800-2017 §9.4.2.2):
/// the t=5 write of `v[1]` wakes it. iv = s2v = PRE.
/// Cell `C01_star_sel`.
#[test]
fn c01_star_select_wakes_on_the_whole_identifier() {
    pin(
        "module top;\n\
  logic [1:0] v = 0; logic y;\n\
  always @* begin y = v[0]; $display(\"C t=%0t\", $time); end\n\
  initial begin #5 v[1] = 1; #3 v[0] = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=5", "C t=8", "A end"],
    );
}

/// R2.5: `always @*` takes the identifiers in its statement, not the body of an automatic function it calls
/// (§9.4.2.2): the t=6 write of `g` (read only inside `f`) does not wake it. iv = s2v = PRE.
/// Cell `C04_star_afn`.
#[test]
fn c04_star_does_not_read_an_automatic_function_body() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y;\n\
  function automatic logic f(); return g; endfunction\n\
  always @* begin y = a ^ f(); $display(\"C t=%0t\", $time); end\n\
  initial begin #5 a = 1; #1 g = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=5", "A end"],
    );
}

/// R2.5: `always @*` does not read the body of an automatic void function called as a statement (§9.4.2.2):
/// the t=6 write of `g` does not wake it, `z` stays 0 (§9.2.2.2.2 "always @* is only sensitive to changes to the
/// arguments of a function"). iv = s2v = PRE; vl "C t=6 z=1 | A end z=1" (it reads the body; no oracle here).
/// Cell `C07d_star_voidfn_auto_read`.
#[test]
fn c07d_star_does_not_read_a_void_function_body() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y, z;\n\
  function automatic void upd(); z = g; endfunction\n\
  always @* begin y = a; upd(); $display(\"C t=%0t z=%0d\", $time, z); end\n\
  initial begin #5 a = 1; #1 g = 1; #2 $display(\"A end z=%0d\", z); $finish; end\n\
endmodule\n\
",
        &["C t=5 z=0", "A end z=0"],
    );
}

/// R2.5: `always @*` adds the identifiers of an immediate assertion's action block (they appear in the
/// statement, §9.4.2.2; the always_comb exclusion of §9.2.2.2.1 does not apply): the t=6 write of `k` wakes it.
/// iv = PRE; s2v drops the assertion (prints only `A end`); vl wakes at t=5 and t=6 (each line twice).
/// Cell `C08_star_action`.
#[test]
fn c08_star_reads_an_assertion_action_block() {
    pin(
        "module top;\n\
  logic a = 0; int k = 0;\n\
  always @* assert (a == 0) else $display(\"C t=%0t k=%0d\", $time, k);\n\
  initial begin #5 a = 1; #1 k = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=5 k=0", "C t=6 k=1", "A end"],
    );
}

/// R2.5: two `always @*` call one static function; the second caller's call `f(b)` writes the shared formal
/// `x`, which is not an identifier of the first statement (§9.4.2.2): the t=5 write of `b` does not wake
/// the first block. iv = s2v = PRE.
/// Cell `G02_star_sfn_shared_formal`.
#[test]
fn g02_star_ignores_a_static_function_formal_another_caller_writes() {
    pin(
        "module top;\n\
  logic a = 0, b = 0, ya, yb;\n\
  function logic f(input logic x); f = ~x; endfunction\n\
  always @* begin ya = f(a); $display(\"A t=%0t\", $time); end\n\
  always @* begin yb = f(b); end\n\
  initial begin #5 b = 1; #3 a = 1; #2 $display(\"E end\"); $finish; end\n\
endmodule\n\
",
        &["A t=8", "E end"],
    );
}

/// R2.1: `always_comb` reads the body of a static function it calls (§9.2.2.2.1): the t=6 write of `g`
/// wakes it. iv = s2v = PRE.
/// Cell `C05_comb_sfn`.
#[test]
fn c05_comb_reads_a_static_function_body() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y;\n\
  function logic f(); f = g; endfunction\n\
  always_comb begin y = a ^ f(); $display(\"C t=%0t\", $time); end\n\
  initial begin #5 a = 1; #1 g = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0", "C t=5", "C t=6", "A end"],
    );
}

/// R2.1: `always_comb` reads the body of an automatic function it calls (§9.2.2.2.1): the t=6 write of
/// `g` wakes it. iv = s2v = PRE.
/// Cell `C06_comb_afn`.
#[test]
fn c06_comb_reads_an_automatic_function_body() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y;\n\
  function automatic logic f(); return g; endfunction\n\
  always_comb begin y = a ^ f(); $display(\"C t=%0t\", $time); end\n\
  initial begin #5 a = 1; #1 g = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0", "C t=5", "C t=6", "A end"],
    );
}

/// R2.1: `always_comb` reads the body of a static void function called as a statement (§9.2.2.2.1): the
/// t=6 write of `g` wakes it and `z` follows. iv = s2v = PRE.
/// Cell `C07c_comb_voidfn_static_read`.
#[test]
fn c07c_comb_reads_a_static_void_function_body() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y, z;\n\
  function void upd(); z = g; endfunction\n\
  always_comb begin y = a; upd(); $display(\"C t=%0t z=%0d\", $time, z); end\n\
  initial begin #5 a = 1; #1 g = 1; #2 $display(\"A end z=%0d\", z); $finish; end\n\
endmodule\n\
",
        &["C t=0 z=0", "C t=5 z=0", "C t=6 z=1", "A end z=1"],
    );
}

/// R2.1: `always_comb` reads the body of an automatic value function (§9.2.2.2.1): with `a` back at 0, the
/// t=7 write of `g` wakes it and `y` becomes 1. iv = s2v = PRE.
/// Cell `C07f_comb_valfn_read`.
#[test]
fn c07f_comb_reads_a_value_function_body() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, y;\n\
  function automatic logic rd(); return g; endfunction\n\
  always_comb begin y = a | rd(); $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 a = 1; #1 a = 0; #1 g = 1; #2 $display(\"A end y=%0d\", y); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 y=0",
            "C t=5 y=1",
            "C t=6 y=0",
            "C t=7 y=1",
            "A end y=1",
        ],
    );
}

/// R2.1 / exception a): two `always_comb` call one static function; the formal `x` is declared within the
/// function (§9.2.2.2.1 a), so the second caller's t=5 write of it (from `b`) does not wake the first block.
/// iv = s2v = PRE.
/// Cell `G01_comb_sfn_shared_formal`.
#[test]
fn g01_comb_ignores_a_static_function_formal_another_caller_writes() {
    pin(
        "module top;\n\
  logic a = 0, b = 0, ya, yb;\n\
  function logic f(input logic x); f = ~x; endfunction\n\
  always_comb begin ya = f(a); $display(\"A t=%0t\", $time); end\n\
  always_comb begin yb = f(b); end\n\
  initial begin #5 b = 1; #3 a = 1; #2 $display(\"E end\"); $finish; end\n\
endmodule\n\
",
        &["A t=0", "A t=8", "E end"],
    );
}

/// R2.2: an automatic task called in `always_comb` adds its argument `a` only, not its body (§9.2.2.2.1
/// "the contents of the tasks do not add anything"): the t=6 write of `g` does not wake it, `o` stays 0.
/// iv = s2v = PRE; vl "C t=0 | A end o=1" (no t=5 line though the argument `a` changed, yet `o` follows
/// the task body's `g`: no oracle here).
/// Cell `C22a_comb_autotask_read`.
#[test]
fn c22a_comb_does_not_read_an_automatic_task_body() {
    pin(
        "module top;\n\
  logic a = 0, g = 0, o;\n\
  task automatic t(input logic x); o = x & g; endtask\n\
  always_comb begin t(a); $display(\"C t=%0t\", $time); end\n\
  initial begin #5 a = 1; #1 g = 1; #2 $display(\"A end o=%0d\", o); $finish; end\n\
endmodule\n\
",
        &["C t=0", "C t=5", "A end o=0"],
    );
}

/// R3.1: a variable index ends the longest static prefix (§11.5.3), so `v[i +: 2]` contributes all of `v`
/// (§9.2.2.2.1): the t=5 write of `v[0]`, outside the selected bits, wakes it. iv = s2v = PRE.
/// Cell `C14_comb_varpart`.
#[test]
fn c14_comb_variable_part_select_waits_on_the_whole_vector() {
    pin(
        "module top;\n\
  logic [3:0] v = 0; int i = 2; logic [1:0] y;\n\
  always_comb begin y = v[i +: 2]; $display(\"C t=%0t\", $time); end\n\
  initial begin #5 v[0] = 1; #3 v[3] = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0", "C t=5", "C t=8", "A end"],
    );
}

/// R2.6 / R3.2 exception b): the block writes `v[0]` and reads `v[1]`; `v[1]` is not an expression the
/// block writes (§9.2.2.2.1 b, ieee1800-2017.txt:9045-9050), so the t=5 write of `v[1]` wakes it; the t=8
/// line is the write of `a`, which the block reads (`v[0] = a`), and stays under R3.2 (only `v` narrows).
/// hand-IEEE = s2v = PRE; iv deviates ("C t=0 y=x | C t=8 y=1", whole-variable exclusion, and
/// "sorry: constant selects in always_* processes are not fully supported"); vl two-state (`y=0` at t=0).
/// Cell `C26b_comb_write_other_read`.
#[test]
fn c26b_comb_reads_the_part_it_does_not_write() {
    pin(
        "module top;\n\
  logic [1:0] v; logic a = 0, y;\n\
  always_comb begin v[0] = a; y = v[1]; $display(\"C t=%0t y=%0d\", $time, y); end\n\
  initial begin #5 v[1] = 1; #3 a = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0 y=x", "C t=5 y=1", "C t=8 y=1", "A end"],
    );
}

/// R2.1 / R2.3: a static deferred assertion is treated as an `always_comb` (§16.4.3,
/// ieee1800-2017.txt:16690-16692); its condition adds as an `if` condition, with the body of the function it
/// calls (§9.2.2.2.1): the t=5 write of `g` wakes it and the check fails. hand-IEEE = PRE; vl wakes at t=5
/// ("P t=0 | F t=5 | F t=5 | E end" then post-`$finish` lines); iv "sorry: Deferred assertions are not
/// supported"; s2v drops the assertion (`E end` only).
/// Cell `G07_item_cond_afn_read`.
#[test]
fn g07_static_deferred_assertion_reads_its_condition_function() {
    pin(
        "module top;\n\
  logic g = 0;\n\
  function automatic logic ok(); return g == 0; endfunction\n\
  a1: assert #0 (ok()) $display(\"P t=%0t\", $time); else $display(\"F t=%0t\", $time);\n\
  initial begin #5 g = 1; #2 $display(\"E end\"); $finish; end\n\
endmodule\n\
",
        &["P t=0", "F t=5", "E end"],
    );
}

/// row 30 control (cell r9__B-veer__repros__always_comb_shared_vector_pingpong__own): each generate bank
/// writes its own temporary `a` before reading it (exception b) and one element of `any`; the t=1 write of
/// `m` wakes both banks. iv = s2v = vl = PRE `any=11`.
/// Cell `PP_own`.
#[test]
fn pingpong_own_temporary_per_bank() {
    pin(
        "module top;\n\
  logic [1:0]      any;\n\
  logic [1:0][1:0] m;\n\
  for (genvar k = 0; k < 2; k++) begin : BANKS\n\
    logic a;                     // per-iteration temporary, then one write of the shared vector\n\
    always_comb begin\n\
      a = '0;\n\
      for (int l = 0; l < 2; l++) a |= m[k][l];\n\
      any[k] = a;\n\
    end\n\
  end\n\
  initial begin #1 m = 4'b0110; #1 $display(\"any=%b\", any); $finish; end\n\
endmodule\n\
",
        &["any=11"],
    );
}

/// row 30 control (cell r9__B-veer__repros__always_comb_shared_vector_pingpong__single): one block writes
/// `any[k]` then reads it through `|=` (exception b) and reads `m`; the t=1 write of `m` wakes it.
/// s2v = vl = PRE `any=11`; iv rejects the variable index (`k` not constant).
/// Cell `PP_single`.
#[test]
fn pingpong_single_process_writes_both_elements() {
    pin(
        "module top;\n\
  logic [1:0]      any;\n\
  logic [1:0][1:0] m;\n\
  always_comb begin              // one process writes both elements\n\
    for (int k = 0; k < 2; k++) begin\n\
      any[k] = '0;\n\
      for (int l = 0; l < 2; l++) any[k] |= m[k][l];\n\
    end\n\
  end\n\
  initial begin #1 m = 4'b0110; #1 $display(\"any=%b\", any); $finish; end\n\
endmodule\n\
",
        &["any=11"],
    );
}
