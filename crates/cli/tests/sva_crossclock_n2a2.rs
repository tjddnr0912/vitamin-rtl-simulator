//! Slice N2a-2 — cross-clock SVA sequence GENERALIZATION: N-clock chains (3+
//! segments) and the non-overlap `|=>` cross-clock consequent. N2a-1 handled the
//! two-segment overlap form `@(c1) a ##1 @(c2) b |-> c`; this generalizes the
//! handoff to a CHAIN (`@(c1) a ##1 @(c2) b ##1 @(c3) c |-> d`) and adds `|=>`
//! (the consequent on the NEXT edge of the final clock — one more handoff stage).
//! Pure IR-0 (sim-ir frozen / format_version 8); the N=2 overlap case is
//! byte-identical (the sva_crossclock.rs suite is the guard).
//!
//! iverilog 13.0 rejects all concurrent assertions (NULL oracle) → hand-IEEE,
//! value-pinned. Clocks (distinct edges):
//!   c1 posedge @5,15,25,35,45    (always #5 c1=~c1)
//!   c2 posedge @8,18,28,38,48    (initial #3 forever #5 c2=~c2)
//!   c3 posedge @11,21,31,41      (initial #6 forever #5 c3=~c3)
//!
//! LOUD (still out of subset): a multi-term / non-re-clocked segment, `##n` (n≠1),
//! an OR-of-clocks edge.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svaxc2_{}_{n}", std::process::id()));
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

fn fires(out: &str, err: &str) -> usize {
    format!("{err}{out}")
        .matches("Assertion property violation")
        .count()
}

// Three distinct clocks: c1 @5,15,25,…; c2 @8,18,28,…; c3 @11,21,31,….
const CLK3: &str = "reg c1=0, c2=0, c3=0;\n\
     always #5 c1=~c1;\n\
     initial begin #3 forever #5 c2=~c2; end\n\
     initial begin #6 forever #5 c3=~c3; end\n";

// Two clocks (c1,c2) for the |=> tests.
const CLK2: &str = "reg c1=0, c2=0;\n\
     always #5 c1=~c1;\n\
     initial begin #3 forever #5 c2=~c2; end\n";

// ───────────────────────── N-clock chains (3 segments) ─────────────────────────

#[test]
fn crossclock3_holds() {
    // a@c1 ##1 b@c2 ##1 c@c3 |-> d, all 1 → d held on every completing c3 → clean.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK3}reg a=1, b=1, cc=1, d=1;\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b ##1 @(posedge c3) cc |-> d; endproperty\n\
         initial assert property(p);\n\
         initial #45 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(0),
        "3-clock chain holds. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !err.contains("VITA-E"),
        "must synthesize, not reject:\n{err}"
    );
}

#[test]
fn crossclock3_fires() {
    // d=0 → every completed 3-chain checks d on c3 and fires.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK3}reg a=1, b=1, cc=1, d=0;\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b ##1 @(posedge c3) cc |-> d; endproperty\n\
         initial assert property(p);\n\
         initial #45 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(1),
        "3-clock chain with d=0 fires. stderr:\n{err}\nout:\n{out}"
    );
    assert!(fires(&out, &err) >= 1, "at least one fire:\n{err}\n{out}");
}

#[test]
fn crossclock3_consequent_sampled_on_c3() {
    // SKEW PROOF: d=0 only across the c3 edge @21 (window [20,22]) — covers @21 but
    // not c1 @25 or c2 @18 or c3 @11/@31. Fires ONLY if d is sampled on c3 (the final
    // clock) AND the 3-chain completes there. Exactly one violation.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK3}reg a=1, b=1, cc=1, d=1;\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b ##1 @(posedge c3) cc |-> d; endproperty\n\
         initial assert property(p);\n\
         initial begin #20 d=0; #2 d=1; #45 $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(1),
        "d sampled on c3@21 → fire. stderr:\n{err}\nout:\n{out}"
    );
    assert_eq!(
        fires(&out, &err),
        1,
        "exactly ONE violation (only c3@21 sees d=0):\n{err}\n{out}"
    );
}

