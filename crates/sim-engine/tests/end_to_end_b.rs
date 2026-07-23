//! End-to-end sim-engine tests: build a SimIr via the real lex → parse →
//! elaborate pipeline, simulate it, and assert on captured $display output and
//! the generated VCD file.

use sim_engine::{simulate_capture, Backend, FinishReason, SimOpts};

#[path = "end_to_end_util/mod.rs"]
mod util;
#[allow(unused_imports)]
use util::*;

#[test]
fn second_monitor_replaces_first() {
    // Monitor a (=0) at t=0; a→1 at t=10. At t=20 a SECOND $monitor on b (=7)
    // replaces the first. a→2 at t=30 is now invisible. b→8 at t=40 prints.
    let src = "module m; reg [3:0] a; reg [3:0] b; \
               initial begin a=4'd0; b=4'd7; \
                 $monitor(\"a=%0d\", a); \
                 #10 a=4'd1; \
                 #10 $monitor(\"b=%0d\", b); \
                 #10 a=4'd2; \
                 #10 b=4'd8; \
                 #10 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // a establish(0) → a(1) → b establish(7) → [a→2 invisible] → b(8)
    assert_eq!(out, "a=0\na=1\nb=7\nb=8\n");
}

#[test]
fn strobe_then_monitor_ordering_in_one_step() {
    // In a single timestep both a $strobe fires and the monitor changes. Frozen
    // tie-break: strobe line FIRST, then the monitor line.
    let src = "module m; reg clk; reg [3:0] a; \
               always @(posedge clk) $strobe(\"S=%0d\", a); \
               initial begin clk=0; a=4'd0; \
                 $monitor(\"M=%0d\", a); \
                 #5 a=4'd5; clk=1; \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // t=0 postponed: monitor establish prints M=0 (no strobe yet).
    // t=5 postponed: a changed 0→5 AND a strobe fired this step → strobe first
    // (S=5), then monitor (M=5).
    assert_eq!(out, "M=0\nS=5\nM=5\n");
}

#[test]
fn strobe_then_finish_same_step_flushes() {
    // $strobe then $finish in the SAME active region with no intervening delay:
    // P1-6 (IEEE 1364-2005 §5.4/§17): the CURRENT timestep's postponed region is
    // drained BEFORE terminating on $finish, so the strobe prints — matching
    // Icarus/VCS. (The old MVP skipped the flush; that divergence is gone.)
    let src = "module m; reg [3:0] a; \
               initial begin a=4'd3; $strobe(\"s=%0d\", a); $finish; end endmodule";
    let ir = build(src);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "s=3\n", "same-step $strobe must flush before $finish");
}

#[test]
fn strobe_defers_past_later_blocking_writes() {
    // Within one initial block: $strobe(a) is registered while a=1, then a
    // blocking a=2 runs, then $display(a) prints 2. The strobe, deferred to the
    // postponed region, samples the FINAL settled a=2 — proving the strobe reads
    // end-of-timestep state, not the call-site value, even with blocking writes.
    let src = "module m; reg [3:0] a; \
               initial begin a=4'd1; $strobe(\"s=%0d\", a); a=4'd2; \
                 $display(\"d=%0d\", a); #1 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // $display prints d=2 immediately (active region). The strobe flushes at the
    // settle of t=0 (before the #1 advances time) sampling a=2.
    assert_eq!(out, "d=2\ns=2\n");
}

#[test]
fn strobe_monitor_deterministic_repeat() {
    let src = "module m; reg clk; reg [3:0] a; \
               always @(posedge clk) $strobe(\"s=%0d\", a); \
               initial begin clk=0; a=4'd0; \
                 $monitor(\"m=%0d\", a); \
                 #5 a=4'd1; clk=1; \
                 #5 clk=0; #5 a=4'd2; clk=1; \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (_r1, o1) = simulate_capture(&ir, SimOpts::default());
    let (_r2, o2) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(o1, o2, "same SimIr → byte-identical strobe+monitor output");
}

#[test]
fn monitor_reestablish_same_signal_reprints() {
    // Monitor a (=5) → establish prints. a unchanged. Re-issue $monitor on the
    // SAME a: replace semantics reset `last`, so it prints again at that step
    // even though a's value did not change.
    let src = "module m; reg [3:0] a; \
               initial begin a=4'd5; \
                 $monitor(\"a=%0d\", a); \
                 #5 $monitor(\"a=%0d\", a); \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // First establish prints a=5; re-establish resets last_vals=None → prints
    // a=5 again.
    assert_eq!(out, "a=5\na=5\n");
}

