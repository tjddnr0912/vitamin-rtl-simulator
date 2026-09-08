//! An UNTYPED parameter whose initializer is an OPERATOR takes that operator's
//! self-determined width (IEEE Table 11-21), not the folded value's minimal
//! width. Before, `param_decl_width_opt`'s value-inferred tail
//! (`min_signed_bits(v).max(32)`) answered 32 for every one of them:
//! `localparam E = ~8'h5A` printed `ffffffa5` where both oracles print `a5`,
//! `8'hFF << 8'd9` kept the bits both shift out, `2'd3 ** 4'd10` answered 59049
//! where both answer 1, and `8'h5A > 8'h01` was 32 bits wide where both say 1.
//!
//! The arm covers `~`/`-`/`+` and EVERY binary operator, `+`/`-`/`*` included.
//! Those three were filed as an oracle split — iverilog grows the result to hold
//! the value (`8'd200 + 8'd100` is 9 bits and `12c`) where verilator keeps
//! Table 11-21's max-of-operands (8 and `2c`) — and they are not one. iverilog
//! contradicts itself twice inside a single design: its own
//! `$bits(8'd200 + 8'd100)` is **8**, the same 8 all three tools give, while the
//! parameter binding grows to 9; and it binds `32'd100000 * 32'd100000` at 64
//! bits but `32'd1 << 32'd33` at 32. Excluding the three would have handed vita
//! that same inconsistency — `const_expr_self_consistency` pins the property and
//! catches it — so the accept set is the whole operator table and it lands on
//! verilator's answer in every measured cell.
//!
//! Every value below was measured three-way on iverilog 13.0 (`-g2012`) and
//! verilator 5.052 (`--binary --timing`); the two agree on all 42 non-arithmetic
//! cells, and the arithmetic ones are pinned to verilator with iverilog's own
//! answer recorded beside them.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_upow_{}_{n}", std::process::id()));
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

/// `localparam X = <init>;` then `%h` of the value and `%0d` of `$bits`.
fn hb(init: &str) -> String {
    format!(
        "module t;\n  localparam DD = 8'hA5;\n\
         \x20 function automatic byte fb(input int z); fb = 8'h5A; endfunction\n\
         \x20 localparam X = {init};\n\
         \x20 initial begin $display(\"R %h %0d\", X, $bits(X)); $finish; end\nendmodule\n"
    )
}

fn expect(init: &str, want: &str) {
    let (out, code) = run(&hb(init));
    assert_eq!(code, Some(0), "`{init}` must elaborate:\n{out}");
    assert!(
        out.contains(&format!("R {want}")),
        "`localparam X = {init};` must read `R {want}` (both oracles):\n{out}"
    );
}

#[test]
fn a_bitwise_not_keeps_its_operands_width() {
    // The headline cell: 32-bit `ffffffa5` became the oracles' 8-bit `a5`.
    expect("~8'h5A", "a5 8");
    expect("~DD", "5a 8");
    expect("~fb(0)", "a5 8");
    expect("~(4'(8'h5A))", "5 4");
}

#[test]
fn a_bitwise_pair_takes_the_wider_operand() {
    expect("8'h5A & 8'hF0", "50 8");
    expect("8'h5A | 8'h0F", "5f 8");
    expect("8'h5A ^ 8'hFF", "a5 8");
    expect("8'h5A ~^ 8'h0F", "aa 8");
    // Mixed widths: the WIDER side, on both oracles.
    expect("16'hF0F0 & 8'h0F", "0000 16");
    expect("4'h5 | 12'h0F0", "0f5 12");
    expect("4'h5 ^ 20'h5", "00000 20");
    // Above 32 the value-inferred tail was wrong in the other direction too.
    expect("33'h1_0000_0000 & 8'hFF", "000000000 33");
}

#[test]
fn a_shift_takes_the_left_operand_and_drops_what_leaves_it() {
    // Value-wrong before, not merely width-wrong: the shifted-out bits survived.
    expect("8'hFF << 2", "fc 8");
    expect("8'hFF << 8'd9", "00 8");
    expect("8'h0F <<< 4'd4", "f0 8");
    expect("8'hFF >> 2", "3f 8");
    expect("8'sh80 >>> 2", "e0 8");
    expect("16'sh8000 >>> 4", "f800 16");
}

#[test]
fn a_power_takes_the_base_width_and_wraps_in_it() {
    expect("8'd3 ** 8'd2", "09 8");
    expect("8'd3 ** 8'd6", "d9 8");
    // 3**10 is 59049; at the base's 2 bits both oracles answer 1.
    expect("2'd3 ** 4'd10", "1 2");
}

#[test]
fn division_and_modulo_take_the_wider_operand() {
    expect("8'h5A / 8'h03", "1e 8");
    expect("8'h5A % 8'h07", "06 8");
    expect("16'h5AFF / 8'h03", "1e55 16");
    expect("8'h5A / 16'h0003", "001e 16");
    expect("8'h5A % 16'h0007", "0006 16");
}

#[test]
fn a_comparison_or_logical_operator_is_one_bit() {
    expect("8'h5A > 8'h01", "1 1");
    expect("8'h5A == 8'h5A", "1 1");
    expect("8'h5A && 8'h01", "1 1");
    // …and an operator ABOVE one of them inherits the single bit.
    expect("~(8'h5A > 8'h01)", "0 1");
    expect("-(8'h5A > 8'h01)", "1 1");
}

