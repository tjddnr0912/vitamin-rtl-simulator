//! HIER-REST: a hierarchical WHOLE-net WRITE target (`tb.dut.x = …`). The read
//! side (N3) was already supported; the write side stayed a loud E3009. This
//! defers the lvalue net (the child instance's nets do not exist when the lvalue
//! is lowered) and patches the `LvalChunk` once every instance is elaborated —
//! symmetric with `resolve_deferred_hier`. Pure IR-0 (elaborate-only).
//!
//! Every expected value is pinned to LIVE iverilog 13.0 (which supports both the
//! hierarchical write and the hierarchical read used to observe it). A write to a
//! `wire`, to a whole array / multi-dim packed net, or an element/part-select
//! write stays a loud E-code — NOT a silent wrong value.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_hw_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

fn is_loud(out: &str, code: Option<i32>) -> bool {
    out.contains("VITA-E30") || code == Some(1)
}

#[test]
fn blocking_hierarchical_write() {
    // top writes dut.x then reads it back hierarchically: x = 42.
    let (out, _c) = run("module sub; reg [7:0] x; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.x = 8'h42; #1 $display(\"R %h\", dut.x); end\n\
         endmodule\n");
    assert!(out.contains("R 42"), "blocking hierarchical write:\n{out}");
}

#[test]
fn nonblocking_hierarchical_write() {
    let (out, _c) = run("module sub; reg [7:0] x; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.x <= 8'h99; #1 $display(\"R %h\", dut.x); end\n\
         endmodule\n");
    assert!(
        out.contains("R 99"),
        "nonblocking hierarchical write:\n{out}"
    );
}

#[test]
fn write_drives_submodule_combinational_logic() {
    // dut.x feeds a continuous assign `y = x + 1` inside the child — the
    // hierarchical write must schedule the child's logic. x=10 => y=11.
    let (out, _c) = run("module sub; reg [7:0] x; wire [7:0] y = x + 1; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.x = 8'h10; #1 $display(\"R %h\", dut.y); end\n\
         endmodule\n");
    assert!(
        out.contains("R 11"),
        "write must drive child comb logic:\n{out}"
    );
}

#[test]
fn write_triggers_submodule_always_block() {
    // dut.x feeds `always @(x) z = x ^ 8'hff`. x=0f => z=f0.
    let (out, _c) = run(
        "module sub; reg [7:0] x; reg [7:0] z; always @(x) z = x ^ 8'hff; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.x = 8'h0f; #1 $display(\"R %h\", dut.z); end\n\
         endmodule\n",
    );
    assert!(
        out.contains("R f0"),
        "write must trigger child always:\n{out}"
    );
}

#[test]
fn three_level_hierarchical_write() {
    let (out, _c) = run("module leaf; reg [7:0] x; endmodule\n\
         module mid; leaf l(); endmodule\n\
         module top; mid m();\n\
           initial begin m.l.x = 8'h7e; #1 $display(\"R %h\", m.l.x); end\n\
         endmodule\n");
    assert!(out.contains("R 7e"), "3-level hierarchical write:\n{out}");
}

#[test]
fn multiple_distinct_hierarchical_writes() {
    // two distinct deferred writes must each resolve to their own net (sentinel
    // uniqueness): a=11, b=22 in two child instances.
    let (out, _c) = run("module sub; reg [7:0] v; endmodule\n\
         module top; sub p(); sub q();\n\
           initial begin\n\
             p.v = 8'h11; q.v = 8'h22; #1 $display(\"R %h %h\", p.v, q.v);\n\
           end\n\
         endmodule\n");
    assert!(
        out.contains("R 11 22"),
        "two distinct deferred writes:\n{out}"
    );
}

#[test]
fn forward_reference_write_before_read() {
    // the write statement textually precedes the child's own use of the net.
    let (out, _c) = run("module top; sub dut();\n\
           initial begin dut.x = 8'hab; #1 $display(\"R %h\", dut.x); end\n\
         endmodule\n\
         module sub; reg [7:0] x; endmodule\n");
    assert!(out.contains("R ab"), "forward-declared child write:\n{out}");
}

#[test]
fn write_to_wire_is_loud() {
    // procedural hierarchical write to a `wire` is E3018 (iverilog rejects too).
    let (out, code) = run("module sub; wire [7:0] w; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.w = 8'h42; end\n\
         endmodule\n");
    assert!(
        is_loud(&out, code),
        "write to wire must be loud: {out} {code:?}"
    );
}

#[test]
fn write_to_undeclared_hier_name_is_loud() {
    let (out, code) = run("module sub; reg [7:0] x; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.nope = 8'h42; end\n\
         endmodule\n");
    assert!(
        is_loud(&out, code),
        "undeclared hierarchical write must be loud: {out} {code:?}"
    );
}

#[test]
fn write_to_whole_array_is_loud() {
    // a whole unpacked array has no plain whole-net write value (element write is
    // a deferred follow-on) — loud, NOT a silent word-0 write.
    let (out, code) = run("module sub; reg [7:0] mem [0:3]; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.mem = 8'h42; end\n\
         endmodule\n");
    assert!(
        is_loud(&out, code),
        "whole-array hierarchical write must be loud: {out} {code:?}"
    );
}

#[test]
fn hierarchical_element_write_round_trips() {
    // a hierarchical element write (`dut.mem[i] = …`) is now supported (HIER-REST①,
    // see hier_elem_rw.rs); it writes exactly element `i`, NOT a silent flat write.
    let (out, _c) = run("module sub; reg [7:0] mem [0:3]; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.mem[1] = 8'h42; #1 $display(\"R %h\", dut.mem[1]); end\n\
         endmodule\n");
    assert!(
        out.contains("R 42"),
        "hierarchical element write round-trips:\n{out}"
    );
}

#[test]
fn force_hierarchical_net() {
    // `force dut.x = v` also routes through the deferred lvalue path.
    let (out, _c) = run("module sub; reg [7:0] x; endmodule\n\
         module top; sub dut();\n\
           initial begin force dut.x = 8'h5a; #1 $display(\"R %h\", dut.x); end\n\
         endmodule\n");
    assert!(out.contains("R 5a"), "force on a hierarchical net:\n{out}");
}