#[test]
fn no_arg_monitor_emits_nothing() {
    // A bare `$monitor;` (fmt=None, args=[]) has zero monitored expressions. The
    // flush guard skips it entirely — it must NOT inject a lone "\n" into RTL
    // output at the establishing timestep (or any later step). This pins the
    // deliberate decision from §7.4 / the flush no-arg guard so the output is
    // golden-checked, not emergent.
    //
    // NOTE: depends on elaborate lowering a bare `$monitor;` to a Monitor node
    // with no args. If the front end does not yet emit such a node, gate this test
    // behind the same support; the assertion (empty output) is the contract.
    let src = "module m; reg flag; \
               initial begin flag=0; \
                 $monitor; \
                 #5 flag=1; \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // Zero-expression monitor: no establishment line, no per-step line.
    assert_eq!(
        out, "",
        "no-arg $monitor emits no bytes, not even a newline"
    );
}

#[test]
fn monitor_reprints_on_unknown_to_unknown_value_change() {
    // IEEE-correctness regression for value-level (not rendered-string) change
    // detection. q is a 4-bit reg. Each value is PARTIALLY unknown (known 0s + x),
    // so `%d` renders the uppercase letter `X` (§21.2.1.2) — and 4'b00xx and
    // 4'b0x00 render to the SAME string "X". So a rendered-string diff would
    // suppress the second print; value-level 4-state equality detects the change.
    //
    //   t=0  establish: q = 4'b00xx → "X"   (print)
    //   t=5  q = 4'b0x00           → "X"   (DIFFERENT value, same string → MUST print)
    //   t=10 q = 4'b0x00           → "X"   (unchanged value+string → silent)
    let src = "module m; reg [3:0] q; \
               initial begin q=4'b00xx; \
                 $monitor(\"q=%d\", q); \
                 #5 q=4'b0x00; \
                 #5 q=4'b0x00; \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // Three lines? No — two: establish + the X→X value change. The third step is
    // a true no-op (identical (val,unk) planes) and stays silent. All three
    // render to "q= X" (4-bit %d field width 2, X right-justified); only value-level
    // equality distinguishes them. (iverilog-pinned.)
    assert_eq!(out, "q= X\nq= X\n");
}

// ── FORK 2. join waits for the LATER child (monitor each child) ──────────────
#[test]
fn fork_join_waits_for_all_children() {
    let src = "module m; reg a; reg b; \
               initial begin a=0; b=0; \
                 fork #3 begin b=1; $display(\"b@%0d\", $time); end \
                      #5 begin a=1; $display(\"a@%0d\", $time); end join \
                 $display(\"done@%0d\", $time); \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Quiescent);
    // Concurrent: b@3 first, a@5, then parent done@5. Sequential would give
    // b@3,a@8,done@8. "done@5" FAILS the old path.
    assert_eq!(out, "b@3\na@5\ndone@5\n");
}

// ── FORK 3. join_any unblocks at the FIRST completer, surplus runs on ────────
#[test]
fn fork_join_any_unblocks_at_first() {
    let src = "module m; reg slow; reg fast; \
               initial begin slow=0; fast=0; \
                 fork #5 slow=1; #3 fast=1; join_any \
                 $display(\"resume@%0d fast=%b slow=%b\", $time, fast, slow); \
                 #10 $display(\"late@%0d slow=%b\", $time, slow); \
                 $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // join_any resumes at t=3 (fast done), slow still 0 then. Background #5 sets
    // slow=1 at t=5, observed by the late print at t=13. Sequential lowering has
    // no join_any concept → "resume@3" FAILS the old path.
    assert_eq!(out, "resume@3 fast=1 slow=0\nlate@13 slow=1\n");
}

// ── FORK 4. join_none continues IMMEDIATELY (zero blocking) ──────────────────
#[test]
fn fork_join_none_continues_immediately() {
    // `c` is a vector so the literal 9 is representable (the spec's `reg c` would
    // truncate 9→1 in a 1-bit reg; widen to keep the c=9 observation meaningful).
    let src = "module m; reg a; reg [7:0] c; \
               initial begin a=0; c=0; \
                 fork #5 a=1; join_none \
                 c=9; $display(\"cont@%0d c=%0d a=%b\", $time, c, a); \
                 #6 $display(\"after@%0d a=%b\", $time, a); \
                 $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // join_none → c=9 runs at t=0 (no delay), a still 0. Background child sets
    // a=1 at t=5, observed at t=6. Sequential lowering executes #5 a=1 BEFORE
    // c=9 → "cont@5"/a=1. "cont@0 c=9 a=0" FAILS the old path.
    assert_eq!(out, "cont@0 c=9 a=0\nafter@6 a=1\n");
}

