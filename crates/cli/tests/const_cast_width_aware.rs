//! §2 「다음 착수 순서」 #1 — a SIZE cast folds its operand at the cast's width.
//!
//! `const_eval_cast`'s `Size`/`Named` arm used to fold the operand in the
//! width-UNLIMITED i64 domain and then truncate. Truncating an un-narrowed
//! operand is not the same operation as evaluating it narrow — `4'((4'd8 +
//! 4'd8) / 4'd3)` divided 16 by 3 and stored 5 where the 4-bit sum is 0, so SV
//! (and iverilog) store 0 — and the arm compensated by folding ONLY where the
//! signed and unsigned readings agree, i.e. a non-negative value with the
//! target's sign bit clear. That refused every ordinary narrowing cast.
//!
//! The operand is sized FIRST now, at `max(its self width, N)` with its own
//! signedness, through the SAME assignment funnel the constant-function body
//! arm already used (§4.5.316 pinned that rule; §4.5.345 laid the routing).
//! The truncation stops being an approximation, so the sign no longer has to be
//! guessed away.
//!
//! Oracle: iverilog 13.0 for every value here.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_raw(src: &str) -> (String, Option<i32>, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_ccwa_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Put the value in a `localparam`, the position where a wrong fold is a
/// silently wrong parameter.
fn param(expr: &str) -> String {
    format!(
        "module top;\n\
         parameter integer W = 8;\n\
         parameter [7:0] P = 8'd200;\n\
         parameter signed [7:0] SP8 = -8'sd100;\n\
         parameter string MODE = \"Y\";\n\
         parameter [3:0] PW = 4'b1101;\n\
         function integer cf(input integer a); cf = a + 1; endfunction\n\
         localparam integer L = {expr};\n\
         initial begin $display(\"R=%0d\", L); #1 $finish; end\n\
         endmodule\n"
    )
}

fn folds(expr: &str, want: i64) {
    let (out, code, err) = run_raw(&param(expr));
    assert_eq!(code, Some(0), "`{expr}` should fold, stderr:\n{err}");
    assert!(
        out.contains(&format!("R={want}")),
        "`{expr}` want R={want}; got:\n{out}"
    );
}

fn loud(expr: &str) {
    let (_, code, err) = run_raw(&param(expr));
    assert_eq!(code, Some(1), "`{expr}` should be loud:\n{err}");
    assert!(
        err.contains("is not a foldable constant expression"),
        "`{expr}` unexpected diagnostic:\n{err}"
    );
}

/// The headline silent-wrong: truncating an operand folded wide is not the same
/// as folding it narrow. The 4-bit sum is 0, so the quotient is 0 — the old arm
/// divided the un-narrowed 16.
#[test]
fn a_narrowing_cast_folds_its_operand_narrow() {
    folds("4'((4'd8+4'd8)/4'd3)", 0);
    folds("4'(4'd15+4'd1)", 0);
    folds("2'((8'sd100+8'sd100)>>1)", 0);
    folds("8'(8'd200+8'd100)", 44);
    folds("32'(100000*100000)", 1410065408);
    // A comparison inside the cast sizes its operands against each other at the
    // cast's context — the unlimited domain answered 1 for both of these.
    folds("8'((4'd15 + 4'd1) > 4'd0)", 0);
    folds("8'((8'd200 + 8'd100) > 8'd200)", 0);
}

/// The sign is INHERITED from the operand (§6.24.1), so an ordinary narrowing
/// cast whose result has the target's top bit set is a value, not a diagnostic.
/// Every one of these was loud before.
#[test]
fn a_size_cast_inherits_the_operand_sign() {
    for (e, want) in [
        ("8'(255)", -1),
        ("4'(9)", -7),
        ("1'(3)", -1),
        ("8'(-1)", -1),
        ("16'(-5)", -5),
        ("W'(511)", -1),
        ("64'(-1)", -1),
        ("63'(-1)", -1),
        ("8'(8'sd100+8'sd100)", -56),
        ("8'(SP8)", -100),
        ("4'(SP8)", -4),
        ("8'(SP8+1)", -99),
        // …and an UNSIGNED operand is not sign-extended.
        ("8'(P)", 200),
        ("8'(P+1)", 201),
    ] {
        folds(e, want);
    }
}

/// The i64 constant domain is the boundary, and both halves of it are load
/// bearing: at `N > 64` the operand still evaluates at 64 bits, so a carry past
/// bit 63 would be lost; at exactly 64 an UNSIGNED result with its top bit set
/// escapes as a negative i64 (`coerce_int_width` is the identity there). Both
/// decline — the same fit rule the placement folder uses, and the same
/// pre-existing 64-bit class ROADMAP §2 records for a bare literal.
#[test]
fn a_cast_outside_the_i64_domain_declines() {
    loud("65'(64'd18446744073709551615 + 64'd1) >> 64");
    loud("(64'(64'hFFFFFFFFFFFFFFFF) > 0)");
    loud("65'(1)");
    loud("128'(1)");
    // The signed 64-bit neighbour is inside the domain and still folds.
    folds("64'(-1)", -1);
    folds("64'(64'h7FFFFFFFFFFFFFFF) >> 32", 2147483647);
}

