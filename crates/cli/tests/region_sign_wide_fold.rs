//! §11.8.1 / §11.8.2 in the wide constant fold (ROADMAP §2 🆕 R and 🆕 F; the WALL that
//! blocked rows 14, 15, 16, 25, 26, 30 and the override lane's plain-tree rule).
//!
//! `const_wide::fold_bits_at` used to decide a node's sign NODE-LOCALLY and extend each
//! operand with the sign of the node it fed, and it folded a self-determined position
//! with no context. §11.8.2 evaluates a REGION: its width and sign are decided first over
//! the whole tree (unsigned if any context-determined operand is unsigned), then pushed
//! down into every context-determined operand, and an operand that must be extended is
//! sign-extended only if that propagated type is signed (§11.8.3 step 4). The entry now
//! makes two passes — the first learns the region's width and sign, the second refolds
//! at that width with that sign — and a comparison's operands form a region of their
//! own, sized to the larger side and signed only if both are.
//!
//! Every `localparam` here is 65 bits or wider (or contains a `$signed` the i64 walk
//! declines), so it folds in the wide domain; the ≤64-bit lane folds through the
//! width-unlimited i64 walk and is ROADMAP §2 rows 14 / 30 (its cells are pinned
//! known-wrong at the bottom). Oracles: iverilog 13.0 `-g2012` and verilator 5.052
//! `--binary --timing` print every value pinned below; the runtime twin (`assign r =
//! <same text>`) prints the same in all three tools.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_rswf_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "expected exit 0, got:\n{s}");
    s
}