// ── FORK 5. two children write DIFFERENT nets, both visible after join ───────
#[test]
fn fork_join_two_children_different_nets() {
    let src = "module m; reg x; reg y; \
               initial begin x=0; y=0; \
                 fork x=1; y=1; join \
                 $display(\"%b %b\", x, y); $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // Both children zero-delay → complete at t=0; join releases parent at t=0.
    assert_eq!(out, "1 1\n");
    assert_eq!(res.sim_time, 0);
}

// ── FORK 6. nested begin…end inside a fork child (multi-block child chain) ───
#[test]
fn fork_child_with_nested_begin() {
    let src = "module m; reg p; reg q; \
               initial begin p=0; q=0; \
                 fork \
                   begin #2 p=1; #2 p=0; end \
                   #3 q=1; \
                 join \
                 $display(\"%0d %b %b\", $time, p, q); $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // Child-0 chain: p=1@2, p=0@4 (own delays). Child-1: q=1@3. join waits for the
    // later (p=0@4). Parent prints at t=4: p=0,q=1. "4 0 1" FAILS the old path.
    assert_eq!(out, "4 0 1\n");
}

// ── FORK 7. deterministic same-instant ordering: child-0 before child-1 ──────
#[test]
fn fork_same_instant_declaration_order() {
    let src = "module m; integer z; \
               initial begin z=0; \
                 fork $display(\"c0\"); $display(\"c1\"); $display(\"c2\"); join \
                 $display(\"parent\"); $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // All zero-delay, same instant → declaration order c0,c1,c2, then parent.
    assert_eq!(out, "c0\nc1\nc2\nparent\n");
}

// ── FORK 8. same-net last-writer-in-declaration-order wins (documented race) ─
#[test]
fn fork_same_net_last_writer_wins() {
    let src = "module m; reg w; \
               initial begin w=0; \
                 fork w=0; w=1; join \
                 $display(\"%b\", w); $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // Declaration order: child-0 w=0 then child-1 w=1, both at t=0 → w==1.
    assert_eq!(out, "1\n");
}

// ── FORK 9. a child blocks on @event, parent join waits for it ───────────────
#[test]
fn fork_child_waits_on_event() {
    let src = "module m; reg clk; reg got; \
               initial begin clk=0; got=0; \
                 fork \
                   begin @(posedge clk) got=1; $display(\"woke@%0d\", $time); end \
                   #4 clk=1; \
                 join \
                 $display(\"join@%0d got=%b\", $time, got); $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // Child-0 suspends on posedge clk; child-1 drives clk=1 at t=4 → child-0 wakes
    // at t=4, got=1, then join releases parent at t=4. Exercises suspend_on with a
    // CHILD activity id (the collision the scheme fixes).
    assert_eq!(out, "woke@4\njoin@4 got=1\n");
}

// ── FORK 10. parent continuation after join SEES children's net effects ──────
#[test]
fn fork_parent_sees_children_effects() {
    let src = "module m; integer sum; reg d1; reg d2; \
               initial begin sum=0; d1=0; d2=0; \
                 fork #1 d1=1; #2 d2=1; join \
                 if (d1 && d2) sum=42; \
                 $display(\"%0d\", sum); $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // After join (t=2) both d1,d2 are 1 (shared scope) → sum=42. "42" FAILS any
    // path where join releases before all children (would print 0).
    assert_eq!(out, "42\n");
}

// ── FORK 11. empty fork…join resumes immediately (zero children) ─────────────
#[test]
fn fork_join_empty_resumes_immediately() {
    let src = "module m; reg r; \
               initial begin r=0; \
                 fork join \
                 r=1; $display(\"%0d %b\", $time, r); $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // Zero children → barrier (count 0, ALL) fires same instant → r=1 at t=0.
    assert_eq!(out, "0 1\n");
}

// ── FORK 12. join_any leaves the parent runnable while a slow child survives ──
#[test]
fn fork_join_any_surplus_child_survives_to_finish() {
    let src = "module m; reg first; reg second; \
               initial begin first=0; second=0; \
                 fork #2 first=1; #7 second=1; join_any \
                 $display(\"unblock@%0d\", $time); \
                 #10 $display(\"final@%0d first=%b second=%b\", $time, first, second); \
                 $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // join_any unblocks at t=2 (first child). Surplus #7 child survives, sets
    // second=1 at t=7. Final print at t=12 sees both. "unblock@2" FAILS the old.
    assert_eq!(out, "unblock@2\nfinal@12 first=1 second=1\n");
}

