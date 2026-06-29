//! v9 file-read primitives `$fgetc` / `$feof` / `$ungetc` (Medium-bundle
//! rank 5, SYS-READ; pure IR-0 — the SysFuncId variants exist from the v9
//! shape bump). Each is a direct-rhs-of-blocking-assign special form (the
//! $fopen StmtEffect family) because reading/advancing the fd read position is
//! a side effect that the pure eval funnel cannot model.
//!
//! Every expected value is pinned to LIVE iverilog 13.0 (`iverilog -g2012` +
//! `vvp`), including the corrections live-probing forced over the design's
//! initial assumptions:
//!   - `$fgetc` at EOF returns -1 and stays -1; `$feof` is LAZY (set by the
//!     read that returns -1, not by exhausting the data);
//!   - `$feof` on a bad/closed fd returns -1 (NOT 0);
//!   - `$ungetc` returns 0 on success / -1 on EOF-arg, and its pushback is a
//!     LIFO STACK (push A,B,C then read => C,B,A,natural), NOT 1-deep.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `vita` on `src` in a fresh temp dir, after writing `files` (name,
/// contents) into it (the input data $fopen will read). Returns stdout + code.
fn run(src: &str, files: &[(&str, &[u8])]) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fg_{}_{n}", std::process::id()));
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
fn fgetc_sequence_and_eof() {
    // "AB\nCD" => 65,66,10,67,68 then -1 forever.
    let (out, _c) = run(
        "module t;\n\
         integer fd, c, i;\n\
         initial begin\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           for (i=0;i<8;i=i+1) begin c=$fgetc(fd); $display(\"%0d\", c); end\n\
         end\n\
         endmodule\n",
        &[("in.txt", b"AB\nCD")],
    );
    let nums: Vec<&str> = out
        .lines()
        .filter(|l| l.trim().parse::<i64>().is_ok())
        .collect();
    assert_eq!(
        nums,
        ["65", "66", "10", "67", "68", "-1", "-1", "-1"],
        "fgetc sequence:\n{out}"
    );
}

#[test]
fn feof_is_lazy() {
    // feof stays 0 right after reading the last valid byte; it becomes 1 only
    // after the read that returns -1.
    let (out, _c) = run(
        "module t;\n\
         integer fd, c, e;\n\
         initial begin\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           c=$fgetc(fd); e=$feof(fd); $display(\"a %0d %0d\", c, e);\n\
           c=$fgetc(fd); e=$feof(fd); $display(\"b %0d %0d\", c, e);\n\
           c=$fgetc(fd); e=$feof(fd); $display(\"c %0d %0d\", c, e);\n\
         end\n\
         endmodule\n",
        &[("in.txt", b"XY")],
    );
    assert!(out.contains("a 88 0"), "{out}");
    assert!(out.contains("b 89 0"), "{out}");
    assert!(out.contains("c -1 1"), "{out}");
}

#[test]
fn feof_bad_fd_returns_neg1() {
    let (out, _c) = run(
        "module t;\n\
         integer fd, e;\n\
         initial begin\n\
           fd = $fopen(\"/no/such/path/here\", \"r\");\n\
           e = $feof(fd);\n\
           $display(\"badfeof %0d fd %0d\", e, fd);\n\
         end\n\
         endmodule\n",
        &[],
    );
    // $fopen fails => fd=0; $feof on a bad fd is -1 (NOT 0).
    assert!(out.contains("badfeof -1 fd 0"), "{out}");
}

#[test]
fn ungetc_returns_zero_and_pushes_back() {
    // ungetc returns 0 (success), the next fgetc returns the pushed char, then
    // the natural stream resumes.
    let (out, _c) = run(
        "module t;\n\
         integer fd, c, r;\n\
         initial begin\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           c=$fgetc(fd); $display(\"g1 %0d\", c);\n\
           r=$ungetc(99, fd); $display(\"un %0d\", r);\n\
           c=$fgetc(fd); $display(\"g2 %0d\", c);\n\
           c=$fgetc(fd); $display(\"g3 %0d\", c);\n\
         end\n\
         endmodule\n",
        &[("in.txt", b"XY")],
    );
    assert!(out.contains("g1 88"), "{out}"); // 'X'
    assert!(out.contains("un 0"), "{out}"); // success = 0, NOT the char
    assert!(out.contains("g2 99"), "{out}"); // pushed-back 'c'
    assert!(out.contains("g3 89"), "{out}"); // natural 'Y'
}

#[test]
fn ungetc_is_a_lifo_stack() {
    // push A,B,C then read => C,B,A, natural Z, then EOF (iverilog retains
    // EVERY pushed byte; it is NOT a 1-deep buffer).
    let (out, _c) = run(
        "module t;\n\
         integer fd, c, r;\n\
         initial begin\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           r=$ungetc(65, fd); r=$ungetc(66, fd); r=$ungetc(67, fd);\n\
           c=$fgetc(fd); $display(\"%0d\", c);\n\
           c=$fgetc(fd); $display(\"%0d\", c);\n\
           c=$fgetc(fd); $display(\"%0d\", c);\n\
           c=$fgetc(fd); $display(\"%0d\", c);\n\
           c=$fgetc(fd); $display(\"%0d\", c);\n\
         end\n\
         endmodule\n",
        &[("in.txt", b"Z")],
    );
    let nums: Vec<&str> = out
        .lines()
        .filter(|l| l.trim().parse::<i64>().is_ok())
        .collect();
    assert_eq!(
        nums,
        ["67", "66", "65", "90", "-1"],
        "LIFO pushback:\n{out}"
    );
}

