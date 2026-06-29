//! N6 non-uniform `$dist_*` transcendentals (IEEE 1364 Annex): `$dist_normal`,
//! `$dist_exponential`, `$dist_poisson`, `$dist_chi_square`, `$dist_t`,
//! `$dist_erlang`. Each advances the ref seed VAR (a direct-rhs-only intercept,
//! like `$dist_uniform`/`$random(seed)`) and returns an int.
//!
//! CONTRACT (D3 — internal-determinism-first): the SEED stream is byte-identical
//! to iverilog 13.0 (pure-integer Annex LCG — verified here), so the algorithms
//! issue the exact same `uniform` draws as the reference. The returned int is
//! vitamin's 3-OS deterministic value (computed via the vendored pure-Rust libm);
//! it matches iverilog for most draws and differs by ±1 on a few (the final
//! float→int cast where the vendored libm differs from iverilog's platform libm
//! by ~1 ULP). These pins are the vitamin reproducibility contract, NOT an
//! iverilog byte-match — `$dist_*` non-uniform results are implementation-defined.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita(src: &str) -> std::process::Output {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_dist_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    out
}

fn run(src: &str) -> String {
    let out = vita(src);
    assert!(
        out.status.success(),
        "vita failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut s = String::new();
    for l in so.lines().filter(|l| {
        !l.starts_with("simulation ended") && !l.contains("VITA-W") && !l.trim().is_empty()
    }) {
        s.push_str(l.trim());
        s.push('\n');
    }
    s
}

fn run_loud(src: &str) {
    let out = vita(src);
    assert!(
        !out.status.success(),
        "expected a loud refusal, but vita succeeded:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

/// The SEED advances are byte-identical to iverilog (the integer-deterministic
/// structural contract). The result ints are vitamin's deterministic pins; the
/// iverilog cross-reference is noted where it differs by the ±1 D3 rounding gap.
#[test]
fn dist_seed_streams_match_iverilog() {
    let out = run("module top;\n\
        integer s, i, r;\n\
        initial begin\n\
          s=1; for(i=0;i<3;i=i+1) begin r=$dist_exponential(s,20); $display(\"E %0d %0d\",r,s); end\n\
          s=1; r=$dist_normal(s,100,10);  $display(\"N %0d %0d\",r,s);\n\
          s=1; r=$dist_poisson(s,5);      $display(\"P %0d %0d\",r,s);\n\
          s=1; r=$dist_chi_square(s,4);   $display(\"C %0d %0d\",r,s);\n\
          s=1; r=$dist_t(s,4);            $display(\"T %0d %0d\",r,s);\n\
          s=1; r=$dist_erlang(s,3,20);    $display(\"G %0d %0d\",r,s);\n\
        end\n\
      endmodule\n");
    // Column 2 (seed) is byte-identical to iverilog. Column 1 (result) is the
    // vitamin pin; iverilog: E 221 (Δ1), N 106 (Δ1), the rest identical.
    assert_eq!(
        out,
        "E 220 69070\n\
         E 44 475628535\n\
         E 5 -1017563188\n\
         N 105 772999773\n\
         P 0 69070\n\
         C 26 475628535\n\
         T 0 772999773\n\
         G 90 -1017563188\n"
    );
}

/// Determinism: the same seed replays the same stream (the reproducibility pin).
#[test]
fn dist_is_replayable() {
    let out = run("module top;\n\
        integer s, a, b;\n\
        initial begin\n\
          s=42; a=$dist_poisson(s,7);\n\
          s=42; b=$dist_poisson(s,7);\n\
          $display(\"%0d\", a==b);\n\
        end\n\
      endmodule\n");
    assert_eq!(out, "1\n");
}

/// A nested (non-direct-rhs) placement is LOUD, never a silent unseeded draw —
/// matching the `$dist_uniform`/`$random(seed)` policy.
#[test]
fn nested_dist_is_loud() {
    run_loud(
        "module top;\n\
        integer s, r;\n\
        initial begin s=1; r = $dist_normal(s,0,1) + 1; $display(\"%0d\", r); end\n\
      endmodule\n",
    );
}

/// A sub-32-bit seed is LOUD (the 32-bit LCG state would truncate on write-back).
#[test]
fn narrow_seed_is_loud() {
    run_loud(
        "module top;\n\
        reg [15:0] s; integer r;\n\
        initial begin s=1; r = $dist_exponential(s, 10); $display(\"%0d\", r); end\n\
      endmodule\n",
    );
}
