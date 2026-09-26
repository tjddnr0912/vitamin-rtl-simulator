//! A continuous assign that reads the simulation time is re-evaluated when an OPERAND changes,
//! not as time advances (IEEE 1800 §10.3.2) — ROADMAP §2 "Delays / events".
//!
//! `levelize::ca_deps` certifies which assigns may be skipped when their dependency nets did
//! not move; every system function made an assign uncertifiable, so one reading `$time`,
//! `$stime` or `$realtime` sat in `ca_always` and was re-run on every settle, reading the
//! time of whatever step happened to settle: `wire [31:0] w1 = int'($realtime * 1.5);` was
//! `00000002` at 1 and `0000000f` at 10 where both oracles keep the time-0 `00000000`. The
//! time reads now contribute no dependency (time advancing is no event), and the one-operand
//! conversions (`int'(r)` lowers to `RealToInt`) are certified like any pure operator, in the
//! assign itself and in a callee body.
//!
//! Oracles: iverilog 13.0 `-g2012` gives every value below. verilator 5.052 agrees on the
//! time-only cells and re-evaluates the others at points of its own (`$time + a` with `a`
//! changing once at 4 reads 0, 1, 5, 6, 9, 11 at 1, 3, 5, 7, 9, 12) — the language rule is
//! iverilog's, and it is what vita now does.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> Vec<String> {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_catime_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let s =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "expected exit 0, got:\n{s}");
    s.lines()
        .filter(|l| l.starts_with('T'))
        .map(|l| l.trim().to_string())
        .collect()
}