/// The sign push-down (§2 🆕 R, second half; the root of 🆕 F). PRE extended the signed
/// 8-bit operand with its OWN sign: `a01` was `0…02`, `a03` `ff…fe`, `a04` `ff…f80`,
/// `d05` `ff…fd`, `d16` `ff…fd0`, `d31` `0…09`, `d33` `0…03`, `h08` `0…03`.
#[test]
fn a_signed_operand_of_an_unsigned_region_is_zero_extended_before_its_operator() {
    let o = run("module top;\n  localparam logic signed [7:0] S8 = -8'sd3;\n  localparam logic signed [7:0] S8n = -8'sd3;\n  localparam logic signed [63:0] S64 = -1;\n  localparam logic [7:0] U8 = 8'hfd;\n  localparam logic [127:0] L_a01 = ~S8 + 128'd0;\n  localparam logic [127:0] L_a02 = -(8'sh80) + 128'd0;\n  localparam logic [127:0] L_a03 = (S8 >>> 1) + 128'd0;\n  localparam logic [127:0] L_a04 = (8'sh80 * 8'sd1) + 128'd0;\n  localparam logic [127:0] L_a05 = (-8'sd1 >>> 1) ^ 128'h1_0000_0000_0000_0000;\n  localparam logic [127:0] L_a06 = (S8n + 8'sd1) * 128'd1;\n  localparam logic [255:0] L_a09 = 128'd0 + {(S8n + 128'sd0) | 128'd0};\n  localparam logic [127:0] L_d05 = (S8 + 128'sd0) + 128'd0;\n  localparam logic [127:0] L_d16 = (S8 << 4) + 128'd0;\n  localparam logic [127:0] L_d17 = (S8 & 8'shff) + 128'd0;\n  localparam logic [127:0] L_d21 = (S8 ^ 8'sh0f) | 128'd0;\n  localparam logic [127:0] L_d22 = (S8 - 8'sd1) - 128'd0;\n  localparam logic [127:0] L_d23 = ((S8 + 8'sd0) + 8'sd0) + 128'd0;\n  localparam logic [127:0] L_d28 = (1'b0 ? 128'sd0 : S8) + 128'd0;\n  localparam logic [127:0] L_d31 = (S8 * S8) + 128'd0;\n  localparam logic [127:0] L_d33 = (-S8) + 128'd0;\n  localparam logic [127:0] L_h06 = (S8 ** 2) + 128'd0;\n  localparam logic [127:0] L_h08 = (S8 > 8'sd0 ? S8 : -S8) + 128'd0;\n  localparam logic [127:0] L_h11 = ((S8 + 8'sd0) >>> 1) + 128'd0;\n  localparam logic [127:0] L_h19 = S8 << 1 | 128'd0;\n  localparam logic [127:0] L_h20 = S8 >> 1 | 128'd0;\n  localparam logic [127:0] L_h22 = (-(S8 >>> 1)) + 128'd0;\n  localparam logic [127:0] L_h23 = $signed((S8 >>> 1) + 128'd0);\n  localparam logic [255:0] L_h25 = {(S8 >>> 1) + 128'd0, (S8 >>> 1) + 128'sd0};\n  localparam logic [127:0] L_h31 = (S64 * S64) + 128'd0;\n  localparam logic [127:0] L_h33 = (S8 >>> 1) + 128'sd0 + 128'd0;\n  localparam logic [127:0] L_h34 = (S8 + 128'sd0) + (S8 + 128'd0);\n  localparam logic signed [127:0] L_h35 = (S8 + 128'sd0) + (S8 + 128'd0);\n  localparam logic [127:0] L_h36 = (S8 >>> 1) + 128'd0 + 128'sd0;\n  initial begin\n    $display(\"a01 %h\", L_a01);\n    $display(\"a02 %h\", L_a02);\n    $display(\"a03 %h\", L_a03);\n    $display(\"a04 %h\", L_a04);\n    $display(\"a05 %h\", L_a05);\n    $display(\"a06 %h\", L_a06);\n    $display(\"a09 %h\", L_a09);\n    $display(\"d05 %h\", L_d05);\n    $display(\"d16 %h\", L_d16);\n    $display(\"d17 %h\", L_d17);\n    $display(\"d21 %h\", L_d21);\n    $display(\"d22 %h\", L_d22);\n    $display(\"d23 %h\", L_d23);\n    $display(\"d28 %h\", L_d28);\n    $display(\"d31 %h\", L_d31);\n    $display(\"d33 %h\", L_d33);\n    $display(\"h06 %h\", L_h06);\n    $display(\"h08 %h\", L_h08);\n    $display(\"h11 %h\", L_h11);\n    $display(\"h19 %h\", L_h19);\n    $display(\"h20 %h\", L_h20);\n    $display(\"h22 %h\", L_h22);\n    $display(\"h23 %h\", L_h23);\n    $display(\"h25 %h\", L_h25);\n    $display(\"h31 %h\", L_h31);\n    $display(\"h33 %h\", L_h33);\n    $display(\"h34 %h\", L_h34);\n    $display(\"h35 %h\", L_h35);\n    $display(\"h36 %h\", L_h36);\n    $finish;\n  end\nendmodule\n");
    for want in [
        "a01 ffffffffffffffffffffffffffffff02",
        "a02 ffffffffffffffffffffffffffffff80",
        "a03 0000000000000000000000000000007e",
        "a04 00000000000000000000000000000080",
        "a05 7ffffffffffffffeffffffffffffffff",
        "a06 000000000000000000000000000000fe",
        "a09 00000000000000000000000000000000000000000000000000000000000000fd",
        "d05 000000000000000000000000000000fd",
        "d16 00000000000000000000000000000fd0",
        "d17 000000000000000000000000000000fd",
        "d21 000000000000000000000000000000f2",
        "d22 000000000000000000000000000000fc",
        "d23 000000000000000000000000000000fd",
        "d28 000000000000000000000000000000fd",
        "d31 0000000000000000000000000000fa09",
        "d33 ffffffffffffffffffffffffffffff03",
        "h06 0000000000000000000000000000fa09",
        "h08 ffffffffffffffffffffffffffffff03",
        "h11 0000000000000000000000000000007e",
        "h19 000000000000000000000000000001fa",
        "h20 0000000000000000000000000000007e",
        "h22 ffffffffffffffffffffffffffffff82",
        "h23 0000000000000000000000000000007e",
        "h25 0000000000000000000000000000007efffffffffffffffffffffffffffffffe",
        "h31 fffffffffffffffe0000000000000001",
        "h33 0000000000000000000000000000007e",
        "h34 000000000000000000000000000001fa",
        "h35 000000000000000000000000000001fa",
        "h36 0000000000000000000000000000007e",
    ] {
        assert!(o.contains(want), "missing `{want}` in:\n{o}");
    }
}

