//! IEEE §9.2.2.2/§9.2.2.3/§9.2.2.4: a variable written by `always_comb`,
//! `always_ff` or `always_latch` "shall not be written by any other process".
//!
//! Every row here is a measured oracle cell, not a design opinion — verilator
//! 5.052 `--lint-only` (MULTIDRIVEN) and an external xcelium report (`*E,MULAXX`);
//! iverilog says nothing about any of them. The rows split three ways:
//!
//! - both tools reject               -> `VITA-E3001` E-ELAB-MULTIDRIVER
//! - only xcelium rejects            -> `VITA-W3060` W-ELAB-MULTIDRIVER-STRICT
//! - both tools accept               -> silent
//!
//! The silent rows are the teeth: a diagnostic on RTL every tool accepts is a
//! regression, so `always` + `always`, a clock generator, a write reaching the
//! variable only through a TASK CALL, and every pair where either side writes only a
//! PART of the variable are each pinned as producing nothing.

use std::process::Command;

/// Run one design through one-shot `vita` and return its combined output.
fn run(name: &str, body: &str, extra: &[&str]) -> (Option<i32>, String) {
    let dir = std::env::temp_dir().join(format!("vita_mdp_{}_{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sv = dir.join("d.sv");
    std::fs::write(&sv, body).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .current_dir(&dir)
        .args(extra)
        .arg(&sv)
        .output()
        .expect("run vita");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code(), text)
}

/// The design errored with `VITA-E3001` naming `var`.
fn expect_multidriver(name: &str, body: &str, var: &str, a: &str, b: &str) {
    let (rc, text) = run(name, body, &[]);
    assert_eq!(rc, Some(1), "expected an elaborate error, got:\n{text}");
    let want = format!("variable `{var}` is written by {a} AND by {b}");
    assert!(
        text.contains("VITA-E3001") && text.contains(&want),
        "wanted E3001 `{want}`, got:\n{text}"
    );
}

/// The design produced neither the error nor the warning.
fn expect_silent(name: &str, body: &str) {
    let (rc, text) = run(name, body, &[]);
    assert_eq!(rc, Some(0), "expected a clean run, got:\n{text}");
    assert!(
        !text.contains("VITA-E3001") && !text.contains("VITA-W3060"),
        "expected no driver diagnostic, got:\n{text}"
    );
}

// ── Rule B: an always_* writer plus ANY other module-scope writer ────────────

#[test]
fn initial_plus_always_ff_is_an_error() {
    expect_multidriver(
        "n",
        "module t(input logic clk);\n  int n; initial n = 0;\n  always_ff @(posedge clk) n <= n + 1;\nendmodule\n",
        "n",
        "`always_ff`",
        "`initial`",
    );
}

#[test]
fn initial_plus_always_ff_incdec_is_an_error() {
    // `m++` is parsed as a blocking assignment, so the direct-write walk sees it.
    expect_multidriver(
        "m",
        "module t(input logic clk);\n  int m; initial m = 0;\n  always_ff @(posedge clk) m++;\nendmodule\n",
        "m",
        "`always_ff`",
        "`initial`",
    );
}

#[test]
fn two_always_ff_on_one_variable_is_an_error() {
    expect_multidriver(
        "ff2",
        "module t(input logic clk);\n  logic [3:0] ff2;\n  always_ff @(posedge clk) ff2 <= 1;\n  always_ff @(negedge clk) ff2 <= 2;\nendmodule\n",
        "ff2",
        "`always_ff`",
        "`always_ff`",
    );
}

#[test]
fn always_ff_plus_plain_always_is_an_error() {
    expect_multidriver(
        "ffa",
        "module t(input logic clk);\n  logic [3:0] ffa;\n  always_ff @(posedge clk) ffa <= 1;\n  always @(negedge clk) ffa <= 2;\nendmodule\n",
        "ffa",
        "`always_ff`",
        "`always`",
    );
}

#[test]
fn initial_plus_always_comb_is_an_error() {
    expect_multidriver(
        "cmi",
        "module t(input logic a);\n  logic [3:0] cmi; initial cmi = 0;\n  always_comb cmi = a;\nendmodule\n",
        "cmi",
        "`always_comb`",
        "`initial`",
    );
}

#[test]
fn initial_plus_always_latch_is_a_warning() {
    // verilator is SILENT on every `always_latch` pair measured, while reporting
    // MULTIDRIVEN for the `always_comb` and `always_ff` twins. xcelium rejects it.
    // The severity follows the tools, not IEEE §9.2.2.3's wording, so this row is
    // the warning and its two neighbours are errors.
    let (rc, text) = run(
        "lti",
        "module t(input logic a);\n  logic [3:0] lti; initial lti = 0;\n  always_latch if (a) lti = 1;\n  initial #1 $finish;\nendmodule\n",
        &[],
    );
    assert_eq!(rc, Some(0), "expected a clean run, got:\n{text}");
    assert!(
        text.contains("VITA-W3060")
            && text.contains("variable `lti` is written by `always_latch` AND by `initial`"),
        "got:\n{text}"
    );
    assert!(!text.contains("VITA-E3001"), "got:\n{text}");
}

/// The control twin for the row above: the same pair with `always_comb` in place of
/// `always_latch` IS an error, so the demotion is scoped to the latch and not to the
/// `initial` half of the pair.
#[test]
fn always_latch_demotion_does_not_reach_always_comb() {
    expect_multidriver(
        "ltic",
        "module t(input logic a);\n  logic [3:0] x; initial x = 0;\n  always_comb x = a;\nendmodule\n",
        "x",
        "`always_comb`",
        "`initial`",
    );
}

#[test]
fn always_comb_plus_continuous_assign_is_an_error() {
    expect_multidriver(
        "cfa",
        "module t(input logic a);\n  logic [3:0] cfa;\n  always_comb cfa = a;\n  assign cfa = 1;\nendmodule\n",
        "cfa",
        "`always_comb`",
        "a continuous `assign`",
    );
}

#[test]
fn always_ff_plus_final_is_an_error() {
    expect_multidriver(
        "fin",
        "module t(input logic clk);\n  logic [3:0] fin;\n  always_ff @(posedge clk) fin <= 1;\n  final fin = 0;\nendmodule\n",
        "fin",
        "`always_ff`",
        "`final`",
    );
}

// ── Rule A: a declaration initializer plus `always_comb` (unchanged) ─────────
//
// Rule A runs FIRST and owns the variable: an initialized variable with two
// `always_comb` writers is ONE diagnostic, the initializer one, not that plus a
// Rule B line at the same caret.

#[test]
fn decl_initializer_plus_always_comb_is_an_error() {
    let (rc, text) = run(
        "cmb",
        "module t;\n  logic [3:0] cmb = 0;\n  always_comb cmb = 4'd1;\nendmodule\n",
        &[],
    );
    assert_eq!(rc, Some(1), "got:\n{text}");
    assert!(
        text.contains("VITA-E3001")
            && text.contains(
                "variable `cmb` has a declaration initializer AND is written by `always_comb`"
            ),
        "got:\n{text}"
    );
}

// ── Rule C: a declaration initializer plus always_ff / always_latch = WARNING ─

/// The design ran clean (exit 0) and emitted `VITA-W3060` naming `var` and `kind`.
fn expect_strict_warning(name: &str, body: &str, var: &str, kind: &str) {
    let (rc, text) = run(name, body, &[]);
    assert_eq!(rc, Some(0), "expected a clean run, got:\n{text}");
    let want = format!("variable `{var}` has a declaration initializer AND is written by `{kind}`");
    assert!(
        text.contains("VITA-W3060") && text.contains(&want),
        "wanted W3060 `{want}`, got:\n{text}"
    );
    // …and it is a warning, not an error.
    assert!(!text.contains("VITA-E3001"), "got:\n{text}");
}

#[test]
fn decl_initializer_plus_always_ff_blocking_is_a_warning() {
    expect_strict_warning(
        "nblk",
        "module t(input logic clk);\n  logic [3:0] n_blk = 0;\n  always_ff @(posedge clk) n_blk = n_blk + 1;\n  initial #1 $finish;\nendmodule\n",
        "n_blk",
        "always_ff",
    );
}

#[test]
fn decl_initializer_plus_always_ff_nonblocking_is_a_warning() {
    expect_strict_warning(
        "qnba",
        "module t(input logic clk);\n  logic [3:0] q_nba = 0;\n  always_ff @(posedge clk) q_nba <= q_nba + 1;\n  initial #1 $finish;\nendmodule\n",
        "q_nba",
        "always_ff",
    );
}

#[test]
fn decl_initializer_plus_always_latch_is_a_warning() {
    expect_strict_warning(
        "lat",
        "module t(input logic clk);\n  logic [3:0] lat = 0;\n  always_latch if (clk) lat = 4'd2;\n  initial #1 $finish;\nendmodule\n",
        "lat",
        "always_latch",
    );
}

#[test]
fn strict_warning_is_suppressible() {
    let body = "module t(input logic clk);\n  logic [3:0] q_nba = 0;\n  always_ff @(posedge clk) q_nba <= q_nba + 1;\n  initial #1 $finish;\nendmodule\n";
    let (rc, text) = run("sup", body, &["-Wno-W-ELAB-MULTIDRIVER-STRICT"]);
    assert_eq!(rc, Some(0), "got:\n{text}");
    assert!(!text.contains("VITA-W3060"), "got:\n{text}");
}

// ── The silent rows: both oracles accept, so vita must say nothing ───────────

#[test]
fn initial_plus_plain_always_is_silent() {
    // No `always_*` procedure is involved, so no §9.2.2.x clause applies.
    expect_silent(
        "nok",
        "module t(input logic clk);\n  logic n_ok; initial n_ok = 0;\n  always @(posedge clk) n_ok <= ~n_ok;\n  initial #1 $finish;\nendmodule\n",
    );
}

#[test]
fn clock_generator_initializer_is_silent() {
    // `logic c = 0; always #5 c = ~c;` is the clock every testbench has.
    expect_silent(
        "clkgen",
        "module t;\n  logic c = 0;\n  always #5 c = ~c;\n  initial #20 $finish;\nendmodule\n",
    );
}

#[test]
fn write_through_a_task_call_is_silent() {
    // The precision rule: only an lvalue ROOT counts as a write here. `initial
    // wt();` reaches `tk` through the task BODY, and the conservative
    // definite-assignment walk would also call `foo(tk)` with an INPUT formal a
    // write — verilator calls neither pair MULTIDRIVEN.
    expect_silent(
        "task",
        "module t(input logic clk);\n  logic [3:0] tk;\n  task automatic wt(); tk = 3; endtask\n  always_ff @(posedge clk) tk <= 1;\n  initial wt();\n  initial #1 $finish;\nendmodule\n",
    );
}

#[test]
fn a_block_local_shadow_is_silent() {
    // The SHADOW guard: the `always_ff` declares its OWN `n`, so the module-scope
    // `n` has exactly one writer. iverilog, verilator and xrun all accept this.
    expect_silent(
        "shadow",
        "module t(input logic clk);\n  int n; initial n = 7;\n  always_ff @(posedge clk) begin\n    int n;\n    n = 3;\n  end\n  initial #1 $finish;\nendmodule\n",
    );
}

#[test]
fn a_single_always_ff_writer_is_silent() {
    // The control twin for every row above: one writer, no initializer.
    expect_silent(
        "sole",
        "module t(input logic clk);\n  logic [3:0] q;\n  always_ff @(posedge clk) q <= q + 1;\n  initial #1 $finish;\nendmodule\n",
    );
}

// ── Both writers must write the WHOLE variable ──────────────────────────────
//
// Measured, verilator 5.052 `--lint-only` is silent on every pair where at least one
// side writes a struct member, an array element, a bit or a part select — ten shapes,
// `always_ff` against `always_ff` and `initial` against `always_ff`, whole-against-
// partial in both directions. Its rule is "both writers write the whole variable".
// Xcelium on a partial write is UNMEASURED (zero observations, not "accepts"), so vita
// follows the one tool that was run.
//
// The two positive controls come first: with BOTH sides whole, verilator does report
// MULTIDRIVEN, including for `force` and for a procedural `assign`. Without them the
// silent rows below would pass on a check that had simply stopped working.

#[test]
fn force_from_initial_beside_always_ff_is_an_error() {
    expect_multidriver(
        "fr",
        "module t(input logic clk);\n  logic [3:0] fr;\n  always_ff @(posedge clk) fr <= 1;\n  initial begin force fr = 4'd2; #10 release fr; end\nendmodule\n",
        "fr",
        "`always_ff`",
        "`initial`",
    );
}

#[test]
fn procedural_assign_from_initial_beside_always_ff_is_an_error() {
    expect_multidriver(
        "pca",
        "module t(input logic clk);\n  logic [3:0] pca;\n  always_ff @(posedge clk) pca <= 1;\n  initial assign pca = 3;\nendmodule\n",
        "pca",
        "`always_ff`",
        "`initial`",
    );
}

/// Both writers are `always_ff`, and each writes a different PACKED STRUCT MEMBER.
/// The parser desugars `s.x` to a part-select, so this is a partial write.
#[test]
fn two_always_ff_on_distinct_struct_members_is_silent() {
    expect_silent(
        "smem",
        "module t(input logic clk, input logic d);\n  typedef struct packed { logic x; logic y; } st;\n  st s;\n  always_ff @(posedge clk) s.x <= d;\n  always_ff @(negedge clk) s.y <= d;\n  initial #1 $finish;\nendmodule\n",
    );
}

/// Two `always_ff` writing different, DYNAMIC array elements.
#[test]
fn two_always_ff_on_array_elements_is_silent() {
    expect_silent(
        "amem",
        "module t(input logic clk, input logic [1:0] a);\n  logic [7:0] mem [4];\n  always_ff @(posedge clk) mem[a] <= 1;\n  always_ff @(negedge clk) mem[a+1] <= 2;\n  initial #1 $finish;\nendmodule\n",
    );
}

/// Two `always_ff` writing different BIT SELECTS of one vector.
#[test]
fn two_always_ff_on_distinct_bit_selects_is_silent() {
    expect_silent(
        "bits",
        "module t(input logic clk, input logic d);\n  logic [3:0] bits;\n  always_ff @(posedge clk) bits[0] <= d;\n  always_ff @(negedge clk) bits[1] <= d;\n  initial #1 $finish;\nendmodule\n",
    );
}

/// An `initial` LOOP writing every element, beside an `always_ff` element write. The
/// `for` init and step are walked exactly like any other statement, so the loop body's
/// `m1[i] = 0` is a partial write and neither side counts.
#[test]
fn initial_element_loop_beside_always_ff_element_is_silent() {
    expect_silent(
        "m1",
        "module t(input logic clk, input logic [1:0] a);\n  logic [7:0] m1 [4];\n  initial for (int i=0;i<4;i++) m1[i]=0;\n  always_ff @(posedge clk) m1[a] <= 1;\n  initial #1 $finish;\nendmodule\n",
    );
}

/// WHOLE on the `initial` side, PARTIAL on the `always_ff` side.
#[test]
fn whole_initial_beside_partial_always_ff_is_silent() {
    expect_silent(
        "m2",
        "module t(input logic clk, input logic [1:0] a);\n  logic [7:0] m2 [4];\n  initial m2 = '{default:0};\n  always_ff @(posedge clk) m2[a] <= 1;\n  initial #1 $finish;\nendmodule\n",
    );
}

/// …and the same pair the other way round: PARTIAL `initial`, WHOLE `always_ff`. Both
/// directions are pinned because a one-sided predicate would pass one and fail the other.
#[test]
fn partial_initial_beside_whole_always_ff_is_silent() {
    expect_silent(
        "m3",
        "module t(input logic clk);\n  logic [7:0] m3 [4];\n  initial m3[0] = 0;\n  always_ff @(posedge clk) m3 <= '{default:1};\n  initial #1 $finish;\nendmodule\n",
    );
}

/// A struct member from `always_ff` beside the other member from `initial`.
#[test]
fn always_ff_struct_member_beside_initial_member_is_silent() {
    expect_silent(
        "sinit",
        "module t(input logic clk, input logic d);\n  typedef struct packed { logic x; logic y; } st;\n  st s;\n  always_ff @(posedge clk) s.x <= d;\n  initial s.y = 0;\n  initial #1 $finish;\nendmodule\n",
    );
}

/// A BIT select from `always_ff` beside a WHOLE write from `initial`.
#[test]
fn partial_always_ff_bit_beside_whole_initial_is_silent() {
    expect_silent(
        "b1",
        "module t(input logic clk, input logic d);\n  logic [3:0] b1;\n  always_ff @(posedge clk) b1[0] <= d;\n  initial b1 = 0;\n  initial #1 $finish;\nendmodule\n",
    );
}

/// Two DIFFERENT bit selects, one per process.
#[test]
fn two_processes_on_distinct_bits_is_silent() {
    expect_silent(
        "b2",
        "module t(input logic clk, input logic d);\n  logic [3:0] b2;\n  always_ff @(posedge clk) b2[0] <= d;\n  initial b2[1] = 0;\n  initial #1 $finish;\nendmodule\n",
    );
}

/// A WHOLE `always_ff` write beside a PARTIAL `initial` write.
#[test]
fn whole_always_ff_beside_partial_initial_is_silent() {
    expect_silent(
        "w",
        "module t(input logic clk);\n  logic [3:0] w;\n  always_ff @(posedge clk) w <= 1;\n  initial w[1] = 0;\n  initial #1 $finish;\nendmodule\n",
    );
}

/// `$readmemh` fills the array from an `initial`, and an `always_ff` writes one
/// element. The system task is a call, so it is not a write here at all — and the
/// `always_ff` side is partial, so the pair is silent twice over.
#[test]
fn readmemh_beside_always_ff_element_is_silent() {
    expect_silent(
        "rm",
        "module t(input logic clk, input logic [1:0] a);\n  logic [7:0] rm [4];\n  initial $readmemh(\"x.hex\", rm);\n  always_ff @(posedge clk) rm[a] <= 1;\n  initial #1 $finish;\nendmodule\n",
    );
}

/// Rule C takes the same walk: a declaration initializer beside a PARTIAL `always_ff`
/// write is not the power-on pair the warning is about.
#[test]
fn decl_initializer_beside_partial_always_ff_is_silent() {
    expect_silent(
        "cinit",
        "module t(input logic clk, input logic d);\n  logic [3:0] pi = 0;\n  always_ff @(posedge clk) pi[0] <= d;\n  initial #1 $finish;\nendmodule\n",
    );
}
