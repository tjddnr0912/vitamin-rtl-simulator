//! ROADMAP §2 "Index sealing" (I4): a parameter override whose TOP is an operator and
//! whose width is past 64 bits.
//!
//! ## The mechanism
//!
//! An override expression reaches the child through one of two channels. The WIDE
//! channel (`Elaborator::override_bits`) folds in the bit domain at the expression's own
//! width; it used to admit only a self-determined top or a bitwise `& | ^` tree. Every
//! other operator top (`~`, unary `-`/`+`, arithmetic, shifts, `%`, `**`, `?:`) fell to
//! the i64 OPERATOR channel (`override_self_meta` + `override_self_value`), whose accept
//! set requires `const_ctx_within_i64` — false for any known width past 64. So a
//! `#(.P(~128'd0))` bound through the bare i64 with the DEFAULT literal's type: 32 bits
//! of ones where both oracles bind 128, and a shift or division that the i64 could not
//! fold at all was E3009.
//!
//! ## The rule
//!
//! `override_bits` now also folds an operator top the domain has an arm for, when the
//! tree is PLAIN (`wide_operator_tree_is_plain`) and its SELF-determined width
//! (Table 11-21; names answered from DECLARED provenance only, so `~W` over a
//! `parameter [127:0] W` counts as 128) is past 64 bits. PLAIN means:
//!
//! * every self-determined position (cast operand, concat part, comparison / logical
//!   operand, shift count, `**` exponent, ternary condition, reduction operand, select
//!   base / index, system-function argument) holds a plain leaf — a sized literal or a
//!   name — never an operator, cast, concatenation, select or call;
//! * the tree is sign-homogeneous: every node reached through the context-determined
//!   arms has the top's sign (a position's leaf is exempt: it keeps its own type).
//!
//! No fill anywhere, and the folded value has no x/z bit. The width comes from a first
//! fold at no context; the value from a second fold AT that width, because §11.6.1 pushes
//! the tree's width into every context-determined operand (`~8'd1 + 128'd0` complements
//! at 128, not at 8). Every other tree keeps the pre-slice route, byte for byte.
//!
//! WHY the plain rule and not more: the shared wide walk (`fold_bits_at`) folds a
//! self-determined position with no context (`{~8'd1 + 120'd0} + 128'd0` gives `0…0fe`,
//! both oracles `00ff…fe`) and does not push §11.8.2's expression sign into the operands
//! (`~S8 + 128'd0` gives `0…02`, both oracles `ff…f02`). Both defects are pre-existing on
//! the localparam lane; repairing that walk is a prerequisite slice. Three review rounds
//! patched exclusions over it and each round found the next hole on the same axis, so
//! this slice ships only the trees on which the walk is already right.
//!
//! The width is verilator's for `+ - *`; iverilog binds those at max+1 (or the sum for
//! `*`) while its own `$bits` of the same text says the operand width — the §4.5.466
//! self-contradiction, so it is not evidence for the width column there.
//!
//! ## Typed targets
//!
//! On a `parameter logic [127:0] P` the self-folded value is extended to 128 bits. Where
//! the expression is narrower than the target the two oracles split (iverilog folds in
//! the TARGET's context, verilator self-folds and extends — ROADMAP §2 row 16); vita lands
//! on verilator's side, the rule `~64'd0` already followed. Those cells are pinned as the
//! recorded split, not as two-oracle support.
//!
//! Oracles: iverilog 13.0 `-g2012`, verilator 5.052 `--binary --timing`. Every design
//! carries a `#N $finish` watchdog.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_owot_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
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

/// The value lines, sorted — several instances share one design, and the sort keeps the
/// comparison independent of display order.
fn lines(out: &str) -> Vec<String> {
    let mut v: Vec<String> = out
        .lines()
        .filter(|l| l.contains(" bits="))
        .map(|l| l.trim().to_string())
        .collect();
    v.sort();
    v
}

const SUB: &str = "module sub #(parameter P = 1) (); initial $display(\"%m bits=%0d hex=%h\", $bits(P), P); endmodule\n";
const TSUB: &str = "module tsub #(parameter logic [127:0] P = 1) (); initial $display(\"%m bits=%0d hex=%h\", $bits(P), P); endmodule\n";
const WS: &str = "  parameter [127:0] W = 128'h0000_0000_0000_0001_0000_0000_0000_00F0;\n  parameter [7:0] W8 = 8'hCB;\n";