// ── FORK 14. monotonic-append identity stability: a top-level edge process keeps
//    firing AFTER a fork appends activities. ──────────────────────────────────
#[test]
fn fork_does_not_disturb_toplevel_edge_process() {
    let src = "module m; reg clk; integer ticks; \
               always @(posedge clk) ticks = ticks + 1; \
               initial begin clk=0; ticks=0; \
                 fork #1 clk=1; #2 clk=0; #3 clk=1; join \
                 $display(\"ticks=%0d\", ticks); $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // The always-block (a top-level EDGE activity armed at t0 into net_to_edge)
    // still fires on each posedge driven by the fork CHILDREN (clk 0→1 at t=1, 0→1
    // at t=3) AFTER the fork appended child activities. Two posedges → ticks=2.
    assert_eq!(out, "ticks=2\n");
}

// ── FORK 15. background join_none child loops forever; parent $finish halts. ──
#[test]
fn fork_join_none_background_child_does_not_block_finish() {
    let src = "module m; reg t; \
               initial begin t=0; \
                 fork begin forever #1 t = ~t; end join_none \
                 #5 $display(\"fin@%0d\", $time); $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    // The forever-looping monitor child keeps the wheel live forever → Quiescent is
    // NEVER reached. The parent's `#5 $finish` is what halts the run. Asserting
    // Finish at t=5 proves: (a) join_none did not block the parent, (b) the
    // background child does not prevent $finish, (c) termination is via $finish.
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(res.sim_time, 5);
    assert_eq!(out, "fin@5\n");
}

// ── determinism regression: FORK 7's source re-run is byte-equal run-to-run. ──
#[test]
fn fork_determinism_regression() {
    let src = "module m; integer z; \
               initial begin z=0; \
                 fork $display(\"c0\"); $display(\"c1\"); $display(\"c2\"); join \
                 $display(\"parent\"); $finish; \
               end endmodule";
    let (ir1, opts1) = build_fork(src);
    let (_r1, o1) = simulate_capture(&ir1, opts1);
    let (ir2, opts2) = build_fork(src);
    let (_r2, o2) = simulate_capture(&ir2, opts2);
    assert_eq!(o1, o2);
    assert_eq!(o1, "c0\nc1\nc2\nparent\n");
}

// ── FORK 17. join_any with TWO children completing at the SAME instant: the
//    parent continuation runs EXACTLY ONCE (the `fired` double-fire guard).
//    (Adversarial-review NIT: previously traced sound but untested.) ──────────
#[test]
fn fork_join_any_same_instant_fires_once() {
    let src = "module m; reg [7:0] a; reg [7:0] b; \
               initial begin a=0; b=0; \
                 fork #3 a=1; #3 b=1; join_any \
                 $display(\"resumed t=%0d\", $time); \
                 #5 $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (_r, out) = simulate_capture(&ir, opts);
    // Both children fire at t=3; a double-fire would print "resumed t=3" twice.
    assert_eq!(out, "resumed t=3\n");
}

// ── FORK 18. two SEQUENTIAL forks in one process use DISTINCT join barriers /
//    join_bb sentinels — the second fork must not satisfy the first's barrier.
//    (Adversarial-review NIT: barrier/sentinel disambiguation, untested.) ─────
#[test]
fn fork_two_sequential_forks_distinct_barriers() {
    let src = "module m; reg [7:0] a; reg [7:0] b; reg [7:0] c; reg [7:0] d; \
               initial begin a=0; b=0; c=0; d=0; \
                 fork #2 a=1; #4 b=1; join \
                 fork #2 c=1; #4 d=1; join \
                 $display(\"a=%0d b=%0d c=%0d d=%0d t=%0d\", a, b, c, d, $time); \
                 $finish; \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (_r, out) = simulate_capture(&ir, opts);
    // First fork joins at t=4 (a,b set); second runs t=4..8 and joins at t=8.
    assert_eq!(out, "a=1 b=1 c=1 d=1 t=8\n");
}

// 1. real division is real: 1.0/3.0 prints 0.333333 via %f
#[test]
fn real_division_is_real() {
    let out =
        run_sv("module t; real r; initial begin r = 1.0 / 3.0; $display(\"%f\", r); end endmodule");
    assert_eq!(out.trim(), "0.333333");
}

// 2. int+real promotion: i/2.0 promotes → 3.5 ; i/2 stays integer 3 (then to real)
#[test]
fn int_real_promotion() {
    let out = run_sv(
        "module t; integer i; real r; \
         initial begin i = 7; r = i / 2.0; $display(\"%g\", r); r = i / 2; $display(\"%g\", r); end \
         endmodule",
    );
    assert_eq!(out.trim(), "3.5\n3");
}

