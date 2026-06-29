//! Empty-match consecutive repetition `[*0:n]` / `[*0:$]` / `[*]` / `sig[*]`
//! (ROADMAP §4.5 ② SVA residual, slice 2026-06-25). `b[*0:n]` ≡ `empty | b[*1:n]`:
//! the EMPTY branch (zero repetitions) is a zero-extent match.
//!
//! SUPPORTED (P1, slice A.1 2026-06-26): the empty branch surrounded by FIXED
//! delays `##d` (d ≥ 1) on BOTH sides — `a ##h_in b[*0:n] ##h_out c`, whose empty
//! (k=0) alternative has net delay `D = (h_in - 1) + h_out` (the empty consumes
//! exactly one clock of the leading hop, per §16.9.2.1 `(r ##n empty)=(r ##(n-1)
//! `true)`). The canonical `##1`/`##1` case (D=1) is byte-identical to before.
//! The suffix `a ##1 b[*0:n]` (empty ≡ `a`) is likewise supported.
//!
//! HONEST-LOUD (subtle IEEE 1800-2017 §16.9.2.1 algebra with NO differential
//! oracle — a guessed delay would be silent-wrong, so it is loud instead): the
//! empty adjacent to a `##0` delay (leading OR trailing — the absorption is a
//! genuine §16.9.2.1 discontinuity); an UNBOUNDED `##[m:$]` hop around the empty;
//! the empty as the SEED (a leading / standalone `b[*0:n]`); the empty in a
//! CONSEQUENT or under `throughout`/`within`; and `(a, x=d)` per-attempt local
//! vars. (An earlier draft tried to fuse arbitrary `##d` delays and an adversarial
//! review found a trailing-`##0` off-by-one — hence `##0`/unbounded stay loud;
//! see the *_is_loud tests. The fixed-hop non-`##1` cases are P1, supported.)
//!
//! iverilog 13 supports NO concurrent assertions → every verdict is HAND-IEEE
//! (no differential oracle), independently cross-checked before freezing.
//! Clock: `always #5 clk=~clk` → posedges at t=5,15,25,35,45,55,…; a value set
//! at `#10k` is sampled at the next posedge.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svaem_{}_{n}", std::process::id()));
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

fn fires(out: &str, err: &str, code: Option<i32>, ctx: &str) {
    assert_eq!(
        code,
        Some(1),
        "{ctx}: expected a violation (exit 1).\nstderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{ctx}: a violation diagnostic was expected.\nstderr:\n{err}\nout:\n{out}"
    );
}

fn holds(out: &str, err: &str, code: Option<i32>, ctx: &str) {
    assert_eq!(
        code,
        Some(0),
        "{ctx}: expected a clean pass (exit 0).\nstderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "{ctx}: no violation expected.\nstderr:\n{err}\nout:\n{out}"
    );
}

fn loud(out: &str, err: &str, code: Option<i32>, ctx: &str) {
    assert_ne!(code, Some(0), "{ctx}: must not silently pass.\n{err}{out}");
    assert!(
        format!("{err}{out}").contains("VITA-E"),
        "{ctx}: expected a loud diagnostic.\nstderr:\n{err}\nout:\n{out}"
    );
}

// ── MIDDLE: `a ##1 b[*0:2] ##1 c` — k=0 (empty), k=1, k=2, upper bound ──

#[test]
fn middle_empty_branch_k0_fires() {
    // k=0 (empty) ≡ `a ##1 c` (the two ##1 fuse): a@t15, NO b, c@t25, d=0 → the
    // empty alternative completes at t25 → obliges d@t25 → fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b[*0:2] ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0; c=1; d=0;\n\
           #10 c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(&out, &err, code, "a ##1 b[*0:2] ##1 c, k=0 empty branch");
}

#[test]
fn middle_k1_branch_fires() {
    // k=1 ≡ `a ##1 b ##1 c`: a@t15, b@t25, c@t35, d=0 → fire at t35.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b[*0:2] ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0; b=1;\n\
           #10 b=0; c=1; d=0;\n\
           #10 c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(&out, &err, code, "a ##1 b[*0:2] ##1 c, k=1 branch");
}

#[test]
fn middle_k2_branch_fires() {
    // k=2 ≡ `a ##1 b ##1 b ##1 c`: a@t15, b@t25, b@t35, c@t45, d=0 → fire at t45.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b[*0:2] ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0; b=1;\n\
           #10 b=1;\n\
           #10 b=0; c=1; d=0;\n\
           #10 c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(&out, &err, code, "a ##1 b[*0:2] ##1 c, k=2 branch");
}

