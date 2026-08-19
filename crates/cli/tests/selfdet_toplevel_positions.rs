//! §2 「다음 착수 순서」 #1 — SELF-DETERMINED positions at the TOP of a constant
//! expression.
//!
//! `const_eval_in_scope` is the width-UNLIMITED module-scope fold, and it was
//! answering positions IEEE 1800 §11.6.1 Table 11-21 sizes by themselves:
//!
//!   * a comparison / equality / logical operator delivers ONE bit and sizes its
//!     operands against EACH OTHER, so the whole node takes no context —
//!     `(4'd15 + 4'd1) > 4'd0` was 1 where the 4-bit sum wraps to 0;
//!   * `!e` yields one bit from a self-determined operand;
//!   * a ternary CONDITION (§11.4.11) and a generate-if condition are sized by
//!     themselves, so `generate if (4'd15 + 4'd1)` silently elaborated the wrong
//!     branch at exit 0.
//!
//! ORACLE POLICY: every value here is agreed by iverilog 13.0 AND verilator
//! 5.050. The census that scoped this slice also found two SPLIT axes, and they
//! are deliberately untouched — an untyped `localparam L = 4'd15 + 4'd1`
//! (iverilog 16, verilator 0; vita keeps iverilog's answer) and a `repeat` count
//! (iverilog 2, verilator 18). Both are recorded in ROADMAP §2's divergence list.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_raw(src: &str) -> (String, Option<i32>, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_sdtp_{}_{n}.sv", std::process::id()));
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

/// Land the value in a `localparam integer` — the position where a wrong fold is
/// a silently wrong parameter.
fn folds(expr: &str, want: i64) {
    let src = format!(
        "module top;\n\
         parameter integer W = 8;\n\
         parameter [7:0] P = 8'd200;\n\
         parameter signed [3:0] SN = -4'sd3;\n\
         parameter string MODE = \"Y\";\n\
         localparam integer L = {expr};\n\
         initial begin $display(\"R=%0d\", L); #1 $finish; end\n\
         endmodule\n"
    );
    let (out, code, err) = run_raw(&src);
    assert_eq!(code, Some(0), "`{expr}` should fold, stderr:\n{err}");
    assert!(
        out.contains(&format!("R={want}")),
        "`{expr}` want R={want}; got:\n{out}"
    );
}

/// A comparison's operands size against EACH OTHER, so a narrow pair wraps before
/// the compare. Ten operators, both wrap directions, and a mixed-sign pair.
#[test]
fn a_comparison_sizes_its_operands_against_each_other() {
    for (e, want) in [
        ("((4'd15 + 4'd1) > 4'd0)", 0),
        ("((4'd15 + 4'd1) == 4'd0)", 1),
        ("((4'd15 + 4'd1) < 4'd1)", 1),
        ("((4'd15 + 4'd1) != 4'd0)", 0),
        ("((4'd15 + 4'd1) >= 4'd1)", 0),
        ("((4'd15 + 4'd1) <= 4'd0)", 1),
        ("((4'd15 + 4'd1) === 4'd0)", 1),
        ("((4'd15 + 4'd1) !== 4'd0)", 0),
        ("((8'd200 + 8'd100) >= 8'd45)", 0),
        ("((8'd200 + 8'd100) == 8'd44)", 1),
        // A comparison unifies SIGNEDNESS across both operands: an unsigned
        // sibling reinterprets the signed one.
        ("(SN < 4'd1)", 0),
        ("(SN < 8'd1)", 0),
    ] {
        folds(e, want);
    }
}

/// `!e` and the two logical operators are the same rule one operator over.
#[test]
fn logical_operators_take_a_self_determined_operand() {
    folds("!(4'd15 + 4'd1)", 1);
    folds("((4'd15 + 4'd1) && 1'b1)", 0);
    folds("((4'd15 + 4'd1) || 1'b0)", 0);
    // `~` is context-determined and must NOT move.
    folds("(~(4'd15 + 4'd1))", -17);
    folds("(-(4'd15 + 4'd1))", -16);
}

