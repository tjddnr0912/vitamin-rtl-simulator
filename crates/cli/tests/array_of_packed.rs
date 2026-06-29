//! N3.2 — LOCAL array-of-packed sub-element read/write (`qm[i][j]`).
//!
//! An array whose ELEMENT is a multi-dim packed vector (`reg [3:0][7:0] qm [0:1]`)
//! is recorded in BOTH `unpacked_array_nets` (it is a static array) AND `packed_dims`
//! (the element has packed dims). A sub-element select `qm[i][j]` must therefore pick
//! the j-th packed sub-vector (here the j-th 8-bit byte), NOT a single bit.
//!
//! PRE-FIX BUG (silent-wrong): the local read/write lowering treated the trailing
//! index as a single bit-select (`qm[0][0]` read 1 bit, `qm[0][1]=8'hEE` wrote 1 bit),
//! diverging from iverilog. N3.1-followon already loud-rejected this on the HIERARCHICAL
//! read path; the LOCAL path is the original pre-existing gap on BOTH read and write.
//!
//! FIX (IR-0): route the trailing indices through the packed-dim flatten
//! (`flatten_word(packed_dims, trailing)` offset + product-of-remaining width,
//! `SelKind::PartIdxUp`), mirroring `lower_packed_read`/`collect_packed_write` but with
//! `word: Some(element)` as the base. Engine needs no change (`{word:Some, PartIdxUp}`
//! already lands at `base + off` in `write_chunk`).
//!
//! ORACLE: iverilog 13.0 supports array-of-packed sub-element select — used as the
//! differential oracle for both DESCENDING and (since N3.3) ASCENDING packed dims.
//! N3.3 made `flatten_word` direction-aware (`coord = hi - i` for a little-endian
//! `[lo:hi]` dim), so array-of-packed and plain `lower_packed_read` are both
//! iverilog-correct AND byte-consistent (see `ascending_matches_plain_packed_n33`).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_aop_{}_{n}", std::process::id()));
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
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

#[test]
fn read_byte_descending_matches_iverilog() {
    // reg [3:0][7:0] qm: index j selects byte j (descending [3:0], lo=0 ⇒ offset j*8).
    // qm[0]=AABBCCDD ⇒ qm[0][0]=DD … qm[0][3]=AA; qm[1]=11223344 ⇒ qm[1][1]=33.
    let (out, _err, _c) = run("module top;\n\
         reg [3:0][7:0] qm [0:1];\n\
         integer j;\n\
         initial begin\n\
           qm[0]=32'hAABBCCDD; qm[1]=32'h11223344;\n\
           $write(\"Q:\"); for(j=0;j<4;j=j+1) $write(\" %h\", qm[0][j]); $display(\"\");\n\
           $display(\"q11=%h\", qm[1][1]);\n\
         end\n\
         endmodule\n");
    assert!(
        out.contains("Q: dd cc bb aa"),
        "byte read (1-bit bug):\n{out}"
    );
    assert!(out.contains("q11=33"), "byte read elem1:\n{out}");
}

#[test]
fn write_byte_descending_matches_iverilog() {
    // qm[0][1]=8'hEE writes the FULL byte (offset 8, width 8), not 1 bit.
    let (out, _err, _c) = run("module top;\n\
         reg [3:0][7:0] qm [0:1];\n\
         initial begin\n\
           qm[0]=32'hAABBCCDD;\n\
           qm[0][1]=8'hEE;\n\
           $display(\"qm0=%h\", qm[0]);\n\
         end\n\
         endmodule\n");
    assert!(
        out.contains("qm0=aabbeedd"),
        "byte write (1-bit bug):\n{out}"
    );
}

