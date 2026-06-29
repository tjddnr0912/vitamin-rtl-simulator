//! SVA-REST — the residual SVA surface (all pure IR-0 desugars; iverilog 13.0
//! rejects concurrent assertions, so these are hand-IEEE, verified by hand-computed
//! verdicts). Covers: `assume property` (sim-checked like assert, §16.12); the
//! property operators `always`/`not`/`implies`/`iff`/`until`/`s_until`/
//! `s_eventually`/`nexttime` (liveness = an end-of-sim `final` obligation check);
//! `cover property` (match counter + report); `let` expression macros;
//! `$assertoff`/`$asserton`/`$assertkill` (a global fire-gate); and the `seq[+]`
//! (`[*1:$]`) repetition sugar.
//!
//! Honest-loud residual (never silent-wrong): empty-match repetition
//! (`[*0:n]`/`[*0:$]`/bare `[*]`), multi-term cross-clock segments, sequence local
//! variables (N2c), and the advanced outer-`|=>` prop-ref skew — see docs/ROADMAP.md.
//!
//! Documented liveness caveat (pre-existing scheduler behavior, NOT a regression):
//! a `$finish` that coincides EXACTLY with the assertion clock's sampling posedge
//! is not observed by the clocked checker (the same reason a clocked `cnt<=cnt+1`
//! misses a finish-coincident edge), so the end-of-sim `final` obligation reads the
//! prior edge's state. The tests below finish at NON-edge times; offset `$finish`
//! from the sampling edge in practice. See `run_finals` + docs/DEVLOG.md (49탄).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svar_{}_{n}", std::process::id()));
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
fn assume_property_violation_is_reported() {
    // An assumption violated in simulation IS reported (checked like an assert).
    let (_out, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        always #5 clk = ~clk;\n\
        assume property(@(posedge clk) a |-> b);\n\
        initial begin #10 a=1; b=1; #10 a=1; b=0; #10 $finish; end\n\
        endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "violated assume must exit 1 (checked like assert)"
    );
}

#[test]
fn assume_property_satisfied_is_clean() {
    // A satisfied assumption produces no violation (exit 0).
    let (out, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        always #5 clk = ~clk;\n\
        assume property(@(posedge clk) a |-> b);\n\
        initial begin #10 a=1; b=1; #10 a=0; b=0; #10 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "satisfied assume exits 0:\n{out}");
}

#[test]
fn procedural_assume_property() {
    // `assume property` inside an initial/always block (procedural placement).
    let (_out, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        always #5 clk = ~clk;\n\
        initial assume property(@(posedge clk) a |-> b);\n\
        initial begin #10 a=1; b=1; #10 a=1; b=0; #10 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(1), "procedural assume violation exits 1");
}

// ── Property operators (always/implies/until/not/nexttime/s_eventually) ──────

#[test]
fn always_implication_violation() {
    // `always (a |-> b)` ≡ `a |-> b` under per-clock re-attempt.
    let (_o, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) always (a |-> b));\n\
        initial begin #10 a=1; b=1; #10 a=1; b=0; #10 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(1), "always(a|->b) with a&&!b fires");
}

#[test]
fn always_implication_clean() {
    let (o, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) always (a |-> b));\n\
        initial begin #10 a=1; b=1; #10 a=0; b=0; #10 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "always(a|->b) satisfied:\n{o}");
}

#[test]
fn implies_violation() {
    // `p implies q` ≡ `(not p) or q`: violated when p holds and q does not.
    let (_o, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) a implies b);\n\
        initial begin #10 a=1; b=1; #10 a=1; b=0; #10 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(1), "a implies b with a&&!b fires");
}

#[test]
fn implies_clean() {
    let (o, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) a implies b);\n\
        initial begin #10 a=0; b=0; #10 a=1; b=1; #10 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "a implies b satisfied:\n{o}");
}

#[test]
fn not_property_violation() {
    // `not (a && b)` holds iff a&&b is false; violated when a&&b true.
    let (_o, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) not (a && b));\n\
        initial begin #10 a=1; b=0; #10 a=1; b=1; #10 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(1), "not(a&&b) fires when a&&b");
}

