//! §4.5.418 census (part 2 of 11): a multi-dimensional packed tf-port formal
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
    let d = std::env::temp_dir().join(format!("vita_mdformal2_{}_{n}", std::process::id()));
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
fn c_l8_neg8d3() {
    // c_l8_neg8d3: stays loud (declined shape)
    loud(
        "c_l8_neg8d3",
        r#"module tb;
  localparam logic [7:0] X = -8'd3;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_l8_s16sh8000: stays loud (both oracles refuse it too)
    loud(
        "c_l8_s16sh8000",
        r#"module tb;
  localparam logic [7:0] X = 16'sh8000;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_l8_s32d12: stays loud (declined shape)
    loud(
        "c_l8_s32d12",
        r#"module tb;
  localparam logic [7:0] X = 32'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_l8_s32sd5: stays loud (declined shape)
    loud(
        "c_l8_s32sd5",
        r#"module tb;
  localparam logic [7:0] X = 32'sd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_l8_s33d9() {
    // c_l8_s33d9: stays loud (declined shape)
    loud(
        "c_l8_s33d9",
        r#"module tb;
  localparam logic [7:0] X = 33'd9;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_l8_s40d5: stays loud (declined shape)
    loud(
        "c_l8_s40d5",
        r#"module tb;
  localparam logic [7:0] X = 40'd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_l8_s4d20: stays loud (declined shape)
    loud(
        "c_l8_s4d20",
        r#"module tb;
  localparam logic [7:0] X = 4'd20;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_l8_s4sd12: stays loud (declined shape)
    loud(
        "c_l8_s4sd12",
        r#"module tb;
  localparam logic [7:0] X = 4'sd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_l8_s4x() {
    // c_l8_s4x: stays loud (declined shape)
    loud(
        "c_l8_s4x",
        r#"module tb;
  localparam logic [7:0] X = 4'b1x00;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_l8_s64all: stays loud (declined shape)
    loud(
        "c_l8_s64all",
        r#"module tb;
  localparam logic [7:0] X = 64'hffff_ffff_ffff_ffff;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_l8_s64d5: stays loud (declined shape)
    loud(
        "c_l8_s64d5",
        r#"module tb;
  localparam logic [7:0] X = 64'd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_l8_s8d12: stays loud (declined shape)
    loud(
        "c_l8_s8d12",
        r#"module tb;
  localparam logic [7:0] X = 8'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_l8_s8hff() {
    // c_l8_s8hff: stays loud (declined shape)
    loud(
        "c_l8_s8hff",
        r#"module tb;
  localparam logic [7:0] X = 8'hff;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_l8_sum: stays loud (declined shape)
    loud(
        "c_l8_sum",
        r#"module tb;
  localparam logic [7:0] X = 32'd12 + 1;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_l8_u_d12: stays loud (declined shape)
    loud(
        "c_l8_u_d12",
        r#"module tb;
  localparam logic [7:0] X = 'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_longint_neg8d3: stays loud (declined shape)
    loud(
        "c_longint_neg8d3",
        r#"module tb;
  localparam longint X = -8'd3;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_longint_s16sh8000() {
    // c_longint_s16sh8000: iverilog (the other refuses the shape)
    digest(
        "c_longint_s16sh8000",
        r#"module tb;
  localparam longint X = 16'sh8000;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "-32768 32771 7ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    );
    // c_longint_s32d12: both oracles
    digest(
        "c_longint_s32d12",
        r#"module tb;
  localparam longint X = 32'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "12 13 1fff",
    );
    // c_longint_s32sd5: both oracles
    digest(
        "c_longint_s32sd5",
        r#"module tb;
  localparam longint X = 32'sd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "5 6 3f",
    );
    // c_longint_s33d9: both oracles
    digest(
        "c_longint_s33d9",
        r#"module tb;
  localparam longint X = 33'd9;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "9 10 3ff",
    );
}

#[test]
fn c_longint_s40d5() {
    // c_longint_s40d5: both oracles
    digest(
        "c_longint_s40d5",
        r#"module tb;
  localparam longint X = 40'd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "5 6 3f",
    );
    // c_longint_s4d20: stays loud (declined shape)
    loud(
        "c_longint_s4d20",
        r#"module tb;
  localparam longint X = 4'd20;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_longint_s4sd12: iverilog (the other refuses the shape)
    digest(
        "c_longint_s4sd12",
        r#"module tb;
  localparam longint X = 4'sd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "-4 7 7f",
    );
    // c_longint_s4x: stays loud (declined shape)
    loud(
        "c_longint_s4x",
        r#"module tb;
  localparam longint X = 4'b1x00;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_longint_s64all() {
    // c_longint_s64all: stays loud (declined shape)
    loud(
        "c_longint_s64all",
        r#"module tb;
  localparam longint X = 64'hffff_ffff_ffff_ffff;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_longint_s64d5: both oracles
    digest(
        "c_longint_s64d5",
        r#"module tb;
  localparam longint X = 64'd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "5 6 3f",
    );
    // c_longint_s8d12: both oracles
    digest(
        "c_longint_s8d12",
        r#"module tb;
  localparam longint X = 8'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "12 13 1fff",
    );
    // c_longint_s8hff: both oracles
    digest(
        "c_longint_s8hff",
        r#"module tb;
  localparam longint X = 8'hff;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "255 256 ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    );
}

#[test]
fn c_longint_sum() {
    // c_longint_sum: stays loud (declined shape)
    loud(
        "c_longint_sum",
        r#"module tb;
  localparam longint X = 32'd12 + 1;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_longint_u_d12: both oracles
    digest(
        "c_longint_u_d12",
        r#"module tb;
  localparam longint X = 'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "12 13 1fff",
    );
    // c_narrow_sum_new: stays loud (declined shape)
    loud(
        "c_narrow_sum_new",
        r#"module tb;
  localparam logic [3:0] C = 4'hf;
  localparam logic [3:0] D = 4'h1;
  typedef struct packed { logic [C+D:0] m; } t;
  t v;
  initial begin $display("DIGEST=%0d", $bits(v)); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_pkg_uint: both oracles
    digest(
        "c_pkg_uint",
        r#"package q;
  parameter int unsigned W = 32'd12;
  typedef struct packed { logic [W-1:0] req; } t;
endpackage
module tb;
  import q::*;
  t v;
  initial begin v = '1; $display("DIGEST=%0d %h", $bits(v), v); #1 $finish; end
endmodule
"#,
        "12 fff",
    );
}

#[test]
fn c_shadow_decl() {
    // c_shadow_decl: both oracles
    digest(
        "c_shadow_decl",
        r#"module tb;
  localparam int W = 32'd6;
  typedef struct packed { logic [W-1:0] a; } s_t;
  initial begin : b
    int W;
    s_t s;
    W = 2; s = '1;
    $display("DIGEST=%0d %0d", W, $bits(s));
  end
  initial #1 $finish;
endmodule
"#,
        "2 6",
    );
    // c_shortint_neg8d3: stays loud (declined shape)
    loud(
        "c_shortint_neg8d3",
        r#"module tb;
  localparam shortint X = -8'd3;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_shortint_s16sh8000: stays loud (declined shape)
    loud(
        "c_shortint_s16sh8000",
        r#"module tb;
  localparam shortint X = 16'sh8000;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_shortint_s32d12: stays loud (declined shape)
    loud(
        "c_shortint_s32d12",
        r#"module tb;
  localparam shortint X = 32'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_shortint_s32sd5() {
    // c_shortint_s32sd5: stays loud (declined shape)
    loud(
        "c_shortint_s32sd5",
        r#"module tb;
  localparam shortint X = 32'sd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_shortint_s33d9: stays loud (declined shape)
    loud(
        "c_shortint_s33d9",
        r#"module tb;
  localparam shortint X = 33'd9;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_shortint_s40d5: stays loud (declined shape)
    loud(
        "c_shortint_s40d5",
        r#"module tb;
  localparam shortint X = 40'd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_shortint_s4d20: stays loud (declined shape)
    loud(
        "c_shortint_s4d20",
        r#"module tb;
  localparam shortint X = 4'd20;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_shortint_s4sd12() {
    // c_shortint_s4sd12: stays loud (declined shape)
    loud(
        "c_shortint_s4sd12",
        r#"module tb;
  localparam shortint X = 4'sd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_shortint_s4x: stays loud (declined shape)
    loud(
        "c_shortint_s4x",
        r#"module tb;
  localparam shortint X = 4'b1x00;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_shortint_s64all: stays loud (declined shape)
    loud(
        "c_shortint_s64all",
        r#"module tb;
  localparam shortint X = 64'hffff_ffff_ffff_ffff;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_shortint_s64d5: stays loud (declined shape)
    loud(
        "c_shortint_s64d5",
        r#"module tb;
  localparam shortint X = 64'd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_shortint_s8d12() {
    // c_shortint_s8d12: stays loud (declined shape)
    loud(
        "c_shortint_s8d12",
        r#"module tb;
  localparam shortint X = 8'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_shortint_s8hff: stays loud (declined shape)
    loud(
        "c_shortint_s8hff",
        r#"module tb;
  localparam shortint X = 8'hff;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_shortint_sum: stays loud (declined shape)
    loud(
        "c_shortint_sum",
        r#"module tb;
  localparam shortint X = 32'd12 + 1;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_shortint_u_d12: stays loud (declined shape)
    loud(
        "c_shortint_u_d12",
        r#"module tb;
  localparam shortint X = 'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_sum_typed() {
    // c_sum_typed: both oracles
    digest(
        "c_sum_typed",
        r#"module tb;
  localparam int A = 32'd3;
  localparam int B = 32'd4;
  typedef struct packed { logic [A+B:0] a; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d", $bits(s)); #1 $finish; end
endmodule
"#,
        "8",
    );
    // c_uint_neg8d3: stays loud (both oracles refuse it too)
    loud(
        "c_uint_neg8d3",
        r#"module tb;
  localparam int unsigned X = -8'd3;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_uint_s16sh8000: stays loud (both oracles refuse it too)
    loud(
        "c_uint_s16sh8000",
        r#"module tb;
  localparam int unsigned X = 16'sh8000;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_uint_s32d12: both oracles
    digest(
        "c_uint_s32d12",
        r#"module tb;
  localparam int unsigned X = 32'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "12 13 1fff",
    );
}

#[test]
fn c_uint_s32sd5() {
    // c_uint_s32sd5: both oracles
    digest(
        "c_uint_s32sd5",
        r#"module tb;
  localparam int unsigned X = 32'sd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "5 6 3f",
    );
    // c_uint_s33d9: both oracles
    digest(
        "c_uint_s33d9",
        r#"module tb;
  localparam int unsigned X = 33'd9;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "9 10 3ff",
    );
    // c_uint_s40d5: both oracles
    digest(
        "c_uint_s40d5",
        r#"module tb;
  localparam int unsigned X = 40'd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "5 6 3f",
    );
    // c_uint_s4d20: stays loud (declined shape)
    loud(
        "c_uint_s4d20",
        r#"module tb;
  localparam int unsigned X = 4'd20;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_uint_s4sd12() {
    // c_uint_s4sd12: stays loud (both oracles refuse it too)
    loud(
        "c_uint_s4sd12",
        r#"module tb;
  localparam int unsigned X = 4'sd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_uint_s4x: stays loud (declined shape)
    loud(
        "c_uint_s4x",
        r#"module tb;
  localparam int unsigned X = 4'b1x00;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_uint_s64all: stays loud (both oracles refuse it too)
    loud(
        "c_uint_s64all",
        r#"module tb;
  localparam int unsigned X = 64'hffff_ffff_ffff_ffff;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_uint_s64d5: both oracles
    digest(
        "c_uint_s64d5",
        r#"module tb;
  localparam int unsigned X = 64'd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "5 6 3f",
    );
}

#[test]
fn c_uint_s8d12() {
    // c_uint_s8d12: both oracles
    digest(
        "c_uint_s8d12",
        r#"module tb;
  localparam int unsigned X = 8'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "12 13 1fff",
    );
    // c_uint_s8hff: both oracles
    digest(
        "c_uint_s8hff",
        r#"module tb;
  localparam int unsigned X = 8'hff;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "255 256 ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    );
    // c_uint_sum: stays loud (declined shape)
    loud(
        "c_uint_sum",
        r#"module tb;
  localparam int unsigned X = 32'd12 + 1;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_uint_u_d12: both oracles
    digest(
        "c_uint_u_d12",
        r#"module tb;
  localparam int unsigned X = 'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "12 13 1fff",
    );
}