/// §2 🆕 R, first half: a signing cast's operand, a concatenation part, a replication
/// part and `$unsigned`'s argument fold their inner at the INNER region's width, so
/// `~8'd1` inside `~8'd1 + 128'd0` complements at 128. PRE: `b01` `0…0fe`, `b02`
/// `0…0fe`, `b05` `0…0fe000000000000fe`, `b07` `0…0fe`. `b03`/`b04`/`b06`/`b08`–`b11`
/// were already right (a size cast and `$bits` carry their context; a lone `-8'sd1`
/// extends by its own sign in a signed region).
#[test]
fn a_self_determined_position_folds_its_inner_as_a_region_of_its_own() {
    let o = run("module top;\n  localparam logic signed [7:0] S8 = -8'sd3;\n  localparam logic signed [7:0] S8n = -8'sd3;\n  localparam logic signed [63:0] S64 = -1;\n  localparam logic [7:0] U8 = 8'hfd;\n  localparam logic [127:0] L_b01 = $signed(~8'd1 + 128'd0);\n  localparam logic [127:0] L_b02 = {~8'd1 + 120'd0} + 128'd0;\n  localparam logic [255:0] L_b03 = 128'(~8'd1 + 64'd0);\n  localparam logic [3:0] L_b04 = (~8'd1 + 64'd0) == 64'hffff_ffff_ffff_fffe;\n  localparam logic [127:0] L_b05 = {2{~8'd1 + 56'd0}} + 128'd0;\n  localparam logic [3:0] L_b06 = $bits(~8'd1 + 120'd0);\n  localparam logic [127:0] L_b07 = $unsigned(~8'd1 + 128'd0);\n  localparam logic [127:0] L_b08 = (~8'd1 + 128'd0) ^ 128'd0;\n  localparam logic [127:0] L_b09 = -(8'd1 + 128'd0);\n  localparam logic [127:0] L_b10 = {(-8'sd1) + 120'd0} + 128'd0;\n  localparam logic [127:0] L_b11 = {(-8'sd1) + 120'sd0} + 128'd0;\n  initial begin\n    $display(\"b01 %h\", L_b01);\n    $display(\"b02 %h\", L_b02);\n    $display(\"b03 %h\", L_b03);\n    $display(\"b04 %h\", L_b04);\n    $display(\"b05 %h\", L_b05);\n    $display(\"b06 %h\", L_b06);\n    $display(\"b07 %h\", L_b07);\n    $display(\"b08 %h\", L_b08);\n    $display(\"b09 %h\", L_b09);\n    $display(\"b10 %h\", L_b10);\n    $display(\"b11 %h\", L_b11);\n    $finish;\n  end\nendmodule\n");
    for want in [
        "b01 fffffffffffffffffffffffffffffffe",
        "b02 00fffffffffffffffffffffffffffffe",
        "b03 00000000000000000000000000000000fffffffffffffffffffffffffffffffe",
        "b04 1",
        "b05 0000fffffffffffffefffffffffffffe",
        "b06 8",
        "b07 fffffffffffffffffffffffffffffffe",
        "b08 fffffffffffffffffffffffffffffffe",
        "b09 ffffffffffffffffffffffffffffffff",
        "b10 00ffffffffffffffffffffffffffffff",
        "b11 00ffffffffffffffffffffffffffffff",
    ] {
        assert!(o.contains(want), "missing `{want}` in:\n{o}");
    }
}

