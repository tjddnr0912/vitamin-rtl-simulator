//! §4.5.418 census (part 9 of 11): a multi-dimensional packed tf-port formal
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
    let d = std::env::temp_dir().join(format!("vita_mdformal9_{}_{n}", std::process::id()));
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
fn r_m2_idx_ansi_fn() {
    // r_m2_idx_ansi_fn: both oracles
    digest(
        "r_m2_idx_ansi_fn",
        r#"module tb;
  function automatic logic [31:0] f(input logic [0:1][3:0] a);
    return a[0][1+:2];
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_idx_ansi_ref: verilator (the other refuses the shape)
    digest(
        "r_m2_idx_ansi_ref",
        r#"module tb;
  function automatic logic [31:0] f(ref logic [0:1][3:0] a);
    return a[0][1+:2];
  endfunction
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; $display("DIGEST=%h", f(v)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_idx_ctl_par: verilator (the other refuses the shape)
    digest(
        "r_m2_idx_ctl_par",
        r#"module tb;
  localparam logic [0:1][3:0] a = 8'h5a;
  function automatic logic [31:0] f();
    return a[0][1+:2];
  endfunction
  initial begin $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_idx_ctl_var: both oracles
    digest(
        "r_m2_idx_ctl_var",
        r#"module tb;
  logic [0:1][3:0] a;
  function automatic logic [31:0] f();
    return a[0][1+:2];
  endfunction
  initial begin a = 8'h5a; $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000002",
    );
}

#[test]
fn r_m2_idx_nonansi() {
    // r_m2_idx_nonansi: both oracles
    digest(
        "r_m2_idx_nonansi",
        r#"module tb;
  function automatic logic [31:0] f;
    input logic [0:1][3:0] a;
    return a[0][1+:2];
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_idx_pkg: both oracles
    digest(
        "r_m2_idx_pkg",
        r#"package p;
  function automatic logic [31:0] f(logic [0:1][3:0] a);
    return a[0][1+:2];
  endfunction
endpackage
module tb;
  import p::*;
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_idx_pkgtd: both oracles
    digest(
        "r_m2_idx_pkgtd",
        r#"package p;
  typedef logic [0:1][3:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return a[0][1+:2];
  endfunction
endpackage
module tb;
  initial begin $display("DIGEST=%h", p::f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_idx_task: both oracles
    digest(
        "r_m2_idx_task",
        r#"module tb;
  task automatic t(input logic [0:1][3:0] a, output logic [31:0] r);
    r = a[0][1+:2];
  endtask
  logic [31:0] r;
  initial begin t(8'h5a, r); $display("DIGEST=%h", r); #1 $finish; end
endmodule
"#,
        "00000002",
    );
}

#[test]
fn r_m2_idx_typedef() {
    // r_m2_idx_typedef: both oracles
    digest(
        "r_m2_idx_typedef",
        r#"module tb;
  typedef logic [0:1][3:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return a[0][1+:2];
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_left_ansi_fn: both oracles
    digest(
        "r_m2_left_ansi_fn",
        r#"module tb;
  function automatic logic [31:0] f(input logic [0:1][3:0] a);
    return $left(a);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000000",
    );
    // r_m2_left_ansi_ref: verilator (the other refuses the shape)
    digest(
        "r_m2_left_ansi_ref",
        r#"module tb;
  function automatic logic [31:0] f(ref logic [0:1][3:0] a);
    return $left(a);
  endfunction
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; $display("DIGEST=%h", f(v)); #1 $finish; end
endmodule
"#,
        "00000000",
    );
    // r_m2_left_ctl_par: verilator (the other refuses the shape)
    digest(
        "r_m2_left_ctl_par",
        r#"module tb;
  localparam logic [0:1][3:0] a = 8'h5a;
  function automatic logic [31:0] f();
    return $left(a);
  endfunction
  initial begin $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000000",
    );
}

#[test]
fn r_m2_left_ctl_var() {
    // r_m2_left_ctl_var: both oracles
    digest(
        "r_m2_left_ctl_var",
        r#"module tb;
  logic [0:1][3:0] a;
  function automatic logic [31:0] f();
    return $left(a);
  endfunction
  initial begin a = 8'h5a; $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000000",
    );
    // r_m2_left_nonansi: both oracles
    digest(
        "r_m2_left_nonansi",
        r#"module tb;
  function automatic logic [31:0] f;
    input logic [0:1][3:0] a;
    return $left(a);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000000",
    );
    // r_m2_left_pkg: both oracles
    digest(
        "r_m2_left_pkg",
        r#"package p;
  function automatic logic [31:0] f(logic [0:1][3:0] a);
    return $left(a);
  endfunction
endpackage
module tb;
  import p::*;
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000000",
    );
    // r_m2_left_pkgtd: both oracles
    digest(
        "r_m2_left_pkgtd",
        r#"package p;
  typedef logic [0:1][3:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return $left(a);
  endfunction
endpackage
module tb;
  initial begin $display("DIGEST=%h", p::f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000000",
    );
}

#[test]
fn r_m2_left_task() {
    // r_m2_left_task: both oracles
    digest(
        "r_m2_left_task",
        r#"module tb;
  task automatic t(input logic [0:1][3:0] a, output logic [31:0] r);
    r = $left(a);
  endtask
  logic [31:0] r;
  initial begin t(8'h5a, r); $display("DIGEST=%h", r); #1 $finish; end
endmodule
"#,
        "00000000",
    );
    // r_m2_left_typedef: both oracles
    digest(
        "r_m2_left_typedef",
        r#"module tb;
  typedef logic [0:1][3:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return $left(a);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000000",
    );
    // r_m2_nest_ansi_fn: both oracles
    digest(
        "r_m2_nest_ansi_fn",
        r#"module tb;
  function automatic logic [31:0] f(input logic [0:1][3:0] a);
    return a[a[0]%2];
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_m2_nest_ansi_ref: verilator (the other refuses the shape)
    digest(
        "r_m2_nest_ansi_ref",
        r#"module tb;
  function automatic logic [31:0] f(ref logic [0:1][3:0] a);
    return a[a[0]%2];
  endfunction
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; $display("DIGEST=%h", f(v)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
}

#[test]
fn r_m2_nest_ctl_par() {
    // r_m2_nest_ctl_par: verilator (the other refuses the shape)
    digest(
        "r_m2_nest_ctl_par",
        r#"module tb;
  localparam logic [0:1][3:0] a = 8'h5a;
  function automatic logic [31:0] f();
    return a[a[0]%2];
  endfunction
  initial begin $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_m2_nest_ctl_var: both oracles
    digest(
        "r_m2_nest_ctl_var",
        r#"module tb;
  logic [0:1][3:0] a;
  function automatic logic [31:0] f();
    return a[a[0]%2];
  endfunction
  initial begin a = 8'h5a; $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_m2_nest_nonansi: both oracles
    digest(
        "r_m2_nest_nonansi",
        r#"module tb;
  function automatic logic [31:0] f;
    input logic [0:1][3:0] a;
    return a[a[0]%2];
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_m2_nest_pkg: both oracles
    digest(
        "r_m2_nest_pkg",
        r#"package p;
  function automatic logic [31:0] f(logic [0:1][3:0] a);
    return a[a[0]%2];
  endfunction
endpackage
module tb;
  import p::*;
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
}

#[test]
fn r_m2_nest_pkgtd() {
    // r_m2_nest_pkgtd: both oracles
    digest(
        "r_m2_nest_pkgtd",
        r#"package p;
  typedef logic [0:1][3:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return a[a[0]%2];
  endfunction
endpackage
module tb;
  initial begin $display("DIGEST=%h", p::f(8'h5a)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_m2_nest_task: both oracles
    digest(
        "r_m2_nest_task",
        r#"module tb;
  task automatic t(input logic [0:1][3:0] a, output logic [31:0] r);
    r = a[a[0]%2];
  endtask
  logic [31:0] r;
  initial begin t(8'h5a, r); $display("DIGEST=%h", r); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_m2_nest_typedef: both oracles
    digest(
        "r_m2_nest_typedef",
        r#"module tb;
  typedef logic [0:1][3:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return a[a[0]%2];
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "0000000a",
    );
    // r_m2_part_ansi_fn: both oracles
    digest(
        "r_m2_part_ansi_fn",
        r#"module tb;
  function automatic logic [31:0] f(input logic [0:1][3:0] a);
    return a[1][2:1];
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000001",
    );
}

#[test]
fn r_m2_part_ansi_ref() {
    // r_m2_part_ansi_ref: verilator (the other refuses the shape)
    digest(
        "r_m2_part_ansi_ref",
        r#"module tb;
  function automatic logic [31:0] f(ref logic [0:1][3:0] a);
    return a[1][2:1];
  endfunction
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; $display("DIGEST=%h", f(v)); #1 $finish; end
endmodule
"#,
        "00000001",
    );
    // r_m2_part_ctl_par: verilator (the other refuses the shape)
    digest(
        "r_m2_part_ctl_par",
        r#"module tb;
  localparam logic [0:1][3:0] a = 8'h5a;
  function automatic logic [31:0] f();
    return a[1][2:1];
  endfunction
  initial begin $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000001",
    );
    // r_m2_part_ctl_var: both oracles
    digest(
        "r_m2_part_ctl_var",
        r#"module tb;
  logic [0:1][3:0] a;
  function automatic logic [31:0] f();
    return a[1][2:1];
  endfunction
  initial begin a = 8'h5a; $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000001",
    );
    // r_m2_part_nonansi: both oracles
    digest(
        "r_m2_part_nonansi",
        r#"module tb;
  function automatic logic [31:0] f;
    input logic [0:1][3:0] a;
    return a[1][2:1];
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000001",
    );
}

#[test]
fn r_m2_part_pkg() {
    // r_m2_part_pkg: both oracles
    digest(
        "r_m2_part_pkg",
        r#"package p;
  function automatic logic [31:0] f(logic [0:1][3:0] a);
    return a[1][2:1];
  endfunction
endpackage
module tb;
  import p::*;
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000001",
    );
    // r_m2_part_pkgtd: both oracles
    digest(
        "r_m2_part_pkgtd",
        r#"package p;
  typedef logic [0:1][3:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return a[1][2:1];
  endfunction
endpackage
module tb;
  initial begin $display("DIGEST=%h", p::f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000001",
    );
    // r_m2_part_task: both oracles
    digest(
        "r_m2_part_task",
        r#"module tb;
  task automatic t(input logic [0:1][3:0] a, output logic [31:0] r);
    r = a[1][2:1];
  endtask
  logic [31:0] r;
  initial begin t(8'h5a, r); $display("DIGEST=%h", r); #1 $finish; end
endmodule
"#,
        "00000001",
    );
    // r_m2_part_typedef: both oracles
    digest(
        "r_m2_part_typedef",
        r#"module tb;
  typedef logic [0:1][3:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return a[1][2:1];
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000001",
    );
}

#[test]
fn r_m2_size_ansi_fn() {
    // r_m2_size_ansi_fn: both oracles
    digest(
        "r_m2_size_ansi_fn",
        r#"module tb;
  function automatic logic [31:0] f(input logic [0:1][3:0] a);
    return $size(a);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_size_ansi_ref: verilator (the other refuses the shape)
    digest(
        "r_m2_size_ansi_ref",
        r#"module tb;
  function automatic logic [31:0] f(ref logic [0:1][3:0] a);
    return $size(a);
  endfunction
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; $display("DIGEST=%h", f(v)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_size_ctl_par: verilator (the other refuses the shape)
    digest(
        "r_m2_size_ctl_par",
        r#"module tb;
  localparam logic [0:1][3:0] a = 8'h5a;
  function automatic logic [31:0] f();
    return $size(a);
  endfunction
  initial begin $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_size_ctl_var: both oracles
    digest(
        "r_m2_size_ctl_var",
        r#"module tb;
  logic [0:1][3:0] a;
  function automatic logic [31:0] f();
    return $size(a);
  endfunction
  initial begin a = 8'h5a; $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "00000002",
    );
}

#[test]
fn r_m2_size_nonansi() {
    // r_m2_size_nonansi: both oracles
    digest(
        "r_m2_size_nonansi",
        r#"module tb;
  function automatic logic [31:0] f;
    input logic [0:1][3:0] a;
    return $size(a);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_size_pkg: both oracles
    digest(
        "r_m2_size_pkg",
        r#"package p;
  function automatic logic [31:0] f(logic [0:1][3:0] a);
    return $size(a);
  endfunction
endpackage
module tb;
  import p::*;
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_size_pkgtd: both oracles
    digest(
        "r_m2_size_pkgtd",
        r#"package p;
  typedef logic [0:1][3:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return $size(a);
  endfunction
endpackage
module tb;
  initial begin $display("DIGEST=%h", p::f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_size_task: both oracles
    digest(
        "r_m2_size_task",
        r#"module tb;
  task automatic t(input logic [0:1][3:0] a, output logic [31:0] r);
    r = $size(a);
  endtask
  logic [31:0] r;
  initial begin t(8'h5a, r); $display("DIGEST=%h", r); #1 $finish; end
endmodule
"#,
        "00000002",
    );
}

#[test]
fn r_m2_size_typedef() {
    // r_m2_size_typedef: both oracles
    digest(
        "r_m2_size_typedef",
        r#"module tb;
  typedef logic [0:1][3:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return $size(a);
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "00000002",
    );
    // r_m2_whole_ansi_fn: both oracles
    digest(
        "r_m2_whole_ansi_fn",
        r#"module tb;
  function automatic logic [31:0] f(input logic [0:1][3:0] a);
    return a;
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "0000005a",
    );
    // r_m2_whole_ansi_ref: verilator (the other refuses the shape)
    digest(
        "r_m2_whole_ansi_ref",
        r#"module tb;
  function automatic logic [31:0] f(ref logic [0:1][3:0] a);
    return a;
  endfunction
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; $display("DIGEST=%h", f(v)); #1 $finish; end
endmodule
"#,
        "0000005a",
    );
    // r_m2_whole_ctl_par: verilator (the other refuses the shape)
    digest(
        "r_m2_whole_ctl_par",
        r#"module tb;
  localparam logic [0:1][3:0] a = 8'h5a;
  function automatic logic [31:0] f();
    return a;
  endfunction
  initial begin $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "0000005a",
    );
}

#[test]
fn r_m2_whole_ctl_var() {
    // r_m2_whole_ctl_var: both oracles
    digest(
        "r_m2_whole_ctl_var",
        r#"module tb;
  logic [0:1][3:0] a;
  function automatic logic [31:0] f();
    return a;
  endfunction
  initial begin a = 8'h5a; $display("DIGEST=%h", f()); #1 $finish; end
endmodule
"#,
        "0000005a",
    );
    // r_m2_whole_nonansi: both oracles
    digest(
        "r_m2_whole_nonansi",
        r#"module tb;
  function automatic logic [31:0] f;
    input logic [0:1][3:0] a;
    return a;
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "0000005a",
    );
    // r_m2_whole_pkg: both oracles
    digest(
        "r_m2_whole_pkg",
        r#"package p;
  function automatic logic [31:0] f(logic [0:1][3:0] a);
    return a;
  endfunction
endpackage
module tb;
  import p::*;
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "0000005a",
    );
    // r_m2_whole_pkgtd: both oracles
    digest(
        "r_m2_whole_pkgtd",
        r#"package p;
  typedef logic [0:1][3:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return a;
  endfunction
endpackage
module tb;
  initial begin $display("DIGEST=%h", p::f(8'h5a)); #1 $finish; end
endmodule
"#,
        "0000005a",
    );
}

#[test]
fn r_m2_whole_task() {
    // r_m2_whole_task: both oracles
    digest(
        "r_m2_whole_task",
        r#"module tb;
  task automatic t(input logic [0:1][3:0] a, output logic [31:0] r);
    r = a;
  endtask
  logic [31:0] r;
  initial begin t(8'h5a, r); $display("DIGEST=%h", r); #1 $finish; end
endmodule
"#,
        "0000005a",
    );
    // r_m2_whole_typedef: both oracles
    digest(
        "r_m2_whole_typedef",
        r#"module tb;
  typedef logic [0:1][3:0] t_t;
  function automatic logic [31:0] f(t_t a);
    return a;
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "0000005a",
    );
    // w_a2_bit_ctl_var: both oracles
    digest(
        "w_a2_bit_ctl_var",
        r#"module tb;
  logic [0:1][0:3] a;
  task automatic t();
    a[1] = 4'hf;
  endtask
  initial begin a = 8'h5a; t(); $display("DIGEST=%h", a); #1 $finish; end
endmodule
"#,
        "5f",
    );
    // w_a2_bit_inout: both oracles
    digest(
        "w_a2_bit_inout",
        r#"module tb;
  task automatic t(inout logic [0:1][0:3] a);
    a[1] = 4'hf;
  endtask
  logic [0:1][0:3] v;
  initial begin v = 8'h5a; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "5f",
    );
}

#[test]
fn w_a2_bit_local() {
    // w_a2_bit_local: both oracles
    digest(
        "w_a2_bit_local",
        r#"module tb;
  function automatic logic [7:0] f(input logic [0:1][0:3] a0);
    logic [0:1][0:3] a;
    a = a0;
    a[1] = 4'hf;
    return a;
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "5f",
    );
    // w_a2_bit_output: iverilog (4-state; verilator is 2-state and prints '0f')
    digest(
        "w_a2_bit_output",
        r#"module tb;
  task automatic t(output logic [0:1][0:3] a);
    a[1] = 4'hf;
  endtask
  logic [0:1][0:3] v;
  initial begin v = 8'h5a; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "xf",
    );
    // w_a2_bit_ref: verilator (the other refuses the shape)
    digest(
        "w_a2_bit_ref",
        r#"module tb;
  task automatic t(ref logic [0:1][0:3] a);
    a[1] = 4'hf;
  endtask
  logic [0:1][0:3] v;
  initial begin v = 8'h5a; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "5f",
    );
    // w_a2_idx_ctl_var: stays loud (declined shape)
    loud(
        "w_a2_idx_ctl_var",
        r#"module tb;
  logic [0:1][0:3] a;
  task automatic t();
    a[0][1+:2] = 2'b11;
  endtask
  initial begin a = 8'h5a; t(); $display("DIGEST=%h", a); #1 $finish; end
endmodule
"#,
        "E-ELAB-UNSUPPORTED: nested lvalue select (v1: single-level) [in tb.$func$t]",
    );
}