#[test]
fn crossclock3_broken_chain_no_fire() {
    // b=0 → the middle segment never matches → hf1 never arms → the chain never
    // completes → no fire even with d=0.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK3}reg a=1, b=0, cc=1, d=0;\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b ##1 @(posedge c3) cc |-> d; endproperty\n\
         initial assert property(p);\n\
         initial #45 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(0),
        "broken chain (b=0) never completes → clean. stderr:\n{err}\nout:\n{out}"
    );
    assert_eq!(fires(&out, &err), 0, "no fire:\n{err}\n{out}");
}

// ─────────────────────── |=> cross-clock consequent ───────────────────────────

#[test]
fn crossclock_nonoverlap_holds() {
    // a@c1 ##1 b@c2 |=> c — c checked on the NEXT c2 after b; c=1 → clean.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK2}reg a=1, b=1, c=1;\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b |=> c; endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(0),
        "|=> cross-clock holds. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !err.contains("VITA-E"),
        "must synthesize (|=> no longer loud):\n{err}"
    );
}

#[test]
fn crossclock_nonoverlap_fires() {
    // c=0 → the consequent fails on the c2 edge after each completion.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK2}reg a=1, b=1, c=0;\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b |=> c; endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(1),
        "|=> with c=0 fires. stderr:\n{err}\nout:\n{out}"
    );
    assert!(fires(&out, &err) >= 1, "at least one fire:\n{err}\n{out}");
}

