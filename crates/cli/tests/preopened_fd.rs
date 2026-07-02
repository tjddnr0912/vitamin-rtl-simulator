//! Pre-opened file descriptors (IEEE 1800 §21.3.4): STDIN 32'h8000_0000,
//! STDOUT 32'h8000_0001, STDERR 32'h8000_0002 — iverilog-13.0-pinned.
//!
//! Before this slice vita treated all three as invalid/closed (W4022 warn +
//! DROPPED write): a `$fdisplay(32'h8000_0001, …)` printed nothing. Now STDOUT
//! routes through the same deterministic sink as `$display` (statement-order
//! interleave), STDERR reaches the process stderr, `$fclose` on a pre-opened fd
//! warns and keeps it usable, and reads follow the write-only-fd rule
//! (`$fgetc`=-1, `$feof`=0, no warning). STDIN writes stay W4022 warn+drop
//! (iverilog drops them silently); STDIN reads stay deferred (W4022 + -1).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pfd_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .stdin(std::process::Stdio::null())
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn stdout_fd_writes_interleave_with_display() {
    // $fwrite/$fdisplay to 32'h8000_0001 hit the SAME stdout stream as
    // $display, in statement order (iverilog-pinned byte sequence).
    let (out, _, code) = run("module t; initial begin\n\
           $fwrite(32'h8000_0001, \"w-out %0d|\", 1);\n\
           $fdisplay(32'h8000_0001, \"d-out %0d\", 2);\n\
           $display(\"plain display\");\n\
           $fdisplay(32'h8000_0001, \"after\");\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(
        out.contains("w-out 1|d-out 2\nplain display\nafter\n"),
        "stdout fd interleaves in order:\n{out}"
    );
}

#[test]
fn stderr_fd_reaches_process_stderr() {
    // 32'h8000_0002 goes to stderr (iverilog parity); stdout stays separate.
    let (out, err, code) = run("module t; initial begin\n\
           $fdisplay(32'h8000_0002, \"to-stderr %0d\", 7);\n\
           $display(\"to-stdout\");\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(out.contains("to-stdout"), "stdout intact:\n{out}");
    assert!(
        !out.contains("to-stderr"),
        "stderr text NOT on stdout:\n{out}"
    );
    assert!(err.contains("to-stderr 7"), "stderr payload:\n{err}");
}

#[test]
fn stdin_write_warns_and_drops() {
    // A write to the read-only STDIN drops the text. iverilog drops SILENTLY;
    // vita adds a W4022 warn (strictly more diagnostic, same output bytes).
    let (out, err, code) = run("module t; initial begin\n\
           $fwrite(32'h8000_0000, \"to-stdin?\");\n\
           $display(\"alive\");\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(out.contains("alive") && !out.contains("to-stdin"), "{out}");
    assert!(err.contains("W4022"), "stdin write warns:\n{err}");
}

#[test]
fn fclose_preopened_warns_and_stays_usable() {
    // $fclose(STDOUT) = warn + no-op; later writes still print
    // (iverilog-pinned: "could not close file descriptor STDOUT").
    let (out, err, code) = run("module t; initial begin\n\
           $fclose(32'h8000_0001);\n\
           $fdisplay(32'h8000_0001, \"after close\");\n\
           $display(\"display still works\");\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(
        out.contains("after close\ndisplay still works"),
        "descriptor survives $fclose:\n{out}"
    );
    assert!(
        err.contains("cannot close the pre-opened STDOUT"),
        "close warns:\n{err}"
    );
}

#[test]
fn reads_on_preopened_follow_write_only_rule() {
    // $fgetc(STDOUT/STDERR) = -1 with NO warning, $feof(STDOUT) = 0,
    // $ungetc(STDOUT) = -1 (iverilog-pinned). STDIN reads stay deferred:
    // -1 with a W4022 warn.
    let (out, err, code) = run("module t; integer c; integer e; integer u; initial begin\n\
           c = $fgetc(32'h8000_0001);\n\
           e = $feof(32'h8000_0001);\n\
           u = $ungetc(65, 32'h8000_0001);\n\
           $display(\"out fd: fgetc=%0d feof=%0d ungetc=%0d\", c, e, u);\n\
           c = $fgetc(32'h8000_0002);\n\
           $display(\"err fd: fgetc=%0d\", c);\n\
           c = $fgetc(32'h8000_0000);\n\
           $display(\"stdin: fgetc=%0d\", c);\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(
        out.contains("out fd: fgetc=-1 feof=0 ungetc=-1"),
        "write-only rule on STDOUT:\n{out}"
    );
    assert!(out.contains("err fd: fgetc=-1"), "{out}");
    assert!(out.contains("stdin: fgetc=-1"), "{out}");
    // exactly ONE W4022 (the deferred stdin read) — stdout/stderr reads are quiet.
    assert_eq!(
        err.matches("W4022").count(),
        1,
        "only the stdin read warns:\n{err}"
    );
}

#[test]
fn real_file_fd_path_unchanged() {
    // Regression: a normal $fopen'd fd still round-trips (write then read back).
    let (out, _, code) = run("module t; integer fd; integer c; initial begin\n\
           fd = $fopen(\"pfd_regress.txt\", \"w\");\n\
           $fdisplay(fd, \"AB\");\n\
           $fclose(fd);\n\
           fd = $fopen(\"pfd_regress.txt\", \"r\");\n\
           c = $fgetc(fd);\n\
           $display(\"first=%0d\", c);\n\
           $fclose(fd);\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(out.contains("first=65"), "file fd round-trip:\n{out}");
}

#[test]
fn mcd_bit0_stdout_unchanged() {
    // Regression: the MCD form (bit 0 = stdout) is untouched.
    let (out, _, code) = run("module t; initial begin\n\
           $fwrite(1, \"mcd-bit0 %0d|\", 3);\n\
           $fdisplay(1, \"mcd-d\");\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(out.contains("mcd-bit0 3|mcd-d"), "{out}");
}
