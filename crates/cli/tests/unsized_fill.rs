//! Unsized fill literals `'0` / `'1` / `'x` / `'z` (IEEE §5.7.1, §11.4.4) are
//! CONTEXT-determined in width: the fill bit replicates to the width of the
//! assignment target, not a fixed 32 bits. Previously vita self-sized them to
//! 32 bits, so a wider-than-32 target only got its low 32 bits filled (a
//! silent-wrong surfaced by the parameterized-class hunt). Oracle: iverilog
//! -g2012 (`a='1` into a 64-bit reg → `ffffffffffffffff`).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_fill_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut s = String::new();
    for l in so.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    s
}

#[test]
fn ones_fill_into_64bit_procedural() {
    let out = run("module t;\n\
           logic [63:0] a;\n\
           initial begin a = '1; $display(\"%h\", a); end\n\
         endmodule\n");
    assert_eq!(out, "ffffffffffffffff\n");
}

#[test]
fn zero_fill_into_64bit() {
    let out = run("module t;\n\
           logic [63:0] b;\n\
           initial begin b = '0; $display(\"%h\", b); end\n\
         endmodule\n");
    assert_eq!(out, "0000000000000000\n");
}

#[test]
fn ones_fill_into_40bit() {
    // 40 bits all-ones = 10 hex nibbles.
    let out = run("module t;\n\
           logic [39:0] c;\n\
           initial begin c = '1; $display(\"%h\", c); end\n\
         endmodule\n");
    assert_eq!(out, "ffffffffff\n");
}

#[test]
fn ones_fill_into_100bit() {
    // 100 bits all-ones = 25 hex nibbles.
    let out = run("module t;\n\
           logic [99:0] w;\n\
           initial begin w = '1; $display(\"%h\", w); end\n\
         endmodule\n");
    assert_eq!(out, "fffffffffffffffffffffffff\n");
}

#[test]
fn x_fill_into_64bit() {
    let out = run("module t;\n\
           logic [63:0] xx;\n\
           initial begin xx = 'x; $display(\"%h\", xx); end\n\
         endmodule\n");
    assert_eq!(out, "xxxxxxxxxxxxxxxx\n");
}

#[test]
fn z_fill_into_64bit() {
    let out = run("module t;\n\
           logic [63:0] zz;\n\
           initial begin zz = 'z; $display(\"%h\", zz); end\n\
         endmodule\n");
    assert_eq!(out, "zzzzzzzzzzzzzzzz\n");
}

#[test]
fn ones_fill_declaration_initializer() {
    let out = run("module t;\n\
           logic [63:0] a = '1;\n\
           initial $display(\"%h\", a);\n\
         endmodule\n");
    assert_eq!(out, "ffffffffffffffff\n");
}

#[test]
fn ones_fill_continuous_assign() {
    let out = run("module t;\n\
           logic [63:0] a;\n\
           assign a = '1;\n\
           initial begin #1; $display(\"%h\", a); end\n\
         endmodule\n");
    assert_eq!(out, "ffffffffffffffff\n");
}

#[test]
fn ones_fill_nonblocking() {
    let out = run("module t;\n\
           logic [63:0] a;\n\
           initial begin a <= '1; #1; $display(\"%h\", a); end\n\
         endmodule\n");
    assert_eq!(out, "ffffffffffffffff\n");
}

#[test]
fn fill_in_binary_op_context_stays_correct() {
    // A binary-op context expands `'1` to the operand width (≥32). The 32-bit
    // self-determined default makes this match iverilog (`'1 + 0` → ffffffff);
    // it is the reason we do NOT shrink the self-determined fill to 1 bit. (The
    // residual bare-`$display("%h",'1)` divergence is a documented known limit.)
    let out = run("module t;\n\
           logic [31:0] r;\n\
           initial begin r = '1 + 0; $display(\"%h\", r); end\n\
         endmodule\n");
    assert_eq!(out, "ffffffff\n");
}
