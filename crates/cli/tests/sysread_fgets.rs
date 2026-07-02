//! v9 `$fgets(str, fd)` (Medium-bundle rank 5, SYS-READ part 3a; pure IR-0).
//! A direct-rhs-of-blocking-assign special form in the $value$plusargs family:
//! it writes the str destination (arg 0) AND returns the byte count to the lhs.
//!
//! Every expected value is pinned to LIVE iverilog 13.0: $fgets reads up to the
//! destination width in WHOLE bytes (the FULL N, not C's N-1 — no NUL is
//! reserved) OR through a newline (retained), packs the bytes right-justified
//! MSB-first (first byte = most significant) with the high bytes zero-filled,
//! and returns the byte count. At EOF it returns 0 and leaves the destination
//! UNCHANGED.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str, files: &[(&str, &[u8])]) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fgets_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    for (name, bytes) in files {
        std::fs::write(d.join(name), bytes).unwrap();
    }
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
fn fgets_line_with_newline_right_justified() {
    // "hello\nworld\n" into a 128-bit reg => n=6 each, bytes in the low 6 bytes
    // MSB-first ('h' high, '\n' low), high zero-filled; 3rd call EOF=0 unchanged.
    let (out, _c) = run(
        "module t;\n\
         reg [128:1] line; integer fd, n;\n\
         initial begin\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           n = $fgets(line, fd); $display(\"n1 %0d %h\", n, line);\n\
           n = $fgets(line, fd); $display(\"n2 %0d %h\", n, line);\n\
           n = $fgets(line, fd); $display(\"n3 %0d %h\", n, line);\n\
         end\n\
         endmodule\n",
        &[("in.txt", b"hello\nworld\n")],
    );
    assert!(
        out.contains("n1 6 0000000000000000000068656c6c6f0a"),
        "fgets line 1:\n{out}"
    );
    assert!(
        out.contains("n2 6 00000000000000000000776f726c640a"),
        "fgets line 2:\n{out}"
    );
    // EOF: n=0, destination UNCHANGED (still 'world\n').
    assert!(
        out.contains("n3 0 00000000000000000000776f726c640a"),
        "fgets EOF leaves dest unchanged:\n{out}"
    );
}

#[test]
fn fgets_full_width_stops_at_newline() {
    // 4-byte dest over "ABCDEFGHIJ\nXY\n": reads the FULL 4 bytes (not 3),
    // stopping early at a newline. s1=ABCD s2=EFGH s3=IJ\n(3) s4=XY\n(3) s5=0.
    let (out, _c) = run(
        "module t;\n\
         reg [32:1] buf4; integer fd, n;\n\
         initial begin\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           n = $fgets(buf4, fd); $display(\"s1 %0d %h\", n, buf4);\n\
           n = $fgets(buf4, fd); $display(\"s2 %0d %h\", n, buf4);\n\
           n = $fgets(buf4, fd); $display(\"s3 %0d %h\", n, buf4);\n\
           n = $fgets(buf4, fd); $display(\"s4 %0d %h\", n, buf4);\n\
           n = $fgets(buf4, fd); $display(\"s5 %0d %h\", n, buf4);\n\
         end\n\
         endmodule\n",
        &[("in.txt", b"ABCDEFGHIJ\nXY\n")],
    );
    assert!(out.contains("s1 4 41424344"), "{out}"); // ABCD
    assert!(out.contains("s2 4 45464748"), "{out}"); // EFGH
    assert!(out.contains("s3 3 00494a0a"), "{out}"); // IJ\n
    assert!(out.contains("s4 3 0058590a"), "{out}"); // XY\n
    assert!(out.contains("s5 0 0058590a"), "{out}"); // EOF, unchanged
}

#[test]
fn fgets_bad_fd_leaves_dest_unchanged() {
    // a failed $fopen => fd 0; $fgets returns 0 and does NOT touch the dest.
    let (out, _c) = run(
        "module t;\n\
         reg [32:1] buf4; integer fd, n;\n\
         initial begin\n\
           buf4 = 32'hdeadbeef;\n\
           fd = $fopen(\"/no/such/path/x\", \"r\");\n\
           n = $fgets(buf4, fd); $display(\"n %0d %h\", n, buf4);\n\
         end\n\
         endmodule\n",
        &[],
    );
    assert!(
        out.contains("n 0 deadbeef"),
        "bad-fd fgets unchanged:\n{out}"
    );
}

