//! An element-select `s[i]` of a `string` FORMAL in a FRAME function (one that
//! lowers via the frame path — a 2-state return `int`/`bit`/`byte`, an `automatic`
//! function, or any multi-statement / control-flow body) was silently wrong: the
//! string formal lowers to a 1-bit wire slot (its literal actual is materialized as
//! a string Value in the frame slot via the `str_params` mask), and `s[i]` fell
//! through to a packed BIT-select on that wire → 0 (or X in an arithmetic context).
//!
//! Fix (companion to §4.5.93's inline path): elaborate `handle_is_str_readable` now
//! accepts any word-less Signal handle (the base is already string-confirmed by
//! `expr_is_string_ast`), so a frame string formal routes to the `.getc(i)` byte
//! primitive; and the engine `handle_str_bytes` reads a non-string-net Signal's
//! bytes via an `eval` fallback (the frame slot holds the materialized string
//! Value). The result matches iverilog. A frame string LOCAL is a separate gap
//! (`s = "..."` is a loud E3018 — a string can't be assigned to a frame local net).
//! Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_frmstr_{}_{n}", std::process::id()));
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
fn frame_int_return_formal_index() {
    // 2-state `int` return forces the frame path. s[0] = 'A' = 65 (was silently 0).
    let src = "module m;\n\
         function int h(input string s); h = s[0]; endfunction\n\
         initial begin $display(\"r=%0d\", h(\"ABCD\")); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=65"), "frame int(string) s[0], got:\n{out}");
}

#[test]
fn frame_bit_return_formal_index() {
    // 2-state `bit [7:0]` return → frame. s[1] = 'B' = 66.
    let src = "module m;\n\
         function bit [7:0] h(input string s); h = s[1]; endfunction\n\
         initial begin $display(\"r=%0d\", h(\"ABCD\")); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=66"), "frame bit(string) s[1], got:\n{out}");
}

#[test]
fn frame_automatic_formal_index() {
    // `automatic` forces the frame path even with a 4-state return. s[0] = 65.
    let src = "module m;\n\
         function automatic integer h(input string s); h = s[0]; endfunction\n\
         initial begin $display(\"r=%0d\", h(\"ABCD\")); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=65"), "frame automatic s[0], got:\n{out}");
}

#[test]
fn frame_formal_index_arithmetic() {
    // The regression the §4.5.93 review flagged: `s[0]+s[1]` in a frame function used
    // to poison to X (the byte was unreadable). Now 'A'+'B' = 65+66 = 131.
    let src = "module m;\n\
         function bit [8:0] f(input string s); f = s[0] + s[1]; endfunction\n\
         initial begin $display(\"r=%0d\", f(\"ABCD\")); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=131"), "frame s[0]+s[1], got:\n{out}");
}

#[test]
fn frame_formal_oob_index_zero() {
    // OOB reads 0 (IEEE §6.16.2), in the frame path too.
    let src = "module m;\n\
         function int h(input string s); h = s[10]; endfunction\n\
         initial begin $display(\"r=%0d\", h(\"ABCD\")); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=0"), "frame OOB s[10], got:\n{out}");
}

#[test]
fn frame_formal_runtime_index() {
    // Runtime index into a frame string formal. s[3] = 'D' = 68.
    let src = "module m;\n\
         function int h(input string s, input int i); h = s[i]; endfunction\n\
         initial begin $display(\"r=%0d\", h(\"ABCD\", 3)); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=68"), "frame runtime index, got:\n{out}");
}

#[test]
fn frame_formal_loop_checksum() {
    // `s[i]` inside a for-loop (multi-statement → frame). sum of 'A'..'D' = 266.
    let src = "module m;\n\
         function int h(input string s);\n\
           int i; int acc; acc = 0;\n\
           for (i = 0; i < 4; i = i + 1) acc = acc + s[i];\n\
           h = acc;\n\
         endfunction\n\
         initial begin $display(\"r=%0d\", h(\"ABCD\")); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=266"), "frame loop checksum, got:\n{out}");
}

#[test]
fn frame_string_compare_unchanged() {
    // GUARD: a frame string formal WHOLE compare already worked (reads the frame
    // slot's materialized string) — must stay correct. ("ABCD" == "ABCD") = 1.
    let src = "module m;\n\
         function int h(input string s); h = (s == \"ABCD\"); endfunction\n\
         initial begin $display(\"r=%0d\", h(\"ABCD\")); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=1"), "frame string compare, got:\n{out}");
}

#[test]
fn frame_string_local_now_supported() {
    // round-14 V1: a frame string LOCAL is now a real `NetKind::String` heap slot
    // (`frame_local_net_kind`), procedurally assignable, and its byte/method reads
    // route through the engine's frame-aware `str_bytes` — the boundary guard that
    // previously asserted E3018-loud here has flipped to supported. s[1] of "ABCD"
    // = 'B' = 66. (Was the pre-existing "still loud" gap the follow-on comment named.)
    let src = "module m;\n\
         function int h; string s; s = \"ABCD\"; h = s[1]; endfunction\n\
         initial begin $display(\"r=%0d\", h()); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(
        code,
        Some(0),
        "expected supported, got code={code:?}\n{out}"
    );
    assert!(
        out.contains("r=66"),
        "frame string-local index, got:\n{out}"
    );
}