#[track_caller]
fn check(src: &str, want: &[&str]) {
    let (o, c) = run(src);
    assert_eq!(c, Some(0), "{o}");
    assert!(!o.contains("error["), "{o}");
    let mut want: Vec<String> = want.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(lines(&o), want, "{o}");
}

/// Untyped target, silent → correct. PRE bound every one at the default literal's 32
/// bits from the truncated i64 (`u01`–`v15` below were `bits=32`, and `v15`'s value was
/// `ffffffff`). Both oracles agree on every VALUE; on `+ - *` (`u07 u10 u13 v13`) the
/// width is verilator's (iverilog 129/129/66/256 — its own `$bits` says 128/128/65/128).
#[test]
fn untyped_target_silent_cells_bind_the_expression_own_width() {
    let src = format!(
        "{SUB}module top;\n{WS}\
         \x20 sub #(.P(~128'd0)) u01();\n\
         \x20 sub #(.P(~65'd0)) u03();\n\
         \x20 sub #(.P(-128'sd1)) u05();\n\
         \x20 sub #(.P(128'd1 + 128'd2)) u07();\n\
         \x20 sub #(.P(128'd0 - 128'd1)) u10();\n\
         \x20 sub #(.P(~(128'd0))) u11();\n\
         \x20 sub #(.P(~128'd0 & 128'hFF)) u12();\n\
         \x20 sub #(.P(65'd1 - 65'd2)) u13();\n\
         \x20 sub #(.P(~96'd0)) u16();\n\
         \x20 sub #(.P(1 ? ~128'd0 : 128'd1)) v06();\n\
         \x20 sub #(.P(~128'sd0)) v07();\n\
         \x20 sub #(.P(-(128'sd5))) v08();\n\
         \x20 sub #(.P(128'd3 * 128'd5)) v13();\n\
         \x20 sub #(.P(~128'd0 % 128'd7)) v15();\n\
         \x20 initial #10 $finish;\nendmodule\n"
    );
    check(
        &src,
        &[
            "top.u01 bits=128 hex=ffffffffffffffffffffffffffffffff",
            "top.u03 bits=65 hex=1ffffffffffffffff",
            "top.u05 bits=128 hex=ffffffffffffffffffffffffffffffff",
            "top.u07 bits=128 hex=00000000000000000000000000000003",
            "top.u10 bits=128 hex=ffffffffffffffffffffffffffffffff",
            "top.u11 bits=128 hex=ffffffffffffffffffffffffffffffff",
            "top.u12 bits=128 hex=000000000000000000000000000000ff",
            "top.u13 bits=65 hex=1ffffffffffffffff",
            "top.u16 bits=96 hex=ffffffffffffffffffffffff",
            "top.v06 bits=128 hex=ffffffffffffffffffffffffffffffff",
            "top.v07 bits=128 hex=ffffffffffffffffffffffffffffffff",
            "top.v08 bits=128 hex=fffffffffffffffffffffffffffffffb",
            "top.v13 bits=128 hex=0000000000000000000000000000000f",
            "top.v15 bits=128 hex=00000000000000000000000000000003",
        ],
    );
}

/// Untyped target, loud → value. PRE: `W3056` + `E3009` (the i64 could not fold the
/// value). Both oracles agree on all nine; `v10`'s width is verilator's (iverilog 129).
/// `v04` is the declared-width NAME leaf that `const_ctx_within_i64` passes as ≤64.
#[test]
fn untyped_target_loud_cells_now_bind() {
    let src = format!(
        "{SUB}module top;\n{WS}\
         \x20 sub #(.P(128'hFF << 64)) u04();\n\
         \x20 sub #(.P(~128'd0 >> 1)) u06();\n\
         \x20 sub #(.P(~65'd0 >> 1)) u14();\n\
         \x20 sub #(.P((~128'd0) >> 64)) u15();\n\
         \x20 sub #(.P(~W)) v04();\n\
         \x20 sub #(.P(~128'd0 ^ W)) v05();\n\
         \x20 sub #(.P(W + 128'd1)) v10();\n\
         \x20 sub #(.P(128'hFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF / 128'd3)) v14();\n\
         \x20 sub #(.P(~(128'd0) & W)) v16();\n\
         \x20 initial #10 $finish;\nendmodule\n"
    );
    check(
        &src,
        &[
            "top.u04 bits=128 hex=00000000000000ff0000000000000000",
            "top.u06 bits=128 hex=7fffffffffffffffffffffffffffffff",
            "top.u14 bits=65 hex=0ffffffffffffffff",
            "top.u15 bits=128 hex=0000000000000000ffffffffffffffff",
            "top.v04 bits=128 hex=fffffffffffffffeffffffffffffff0f",
            "top.v05 bits=128 hex=fffffffffffffffeffffffffffffff0f",
            "top.v10 bits=128 hex=000000000000000100000000000000f1",
            "top.v14 bits=128 hex=55555555555555555555555555555555",
            "top.v16 bits=128 hex=000000000000000100000000000000f0",
        ],
    );
}

