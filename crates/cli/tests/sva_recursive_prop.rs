//! Slice N2d — property-level `and`/`or` operators + recursive properties.
//!
//! A property body using property-level `and`/`or` (or a legal tail-`|=>`
//! self-reference) parses to a `PropExpr` tree (AST flip → `.vu` re-pins once,
//! sim-ir frozen / format_version 8 — pure IR-0). Elaborate's `synth_prop_expr`
//! reduces the tree to a SINGLE per-clock boolean violation check:
//!   `L and R` → `viol(L) || viol(R)`   (both must hold)
//!   `L or R`  → `viol(L) && viol(R)`   (either must hold)
//!   `a |-> c` → `a && viol(c)`          (same clock)
//!   `a |=> c` → 1-bit pend reg, `pend && viol(c)`   (next clock)
//! and the legal TAIL recursion `… |=> NAME` drops to no-violation (`1'b0`) — its
//! next-clock obligation is discharged by the per-clock re-attempt that
//! `assert property(NAME)` spawns. So the canonical idioms reduce exactly:
//!   always:     `property p; @(c) b and (1'b1 |=> p); endproperty`        → `if(!b)`
//!   weak-until: `property p; @(c) q or (b and (1'b1 |=> p)); endproperty` → `if(!q && !b)`
//!
//! iverilog 13.0 rejects ALL concurrent assertions (NULL oracle) → hand-IEEE; every
//! expectation is hand-traced + value-pinned. Single clock `always #5 clk=~clk`
//! → posedges @5,15,25,35,…; `#40 $finish` ⇒ exactly four checks (@5,15,25,35).
//!
//! LOUD (out of subset): overlap (`|->`) recursion, a bare/antecedent self-reference,
//! a reference to ANOTHER named property inside a tree, a multi-term/re-clocked
//! sequence operand, `disable iff` / a pass action / a parameterized property
//! combined with `and`/`or`.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svarec_{}_{n}", std::process::id()));
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

fn fire_count(out: &str, err: &str) -> usize {
    format!("{err}{out}")
        .matches("Assertion property violation")
        .count()
}

const CLK: &str = "reg clk=0; always #5 clk=~clk;\n";

// ─────────────────────────── recursion: `always b` ───────────────────────────

#[test]
fn always_recursion_holds_when_b_steady() {
    // `b and (1'b1 |=> p)` ≡ "b at every clock" → b=1 always ⇒ no violation.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg b=1;\n\
         property p; @(posedge clk) b and (1'b1 |=> p); endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(0),
        "b steady-1 ⇒ clean. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !err.contains("VITA-E"),
        "must synthesize, not reject:\n{err}"
    );
}

#[test]
fn always_recursion_fires_once_on_single_b_drop() {
    // b=0 only across the @25 posedge (pulse [22,28]) → exactly ONE check sees !b.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg b=1;\n\
         property p; @(posedge clk) b and (1'b1 |=> p); endproperty\n\
         initial assert property(p);\n\
         initial begin #22 b=0; #6 b=1; #40 $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(1),
        "a b-drop over @25 must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert_eq!(
        fire_count(&out, &err),
        1,
        "exactly ONE violation (only the @25 check sees b=0):\n{err}\n{out}"
    );
}