#[test]
fn middle_upper_bound_k3_excluded_holds() {
    // 3 consecutive b's then c is k=3 — OUTSIDE [*0:2] → NO alternative matches →
    // no obligation → clean (pins the bounded upper edge, even with d=0).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b[*0:2] ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0; b=1;\n\
           #10 b=1;\n\
           #10 b=1;\n\
           #10 b=0; c=1; d=0;\n\
           #10 c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    holds(
        &out,
        &err,
        code,
        "a ##1 b[*0:2] ##1 c, k=3 outside the bound",
    );
}

// ── SUFFIX: `a ##1 b[*0:1]` as an antecedent — empty branch ≡ `a` ──

#[test]
fn suffix_empty_branch_is_just_a_fires() {
    // `a ##1 b[*0:1] |-> c`: empty (k=0) ≡ `a ##1 empty` ≡ `a` → antecedent
    // completes at a's clock t15; c=0 there → fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b[*0:1] |-> c);\n\
         initial begin\n\
           #10 a=1; c=0;\n\
           #10 a=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(&out, &err, code, "a ##1 b[*0:1] suffix empty ≡ a");
}

// ── HONEST-LOUD: non-`##1` adjacency (the fusion is subtle without an oracle) ──

#[test]
fn trailing_zero_delay_after_empty_is_loud() {
    // `a ##1 b[*0:1] ##0 c |-> e`: a trailing `##0` after the empty. An adversarial
    // review found an earlier draft fused this ONE CLOCK TOO EARLY — the §16.9.2.1
    // semantics here are subtle and oracle-free, so it is honest-loud, NOT a
    // guessed delay. (Regression for the review's silent-wrong finding.)
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, e=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b[*0:1] ##0 c |-> e);\n\
         initial begin #10 a=1; #40 $finish; end\n\
         endmodule\n");
    loud(&out, &err, code, "trailing ##0 after empty");
}

#[test]
fn leading_zero_delay_before_empty_is_loud() {
    // `a ##0 b[*0:1] ##1 c |-> e`: a leading `##0` before the empty is likewise
    // unverifiable → loud.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, e=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##0 b[*0:1] ##1 c |-> e);\n\
         initial begin #10 a=1; #40 $finish; end\n\
         endmodule\n");
    loud(&out, &err, code, "leading ##0 before empty");
}

// ── SLICE A.1: empty-match at NON-`##1` adjacency, P1 (hop_in>=1 AND ──
// ── hop_out>=1, both Fixed). §16.9.2.1 `(r ##n empty)=(r ##(n-1) `true)`:        ──
// ── for `X ##hop_in b[*0:n] ##hop_out Y` the empty branch has net delay          ──
// ── D = (hop_in-1) + hop_out (the empty's length-0 absorbs exactly one clock of  ──
// ── hop_in). Validated by hand clock-count; iverilog has no SVA oracle.          ──

#[test]
fn a1_middle_fixed_hops_empty_fires() {
    // TC1: `a ##2 b[*0:2] ##1 c |-> d`. Empty branch D=(2-1)+1=2: a's start clock
    // +2 → completion. a@t15 (start), NO b, c@t35, d=0 → empty alt completes @t35,
    // obliges d@t35, d=0 → FIRES.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##2 b[*0:2] ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0;\n\
           #10 c=1; d=0;\n\
           #10 c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(&out, &err, code, "a ##2 b[*0:2] ##1 c, empty branch D=2");
}

#[test]
fn a1_middle_fixed_hops_k1_fanout_fires() {
    // TC2 (k=1 fan-out regression at non-##1 hop_in): same prop. a@t15, b@t35,
    // c@t45, d=0. k=1 branch = `a ##2 b ##1 c`: a@t15, b two clocks later @t35,
    // c one clock later @t45 → FIRES @t45.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##2 b[*0:2] ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0;\n\
           #10 b=1;\n\
           #10 b=0; c=1; d=0;\n\
           #10 c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(
        &out,
        &err,
        code,
        "a ##2 b[*0:2] ##1 c, k=1 fan-out @non-##1 hop_in",
    );
}

#[test]
fn a1_middle_fixed_hop_out_empty_fires() {
    // TC3: `a ##1 b[*0:1] ##2 c |-> d`. Empty branch D=(1-1)+2=2: a@t15 (start),
    // NO b, c@t35, d=0 → empty alt completes @t35 → FIRES.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b[*0:1] ##2 c |-> d);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0;\n\
           #10 c=1; d=0;\n\
           #10 c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(&out, &err, code, "a ##1 b[*0:1] ##2 c, empty branch D=2");
}

#[test]
fn a1_middle_pins_delay_two_not_one_holds() {
    // TC4: TC1 prop `a ##2 b[*0:2] ##1 c |-> d`, but c high ONLY @t25 (one clock
    // too early for D=2). If the fused delay were wrongly D=1 the empty alt would
    // complete @t25 and fire; with the correct D=2 NO alternative completes (c is
    // gone by t35) → no obligation → clean (exit 0). Pins the off-by-one.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##2 b[*0:2] ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0; c=1; d=0;\n\
           #10 c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    holds(
        &out,
        &err,
        code,
        "a ##2 b[*0:2] ##1 c pins D=2 (c@t25 too early)",
    );
}