/// A DECLARED-width name leaf past 64 bits, package-qualified, wildcard-imported and
/// plain — the `C1_pkq80_not` / `C1_plp80_not` residues `declared_leaf_certification.rs`
/// recorded. PRE bound the default literal's 1 bit (`bits=1`). Both oracles: 80 bits.
#[test]
fn a_wide_declared_name_leaf_under_an_operator_binds_its_width() {
    let src = "package pk;\n  parameter logic [79:0] PW80 = 80'h0a;\nendpackage\n\
         module leaf #(parameter P = 1'b0) (); initial $display(\"%m bits=%0d hex=%h\", $bits(P), P); endmodule\n\
         module top;\n\
         \x20 import pk::*;\n\
         \x20 parameter logic [79:0] Q = 80'h0a;\n\
         \x20 leaf #(.P(~pk::PW80)) pkq();\n\
         \x20 leaf #(.P(~Q)) plp();\n\
         \x20 leaf #(.P(-PW80)) pkimp();\n\
         \x20 initial #10 $finish;\nendmodule\n";
    check(
        src,
        &[
            "top.pkimp bits=80 hex=fffffffffffffffffff6",
            "top.pkq bits=80 hex=fffffffffffffffffff5",
            "top.plp bits=80 hex=fffffffffffffffffff5",
        ],
    );
}

/// The remaining operator arms and the sign axis, measured at the slice: `**`, signed
/// `/` and `%`, `>>>` on a signed operand, `<<<` over a declared name, a shift past the
/// width, a narrow name beside a wide literal, and a `signed` keyword on the target
/// (§12.2.1 keeps it; the range comes from the override). PRE: `bits=32` for all but
/// `x09 x14`, which were E3009. Both oracles agree on every value; `x10 x11` widths are
/// verilator's (iverilog 129). (`{64'd1, 64'd2} + 1` mixes an unsigned concat with the
/// signed `1` and keeps its route — `a_mixed_sign_tree_keeps_its_route`.) `~128'bx` stays E3009 (the fold
/// declines an unknown operand) where both oracles bind 128 x's — unchanged loud.
#[test]
fn further_operator_arms_and_the_sign_keyword() {
    let src = format!(
        "{SUB}module sgn #(parameter signed P = 1) (); initial $display(\"%m bits=%0d hex=%h dec=%0d\", $bits(P), P, P); endmodule\n\
         module top;\n{WS}\
         \x20 sub #(.P(128'd2 ** 3)) x02();\n\
         \x20 sub #(.P(-128'sd7 / 128'sd2)) x03();\n\
         \x20 sub #(.P(-128'sd8 >>> 1)) x04();\n\
         \x20 sgn #(.P(~128'd0)) x08();\n\
         \x20 sub #(.P(W <<< 4)) x09();\n\
         \x20 sub #(.P(W8 + 128'd0)) x10();\n\
         \x20 sub #(.P(~128'd0 + 1'b1)) x11();\n\
         \x20 sub #(.P(-128'sd7 % 128'sd2)) x13();\n\
         \x20 sub #(.P(128'd1 << 200)) x14();\n\
         \x20 initial #10 $finish;\nendmodule\n"
    );
    check(
        &src,
        &[
            "top.x02 bits=128 hex=00000000000000000000000000000008",
            "top.x03 bits=128 hex=fffffffffffffffffffffffffffffffd",
            "top.x04 bits=128 hex=fffffffffffffffffffffffffffffffc",
            "top.x08 bits=128 hex=ffffffffffffffffffffffffffffffff dec=-1",
            "top.x09 bits=128 hex=00000000000000100000000000000f00",
            "top.x10 bits=128 hex=000000000000000000000000000000cb",
            "top.x11 bits=128 hex=00000000000000000000000000000000",
            "top.x13 bits=128 hex=ffffffffffffffffffffffffffffffff",
            "top.x14 bits=128 hex=00000000000000000000000000000000",
        ],
    );
}