#[test]
fn always_recursion_fires_each_clock_b_low() {
    // b=0 from @22 onward → @25 and @35 both fire (2 checks see !b). The finish is
    // an ABSOLUTE @40 (separate initial), so exactly posedges @5,15,25,35 occur.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg b=1;\n\
         property p; @(posedge clk) b and (1'b1 |=> p); endproperty\n\
         initial assert property(p);\n\
         initial #22 b=0;\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(code, Some(1), "stderr:\n{err}\nout:\n{out}");
    assert_eq!(
        fire_count(&out, &err),
        2,
        "b low across @25 and @35 ⇒ 2 violations:\n{err}\n{out}"
    );
}

// ─────────────────────────── recursion: weak-until ───────────────────────────

#[test]
fn weak_until_holds_while_b_steady() {
    // `q or (b and (1'b1 |=> p))` → `if(!q && !b)`. q=0, b=1 always ⇒ never fires.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg b=1, q=0;\n\
         property p; @(posedge clk) q or (b and (1'b1 |=> p)); endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(0),
        "b holds, q never ⇒ clean (weak until). stderr:\n{err}\nout:\n{out}"
    );
    assert!(!err.contains("VITA-E"), "must synthesize:\n{err}");
}

#[test]
fn weak_until_fires_when_b_drops_before_q() {
    // q=0 throughout; b=0 over @25 → !q && !b at @25 → fire once.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg b=1, q=0;\n\
         property p; @(posedge clk) q or (b and (1'b1 |=> p)); endproperty\n\
         initial assert property(p);\n\
         initial begin #22 b=0; #6 b=1; #40 $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(code, Some(1), "stderr:\n{err}\nout:\n{out}");
    assert_eq!(
        fire_count(&out, &err),
        1,
        "one !q&&!b clock (@25):\n{err}\n{out}"
    );
}

#[test]
fn weak_until_q_rescues_later_b_drop() {
    // q rises at @12 (held); b drops at @22. Once q holds, b is irrelevant.
    // @5: q=0,b=1 (no) · @15: q=1 (no) · @25: q=1,b=0 → !q=0 ⇒ NO fire (q rescued).
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg b=1, q=0;\n\
         property p; @(posedge clk) q or (b and (1'b1 |=> p)); endproperty\n\
         initial assert property(p);\n\
         initial begin #12 q=1; #10 b=0; #40 $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(0),
        "q held from @15 rescues the later b-drop (weak until). stderr:\n{err}\nout:\n{out}"
    );
    assert_eq!(fire_count(&out, &err), 0, "no fire after q:\n{err}\n{out}");
}

// ───────────────────────── non-recursive `and` / `or` ─────────────────────────

#[test]
fn nonrecursive_and_holds() {
    // `(a |-> b) and (c |-> d)` → viol = (a&&!b) || (c&&!d). All hold ⇒ clean.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg a=1, b=1, c=1, d=1;\n\
         initial assert property(@(posedge clk) (a |-> b) and (c |-> d));\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(0),
        "all leaves hold ⇒ clean. stderr:\n{err}\nout:\n{out}"
    );
    assert!(!err.contains("VITA-E"), "must synthesize:\n{err}");
}

#[test]
fn nonrecursive_and_fires_when_either_leaf_violates() {
    // a=1,b=0 (first leaf violates) → `and` fails every clock.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg a=1, b=0, c=1, d=1;\n\
         initial assert property(@(posedge clk) (a |-> b) and (c |-> d));\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(1),
        "first leaf violates ⇒ and fails. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        fire_count(&out, &err) >= 1,
        "at least one fire:\n{err}\n{out}"
    );
}

#[test]
fn nonrecursive_or_holds_when_one_leaf_holds() {
    // `(a |-> b) or (c |-> d)` → viol = (a&&!b) && (c&&!d). a=1,b=0 violates the
    // first leaf, but c|->d HOLDS (c=1,d=1) → or is satisfied ⇒ NO fire.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg a=1, b=0, c=1, d=1;\n\
         initial assert property(@(posedge clk) (a |-> b) or (c |-> d));\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(0),
        "or: one holding leaf is enough ⇒ clean. stderr:\n{err}\nout:\n{out}"
    );
    assert_eq!(fire_count(&out, &err), 0, "no fire:\n{err}\n{out}");
}

#[test]
fn nonrecursive_or_fires_when_both_leaves_violate() {
    // a=1,b=0 AND c=1,d=0 → BOTH leaves violate → or fails.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg a=1, b=0, c=1, d=0;\n\
         initial assert property(@(posedge clk) (a |-> b) or (c |-> d));\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(1),
        "both leaves violate ⇒ or fails. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        fire_count(&out, &err) >= 1,
        "at least one fire:\n{err}\n{out}"
    );
}

#[test]
fn bare_boolean_and_requires_both() {
    // `a and b` (bare booleans as properties) → viol = !a || !b. a=1, b=0 → fire.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg a=1, b=0;\n\
         initial assert property(@(posedge clk) a and b);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(1),
        "b=0 ⇒ `a and b` fails. stderr:\n{err}\nout:\n{out}"
    );
    assert!(fire_count(&out, &err) >= 1, "fires:\n{err}\n{out}");
}

