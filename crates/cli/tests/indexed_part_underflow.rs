//! P0-IPU: an indexed part-select WRITE whose window extends BELOW bit 0 (or below
//! the net's declared LSB) — `v[-2+:4]=…`, `v[1-:3]=…`, runtime `v[k-:3]` — must
//! write ONLY the in-range bits, dropping the out-of-range low bits (exactly like
//! the READ side and iverilog). The old engine path used unsigned/saturating
//! offset arithmetic: a negative `+:` offset wrapped to a huge u32 (whole write
//! dropped), and a `-:` low-bound `saturating_sub`bed to 0 (the window shifted UP),
//! both silent-wrong. Fixed in `write_chunk` by keeping the low net-bit position
//! signed (i64), mirroring `eval_select`. The positive top-overflow side was
//! already correct. sim-engine runtime fix (no IR/golden change).
//!
//! Every value is pinned to LIVE iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ipu_{}_{n}", std::process::id()));
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
fn plus_colon_underflow_write() {
    // v[-2+:4] selects bits {-2,-1,0,1}; only 0,1 are in range. rhs=1111 ⇒ net
    // bit0=rhs[2], bit1=rhs[3] ⇒ 0x03 (NOT 0x00).
    let out = run("module t; reg [7:0] v;\n\
         initial begin v=8'h00; v[-2+:4]=4'hF; #1 $display(\"R %h\", v); end\n\
         endmodule\n");
    assert!(out.contains("R 03"), "+: underflow write:\n{out}");
}

#[test]
fn minus_colon_underflow_write() {
    // v[1-:3] selects bits {1,0,-1}; only 0,1 in range ⇒ 0x03 (NOT 0x07).
    let out = run("module t; reg [7:0] v;\n\
         initial begin v=8'h00; v[1-:3]=3'h7; #1 $display(\"R %h\", v); end\n\
         endmodule\n");
    assert!(out.contains("R 03"), "-: underflow write:\n{out}");
}

#[test]
fn runtime_index_underflow_write() {
    let out = run("module t; reg [7:0] v; integer k;\n\
         initial begin v=8'h00; k=1; v[k-:3]=3'h7; #1 $display(\"R %h\", v); end\n\
         endmodule\n");
    assert!(out.contains("R 03"), "runtime -: underflow write:\n{out}");
}

#[test]
fn underflow_write_preserves_other_bits() {
    // v starts 0xF0; v[-2+:4]=F writes only bits 0,1, leaving the high nibble ⇒ 0xf3.
    let out = run("module t; reg [7:0] v;\n\
         initial begin v=8'hF0; v[-2+:4]=4'hF; #1 $display(\"R %h\", v); end\n\
         endmodule\n");
    assert!(
        out.contains("R f3"),
        "underflow write must preserve other bits:\n{out}"
    );
}

#[test]
fn hierarchical_underflow_write() {
    let out = run("module sub; reg [7:0] v; endmodule\n\
         module top; sub d();\n\
           initial begin d.v=8'h00; d.v[-1+:3]=3'h7; #1 $display(\"R %h\", d.v); end\n\
         endmodule\n");
    assert!(out.contains("R 03"), "hierarchical underflow write:\n{out}");
}

#[test]
fn nonzero_lsb_underflow_write() {
    // reg [15:8] w: w[9-:3] selects source bits {9,8,7}; bit 7 is below LSB 8 ⇒
    // only 9,8 written ⇒ internal bits {1,0} ⇒ field 0x03.
    let out = run("module sub; reg [15:8] w; endmodule\n\
         module top; sub d();\n\
           initial begin d.w=8'h00; d.w[9-:3]=3'h7; #1 $display(\"R %h\", d.w); end\n\
         endmodule\n");
    assert!(out.contains("R 03"), "non-zero-LSB underflow write:\n{out}");
}

#[test]
fn in_range_and_top_overflow_unchanged() {
    // regression guard: the in-range and positive top-overflow cases are unchanged.
    let a = run("module t; reg [7:0] v;\n\
         initial begin v=8'h00; v[0+:3]=3'h7; #1 $display(\"R %h\", v); end\n\
         endmodule\n");
    assert!(a.contains("R 07"), "in-range +: unchanged:\n{a}");
    let b = run("module t; reg [7:0] v;\n\
         initial begin v=8'h00; v[6+:4]=4'hF; #1 $display(\"R %h\", v); end\n\
         endmodule\n");
    assert!(b.contains("R c0"), "top-overflow unchanged:\n{b}");
}
