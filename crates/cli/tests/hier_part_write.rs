//! HIER-REST-PS: a hierarchical PART-select (range / indexed) WRITE — `dut.v[3:0]=…`,
//! `dut.v[o+:w]=…`, `dut.v[o-:w]=…`, and the array-element form `dut.mem[i][3:0]=…`.
//! The hierarchical part-select READ already worked; the WRITE was a loud follow-on.
//! It defers like the element/bit write (HIER-REST①), carrying the lowered offset +
//! const width, and rebuilds the part-select chunk (offset normalized against the
//! net's LSB) once the target net is known. Pure IR-0 (elaborate lowering only).
//!
//! Every value is pinned to LIVE iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_hpw_{}_{n}", std::process::id()));
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
fn const_part_select_write() {
    let out = run("module sub; reg [7:0] v; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.v=8'h00; dut.v[3:0]=4'ha; #1 $display(\"R %h\", dut.v); end\n\
         endmodule\n");
    assert!(out.contains("R 0a"), "const part-select write:\n{out}");
}

#[test]
fn const_part_select_write_mid() {
    // dut.v[5:2] = f ⇒ bits 5..2 set ⇒ 0x3c.
    let out = run("module sub; reg [7:0] v; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.v=8'h00; dut.v[5:2]=4'hf; #1 $display(\"R %h\", dut.v); end\n\
         endmodule\n");
    assert!(out.contains("R 3c"), "mid part-select write:\n{out}");
}

#[test]
fn indexed_part_select_plus_write() {
    // dut.v[4+:3] = 7 ⇒ bits 6..4 set ⇒ 0x70.
    let out = run("module sub; reg [7:0] v; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.v=8'h00; dut.v[4+:3]=3'h7; #1 $display(\"R %h\", dut.v); end\n\
         endmodule\n");
    assert!(out.contains("R 70"), "indexed +: part-select write:\n{out}");
}

#[test]
fn indexed_part_select_minus_write() {
    // dut.v[6-:3] = 7 ⇒ bits 6..4 set ⇒ 0x70.
    let out = run("module sub; reg [7:0] v; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.v=8'h00; dut.v[6-:3]=3'h7; #1 $display(\"R %h\", dut.v); end\n\
         endmodule\n");
    assert!(out.contains("R 70"), "indexed -: part-select write:\n{out}");
}

#[test]
fn nonzero_lsb_part_select_write() {
    // reg [15:8] v: part-select [11:8] normalizes against LSB 8 ⇒ internal bits 3..0
    // ⇒ 0x0d.
    let out = run("module sub; reg [15:8] v; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.v=8'h00; dut.v[11:8]=4'hd; #1 $display(\"R %h\", dut.v); end\n\
         endmodule\n");
    assert!(
        out.contains("R 0d"),
        "non-zero-LSB part-select write:\n{out}"
    );
}

#[test]
fn three_level_part_select_write() {
    let out = run("module leaf; reg [7:0] v; endmodule\n\
         module mid; leaf l(); endmodule\n\
         module top; mid m();\n\
           initial begin m.l.v=8'h00; m.l.v[3:0]=4'hc; #1 $display(\"R %h\", m.l.v); end\n\
         endmodule\n");
    assert!(out.contains("R 0c"), "3-level part-select write:\n{out}");
}

#[test]
fn array_element_part_select_write() {
    // dut.mem[1][3:0] = 9 — array element word + a part-select within it.
    let out = run("module sub; reg [7:0] mem [0:3]; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.mem[1]=8'h00; dut.mem[1][3:0]=4'h9; #1 $display(\"R %h\", dut.mem[1]); end\n\
         endmodule\n");
    assert!(
        out.contains("R 09"),
        "array-element part-select write:\n{out}"
    );
}
