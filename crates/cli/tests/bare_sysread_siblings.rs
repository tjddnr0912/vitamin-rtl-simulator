//! §4.5.122 sibling: a bare-statement `$value$plusargs`/`$fgetc`/`$ungetc`
//! (return discarded) must still perform its side-effect — `$value$plusargs`
//! writes its ref var, `$fgetc` advances the fd, `$ungetc` pushes a char back.
//! Like the scanf family (§4.5.122), these are side-effects of evaluating the
//! `SysFunc` and were emitted only from the assignment-rhs path; a bare
//! statement silently dropped them. Routed through the same helpers (with the
//! count discarded) behind the shared `in_frame_body` gate. Pinned to iverilog
//! 13.0. ($feof is a pure query — a bare $feof is a harmless no-op, not routed.)
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_args(src: &str, extra: &[&str]) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_bss_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .args(extra)
        .current_dir(&d)
        .output()
        .expect("run vita");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn run(src: &str) -> String {
    run_args(src, &[])
}

#[test]
fn bare_value_plusargs_writes_ref() {
    // A bare `$value$plusargs` writes its ref var (`+N=7` on the command line).
    let out = run_args(
        "module m; int a; initial begin \
         $value$plusargs(\"N=%d\", a); $display(\"R=%0d\", a); #1 $finish; end endmodule\n",
        &["+N=7"],
    );
    assert!(
        out.contains("R=7"),
        "bare $value$plusargs write; got:\n{out}"
    );
}

#[test]
fn bare_fgetc_advances_fd() {
    // A bare `$fgetc` consumes a char (advances the fd) even with the char
    // discarded — the next $fgetc then reads the SECOND char.
    let out = run("module m; int fd, c2; initial begin \
         fd = $fopen(\"/tmp/vita_bss_fgetc.txt\", \"w\"); $fwrite(fd, \"AB\"); $fclose(fd); \
         fd = $fopen(\"/tmp/vita_bss_fgetc.txt\", \"r\"); \
         $fgetc(fd); c2 = $fgetc(fd); $fclose(fd); \
         $display(\"R=%0d\", c2); #1 $finish; end endmodule\n");
    // 'B' == 66 (bare $fgetc consumed 'A' == 65 first).
    assert!(out.contains("R=66"), "bare $fgetc advances fd; got:\n{out}");
}

#[test]
fn bare_ungetc_pushes_back() {
    // A bare `$ungetc` pushes a char back onto the fd; the next $fgetc re-reads it.
    let out = run("module m; int fd, c; initial begin \
         fd = $fopen(\"/tmp/vita_bss_ungetc.txt\", \"w\"); $fwrite(fd, \"X\"); $fclose(fd); \
         fd = $fopen(\"/tmp/vita_bss_ungetc.txt\", \"r\"); \
         c = $fgetc(fd); $ungetc(c, fd); c = $fgetc(fd); $fclose(fd); \
         $display(\"R=%0d\", c); #1 $finish; end endmodule\n");
    // 'X' == 88, read then pushed back then re-read.
    assert!(
        out.contains("R=88"),
        "bare $ungetc pushes back; got:\n{out}"
    );
}

#[test]
fn assign_form_value_plusargs_unchanged() {
    // Regression guard: `ok = $value$plusargs(fmt, var)` still returns the flag
    // AND writes the var (byte-identical to before the Option refactor).
    let out = run_args(
        "module m; int a, ok; initial begin \
         ok = $value$plusargs(\"N=%d\", a); $display(\"R=%0d %0d\", ok, a); #1 $finish; end endmodule\n",
        &["+N=9"],
    );
    assert!(
        out.contains("R=1 9"),
        "assign-form $value$plusargs; got:\n{out}"
    );
}

#[test]
fn bare_sibling_in_frame_function_does_not_hard_error() {
    // A bare sibling inside a FRAME function body stays on the pre-existing
    // warn+skip path (shared `in_frame_body` gate) — no misleading hard-error.
    // `a` stays 0 (skipped), so f() = 0 + 100 = 100.
    let out = run_args(
        "module m;\n\
         function automatic int f(); int a;\n\
           begin $value$plusargs(\"N=%d\", a); f = a + 100; end endfunction\n\
         initial begin $display(\"R=%0d\", f()); #1 $finish; end endmodule\n",
        &["+N=5"],
    );
    assert!(
        out.contains("R=100"),
        "frame-body bare sibling must elaborate (warn+skip):\n{out}"
    );
}
