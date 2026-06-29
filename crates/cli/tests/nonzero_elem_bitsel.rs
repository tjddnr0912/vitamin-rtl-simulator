//! P0-NZE: a trailing BIT-SELECT on an unpacked-array ELEMENT whose packed range
//! has a non-zero LSB (`reg [11:4] mem [0:3]`) or is ascending (`reg [4:11] …`)
//! must normalize the bit index against the element's LSB — exactly like a plain
//! bit-select on a vector. The old array-element path lowered the trailing index
//! RAW (bypassing `norm_offset_for_net`), silently reading/writing the wrong bit
//! (or dropping an out-of-internal-range index). This affected BOTH the local
//! path (`mem[i][b]`) and — newly — the hierarchical element read/write
//! (HIER-REST①), which faithfully mirrors it. Fixed at all four sites; a `[N:0]`
//! (zero-based) element is a no-op, so the golden stays byte-identical.
//!
//! Every value is pinned to LIVE iverilog 13.0. Pure IR-0 (elaborate lowering).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_nze_{}_{n}", std::process::id()));
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

// ── LOCAL path ─────────────────────────────────────────────────────────────

#[test]
fn local_write_nonzero_lsb_element() {
    // reg [4:1] mem: element bit indices 4..1, so bit 2 is internal bit 1 ⇒ 0x2.
    let out = run("module top; reg [4:1] mem [0:3];\n\
         initial begin mem[1]=4'h0; mem[1][2]=1'b1; #1 $display(\"R %h\", mem[1]); end\n\
         endmodule\n");
    assert!(
        out.contains("R 2"),
        "local non-zero-LSB element write:\n{out}"
    );
}

#[test]
fn local_read_nonzero_lsb_element() {
    // reg [11:4] mem = 0x81 (internal bits 7,0): source bit 4 = internal 0 = 1,
    // source bit 11 = internal 7 = 1, source bit 7 = internal 3 = 0.
    let out = run("module top; reg [11:4] mem [0:1];\n\
         initial begin mem[0]=8'h81; #1\n\
           $display(\"b4=%b b11=%b b7=%b\", mem[0][4], mem[0][11], mem[0][7]); end\n\
         endmodule\n");
    assert!(
        out.contains("b4=1 b11=1 b7=0"),
        "local non-zero-LSB element read:\n{out}"
    );
}

// ── HIERARCHICAL path (HIER-REST①) ─────────────────────────────────────────

#[test]
fn hier_write_nonzero_lsb_element() {
    let out = run("module sub; reg [11:4] mem [0:3]; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.mem[2]=8'h00; dut.mem[2][4]=1'b1; dut.mem[2][11]=1'b1;\n\
             #1 $display(\"R %h\", dut.mem[2]); end\n\
         endmodule\n");
    assert!(
        out.contains("R 81"),
        "hier non-zero-LSB element write:\n{out}"
    );
}

#[test]
fn hier_read_3seg_nonzero_lsb_element() {
    let out = run("module leaf; reg [11:4] m [0:1]; endmodule\n\
         module mid; leaf lf(); endmodule\n\
         module top; mid md();\n\
           initial begin md.lf.m[0]=8'h81; #1\n\
             $display(\"b4=%b b11=%b b7=%b\", md.lf.m[0][4], md.lf.m[0][11], md.lf.m[0][7]); end\n\
         endmodule\n");
    assert!(
        out.contains("b4=1 b11=1 b7=0"),
        "3-seg hier non-zero-LSB element read:\n{out}"
    );
}

#[test]
fn hier_write_ascending_element() {
    // reg [4:11] mem: ascending element range, the larger index (11) is internal
    // bit 0 — bit 4 ⇒ internal 7, bit 11 ⇒ internal 0 ⇒ 0x81.
    let out = run("module sub; reg [4:11] mem [0:1]; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.mem[0]=8'h00; dut.mem[0][4]=1'b1; dut.mem[0][11]=1'b1;\n\
             #1 $display(\"R %h\", dut.mem[0]); end\n\
         endmodule\n");
    assert!(out.contains("R 81"), "hier ascending element write:\n{out}");
}

// ── regression guard: zero-based elements stay byte-identical ───────────────

#[test]
fn zero_based_element_bit_unchanged() {
    // reg [7:0] mem: bit 3 is internal bit 3 ⇒ 0x08 (the common case, a no-op for
    // the normalization — must remain exactly correct).
    let out = run("module sub; reg [7:0] mem [0:3]; endmodule\n\
         module top; sub dut();\n\
           initial begin dut.mem[1]=8'h0; dut.mem[1][3]=1'b1; #1 $display(\"R %h\", dut.mem[1]); end\n\
         endmodule\n");
    assert!(
        out.contains("R 08"),
        "zero-based element bit unchanged:\n{out}"
    );
}
