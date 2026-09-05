//! §4.5.418 census (part 11 of 11): a multi-dimensional packed tf-port formal
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
    let d = std::env::temp_dir().join(format!("vita_mdformal11_{}_{n}", std::process::id()));
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
fn w_d3_mid_inout() {
    // w_d3_mid_inout: both oracles
    digest(
        "w_d3_mid_inout",
        r#"module tb;
  task automatic t(inout logic [1:0][2:0][1:0] a);
    a[0][2:1] = 4'b1001;
  endtask
  logic [1:0][2:0][1:0] v;
  initial begin v = 12'ha5c; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "a64",
    );
    // w_d3_mid_local: both oracles (§4.5.427: a mid-dim write on a partially indexed packed net)
    digest(
        "w_d3_mid_local",
        r#"module tb;
  function automatic logic [11:0] f(input logic [1:0][2:0][1:0] a0);
    logic [1:0][2:0][1:0] a;
    a = a0;
    a[0][2:1] = 4'b1001;
    return a;
  endfunction
  initial begin $display("DIGEST=%h", f(12'ha5c)); #1 $finish; end
endmodule
"#,
        "a64",
    );
    // w_d3_mid_output: iverilog (4-state; verilator is 2-state and prints '024')
    digest(
        "w_d3_mid_output",
        r#"module tb;
  task automatic t(output logic [1:0][2:0][1:0] a);
    a[0][2:1] = 4'b1001;
  endtask
  logic [1:0][2:0][1:0] v;
  initial begin v = 12'ha5c; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "xXX",
    );
    // w_d3_mid_ref: verilator (the other refuses the shape)
    digest(
        "w_d3_mid_ref",
        r#"module tb;
  task automatic t(ref logic [1:0][2:0][1:0] a);
    a[0][2:1] = 4'b1001;
  endtask
  logic [1:0][2:0][1:0] v;
  initial begin v = 12'ha5c; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "a64",
    );
}

#[test]
fn w_d3_part_ctl_var() {
    // w_d3_part_ctl_var: both oracles
    digest(
        "w_d3_part_ctl_var",
        r#"module tb;
  logic [1:0][2:0][1:0] a;
  task automatic t();
    a[1][2][1:0] = 2'b10;
  endtask
  initial begin a = 12'ha5c; t(); $display("DIGEST=%h", a); #1 $finish; end
endmodule
"#,
        "a5c",
    );
    // w_d3_part_inout: both oracles
    digest(
        "w_d3_part_inout",
        r#"module tb;
  task automatic t(inout logic [1:0][2:0][1:0] a);
    a[1][2][1:0] = 2'b10;
  endtask
  logic [1:0][2:0][1:0] v;
  initial begin v = 12'ha5c; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "a5c",
    );
    // w_d3_part_local: both oracles
    digest(
        "w_d3_part_local",
        r#"module tb;
  function automatic logic [11:0] f(input logic [1:0][2:0][1:0] a0);
    logic [1:0][2:0][1:0] a;
    a = a0;
    a[1][2][1:0] = 2'b10;
    return a;
  endfunction
  initial begin $display("DIGEST=%h", f(12'ha5c)); #1 $finish; end
endmodule
"#,
        "a5c",
    );
    // w_d3_part_output: iverilog (4-state; verilator is 2-state and prints '800')
    digest(
        "w_d3_part_output",
        r#"module tb;
  task automatic t(output logic [1:0][2:0][1:0] a);
    a[1][2][1:0] = 2'b10;
  endtask
  logic [1:0][2:0][1:0] v;
  initial begin v = 12'ha5c; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "Xxx",
    );
}

#[test]
fn w_d3_part_ref() {
    // w_d3_part_ref: verilator (the other refuses the shape)
    digest(
        "w_d3_part_ref",
        r#"module tb;
  task automatic t(ref logic [1:0][2:0][1:0] a);
    a[1][2][1:0] = 2'b10;
  endtask
  logic [1:0][2:0][1:0] v;
  initial begin v = 12'ha5c; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "a5c",
    );
    // w_m2_bit_ctl_var: both oracles
    digest(
        "w_m2_bit_ctl_var",
        r#"module tb;
  logic [0:1][3:0] a;
  task automatic t();
    a[1] = 4'hf;
  endtask
  initial begin a = 8'h5a; t(); $display("DIGEST=%h", a); #1 $finish; end
endmodule
"#,
        "5f",
    );
    // w_m2_bit_inout: both oracles
    digest(
        "w_m2_bit_inout",
        r#"module tb;
  task automatic t(inout logic [0:1][3:0] a);
    a[1] = 4'hf;
  endtask
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "5f",
    );
    // w_m2_bit_local: both oracles
    digest(
        "w_m2_bit_local",
        r#"module tb;
  function automatic logic [7:0] f(input logic [0:1][3:0] a0);
    logic [0:1][3:0] a;
    a = a0;
    a[1] = 4'hf;
    return a;
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "5f",
    );
}

#[test]
fn w_m2_bit_output() {
    // w_m2_bit_output: iverilog (4-state; verilator is 2-state and prints '0f')
    digest(
        "w_m2_bit_output",
        r#"module tb;
  task automatic t(output logic [0:1][3:0] a);
    a[1] = 4'hf;
  endtask
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "xf",
    );
    // w_m2_bit_ref: verilator (the other refuses the shape)
    digest(
        "w_m2_bit_ref",
        r#"module tb;
  task automatic t(ref logic [0:1][3:0] a);
    a[1] = 4'hf;
  endtask
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "5f",
    );
    // w_m2_idx_ctl_var: stays loud (declined shape)
    loud(
        "w_m2_idx_ctl_var",
        r#"module tb;
  logic [0:1][3:0] a;
  task automatic t();
    a[0][1+:2] = 2'b11;
  endtask
  initial begin a = 8'h5a; t(); $display("DIGEST=%h", a); #1 $finish; end
endmodule
"#,
        "E-ELAB-UNSUPPORTED: nested lvalue select (v1: single-level) [in tb.$func$t]",
    );
    // w_m2_idx_inout: both oracles
    digest(
        "w_m2_idx_inout",
        r#"module tb;
  task automatic t(inout logic [0:1][3:0] a);
    a[0][1+:2] = 2'b11;
  endtask
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "7a",
    );
}

#[test]
fn w_m2_idx_local() {
    // w_m2_idx_local: stays loud (declined shape)
    loud(
        "w_m2_idx_local",
        r#"module tb;
  function automatic logic [7:0] f(input logic [0:1][3:0] a0);
    logic [0:1][3:0] a;
    a = a0;
    a[0][1+:2] = 2'b11;
    return a;
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "E-ELAB-UNSUPPORTED: nested lvalue select (v1: single-level) [in tb.$func$f]",
    );
    // w_m2_idx_output: iverilog (4-state; verilator is 2-state and prints '60')
    digest(
        "w_m2_idx_output",
        r#"module tb;
  task automatic t(output logic [0:1][3:0] a);
    a[0][1+:2] = 2'b11;
  endtask
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "Xx",
    );
    // w_m2_idx_ref: verilator (the other refuses the shape)
    digest(
        "w_m2_idx_ref",
        r#"module tb;
  task automatic t(ref logic [0:1][3:0] a);
    a[0][1+:2] = 2'b11;
  endtask
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "7a",
    );
    // w_m2_nest_ctl_var: both oracles
    digest(
        "w_m2_nest_ctl_var",
        r#"module tb;
  logic [0:1][3:0] a;
  task automatic t();
    a[a[0]%2] = 4'h7;
  endtask
  initial begin a = 8'h5a; t(); $display("DIGEST=%h", a); #1 $finish; end
endmodule
"#,
        "57",
    );
}

#[test]
fn w_m2_nest_inout() {
    // w_m2_nest_inout: both oracles
    digest(
        "w_m2_nest_inout",
        r#"module tb;
  task automatic t(inout logic [0:1][3:0] a);
    a[a[0]%2] = 4'h7;
  endtask
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "57",
    );
    // w_m2_nest_local: both oracles
    digest(
        "w_m2_nest_local",
        r#"module tb;
  function automatic logic [7:0] f(input logic [0:1][3:0] a0);
    logic [0:1][3:0] a;
    a = a0;
    a[a[0]%2] = 4'h7;
    return a;
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "57",
    );
    // w_m2_nest_output: iverilog (4-state; verilator is 2-state and prints '70')
    digest(
        "w_m2_nest_output",
        r#"module tb;
  task automatic t(output logic [0:1][3:0] a);
    a[a[0]%2] = 4'h7;
  endtask
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "xx",
    );
    // w_m2_nest_ref: verilator (the other refuses the shape)
    digest(
        "w_m2_nest_ref",
        r#"module tb;
  task automatic t(ref logic [0:1][3:0] a);
    a[a[0]%2] = 4'h7;
  endtask
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "57",
    );
}

#[test]
fn w_m2_part_ctl_var() {
    // w_m2_part_ctl_var: both oracles
    digest(
        "w_m2_part_ctl_var",
        r#"module tb;
  logic [0:1][3:0] a;
  task automatic t();
    a[1][3:2] = 2'b10;
  endtask
  initial begin a = 8'h5a; t(); $display("DIGEST=%h", a); #1 $finish; end
endmodule
"#,
        "5a",
    );
    // w_m2_part_inout: both oracles
    digest(
        "w_m2_part_inout",
        r#"module tb;
  task automatic t(inout logic [0:1][3:0] a);
    a[1][3:2] = 2'b10;
  endtask
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "5a",
    );
    // w_m2_part_local: both oracles
    digest(
        "w_m2_part_local",
        r#"module tb;
  function automatic logic [7:0] f(input logic [0:1][3:0] a0);
    logic [0:1][3:0] a;
    a = a0;
    a[1][3:2] = 2'b10;
    return a;
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "5a",
    );
    // w_m2_part_output: iverilog (4-state; verilator is 2-state and prints '08')
    digest(
        "w_m2_part_output",
        r#"module tb;
  task automatic t(output logic [0:1][3:0] a);
    a[1][3:2] = 2'b10;
  endtask
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "xX",
    );
}

#[test]
fn w_m2_part_ref() {
    // w_m2_part_ref: verilator (the other refuses the shape)
    digest(
        "w_m2_part_ref",
        r#"module tb;
  task automatic t(ref logic [0:1][3:0] a);
    a[1][3:2] = 2'b10;
  endtask
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "5a",
    );
    // w_m2_whole_ctl_var: both oracles
    digest(
        "w_m2_whole_ctl_var",
        r#"module tb;
  logic [0:1][3:0] a;
  task automatic t();
    a = 8'h81;
  endtask
  initial begin a = 8'h5a; t(); $display("DIGEST=%h", a); #1 $finish; end
endmodule
"#,
        "81",
    );
    // w_m2_whole_inout: both oracles
    digest(
        "w_m2_whole_inout",
        r#"module tb;
  task automatic t(inout logic [0:1][3:0] a);
    a = 8'h81;
  endtask
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "81",
    );
    // w_m2_whole_local: both oracles
    digest(
        "w_m2_whole_local",
        r#"module tb;
  function automatic logic [7:0] f(input logic [0:1][3:0] a0);
    logic [0:1][3:0] a;
    a = a0;
    a = 8'h81;
    return a;
  endfunction
  initial begin $display("DIGEST=%h", f(8'h5a)); #1 $finish; end
endmodule
"#,
        "81",
    );
}

#[test]
fn w_m2_whole_output() {
    // w_m2_whole_output: both oracles
    digest(
        "w_m2_whole_output",
        r#"module tb;
  task automatic t(output logic [0:1][3:0] a);
    a = 8'h81;
  endtask
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "81",
    );
    // w_m2_whole_ref: verilator (the other refuses the shape)
    digest(
        "w_m2_whole_ref",
        r#"module tb;
  task automatic t(ref logic [0:1][3:0] a);
    a = 8'h81;
  endtask
  logic [0:1][3:0] v;
  initial begin v = 8'h5a; t(v); $display("DIGEST=%h", v); #1 $finish; end
endmodule
"#,
        "81",
    );
}
