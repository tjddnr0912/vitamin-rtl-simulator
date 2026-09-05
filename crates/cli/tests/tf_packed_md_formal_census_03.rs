//! §4.5.418 census (part 3 of 11): a multi-dimensional packed tf-port formal
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
    let d = std::env::temp_dir().join(format!("vita_mdformal3_{}_{n}", std::process::id()));
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
fn c_untyped_narrow_sum() {
    // c_untyped_narrow_sum: stays loud (declined shape)
    loud(
        "c_untyped_narrow_sum",
        r#"module tb;
  localparam C = 4'hf;
  localparam D = 4'h1;
  typedef struct packed { logic [C+D:0] m; } t;
  t v;
  initial begin $display("DIGEST=%0d", $bits(v)); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_untyped_neg8d3: stays loud (declined shape)
    loud(
        "c_untyped_neg8d3",
        r#"module tb;
  localparam  X = -8'd3;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_untyped_s16sh8000: stays loud (declined shape)
    loud(
        "c_untyped_s16sh8000",
        r#"module tb;
  localparam  X = 16'sh8000;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_untyped_s32d12: both oracles
    digest(
        "c_untyped_s32d12",
        r#"module tb;
  localparam  X = 32'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "12 13 1fff",
    );
}

#[test]
fn c_untyped_s32sd5() {
    // c_untyped_s32sd5: both oracles
    digest(
        "c_untyped_s32sd5",
        r#"module tb;
  localparam  X = 32'sd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "5 6 3f",
    );
    // c_untyped_s33d9: both oracles
    digest(
        "c_untyped_s33d9",
        r#"module tb;
  localparam  X = 33'd9;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "9 10 3ff",
    );
    // c_untyped_s40d5: both oracles
    digest(
        "c_untyped_s40d5",
        r#"module tb;
  localparam  X = 40'd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "5 6 3f",
    );
    // c_untyped_s4d20: stays loud (declined shape)
    loud(
        "c_untyped_s4d20",
        r#"module tb;
  localparam  X = 4'd20;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
}

#[test]
fn c_untyped_s4sd12() {
    // c_untyped_s4sd12: stays loud (declined shape)
    loud(
        "c_untyped_s4sd12",
        r#"module tb;
  localparam  X = 4'sd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_untyped_s4x: stays loud (declined shape)
    loud(
        "c_untyped_s4x",
        r#"module tb;
  localparam  X = 4'b1x00;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_untyped_s64all: stays loud (declined shape)
    loud(
        "c_untyped_s64all",
        r#"module tb;
  localparam  X = 64'hffff_ffff_ffff_ffff;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_untyped_s64d5: both oracles
    digest(
        "c_untyped_s64d5",
        r#"module tb;
  localparam  X = 64'd5;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "5 6 3f",
    );
}

#[test]
fn c_untyped_s8d12() {
    // c_untyped_s8d12: stays loud (declined shape)
    loud(
        "c_untyped_s8d12",
        r#"module tb;
  localparam  X = 8'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_untyped_s8hff: stays loud (declined shape)
    loud(
        "c_untyped_s8hff",
        r#"module tb;
  localparam  X = 8'hff;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_untyped_sum: stays loud (declined shape)
    loud(
        "c_untyped_sum",
        r#"module tb;
  localparam  X = 32'd12 + 1;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or",
    );
    // c_untyped_u_d12: both oracles
    digest(
        "c_untyped_u_d12",
        r#"module tb;
  localparam  X = 'd12;
  typedef struct packed { logic [X-1:0] a; logic c; } s_t;
  s_t s;
  initial begin s = '1; $display("DIGEST=%0d %0d %h", X, $bits(s), s); #1 $finish; end
endmodule
"#,
        "12 13 1fff",
    );
}

#[test]
fn c_untyped_wide_sum() {
    // c_untyped_wide_sum: both oracles
    digest(
        "c_untyped_wide_sum",
        r#"module tb;
  localparam C = 32'hf;
  localparam D = 32'h1;
  typedef struct packed { logic [C+D:0] m; } t;
  t v;
  initial begin $display("DIGEST=%0d", $bits(v)); #1 $finish; end
endmodule
"#,
        "17",
    );
    // m_arr_formal_loud: stays loud (declined shape)
    loud(
        "m_arr_formal_loud",
        r#"module tb;
  function automatic logic [3:0] f(logic [1:0][3:0] a [2]);
    return a[0][1] ^ a[1][0];
  endfunction
  logic [1:0][3:0] arr [2];
  initial begin arr[0] = 8'h12; arr[1] = 8'h34; $display("DIGEST=%h", f(arr)); #1 $finish; end
endmodule
"#,
        "E-PARSE-UNEXPECTED-TOKEN: expected a multi-dimensional packed formal with unpacked dim",
    );
    // m_call_param_actual: verilator (the other refuses the shape)
    digest(
        "m_call_param_actual",
        r#"module tb;
  localparam logic [15:0][3:0] SB = 64'h0123456789abcdef;
  function automatic logic [7:0] f(logic [7:0] x, logic [15:0][3:0] s);
    logic [7:0] o;
    for (int k = 0; k < 2; k++) o[k*4 +: 4] = s[x[k*4 +: 4]];
    return o;
  endfunction
  initial begin $display("DIGEST=%h %h", f(8'h3a, SB), f(8'hf0, 64'hfedcba9876543210)); #1 $finish; end
endmodule
"#,
        "c5 f0",
    );
    // m_call_var_actual_elem: both oracles
    digest(
        "m_call_var_actual_elem",
        r#"module tb;
  logic [1:0][15:0][3:0] arr;
  function automatic logic [3:0] f(logic [15:0][3:0] s, int i);
    return s[i];
  endfunction
  initial begin arr[0] = 64'h0123456789abcdef; arr[1] = ~64'h0123456789abcdef; $display("DIGEST=%h %h", f(arr[0], 3), f(arr[1], 3)); #1 $finish; end
endmodule
"#,
        "c 3",
    );
}

#[test]
fn m_concat_lvalue() {
    // m_concat_lvalue: stays loud (declined shape)
    loud(
        "m_concat_lvalue",
        r#"module tb;
  task automatic t(output logic [1:0][3:0] a, output logic [3:0] b);
    {a[1], b, a[0]} = 12'h9ab;
  endtask
  logic [1:0][3:0] v; logic [3:0] b;
  initial begin t(v, b); $display("DIGEST=%h %h", v, b); #1 $finish; end
endmodule
"#,
        "E-ELAB-UNSUPPORTED: frame function/task `t` body uses a concatenation-target ass",
    );
    // m_cont: both oracles
    digest(
        "m_cont",
        r#"module tb;
  function automatic logic [7:0] f(logic [1:0][3:0] a, b);
    return {a[1], b[0]};
  endfunction
  initial begin $display("DIGEST=%h", f(8'h12, 8'h34)); #1 $finish; end
endmodule
"#,
        "14",
    );
    // m_cont_dir: both oracles
    digest(
        "m_cont_dir",
        r#"module tb;
  function automatic logic [7:0] f(input logic [1:0][3:0] a, input logic [3:0] c, b);
    return {a[0], c} ^ b;
  endfunction
  initial begin $display("DIGEST=%h", f(8'hab, 4'h5, 4'h1)); #1 $finish; end
endmodule
"#,
        "b4",
    );
    // m_cont_reset: both oracles
    digest(
        "m_cont_reset",
        r#"module tb;
  function automatic logic [7:0] f(input logic [1:0][3:0] a, input b);
    return {a[0], 3'b0, b};
  endfunction
  initial begin $display("DIGEST=%h", f(8'hab, 1'b1)); #1 $finish; end
endmodule
"#,
        "b1",
    );
}

#[test]
fn m_default() {
    // m_default: both oracles
    digest(
        "m_default",
        r#"module tb;
  function automatic logic [3:0] f(logic [3:0] x, logic [1:0][3:0] a = 8'ha5);
    return x ^ a[1] ^ a[0];
  endfunction
  initial begin $display("DIGEST=%h %h", f(4'h1), f(4'h1, 8'h30)); #1 $finish; end
endmodule
"#,
        "e 2",
    );
    // m_for_loop_elem: both oracles
    digest(
        "m_for_loop_elem",
        r#"module tb;
  function automatic logic [31:0] f(logic [31:0] s, logic [15:0][3:0] shifts);
    logic [31:0] o;
    for (int k = 0; k < 16; k++) o[k*2 +: 2] = s[shifts[k]*2 +: 2];
    return o;
  endfunction
  initial begin $display("DIGEST=%h", f(32'h89abcdef, 64'hfedcba9876543210)); #1 $finish; end
endmodule
"#,
        "89abcdef",
    );
    // m_local_same_dims: both oracles
    digest(
        "m_local_same_dims",
        r#"module tb;
  function automatic logic [3:0] f(logic [1:0][3:0] a);
    logic [1:0][3:0] a2;
    a2 = a; a2[1] = 4'hf;
    return a2[1] & a[0] | a2[0];
  endfunction
  initial begin $display("DIGEST=%h", f(8'h3c)); #1 $finish; end
endmodule
"#,
        "c",
    );
    // m_lvalue_whole_then_elem: both oracles
    digest(
        "m_lvalue_whole_then_elem",
        r#"module tb;
  task automatic t(output logic [1:0][3:0] a);
    a = 8'h00; a[1] = 4'h9; a[0][3:2] = 2'b11;
  endtask
  logic [1:0][3:0] v;
  initial begin t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "9c",
    );
}

#[test]
fn m_out_of_range_read() {
    // m_out_of_range_read: iverilog (4-state; verilator is 2-state and prints '2 1')
    digest(
        "m_out_of_range_read",
        r#"module tb;
  function automatic logic [3:0] f(logic [1:0][3:0] a, int i);
    return a[i];
  endfunction
  initial begin $display("DIGEST=%h %h", f(8'h12, 2), f(8'h12, 1)); #1 $finish; end
endmodule
"#,
        "x 1",
    );
    // m_out_of_range_write: stays loud (both oracles refuse it too)
    loud(
        "m_out_of_range_write",
        r#"module tb;
  task automatic t(inout logic [1:0][3:0] a, int i);
    a[i] = 4'hf;
  endtask
  logic [1:0][3:0] v;
  initial begin v = 8'h12; t(v, 2); $display("DIGEST=%h", v); t(v, 1); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "E-ELAB-UNSUPPORTED: task `t` output/inout arg must be a simple net [in tb]",
    );
    // m_param_dims: stays loud (both oracles refuse it too)
    loud(
        "m_param_dims",
        r#"module tb #(parameter int N = 4);
  function automatic logic [3:0] f(logic [N-1:0][3:0] a);
    return a[N-1] ^ a[0];
  endfunction
  initial begin $display("DIGEST=%h %0d", f(16'h1234), $size(f(16'h1234))); #1 $finish; end
endmodule
"#,
        "E-ELAB-UNSUPPORTED: unsupported system function in expression [in tb]",
    );
    // m_perm_write: both oracles
    digest(
        "m_perm_write",
        r#"module tb;
  function automatic logic [7:0] f(logic [7:0] s, logic [7:0][2:0] perm);
    logic [7:0] o;
    for (int k = 0; k < 8; k++) o[perm[k]] = s[k];
    return o;
  endfunction
  initial begin $display("DIGEST=%h", f(8'h96, 24'o76543210), f(8'h96, 24'o01234567)); #1 $finish; end
endmodule
"#,
        "96105",
    );
}

#[test]
fn m_pkg_shadow_export() {
    // m_pkg_shadow_export: verilator (the other refuses the shape)
    digest(
        "m_pkg_shadow_export",
        r#"package p;
  parameter logic [1:0][3:0] P = 8'hab;
  function automatic logic [3:0] f(logic [3:0][1:0] P);
    return {P[3], P[0]};
  endfunction
endpackage
module tb;
  import p::*;
  initial begin $display("DIGEST=%h %h %0d", f(8'hc3), P[1], $size(P)); #1 $finish; end
endmodule
"#,
        "f a 2",
    );
    // m_pkgconst_dims: both oracles
    digest(
        "m_pkgconst_dims",
        r#"package p;
  localparam int N = 4;
  typedef logic [N-1:0][3:0] t_t;
endpackage
module tb;
  function automatic logic [3:0] f(p::t_t a);
    return a[3] ^ a[0];
  endfunction
  initial begin $display("DIGEST=%h", f(16'h1234)); #1 $finish; end
endmodule
"#,
        "5",
    );
    // m_recursive: both oracles
    digest(
        "m_recursive",
        r#"module tb;
  function automatic logic [3:0] f(logic [1:0][3:0] a, int n);
    if (n == 0) return a[0];
    return f({a[0], a[1]}, n - 1) + 1;
  endfunction
  initial begin $display("DIGEST=%h %h", f(8'h12, 0), f(8'h12, 1)); #1 $finish; end
endmodule
"#,
        "2 2",
    );
    // m_shadow_after: verilator (the other refuses the shape)
    digest(
        "m_shadow_after",
        r#"module tb;
  parameter logic [1:0][3:0] P = 8'hab;
  function automatic logic [3:0] f(logic [3:0][1:0] P);
    return {P[3], P[0]};
  endfunction
  logic [3:0] y;
  initial begin y = f(8'hc3); $display("DIGEST=%h %h %h", y, P[1], $size(P)); #1 $finish; end
endmodule
"#,
        "f a 00000002",
    );
}

#[test]
fn m_shadow_param() {
    // m_shadow_param: verilator (the other refuses the shape)
    digest(
        "m_shadow_param",
        r#"module tb;
  parameter logic [1:0][3:0] P = 8'hab;
  function automatic logic [3:0] f(logic [3:0][1:0] P);
    return {P[3], P[0]};
  endfunction
  function automatic logic [3:0] g(logic [1:0][3:0] Q);
    return P[1] ^ Q[1];
  endfunction
  initial begin $display("DIGEST=%h %h %h", f(8'hc3), g(8'h0f), P[0]); #1 $finish; end
endmodule
"#,
        "f a b",
    );
    // m_signed: both oracles
    digest(
        "m_signed",
        r#"module tb;
  function automatic logic [3:0] g(logic signed [1:0][3:0] a);
    return a[1] >>> 1;
  endfunction
  function automatic int s(logic signed [1:0][3:0] a);
    return a;
  endfunction
  initial begin $display("DIGEST=%h %h %0d", g(8'h80), g(8'h70), s(8'h80)); #1 $finish; end
endmodule
"#,
        "4 3 -128",
    );
    // m_two_ports: both oracles
    digest(
        "m_two_ports",
        r#"module tb;
  function automatic logic [7:0] f(logic [1:0][3:0] a, logic [3:0][1:0] b);
    return {a[1], b[3], b[0], a[0][1:0]};
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a, 8'h93)); #1 $finish; end
endmodule
"#,
        "6e",
    );
    // r_a2_bit_ansi_fn: both oracles
    digest(
        "r_a2_bit_ansi_fn",
        r#"module tb;
  function automatic logic [31:0] f(input logic [0:1][0:3] a);
    return a[1];
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
}

#[test]
fn r_a2_bit_ansi_ref() {
    // r_a2_bit_ansi_ref: verilator (the other refuses the shape)
    digest(
        "r_a2_bit_ansi_ref",
        r#"module tb;
  function automatic logic [31:0] f(ref logic [0:1][0:3] a);
    return a[1];
  endfunction
  logic [0:1][0:3] v;
  initial begin v = 8'h5a; $display("DIGEST=%h", f(v)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_a2_bit_ctl_par: verilator (the other refuses the shape)
    digest(
        "r_a2_bit_ctl_par",
        r#"module tb;
  localparam logic [0:1][0:3] a = 8'h5a;
  function automatic logic [31:0] f();
    return a[1];
  endfunction
  initial begin $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_a2_bit_ctl_var: both oracles
    digest(
        "r_a2_bit_ctl_var",
        r#"module tb;
  logic [0:1][0:3] a;
  function automatic logic [31:0] f();
    return a[1];
  endfunction
  initial begin a = 8'h5a; $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_a2_bit_nonansi: both oracles
    digest(
        "r_a2_bit_nonansi",
        r#"module tb;
  function automatic logic [31:0] f;
    input logic [0:1][0:3] a;
    return a[1];
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
}

#[test]
fn r_a2_bit_pkg() {
    // r_a2_bit_pkg: both oracles
    digest(
        "r_a2_bit_pkg",
        r#"package p;
  function automatic logic [31:0] f(logic [0:1][0:3] a);
    return a[1];
  endfunction
endpackage
module tb;
  import p::*;
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_a2_bit_pkgtd: both oracles
    digest(
        "r_a2_bit_pkgtd",
        r#"package p;
  typedef logic [0:1][0:3] t_t;
  function automatic logic [31:0] f(t_t a);
    return a[1];
  endfunction
endpackage
module tb;
  initial begin $display("DIGEST=%h", p::f(8'h5a)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_a2_bit_task: both oracles
    digest(
        "r_a2_bit_task",
        r#"module tb;
  task automatic t(input logic [0:1][0:3] a, output logic [31:0] r);
    r = a[1];
  endtask
  logic [31:0] r;
  initial begin t(8'h5a, r); $display("DIGEST=%h", r); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_a2_bit_typedef: both oracles
    digest(
        "r_a2_bit_typedef",
        r#"module tb;
  typedef logic [0:1][0:3] t_t;
  function automatic logic [31:0] f(t_t a);
    return a[1];
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
}

#[test]
fn r_a2_bitsel_ansi_fn() {
    // r_a2_bitsel_ansi_fn: both oracles
    digest(
        "r_a2_bitsel_ansi_fn",
        r#"module tb;
  function automatic logic [31:0] f(input logic [0:1][0:3] a);
    return $bits(a[1]);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000004",
    );
    // r_a2_bitsel_ansi_ref: verilator (the other refuses the shape)
    digest(
        "r_a2_bitsel_ansi_ref",
        r#"module tb;
  function automatic logic [31:0] f(ref logic [0:1][0:3] a);
    return $bits(a[1]);
  endfunction
  logic [0:1][0:3] v;
  initial begin v = 8'h5a; $display("DIGEST=%h", f(v)); #1 $finish; end
endmodule
"#,
        "00000004",
    );
    // r_a2_bitsel_ctl_par: verilator (the other refuses the shape)
    digest(
        "r_a2_bitsel_ctl_par",
        r#"module tb;
  localparam logic [0:1][0:3] a = 8'h5a;
  function automatic logic [31:0] f();
    return $bits(a[1]);
  endfunction
  initial begin $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000004",
    );
    // r_a2_bitsel_ctl_var: both oracles
    digest(
        "r_a2_bitsel_ctl_var",
        r#"module tb;
  logic [0:1][0:3] a;
  function automatic logic [31:0] f();
    return $bits(a[1]);
  endfunction
  initial begin a = 8'h5a; $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000004",
    );
}

#[test]
fn r_a2_bitsel_nonansi() {
    // r_a2_bitsel_nonansi: both oracles
    digest(
        "r_a2_bitsel_nonansi",
        r#"module tb;
  function automatic logic [31:0] f;
    input logic [0:1][0:3] a;
    return $bits(a[1]);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000004",
    );
    // r_a2_bitsel_pkg: both oracles
    digest(
        "r_a2_bitsel_pkg",
        r#"package p;
  function automatic logic [31:0] f(logic [0:1][0:3] a);
    return $bits(a[1]);
  endfunction
endpackage
module tb;
  import p::*;
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000004",
    );
    // r_a2_bitsel_pkgtd: both oracles
    digest(
        "r_a2_bitsel_pkgtd",
        r#"package p;
  typedef logic [0:1][0:3] t_t;
  function automatic logic [31:0] f(t_t a);
    return $bits(a[1]);
  endfunction
endpackage
module tb;
  initial begin $display("DIGEST=%h", p::f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000004",
    );
    // r_a2_bitsel_task: both oracles
    digest(
        "r_a2_bitsel_task",
        r#"module tb;
  task automatic t(input logic [0:1][0:3] a, output logic [31:0] r);
    r = $bits(a[1]);
  endtask
  logic [31:0] r;
  initial begin t(8'h5a, r); $display("DIGEST=%h", r); #1 $finish; end
endmodule
"#,
        "00000004",
    );
}

#[test]
fn r_a2_bitsel_typedef() {
    // r_a2_bitsel_typedef: both oracles
    digest(
        "r_a2_bitsel_typedef",
        r#"module tb;
  typedef logic [0:1][0:3] t_t;
  function automatic logic [31:0] f(t_t a);
    return $bits(a[1]);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000004",
    );
    // r_a2_dims_ansi_fn: both oracles
    digest(
        "r_a2_dims_ansi_fn",
        r#"module tb;
  function automatic logic [31:0] f(input logic [0:1][0:3] a);
    return $dimensions(a);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_a2_dims_ansi_ref: verilator (the other refuses the shape)
    digest(
        "r_a2_dims_ansi_ref",
        r#"module tb;
  function automatic logic [31:0] f(ref logic [0:1][0:3] a);
    return $dimensions(a);
  endfunction
  logic [0:1][0:3] v;
  initial begin v = 8'h5a; $display("DIGEST=%h", f(v)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_a2_dims_ctl_par: verilator (the other refuses the shape)
    digest(
        "r_a2_dims_ctl_par",
        r#"module tb;
  localparam logic [0:1][0:3] a = 8'h5a;
  function automatic logic [31:0] f();
    return $dimensions(a);
  endfunction
  initial begin $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000002",
    );
}