#[test]
fn fgets_nested_placement_is_loud() {
    let (out, code) = run(
        "module t;\n\
         reg [32:1] buf4; integer fd, x;\n\
         initial begin\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           x = $fgets(buf4, fd) + 1;\n\
         end\n\
         endmodule\n",
        &[("in.txt", b"AB")],
    );
    assert!(
        out.contains("VITA-E3009") || code == Some(1),
        "nested $fgets must be loud: {out} code={code:?}"
    );
}

#[test]
fn fgets_stops_at_embedded_nul_but_consumes_line() {
    // "AB\0CD\nEF": the returned string STOPS at the first NUL (n=2, value AB),
    // yet the whole line is consumed from the stream (the next $fgetc returns
    // 'E'=69). iverilog-pinned (C fgets + NUL-terminated buffer).
    let (out, _c) = run(
        "module t;\n\
         reg [128:1] line; integer fd, n, c;\n\
         initial begin\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           n = $fgets(line, fd); $display(\"L %0d %h\", n, line);\n\
           c = $fgetc(fd); $display(\"next %0d\", c);\n\
         end\n\
         endmodule\n",
        &[("in.txt", &[0x41, 0x42, 0x00, 0x43, 0x44, 0x0a, 0x45, 0x46])],
    );
    assert!(
        out.contains("L 2 00000000000000000000000000004142"),
        "fgets stops at NUL:\n{out}"
    );
    assert!(
        out.contains("next 69"),
        "line consumed past the NUL:\n{out}"
    );
}

#[test]
fn fgets_leading_nul_clears_dest() {
    // "\0ABC": a leading NUL => empty string => n=0 AND the dest is CLEARED to
    // 0 (NOT left at its prior value — distinct from genuine EOF). iverilog-pin.
    let (out, _c) = run(
        "module t;\n\
         reg [64:1] w8; integer fd, n;\n\
         initial begin\n\
           w8 = 64'hDEADBEEFDEADBEEF;\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           n = $fgets(w8, fd); $display(\"NS %0d %h\", n, w8);\n\
         end\n\
         endmodule\n",
        &[("in.txt", &[0x00, 0x41, 0x42, 0x43])],
    );
    assert!(
        out.contains("NS 0 0000000000000000"),
        "leading NUL clears:\n{out}"
    );
}

#[test]
fn fgets_string_dest_reads_full_line() {
    // v7 string dest: a `string` is a dynamic HANDLE (net width 0). Before the
    // fix it fell into the sub-byte branch and returned 0 with an empty string
    // (silent-wrong). iverilog-pinned: each $fgets reads the WHOLE line through
    // its retained newline (no width cap), returning the byte count; the 3rd
    // call hits EOF (n=0, dest UNCHANGED).
    let (out, _c) = run(
        "module t;\n\
         string a, b, c; integer fd, n1, n2, n3;\n\
         initial begin\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           n1 = $fgets(a, fd);\n\
           n2 = $fgets(b, fd);\n\
           n3 = $fgets(c, fd);\n\
           $display(\"n1=%0d a=<%s>\", n1, a);\n\
           $display(\"n2=%0d b=<%s>\", n2, b);\n\
           $display(\"n3=%0d c=<%s>\", n3, c);\n\
         end\n\
         endmodule\n",
        &[("in.txt", b"123\n456\n")],
    );
    assert!(out.contains("n1=4 a=<123\n>"), "string line 1:\n{out}");
    assert!(out.contains("n2=4 b=<456\n>"), "string line 2:\n{out}");
    assert!(out.contains("n3=0 c=<>"), "string EOF empty:\n{out}");
}

#[test]
fn fgets_string_dest_no_trailing_newline() {
    // Last line with NO trailing newline: read to EOF, count excludes a newline
    // (n=5 for "world"), next read is a clean EOF. iverilog-pinned.
    let (out, _c) = run(
        "module t;\n\
         string a, b, c; integer fd, n1, n2, n3;\n\
         initial begin\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           n1 = $fgets(a, fd);\n\
           n2 = $fgets(b, fd);\n\
           n3 = $fgets(c, fd);\n\
           $display(\"n1=%0d a=<%s>\", n1, a);\n\
           $display(\"n2=%0d b=<%s>\", n2, b);\n\
           $display(\"n3=%0d c=<%s>\", n3, c);\n\
         end\n\
         endmodule\n",
        &[("in.txt", b"hello\nworld")],
    );
    assert!(out.contains("n1=6 a=<hello\n>"), "string line 1:\n{out}");
    assert!(
        out.contains("n2=5 b=<world>"),
        "string no-newline tail:\n{out}"
    );
    assert!(out.contains("n3=0 c=<>"), "string EOF:\n{out}");
}

