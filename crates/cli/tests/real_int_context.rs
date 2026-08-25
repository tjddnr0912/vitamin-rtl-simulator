//! A `real` constant reaching an INTEGER context — `logic [int'(R)-1:0]`,
//! `$clog2(R)`, `{int'(R){1'b1}}`, `localparam int M = R*2.0`.
//!
//! §0 T2 item 8's residue. Real ARITHMETIC folds (§4.5.232) and so do the
//! generate-scope and generate-control spellings (§4.5.241/242), but every
//! position that wants an INTEGER out of that real stayed loud — and vita's own
//! runtime has answered these correctly all along (`lower_real_to_int_cast`).
//!
//! §4.5.232 tried to close it by registering an i64 TWIN for a real parameter and
//! withdrew over five silent-wrongs: a twin lets the INTEGER domain succeed on an
//! expression that mentions a real, and only the real domain applies §11.8.1's
//! "any real operand ⇒ evaluate in the real domain", so `generate if (R/2 > 2)`
//! with R = 5.0 took the ELSE branch. That withdrawal named its own remedy —
//! *"routing those sites through the real domain, which is its own slice"* — and
//! this is that slice: the expression folds WHOLE in the real domain and only its
//! rounded RESULT crosses over, at the node the language already calls a
//! conversion. `real_domain_still_owns_a_generate_condition` pins the shape the
//! twin broke.
//!
//! ⚠️ An IMPLICIT conversion is deliberately NOT covered, and it is not a gap: the
//! two oracles split there, in opposite directions. iverilog rejects `{R{1'b1}}`
//! while verilator replicates; verilator rejects `logic [R-1:0]` while iverilog
//! sizes it 3. `a_bare_real_*_stays_loud` pin both.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_raw(src: &str) -> (String, bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_r2i_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut s = String::new();
    for l in so.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    (
        s,
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn run(src: &str) -> String {
    let (out, ok, err) = run_raw(src);
    assert!(ok, "expected success, stderr:\n{err}");
    out
}

fn loud(src: &str, needle: &str) {
    let (_, ok, err) = run_raw(src);
    assert!(!ok, "expected a loud reject");
    assert!(err.contains(needle), "unexpected diagnostic:\n{err}");
}

/// `logic [int'(R)-1:0] v;` — both oracles size it 3; vita declared ONE bit's
/// worth of nothing (the bound did not fold, so the whole declaration was loud).
#[test]
fn a_cast_of_a_real_param_sizes_a_width() {
    let out = run("module t;\n\
           localparam real R = 3.0;\n\
           logic [int'(R)-1:0] v;\n\
           initial $display(\"%0d\", $bits(v));\n\
         endmodule\n");
    assert_eq!(out, "3\n");
}

/// `$clog2` of a real converts FIRST and takes the log of that (§20.8.1 has no
/// bit pattern to read out of a real): `$clog2(2.4)` is 1 and `$clog2(0.5)` is 0,
/// on both oracles.
#[test]
fn clog2_of_a_real_converts_then_takes_the_log() {
    let out = run("module t;\n\
           localparam real A = 2.4, B = 0.5, C = 9.0;\n\
           initial $display(\"%0d %0d %0d\", $clog2(A), $clog2(B), $clog2(C));\n\
         endmodule\n");
    assert_eq!(out, "1 0 4\n");
}

/// A `$clog2` over a real sizing a declaration — the width path and the value
/// path are different consumers and both had to learn it.
#[test]
fn a_clog2_of_a_real_param_sizes_a_width() {
    let out = run("module t;\n\
           localparam real R = 3.0;\n\
           logic [$clog2(R)-1:0] v;\n\
           initial $display(\"%0d\", $bits(v));\n\
         endmodule\n");
    assert_eq!(out, "2\n");
}

/// A replication COUNT is a third consumer, with its own gate. Both oracles: 7.
#[test]
fn a_cast_of_a_real_param_is_a_replication_count() {
    let out = run("module t;\n\
           localparam real R = 3.0;\n\
           wire [7:0] w = {int'(R){1'b1}};\n\
           initial $display(\"%0d\", w);\n\
         endmodule\n");
    assert_eq!(out, "7\n");
}

/// A DECLARATION is a context boundary too (§6.24.1), so a declared-integral
/// localparam converts its real initializer even with no cast written.
#[test]
fn a_declared_integral_localparam_converts_a_real() {
    let out = run("module t;\n\
           localparam real R = 3.0;\n\
           localparam int M = R*2.0;\n\
           localparam int H = R/2.0;\n\
           initial $display(\"%0d %0d\", M, H);\n\
         endmodule\n");
    // R/2.0 is 1.5, which rounds AWAY from zero — both oracles print 2, not 1.
    assert_eq!(out, "6 2\n");
}

/// §6.24.1 rounds half AWAY FROM ZERO, in both directions. Pinned as values
/// because the plausible wrong answer (tie-to-even) agrees on 3.5 and differs on
/// 2.5 — one cell cannot tell them apart.
#[test]
fn the_conversion_rounds_half_away_from_zero() {
    let out = run("module t;\n\
           localparam real A = 2.5, B = 3.5, C = -2.5, D = -0.5;\n\
           localparam int  P = int'(A), Q = int'(B), S = int'(C), U = int'(D);\n\
           initial $display(\"%0d %0d %0d %0d\", P, Q, S, U);\n\
         endmodule\n");
    assert_eq!(out, "3 4 -3 -1\n");
}

/// `$rtoi` TRUNCATES (§20.10) where a cast ROUNDS — the pair must not be unified.
#[test]
fn rtoi_truncates_where_a_cast_rounds() {
    let out = run("module t;\n\
           localparam real R = 2.9;\n\
           localparam int  A = $rtoi(R), B = int'(R);\n\
           initial $display(\"%0d %0d\", A, B);\n\
         endmodule\n");
    assert_eq!(out, "2 3\n");
}

/// The converted value is then narrowed to the DECLARED width, like any other
/// parameter value: 300 in a `byte` is 44 on both oracles.
#[test]
fn the_converted_value_is_narrowed_to_the_declared_width() {
    let out = run("module t;\n\
           localparam real R = 3.0;\n\
           localparam byte M = R*100.0;\n\
           localparam bit  B = R*2.0;\n\
           initial $display(\"%0d %0d\", M, B);\n\
         endmodule\n");
    assert_eq!(out, "44 0\n");
}

/// The three declaration sites spell the same fold three different ways, so the
/// same text used to answer at module scope and go loud one `generate` deeper.
#[test]
fn the_same_conversion_folds_at_module_generate_and_package_scope() {
    let m = run("module t;\n\
           localparam real R = 3.0;\n\
           localparam int M = R*2.0;\n\
           initial $display(\"%0d\", M);\n\
         endmodule\n");
    let g = run("module t;\n\
           localparam real R = 3.0;\n\
           generate if (1) begin : g\n\
             localparam int M = R*2.0;\n\
             initial $display(\"%0d\", M);\n\
           end endgenerate\n\
         endmodule\n");
    let p = run("package pk;\n\
           localparam real R = 3.0;\n\
           localparam int M = R*2.0;\n\
         endpackage\n\
         module t;\n\
           initial $display(\"%0d\", pk::M);\n\
         endmodule\n");
    assert_eq!((m.as_str(), g.as_str(), p.as_str()), ("6\n", "6\n", "6\n"));
}

/// ⚠️ The shape the withdrawn i64 twin broke, and the reason the conversion lives
/// at the consumer rather than the leaf. A generate CONDITION is not an integer
/// context: `R/2 > 2` with R = 5.0 is evaluated wholly in the real domain
/// (2.5 > 2 ⇒ true) and only its 1-bit result crosses over. A leaf conversion
/// makes it 2 > 2 ⇒ the ELSE branch, silently, at exit 0.
#[test]
fn real_domain_still_owns_a_generate_condition() {
    let out = run("module t;\n\
           localparam real R = 5.0;\n\
           generate if (R/2 > 2) begin : y initial $display(\"THEN\"); end\n\
                    else         begin : n initial $display(\"ELSE\"); end\n\
           endgenerate\n\
         endmodule\n");
    assert_eq!(out, "THEN\n");
}

/// …and a `real` localparam still keeps its real value. `CLK/2` is 2.5, not 2.
#[test]
fn a_real_localparam_still_divides_in_the_real_domain() {
    let out = run("module t;\n\
           localparam real CLK = 5.0;\n\
           localparam real HALF = CLK/2;\n\
           initial $display(\"%0.2f\", HALF);\n\
         endmodule\n");
    assert_eq!(out, "2.50\n");
}

/// ⚠️ An UNTYPED parameter takes its TYPE from its value (§6.20.2), so this one is
/// a REAL parameter and converting it would be the silent-wrong §4.5.232 withdrew
/// over. Loud is the correct rung here; both oracles print 2.50 and closing the
/// gap means giving the untyped declaration a real type, not rounding it.
#[test]
fn an_untyped_localparam_with_a_real_value_stays_loud() {
    loud(
        "module t;\n\
           localparam real R = 5.0;\n\
           localparam M = R/2.0;\n\
           initial $display(\"%0d\", M);\n\
         endmodule\n",
        "not a foldable constant expression",
    );
}

/// ⚠️ An IMPLICIT conversion in a WIDTH bound: iverilog sizes it 3, verilator
/// rejects it outright. An axis where the oracles split is one vita stays loud on.
#[test]
fn a_bare_real_in_a_width_bound_stays_loud() {
    loud(
        "module t;\n\
           localparam real R = 3.0;\n\
           logic [R-1:0] v;\n\
           initial $display(\"%0d\", $bits(v));\n\
         endmodule\n",
        "cannot be used in a width",
    );
}

/// ⚠️ …and the same split runs the OTHER way for a replication count: iverilog
/// rejects ("Concatenation repeat expression can not be REAL"), verilator
/// replicates 3 times. Two cells, opposite tools — which is why the stand-down is
/// keyed on an explicit conversion and not on "vita can compute a number here".
#[test]
fn a_bare_real_replication_count_stays_loud() {
    loud(
        "module t;\n\
           localparam real R = 3.0;\n\
           wire [7:0] w = {R{1'b1}};\n\
           initial $display(\"%0d\", w);\n\
         endmodule\n",
        "replication count that reads a real parameter",
    );
}

/// ⚠️ Fail-closed: a cast the const domain CANNOT fold must stay loud, because the
/// alternative in a width bound is a SILENT one-bit net. This was pre-existing —
/// `logic [int'(NOPE)-1:0] v;` ran at exit 0 with `$bits(v)` = 1 while the bare
/// `[NOPE-1:0]` twin was already loud about that very name (`nonconst_bound_reason`
/// had no `Cast` arm, the same hole §4.5.380 closed for `SysCall` one node over).
#[test]
fn an_unfoldable_cast_in_a_width_bound_is_loud() {
    loud(
        "module t;\n\
           logic [int'(NOPE)-1:0] v;\n\
           initial $display(\"%0d\", $bits(v));\n\
         endmodule\n",
        "undefined name `NOPE`",
    );
}

/// ⚠️ The shadow guard. `const_int_via_real` resolves names at MODULE scope, so a
/// const-function body whose integer FORMAL shares a name with a module `real`
/// must not reach it — mirroring the `$rtoi` arm into the const-function walk
/// folded `f(9)` to 3 (the parameter) where iverilog gives 9 (the argument), and
/// PRE was loud, so that was loud → silent-wrong. The walk that owns `env` owns
/// the shadow rule; this stays loud rather than answering from the wrong binding.
#[test]
fn a_const_function_formal_shadowing_a_real_param_is_not_read_through() {
    let (_, ok, err) = run_raw(
        "module t;\n\
           localparam real N = 3.9;\n\
           function automatic int f(int N);\n\
             f = $rtoi(N);\n\
           endfunction\n\
           localparam int M = f(9);\n\
           initial $display(\"%0d\", M);\n\
         endmodule\n",
    );
    assert!(
        !ok,
        "must not answer from the module-scope real; iverilog gives 9 (the argument)"
    );
    assert!(err.contains("E3009"), "unexpected diagnostic:\n{err}");
}

/// ⚠️ `$rtoi` asks the INTEGER domain first, and the order is not stylistic: the
/// real domain promotes an integral subtree with `as f64`, which above 2^53 is
/// lossy. 2^53+1 came back one less until the order was fixed. (Both oracles
/// truncate this to a 32-bit container instead — iverilog −2, verilator
/// 2147483647 — the same family as the recorded `$itor` divergence, so this is
/// pinned hand-IEEE.)
#[test]
fn rtoi_of_a_large_integral_argument_is_exact() {
    let out = run("module t;\n\
           localparam longint M = $rtoi(64'd9007199254740993);\n\
           initial $display(\"%0d\", M);\n\
         endmodule\n");
    assert_eq!(out, "9007199254740993\n");
}

/// ⭐ A real LITERAL is the same axis, and the census missed it — every grid cell
/// named a `parameter real`. The suite's own pin surfaced it: `$clog2(8.0)` used
/// to fold to a SILENT 0-width replication, defended in that test's docstring as
/// "declines to 0, never a wrong non-zero". Both oracles print 3.
#[test]
fn a_real_literal_in_an_integer_context_converts_too() {
    let out = run("module t;\n\
           logic [int'(3.0)-1:0] v;\n\
           logic [$clog2(8.0)-1:0] u;\n\
           localparam int K = int'(2.5);\n\
           initial $display(\"%0d %0d %0d\", $bits(v), $bits(u), K);\n\
         endmodule\n");
    assert_eq!(out, "3 3 3\n");
}
