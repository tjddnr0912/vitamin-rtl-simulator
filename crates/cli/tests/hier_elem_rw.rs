//! HIER-REST①: a hierarchical ELEMENT / bit-select READ and WRITE
//! (`m.l.mem[i]`, `dut.mem[i] = …`, `dut.v[3] <= …`). The whole-net read (N3) and
//! whole-net write were already supported; the indexed READ worked for a
//! 2-segment base (`dut.mem[i]`) but not deeper, and an element/bit-select WRITE
//! was a loud follow-on for every depth. This adds: (a) 3+-segment indexed reads
//! (generalising the read-side `hier_sel_chain`), and (b) a deferred element/bit-
//! select WRITE that rebuilds the `LvalChunk` (array element word / packed
//! bit-slice / vector bit) from the resolved net's shape. Pure IR-0 (elaborate
//! lowering only).
//!
//! Every value is pinned to LIVE iverilog 13.0 (which supports hierarchical
//! element reads and writes). A write to a hierarchical `wire` bit or an
//! undeclared hierarchical element stays a loud E-code — NOT a silent value.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_hrw_{}_{n}", std::process::id()));
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

// ── READ: 3+-segment hierarchical element select ───────────────────────────

#[test]
fn three_level_array_element_read() {
    // m.l.mem[1] reads the leaf element across two scope levels.
    let (out, _c) = run(
        "module leaf; reg [7:0] mem [0:3]; initial mem[1]=8'h7e; endmodule\n\
         module mid; leaf l(); endmodule\n\
         module top; mid m();\n\
           initial begin #1 $display(\"R %h\", m.l.mem[1]); end\n\
         endmodule\n",
    );
    assert!(out.contains("R 7e"), "3-level element read:\n{out}");
}

// ── WRITE: hierarchical element / bit select ───────────────────────────────

#[test]
fn two_seg_array_element_write() {
    let (out, _c) = run("module sub; reg [7:0] mem [0:3]; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.mem[1]=8'h42; #1 $display(\"R %h\", dut.mem[1]); end\n\
         endmodule\n");
    assert!(out.contains("R 42"), "2-seg element write:\n{out}");
}

#[test]
fn three_level_array_element_write() {
    let (out, _c) = run("module leaf; reg [7:0] mem [0:3]; endmodule\n\
         module mid; leaf l(); endmodule\n\
         module top; mid m();\n\
           initial begin m.l.mem[2]=8'h33; #1 $display(\"R %h\", m.l.mem[2]); end\n\
         endmodule\n");
    assert!(out.contains("R 33"), "3-level element write:\n{out}");
}

#[test]
fn vector_bit_select_write() {
    // dut.v[3] = 1 sets bit 3 only (v: 00 -> 08).
    let (out, _c) = run("module sub; reg [7:0] v; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.v=8'h00; dut.v[3]=1'b1; #1 $display(\"R %h\", dut.v); end\n\
         endmodule\n");
    assert!(out.contains("R 08"), "vector bit-select write:\n{out}");
}

#[test]
fn nonblocking_element_write() {
    let (out, _c) = run("module sub; reg [7:0] mem [0:3]; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.mem[0]<=8'haa; #1 $display(\"R %h\", dut.mem[0]); end\n\
         endmodule\n");
    assert!(out.contains("R aa"), "nonblocking element write:\n{out}");
}

#[test]
fn runtime_index_element_write() {
    // a runtime (variable) index: k=2 ⇒ writes mem[2].
    let (out, _c) = run("module sub; reg [7:0] mem [0:3]; endmodule\n\
         module top; sub dut(); integer k;\n\
           initial begin k=2; dut.mem[k]=8'h5c; #1 $display(\"R %h\", dut.mem[2]); end\n\
         endmodule\n");
    assert!(out.contains("R 5c"), "runtime-index element write:\n{out}");
}

#[test]
fn element_write_drives_child_logic() {
    // dut.mem[0] feeds a child continuous assign `y = mem[0]+1` — the hierarchical
    // element write must schedule the child's logic. mem[0]=10 ⇒ y=11.
    let (out, _c) = run(
        "module sub; reg [7:0] mem [0:1]; wire [7:0] y = mem[0]+1; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.mem[0]=8'h10; #1 $display(\"R %h\", dut.y); end\n\
         endmodule\n",
    );
    assert!(
        out.contains("R 11"),
        "element write must drive child logic:\n{out}"
    );
}