// ─────────────────── non-recursive `|=>` inside a tree ───────────────────────
// (review N2d HIGH): a `|=>` operand (clock skew 1) may combine with `and`/`or`
// ONLY against same-skew siblings — i.e. another `|=>` — never a skew-0 operand
// (a boolean / `|->`). Mixing skews would pair two different attempt-start clocks
// (a silent false-pass / false-fire), so it is loud-rejected.

#[test]
fn double_nonoverlap_in_tree_holds_then_fires() {
    // `(a |=> b) and (c |=> d)` — BOTH operands skew 1 (uniform) → verdict-correct.
    // a=c=1, b=d=1 always ⇒ no implication fails ⇒ clean.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg a=1, b=1, c=1, d=1;\n\
         initial assert property(@(posedge clk) (a |=> b) and (c |=> d));\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(0),
        "uniform-skew `|=>` tree must synthesize and hold. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !err.contains("VITA-E"),
        "must synthesize, not reject:\n{err}"
    );

    // Now b=0 always → the first implication (a|=>b) fails every clock after the
    // first → fires (one clock skewed, but a real failure is reported).
    let (out2, err2, code2) = run(&format!(
        "module top;\n{CLK}reg a=1, b=0, c=1, d=1;\n\
         initial assert property(@(posedge clk) (a |=> b) and (c |=> d));\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code2,
        Some(1),
        "a|=>b violated ⇒ fires. stderr:\n{err2}\nout:\n{out2}"
    );
    assert!(fire_count(&out2, &err2) >= 1, "fires:\n{err2}\n{out2}");
}

#[test]
fn skew_mismatch_nonoverlap_with_boolean_is_loud() {
    // `(a |=> b) and 1'b1` mixes a skew-1 `|=>` with a skew-0 boolean → loud-reject
    // (review N2d HIGH: this misalignment was a silent false-pass / false-fire).
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg a=1, b=0;\n\
         initial assert property(@(posedge clk) (a |=> b) and 1'b1);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_loud(&out, &err, code, "skew mismatch (|=> and boolean)");
}

#[test]
fn skew_mismatch_nonoverlap_or_boolean_is_loud() {
    // `(a |=> b) or q` — the exact HIGH finding. Skew-1 `|=>` OR skew-0 boolean →
    // loud-reject (previously a silent verdict flip under time-varying stimulus).
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg a=0, b=0, q=0;\n\
         property p; @(posedge clk) (a |=> b) or q; endproperty\n\
         initial assert property(p);\n\
         initial begin #11 a=1; #6 a=0; #4 q=1; #60 $finish; end\n\
         endmodule\n"
    ));
    assert_loud(&out, &err, code, "skew mismatch (|=> or boolean)");
}

#[test]
fn nonoverlap_with_tree_consequent_holds() {
    // `b |=> (c and d)` — a `|=>` whose consequent is a same-clock and/or tree
    // (cons skew 0). All operands of the inner `and` are same-clock → CORRECT
    // (the pend delays b by one; c,d are both sampled next clock). Holds when
    // b=1,c=1,d=1.  NOTE: no top-level and/or here, so it parses to the flat... no —
    // `b |=> c and d` has a top-level `and` → tree; cons of |=> is `(c and d)`.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg b=1, c=1, d=1;\n\
         initial assert property(@(posedge clk) b |=> c and d);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(0),
        "b |=> (c and d) holds. stderr:\n{err}\nout:\n{out}"
    );
    assert!(!err.contains("VITA-E"), "must synthesize:\n{err}");
}

