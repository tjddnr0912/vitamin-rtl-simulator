//! Module-scope constant expressions. vita used to evaluate an untyped `localparam`
//! initializer in one unbounded integer domain, so `localparam E = (8'd200 + 8'd100) >> 2`
//! answered 75 where the self-determined width says 11. That lane is WIDTH-AWARE now
//! (`param_init_width_aware_ok`) and answers 11, which is verilator's and iverilog's.
//! What this file pins is unchanged, and it is the reason the fix could not simply
//! adopt "iverilog's answer".
//!
//! Grounding it dissolved the item: **iverilog's own untyped-parameter folding is
//! internally inconsistent**, in three independent ways measured on iverilog 13.0:
//!
//!   1. Wrapping a sub-expression in a SAME-WIDTH `+ 8'd0` changes its value:
//!      `(8'd200+8'd100) >> 2`           -> 11
//!      `((8'd200+8'd100) >> 2) + 8'd0`  -> 75   (same shift, same operands, and
//!                                                iverilog's own `$bits` here is 9)
//!      ⚠️ The 32-bit `+ 0` spelling is NOT this defect — a 32-bit sibling widens the
//!      context (§11.6.1), so 75 is the right answer there in all three tools.
//!   2. `+` and `*` are folded UNBOUNDED while `<<` is folded at 32 bits:
//!      `32'd2000000000 + 32'd2000000000` -> 4000000000  (no 32-bit wrap)
//!      `32'd100000 * 32'd100000`         -> 10000000000 (no 32-bit wrap)
//!      `32'd1 << 32'd33`                 -> 0           (32-bit wrap)
//!   3. So the same "what is the context width" question is answered 64-bit by
//!      one operator and 32-bit by another in the same expression position.
//!
//! No single width model reproduces all of iverilog's answers, so it is not the oracle
//! HERE (ENGINEERING_RULES: when a tool contradicts itself, target spec-correctness and
//! keep vita self-consistent). verilator is, and vita is byte-identical to it on every
//! cell in this file. What the file pins is the property that needs no oracle at all:
//! an expression's value must not depend on whether it is wrapped in a value-preserving
//! operation OF ITS OWN WIDTH.
//!
//! ⚠️ That width qualifier was missing until the width-aware slice landed, and its
//! absence is why these tests passed while every column was uniformly wrong. A
//! self-consistency pin certifies the RELATION, never the values — so read it beside a
//! test that pins the values against an oracle (`untyped_param_operator_width`).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ces_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

/// Wrapping an expression in a value-preserving operation of ITS OWN WIDTH must not
/// change it — parentheses, a double negation, `+ 8'd0`, `* 8'd1`.
///
/// ⚠️ The width qualifier is the whole rule, and this test used to omit it. It asserted
/// that `+ 0` was value-preserving too, and PASSED only because every column was
/// uniformly WRONG (all 75, where a self-determined `(8'd200 + 8'd100) >> 2` is 11). It
/// is not value-preserving: `0` is a 32-bit decimal literal, so §11.6.1 makes the whole
/// expression 32 bits, the sum keeps 300 and the shift keeps the bit an 8-bit sum drops.
/// Measured, all six columns: verilator is byte-identical to vita
/// (`11 75 75 11 11 11` at `$bits` `8 32 32 8 8 8`), vita's own runtime spelling agrees,
/// and iverilog answers 75 for the 8-bit wrapper too — at `$bits` 9, contradicting its
/// own expression width, which is the inconsistency this file exists to keep out.
#[test]
fn a_value_preserving_wrapper_does_not_change_a_const_expression() {
    let (out, c) = run("module m;\n\
           localparam BARE = (8'd200 + 8'd100) >> 2;\n\
           localparam ADD0 = ((8'd200 + 8'd100) >> 2) + 8'd0;\n\
           localparam MUL1 = ((8'd200 + 8'd100) >> 2) * 8'd1;\n\
           localparam PAREN = (((8'd200 + 8'd100) >> 2));\n\
           localparam NEG2 = -(-((8'd200 + 8'd100) >> 2));\n\
           initial begin\n\
             $display(\"W=%0d %0d %0d %0d %0d\", BARE, ADD0, MUL1, PAREN, NEG2);\n\
             #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    let line = out
        .lines()
        .find(|l| l.starts_with("W="))
        .unwrap_or_else(|| panic!("no W= output (silent drop); got:\n{out}"));
    let vals: Vec<&str> = line.trim_start_matches("W=").split_whitespace().collect();
    assert!(
        vals.windows(2).all(|w| w[0] == w[1]),
        "a same-width wrapper must not change the value; got:\n{out}"
    );
    assert!(
        line.starts_with("W=11 "),
        "and the value is 11, not 75:\n{out}"
    );

    // The control the rule needs on the other side: a WIDER wrapper changes it, and
    // that is the language, not a defect. Both spellings are verilator's.
    let (out, c) = run("module m;\n\
           localparam BARE = (8'd200 + 8'd100) >> 2;\n\
           localparam ADD0 = ((8'd200 + 8'd100) >> 2) + 0;\n\
           initial begin\n\
             $display(\"V=%0d %0d %0d %0d\", BARE, $bits(BARE), ADD0, $bits(ADD0));\n\
             #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(
        out.contains("V=11 8 75 32"),
        "a 32-bit sibling widens the context (§11.6.1):\n{out}"
    );
}

/// One context width for every operator: `*` and `<<` both produce a value above
/// 32 bits here, so in a single domain BOTH keep it (vita) or BOTH wrap. iverilog
/// keeps the product (10000000000) and wraps the shift (0) — inconsistency #2.
#[test]
fn every_operator_folds_in_the_same_domain() {
    let (out, c) = run("module m;\n\
           localparam MUL = 32'd100000 * 32'd100000;   // 1e10, needs 34 bits\n\
           localparam SHL = 32'd1 << 32'd33;           // 2^33, needs 34 bits\n\
           initial begin\n\
             $display(\"W=%0d\", (MUL > 32'hFFFF_FFFF) == (SHL > 32'hFFFF_FFFF));\n\
             #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    // 1 = the two operators agree about whether the domain is wider than 32 bits.
    // Which answer they agree ON is the open design question; DISAGREEING is the
    // defect, and it is the one iverilog has.
    assert!(
        out.contains("W=1"),
        "`*` and `<<` must not disagree about the context width; got:\n{out}"
    );
}
