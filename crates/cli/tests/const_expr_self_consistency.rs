//! Module-scope constant expressions: vita evaluates an untyped `localparam`
//! initializer in one unbounded integer domain. That was queued as a possible
//! width-awareness gap because iverilog prints a different value for
//! `localparam E = (8'd200 + 8'd100) >> 2` (11, where vita says 75).
//!
//! Grounding it dissolved the item: **iverilog's own untyped-parameter folding is
//! internally inconsistent**, in three independent ways measured on iverilog 13.0:
//!
//!   1. Wrapping a sub-expression in `+ 0` changes its value:
//!      `(8'd200+8'd100) >> 2`        -> 11
//!      `((8'd200+8'd100) >> 2) + 0`  -> 75      (same shift, same operands)
//!   2. `+` and `*` are folded UNBOUNDED while `<<` is folded at 32 bits:
//!      `32'd2000000000 + 32'd2000000000` -> 4000000000  (no 32-bit wrap)
//!      `32'd100000 * 32'd100000`         -> 10000000000 (no 32-bit wrap)
//!      `32'd1 << 32'd33`                 -> 0           (32-bit wrap)
//!   3. So the same "what is the context width" question is answered 64-bit by
//!      one operator and 32-bit by another in the same expression position.
//!
//! No single width model reproduces all of iverilog's answers, so there is no
//! oracle to converge on here (ENGINEERING_RULES: when the oracle contradicts
//! itself, target spec-correctness and keep vita self-consistent). vita's answers
//! ARE self-consistent — one context, applied uniformly — and this file is the
//! teeth for that property. It pins vita against ITSELF, which needs no oracle:
//! an expression's value must not depend on whether it is wrapped in a
//! value-preserving operation.
//!
//! This is a NON-GOAL record, not a supported-feature claim. If a future slice
//! makes the module-scope domain width-aware, these identities must still hold —
//! they are what "self-consistent" means, whatever width is chosen.
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

/// Wrapping an expression in a value-preserving operation must not change it.
/// This is the identity iverilog breaks (11 vs 75) and the one vita must keep.
#[test]
fn a_value_preserving_wrapper_does_not_change_a_const_expression() {
    let (out, c) = run("module m;\n\
           localparam BARE = (8'd200 + 8'd100) >> 2;\n\
           localparam ADD0 = ((8'd200 + 8'd100) >> 2) + 0;\n\
           localparam MUL1 = ((8'd200 + 8'd100) >> 2) * 1;\n\
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
        "the same shift must not change value when wrapped; got:\n{out}"
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