#[test]
fn a_unary_minus_carries_the_operand_shape_the_peel_loop_cannot_reach() {
    // The peel loop above this arm reaches a bare LITERAL through `-`/`+`
    // (`-8'h5A` was already `a6 8`); these are the shapes it cannot reach.
    expect("-8'h5A", "a6 8");
    expect("-(~8'h5A)", "5b 8");
    expect("-{4'h5,4'hA}", "a6 8");
    expect("~{4'h5,4'hA}", "a5 8");
    expect("-DD[3:0]", "b 4");
    expect("~DD[3:0]", "a 4");
    expect("-fb(0)", "a6 8");
    expect("-(8'h5A & 8'hF0)", "b0 8");
    expect("-(8'h5A << 1)", "4c 8");
    expect("~(1 ? 8'h5A : 8'h01)", "a5 8");
}

#[test]
fn the_arithmetic_operators_take_max_of_operands() {
    // Pinned to verilator and to Table 11-21. iverilog answers `05b 9` / `059 9`
    // / `00b4 16` here, and `12c 9` for the overflowing sum below — the same tool
    // whose `$bits` of those very expressions answers 8.
    expect("8'h5A + 8'h01", "5b 8");
    expect("8'h5A - 8'h01", "59 8");
    expect("8'h5A * 8'h02", "b4 8");
    // The cells where the width is not merely cosmetic: the sum overflows its
    // operands, so 9 bits and 8 bits hold DIFFERENT values.
    expect("8'd200 + 8'd100", "2c 8");
    expect("(8'd200 + 8'd100) << 0", "2c 8");
    expect("(8'd200 + 8'd100) ** 1", "2c 8");
    expect("~(8'h5A + 8'h01)", "a4 8");
    expect("{(8'h5A + 8'h01)} & 8'hFF", "5b 8");
    expect("(8'h5A + 8'h01) << 1", "b6 8");
    // A shift's RIGHT operand is self-determined and does not widen the result.
    expect("8'hFF << (4'd1 + 4'd1)", "fc 8");
}

#[test]
fn the_operators_no_longer_disagree_about_the_domain() {
    // `const_expr_self_consistency` owns this property; the cell is here too
    // because it is what forced `+`/`-`/`*` INTO the accept set. With them out,
    // `*` kept the wide value while `<<` wrapped at 32 — iverilog's own split, in
    // one design. Both wrap now, which is verilator's answer for both.
    let (out, code) = run(
        "module t;\n  localparam MUL = 32'd100000 * 32'd100000;\n\
         \x20 localparam SHL = 32'd1 << 32'd33;\n\
         \x20 initial begin $display(\"R %0d %0d %0d %0d\", MUL, $bits(MUL), SHL, $bits(SHL)); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("R 1410065408 32 0 32"),
        "both operators fold in the 32-bit domain (verilator):\n{out}"
    );
}

#[test]
fn the_value_folds_width_unlimited_below_the_top_operator() {
    // NOT a bug pin for this slice — a KNOWN residue, recorded so a later reader
    // does not read the passing width as a passing value. The width rule now says
    // 8 for `(8'd200 + 8'd100) >> 1`, and both the top-level mask and every other
    // cell above agree with verilator; but the VALUE still folds through
    // `const_eval_in_scope`, which is width-UNLIMITED, so the inner sum keeps 300
    // and the shift reads a bit the 8-bit sum does not have: vita `96`, verilator
    // `16`. PRE answered `96` at 32 bits, so this is an unfixed pre-existing
    // defect of the value fold (its width-aware twin is `eval_const_env_self`),
    // not one this arm introduced — ROADMAP §2 carries it as its own row, because
    // routing it changes the value of every untyped parameter.
    expect("(8'd200 + 8'd100) >> 1", "96 8");
}

#[test]
fn the_shapes_that_were_already_right_do_not_move() {
    // Control twins: every one of these was correct before the arm and reads the
    // same after it — the sized/decimal literal arms, the reduction and select
    // arms, the concatenation arm, the ternary arm, the cast and call arms.
    expect("8'h5A", "5a 8");
    expect("&8'h5A", "0 1");
    expect("!8'h5A", "0 1");
    expect("{4'h5,4'hA}", "5a 8");
    expect("{2{4'h5}}", "55 8");
    expect("DD[3:0]", "5 4");
    expect("1 ? 8'h5A : 8'h01", "5a 8");
    expect("4'(8'h5A)", "a 4");
    expect("fb(0)", "5a 8");
    expect("DD", "a5 8");
}

#[test]
fn a_declared_width_still_wins_over_the_operator() {
    // The arm is the UNTYPED tail only: a declared range and a declared atom
    // state the width outright and must keep doing so.
    expect("8'h5A", "5a 8");
    let (out, code) = run(
        "module t;\n  localparam logic [15:0] X = ~8'h5A;\n  localparam int Y = ~8'h5A;\n\
         \x20 initial begin $display(\"R %h %0d %h %0d\", X, $bits(X), Y, $bits(Y)); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    // `ffa5`, not `00a5`: a declared 16-bit CONTEXT widens the operand before `~`
    // inverts it. All three tools agree, and that is the point — the declaration
    // decides, not the operator.
    assert!(
        out.contains("R ffa5 16 ffffffa5 32"),
        "a declared 16-bit and a declared `int` keep their own widths:\n{out}"
    );
}
