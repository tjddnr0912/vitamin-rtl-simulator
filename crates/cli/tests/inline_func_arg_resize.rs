//! An INLINE (static, simple-body) function's INPUT formal is bound to its actual
//! by SUBSTITUTION. The actual was previously substituted at its own width, so a
//! NARROW actual was NOT extended to the formal's width — a later part-select of the
//! formal (`c[7:4]`) read the un-filled high bits as X (silent-wrong). iverilog
//! (like an automatic/frame function) assigns the actual to the formal, extending
//! it to the port width. The fix resizes each actual to the formal's WIDTH at the
//! inline binding (`reduce_function_body`), extending by the ACTUAL's own sign
//! (§11.6.1), via the return/local-assign helper `resize_inline_assign`.
//!
//! A width-0 heap-HANDLE formal (string/class/event) is bound verbatim — a bit-
//! resize would corrupt the handle (NetKind discriminator precedes the width path).
//!
//! Pinned to iverilog 13.0. NOTE: the fix deliberately does NOT re-stamp the
//! formal's declared SIGNEDNESS onto the value — that is a separate deferred gap
//! (the inline path evaluates body sub-expressions at self-width, not the
//! assignment context width, so a `signed + unsigned` or a forced-signed overflow
//! would still be wrong). Applying formal signedness needs that context-width work.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_inlarg_{}_{n}", std::process::id()));
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
fn narrow_unsigned_actual_zero_extends() {
    // 4-bit actual -> 8-bit unsigned formal; `c[7:4]` must read 0, not X. iverilog 07.
    let (out, code) = run("module m;\n\
         function [7:0] f(input [7:0] c); f = {c[7:4], c[3:0]}; endfunction\n\
         initial begin $display(\"%02h\", f(4'h7)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("07"),
        "narrow actual must zero-extend, got:\n{out}"
    );
}

#[test]
fn narrow_signed_actual_sign_extends() {
    // 4-bit signed -1 -> 8-bit SIGNED formal; sign-extends to 8'hFF. iverilog 00ff.
    let (out, code) = run("module m;\n\
         function [15:0] f(input signed [7:0] c); f = {8'h0, c[7:0]}; endfunction\n\
         initial begin $display(\"%04h\", f(-4'sd1)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("00ff"),
        "narrow signed actual must sign-extend, got:\n{out}"
    );
}

#[test]
fn wide_actual_truncates() {
    // 16-bit actual -> 8-bit formal; truncates to the low byte. iverilog cd.
    let (out, code) = run("module m;\n\
         function [7:0] f(input [7:0] c); f = c; endfunction\n\
         initial begin $display(\"%02h\", f(16'hABCD)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("cd"), "{out}");
}

#[test]
fn atom_formal_narrow_actual() {
    // A `byte` (8-bit atom) formal with a 4-bit actual — extends to 8. iverilog 07.
    let (out, code) = run("module m;\n\
         function [7:0] f(input byte c); f = {c[7:4], c[3:0]}; endfunction\n\
         initial begin $display(\"%02h\", f(4'h7)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("07"), "{out}");
}

#[test]
fn nested_inline_call() {
    // The extension applies through a nested inline call `f(g(4'h7))`. iverilog 07.
    let (out, code) = run("module m;\n\
         function [7:0] g(input [7:0] x); g = x; endfunction\n\
         function [7:0] f(input [7:0] c); f = {c[7:4], c[3:0]}; endfunction\n\
         initial begin $display(\"%02h\", f(g(4'h7))); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("07"), "{out}");
}

#[test]
fn multi_arg_one_narrow() {
    // Two formals, one narrow actual. iverilog: a[7:4]=0, b[3:0]=5 -> 05.
    let (out, code) = run("module m;\n\
         function [7:0] f(input [7:0] a, input [7:0] b); f = a[7:4] + b[3:0]; endfunction\n\
         initial begin $display(\"%02h\", f(4'h7, 8'hA5)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("05"), "{out}");
}

#[test]
fn exact_width_actual_byte_identical() {
    // An exact-width actual is unchanged by the resize (byte-identity). iverilog ab.
    let (out, code) = run("module m;\n\
         function [7:0] f(input [7:0] c); f = {c[7:4], c[3:0]}; endfunction\n\
         initial begin $display(\"%02h\", f(8'hAB)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("ab"), "{out}");
}

#[test]
fn string_formal_bound_verbatim() {
    // A `string` formal is a heap handle, NOT a bit-vector — it must be bound
    // verbatim (the resize is skipped for width-0 handle kinds), so a relational
    // compare through the formal still works. A string VARIABLE actual is used (a
    // string-LITERAL actual hits a separate pre-existing gap). iverilog: lt=1, lt2=0.
    let (out, code) = run("module m;\n\
         string s; integer r;\n\
         function integer strlt(input string a, input string b); strlt = (a < b); endfunction\n\
         initial begin s = \"ab\"; r = strlt(s, \"b\"); $display(\"lt=%0d\", r);\n\
         r = strlt(s, \"aa\"); $display(\"lt2=%0d\", r); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("lt=1") && out.contains("lt2=0"),
        "string formal corrupted:\n{out}"
    );
}

#[test]
fn automatic_function_unchanged() {
    // An `automatic` function uses the frame path (already correct) — unchanged.
    let (out, code) = run("module m;\n\
         function automatic [7:0] f(input [7:0] c); f = {c[7:4], c[3:0]}; endfunction\n\
         initial begin $display(\"%02h\", f(4'h7)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("07"), "{out}");
}
