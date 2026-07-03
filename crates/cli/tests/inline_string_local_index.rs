//! An element-select `s[i]` of a `string` LOCAL or FORMAL inside an INLINE
//! (non-`automatic`, straight-line) function was silently wrong: the inline path
//! binds the local by SUBSTITUTION to its string value, and `s[i]` fell through to
//! a packed BIT-select on that handle — yielding 0 (bit 0) instead of the byte at i.
//!
//! Two fixes: (A) elaborate `string_index_read` recognizes an inline string base
//! (a declared-`string` formal/local, via `formal_str`/`subst`, exactly as the
//! string compare/method paths do) and routes `s[i]` to the `.getc(i)` byte
//! primitive; (B) the engine's `handle_str_bytes` reads a string-valued `Const`
//! operand (an inlined local bound to a string literal is a compile-time const, not
//! a runtime string net) so `.getc` works on it. The result now matches module-level
//! `s[i]` exactly. Pinned to iverilog 13.0. A non-string packed local element-select
//! (a legitimate bit/part-select) is unaffected.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_inlstr_{}_{n}", std::process::id()));
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
fn inline_local_string_literal_index() {
    // (A)+(B) inline string LOCAL bound to a literal (a Const), byte-select.
    // s = "ABCD"; s[0] = 'A' = 65 (was silently 0).
    let src = "module m;\n\
         function integer h; string s; s = \"ABCD\"; h = s[0]; endfunction\n\
         initial begin $display(\"r=%0d\", h()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=65"), "inline local s[0], got:\n{out}");
}

#[test]
fn inline_formal_string_index() {
    // (A)+(B) inline string FORMAL bound to a literal actual, byte-select → 'A' = 65.
    let src = "module m;\n\
         function integer h(input string s); h = s[0]; endfunction\n\
         initial begin $display(\"r=%0d\", h(\"ABCD\")); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=65"), "inline formal s[0], got:\n{out}");
}

#[test]
fn inline_local_from_runtime_string_index() {
    // (A) inline string LOCAL sourced from a runtime string NET (a Signal handle),
    // byte-select. s = g; s[1] = 'B' = 66.
    let src = "module m;\n\
         string g;\n\
         function integer h; string s; s = g; h = s[1]; endfunction\n\
         initial begin g = \"ABCD\"; $display(\"r=%0d\", h()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=66"), "runtime-sourced s[1], got:\n{out}");
}

#[test]
fn inline_string_runtime_index() {
    // Runtime (formal) index into a const-string local. s[2] = 'C' = 67.
    let src = "module m;\n\
         function integer h(input integer i); string s; s = \"ABCD\"; h = s[i]; endfunction\n\
         initial begin $display(\"r=%0d\", h(2)); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=67"), "runtime index s[2], got:\n{out}");
}

#[test]
fn inline_string_oob_index_zero() {
    // IEEE §6.16.2: an out-of-range string index reads 0.
    let src = "module m;\n\
         function integer h; string s; s = \"ABCD\"; h = s[10]; endfunction\n\
         initial begin $display(\"r=%0d\", h()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=0"), "OOB s[10], got:\n{out}");
}

#[test]
fn inline_string_index_in_concat() {
    // `s[i]` used inside a larger expression (concat of two bytes) — the byte-select
    // composes. {s[0], s[1]} = {'A','B'} = 0x4142 = 16706.
    let src = "module m;\n\
         function integer h; string s; s = \"ABCD\"; h = {s[0], s[1]}; endfunction\n\
         initial begin $display(\"r=%0d\", h()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=16706"), "concat {{s[0],s[1]}}, got:\n{out}");
}

#[test]
fn inline_string_reassigned_index() {
    // Innermost-wins subst binding: a re-assigned string local indexes the LATEST
    // value. s = "ABCD"; s = "WXYZ"; s[0] = 'W' = 87.
    let src = "module m;\n\
         function integer h; string s; s = \"ABCD\"; s = \"WXYZ\"; h = s[0]; endfunction\n\
         initial begin $display(\"r=%0d\", h()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=87"), "reassigned s[0], got:\n{out}");
}

#[test]
fn inline_packed_local_bitselect_unchanged() {
    // GUARD: a non-string packed local element-select is a legitimate bit/part-select
    // and must be byte-identical (string_index_read returns None → plain select).
    // logic [31:0] s = 0x41424344; s[7:0] = 0x44 = 68.
    let src = "module m;\n\
         function integer h; logic [31:0] s; s = 32'h41424344; h = s[7:0]; endfunction\n\
         initial begin $display(\"r=%0d\", h()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=68"),
        "packed local part-select, got:\n{out}"
    );
}

#[test]
fn module_string_index_unchanged() {
    // GUARD: module-level string byte-select (the pre-existing working path) is
    // unaffected by the engine `handle_str_bytes` change. s[0] = 'A' = 65.
    let src = "module m;\n\
         string s; integer r;\n\
         initial begin s = \"ABCD\"; r = s[0]; $display(\"r=%0d\", r); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=65"), "module s[0], got:\n{out}");
}
