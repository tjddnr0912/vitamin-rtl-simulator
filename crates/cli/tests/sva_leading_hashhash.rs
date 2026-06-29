//! A3 (Tier-ⓐ close): leading-`##` SVA consequent — `a |-> ##1 b`. The parser
//! synthesizes an implicit `1` (true) leaf so `##N b` parses as `1 ##N b` (IEEE
//! 1800 §16.9; golden-neutral). Equivalent to `a |=> b` for `##1`. Hand-IEEE
//! (iverilog 13 supports no concurrent assertions).
//!
//! Clock: `always #5 clk=~clk` → posedges at t=5,15,25,35,45,…; a value set at
//! `#10k` is sampled at the next posedge.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svalh_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

fn fires(out: &str, err: &str, code: Option<i32>, ctx: &str) {
    assert_eq!(
        code,
        Some(1),
        "{ctx}: expected a violation (exit 1).\nstderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{ctx}: a violation diagnostic was expected.\nstderr:\n{err}\nout:\n{out}"
    );
}

fn holds(out: &str, err: &str, code: Option<i32>, ctx: &str) {
    assert_eq!(
        code,
        Some(0),
        "{ctx}: expected a clean pass (exit 0).\nstderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "{ctx}: no violation expected.\nstderr:\n{err}\nout:\n{out}"
    );
}

#[test]
fn leading_hashhash_consequent_fires() {
    // a |-> ##1 b: at t15 a=1 (match) → b obliged at t25; b=0 there → fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> ##1 b);\n\
         initial begin\n\
           #10 a=1; b=0;\n\
           #10 a=0; b=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(&out, &err, code, "a |-> ##1 b with b low next clock");
}

#[test]
fn leading_hashhash_consequent_holds() {
    // a |-> ##1 b: at t15 a=1 → b obliged at t25; b=1 there → hold.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> ##1 b);\n\
         initial begin\n\
           #10 a=1; b=0;\n\
           #10 a=0; b=1;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    holds(&out, &err, code, "a |-> ##1 b with b high next clock");
}

#[test]
fn leading_hashhash_matches_implication_arrow() {
    // `a |-> ##1 b` must behave identically to `a |=> b` (the established sugar).
    // Same stimulus that HOLDS for |=> must hold here.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |=> b);\n\
         initial begin\n\
           #10 a=1; b=0;\n\
           #10 a=0; b=1;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    holds(&out, &err, code, "a |=> b reference");
}

#[test]
fn leading_hashhash_two_cycle_delay_fires() {
    // a |-> ##2 b: at t15 a=1 → b obliged at t35; b=0 there → fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> ##2 b);\n\
         initial begin\n\
           #10 a=1; b=0;\n\
           #10 a=0; b=1;\n\
           #10 b=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(&out, &err, code, "a |-> ##2 b, b low at t35");
}

#[test]
fn leading_hashhash_chained_delays_fires() {
    // a |-> ##1 b ##1 c: at t15 a=1 → b@t25 then c@t35. b=1@t25 but c=0@t35 → fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> ##1 b ##1 c);\n\
         initial begin\n\
           #10 a=1; b=0; c=0;\n\
           #10 a=0; b=1;\n\
           #10 b=0; c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(&out, &err, code, "a |-> ##1 b ##1 c, c low at t35");
}

#[test]
fn leading_hashhash_zero_delay_same_clock_fires() {
    // a |-> ##0 b: ##0 = same clock; equivalent to `a |-> b`. At t15 a=1, b=0 → fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> ##0 b);\n\
         initial begin\n\
           #10 a=1; b=0;\n\
           #10 a=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(&out, &err, code, "a |-> ##0 b same clock, b low");
}