// 3. real→int assignment ROUNDS half-away
#[test]
fn real_to_int_assignment_rounds_half_away() {
    let out = run_sv(
        "module t; real r; integer n; \
         initial begin r = 2.5; n = r; $display(\"%0d\", n); r = -2.5; n = r; $display(\"%0d\", n); end \
         endmodule",
    );
    assert_eq!(out.trim(), "3\n-3");
}

// 4. $rtoi TRUNCATES toward zero (contrast with #3)
#[test]
fn rtoi_truncates_toward_zero() {
    let out = run_sv(
        "module t; real r; integer n; \
         initial begin r = 2.9; n = $rtoi(r); $display(\"%0d\", n); r = -2.9; n = $rtoi(r); $display(\"%0d\", n); end \
         endmodule",
    );
    assert_eq!(out.trim(), "2\n-2");
}

// 5. $itor exact int→real
#[test]
fn itor_converts() {
    let out =
        run_sv("module t; real r; initial begin r = $itor(7); $display(\"%g\", r); end endmodule");
    assert_eq!(out.trim(), "7");
}

// 6. $realtobits / $bitstoreal round-trip is identity
#[test]
fn realtobits_bitstoreal_roundtrip() {
    let out = run_sv(
        "module t; real r; reg [63:0] b; real r2; \
         initial begin r = 3.14159; b = $realtobits(r); r2 = $bitstoreal(b); $display(\"%g\", r2); end \
         endmodule",
    );
    assert_eq!(out.trim(), "3.14159");
}

// 7. $realtime returns a real with fractional time (MVP ratio=1)
#[test]
fn realtime_returns_real() {
    let out = run_sv("module t; initial begin #1 $display(\"%g\", $realtime); end endmodule");
    assert_eq!(out.trim(), "1");
}

// 8. %g shortest formatting (C/LRM): exp(0.00001) = -5 < -4 → "1e-05".
#[test]
fn percent_g_shortest() {
    let out = run_sv(
        "module t; real r; \
         initial begin r = 1500.0; $display(\"%g\", r); r = 0.0001; $display(\"%g\", r); r = 0.00001; $display(\"%g\", r); end \
         endmodule",
    );
    assert_eq!(out.trim(), "1500\n0.0001\n1e-05");
}

// 9. %f vs %e — %e is LRM/printf form: 6 mantissa digits, signed 2-digit exponent.
#[test]
fn percent_f_and_e() {
    let out = run_sv(
        "module t; real r; initial begin r = 1500.0; $display(\"%f|%e\", r, r); end endmodule",
    );
    assert_eq!(out.trim(), "1500.000000|1.500000e+03");
}

// 10. %d on a real ROUNDS half-away
#[test]
fn percent_d_on_real_rounds() {
    let out =
        run_sv("module t; real r; initial begin r = 2.7; $display(\"%0d\", r); end endmodule");
    assert_eq!(out.trim(), "3");
}

// 11. real delay #1.5 rounds to integer ticks; $time after = 2
#[test]
fn real_delay_rounds_to_ticks() {
    let out = run_sv("module t; initial begin #1.5 $display(\"%0d\", $time); end endmodule");
    assert_eq!(out.trim(), "2");
}

// 12. NetKind::Real net round-trips through write/read
#[test]
fn real_net_write_read_roundtrip() {
    let out =
        run_sv("module t; real r; initial begin r = 6.022; $display(\"%g\", r); end endmodule");
    assert_eq!(out.trim(), "6.022");
}

// 13. real comparison: value compare, +0.0 == -0.0
#[test]
fn real_compare_value_semantics() {
    let out = run_sv(
        "module t; real a, b; \
         initial begin a = 0.0; b = -0.0; $display(\"%0d\", (a == b)); a = 1.5; b = 2.5; $display(\"%0d\", (a < b)); end \
         endmodule",
    );
    assert_eq!(out.trim(), "1\n1");
}

// 14. unary minus on real
#[test]
fn real_unary_minus() {
    let out =
        run_sv("module t; real r; initial begin r = -(2.5); $display(\"%g\", r); end endmodule");
    assert_eq!(out.trim(), "-2.5");
}

// 14b. signed-zero display: -0.0 canonicalizes to "0"/"0.000000" across %g AND
// %f/%e, matching iverilog (which flushes a constant/literal -0.0 to +0.0). Prior
// vita kept the sign in %f ("-0.000000") — a $display silent-wrong vs iverilog.
#[test]
fn real_negative_zero_display() {
    let out = run_sv(
        "module t; real r; initial begin r = -(0.0); $display(\"%g|%f\", r, r); end endmodule",
    );
    assert_eq!(out.trim(), "0|0.000000");
}