#[test]
fn weak_until_violation() {
    // `a until b` (weak): a must hold every clock until b; violated when both fail.
    let (_o, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) a until b);\n\
        initial begin #10 a=1; b=0; #10 a=0; b=0; #10 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(1), "a until b: a fails before b → fires");
}

#[test]
fn weak_until_clean() {
    // a holds until b holds — no violation; weak so b need not be reached cleanly.
    let (o, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) a until b);\n\
        initial begin a=1; #20 b=1; #10 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "a until b satisfied:\n{o}");
}

#[test]
fn nexttime_violation() {
    // `nexttime p` ≡ `1'b1 |=> p`: p must hold on the next clock.
    let (_o, code) = run("module t;\n\
        logic clk=0, p=1;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) nexttime p);\n\
        initial begin #12 p=0; #20 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(1), "nexttime p fires when p=0 next clock");
}

#[test]
fn s_eventually_unsatisfied_fails_at_end() {
    // Strong liveness: ack never asserts → end-of-sim `final` obligation fails.
    let (_o, code) = run("module t;\n\
        logic clk=0, ack=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) s_eventually ack);\n\
        initial begin #40 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(1), "s_eventually ack never satisfied → exit 1");
}

#[test]
fn s_eventually_satisfied_clean() {
    // ack asserts and stays → liveness satisfied, exit 0.
    let (o, code) = run("module t;\n\
        logic clk=0, ack=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) s_eventually ack);\n\
        initial begin #20 ack=1; #20 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "s_eventually ack satisfied:\n{o}");
}

#[test]
fn req_then_s_eventually_ack_fails() {
    // `req |=> s_eventually ack`: req pulses, ack never → liveness fail.
    let (_o, code) = run("module t;\n\
        logic clk=0, req=0, ack=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) req |=> s_eventually ack);\n\
        initial begin #10 req=1; #10 req=0; #30 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(1), "req |=> s_eventually ack, no ack → exit 1");
}

#[test]
fn req_then_s_eventually_ack_clean() {
    // req pulses, ack follows later → satisfied.
    let (o, code) = run("module t;\n\
        logic clk=0, req=0, ack=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) req |=> s_eventually ack);\n\
        initial begin #10 req=1; #10 req=0; #10 ack=1; #20 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "req |=> s_eventually ack satisfied:\n{o}");
}

#[test]
fn s_until_strong_liveness_fails() {
    // `a s_until b`: a holds but b never → strong liveness (b must occur) fails.
    let (_o, code) = run("module t;\n\
        logic clk=0, a=1, b=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) a s_until b);\n\
        initial begin #40 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(1), "a s_until b, b never → exit 1");
}

// ── cover property ──────────────────────────────────────────────────────────

#[test]
fn cover_property_reports_hits() {
    // Counts matches of `a` and reports the hit count at end-of-sim; never fails.
    let (out, code) = run("module t;\n\
        logic clk=0, a=0;\n\
        always #5 clk = ~clk;\n\
        cover property(@(posedge clk) a);\n\
        initial begin #10 a=1; #10 a=0; #10 a=1; #10 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "cover never fails:\n{out}");
    assert!(
        out.contains("Cover property hits:"),
        "cover prints a hit report:\n{out}"
    );
}

#[test]
fn cover_property_sequence() {
    // A sequence cover `a ##1 b` — counts completed matches.
    let (out, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        always #5 clk = ~clk;\n\
        cover property(@(posedge clk) a ##1 b);\n\
        initial begin #10 a=1; #10 a=0; b=1; #10 b=0; #10 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "sequence cover never fails:\n{out}");
    assert!(
        out.contains("Cover property hits:"),
        "report present:\n{out}"
    );
}

// ── let declaration ─────────────────────────────────────────────────────────

#[test]
fn let_macro_substitution() {
    // `let inc = a + 1;` — used in ordinary RTL.
    let (out, _code) = run("module t;\n\
        logic [7:0] a=8'd5, r=0;\n\
        let inc = a + 1;\n\
        initial begin #1 r = inc; $display(\"r=%0d\", r); $finish; end\n\
        endmodule\n");
    assert!(out.contains("r=6"), "let inc=a+1 → r=6:\n{out}");
}

#[test]
fn let_with_formals() {
    // `let max2(x,y) = (x>y)?x:y;` — positional formal binding at the use site.
    let (out, _code) = run("module t;\n\
        logic [7:0] p=8'd3, q=8'd9, r=0;\n\
        let max2(x,y) = (x > y) ? x : y;\n\
        initial begin #1 r = max2(p, q); $display(\"r=%0d\", r); $finish; end\n\
        endmodule\n");
    assert!(out.contains("r=9"), "max2(3,9) → 9:\n{out}");
}

#[test]
fn let_in_assertion() {
    // `let` used inside a concurrent assertion antecedent.
    let (_o, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        let both = a && b;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) both |-> b);\n\
        initial begin #10 a=1; b=1; #10 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "let in assertion antecedent, satisfied");
}

// ── $assertoff / $asserton / $assertkill ────────────────────────────────────

#[test]
fn assertoff_suppresses_violation() {
    // `$assertoff` before the violation → the fire is suppressed (exit 0).
    let (o, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) a |-> b);\n\
        initial begin $assertoff; #10 a=1; b=0; #10 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "assertoff suppresses the fire:\n{o}");
}

#[test]
fn asserton_reenables() {
    // `$assertoff` then `$asserton` → the later violation DOES fire (exit 1).
    let (_o, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) a |-> b);\n\
        initial begin $assertoff; #2 $asserton; #10 a=1; b=0; #10 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(1), "asserton re-enables the fire");
}

#[test]
fn assertkill_suppresses_violation() {
    // `$assertkill` (= off) suppresses subsequent fires.
    let (o, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) a |-> b);\n\
        initial begin $assertkill; #10 a=1; b=0; #10 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "assertkill suppresses the fire:\n{o}");
}

#[test]
fn assertoff_baseline_still_fires() {
    // Control: no assertoff → the violation fires (the gate is not always-on).
    let (_o, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) a |-> b);\n\
        initial begin #10 a=1; b=0; #10 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(1), "without assertoff, the fire still happens");
}

// ── repetition sugar `[+]` (= `[*1:$]`) + empty-match loud ───────────────────

#[test]
fn consec_plus_sugar_equals_explicit() {
    // `seq[+]` is exactly `seq[*1:$]` (one-or-more, the S13 run-latch) — same verdict.
    let src = |rep: &str| {
        format!(
            "module t;\n\
             logic clk=0, a=0, b=0, c=0;\n\
             always #5 clk = ~clk;\n\
             assert property(@(posedge clk) (a ##1 {rep} ##1 c) |-> c);\n\
             initial begin #10 a=1; #10 a=0; b=1; #10 b=1; #10 b=0; c=1; #10 c=0; #10 $finish; end\n\
             endmodule\n"
        )
    };
    let (_o1, plus) = run(&src("b[+]"));
    let (_o2, explicit) = run(&src("b[*1:$]"));
    assert_eq!(plus, explicit, "b[+] ≡ b[*1:$] (same verdict)");
}

#[test]
fn bare_star_empty_match_now_supported_in_suffix() {
    // 2026-06-25: `b[*]` (≡ `[*0:$]` empty-or-more) LANDED in SUFFIX position. With
    // no stimulus the antecedent `a ##1 b[*]` never matches → a clean run (exit 0),
    // never a parse error. Full empty-match semantics live in `sva_empty_match.rs`.
    let (out, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) a ##1 b[*] |-> b);\n\
        initial begin #10 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(0), "a ##1 b[*] (suffix) now runs cleanly: {out}");
    assert!(!out.contains("VITA-E2002"), "must parse, not error: {out}");
}

#[test]
fn zero_repeat_range_now_supported_in_suffix() {
    // `b[*0:2]` empty-match in SUFFIX position is supported; no stimulus → clean.
    let (out, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) a ##1 b[*0:2] |-> b);\n\
        initial begin #10 $finish; end\n\
        endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "a ##1 b[*0:2] (suffix) now runs cleanly: {out}"
    );
    assert!(!out.contains("VITA-E2002"), "must parse, not error: {out}");
}

#[test]
fn let_does_not_shadow_function() {
    // Adversarial fix: an illegal co-declaration of `function FOO` and `let FOO`
    // must NOT silently let the `let` shadow the real callable — the function wins.
    let (out, _code) = run("module t;\n\
        logic [7:0] r=0;\n\
        function automatic [7:0] foo(input [7:0] x); foo = x*2; endfunction\n\
        let foo = 8'd99;\n\
        initial begin #1 r = foo(8'd5); $display(\"r=%0d\", r); $finish; end\n\
        endmodule\n");
    // foo(5) must call the function (10), not substitute the let (which is arity-0).
    assert!(
        out.contains("r=10"),
        "function wins over let, foo(5)=10:\n{out}"
    );
}

#[test]
fn scoped_assertoff_is_loud() {
    // Adversarial fix: `$assertoff(level, scope)` (scoped) would silently over-disable
    // (no per-scope grouping) → loud, never a false PASS.
    let (_o, code) = run("module t;\n\
        logic clk=0, a=0, b=0;\n\
        always #5 clk = ~clk;\n\
        assert property(@(posedge clk) a |-> b);\n\
        initial begin $assertoff(1, t); #10 a=1; b=0; #10 $finish; end\n\
        endmodule\n");
    assert_eq!(code, Some(1), "scoped $assertoff is a loud error (exit 1)");
}

#[test]
fn multiterm_crossclock_seg0_now_synthesizes() {
    // Slice A.2: a same-clock MULTI-TERM cross-clock segment 0 (`@(c1) a ##1 b ##1
    // @(c2) c …`, segment 0 = `(a ##1 b)`) now synthesizes its own c1-clocked pipeline
    // instead of loud-rejecting. With the inputs all 0 the antecedent never completes →
    // no violation, but it must NOT emit a loud E3009 either.
    let (o, code) = run("module t;\n\
        logic c1=0, c2=0, a=0, b=0, c=0, q=0;\n\
        always #5 c1 = ~c1;\n\
        always #7 c2 = ~c2;\n\
        assert property(@(posedge c1) a ##1 b ##1 @(posedge c2) c |-> q);\n\
        initial begin #40 $finish; end\n\
        endmodule\n");
    assert!(
        !o.contains("E3009"),
        "multi-term seg-0 must synthesize, not loud:\n{o}"
    );
    assert_eq!(
        code,
        Some(0),
        "no input ever true → no completion → clean:\n{o}"
    );
}

#[test]
fn nested_reclock_in_segment_is_loud() {
    // A NESTED re-clock inside a cross-clock segment (`@(c2)(c ##1 @(c3) d)`) is a 4th
    // clock boundary — still the deferred lane (loud, never a silent miscompile).
    let (o, code) = run("module t;\n\
        logic c1=0, c2=0, c3=0, a=1, c=1, d=1, q=1;\n\
        always #5 c1 = ~c1;\n\
        always #7 c2 = ~c2;\n\
        always #9 c3 = ~c3;\n\
        assert property(@(posedge c1) a ##1 @(posedge c2)(c ##1 @(posedge c3) d) |-> q);\n\
        initial begin #40 $finish; end\n\
        endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "nested re-clock inside a segment is loud:\n{o}"
    );
}

#[test]
fn plus_index_not_mislexed() {
    // Regression: `[+]` is atomic, so a `[+x]` indexed expression is NOT mis-lexed.
    let (out, _code) = run("module t;\n\
        logic [7:0] mem [0:3]; logic [7:0] r;\n\
        integer i;\n\
        initial begin i=2; mem[2]=8'd7; #1 r = mem[+i]; $display(\"r=%0d\", r); $finish; end\n\
        endmodule\n");
    assert!(out.contains("r=7"), "mem[+i] reads mem[2]=7:\n{out}");
}