/// `>>>` vacates with the sign bit only if the REGION is signed, `/` and `%` divide
/// magnitudes only then, and `**`'s base extends with it. PRE: `d10` `ff…fc`, `d11`
/// `ff…fe`, `d41` `1ff…fe`, `d43` `1ff…fd`. The signed twins (`d08`, `d24`–`d27`) were
/// already right and stay so. (`(-8'sd2) ** 2 + 128'd0` is an oracle split: iverilog
/// 4, verilator `fc04`; vita 4 — not pinned.)
#[test]
fn the_arithmetic_shift_division_modulus_and_power_run_with_the_region_sign() {
    let o = run("module top;\n  localparam logic signed [7:0] S8 = -8'sd3;\n  localparam logic signed [7:0] S8n = -8'sd3;\n  localparam logic signed [63:0] S64 = -1;\n  localparam logic [7:0] U8 = 8'hfd;\n  localparam logic [127:0] L_d08 = (S8 >>> 1) + 128'sd0;\n  localparam logic [127:0] L_d10 = (-8'sd8) / 8'sd2 + 128'd0;\n  localparam logic [127:0] L_d11 = (-8'sd8) % 8'sd3 + 128'd0;\n  localparam logic [127:0] L_d24 = (S8 >>> 1) | 128'sd0;\n  localparam logic [127:0] L_d25 = (-8'sd8) / 8'sd2 + 128'sd0;\n  localparam logic [127:0] L_d26 = (-8'sd8) % 8'sd3 + 128'sd0;\n  localparam logic [127:0] L_d27 = (-8'sd2) ** 2 + 128'sd0;\n  localparam logic [64:0] L_d41 = (S8 >>> 1) + 65'd0;\n  localparam logic [64:0] L_d43 = (S8 * 8'sd1) + 65'd0;\n  initial begin\n    $display(\"d08 %h\", L_d08);\n    $display(\"d10 %h\", L_d10);\n    $display(\"d11 %h\", L_d11);\n    $display(\"d24 %h\", L_d24);\n    $display(\"d25 %h\", L_d25);\n    $display(\"d26 %h\", L_d26);\n    $display(\"d27 %h\", L_d27);\n    $display(\"d41 %h\", L_d41);\n    $display(\"d43 %h\", L_d43);\n    $finish;\n  end\nendmodule\n");
    for want in [
        "d08 fffffffffffffffffffffffffffffffe",
        "d10 7ffffffffffffffffffffffffffffffc",
        "d11 00000000000000000000000000000002",
        "d24 fffffffffffffffffffffffffffffffe",
        "d25 fffffffffffffffffffffffffffffffc",
        "d26 fffffffffffffffffffffffffffffffe",
        "d27 00000000000000000000000000000004",
        "d41 0000000000000007e",
        "d43 000000000000000fd",
    ] {
        assert!(o.contains(want), "missing `{want}` in:\n{o}");
    }
}

