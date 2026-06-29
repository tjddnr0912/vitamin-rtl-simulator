//! N7-REST ⓑ-breadth: SystemVerilog string conversion methods (IEEE 1800 §6.16).
//!
//! Oracle: iverilog -g2012 LIVE for atoi(non-negative)/atohex/atoreal/itoa/hextoa/
//! octtoa/bintoa. HAND-IEEE pins where iverilog diverges or lacks the method:
//! - atoi of a NEGATIVE string: IEEE §6.16.9 parses a leading sign (`"-7"` → -7);
//!   iverilog 13 returns 0 (its bug) — vita follows IEEE.
//! - atooct / atobin: IEEE §6.16.x methods iverilog 13 does NOT implement.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_full(src: &str) -> (String, String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_strm_{}_{n}.sv", std::process::id()));
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

// ── atoi family (string → int, expression) ───────────────────────────────────

#[test]
fn atoi_decimal() {
    let out = run("module t; string s; int x;\n\
         initial begin s=\"42\"; x=s.atoi(); $display(\"%0d\", x); end endmodule\n");
    assert_eq!(out, "42\n");
}

#[test]
fn atoi_negative_is_ieee_signed() {
    // IEEE §6.16.9: leading sign honored → -7. (iverilog 13 returns 0; its bug.)
    let out = run("module t; string s; int x;\n\
         initial begin s=\"-7\"; x=s.atoi(); $display(\"%0d\", x); end endmodule\n");
    assert_eq!(out, "-7\n");
}

#[test]
fn atoi_stops_at_nondigit() {
    let out = run("module t; string s; int x;\n\
         initial begin s=\"12ab\"; x=s.atoi(); $display(\"%0d\", x); end endmodule\n");
    assert_eq!(out, "12\n");
}

#[test]
fn atoi_empty_is_zero() {
    let out = run("module t; string s; int x;\n\
         initial begin s=\"\"; x=s.atoi(); $display(\"%0d\", x); end endmodule\n");
    assert_eq!(out, "0\n");
}

#[test]
fn atohex() {
    let out = run("module t; string s; int x;\n\
         initial begin s=\"ff\"; x=s.atohex(); $display(\"%0d\", x); end endmodule\n");
    assert_eq!(out, "255\n");
}

#[test]
fn atooct_hand_ieee() {
    // iverilog 13 lacks atooct; IEEE: octal "17" = 15.
    let out = run("module t; string s; int x;\n\
         initial begin s=\"17\"; x=s.atooct(); $display(\"%0d\", x); end endmodule\n");
    assert_eq!(out, "15\n");
}

#[test]
fn atobin_hand_ieee() {
    // iverilog 13 lacks atobin; IEEE: binary "101" = 5.
    let out = run("module t; string s; int x;\n\
         initial begin s=\"101\"; x=s.atobin(); $display(\"%0d\", x); end endmodule\n");
    assert_eq!(out, "5\n");
}

#[test]
fn atoreal() {
    let out = run("module t; string s; real r;\n\
         initial begin s=\"3.5\"; r=s.atoreal(); $display(\"%0.2f\", r); end endmodule\n");
    assert_eq!(out, "3.50\n");
}

// ── itoa family (int → string, in-place mutator statement) ───────────────────

#[test]
fn itoa() {
    let out = run("module t; string s;\n\
         initial begin s.itoa(255); $display(\"%s\", s); end endmodule\n");
    assert_eq!(out, "255\n");
}

#[test]
fn itoa_negative() {
    let out = run("module t; string s;\n\
         initial begin s.itoa(-9); $display(\"%s\", s); end endmodule\n");
    assert_eq!(out, "-9\n");
}

#[test]
fn hextoa() {
    let out = run("module t; string s;\n\
         initial begin s.hextoa(255); $display(\"%s\", s); end endmodule\n");
    assert_eq!(out, "ff\n");
}

#[test]
fn octtoa() {
    let out = run("module t; string s;\n\
         initial begin s.octtoa(255); $display(\"%s\", s); end endmodule\n");
    assert_eq!(out, "377\n");
}

#[test]
fn bintoa() {
    let out = run("module t; string s;\n\
         initial begin s.bintoa(5); $display(\"%s\", s); end endmodule\n");
    assert_eq!(out, "101\n");
}

#[test]
fn atoi_roundtrip_itoa() {
    let out = run("module t; string s; int x;\n\
         initial begin s.itoa(1234); x=s.atoi(); $display(\"%0d\", x); end endmodule\n");
    assert_eq!(out, "1234\n");
}
