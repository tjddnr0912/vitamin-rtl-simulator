//! The constant domain computes in the width the DECLARATION states, not in i64.
//!
//! Two external reports (round-33 and an AES IP audit) converged on one axis: every
//! crypto/AXI constant idiom that mixes a NAME with a wide value was rejected, and
//! WHICH OPERATOR you wrote decided whether the parameter existed at all — `A ^ 128'h1`
//! folded and `A ^ B` did not, for the same value, because the wide fold's name
//! resolver consulted `wide_param_bits` alone and that table holds nothing under 65
//! bits.
//!
//! ⚠️⚠️ The width source is the whole soundness argument, and it is `param_range`, not
//! `param_meta`. §4.5.373 built the reduction operators on `param_meta` — where widths
//! INFERRED from a folded value are recorded beside declared ones — and measured
//! `localparam W = 4'hF | 4'h0;` reducing 32 bits where both oracles hold 4, picking
//! the opposite generate branch at exit 0. It reverted. `param_range` records only what
//! a DECLARATION states, and the value stored beside it is coerced to that width at
//! binding, so the pair is canonical. The counterexample is pinned below.
//!
//! Every value here was measured on iverilog 13.0 AND verilator 5.050 unless a cell
//! says otherwise.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_wcd_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// Wrap `decls` and print `R=<fmt>` of `expr`.
fn m(decls: &str, fmt: &str, expr: &str) -> String {
    format!(
        "`timescale 1ns/1ns\nmodule tb;\n{decls}\n  \
         initial begin #1 $display(\"R={fmt}\", {expr}); $finish; end\nendmodule\n"
    )
}

fn folds(decls: &str, fmt: &str, expr: &str, want: &str) {
    let (out, code) = run(&m(decls, fmt, expr));
    assert_eq!(code, Some(0), "should fold:\n{out}");
    assert!(out.contains(&format!("R={want}")), "want R={want}:\n{out}");
}

fn loud(decls: &str, fmt: &str, expr: &str) {
    let (out, code) = run(&m(decls, fmt, expr));
    assert_ne!(code, Some(0), "must stay loud:\n{out}");
}

/// ⭐ The headline: a NARROW named parameter is an operand of the wide fold now.
#[test]
fn a_named_narrow_parameter_is_a_wide_operand() {
    let d = "  localparam logic [127:0] A = 128'he1000000000000000000000000000001;\n  \
             localparam logic [127:0] B = 128'h1;\n  \
             localparam logic [127:0] R = A ^ B;";
    folds(d, "%h", "R", "e1000000000000000000000000000000");
    // …and the literal spelling it used to disagree with.
    let e = "  localparam logic [127:0] A = 128'he1000000000000000000000000000001;\n  \
             localparam logic [127:0] R = A ^ 128'h1;";
    folds(e, "%h", "R", "e1000000000000000000000000000000");
}

/// §11.4.3 / §11.4.4: arithmetic and comparison inside the declared width.
#[test]
fn wide_arithmetic_and_comparison_fold() {
    let base = "  localparam logic [127:0] A = 128'he1000000000000000000000000000001;\n  \
                localparam logic [127:0] W = 128'h0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f;\n";
    folds(
        &format!("{base}  localparam logic [127:0] R = A + W;"),
        "%h",
        "R",
        "f00f0f0f0f0f0f0f0f0f0f0f0f0f0f10",
    );
    folds(
        &format!("{base}  localparam logic R = (A == W);"),
        "%b",
        "R",
        "0",
    );
    // ⭐ The carry must be DISCARDED at the context width, not carried out of it:
    // `128'h1 - 128'h3` is `…fffe`, and the i64 domain's −2 zero-extended into an
    // unsigned 128-bit parameter as `0000000000000000fffffffffffffffe` — a
    // pre-existing silent-wrong this closes.
    folds(
        "  localparam logic [127:0] A = 128'h1;\n  localparam logic [127:0] B = 128'h3;\n  \
         localparam logic [127:0] R = A - B;",
        "%h",
        "R",
        "fffffffffffffffffffffffffffffffe",
    );
    // ⚠️ Division and modulus WERE deliberately out — "a wide divide is a different
    // algorithm and a subtly wrong one produces a silent wrong parameter". Round 34
    // closed that by MOVING the engine's limb `mw_divmod` into `sim-ir` so both
    // domains call the same function, leaving no second spelling to be wrong. The
    // value is live iverilog 13.0's and an independent Python golden's.
    folds(
        "  localparam logic [127:0] A = 128'he1000000000000000000000000000001;\n  \
         localparam logic [127:0] R = A / 3;",
        "%h",
        "R",
        "4b000000000000000000000000000000",
    );
    // What genuinely has no value stays loud: §11.4.3 makes `x / 0` an `x`, and
    // iverilog and vita's own runtime both give x there.
    loud(
        "  localparam logic [127:0] A = 128'he1000000000000000000000000000001;\n  \
         localparam logic [127:0] Z = 128'h0;\n  \
         localparam logic [127:0] R = A / Z;",
        "%h",
        "R",
    );
}