/// Typed `logic [127:0]` target where the expression is already 128 bits, so the two
/// oracles agree. PRE: `tu01 tu10 tu11` were `0000000000000000ffffffffffffffff` (the
/// truncated i64 extended), `tu04 tu06 tu15` were E3009.
#[test]
fn typed_target_two_oracle_cells() {
    let src = format!(
        "{TSUB}module top;\n\
         \x20 tsub #(.P(~128'd0)) tu01();\n\
         \x20 tsub #(.P(128'd0 - 128'd1)) tu10();\n\
         \x20 tsub #(.P(~(128'd0))) tu11();\n\
         \x20 tsub #(.P(128'hFF << 64)) tu04();\n\
         \x20 tsub #(.P(~128'd0 >> 1)) tu06();\n\
         \x20 tsub #(.P((~128'd0) >> 64)) tu15();\n\
         \x20 initial #10 $finish;\nendmodule\n"
    );
    check(
        &src,
        &[
            "top.tu01 bits=128 hex=ffffffffffffffffffffffffffffffff",
            "top.tu04 bits=128 hex=00000000000000ff0000000000000000",
            "top.tu06 bits=128 hex=7fffffffffffffffffffffffffffffff",
            "top.tu10 bits=128 hex=ffffffffffffffffffffffffffffffff",
            "top.tu11 bits=128 hex=ffffffffffffffffffffffffffffffff",
            "top.tu15 bits=128 hex=0000000000000000ffffffffffffffff",
        ],
    );
}

/// Typed `logic [127:0]` target where the expression is NARROWER than the target: a
/// recorded oracle split (ROADMAP §2 row 16), NOT two-oracle support. vita gives
/// verilator's answer (self-fold, then zero-extend); iverilog folds in the target's
/// context and prints `ff…ff` for the first three and `7ff…ff` for `tu14`. `tu08` is the
/// ≤64-bit cell that already took verilator's side before this slice.
/// PRE: `tu03 tu13 tu16` were `0000000000000000ffffffffffffffff`, `tu14` was E3009.
#[test]
fn typed_target_narrower_expression_is_the_recorded_split() {
    let src = format!(
        "{TSUB}module top;\n\
         \x20 tsub #(.P(~65'd0)) tu03();\n\
         \x20 tsub #(.P(65'd1 - 65'd2)) tu13();\n\
         \x20 tsub #(.P(~96'd0)) tu16();\n\
         \x20 tsub #(.P(~65'd0 >> 1)) tu14();\n\
         \x20 tsub #(.P(~64'd0)) tu08();\n\
         \x20 initial #10 $finish;\nendmodule\n"
    );
    check(
        &src,
        &[
            "top.tu03 bits=128 hex=0000000000000001ffffffffffffffff",
            "top.tu08 bits=128 hex=0000000000000000ffffffffffffffff",
            "top.tu13 bits=128 hex=0000000000000001ffffffffffffffff",
            "top.tu14 bits=128 hex=0000000000000000ffffffffffffffff",
            "top.tu16 bits=128 hex=00000000ffffffffffffffffffffffff",
        ],
    );
}