// 16. %d of a NaN real → "0"; %d of a huge real saturates to i64::MAX.
#[test]
fn percent_d_real_nan_and_huge() {
    let out = run_sv(
        "module t; real r; \
         initial begin r = 0.0/0.0; $display(\"%0d\", r); r = 1.0e30; $display(\"%0d\", r); end \
         endmodule",
    );
    assert_eq!(out.trim(), "0\n9223372036854775807");
}

// 17. real division by zero is ±inf (NOT X), printed as "inf"/"-inf".
#[test]
fn real_div_zero_is_inf() {
    let out = run_sv(
        "module t; real r; initial begin r = 1.0/0.0; $display(\"%g\", r); r = -1.0/0.0; $display(\"%g\", r); end endmodule",
    );
    assert_eq!(out.trim(), "inf\n-inf");
}

#[test]
fn pct_d_of_real_has_no_default_field_width() {
    // A bare `%d`/`%0d` of a REAL is UNPADDED (iverilog prints the rounded value,
    // no default field width: 3.7→4, -2.9→-3). An explicit `%Nd`/`%-Nd`/`%0Nd`
    // still pads. An INTEGER `%d` keeps its default decimal field width (an 8-bit
    // reg → width 3, so 42 → " 42"). iverilog-13.0-pinned.
    let out = run_sv(
        "module t; real r; reg [7:0] b; initial begin \
         r = 3.7; b = 8'd42; \
         $display(\"[%d][%0d][%6d][%-6d][%06d]\", r, r, r, r, r); \
         $display(\"int[%d] real[%d]\", b, r); \
         r = -2.9; $display(\"neg[%d][%5d]\", r, r); \
         end endmodule",
    );
    assert_eq!(
        out,
        "[4][4][     4][4     ][000004]\nint[ 42] real[4]\nneg[-3][   -3]\n"
    );
}

// 18b. `$clog2(real)` rounds the real to the nearest integer (ties away from zero),
// then computes clog2 — iverilog accepts a real arg as arithmetic (unlike the
// bit-query siblings $countones/$onehot, which stay loud). Reading the IEEE-754 bit
// pattern directly was silently wrong (100.0 → 63). A negative / non-finite "size"
// has no meaningful clog2 → X, never a confident wrong number. iverilog-13.0-pinned.
#[test]
fn clog2_of_real_rounds_then_counts() {
    let out = run_sv(
        "module t; real r; initial begin \
         $display(\"%0d %0d %0d %0d\", $clog2(100.0), $clog2(7.0), $clog2(1.5), $clog2(0.5)); \
         $display(\"%0d %0d %0d\", $clog2(2.5), $clog2(4.5), $clog2(8.5)); \
         r = 5000000000.0; $display(\"%0d\", $clog2(r)); \
         $display(\"%0d %0d\", $clog2(16), $clog2(17)); \
         r = -2.0; $display(\"%0d\", $clog2(r)); \
         r = 18446744073709549568.0; $display(\"%0d\", $clog2(r)); \
         r = 18446744073709551616.0; $display(\"%0d\", $clog2(r)); \
         end endmodule",
    );
    // 100→7  7→3  1.5→2→1  0.5→1→0 | 2.5→3→2  4.5→5→3  8.5→9→4 | 5e9→33 | 16→4 17→5
    // | −2.0→X | 2^64−2048 (largest f64 < 2^64) → 64 | exactly 2^64 → X (out of u64,
    // must not wrap to a confident 0).
    assert_eq!(out, "7 3 1 0\n2 3 4\n33\n4 5\nx\n64\nx\n");
}

// 18c. `%s` of a NUMERIC const renders every 0x00 byte as a space (iverilog rule),
// across the full reg width and with no trailing-NUL stripping — byte-identical to
// the runtime-value path (fmt_packed_chars). Previously a numeric const routed
// through const_string (strip trailing NUL, emit embedded NUL literally). Explicit-
// width `%0s`/`%Ns` strip leading NUL padding; bare `%s` pads with spaces; `%-Ns`
// left-justifies in the reg byte width. iverilog-13.0-pinned (verified via hexdump).
#[test]
fn pct_s_of_numeric_const_maps_nul_to_space() {
    let out = run_sv(
        "module t; initial begin \
         $write(\"[%s][%s][%s][%s][%s][%s][%s]\\n\", \
           16'h0041, 16'h4100, 24'h004241, 24'h410042, 8'h00, 32'h00004241, 16'h4142); \
         $display(\"|%0s|%5s|%-5s|\", 16'h0041, 16'h0041, 16'h0041); \
         end endmodule",
    );
    // 0x00→space, full width: 16'h0041→" A", 16'h4100→"A ", 24'h004241→" BA",
    // 24'h410042→"A B", 8'h00→" ", 32'h00004241→"  BA", 16'h4142→"AB" (no NUL).
    // %0s strips leading NUL→"A"; %5s→"    A"; %-5s→"A    ".
    assert_eq!(out, "[ A][A ][ BA][A B][ ][  BA][AB]\n|A|    A|A    |\n");
}

