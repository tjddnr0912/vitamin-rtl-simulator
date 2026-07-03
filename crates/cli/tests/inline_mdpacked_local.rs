//! A multi-dim PACKED local in an INLINE (non-`automatic`, straight-line) function
//! was silently wrong: the inline path binds a local by SUBSTITUTION with only the
//! OUTER packed-dim width (`logic [1:0][7:0] p` ⇒ width 2, not 16), so a whole-value
//! assign (`p = 16'hABCD`) truncated to 2 bits, and an element-select (`p[0]`) folded
//! as a bit-select on that flat value — both wrong.
//!
//! Two fixes: (A) `reduce_function_body` gives an md-packed local its FULL flat width
//! (product of packed dims) in `local_dims`, so a whole-value assign is not truncated;
//! (B) a function that ELEMENT-SELECTS an md-packed local is routed to the (correct)
//! frame path when self-contained (`build_frame_set`), else loud-rejected (a
//! non-local-reading body can't be framed without dropping that read from an
//! always_comb sensitivity list). Pinned to iverilog 13.0. A 1-D `logic [7:0] p; p[0]`
//! is a legitimate bit-select and is unaffected.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_inlmd_{}_{n}", std::process::id()));
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
fn mdpacked_whole_use_full_width() {
    // (A) whole-value assign+read must NOT truncate to the outer dim: p = 0xABCD → h = 0xABCD.
    let src = "module m;\n\
         function integer h; logic [1:0][7:0] p; p = 16'hABCD; h = p; endfunction\n\
         initial begin $display(\"r=%h\", h()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=0000abcd"), "whole-use width, got:\n{out}");
}

#[test]
fn mdpacked_element_read_routed_to_frame() {
    // (B) element-select of a self-contained function → frame path → correct element
    // (byte), not a bit-select. p[0] = byte0 = 0xCD, p[1] = byte1 = 0xAB.
    for (idx, want) in [("0", "000000cd"), ("1", "000000ab")] {
        let src = format!(
            "module m;\n\
             function integer h; logic [1:0][7:0] p; p = 16'hABCD; h = p[{idx}]; endfunction\n\
             initial begin $display(\"r=%h\", h()); #1 $finish; end endmodule\n"
        );
        let (out, code) = run(&src);
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains(&format!("r={want}")), "p[{idx}], got:\n{out}");
    }
}

#[test]
fn mdpacked_global_reader_element_select_loud() {
    // (B) a body that reads a NON-local (module net) can't be routed to frame without
    // a sensitivity regression — loud-reject rather than fold the wrong element.
    let src = "module m;\n\
         logic [15:0] gv;\n\
         function integer h; logic [1:0][7:0] p; p = gv; h = p[0]; endfunction\n\
         initial begin gv = 16'hABCD; $display(\"r=%h\", h()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(
        code,
        Some(1),
        "expected loud reject, got code={code:?}\n{out}"
    );
    assert!(!out.contains("r="), "must not silently fold, got:\n{out}");
}

#[test]
fn one_d_packed_bitselect_unchanged() {
    // A 1-D packed local element-select IS a legitimate bit-select — must NOT be
    // rerouted/rejected. p[0] of 8'hAB = bit 0 = 1.
    let src = "module m;\n\
         function integer h; logic [7:0] p; p = 8'hAB; h = p[0]; endfunction\n\
         initial begin $display(\"r=%h\", h()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=00000001"), "1-D bit-select, got:\n{out}");
}

#[test]
fn mdpacked_element_write_loud() {
    // An element-select WRITE (`p[k] = ..`) to an md-packed local is loud in both the
    // inline and frame subsets (not reducible / outside the frame subset).
    let src = "module m;\n\
         function integer h; logic [1:0][7:0] p; p[0] = 8'hcd; p[1] = 8'hab; h = p[0]; endfunction\n\
         initial begin $display(\"r=%h\", h()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(1), "expected loud, got code={code:?}\n{out}");
    assert!(
        !out.contains("r="),
        "must not silently compute, got:\n{out}"
    );
}

#[test]
fn plain_local_arith_unaffected() {
    // GUARD: a plain 1-D local in an inline function is byte-identical (no md-packed).
    let src = "module m;\n\
         function integer f; logic [7:0] a; a = 8'h20; f = a + 8'h05; endfunction\n\
         initial begin $display(\"r=%0d\", f()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=37"), "plain local arith, got:\n{out}");
}