/// The other channels share the one helper: a `defparam` (`instance.rs`'s collector) and
/// an interface `#()` override (`iface_inst.rs`). PRE: all three were `bits=32`. Both
/// oracles agree except `if02`'s width (iverilog 129, verilator 128). The ≤64-bit twins
/// (`dp02`, `if03`) are unchanged.
#[test]
fn defparam_and_interface_channels_take_the_same_rule() {
    let src = format!(
        "{SUB}interface ifc #(parameter P = 1) (); initial $display(\"%m bits=%0d hex=%h\", $bits(P), P); endinterface\n\
         module top;\n\
         \x20 sub dp01();\n\
         \x20 defparam dp01.P = ~128'd0;\n\
         \x20 sub dp02();\n\
         \x20 defparam dp02.P = ~32'd0;\n\
         \x20 ifc #(.P(~128'd0)) if01();\n\
         \x20 ifc #(.P(128'd1 + 128'd2)) if02();\n\
         \x20 ifc #(.P(~32'd0)) if03();\n\
         \x20 initial #10 $finish;\nendmodule\n"
    );
    check(
        &src,
        &[
            "top.dp01 bits=128 hex=ffffffffffffffffffffffffffffffff",
            "top.dp02 bits=32 hex=ffffffff",
            "top.if01 bits=128 hex=ffffffffffffffffffffffffffffffff",
            "top.if02 bits=128 hex=00000000000000000000000000000003",
            "top.if03 bits=32 hex=ffffffff",
        ],
    );
}

/// CONTROLS — the exact pre-slice strings. `v01 v02 v03 v11 u08` are ≤64-bit operator
/// tops (the operator channel's, byte-identical); `u02 v12` wide literals, `u09` a
/// bitwise literal tree, `v09` a cast top (the wide channel's self-determined arm);
/// `v17` a typed target. `v03`'s width is verilator's (iverilog 9, its own `$bits` 8).
#[test]
fn a_64_bit_and_narrower_operator_top_is_unchanged() {
    let src = format!(
        "{SUB}{TSUB}module top;\n{WS}\
         \x20 sub #(.P(~32'd0)) v01();\n\
         \x20 sub #(.P(-8'sd1)) v02();\n\
         \x20 sub #(.P(8'd1 + 8'd2)) v03();\n\
         \x20 sub #(.P(~W8)) v11();\n\
         \x20 sub #(.P(~64'd0)) u08();\n\
         \x20 sub #(.P(128'd5)) u02();\n\
         \x20 sub #(.P(128'hDEADBEEF_00000000_11112222_33334444)) v12();\n\
         \x20 sub #(.P(128'hFFFFFFFFFFFFFFFF0000000000000001 | 128'd0)) u09();\n\
         \x20 sub #(.P($signed(~128'd0))) v09();\n\
         \x20 tsub #(.P(128'hDEADBEEF_00000000_11112222_33334444)) v17();\n\
         \x20 initial #10 $finish;\nendmodule\n"
    );
    check(
        &src,
        &[
            "top.u02 bits=128 hex=00000000000000000000000000000005",
            "top.u08 bits=64 hex=ffffffffffffffff",
            "top.u09 bits=128 hex=ffffffffffffffff0000000000000001",
            "top.v01 bits=32 hex=ffffffff",
            "top.v02 bits=8 hex=ff",
            "top.v03 bits=8 hex=03",
            "top.v09 bits=128 hex=ffffffffffffffffffffffffffffffff",
            "top.v11 bits=8 hex=34",
            "top.v12 bits=128 hex=deadbeef000000001111222233334444",
            "top.v17 bits=128 hex=deadbeef000000001111222233334444",
        ],
    );
}

/// X1 (review round 1): a context-determined operand NARROWER than the tree is computed
/// at the tree's width (§11.6.1). A one-pass fold at no context computed `~8'd1` at 8
/// bits and zero-extended it. Typed targets: PRE (the i64 route) and both oracles agree
/// on every cell; the cells must stay there. `f` is a `defparam`, `i` an interface.
#[test]
fn a_narrow_context_determined_operand_is_computed_at_the_tree_width_typed() {
    let src = "module si #(parameter int P = 7) (); initial $display(\"%m int hex=%h\", P); endmodule\n\
         module sl #(parameter longint P = 7) (); initial $display(\"%m longint hex=%h\", P); endmodule\n\
         module ss #(parameter logic signed [127:0] P = 7) (); initial $display(\"%m s128 hex=%h\", P); endmodule\n\
         interface ifc #(parameter int P = 7) (); endinterface\n\
         module top;\n\
         \x20 si #(.P(128'd0 + (8'd200 + 8'd100))) a();\n\
         \x20 si #(.P(~16'd1 * 128'd1)) b();\n\
         \x20 sl #(.P(~8'd1 + 128'd0)) c();\n\
         \x20 sl #(.P(-8'd1 + 128'd0)) d();\n\
         \x20 ss #(.P(~8'd1 + 128'd0)) e();\n\
         \x20 si f();\n\
         \x20 defparam f.P = 128'd0 + (8'd200 + 8'd100);\n\
         \x20 ifc #(.P(~8'd1 + 128'd0)) i();\n\
         \x20 initial begin #1 $display(\"top.i int hex=%h\", i.P); #10 $finish; end\n\
         endmodule\n";
    let (o, c) = run(src);
    assert_eq!(c, Some(0), "{o}");
    let mut got: Vec<&str> = o.lines().filter(|l| l.starts_with("top.")).collect();
    got.sort();
    assert_eq!(
        got,
        [
            "top.a int hex=0000012c",
            "top.b int hex=fffffffe",
            "top.c longint hex=fffffffffffffffe",
            "top.d longint hex=ffffffffffffffff",
            "top.e s128 hex=fffffffffffffffffffffffffffffffe",
            "top.f int hex=0000012c",
            "top.i int hex=fffffffe",
        ],
        "{o}"
    );
}