#[test]
fn fgets_string_dest_eof_leaves_unchanged() {
    // Empty file / immediate EOF: n=0 and the string keeps its PRIOR value
    // (NOT cleared — distinct from a sub-byte reg dest). iverilog-pinned.
    let (out, _c) = run(
        "module t;\n\
         string a = \"PRESET\"; integer fd, n1;\n\
         initial begin\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           n1 = $fgets(a, fd);\n\
           $display(\"n1=%0d a=<%s>\", n1, a);\n\
         end\n\
         endmodule\n",
        &[("in.txt", b"")],
    );
    assert!(
        out.contains("n1=0 a=<PRESET>"),
        "string EOF leaves dest unchanged:\n{out}"
    );
}

#[test]
fn fgets_string_dest_truncates_at_nul() {
    // C-string semantics (iverilog parity, mirroring the reg path): the STORED
    // string and the returned count stop at the first NUL, though the whole line
    // is consumed from the stream. A leading NUL => n=0 and dest CLEARED to "".
    // Bytes: "ab\0cd\n" then "ef\n" — line 1 truncates at the NUL (n=2 "ab"),
    // line 2 is a normal read proving the stream advanced past the whole line 1.
    let (out, _c) = run(
        "module t;\n\
         string a, b; integer fd, n1, n2; int la;\n\
         initial begin\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           n1 = $fgets(a, fd); la = a.len();\n\
           n2 = $fgets(b, fd);\n\
           $display(\"n1=%0d la=%0d a=<%s>\", n1, la, a);\n\
           $display(\"n2=%0d b=<%s>\", n2, b);\n\
         end\n\
         endmodule\n",
        &[(
            "in.txt",
            &[0x61, 0x62, 0x00, 0x63, 0x64, 0x0a, 0x65, 0x66, 0x0a],
        )],
    );
    assert!(out.contains("n1=2 la=2 a=<ab>"), "truncate at NUL:\n{out}");
    // stream advanced past the whole NUL line => next read is "ef\n".
    assert!(
        out.contains("n2=3 b=<ef\n>"),
        "stream consumed past NUL:\n{out}"
    );
}

#[test]
fn fgets_string_dest_leading_nul_clears() {
    // A leading NUL (bytes "\0xyz\n") => n=0 and the string is set to EMPTY
    // (overwriting a preset value) — distinct from genuine EOF, which leaves the
    // dest unchanged. iverilog-pinned.
    let (out, _c) = run(
        "module t;\n\
         string a = \"PRE\"; integer fd, n1; int la;\n\
         initial begin\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           n1 = $fgets(a, fd); la = a.len();\n\
           $display(\"n1=%0d la=%0d a=<%s>\", n1, la, a);\n\
         end\n\
         endmodule\n",
        &[("in.txt", &[0x00, 0x78, 0x79, 0x7a, 0x0a])],
    );
    assert!(
        out.contains("n1=0 la=0 a=<>"),
        "leading NUL clears string to empty:\n{out}"
    );
}

#[test]
fn fgets_sub_byte_dest_clears_without_consuming() {
    // a dest narrower than one byte (reg[3:0]): iverilog reads NO stream byte
    // but CLEARS the dest to 0 (n=0); the next $fgetc still returns byte 1.
    let (out, _c) = run(
        "module t;\n\
         reg [3:0] nib; integer fd, n, c;\n\
         initial begin\n\
           nib = 4'ha;\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           n = $fgets(nib, fd); $display(\"NB %0d %h\", n, nib);\n\
           c = $fgetc(fd); $display(\"nbnext %0d\", c);\n\
         end\n\
         endmodule\n",
        &[("in.txt", &[0x01, 0x02, 0x03])],
    );
    assert!(out.contains("NB 0 0"), "sub-byte dest cleared:\n{out}");
    assert!(out.contains("nbnext 1"), "no stream byte consumed:\n{out}");
}