/// §11.5.1 selects — PLACEMENT, so the >64-bit base needs no i64 value.
#[test]
fn a_select_of_a_wide_parameter_folds() {
    let d = "  localparam logic [127:0] A = 128'he1000000000000000000000000000001;\n";
    folds(
        &format!("{d}  localparam logic [63:0] R = A[127:64];"),
        "%h",
        "R",
        "e100000000000000",
    );
    folds(
        &format!("{d}  localparam logic R = A[127];"),
        "%b",
        "R",
        "1",
    );
    folds(
        &format!("{d}  localparam logic [127:0] R = A[127:0];"),
        "%h",
        "R",
        "e1000000000000000000000000000001",
    );
    // The indexed form, which is how a per-port vector is sliced.
    folds(
        "  localparam logic [63:0] V = {32'd7, 32'd4};\n  \
         localparam logic [31:0] R = V[0 +: 32];",
        "%0d",
        "R",
        "4",
    );
}

/// §11.4.14 reductions and §20.9 bit-vector system functions — at ANY width.
#[test]
fn reductions_and_bit_functions_fold() {
    let a = "  localparam logic [7:0] A = 8'hA5;\n";
    folds(&format!("{a}  localparam logic R = ^A;"), "%b", "R", "0");
    folds(&format!("{a}  localparam logic R = ~^A;"), "%b", "R", "1");
    folds(
        "  localparam logic [7:0] A = 8'hFF;\n  localparam logic R = &A;",
        "%b",
        "R",
        "1",
    );
    folds(
        "  localparam logic [7:0] A = 8'h00;\n  localparam logic R = |A;",
        "%b",
        "R",
        "0",
    );
    folds(
        &format!("{a}  localparam int R = $countones(A);"),
        "%0d",
        "R",
        "4",
    );
    folds(
        "  localparam logic [7:0] A = 8'h08;\n  localparam logic R = $onehot(A);",
        "%b",
        "R",
        "1",
    );
    folds(
        "  localparam logic [7:0] A = 8'hFF;\n  localparam int R = $signed(A);",
        "%0d",
        "R",
        "-1",
    );
}

/// ⚠️⚠️ §4.5.373's counterexample, pinned as a DECLINE.
///
/// An UNTYPED parameter's width is inferred from its folded value, and the inference
/// disagrees with the language: `localparam W = 4'hF | 4'h0;` is 4 bits in both
/// oracles and 32 in `param_meta`. A reduction over it would pick the opposite
/// generate branch at exit 0, which is exactly what that slice measured before
/// reverting. It stays loud, and the DECLARED twin folds beside it.
#[test]
fn an_inferred_width_never_supplies_a_reduction() {
    loud(
        "  parameter A = 4'h1;\n  localparam W = A | 4'h0;\n  localparam logic R = ^W;",
        "%b",
        "R",
    );
    // The same value with the range DECLARED: canonical, and it folds.
    folds(
        "  parameter A = 4'h1;\n  localparam logic [3:0] W = A | 4'h0;\n  \
         localparam logic R = ^W;",
        "%b",
        "R",
        "1",
    );
    // …and the shape the memory records as the canonicality question: with the range
    // declared, the stored value IS canonical at that width. Both tools: 0.
    folds(
        "  parameter A = 4'h1;\n  localparam logic [3:0] W = A << 4;\n  \
         localparam logic [3:0] R = W;",
        "%0d",
        "R",
        "0",
    );
}

/// ⭐ A CONTEXT-determined top is folded AT THE CONTEXT WIDTH — §2 row 21, and this pin
/// named its own prerequisite before it landed.
///
/// `65'(64'd18446744073709551615 + 64'd1) >> 64` is 1 in both oracles: the cast
/// establishes a 65-bit context and the sum carries into bit 64. This walk used to fold
/// the operand at its own 64 bits, wrap it to 0 and then widen, so it declined rather
/// than answer wrongly. It carries a context width now, and answers 1.
///
/// ⚠️ The SELF-determined neighbour must keep folding, and a cast must remain its OWN
/// context rather than passing the outer one through — `128'(1)` is 1, not 1 widened
/// from something else.
#[test]
fn a_context_determined_top_folds_at_the_context_width() {
    folds(
        "  localparam integer R = 65'(64'd18446744073709551615 + 64'd1) >> 64;",
        "%0d",
        "R",
        "1",
    );
    folds("  localparam integer R = 128'(1);", "%0d", "R", "1");
}

/// ⭐ A replication COUNT may be a NAME — §4.5.371's three blocking defects, answered
/// by folding the count through the SAME resolver the surrounding fold uses.
#[test]
fn a_replication_count_may_be_a_name() {
    folds(
        "  localparam int N = 3;\n  localparam logic [5:0] PV = {N{2'b01}};",
        "%b",
        "PV",
        "010101",
    );
    // The per-port-vector idiom every parameterised AXI / Ethernet core emits.
    folds(
        "  localparam int S = 2;\n  localparam logic [S*32-1:0] MASK = {S{32'hDEADBEEF}};",
        "%h",
        "MASK",
        "deadbeefdeadbeef",
    );
}

