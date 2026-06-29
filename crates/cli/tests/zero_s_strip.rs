//! `%0s` (minimal-width string) on a packed reg strips the LEADING NUL bytes — the
//! high zero-byte padding of a string stored in a wider reg — rather than rendering
//! them as spaces. Plain `%s` pads to the full reg width (NUL → space); only the
//! `0` flag triggers the strip. After the first non-NUL byte, every later byte is
//! emitted (an embedded or trailing NUL still becomes a space); an all-NUL value
//! yields the empty string. vita previously ignored the `0` flag for `%s` and
//! always padded — a silent-wrong. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_z0s_{}_{n}", std::process::id()));
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
fn zero_s_strips_leading_nul() {
    // "hi" in an 8-byte reg: `%0s` → "hi" (leading NULs dropped); `%s` → "      hi".
    let out = run("module top; reg [8*8:1] s; initial begin s=\"hi\"; \
         $display(\"a[%0s]b[%s]\",s,s); $finish; end endmodule\n");
    assert!(
        out.contains("a[hi]b[      hi]"),
        "%0s strips, %s pads; got:\n{out}"
    );
}

#[test]
fn zero_s_all_nul_is_empty() {
    // All-NUL → "" under %0s, "    " (spaces) under %s.
    let out = run("module top; reg [8*4:1] s; initial begin s=0; \
         $display(\"a[%0s]b[%s]\",s,s); $finish; end endmodule\n");
    assert!(out.contains("a[]b[    ]"), "all-NUL %0s empty; got:\n{out}");
}

#[test]
fn zero_s_trailing_nul_becomes_space() {
    // "hi\0\0" (0x68690000): trailing NULs are NOT stripped — they render as spaces.
    let out = run("module top; reg [8*4:1] s; initial begin s=32'h68690000; \
         $display(\"[%0s]\",s); $finish; end endmodule\n");
    assert!(out.contains("[hi  ]"), "trailing NUL → space; got:\n{out}");
}

#[test]
fn zero_s_embedded_nul_after_leading() {
    // "\0h\0i" (0x00680069): leading NUL stripped, embedded NUL → space → "h i".
    let out = run("module top; reg [8*4:1] s; initial begin s=32'h00680069; \
         $display(\"[%0s]\",s); $finish; end endmodule\n");
    assert!(
        out.contains("[h i]"),
        "lead stripped, embedded → space; got:\n{out}"
    );
}

#[test]
fn xz_bytes_render_as_space() {
    // x AND z bytes both render as a space (a NUL): iverilog masks unknown bits off.
    // vita's z-bit encoding is val=1, which previously leaked 0xFF — fixed by masking
    // x/z off before reading the byte, in BOTH `%s` and `%0s`. `16'hzz41` → `%s` " A",
    // `%0s` "A" (leading z stripped). All-z → `%0s` "", `%s` "    ".
    let out = run(
        "module top; reg [15:0] s; reg [31:0 ]az; initial begin s=16'hzz41; az=32'hzzzzzzzz; \
         $display(\"a[%0s]b[%s]c[%0s]\",s,s,az); $finish; end endmodule\n",
    );
    assert!(
        out.contains("a[A]b[ A]c[]"),
        "x/z bytes → space; got:\n{out}"
    );
}

#[test]
fn zero_s_string_type_unchanged() {
    // A dynamic `string` renders exact bytes — %0s is a no-op for it.
    let out = run("module top; string s; initial begin s=\"hi\"; \
         $display(\"[%0s]\",s); $finish; end endmodule\n");
    assert!(out.contains("[hi]"), "string-type %0s; got:\n{out}");
}

#[test]
fn zero_s_exact_fit_unchanged() {
    // An exactly-sized packed reg has no leading NULs — %0s == %s.
    let out = run("module top; reg [8*2:1] s; initial begin s=\"hi\"; \
         $display(\"a[%0s]b[%s]\",s,s); $finish; end endmodule\n");
    assert!(out.contains("a[hi]b[hi]"), "exact-fit %0s==%s; got:\n{out}");
}