#[test]
fn ungetc_neg1_is_noop_and_clears_eof() {
    // $ungetc(-1) returns -1 and is a no-op; a real $ungetc at EOF clears feof.
    let (out, _c) = run(
        "module t;\n\
         integer fd, c, e, r;\n\
         initial begin\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           c=$fgetc(fd); c=$fgetc(fd); c=$fgetc(fd);\n\
           e=$feof(fd); $display(\"eofb %0d\", e);\n\
           r=$ungetc(-1, fd); $display(\"un1 %0d\", r);\n\
           r=$ungetc(88, fd); e=$feof(fd); $display(\"un2 %0d eof %0d\", r, e);\n\
           c=$fgetc(fd); $display(\"c %0d\", c);\n\
         end\n\
         endmodule\n",
        &[("in.txt", b"XY")],
    );
    assert!(out.contains("eofb 1"), "{out}"); // at EOF after 3 reads
    assert!(out.contains("un1 -1"), "{out}"); // $ungetc(-1) = -1, no-op
    assert!(out.contains("un2 0 eof 0"), "{out}"); // success + EOF cleared
    assert!(out.contains("c 88"), "{out}"); // the pushed byte
}

#[test]
fn nested_placement_is_loud() {
    // any non-direct-rhs placement is a clean E3009, not a silent wrong value.
    let (out, code) = run(
        "module t;\n\
         integer fd, x;\n\
         initial begin\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           x = $fgetc(fd) + 1;\n\
         end\n\
         endmodule\n",
        &[("in.txt", b"AB")],
    );
    assert!(
        out.contains("VITA-E3009") || code == Some(1),
        "nested $fgetc must be loud: {out} code={code:?}"
    );
}

#[test]
fn write_only_fd_rejects_reads() {
    // a plain "w" (write-only) fd: $fgetc=-1 (NO eof latch), $feof=0,
    // $ungetc=-1 — iverilog never makes a write stream readable.
    let (out, _c) = run(
        "module t;\n\
         integer fd, c, e, r;\n\
         initial begin\n\
           fd = $fopen(\"out.txt\", \"w\");\n\
           c=$fgetc(fd); e=$feof(fd); $display(\"g %0d feof %0d\", c, e);\n\
           r=$ungetc(65, fd); $display(\"un %0d\", r);\n\
           c=$fgetc(fd); $display(\"g2 %0d\", c);\n\
         end\n\
         endmodule\n",
        &[],
    );
    assert!(
        out.contains("g -1 feof 0"),
        "write-only: fgetc -1, feof 0:\n{out}"
    );
    assert!(
        out.contains("un -1"),
        "ungetc on write-only fd = -1:\n{out}"
    );
    assert!(
        out.contains("g2 -1"),
        "still not readable after ungetc:\n{out}"
    );
}

#[test]
fn read_write_plus_mode_is_readable() {
    // "w+" is read-AND-write: on the fresh (empty) file $fgetc=-1 + $feof=1,
    // and $ungetc works (returns 0; the next read returns the pushed byte).
    let (out, _c) = run(
        "module t;\n\
         integer fd, c, e, r;\n\
         initial begin\n\
           fd = $fopen(\"rw.txt\", \"w+\");\n\
           c=$fgetc(fd); e=$feof(fd); $display(\"g %0d feof %0d\", c, e);\n\
           r=$ungetc(65, fd); $display(\"un %0d\", r);\n\
           c=$fgetc(fd); $display(\"g2 %0d\", c);\n\
         end\n\
         endmodule\n",
        &[],
    );
    assert!(
        out.contains("g -1 feof 1"),
        "w+ empty: fgetc -1, feof 1:\n{out}"
    );
    assert!(out.contains("un 0"), "ungetc on w+ succeeds:\n{out}");
    assert!(out.contains("g2 65"), "w+ reads the pushed byte:\n{out}");
}

#[test]
fn ungetc_xz_char_pushes_low_byte_x_as_zero() {
    // x/z in c is NOT the EOF sentinel (only the exact int -1 is): iverilog
    // pushes the low byte with x/z bits coerced to 0, returning 0.
    let (out, _c) = run(
        "module t;\n\
         integer fd, c, r;\n\
         initial begin\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           r=$ungetc(32'bx, fd); $display(\"ux %0d\", r);\n\
           c=$fgetc(fd); $display(\"a %0d\", c);\n\
           c=$fgetc(fd); $display(\"b %0d\", c);\n\
         end\n\
         endmodule\n",
        &[("in.txt", b"XY")],
    );
    assert!(out.contains("ux 0"), "ungetc(x) succeeds (not EOF):\n{out}");
    assert!(out.contains("a 0"), "pushed byte = x coerced to 0:\n{out}");
    assert!(
        out.contains("b 88"),
        "natural stream resumes (X=88):\n{out}"
    );
}

#[test]
fn ungetc_partial_xz_keeps_known_low_byte() {
    // {24'bx, 8'h41}: the high x bits drop; the known low byte 0x41='A' pushes.
    let (out, _c) = run(
        "module t;\n\
         integer fd, c, r;\n\
         initial begin\n\
           fd = $fopen(\"in.txt\", \"r\");\n\
           r=$ungetc({24'bx, 8'h41}, fd); $display(\"up %0d\", r);\n\
           c=$fgetc(fd); $display(\"p %0d\", c);\n\
         end\n\
         endmodule\n",
        &[("in.txt", b"Z")],
    );
    assert!(out.contains("up 0"), "{out}");
    assert!(out.contains("p 65"), "pushed low byte 0x41=65:\n{out}");
}