// 18d. A bare string-VARIABLE argument (with no leading format string) is itself a
// format template (IEEE 1364-2005 §17.1): its `%` specs consume the following args,
// exactly like a string literal. Previously a runtime `string` printed as a
// packed-ASCII decimal (e.g. "hello" → 448378203247). iverilog-13.0-pinned.
#[test]
fn bare_string_var_is_a_format_template() {
    let out = run_sv(
        "module t; string s; string f; initial begin \
         s = \"hello\"; $write(\"[\"); $write(s); $write(\"]\\n\"); \
         f = \"x=%0d b=%0d\"; $display(f, 7, 9); \
         f = \"50%%done\"; $display(f); \
         s = \"\"; $write(\"E[\"); $write(s); $write(\"]\\n\"); \
         end endmodule",
    );
    // $write(s) prints text; a string var with % specs consumes following args;
    // %% → literal %; an empty string var prints nothing.
    assert_eq!(out, "[hello]\nx=7 b=9\n50%done\nE[]\n");
}

// 19. [P4 · backend seam] Backend selection rides out-of-band on SimOpts. The
// Bytecode backend currently falls back to the interpreter for every body
// (Stage B: no VM yet), so it is byte-identical to Interpreter — stdout AND the
// SimResult summary. When Stage C lands the VM, this same assertion becomes the
// meaningful equivalence check (subsumed by the P5 differential gate, which adds
// VCD-byte comparison + a corpus).
#[test]
fn backend_bytecode_falls_back_byte_identical() {
    let ir = build(
        "module t; reg [3:0] c; integer k; \
         initial begin c = 0; for (k = 0; k < 5; k = k + 1) #1 c = c + 1; \
         $display(\"%0d\", c); $finish; end endmodule",
    );
    let (r_i, out_i) = simulate_capture(
        &ir,
        SimOpts {
            backend: Backend::Interpreter,
            ..Default::default()
        },
    );
    let (r_b, out_b) = simulate_capture(
        &ir,
        SimOpts {
            backend: Backend::Bytecode,
            ..Default::default()
        },
    );
    assert_eq!(out_i.trim(), "5", "interpreter sanity");
    assert_eq!(out_i, out_b, "stdout must match across backends");
    assert_eq!(r_i.sim_time, r_b.sim_time);
    assert_eq!(r_i.finish_reason, r_b.finish_reason);
    assert_eq!(r_i.exit_class, r_b.exit_class);
}

// ════════════════════════════════════════════════════════════════════════════
// REAL-DESIGN CORPUS — representative RTL patterns through the full pipeline.
// Each is a self-checking testbench; the asserted $display output is the golden.
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ansi_port_multiname_shares_range() {
    // REMAINING_WORK (corpus-found defect): `input [7:0] a, b` makes BOTH a and b
    // 8-bit — the range/type is inherited by the comma-continued name, not just the
    // direction. Was truncating b to a scalar (b=3 read as 1).
    let src = "module m(input [7:0] a, b, output [7:0] y); assign y = a + b; endmodule \
               module tb; wire [7:0] z; m u(8'd200, 8'd55, z); \
                 initial begin #1 $display(\"%0d\", z); $finish; end endmodule";
    let (_r, out) = simulate_capture(&build(src), SimOpts::default());
    assert_eq!(out, "255\n"); // 200 + 55, both 8-bit (would truncate if b were scalar)
}

#[test]
fn corpus_alu_combinational() {
    let src = "module alu(input [7:0] a, b, input [1:0] op, output reg [7:0] y); \
                 always @* case (op) 2'd0: y=a+b; 2'd1: y=a-b; 2'd2: y=a&b; 2'd3: y=a|b; endcase \
               endmodule \
               module tb; reg [7:0] a, b; reg [1:0] op; wire [7:0] y; alu u(a, b, op, y); \
                 initial begin a=8'd10; b=8'd3; \
                   op=2'd0; #1 $display(\"%0d\", y); op=2'd1; #1 $display(\"%0d\", y); \
                   op=2'd2; #1 $display(\"%0d\", y); op=2'd3; #1 $display(\"%0d\", y); \
                   $finish; end endmodule";
    let (_r, out) = simulate_capture(&build(src), SimOpts::default());
    assert_eq!(out, "13\n7\n2\n11\n"); // 10+3, 10-3, 10&3, 10|3
}