#[test]
fn two_dim_array_element_write() {
    let (out, _c) = run("module sub; reg [7:0] grid [0:1][0:1]; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.grid[1][0]=8'h9a; #1 $display(\"R %h\", dut.grid[1][0]); end\n\
         endmodule\n");
    assert!(out.contains("R 9a"), "2-D array element write:\n{out}");
}

#[test]
fn multidim_packed_element_write() {
    // dut.pm is `reg [1:0][7:0]`: dut.pm[1] writes the high 8-bit element.
    // Observed through the per-element read (a whole multi-dim packed read/write is
    // a separate pre-existing limitation).
    let (out, _c) = run("module sub; reg [1:0][7:0] pm; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.pm[0]=8'h00; dut.pm[1]=8'hcd; #1 $display(\"R %h\", dut.pm[1]); end\n\
         endmodule\n");
    assert!(
        out.contains("R cd"),
        "multi-dim packed element write:\n{out}"
    );
}

#[test]
fn array_element_trailing_bit_write() {
    // dut.mem[1][3] = 1 — an unpacked element plus a trailing bit-select.
    let (out, _c) = run("module sub; reg [7:0] mem [0:3]; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.mem[1]=8'h0; dut.mem[1][3]=1'b1; #1 $display(\"R %h\", dut.mem[1]); end\n\
         endmodule\n");
    assert!(
        out.contains("R 08"),
        "array element trailing-bit write:\n{out}"
    );
}

// ── guards: still loud, NOT a silent value ─────────────────────────────────

#[test]
fn hierarchical_wire_bit_write_is_loud() {
    // a procedural hierarchical bit-select write to a `wire` is loud (iverilog rejects).
    let (out, code) = run("module sub; wire [7:0] w; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.w[3]=1'b1; end\n\
         endmodule\n");
    assert!(
        is_loud(&out, code),
        "hierarchical wire bit write must be loud: {out} {code:?}"
    );
}

#[test]
fn undeclared_hierarchical_element_write_is_loud() {
    let (out, code) = run("module sub; reg [7:0] mem [0:3]; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.nope[1]=8'h1; end\n\
         endmodule\n");
    assert!(
        is_loud(&out, code),
        "undeclared hierarchical element write must be loud: {out} {code:?}"
    );
}

#[test]
fn force_on_hierarchical_bit_select_is_loud() {
    // a bit/part-select is not a legal force target (parity with the local path) — the
    // deferred sel-write must NOT silently force the wrong bits. iverilog allows it, but
    // vita's force model is whole-net only, so this is loud, NOT a silent value.
    let (out, code) = run("module sub; reg [7:0] v; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.v=8'h00; force dut.v[2]=1'b1; #1 $display(\"R %h\", dut.v); end\n\
         endmodule\n");
    assert!(
        is_loud(&out, code),
        "force on a hierarchical bit-select must be loud: {out} {code:?}"
    );
}

#[test]
fn release_on_hierarchical_element_is_loud() {
    let (out, code) = run("module sub; reg [7:0] mem [0:3]; endmodule\n\
         module top; sub dut();\n\
           initial begin release dut.mem[1]; end\n\
         endmodule\n");
    assert!(
        is_loud(&out, code),
        "release on a hierarchical element must be loud: {out} {code:?}"
    );
}

#[test]
fn hierarchical_part_select_write_round_trips() {
    // a hierarchical PART-select (range) write `dut.v[3:0] = …` is now supported
    // (HIER-REST-PS, see hier_part_write.rs); it writes exactly bits 3..0.
    let (out, _code) = run("module sub; reg [7:0] v; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.v=8'h00; dut.v[3:0]=4'hc; #1 $display(\"R %h\", dut.v); end\n\
         endmodule\n");
    assert!(
        out.contains("R 0c"),
        "hierarchical part-select write round-trips:\n{out}"
    );
}