/// Controls, unchanged: an all-signed region sign-extends (`d01`–`d03`, `d07`, `d19`),
/// an unsigned leaf beside a signed literal makes the region unsigned (`d04`, `d06`,
/// `d18`, `d20`), `$signed` / `$unsigned` / a size cast start a region of their own
/// (`d13`–`d15`, `d37`), a fill takes the region (`d12`), and the sign of the TARGET
/// does not reach the right-hand side (`d02`, `d29`).
#[test]
fn a_signed_region_still_sign_extends_and_a_cast_or_signing_call_keeps_its_own_region() {
    let o = run("module top;\n  localparam logic signed [7:0] S8 = -8'sd3;\n  localparam logic signed [7:0] S8n = -8'sd3;\n  localparam logic signed [63:0] S64 = -1;\n  localparam logic [7:0] U8 = 8'hfd;\n  localparam logic signed [127:0] L_d01 = S8 + 128'sd0;\n  localparam logic [127:0] L_d02 = S8 + 128'sd0;\n  localparam logic [127:0] L_d03 = $signed(S8) + $signed(128'd0);\n  localparam logic [127:0] L_d04 = S8 + 128'd0;\n  localparam logic [127:0] L_d06 = 1'b1 ? S8 : 128'd0;\n  localparam logic [127:0] L_d07 = 1'b1 ? S8 : 128'sd0;\n  localparam logic [127:0] L_d12 = '1 + S8;\n  localparam logic [127:0] L_d13 = $signed(S8 + 8'sd0) + 128'd0;\n  localparam logic [127:0] L_d14 = $unsigned(S8) + 128'sd0;\n  localparam logic [127:0] L_d15 = 16'(S8) + 128'd0;\n  localparam logic [127:0] L_d18 = S64 + 128'd0;\n  localparam logic [127:0] L_d19 = S64 + 128'sd0;\n  localparam logic [127:0] L_d20 = $signed(4'hF) + 128'd1;\n  localparam logic signed [127:0] L_d29 = -(8'd1) + 128'sd0;\n  localparam logic [127:0] L_d30 = ~S8 & 128'sd0 | 128'd0;\n  localparam logic [127:0] L_d32 = (U8 + 8'sd0) + 128'sd0;\n  localparam logic [127:0] L_d34 = (-S8) + 128'sd0;\n  localparam logic [127:0] L_d35 = S8 >>> 1;\n  localparam logic [127:0] L_d36 = S8 + 128'd0 + S8;\n  localparam logic [127:0] L_d37 = 8'(S8) + 128'd0;\n  localparam logic [64:0] L_d42 = S8 + 65'd0;\n  initial begin\n    $display(\"d01 %h\", L_d01);\n    $display(\"d02 %h\", L_d02);\n    $display(\"d03 %h\", L_d03);\n    $display(\"d04 %h\", L_d04);\n    $display(\"d06 %h\", L_d06);\n    $display(\"d07 %h\", L_d07);\n    $display(\"d12 %h\", L_d12);\n    $display(\"d13 %h\", L_d13);\n    $display(\"d14 %h\", L_d14);\n    $display(\"d15 %h\", L_d15);\n    $display(\"d18 %h\", L_d18);\n    $display(\"d19 %h\", L_d19);\n    $display(\"d20 %h\", L_d20);\n    $display(\"d29 %h\", L_d29);\n    $display(\"d30 %h\", L_d30);\n    $display(\"d32 %h\", L_d32);\n    $display(\"d34 %h\", L_d34);\n    $display(\"d35 %h\", L_d35);\n    $display(\"d36 %h\", L_d36);\n    $display(\"d37 %h\", L_d37);\n    $display(\"d42 %h\", L_d42);\n    $finish;\n  end\nendmodule\n");
    for want in [
        "d01 fffffffffffffffffffffffffffffffd",
        "d02 fffffffffffffffffffffffffffffffd",
        "d03 fffffffffffffffffffffffffffffffd",
        "d04 000000000000000000000000000000fd",
        "d06 000000000000000000000000000000fd",
        "d07 fffffffffffffffffffffffffffffffd",
        "d12 000000000000000000000000000000fc",
        "d13 000000000000000000000000000000fd",
        "d14 000000000000000000000000000000fd",
        "d15 0000000000000000000000000000fffd",
        "d18 0000000000000000ffffffffffffffff",
        "d19 ffffffffffffffffffffffffffffffff",
        "d20 00000000000000000000000000000010",
        "d29 ffffffffffffffffffffffffffffffff",
        "d30 00000000000000000000000000000000",
        "d32 000000000000000000000000000000fd",
        "d34 00000000000000000000000000000003",
        "d35 fffffffffffffffffffffffffffffffe",
        "d36 000000000000000000000000000001fa",
        "d37 000000000000000000000000000000fd",
        "d42 000000000000000fd",
    ] {
        assert!(o.contains(want), "missing `{want}` in:\n{o}");
    }
}