#[test]
fn nested_skew_consequent_is_loud() {
    // `(a |=> (c |=> d)) and z` — a nested multi-clock-skew `|=>` consequent needs
    // a multi-stage pend network → out of subset, loud-reject.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg a=1, c=1, d=1, z=1;\n\
         initial assert property(@(posedge clk) (a |=> (c |=> d)) and z);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_loud(&out, &err, code, "nested skew consequent");
}

// ───────────────────────────── fail action works ─────────────────────────────

#[test]
fn custom_fail_action_on_andor() {
    // A custom `else` fail statement replaces the default $error on an and/or tree.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg a=1, b=0;\n\
         initial assert property(@(posedge clk) a and b) else $error(\"N2D_CUSTOM_FAIL\");\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(code, Some(1), "stderr:\n{err}\nout:\n{out}");
    assert!(
        format!("{err}{out}").contains("N2D_CUSTOM_FAIL"),
        "custom fail message must appear:\n{err}\n{out}"
    );
}

// ───────────────────────────────── determinism ────────────────────────────────

#[test]
fn determinism_recursion() {
    let src = format!(
        "module top;\n{CLK}reg b=1, q=0;\n\
         property p; @(posedge clk) q or (b and (1'b1 |=> p)); endproperty\n\
         initial assert property(p);\n\
         initial begin #22 b=0; #6 b=1; #40 $finish; end\n\
         endmodule\n"
    );
    let (o1, e1, c1) = run(&src);
    let (o2, e2, c2) = run(&src);
    assert_eq!(
        (o1, e1, c1),
        (o2, e2, c2),
        "N2d output must be deterministic"
    );
}

// ───────────────────────────────── loud rejects ───────────────────────────────

fn assert_loud(out: &str, err: &str, code: Option<i32>, needle: &str) {
    assert_ne!(
        code,
        Some(0),
        "must be loud-rejected, not silently accepted:\n{err}\n{out}"
    );
    let blob = format!("{err}{out}").to_lowercase();
    assert!(
        blob.contains("unsupported") || blob.contains("vita-e"),
        "expected a loud unsupported diagnostic ({needle}):\n{err}\n{out}"
    );
}

#[test]
fn overlap_recursion_is_loud() {
    // `1'b1 |-> p` (overlap, same-tick) recursion is an illegal fixpoint.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg b=1;\n\
         property p; @(posedge clk) b and (1'b1 |-> p); endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_loud(&out, &err, code, "overlap recursion");
}

#[test]
fn bare_self_reference_is_loud() {
    // `b or p` — `p` is referenced bare (not as a `|=>` consequent).
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg b=1;\n\
         property p; @(posedge clk) b or p; endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_loud(&out, &err, code, "bare self reference");
}

#[test]
fn cross_property_reference_in_tree_is_loud() {
    // `b and q` where `q` is ANOTHER declared property — cross-property trees are
    // out of subset.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg b=1, x=1;\n\
         property q; @(posedge clk) 1'b1 |-> x; endproperty\n\
         property p; @(posedge clk) b and q; endproperty\n\
         initial assert property(p);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_loud(&out, &err, code, "cross-property reference");
}

#[test]
fn reclocked_operand_in_tree_is_loud() {
    // A re-clocked `@(posedge c2) c |-> d` operand of `and` (multi-clock + tree).
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg c2=0; always #7 c2=~c2;\n\
         reg a=1, b=1, c=1, d=1;\n\
         initial assert property(@(posedge clk) (a |-> b) and (@(posedge c2) c |-> d));\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_loud(&out, &err, code, "re-clocked operand");
}

#[test]
fn repeat_operand_then_andor_is_cleanly_loud() {
    // `a[*2] and b` — an SVA repeat operand BEFORE a top-level `and`. The lookahead
    // scans must count the `[*` bracket (review N2d parser MEDIUM) so the `and` is
    // still detected at depth 0 → the repeat operand reaches the clean
    // "operand must be a boolean (multi-term ... unsupported)" reject, not a
    // degraded parse-error storm.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg a=1, b=0;\n\
         initial assert property(@(posedge clk) a[*2] and b);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_loud(&out, &err, code, "repeat operand before and/or");
    assert!(
        format!("{err}{out}").contains("must be a boolean"),
        "expected the targeted multi-term-operand diagnostic, not a parse storm:\n{err}\n{out}"
    );
}

#[test]
fn deep_andor_nesting_is_loud_not_a_crash() {
    // A pathologically deep `a and a and …` chain must hit the loud depth cap, not
    // overflow the stack (review N2d determinism MEDIUM).
    let chain = std::iter::repeat("a")
        .take(2000)
        .collect::<Vec<_>>()
        .join(" and ");
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg a=1;\n\
         initial assert property(@(posedge clk) {chain});\n\
         initial #20 $finish;\n\
         endmodule\n"
    ));
    // Must terminate with a diagnostic (not SIGABRT/stack overflow, exit 134).
    assert_ne!(code, Some(134), "must NOT stack-overflow:\n{err}\n{out}");
    assert_loud(&out, &err, code, "deep and/or nesting cap");
}