/// §11.4.11: the ternary CONDITION is self-determined — the arms and the
/// surrounding assignment give it no width. Nesting one inside another proves the
/// rule applies at every level, not just the outermost.
#[test]
fn a_ternary_condition_is_self_determined() {
    folds("(4'd15 + 4'd1) ? 7 : 9", 9);
    folds("((4'd15 + 4'd1) ? 1 : 0) ? 7 : 9", 9);
    folds("(8'd200 + 8'd100) ? 7 : 9", 7);
}

/// A generate-if condition is the same position, and getting it wrong is the
/// worst shape in this family: the wrong module hierarchy is elaborated, silently,
/// at exit 0. Both oracles take the `else` on all three.
#[test]
fn a_generate_if_condition_is_self_determined() {
    for (cond, want) in [
        ("4'd15 + 4'd1", 222),
        ("(4'd15 + 4'd1) != 4'd0", 222),
        ("(4'd15 + 4'd1) > 4'd0", 222),
        // …and the neighbours that must still take the `then`.
        ("W > 4", 111),
        ("MODE == \"Y\"", 111),
        ("8'd200 + 8'd100", 111),
    ] {
        let src = format!(
            "module top;\n\
             parameter integer W = 8;\n\
             parameter string MODE = \"Y\";\n\
             generate if ({cond}) begin : y\n\
             initial begin $display(\"R=%0d\", 111); #1 $finish; end\n\
             end else begin : n\n\
             initial begin $display(\"R=%0d\", 222); #1 $finish; end\n\
             end endgenerate\n\
             endmodule\n"
        );
        let (out, code, err) = run_raw(&src);
        assert_eq!(code, Some(0), "generate-if `{cond}`, stderr:\n{err}");
        assert!(
            out.contains(&format!("R={want}")),
            "generate-if `{cond}` want {want}; got:\n{out}"
        );
    }
}

/// The context-determined operators keep the unlimited walk — the split is what
/// this slice states once, and moving an operator across it is the mutation this
/// pins. A shift, an add, a bitwise and, and `**` all stay where they were.
#[test]
fn context_determined_operators_are_unchanged() {
    for (e, want) in [
        ("((4'd15 + 4'd1) >> 1)", 8),
        ("((4'd15 + 4'd1) + 32'd0)", 16),
        ("((4'd15 + 4'd1) & 32'hFF)", 16),
        ("((4'd15 + 4'd1) * 2)", 32),
        ("(4'd3 ** 4'd3)", 27),
        ("(P > 8'd100)", 1),
        ("(P > W)", 1),
        ("((32'd100000 * 32'd100000) > 32'd0)", 1),
        ("(MODE == \"Y\")", 1),
        // A comparison inside a cast was already width-honest (§4.5.346); it
        // must agree with the bare one now, which is the self-inconsistency
        // this slice closes.
        ("8'((4'd15 + 4'd1) > 4'd0)", 0),
    ] {
        folds(e, want);
    }
}

/// A SHIFT COUNT is the same position, and leaving it out made the slice's own
/// redirect produce two answers for one subexpression: `8'd1 << (4'd15 + 4'd1)`
/// folded 65536 on its own and 1 under a comparison, because the width-aware twin
/// routes all five of `**` and the four shifts while the module-scope arm routed
/// only `**`. Both oracles say the 4-bit count wraps to 0.
#[test]
fn a_shift_count_is_self_determined_in_both_spellings() {
    folds("8'd1 << (4'd15 + 4'd1)", 1);
    folds("((8'd1 << (4'd15 + 4'd1)) > 8'd200)", 0);
    folds("8'd128 >> (4'd15 + 4'd1)", 128);
    folds("8'sd64 >>> (4'd15 + 4'd1)", 64);
    folds("8'd1 <<< (4'd15 + 4'd1)", 1);
    // A count that does NOT wrap is unchanged.
    folds("8'd1 << 4'd3", 8);
    // `**` was already routed (§4.5.319) and must stay.
    folds("4'd3 ** (4'd15 + 4'd1)", 1);
}

