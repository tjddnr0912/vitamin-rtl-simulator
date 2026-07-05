//! A multi-dim PACKED array declared as a LOCAL of a class METHOD (`logic [1:0][7:0] p`)
//! was reserved (`reserve_class_method`) using only its OUTER packed dim
//! (`range_to_dims(d.range)` ignores `d.packed`) — the same net-based-reserve bug fixed
//! for frame functions/tasks/block-locals — so the slot net was 2 bits: `p = 16'hABCD`
//! truncated and `p[0]` became a 1-bit bit-select (`01`) instead of the 8-bit element
//! (`cd`), and `$bits`/`$size`/`$dimensions` were wrong. The fix reuses the shared
//! `frame_packed_width`/`register_frame_packed` helpers (full `product(packed widths)`
//! width + `packed_dims` + `dim_desc`). Pinned to iverilog 13.0. (An element-WRITE
//! lvalue `p[0]=…` was a loud E3009 here — outside the frame-call subset — and is now
//! supported by EXT2-H's frame part-select write. The inline function/task subst-bound local paths are a separate,
//! deeper follow-on. Class-method 2-state (`bit`) locals not coercing a runtime X→0 —
//! `reserve_class_method` never sets `intro_kind`, for scalar AND packed alike — is a
//! separate pre-existing gap (a different axis than width), left untouched here.)
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cmpk_{}_{n}", std::process::id()));
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

fn method_body(body: &str, fmt: &str) -> String {
    format!(
        "module m;\n\
         class C; function {body} endfunction endclass\n\
         C c; initial begin c = new; $display(\"{fmt}\", c.f()); #1 $finish; end endmodule\n"
    )
}

#[test]
fn class_method_packed_element_read() {
    // `p = 16'hABCD; p[0]` = 0xCD, `p[1]` = 0xAB (was 01 / 00).
    for (idx, want) in [("0", "cd"), ("1", "ab")] {
        let (out, code) = run(&method_body(
            &format!("[7:0] f; logic [1:0][7:0] p; begin p = 16'hABCD; f = p[{idx}]; end"),
            "%h",
        ));
        assert_eq!(code, Some(0), "{out}");
        assert!(
            out.contains(want),
            "class p[{idx}] want {want}, got:\n{out}"
        );
    }
}

#[test]
fn class_method_packed_whole_and_size() {
    let (out, code) = run(&method_body(
        "[15:0] f; logic [1:0][7:0] p; begin p = 16'hABCD; f = p; end",
        "%h",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("abcd"), "whole p, got:\n{out}");
    // `$size` = 2 (outer dim), NOT the widened net's 16.
    let (out, code) = run(&method_body(
        "[15:0] f; logic [1:0][7:0] p; begin p = 0; f = $size(p); end",
        "%0d",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains('2'), "$size(p), got:\n{out}");
}

#[test]
fn class_method_packed_signed_and_subindex() {
    let (out, code) = run(&method_body(
        "signed [7:0] f; logic signed [1:0][7:0] p; begin p = 16'h80FF; f = p[0]; end",
        "%0d",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("-1"), "signed p[0], got:\n{out}");
    let (out, code) = run(&method_body(
        "[3:0] f; logic [1:0][7:0] p; begin p = 16'hABCD; f = p[0][3:0]; end",
        "%h",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains('d'), "p[0][3:0], got:\n{out}");
}

#[test]
fn class_method_packed_scalar_unchanged() {
    let (out, code) = run(&method_body(
        "[7:0] f; logic [7:0] x; begin x = 8'hAB; f = x; end",
        "%h",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("ab"), "scalar local, got:\n{out}");
}

#[test]
fn class_method_packed_virtual_and_inherited() {
    // A `virtual` method and an inherited (non-overridden) method both resolve the
    // packed local through `reserve_class_method`.
    let (out, code) = run("module m;\n\
         class B; virtual function [7:0] f; logic [1:0][7:0] p; begin p = 16'hABCD; f = p[0]; end endfunction endclass\n\
         B b; initial begin b = new; $display(\"%h\", b.f()); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("cd"), "virtual method, got:\n{out}");
    let (out, code) = run("module m;\n\
         class B; function [7:0] f; logic [1:0][7:0] p; begin p = 16'hABCD; f = p[0]; end endfunction endclass\n\
         class D extends B; endclass\n\
         D d; initial begin d = new; $display(\"%h\", d.f()); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("cd"), "inherited method, got:\n{out}");
}

#[test]
fn class_method_packed_element_write_supported() {
    // EXT2-H (§4b H): an md-packed element-WRITE lvalue (`p[0] = 8'hCD`) is now inside
    // the frame-call subset — the engine read-modify-writes the frame slot, so p[0]
    // sets the low byte → p = 16'h00CD (was a loud E3009 before EXT2-H).
    let (out, code) = run(&method_body(
        "[15:0] f; logic [1:0][7:0] p; begin p = 0; p[0] = 8'hCD; f = p; end",
        "%h",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("00cd"),
        "p[0]=CD sets the low byte, got:\n{out}"
    );
}
