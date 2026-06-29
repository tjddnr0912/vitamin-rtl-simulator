//! A 2-state function RETURN type (`byte`/`int`/`shortint`/`longint`/`bit`)
//! must coerce X/Z to 0 on the return assignment (IEEE 1800 §6.11.3), like any
//! 2-state variable. `function byte f; f = x_bearing;` previously leaked X on
//! BOTH the inline (non-automatic) and frame (automatic) paths because the
//! return type's 2-state-ness was dropped at parse time (`ParamType` conflates
//! `int` with 4-state `integer`; byte/shortint/longint look like `reg [N]`). A
//! 4-state (`logic`/`reg`/`integer`) return still retains X. Oracle: iverilog.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_rts_{}_{n}.sv", std::process::id()));
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
fn inline_byte_return_coerces_x() {
    let out = run("module top;\n\
           reg [7:0] q;\n\
           function byte f(input [7:0] s); f = s; endfunction\n\
           initial begin q = f(8'bxxxx_0101); $display(\"%b\", q); end\n\
         endmodule\n");
    assert_eq!(out, "00000101\n");
}

#[test]
fn automatic_byte_return_coerces_x() {
    let out = run("module top;\n\
           reg [7:0] q;\n\
           function automatic byte f(input [7:0] s); f = s; endfunction\n\
           initial begin q = f(8'bxxxx_0101); $display(\"%b\", q); end\n\
         endmodule\n");
    assert_eq!(out, "00000101\n");
}

#[test]
fn inline_int_return_coerces_x() {
    let out = run("module top;\n\
           reg [7:0] q;\n\
           function int f(input [7:0] s); f = s; endfunction\n\
           initial begin q = f(8'bxxxx_0101); $display(\"%b\", q[7:0]); end\n\
         endmodule\n");
    assert_eq!(out, "00000101\n");
}

#[test]
fn inline_shortint_return_coerces_x() {
    let out = run("module top;\n\
           reg [15:0] q;\n\
           function shortint f(input [15:0] s); f = s; endfunction\n\
           initial begin q = f(16'bxx00_0000_1111_0101); $display(\"%b\", q); end\n\
         endmodule\n");
    assert_eq!(out, "0000000011110101\n");
}

#[test]
fn longint_return_coerces_x() {
    let out = run("module top;\n\
           reg [7:0] q;\n\
           function longint f(input [7:0] s); f = s; endfunction\n\
           initial begin q = f(8'bxxxx_0101); $display(\"%b\", q[7:0]); end\n\
         endmodule\n");
    assert_eq!(out, "00000101\n");
}

#[test]
fn four_state_logic_return_keeps_x() {
    // CONTROL: a 4-state return must NOT coerce — X survives.
    let out = run("module top;\n\
           reg [7:0] q;\n\
           function logic [7:0] f(input [7:0] s); f = s; endfunction\n\
           initial begin q = f(8'bxxxx_0101); $display(\"%b\", q); end\n\
         endmodule\n");
    assert_eq!(out, "xxxx0101\n");
}

#[test]
fn integer_return_keeps_x() {
    // CONTROL: `integer` is 4-state (distinct from 2-state `int`) — X survives.
    let out = run("module top;\n\
           reg [7:0] q;\n\
           function integer f(input [7:0] s); f = s; endfunction\n\
           initial begin q = f(8'bxxxx_0101); $display(\"%b\", q[7:0]); end\n\
         endmodule\n");
    assert_eq!(out, "xxxx0101\n");
}

#[test]
fn clean_int_return_unaffected() {
    // A clean (no X) 2-state return still computes correctly.
    let out = run("module top;\n\
           int q;\n\
           function int addone(input int a); addone = a + 1; endfunction\n\
           initial begin q = addone(41); $display(\"%0d\", q); end\n\
         endmodule\n");
    assert_eq!(out, "42\n");
}