/// ⭐ A PACKAGE parameter may be wider than 64 bits — the one scope that had no
/// fourth domain to fall into, while the identical declaration in a module, a `#()`
/// header, a one-element array and a packed struct all worked.
#[test]
fn a_package_parameter_may_be_wider_than_64_bits() {
    let (out, code) = run("`timescale 1ns/1ns\n\
         package p;\n  localparam logic [127:0] K = 128'he1000000000000000000000000000000;\n  \
         localparam logic [127:0] M = ~K;\nendpackage\n\
         module tb; import p::*;\n  logic [127:0] r; assign r = K;\n  \
         localparam logic [127:0] X = K ^ M;\n  \
         localparam logic [63:0] Y = p::K[127:64];\n  \
         initial begin #1 $display(\"R=%032h %032h %016h\", r, X, Y); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains(
            "R=e1000000000000000000000000000000 ffffffffffffffffffffffffffffffff e100000000000000"
        ),
        "wildcard import, intra-package sibling, and `pkg::K[…]`:\n{out}"
    );
}

/// ⭐ §6.20.2: an untyped parameter takes the RANGE of its final override value, and
/// the i64 override channel carried no range — so `#(.M_ISSUE(M_ISSUE))` forwarding
/// `{2{32'd4}}` arrived 35 bits wide (the value's minimal signed width) where every
/// other tool has 64, and the per-port slice `M_ISSUE[32 +: 32]` then read past the
/// end. This is `axi_crossbar`'s shape.
#[test]
fn an_override_carries_its_width() {
    let (out, code) = run("`timescale 1ns/1ns\n\
         module leaf #(parameter M_COUNT = 2, parameter M_ISSUE = {M_COUNT{32'd4}})\n  \
         (output logic o);\n  genvar n;\n  generate\n  \
         for (n = 0; n < M_COUNT; n = n + 1) begin : mi\n    \
         localparam int C = M_ISSUE[n*32 +: 32];\n    \
         initial $display(\"C%0d=%0d\", n, C);\n  end\n  endgenerate\n  \
         assign o = 1'b1;\nendmodule\n\
         module mid #(parameter M_COUNT = 2, parameter M_ISSUE = {M_COUNT{32'd4}})\n  \
         (output logic o);\n  leaf #(.M_COUNT(M_COUNT), .M_ISSUE(M_ISSUE)) u(.o(o));\nendmodule\n\
         module tb; logic o; mid u(.o(o));\n  initial begin #1 $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("C0=4") && out.contains("C1=4"),
        "iverilog: 4 4\n{out}"
    );
}

/// ⭐ A package `real` reaches a MODULE-scope constant. The value crossed the package
/// boundary all along — a bare read, a procedural `int'()`, and an intra-package
/// sibling all worked — and only the constant domain refused it, because the
/// predicate that CHOOSES the real domain had no `pkg::` arm to see it with.
#[test]
fn a_package_real_reaches_module_scope_constants() {
    let (out, code) = run("`timescale 1ns/1ns\n\
         package pk; parameter real R = 3.5; endpackage\n\
         module t2 #(parameter real Q = pk::R); initial $display(\"PORT=%0f\", Q); endmodule\n\
         module tb;\n  localparam real Q = pk::R * 2.0;\n  \
         localparam int N = int'(pk::R * 2.0);\n  logic [$clog2(pk::R)-1:0] v;\n  t2 u();\n  \
         initial begin #1 $display(\"R=%0f %0d %0d\", Q, N, $bits(v));\n    \
         if (pk::R > 2.0) $display(\"GEN=then\"); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("PORT=3.500000"), "{out}");
    assert!(
        out.contains("R=7.000000 7 2"),
        "both oracles: 7.0 / 7 / 2\n{out}"
    );
    assert!(out.contains("GEN=then"), "{out}");
}

/// String methods with an integral result fold in a constant context.
#[test]
fn a_string_method_folds_in_a_constant_context() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  localparam string S = \"abcd\";\n  \
         localparam int W = S.len();\n  localparam string N = \"42\";\n  \
         localparam int A = N.atoi();\n  logic [S.len()-1:0] v;\n  \
         initial begin #1 $display(\"R=%0d %0d %0d\", W, A, $bits(v)); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("R=4 42 4"), "both oracles: 4 42 4\n{out}");
}

/// A `pkg::`-qualified value may take a method — three parse errors at one column
/// before, none of which said "method", "package" or "::".
#[test]
fn a_qualified_value_may_take_a_method() {
    let (out, code) = run(
        "`timescale 1ns/1ns\npackage pk; localparam string S = \"hello\"; endpackage\n\
         module tb;\n  localparam int L = pk::S.len();\n  int r;\n  \
         initial begin #1 r = pk::S.len(); $display(\"R=%0d %0d\", L, r); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("R=5 5"), "iverilog: 5 5\n{out}");
}