#[test]
fn a1_unbounded_hop_around_empty_is_loud() {
    // TC7 (KEEP-LOUD P4): `a ##[1:$] b[*0:1] ##1 c |-> d`. An `##[m:$]` adjacent to
    // the empty is a NON-Fixed hop — the fused delay is a range with no window-
    // length argument and no oracle → honest-loud (NOT a guessed delay).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##[1:$] b[*0:1] ##1 c |-> d);\n\
         initial begin #10 a=1; #60 $finish; end\n\
         endmodule\n");
    loud(&out, &err, code, "a ##[1:$] b[*0:1] ##1 c (AtLeast hop)");
}

// ── UNBOUNDED `[*0:$]` and the `[*]` / `sig[*]` sugars ──

#[test]
fn unbounded_zero_lower_empty_branch_fires() {
    // `a ##1 b[*0:$] ##1 c |-> d`: empty branch ≡ `a ##1 c`. a@t15, NO b, c@t25,
    // d=0 → fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b[*0:$] ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0; c=1; d=0;\n\
           #10 c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(&out, &err, code, "a ##1 b[*0:$] ##1 c, empty branch");
}

#[test]
fn bare_star_sugar_parses_and_empty_branch_fires() {
    // `b[*]` ≡ `b[*0:$]`: same empty branch as above.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b[*] ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0; c=1; d=0;\n\
           #10 c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(
        &out,
        &err,
        code,
        "a ##1 b[*] ##1 c, empty branch (bare-star sugar)",
    );
}

// ── HONEST-LOUD: empty match as the SEED (leading / standalone) ──

#[test]
fn leading_empty_seed_is_honest_loud() {
    // `b[*0:2] ##1 c |-> d`: the empty branch would be the SEED of the match —
    // its -1 start offset is not expressible here → loud (not a silent miss).
    let (out, err, code) = run("module top;\n\
         reg clk=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) b[*0:2] ##1 c |-> d);\n\
         initial begin #10 c=1; #40 $finish; end\n\
         endmodule\n");
    loud(&out, &err, code, "leading b[*0:2] empty seed");
}

#[test]
fn standalone_empty_is_honest_loud() {
    // `b[*0:2] |-> c`: the antecedent's empty alternative is a bare seed → loud.
    let (out, err, code) = run("module top;\n\
         reg clk=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) b[*0:2] |-> c);\n\
         initial begin #10 c=1; #40 $finish; end\n\
         endmodule\n");
    loud(&out, &err, code, "standalone b[*0:2]");
}

// ── regression: a positive-bound `[*1:2]` is UNAFFECTED by the min=0 path ──

#[test]
fn positive_bound_repeat_still_fires() {
    // `a ##1 b[*1:2] ##1 c |-> d`: no empty branch — a@t15, b@t25, c@t35, d=0 →
    // k=1 fires (pins that the min>=1 fan-out is byte-unchanged).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##1 b[*1:2] ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0; b=1;\n\
           #10 b=0; c=1; d=0;\n\
           #10 c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(
        &out,
        &err,
        code,
        "a ##1 b[*1:2] ##1 c, k=1 (no empty branch)",
    );
}

// ── regression: exact-empty `[*0]` / `[*0:0]` SINGLE-alt path at non-`##1` ──
// `b[*0]` lowers to ONLY the Empty alternative (no `[*1:n]` fan-out), a distinct
// code path from `[*0:n]` (n>=1). Pin that the P1 net-delay D=(h_in-1)+h_out is
// correct on that path too (hand-IEEE; review A.1 flagged it as live-verified but
// uncommitted).

#[test]
fn a1_exact_empty_star0_fixed_hops_fires() {
    // `a ##2 b[*0] ##1 c |-> d`: ONLY the empty alt exists. D=(2-1)+1=2: a@t15,
    // c@t35, d=0 → completes @t35 → fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##2 b[*0] ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0;\n\
           #10 c=1; d=0;\n\
           #10 c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(&out, &err, code, "a ##2 b[*0] ##1 c exact-empty, D=2");
}

#[test]
fn a1_exact_empty_star0_pins_delay_two_holds() {
    // Same prop, but c high only @t25 (one clock early for D=2). The single empty
    // alt must NOT complete @t25 → clean (pins the off-by-one on the [*0] path).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a ##2 b[*0] ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1;\n\
           #10 a=0; c=1; d=0;\n\
           #10 c=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    holds(&out, &err, code, "a ##2 b[*0] ##1 c pins D=2 (c@t25 early)");
}
