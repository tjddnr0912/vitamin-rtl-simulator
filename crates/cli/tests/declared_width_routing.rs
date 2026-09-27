//! A parameter declared 64 bits or narrower folds its initializer at its declared width, in
//! the region-aware wide walk (IEEE 1800-2017 §11.6.1, §11.8.2), whenever the initializer names
//! a constant or has an operand wider than 64 bits — ROADMAP §2 row 14 and the "Constant
//! domain" `/ % >>>` bullet (C2).
//!
//! Those initializers folded through the width-UNLIMITED i64 walk, which holds a narrow SIGNED
//! leaf already sign-extended and never wraps at the region's width: a signed 8-bit `-2` in an
//! unsigned 64-bit region stayed `ff…fe` (both oracles `0…0fe`), `(P + 8'd100) % 8'd7` over
//! `P = 200` kept the carry (6 for 2), and `N % 64'd10` over a signed 64-bit `-5` divided signed
//! (18446744073709551611 for 1). `Elaborator::param_init_at_declared_width` routes them through
//! `fold_bits_at` at the declared width in every binder — module header and body, generate
//! scope, package, interface — so the four lanes of one declaration still answer alike. Its
//! comparison arm now also refolds a SIGNED operand of an unsigned comparison region with the
//! region's sign (`(P >>> 60) > 64'd100` over a signed `-100` shifts in zeros: 0).
//!
//! Every value is what iverilog 13.0 and verilator 5.052 both print unless a cell says
//! otherwise; `PRE` is what vita printed before.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn lines(src: &str) -> Vec<String> {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_dwr_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let s =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "expected exit 0, got:\n{s}");
    s.lines()
        .filter(|l| l.starts_with("T "))
        .map(|l| l.trim_end().to_string())
        .collect()
}

fn check(src: &str, want: &[&str]) {
    assert_eq!(lines(src), want, "{src}");
}

/// A signed leaf in an unsigned region zero-extends at its own width; the region wraps at its
/// width. PRE: `ff…fe`, `fffe`, `fffd`, `6` (the i64 kept the carry of `200 + 100`).
#[test]
fn a_named_leaf_folds_in_its_region() {
    check(
        "module t;\n  localparam logic signed [7:0] NM = -8'sd2;\n  localparam logic [63:0] X = NM ^ 64'h0;\n  \
         localparam logic [15:0] Y = NM + 16'd0;\n  localparam logic signed [7:0] S8 = -8'sd3;\n  \
         localparam logic [15:0] A = S8 + 16'd0;\n  localparam logic [15:0] B = (S8 >>> 1) + 16'd0;\n  \
         localparam logic [63:0] C = (S8 >>> 1) + 64'd0;\n  localparam logic [7:0] P = 200;\n  \
         localparam logic [7:0] M = (P + 8'd100) % 8'd7;\n  localparam logic signed [15:0] J = S8 + 16'sd0;\n  \
         initial begin #1 $display(\"T %h %h %h %h %h %h %0d %h\", X, Y, A, B, C, M, M, J); $finish; end\nendmodule\n",
        &["T 00000000000000fe 00fe 00fd 007e 000000000000007e 02 2 fffd"],
    );
}

/// C2: `/`, `%` and `>>>` over a signed 64-bit leaf in an unsigned 64-bit region. PRE:
/// `18446744073709551611` and `0`; `>>>` was already right.
#[test]
fn a_sixty_four_bit_signed_leaf_divides_unsigned() {
    check(
        "module t;\n  localparam logic signed [63:0] N = -64'sd5;\n  localparam logic [63:0] A = N % 64'd10;\n  \
         localparam logic [63:0] B = N / 64'd10;\n  localparam logic [63:0] C = N >>> 4;\n  \
         localparam logic [63:0] E = N >> 4;\n  \
         initial begin #1 $display(\"T %0d %0d %0d %0d\", A, B, C, E); $finish; end\nendmodule\n",
        &["T 1 1844674407370955161 18446744073709551615 1152921504606846975"],
    );
}

