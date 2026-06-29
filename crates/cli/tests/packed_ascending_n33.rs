//! N3.3 — ascending (little-endian) packed-dim direction (`reg [0:3][7:0]`).
//!
//! PRE-FIX BUG (silent-wrong): `flatten_word`/`flatten_word_eids` computed the packed
//! coordinate as `idx - lo` (lo = min endpoint), DISCARDING the declared direction. For
//! a descending `[3:0]` dim that is correct (idx 0 = LSB byte); for an ASCENDING `[0:3]`
//! dim it is wrong — iverilog maps idx 0 to the MSB byte. So `reg [0:3][7:0] x;
//! x=32'hAABBCCDD; x[0]` returned DD (vita) vs AA (iverilog), with NO diagnostic. The
//! defect was array-independent (plain multi-dim packed AND array-of-packed) and shared
//! by the hierarchical read path (`flatten_word_eids`).
//!
//! FIX (IR-0): record per-packed-dim `ascending` in `packed_dims`/`packed_extents` and,
//! for an ascending dim, compute `coord = hi - idx` (hi = lo+size-1 = the lsb endpoint),
//! mirroring `norm_offset_for_net` for plain vectors. Descending dims and the unpacked
//! array path are byte-identical (they pass an empty `ascending` slice ⇒ all `false`).
//! Engine/sim-ir/format_version unchanged.
//!
//! ORACLE: iverilog 13.0 (every expected value below verified live).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pa33_{}_{n}", std::process::id()));
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
fn ascending_2dim_read_idx0_is_msb_byte() {
    // [0:3] ascending ⇒ x[0]=MSB byte (AA) … x[3]=LSB byte (DD).
    let out = run("module top;\n\
         reg [0:3][7:0] x;\n\
         initial begin\n\
           x=32'hAABBCCDD;\n\
           $display(\"x0=%h x1=%h x2=%h x3=%h\", x[0],x[1],x[2],x[3]);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("x0=aa x1=bb x2=cc x3=dd"), "asc read:\n{out}");
}

#[test]
fn ascending_2dim_write_hits_msb_byte() {
    // x[0]=8'hEE writes the MSB byte (offset 24, width 8) ⇒ EEBBCCDD.
    let out = run("module top;\n\
         reg [0:3][7:0] x;\n\
         initial begin\n\
           x=32'hAABBCCDD;\n\
           x[0]=8'hEE;\n\
           $display(\"x=%h\", x);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("x=eebbccdd"), "asc write:\n{out}");
}

#[test]
fn ascending_variable_index() {
    // dynamic index on an ascending dim: i=1 ⇒ coord 2 ⇒ byte BB.
    let out = run("module top;\n\
         reg [0:3][7:0] x;\n\
         integer i;\n\
         initial begin x=32'hAABBCCDD; i=1; $display(\"v=%h\", x[i]); end\n\
         endmodule\n");
    assert!(out.contains("v=bb"), "asc var index (i=1 ⇒ BB):\n{out}");
}

#[test]
fn descending_byte_identical_regression() {
    // [3:0] descending stays exactly as before (idx 0 = LSB byte) — byte-identity guard.
    let out = run("module top;\n\
         reg [3:0][7:0] x;\n\
         initial begin x=32'hAABBCCDD; $display(\"x0=%h x3=%h\", x[0],x[3]); end\n\
         endmodule\n");
    assert!(out.contains("x0=dd x3=aa"), "desc regression:\n{out}");
}

#[test]
fn mixed_ascending_outer_descending_inner() {
    // [0:1] ascending outer, [7:0] descending inner: x[0]=MSB byte AA, x[1]=BB.
    let out = run("module top;\n\
         reg [0:1][7:0] x;\n\
         initial begin x=16'hAABB; $display(\"x0=%h x1=%h\", x[0],x[1]); end\n\
         endmodule\n");
    assert!(out.contains("x0=aa x1=bb"), "asc-outer desc-inner:\n{out}");
}

#[test]
fn ascending_outer_and_inner_bitselect() {
    // [0:1][0:7] both ascending. x[0]=AA byte; within it bit[0]=MSB(1), bit[7]=LSB(0).
    let out = run("module top;\n\
         reg [0:1][0:7] x;\n\
         initial begin\n\
           x=16'hAABB;\n\
           $display(\"x0=%h x0b0=%b x0b7=%b\", x[0], x[0][0], x[0][7]);\n\
         end\n\
         endmodule\n");
    assert!(
        out.contains("x0=aa x0b0=1 x0b7=0"),
        "asc inner bit-select:\n{out}"
    );
}

#[test]
fn descending_outer_ascending_inner_bitselect() {
    // [1:0] desc outer, [0:7] asc inner. x[0]=LSB byte BB; x[0][0]=MSB of BB = 1.
    let out = run("module top;\n\
         reg [1:0][0:7] x;\n\
         initial begin\n\
           x=16'hAABB;\n\
           $display(\"x0=%h x1=%h x0b0=%b\", x[0], x[1], x[0][0]);\n\
         end\n\
         endmodule\n");
    assert!(
        out.contains("x0=bb x1=aa x0b0=1"),
        "desc-outer asc-inner:\n{out}"
    );
}

#[test]
fn three_dim_mixed_direction() {
    // [0:1][0:1][7:0]: asc/asc/desc. c=01020304 ⇒ c[0][0][0]=1, c[0][1][1]=1, c[1][0][1]=1.
    let out = run("module top;\n\
         reg [0:1][0:1][7:0] c;\n\
         initial begin\n\
           c=32'h01020304;\n\
           $display(\"c=%b %b %b\", c[0][0][0], c[0][1][1], c[1][0][1]);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("c=1 1 1"), "3-dim mixed direction:\n{out}");
}

#[test]
fn hierarchical_ascending_read() {
    // flatten_word_eids (hierarchical read path) is fixed too: dut.x[0]=MSB byte AA.
    let out = run(
        "module child; reg [0:3][7:0] x; initial x=32'hAABBCCDD; endmodule\n\
         module top;\n\
           child dut();\n\
           initial #1 $display(\"hx0=%h hx3=%h\", dut.x[0], dut.x[3]);\n\
         endmodule\n",
    );
    assert!(
        out.contains("hx0=aa hx3=dd"),
        "hierarchical asc read:\n{out}"
    );
}
