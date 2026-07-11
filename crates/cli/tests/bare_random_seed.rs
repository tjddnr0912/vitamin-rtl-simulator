//! A bare-statement seeded `$random(seed);` (return discarded) silently dropped
//! the SEED WRITEBACK: it hit `lower_systask` (W3056 skip) instead of advancing the
//! ref seed, so a later `$random(seed)` drew from the un-advanced seed and diverged
//! from iverilog. `$random`'s seed writeback is an rhs-based statement effect
//! (`StmtEffect::SeededRandom`), so routing the bare form through the same special
//! (with the return discarded via `emit_discarded_call`) advances the seed exactly
//! as iverilog does. §4.5.123 sibling. vita's LCG matches iverilog's, so the drawn
//! values are pinned to LIVE iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_brs_{}_{n}", std::process::id()));
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

/// Run `src` (which dumps to `w.vcd`) and return the generated VCD text.
fn run_vcd(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_brsv_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    std::fs::read_to_string(d.join("w.vcd")).unwrap_or_default()
}

#[test]
fn bare_random_discard_net_not_in_vcd() {
    // The bare form routes through `emit_discarded_call` → a `$ia_tmp$<n>` throwaway
    // net. That internal net must NOT surface in `$dumpvars` output (iverilog emits
    // no such net); only the user's `seed`/`r` do. (Also fixes the pre-existing leak
    // of `$ia_tmp$` for bare `$sscanf` / intra-delay `$random`.)
    let vcd = run_vcd("module tb; integer seed; reg [31:0] r; integer i;\n\
         initial begin $dumpfile(\"w.vcd\"); $dumpvars(0, tb);\n\
           seed = 5; $random(seed); for (i=0;i<3;i=i+1) r = $random(seed); #1 $finish; end endmodule\n");
    assert!(!vcd.is_empty(), "VCD not generated");
    assert!(
        !vcd.contains("$ia_tmp$"),
        "internal $ia_tmp net leaked into VCD:\n{vcd}"
    );
    assert!(
        vcd.contains("seed"),
        "user net `seed` missing from VCD:\n{vcd}"
    );
}

#[test]
fn bare_random_advances_seed() {
    // `$random(s); x=$random(s);` — the bare call advances the seed, so the second
    // draw differs from a fresh seed. Pinned to iverilog: s ends -1917100901, x=230383387.
    let (out, c) = run("module t; integer s, x; initial begin \
         s = 5; $random(s); x = $random(s); \
         $display(\"R=%0d %0d\", s, x); #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("R=-1917100901 230383387"),
        "bare $random(seed) must advance the seed; got:\n{out}"
    );
}

#[test]
fn bare_random_multiple_and_loop() {
    // Two bare calls, and three in a for-loop, each advancing the seed — pinned to
    // iverilog (1129920902 and 778959708 respectively).
    let (out, c) = run("module t; integer s, x, i, y; initial begin \
         s = 1; $random(s); $random(s); x = $random(s); \
         s = 7; for (i = 0; i < 3; i = i + 1) $random(s); y = $random(s); \
         $display(\"R=%0d %0d\", x, y); #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("R=1129920902 778959708"),
        "multiple / looped bare $random advance the seed; got:\n{out}"
    );
}

#[test]
fn assignment_form_unchanged() {
    // Byte-identity: the assignment form `x = $random(s)` is unchanged — the seed
    // advances and the draw matches iverilog (s=345346, x=-2147138048).
    let (out, c) = run("module t; integer s, x; initial begin \
         s = 5; x = $random(s); \
         $display(\"R=%0d %0d\", s, x); #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("R=345346 -2147138048"),
        "assignment $random unchanged; got:\n{out}"
    );
}

#[test]
fn unseeded_bare_random_is_noop() {
    // An UNSEEDED bare `$random;` has no ref seed → nothing to write back → it is a
    // harmless discarded draw (no crash, exit 0). The subsequent unseeded draw is
    // well-defined; we only assert clean completion.
    let (out, c) = run("module t; integer x; initial begin \
         $random; x = $random; \
         $display(\"R=done\"); #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("R=done"),
        "unseeded bare $random is a no-op; got:\n{out}"
    );
}

#[test]
fn reg_seed_advances() {
    // A `reg [31:0]` seed works the same as `integer` (pinned to iverilog 646214477).
    let (out, c) = run("module t; reg [31:0] s; integer x; initial begin \
         s = 42; $random(s); x = $random(s); \
         $display(\"R=%0d\", x); #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("R=646214477"),
        "reg seed advances; got:\n{out}"
    );
}