#[test]
fn bit_of_byte_full_index_is_one_bit() {
    // qm[i][j][k]: full index depth (d=1 + 1 packed dim consumed leaves a byte; the
    // 3rd index selects a bit of that byte). qm[0][0][0]=bit0(DD)=1, qm[0][3][0]=bit0(AA)=0.
    let (out, _err, _c) = run("module top;\n\
         reg [3:0][7:0] qm [0:1];\n\
         initial begin\n\
           qm[0]=32'hAABBCCDD;\n\
           $display(\"b=%b %b\", qm[0][0][0], qm[0][3][0]);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("b=1 0"), "bit-of-byte full index:\n{out}");
}

#[test]
fn three_dim_packed_element_read_write_matches_iverilog() {
    // reg [1:0][1:0][7:0] cube [0:1]: element is 2x2 bytes (32-bit). Strides 16/8/1.
    // cube[0]=01020304 ⇒ cube[0][0][0]=04, cube[0][1][1]=01, cube[1][0][1]=0c.
    // write cube[0][1][0]=FF (offset 16, byte2) ⇒ 01ff0304.
    let (out, _err, _c) = run("module top;\n\
         reg [1:0][1:0][7:0] cube [0:1];\n\
         initial begin\n\
           cube[0]=32'h01020304; cube[1]=32'h0a0b0c0d;\n\
           $display(\"c=%h %h %h\", cube[0][0][0], cube[0][1][1], cube[1][0][1]);\n\
           cube[0][1][0]=8'hFF;\n\
           $display(\"cw=%h\", cube[0]);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("c=04 01 0c"), "3-dim packed elem read:\n{out}");
    assert!(
        out.contains("cw=01ff0304"),
        "3-dim packed elem write:\n{out}"
    );
}

#[test]
fn variable_index_byte_select() {
    // dynamic packed sub-index (offset = i*8 evaluated at runtime).
    let (out, _err, _c) = run("module top;\n\
         reg [3:0][7:0] qm [0:1];\n\
         integer i;\n\
         initial begin\n\
           qm[0]=32'hAABBCCDD; i=2;\n\
           $display(\"v=%h\", qm[0][i]);\n\
         end\n\
         endmodule\n");
    assert!(
        out.contains("v=bb"),
        "variable byte index (i=2 ⇒ byte2=BB):\n{out}"
    );
}

#[test]
fn bit_of_bit_over_index_is_loud() {
    // qm[0][1][2][3]: 3 trailing indices > 2 packed dims ⇒ loud (not silent X).
    let (_out, err, _c) = run("module top;\n\
         reg [3:0][7:0] qm [0:1];\n\
         initial begin\n\
           qm[0]=32'h0;\n\
           $display(\"%b\", qm[0][1][2][3]);\n\
         end\n\
         endmodule\n");
    assert!(
        err.contains("bit-select then bit-select") || err.contains("VITA-E3009"),
        "over-index must be loud:\n{err}"
    );
}

#[test]
fn whole_element_read_is_unchanged() {
    // qm[i] (no sub-index) = whole 32-bit element — regression guard.
    let (out, _err, _c) = run("module top;\n\
         reg [3:0][7:0] qm [0:1];\n\
         initial begin\n\
           qm[0]=32'hAABBCCDD;\n\
           $display(\"w=%h\", qm[0]);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("w=aabbccdd"), "whole-element read:\n{out}");
}

#[test]
fn plain_byte_array_bit_select_is_unchanged() {
    // reg [7:0] mem [0:3] is NOT array-of-packed (no extra packed dims) ⇒ mem[i][j]
    // stays a 1-BIT select (byte-identical to pre-fix). mem[0]=A5 ⇒ bit0=1, bit7=1.
    let (out, _err, _c) = run("module top;\n\
         reg [7:0] mem [0:3];\n\
         initial begin\n\
           mem[0]=8'hA5;\n\
           $display(\"m=%b %b\", mem[0][0], mem[0][7]);\n\
         end\n\
         endmodule\n");
    assert!(
        out.contains("m=1 1"),
        "plain byte-array bit-select regression:\n{out}"
    );
}

#[test]
fn ascending_matches_plain_packed_n33() {
    // N3.3: ASCENDING packed `[0:3]` — index 0 is the MSB byte (iverilog: am[0][0]=AA,
    // am[0][3]=DD for AABBCCDD). array-of-packed reuses the same `flatten_word`, so it
    // is BOTH iverilog-correct AND byte-consistent with plain packed (am[0][j]===pa[j]).
    let (out, _err, _c) = run("module top;\n\
         reg [0:3][7:0] am [0:1];\n\
         reg [0:3][7:0] pa;\n\
         initial begin\n\
           am[0]=32'hAABBCCDD; pa=32'hAABBCCDD;\n\
           $display(\"a0=%h p0=%h a3=%h p3=%h\", am[0][0], pa[0], am[0][3], pa[3]);\n\
         end\n\
         endmodule\n");
    // iverilog-correct (ascending: idx 0 = MSB byte) AND consistent (am === pa).
    assert!(
        out.contains("a0=aa p0=aa a3=dd p3=dd"),
        "ascending array-of-packed must match iverilog + plain packed:\n{out}"
    );
}