/// X1, untyped target: loud → value (`l1`–`l5`, `sh` were E3009 PRE) and silent →
/// correct (`l6` `nw` `o` were 32 bits PRE: `ffffffff`, `ffffff34`, `0000012c`). Both
/// oracles agree on every value; `l2 l3 nw o` widths are verilator's (iverilog 129) and
/// `l5` is verilator's (iverilog 136).
#[test]
fn a_narrow_context_determined_operand_is_computed_at_the_tree_width_untyped() {
    let src = format!(
        "{SUB}module top;\n{WS}\
         \x20 sub #(.P(~32'd5 & 128'hFFFF_0000_0000_0000_FFFF_FFFF_FFFF_FFFF)) l1();\n\
         \x20 sub #(.P((8'hFF + 8'd1) + 128'h1_0000_0000_0000_0000)) l2();\n\
         \x20 sub #(.P(-32'd1 + 128'h1_0000_0000_0000_0000)) l3();\n\
         \x20 sub #(.P((8'hF0 << 4) | 128'h1_0000_0000_0000_0000)) l4();\n\
         \x20 sub #(.P((~8'd0) * 128'h1_0000_0000_0000_0000)) l5();\n\
         \x20 sub #(.P(1 ? ~8'd0 : 128'd0)) l6();\n\
         \x20 sub #(.P((~8'd1 + 128'd0) >> 1)) sh();\n\
         \x20 sub #(.P(~W8 + 128'd0)) nw();\n\
         \x20 sub #(.P(128'd0 + (8'd200 + 8'd100))) o();\n\
         \x20 initial #10 $finish;\nendmodule\n"
    );
    check(
        &src,
        &[
            "top.l1 bits=128 hex=ffff000000000000fffffffffffffffa",
            "top.l2 bits=128 hex=00000000000000010000000000000100",
            "top.l3 bits=128 hex=0000000000000000ffffffffffffffff",
            "top.l4 bits=128 hex=00000000000000010000000000000f00",
            "top.l5 bits=128 hex=ffffffffffffffff0000000000000000",
            "top.l6 bits=128 hex=ffffffffffffffffffffffffffffffff",
            "top.nw bits=128 hex=ffffffffffffffffffffffffffffff34",
            "top.o bits=128 hex=0000000000000000000000000000012c",
            "top.sh bits=128 hex=7fffffffffffffffffffffffffffffff",
        ],
    );
}

/// X2: an x/z bit in the folded value keeps the pre-slice E3009. The binder drops the
/// unknown plane when the value bits fit the i64 lane (ROADMAP §2 row 15), so binding it
/// would print zeros where both oracles keep the x's. One design per cell: each is loud
/// on its own.
#[test]
fn an_unknown_bit_keeps_the_override_loud() {
    for ov in [
        "sub #(.P(128'hx0 >> 4)) x();",
        "sub #(.P(+128'hx0)) x();",
        "sub #(.P(1'b1 ? 128'hx0 : 128'd0)) x();",
        "ifc #(.P(+128'hx0)) x();",
    ] {
        let src = format!(
            "{SUB}interface ifc #(parameter P = 1) (); endinterface\n\
             module top;\n  {ov}\n  initial #10 $finish;\nendmodule\n"
        );
        let (o, c) = run(&src);
        assert_eq!(c, Some(1), "{ov}\n{o}");
        assert!(o.contains("VITA-E3009"), "{ov}\n{o}");
    }
}