#[test]
fn crossclock_nonoverlap_skew_proof() {
    // SKEW PROOF: antecedent completes at c2 @8 (b@8), and `|=>` checks c on the NEXT
    // c2 @18. c=0 only across @18 (window [17,19]) → fire at @18, NOT @8. Proves the
    // one-final-clock delay (an overlap `|->` would check c @8 and see c=1 → no fire).
    let (out, err, code) = run(&format!(
        "module top;\n{CLK2}reg a=1, b=1, c=1;\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b |=> c; endproperty\n\
         initial assert property(p);\n\
         initial begin #17 c=0; #2 c=1; #40 $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(1),
        "|=> checks c on the NEXT c2 (@18, c=0) → fire. stderr:\n{err}\nout:\n{out}"
    );
    assert_eq!(
        fires(&out, &err),
        1,
        "exactly ONE violation (only c2@18 sees c=0):\n{err}\n{out}"
    );
}

#[test]
fn crossclock3_nonoverlap_combo() {
    // N-clock chain AND |=> together: a@c1 ##1 b@c2 ##1 cc@c3 |=> d — d checked on the
    // NEXT c3 after the 3-chain completes. d=0 → fires.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK3}reg a=1, b=1, cc=1, d=0;\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b ##1 @(posedge c3) cc |=> d; endproperty\n\
         initial assert property(p);\n\
         initial #50 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(1),
        "3-clock + |=> fires. stderr:\n{err}\nout:\n{out}"
    );
    assert!(fires(&out, &err) >= 1, "at least one fire:\n{err}\n{out}");

    // And holds when d=1.
    let (o2, e2, c2) = run(&format!(
        "module top;\n{CLK3}reg a=1, b=1, cc=1, d=1;\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b ##1 @(posedge c3) cc |=> d; endproperty\n\
         initial assert property(p);\n\
         initial #50 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        c2,
        Some(0),
        "3-clock + |=> holds with d=1. stderr:\n{e2}\nout:\n{o2}"
    );
}

// ─────────────────── A.2 cross-clock MULTI-TERM segments ───────────────────────
// A segment may now be a parenthesized MULTI-TERM sequence `@(ck)(x ##1 y)`. Each
// segment expands to its OWN shift-register pipeline clocked on that segment's clock.

#[test]
fn xc_multiterm_both_segments_holds() {
    // `@(c1)(a ##1 b) ##1 @(c2)(d ##1 e) |-> f`. Both segments multi-term. f=1 at every
    // completion → 0 fires. (Proves the multi-term lane synthesizes, not loud.)
    let (out, err, code) = run(&format!(
        "module top;\n{CLK2}reg a=1, b=1, d=1, e=1, f=1;\n\
         property p; @(posedge c1)(a ##1 b) ##1 @(posedge c2)(d ##1 e) |-> f; endproperty\n\
         initial assert property(p);\n\
         initial #50 $finish;\n\
         endmodule\n"
    ));
    assert!(
        !err.contains("VITA-E"),
        "multi-term segments must synthesize, not loud:\n{err}"
    );
    assert_eq!(
        code,
        Some(0),
        "multi-term both-segments holds (f=1). stderr:\n{err}\nout:\n{out}"
    );
    assert_eq!(fires(&out, &err), 0, "no fire:\n{err}\n{out}");
}

#[test]
fn xc_multiterm_both_segments_fires() {
    // Same shape but f=0 at completion → the antecedent completes and the overlap `|->`
    // checks f=0 on the final c2 edge → at least one violation.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK2}reg a=1, b=1, d=1, e=1, f=0;\n\
         property p; @(posedge c1)(a ##1 b) ##1 @(posedge c2)(d ##1 e) |-> f; endproperty\n\
         initial assert property(p);\n\
         initial #50 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(1),
        "multi-term both-segments fires (f=0). stderr:\n{err}\nout:\n{out}"
    );
    assert!(fires(&out, &err) >= 1, "at least one fire:\n{err}\n{out}");
}

#[test]
fn xc_multiterm_seg0_only() {
    // Segment 0 multi-term, segment 1 single-boolean: `@(c1)(a ##1 b) ##1 @(c2) c |-> d`.
    // a=b=c=1 → the chain completes; d=0 → fires (exercises the seg-0 pipeline + a lone
    // c2 boolean segment).
    let (out, err, code) = run(&format!(
        "module top;\n{CLK2}reg a=1, b=1, c=1, d=0;\n\
         property p; @(posedge c1)(a ##1 b) ##1 @(posedge c2) c |-> d; endproperty\n\
         initial assert property(p);\n\
         initial #50 $finish;\n\
         endmodule\n"
    ));
    assert!(
        !err.contains("E3009"),
        "seg-0 multi-term must synthesize, not loud:\n{err}"
    );
    assert_eq!(
        code,
        Some(1),
        "seg-0 multi-term, d=0 → fires. stderr:\n{err}\nout:\n{out}"
    );
    assert!(fires(&out, &err) >= 1, "at least one fire:\n{err}\n{out}");

    // Holds when d=1.
    let (o2, e2, c2) = run(&format!(
        "module top;\n{CLK2}reg a=1, b=1, c=1, d=1;\n\
         property p; @(posedge c1)(a ##1 b) ##1 @(posedge c2) c |-> d; endproperty\n\
         initial assert property(p);\n\
         initial #50 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        c2,
        Some(0),
        "seg-0 multi-term holds (d=1). stderr:\n{e2}\nout:\n{o2}"
    );
}

#[test]
fn xc_multiterm_seg1_only_advances_on_c2() {
    // Segment 1 multi-term: `@(c1) a ##1 @(c2)(c ##1 e) |-> f`. The inner `c ##1 e`
    // pipeline must advance on c2 edges ONLY. a=c=e=1, f=0 → completes & fires.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK2}reg a=1, c=1, e=1, f=0;\n\
         property p; @(posedge c1) a ##1 @(posedge c2)(c ##1 e) |-> f; endproperty\n\
         initial assert property(p);\n\
         initial #50 $finish;\n\
         endmodule\n"
    ));
    assert!(
        !err.contains("E3009"),
        "seg-1 multi-term must synthesize, not loud:\n{err}"
    );
    assert_eq!(
        code,
        Some(1),
        "seg-1 multi-term, f=0 → fires. stderr:\n{err}\nout:\n{out}"
    );

    // c2-ONLY advance proof: c=1 ONLY across the c1 edge @15 (window [14,16]) and 0
    // everywhere else. The inner `c ##1 e` needs c true on one c2 edge — but c is never
    // 1 at ANY c2 edge (c2 @8,18,28,…), so the inner sequence NEVER completes → no fire
    // even with f=0. If the inner shift wrongly clocked on c1 it would capture c@15.
    let (o2, e2, c2) = run(&format!(
        "module top;\n{CLK2}reg a=1, c=0, e=1, f=0;\n\
         property p; @(posedge c1) a ##1 @(posedge c2)(c ##1 e) |-> f; endproperty\n\
         initial assert property(p);\n\
         initial begin #14 c=1; #2 c=0; #40 $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(
        c2,
        Some(0),
        "c only at c1@15 (never a c2 edge) → inner never completes → clean. stderr:\n{e2}\nout:\n{o2}"
    );
    assert_eq!(fires(&o2, &e2), 0, "c2-only advance: no fire:\n{e2}\n{o2}");
}

#[test]
fn xc_multiterm_nonoverlap() {
    // `|=>` with multi-term segments: `@(c1)(a ##1 b) ##1 @(c2)(c ##1 e) |=> f`. f checked
    // on the NEXT c2 edge after the chain completes. f=0 → fires.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK2}reg a=1, b=1, c=1, e=1, f=0;\n\
         property p; @(posedge c1)(a ##1 b) ##1 @(posedge c2)(c ##1 e) |=> f; endproperty\n\
         initial assert property(p);\n\
         initial #55 $finish;\n\
         endmodule\n"
    ));
    assert!(
        !err.contains("E3009"),
        "|=> multi-term must synthesize, not loud:\n{err}"
    );
    assert_eq!(
        code,
        Some(1),
        "|=> multi-term f=0 → fires. stderr:\n{err}\nout:\n{out}"
    );
    assert!(fires(&out, &err) >= 1, "at least one fire:\n{err}\n{out}");

    // Holds when f=1.
    let (o2, e2, c2) = run(&format!(
        "module top;\n{CLK2}reg a=1, b=1, c=1, e=1, f=1;\n\
         property p; @(posedge c1)(a ##1 b) ##1 @(posedge c2)(c ##1 e) |=> f; endproperty\n\
         initial assert property(p);\n\
         initial #55 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        c2,
        Some(0),
        "|=> multi-term holds (f=1). stderr:\n{e2}\nout:\n{o2}"
    );
}

#[test]
fn xc_multiterm_three_clocks() {
    // 3-clock chain with a multi-term FIRST and SECOND segment, lone third:
    // `@(c1)(a ##1 b) ##1 @(c2)(c ##1 d) ##1 @(c3) e |-> g`. g=0 → fires.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK3}reg a=1, b=1, c=1, d=1, e=1, g=0;\n\
         property p; @(posedge c1)(a ##1 b) ##1 @(posedge c2)(c ##1 d) ##1 @(posedge c3) e |-> g; endproperty\n\
         initial assert property(p);\n\
         initial #55 $finish;\n\
         endmodule\n"
    ));
    assert!(
        !err.contains("E3009"),
        "3-clock multi-term must synthesize, not loud:\n{err}"
    );
    assert_eq!(
        code,
        Some(1),
        "3-clock multi-term g=0 → fires. stderr:\n{err}\nout:\n{out}"
    );
    assert!(fires(&out, &err) >= 1, "at least one fire:\n{err}\n{out}");

    // Holds when g=1.
    let (o2, e2, c2) = run(&format!(
        "module top;\n{CLK3}reg a=1, b=1, c=1, d=1, e=1, g=1;\n\
         property p; @(posedge c1)(a ##1 b) ##1 @(posedge c2)(c ##1 d) ##1 @(posedge c3) e |-> g; endproperty\n\
         initial assert property(p);\n\
         initial #55 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        c2,
        Some(0),
        "3-clock multi-term holds (g=1). stderr:\n{e2}\nout:\n{o2}"
    );
}

#[test]
fn xc_multiterm_broken_inner_no_fire() {
    // Inner second term of segment 1 is 0: `@(c1)(a ##1 b) ##1 @(c2)(d ##1 e) |-> f`
    // with e=0 → the inner `d ##1 e` never completes → the chain never completes → no
    // fire even with f=0 (proves the seg-1 pipeline really gates completion).
    let (out, err, code) = run(&format!(
        "module top;\n{CLK2}reg a=1, b=1, d=1, e=0, f=0;\n\
         property p; @(posedge c1)(a ##1 b) ##1 @(posedge c2)(d ##1 e) |-> f; endproperty\n\
         initial assert property(p);\n\
         initial #50 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(0),
        "broken inner (e=0) → chain never completes → clean. stderr:\n{err}\nout:\n{out}"
    );
    assert_eq!(fires(&out, &err), 0, "no fire:\n{err}\n{out}");
}

#[test]
fn xc_multiterm_nested_reclock_is_loud() {
    // A NESTED re-clock inside a segment (`@(c2)(c ##1 @(c3) d)`) is a 4th clock
    // boundary — kept LOUD (the segment pipeline is single-clock).
    let (out, err, code) = run(&format!(
        "module top;\n{CLK3}reg a=1, c=1, d=1, f=1;\n\
         property p; @(posedge c1) a ##1 @(posedge c2)(c ##1 @(posedge c3) d) |-> f; endproperty\n\
         initial assert property(p);\n\
         initial #45 $finish;\n\
         endmodule\n"
    ));
    assert_ne!(
        code,
        Some(0),
        "nested re-clock must loud-reject. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("nested")
            || format!("{err}{out}").to_lowercase().contains("unsupported"),
        "expected a loud nested-reclock diagnostic:\n{err}\n{out}"
    );
}

#[test]
fn xc_multiterm_deterministic() {
    let src = format!(
        "module top;\n{CLK2}reg a=1, b=1, d=1, e=1, f=0;\n\
         property p; @(posedge c1)(a ##1 b) ##1 @(posedge c2)(d ##1 e) |-> f; endproperty\n\
         initial assert property(p);\n\
         initial #50 $finish;\n\
         endmodule\n"
    );
    let (o1, e1, r1) = run(&src);
    let (o2, e2, r2) = run(&src);
    assert_eq!(
        (o1, e1, r1),
        (o2, e2, r2),
        "multi-term output must be deterministic"
    );
}

// ───────────────────────────── determinism + loud ─────────────────────────────

#[test]
fn crossclock3_deterministic() {
    let src = format!(
        "module top;\n{CLK3}reg a=1, b=1, cc=1, d=0;\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b ##1 @(posedge c3) cc |-> d; endproperty\n\
         initial assert property(p);\n\
         initial #45 $finish;\n\
         endmodule\n"
    );
    let (o1, e1, r1) = run(&src);
    let (o2, e2, r2) = run(&src);
    assert_eq!(
        (o1, e1, r1),
        (o2, e2, r2),
        "N2a-2 output must be deterministic"
    );
}

#[test]
fn crossclock_non_reclocked_segment_is_loud() {
    // `a ##1 @(c2) b ##1 e` — the last `##1` operand `e` is NOT re-clocked (a same-clock
    // multi-term segment) → loud-reject (deferred multi-term lane), not a silent accept.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK2}reg a=1, b=1, e=1, d=1;\n\
         property p; @(posedge c1) a ##1 @(posedge c2) b ##1 e |-> d; endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_ne!(
        code,
        Some(0),
        "must loud-reject. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("re-clocked")
            || format!("{err}{out}").to_lowercase().contains("unsupported"),
        "expected a loud multi-term/re-clock diagnostic:\n{err}\n{out}"
    );
}

#[test]
fn crossclock_hashn_gt1_is_loud() {
    // `##2` across distinct clocks is deferred (only `##1` is well-defined §16.13).
    let (out, err, code) = run(&format!(
        "module top;\n{CLK2}reg a=1, b=1, c=1;\n\
         property p; @(posedge c1) a ##2 @(posedge c2) b |-> c; endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_ne!(
        code,
        Some(0),
        "##2 cross-clock must loud-reject. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").contains("##1")
            || format!("{err}{out}").to_lowercase().contains("unsupported"),
        "expected a loud `##1`-only diagnostic:\n{err}\n{out}"
    );
}