/// The row's own cells: time-only drivers under a cast (both oracles keep the time-0 value). PRE: `00000002 …` at 1, `00000012 …` at 12.
#[test]
fn time_read_c1() {
    let got: Vec<String> = run(r#"`timescale 1ns/1ns
module t;
  wire [31:0] w1 = int'($realtime * 1.5);
  wire [63:0] w2 = longint'(-$realtime * 1.0e18);
  initial begin
    
  end
  initial begin
    #1 $display("T1 %h", w1,  w2);
    #2 $display("T3 %h %h", w1, w2);
    #2 $display("T5 %h %h", w1, w2);
    #2 $display("T7 %h %h", w1, w2);
    #2 $display("T9 %h %h", w1, w2);
    #3 $display("T12 %h %h", w1, w2);
    $finish;
  end
endmodule
"#)
    .into_iter()
    .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
    .collect();
    assert_eq!(
        got,
        vec![
            r#"T1 00000000 0"#,
            r#"T3 00000000 0000000000000000"#,
            r#"T5 00000000 0000000000000000"#,
            r#"T7 00000000 0000000000000000"#,
            r#"T9 00000000 0000000000000000"#,
            r#"T12 00000000 0000000000000000"#
        ]
    );
}

/// `$time + a`, `a` changing at 4 and 7: evaluated at the seed and at each change, reading that moment's time. PRE read the time of every step (1, 3, 6, 8, b, e).
#[test]
fn time_read_c2() {
    let got: Vec<String> = run(r#"`timescale 1ns/1ns
module t;
  reg [7:0] a = 0;
  wire [63:0] w = $time + a;
  initial begin
    #4 a = 1; #3 a = 2;
  end
  initial begin
    #1 $display("T1 %h", w);
    #2 $display("T3 %h", w);
    #2 $display("T5 %h", w);
    #2 $display("T7 %h", w);
    #2 $display("T9 %h", w);
    #3 $display("T12 %h", w);
    $finish;
  end
endmodule
"#)
    .into_iter()
    .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
    .collect();
    assert_eq!(
        got,
        vec![
            r#"T1 0000000000000000"#,
            r#"T3 0000000000000000"#,
            r#"T5 0000000000000005"#,
            r#"T7 0000000000000005"#,
            r#"T9 0000000000000009"#,
            r#"T12 0000000000000009"#
        ]
    );
}

/// `int'($realtime) * 2 + a`: the conversion certifies too. PRE `00000002` at 1.
#[test]
fn time_read_c3() {
    let got: Vec<String> = run(r#"`timescale 1ns/1ns
module t;
  reg [7:0] a = 0;
  wire [31:0] w = int'($realtime) * 2 + a;
  initial begin
    #4 a = 1;
  end
  initial begin
    #1 $display("T1 %h", w);
    #2 $display("T3 %h", w);
    #2 $display("T5 %h", w);
    #2 $display("T7 %h", w);
    #2 $display("T9 %h", w);
    #3 $display("T12 %h", w);
    $finish;
  end
endmodule
"#)
    .into_iter()
    .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
    .collect();
    assert_eq!(
        got,
        vec![
            r#"T1 00000000"#,
            r#"T3 00000000"#,
            r#"T5 00000009"#,
            r#"T7 00000009"#,
            r#"T9 00000009"#,
            r#"T12 00000009"#
        ]
    );
}

/// A ternary selecting `$time`: 99 until `a` rises at 4, then 4 (not 5, 7, 9, c).
#[test]
fn time_read_c4() {
    let got: Vec<String> = run(r#"`timescale 1ns/1ns
module t;
  reg [7:0] a = 0;
  wire [63:0] w = a ? $time : 64'd99;
  initial begin
    #4 a = 1; #3 a = 0; #1 a = 1;
  end
  initial begin
    #1 $display("T1 %h", w);
    #2 $display("T3 %h", w);
    #2 $display("T5 %h", w);
    #2 $display("T7 %h", w);
    #2 $display("T9 %h", w);
    #3 $display("T12 %h", w);
    $finish;
  end
endmodule
"#)
    .into_iter()
    .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
    .collect();
    assert_eq!(
        got,
        vec![
            r#"T1 0000000000000063"#,
            r#"T3 0000000000000063"#,
            r#"T5 0000000000000004"#,
            r#"T7 0000000000000004"#,
            r#"T9 0000000000000008"#,
            r#"T12 0000000000000008"#
        ]
    );
}

/// `$stime + a`.
#[test]
fn time_read_c5() {
    let got: Vec<String> = run(r#"`timescale 1ns/1ns
module t;
  reg [7:0] a = 0;
  wire [31:0] w = $stime + a;
  initial begin
    #4 a = 1;
  end
  initial begin
    #1 $display("T1 %h", w);
    #2 $display("T3 %h", w);
    #2 $display("T5 %h", w);
    #2 $display("T7 %h", w);
    #2 $display("T9 %h", w);
    #3 $display("T12 %h", w);
    $finish;
  end
endmodule
"#)
    .into_iter()
    .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
    .collect();
    assert_eq!(
        got,
        vec![
            r#"T1 00000000"#,
            r#"T3 00000000"#,
            r#"T5 00000005"#,
            r#"T7 00000005"#,
            r#"T9 00000005"#,
            r#"T12 00000005"#
        ]
    );
}

/// `assign w = $time * 10 + a;` as a separate assign statement.
#[test]
fn time_read_c6() {
    let got: Vec<String> = run(r#"`timescale 1ns/1ns
module t;
  reg [7:0] a = 0;
  wire [63:0] w;
  assign w = $time * 10 + a;
  initial begin
    #4 a = 3;
  end
  initial begin
    #1 $display("T1 %h", w);
    #2 $display("T3 %h", w);
    #2 $display("T5 %h", w);
    #2 $display("T7 %h", w);
    #2 $display("T9 %h", w);
    #3 $display("T12 %h", w);
    $finish;
  end
endmodule
"#)
    .into_iter()
    .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
    .collect();
    assert_eq!(
        got,
        vec![
            r#"T1 0000000000000000"#,
            r#"T3 0000000000000000"#,
            r#"T5 000000000000002b"#,
            r#"T7 000000000000002b"#,
            r#"T9 000000000000002b"#,
            r#"T12 000000000000002b"#
        ]
    );
}

/// `($time > 5) & a`: the comparison re-evaluates only when `a` moves — 0 at 9 although `$time > 5` holds (PRE 1 at 7).
#[test]
fn time_read_c7() {
    let got: Vec<String> = run(r#"`timescale 1ns/1ns
module t;
  reg a = 0;
  wire t = ($time > 5) & a;
  initial begin
    #2 a = 1; #6 a = 0; #1 a = 1;
  end
  initial begin
    #1 $display("T1 %h", t);
    #2 $display("T3 %h", t);
    #2 $display("T5 %h", t);
    #2 $display("T7 %h", t);
    #2 $display("T9 %h", t);
    #3 $display("T12 %h", t);
    $finish;
  end
endmodule
"#)
    .into_iter()
    .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
    .collect();
    assert_eq!(
        got,
        vec![r#"T1 0"#, r#"T3 0"#, r#"T5 0"#, r#"T7 0"#, r#"T9 0"#, r#"T12 1"#]
    );
}

/// A callee that reads `$time` (`f(a)` returns `$time + x`): the callee body is certified the same way. PRE re-read the time every step.
#[test]
fn time_read_c8() {
    let got: Vec<String> = run(r#"`timescale 1ns/1ns
module t;
  reg [7:0] a = 0;
  function automatic [63:0] f(input [7:0] x); f = $time + x; endfunction
  wire [63:0] w = f(a);
  initial begin
    #4 a = 1;
  end
  initial begin
    #1 $display("T1 %h", w);
    #2 $display("T3 %h", w);
    #2 $display("T5 %h", w);
    #2 $display("T7 %h", w);
    #2 $display("T9 %h", w);
    #3 $display("T12 %h", w);
    $finish;
  end
endmodule
"#)
    .into_iter()
    .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
    .collect();
    assert_eq!(
        got,
        vec![
            r#"T1 0000000000000000"#,
            r#"T3 0000000000000000"#,
            r#"T5 0000000000000005"#,
            r#"T7 0000000000000005"#,
            r#"T9 0000000000000005"#,
            r#"T12 0000000000000005"#
        ]
    );
}

/// A forced then released time-driven wire: the release re-evaluates the driver (`redirty_drivers_of`), reading the release time — `6` at 7, verilator's answer (iverilog keeps the value it computed at 0 and prints `0`; an oracle split, pinned on vita's side). Everything else is both oracles'.
#[test]
fn time_read_after_release() {
    assert_eq!(
        run(r#"`timescale 1ns/1ns
module t;
  reg [7:0] a = 0;
  wire [63:0] w = $time + a;
  initial begin #3 force w = 64'd77; #3 release w; end
  initial begin
    #8 a = 2;
  end
  initial begin
    #1 $display("T1 %h", w);
    #2 $display("T3 %h", w);
    #2 $display("T5 %h", w);
    #2 $display("T7 %h", w);
    #2 $display("T9 %h", w);
    #3 $display("T12 %h", w);
    $finish;
  end
endmodule
"#),
        vec![
            r#"T1 0000000000000000"#,
            r#"T3 000000000000004d"#,
            r#"T5 000000000000004d"#,
            r#"T7 0000000000000006"#,
            r#"T9 000000000000000a"#,
            r#"T12 000000000000000a"#
        ]
    );
}

/// NOT this slice, pinned as measured (ROADMAP §2 "Delays / events"): a continuous assign in a module whose time unit differs from the process that moved its operand reads `$time` / `$realtime` at the wrong scale — `T6 1 1` where both oracles print `T6 5 41`. A continuous assign carries no module time multiplier; it evaluates under whatever process context is current.
#[test]
fn a_time_read_in_another_time_unit_is_the_recorded_residue() {
    assert_eq!(
        run(r#"`timescale 1ns/1ps
module sub(output [63:0] o, output [31:0] r, input [7:0] a);
  assign o = $time + a;
  assign r = int'($realtime * 10.0) + a;
endmodule
`timescale 1us/1ns
module t;
  reg [7:0] a = 0;
  wire [63:0] o; wire [31:0] r;
  sub u(.o(o), .r(r), .a(a));
  initial begin #0.004 a = 1; end
  initial begin
    #0.001 $display("T1 %0d %0d", o, r);
    #0.005 $display("T6 %0d %0d", o, r);
    #0.010 $display("T16 %0d %0d", o, r);
    $finish;
  end
endmodule
"#),
        vec![r#"T1 0 0"#, r#"T6 1 1"#, r#"T16 1 1"#]
    );
}
