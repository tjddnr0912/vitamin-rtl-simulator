//! A bare-statement `$dist_uniform(seed, lo, hi);` (return discarded) dropped the
//! SEED WRITEBACK: it hit `lower_systask` (W3056 skip) instead of advancing the ref
//! seed, so a later draw diverged from iverilog. The seed writeback is an rhs-based
//! statement effect (`StmtEffect::SeededDist`), so routing the bare form through the
//! same `dist_uniform_special` (with the return discarded via `emit_discarded_call`)
//! advances the seed exactly as iverilog does. §4.5.125 sibling. Only `$dist_uniform`
//! is routed — its integer LCG advance and draw match iverilog for all inputs; the
//! NON-uniform siblings have a pre-existing vita-vs-iverilog algorithm divergence
//! (seed LCG for chi_square/t, vendored-libm draw value for normal/exponential/
//! erlang) and stay warn+skip (a separate follow-on). Values pinned to LIVE
//! iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_bdu_{}_{n}", std::process::id()));
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
fn bare_dist_uniform_advances_seed() {
    // `$dist_uniform(s,0,100); x=$dist_uniform(s,0,100);` — the bare call advances the
    // seed, so the second draw differs from a fresh seed. Pinned to iverilog:
    // s ends -1917100901, x=55.
    let (out, c) = run("module t; integer s, x; initial begin \
         s = 5; $dist_uniform(s, 0, 100); x = $dist_uniform(s, 0, 100); \
         $display(\"R=%0d %0d\", s, x); #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("R=-1917100901 55"),
        "bare $dist_uniform must advance the seed; got:\n{out}"
    );
}

#[test]
fn bare_dist_uniform_seed_via_random() {
    // Isolate the SEED advance: after the bare `$dist_uniform`, a `$random(s)` (whose
    // LCG matches iverilog) draws — proving the seed advanced to iverilog's value
    // (230383387).
    let (out, c) = run("module t; integer s, r; initial begin \
         s = 5; $dist_uniform(s, 0, 9); r = $random(s); \
         $display(\"R=%0d\", r); #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("R=230383387"),
        "bare $dist_uniform seed advance matches iverilog; got:\n{out}"
    );
}

#[test]
fn assignment_form_unchanged() {
    // Byte-identity: `x = $dist_uniform(s,0,100)` is unchanged — the seed advances and
    // the draw matches iverilog (s=345346, x=0).
    let (out, c) = run("module t; integer s, x; initial begin \
         s = 5; x = $dist_uniform(s, 0, 100); \
         $display(\"R=%0d %0d\", s, x); #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("R=345346 0"),
        "assignment $dist_uniform unchanged; got:\n{out}"
    );
}

#[test]
fn bare_dist_uniform_in_loop() {
    // Three bare calls in a for-loop, each advancing the seed, then a draw — pinned
    // to iverilog.
    let (out, c) = run("module t; integer s, i, x; initial begin \
         s = 7; for (i = 0; i < 3; i = i + 1) $dist_uniform(s, 0, 50); x = $random(s); \
         $display(\"R=%0d\", x); #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("R=778959708"),
        "looped bare $dist_uniform advances the seed; got:\n{out}"
    );
}

#[test]
fn nonuniform_dist_bare_stays_skip() {
    // Correct-or-loud: `$dist_normal` (and the other non-uniform siblings) have a
    // pre-existing algorithm divergence, so their BARE form is NOT routed — it stays
    // a warn+skip that does NOT advance the seed (base==fix, NOT iverilog-matching —
    // iverilog would advance the seed here). `s` is unchanged by the bare call, so
    // `$random(s)` draws from the original seed 5 = -2147138048 (§4.5.125's first draw).
    let (out, c) = run("module t; integer s, r; initial begin \
         s = 5; $dist_normal(s, 0, 10); r = $random(s); \
         $display(\"R=%0d\", r); #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("R=-2147138048"),
        "non-uniform dist bare stays skip (seed unchanged); got:\n{out}"
    );
}
