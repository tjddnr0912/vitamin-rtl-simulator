//! A 2-state formal/local (`byte`/`int`/`shortint`/`longint`/`bit`) of an
//! AUTOMATIC (frame) task or function must coerce any X/Z to 0 on every write
//! (IEEE 1800 §6.11.3) — the copy-IN of an X-bearing actual, a body-local
//! assignment, and the return. The frame slot-write path bypassed the 2-state
//! coercion that `write_chunk` applies on the inline path, so X leaked. A
//! 4-state formal (`logic`/`reg`) must still retain X. Oracle: iverilog -g2012.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_fts_{}_{n}.sv", std::process::id()));
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
fn automatic_inout_byte_coerces_x_whole() {
    // data = xxxx_1111; the byte formal can't hold X -> 0000_1111 = 15; +1 = 16.
    let out = run("module top;\n\
           reg [7:0] data;\n\
           task automatic inc(inout byte x); x = x + 1; endtask\n\
           initial begin data = 8'bxxxx_1111; inc(data); $display(\"%b\", data); end\n\
         endmodule\n");
    assert_eq!(out, "00010000\n");
}

#[test]
fn automatic_inout_byte_coerces_x_partsel() {
    let out = run("module top;\n\
           reg [7:0] data;\n\
           task automatic inc(inout byte x); x = x + 1; endtask\n\
           initial begin data = 8'bxxxx_1111; inc(data[7:0]); $display(\"%b\", data); end\n\
         endmodule\n");
    assert_eq!(out, "00010000\n");
}

#[test]
fn automatic_function_input_byte_coerces_x() {
    let out = run("module top;\n\
           reg [7:0] data; reg [7:0] res;\n\
           function automatic byte addone(input byte x); addone = x + 1; endfunction\n\
           initial begin data = 8'bxxxx_1111; res = addone(data); $display(\"%b\", res); end\n\
         endmodule\n");
    assert_eq!(out, "00010000\n");
}

#[test]
fn automatic_inout_int_coerces_x() {
    let out = run("module top;\n\
           reg [7:0] data;\n\
           task automatic inc(inout int x); x = x + 1; endtask\n\
           initial begin data = 8'bxxxx_1111; inc(data); $display(\"%b\", data); end\n\
         endmodule\n");
    assert_eq!(out, "00010000\n");
}

#[test]
fn automatic_inout_shortint_coerces_x() {
    let out = run("module top;\n\
           reg [7:0] data;\n\
           task automatic inc(inout shortint x); x = x + 1; endtask\n\
           initial begin data = 8'bxx00_1111; inc(data); $display(\"%b\", data); end\n\
         endmodule\n");
    assert_eq!(out, "00010000\n");
}

#[test]
fn frame_body_local_two_state_coerces_x() {
    // A body-local `int t = src` where src carries X — the 2-state assignment
    // coerces X->0 before t+1. f(xxxx_0001) -> t=1 -> f=2.
    let out = run("module top;\n\
           reg [7:0] q;\n\
           function automatic int f(input [7:0] src);\n\
             int t; t = src; f = t + 1;\n\
           endfunction\n\
           initial begin q = f(8'bxxxx_0001); $display(\"%b\", q[7:0]); end\n\
         endmodule\n");
    assert_eq!(out, "00000010\n");
}

#[test]
fn automatic_inout_four_state_keeps_x() {
    // CONTROL: a 4-state `logic` formal must NOT coerce — X stays X.
    let out = run("module top;\n\
           reg [7:0] data;\n\
           task automatic inc(inout logic [7:0] x); x = x + 1; endtask\n\
           initial begin data = 8'bxxxx_1111; inc(data); $display(\"%b\", data); end\n\
         endmodule\n");
    assert_eq!(out, "xxxxxxxx\n");
}
