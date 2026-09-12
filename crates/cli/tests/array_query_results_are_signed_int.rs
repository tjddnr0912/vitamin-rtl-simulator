//! IEEE 1800-2017 §20.6.2 / §20.7: `$bits`, `$size`, `$left`, `$right`, `$low`,
//! `$high`, `$increment`, `$dimensions` and `$unpacked_dimensions` return `int` —
//! a SIGNED 32-bit value.
//!
//! vita folds every one of them to a constant at elaborate, and the constant was
//! built UNSIGNED (`const_u32_expr` / `const_param_expr`'s legacy shape), so a sum
//! with a negative sibling picked the unsigned context and zero-extended it:
//! `$bits(u8) + $signed(q8)` with `q8 = -32` printed `000000e8` where both oracles
//! print `ffffffe8`; `$bits(u8) / -2` was `0` for `-4`. One spelling
//! (`int_result_expr`) now materializes the whole family as a signed 32-bit const,
//! including the deferred hierarchical `$bits(u.x)` placeholder and its patch.
//!
//! Surfaced by the it8 differential lens: the false-loud E3001 that
//! `always_comb_sysfunc_arg_is_a_read.rs` removes used to hide this class in the
//! one shape `always_comb a = $bits(u8) + …` beside an initializer; the class was
//! reachable in an `initial` on PRE already.
//!
//! ORACLES: iverilog 13.0 (-g2012) and verilator 5.052 (--binary --timing); every
//! value below was measured in BOTH. PRE values are from a release binary built at
//! the parent commit.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_aqsi_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let so = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::remove_dir_all(&d);
    so
}

/// ① The whole family beside a negative signed sibling, and three consumers that
/// only a signed result answers right: a signed division, a comparison against a
/// negative, a subtraction below zero.
#[test]
fn every_array_query_result_is_a_signed_int() {
    let o = run(r#"module t;
  logic [7:0] u8 = 8'hF7;
  logic signed [7:0] q8 = -32;
  logic [3:0] arr [0:6];
  logic [31:0] a, b, c, d, e, f, g, h, i2;
  initial begin
    a = $bits(u8) + $signed(q8);
    b = $size(arr) + $signed(q8);
    c = $high(arr) + $signed(q8);
    d = $low(arr) + $signed(q8);
    e = $left(u8) + $signed(q8);
    f = $right(u8) + $signed(q8);
    g = $increment(u8) + $signed(q8);
    h = $dimensions(arr) + $signed(q8);
    i2 = $unpacked_dimensions(arr) + $signed(q8);
    $display("%h %h %h %h %h %h %h %h %h", a, b, c, d, e, f, g, h, i2);
    $display("%0d %0d %0d", $bits(u8) / -2, $size(arr) > -1, $bits(u8) - 9);
    #1 $finish;
  end
endmodule
"#);
    // PRE: `000000e8 000000e7 000000e6 000000e0 000000e7 000000e0 000000e1 000000e2
    // 000000e1` and `0 0 4294967295`.
    assert_eq!(
        o,
        "ffffffe8 ffffffe7 ffffffe6 ffffffe0 ffffffe7 ffffffe0 ffffffe1 ffffffe2 ffffffe1\n\
         -4 1 -1"
    );
}

/// ② The `always_comb` shape the removed false-loud used to hide, a `$bits` of a
/// literal / concat / hierarchical net, and the values that never depended on the
/// sign (the plain `$countones` / `$clog2` siblings and an unsigned context).
#[test]
fn the_always_comb_shape_and_the_sign_free_controls() {
    let o = run(r#"module sub; logic [11:0] x; endmodule
module t;
  sub u();
  logic [7:0] u8 = 8'hF7, b8 = 8'hFF;
  logic signed [7:0] s8 = -9, q8 = -32;
  logic [31:0] a4, k1, k2, k3, k4, k5;
  always_comb a4 = $bits(u8) + $signed(q8);
  initial begin
    k1 = $bits(16'h1234) + $signed(q8);
    k2 = $bits({u8, b8}) + $signed(q8);
    k3 = $bits(u.x) + $signed(q8);
    k4 = $countones(s8) + $clog2(b8) + $signed(q8);
    k5 = $bits(u8) + u8;
    #1 $display("%h %h %h %h %h %h", a4, k1, k2, k3, k4, k5);
    #1 $finish;
  end
endmodule
"#);
    // PRE: E3001 on `u8` (the false-loud); with the loud removed and the fold still
    // unsigned, the first and fourth cells read `000000e8` / `000000ec`.
    assert_eq!(o, "ffffffe8 fffffff0 fffffff0 ffffffec ffffffef 000000ff");
}
