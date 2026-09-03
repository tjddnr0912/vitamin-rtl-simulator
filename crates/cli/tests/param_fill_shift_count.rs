//! A shift AMOUNT written as an unsized fill is ONE BIT, not 32.
//!
//! §11.4.10 makes a shift amount self-determined and §5.7.1 gives an unsized fill in a
//! self-determined position a width of one, so `'1` is a shift by 1 and `'0` a shift by
//! 0. The wide constant folder's `fold_shift_count` had no fill arm: it asked
//! `const_eval_u32`, which sizes a fill at a hard 32, got `0xFFFFFFFF`, and saturated
//! every such shift to zero. `localparam logic [39:0] R = 40'hFF << '1;` was
//! `0000000000` at exit 0 where iverilog and verilator both give `00000001fe`.
//!
//! CENSUS: 880 module-scope cells (5 left operands × 4 shift operators × {`'0`, `'1`}
//! × 11 declared widths × {unsigned, signed}). **288 FIXED, 0 regressed, 0 oracle
//! splits.** Every cell moved has `'1` as the amount; `'0` is a shift by zero at any
//! width and was accidentally right already.
//!
//! ⚠️ 104 cells stay wrong and this file says which: `>>`/`>>>` with `'1`, where the
//! width-UNLIMITED i64 walk (`const_eval_in_scope`) folds the expression FIRST and
//! never reaches this domain. A right shift by `0xFFFFFFFF` is a legal i64 fold that
//! answers 0, where a left shift by it overflows and declines — which is the whole
//! reason only some spellings moved. Closing those is the same hard-coded 32 at
//! `literal.rs`'s `parse_int_literal`, in the OTHER constant lane; ROADMAP §2 owns it
//! as one class with `$clog2('1)`, `$bits('1)` and the untyped-parameter fill.
//!
//! ORACLE: iverilog 13.0 (`-g2012`) and verilator 5.050 (`--binary --timing`), which
//! agree on every value asserted below.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pfsc_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let s = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&d);
    (out.status.success(), s)
}

fn param(decl: &str, expr: &str) -> String {
    format!(
        "module tb;\n\
         \x20 localparam {decl} B = {expr};\n\
         \x20 initial begin $display(\"B=%h\", B); #1 $finish; end\n\
         endmodule\n"
    )
}

fn value(decl: &str, expr: &str) -> String {
    let (ok, out) = run(&param(decl, expr));
    assert!(ok, "expected `{decl} = {expr}` to elaborate; got:\n{out}");
    out.lines()
        .find_map(|l| l.strip_prefix("B="))
        .unwrap_or_else(|| panic!("no B= line for `{decl} = {expr}`:\n{out}"))
        .to_string()
}

/// The cells the slice moves. Every value is iverilog's and verilator's; every one was
/// `0` before, because the amount was `0xFFFFFFFF` and the shift saturated.
#[test]
fn a_fill_shift_amount_is_one_bit() {
    for (decl, expr, want) in [
        ("logic [39:0]", "40'hFF << '1", "00000001fe"),
        ("logic [39:0]", "40'hFF <<< '1", "00000001fe"),
        ("logic [39:0]", "{8'h00, 8'hFF} << '1", "00000001fe"),
        ("logic [39:0]", "32'hff00 << '1", "000001fe00"),
        ("logic signed [39:0]", "40'hFF << '1", "00000001fe"),
        // wider than the i64 lane, so this domain owns both directions
        (
            "logic [94:0]",
            "128'hFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF >> '1",
            "7fffffffffffffffffffffff",
        ),
        (
            "logic [94:0]",
            "128'hFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF >>> '1",
            "7fffffffffffffffffffffff",
        ),
        (
            "logic [94:0]",
            "128'hFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF << '1",
            "7ffffffffffffffffffffffe",
        ),
    ] {
        assert_eq!(value(decl, expr), want, "`{decl} = {expr}`");
    }
}

/// `'0` is a shift by zero whatever width it is given, so it was right before and must
/// stay right — the fix must not move it. These are the control column of the census.
#[test]
fn a_zero_fill_amount_was_already_right_and_stays_right() {
    for (decl, expr, want) in [
        ("logic [39:0]", "40'hFF << '0", "00000000ff"),
        ("logic [39:0]", "40'hFF >> '0", "00000000ff"),
        (
            "logic [94:0]",
            "128'hFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF >> '0",
            "7fffffffffffffffffffffff",
        ),
    ] {
        assert_eq!(value(decl, expr), want, "`{decl} = {expr}`");
    }
}

/// ⚠️ KNOWN DIVERGENCE, pinned so closing it is visible. A RIGHT shift by the
/// mis-sized 32-bit amount is a legal i64 fold that answers 0, so the width-unlimited
/// i64 walk succeeds first and this domain is never asked. Both oracles give
/// `000000007f` and `0000000001`. 104 of the census's 880 cells are this shape; the
/// cause is the same hard-coded 32 in the other constant lane.
#[test]
fn a_right_shift_by_a_fill_is_still_answered_by_the_i64_walk() {
    assert_eq!(value("logic [39:0]", "40'hFF >> '1"), "0000000000");
    assert_eq!(value("logic [39:0]", "40'hFF >>> '1"), "0000000000");
    assert_eq!(value("logic [3:0]", "4'hF >> '1"), "0");
}

/// An x/z amount has no shift to name and stays loud, exactly as before.
#[test]
fn an_unknown_fill_amount_stays_loud() {
    for expr in ["40'hFF << 'x", "40'hFF << 'z"] {
        let (ok, out) = run(&param("logic [39:0]", expr));
        assert!(!ok, "`{expr}` must stay loud; got:\n{out}");
        assert!(out.contains("VITA-E3009"), "{out}");
    }
}

/// The amount is folded in one place, so every declaration scope sees the same value.
#[test]
fn every_declaration_scope_agrees() {
    let src = "package pk; localparam logic [39:0] PK = 40'hFF << '1; endpackage\n\
       module child #(parameter logic [39:0] CK = 40'hFF << '1)();\n\
       \x20 initial $display(\"CK=%h\", CK);\n\
       endmodule\n\
       module tb;\n\
       \x20 localparam logic [39:0] MB = 40'hFF << '1;\n\
       \x20 generate if (1) begin : g\n\
       \x20   localparam logic [39:0] GB = 40'hFF << '1;\n\
       \x20   initial $display(\"GB=%h\", GB);\n\
       \x20 end endgenerate\n\
       \x20 child u();\n\
       \x20 initial begin $display(\"MB=%h PK=%h\", MB, pk::PK); #1 $finish; end\n\
       endmodule\n";
    let (ok, out) = run(src);
    assert!(ok, "{out}");
    for want in [
        "MB=00000001fe PK=00000001fe",
        "GB=00000001fe",
        "CK=00000001fe",
    ] {
        assert!(out.contains(want), "missing `{want}`:\n{out}");
    }
}
