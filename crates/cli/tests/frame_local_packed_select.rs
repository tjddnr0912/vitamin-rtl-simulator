//! A multi-dim PACKED array declared as a LOCAL of a FRAME (automatic / recursive /
//! 2-state-return) function — `logic [1:0][7:0] p` — was reserved using only its OUTER
//! packed dimension (`range_to_dims(range)` ignores `d.packed`), so the frame slot net
//! was 2 bits wide: `p = 16'hABCD` truncated to 2 bits and an element read `p[0]` became
//! a 1-bit bit-select (`01`) instead of the 8-bit element (`cd`). The fix mirrors the
//! module-scope decl: widen to `product(packed widths)` and register `packed_dims` so
//! `p[i]` lowers through `lower_packed_read` to the element-width part-select. Module
//! scope was always correct. The same fix (shared `frame_packed_width` /
//! `register_frame_packed` helpers) covers a frame TASK body local and a `begin…end`
//! BLOCK local, and registers `dim_desc` so `$bits`/`$size`/`$dimensions`/`$left`/`$right`
//! are correct too. Pinned to iverilog 13.0. (An element-WRITE lvalue `p[0]=…` was a
//! loud E3009 here — outside the frame-call subset — and is now supported by EXT2-H's
//! frame part-select write.)
//!
//! Pre-existing residuals (branch==main, NOT touched here — each a separate path that
//! also `range_to_dims`-truncates a multi-dim packed local): a NON-automatic INLINE task
//! (`hoist_inline_task_locals`), an INLINE function body local (`fold_straight_line`),
//! and a class METHOD local (`reserve_class_method`); plus a packed+UNPACKED frame local
//! (`logic [1:0][7:0] p [0:1]`) whose `$bits` still ignores the unpacked dim.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_flpk_{}_{n}", std::process::id()));
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
fn frame_local_packed_element_read() {
    // `p = 16'hABCD; p[0]` = low element 0xCD, `p[1]` = high element 0xAB (was 01 / 00).
    for (idx, want) in [("0", "cd"), ("1", "ab")] {
        let src = format!(
            "module m;\n\
             function automatic [7:0] f; logic [1:0][7:0] p; begin p = 16'hABCD; f = p[{idx}]; end endfunction\n\
             initial begin $display(\"%h\", f()); #1 $finish; end endmodule\n"
        );
        let (out, code) = run(&src);
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains(want), "p[{idx}] want {want}, got:\n{out}");
    }
}