/// §2 🆕 F: `8'hFF - (-1'sb1)` and `8'hF0 | (-1'sb1)`. The headline cells at 8 bits
/// (and 16 / 32 / 64) were already the oracles' `00` / `ff` at HEAD (the i64 lane had
/// closed them); the 65-bit twins (`c10`, `f65*`) and the `$signed` shape `c03`
/// (`($signed(4'hF)+1) | 8'h00`, `10`) were the wide lane's and are closed here. PRE:
/// `c03` `00`, `c04` `0…0fe`, `c05` `0…0f1`, `c06` `0…00`, `c10` `0…0fe`, `f65sm`
/// `0…0fe`, `f65so` `0…0f1`.
#[test]
fn row_f_cells_the_wide_lane_twins_and_the_headline_shapes() {
    let o = run("module top;\n  localparam logic signed [7:0] S8 = -8'sd3;\n  localparam logic signed [7:0] S8n = -8'sd3;\n  localparam logic signed [63:0] S64 = -1;\n  localparam logic [7:0] U8 = 8'hfd;\n  localparam logic [7:0] L_c01 = 8'hFF - (-1'sb1);\n  localparam logic [7:0] L_c02 = 8'hF0 | (-1'sb1);\n  localparam logic [7:0] L_c03 = ($signed(4'hF)+1) | 8'h00;\n  localparam logic [127:0] L_c04 = 128'hFF - (-1'sb1);\n  localparam logic [127:0] L_c05 = 128'hF0 | (-1'sb1);\n  localparam logic [127:0] L_c06 = ($signed(4'hF)+1) | 128'h00;\n  localparam logic [15:0] L_c07 = 16'hFF - (-1'sb1);\n  localparam logic [31:0] L_c08 = 32'hFF - (-1'sb1);\n  localparam logic [63:0] L_c09 = 64'hFF - (-1'sb1);\n  localparam logic [64:0] L_c10 = 65'hFF - (-1'sb1);\n  localparam logic [7:0] L_f08um = 8'hFF - (-1'sb1);\n  localparam logic [7:0] L_f08uo = 8'hF0 | (-1'sb1);\n  localparam logic signed [7:0] L_f08sm = 8'hFF - (-1'sb1);\n  localparam logic signed [7:0] L_f08so = 8'hF0 | (-1'sb1);\n  localparam logic [63:0] L_f64um = 64'hFF - (-1'sb1);\n  localparam logic [63:0] L_f64uo = 64'hF0 | (-1'sb1);\n  localparam logic [64:0] L_f65um = 65'hFF - (-1'sb1);\n  localparam logic [64:0] L_f65uo = 65'hF0 | (-1'sb1);\n  localparam logic signed [64:0] L_f65sm = 65'hFF - (-1'sb1);\n  localparam logic signed [64:0] L_f65so = 65'hF0 | (-1'sb1);\n  initial begin\n    $display(\"c01 %h\", L_c01);\n    $display(\"c02 %h\", L_c02);\n    $display(\"c03 %h\", L_c03);\n    $display(\"c04 %h\", L_c04);\n    $display(\"c05 %h\", L_c05);\n    $display(\"c06 %h\", L_c06);\n    $display(\"c07 %h\", L_c07);\n    $display(\"c08 %h\", L_c08);\n    $display(\"c09 %h\", L_c09);\n    $display(\"c10 %h\", L_c10);\n    $display(\"f08um %h\", L_f08um);\n    $display(\"f08uo %h\", L_f08uo);\n    $display(\"f08sm %h\", L_f08sm);\n    $display(\"f08so %h\", L_f08so);\n    $display(\"f64um %h\", L_f64um);\n    $display(\"f64uo %h\", L_f64uo);\n    $display(\"f65um %h\", L_f65um);\n    $display(\"f65uo %h\", L_f65uo);\n    $display(\"f65sm %h\", L_f65sm);\n    $display(\"f65so %h\", L_f65so);\n    $finish;\n  end\nendmodule\n");
    for want in [
        "c01 00",
        "c02 ff",
        "c03 10",
        "c04 00000000000000000000000000000100",
        "c05 ffffffffffffffffffffffffffffffff",
        "c06 00000000000000000000000000000010",
        "c07 0100",
        "c08 00000100",
        "c09 0000000000000100",
        "c10 00000000000000100",
        "f08um 00",
        "f08uo ff",
        "f08sm 00",
        "f08so ff",
        "f64um 0000000000000100",
        "f64uo ffffffffffffffff",
        "f65um 00000000000000100",
        "f65uo 1ffffffffffffffff",
        "f65sm 00000000000000100",
        "f65so 1ffffffffffffffff",
    ] {
        assert!(o.contains(want), "missing `{want}` in:\n{o}");
    }
}

