//! Two override shapes the channels could not type, so an untyped target took the DEFAULT
//! literal's type (IEEE 1800-2017 §6.20.2 gives it the override's).
//!
//! - An operator over an ELEMENT of an array parameter (ROADMAP §2 row 25): the operator
//!   channel certifies a leaf's declared width through `declared_override_widths`, which needs
//!   a scalar parameter, so `#(.P(~A[0][3:0]))` onto `parameter P = 0` bound 32 signed bits
//!   `fffffffa` where verilator binds 4 bits `a` (iverilog has no unpacked-array parameters).
//!   Such a tree now folds in the wide walk, which reads an element at its declared width.
//! - A FILL inside an operator tree wider than 64 bits (§2 "Index sealing"): a fill has no self
//!   width, so the wide channel declined it and `#(.P(~128'd0 | '1))` bound `ffffffff` at 32
//!   bits where both oracles bind 128 ones. The tree's other operands now give the width.
//!
//! Row 25's values are verilator 5.052's; the fill cells are iverilog 13.0's and verilator's
//! unless a cell says otherwise. `PRE` is what vita printed before.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn lines(src: &str) -> Vec<String> {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_oeft_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let s =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "expected exit 0, got:\n{s}");
    let mut v: Vec<String> = s
        .lines()
        .filter(|l| l.starts_with("T "))
        .map(|l| l.trim_end().to_string())
        .collect();
    v.sort();
    v
}

fn check(src: &str, want: &[&str]) {
    let mut w: Vec<String> = want.iter().map(|s| s.to_string()).collect();
    w.sort();
    assert_eq!(lines(src), w, "{src}");
}

const SUB: &str =
    "module sub #(parameter P = 0) (); initial #1 $display(\"T %m %h %0d\", P, $bits(P)); endmodule\n";

/// Row 25: the element's declared width and sign type the tree (§11.6.1). PRE: every cell was
/// 32 bits (`fffffffa`, `0000000d`, `ffffffca`, `00000071`, `fffffffb`, `00000018`,
/// `fffffffd`, `00000002`, `0000003b`).
#[test]
fn an_operator_over_an_array_element_takes_the_elements_type() {
    check(
        &format!(
            "{SUB}module t;\n  localparam logic [7:0] A [0:1] = '{{8'h35, 8'h3C}};\n  \
             localparam logic signed [7:0] SA [0:1] = '{{-8'sd3, 8'sd60}};\n  \
             sub #(.P(~A[0][3:0])) u00();\n  sub #(.P(A[1][3:0] + 4'd1)) u01();\n  sub #(.P(A[0])) u02();\n  \
             sub #(.P(~A[0])) u03();\n  sub #(.P(A[0] + A[1])) u04();\n  sub #(.P(-A[0][3:0])) u05();\n  \
             sub #(.P(A[1][3:0] * 4'd2)) u06();\n  sub #(.P(SA[0] + 8'sd0)) u08();\n  sub #(.P(~SA[0])) u09();\n  \
             sub #(.P(A[1] - 8'd1)) u11();\n  initial #2 $finish;\nendmodule\n"
        ),
        &[
            "T t.u00 a 4",
            "T t.u01 d 4",
            "T t.u02 35 8",
            "T t.u03 ca 8",
            "T t.u04 71 8",
            "T t.u05 b 4",
            "T t.u06 8 4",
            "T t.u08 fd 8",
            "T t.u09 02 8",
            "T t.u11 3b 8",
        ],
    );
}

/// Other positions of an element: a ternary, a shift, a concatenation, a package array, the
/// positional and `defparam` channels, a product. A comparison is one bit (verilator binds
/// `A[0] > A[1]` at 32 bits while it binds `A[0] == 8'h35` at 1, so it is not an oracle for
/// the first; vita's 1 bit is §11.4.4's and was PRE's).
#[test]
fn an_element_types_the_tree_in_every_position() {
    check(
        &format!(
            "{SUB}module bsub; parameter P = 0; initial #1 $display(\"T %m %h %0d\", P, $bits(P)); endmodule\n\
             package pk; localparam logic [7:0] PA [0:1] = '{{8'h35, 8'h3C}}; endpackage\n\
             module t;\n  localparam logic [7:0] A [0:1] = '{{8'h35, 8'h3C}};\n  \
             localparam logic signed [7:0] SA [0:1] = '{{-8'sd3, 8'sd60}};\n  \
             localparam logic [3:0] N4 [0:2] = '{{4'h1, 4'h9, 4'hF}};\n  \
             sub #(.P(A[0] > A[1])) u01();\n  sub #(.P(A[1] ? A[0] : 8'd0)) u02();\n  sub #(.P(N4[2] + N4[1])) u03();\n  \
             sub #(.P(SA[0] >>> 1)) u06();\n  sub #(.P(A[0] << 2)) u07();\n  sub #(.P({{A[0], A[1]}} + 16'd1)) u08();\n  \
             sub #(.P(pk::PA[1] + 8'd1)) u09();\n  sub #(~A[0]) u10();\n  bsub u11();\n  defparam u11.P = A[1] - 8'd1;\n  \
             sub #(.P(A[0] * A[1])) u14();\n  initial #2 $finish;\nendmodule\n"
        ),
        &[
            "T t.u01 0 1",
            "T t.u02 35 8",
            "T t.u03 8 4",
            "T t.u06 fe 8",
            "T t.u07 d4 8",
            "T t.u08 353d 16",
            "T t.u09 3d 8",
            "T t.u10 ca 8",
            "T t.u11 3b 8",
            "T t.u14 6c 8",
        ],
    );
}