/// A leaf wider than 64 bits keeps its width (a 65-bit `-100` divided and reduced as 65 bits),
/// and a literal wider than 64 bits sizes the region of an 8-bit target (the 1-bit `-1'sb1`
/// zero-extends in the unsigned 65-bit region). PRE: `ff…f2`, `ff…fe`, `fe`, `f1`.
#[test]
fn a_wide_operand_sizes_the_region_of_a_narrow_target() {
    check(
        "module t;\n  localparam logic signed [64:0] N65 = -65'sd100;\n  localparam logic [63:0] A = N65 / 64'd7;\n  \
         localparam logic [63:0] B = N65 % 64'd7;\n  localparam logic signed [63:0] C = N65 / 7;\n  \
         localparam logic [63:0] D = N65 >>> 1;\n  localparam logic [7:0] E = 8'hFF - (-1'sb1) + 65'd0;\n  \
         localparam logic [7:0] F = 8'hF0 | (-1'sb1) | 65'd0;\n  \
         initial begin #1 $display(\"T %h %h %h %h %h %h\", A, B, C, D, E, F); $finish; end\nendmodule\n",
        &["T 4924924924924916 0000000000000002 fffffffffffffff2 ffffffffffffffce 00 ff"],
    );
}

/// An untyped leaf is its literal's type (a decimal is a 32-bit signed integer, a sized
/// literal its own), and a chain of declarations carries each declared width. PRE: `fffe`,
/// `fffa`, `0000fffe`.
#[test]
fn untyped_leaves_and_chains_take_their_own_types() {
    check(
        "module t;\n  localparam UN = 200;\n  localparam logic [7:0] A = UN + 8'd100;\n  \
         localparam logic [7:0] B = (UN + 8'd100) % 8'd7;\n  localparam W = 8'hFE;\n  \
         localparam logic [15:0] D = W + 16'd0;\n  localparam SW = 8'shFE;\n  localparam logic [15:0] F = SW + 16'd0;\n  \
         localparam logic [15:0] G = SW + 16'sd0;\n  localparam logic signed [7:0] S8 = -8'sd3;\n  \
         localparam logic [15:0] H = S8 + 16'd0;\n  localparam logic [15:0] K = H * 2;\n  localparam L = H + 1;\n  \
         localparam logic [31:0] M = L;\n  \
         initial begin #1 $display(\"T %h %h %h %h %h %h %h %0d %h\", A, B, D, F, G, H, K, L, M); $finish; end\nendmodule\n",
        &["T 2c 06 00fe 00fe fffe 00fd 01fa 254 000000fe"],
    );
}

/// Every binder takes the route: a package read as `pk::X` and a module reading `pk::PA`, a
/// header parameter (default and overridden), an interface body. PRE: `ff…fe`, `fffe`, `fffd`;
/// `fffe fffe` / `fffa ffff`; `fffe`. A generate scope whose `time` declaration shadows a
/// module one was already right (the row's "`2c` for `12c`" cell had closed before).
#[test]
fn every_binder_takes_the_route() {
    check(
        "package pk;\n  localparam logic signed [7:0] PA = 8'hFE;\n  localparam int PI = -3;\n  \
         localparam logic [63:0] X = PA ^ 64'h0;\nendpackage\n\
         module t;\n  localparam logic [15:0] B = pk::PA + 16'd0;\n  localparam logic [15:0] C = pk::PI % 16'd5;\n  \
         localparam int D = 3, E = 17;\n  localparam int F = (D - E) / 2;\n  localparam logic [7:0] G = (D - E) / 2;\n  \
         initial begin #1 $display(\"T %h %h %h %0d %h\", pk::X, B, C, F, G); $finish; end\nendmodule\n",
        &["T 00000000000000fe 00fe 0003 -7 f9"],
    );
    check(
        "module sub #(parameter logic signed [7:0] P = -8'sd2, parameter logic [15:0] Q = P + 16'd0, \
         parameter logic [15:0] R = P % 16'd5) ();\n  initial #1 $display(\"T %m %h %h\", Q, R);\nendmodule\n\
         module t;\n  sub u0();\n  sub #(.P(-8'sd6)) u1();\n  initial #2 $finish;\nendmodule\n",
        &["T t.u0 00fe 0004", "T t.u1 00fa 0000"],
    );
    check(
        "interface ifc;\n  parameter logic signed [7:0] P = -8'sd2;\n  parameter logic [15:0] Q = P + 16'd0;\n  \
         initial #1 $display(\"T %h\", Q);\nendinterface\nmodule t; ifc i(); initial #2 $finish; endmodule\n",
        &["T 00fe"],
    );
    check(
        "module t;\n  localparam logic [7:0] NM = 8'h2c;\n  if (1) begin : g\n    localparam time NM = 12;\n    \
         localparam logic [63:0] X = NM | 64'h0;\n    initial #1 $display(\"T %h\", X);\n  end\n  initial #2 $finish;\nendmodule\n",
        &["T 000000000000000c"],
    );
}

