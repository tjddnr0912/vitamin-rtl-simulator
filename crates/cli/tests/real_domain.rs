//! CLI smoke test (§5 test #15): pipe a real testbench through the `vita`
//! oneshot binary and assert the printed real. End-to-end coverage of the
//! real/realtime domain through the production CLI entry point.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn tmp_sv() -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("vita_real_{}_{n}.v", std::process::id()))
}

/// Write a temp `.sv`, run `vita <file>` (oneshot), capture stdout.
fn run_vita_oneshot(src: &str) -> String {
    let path = tmp_sv();
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("failed to run vita");
    let _ = std::fs::remove_file(&path);
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// 15. cli smoke — a real testbench computes the harmonic sum H4 and prints it.
// NOTE (deviation from the spec literal): `for` loops are an UNRELATED MVP gap
// in this codebase (elaborate emits VITA-W3008 "for loop skipped"), so the H4
// accumulation is written as the equivalent unrolled real assignments. The
// assertion (H4 = 1 + 1/2 + 1/3 + 1/4 = 2.083333) is identical.
#[test]
fn cli_smoke_real_testbench() {
    let src = r#"
module tb; real acc; integer k;
initial begin
  acc = 0.0;
  k = 1; acc = acc + (1.0 / k);
  k = 2; acc = acc + (1.0 / k);
  k = 3; acc = acc + (1.0 / k);
  k = 4; acc = acc + (1.0 / k);
  $display("H4=%f", acc);
end
endmodule
"#;
    let out = run_vita_oneshot(src);
    // 1 + 0.5 + 0.333333... + 0.25 = 2.083333...
    assert!(
        out.lines().any(|l| l == "H4=2.083333"),
        "expected 'H4=2.083333' in vita output, got:\n{out}"
    );
}

// Adversarial-review fix: `to_i128_signed` only reconstructed the sign for
// width ≤ 64, so a `signed [99:0]` holding −5 coerced to real as 0.0 (the
// 128-bit lane now covers 65..=128; iverilog: -5).
#[test]
fn wide_signed_negative_to_real() {
    let src = r#"
module t; reg signed [99:0] x; reg [99:0] u; real r;
initial begin
  x = -100'sd5; r = x; $display("neg=%g", r);
  u = 100'd5 <<< 70; r = u; $display("bigu=%g", r);
end
endmodule
"#;
    let out = run_vita_oneshot(src);
    assert!(out.contains("neg=-5\n"), "wide signed → real:\n{out}");
    // unsigned 65..128-bit values also flow through the widened lane
    // (5 × 2^70 ≈ 5.90295810358706e+21 in %g).
    assert!(
        out.contains("bigu=5.90296e+21"),
        "wide unsigned → real:\n{out}"
    );
}

/// A6 ABSOLUTE ANCHOR — the `real` domain on TIER-3, iverilog-pinned.
///
/// `real` was refused by TWO gate rows that named it the same way (the design
/// gate's `real` and the storage gate's `real: S2 width class`), and the width
/// was never the problem: an f64's 64 bits are ordinary word storage and the
/// engine keeps them in exactly that. What tier-3 lacked was the FLAG — a
/// `Slot::is_real` stamped onto every read — and two of the four arms of the
/// real↔int assignment coercion, which are now one shared `value::coerce_assign`
/// that both write funnels call.
///
/// Every line below is a discriminator for one of those pieces:
///   * `r = 5` / `r = b` — the int→real CONVERT arm, unreachable on tier-3 until
///     a real DESTINATION could exist. Without it the integer bit pattern lands
///     in the slot and `%f` prints 0.000000 (5 reinterpreted as an f64 is
///     2.5e-323). `r = b` makes the source a NET, so it also rides the store.
///   * `i = s` and `i = -s` — the real→int ROUND arm (half-AWAY-from-zero, not
///     to-even: 3.5 → 4 and -3.5 → -4), which S1c mirrored by hand.
///   * `s = r / 2.0` and `m[1] + m[3]` — arithmetic that only happens if the
///     READ stamped `is_real`; without the stamp these are integer ops on IEEE
///     bit patterns.
///   * `{a, b} = 3.7` — the multi-chunk round (the width is the SUM of the
///     chunks, not one net's).
///   * `x[3] = 1.5` — a real VALUE into a bit-select: rounds at the NET's width
///     (32) and the resize then takes bit 0, so `x` stays 0. This is the arm
///     that must NOT see a real destination.
///   * `m[9] ? 1 : 0` — the out-of-range element, and the ONLY shape that
///     discriminates the OOB arm's stamp: rendering does not (an all-X real and
///     an all-X integer both print 0.000000), but `truthiness` does — a real
///     asks "nonzero?" and answers 0, an integer sees X and answers X. Measured
///     by mutation, not assumed.
///
/// ⚠️ ANTI-VACUITY: run.json must say the design actually ran natively. A refused
/// design falls back to the VM, where every one of these lines already passed.
///
/// ⚠️ `$bits(r)` is deliberately absent: vita and verilator both say 64,
/// iverilog says 1, and a known divergence in an anchor stops it being an anchor.
#[test]
fn real_domain_on_tier_3_matches_iverilog() {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_real_nat_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(
        &f,
        "module top;\n\
           real r, s;\n\
           real m [0:3];\n\
           integer i;\n\
           reg [7:0] b, aa, bb;\n\
           reg [31:0] x;\n\
           initial begin\n\
             r = 5;                          // int -> real CONVERT\n\
             $display(\"a %0f\", r);\n\
             b = 8'd7; r = b;                // net int -> real\n\
             $display(\"b %0f\", r);\n\
             s = r / 2.0;                    // real arithmetic (needs the read stamp)\n\
             $display(\"c %0f\", s);\n\
             i = s;  $display(\"d %0d\", i);   // real -> int ROUND, half away\n\
             i = -s; $display(\"e %0d\", i);\n\
             m[0] = 0.0; m[1] = 1.5; m[2] = 3.0; m[3] = 4.5;\n\
             m[2] = m[1] + m[3];\n\
             $display(\"f %0f %0f\", m[2], m[0]);\n\
             {aa, bb} = 3.7;                 // multi-chunk round\n\
             $display(\"g %0d %0d\", aa, bb);\n\
             x = 0; x[3] = 1.5;              // real into a bit-select\n\
             $display(\"h %0d\", x);\n\
             i = 9;\n\
             $display(\"i %0d\", m[i] ? 1 : 0);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--backend")
        .arg("native")
        .arg("--obs-dir")
        .arg("obs")
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let txt = String::from_utf8_lossy(&out.stdout).into_owned();
    let rj = std::fs::read_to_string(d.join("obs").join("run.json")).unwrap_or_default();
    assert!(
        rj.contains("\"backend\": \"native\""),
        "the design did not run natively:\n{rj}\n{txt}"
    );
    let mut body = String::new();
    for l in txt.lines().filter(|l| !l.starts_with("simulation ended")) {
        body.push_str(l);
        body.push('\n');
    }
    assert_eq!(
        body,
        "a 5.000000\n\
         b 7.000000\n\
         c 3.500000\n\
         d 4\n\
         e -4\n\
         f 6.000000 0.000000\n\
         g 0 4\n\
         h 0\n\
         i 0\n",
        "iverilog-pinned real behaviour on tier-3"
    );
}
