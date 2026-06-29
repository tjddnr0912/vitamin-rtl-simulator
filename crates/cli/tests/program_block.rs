//! N7-REST ⓑ-breadth: SystemVerilog `program` blocks (IEEE 1800 §24).
//!
//! Oracle: iverilog -g2012 LIVE. vita parses a `program … endprogram` into the
//! SAME module AST (program ≈ module top-level container) — a pure parser/lexer
//! addition (IR-0). The §24 Reactive-region scheduling of program processes is
//! approximated as the Active region (documented limitation); for the common
//! standalone-testbench shapes below the observable behavior matches iverilog.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_full(src: &str) -> (String, String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_prog_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("failed to run vita");
    let _ = std::fs::remove_file(&path);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

fn run(src: &str) -> String {
    let (out, err, ok) = run_full(src);
    assert!(ok, "vita must succeed; stderr:\n{err}");
    let mut s = String::new();
    for l in out.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    s
}

#[test]
fn basic_program_initial() {
    assert_eq!(
        run("program p; initial $display(\"hi\"); endprogram\n"),
        "hi\n"
    );
}

#[test]
fn program_with_local_var() {
    let out = run("program p; int x;\n\
         initial begin x = 5; x = x + 3; $display(\"x=%0d\", x); end\n\
         endprogram\n");
    assert_eq!(out, "x=8\n");
}

#[test]
fn program_coexists_with_module() {
    // a module + a top-level program both elaborate; the program's delayed
    // initial runs after the module settles.
    let out = run("module m; logic [3:0] c; initial c = 7; endmodule\n\
         program p; initial #1 $display(\"ok\"); endprogram\n");
    assert_eq!(out, "ok\n");
}

#[test]
fn program_reads_port_from_module() {
    let out = run("module m; logic a; initial a = 1; tb t(a); endmodule\n\
         program tb(input a); initial #1 $display(\"a=%b\", a); endprogram\n");
    assert_eq!(out, "a=1\n");
}

#[test]
fn program_with_function() {
    let out = run("program p;\n\
         function int dbl(int v); return v*2; endfunction\n\
         initial $display(\"%0d\", dbl(21));\n\
         endprogram\n");
    assert_eq!(out, "42\n");
}