#[test]
fn corpus_shift_register() {
    let src = "module tb; reg [7:0] sr; integer i; \
               initial begin sr = 8'b0000_0001; for (i=0;i<3;i=i+1) sr = sr << 1; \
                 $display(\"%0d\", sr); $finish; end endmodule";
    let (_r, out) = simulate_capture(&build(src), SimOpts::default());
    assert_eq!(out, "8\n"); // 1 << 3
}

#[test]
fn corpus_fsm_modular_state() {
    let src = "module tb; reg [1:0] state; integer i; \
               initial begin state = 2'd0; \
                 for (i=0;i<5;i=i+1) state = (state==2'd2) ? 2'd0 : state + 2'd1; \
                 $display(\"%0d\", state); $finish; end endmodule";
    let (_r, out) = simulate_capture(&build(src), SimOpts::default());
    assert_eq!(out, "2\n"); // 0→1→2→0→1→2
}

#[test]
fn corpus_memory_accumulate() {
    let src = "module tb; reg [7:0] mem[0:7]; integer i; reg [7:0] sum; \
               initial begin for (i=0;i<8;i=i+1) mem[i] = i*2; \
                 sum = 0; for (i=0;i<8;i=i+1) sum = sum + mem[i]; \
                 $display(\"%0d\", sum); $finish; end endmodule";
    let (_r, out) = simulate_capture(&build(src), SimOpts::default());
    assert_eq!(out, "56\n"); // 2*(0+1+…+7)
}

#[test]
fn corpus_clocked_dff_hierarchy() {
    let src = "module dff(input clk, d, output reg q); always @(posedge clk) q <= d; endmodule \
               module tb; reg clk, d; wire q; dff u(clk, d, q); \
                 initial begin clk=0; d=1; #5 clk=1; #5 clk=0; $display(\"%0d\", q); $finish; end \
               endmodule";
    let (_r, out) = simulate_capture(&build(src), SimOpts::default());
    assert_eq!(out, "1\n"); // posedge samples d=1
}

#[test]
fn corpus_counter_with_reset() {
    let src = "module counter(input clk, rst, output reg [3:0] cnt); \
                 always @(posedge clk) if (rst) cnt <= 4'd0; else cnt <= cnt + 4'd1; \
               endmodule \
               module tb; reg clk, rst; wire [3:0] cnt; counter c(clk, rst, cnt); integer k; \
                 initial begin clk=0; rst=1; #5 clk=1; #5 clk=0; rst=0; \
                   for (k=0;k<5;k=k+1) begin #5 clk=1; #5 clk=0; end \
                   $display(\"%0d\", cnt); $finish; end endmodule";
    let (_r, out) = simulate_capture(&build(src), SimOpts::default());
    assert_eq!(out, "5\n"); // reset, then 5 increments
}

#[test]
fn packed_2d_element_rw() {
    // `logic [3:0][7:0] m` is a 4×8 = 32-bit packed array; m[i] is an 8-bit slice
    // (bits [i*8 +: 8]). Fill each element and read back.
    let src = "module t; logic [3:0][7:0] m; integer i; \
               initial begin for (i=0;i<4;i=i+1) m[i] = i*16 + i; \
                 $display(\"%0h %0h %0h %0h\", m[0], m[1], m[2], m[3]); $finish; end endmodule";
    let (_r, out) = simulate_capture(&build(src), SimOpts::default());
    assert_eq!(out, "0 11 22 33\n"); // i*17
}

#[test]
fn packed_2d_ansi_port() {
    // A packed multi-dim ANSI PORT `input [1:0][7:0] m` — the submodule reads element
    // slices. (Exercises the port-net path, not just body decls.)
    let src = "module sub(input [1:0][7:0] m, output [7:0] y); assign y = m[1]; endmodule \
               module tb; wire [7:0] z; sub u(16'hABCD, z); \
                 initial begin #1 $display(\"%0h\", z); $finish; end endmodule";
    let (_r, out) = simulate_capture(&build(src), SimOpts::default());
    assert_eq!(out, "ab\n"); // m = ABCD; m[1] = high byte AB
}

#[test]
fn packed_2d_bit_select() {
    // m[i][j] = bit j of the 8-bit element i.
    let src = "module t; logic [3:0][7:0] m; \
               initial begin m[1] = 8'hAB; m[1][0] = 1'b0; \
                 $display(\"%0h %0d %0d\", m[1], m[1][0], m[1][7]); $finish; end endmodule";
    let (_r, out) = simulate_capture(&build(src), SimOpts::default());
    // 0xAB with bit0 cleared → 0xAA; bit0=0, bit7=1.
    assert_eq!(out, "aa 0 1\n");
}