/// X3: a tree whose self-determined width is at most 64 keeps the i64 operator route
/// even when a SELF-determined sub-node (comparison operand, shift count) is 128 bits —
/// the exact PRE strings. The typed `logic [15:0]` cells are a live oracle split
/// (iverilog `0100` / `01fe`, verilator `0000` / `00fe`) and stay beside their text twin
/// `8'hFF + 8'd1`; the untyped cells are the i64 route's pre-existing 32-bit answer
/// (both oracles: `shcnt` 8/04, `add_mix` 8/fd (VL), `not1` 1/0).
#[test]
fn a_narrow_top_with_a_wide_self_determined_operand_keeps_its_route() {
    let src = "module su #(parameter P = 1) (); initial $display(\"%m bits=%0d hex=%h\", $bits(P), P); endmodule\n\
         module st #(parameter logic [15:0] P = 1) (); initial $display(\"%m bits=%0d hex=%h\", $bits(P), P); endmodule\n\
         module top;\n\
         \x20 st #(.P(8'hFF + (128'd1 > 128'd0))) t_add();\n\
         \x20 st #(.P(8'hFF << (128'd1 > 128'd0))) t_shl();\n\
         \x20 st #(.P((8'hFF + 8'd1) + 16'd0 + (128'd1 > 128'd0))) k2();\n\
         \x20 su #(.P(8'd1 << 128'd2)) shcnt();\n\
         \x20 su #(.P(-8'sd4 + (128'd1 > 128'd0))) add_mix();\n\
         \x20 su #(.P(~(128'd1 != 128'd0))) not1();\n\
         \x20 initial #10 $finish;\nendmodule\n";
    check(
        src,
        &[
            "top.add_mix bits=32 hex=fffffffd",
            "top.k2 bits=16 hex=0101",
            "top.not1 bits=32 hex=fffffffe",
            "top.shcnt bits=32 hex=00000004",
            "top.t_add bits=16 hex=0100",
            "top.t_shl bits=16 hex=01fe",
        ],
    );
}

/// A MIXED-sign tree keeps its pre-slice route: the exact PRE strings. The wide walk
/// does not push §11.8.2's expression sign into the operands, so these are the trees
/// where folding could import that defect (`o2 o3 o5 o6` would; the localparam lane
/// gives `localparam logic [127:0] L = ~S8 + 128'd0;` as `0…02`, both oracles `ff…f02`
/// — ROADMAP §2 residue). `o4 o7 x07` would fold right today and are over-excluded by
/// the structural rule: their PRE answers (32 bits, or E3009 for `x07`) are the
/// pre-existing residue, both oracles giving 128 bits `0…0fd` / `ff…ff` / `0…10…03`.
#[test]
fn a_mixed_sign_tree_keeps_its_route() {
    let src = format!(
        "{SUB}module top;\n\
         \x20 parameter signed [7:0] S8 = -8'sd3;\n\
         \x20 sub #(.P(~S8 + 128'd0)) o2();\n\
         \x20 sub #(.P(-(8'sh80) + 128'd0)) o3();\n\
         \x20 sub #(.P(S8 + 128'd0)) o4();\n\
         \x20 sub #(.P((S8 >>> 1) + 128'd0)) o5();\n\
         \x20 sub #(.P((8'sh80 * 8'sd1) + 128'd0)) o6();\n\
         \x20 sub #(.P(-8'sd1 + 128'd0)) o7();\n\
         \x20 initial #10 $finish;\nendmodule\n"
    );
    check(
        &src,
        &[
            "top.o2 bits=32 hex=00000002",
            "top.o3 bits=32 hex=00000080",
            "top.o4 bits=32 hex=fffffffd",
            "top.o5 bits=32 hex=fffffffe",
            "top.o6 bits=32 hex=ffffff80",
            "top.o7 bits=32 hex=ffffffff",
        ],
    );
    for ov in [
        "(-8'sd1 >>> 1) ^ 128'h1_0000_0000_0000_0000",
        "{64'd1, 64'd2} + 1",
    ] {
        let (o, c) = run(&format!(
            "{SUB}module top;\n  sub #(.P({ov})) a();\n  initial #10 $finish;\nendmodule\n"
        ));
        assert_eq!(c, Some(1), "{ov}\n{o}");
        assert!(o.contains("VITA-E3009"), "{ov}\n{o}");
    }
}

