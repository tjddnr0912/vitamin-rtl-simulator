//! End-to-end sim-engine tests: build a SimIr via the real lex → parse →
//! elaborate pipeline, simulate it, and assert on captured $display output and
//! the generated VCD file.

use sim_engine::{simulate, simulate_capture, FinishReason, SimOpts};

#[path = "end_to_end_util/mod.rs"]
mod util;
#[allow(unused_imports)]
use util::*;

#[test]
fn timescale_delay_scales_to_precision_ticks() {
    // `timescale 1ns/1ps` → tick = 1ps, multiplier 1000. `#5` then `#2` advance
    // 5000 + 2000 = 7000 ticks (not 7). Proves elaborate scales `#delay`.
    let (ir, opts) = build_timescaled(
        "`timescale 1ns/1ps\nmodule top; initial begin #5; #2; $finish; end endmodule\n",
    );
    let (res, _out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(res.sim_time, 7000);
}

#[test]
fn timescale_fractional_delay_exact() {
    // Fractional `#2.5` in 1ns/1ps (M=1000) = exactly 2500 ticks (round(2.5*1000)),
    // NOT round(2.5)*1000 = 3000. The multiply is inside the rounding.
    let (ir, opts) = build_timescaled(
        "`timescale 1ns/1ps\nmodule top; initial begin #2.5; $finish; end endmodule\n",
    );
    let (res, _out) = simulate_capture(&ir, opts);
    assert_eq!(res.sim_time, 2500);
}

/// A continuous-assign delay wider than `u32` used to WRAP, firing early and
/// silently. `assign #5000000000` under 1ns/1ns fired at t=705032704
/// (= 5e9 mod 2^32, 7.09x early) with `errors=0`; iverilog never fires it within
/// the run. The elaborate-time fold took the low 32 bits of the literal before
/// the saturation its own two sibling branches already applied.
///
/// Oracle: iverilog 13 prints `w=xx` at every probe point, POST matches.
#[test]
fn ca_delay_wider_than_u32_does_not_wrap() {
    let (ir, opts) = build_timescaled(
        "`timescale 1ns/1ns\n\
         module top; reg [7:0] src; wire [7:0] w;\n\
           assign #5000000000 w = src;\n\
           initial begin src = 8'h5a;\n\
             #705032704 $display(\"A w=%0h\", w);\n\
             #100000    $display(\"B w=%0h\", w);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    let (_res, out) = simulate_capture(&ir, opts);
    // The WRAPPED delay would have landed exactly at the first probe.
    assert!(
        out.contains("A w=xx") && out.contains("B w=xx"),
        "a >u32 CA delay fired early (wrap regression): {out}"
    );
}

/// A NEGATIVE real delay used to fire immediately while a negative INTEGER delay
/// never fired — one function, two answers. iverilog fires neither. But the
/// boundary is the ROUNDED value, not the raw one: `#(-1e-9)` rounds to zero and
/// iverilog DOES fire it at 0, so a sign test on the product overshoots.
///
/// Three-way pinned (vita/PRE/iverilog) over 50 designs x 5 timescales.
#[test]
fn negative_delays_never_fire_but_tiny_negatives_round_to_zero() {
    let (ir, opts) = build_timescaled(
        "module top; real r; integer n; reg [7:0] a;\n\
           initial begin a = 0; r = -1.0; n = -3;\n\
             fork begin #(r)     a = 1; end join_none\n\
             fork begin #(-0.5)  a = 2; end join_none\n\
             fork begin #(n)     a = 3; end join_none\n\
             fork begin #(-7)    a = 4; end join_none\n\
             #50 $display(\"NEG a=%0d\", a);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    let (_res, out) = simulate_capture(&ir, opts);
    assert!(out.contains("NEG a=0"), "a negative delay fired: {out}");

    // …and the rounds-to-zero case still fires, which a raw-sign test would kill.
    let (ir2, opts2) = build_timescaled(
        "module top; real r; reg [7:0] a;\n\
           initial begin a = 0; r = -1e-9;\n\
             fork begin #(r) a = 1; #3 a = 2; end join_none\n\
             #50 $display(\"TINY a=%0d\", a);\n\
             $finish;\n\
           end\n\
         endmodule\n",
    );
    let (_res2, out2) = simulate_capture(&ir2, opts2);
    assert!(
        out2.contains("TINY a=2"),
        "a tiny negative delay that rounds to zero must still fire (iverilog does): {out2}"
    );
}

#[test]
fn timescale_default_is_1ns_1ns() {
    // No `timescale → 1ns/1ns base, multiplier 1: `#5` advances 5 ticks (unchanged).
    let (ir, opts) = build_timescaled("module top; initial begin #5; $finish; end endmodule\n");
    let (res, _out) = simulate_capture(&ir, opts);
    assert_eq!(res.sim_time, 5);
}

#[test]
fn timescale_time_and_realtime_scaled() {
    // doc-08 example: 1ns/1ps, after #2.5 (= 2500 ticks) → $time = 3 ($time ROUNDS
    // the 2.5ns module-unit time to nearest per IEEE 1800-2017 §20.3.1 — 2.5
    // half-up → 3, NOT truncated to 2; iverilog-13.0-pinned), $realtime = 2.5
    // (sub-unit fraction kept).
    let (ir, opts) = build_timescaled(
        "`timescale 1ns/1ps\nmodule top; initial begin #2.5; \
         $display(\"%0d %g\", $time, $realtime); $finish; end endmodule\n",
    );
    let (_res, out) = simulate_capture(&ir, opts);
    assert_eq!(out, "3 2.5\n");
}

#[test]
fn timescale_time_default_unscaled() {
    // No timescale → M=1, $time == raw tick.
    let (ir, opts) = build_timescaled(
        "module top; initial begin #5; $display(\"%0d\", $time); $finish; end endmodule\n",
    );
    let (_res, out) = simulate_capture(&ir, opts);
    assert_eq!(out, "5\n");
}

#[test]
fn timescale_rounding_doc08_case1() {
    // doc-08 §정밀도 회귀 case 1: 1ns/100ps (M=10), round-half-away. #1.44→14.4→14
    // ticks (1400ps); #0.05→0.5→1 tick (1500ps); #0.04→0.4→0 ticks (no advance).
    // Total advanced time = 15 ticks (= 1.5ns). DELAY rounding (→15 ticks) and
    // $time rounding are distinct: $time ROUNDS 1.5→2 (half-up, IEEE §20.3.1 —
    // NOT truncated to 1; iverilog-13.0-pinned $time=2, $realtime=1.5).
    let (ir, opts) = build_timescaled(
        "`timescale 1ns/100ps\nmodule top; reg a; initial begin \
         a=0; #1.44 a=1; #0.05 a=0; #0.04 a=1; $display(\"%0d\", $time); $finish; end endmodule\n",
    );
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.sim_time, 15);
    assert_eq!(out, "2\n");
}

#[test]
fn timescale_mixed_modules_global_min_precision() {
    // doc-08 case 2 idea: two modules with different timescales share the design-wide
    // finest tick (100ps). fast (1ns/100ps, M=10) `#2.5`→25 ticks; slow (1us/10ns,
    // M=10^(-6 − -10)=10^4) `#1`→10000 ticks. The later $finish bounds the run.
    let (ir, opts) = build_timescaled(
        "`timescale 1ns/100ps\nmodule fast; reg q; initial begin #2.5 q=1; end endmodule\n\
         `timescale 1us/10ns\nmodule slow; reg r; initial begin #1 r=1; $display(\"%0d\", $time); $finish; end endmodule\n\
         `timescale 1ns/100ps\nmodule top; fast f(); slow s(); initial #20000 ; endmodule\n",
    );
    let (res, out) = simulate_capture(&ir, opts);
    // slow's #1 = 1us = 10000 ticks of 100ps; slow $time = 10000 / 10^4 = 1.
    assert_eq!(res.sim_time, 10000);
    assert_eq!(out, "1\n");
}

// ── 1. combinational assign y = a & b ──────────────────────────────────────

#[test]
fn comb_and_writes_correct_value() {
    let src = "module m; reg a; reg b; wire y; \
               assign y = a & b; \
               initial begin a = 1'b1; b = 1'b1; #1 $finish; end endmodule";
    let ir = build(src);
    let (res, _out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // After a=1,b=1 settle, y must be 1. We re-check via a $display variant below;
    // here we just assert the run finished cleanly at t>=1.
    assert!(res.sim_time >= 1);
}

#[test]
fn comb_and_display_truth() {
    // Drive all 4 input combos and print y each time.
    let src = "module m; reg a; reg b; wire y; \
               assign y = a & b; \
               initial begin \
                 a=0; b=0; #1 $display(\"%b\", y); \
                 a=0; b=1; #1 $display(\"%b\", y); \
                 a=1; b=0; #1 $display(\"%b\", y); \
                 a=1; b=1; #1 $display(\"%b\", y); \
                 $finish; \
               end endmodule";
    let ir = build(src);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "0\n0\n0\n1\n", "AND truth table via continuous assign");
}

// ── 2. flip-flop q <= d on posedge clk ─────────────────────────────────────

#[test]
fn flipflop_follows_d_after_edge() {
    let src = "module m; reg clk; reg d; reg q; \
               always @(posedge clk) q <= d; \
               initial begin \
                 clk=0; d=1; q=0; \
                 #5 $display(\"before %b\", q); \
                 clk=1; \
                 #1 $display(\"after %b\", q); \
                 $finish; \
               end endmodule";
    let ir = build(src);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // q is 0 before the edge, follows d (=1) after the posedge.
    assert_eq!(out, "before 0\nafter 1\n");
}

// ── 3. initial begin a=1; #5 a=0; $finish advances time to 5 ───────────────

#[test]
fn delay_advances_time_and_finish_stops() {
    let src = "module m; reg a; \
               initial begin a=1; #5 a=0; $finish; end endmodule";
    let ir = build(src);
    let sink = DiagSink::default();
    let res = simulate(&ir, &sink, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(
        res.sim_time, 5,
        "time advanced to the #5 delay before $finish"
    );
}

// ── 4. $display formatting (%d %h %b %0d) ──────────────────────────────────

#[test]
fn display_format_specifiers() {
    let src = "module m; reg [7:0] v; \
               initial begin v = 8'd171; \
                 $display(\"d=%d h=%h b=%b z=%0d\", v, v, v, v); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // 171 = 0xAB = 0b10101011
    assert_eq!(out, "d=171 h=ab b=10101011 z=171\n");
}

// ── 5. NBA ordering: b<=a; c<=b gives OLD b ────────────────────────────────

#[test]
fn nba_uses_sampled_rhs() {
    // At the single posedge: a=5, b=0, c=0. NBA samples RHS (a→b gets 5, b→c
    // gets OLD b=0). So after edge: b=5, c=0 (NOT 5).
    let src = "module m; reg clk; reg [3:0] a; reg [3:0] b; reg [3:0] c; \
               always @(posedge clk) begin b <= a; c <= b; end \
               initial begin \
                 clk=0; a=4'd5; b=4'd0; c=4'd0; \
                 #5 clk=1; \
                 #1 $display(\"b=%0d c=%0d\", b, c); \
                 $finish; \
               end endmodule";
    let ir = build(src);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "b=5 c=0\n", "NBA samples old b for c");
}

#[test]
fn nba_shifts_one_stage_per_clock() {
    // Two clock pulses: stage propagates one step each posedge.
    // a=7 constant. b<=a; c<=b.
    // After 1st posedge: b=7, c=0.  After 2nd posedge: b=7, c=7.
    let src = "module m; reg clk; reg [3:0] a; reg [3:0] b; reg [3:0] c; \
               always @(posedge clk) begin b <= a; c <= b; end \
               initial begin \
                 clk=0; a=4'd7; b=4'd0; c=4'd0; \
                 #5 clk=1; #5 clk=0; \
                 #1 $display(\"p1 b=%0d c=%0d\", b, c); \
                 #4 clk=1; #5 clk=0; \
                 #1 $display(\"p2 b=%0d c=%0d\", b, c); \
                 $finish; \
               end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "p1 b=7 c=0\np2 b=7 c=7\n");
}

// ── 6. if/else branch + arithmetic ─────────────────────────────────────────

#[test]
fn if_else_and_arithmetic() {
    let src = "module m; reg [3:0] a; reg [3:0] b; reg [3:0] r; \
               initial begin a=4'd6; b=4'd3; \
                 if (a > b) r = a - b; else r = b - a; \
                 $display(\"%0d\", r); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "3\n"); // 6 > 3 → r = 6-3 = 3
}

#[test]
fn else_branch_taken() {
    let src = "module m; reg [3:0] a; reg [3:0] b; reg [3:0] r; \
               initial begin a=4'd2; b=4'd9; \
                 if (a > b) r = a - b; else r = b - a; \
                 $display(\"%0d\", r); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "7\n"); // 2 > 9 false → r = 9-2 = 7
}

// ── 7. VCD output: $dumpfile + $dumpvars writes a value change ─────────────

#[test]
fn vcd_dump_initial_and_change() {
    let path = tmp_vcd("dump");
    let _ = std::fs::remove_file(&path);
    let src = "module m; reg [3:0] a; \
               initial begin $dumpfile(\"ignored.vcd\"); $dumpvars(0, m); \
                 a=4'd3; #5 a=4'd9; #5 $finish; end endmodule";
    let ir = build(src);
    let (res, _out) = simulate_capture(&ir, opts_with_vcd(&path));
    assert_eq!(res.finish_reason, FinishReason::Finish);
    let vcd = std::fs::read_to_string(&path).expect("vcd written");
    // header, dumpvars, a var declared as n0, and value changes for 3 then 9.
    assert!(vcd.contains("$dumpvars"), "has dumpvars block");
    // `reg [3:0]` carries its `[3:0]` bit-range reference (IEEE 1364 §18.2.3.2;
    // iverilog-pinned — lets a viewer label ascending/non-zero-base bits right).
    assert!(
        vcd.contains("$var reg 4 ! n0 [3:0] $end"),
        "net declared with range:\n{vcd}"
    );
    assert!(vcd.contains("b0011 !"), "a=3 appears:\n{vcd}");
    assert!(vcd.contains("b1001 !"), "a=9 appears:\n{vcd}");
    assert!(vcd.contains("#5"), "time 5 recorded");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn vcd_hierarchical_scopes_and_real_names() {
    // REMAINING_WORK BLOCKER: VCD emits real hierarchical $scope (by instance name)
    // and real $var names, not a flat `top` scope with synthetic n0..nN.
    let path = tmp_vcd("hier");
    let _ = std::fs::remove_file(&path);
    let src = "module sub(input wire a, output wire b); assign b = ~a; endmodule \
               module top; reg clk; wire q; sub u(.a(clk), .b(q)); \
                 initial begin $dumpfile(\"x\"); $dumpvars(0, top); \
                   clk=0; #5 clk=1; #5 $finish; end endmodule";
    let (ir, names) = build_named(src);
    let mut opts = opts_with_vcd(&path);
    opts.net_names = names;
    let (res, _out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    let vcd = std::fs::read_to_string(&path).expect("vcd written");
    assert!(vcd.contains("$scope module top $end"), "top scope:\n{vcd}");
    assert!(
        vcd.contains("$scope module u $end"),
        "sub instance scope:\n{vcd}"
    );
    assert!(vcd.contains(" clk $end"), "real name clk:\n{vcd}");
    assert!(
        !vcd.contains(" n0 $end") && !vcd.contains(" n1 $end"),
        "no synthetic names:\n{vcd}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn vcd_dumpvars_declares_memory_array() {
    // Phase-1.x ⑤: $dumpvars of a design with a memory declares one $var PER
    // ELEMENT (`mem[0]`..`mem[3]`) — v1 used to declare a single word-0 var.
    let path = tmp_vcd("memdump");
    let src = "module top; reg [7:0] mem[0:3]; reg [3:0] a; \
               initial begin $dumpfile(\"x\"); $dumpvars(0, top); \
                 a = 4'd5; mem[1] = 8'hAB; #5 $finish; end endmodule";
    let (ir, names) = build_named(src);
    let mut opts = opts_with_vcd(&path);
    opts.net_names = names;
    let _ = simulate_capture(&ir, opts);
    let vcd = std::fs::read_to_string(&path).expect("vcd");
    for k in 0..4 {
        // each element is a `reg [7:0]` word → carries its `[7:0]` bit range.
        assert!(
            vcd.contains(&format!("mem[{k}] [7:0] $end")),
            "element {k} declared with range:\n{vcd}"
        );
    }
    assert!(
        !vcd.contains(" mem $end") && !vcd.contains(" mem [7:0] $end"),
        "the old single word-0 var must be gone:\n{vcd}"
    );
    assert!(
        vcd.contains(" a [3:0] $end"),
        "vector net declared with range:\n{vcd}"
    );
    // mem[1] = 8'hAB must land on element 1's id (10101011).
    assert!(vcd.contains("b10101011 "), "element change record:\n{vcd}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn vcd_golden_byte_exact() {
    // Byte-exact golden of the full VCD (cross-OS determinism + format regression).
    // Only the $version block is version-dependent and is stripped before compare.
    let path = tmp_vcd("golden");
    let src = "module top; reg [3:0] a; initial begin $dumpfile(\"x\"); $dumpvars(0, top); \
               a = 4'd5; #5 a = 4'd9; #5 $finish; end endmodule";
    let (ir, names) = build_named(src);
    let mut opts = opts_with_vcd(&path);
    opts.net_names = names;
    let _ = simulate_capture(&ir, opts);
    let vcd = strip_version_block(&std::fs::read_to_string(&path).expect("vcd"));
    let golden =
        "$date\n   vitamin-sim\n$end\n$comment\n   Generated by vitamin RTL simulator\n$end\n\
                  $timescale 1ns $end\n$scope module top $end\n$var reg 4 ! a [3:0] $end\n$upscope $end\n\
                  $enddefinitions $end\n$dumpvars\nbxxxx !\n$end\n#0\nb0101 !\n#5\nb1001 !\n";
    assert_eq!(vcd, golden, "golden VCD drift");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn vcd_var_reference_carries_declared_range() {
    // IEEE 1364-2005 §18.2.3.2: a vector `$var` reference carries `[msb:lsb]` so a
    // viewer labels an ascending `[0:3]` or non-zero-base `[7:4]` vector correctly
    // instead of defaulting to `[width-1:0]`. Was silently omitted, collapsing
    // `[0:3]` and `[3:0]` to the indistinguishable `reg 4` (iverilog-pinned:
    // iverilog emits the range for each vector; a scalar and a real carry none).
    let path = tmp_vcd("varrange");
    // b5 = a 1-bit vector at a NON-ZERO index (records `[5:5]`, iverilog-pinned);
    // z = `[0:0]` and sc = a true scalar both collapse to a bare name; pm = a
    // packed multi-dim whose OUTER dim is ascending — VCD flattens it to the
    // `[width-1:0]` flat range `[31:0]` (iverilog-pinned), NOT the stale outer
    // `[31:3]` that elaborate's inconsistent NetVar.msb/lsb would otherwise give.
    let src = "module m; reg [3:0] d; reg [0:3] asc; reg [7:4] hi; reg sc; \
               reg [5:5] b5; reg [0:0] z; logic [0:3][7:0] pm; real r; \
               initial begin $dumpfile(\"x\"); $dumpvars(0, m); \
                 d=1; asc=2; hi=3; sc=1; b5=1; z=1; pm=32'h1; r=1.5; #1 $finish; end endmodule";
    let (ir, names) = build_named(src);
    let mut opts = opts_with_vcd(&path);
    opts.net_names = names;
    let _ = simulate_capture(&ir, opts);
    let vcd = std::fs::read_to_string(&path).expect("vcd");
    assert!(vcd.contains(" d [3:0] $end"), "descending [3:0]:\n{vcd}");
    assert!(
        vcd.contains(" pm [31:0] $end"),
        "packed multi-dim flattens to [31:0], not the stale [31:3]:\n{vcd}"
    );
    assert!(
        vcd.contains(" b5 [5:5] $end"),
        "single-bit non-zero index keeps [5:5]:\n{vcd}"
    );
    assert!(
        vcd.contains(" z $end"),
        "[0:0] collapses to no range:\n{vcd}"
    );
    assert!(!vcd.contains(" z ["), "[0:0] must NOT get a range:\n{vcd}");
    assert!(
        vcd.contains(" asc [0:3] $end"),
        "ascending [0:3] preserved:\n{vcd}"
    );
    assert!(
        vcd.contains(" hi [7:4] $end"),
        "non-zero base [7:4]:\n{vcd}"
    );
    assert!(vcd.contains(" sc $end"), "scalar carries no range:\n{vcd}");
    // leading space so the `asc [0:3]` line (ends in "sc [") is not a false match.
    assert!(
        !vcd.contains(" sc ["),
        "scalar must NOT get a range:\n{vcd}"
    );
    assert!(vcd.contains(" r $end"), "real carries no range:\n{vcd}");
    assert!(
        !vcd.contains(" r ["),
        "real (dimensionless) must NOT get a range:\n{vcd}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn vcd_dumps_xz_and_real_values() {
    // VCD VALUE dumps for 4-state (x/z) and real signals. The decoded waveform
    // matches iverilog 13.0 (verified by a left-extend + last-write-wins decoder:
    // vita writes the full-width form `b10xz01xz` / `bzzzz` where iverilog uses
    // the leading-redundant-strip form `bx`/`bz`, but both decode identically).
    // This pins the value-line content so a future value-formatting change (e.g.
    // adding VCD compression) is a conscious, reviewed golden update.
    let path = tmp_vcd("xzreal");
    let src = "module m; reg [7:0] v; real r; reg [3:0] z; \
               initial begin $dumpfile(\"x\"); $dumpvars(0, m); \
                 v = 8'b10xz_01xz; r = -2.5; z = 4'bzzzz; #1 $finish; end endmodule";
    let (ir, names) = build_named(src);
    let mut opts = opts_with_vcd(&path);
    opts.net_names = names;
    let _ = simulate_capture(&ir, opts);
    let vcd = std::fs::read_to_string(&path).expect("vcd");
    assert!(
        vcd.contains("b10xz01xz "),
        "mixed 4-state value dumped:\n{vcd}"
    );
    assert!(vcd.contains("bzzzz "), "all-z value dumped:\n{vcd}");
    assert!(
        vcd.contains("r-2.5 "),
        "real value dumped in r-format:\n{vcd}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn vcd_clock_toggles_recorded() {
    let path = tmp_vcd("clk");
    let _ = std::fs::remove_file(&path);
    let src = "module m; reg clk; \
               initial begin $dumpfile(\"x\"); $dumpvars; clk=0; \
                 #5 clk=1; #5 clk=0; #5 clk=1; #5 $finish; end endmodule";
    let ir = build(src);
    let (res, _out) = simulate_capture(&ir, opts_with_vcd(&path));
    assert_eq!(res.finish_reason, FinishReason::Finish);
    let vcd = std::fs::read_to_string(&path).expect("vcd written");
    // scalar clk = '!': expect 0! at #0-ish, then 1!,0!,1!.
    assert!(vcd.contains("1!"), "posedge recorded:\n{vcd}");
    assert!(vcd.contains("0!"), "negedge recorded:\n{vcd}");
    // distinct timestamps present
    assert!(vcd.contains("#5") && vcd.contains("#10") && vcd.contains("#15"));
    let _ = std::fs::remove_file(&path);
}

// ── 8. infinite-delta guard (combinational loop) ───────────────────────────

#[test]
fn comb_loop_settles_to_x_not_infinite() {
    // In 4-state logic `assign a = ~a;` settles to X in one delta (it does NOT
    // oscillate — ~Z=X, ~X=X). This documents the 4-state convergence: the run
    // finishes normally rather than tripping the delta guard.
    let src = "module m; wire a; assign a = ~a; \
               initial begin #1 $finish; end endmodule";
    let ir = build(src);
    let opts = SimOpts {
        max_deltas: 1000,
        ..SimOpts::default()
    };
    let sink = DiagSink::default();
    let res = simulate(&ir, &sink, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
}

#[test]
fn infinite_delta_guard_trips() {
    // Two processes ping-pong: each change of `a` bumps `b`, each change of `b`
    // copies back to `a`, so a/b increment forever (4-bit wrap) and never settle
    // → the infinite-delta guard must fire. NOTE: a single-process self-write
    // oscillator (`always @(a) a = a + 1;`) is NOT infinite — IEEE §9 (matched by
    // iverilog: ticks once, a settles at 1) does not re-trigger a process on its
    // OWN blocking write, so this uses a genuine CROSS-process loop instead.
    let src = "module m; reg [3:0] a, b; \
               always @(a) b = a + 1; \
               always @(b) a = b; \
               initial begin a = 0; #1 $finish; end endmodule";
    let ir = build(src);
    let opts = SimOpts {
        max_deltas: 500,
        ..SimOpts::default()
    };
    let sink = DiagSink::default();
    let res = simulate(&ir, &sink, opts);
    assert_eq!(res.finish_reason, FinishReason::DeltaLimit);
    assert_eq!(res.exit_class, sim_engine::ExitClass::Fatal);
}

// ── 9. determinism: identical SimIr → identical output twice ───────────────

#[test]
fn deterministic_repeat_runs() {
    let src = "module m; reg clk; reg [3:0] cnt; \
               always @(posedge clk) cnt <= cnt + 1; \
               initial begin clk=0; cnt=0; \
                 #5 clk=1; #5 clk=0; #5 clk=1; #5 clk=0; \
                 $display(\"%0d\", cnt); $finish; end endmodule";
    let ir = build(src);
    let (_r1, o1) = simulate_capture(&ir, SimOpts::default());
    let (_r2, o2) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(o1, o2, "same SimIr → identical stdout");
    assert_eq!(o1, "2\n", "counter incremented twice");
}

// ── 10. quiescent end (no $finish) ─────────────────────────────────────────

#[test]
fn quiescent_when_no_finish() {
    let src = "module m; reg a; initial begin a=1; #3 a=0; end endmodule";
    let ir = build(src);
    let sink = DiagSink::default();
    let res = simulate(&ir, &sink, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Quiescent);
    assert_eq!(res.sim_time, 3);
}

// ── 11. reduction + bitwise ops with X propagation ─────────────────────────

#[test]
fn reduction_and_xprop() {
    // &4'b1111 = 1 ; |4'b0000 = 0 ; ^4'b1010 = 0
    let src = "module m; reg [3:0] a; reg [3:0] b; reg [3:0] c; \
               initial begin a=4'b1111; b=4'b0000; c=4'b1010; \
                 $display(\"%b %b %b\", &a, |b, ^c); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "1 0 0\n");
}

#[test]
fn x_value_displays_as_x() {
    // Uninitialized reg is X; %d of an all-X value prints x, right-justified in the
    // 4-bit operand's default decimal field width (2 chars: max 15 → " x").
    let src = "module m; reg [3:0] a; \
               initial begin $display(\"%d\", a); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, " x\n");
}

#[test]
fn percent_d_default_field_width() {
    // IEEE %d: bare `%d` right-justifies in the operand's default decimal field width
    // (8-bit → 3, 4-bit → 2); `%0d` is minimal; `%5d` is an explicit width.
    let src = "module t; reg [7:0] a; reg [3:0] b; \
               initial begin a=8'd5; b=4'd7; \
                 $display(\"[%d][%0d][%d][%5d]\", a, a, b, b); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // a→"  5" (field 3), %0d→"5", b→" 7" (field 2), %5d→"    7".
    assert_eq!(out, "[  5][5][ 7][    7]\n");
}

// ── 12. ternary + concat ───────────────────────────────────────────────────

#[test]
fn ternary_and_concat() {
    let src = "module m; reg sel; reg [3:0] a; reg [3:0] b; reg [7:0] r; \
               initial begin sel=1; a=4'hA; b=4'h5; \
                 r = sel ? {a, b} : {b, a}; \
                 $display(\"%h\", r); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // sel=1 → {a,b} = {A,5} = 0xA5
    assert_eq!(out, "a5\n");
}

// ── 13. signed arithmetic + signed %d ──────────────────────────────────────

#[test]
fn signed_subtraction_prints_negative() {
    // signed 8-bit: 3 - 10 = -7, printed as a signed decimal.
    let src = "module m; reg signed [7:0] a; reg signed [7:0] b; reg signed [7:0] r; \
               initial begin a=8'sd3; b=8'sd10; r = a - b; \
                 $display(\"%0d\", r); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "-7\n", "signed 3-10 = -7");
}

// ── 14. negedge flip-flop ──────────────────────────────────────────────────

#[test]
fn negedge_flipflop() {
    // q follows d on the negedge of clk (1→0), not on the posedge.
    let src = "module m; reg clk; reg d; reg q; \
               always @(negedge clk) q <= d; \
               initial begin clk=1; d=1; q=0; \
                 #5 clk=0; \
                 #1 $display(\"%b\", q); \
                 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "1\n", "negedge clk captures d=1");
}

#[test]
fn probe_blocking_edge_counter_no_rearm_dup() {
    // Blocking edge body: cnt MUST increment exactly once per posedge.
    // The rearm-duplication bug makes this 2^k-1.
    let src = "module m; reg clk; reg [7:0] cnt; \
               always @(posedge clk) cnt = cnt + 1; \
               initial begin clk=0; cnt=0; \
                 #5 clk=1; #5 clk=0; #5 clk=1; #5 clk=0; #5 clk=1; #5 clk=0; \
                 $display(\"%0d\", cnt); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(
        out, "3\n",
        "blocking edge body increments once per posedge (3 posedges)"
    );
}

#[test]
fn probe_mixed_sign_equality_zero_extends() {
    // 4'sb1111 (=-1 signed) compared to 8'hFF unsigned. Per IEEE 1364 §4.5,
    // if EITHER operand is unsigned the comparison is unsigned: the 4-bit signed
    // operand ZERO-extends to 8'h0F, which != 0xFF → result 0 (not 1).
    let src = "module m; reg signed [3:0] a; reg [7:0] b; reg r; \
               initial begin a=4'sb1111; b=8'hFF; r = (a == b); \
                 $display(\"%b\", r); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(
        out, "0\n",
        "mixed signed/unsigned == zero-extends the signed operand"
    );
}

#[test]
fn probe_shift_context_width() {
    // y[7:0] = (a4 << 5), a4 = 4'b0001. The shifted-in bit must survive into the
    // wider 8-bit LHS → 8'h20 = 32. (The engine grows the left-shift result so no
    // bit is lost; `write_lvalue` then truncates to the LHS width.)
    let src = "module m; reg [3:0] a4; reg [7:0] y; \
               initial begin a4=4'b0001; y = a4 << 5; \
                 $display(\"%0d\", y); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(
        out, "32\n",
        "left-shift into a wider LHS keeps the shifted-in bit (0x20)"
    );
}

#[test]
fn probe_cont_assign_oscillator_bounded() {
    // A 2-net combinational ring `a=~b; b=a`. In 4-state this settles to X (no
    // real oscillation), so it finishes — what we assert is that it TERMINATES
    // (bounded), not that it trips. The HIGH fix guarantees that even a divergent
    // cont-assign loop is bounded by the single shared delta budget.
    let src = "module m; wire a; wire b; assign a = ~b; assign b = a; \
               initial begin #1 $finish; end endmodule";
    let ir = build(src);
    let opts = SimOpts {
        max_deltas: 1000,
        ..SimOpts::default()
    };
    let sink = DiagSink::default();
    let res = simulate(&ir, &sink, opts);
    // Must terminate one way or the other (Finish or DeltaLimit), never hang.
    assert!(matches!(
        res.finish_reason,
        FinishReason::Finish | FinishReason::DeltaLimit | FinishReason::Quiescent
    ));
}

#[test]
fn probe_in_body_edge_wait_fires_once() {
    // In-body `@(posedge clk)` must resume exactly once and not leave a standing
    // net_to_edge orphan that re-fires on later clk edges. We count resumes by
    // incrementing a blocking counter after each wait; two posedges → exactly 2.
    let src = "module m; reg clk; reg [7:0] n; \
               initial begin n=0; @(posedge clk) n=n+1; @(posedge clk) n=n+1; \
                 $display(\"%0d\", n); $finish; end \
               initial begin clk=0; #5 clk=1; #5 clk=0; #5 clk=1; #5 clk=0; end \
               endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "2\n", "two in-body posedge waits resume exactly twice");
}

// ── part-select read/write (regression: Select.width / LvalChunk.offset+width
//    are ExprId const-expr edges, not literal counts; must be const-folded) ──

#[test]
fn part_select_read_folds_width() {
    // c[11:4] of 0xABC = 0xAB. Before the fold fix this read the raw width
    // ExprId as a bit count and produced garbage (0x0B).
    let src = "module m; reg [11:0] c; reg [7:0] hi; \
               initial begin c=12'hABC; #1 hi=c[11:4]; $display(\"%h\", hi); $finish; end \
               endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "ab\n", "part-select reads the correct byte");
}

#[test]
fn part_select_write_folds_offset_and_width() {
    // q[7:4]=f then q[3:0]=a → q=0xFA. Exercises the LHS chunk offset+width fold.
    let src = "module m; reg [7:0] q; \
               initial begin q=8'h00; #1 q[7:4]=4'hf; q[3:0]=4'ha; $display(\"%h\", q); $finish; end \
               endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(
        out, "fa\n",
        "two part-select writes land in the right nibbles"
    );
}

#[test]
fn bit_select_write_folds_offset() {
    // b[3]=1 on a zero reg → 0x08. Exercises the bit-select LHS offset fold.
    let src = "module m; reg [7:0] b; \
               initial begin b=8'h00; #1 b[3]=1'b1; $display(\"%h\", b); $finish; end \
               endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "08\n", "bit-select write targets the indexed bit");
}

// ── $strobe / $monitor postponed-region semantics (§5.1–5.14) ───────────────

#[test]
fn strobe_shows_post_nba_value_vs_display_pre() {
    // q starts 0, d=1. On the posedge: $display(q) prints pre-update 0; the NBA
    // q<=d schedules q→1 (applied in NBA region); $strobe(q) defers to the
    // postponed region and samples the settled post-NBA value 1.
    let src = "module m; reg clk; reg d; reg q; \
               always @(posedge clk) begin \
                 $display(\"disp %b\", q); q <= d; $strobe(\"strb %b\", q); \
               end \
               initial begin clk=0; d=1; q=0; \
                 #5 clk=1; \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // $display fires in the active region (q still 0). $strobe fires in the
    // postponed region of the SAME timestep, after NBA set q=1.
    assert_eq!(out, "disp 0\nstrb 1\n");
}

#[test]
fn two_strobes_print_in_call_order() {
    // In one posedge step: register $strobe(a) then $strobe(b). a is NBA-updated
    // to 9 this step. Postponed FIFO drains in call order: a-line (settled 9)
    // before b-line (2).
    let src = "module m; reg clk; reg [3:0] a; reg [3:0] b; \
               always @(posedge clk) begin \
                 $strobe(\"a=%0d\", a); $strobe(\"b=%0d\", b); a <= 4'd9; \
               end \
               initial begin clk=0; a=4'd1; b=4'd2; \
                 #5 clk=1; \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // Both strobes sample at end-of-timestep regardless of enqueue position:
    // a shows its settled post-NBA value 9; order is call order (a then b).
    assert_eq!(out, "a=9\nb=2\n");
}

#[test]
fn strobe_is_one_shot_per_call() {
    // $strobe runs once inside the posedge body. The next timestep (a later #5
    // with NO posedge) must NOT reprint it: the FIFO was cleared at flush.
    let src = "module m; reg clk; reg [3:0] a; \
               always @(posedge clk) $strobe(\"s=%0d\", a); \
               initial begin clk=0; a=4'd4; \
                 #5 clk=1; \
                 #5 a=4'd7; \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // Exactly one strobe line (from the single posedge); the later a=7 step does
    // not reprint because the strobe FIFO is cleared every flush.
    assert_eq!(out, "s=4\n");
}

#[test]
fn monitor_prints_once_on_establish() {
    // Establish $monitor on flag (=0). It prints once in the postponed region of
    // the establishing timestep (establishment-prints-immediately rule).
    let src = "module m; reg flag; \
               initial begin flag=0; \
                 $monitor(\"flag=%b\", flag); \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "flag=0\n");
}

#[test]
fn monitor_prints_only_on_change() {
    // Establish at t=0 (flag=0 → print). t=10 flag→1 (print). t=20 flag unchanged
    // (NO print). t=30 flag→0 (print). Three lines, the unchanged step is silent.
    let src = "module m; reg flag; \
               initial begin flag=0; \
                 $monitor(\"flag=%b\", flag); \
                 #10 flag=1; \
                 #10 flag=1; \
                 #10 flag=0; \
                 #10 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // establish(0) → 1 → [unchanged, silent] → 0
    assert_eq!(out, "flag=0\nflag=1\nflag=0\n");
}

#[test]
fn monitor_detects_x_transition() {
    // flag starts X (uninitialized 1-bit reg). Establish prints "flag=x". Then
    // flag→0 is a value change (X→0) and prints "flag=0". 4-state-aware equality:
    // a defined↔X transition counts as a change.
    let src = "module m; reg flag; \
               initial begin \
                 $monitor(\"flag=%b\", flag); \
                 #5 flag=0; \
                 #5 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // %b of an X 1-bit reg renders 'x' (see fmt_radix X handling).
    assert_eq!(out, "flag=x\nflag=0\n");
}

// ═══════════════════════════════════════════════════════════════════════════
//   FORK / JOIN / JOIN_ANY / JOIN_NONE — concurrent execution
//
// Every behavioral assertion below is chosen to FAIL under the OLD sequential
// fork lowering (noted per test). Output ordering uses the declaration-order
// determinism rule (composed child tie). FORK 13 (.velab sidecar round-trip) is
// DEFERRED: staged .velab trailer lands with vcmp/velab/vrun.
// ═══════════════════════════════════════════════════════════════════════════

// ── FORK 1. concurrent delays interleave: b at 3, a at 5 (NOT a@5 then b@8) ──
#[test]
fn fork_join_concurrent_delays_interleave() {
    let src = "module m; reg a; reg b; \
               initial begin a=0; b=0; \
                 fork #5 a=1; #3 b=1; join \
                 $display(\"%0d %b %b\", $time, a, b); \
               end endmodule";
    let (ir, opts) = build_fork(src);
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Quiescent);
    // join waits for ALL → parent prints at t=5 with both set. Sequential would
    // give a@5 then b@8 → print at t=8. The time token 5 FAILS the old path.
    assert_eq!(out, "5 1 1\n");
    assert_eq!(res.sim_time, 5);
}

// 18. [P3 · backend determinism contract] The WHOLE float-format surface as ONE
// byte-image. Every OS MUST produce these exact bytes: the formatters deliberately
// avoid libm transcendentals (no log10) and use only Rust's deterministic
// `{:e}`/`{:.*}`, so the bytecode-VM path (P0a) reuses them VERBATIM — no
// re-implementation, no fast-math (see doc-18 §결정 기록 / §P3). A regression in
// any frozen formatter (`fmt_real`/`fmt_real_e`/`format_g`/`fmt_dec`/`dec_field_width`,
// builtins.rs) or in `value.rs` real ops flips this golden. Covers %f, %e (2-digit
// padded exp), %g (exp form + signed-zero canon + inf), %d-on-real (round half-away,
// 64-bit field width 20), and the >128-bit %d field width (200-bit → 61, the only
// f64-multiply path: `n * LOG10_2`), plus %g on $realtime.
#[test]
fn float_format_determinism_golden() {
    let out = run_sv(
        "module t;\n\
           real r; reg [199:0] big;\n\
           initial begin\n\
             r = 1.0 / 3.0;     $display(\"%f\", r);\n\
             r = 1500.0;        $display(\"%e\", r);\n\
             r = 0.00001;       $display(\"%g\", r);\n\
             r = -(0.0);        $display(\"%g|%f\", r, r);\n\
             r = 1.0 / 0.0;     $display(\"%g\", r);\n\
             r = 2.5;           $display(\"%d\", r);\n\
             big = 200'd123456; $display(\"[%d]\", big);\n\
             #3 $display(\"%g\", $realtime);\n\
           end\n\
         endmodule",
    );
    // bare `%d` on a REAL is UNPADDED — iverilog prints the rounded value with no
    // default field width (2.5 → "3", not a 20-wide u64 field). `%d` on a 200-bit
    // INTEGER still uses the n*LOG10_2 path → field width 61 (unchanged; the 61
    // width is reconstructed via format! so it is self-documenting).
    let expected = format!(
        "0.333333\n1.500000e+03\n1e-05\n0|0.000000\ninf\n3\n[{:>61}]\n3\n",
        123456,
    );
    assert_eq!(out, expected);
}
