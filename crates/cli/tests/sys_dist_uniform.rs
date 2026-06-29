//! v9 Medium-bundle rank 6: `$dist_uniform(seed, start, end)` (IEEE 1364-2005
//! Annex `rtl_dist_uniform`). A uniform integer in `[start, end]` inclusive that
//! advances the ref `seed` by one Annex draw. Pure f64 mul/add/floor — no libm,
//! so it is BOTH 3-OS deterministic AND iverilog-byte-identical (every value
//! here is pinned to LIVE iverilog 13.0).
//!
//! (`$dist_normal`/`exponential`/`poisson`/`chi_square`/`t`/`erlang` LANDED in
//! v19 via the vendored libm — see `crates/cli/tests/dist_transcendental.rs`.
//! Their SEED stream is iverilog-byte-identical; the result int is vitamin's
//! 3-OS deterministic pin, NOT an iverilog byte-match — D3.)
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_distu_{}_{n}", std::process::id()));
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
        out.status.code(),
    )
}

#[test]
fn sequence_and_seed_writeback() {
    // iverilog live: s=1 → 0 (seed 69070) → 11 (seed 475628535).
    let (out, _c) = run("module t;\n\
         integer s, r;\n\
         initial begin\n\
           s = 1;\n\
           r = $dist_uniform(s, 0, 99); $display(\"A %0d %0d\", r, s);\n\
           r = $dist_uniform(s, 0, 99); $display(\"B %0d %0d\", r, s);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("A 0 69070"), "first draw + seed:\n{out}");
    assert!(out.contains("B 11 475628535"), "second draw + seed:\n{out}");
}

#[test]
fn negative_and_wide_ranges() {
    // iverilog live: (-5,5) s=3 → -5 (seed 207208); (0,999999) s=99 → 1592 (seed 6837832).
    let (out, _c) = run("module t;\n\
         integer s, r;\n\
         initial begin\n\
           s = 3;  r = $dist_uniform(s, -5, 5);      $display(\"N %0d %0d\", r, s);\n\
           s = 99; r = $dist_uniform(s, 0, 999999);  $display(\"W %0d %0d\", r, s);\n\
         end\n\
         endmodule\n");
    assert!(out.contains("N -5 207208"), "negative range:\n{out}");
    assert!(out.contains("W 1592 6837832"), "wide range:\n{out}");
}

#[test]
fn variable_bounds() {
    // bounds may be runtime expressions: (100,200) s=2 → 100 (seed 138139).
    let (out, _c) = run("module t;\n\
         integer s, r, lo, hi;\n\
         initial begin s=2; lo=100; hi=200; r=$dist_uniform(s,lo,hi); $display(\"V %0d %0d\", r, s); end\n\
         endmodule\n");
    assert!(out.contains("V 100 138139"), "variable bounds:\n{out}");
}

#[test]
fn zero_seed_substitution() {
    // seed 0 → 259341593 before the LCG (Annex / iverilog): (0,99) s=0 → 57
    // (seed -1844104698), NOT 0 (which the un-substituted 69069*0+1=1 would give).
    let (out, _c) = run("module t;\n\
         integer s, r;\n\
         initial begin s=0; r=$dist_uniform(s,0,99); $display(\"Z %0d %0d\", r, s); end\n\
         endmodule\n");
    assert!(
        out.contains("Z 57 -1844104698"),
        "zero-seed substitution:\n{out}"
    );
}

#[test]
fn degenerate_range_returns_start_no_advance() {
    // start >= end returns start and does NOT advance the seed (Annex guard):
    // s=5, (50,50) → 50, seed still 5.
    let (out, _c) = run("module t;\n\
         integer s, r;\n\
         initial begin s=5; r=$dist_uniform(s,50,50); $display(\"D %0d %0d\", r, s); end\n\
         endmodule\n");
    assert!(out.contains("D 50 5"), "degenerate range:\n{out}");
}

#[test]
fn nondirect_placement_is_loud() {
    // $dist_uniform advances the ref seed: legal ONLY as the direct rhs of a
    // blocking assign. A nested placement is loud E3009, never a silent draw.
    let (out, code) = run("module t;\n\
         integer s, r;\n\
         initial begin s=1; r = 1 + $dist_uniform(s, 0, 9); end\n\
         endmodule\n");
    assert!(
        out.contains("VITA-E3009") || code == Some(1),
        "nested $dist_uniform must be loud: {out} code={code:?}"
    );
}

#[test]
fn nonuniform_dist_funcs_now_work() {
    // v19: the transcendental siblings LANDED (vendored libm). As a direct-rhs
    // seeded draw each succeeds (exit 0) — full value/seed pins live in
    // dist_transcendental.rs; here we just confirm they are no longer loud.
    for f in [
        "$dist_normal(s,50,10)",
        "$dist_exponential(s,10)",
        "$dist_poisson(s,5)",
        "$dist_chi_square(s,4)",
        "$dist_t(s,4)",
        "$dist_erlang(s,3,10)",
    ] {
        let (out, code) = run(&format!(
            "module t;\n integer s, r;\n initial begin s=3; r={f}; $display(\"%0d\", r); end\nendmodule\n"
        ));
        assert!(
            code == Some(0) && !out.contains("VITA-E3009"),
            "{f} must now succeed: {out} code={code:?}"
        );
    }
}

#[test]
fn arity_error_is_loud() {
    // $dist_uniform needs exactly (seed, start, end).
    let (_o, code) = run("module t;\n\
         integer s, r;\n\
         initial begin s=1; r=$dist_uniform(s, 0); end\n\
         endmodule\n");
    assert_eq!(code, Some(1), "wrong arity must be loud");
}

#[test]
fn sub_32bit_seed_is_loud() {
    // review M1: iverilog rejects a seed narrower than 32 bits; vita must too (a
    // narrower seed would truncate the 32-bit LCG state on write-back → a silently
    // wrong sequence), NOT silently accept and truncate. (integer / reg[31:0] are ok.)
    let (out, code) = run("module t;\n\
         reg [7:0] s; integer r;\n\
         initial begin s = 1; r = $dist_uniform(s, 0, 99); end\n\
         endmodule\n");
    assert!(
        out.contains("VITA-E3009") || code == Some(1),
        "sub-32-bit seed must be loud: {out} code={code:?}"
    );
}