/// A self-determined position whose inner is an OPERATOR keeps its pre-slice route:
/// the walk folds a position with no context, so an operator inside it narrower than
/// the position computes at its own width (`e19 e20 c2 c1` would, both oracles giving
/// `ff…fe` / `00ff…fe` / `0` / `1`; the localparam lane has the same defect — ROADMAP §2
/// residue). `f1 f4` (E3009) and `f5` (32 bits) would fold right today and are
/// over-excluded by the structural rule. Exact PRE strings / E3009, one design each.
#[test]
fn a_position_whose_inner_is_an_operator_keeps_its_route() {
    for ov in [
        "$signed(~8'd1 + 128'd0) + 128'sd0",
        "{~8'd1 + 120'd0} + 128'd0",
        "128'd1 << (~4'd0 + 8'd0)",
        "$signed(128'd1 + 128'd2) + 128'sd0",
        "~128'd0 >> (8'd2 + 8'd2)",
    ] {
        let (o, c) = run(&format!(
            "{SUB}module top;\n  sub #(.P({ov})) a();\n  initial #10 $finish;\nendmodule\n"
        ));
        assert_eq!(c, Some(1), "{ov}\n{o}");
        assert!(o.contains("VITA-E3009"), "{ov}\n{o}");
    }
    check(
        &format!(
            "{SUB}module top;\n  sub #(.P((~8'd1 == 16'hFFFE) + 128'd0)) c1();\n  sub #(.P((~128'd0 == ~128'd0) + 128'd8)) f5();\n  initial #10 $finish;\nendmodule\n"
        ),
        &["top.c1 bits=32 hex=00000001", "top.f5 bits=32 hex=00000009"],
    );
}

/// Plain trees fold. Leaf-only positions (a concat of literals or names, a comparison
/// of literals, a select of a name) and a sign-homogeneous tree. PRE: `f2 p1 p2 p3` were
/// E3009, `f3` 32 bits. Both oracles agree on every value; widths on `+` are verilator's
/// (iverilog one wider).
#[test]
fn leaf_only_positions_fold() {
    let src = format!(
        "{SUB}module top;\n{WS}\
         \x20 sub #(.P({{8'd1, 120'd0}} + 128'd0)) f2();\n\
         \x20 sub #(.P((128'd1 > 128'd0) + 128'd7)) f3();\n\
         \x20 sub #(.P({{W8, W}} + 136'd0)) p1();\n\
         \x20 sub #(.P(W[127:60] + 68'd1)) p2();\n\
         \x20 sub #(.P(~W[70:0])) p3();\n\
         \x20 initial #10 $finish;\nendmodule\n"
    );
    check(
        &src,
        &[
            "top.f2 bits=128 hex=01000000000000000000000000000000",
            "top.f3 bits=128 hex=00000000000000000000000000000008",
            "top.p1 bits=136 hex=cb000000000000000100000000000000f0",
            "top.p2 bits=68 hex=00000000000000011",
            "top.p3 bits=71 hex=7effffffffffffff0f",
        ],
    );
}

/// An ALL-signed tree folds: every extension is a sign extension, which is what
/// §11.8.2 asks for when the whole expression is signed. PRE: 32 bits each. Both oracles
/// agree on every value (iverilog 129 / 256 bits on `+` / `*`).
#[test]
fn an_all_signed_tree_folds() {
    let src = format!(
        "{SUB}module top;\n\
         \x20 parameter signed [127:0] SW = -128'sd100;\n\
         \x20 sub #(.P(-128'sd1 + 128'sd0)) s1();\n\
         \x20 sub #(.P(-128'sd5 * 128'sd3)) s2();\n\
         \x20 sub #(.P(SW >>> 4)) s3();\n\
         \x20 sub #(.P(-SW - 128'sd1)) s4();\n\
         \x20 initial #10 $finish;\nendmodule\n"
    );
    check(
        &src,
        &[
            "top.s1 bits=128 hex=ffffffffffffffffffffffffffffffff",
            "top.s2 bits=128 hex=fffffffffffffffffffffffffffffff1",
            "top.s3 bits=128 hex=fffffffffffffffffffffffffffffff9",
            "top.s4 bits=128 hex=00000000000000000000000000000063",
        ],
    );
}
