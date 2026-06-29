//! Multi-clock concurrent assertion — canonical two-clock pattern (Phase-3 slice
//! A3). `assert property(@(posedge c1) ante |=> @(posedge c2) cons)` samples the
//! (boolean) antecedent on c1 into a 1-bit handoff register, which a SECOND
//! synthesized process — clocked by c2 — consumes on its next c2 edge to check the
//! (boolean) consequent. Pure IR-0 two-process synthesis (sim-ir frozen,
//! format_version 8); the AST gains a `consequent_clock` field, so the `.vu`
//! AST-hash re-pins once (the `.velab`/SimIr golden is unchanged).
//!
//! iverilog 13.0 rejects this entirely (NULL oracle) → hand-IEEE. TIE SEMANTICS:
//! when c1 and c2 tick the same instant the consume process reads the PRIOR-edge
//! handoff (NBA-region ordering) — a documented hand-IEEE pin, not an IEEE-conformance
//! claim. Everything outside the canonical pattern is LOUD (see the reject tests).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svamc_{}_{n}", std::process::id()));
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

// ── canonical pattern accepted ─────────────────────────────────────────────

#[test]
fn multiclock_handoff_violation_fires() {
    // @(posedge c1) a |=> @(posedge c2) b : a=1 at c1 (t=5) sets handoff; at the next
    // c2 edge (t=7) handoff=1 and b=0 → violation fires.
    let (out, err, code) = run("module top;\n\
         reg c1=0, c2=0, a=1, b=0;\n\
         always #5 c1=~c1;\n\
         always #7 c2=~c2;\n\
         initial assert property(@(posedge c1) a |=> @(posedge c2) b);\n\
         initial begin #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "the c1→c2 handoff violation must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "expected an assertion violation:\n{err}\n{out}"
    );
}

#[test]
fn multiclock_handoff_holds_no_error() {
    // Same, but b=1 at the consume edge → handoff && b holds → clean.
    let (out, err, code) = run("module top;\n\
         reg c1=0, c2=0, a=1, b=1;\n\
         always #5 c1=~c1;\n\
         always #7 c2=~c2;\n\
         initial assert property(@(posedge c1) a |=> @(posedge c2) b);\n\
         initial begin #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "handoff ⇒ b with b=1 holds → clean exit. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\nstderr={err}\nout={out}"
    );
}

#[test]
fn multiclock_no_fire_before_first_c1_edge() {
    // handoff is X-init; a consume (c2) edge BEFORE any c1 edge must NOT fire
    // (`if (X && !b)` is not taken). c2 (period 3) ticks at t=3 before c1 (t=5).
    let (out, err, code) = run("module top;\n\
         reg c1=0, c2=0, a=0, b=0;\n\
         always #5 c1=~c1;\n\
         always #3 c2=~c2;\n\
         initial assert property(@(posedge c1) a |=> @(posedge c2) b);\n\
         initial begin #4 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "no fire before the first c1 edge (X handoff). stderr:\n{err}\nout:\n{out}"
    );
}

#[test]
fn multiclock_runs_deterministically() {
    // Two runs of a multi-clock design must produce byte-identical output.
    let src = "module top;\n\
         reg c1=0, c2=0, a=1, b=0;\n\
         always #5 c1=~c1;\n\
         always #7 c2=~c2;\n\
         initial assert property(@(posedge c1) a |=> @(posedge c2) b);\n\
         initial begin #20 $finish; end\n\
         endmodule\n";
    let (o1, e1, c1) = run(src);
    let (o2, e2, c2) = run(src);
    assert_eq!(
        (o1, e1, c1),
        (o2, e2, c2),
        "multi-clock output must be deterministic"
    );
}

#[test]
fn single_match_fires_once_when_c2_faster_than_c1() {
    // REVIEW (handoff-timing lens, HIGH, 2026-06-16): a SINGLE antecedent match must
    // be consumed at EXACTLY ONE c2 edge. When c2 is faster than c1, a level-held
    // handoff re-fired on every c2 edge in the window. With set-only/discharge it
    // fires once. c1 edges 10,30; c2 edges 3,9,15,21,27; a=1 only around the t=10 c1
    // edge → one obligation, consumed at t=15, NOT re-fired at t=21/t=27.
    let (out, err, code) = run("module top;\n\
         reg c1=0, c2=0, a=0, b=0;\n\
         always #10 c1=~c1;\n\
         always #3 c2=~c2;\n\
         initial assert property(@(posedge c1) a |=> @(posedge c2) b);\n\
         initial begin #9 a=1; #2 a=0; #40 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "the single match must fire. stderr:\n{err}\nout:\n{out}"
    );
    let fires = format!("{err}{out}")
        .matches("Assertion property violation")
        .count();
    assert_eq!(
        fires, 1,
        "a single match must fire EXACTLY once (no stale re-fire on later c2 edges); \
         got {fires}.\nstderr:\n{err}\nout:\n{out}"
    );
}

// ── outside the canonical subset → LOUD ────────────────────────────────────

#[test]
fn overlap_implication_with_consequent_clock_is_loud() {
    // `|->` (overlap) with a consequent clock has no coherent same-tick cross-clock
    // check → loud, not silent acceptance.
    let (_o, err, code) = run("module top;\n\
         reg c1=0, c2=0, a=0, b=0;\n\
         always #5 c1=~c1;\n\
         always #7 c2=~c2;\n\
         initial assert property(@(posedge c1) a |-> @(posedge c2) b);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "|-> with a consequent clock must be loud. {err}"
    );
    assert!(err.contains("VITA-E"), "{err}");
}

#[test]
fn consequent_or_of_clocks_is_loud() {
    // The consequent clock must be a single edge-event; an OR-of-clocks is multi-clock.
    let (_o, err, code) = run("module top;\n\
         reg c1=0, c2=0, a=0, b=0;\n\
         always #5 c1=~c1;\n\
         always #7 c2=~c2;\n\
         initial assert property(@(posedge c1) a |=> @(posedge c2 or posedge c1) b);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "an OR-of-clocks consequent must be loud. {err}"
    );
    assert!(err.contains("VITA-E"), "{err}");
}

#[test]
fn ante_or_of_clocks_still_loud() {
    // The existing single-clock gate on the antecedent clock is unchanged.
    let (_o, err, code) = run("module top;\n\
         reg c1=0, c2=0, a=0, b=0;\n\
         always #5 c1=~c1;\n\
         always #7 c2=~c2;\n\
         initial assert property(@(posedge c1 or posedge c2) a |-> b);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "an OR-of-clocks antecedent must be loud. {err}"
    );
    assert!(
        err.to_lowercase().contains("single clock") || err.contains("VITA-E"),
        "{err}"
    );
}

#[test]
fn multiclock_named_property_holds() {
    // The consequent clock is carried on a NAMED property too: `property p; @(c1) a
    // |=> @(c2) b; endproperty` spliced at `assert property(p)`.
    let (out, err, code) = run("module top;\n\
         reg c1=0, c2=0, a=1, b=1;\n\
         always #5 c1=~c1;\n\
         always #7 c2=~c2;\n\
         property p_mc; @(posedge c1) a |=> @(posedge c2) b; endproperty\n\
         initial assert property(p_mc);\n\
         initial begin #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "named multi-clock property holds → clean. stderr:\n{err}\nout:\n{out}"
    );
}