/// An element the OLD operator arm types keeps that arm's value. The arm's name walk does not
/// enter `$clog2`, `$bits` or `$rtoi`, so an element there leaves `declared_override_widths`
/// answering and the tree takes the width-aware assignment walk at that type — not the wide
/// fold, which declines a `[11:4]` element and a `$rtoi` and would leave the value to the
/// unlimited-precision parent fold (`($clog2(AL[1]) - 32'd7) / 32'd2` was `0`, `% 32'd5` was
/// `ffffffff`, a wrapped product `% 32'd7` was `1` in review). Verilator's values; the scalar
/// twin of `-$clog2(AL[1]) / 32'd4` is `3ffffffe` in iverilog too.
#[test]
fn an_element_under_a_call_keeps_the_operator_arms_value() {
    check(
        &format!(
            "{SUB}module t;\n  localparam logic [11:4] AL [0:1] = '{{8'h35, 8'h3C}};\n  \
             localparam logic [0:7] AA [0:1] = '{{8'h35, 8'h3C}};\n  localparam logic [7:0] A [0:1] = '{{8'h35, 8'h3C}};\n  \
             sub #(.P(($clog2(AL[1]) - 32'd7) / 32'd2)) u0();\n  sub #(.P(($clog2(AL[1]) - 32'd7) % 32'd5)) u1();\n  \
             sub #(.P(($rtoi(A[1]) - 32'd61) / 32'd2)) u2();\n  sub #(.P(($clog2(AA[1]) - 32'd7) / 32'd2)) u3();\n  \
             sub #(.P(-$clog2(AL[1]) / 32'd4)) u4();\n  sub #(.P(($bits(AL[0]) - 32'd9) / 32'd2)) u5();\n  \
             sub #(.P(($rtoi(A[1]) * 32'h80000000) % 32'd7)) u6();\n  initial #2 $finish;\nendmodule\n"
        ),
        &[
            "T t.u0 7fffffff 32",
            "T t.u1 00000000 32",
            "T t.u2 7fffffff 32",
            "T t.u3 7fffffff 32",
            "T t.u4 3ffffffe 32",
            "T t.u5 7fffffff 32",
            "T t.u6 00000000 32",
        ],
    );
}

/// A fill in an operator tree wider than 64 bits takes the width of the tree's other
/// operands, on an untyped and on a declared target. PRE: `ffffffff 32` for the untyped cells
/// and `…ffffffff00000006` for the declared one. `128'd0 + '1` is a width split (iverilog binds
/// 129 bits `1ff…f`, verilator 128); vita binds verilator's.
#[test]
fn a_fill_in_a_wide_override_tree_takes_the_trees_width() {
    check(
        &format!(
            "{SUB}module wsub #(parameter [127:0] K = 0) (); initial #1 $display(\"T %m %h %0d\", K, $bits(K)); endmodule\n\
             module t;\n  sub #(.P(~128'd0 | '1)) u0();\n  sub #(.P(128'd0 + '1)) u1();\n  sub #(.P('1 ^ 96'h0)) u2();\n  \
             sub #(.P(~('0 & 80'h1))) u3();\n  wsub #(.K(128'd5 - '1)) w0();\n  wsub #(.K('1 & 100'hF)) w1();\n  \
             initial #2 $finish;\nendmodule\n"
        ),
        &[
            "T t.u0 ffffffffffffffffffffffffffffffff 128",
            "T t.u1 ffffffffffffffffffffffffffffffff 128",
            "T t.u2 ffffffffffffffffffffffff 96",
            "T t.u3 ffffffffffffffffffff 80",
            "T t.w0 00000000000000000000000000000006 128",
            "T t.w1 0000000000000000000000000000000f 128",
        ],
    );
}

/// NOT this change, pinned as measured (ROADMAP §2 "Index sealing"): a fill in an override
/// tree of 64 bits or less keeps the default literal's type — `#(.P(8'd1 | '1))` binds 32
/// bits `ffffffff` where both oracles bind 8 bits `ff`. The operator channel excludes a fill,
/// citing `'1 ^ 1'b0` as an oracle split (iverilog 1 bit, verilator 32) that verilator 5.052
/// does not show.
#[test]
fn a_fill_in_a_narrow_override_tree_is_the_recorded_residue() {
    check(
        &format!(
            "{SUB}module t;\n  sub #(.P(8'd1 | '1)) u5();\n  initial #2 $finish;\nendmodule\n"
        ),
        &["T t.u5 ffffffff 32"],
    );
}
