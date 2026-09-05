//! §4.5.418 census (part 1 of 11): a multi-dimensional packed tf-port formal
//! (`logic [15:0][3:0] shifts`, inline / typedef / package typedef, ANSI / non-ANSI /
//! task / `ref`, output-inout-ref writes, continuation, default, shadowing) declared
//! flat and rewritten in the body, and a lone based literal (`32'd12`) as a package /
//! localparam value in the parse-time constant table (struct member width). Every value
//! pin is the line both oracles print (iverilog 13.0 `-g2012`, verilator 5.050); a cell
//! where the two split on 4-state (an output formal's unwritten elements, an out-of-range
//! read) carries iverilog's line and says so. `loud` pins keep a declined shape loud.
//! Control twins (`ctl_var` = the same body on a module variable, `ctl_par` = the
//! packed-md parameter spelling) are pinned beside their formal cells. Generated from
//! the census harness; regenerate rather than hand-edit.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_mdformal1_{}_{n}", std::process::id()));
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

/// Every `DIGEST=` line, in emission order, joined by `|` (the census harness format).
fn digest(name: &str, src: &str, expect: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{name}: expected exit 0, got {rc:?}:\n{out}");
    let v: Vec<&str> = out
        .lines()
        .filter_map(|l| l.strip_prefix("DIGEST="))
        .collect();
    assert_eq!(v.join("|"), expect, "{name}:\n{out}");
}

fn loud(name: &str, src: &str, needle: &str) {
    let (out, rc) = run(src);
    assert_ne!(rc, Some(0), "{name}: expected a loud reject:\n{out}");
    assert!(
        out.contains(needle),
        "{name}: expected `{needle}` in:\n{out}"
    );
}

