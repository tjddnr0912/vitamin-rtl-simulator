//! §4.5.420 (ROADMAP §2 🆕 E): an unsized fill INSIDE a sized parameter's initializer is
//! sized by the declared width (§5.7.1 / §11.6.1) in both constant lanes — the i64 lane
//! (33–64 bits) through the width-aware assignment walk, the wide lane (>64) through a
//! fill leaf with a context — and a fill in a self-determined position (a comparison's
//! operand, a logical operand, a ternary's condition, `**`'s exponent) takes its peer's
//! width or one bit. Every expected line is the output of iverilog 13.0 and verilator
//! 5.050 (they agree on all of them).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pfsi_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

fn lines(src: &str, prefix: &str) -> Vec<String> {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "expected exit 0, got {rc:?}:\n{out}");
    out.lines()
        .filter(|l| l.starts_with(prefix))
        .map(|l| l.to_string())
        .collect()
}

fn digest(decls: &str, names: &[&str], fmt: &str) -> String {
    let args = names.join(", ");
    let src = format!("module top;\n{decls}\n  initial begin $display(\"D={fmt}\", {args}); #1 $finish; end\nendmodule\n");
    let v = lines(&src, "D=");
    assert_eq!(v.len(), 1, "{src}");
    v[0][2..].to_string()
}

#[test]
fn row_e_shapes_at_40_bits() {
    let d = "  localparam logic [39:0] V1 = ('1 ^ 1'b0) + (('1 > 1'b0) ? 1'b1 : 1'b0);\n  localparam logic [39:0] V2 = ('1 ^ 1'b0) + (('1 == 1'b1) ? 1'b1 : 1'b0);\n  localparam logic [39:0] V3 = ('1 ^ 1'b0) + ('1 > 1'b0);\n  localparam logic [39:0] V4 = ('1 ^ 1'b0) + (('1 > 1'b0) ? '1 : '0);\n  localparam logic [39:0] V5 = 40'd1 + ('1 > 1'b0);";
    assert_eq!(
        digest(d, &["V1", "V2", "V3", "V4", "V5"], "%h %h %h %h %h"),
        "0000000000 0000000000 0000000000 fffffffffe 0000000002"
    );
}

#[test]
fn fill_in_a_self_determined_position() {
    let d = "  localparam logic [39:0] A1 = ('1 ^ 1'b0) + ('1 > 8'd200);\n  localparam logic [39:0] A2 = ('1 ^ 1'b0) + ('1 == 8'hff);\n  localparam logic [39:0] A3 = ('1 ^ 1'b0) + ('1 == 1'b1);\n  localparam logic [39:0] A4 = ('1 ^ 1'b0) + ('1 == '1);\n  localparam logic [39:0] A5 = ('1 ^ 1'b0) + (8'd200 < '1);\n  localparam logic [39:0] A6 = ('1 ^ 1'b0) + ('1 && 1'b1);\n  localparam logic [39:0] A7 = ('1 ^ 1'b0) + ('0 || 1'b0);\n  localparam logic [39:0] A8 = ('1 ^ 1'b0) + ('1 ? 1'b1 : 1'b0);\n  localparam logic [39:0] A9 = ('1 ^ 1'b0) + (2 ** '1);\n  localparam logic [39:0] B1 = ('1 ^ 1'b0) + ('1 != 40'hffffffffff);\n  localparam logic [39:0] B2 = ('1 ^ 1'b0) + ('1 === 8'hff);";
    assert_eq!(
        digest(d, &["A1", "A2", "A3", "A4", "A5", "A6", "A7", "A8", "A9", "B1", "B2"], "%h %h %h %h %h %h %h %h %h %h %h"),
        "0000000000 0000000000 0000000000 0000000000 0000000000 0000000000 ffffffffff 0000000000 0000000001 ffffffffff 0000000000"
    );
}

#[test]
fn i64_lane_32_bit_context_unchanged() {
    let d = "  localparam logic [31:0] C1 = (32'd1) + ('1 > 1'b0);\n  localparam logic [31:0] C2 = (32'd1) + ('1 > 8'd200);\n  localparam int C3 = ('1 > 1'b0) ? 5 : 6;\n  localparam int C4 = ('1 && 1'b1) + 1;\n  localparam logic [31:0] C5 = (32'd1) + ('1 == 8'hff);\n  localparam logic [31:0] C6 = (32'd1) + ('1 == '1);\n  localparam logic [31:0] D2 = (32'd1) + (1'b1 > 1'b0);";
    assert_eq!(
        digest(
            d,
            &["C1", "C2", "C3", "C4", "C5", "C6", "D2"],
            "%h %h %0d %0d %h %h %h"
        ),
        "00000002 00000002 5 2 00000002 00000002 00000002"
    );
}

#[test]
fn fill_inside_an_operator_across_widths() {
    let d = "  localparam logic [39:0] N1 = '1 ^ 1'b0;\n  localparam logic [39:0] N2 = ('1 ^ 1'b0);\n  localparam logic [39:0] N3 = '1 & 1'b1;\n  localparam logic [39:0] N5 = ('1);\n  localparam logic [39:0] N6 = ('1) + 1'b0;\n  localparam logic [39:0] N8 = ('1 ^ 8'h0);\n  localparam logic [39:0] M1 = ~('1 ^ 1'b0);\n  localparam logic [39:0] M2 = ('1 ^ 1'b0) >> 4;\n  localparam logic [39:0] M4 = 1'b1 ? ('1 ^ 1'b0) : 40'd0;\n  localparam logic [39:0] M5 = ('0 - 1'b1);\n  localparam logic [39:0] M6 = ('1 * 1'b1);\n  localparam int unsigned M7 = ('1 ^ 1'b0);\n  localparam logic [63:0] Y1 = ('1 ^ 1'b0) + (1'b1 > 1'b0);\n  localparam logic [127:0] Y3 = ('1 ^ 1'b0) + (1'b1 > 1'b0);\n  localparam logic [127:0] Y4 = ('1 ^ 1'b0);\n  localparam logic [39:0] Z3 = ('1 ^ 1'b0) | ('1 > 1'b0);";
    assert_eq!(
        digest(d, &["N1", "N2", "N3", "N5", "N6", "N8", "M1", "M2", "M4", "M5", "M6", "M7"], "%h %h %h %h %h %h %h %h %h %h %h %h"),
        "ffffffffff ffffffffff 0000000001 ffffffffff ffffffffff ffffffffff 0000000000 0fffffffff ffffffffff ffffffffff ffffffffff ffffffff"
    );
    assert_eq!(digest(d, &["Y1", "Y3", "Y4", "Z3"], "%h %h %h %h"), "0000000000000000 00000000000000000000000000000000 ffffffffffffffffffffffffffffffff ffffffffff");
}

#[test]
fn generate_scope_binder_takes_the_same_lane() {
    // review B BLOCKING-1: both oracles `ffffffffff` for every binder
    let src = "module top;\n  localparam logic [39:0] M = '1 ^ 1'b0;\n  if (1) begin : g\n    localparam logic [39:0] X = '1 ^ 1'b0;\n    localparam logic [39:0] L = '1;\n  end\n  for (genvar i = 0; i < 1; i++) begin : f\n    localparam logic [39:0] Y = '1 ^ 1'b0;\n  end\n  initial begin $display(\"D=%h %h %h %h\", M, g.X, g.L, f[0].Y); #1 $finish; end\nendmodule\n";
    assert_eq!(
        lines(src, "D="),
        vec!["D=ffffffffff ffffffffff ffffffffff ffffffffff"]
    );
}
