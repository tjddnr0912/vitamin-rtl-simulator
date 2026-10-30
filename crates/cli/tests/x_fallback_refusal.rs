//! §5.2 Q-XFALLBACK: a parameter or a declaration's range bound whose width-aware walk PROVES an `x` — a `/` or `%` by a divisor
//! that is 0 at its region width, or a `**` whose base is 0 there under a negative exponent (IEEE 1800-2017 §11.4.3, Table 11-4)
//! — used to take the width-UNLIMITED fallback's value silently (`localparam int L = (16'd1 % ('1 + 1'b1)) + 1;` bound 2;
//! iverilog `x`, verilator `x`).  Those refuse with E3009 now (`refuses_*`), while every right answer, every absorbed `x`
//! (`&` a known 0, `|` a known 1, a shift moving every `x` bit out of the target), every untaken arm, the decline texts and
//! the route-order discriminators keep today's bytes (`keeps_*`).  The `keeps_*` expectations are the values the pre-change
//! binary prints, measured; the oracle readings behind both groups are in the slice's commit message.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run one design: (stdout minus the trailer, stderr + stdout).  A unique per-test dir keeps the parallel runners apart.
fn run_full(src: &str) -> (String, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_xfb_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    (
        so.lines()
            .filter(|l| !l.contains("simulation ended"))
            .collect::<Vec<_>>()
            .join("\n"),
        format!("{}\n{so}", String::from_utf8_lossy(&out.stderr)),
    )
}

