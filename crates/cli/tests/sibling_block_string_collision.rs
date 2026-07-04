//! Two sibling `begin…end` blocks that declare a local with the SAME NAME but
//! CONFLICTING string/non-string types (`begin string s; … end  begin logic [7:0]
//! s; … end`) were silently wrong: v1 flattens block-locals into one name-keyed
//! binding — for an INLINE (straight-line, 4-state-return) function the flat
//! name-keyed `formal_str` is collected UPFRONT (innermost-wins), so one block's
//! local silently adopts the other's classification (a string `s[i]` folds as a
//! bit-select where a byte is meant, or vice versa) regardless of order or use. v1
//! has no per-block-scope inline namespace, so this is now a loud reject (rename
//! one) rather than a miscompute.
//!
//! Scoped to the inline path: the module/process path (`hoist_block_local_nets`)
//! coalesces per net, so a write-only conflicting local there is harmless — that
//! (read-gated) case is a separate follow-on. NOT rejected here (unaffected):
//! same-name reuse with the SAME type (a common `for`-temp idiom), a single
//! block-local, and packed-WIDTH-only collisions (a separate deferred axis). Pinned
//! to iverilog 13.0 for the non-rejected cases.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sibstr_{}_{n}", std::process::id()));
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
fn inline_sibling_string_then_packed_loud() {
    // b1 `string s`, b2 `logic [7:0] s` in an INLINE (integer-return) function.
    // Was silently 1 (b1's string s[0] folded as a bit-select → 0); now loud.
    let src = "module m;\n\
         function integer h;\n\
           integer r1; integer r2;\n\
           begin : b1 string s; s = \"AB\"; r1 = s[0]; end\n\
           begin : b2 logic [7:0] s; s = 8'h41; r2 = s[0]; end\n\
           h = r1 * 1000 + r2;\n\
         endfunction\n\
         initial begin $display(\"r=%0d\", h()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(1), "expected loud, got code={code:?}\n{out}");
    assert!(
        !out.contains("r="),
        "must not silently compute, got:\n{out}"
    );
}

#[test]
fn inline_sibling_packed_then_string_loud() {
    // Reverse order — also silently wrong before (both misclassified), now loud.
    let src = "module m;\n\
         function integer h;\n\
           integer r1; integer r2;\n\
           begin : b1 logic [7:0] s; s = 8'h41; r1 = s[0]; end\n\
           begin : b2 string s; s = \"AB\"; r2 = s[0]; end\n\
           h = r1 * 1000 + r2;\n\
         endfunction\n\
         initial begin $display(\"r=%0d\", h()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(1), "expected loud, got code={code:?}\n{out}");
    assert!(
        !out.contains("r="),
        "must not silently compute, got:\n{out}"
    );
}

#[test]
fn inline_sibling_same_type_ok() {
    // GUARD: same-name reuse with the SAME type is safe (blocks never overlap) —
    // must NOT be rejected. b1 s[0] of 0x41 = 1, b2 s[7] of 0x80 = 1 → 11.
    let src = "module m;\n\
         function integer h;\n\
           integer r1; integer r2;\n\
           begin : b1 logic [7:0] s; s = 8'h41; r1 = s[0]; end\n\
           begin : b2 logic [7:0] s; s = 8'h80; r2 = s[7]; end\n\
           h = r1 * 10 + r2;\n\
         endfunction\n\
         initial begin $display(\"r=%0d\", h()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=11"), "same-type sibling reuse, got:\n{out}");
}

#[test]
fn inline_sibling_same_string_name_ok() {
    // GUARD: two string block-locals of the same name (both string, no conflict) —
    // must WORK. b1 "AB"[0]='A'=65, b2 "CD"[0]='C'=67 → 65067.
    let src = "module m;\n\
         function integer h;\n\
           integer r1; integer r2;\n\
           begin : b1 string s; s = \"AB\"; r1 = s[0]; end\n\
           begin : b2 string s; s = \"CD\"; r2 = s[0]; end\n\
           h = r1 * 1000 + r2;\n\
         endfunction\n\
         initial begin $display(\"r=%0d\", h()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=65067"),
        "same-string-name reuse, got:\n{out}"
    );
}

#[test]
fn inline_single_string_block_local_ok() {
    // GUARD: a single string block-local (no sibling) is unaffected. "AB"[0] = 65.
    let src = "module m;\n\
         function integer h;\n\
           integer r;\n\
           begin : b1 string s; s = \"AB\"; r = s[0]; end\n\
           h = r;\n\
         endfunction\n\
         initial begin $display(\"r=%0d\", h()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=65"),
        "single string block-local, got:\n{out}"
    );
}

#[test]
fn inline_sibling_packed_width_not_rejected() {
    // GUARD: a packed-WIDTH-only collision (no string) is a separate deferred axis —
    // the string/handle detection must NOT reject it. b1 8-bit 0xFF = 255, b2 16-bit
    // 0xFFFF = 65535 → 65790 (values fit here; the truncating sub-case is a
    // documented follow-on, not this slice).
    let src = "module m;\n\
         function integer h;\n\
           integer r1; integer r2;\n\
           begin : b1 logic [7:0] s; s = 8'hFF; r1 = s; end\n\
           begin : b2 logic [15:0] s; s = 16'hFFFF; r2 = s; end\n\
           h = r1 + r2;\n\
         endfunction\n\
         initial begin $display(\"r=%0d\", h()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=65790"), "packed-width sibling, got:\n{out}");
}