#[test]
fn frame_local_packed_three_elem() {
    // 3-element packed: `q = 24'hAABBCC; q[2]` = 0xAA (top element).
    let (out, code) = run("module m;\n\
         function automatic [7:0] f; logic [2:0][7:0] q; begin q = 24'hAABBCC; f = q[2]; end endfunction\n\
         initial begin $display(\"%h\", f()); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("aa"), "q[2], got:\n{out}");
}

#[test]
fn frame_local_packed_whole_net() {
    // The whole net is now the full 16 bits: `p = 16'hABCD; f = p` = 0xABCD (was truncated).
    let (out, code) = run("module m;\n\
         function automatic [15:0] f; logic [1:0][7:0] p; begin p = 16'hABCD; f = p; end endfunction\n\
         initial begin $display(\"%h\", f()); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("abcd"), "whole p, got:\n{out}");
}

#[test]
fn frame_local_packed_outer_part_select() {
    // A `[1:0]` part-select over the OUTER packed dim selects whole 8-bit elements.
    let (out, code) = run("module m;\n\
         function automatic [15:0] f; logic [1:0][7:0] p; begin p = 16'hABCD; f = p[1:0]; end endfunction\n\
         initial begin $display(\"%h\", f()); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("abcd"), "p[1:0], got:\n{out}");
}

#[test]
fn frame_local_packed_signed_element() {
    // Element read carries the declared sign: `p = 16'h80FF; p[0]` = 0xFF = -1 signed.
    let (out, code) = run("module m;\n\
         function automatic signed [7:0] f; logic signed [1:0][7:0] p; begin p = 16'h80FF; f = p[0]; end endfunction\n\
         initial begin $display(\"%0d\", f()); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("-1"), "signed p[0], got:\n{out}");
}

#[test]
fn frame_local_packed_sub_index() {
    // A bit-slice INTO the element: `p[0][3:0]` = low nibble of element 0 (0xCD → d).
    let (out, code) = run("module m;\n\
         function automatic [3:0] f; logic [1:0][7:0] p; begin p = 16'hABCD; f = p[0][3:0]; end endfunction\n\
         initial begin $display(\"%h\", f()); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("d"), "p[0][3:0], got:\n{out}");
}

#[test]
fn frame_local_packed_recursion() {
    // Framed via recursion — the packed local element read still resolves per call.
    let (out, code) = run("module m;\n\
         function automatic [7:0] f(input [7:0] n); logic [1:0][7:0] p;\n\
         begin p = 16'hABCD; if (n==0) f = p[0]; else f = f(n-1); end endfunction\n\
         initial begin $display(\"%h\", f(3)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("cd"), "recursive p[0], got:\n{out}");
}

#[test]
fn scalar_frame_local_unchanged() {
    // A non-packed scalar frame local is unaffected (no `d.packed` ⇒ old width path).
    let (out, code) = run("module m;\n\
         function automatic [7:0] f; logic [7:0] x; begin x = 8'hAB; f = x; end endfunction\n\
         initial begin $display(\"%h\", f()); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("ab"), "scalar local, got:\n{out}");
}

#[test]
fn frame_local_packed_element_write_supported() {
    // EXT2-H (§4b H): an md-packed element-WRITE lvalue (`p[0] = 8'hCD`) — a part-select
    // of the flat frame local — is now inside the frame-call subset (the engine
    // read-modify-writes the frame slot). p[0] sets the low byte → p = 16'h00CD (was a
    // loud E3009 before EXT2-H).
    let (out, code) = run("module m;\n\
         function automatic [15:0] f; logic [1:0][7:0] p; begin p = 16'h0000; p[0] = 8'hCD; f = p; end endfunction\n\
         initial begin $display(\"%h\", f()); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("00cd"),
        "p[0]=CD sets the low byte, got:\n{out}"
    );
}

#[test]
fn frame_task_packed_element_read() {
    // An `automatic` (frame) TASK body local — the same fix via `reserve_frame_task`.
    let (out, code) = run("module m; logic [7:0] r;\n\
         task automatic t(output [7:0] o); logic [1:0][7:0] p; begin p = 16'hABCD; o = p[1]; end endtask\n\
         initial begin t(r); $display(\"%h\", r); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("ab"), "frame task p[1], got:\n{out}");
}

#[test]
fn frame_block_local_packed_element_read() {
    // A packed array declared inside a `begin…end` block of a frame function — the same
    // fix via `reserve_frame_block_locals`.
    let (out, code) = run("module m;\n\
         function automatic [7:0] f; begin: blk logic [1:0][7:0] p; p = 16'hABCD; f = p[0]; end endfunction\n\
         initial begin $display(\"%h\", f()); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("cd"), "block-local p[0], got:\n{out}");
}

#[test]
fn frame_local_packed_dim_sysfuncs() {
    // `dim_desc` registration: `$bits`=16, `$size`=2 (outer dim, NOT the widened net's
    // 16 — the pre-fix coincidental-2 must not regress to 16), `$dimensions`=2.
    for (sys, want) in [("$bits", "16"), ("$size", "2"), ("$dimensions", "2")] {
        let src = format!(
            "module m;\n\
             function automatic [15:0] f; logic [1:0][7:0] p; begin p = 16'hABCD; f = {sys}(p); end endfunction\n\
             initial begin $display(\"%0d\", f()); #1 $finish; end endmodule\n"
        );
        let (out, code) = run(&src);
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains(want), "{sys}(p) want {want}, got:\n{out}");
    }
}