#[test]
fn refuses_m1_int() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = ((16'd1 % ('1 + 1'b1)) + 1);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_m1_untyped() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam CV = ((16'd1 % ('1 + 1'b1)) + 1);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_m1_logic8() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam logic [7:0] CV = ((16'd1 % ('1 + 1'b1)) + 1);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_m1_logic40() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam logic [39:0] CV = ((16'd1 % ('1 + 1'b1)) + 1);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_m1_bit8() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam bit [7:0] CV = ((16'd1 % ('1 + 1'b1)) + 1);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_m1_logic64_edge() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam logic [63:0] CV = ((16'd1 % ('1 + 1'b1)) + 1);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_m1_generate() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  if (1) begin : gb
    localparam int CV = ((16'd1 % ('1 + 1'b1)) + 1);
    initial #1 $display("V c=%0d", CV);
  end
  initial #2 $finish;
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_m1_package() {
    let (out, err) = run_full(
        r#"package pq;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = ((16'd1 % ('1 + 1'b1)) + 1);
endpackage
module top;
  initial begin #1 $display("V c=%0d", pq::CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
    assert!(
        err.contains("package parameter `CV` value is not a foldable constant"),
        "package text lost: {err}"
    );
}

#[test]
fn refuses_m1_unit() {
    let (out, err) = run_full(
        r#"localparam logic [15:0] ZZ = 16'd0;
localparam logic [0:0] N1 = 1'b1;
localparam logic [1:0] N3 = 2'd3;
localparam int CV = ((16'd1 % ('1 + 1'b1)) + 1);
module top;
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_m1_header_int() {
    let (out, err) = run_full(
        r#"module sh #(parameter int CV = ((16'd1 % ('1 + 1'b1)) + 1));
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  initial #1 $display("V c=%0d", CV);
endmodule
module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  sh u ();
  initial #2 $finish;
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_m1_header_untyped() {
    let (out, err) = run_full(
        r#"module sh #(parameter CV = ((16'd1 % ('1 + 1'b1)) + 1));
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  initial #1 $display("V c=%0d", CV);
endmodule
module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  sh u ();
  initial #2 $finish;
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_d1_div() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = ((16'd1 / ('1 + 1'b1)) + 1);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_pw_zero_base_negative_power() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = ((('1 + 1'b1) ** -1) + 5);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

/// KNOWN-WRONG on a 4-state target, kept: the width-unlimited fallback is 0 under every x bit,
/// which is the value a 2-state target holds (IEEE 1800-2017 §6.11.2), and the binder cannot
/// tell `logic` from `bit` (Q-PARAM2STATE refuses it once it can).  PRE 0000000000000000 ;
/// iverilog xxxxxxxxxxxxxxxx ; verilator xxxxxxxxxxxxxxxx (Q/plan/pairs pz16__B16).
#[test]
fn keeps_pz16_fill_free_zero_base_known_wrong() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam logic [15:0] CV = ((16'hFFFF + 16'd1) ** (-1));
  initial begin #1 $display("V c=%b", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=0000000000000000"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// The fill-free zero `**` base with a fallback that is NOT 0 under the x bits: refused.
/// PRE 0000000000000001 ; iverilog, sv2v and verilator xxxxxxxxxxxxxxxx.
#[test]
fn refuses_pz16_plus_one_fill_free_zero_base() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] CV = ((16'hFFFF + 16'd1) ** (-1)) + 16'd1;
  initial begin #1 $display("V c=%b", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("Table 11-4"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_w8_logic8() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam logic [7:0] CV = ((8'd1 / (8'd255 + 8'd1)) + 8'd1);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_shl32_logic40() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam logic [39:0] CV = (((16'd1 % ('1 + 1'b1)) << 32) + 5);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_mul0() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = (((16'd1 % ('1 + 1'b1)) * 0) + 5);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_tk_taken_arm() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = (1'b0 ? 16'd5 : ((16'd1 % ('1 + 1'b1)) + 1));
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_neg() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = (-(16'd1 % ('1 + 1'b1)) + 7);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_x_pow_0() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam logic [15:0] CV = ((16'd1 % ('1 + 1'b1)) ** 0);
  initial begin #1 $display("V c=%b", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_andlo8_logic16() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam logic [15:0] CV = ((16'd1 % ('1 + 1'b1)) & 16'h00FF);
  initial begin #1 $display("V c=%b", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_xor0_logic16() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam logic [15:0] CV = ((16'd1 % ('1 + 1'b1)) ^ 16'h0000);
  initial begin #1 $display("V c=%b", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_shl8_logic16() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam logic [15:0] CV = ((16'd1 % ('1 + 1'b1)) << 8);
  initial begin #1 $display("V c=%b", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_nm_named_region() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = ((16'd1 % ('1 + N1)) + 1);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_bound_rb() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  logic [((16'd1 % ('1 + 1'b1)) + 1):0] d;
  initial begin #1 $display("V n=%0d", $bits(d)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_bound_ub() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  logic u [0:((16'd1 % ('1 + 1'b1)) + 1)];
  initial begin #1 $display("V n=%0d", $size(u)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_bound_us() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  logic s [((16'd1 % ('1 + 1'b1)) + 1)];
  initial begin #1 $display("V n=%0d", $size(s)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_bound_pd() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam logic [((16'd1 % ('1 + 1'b1)) + 1):0] PDX = '0;
  initial begin #1 $display("V n=%0d", $bits(PDX)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_bound_pk() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  logic [1:0][((16'd1 % ('1 + 1'b1)) + 1):0] d;
  initial begin #1 $display("V n=%0d", $bits(d)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_bound_td() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  typedef logic [((16'd1 % ('1 + 1'b1)) + 1):0] t_t; t_t d;
  initial begin #1 $display("V n=%0d", $bits(d)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_bound_pt() {
    let (out, err) = run_full(
        r#"module sp(input logic [((16'd1 % ('1 + 1'b1)) + 1):0] p);
  initial #1 $display("V n=%0d", $bits(p));
endmodule
module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  logic [63:0] w = '0;
  sp u (.p(w));
  initial #2 $finish;
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn refuses_m24_p11_four_bit() {
    let (out, err) = run_full(
        r#"module t;
  localparam logic [3:0] N = 4'd1 % (4'd15 + 4'd1);
  initial begin #1 $display("V n=%b", N); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("E3009"), "expected E3009: {err}");
    assert!(
        err.contains("is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

#[test]
fn keeps_and0_absorbed() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = (((16'd1 % ('1 + 1'b1)) & 32'd0) + 5);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=5"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_shl32_int_absorbed() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = (((16'd1 % ('1 + 1'b1)) << 32) + 5);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=5"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_shl8_into_logic8_absorbed() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam logic [7:0] CV = ((16'd1 % ('1 + 1'b1)) << 8);
  initial begin #1 $display("V c=%b", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=00000000"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_shr8_and_hi_logic16_absorbed() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam logic [15:0] CV = (((16'd1 % ('1 + 1'b1)) >> 8) & 16'hFF00);
  initial begin #1 $display("V c=%b", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=0000000000000000"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_or_fill_logic16_absorbed() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam logic [15:0] CV = ((16'd1 % ('1 + 1'b1)) | '1);
  initial begin #1 $display("V c=%b", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=1111111111111111"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_untk_untaken_arm() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = (1'b1 ? 16'd5 : (16'd1 % ('1 + 1'b1)));
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=5"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_q9_ctrl() {
    let (out, err) = run_full(
        r#"module top;
  // row 48 r2 F3 control: an untaken x/0 arm under a KNOWN condition whose taken arm has no sign question (A4 right today?),
  // beside review r1's A3 (`(Z != 0) ? (16'd100 % Z) : NM`, the sign question) and a bare taken-arm read (A5).
  localparam logic signed [7:0] NM = -8'sd2;
  localparam logic [15:0] Z = 16'd0;
  localparam logic [15:0] A3 = (Z != 0) ? (16'd100 % Z) : NM;
  localparam logic [15:0] A4 = (Z != 0) ? (16'd100 % Z) : 16'd5;
  localparam logic [15:0] A5 = (Z != 0) ? 16'd7 : 16'd5;
  initial begin #1 $display("V A3=%h A4=%h A5=%h", A3, A4, A5); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(
        got,
        ["V A3=fffe A4=0005 A5=0005"],
        "value moved: {out} {err}"
    );
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_disc_int() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = ((16'd5 % ('1 + 2'd3)) + 3);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=4"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_disc_bound() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  logic [((16'd5 % ('1 + 2'd3)) + 3):0] d;
  initial begin #1 $display("V n=%0d", $bits(d)); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V n=5"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_nm2_named_discriminator() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = ((16'd5 % ('1 + N3)) + 3);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=4"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_w8_int() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = ((8'd1 / (8'd255 + 8'd1)) + 8'd1);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=1"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_zero_base_pow0() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = ((('1 + 1'b1) ** 0) + 5);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=6"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_zero_base_pow2() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = ((('1 + 1'b1) ** 2) + 5);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=5"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_call_route_bound() {
    let (out, err) = run_full(
        r#"module top;
  function automatic logic [7:0] f8(); return 8'hFF; endfunction
  logic [((f8() + 8'd1) + 8'd4) : 0] d;
  initial begin #1 $display("V n=%0d", $bits(d)); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V n=5"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_k_pax() {
    let (out, err) = run_full(
        r#"module top;
  parameter real R = 4;
  parameter PA = R;
  localparam int A2 = PA / 8 + (16'd1 % ('1 + 1'b1));
  initial begin #1 $display("V A2=%0d", A2); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V A2=1"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_r1_tdtwin() {
    let (out, err) = run_full(
        r#"module top;
  // review r3 Q1c: a typedef'd real with an exact-integer initializer (is it given an i64 twin?) inside Q-XFALLBACK's x / 0 region.
  // IEEE: X / 8 = 0.5 (real); the integral operand (16'd1 % ('1 + 1'b1)) is x, converted to real as 0 (§6.12.2) -> 0.5 -> int 1.
  // A1 = X / 8 -> 1 ; A2 = X / 8 + x-region -> 1 ; A3 = (X / 8) * 2 -> 1
  typedef real rt;
  localparam rt X = 4;
  localparam int A1 = X / 8;
  localparam int A2 = X / 8 + (16'd1 % ('1 + 1'b1));
  localparam int A3 = (X / 8) * 2;
  initial begin #1 $display("V A1=%0d A2=%0d A3=%0d", A1, A2, A3); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V A1=0 A2=1 A3=0"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_im_exact() {
    let (out, err) = run_full(
        r#"`timescale 1ns/1ns
// r4 P3: an EXPLICITLY imported package real with an exact-integer value (pkg_consts twin): does the import lane bind it real, as
// its i64 twin alone (override_type.rs:29-31), or loud? IEEE: A1 = 4.0 / 8 = 0.5 -> 1 ; A2 = 0.5 + 0 -> 1 ; A3 = (0.5 > 0) + 2 = 3
package p;
  localparam real PX = 4;
endpackage
module top;
  import p::PX;
  localparam int A1 = PX / 8;
  localparam int A2 = PX / 8 + (16'd1 % ('1 + 1'b1));
  localparam int A3 = (PX / 8 > 0) + 2;
  initial begin #1 $display("V A1=%0d A2=%0d A3=%0d", A1, A2, A3); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V A1=0 A2=1 A3=2"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// KNOWN-WRONG, kept on purpose: a local `int PX` beside an imported package real `PX` —
/// c3 (`names_imported_real`) is conservative and does not certify the tree, so PRE's
/// width-unlimited fallback binds.  PRE A=1 B=1 ; iverilog A=0 B=x ; sv2v A=x B=x ;
/// verilator A=x B=x (review-design/probe/k_shx).  The cost of c3, stated by the row.
#[test]
fn keeps_k_shx() {
    let (out, err) = run_full(
        r#"package p;
  parameter real PX = 4;
endpackage
module top;
  import p::*;
  localparam int PX = 4;
  localparam int A = PX / 8 + (16'd1 % ('1 + 1'b1));
  initial begin #1 $display("V A=%0d", A); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V A=1"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_cast2() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = (int'((16'd1 % ('1 + 1'b1))) + 5);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=6"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_eq_bound_one_bit() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  logic [(((16'd1 % ('1 + 1'b1)) == 0) + 5):0] d;
  initial begin #1 $display("V n=%0d", $bits(d)); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V n=1"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_xlitb_bound_one_bit() {
    let (out, err) = run_full(
        r#"module top;
  logic [((4'b1x00 == 4'b1000) + 1):0] d;
  initial begin #1 $display("V n=%0d", $bits(d)); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V n=1"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

#[test]
fn keeps_text_eq() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = (((16'd1 % ('1 + 1'b1)) == 0) + 5);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("parameter `CV` value is not a constant: the `==` operation has no constant-fold arm [in top]"), "text moved: {err}");
}

#[test]
fn keeps_text_condx() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = ((16'd1 % ('1 + 1'b1)) ? 5 : 3);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("parameter `CV` value is not a constant: the conditional `? :` has no constant-fold arm [in top]"), "text moved: {err}");
}

#[test]
fn keeps_text_shc() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = ((5 << (16'd1 % ('1 + 1'b1))) + 1);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("parameter `CV` value is not a constant: the `<<` operation has no constant-fold arm [in top]"), "text moved: {err}");
}

#[test]
fn keeps_text_1powx() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = ((16'd1 ** (16'd1 % ('1 + 1'b1))) + 5);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("parameter `CV` value is not a constant: the `**` operation has no constant-fold arm [in top]"), "text moved: {err}");
}

#[test]
fn keeps_text_lnot() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = (!(16'd1 % ('1 + 1'b1)) + 5);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("parameter `CV` value is not a constant: the `!` operation has no constant-fold arm [in top]"), "text moved: {err}");
}

#[test]
fn keeps_text_red() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = ((|(16'd1 % ('1 + 1'b1))) + 5);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("parameter `CV` value is not a constant: a reduction of an operand whose width the constant domain cannot read: a parameter sized from its value (no range, type or sized literal), or one declared ascending `[0:N]` / with a non-zero low bound `[H:L]` [in top]"), "text moved: {err}");
}

#[test]
fn keeps_text_clog() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = ($clog2((16'd1 % ('1 + 1'b1))) + 5);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(
        err.contains(
            "parameter `CV` value is not a constant: `$clog2(…)` has no constant-fold arm [in top]"
        ),
        "text moved: {err}"
    );
}

#[test]
fn keeps_text_cat() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  localparam int CV = ({(16'd1 % ('1 + 1'b1))} + 5);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("parameter `CV` value is not a constant: the concatenation `{…}` has no constant-fold arm [in top]"), "text moved: {err}");
}

#[test]
fn keeps_text_literal_div_zero() {
    let (out, err) = run_full(
        r#"module top;
  localparam int P = 1 / 0;
  initial begin #1 $display("V c=%0d", P); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(err.contains("parameter `P` value is not a constant: the `/` operation divides by zero, which is `x` and not a constant [in top]"), "text moved: {err}");
}

#[test]
fn keeps_text_const_function_return() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  function automatic int g(); return ((16'd1 % ('1 + 1'b1)) + 1); endfunction
  localparam int CV = g();
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(
        err.contains(
            "parameter `CV` value is not a constant: `g(…)` has no constant-fold arm [in top]"
        ),
        "text moved: {err}"
    );
}

#[test]
fn keeps_text_shl32_bound() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] ZZ = 16'd0;
  localparam logic [0:0] N1 = 1'b1;
  localparam logic [1:0] N3 = 2'd3;
  logic [(((16'd1 % ('1 + 1'b1)) << 32) + 5):0] d;
  initial begin #1 $display("V n=%0d", $bits(d)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(
        err.contains("declared net width 4294967296 exceeds the v1 cap (1048576) [in top]"),
        "text moved: {err}"
    );
}

#[test]
fn keeps_text_package_non_x() {
    let (out, err) = run_full(
        r#"package p;
  function automatic int pf(input int k); return k * 3; endfunction
  parameter logic [1:0][7:0] E = '{pf(1), pf(2)};
endpackage
module t;
  initial $display("V %h", p::E);
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(
        err.contains("package parameter `E` value is not a foldable constant [in p]"),
        "text moved: {err}"
    );
}

/// An absorbed x (`& 0`) under `*` computes exactly (F-ABSARITH): 5 in PRE, iverilog, sv2v, verilator.
#[test]
fn keeps_a1_and0_mul() {
    let (out, err) = run_full(
        r#"module top;
  localparam int CV = ((((16'd1 % ('1 + 1'b1)) & 32'd0) * 3) + 5);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=5"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// An absorbed x under unary `-`: 5 in PRE, iverilog, sv2v, verilator.
#[test]
fn keeps_a2_and0_neg() {
    let (out, err) = run_full(
        r#"module top;
  localparam int CV = ((-((16'd1 % ('1 + 1'b1)) & 32'd0)) + 5);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=5"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// An absorbed x as a `**` base: 5 in PRE, iverilog, sv2v, verilator.
#[test]
fn keeps_a3_and0_pow1() {
    let (out, err) = run_full(
        r#"module top;
  localparam int CV = ((((16'd1 % ('1 + 1'b1)) & 32'd0) ** 1) + 5);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=5"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// An absorbed x under unary `-` in a range bound: 6 bits in PRE, iverilog, sv2v, verilator.
#[test]
fn keeps_a5_and0_bound() {
    let (out, err) = run_full(
        r#"module top;
  logic [((-((16'd1 % ('1 + 1'b1)) & 32'd0)) + 5) : 0] d;
  initial begin #1 $display("V bitsd=%0d", $bits(d)); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V bitsd=6"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// RIGHT for a 2-state target, kept (F-2STATE): the fallback is 0 under every x bit, the value
/// a 2-state type holds (IEEE 1800-2017 §6.11.2).  PRE 0 ; iverilog 0 ; sv2v 0 ; verilator x.
#[test]
fn keeps_s2a_int_zero_dividend() {
    let (out, err) = run_full(
        r#"module top;
  localparam int L = 16'd0 % ('1 + 1'b1);
  initial begin #1 $display("V L=%0d %h", L, L); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V L=0 00000000"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// RIGHT for a 2-state target, kept (F-2STATE): the fallback is 0 under every x bit, the value
/// a 2-state type holds (IEEE 1800-2017 §6.11.2).  PRE 0 ; iverilog 0 ; sv2v x ; verilator x.
#[test]
fn keeps_s2b_int_mul0_bare() {
    let (out, err) = run_full(
        r#"module top;
  localparam int L = ((16'd1 % ('1 + 1'b1)) * 32'd0);
  initial begin #1 $display("V L=%0d %h", L, L); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V L=0 00000000"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// RIGHT for a 2-state target, kept (F-2STATE): the fallback is 0 under every x bit, the value
/// a 2-state type holds (IEEE 1800-2017 §6.11.2).  PRE 0 ; iverilog 0 ; sv2v x ; verilator x.
#[test]
fn keeps_s2c_bit8_zero_div() {
    let (out, err) = run_full(
        r#"module top;
  localparam bit [7:0] L = 8'd0 / ('1 + 1'b1);
  initial begin #1 $display("V L=%0d %h", L, L); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V L=0 00"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// RIGHT for a 2-state target, kept (F-2STATE): the fallback is 0 under every x bit, the value
/// a 2-state type holds (IEEE 1800-2017 §6.11.2).  PRE 0 ; iverilog 0 ; sv2v 0 ; verilator x.
#[test]
fn keeps_s2d_int_xsubx() {
    let (out, err) = run_full(
        r#"module top;
  localparam int L = (16'd1 % ('1 + 1'b1)) - (16'd1 % ('1 + 1'b1));
  initial begin #1 $display("V L=%0d %h", L, L); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V L=0 00000000"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// RIGHT for a 2-state target, kept (F-2STATE): the fallback is 0 under every x bit, the value
/// a 2-state type holds (IEEE 1800-2017 §6.11.2).  PRE 255 00ff ; iverilog 255 00ff ; sv2v xxff ; verilator xxff.
#[test]
fn keeps_s2f_shortint_partial() {
    let (out, err) = run_full(
        r#"module top;
  localparam shortint L = ((16'd0 % ('1 + 1'b1)) << 8) | 16'h00FF;
  initial begin #1 $display("V L=%0d %h", L, L); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V L=255 00ff"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// A range bound made `x` by a `/` and by a zero `**` base under a negative exponent: the
/// bound prefilter reaches all three origin operators.  PRE 3 / 3 bits ; iverilog 1 / 1 ;
/// sv2v x / 3 ; verilator rejects both (IEEE 1800-2017 §6.9.1).
#[test]
fn refuses_bound_div_and_pow() {
    let (out, err) = run_full(
        r#"module top;
  logic [((16'd1 / ('1 + 1'b1)) + 2) : 0] d;
  logic [((('1 + 1'b1) ** -1) + 2) : 0] e;
  initial begin #1 $display("V bitsd=%0d bitse=%0d", $bits(d), $bits(e)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        err.matches("this range bound is `x`").count(),
        2,
        "expected two x bounds: {err}"
    );
    assert!(
        err.contains("the `/` is `x`") && err.contains("the `**` is `x`"),
        "expected both x reasons: {err}"
    );
}

/// A constant function returning an ABSORBED tree keeps today's loud: the interpreter reads
/// the legacy entry, which answers `None` for a `Four` with no x bit (sticky).  PRE loud ;
/// iverilog, sv2v, verilator 5.  A broken sticky rule would bind 5 here.
#[test]
fn keeps_text_cf_absorbed_return() {
    let (out, err) = run_full(
        r#"module top;
  function automatic int g();
    return (((16'd1 % ('1 + 1'b1)) & 32'd0) + 5);
  endfunction
  localparam int CV = g();
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(
        err.contains(
            "parameter `CV` value is not a constant: `g(…)` has no constant-fold arm [in top]"
        ),
        "text moved: {err}"
    );
}

/// A reduction over a constant-function LOCAL takes the walk's reduction arm (a boundary: the
/// operand is read through the legacy entry).  PRE loud ; iverilog 0 ; sv2v x ; verilator x.
#[test]
fn keeps_text_cf_reduction_local() {
    let (out, err) = run_full(
        r#"module top;
  function automatic int g();
    logic [15:0] t;
    t = 16'd1;
    return (|(t % ('1 + 1'b1))) + 5;
  endfunction
  localparam int CV = g();
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(
        err.contains(
            "parameter `CV` value is not a constant: `g(…)` has no constant-fold arm [in top]"
        ),
        "text moved: {err}"
    );
}

/// `~` over a proved x keeps the x bits (bitwise, §11.4.8), then `+` makes every bit x.  PRE
/// 1111111111111111 ; iverilog, sv2v, verilator xxxxxxxxxxxxxxxx.
#[test]
fn refuses_b1_bitnot_add() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] CV = (~(16'd1 % ('1 + 1'b1))) + 16'd1;
  initial begin #1 $display("V c=%b", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(
        err.contains("parameter `CV` value is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

/// `~^` keeps the x bits of either side.  PRE 1111111111111110 ; iverilog, sv2v, verilator
/// xxxxxxxxxxxxxxxx.
#[test]
fn refuses_b2_xnor() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] CV = ((16'd1 % ('1 + 1'b1)) ~^ 16'h0000);
  initial begin #1 $display("V c=%b", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(
        err.contains("parameter `CV` value is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

/// A SIGNED region's `>>>` refills from an x sign bit (§11.4.10): x only at bit 15, `>>> 4`
/// makes [15:12] x and `& F000` keeps exactly those.  PRE 1111000000000000 ; iverilog, sv2v,
/// verilator xxxx000000000000.
#[test]
fn refuses_b3_signed_ashr_refill() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic signed [15:0] CV = (((((16'sd1 % (16'sh7FFF + 16'sh7FFF + 16'sd2)) & 16'sh8000) ^ 16'sh8000) >>> 4) & 16'shF000);
  initial begin #1 $display("V c=%b", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(
        err.contains("parameter `CV` value is `x`") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

/// The same tree in an UNSIGNED region: `>>>` is logical, the x bit moves to bit 11 and
/// `& F000` clears it (absorbed).  0000000000000000 in PRE, iverilog, sv2v, verilator.
#[test]
fn keeps_b4_unsigned_ashr_control() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] CV = (((((16'd1 % (16'h7FFF + 16'h7FFF + 16'd2)) & 16'h8000) ^ 16'h8000) >>> 4) & 16'hF000);
  initial begin #1 $display("V c=%b", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=0000000000000000"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// A typedef'd 40-bit packed container's binder: `PX` is x in its 40-bit region.  PRE
/// `PT=0000000001 PX=0000000001 r=xx` ; iverilog and verilator PX=xxxxxxxxxx (sv2v
/// 0000000001, but it also sizes `'1` at 32 inside the region: PT=5).
#[test]
fn refuses_p11_typed() {
    let (out, err) = run_full(
        r#"module top;
  typedef struct packed { logic [15:8] a; logic [0:7] c; logic [8:15] d; logic [1:0][3:0] m; logic [7:0] z; } h_t;
  localparam h_t PT = 16'd5 % ('1 + 2'd3);
  localparam h_t PX = 16'd1 % ('1 + 1'b1);
  localparam bit [3:0] N = 4'd7;
  initial begin #1 $display("V PT=%h PX=%h r=%b", PT, PX, PX.a[~N +: 2]); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(
        err.contains("parameter `PX` value is `x`")
            && err.contains("is 0 in its 40-bit context")
            && !err.contains("parameter `PT`"),
        "expected PX refused at 40 bits, PT kept: {err}"
    );
}

/// tq03 (a row-24 cell): a 40-bit and a 4-bit binder, each x in its own region.  PRE `A
/// v=0000000001 n=0001 rt=xxxx / A vs=00 r=0` ; iverilog `A v=xxxxxxxxxx n=xxxx rt=xxxx / A
/// vs=xx r=x` ; sv2v n=xxxx r=x ; verilator rejects the select (index constantly unknown).
#[test]
fn refuses_tq03_plain_twin() {
    let (out, err) = run_full(
        r#"module t;
  localparam logic [39:0] V = 16'd1 % ('1 + 1'b1);
  localparam logic [3:0] N = 4'd1 % (4'd15 + 4'd1);
  localparam logic [15:8] VA = 8'hA5;
  logic [3:0] rt;
  initial begin
    rt = 4'd1 % (4'd15 + 4'd1);
    #1;
    $display("A v=%h n=%b rt=%b", V, N, rt);
    $display("A vs=%b r=%b", V[9:8], VA[~N]);
  end
  initial #10 $finish;
endmodule
"#,
    );
    assert!(!out.contains("A "), "a value was bound: {out}");
    assert!(
        err.contains("parameter `V` value is `x`") && err.contains("parameter `N` value is `x`"),
        "expected V and N refused: {err}"
    );
}

/// tq07: the h_t container and its 40-bit twin, both x.  PRE `A x=00` ; iverilog `A x=xx` ;
/// verilator `A x=xx` (sv2v 00).
#[test]
fn refuses_tq07_container_twin() {
    let (out, err) = run_full(
        r#"module t;
  typedef struct packed { logic [15:8] a; logic [0:7] c; logic [8:15] d; logic [1:0][3:0] m; logic [7:0] z; } h_t;
  localparam h_t P = 16'd1 % ('1 + 1'b1);
  localparam logic [39:0] VQ0 = 16'd1 % ('1 + 1'b1);
  localparam logic [15:8] VQ = VQ0[39:32];
  localparam bit [3:0] N = 4'd7;
  localparam logic [1:0] X = VQ[~N +: 2];
  initial $display("A x=%b", X);
  initial #10 $finish;
endmodule
"#,
    );
    assert!(!out.contains("A "), "a value was bound: {out}");
    assert!(
        err.contains("parameter `P` value is `x`") && err.contains("parameter `VQ0` value is `x`"),
        "expected P and VQ0 refused: {err}"
    );
}

/// tq20: a 4-bit index parameter that is x.  PRE `A x=0` ; iverilog `A x=x` ; sv2v `A x=x` ;
/// verilator internal error on 4'bxxxx.
#[test]
fn refuses_tq20_index_param() {
    let (out, err) = run_full(
        r#"typedef struct packed { logic [15:8] a; logic [0:7] c; logic [8:15] d; logic [1:0][3:0] m; logic [7:0] z; } h_t;
module t;
  localparam logic [3:0] N = 4'd1 % ('1 + 1'b1);
  localparam h_t P = 40'hA53CF0960F;
  localparam logic [15:8] VP = 8'hA5;
  localparam logic X = VP[~N];
  initial $display("A x=%b", X);
  initial #10 $finish;
endmodule
"#,
    );
    assert!(!out.contains("A "), "a value was bound: {out}");
    assert!(
        err.contains("parameter `N` value is `x`") && err.contains("is 0 in its 4-bit context"),
        "expected N refused at 4 bits: {err}"
    );
}

/// tq22: the same index read by a run-time select.  PRE `A r=0` ; iverilog `A r=x` ; sv2v `A
/// r=x` ; verilator rejects (index constantly unknown).
#[test]
fn refuses_tq22_runtime_select() {
    let (out, err) = run_full(
        r#"typedef struct packed { logic [15:8] a; logic [0:7] c; logic [8:15] d; logic [1:0][3:0] m; logic [7:0] z; } h_t;
module t;
  localparam logic [3:0] N = 4'd1 % ('1 + 1'b1);
  localparam h_t P = 40'hA53CF0960F;
  localparam logic [15:8] VP = 8'hA5;
  initial begin #1; $display("A r=%b", VP[~N]); end
  initial #10 $finish;
endmodule
"#,
    );
    assert!(!out.contains("A "), "a value was bound: {out}");
    assert!(
        err.contains("parameter `N` value is `x`"),
        "expected N refused: {err}"
    );
}

/// A refused binder whose tree ALSO holds a literal `1 / 0` (in the untaken arm): the x clause
/// is asked before the specific reasons (which walk every arm), so the cause is the TAKEN arm's
/// `%` — never the untaken `/` the value does not evaluate (lens C-W2 / F3).  PRE 2 ; iverilog 0
/// (an `int`) ; sv2v x ; verilator x.
#[test]
fn refuses_m31_untaken_arm_reason() {
    let (out, err) = run_full(
        r#"module top;
  localparam int CV = 1'b1 ? ((16'd1 % ('1 + 1'b1)) + 1) : (1 / 0);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(
        err.contains("parameter `CV` value is `x`: the divisor of the `%` (the `+` operation) is 0 in its 32-bit context, so the `%` is `x` (IEEE 1800-2017 §11.4.3), and a parameter has no x/z plane [in top]"),
        "text moved: {err}"
    );
    assert!(
        !err.contains("divides by zero"),
        "named the untaken arm: {err}"
    );
}

/// KNOWN-WRONG (Q-DECLSELBOUND): a 65-bit bound region — the divisor is 2^64, not 0, at 65
/// bits — that neither walk folds keeps the documented 1-bit default.  PRE 1 ; iverilog,
/// sv2v, verilator 3.  The x origin is decided only at a region width of 1..=64.
#[test]
fn keeps_g1_wide65_bound_known_wrong() {
    let (out, err) = run_full(
        r#"module top;
  logic [((65'd1 % (65'h0FFFFFFFFFFFFFFFF + 65'd1)) + 1) : 0] d;
  initial begin #1 $display("V bitsd=%0d", $bits(d)); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V bitsd=1"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// An untyped parameter retyped by its override (§6.20.2: `C = 3'd1` overridden by `8'd1` is 8
/// bits): at the stale 3-bit width `C + 3'd7` wraps to 0, at the true 8 bits it is 8, so `L`
/// is 1.  The certificate (c2: a value-inferred width is not a fact) keeps the value.  PRE
/// `L=1` ; iverilog `bitsC=8 L=1` ; verilator `bitsC=3 L=x` (it keeps the default's width).
#[test]
fn keeps_h1_ovr_untyped() {
    let (out, err) = run_full(
        r#"module grand #(parameter Q = 1) ();
  initial begin #1 $display("V grand bitsQ=%0d Q=%0d", $bits(Q), Q); end
endmodule
module child #(parameter C = 3'd1) ();
  localparam logic [2:0] L = (3'd1 % (C + 3'd7)) | '0;
  grand #(.Q(C | 1'b0)) g();
  initial begin #1 $display("V bitsC=%0d C=%0d L=%0d", $bits(C), C, L); end
endmodule
module top;
  child #(.C(8'd1)) u();
  initial #3 $finish;
endmodule
"#,
    );
    let mut got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    got.sort_unstable();
    assert_eq!(
        got,
        ["V bitsC=8 C=1 L=1", "V grand bitsQ=8 Q=1"],
        "value moved: {out} {err}"
    );
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// Round C battery gap (M11: a left shift that keeps the x mask in place).  At a 32-bit bound
/// region `X << 16` holds x in bits 16-31 only, so `& 32'h0000FFFF` absorbs every x bit and
/// the bound is 3.  The binder cannot see this mutant (a shifted-in bit is a known 0, so the
/// fallback has 0 there — F-2STATE keeps it); the bound has no 2-state rule.  PRE 4 ; iverilog,
/// sv2v, verilator 4.
#[test]
fn keeps_k11_shl_then_mask_bound() {
    let (out, err) = run_full(
        r#"module top;
  logic [(((((16'd1 % ('1 + 1'b1)) << 16) & 32'h0000FFFF) + 3)) : 0] d;
  initial begin #1 $display("V bitsd=%0d", $bits(d)); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V bitsd=4"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// Round C battery gap (M13: a logical right shift that moves the x mask left).  `X >> 16`
/// holds x in bits 0-15 only, so `& 32'hFFFF0000` absorbs it and the bound is 3.  PRE 4 ;
/// iverilog, sv2v, verilator 4.
#[test]
fn keeps_k13_shr_then_mask_bound() {
    let (out, err) = run_full(
        r#"module top;
  logic [(((((16'd1 % ('1 + 1'b1)) >> 16) & 32'hFFFF0000) + 3)) : 0] d;
  initial begin #1 $display("V bitsd=%0d", $bits(d)); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V bitsd=4"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// Round C battery gap (M04: a zero base is an origin under any exponent).  `16'd0 ** 2` is 0
/// (Table 11-4: a negative exponent is the only x case), so the bound is 3.  PRE 4 ; iverilog,
/// sv2v, verilator 4.
#[test]
fn keeps_k04b_zero_base_positive_power_bound() {
    let (out, err) = run_full(
        r#"module top;
  logic [(((16'd0 ** 2) + 3)) : 0] d;
  initial begin #1 $display("V bitsd=%0d", $bits(d)); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V bitsd=4"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// The binder twin of the M04 cell: `(16'd0 ** 2) + 5` is 5.  PRE 5 ; iverilog, sv2v,
/// verilator 5.
#[test]
fn keeps_k04p_zero_base_positive_power_binder() {
    let (out, err) = run_full(
        r#"module top;
  localparam int CV = ((16'd0 ** 2) + 5);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=5"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// Round C battery gap (M23: the binder tests x at the expression's SELF width, not the
/// declared one).  The tree is fill-free with a known 32-bit self width; at the declared 40
/// bits `32'd2 ** 6'd40` wraps to 0, so the `%` is x, `<< 32` leaves x in bits 32-39 only and
/// `| 32'd5` keeps it there (a `+` would spread it into bits 0-31, where the self-width test
/// sees it too).  PRE `V c=0100000005` ; iverilog, sv2v, verilator `xx00000005`.
#[test]
fn refuses_m23_self32_declared40() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [39:0] CV = (((32'd1 % (32'd2 ** 6'd40)) << 32) | 32'd5);
  initial begin #1 $display("V c=%h", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert!(
        err.contains("is `x`") && err.contains("40-bit context") && err.contains("§11.4.3"),
        "expected the x reason: {err}"
    );
}

/// The E3009 lines of one run whose text carries `needle`.
fn e3009_with(err: &str, needle: &str) -> usize {
    err.lines()
        .filter(|l| l.contains("E3009") && l.contains(needle))
        .count()
}

// ---- round D: lens F1 (the bound dedup) ----

/// Two instances of one declaration: `u0`'s bound folds to nothing (`5 / 0` in both walks; the
/// documented 1-bit default, no line printed), `u1`'s is `x` in its 32-bit region
/// (`'hFFFFFFFF + 1` wraps to 0).  `u1` is refused: the `x` report keeps its own dedup set, so
/// `u0`'s silent mark does not silence it (lens F1).  PRE `u0 1 u1 4` ; iverilog 1 / 1 ; sv2v
/// x / x ; verilator "left side of bit range isn't a two-state constant" (§6.9.1).
#[test]
fn refuses_dd1_x_bound_after_a_silent_sibling() {
    let (out, err) = run_full(
        r#"module m #(parameter logic [31:0] A = 0, parameter logic [31:0] B = 0) ();
  logic [((16'd5 / (A + B)) + 16'd3):0] v;
  initial begin #1 $display("V %m bits=%0d", $bits(v)); end
endmodule
module top;
  m #(.A(32'd0), .B(32'd0)) u0();
  m #(.A(32'hFFFFFFFF), .B(32'd1)) u1();
  initial #2 $finish;
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(&err, "this range bound is `x`: the divisor of the `/` (the `+` operation) is 0 in its 32-bit context"),
        1,
        "expected one x bound for u1: {err}"
    );
    assert!(err.contains("[in top.u1]"), "expected u1's location: {err}");
}

/// The same declaration with only the `x` instance (the F1 control: refused before and after).
/// PRE `u1 4` ; iverilog 1 ; sv2v x ; verilator rejects (§6.9.1).
#[test]
fn refuses_dd2_x_bound_alone() {
    let (out, err) = run_full(
        r#"module m #(parameter logic [31:0] A = 0, parameter logic [31:0] B = 0) ();
  logic [((16'd5 / (A + B)) + 16'd3):0] v;
  initial begin #1 $display("V %m bits=%0d", $bits(v)); end
endmodule
module top;
  m #(.A(32'hFFFFFFFF), .B(32'd1)) u1();
  initial #2 $finish;
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(&err, "this range bound is `x`"),
        1,
        "expected one x bound: {err}"
    );
}

// ---- round D: lens F2 / C-W1 (the cause follows the x bits that survive) ----

/// `(X % 0) & 16'h0000` is absorbed (§11.4.8), so the x is the `/`'s (§11.4.3): the cause names
/// the `/`.  PRE `c=0003` ; iverilog, sv2v, verilator `xxxx`.
#[test]
fn refuses_a1_absorbed_lhs_names_the_live_div() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] CV = (((16'd1 % ('1 + 1'b1)) & 16'h0000) | ((16'd7 / ('1 + 1'b1)) + 16'd3));
  initial begin #1 $display("V c=%h", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(&err, "parameter `CV` value is `x`: the divisor of the `/` (the `+` operation) is 0 in its 16-bit context, so the `/` is `x`"),
        1,
        "expected the `/`: {err}"
    );
}

/// Lens C-W1's headline: the `%` term is cleared by `& 16'h0`, the `/` term is x.  PRE `P=1` ;
/// iverilog, sv2v, verilator `P=x`.
#[test]
fn refuses_w1_and0_or_live_div_names_the_div() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] P = ((16'd1 % (16'hFFFF + 16'd1)) & 16'h0) | ((16'hFFFF + 16'd1) / (16'hFFFF + 16'd1));
  initial begin #1 $display("V P=%0d h=%h b=%0d", P, P, $bits(P)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(&err, "so the `/` is `x`"),
        1,
        "expected the `/`: {err}"
    );
    assert!(
        !err.contains("the `%` is `x`"),
        "named the absorbed `%`: {err}"
    );
}

/// The control: the live `/` on the LEFT, the absorbed `%` on the right — the `/` either way.
/// PRE `P=1` ; iverilog, sv2v, verilator `P=x`.
#[test]
fn refuses_w1_live_lhs_ctrl_names_the_div() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] P = ((16'hFFFF + 16'd1) / (16'hFFFF + 16'd1)) | ((16'd1 % (16'hFFFF + 16'd1)) & 16'h0);
  initial begin #1 $display("V P=%0d h=%h b=%0d", P, P, $bits(P)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(&err, "so the `/` is `x`"),
        1,
        "expected the `/`: {err}"
    );
}

/// Both terms live: the left origin's x survives the `|`, so it is a true cause and is named.
/// PRE `P=1` ; iverilog, sv2v, verilator `P=x`.
#[test]
fn refuses_w1_both_live_names_the_lhs_mod() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] P = (16'd1 % (16'hFFFF + 16'd1)) | ((16'hFFFF + 16'd1) / (16'hFFFF + 16'd1));
  initial begin #1 $display("V P=%0d h=%h b=%0d", P, P, $bits(P)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(&err, "so the `%` is `x`"),
        1,
        "expected the `%`: {err}"
    );
}

/// The bound twin of the headline: `+ 16'd3` after the `|`.  PRE 5 bits ; iverilog 1 bit ;
/// verilator "left side of bit range isn't a two-state constant" (§6.9.1).
#[test]
fn refuses_w1b_bound_absorbed_lhs_names_the_div() {
    let (out, err) = run_full(
        r#"module top;
  logic [(((16'd1 % (16'hFFFF + 16'd1)) & 16'h0) | ((16'hFFFF + 16'd1) / (16'hFFFF + 16'd1))) + 16'd3:0] v;
  initial begin #1 $display("V b=%0d", $bits(v)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(&err, "this range bound is `x`: the divisor of the `/` (the `+` operation) is 0 in its 16-bit context"),
        1,
        "expected the `/`: {err}"
    );
}

/// An absorbed `**` beside a live `%`: the `%` is named, not the `**`.  PRE `P=1` ; iverilog,
/// sv2v, verilator `P=x`.
#[test]
fn refuses_w1_pow_absorbed_names_the_live_mod() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] P = (((16'hFFFF + 16'd1) ** -1) & 16'h0) | (16'd1 % (16'hFFFF + 16'd1));
  initial begin #1 $display("V P=%0d h=%h b=%0d", P, P, $bits(P)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(&err, "so the `%` is `x`"),
        1,
        "expected the `%`: {err}"
    );
    assert!(!err.contains("`**`"), "named the absorbed `**`: {err}");
}

/// TRUNCATION: the `%` x lives only in [15:8], the 8-bit target drops it, and the `/` x in
/// [7:0] is what the binder refused on.  The one origin kept per node is the `%` (it owns a
/// bit of the `|`), and the target's truncation leaves it nothing — so the clause names no
/// origin rather than the wrong one.  PRE `P=1` ; iverilog, sv2v, verilator `P=x h=xx`.
#[test]
fn refuses_t1_truncated_origin_names_no_origin() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [7:0] P = ((16'd1 % (16'hFFFF + 16'd1)) & 16'hFF00) | (((16'hFFFF + 16'd1) / (16'hFFFF + 16'd1)) & 16'h00FF);
  initial begin #1 $display("V P=%0d h=%h b=%0d", P, P, $bits(P)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(&err, "parameter `P` value is `x`: an operation in it is `x` in its 16-bit context (a divisor that is 0, or a `**` base that is 0 under a negative exponent: IEEE 1800-2017 §11.4.3, Table 11-4), and a parameter has no x/z plane"),
        1,
        "expected the clause without an origin: {err}"
    );
    assert!(
        !err.contains("the `%` is `x`"),
        "named the truncated `%`: {err}"
    );
}

/// The 32-bit truncation spelling (lens-sound wc4b): the `%` x is in [31:16], the target is 16
/// bits.  PRE `c=7` ; iverilog, sv2v, verilator `x`.
#[test]
fn refuses_wc4b_truncated_origin_names_no_origin() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] C = (((32'd1 % ('1 + 1'b1)) & 32'hFFFF0000) | ((32'd7 - (32'd7 / ('1 + 1'b1))) & 32'h0000FFFF));
  initial begin #1 $display("V c=%0d", C); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(
            &err,
            "value is `x`: an operation in it is `x` in its 32-bit context"
        ),
        1,
        "expected the clause without an origin: {err}"
    );
    assert!(
        !err.contains("the `%` is `x`"),
        "named the truncated `%`: {err}"
    );
}

// ---- round D: lens F3 / C-W2 (an untaken arm is never the cause) ----

/// rhs-side untaken arm, `%` by a literal 0.  PRE `P=2` ; iverilog, sv2v, verilator `P=x`.
#[test]
fn refuses_w2_untaken_rhs_mod_lit0_names_the_taken_mod() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] P = ((16'd1 % (16'hFFFF + 16'd1)) + 16'd1) + (1'b1 ? 16'd0 : (16'd1 % 16'd0));
  initial begin #1 $display("V P=%0d h=%h b=%0d", P, P, $bits(P)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(e3009_with(&err, "parameter `P` value is `x`: the divisor of the `%` (the `+` operation) is 0 in its 16-bit context"), 1, "expected the taken `%`: {err}");
    assert!(
        !err.contains("divides by zero"),
        "named the untaken arm: {err}"
    );
}

/// rhs-side untaken arm, `/` by a literal 0.  PRE `P=2` ; iverilog, sv2v, verilator `P=x`.
#[test]
fn refuses_w2_untaken_rhs_div_lit0_names_the_taken_mod() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] P = ((16'd1 % (16'hFFFF + 16'd1)) + 16'd1) + (1'b1 ? 16'd0 : (16'd1 / 16'd0));
  initial begin #1 $display("V P=%0d h=%h b=%0d", P, P, $bits(P)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(&err, "so the `%` is `x`"),
        1,
        "expected the taken `%`: {err}"
    );
    assert!(
        !err.contains("divides by zero"),
        "named the untaken arm: {err}"
    );
}

/// lhs-side untaken arm (`1'b0 ? (1 / 0) : ..`).  PRE `P=2` ; iverilog, sv2v, verilator `P=x`.
#[test]
fn refuses_w2_untaken_lhs_div_names_the_taken_mod() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] P = 1'b0 ? (16'd1 / 16'd0) : ((16'd1 % (16'hFFFF + 16'd1)) + 16'd1);
  initial begin #1 $display("V P=%0d h=%h b=%0d", P, P, $bits(P)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(&err, "so the `%` is `x`"),
        1,
        "expected the taken `%`: {err}"
    );
    assert!(
        !err.contains("divides by zero"),
        "named the untaken arm: {err}"
    );
}

/// A nested ternary whose inner TAKEN arm is inside an outer untaken one.  PRE `P=2` ;
/// iverilog, sv2v, verilator `P=x`.
#[test]
fn refuses_w2_untaken_nested_names_the_taken_mod() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] P = ((16'd1 % (16'hFFFF + 16'd1)) + 16'd1) | (1'b0 ? (1'b1 ? (16'd1 / 16'd0) : 16'd0) : 16'd0);
  initial begin #1 $display("V P=%0d h=%h b=%0d", P, P, $bits(P)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(&err, "so the `%` is `x`"),
        1,
        "expected the taken `%`: {err}"
    );
    assert!(
        !err.contains("divides by zero"),
        "named the untaken arm: {err}"
    );
}

/// An untaken `**` with no constant-fold arm (`16'd0 ** -1`).  PRE `P=2` ; iverilog, sv2v,
/// verilator `P=x`.
#[test]
fn refuses_w2_untaken_pow_names_the_taken_mod() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] P = ((16'd1 % (16'hFFFF + 16'd1)) + 16'd1) + (1'b1 ? 16'd0 : (16'd0 ** -1));
  initial begin #1 $display("V P=%0d h=%h b=%0d", P, P, $bits(P)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(&err, "so the `%` is `x`"),
        1,
        "expected the taken `%`: {err}"
    );
    assert!(
        !err.contains("no constant-fold arm"),
        "named the untaken `**`: {err}"
    );
}

/// The generate-scope binder (lens-sound wc3).  PRE `c=2` ; iverilog, sv2v, verilator `x`.
#[test]
fn refuses_wc3_generate_untaken_names_the_taken_mod() {
    let (out, err) = run_full(
        r#"module top;
  if (1) begin : g
    localparam logic [15:0] C = 1'b1 ? ((16'd1 % ('1 + 1'b1)) + 16'd1) : (1 / 0);
  end
  initial begin #1 $display("V c=%0d", g.C); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(&err, "generate-scope parameter `C` value is `x`: the divisor of the `%` (the `+` operation) is 0 in its 32-bit context"),
        1,
        "expected the taken `%`: {err}"
    );
    assert!(
        !err.contains("divides by zero"),
        "named the untaken arm: {err}"
    );
}

/// A NAMED zero divisor in the untaken arm (lens-sound wc5, `16'd1 / Z`, Z = 0).  PRE `c=2` ;
/// iverilog, sv2v, verilator `x`.
#[test]
fn refuses_wc5_untaken_named_names_the_taken_mod() {
    let (out, err) = run_full(
        r#"module top;
  localparam int Z = 0;
  localparam logic [15:0] C = 1'b1 ? ((16'd1 % ('1 + 1'b1)) + 16'd1) : (16'd1 / Z);
  initial begin #1 $display("V c=%0d", C); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(&err, "parameter `C` value is `x`: the divisor of the `%`"),
        1,
        "expected the taken `%`: {err}"
    );
    assert!(
        !err.contains("divides by zero"),
        "named the untaken arm: {err}"
    );
}

/// The package twin (`package.rs`) never asked `unfoldable_reason`, so it named the taken arm
/// already; pinned with the reorder.  PRE `P=2` ; iverilog, verilator `P=x`.
#[test]
fn refuses_w2_package_untaken_names_the_taken_mod() {
    let (out, err) = run_full(
        r#"package p;
  localparam logic [15:0] P = ((16'd1 % (16'hFFFF + 16'd1)) + 16'd1) + (1'b1 ? 16'd0 : (16'd1 % 16'd0));
endpackage
module top;
  initial begin #1 $display("V P=%0d h=%h", p::P, p::P); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(&err, "package parameter `P` value is not a foldable constant: its value is `x`: the divisor of the `%` (the `+` operation) is 0 in its 16-bit context"),
        1,
        "expected the taken `%`: {err}"
    );
}

// ---- round D: lens F4 (M46, `origin_width`) ----

/// A 65-bit bound region: the origin is decided only at 1..=64 bits, so this keeps PRE's 3 bits
/// (= iverilog, sv2v, verilator: the divisor `'1 - 64'hFFFF_FFFF_FFFF_FFFF` is 1 at 65 bits, not
/// 0).  At ctx 65 the walk's mask is all-ones and the 64-bit literal leaf can read -1, so an
/// origin decided at ANY width would refuse a right value here (M46's killer).
#[test]
fn keeps_m46d_bound65_guard() {
    let (out, err) = run_full(
        r#"module top;
  logic [((65'd5 / ('1 - 64'hFFFF_FFFF_FFFF_FFFF)) + 2) : 0] d;
  initial begin #1 $display("V bitsd=%0d", $bits(d)); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V bitsd=3"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// KNOWN-WRONG (Q-DECLSELBOUND): the 65-bit twin with a fill — `'1 + 1'b1` is 0 at 65 bits (the
/// bound is x: iverilog 1 bit, verilator "isn't a two-state constant"), but the region is past
/// the walk's 64-bit origin width, so PRE's 3 bits stay (the unlimited walk sizes `'1` at 32).
/// PRE 3 ; sv2v 3.
#[test]
fn keeps_m46a_bound65_fill_known_wrong() {
    let (out, err) = run_full(
        r#"module top;
  logic [((65'd5 / ('1 + 1'b1)) + 2) : 0] d;
  initial begin #1 $display("V bitsd=%0d", $bits(d)); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V bitsd=3"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// KNOWN-WRONG (c1 declines; M27's killer): `$rtoi(2.7)` is a call over a real literal, which
/// c1 (every node the const domain descends into has a known self width) does not certify, so
/// the `x` of the `%` is not believed and the fallback's 4 stays.  The `int` target holds 0
/// under IEEE 1800-2017 §6.11.2 (x -> 0): iverilog 0, verilator x ; sv2v does not parse it.
/// PRE 4.  Dropping c1 refuses it (lens-sound b1b) — a wrong -> loud move, which is why c1 is a
/// soundness guard kept by RULINGS-plan open 4, not a gap this row closes.
#[test]
fn keeps_b1b_rtoi_real_literal_known_wrong() {
    let (out, err) = run_full(
        r#"module top;
  localparam int CV = ((16'd1 % ('1 + 1'b1)) + 1) + $rtoi(2.7);
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with("V ")).collect();
    assert_eq!(got, ["V c=4"], "value moved: {out} {err}");
    assert!(!err.contains("E3009"), "refused: {err}");
}

/// A shift moves an origin's own bits with the x bits: `(X % 0) << 16` shifts every x bit of
/// the `%` out of the 16-bit region, so the `+`'s x is the `/`'s (lens-sound w2).  PRE `c=3` ;
/// iverilog, sv2v, verilator `x`.
#[test]
fn refuses_w2_shifted_out_origin_names_the_div() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] CV = (((16'd1 % ('1 + 1'b1)) << 16) + (16'd7 / ('1 + 1'b1))) + 16'd3;
  initial begin #1 $display("V c=%0d", CV); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(&err, "so the `/` is `x`"),
        1,
        "expected the `/`: {err}"
    );
}

/// An arithmetic node whose x comes from an origin no operand still names: the `|` keeps the
/// `%` (it owns [15:8]), the `& 16'h00FF` clears those bits — so nothing owned is left — and
/// the `+` is all-x from the unnamed `/`.  The clause names no origin; an arithmetic node never
/// hands every bit to an origin that owns none.  PRE `P=1` ; iverilog, sv2v, verilator `P=x`.
#[test]
fn refuses_m52_unowned_live_x_names_no_origin() {
    let (out, err) = run_full(
        r#"module top;
  localparam logic [15:0] P = ((((16'd1 % (16'hFFFF + 16'd1)) & 16'hFF00) | (((16'hFFFF + 16'd1) / (16'hFFFF + 16'd1)) & 16'h00FF)) & 16'h00FF) + 16'd0;
  initial begin #1 $display("V P=%0d h=%h b=%0d", P, P, $bits(P)); $finish; end
endmodule
"#,
    );
    assert!(!out.contains("V "), "a value was bound: {out}");
    assert_eq!(
        e3009_with(
            &err,
            "value is `x`: an operation in it is `x` in its 16-bit context"
        ),
        1,
        "expected the clause without an origin: {err}"
    );
}