/// Two comparison folds are WHOLE-NODE facts that do not live in the numeric
/// domain — a `string` parameter equality and an `==?` wildcard pattern. The
/// width-aware walk recurses into a comparison's operands, so routing the cast
/// through it shadowed both; they are shared now. The string switch is the
/// canonical way to select an implementation, and in a width position a decline
/// is a silent 1-bit net, not a diagnostic.
#[test]
fn a_cast_over_a_whole_node_comparison_still_folds() {
    folds("8'(MODE == \"Y\")", 1);
    folds("8'(MODE == \"N\")", 0);
    folds("8'(MODE == \"Y\" ? 5 : 6)", 5);
    folds("8'(PW ==? 4'b1x1x)", 0);
    let (out, code, err) = run_raw(
        "module top;\n\
         parameter string MODE = \"Y\";\n\
         wire [8'(MODE == \"Y\")+7:0] w;\n\
         initial begin $display(\"R=%0d\", $bits(w)); #1 $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "string compare as a width, stderr:\n{err}");
    assert!(out.contains("R=9"), "got:\n{out}");
    let (out, code, err) = run_raw(
        "module top;\n\
         parameter string MODE = \"Y\";\n\
         generate if (8'(MODE == \"Y\")) begin : y\n\
         initial begin $display(\"R=%0d\", 111); #1 $finish; end\n\
         end else begin : n\n\
         initial begin $display(\"R=%0d\", 222); #1 $finish; end\n\
         end endgenerate\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "string compare in generate-if:\n{err}");
    assert!(out.contains("R=111"), "got:\n{out}");
}

/// The whole-node folds are MODULE-SCOPE facts — they resolve names through
/// `const_eval_in_scope` — so the width-aware walk may consult them only where it
/// has no local bindings to shadow. Without that conjunct a body-local answers with
/// the value of a same-named module PARAMETER: proven on a mutant build, where this
/// design returns 0 (the parameter's wildcard result) instead of the honest loud,
/// while the local's own answer is iverilog's 1.
#[test]
fn a_shadowing_local_never_gets_the_module_scope_compare() {
    let (_, code, err) = run_raw(
        "module top;\n\
         parameter [3:0] X = 4'b1101;\n\
         function integer g();\n\
         bit [3:0] X;\n\
         X = 4'b1011;\n\
         g = (X ==? 4'b1x1x);\n\
         endfunction\n\
         localparam integer L = g();\n\
         initial begin $display(\"R=%0d\", L); #1 $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(1), "must not answer the module param:\n{err}");
    assert!(err.contains("is not a foldable constant expression"));
}

/// Neighbours that must not move: a widening cast, a param/call/concat operand,
/// the `Prim` and `Signing` arms (untouched), and the cast in each of the
/// constant CONSUMER positions it feeds.
#[test]
fn widening_and_neighbouring_casts_unchanged() {
    for (e, want) in [
        ("8'(5)", 5),
        ("4'(3)", 3),
        ("8'(4'd15+4'd1)", 16),
        ("16'(8'd200*8'd2)", 400),
        ("16'(W*2)", 16),
        ("8'(cf(2))", 3),
        ("8'({4'h5,4'h3})", 83),
        ("8'({2{2'b11}})", 15),
        ("63'(1)", 1),
        ("int'(4'd15+4'd1)", 16),
    ] {
        folds(e, want);
    }
    // A `signed'`/`unsigned'` cast preserves a width this domain does not track,
    // so it stays loud — the arm is untouched.
    loud("signed'(4'hF)");
    // Consumer positions.
    for (src, want) in [
        (
            "module top;\n  logic [8'(4'd15+4'd1) + 3 : 0] v;\n\
             initial begin v = '1; $display(\"R=%0d\", $bits(v)); #1 $finish; end\nendmodule\n",
            20,
        ),
        (
            "module top;\n  logic [15:0] r;\n\
             initial begin r = {4'(4'd8+4'd8+4'd4){2'b01}}; $display(\"R=%0d\", r); #1 $finish; end\nendmodule\n",
            85,
        ),
        (
            "module top;\n  int c;\n\
             initial begin c = 0; repeat (4'(4'd8+4'd8+4'd5)) c++; $display(\"R=%0d\", c); #1 $finish; end\nendmodule\n",
            5,
        ),
        (
            "module top;\n  localparam integer NW = 8'(8'd200+8'd100);\n  logic [NW-1:0] v;\n\
             initial begin v = '1; $display(\"R=%0d\", $bits(v)); #1 $finish; end\nendmodule\n",
            44,
        ),
    ] {
        let (out, code, err) = run_raw(src);
        assert_eq!(code, Some(0), "consumer position, stderr:\n{err}");
        assert!(out.contains(&format!("R={want}")), "want {want}; got:\n{out}");
    }
}
