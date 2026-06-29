//! ANSI tf-port type stickiness (IEEE 1800 §13.3 / §23.2.2.3). A comma-separated
//! tf-port name with no direction keyword and no type spec inherits the previous
//! port's full type (net/var kind, signedness, range). vita's `parse_tf_port`
//! propagated only the DIRECTION, so a bare `, y` in `task t(input logic [7:0] x,
//! y)` defaulted to 1-bit (a silent-wrong: y read back as the low bit). Tasks were
//! affected; functions happened to work. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_tfport_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn task_comma_shared_type_inherits() {
    // `input logic [7:0] x, y` — y inherits the [7:0] type (was 1-bit → y=1).
    let out = run(
        "module top; task t(input logic [7:0] x, y); $display(\"x=%h y=%h\", x, y); endtask\n\
         initial begin t(8'hAA, 8'hBB); #1 $finish; end endmodule\n",
    );
    assert!(out.contains("x=aa y=bb"), "got:\n{out}");
}

#[test]
fn task_three_names_share_type() {
    let out = run(
        "module top; task t(input logic [7:0] a, b, c); $display(\"%h %h %h\", a, b, c); endtask\n\
         initial begin t(8'h11, 8'h22, 8'h33); #1 $finish; end endmodule\n",
    );
    assert!(out.contains("11 22 33"), "got:\n{out}");
}

#[test]
fn task_output_comma_shared() {
    let out = run(
        "module top; logic [7:0] p, q; task t(output logic [7:0] x, y); x=8'hCC; y=8'hDD; endtask\n\
         initial begin t(p, q); $display(\"%h %h\", p, q); #1 $finish; end endmodule\n",
    );
    assert!(out.contains("cc dd"), "got:\n{out}");
}

#[test]
fn task_reg_comma_shared() {
    let out = run(
        "module top; task t(input reg [7:0] x, y); $display(\"x=%h y=%h\", x, y); endtask\n\
         initial begin t(8'hAA, 8'hBB); #1 $finish; end endmodule\n",
    );
    assert!(out.contains("x=aa y=bb"), "got:\n{out}");
}

#[test]
fn direction_keyword_resets_type_to_default() {
    // A re-stated direction with no type makes that port the default 1-bit
    // (`input y` after `input logic [7:0] x` → y is 1 bit), NOT inherited.
    let out = run(
        "module top; task t(input logic [7:0] x, input y); $display(\"x=%h y=%h\", x, y); endtask\n\
         initial begin t(8'hAA, 8'hBB); #1 $finish; end endmodule\n",
    );
    assert!(out.contains("x=aa y=1"), "got:\n{out}");
}

#[test]
fn direction_keyword_with_new_range() {
    let out = run(
        "module top; task t(input logic [7:0] x, input [3:0] z); $display(\"x=%h z=%h\", x, z); endtask\n\
         initial begin t(8'hAA, 4'hC); #1 $finish; end endmodule\n",
    );
    assert!(out.contains("x=aa z=c"), "got:\n{out}");
}

#[test]
fn new_type_midlist_propagates_onward() {
    // `byte y, z` mid-list: y is byte AND z (bare) inherits byte (both signed).
    let out = run(
        "module top; task t(input logic [7:0] x, byte y, z); $display(\"x=%h y=%0d z=%0d\", x, y, z); endtask\n\
         initial begin t(8'hAA, -3, -5); #1 $finish; end endmodule\n",
    );
    assert!(out.contains("x=aa y=-3 z=-5"), "got:\n{out}");
}

#[test]
fn range_only_then_bare_inherits() {
    let out = run(
        "module top; task t(input [7:0] x, y); $display(\"x=%h y=%h\", x, y); endtask\n\
         initial begin t(8'hAA, 8'hBB); #1 $finish; end endmodule\n",
    );
    assert!(out.contains("x=aa y=bb"), "got:\n{out}");
}

#[test]
fn function_comma_shared_unchanged() {
    // Functions already worked; confirm the parser change keeps them correct.
    let out = run(
        "module top; function [7:0] f(input logic [7:0] x, y); f=y; endfunction\n\
         initial begin $display(\"%h\", f(8'hAA, 8'hBB)); #1 $finish; end endmodule\n",
    );
    assert!(out.contains("bb"), "got:\n{out}");
}

#[test]
fn each_port_fully_typed_unchanged() {
    // The common fully-typed form is byte-identical (type_present → fresh type).
    let out = run(
        "module top; task t(input logic [7:0] x, input logic [7:0] y); $display(\"x=%h y=%h\", x, y); endtask\n\
         initial begin t(8'hAA, 8'hBB); #1 $finish; end endmodule\n",
    );
    assert!(out.contains("x=aa y=bb"), "got:\n{out}");
}
