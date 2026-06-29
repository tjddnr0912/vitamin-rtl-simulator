//! Instance arrays (`buf1 u[3:0](...)`, IEEE 1364-2005 §12.1.2-3) — the
//! Phase-1.x precision item unlocked from the v1 loud-reject. Unroll rule
//! (iverilog-pinned live, 2026-06-12): a connection of width P (the port
//! width) fans out to EVERY instance; width N*P slices MSB-first in DECLARED
//! range order (the leftmost/first-named index gets the most significant
//! chunk — both for `u[3:0]` and `u[0:3]`); any other width is a loud
//! elaboration error.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ia_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

#[test]
fn descending_range_slices_msb_first() {
    // iverilog: y=1010 with u[3] driven by d[3].
    let (out, err, code) = run("module buf1(input wire i, output wire o);\n\
         assign o = i;\n\
         endmodule\n\
         module top;\n\
         reg [3:0] d; wire [3:0] y;\n\
         buf1 u[3:0] (.i(d), .o(y));\n\
         initial begin d = 4'b1010; #1 $display(\"y=%b\", y); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("y=1010"), "got:\n{out}");
}

#[test]
fn ascending_range_first_named_index_gets_msb() {
    // iverilog: u[0:3] also yields y=1010 — slicing follows DECLARED order,
    // u[0] (first named) gets the MSB chunk.
    let (out, err, code) = run("module buf1(input wire i, output wire o);\n\
         assign o = i;\n\
         endmodule\n\
         module top;\n\
         reg [3:0] d; wire [3:0] y;\n\
         buf1 u[0:3] (.i(d), .o(y));\n\
         initial begin d = 4'b1010; #1 $display(\"y=%b\", y); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("y=1010"), "got:\n{out}");
}

#[test]
fn multi_bit_ports_slice_in_chunks() {
    // 2-bit ports × 4 instances over an 8-bit connection (iverilog: 00111010).
    let (out, err, code) = run(
        "module w2 #(parameter W=2) (input wire [W-1:0] a, output wire [W-1:0] z);\n\
         assign z = ~a;\n\
         endmodule\n\
         module top;\n\
         reg [7:0] d; wire [7:0] y;\n\
         w2 u[3:0] (.a(d), .z(y));\n\
         initial begin d = 8'b1100_0101; #1 $display(\"y=%b\", y); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("y=00111010"), "got:\n{out}");
}

#[test]
fn port_width_connection_fans_out_to_all() {
    // A conn of exactly the port width (clk, 1-bit) is shared by every
    // instance; the data conns slice. Classic flop-row shape.
    let (out, err, code) = run("module ff(input wire c, input wire d, output reg q);\n\
         always @(posedge c) q <= d;\n\
         endmodule\n\
         module top;\n\
         reg clk; reg [3:0] d; wire [3:0] q;\n\
         ff u[3:0] (.c(clk), .d(d), .q(q));\n\
         initial begin\n\
           clk = 0; d = 4'b0110;\n\
           #1 clk = 1; #1 clk = 0;\n\
           $display(\"q=%b\", q); $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("q=0110"), "got:\n{out}");
}

#[test]
fn positional_connections_slice_too() {
    let (out, err, code) = run("module buf1(input wire i, output wire o);\n\
         assign o = i;\n\
         endmodule\n\
         module top;\n\
         reg [1:0] d; wire [1:0] y;\n\
         buf1 u[1:0] (d, y);\n\
         initial begin d = 2'b10; #1 $display(\"y=%b\", y); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("y=10"), "got:\n{out}");
}

#[test]
fn param_override_resolves_port_width_before_slicing() {
    // W overridden to 4: two instances over an 8-bit conn = 4-bit chunks.
    let (out, err, code) = run(
        "module inv #(parameter W=1) (input wire [W-1:0] a, output wire [W-1:0] z);\n\
         assign z = ~a;\n\
         endmodule\n\
         module top;\n\
         reg [7:0] d; wire [7:0] y;\n\
         inv #(.W(4)) u[1:0] (.a(d), .z(y));\n\
         initial begin d = 8'b1111_0000; #1 $display(\"y=%b\", y); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("y=00001111"), "got:\n{out}");
}

#[test]
fn mismatched_width_is_loud() {
    // 3 bits is neither P (1) nor N*P (4) — iverilog: elaboration error.
    let (_, err, code) = run("module buf1(input wire i, output wire o);\n\
         assign o = i;\n\
         endmodule\n\
         module top;\n\
         reg [2:0] d; wire [3:0] y;\n\
         buf1 u[3:0] (.i(d), .o(y));\n\
         initial begin d = 3'b101; #1 $display(\"y=%b\", y); $finish; end\n\
         endmodule\n");
    assert_ne!(code, Some(0), "must reject; stderr:\n{err}");
    assert!(
        err.contains("width") || err.contains("VITA-E3009"),
        "loud width diagnostic expected:\n{err}"
    );
}

#[test]
fn non_ansi_child_array_stays_loud() {
    // v1 cut: slicing needs ANSI port widths; a non-ANSI child keeps the
    // loud reject rather than guessing.
    let (_, err, code) = run("module buf1(i, o);\n\
         input i; output o;\n\
         assign o = i;\n\
         endmodule\n\
         module top;\n\
         reg [1:0] d; wire [1:0] y;\n\
         buf1 u[1:0] (.i(d), .o(y));\n\
         initial begin d = 2'b10; #1 $display(\"y=%b\", y); $finish; end\n\
         endmodule\n");
    assert_ne!(code, Some(0), "non-ANSI array stays loud; stderr:\n{err}");
    assert!(err.contains("VITA-E3009"), "E3009 expected:\n{err}");
}

#[test]
fn non_constant_range_is_loud() {
    // (A `[N-1:0]` with N=0 is NOT this case: `[-1:0]` is a LEGAL 2-element
    // range — negative indices are fine — and correctly fails later as a
    // multidriver. The loud lane is a range that does not const-fold.)
    let (_, err, code) = run("module buf1(input wire i, output wire o);\n\
         assign o = i;\n\
         endmodule\n\
         module top;\n\
         reg [1:0] w;\n\
         reg d; wire y;\n\
         buf1 u[w:0] (.i(d), .o(y));\n\
         initial begin d = 1; #1 $display(\"y=%b\", y); $finish; end\n\
         endmodule\n");
    assert_ne!(
        code,
        Some(0),
        "non-constant range stays loud; stderr:\n{err}"
    );
    assert!(err.contains("VITA-E3009"), "E3009 expected:\n{err}");
}