#[test]
fn c_byte_neg8d3() {
    // c_byte_neg8d3: stays loud (declined shape)
    loud(
        "c_byte_neg8d3",
        r#"module tb;
  localparam byte X = -8'd3;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_byte_s16sh8000: stays loud (declined shape)
    loud(
        "c_byte_s16sh8000",
        r#"module tb;
  localparam byte X = 16'sh8000;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_byte_s32d12: stays loud (declined shape)
    loud(
        "c_byte_s32d12",
        r#"module tb;
  localparam byte X = 32'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_byte_s32sd5: stays loud (declined shape)
    loud(
        "c_byte_s32sd5",
        r#"module tb;
  localparam byte X = 32'sd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_byte_s33d9() {
    // c_byte_s33d9: stays loud (declined shape)
    loud(
        "c_byte_s33d9",
        r#"module tb;
  localparam byte X = 33'd9;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_byte_s40d5: stays loud (declined shape)
    loud(
        "c_byte_s40d5",
        r#"module tb;
  localparam byte X = 40'd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_byte_s4d20: stays loud (declined shape)
    loud(
        "c_byte_s4d20",
        r#"module tb;
  localparam byte X = 4'd20;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_byte_s4sd12: stays loud (declined shape)
    loud(
        "c_byte_s4sd12",
        r#"module tb;
  localparam byte X = 4'sd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_byte_s4x() {
    // c_byte_s4x: stays loud (declined shape)
    loud(
        "c_byte_s4x",
        r#"module tb;
  localparam byte X = 4'b1x00;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_byte_s64all: stays loud (declined shape)
    loud(
        "c_byte_s64all",
        r#"module tb;
  localparam byte X = 64'hffff_ffff_ffff_ffff;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_byte_s64d5: stays loud (declined shape)
    loud(
        "c_byte_s64d5",
        r#"module tb;
  localparam byte X = 64'd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_byte_s8d12: stays loud (declined shape)
    loud(
        "c_byte_s8d12",
        r#"module tb;
  localparam byte X = 8'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_byte_s8hff() {
    // c_byte_s8hff: stays loud (declined shape)
    loud(
        "c_byte_s8hff",
        r#"module tb;
  localparam byte X = 8'hff;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_byte_sum: stays loud (declined shape)
    loud(
        "c_byte_sum",
        r#"module tb;
  localparam byte X = 32'd12 + 1;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_byte_u_d12: stays loud (declined shape)
    loud(
        "c_byte_u_d12",
        r#"module tb;
  localparam byte X = 'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_cast_bound: both oracles
    digest(
        "c_cast_bound",
        r#"module tb;
  localparam int W = 32'd6;
  typedef logic [W-1:0] w_t;
  initial begin $display("DIGEST=%h", w_t'(8'hff)); #1 $finish; end
endmodule
"#,
        "3f",
    );
}

#[test]
fn c_gen_index() {
    // c_gen_index: both oracles
    digest(
        "c_gen_index",
        r#"module tb;
  localparam int I = 32'd1;
  genvar g;
  generate for (g = 0; g < 2; g++) begin : blk
    logic [3:0] x;
    initial x = 4'd5 + g;
  end endgenerate
  initial begin #1 $display("DIGEST=%0d", blk[I].x); #1 $finish; end
endmodule
"#,
        "6",
    );
    // c_hdr_param: stays loud (declined shape)
    loud(
        "c_hdr_param",
        r#"module c #(parameter int W = 32'd12) ();
  typedef struct packed { logic [W-1:0] req; } t;
  t v;
  initial begin v = '1; $display("DIGEST=%0d %h", $bits(v), v); end
endmodule
module tb;
  c #(.W(32'd5)) u();
  initial #1 $finish;
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_hdr_param_body: both oracles
    digest(
        "c_hdr_param_body",
        r#"module c #(parameter int Z = 1) ();
  parameter int W = 32'd12;
  typedef struct packed { logic [W-1:0] req; } t;
  t v;
  initial begin v = '1; $display("DIGEST=%0d %h", $bits(v), v); end
endmodule
module tb;
  c #(.Z(2)) u();
  initial #1 $finish;
endmodule
"#,
        "12 fff",
    );
    // c_int_neg8d3: stays loud (declined shape)
    loud(
        "c_int_neg8d3",
        r#"module tb;
  localparam int X = -8'd3;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_int_s16sh8000() {
    // c_int_s16sh8000: both oracles
    digest(
        "c_int_s16sh8000",
        r#"module tb;
  localparam int X = 16'sh8000;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "-32768 32771 7ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    );
    // c_int_s32d12: both oracles
    digest(
        "c_int_s32d12",
        r#"module tb;
  localparam int X = 32'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "12 13 1fff",
    );
    // c_int_s32sd5: both oracles
    digest(
        "c_int_s32sd5",
        r#"module tb;
  localparam int X = 32'sd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "5 6 3f",
    );
    // c_int_s33d9: both oracles
    digest(
        "c_int_s33d9",
        r#"module tb;
  localparam int X = 33'd9;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "9 10 3ff",
    );
}

#[test]
fn c_int_s40d5() {
    // c_int_s40d5: both oracles
    digest(
        "c_int_s40d5",
        r#"module tb;
  localparam int X = 40'd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "5 6 3f",
    );
    // c_int_s4d20: stays loud (declined shape)
    loud(
        "c_int_s4d20",
        r#"module tb;
  localparam int X = 4'd20;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_int_s4sd12: both oracles
    digest(
        "c_int_s4sd12",
        r#"module tb;
  localparam int X = 4'sd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "-4 7 7f",
    );
    // c_int_s4x: stays loud (declined shape)
    loud(
        "c_int_s4x",
        r#"module tb;
  localparam int X = 4'b1x00;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_int_s64all() {
    // c_int_s64all: stays loud (declined shape)
    loud(
        "c_int_s64all",
        r#"module tb;
  localparam int X = 64'hffff_ffff_ffff_ffff;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_int_s64d5: both oracles
    digest(
        "c_int_s64d5",
        r#"module tb;
  localparam int X = 64'd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "5 6 3f",
    );
    // c_int_s8d12: both oracles
    digest(
        "c_int_s8d12",
        r#"module tb;
  localparam int X = 8'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "12 13 1fff",
    );
    // c_int_s8hff: both oracles
    digest(
        "c_int_s8hff",
        r#"module tb;
  localparam int X = 8'hff;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "255 256 ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    );
}

#[test]
fn c_int_sum() {
    // c_int_sum: stays loud (declined shape)
    loud(
        "c_int_sum",
        r#"module tb;
  localparam int X = 32'd12 + 1;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_int_u_d12: both oracles
    digest(
        "c_int_u_d12",
        r#"module tb;
  localparam int X = 'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "12 13 1fff",
    );
    // c_integer_neg8d3: stays loud (declined shape)
    loud(
        "c_integer_neg8d3",
        r#"module tb;
  localparam integer X = -8'd3;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_integer_s16sh8000: both oracles
    digest(
        "c_integer_s16sh8000",
        r#"module tb;
  localparam integer X = 16'sh8000;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "-32768 32771 7ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    );
}

#[test]
fn c_integer_s32d12() {
    // c_integer_s32d12: both oracles
    digest(
        "c_integer_s32d12",
        r#"module tb;
  localparam integer X = 32'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "12 13 1fff",
    );
    // c_integer_s32sd5: both oracles
    digest(
        "c_integer_s32sd5",
        r#"module tb;
  localparam integer X = 32'sd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "5 6 3f",
    );
    // c_integer_s33d9: both oracles
    digest(
        "c_integer_s33d9",
        r#"module tb;
  localparam integer X = 33'd9;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "9 10 3ff",
    );
    // c_integer_s40d5: both oracles
    digest(
        "c_integer_s40d5",
        r#"module tb;
  localparam integer X = 40'd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "5 6 3f",
    );
}

#[test]
fn c_integer_s4d20() {
    // c_integer_s4d20: stays loud (declined shape)
    loud(
        "c_integer_s4d20",
        r#"module tb;
  localparam integer X = 4'd20;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_integer_s4sd12: both oracles
    digest(
        "c_integer_s4sd12",
        r#"module tb;
  localparam integer X = 4'sd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "-4 7 7f",
    );
    // c_integer_s4x: stays loud (declined shape)
    loud(
        "c_integer_s4x",
        r#"module tb;
  localparam integer X = 4'b1x00;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_integer_s64all: stays loud (declined shape)
    loud(
        "c_integer_s64all",
        r#"module tb;
  localparam integer X = 64'hffff_ffff_ffff_ffff;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_integer_s64d5() {
    // c_integer_s64d5: both oracles
    digest(
        "c_integer_s64d5",
        r#"module tb;
  localparam integer X = 64'd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "5 6 3f",
    );
    // c_integer_s8d12: both oracles
    digest(
        "c_integer_s8d12",
        r#"module tb;
  localparam integer X = 8'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "12 13 1fff",
    );
    // c_integer_s8hff: both oracles
    digest(
        "c_integer_s8hff",
        r#"module tb;
  localparam integer X = 8'hff;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "255 256 ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    );
    // c_integer_sum: stays loud (declined shape)
    loud(
        "c_integer_sum",
        r#"module tb;
  localparam integer X = 32'd12 + 1;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_integer_u_d12() {
    // c_integer_u_d12: both oracles
    digest(
        "c_integer_u_d12",
        r#"module tb;
  localparam integer X = 'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "12 13 1fff",
    );
    // c_l40_neg8d3: stays loud (declined shape)
    loud(
        "c_l40_neg8d3",
        r#"module tb;
  localparam logic [39:0] X = -8'd3;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_l40_s16sh8000: stays loud (declined shape)
    loud(
        "c_l40_s16sh8000",
        r#"module tb;
  localparam logic [39:0] X = 16'sh8000;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_l40_s32d12: both oracles
    digest(
        "c_l40_s32d12",
        r#"module tb;
  localparam logic [39:0] X = 32'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "12 13 1fff",
    );
}

#[test]
fn c_l40_s32sd5() {
    // c_l40_s32sd5: both oracles
    digest(
        "c_l40_s32sd5",
        r#"module tb;
  localparam logic [39:0] X = 32'sd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "5 6 3f",
    );
    // c_l40_s33d9: both oracles
    digest(
        "c_l40_s33d9",
        r#"module tb;
  localparam logic [39:0] X = 33'd9;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "9 10 3ff",
    );
    // c_l40_s40d5: both oracles
    digest(
        "c_l40_s40d5",
        r#"module tb;
  localparam logic [39:0] X = 40'd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "5 6 3f",
    );
    // c_l40_s4d20: stays loud (declined shape)
    loud(
        "c_l40_s4d20",
        r#"module tb;
  localparam logic [39:0] X = 4'd20;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_l40_s4sd12() {
    // c_l40_s4sd12: stays loud (declined shape)
    loud(
        "c_l40_s4sd12",
        r#"module tb;
  localparam logic [39:0] X = 4'sd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_l40_s4x: stays loud (declined shape)
    loud(
        "c_l40_s4x",
        r#"module tb;
  localparam logic [39:0] X = 4'b1x00;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_l40_s64all: stays loud (declined shape)
    loud(
        "c_l40_s64all",
        r#"module tb;
  localparam logic [39:0] X = 64'hffff_ffff_ffff_ffff;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_l40_s64d5: both oracles
    digest(
        "c_l40_s64d5",
        r#"module tb;
  localparam logic [39:0] X = 64'd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "5 6 3f",
    );
}

#[test]
fn c_l40_s8d12() {
    // c_l40_s8d12: both oracles
    digest(
        "c_l40_s8d12",
        r#"module tb;
  localparam logic [39:0] X = 8'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "12 13 1fff",
    );
    // c_l40_s8hff: both oracles
    digest(
        "c_l40_s8hff",
        r#"module tb;
  localparam logic [39:0] X = 8'hff;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "255 256 ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    );
    // c_l40_sum: stays loud (declined shape)
    loud(
        "c_l40_sum",
        r#"module tb;
  localparam logic [39:0] X = 32'd12 + 1;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_l40_u_d12: both oracles
    digest(
        "c_l40_u_d12",
        r#"module tb;
  localparam logic [39:0] X = 'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "12 13 1fff",
    );
}
