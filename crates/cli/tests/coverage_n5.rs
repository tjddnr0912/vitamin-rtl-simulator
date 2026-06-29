//! N5 (MVP): functional coverage — `covergroup NAME; coverpoint EXPR; endgroup`,
//! `cg c = new;`, `c.sample()`, `c.get_coverage()`. iverilog 13.0 REJECTS covergroup
//! entirely, so this is HAND-IEEE: each coverpoint gets auto-bins (min(2^W, 64)); a
//! 64-bit hit-bitmap reg ORs in `1 << (value & 63)` on each sample(); get_coverage()
//! reports `sum($countones(bitmap)) * 100 / sum(num_bins)` (an integer %). Pure IR-0
//! (parser + elaborate bitmap synthesis; AST `.vu` re-pin only, sim-ir/fmt_ver 9
//! unchanged). Every expected % is HAND-COMPUTED from the auto-bin model.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cov_{}_{n}", std::process::id()));
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
fn single_coverpoint_auto_bins() {
    // x is 4-bit ⇒ 16 auto-bins; values {0,5} hit 2 bins ⇒ 2*100/16 = 12.5% (REAL).
    // get_coverage() returns a real (§19.5); `%g` shows 12.5 (vs `%0d` which rounds
    // a real to 13).
    let (out, _c) = run("module t;\n\
         reg [3:0] x;\n\
         covergroup cg; cp_x: coverpoint x; endgroup\n\
         cg c = new;\n\
         initial begin x=0; c.sample(); x=5; c.sample(); x=5; c.sample();\n\
           $display(\"COV %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(out.contains("COV 12.5"), "auto-bins real coverage:\n{out}");
}

#[test]
fn full_coverage() {
    // x is 2-bit ⇒ 4 bins; sampling all four ⇒ 100%.
    let (out, _c) = run("module t;\n\
         reg [1:0] x;\n\
         covergroup cg; coverpoint x; endgroup\n\
         cg c = new;\n\
         initial begin x=0;c.sample(); x=1;c.sample(); x=2;c.sample(); x=3;c.sample();\n\
           $display(\"F %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(out.contains("F 100"), "full coverage:\n{out}");
}

#[test]
fn quarter_coverage() {
    // a is 3-bit ⇒ 8 bins; {0,1} ⇒ 2*100/8 = 25%.
    let (out, _c) = run("module t;\n\
         reg [2:0] a;\n\
         covergroup cg; coverpoint a; endgroup\n\
         cg c = new;\n\
         initial begin a=0;c.sample(); a=1;c.sample();\n\
           $display(\"Q %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(out.contains("Q 25"), "quarter coverage:\n{out}");
}

#[test]
fn distinct_value_counted_once() {
    // sampling x=1 twice hits ONE bin ⇒ 1*100/4 = 25% (not 50%).
    let (out, _c) = run("module t;\n\
         reg [1:0] x;\n\
         covergroup cg; coverpoint x; endgroup\n\
         cg c = new;\n\
         initial begin x=1; c.sample(); c.sample();\n\
           $display(\"D %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(out.contains("D 25"), "distinct counting:\n{out}");
}

#[test]
fn multi_coverpoint() {
    // two 2-bit coverpoints, each hit 2/4 bins ⇒ (2+2)*100/(4+4) = 50%.
    let (out, _c) = run("module t;\n\
         reg [1:0] a, b;\n\
         covergroup cg; cp_a: coverpoint a; cp_b: coverpoint b; endgroup\n\
         cg c = new;\n\
         initial begin a=0;b=0;c.sample(); a=1;b=1;c.sample();\n\
           $display(\"M %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(out.contains("M 50"), "multi-coverpoint:\n{out}");
}

#[test]
fn sampling_event_header_no_edge_keeps_explicit() {
    // a `covergroup cg @(posedge clk);` header is accepted; with clk never toggling
    // the auto-sample never fires, so explicit sample() still drives coverage. x=1
    // once ⇒ 1/4 = 25% (slice F coexistence: auto + explicit).
    let (out, _c) = run("module t;\n\
         reg clk; reg [1:0] x;\n\
         covergroup cg @(posedge clk); coverpoint x; endgroup\n\
         cg c = new;\n\
         initial begin x=1; c.sample();\n\
           $display(\"E %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(out.contains("E 25"), "event-header covergroup:\n{out}");
}

#[test]
fn zero_coverage_before_any_sample() {
    let (out, _c) = run("module t;\n\
         reg [1:0] x;\n\
         covergroup cg; coverpoint x; endgroup\n\
         cg c = new;\n\
         initial $display(\"Z %0d\", c.get_coverage());\n\
         endmodule\n");
    assert!(out.contains("Z 0"), "zero coverage before sample:\n{out}");
}

#[test]
fn unknown_covergroup_type_is_loud() {
    let (out, code) = run("module t;\n\
         nosuch c = new;\n\
         initial $display(\"%0d\", c.get_coverage());\n\
         endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "unknown covergroup type must be loud: {out} {code:?}"
    );
}

// ─────────────── slice A: explicit bins (hand-IEEE, no live oracle) ───────────────
// iverilog 13.0 rejects covergroup entirely; every expected % is hand-computed from
// the per-bin model: coverage = covered_counting_bins / counting_bins * 100 (int).

#[test]
fn a1_value_list_bin() {
    // `bins a = {0,1,2}` — ONE counting bin (set membership). x=0 hits it, x=5 misses.
    let (out, _c) = run("module t;\n\
         reg [3:0] x;\n\
         covergroup cg; cp_x: coverpoint x { bins a = {0,1,2}; } endgroup\n\
         cg c = new;\n\
         initial begin x=0; c.sample(); x=5; c.sample();\n\
           $display(\"A1 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(out.contains("A1 100"), "value-list bin:\n{out}");
}

#[test]
fn a2_range_bins() {
    // two range bins ⇒ 2 counting bins; x=2 hits lo, x=7 hits neither ⇒ 1/2 = 50.
    let (out, _c) = run("module t;\n\
         reg [3:0] x;\n\
         covergroup cg; coverpoint x { bins lo = {[0:3]}; bins hi = {[12:15]}; } endgroup\n\
         cg c = new;\n\
         initial begin x=2; c.sample(); x=7; c.sample();\n\
           $display(\"A2 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(out.contains("A2 50"), "range bins:\n{out}");
}

#[test]
fn a3_mixed_open_range_list() {
    // `bins m = {0,[2:4],7}` — ONE bin over {0,2,3,4,7}. x=3 ∈ set ⇒ 100; x=1 ∉ ⇒ 0.
    let (hit, _c) = run("module t;\n\
         reg [3:0] x;\n\
         covergroup cg; coverpoint x { bins m = {0,[2:4],7}; } endgroup\n\
         cg c = new;\n\
         initial begin x=3; c.sample(); $display(\"A3 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(hit.contains("A3 100"), "mixed hit:\n{hit}");
    let (miss, _c) = run("module t;\n\
         reg [3:0] x;\n\
         covergroup cg; coverpoint x { bins m = {0,[2:4],7}; } endgroup\n\
         cg c = new;\n\
         initial begin x=1; c.sample(); $display(\"A3 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(miss.contains("A3 0"), "mixed miss:\n{miss}");
}

#[test]
fn a4_array_bins_one_bit_per_value() {
    // `bins arr[] = {[0:3]}` ⇒ 4 counting bins (one per value). x=0,1 ⇒ 2/4 = 50.
    let (out, _c) = run("module t;\n\
         reg [3:0] x;\n\
         covergroup cg; coverpoint x { bins arr[] = {[0:3]}; } endgroup\n\
         cg c = new;\n\
         initial begin x=0; c.sample(); x=1; c.sample();\n\
           $display(\"A4 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(out.contains("A4 50"), "array-bins partial:\n{out}");
    // all four sampled ⇒ 100.
    let (full, _c) = run("module t;\n\
         reg [3:0] x;\n\
         covergroup cg; coverpoint x { bins arr[] = {[0:3]}; } endgroup\n\
         cg c = new;\n\
         initial begin x=0;c.sample(); x=1;c.sample(); x=2;c.sample(); x=3;c.sample();\n\
           $display(\"A4 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(full.contains("A4 100"), "array-bins full:\n{full}");
}

#[test]
fn a5_ignore_removes_from_denominator() {
    // `bins arr[]={[0:3]}` (4 elems) + `ignore_bins ig={1,2}` ⇒ effective {0,3} = 2
    // counting bins. x=0 ⇒ 1/2 = 50 (ignored 1,2 are removed, NOT capped at 25).
    let (out, _c) = run("module t;\n\
         reg [3:0] x;\n\
         covergroup cg; coverpoint x { bins arr[] = {[0:3]}; ignore_bins ig = {1,2}; } endgroup\n\
         cg c = new;\n\
         initial begin x=0; c.sample();\n\
           $display(\"A5 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        out.contains("A5 50"),
        "ignore removes from denominator:\n{out}"
    );
}

#[test]
fn a6_illegal_bin_errors_and_excluded() {
    // `illegal_bins bad={8,9}` ⇒ sampling x=8 fires $error, not counted. bin a={[0:7]}
    // is the only counting bin; x=3 covers it ⇒ 100, and one illegal error printed.
    let (out, code) = run("module t;\n\
         reg [3:0] x;\n\
         covergroup cg; coverpoint x { bins a = {[0:7]}; illegal_bins bad = {8,9}; } endgroup\n\
         cg c = new;\n\
         initial begin x=8; c.sample(); x=3; c.sample();\n\
           $display(\"A6 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        out.contains("A6 100"),
        "illegal excluded from coverage:\n{out}"
    );
    assert!(
        out.contains("illegal coverage bin") || out.contains("VITA-E") || code == Some(1),
        "illegal bin hit must be loud:\n{out} {code:?}"
    );
}

#[test]
fn a7_default_bin_never_counts() {
    // `bins rest = default` does NOT contribute (§19.5.1). Only `a` counts. x=10 hits
    // neither a counting bin ⇒ 0; x=2 covers a ⇒ 100.
    let (miss, _c) = run("module t;\n\
         reg [3:0] x;\n\
         covergroup cg; coverpoint x { bins a = {[0:3]}; bins rest = default; } endgroup\n\
         cg c = new;\n\
         initial begin x=10; c.sample(); $display(\"A7 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        miss.contains("A7 0"),
        "default never counts (miss):\n{miss}"
    );
    let (hit, _c) = run("module t;\n\
         reg [3:0] x;\n\
         covergroup cg; coverpoint x { bins a = {[0:3]}; bins rest = default; } endgroup\n\
         cg c = new;\n\
         initial begin x=2; c.sample(); $display(\"A7 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(hit.contains("A7 100"), "default never counts (hit):\n{hit}");
}

#[test]
fn a8_all_hit_100_none_0() {
    // four single-value bins; sample all four ⇒ 100; sample only an out-of-bin value ⇒ 0.
    let (full, _c) = run("module t;\n\
         reg [3:0] x;\n\
         covergroup cg; coverpoint x { bins b0={0}; bins b1={1}; bins b2={2}; bins b3={3}; } endgroup\n\
         cg c = new;\n\
         initial begin x=0;c.sample(); x=1;c.sample(); x=2;c.sample(); x=3;c.sample();\n\
           $display(\"A8 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(full.contains("A8 100"), "all hit:\n{full}");
    let (none, _c) = run("module t;\n\
         reg [3:0] x;\n\
         covergroup cg; coverpoint x { bins b0={0}; bins b1={1}; bins b2={2}; bins b3={3}; } endgroup\n\
         cg c = new;\n\
         initial begin x=15; c.sample(); $display(\"A8 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(none.contains("A8 0"), "none hit:\n{none}");
}

#[test]
fn a9_unsupported_forms_are_loud_not_silent() {
    // wildcard / transition / fixed-array / iff-on-ignore must be LOUD (never silently
    // dropped — the pre-slice-A parser swallowed these). (Per-bin iff on a REGULAR bin
    // is supported in slice B; iff on ignore/illegal stays loud — static subtraction.)
    for body in [
        "wildcard bins w = {4'b1??0};",
        "bins t = (0 => 1 => 2);",
        "bins fa[3] = {[0:7]};",
        "ignore_bins ig = {0} iff (en);",
    ] {
        let src = format!(
            "module t;\n reg [3:0] x; reg en;\n\
             covergroup cg; coverpoint x {{ {body} }} endgroup\n\
             cg c = new;\n\
             initial begin x=0; c.sample(); $display(\"U %0d\", c.get_coverage()); end\n\
             endmodule\n"
        );
        let (out, code) = run(&src);
        assert!(
            out.contains("VITA-E") || code == Some(1),
            "unsupported bin form must be loud: `{body}`\n{out} {code:?}"
        );
    }
}

// ─────────────── slice B: iff guards (coverpoint-level + per-bin) ───────────────

#[test]
fn b1_coverpoint_iff_gates_whole_sample() {
    // `coverpoint x iff(en)` — when en=0 the sample is DROPPED (no bin credited);
    // when en=1 it samples normally. Two bins a={0}, b={1}.
    let (gated, _c) = run("module t;\n\
         reg [3:0] x; reg en;\n\
         covergroup cg; coverpoint x iff (en) { bins a = {0}; bins b = {1}; } endgroup\n\
         cg c = new;\n\
         initial begin en=0; x=0; c.sample(); $display(\"B1 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(gated.contains("B1 0"), "iff=0 drops sample:\n{gated}");
    let (open, _c) = run("module t;\n\
         reg [3:0] x; reg en;\n\
         covergroup cg; coverpoint x iff (en) { bins a = {0}; bins b = {1}; } endgroup\n\
         cg c = new;\n\
         initial begin en=1; x=0; c.sample(); $display(\"B1 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(open.contains("B1 50"), "iff=1 samples (1/2):\n{open}");
}

#[test]
fn b2_per_bin_iff() {
    // per-bin `bins a={0} iff(g)` — bin a counts only when g is true; bin b unguarded.
    let (off, _c) = run("module t;\n\
         reg [3:0] x; reg g;\n\
         covergroup cg; coverpoint x { bins a = {0} iff (g); bins b = {1}; } endgroup\n\
         cg c = new;\n\
         initial begin g=0; x=0; c.sample(); $display(\"B2 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        off.contains("B2 0"),
        "per-bin iff=0 not credited (0/2):\n{off}"
    );
    let (on, _c) = run("module t;\n\
         reg [3:0] x; reg g;\n\
         covergroup cg; coverpoint x { bins a = {0} iff (g); bins b = {1}; } endgroup\n\
         cg c = new;\n\
         initial begin g=1; x=0; c.sample(); $display(\"B2 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(on.contains("B2 50"), "per-bin iff=1 credited (1/2):\n{on}");
}

#[test]
fn b3_auto_bins_iff() {
    // coverpoint-level iff on an AUTO-bin coverpoint (no explicit body). 2-bit ⇒ 4 bins.
    // en=0 ⇒ sample dropped ⇒ 0; en=1, x=1 ⇒ 1/4 = 25.
    let (off, _c) = run("module t;\n\
         reg [1:0] x; reg en;\n\
         covergroup cg; coverpoint x iff (en); endgroup\n\
         cg c = new;\n\
         initial begin en=0; x=1; c.sample(); $display(\"B3 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(off.contains("B3 0"), "auto iff=0 drops:\n{off}");
    let (on, _c) = run("module t;\n\
         reg [1:0] x; reg en;\n\
         covergroup cg; coverpoint x iff (en); endgroup\n\
         cg c = new;\n\
         initial begin en=1; x=1; c.sample(); $display(\"B3 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(on.contains("B3 25"), "auto iff=1 samples (1/4):\n{on}");
}

#[test]
fn b4_iff_evaluated_at_sample_time() {
    // The guard is read AT each sample(): first sample en=0 (dropped), then en=1 (kept).
    let (out, _c) = run("module t;\n\
         reg [3:0] x; reg en;\n\
         covergroup cg; coverpoint x iff (en) { bins a = {5}; bins b = {6}; } endgroup\n\
         cg c = new;\n\
         initial begin en=0; x=5; c.sample(); en=1; x=6; c.sample();\n\
           $display(\"B4 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    // first sample dropped (en=0), second credits b ⇒ 1/2 = 50.
    assert!(out.contains("B4 50"), "iff sampled per-call:\n{out}");
}

#[test]
fn b5_coverpoint_iff_gates_illegal_error() {
    // A coverpoint `iff` gates the WHOLE sample including the illegal_bins $error:
    // sampling an illegal value while the guard is FALSE must NOT fire $error.
    let (gated, _c) = run("module t;\n\
         reg [3:0] x; reg en;\n\
         covergroup cg; coverpoint x iff (en) { bins a = {[0:3]}; illegal_bins bad = {9}; } endgroup\n\
         cg c = new;\n\
         initial begin en=0; x=9; c.sample(); $display(\"B5 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(gated.contains("B5 0"), "iff=0 gates sample:\n{gated}");
    assert!(
        !gated.contains("illegal coverage bin"),
        "iff=0 must gate the illegal $error too:\n{gated}"
    );
    // guard TRUE: the illegal hit fires $error.
    let (open, code) = run("module t;\n\
         reg [3:0] x; reg en;\n\
         covergroup cg; coverpoint x iff (en) { bins a = {[0:3]}; illegal_bins bad = {9}; } endgroup\n\
         cg c = new;\n\
         initial begin en=1; x=9; c.sample(); $display(\"B5 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        open.contains("illegal coverage bin") || code == Some(1),
        "iff=1 illegal hit must fire $error:\n{open} {code:?}"
    );
}

#[test]
fn a11_zero_counting_bins_never_falls_back_to_auto() {
    // Adversarial regression: an EXPLICIT body that resolves to ZERO counting bins
    // (all values ignored, reversed range, empty set, only ignore_bins) must report
    // honest coverage — NOT silently revert to the auto-bin `1<<(v&63)` path (which
    // produced 100% / impossible 200%+). The dispatch keys on "had a body", not on
    // "resolved bins is empty".
    let cases = [
        // all values ignored ⇒ 0 counting bins ⇒ 0%
        (
            "coverpoint x { bins a = {[0:15]}; ignore_bins ig = {[0:15]}; }",
            "0",
        ),
        // reversed range ⇒ empty ⇒ 0%
        ("coverpoint x { bins a = {[5:1]}; }", "0"),
        // empty value set ⇒ 0%
        ("coverpoint x { bins a = {}; }", "0"),
        // only ignore_bins, no regular ⇒ 0 counting bins ⇒ 0%
        ("coverpoint x { ignore_bins ig = {5}; }", "0"),
        // empty array after ignore ⇒ 0%
        (
            "coverpoint x { bins r[] = {[2:3]}; ignore_bins ig = {[2:3]}; }",
            "0",
        ),
    ];
    for (cp, want) in cases {
        let src = format!(
            "module t;\n reg [3:0] x;\n\
             covergroup cg; {cp} endgroup\n\
             cg c = new;\n\
             initial begin x=2; c.sample(); x=3; c.sample();\n\
               $display(\"ZB %0d\", c.get_coverage()); end\n\
             endmodule\n"
        );
        let (out, _c) = run(&src);
        assert!(
            out.contains(&format!("ZB {want}")),
            "zero-counting-bin coverpoint must report {want}%, not auto-bin fallback: `{cp}`\n{out}"
        );
    }
}

#[test]
fn a12_multi_cp_zero_bins_no_impossible_percent() {
    // Multi-coverpoint: one cp resolves to ZERO counting bins, the other has 1. The
    // result must be sum(covered)/sum(counting) = 1/1 = 100, NOT an impossible 200%.
    let (out, _c) = run("module t;\n\
         reg [3:0] a; reg [1:0] b;\n\
         covergroup cg;\n\
           cpa: coverpoint a { bins lo = {[0:1]}; ignore_bins ig = {[0:1]}; }\n\
           cpb: coverpoint b { bins z = {2}; }\n\
         endgroup\n\
         cg c = new;\n\
         initial begin a=0; b=2; c.sample();\n\
           $display(\"MC %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        out.contains("MC 100"),
        "multi-cp with a zero-bin cp must be 100, not 200:\n{out}"
    );
}

// ─────────────── slice F: sampling-event auto-sample ───────────────

#[test]
fn f1_auto_sample_on_clock() {
    // `covergroup cg @(posedge clk);` auto-samples on each posedge — NO explicit
    // sample() call. 3 posedges at x=0,1,2 ⇒ 3 of 4 auto-bins ⇒ 75%.
    let (out, _c) = run("module t;\n\
         reg clk; reg [1:0] x;\n\
         covergroup cg @(posedge clk); coverpoint x; endgroup\n\
         cg c = new;\n\
         initial begin\n\
           clk=0; x=0;\n\
           #1 clk=1; #1 clk=0;\n\
           x=1; #1 clk=1; #1 clk=0;\n\
           x=2; #1 clk=1; #1 clk=0;\n\
           $display(\"F1 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(out.contains("F1 75"), "auto-sample on clock:\n{out}");
}

#[test]
fn f2_auto_sample_explicit_bins() {
    // clocked covergroup with EXPLICIT bins — auto-samples each posedge into the bins.
    let (out, _c) = run("module t;\n\
         reg clk; reg [3:0] x;\n\
         covergroup cg @(posedge clk); coverpoint x { bins a = {0}; bins b = {1}; } endgroup\n\
         cg c = new;\n\
         initial begin\n\
           clk=0; x=0; #1 clk=1; #1 clk=0;\n\
           x=1; #1 clk=1; #1 clk=0;\n\
           $display(\"F2 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        out.contains("F2 100"),
        "clocked explicit-bin auto-sample:\n{out}"
    );
}

#[test]
fn f3_auto_and_explicit_sample_coexist() {
    // auto-sample (on negedge) AND an explicit sample() both update the same bitmap.
    // The `#1` after the negedge lets the auto-sample observe x=0 BEFORE the initial
    // block advances x to 3 (without it, the same-timestep negedge process sees the
    // already-updated x=3 — a real race). auto x=0 + explicit x=3 ⇒ 2 of 4 ⇒ 50%.
    let (out, _c) = run("module t;\n\
         reg clk; reg [1:0] x;\n\
         covergroup cg @(negedge clk); coverpoint x; endgroup\n\
         cg c = new;\n\
         initial begin\n\
           clk=1; x=0; #1 clk=0; #1;\n\
           x=3; c.sample();\n\
           $display(\"F3 %0d\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(out.contains("F3 50"), "auto+explicit coexist:\n{out}");
}

// ─────────────── slice E: real-% return + per-coverpoint weighted average ───────────────

#[test]
fn e1_get_coverage_returns_real() {
    // get_coverage() is a real (§19.5): a fractional % shows exactly via %g/%f.
    let (out, _c) = run("module t;\n\
         reg [2:0] a;\n\
         covergroup cg; coverpoint a; endgroup\n\
         cg c = new;\n\
         initial begin a=0;c.sample(); a=1;c.sample(); a=2;c.sample();\n\
           $display(\"E1g %g\", c.get_coverage());\n\
           $display(\"E1f %f\", c.get_coverage()); end\n\
         endmodule\n");
    // 3 of 8 bins ⇒ 37.5%.
    assert!(out.contains("E1g 37.5"), "real %g:\n{out}");
    assert!(out.contains("E1f 37.50000"), "real %f:\n{out}");
}

#[test]
fn e2_per_coverpoint_average_not_pooled() {
    // §19.5: a covergroup's coverage is the AVERAGE of its coverpoints' coverages,
    // NOT the pooled sum(covered)/sum(total). cpa: 1/1=100%, cpb: 1/4=25% ⇒
    // average (100+25)/2 = 62.5% (pooled would be 2/5 = 40%).
    let (out, _c) = run("module t;\n\
         reg [3:0] a; reg [1:0] b;\n\
         covergroup cg; cpa: coverpoint a { bins z = {0}; } cpb: coverpoint b; endgroup\n\
         cg c = new;\n\
         initial begin a=0; b=0; c.sample();\n\
           $display(\"E2 %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(out.contains("E2 62.5"), "per-coverpoint average:\n{out}");
}

#[test]
fn e3_zero_bin_coverpoint_excluded_from_average() {
    // a coverpoint with 0 counting bins (all ignored) is EXCLUDED from the average,
    // not counted as 0%. cpa 0 bins (dropped), cpb 1/1=100% ⇒ average over {cpb} = 100.
    let (out, _c) = run("module t;\n\
         reg [3:0] a; reg [1:0] b;\n\
         covergroup cg;\n\
           cpa: coverpoint a { bins lo = {[0:1]}; ignore_bins ig = {[0:1]}; }\n\
           cpb: coverpoint b { bins z = {2}; }\n\
         endgroup\n\
         cg c = new;\n\
         initial begin a=0; b=2; c.sample(); $display(\"E3 %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        out.contains("E3 100"),
        "zero-bin cp excluded from average:\n{out}"
    );
}

// ─────────────── slice C: cross coverage ───────────────

#[test]
fn c1_basic_cross() {
    // cpa{0,1} × cpb{0,1} = 4 cross-bins. Sample (0,0) and (1,1) ⇒ cross 2/4=50.
    // cpa both hit ⇒ 100, cpb both hit ⇒ 100. avg (100+100+50)/3 = 83.333.
    let (out, _c) = run("module t;\n\
         reg [3:0] a, b;\n\
         covergroup cg;\n\
           cpa: coverpoint a { bins a0={0}; bins a1={1}; }\n\
           cpb: coverpoint b { bins b0={0}; bins b1={1}; }\n\
           ab: cross cpa, cpb;\n\
         endgroup\n\
         cg c = new;\n\
         initial begin a=0;b=0;c.sample(); a=1;b=1;c.sample();\n\
           $display(\"C1 %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(out.contains("C1 83.333"), "basic cross:\n{out}");
}

#[test]
fn c2_cross_full_and_partial() {
    // 2×2 cross; sampling all 4 combinations ⇒ cross 100. Each cp also 100. avg 100.
    let (full, _c) = run("module t;\n\
         reg [3:0] a, b;\n\
         covergroup cg;\n\
           cpa: coverpoint a { bins a0={0}; bins a1={1}; }\n\
           cpb: coverpoint b { bins b0={0}; bins b1={1}; }\n\
           ab: cross cpa, cpb;\n\
         endgroup\n\
         cg c = new;\n\
         initial begin\n\
           a=0;b=0;c.sample(); a=0;b=1;c.sample();\n\
           a=1;b=0;c.sample(); a=1;b=1;c.sample();\n\
           $display(\"C2 %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(full.contains("C2 100"), "full cross:\n{full}");
    // sample only (0,0) ⇒ cross 1/4=25; cpa 1/2=50; cpb 1/2=50. avg (50+50+25)/3 = 41.667.
    let (part, _c) = run("module t;\n\
         reg [3:0] a, b;\n\
         covergroup cg;\n\
           cpa: coverpoint a { bins a0={0}; bins a1={1}; }\n\
           cpb: coverpoint b { bins b0={0}; bins b1={1}; }\n\
           ab: cross cpa, cpb;\n\
         endgroup\n\
         cg c = new;\n\
         initial begin a=0;b=0;c.sample(); $display(\"C2 %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(part.contains("C2 41.66"), "partial cross:\n{part}");
}

#[test]
fn c3_cross_unsupported_is_loud() {
    // cross of unknown cp / >64 product / binsof body must be LOUD (never silent).
    let probes = [
        // unknown coverpoint
        "module t; reg [3:0] a;\n\
         covergroup cg; cpa: coverpoint a { bins z={0}; } x: cross cpa, nope; endgroup\n\
         cg c=new; initial begin a=0;c.sample(); $display(\"%g\",c.get_coverage()); end endmodule",
        // 16×16 = 256 > 64
        "module t; reg [3:0] a,b;\n\
         covergroup cg; cpa: coverpoint a; cpb: coverpoint b; x: cross cpa,cpb; endgroup\n\
         cg c=new; initial begin a=0;b=0;c.sample(); $display(\"%g\",c.get_coverage()); end endmodule",
        // binsof select body
        "module t; reg [3:0] a,b;\n\
         covergroup cg; cpa: coverpoint a {bins z={0};} cpb: coverpoint b {bins z={0};}\n\
           x: cross cpa,cpb { bins q = binsof(cpa); } endgroup\n\
         cg c=new; initial begin a=0;b=0;c.sample(); $display(\"%g\",c.get_coverage()); end endmodule",
    ];
    for src in probes {
        let (out, code) = run(src);
        assert!(
            out.contains("VITA-E") || code == Some(1),
            "cross unsupported form must be loud:\n{out} {code:?}"
        );
    }
}

// ─────────────── slice D: option.at_least + option.weight ───────────────

#[test]
fn d1_at_least_counter() {
    // option.at_least=2: a bin is covered only after 2 hits. bins a={5}, b={6}.
    // x=5 once ⇒ counter 1<2 ⇒ a NOT covered ⇒ 0/2=0. x=5 twice ⇒ a covered ⇒ 1/2=50.
    let (once, _c) = run("module t;\n\
         reg [3:0] x;\n\
         covergroup cg; coverpoint x { option.at_least = 2; bins a={5}; bins b={6}; } endgroup\n\
         cg c = new;\n\
         initial begin x=5; c.sample(); $display(\"D1 %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        once.contains("D1 0"),
        "at_least: 1 hit not covered:\n{once}"
    );
    let (twice, _c) = run("module t;\n\
         reg [3:0] x;\n\
         covergroup cg; coverpoint x { option.at_least = 2; bins a={5}; bins b={6}; } endgroup\n\
         cg c = new;\n\
         initial begin x=5; c.sample(); x=5; c.sample(); $display(\"D1 %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        twice.contains("D1 50"),
        "at_least: 2 hits covered:\n{twice}"
    );
}

#[test]
fn d2_covergroup_level_at_least() {
    // covergroup-level option.at_least applies to all coverpoints; a coverpoint-level
    // option overrides. cg default 2; x=7 sampled 3× ⇒ covered (>=2) ⇒ 1/1=100.
    let (out, _c) = run("module t;\n\
         reg [3:0] x;\n\
         covergroup cg; option.at_least = 2; coverpoint x { bins a={7}; } endgroup\n\
         cg c = new;\n\
         initial begin x=7;c.sample(); x=7;c.sample(); x=7;c.sample();\n\
           $display(\"D2 %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(out.contains("D2 100"), "covergroup-level at_least:\n{out}");
}

#[test]
fn d3_weight_weighted_average() {
    // option.weight: cpa weight 3 (100%), cpb weight 1 (0%) ⇒ (3*100 + 1*0)/(3+1) = 75.
    let (out, _c) = run("module t;\n\
         reg [3:0] a, b;\n\
         covergroup cg;\n\
           cpa: coverpoint a { option.weight = 3; bins z={0}; }\n\
           cpb: coverpoint b { option.weight = 1; bins z={5}; }\n\
         endgroup\n\
         cg c = new;\n\
         initial begin a=0; b=0; c.sample(); $display(\"D3 %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(out.contains("D3 75"), "weighted average:\n{out}");
}

#[test]
fn d4_at_least_on_auto_bins_is_loud() {
    // option.at_least>1 on an auto-bin coverpoint (no explicit bins) is loud — the
    // bitmap model has no per-value counters.
    let (out, code) = run("module t;\n\
         reg [1:0] x;\n\
         covergroup cg; option.at_least = 2; coverpoint x; endgroup\n\
         cg c = new;\n\
         initial begin x=0; c.sample(); $display(\"%g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        out.contains("VITA-E") || code == Some(1),
        "at_least>1 on auto-bins must be loud:\n{out} {code:?}"
    );
}

#[test]
fn d5_other_options_accepted_ignored() {
    // option.goal / option.comment / type_option.* don't change the measured % —
    // accepted and ignored (not loud, not silently wrong). bins z={0}, a=0 ⇒ 100.
    let (out, _c) = run("module t;\n\
         reg [3:0] a;\n\
         covergroup cg;\n\
           option.goal = 90; option.comment = \"hi\"; option.per_instance = 1;\n\
           coverpoint a { option.goal = 100; bins z={0}; }\n\
         endgroup\n\
         cg c = new;\n\
         initial begin a=0; c.sample(); $display(\"D5 %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(out.contains("D5 100"), "other options ignored:\n{out}");
}

#[test]
fn d6_at_least_array_and_iff_interactions() {
    // at_least with array bins: each element has its OWN counter. arr[]={[0:1]},
    // at_least=2: x=0 twice (arr[0] covered), x=1 once (arr[1] not) ⇒ 1/2=50.
    let (arr, _c) = run("module t;\n\
         reg [3:0] x;\n\
         covergroup cg; coverpoint x { option.at_least=2; bins arr[]={[0:1]}; } endgroup\n\
         cg c = new;\n\
         initial begin x=0;c.sample();c.sample(); x=1;c.sample();\n\
           $display(\"D6a %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        arr.contains("D6a 50"),
        "at_least array per-element counters:\n{arr}"
    );
    // at_least + per-bin iff: the counter only increments when the guard holds.
    // guard=0 for 3 samples (no increment), guard=1 for 2 samples ⇒ covered ⇒ 100.
    let (iff, _c) = run("module t;\n\
         reg [3:0] x; reg g;\n\
         covergroup cg; coverpoint x { bins a={5} iff(g); option.at_least=2; } endgroup\n\
         cg c = new;\n\
         initial begin g=0; x=5; c.sample(); c.sample(); c.sample();\n\
           g=1; c.sample(); c.sample(); $display(\"D6b %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        iff.contains("D6b 100"),
        "at_least + iff counter gating:\n{iff}"
    );
}

// ── Slice G: precise expr-coverpoint width ────────────────────────────────────
// A coverpoint on an EXPRESSION (not a bare net) must derive its auto-bin count
// from the expression's self-determined width, not a hardcoded default. Before G,
// any non-ident expr defaulted to w=6 ⇒ 64 bins, so small-domain exprs reported a
// wildly diluted %. iverilog rejects covergroup ⇒ HAND-IEEE (§19.5): every % is
// hand-computed from the precise width.

#[test]
fn g1_binop_width_xor_two_bit() {
    // a,b are 2-bit ⇒ (a^b) is 2-bit ⇒ 4 auto-bins. 2 distinct values ⇒ 2/4 = 50%.
    // (Pre-G: w=6 default ⇒ 64 bins ⇒ 2/64 ≈ 3%.)
    let (out, _c) = run("module t;\n\
         reg [1:0] a, b;\n\
         covergroup cg; coverpoint (a^b); endgroup\n\
         cg c = new;\n\
         initial begin a=0;b=1;c.sample(); a=2;b=2;c.sample();\n\
           $display(\"G1 %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        out.contains("G1 50"),
        "xor 2-bit domain ⇒ 4 bins ⇒ 50%:\n{out}"
    );
}

#[test]
fn g2_partselect_width() {
    // x[2:0] is 3-bit ⇒ 8 auto-bins. values {0,5} ⇒ 2 distinct ⇒ 2/8 = 25%.
    let (out, _c) = run("module t;\n\
         reg [7:0] x;\n\
         covergroup cg; coverpoint x[2:0]; endgroup\n\
         cg c = new;\n\
         initial begin x=8'h00;c.sample(); x=8'h05;c.sample();\n\
           $display(\"G2 %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        out.contains("G2 25"),
        "x[2:0] ⇒ 3-bit ⇒ 8 bins ⇒ 25%:\n{out}"
    );
}

#[test]
fn g3_concat_width_sum() {
    // {a,b} with a,b 1-bit each ⇒ 2-bit concat ⇒ 4 auto-bins. values {2'b01,2'b10}
    // ⇒ 2 distinct ⇒ 2/4 = 50%.
    let (out, _c) = run("module t;\n\
         reg a, b;\n\
         covergroup cg; coverpoint {a,b}; endgroup\n\
         cg c = new;\n\
         initial begin a=0;b=1;c.sample(); a=1;b=0;c.sample();\n\
           $display(\"G3 %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        out.contains("G3 50"),
        "{{a,b}} ⇒ 2-bit ⇒ 4 bins ⇒ 50%:\n{out}"
    );
}

#[test]
fn g4_comparison_is_one_bit() {
    // (a > b) is a 1-bit (relational) domain ⇒ 2 auto-bins. one true + one false
    // ⇒ 2/2 = 100%.
    let (out, _c) = run("module t;\n\
         reg [3:0] a, b;\n\
         covergroup cg; coverpoint (a > b); endgroup\n\
         cg c = new;\n\
         initial begin a=5;b=2;c.sample(); a=1;b=9;c.sample();\n\
           $display(\"G4 %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        out.contains("G4 100"),
        "(a>b) ⇒ 1-bit ⇒ 2 bins ⇒ 100%:\n{out}"
    );
}

// Adversarial-found CARDINAL bug (workflow wnvzdxh6d): the sampled value was
// masked with a hardcoded `& 63` but binned against the now-precise num_bins, so a
// context-widened value (shift/add/mul/signed) could set a bit beyond num_bins ⇒
// IMPOSSIBLE >100% coverage. Fix: bin index = value & (num_bins-1) — truncates to
// the self-determined low bits AND caps the index < num_bins.

#[test]
fn g5_shift_truncates_to_self_width() {
    // (a<<b): W = width(lhs) = width(a) = 2 ⇒ 4 bins, mask=3. Six samples
    // {1,2,4,8,16,32} truncate to {1,2,0,0,0,0} ⇒ 3 distinct bins ⇒ 3/4 = 75%.
    // (Pre-fix: bit 4/8/16/32 set ⇒ $countones=6 ⇒ 6/4 = 150%, impossible.)
    let (out, _c) = run("module t;\n\
         reg [1:0] a; reg [2:0] b;\n\
         covergroup cg; coverpoint (a << b); endgroup\n\
         cg c = new;\n\
         initial begin a=1;b=0;c.sample(); a=1;b=1;c.sample(); a=1;b=2;c.sample();\n\
           a=1;b=3;c.sample(); a=1;b=4;c.sample(); a=1;b=5;c.sample();\n\
           $display(\"G5 %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        out.contains("G5 75"),
        "shift truncates to W=2 ⇒ 75% (not 150%):\n{out}"
    );
}

#[test]
fn g6_add_overflow_truncated() {
    // (a+b), a,b 2-bit ⇒ W=2, 4 bins. {0, 3+1=4→0, 2+2=4→0} ⇒ 1 distinct ⇒ 25%.
    let (out, _c) = run("module t;\n\
         reg [1:0] a, b;\n\
         covergroup cg; coverpoint (a+b); endgroup\n\
         cg c = new;\n\
         initial begin a=0;b=0;c.sample(); a=3;b=1;c.sample(); a=2;b=2;c.sample();\n\
           $display(\"G6 %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        out.contains("G6 25"),
        "add overflow truncates ⇒ 25%:\n{out}"
    );
}

#[test]
fn g7_signed_add_never_exceeds_100() {
    // signed (a+b), a,b [2:0] ⇒ W=3, 8 bins. {-2→6, 2, -6→2} ⇒ {6,2} = 2 distinct
    // ⇒ 25%. Pre-fix the negatives sign-extended to bits 62/58 ⇒ 37.5%.
    let (out, _c) = run("module t;\n\
         reg signed [2:0] a, b;\n\
         covergroup cg; coverpoint (a+b); endgroup\n\
         cg c = new;\n\
         initial begin a=-1;b=-1;c.sample(); a=1;b=1;c.sample(); a=-3;b=-3;c.sample();\n\
           $display(\"G7 %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        out.contains("G7 25"),
        "signed add ⇒ low-3-bit bins ⇒ 25%:\n{out}"
    );
}

#[test]
fn g8_function_call_return_width() {
    // coverpoint on a 2-bit function call ⇒ 4 bins (from the declared return type),
    // all 4 values sampled ⇒ 100% (pre-fix: DEFAULT w=6 ⇒ 64 bins ⇒ 6.25%).
    let (out, _c) = run("module t;\n\
         reg [1:0] a;\n\
         function [1:0] f(input [1:0] z); f = z; endfunction\n\
         covergroup cg; coverpoint f(a); endgroup\n\
         cg c = new;\n\
         initial begin a=0;c.sample(); a=1;c.sample(); a=2;c.sample(); a=3;c.sample();\n\
           $display(\"G8 %g\", c.get_coverage()); end\n\
         endmodule\n");
    assert!(
        out.contains("G8 100"),
        "2-bit function-call coverpoint ⇒ 4 bins ⇒ 100%:\n{out}"
    );
}
