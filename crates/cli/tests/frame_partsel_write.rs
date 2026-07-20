//! EXT2-H (§4b H): a bit/part-select assignment (`r[7:0] = x`, `r[i] = b`) in a FRAME
//! function/task body (a 2-state return, `automatic`, or control-flow body) was a loud
//! E3009 — "outside the frame-call subset". The frame body only supported WHOLE writes
//! to its own locals. Now the engine read-modify-writes the frame slot for a
//! bit/part-select lvalue targeting an IN-FRAME scalar net, so `r[hi:lo] = x` /
//! `r[i] = b` / `r[i +: w] = x` work, matching iverilog. An md-packed element-write
//! (`p[0] = ..`) is covered as a special case of a part-select. A write to a net
//! OUTSIDE the function stays loud (a pre-existing frame limitation). Pinned to
//! iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fpsw_{}_{n}", std::process::id()));
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

#[test]
fn frame_static_partsel_write() {
    // Frame via a for-loop. r[7:0]=AB, r[15:8]=CD → 0xCDAB.
    let src = "module m;\n\
         function integer f;\n\
           reg [15:0] r; integer i;\n\
           for (i=0;i<1;i=i+1) begin r[7:0]=8'hAB; r[15:8]=8'hCD; end\n\
           f = r;\n\
         endfunction\n\
         initial begin $display(\"r=%0h\", f()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=cdab"), "static part-select, got:\n{out}");
}

#[test]
fn frame_bit_select_write() {
    // `automatic` frames. r[0]=1, r[3]=1 → 0x09 = 9.
    let src = "module m;\n\
         function automatic integer f;\n\
           reg [7:0] r; r = 8'h00; r[0]=1'b1; r[3]=1'b1; f = r;\n\
         endfunction\n\
         initial begin $display(\"r=%0d\", f()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=9"), "bit-select write, got:\n{out}");
}

#[test]
fn frame_dynamic_indexed_partsel_write() {
    // A runtime indexed part-select `r[i*4 +: 4]` in a loop → 0x4321.
    let src = "module m;\n\
         function integer f;\n\
           reg [15:0] r; integer i; r = 0;\n\
           for (i=0;i<4;i=i+1) r[i*4 +: 4] = i+1;\n\
           f = r;\n\
         endfunction\n\
         initial begin $display(\"r=%0h\", f()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=4321"),
        "dynamic indexed part-select, got:\n{out}"
    );
}

#[test]
fn frame_dynamic_bit_select_write() {
    // A runtime bit index `r[i]` in a loop → 0x55 (bits 0,2,4,6).
    let src = "module m;\n\
         function integer f;\n\
           reg [7:0] r; integer i; r = 0;\n\
           for (i=0;i<8;i=i+2) r[i] = 1'b1;\n\
           f = r;\n\
         endfunction\n\
         initial begin $display(\"r=%0h\", f()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=55"), "dynamic bit-select, got:\n{out}");
}

#[test]
fn frame_task_output_partsel_write() {
    // A part-select write to an `automatic` task's OUTPUT frame local → 0xCDAB.
    let src = "module m;\n\
         task automatic t(output reg [15:0] o); o = 0; o[7:0] = 8'hAB; o[15:8] = 8'hCD; endtask\n\
         reg [15:0] w;\n\
         initial begin t(w); $display(\"r=%0h\", w); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=cdab"),
        "task output part-select, got:\n{out}"
    );
}

#[test]
fn frame_partsel_oob_bits_dropped() {
    // An out-of-range part-select write drops the OOB bits (IEEE) — r[19:16] on a
    // 16-bit r writes nothing → r stays 0xFFFF.
    let src = "module m;\n\
         function integer f;\n\
           reg [15:0] r; integer i; r = 16'hFFFF;\n\
           for (i=0;i<1;i=i+1) r[19:16] = 4'h0;\n\
           f = r;\n\
         endfunction\n\
         initial begin $display(\"r=%0h\", f()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=ffff"),
        "OOB part-select dropped, got:\n{out}"
    );
}

#[test]
fn frame_mdpacked_element_write() {
    // Bonus (§4.5.83 deferred): an md-packed element-WRITE `p[0]=CD; p[1]=AB` is a
    // part-select of the flat local — now correct → 0xABCD.
    let src = "module m;\n\
         function [15:0] f;\n\
           logic [1:0][7:0] p; integer i;\n\
           for (i=0;i<1;i=i+1) begin p = 0; p[0] = 8'hCD; p[1] = 8'hAB; end\n\
           f = p;\n\
         endfunction\n\
         initial begin $display(\"r=%0h\", f()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=abcd"),
        "md-packed element write, got:\n{out}"
    );
}

