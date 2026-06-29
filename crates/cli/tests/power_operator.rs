//! Power operator `**` corrections (pre-existing silent-wrongs surfaced by the
//! fill width hunt, reproducible without any fill). (1) A narrow UNSIGNED
//! exponent (`1'b1` = +1, `2'd2` = +2) was restamped signed and sign-extended to
//! a negative value, so `2 ** 1'b1` gave 0. (2) A wide base power overflowed u128
//! and returned 0 instead of wrapping mod 2^w, so `64'hF..F ** 64'd3` gave 0 (it
//! is all-ones). Oracle: iverilog -g2012.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_pow_{}_{n}.sv", std::process::id()));
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
fn narrow_unsigned_exponent() {
    let out = run("module t;\n\
           logic [63:0] a;\n\
           initial begin\n\
             a = 2 ** 1'b1; $display(\"%0d\", a);\n\
             a = 3 ** 2'd2; $display(\"%0d\", a);\n\
             a = 2 ** 2'd3; $display(\"%0d\", a);\n\
           end endmodule\n");
    assert_eq!(out, "2\n9\n8\n");
}

#[test]
fn signed_negative_exponent_still_zero() {
    // A GENUINE negative exponent (|base| > 1) is still 0 — the fix only stops a
    // POSITIVE narrow exponent from being read as negative.
    let out = run("module t;\n\
           integer a;\n\
           initial begin a = 2 ** (-1); $display(\"%0d\", a); end\n\
         endmodule\n");
    assert_eq!(out, "0\n");
}

#[test]
fn wide_base_wraps_mod_2w() {
    let out = run("module t;\n\
           logic [63:0] a; logic [127:0] b;\n\
           initial begin\n\
             a = 64'hFFFFFFFFFFFFFFFF ** 64'd3; $display(\"%h\", a);\n\
             a = 64'd10 ** 64'd20;             $display(\"%h\", a);\n\
             b = 128'd3 ** 128'd5;             $display(\"%h\", b);\n\
           end endmodule\n");
    assert_eq!(
        out,
        "ffffffffffffffff\n6bc75e2d63100000\n000000000000000000000000000000f3\n"
    );
}

#[test]
fn left_associative() {
    // IEEE Table 11-2 / iverilog: `**` is left-associative.
    let out = run("module t; initial begin\n\
           $display(\"%0d\", 2 ** 2 ** 3);\n\
           $display(\"%0d\", 2 ** 3 ** 2);\n\
         end endmodule\n");
    // left-assoc: (2**2)**3 = 4**3 = 64 ; (2**3)**2 = 8**2 = 64.
    assert_eq!(out, "64\n64\n");
}

#[test]
fn self_determined_width_is_left_operand() {
    // IEEE Table 11-21: result width = left operand width. In a self-determined
    // context, `b4 ** e8` (b4 is [3:0]) is a 4-bit result.
    let out = run("module t;\n\
           logic [3:0] b4; logic [7:0] e8;\n\
           initial begin\n\
             b4 = 3; e8 = 4; $display(\"%0d\", b4 ** e8);\n\
             b4 = 5; $display(\"%0d\", b4 ** 3);\n\
             $display(\"%b\", 4'bx ** 2);\n\
           end endmodule\n");
    assert_eq!(out, "1\n13\nxxxx\n");
}

#[test]
fn zero_to_negative_is_x() {
    let out = run("module t; initial $display(\"%b\", 0 ** (-1)); endmodule\n");
    assert_eq!(out, "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\n");
}

#[test]
fn exponent_zero_and_one() {
    let out = run("module t;\n\
           logic [31:0] a;\n\
           initial begin\n\
             a = 7 ** 0; $display(\"%0d\", a);\n\
             a = 7 ** 1; $display(\"%0d\", a);\n\
             a = 5 ** 2; $display(\"%0d\", a);\n\
           end endmodule\n");
    assert_eq!(out, "1\n7\n25\n");
}