/// §11.8.3: the operands of a relational or equality operator are sized to the larger
/// side and signed only if both are, and that region is pushed into each operand — so
/// `~8'd1 == 16'hFFFE` complements at 16 and is true (`k01`), `S8 > 16'd255` compares
/// `00fd` against `00ff` (`k02`, false), `'1` takes the other side's width (`k06`,
/// `k07`) and two fills are one bit each (`k08`).
#[test]
fn a_comparison_operands_form_a_region_sized_to_the_larger_side() {
    let o = run("module top;\n  localparam logic signed [7:0] S8 = -8'sd3;\n  localparam logic signed [7:0] S8n = -8'sd3;\n  localparam logic signed [63:0] S64 = -1;\n  localparam logic [7:0] U8 = 8'hfd;\n  localparam logic [127:0] L_k01 = (~8'd1 == 16'hFFFE) + 128'd0;\n  localparam logic [127:0] L_k02 = (S8 > 16'd255) + 128'd0;\n  localparam logic [127:0] L_k03 = (S8 > 16'sd255) + 128'd0;\n  localparam logic [127:0] L_k04 = (S8 < 16'd0) + 128'd0;\n  localparam logic [127:0] L_k05 = ((S8 + 8'd0) < 16'sd0) + 128'd0;\n  localparam logic [127:0] L_k06 = ('1 == 8'hff) + 128'd0;\n  localparam logic [127:0] L_k07 = ('1 != 40'hff_ffff_ffff) + 128'd0;\n  localparam logic [127:0] L_k08 = ('1 == '1) + 128'd0;\n  localparam logic [127:0] L_k09 = (('1 + 8'd0) == 16'hffff) + 128'd0;\n  localparam logic [127:0] L_k10 = ((S8 >>> 1) == 16'h7e) + 128'd0;\n  localparam logic [127:0] L_k11 = ((S8 >>> 1) == 16'shfffe) + 128'd0;\n  localparam logic [127:0] L_k12 = (S8 == 8'hfd) + 128'd0;\n  localparam logic [127:0] L_k13 = (-8'sd1 < 8'd1) + 128'd0;\n  localparam logic [127:0] L_k14 = (-8'sd1 < 8'sd1) + 128'd0;\n  localparam logic [127:0] L_k15 = (128'd0 < -8'sd1) + 128'd0;\n  initial begin\n    $display(\"k01 %h\", L_k01);\n    $display(\"k02 %h\", L_k02);\n    $display(\"k03 %h\", L_k03);\n    $display(\"k04 %h\", L_k04);\n    $display(\"k05 %h\", L_k05);\n    $display(\"k06 %h\", L_k06);\n    $display(\"k07 %h\", L_k07);\n    $display(\"k08 %h\", L_k08);\n    $display(\"k09 %h\", L_k09);\n    $display(\"k10 %h\", L_k10);\n    $display(\"k11 %h\", L_k11);\n    $display(\"k12 %h\", L_k12);\n    $display(\"k13 %h\", L_k13);\n    $display(\"k14 %h\", L_k14);\n    $display(\"k15 %h\", L_k15);\n    $finish;\n  end\nendmodule\n");
    for want in [
        "k01 00000000000000000000000000000001",
        "k02 00000000000000000000000000000000",
        "k03 00000000000000000000000000000000",
        "k04 00000000000000000000000000000000",
        "k05 00000000000000000000000000000000",
        "k06 00000000000000000000000000000001",
        "k07 00000000000000000000000000000000",
        "k08 00000000000000000000000000000001",
        "k09 00000000000000000000000000000001",
        "k10 00000000000000000000000000000001",
        "k11 00000000000000000000000000000001",
        "k12 00000000000000000000000000000001",
        "k13 00000000000000000000000000000000",
        "k14 00000000000000000000000000000001",
        "k15 00000000000000000000000000000001",
    ] {
        assert!(o.contains(want), "missing `{want}` in:\n{o}");
    }
}

