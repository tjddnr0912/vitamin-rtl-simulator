//! §4.5.188 UARR: an `input` unpacked-FIXED array formal on a TASK (`byte b[4]`,
//! `logic [63:0] w[80]`) — the same md-packed `[count][elem_w]` frame slot the
//! FUNCTION path already uses (`reserve_frame_func` / `emit_frame_call`). The slot
//! is a pass-by-VALUE value (no heap), so it works on BOTH the suspendable-frame
//! and the synchronous `run_task_call` path — which unblocks combinational TB
//! helpers (round-16: `shaN_compute` / `hex2bytes` / `shaN_block`, the single
//! largest remaining task class).
//!
//! iverilog 13.0 rejects unpacked-dimension subroutine ports outright ("Subroutine
//! ports with unpacked dimensions are not yet supported"), so the differential
//! oracle is an ELEMENT-WISE reference computing the same values; the array-formal
//! mechanism is vita's, the arithmetic is what iverilog pins.
//!
//! Correct-or-loud boundary: a size-mismatched actual is loud. (§4.5.203 later
//! FORCE-FRAMES a STATIC task with such a formal, so it too is now supported — see
//! `static_task_array_formal.rs`.)
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_utf_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

// ── suspendable automatic task reads elements of an input byte[4] (ref: 10/1/4) ──
#[test]
fn auto_task_unpk_input_sum() {
    let o = run("module top;\n\
         task automatic pt(input byte b[4]);\n\
           int s; s = 0;\n\
           for (int i = 0; i < 4; i++) s += b[i];\n\
           $display(\"sum=%0d first=%0d last=%0d\", s, b[0], b[3]);\n\
         endtask\n\
         byte data[4];\n\
         initial begin\n\
           data[0]=1; data[1]=2; data[2]=3; data[3]=4;\n\
           pt(data);\n\
         end\n\
         endmodule\n");
    assert!(o.contains("sum=10 first=1 last=4"), "got:\n{o}");
}

// ── NON-suspendable task (no timing), output scalar (ref: 26) ──
#[test]
fn nonsusp_task_unpk_input_output_scalar() {
    let o = run("module top;\n\
         task automatic pt(input byte b[4], output int s);\n\
           s = 0;\n\
           for (int i = 0; i < 4; i++) s += b[i];\n\
         endtask\n\
         byte data[4]; int r;\n\
         initial begin\n\
           data[0]=5; data[1]=6; data[2]=7; data[3]=8;\n\
           pt(data, r);\n\
           $display(\"r=%0d\", r);\n\
         end\n\
         endmodule\n");
    assert!(o.contains("r=26"), "got:\n{o}");
}

// ── SIGNED elements: a negative byte reads back negative (ref: 2 / -5 / 10 / -3) ──
#[test]
fn signed_byte_elements() {
    let o = run("module top;\n\
         task automatic pt(input byte b[3]);\n\
           int s; s = 0;\n\
           for (int i = 0; i < 3; i++) s += b[i];\n\
           $display(\"sum=%0d b0=%0d b1=%0d b2=%0d\", s, b[0], b[1], b[2]);\n\
         endtask\n\
         byte data[3];\n\
         initial begin\n\
           data[0]=-5; data[1]=10; data[2]=-3;\n\
           pt(data);\n\
         end\n\
         endmodule\n");
    assert!(o.contains("sum=2 b0=-5 b1=10 b2=-3"), "got:\n{o}");
}

// ── wide 64-bit element (ref: 0x1+0x100+0x10000+0xABCD) ──
#[test]
fn wide_logic64_elements() {
    let o = run("module top;\n\
         task automatic pt(input logic [63:0] w[4]);\n\
           longint unsigned s; s = 0;\n\
           for (int i = 0; i < 4; i++) s += w[i];\n\
           $display(\"sum=%0d w0=%h w3=%h\", s, w[0], w[3]);\n\
         endtask\n\
         logic [63:0] data[4];\n\
         initial begin\n\
           data[0]=64'h1; data[1]=64'h100; data[2]=64'h10000; data[3]=64'hABCD;\n\
           pt(data);\n\
         end\n\
         endmodule\n");
    // 1 + 256 + 65536 + 43981 = 109774
    assert!(
        o.contains("sum=109774 w0=0000000000000001 w3=000000000000abcd"),
        "got:\n{o}"
    );
}

// ── pass-by-VALUE: writing an input formal element does NOT leak to the caller ──
#[test]
fn input_element_write_no_leak() {
    let o = run("module top;\n\
         task automatic pt(input byte b[3]);\n\
           b[0] = 99;\n\
           $display(\"inside b0=%0d\", b[0]);\n\
         endtask\n\
         byte d[3];\n\
         initial begin\n\
           d[0]=1; d[1]=2; d[2]=3;\n\
           pt(d);\n\
           $display(\"after d0=%0d\", d[0]);\n\
         end\n\
         endmodule\n");
    assert!(o.contains("inside b0=99"), "got:\n{o}");
    assert!(o.contains("after d0=1"), "leaked to caller:\n{o}");
}

// ── two calls with different arrays stay isolated (ref: 3 / 30 / 3) ──
#[test]
fn two_calls_isolated() {
    let o = run("module top;\n\
         task automatic pt(input byte b[2]);\n\
           $display(\"s=%0d\", b[0] + b[1]);\n\
         endtask\n\
         byte x[2]; byte y[2];\n\
         initial begin\n\
           x[0]=1; x[1]=2; y[0]=10; y[1]=20;\n\
           pt(x); pt(y); pt(x);\n\
         end\n\
         endmodule\n");
    assert!(o.contains("s=3") && o.contains("s=30"), "got:\n{o}");
}

// ── §4.5.193: an OUTPUT unpacked-fixed array formal is now supported (md-packed slot
// → packed-temp out-bind → post-call unpack). Detailed coverage lives in
// frame_task_output_array.rs; this asserts the former loud is gone. ──
#[test]
fn output_array_formal_now_supported() {
    let o = run("module top;\n\
         task automatic mk(output byte b[3]);\n\
           b[0]=7; b[1]=8; b[2]=9;\n\
         endtask\n\
         byte d[3];\n\
         initial begin mk(d); $display(\"%0d %0d %0d\", d[0], d[1], d[2]); end\n\
         endmodule\n");
    assert!(o.contains("7 8 9"), "got:\n{o}");
}

// ── §4.5.203: a STATIC (non-framed) task's unpacked-fixed formal is now SUPPORTED —
// the task is FORCE-FRAMED (`build_task_frame_set`) so the md-packed slot backs the
// formal (frame ⊇ inline, §4.5.198/199/200). See `static_task_array_formal.rs`. ──
#[test]
fn static_task_unpk_formal_now_supported() {
    let o = run("module top;\n\
         task pt(input byte b[4]);\n\
           $display(\"%0d\", b[0]);\n\
         endtask\n\
         byte d[4];\n\
         initial begin d[0]=1; d[1]=2; d[2]=3; d[3]=4; pt(d); end\n\
         endmodule\n");
    assert!(
        o.contains('1'),
        "static task unpk formal (force-framed):\n{o}"
    );
}

// ── a size-mismatched actual is loud (shape check) ──
#[test]
fn size_mismatch_is_loud() {
    let o = run("module top;\n\
         task automatic pt(input byte b[4]);\n\
           $display(\"%0d\", b[0]);\n\
         endtask\n\
         byte d[3];\n\
         initial begin d[0]=1; d[1]=2; d[2]=3; pt(d); end\n\
         endmodule\n");
    assert!(o.contains("E3009"), "should be loud:\n{o}");
}