/// `==?` / `!=?` are members of the same operator family, and their whole-node
/// helper reads the LHS itself — so it has to read it the same way. It was the one
/// member still answering from the width-unlimited walk while its `===` sibling had
/// moved.
#[test]
fn wildcard_equality_reads_its_lhs_self_determined() {
    folds("((4'd15 + 4'd1) ==? 4'b000x)", 1);
    folds("((4'd15 + 4'd1) !=? 4'b000x)", 0);
    folds("((4'd15 + 4'd1) === 4'd0)", 1);
    folds("((8'd200 + 8'd100) ==? 8'b00101x0x)", 1);
}

/// A REPLICATION operand used to make the enclosing self width unknown, which
/// degraded the width-aware walk straight back to the unlimited domain — so the
/// rule this slice states did not hold for it in any of the four positions. Its
/// concatenation twin was already handled, which is what made the hole visible.
#[test]
fn a_replication_operand_has_a_self_width() {
    folds("(({2{4'd15}} + 4'd1) > 4'd0)", 0);
    folds("!({2{4'd15}} + 4'd1)", 1);
    folds("({2{4'd15}} + 4'd1) ? 7 : 9", 9);
    folds("(({ {2{4'd15}} } + 8'd1) > 8'd0)", 0);
    // the concatenation twin, and the replication's own value, unchanged
    folds("(({4'd15,4'd15} + 8'd1) > 8'd0)", 0);
    folds("{2{4'd15}}", 255);
    let (out, code, err) = run_raw(
        "module top;\n  generate if ({2{4'd15}} + 4'd1) begin : y\n\
         initial begin $display(\"R=%0d\", 111); #1 $finish; end\n\
         end else begin : n\n\
         initial begin $display(\"R=%0d\", 222); #1 $finish; end\n\
         end endgenerate\nendmodule\n",
    );
    assert_eq!(code, Some(0), "generate-if over a replication:\n{err}");
    assert!(out.contains("R=222"), "got:\n{out}");
    // …and a replication still measures as a width where it always did.
    let (out, code, err) = run_raw(
        "module top;\n  logic [{2{2'd1}}:0] v;\n\
         initial begin v = '1; $display(\"R=%0d\", $bits(v)); #1 $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "replication as a bound:\n{err}");
    assert!(out.contains("R=6"), "got:\n{out}");
}

/// The consumers that eat a constant condition must see the corrected value, not
/// just a printed parameter: a width, a replication count, a repeat, a delay.
#[test]
fn consumers_of_a_self_determined_condition_see_the_new_value() {
    for (src, want) in [
        (
            "module top;\n  logic [((4'd15+4'd1) ? 7 : 3):0] v;\n\
             initial begin v = '1; $display(\"R=%0d\", $bits(v)); #1 $finish; end\nendmodule\n",
            4,
        ),
        (
            "module top;\n  logic [31:0] r;\n\
             initial begin r = {(((4'd15+4'd1) > 4'd0) + 4'd2){2'b01}}; $display(\"R=%0d\", r); #1 $finish; end\nendmodule\n",
            5,
        ),
        (
            "module top;\n  int c;\n\
             initial begin c = 0; repeat (!(4'd15+4'd1) + 4'd2) c++; $display(\"R=%0d\", c); #1 $finish; end\nendmodule\n",
            3,
        ),
    ] {
        let (out, code, err) = run_raw(src);
        assert_eq!(code, Some(0), "consumer position, stderr:\n{err}");
        assert!(out.contains(&format!("R={want}")), "want {want}; got:\n{out}");
    }
}