#[test]
fn out_of_frame_partsel_write_still_loud() {
    // GUARD: a part-select write to a MODULE net from a frame body is NOT an in-frame
    // write — it stays loud E3009 (a pre-existing frame limitation), NOT silent.
    let src = "module m;\n\
         reg [15:0] g;\n\
         function automatic integer f; g[7:0] = 8'hAB; f = 1; endfunction\n\
         integer d;\n\
         initial begin d = f(); $display(\"g=%0h\", g); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(1), "expected loud, got code={code:?}\n{out}");
}

#[test]
fn frame_whole_write_unchanged() {
    // GUARD: a whole-net frame write is byte-identical (the non-part-select path). 0xAB.
    let src = "module m;\n\
         function automatic integer f; reg [7:0] r; r = 8'hAB; f = r; endfunction\n\
         initial begin $display(\"r=%0d\", f()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=171"), "whole write unchanged, got:\n{out}");
}

#[test]
fn frame_concat_lvalue_still_loud() {
    // BOUNDARY (review find): a concat-TARGET lvalue (`{a,b} = x`) is multi-chunk; the
    // frame write path is single-chunk — it stays loud, NOT a panic / silent chunk-0
    // write. (The per-chunk relax must not admit a concat.)
    let src = "module m;\n\
         function integer f; reg [7:0] r; integer i; r = 0;\n\
           for (i=0;i<1;i=i+1) {r[7:4], r[3:0]} = 8'hAB;\n\
           f = r; endfunction\n\
         initial begin $display(\"r=%0h\", f()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(
        code,
        Some(1),
        "concat lvalue must loud, got code={code:?}\n{out}"
    );
}

#[test]
fn frame_local_array_element_write_supported() {
    // §4.5.169 (was `..._still_loud`): a frame-local single-dim zero-based UNPACKED
    // ARRAY (`reg [7:0] mem [0:3]`) now reserves as an md-packed `[count][elem_w]` frame
    // slot (the array-FORMAL representation, `reserve_frame_local_decl` →
    // `classify_unpacked_array` Ok), so `mem[k] = x` lowers to a packed part-select
    // write + read — supported, verified vs iverilog. f = mem[1] = 0x22.
    let src = "module m;\n\
         function automatic integer f; reg [7:0] mem [0:3];\n\
           mem[0] = 8'h11; mem[1] = 8'h22; f = mem[1]; endfunction\n\
         initial begin $display(\"r=%0h\", f()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(
        code,
        Some(0),
        "local array element write now supported, got code={code:?}\n{out}"
    );
    assert!(out.contains("r=22"), "expected mem[1]=0x22, got:\n{out}");
}

#[test]
fn frame_block_local_array_element_write_supported() {
    // §4.5.169: an unpacked-array BLOCK-local (`begin: blk reg [7:0] mem [0:3]; …`) also
    // reserves md-packed — ALL frame-local reservation sites share
    // `reserve_frame_local_decl`, so `mem[k]` read/write is supported here too.
    let src = "module m;\n\
         function automatic integer f;\n\
           begin: blk reg [7:0] mem [0:3]; mem[0]=8'h11; mem[1]=8'h22; f=mem[1]; end\n\
         endfunction\n\
         initial begin $display(\"r=%0h\", f()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(
        code,
        Some(0),
        "block-local array element write now supported, got code={code:?}\n{out}"
    );
    assert!(out.contains("r=22"), "expected mem[1]=0x22, got:\n{out}");
}

#[test]
fn frame_nested_array_shadow_write_still_loud() {
    // BOUNDARY (round-2 review find): a block-local unpacked ARRAY (`int y[0:3]`) that
    // SHADOWS a same-named outer scalar (`int y`) coalesces onto the scalar net at
    // `reserve_frame_block_locals` — the fix marks that coalesced net so `y[k]=v` stays
    // loud, not a silent scalar bit-write.
    let src = "module top;\n\
         task automatic t(output int result);\n\
           int y;\n\
           begin int y[0:3]; y[2] = 9; result = y[2]; end\n\
         endtask\n\
         int r;\n\
         initial begin t(r); $display(\"r=%0d\", r); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(
        code,
        Some(1),
        "nested array shadow write must loud, got code={code:?}\n{out}"
    );
}

#[test]
fn frame_xz_index_write_dropped() {
    // An X/Z dynamic index drops the write (IEEE / vita module-path parity via
    // OOR_DROP), NOT writes bit 0. r=8'h55, r[X]=0 → r stays 0x55 (matches iverilog).
    let src = "module m;\n\
         function integer f; reg [7:0] r; logic [2:0] idx;\n\
           r = 8'h55; idx = 3'bxxx;\n\
           for (f=0;f<1;f=f+1) r[idx] = 1'b0;\n\
           f = r; endfunction\n\
         initial begin $display(\"r=%0h\", f()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=55"), "X index dropped, got:\n{out}");
}