#[test]
fn very_deep_andor_chain_does_not_overflow_default_stack() {
    // Regression for the Windows CI stack overflow (after 27d639d): the parser
    // builds `a and a and …` into a left-leaning `PropExpr` tree that
    // `synth_prop_expr` CLONEs (and later drops) recursively. At 40 000 operands
    // that recursion overflows not just Windows' ~1 MiB main-thread stack but the
    // ~8 MiB Linux/macOS default too — so it crashes on EVERY OS unless the driver
    // runs on the large-stack worker thread (see `crates/cli/src/main.rs`). It must
    // still terminate with the loud depth-cap diagnostic, never an overflow abort.
    let chain = std::iter::repeat("a")
        .take(40_000)
        .collect::<Vec<_>>()
        .join(" and ");
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg a=1;\n\
         initial assert property(@(posedge clk) {chain});\n\
         initial #20 $finish;\n\
         endmodule\n"
    ));
    // `134` is the Unix SIGABRT code; on Windows a stack overflow exits with a
    // different (STATUS_STACK_OVERFLOW) code, so `assert_loud` is the real
    // cross-OS guard: it requires a nonzero exit AND a "unsupported"/"vita-e"
    // diagnostic, which a silent overflow abort cannot produce on any OS.
    assert_ne!(code, Some(134), "must NOT stack-overflow:\n{err}\n{out}");
    assert_loud(&out, &err, code, "very deep and/or chain");
}

#[test]
fn disable_iff_with_andor_is_loud() {
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg a=1, b=0, c=1, d=1, rst=0;\n\
         initial assert property(@(posedge clk) disable iff (rst) (a |-> b) and (c |-> d));\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_loud(&out, &err, code, "disable iff + and/or");
}

#[test]
fn pass_action_with_andor_is_loud() {
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg a=1, b=1, c=1, d=1;\n\
         initial assert property(@(posedge clk) (a |-> b) and (c |-> d)) $display(\"ok\");\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_loud(&out, &err, code, "pass action + and/or");
}

// ───────────────────────── byte-identity sanity (flat) ─────────────────────────

#[test]
fn flat_property_still_works_alongside_n2d() {
    // A plain flat `a |-> b` (no and/or) keeps the byte-identical flat path
    // (prop_expr=None). a=1,b=0 ⇒ fire.
    let (out, err, code) = run(&format!(
        "module top;\n{CLK}reg a=1, b=0;\n\
         initial assert property(@(posedge clk) a |-> b);\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    assert_eq!(
        code,
        Some(1),
        "flat path unchanged. stderr:\n{err}\nout:\n{out}"
    );
    assert!(fire_count(&out, &err) >= 1, "fires:\n{err}\n{out}");
}
