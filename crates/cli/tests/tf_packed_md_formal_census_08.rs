//! §4.5.418 census (part 8 of 11): a multi-dimensional packed tf-port formal
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
    let d = std::env::temp_dir().join(format!("vita_mdformal8_{}_{n}", std::process::id()));
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

#[test]
fn r_d3_part_ctl_par() {
    // r_d3_part_ctl_par: verilator (the other refuses the shape)
    digest(
        "r_d3_part_ctl_par",
        r#"module tb;
  localparam logic [1:0][2:0][1:0] a = 12'ha5c;
  function automatic logic [31:0] f();
    return a[1][2:1];
  endfunction
  initial begin $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_d3_part_nonansi: both oracles
    digest(
        "r_d3_part_nonansi",
        r#"module tb;
  function automatic logic [31:0] f;
    input logic [1:0][2:0][1:0] a;
    return a[1][2:1];
  endfunction
  initial begin $display("DIGEST=%h", f(12'ha5c)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_d3_part_pkg: both oracles
    digest(
        "r_d3_part_pkg",
        r#"package p;
  function automatic logic [31:0] f(logic [1:0][2:0][1:0] a);
    return a[1][2:1];
  endfunction
endpackage
module tb;
  import p::*;
  initial begin $display("DIGEST=%h", f(12'ha5c)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_d3_part_pkgtd: both oracles
    digest(
        "r_d3_part_pkgtd",
        r#"package p;
  typedef logic [1:0][2:0][1:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return a[1][2:1];
  endfunction
endpackage
module tb;
  initial begin $display("DIGEST=%h", p::f(12'ha5c)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
}

#[test]
fn r_d3_part_task() {
    // r_d3_part_task: both oracles
    digest(
        "r_d3_part_task",
        r#"module tb;
  task automatic t(input logic [1:0][2:0][1:0] a, output logic [31:0] r);
    r = a[1][2:1];
  endtask
  logic [31:0] r;
  initial begin t(12'ha5c, r); $display("DIGEST=%h", r); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_d3_part_typedef: both oracles
    digest(
        "r_d3_part_typedef",
        r#"module tb;
  typedef logic [1:0][2:0][1:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return a[1][2:1];
  endfunction
  initial begin $display("DIGEST=%h", f(12'ha5c)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_d3_size_ansi_fn: both oracles
    digest(
        "r_d3_size_ansi_fn",
        r#"module tb;
  function automatic logic [31:0] f(input logic [1:0][2:0][1:0] a);
    return $size(a);
  endfunction
  initial begin $display("DIGEST=%h", f(12'ha5c)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_d3_size_ansi_ref: verilator (the other refuses the shape)
    digest(
        "r_d3_size_ansi_ref",
        r#"module tb;
  function automatic logic [31:0] f(ref logic [1:0][2:0][1:0] a);
    return $size(a);
  endfunction
  logic [1:0][2:0][1:0] v;
  initial begin v = 12'ha5c; $display("DIGEST=%h", f(v)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
}

#[test]
fn r_d3_size_ctl_par() {
    // r_d3_size_ctl_par: verilator (the other refuses the shape)
    digest(
        "r_d3_size_ctl_par",
        r#"module tb;
  localparam logic [1:0][2:0][1:0] a = 12'ha5c;
  function automatic logic [31:0] f();
    return $size(a);
  endfunction
  initial begin $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_d3_size_ctl_var: both oracles
    digest(
        "r_d3_size_ctl_var",
        r#"module tb;
  logic [1:0][2:0][1:0] a;
  function automatic logic [31:0] f();
    return $size(a);
  endfunction
  initial begin a = 12'ha5c; $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_d3_size_nonansi: both oracles
    digest(
        "r_d3_size_nonansi",
        r#"module tb;
  function automatic logic [31:0] f;
    input logic [1:0][2:0][1:0] a;
    return $size(a);
  endfunction
  initial begin $display("DIGEST=%h", f(12'ha5c)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_d3_size_pkg: both oracles
    digest(
        "r_d3_size_pkg",
        r#"package p;
  function automatic logic [31:0] f(logic [1:0][2:0][1:0] a);
    return $size(a);
  endfunction
endpackage
module tb;
  import p::*;
  initial begin $display("DIGEST=%h", f(12'ha5c)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
}

#[test]
fn r_d3_size_pkgtd() {
    // r_d3_size_pkgtd: both oracles
    digest(
        "r_d3_size_pkgtd",
        r#"package p;
  typedef logic [1:0][2:0][1:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return $size(a);
  endfunction
endpackage
module tb;
  initial begin $display("DIGEST=%h", p::f(12'ha5c)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_d3_size_task: both oracles
    digest(
        "r_d3_size_task",
        r#"module tb;
  task automatic t(input logic [1:0][2:0][1:0] a, output logic [31:0] r);
    r = $size(a);
  endtask
  logic [31:0] r;
  initial begin t(12'ha5c, r); $display("DIGEST=%h", r); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_d3_size_typedef: both oracles
    digest(
        "r_d3_size_typedef",
        r#"module tb;
  typedef logic [1:0][2:0][1:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return $size(a);
  endfunction
  initial begin $display("DIGEST=%h", f(12'ha5c)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_d3_whole_ansi_fn: both oracles
    digest(
        "r_d3_whole_ansi_fn",
        r#"module tb;
  function automatic logic [31:0] f(input logic [1:0][2:0][1:0] a);
    return a;
  endfunction
  initial begin $display("DIGEST=%h", f(12'ha5c)); #1 $finish; end
endmodule
"#,
        "00000a5c",
    );
}

#[test]
fn r_d3_whole_ansi_ref() {
    // r_d3_whole_ansi_ref: verilator (the other refuses the shape)
    digest(
        "r_d3_whole_ansi_ref",
        r#"module tb;
  function automatic logic [31:0] f(ref logic [1:0][2:0][1:0] a);
    return a;
  endfunction
  logic [1:0][2:0][1:0] v;
  initial begin v = 12'ha5c; $display("DIGEST=%h", f(v)); #1 $finish; end
endmodule
"#,
        "00000a5c",
    );
    // r_d3_whole_ctl_par: verilator (the other refuses the shape)
    digest(
        "r_d3_whole_ctl_par",
        r#"module tb;
  localparam logic [1:0][2:0][1:0] a = 12'ha5c;
  function automatic logic [31:0] f();
    return a;
  endfunction
  initial begin $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000a5c",
    );
    // r_d3_whole_ctl_var: both oracles
    digest(
        "r_d3_whole_ctl_var",
        r#"module tb;
  logic [1:0][2:0][1:0] a;
  function automatic logic [31:0] f();
    return a;
  endfunction
  initial begin a = 12'ha5c; $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000a5c",
    );
    // r_d3_whole_nonansi: both oracles
    digest(
        "r_d3_whole_nonansi",
        r#"module tb;
  function automatic logic [31:0] f;
    input logic [1:0][2:0][1:0] a;
    return a;
  endfunction
  initial begin $display("DIGEST=%h", f(12'ha5c)); #1 $finish; end
endmodule
"#,
        "00000a5c",
    );
}

#[test]
fn r_d3_whole_pkg() {
    // r_d3_whole_pkg: both oracles
    digest(
        "r_d3_whole_pkg",
        r#"package p;
  function automatic logic [31:0] f(logic [1:0][2:0][1:0] a);
    return a;
  endfunction
endpackage
module tb;
  import p::*;
  initial begin $display("DIGEST=%h", f(12'ha5c)); #1 $finish; end
endmodule
"#,
        "00000a5c",
    );
    // r_d3_whole_pkgtd: both oracles
    digest(
        "r_d3_whole_pkgtd",
        r#"package p;
  typedef logic [1:0][2:0][1:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return a;
  endfunction
endpackage
module tb;
  initial begin $display("DIGEST=%h", p::f(12'ha5c)); #1 $finish; end
endmodule
"#,
        "00000a5c",
    );
    // r_d3_whole_task: both oracles
    digest(
        "r_d3_whole_task",
        r#"module tb;
  task automatic t(input logic [1:0][2:0][1:0] a, output logic [31:0] r);
    r = a;
  endtask
  logic [31:0] r;
  initial begin t(12'ha5c, r); $display("DIGEST=%h", r); #1 $finish; end
endmodule
"#,
        "00000a5c",
    );
    // r_d3_whole_typedef: both oracles
    digest(
        "r_d3_whole_typedef",
        r#"module tb;
  typedef logic [1:0][2:0][1:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return a;
  endfunction
  initial begin $display("DIGEST=%h", f(12'ha5c)); #1 $finish; end
endmodule
"#,
        "00000a5c",
    );
}

#[test]
fn r_m2_bit_ansi_fn() {
    // r_m2_bit_ansi_fn: both oracles
    digest(
        "r_m2_bit_ansi_fn",
        r#"module tb;
  function automatic logic [31:0] f(input logic [0:1][3:0] a);
    return a[1];
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_m2_bit_ansi_ref: verilator (the other refuses the shape)
    digest(
        "r_m2_bit_ansi_ref",
        r#"module tb;
  function automatic logic [31:0] f(ref logic [0:1][3:0] a);
    return a[1];
  endfunction
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; $display("DIGEST=%h", f(v)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_m2_bit_ctl_par: verilator (the other refuses the shape)
    digest(
        "r_m2_bit_ctl_par",
        r#"module tb;
  localparam logic [0:1][3:0] a = 8'h5a;
  function automatic logic [31:0] f();
    return a[1];
  endfunction
  initial begin $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_m2_bit_ctl_var: both oracles
    digest(
        "r_m2_bit_ctl_var",
        r#"module tb;
  logic [0:1][3:0] a;
  function automatic logic [31:0] f();
    return a[1];
  endfunction
  initial begin a = 8'h5a; $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
}

#[test]
fn r_m2_bit_nonansi() {
    // r_m2_bit_nonansi: both oracles
    digest(
        "r_m2_bit_nonansi",
        r#"module tb;
  function automatic logic [31:0] f;
    input logic [0:1][3:0] a;
    return a[1];
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_m2_bit_pkg: both oracles
    digest(
        "r_m2_bit_pkg",
        r#"package p;
  function automatic logic [31:0] f(logic [0:1][3:0] a);
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
    // r_m2_bit_pkgtd: both oracles
    digest(
        "r_m2_bit_pkgtd",
        r#"package p;
  typedef logic [0:1][3:0] t_t;
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
    // r_m2_bit_task: both oracles
    digest(
        "r_m2_bit_task",
        r#"module tb;
  task automatic t(input logic [0:1][3:0] a, output logic [31:0] r);
    r = a[1];
  endtask
  logic [31:0] r;
  initial begin t(8'h5a, r); $display("DIGEST=%h", r); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
}

#[test]
fn r_m2_bit_typedef() {
    // r_m2_bit_typedef: both oracles
    digest(
        "r_m2_bit_typedef",
        r#"module tb;
  typedef logic [0:1][3:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return a[1];
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_m2_bitsel_ansi_fn: both oracles
    digest(
        "r_m2_bitsel_ansi_fn",
        r#"module tb;
  function automatic logic [31:0] f(input logic [0:1][3:0] a);
    return $bits(a[1]);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000004",
    );
    // r_m2_bitsel_ansi_ref: verilator (the other refuses the shape)
    digest(
        "r_m2_bitsel_ansi_ref",
        r#"module tb;
  function automatic logic [31:0] f(ref logic [0:1][3:0] a);
    return $bits(a[1]);
  endfunction
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; $display("DIGEST=%h", f(v)); #1 $finish; end
endmodule
"#,
        "00000004",
    );
    // r_m2_bitsel_ctl_par: verilator (the other refuses the shape)
    digest(
        "r_m2_bitsel_ctl_par",
        r#"module tb;
  localparam logic [0:1][3:0] a = 8'h5a;
  function automatic logic [31:0] f();
    return $bits(a[1]);
  endfunction
  initial begin $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000004",
    );
}

#[test]
fn r_m2_bitsel_ctl_var() {
    // r_m2_bitsel_ctl_var: both oracles
    digest(
        "r_m2_bitsel_ctl_var",
        r#"module tb;
  logic [0:1][3:0] a;
  function automatic logic [31:0] f();
    return $bits(a[1]);
  endfunction
  initial begin a = 8'h5a; $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000004",
    );
    // r_m2_bitsel_nonansi: both oracles
    digest(
        "r_m2_bitsel_nonansi",
        r#"module tb;
  function automatic logic [31:0] f;
    input logic [0:1][3:0] a;
    return $bits(a[1]);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000004",
    );
    // r_m2_bitsel_pkg: both oracles
    digest(
        "r_m2_bitsel_pkg",
        r#"package p;
  function automatic logic [31:0] f(logic [0:1][3:0] a);
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
    // r_m2_bitsel_pkgtd: both oracles
    digest(
        "r_m2_bitsel_pkgtd",
        r#"package p;
  typedef logic [0:1][3:0] t_t;
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
}

#[test]
fn r_m2_bitsel_task() {
    // r_m2_bitsel_task: both oracles
    digest(
        "r_m2_bitsel_task",
        r#"module tb;
  task automatic t(input logic [0:1][3:0] a, output logic [31:0] r);
    r = $bits(a[1]);
  endtask
  logic [31:0] r;
  initial begin t(8'h5a, r); $display("DIGEST=%h", r); #1 $finish; end
endmodule
"#,
        "00000004",
    );
    // r_m2_bitsel_typedef: both oracles
    digest(
        "r_m2_bitsel_typedef",
        r#"module tb;
  typedef logic [0:1][3:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return $bits(a[1]);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000004",
    );
    // r_m2_dims_ansi_fn: both oracles
    digest(
        "r_m2_dims_ansi_fn",
        r#"module tb;
  function automatic logic [31:0] f(input logic [0:1][3:0] a);
    return $dimensions(a);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_dims_ansi_ref: verilator (the other refuses the shape)
    digest(
        "r_m2_dims_ansi_ref",
        r#"module tb;
  function automatic logic [31:0] f(ref logic [0:1][3:0] a);
    return $dimensions(a);
  endfunction
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; $display("DIGEST=%h", f(v)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
}

#[test]
fn r_m2_dims_ctl_par() {
    // r_m2_dims_ctl_par: verilator (the other refuses the shape)
    digest(
        "r_m2_dims_ctl_par",
        r#"module tb;
  localparam logic [0:1][3:0] a = 8'h5a;
  function automatic logic [31:0] f();
    return $dimensions(a);
  endfunction
  initial begin $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_dims_ctl_var: both oracles
    digest(
        "r_m2_dims_ctl_var",
        r#"module tb;
  logic [0:1][3:0] a;
  function automatic logic [31:0] f();
    return $dimensions(a);
  endfunction
  initial begin a = 8'h5a; $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_dims_nonansi: both oracles
    digest(
        "r_m2_dims_nonansi",
        r#"module tb;
  function automatic logic [31:0] f;
    input logic [0:1][3:0] a;
    return $dimensions(a);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_dims_pkg: both oracles
    digest(
        "r_m2_dims_pkg",
        r#"package p;
  function automatic logic [31:0] f(logic [0:1][3:0] a);
    return $dimensions(a);
  endfunction
endpackage
module tb;
  import p::*;
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
}

#[test]
fn r_m2_dims_pkgtd() {
    // r_m2_dims_pkgtd: both oracles
    digest(
        "r_m2_dims_pkgtd",
        r#"package p;
  typedef logic [0:1][3:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return $dimensions(a);
  endfunction
endpackage
module tb;
  initial begin $display("DIGEST=%h", p::f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_dims_task: both oracles
    digest(
        "r_m2_dims_task",
        r#"module tb;
  task automatic t(input logic [0:1][3:0] a, output logic [31:0] r);
    r = $dimensions(a);
  endtask
  logic [31:0] r;
  initial begin t(8'h5a, r); $display("DIGEST=%h", r); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_dims_typedef: both oracles
    digest(
        "r_m2_dims_typedef",
        r#"module tb;
  typedef logic [0:1][3:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return $dimensions(a);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_high_ansi_fn: both oracles
    digest(
        "r_m2_high_ansi_fn",
        r#"module tb;
  function automatic logic [31:0] f(input logic [0:1][3:0] a);
    return $high(a,2);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000003",
    );
}

#[test]
fn r_m2_high_ansi_ref() {
    // r_m2_high_ansi_ref: verilator (the other refuses the shape)
    digest(
        "r_m2_high_ansi_ref",
        r#"module tb;
  function automatic logic [31:0] f(ref logic [0:1][3:0] a);
    return $high(a,2);
  endfunction
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; $display("DIGEST=%h", f(v)); #1 $finish; end
endmodule
"#,
        "00000003",
    );
    // r_m2_high_ctl_par: verilator (the other refuses the shape)
    digest(
        "r_m2_high_ctl_par",
        r#"module tb;
  localparam logic [0:1][3:0] a = 8'h5a;
  function automatic logic [31:0] f();
    return $high(a,2);
  endfunction
  initial begin $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000003",
    );
    // r_m2_high_ctl_var: both oracles
    digest(
        "r_m2_high_ctl_var",
        r#"module tb;
  logic [0:1][3:0] a;
  function automatic logic [31:0] f();
    return $high(a,2);
  endfunction
  initial begin a = 8'h5a; $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000003",
    );
    // r_m2_high_nonansi: both oracles
    digest(
        "r_m2_high_nonansi",
        r#"module tb;
  function automatic logic [31:0] f;
    input logic [0:1][3:0] a;
    return $high(a,2);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000003",
    );
}

#[test]
fn r_m2_high_pkg() {
    // r_m2_high_pkg: both oracles
    digest(
        "r_m2_high_pkg",
        r#"package p;
  function automatic logic [31:0] f(logic [0:1][3:0] a);
    return $high(a,2);
  endfunction
endpackage
module tb;
  import p::*;
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000003",
    );
    // r_m2_high_pkgtd: both oracles
    digest(
        "r_m2_high_pkgtd",
        r#"package p;
  typedef logic [0:1][3:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return $high(a,2);
  endfunction
endpackage
module tb;
  initial begin $display("DIGEST=%h", p::f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000003",
    );
    // r_m2_high_task: both oracles
    digest(
        "r_m2_high_task",
        r#"module tb;
  task automatic t(input logic [0:1][3:0] a, output logic [31:0] r);
    r = $high(a,2);
  endtask
  logic [31:0] r;
  initial begin t(8'h5a, r); $display("DIGEST=%h", r); #1 $finish; end
endmodule
"#,
        "00000003",
    );
    // r_m2_high_typedef: both oracles
    digest(
        "r_m2_high_typedef",
        r#"module tb;
  typedef logic [0:1][3:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return $high(a,2);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000003",
    );
}