/// ⚠️ KNOWN-WRONG, pinned so the routing slice (ROADMAP §2 rows 14 / 30) sees what it
/// moves: a declaration of 64 bits or less folds through the width-unlimited i64 walk,
/// which sign-extends the signed leaf whatever the region's sign (`d38`–`d40`), and a
/// `65'd0` beside an 8-bit target does not change that lane (`c11`, `c12`). The same
/// lane decides a generate condition: `if ((~S8 + 128'd0) > 128'd255)` picks the else
/// branch where both oracles pick the then branch. `a07` / `a08` (`int'(…)`) are the
/// prim-cast lane (§2 "Size cast / signedness"). Both oracles' values are beside each pin.
#[test]
fn the_i64_lane_is_not_this_slice_known_wrong_pins() {
    let o = run("module top;\n  localparam logic signed [7:0] S8 = -8'sd3;\n  localparam logic signed [7:0] S8n = -8'sd3;\n  localparam logic signed [63:0] S64 = -1;\n  localparam logic [7:0] U8 = 8'hfd;\n  localparam logic [7:0] L_c11 = 8'hFF - (-1'sb1) + 65'd0;\n  localparam logic [7:0] L_c12 = 8'hF0 | (-1'sb1) | 65'd0;\n  localparam logic [15:0] L_d38 = S8 + 16'd0;\n  localparam logic [15:0] L_d39 = (S8 >>> 1) + 16'd0;\n  localparam logic [63:0] L_d40 = (S8 >>> 1) + 64'd0;\n  localparam logic [127:0] L_a07 = int'(~8'd1) + 128'd0;\n  localparam logic [127:0] L_a08 = int'(S8n) + 128'd0;\n  initial begin\n    $display(\"c11 %h\", L_c11);\n    $display(\"c12 %h\", L_c12);\n    $display(\"d38 %h\", L_d38);\n    $display(\"d39 %h\", L_d39);\n    $display(\"d40 %h\", L_d40);\n    $display(\"a07 %h\", L_a07);\n    $display(\"a08 %h\", L_a08);\n    $finish;\n  end\nendmodule\n");
    for want in [
        "c11 fe",                               // both oracles 00
        "c12 f1",                               // both oracles ff
        "d38 fffd",                             // both oracles 00fd
        "d39 fffe",                             // both oracles 007e
        "d40 fffffffffffffffe",                 // both oracles 000000000000007e
        "a07 0000000000000000fffffffffffffffe", // both oracles 000000000000000000000000fffffffe
        "a08 0000000000000000fffffffffffffffd", // both oracles 000000000000000000000000fffffffd
    ] {
        assert!(o.contains(want), "missing `{want}` in:\n{o}");
    }
}