/// The wide walk's comparison arm refolds a SIGNED operand of an unsigned comparison region
/// with the region's sign (§11.8.2), so an arithmetic shift inside it shifts in zeros. PRE's
/// ≤64-bit comparisons were right by the i64 walk (`E` was the row: `ff…ff`); the >64-bit lane
/// was wrong (`C`, `G`, `J` of the 128-bit cell), and routing the ≤64-bit lane through the arm
/// made those wrong too until the refold.
#[test]
fn a_signed_side_of_an_unsigned_comparison_takes_the_region_sign() {
    check(
        "module t;\n  parameter longint signed P = -100;\n  localparam integer C = ((P >>> 60) > 64'd100);\n  \
         localparam integer C2 = ((P >>> 60) > 100);\n  localparam logic [63:0] E = (P >>> 60) + 64'd0;\n  \
         localparam logic [7:0] G = ((P >>> 60) == 64'hF);\n  \
         initial begin #1 $display(\"T %0d %0d %h %h\", C, C2, E, G); $finish; end\nendmodule\n",
        &["T 0 0 000000000000000f 01"],
    );
    check(
        "module t;\n  localparam logic signed [127:0] P = -100;\n  localparam integer C = ((P >>> 120) > 128'd100);\n  \
         localparam integer G = ((P >>> 120) == 128'hFF);\n  localparam integer H = ((P >>> 120) < 128'd300);\n  \
         localparam logic signed [127:0] Q = -128'sd100;\n  localparam integer J = ((Q >>> 120) != 128'hFF);\n  \
         initial begin #1 $display(\"T %0d %0d %0d %0d\", C, G, H, J); $finish; end\nendmodule\n",
        &["T 1 1 1 0"], // PRE `0 0 1 1`
    );
}

/// Unchanged: a shift's result takes its LEFT operand's sign and a signed literal keeps its
/// own, an unsigned declaration was already right, and an initializer with no name and no
/// operand past 64 bits keeps the width-aware assignment walk it had.
#[test]
fn the_cells_the_route_must_not_move() {
    check(
        "module t;\n  localparam logic signed [7:0] NM = -8'sd2;\n  localparam logic [63:0] A = NM << 0;\n  \
         localparam logic [63:0] B = NM >>> 0;\n  localparam logic [63:0] C = (-8'sd2) ^ 64'h0;\n  \
         localparam logic [7:0] U = 8'hFE;\n  localparam logic [63:0] D = U ^ 64'h0;\n  \
         localparam [63:0] P = 64'hFFFFFFFFFFFFFFFF % 64'd10;\n  localparam logic [7:0] K = (8'd200 + 8'd100) >> 1;\n  \
         initial begin #1 $display(\"T %h %h %h %h %0d %0d\", A, B, C, D, P, K); $finish; end\nendmodule\n",
        &["T fffffffffffffffe fffffffffffffffe fffffffffffffffe 00000000000000fe 5 22"],
    );
}
