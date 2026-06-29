//! End-to-end sim-engine tests: build a SimIr via the real lex → parse →
//! elaborate pipeline, simulate it, and assert on captured $display output and
//! the generated VCD file.

use std::cell::RefCell;
use std::rc::Rc;

use diag::{LogEvent, LogSink};
use sim_engine::{simulate, simulate_capture, Backend, FinishReason, SimOpts};

// ── pipeline + sink helpers ────────────────────────────────────────────────

#[derive(Default)]
struct DiagSink(RefCell<Vec<String>>);
impl LogSink for DiagSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::Diagnostic(d) = e {
            self.0
                .borrow_mut()
                .push(format!("{:?}: {}", d.severity, d.message));
        }
    }
}

fn build(src: &str) -> sim_ir::SimIr {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = DiagSink::default();
    let ir = elaborate::elaborate(&su.expect("source unit"), &sink);
    let diags = sink.0.borrow();
    let hard: Vec<&String> = diags
        .iter()
        .filter(|d| d.starts_with("Error") || d.starts_with("Fatal"))
        .collect();
    assert!(hard.is_empty(), "elaborate errors: {hard:?}");
    ir.expect("elaborate returned None")
}

/// Full front-end incl. preprocess + per-module `timescale` resolution. Returns
/// `(ir, opts)` with `proc_multipliers` threaded so delays scale to global-precision
/// ticks and `$time`/`$realtime` divide by the calling module's multiplier.
fn build_timescaled(src: &str) -> (sim_ir::SimIr, SimOpts) {
    let pp = hdl_preprocess::preprocess_str(
        std::path::Path::new("/v"),
        "t.sv",
        src,
        &hdl_preprocess::PreOpts::default(),
    );
    assert!(!pp.has_errors(), "pp errors: {:?}", pp.diags);
    let (toks, le) = hdl_lexer::lex(&pp.text);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, &pp.text);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let su = su.expect("source unit");
    let modules: Vec<(&str, usize)> = su
        .items
        .iter()
        .filter_map(|it| match it {
            hdl_ast::TopItem::Module(m) => Some((m.name.name.as_str(), m.span.lo as usize)),
            _ => None,
        })
        .collect();
    let rt = hdl_preprocess::resolve_module_timescales(&modules, &pp.timescales);
    let sink = DiagSink::default();
    let (ir, sc) =
        elaborate::elaborate_with_timescale(&su, &sink, &rt.unit_exp, rt.global_prec_exp);
    let hard: Vec<String> = sink
        .0
        .borrow()
        .iter()
        .filter(|d| d.starts_with("Error") || d.starts_with("Fatal"))
        .cloned()
        .collect();
    assert!(hard.is_empty(), "elaborate errors: {hard:?}");
    let opts = SimOpts {
        fork_modes: sc.fork_modes,
        proc_multipliers: sc.proc_multipliers,
        severities: sc.severities,
        assign_ranks: sc.assign_ranks,
        radixes: sc.radixes,
        ..SimOpts::default()
    };
    (ir.expect("elaborate returned None"), opts)
}

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

#[test]
fn timescale_default_is_1ns_1ns() {
    // No `timescale → 1ns/1ns base, multiplier 1: `#5` advances 5 ticks (unchanged).
    let (ir, opts) = build_timescaled("module top; initial begin #5; $finish; end endmodule\n");
    let (res, _out) = simulate_capture(&ir, opts);
    assert_eq!(res.sim_time, 5);
}

#[test]
fn timescale_time_and_realtime_scaled() {
    // doc-08 example: 1ns/1ps, after #2.5 (= 2500 ticks) → $time = 2 (truncated to the
    // module's 1ns unit), $realtime = 2.5 (sub-unit fraction kept).
    let (ir, opts) = build_timescaled(
        "`timescale 1ns/1ps\nmodule top; initial begin #2.5; \
         $display(\"%0d %g\", $time, $realtime); $finish; end endmodule\n",
    );
    let (_res, out) = simulate_capture(&ir, opts);
    assert_eq!(out, "2 2.5\n");
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
    // Total advanced time = 15 ticks (= 1.5ns), so $time truncates to 1.
    let (ir, opts) = build_timescaled(
        "`timescale 1ns/100ps\nmodule top; reg a; initial begin \
         a=0; #1.44 a=1; #0.05 a=0; #0.04 a=1; $display(\"%0d\", $time); $finish; end endmodule\n",
    );
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.sim_time, 15);
    assert_eq!(out, "1\n");
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

/// Elaborate `src` WITH the per-net hierarchical name side table (for VCD naming).
fn build_named(src: &str) -> (sim_ir::SimIr, Vec<String>) {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = DiagSink::default();
    let (ir, _modes, names) = elaborate::elaborate_with_sidecars(&su.expect("source unit"), &sink);
    let diags = sink.0.borrow();
    let hard: Vec<&String> = diags
        .iter()
        .filter(|d| d.starts_with("Error") || d.starts_with("Fatal"))
        .collect();
    assert!(hard.is_empty(), "elaborate errors: {hard:?}");
    (ir.expect("elaborate returned None"), names)
}

/// Elaborate `src` WITH the fork-mode side table and return `(ir, opts)` where
/// `opts` carries `fork_modes`. Existing non-fork tests keep using
/// `build()`/`SimOpts::default()` unchanged. Fork tests do:
///   `let (ir, opts) = build_fork(src); simulate_capture(&ir, opts);`
fn build_fork(src: &str) -> (sim_ir::SimIr, SimOpts) {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = DiagSink::default();
    let (ir, fork_modes) = elaborate::elaborate_with_modes(&su.expect("source unit"), &sink);
    let diags = sink.0.borrow();
    let hard: Vec<&String> = diags
        .iter()
        .filter(|d| d.starts_with("Error") || d.starts_with("Fatal"))
        .collect();
    assert!(hard.is_empty(), "elaborate errors: {hard:?}");
    let ir = ir.expect("elaborate returned None");
    (
        ir,
        SimOpts {
            fork_modes,
            ..SimOpts::default()
        },
    )
}

/// A unique temp VCD path per test.
fn tmp_vcd(tag: &str) -> String {
    let mut p = std::env::temp_dir();
    p.push(format!("vita_sim_{}_{}.vcd", tag, std::process::id()));
    p.to_string_lossy().into_owned()
}

fn opts_with_vcd(path: &str) -> SimOpts {
    SimOpts {
        vcd_path_override: Some(path.to_string()),
        ..SimOpts::default()
    }
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
    assert!(vcd.contains("$var reg 4 ! n0 $end"), "net declared:\n{vcd}");
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
        assert!(
            vcd.contains(&format!("mem[{k}] $end")),
            "element {k} declared:\n{vcd}"
        );
    }
    assert!(
        !vcd.contains(" mem $end"),
        "the old single word-0 var must be gone:\n{vcd}"
    );
    assert!(vcd.contains(" a $end"), "scalar declared:\n{vcd}");
    // mem[1] = 8'hAB must land on element 1's id (10101011).
    assert!(vcd.contains("b10101011 "), "element change record:\n{vcd}");
    let _ = std::fs::remove_file(&path);
}

/// Drop the build-version-dependent `$version … $end` block for a stable golden.
fn strip_version_block(vcd: &str) -> String {
    let mut out = String::new();
    let mut lines = vcd.lines();
    while let Some(l) = lines.next() {
        if l == "$version" {
            for x in lines.by_ref() {
                if x == "$end" {
                    break;
                }
            }
        } else {
            out.push_str(l);
            out.push('\n');
        }
    }
    out
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
                  $timescale 1ns $end\n$scope module top $end\n$var reg 4 ! a $end\n$upscope $end\n\
                  $enddefinitions $end\n$dumpvars\nbxxxx !\n$end\n#0\nb0101 !\n#5\nb1001 !\n";
    assert_eq!(vcd, golden, "golden VCD drift");
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

// helper to silence unused import warnings if a test path drops them
#[allow(dead_code)]
fn _touch() {
    let _ = Rc::new(RefCell::new(0));
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

// ════════════════════════════════════════════════════════════════════════════
// REAL / REALTIME DOMAIN (deliberate sim-ir evolution, format_version 2→3)
// Strings are blessed against the §4.1 formatter algorithms as written.
// ════════════════════════════════════════════════════════════════════════════

/// Build + simulate `src`, returning the captured $display/$write output.
fn run_sv(src: &str) -> String {
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    out
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
    // %d on a real uses the 64-bit field width (20); %d on a 200-bit operand uses
    // the n*LOG10_2 path → field width 61. Reconstruct both widths via format! so
    // they are self-documenting, not a fragile wall of literal spaces.
    let expected = format!(
        "0.333333\n1.500000e+03\n1e-05\n0|0.000000\ninf\n{:>20}\n[{:>61}]\n3\n",
        3, 123456,
    );
    assert_eq!(out, expected);
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

/// Build `src` through lex→parse→elaborate, returning the collected diagnostic
/// strings (severity-prefixed). Used to assert real-operand illegality gates.
fn elaborate_diags(src: &str) -> Vec<String> {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = DiagSink::default();
    let _ = elaborate::elaborate(&su.expect("source unit"), &sink);
    let collected = sink.0.borrow().clone();
    drop(sink);
    collected
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

#[test]
fn instance_array_remaining_cuts_stay_loud() {
    // The 2026-06-12 unroll absorbed the basic form (see cli/tests/
    // inst_array.rs); the REMAINING v1 cuts must stay loud — here the
    // non-ANSI child (port widths cannot fold for slicing).
    let diags = elaborate_diags(
        "module dff(d, q); input d; output q; assign q = d; endmodule \
         module top; wire [3:0] a, b; dff u[3:0] (.d(a), .q(b)); endmodule",
    );
    assert!(
        diags.iter().any(|d| d.contains("non-ANSI")),
        "expected loud non-ANSI instance-array rejection, got: {diags:?}"
    );
}

// E1. %h on a real argument is a STATIC elaborate-time rejection (§4.1a).
#[test]
fn real_percent_h_rejected_at_elaborate() {
    let diags = elaborate_diags(
        "module t; real r; initial begin r = 2.5; $display(\"%h\", r); end endmodule",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.contains("binary/hex/octal format not defined on a real argument")),
        "expected %h-on-real rejection, got: {diags:?}"
    );
}

// E2. `**` (power) on a real operand is a permanent ElabUnsupported (§6.2).
#[test]
fn real_power_rejected_at_elaborate() {
    let diags = elaborate_diags(
        "module t; real r; initial begin r = 2.0 ** 3; $display(\"%g\", r); end endmodule",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.contains("power (**) not defined on real operand")),
        "expected **-on-real rejection, got: {diags:?}"
    );
}

// E3. `%` (modulo) on a real operand is rejected (§6.2).
#[test]
fn real_modulo_rejected_at_elaborate() {
    let diags = elaborate_diags(
        "module t; real r; initial begin r = 2.5 % 1.0; $display(\"%g\", r); end endmodule",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.contains("modulo (%) not defined on real operand")),
        "expected %-on-real rejection, got: {diags:?}"
    );
}

// E4. A `real` net lowers to NetKind::Real (width 64, signed) and default-inits
//     to 0.0 (all-zero bits, not X) — a clean real decl elaborates with no diags.
#[test]
fn real_net_lowers_clean() {
    let (toks, le) = hdl_lexer::lex("module t; real r; realtime rt; initial r = 1.0; endmodule");
    assert!(le.is_empty());
    let (su, pe) = hdl_parser::parse(
        &toks,
        "module t; real r; realtime rt; initial r = 1.0; endmodule",
    );
    assert!(pe.is_empty());
    let sink = DiagSink::default();
    let ir = elaborate::elaborate(&su.expect("su"), &sink).expect("ir");
    assert!(
        sink.0.borrow().is_empty(),
        "unexpected diags: {:?}",
        sink.0.borrow()
    );
    // both `r` and `rt` are NetKind::Real, 64-bit signed, init 0.0 (all-zero).
    let reals: Vec<_> = ir
        .nets
        .iter()
        .filter(|n| matches!(n.kind, sim_ir::NetKind::Real))
        .collect();
    assert_eq!(reals.len(), 2, "expected 2 real nets (real + realtime)");
    for n in reals {
        assert_eq!(n.width, 64);
        assert!(n.signed);
        assert!(
            n.init.val.iter().all(|&w| w == 0),
            "real default must be 0.0 bits"
        );
        assert!(
            n.init.unk.iter().all(|&w| w == 0),
            "real default must have unk=0 (never X)"
        );
    }
}

// A real in a boolean/logical context is true iff != 0.0 (IEEE: -0.0 == 0.0).
// Regression for the adversarial-review MAJOR: truthiness must NOT read a real's
// f64 bits as a 4-state vector (which classified -0.0 — sign bit set — as true).
#[test]
fn real_negative_zero_is_logically_false() {
    let src = "module t; real r; integer n; \
               initial begin \
                 r=-0.0; if (r) $display(\"A\"); else $display(\"B\"); \
                 r=-0.0; n=!r; $display(\"%0d\", n); \
                 r=-0.0; n=(r ? 7 : 9); $display(\"%0d\", n); \
                 r=2.5;  if (r) $display(\"T\"); else $display(\"F\"); \
                 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // -0.0 → false (else "B"), !(-0.0)=1, (-0.0 ? 7 : 9)=9, and 2.5 → true ("T").
    assert_eq!(out, "B\n1\n9\nT\n");
}

// ── procedural `for` loop (desugars to `init; while(cond){body; step}`) ──────

#[test]
fn procedural_for_accumulates() {
    // sum 0..4 = 10; nested 5x5 = 25; never-enters keeps the seed.
    let src = "module t; integer i, j, s, c, z; \
               initial begin \
                 s=0; for (i=0;i<5;i=i+1) s=s+i; \
                 c=0; for (i=0;i<5;i=i+1) for (j=0;j<5;j=j+1) c=c+1; \
                 z=99; for (i=0;i<0;i=i+1) z=z+1; \
                 $display(\"s=%0d c=%0d z=%0d\", s, c, z); $finish; \
               end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "s=10 c=25 z=99\n");
}

// A `for` loop that writes a DYNAMIC bit index `a[i]` — the runtime LHS index is
// resolved at statement time, symmetric with the read side. (Before the fix the
// loop was skipped AND the dynamic write landed on bit 0.)
#[test]
fn for_loop_dynamic_bit_write() {
    let src = "module t; integer i; reg [7:0] a; reg [7:0] b; \
               initial begin a=8'h00; b=8'h00; \
                 for (i=0;i<8;i=i+1) a[i]=1; \
                 for (i=0;i<4;i=i+1) b[i]=a[i*2]; \
                 $display(\"a=%h b=%h\", a, b); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // a = all 8 bits = ff; b reads a[0],a[2],a[4],a[6] (all 1) → low nibble = 0f.
    assert_eq!(out, "a=ff b=0f\n");
}

// NBA with a dynamic LHS index samples the index in the ACTIVE region: in
// `a[i] <= 1; i = i+1;` the write must target the OLD `i`, not the bumped one.
#[test]
fn nba_dynamic_index_samples_old_value() {
    let src = "module t; integer i; reg [7:0] a; \
               initial begin a=0; i=2; #1; a[i] <= 1; i = i + 1; \
                 #1 $display(\"a=%h i=%0d\", a, i); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // a[2] set (OLD i=2) → 0x04; i bumped to 3.
    assert_eq!(out, "a=04 i=3\n");
}

// ── parameters / localparams as resolvable constants (sweep gaps 2-6) ────────

#[test]
fn parameters_resolve_as_values_and_widths() {
    // body param as a runtime value; param-sized vector; localparam expr; the
    // {W{..}} replicate count. Before the fix each errored E3010 or gave a
    // silent wrong value (vector→1 bit, replicate→0).
    let src = "module t; \
               parameter W = 8; parameter A = 4; localparam C = A*3 + 1; \
               reg [W-1:0] a; integer x; reg [7:0] r; \
               initial begin a = 200; x = C; r = {A{1'b1}}; \
                 $display(\"a=%h x=%0d r=%h\", a, x, r); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // a=200=0xc8 (8-bit holds it), C=4*3+1=13, {4{1'b1}}=0x0f.
    assert_eq!(out, "a=c8 x=13 r=0f\n");
}

#[test]
fn parameter_override_and_generate_with_param() {
    // child param overridden via #(.P()); generate-for bound + body indexed by a
    // genvar into a memory both fold to the genvar/param scope value.
    let src = "module sub #(parameter P = 1) (output [7:0] y); assign y = P + 10; endmodule \
               module t; parameter N = 4; wire [7:0] y; logic [7:0] v[0:3]; genvar g; \
               sub #(.P(7)) u (y); \
               generate for (g = 0; g < N; g = g + 1) begin: gen assign v[g] = g*2; end endgenerate \
               initial begin #1 $display(\"y=%0d v=%0d %0d %0d %0d\", y, v[0], v[1], v[2], v[3]); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // y = 7+10 = 17; v[g] = g*2 → 0 2 4 6.
    assert_eq!(out, "y=17 v=0 2 4 6\n");
}

// ── implicit sensitivity: @* / always_comb / always_latch infer the read-set
//    and RE-FIRE on any input change (sweep gaps 10-12). ───────────────────────

#[test]
fn implicit_sensitivity_recomputes_on_change() {
    let src = "module t; reg [7:0] a, b, sc, cc; reg en; reg [7:0] din, q; \
               always @*       sc = a + b; \
               always_comb     cc = a * 2; \
               always_latch    if (en) q = din; \
               initial begin \
                 a=3; b=4; en=0; din=0; q=0; \
                 #1 $display(\"%0d %0d %0d\", sc, cc, q); \
                 a=10; en=1; din=42; \
                 #1 $display(\"%0d %0d %0d\", sc, cc, q); \
                 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // t1: sc=3+4=7, cc=3*2=6, q=0 (en=0). t2: sc=10+4=14, cc=10*2=20, q=42 (en=1).
    assert_eq!(out, "7 6 0\n14 20 42\n");
}

// ── casez / casex wildcard matching: `?`/`z`/`x` label bits are don't-care
//    (sweep gaps 14,15). Before the fix every wildcard label fell to default. ──

#[test]
fn casez_casex_wildcards_match() {
    let src = "module t; reg [3:0] v; reg [7:0] z, x; \
               initial begin \
                 v = 4'b1010; \
                 casez (v) 4'b1???: z = 8'd3; 4'b01??: z = 8'd2; default: z = 8'd9; endcase \
                 casex (v) 4'b10xx: x = 8'd1; default: x = 8'd9; endcase \
                 $display(\"z=%0d x=%0d\", z, x); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // casez: 1010 matches 1??? → z=3. casex: 1010 matches 10xx → x=1.
    assert_eq!(out, "z=3 x=1\n");
}

#[test]
fn casex_scrutinee_xz_is_wildcard() {
    // REMAINING_WORK item: casex must treat SCRUTINEE x/z as don't-care, not only
    // label wildcards. s=1x10 matches label 1010 (the x bit washes out); a definite
    // mismatch (0101 vs 1010) still falls to default.
    let src = "module t; reg [3:0] s; reg [7:0] r; \
               initial begin \
                 s = 4'b1x10; casex (s) 4'b1010: r = 8'd1; default: r = 8'd9; endcase \
                 $display(\"%0d\", r); \
                 s = 4'b0101; casex (s) 4'b1010: r = 8'd1; default: r = 8'd9; endcase \
                 $display(\"%0d\", r); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // 1x10 ~ 1010 → match (1); 0101 vs 1010 → definite mismatch → default (9).
    assert_eq!(out, "1\n9\n");
}

#[test]
fn casez_explicit_x_label_no_longer_warns() {
    // v7: casez is EXACT (CasezEq — an explicit-x label bit compares 4-state,
    // it is no longer approximated as don't-care), so the old
    // W-ELAB-CASEZ-APPROX warning is retired (code reserved in doc-15).
    for src in [
        "module t; reg [3:0] s; reg r; initial \
         casez (s) 4'b10x0: r=1; default: r=0; endcase endmodule",
        "module t; reg [3:0] s; reg r; initial \
         casez (s) 4'b10?0: r=1; default: r=0; endcase endmodule",
    ] {
        let warns = elaborate_diags(src);
        assert!(
            !warns.iter().any(|d| d.contains("explicit-x")),
            "casez no longer warns (exact since v7), got: {warns:?}"
        );
    }
}

#[test]
fn casez_scrutinee_z_is_wildcard() {
    // casez: a SCRUTINEE z bit is don't-care. s=1z10 matches label 1010.
    let src = "module t; reg [3:0] s; reg [7:0] r; \
               initial begin \
                 s = 4'b1z10; casez (s) 4'b1010: r = 8'd1; default: r = 8'd9; endcase \
                 $display(\"%0d\", r); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "1\n");
}

// ── wait(expr) resumes on a false→true transition (sweep gaps 18,19). Before
//    the fix WaitCause::Expr never woke, hanging the process. ──────────────────

#[test]
fn wait_resumes_on_false_to_true() {
    let src = "module t; integer cnt; \
               initial begin cnt = 0; forever #10 cnt = cnt + 1; end \
               initial begin wait(cnt == 3); $display(\"hit@%0d\", $time); $finish; end \
               endmodule";
    let ir = build(src);
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // cnt reaches 3 at t=30; the wait resumes there (not a hang, not never).
    assert_eq!(out, "hit@30\n");
}

// ── generate-if/else: a labeled block is a generate scope (outer nets resolve
//    THROUGH it); both branches + a block-local net work (sweep gap 7). ─────────

#[test]
fn generate_if_else_scoping() {
    let src = "module t; parameter MODE = 1; reg [7:0] a, b; logic [7:0] y; reg [7:0] a2; logic [7:0] y2; \
               generate if (MODE == 1) begin: ga assign y = a + b; end \
                        else            begin: gb assign y = a - b; end endgenerate \
               generate if (MODE == 0) begin: gc assign y2 = 0; end \
                        else begin: gd logic [7:0] tmp; assign tmp = a2 + 1; assign y2 = tmp * 2; end \
               endgenerate \
               initial begin a=20; b=5; a2=5; #1 $display(\"y=%0d y2=%0d\", y, y2); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // MODE=1: y=a+b=25. Second gen takes else (gd): tmp=a2+1=6, y2=tmp*2=12.
    assert_eq!(out, "y=25 y2=12\n");
}

// ── non-ANSI ports (sweep gap 1) + a CLOCKED submodule driven through a port
//    binding: a cont-assign-driven clock edge must reach the child's always. ──

#[test]
fn non_ansi_ports_and_bound_clock_edge() {
    // `addr` has non-ANSI ports (body input/output decls); `dff` is a clocked
    // submodule whose clk arrives via the parent's port binding (a cont-assign).
    let src =
        "module addr(a, b, y); input [7:0] a, b; output [7:0] y; assign y = a + b; endmodule \
               module dff(clk, d, q); input clk, d; output q; reg q; \
                 always @(posedge clk) q <= d; initial q = 0; endmodule \
               module t; reg [7:0] x, z; wire [7:0] o; reg c, di; wire q; \
                 addr ua(x, z, o); dff ud(c, di, q); \
                 initial begin x=10; z=5; c=0; di=1; \
                   #1 c=1;  /* posedge → q<=1 */ \
                   #1 $display(\"o=%0d q=%b\", o, q); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // o = 10+5 = 15 (non-ANSI comb). q = 1 (bound-clock posedge sampled d=1).
    assert_eq!(out, "o=15 q=1\n");
}

// ── runtime (variable) memory word index: mem[k] read AND write where k is a
//    runtime value (sweep gaps 8,9). Word is now an evaluated ExprId. ──────────

#[test]
fn memory_runtime_word_index() {
    let src = "module t; reg [7:0] m[0:3]; reg [7:0] o; integer k; reg [1:0] idx; \
               initial begin \
                 for (k = 0; k < 4; k = k + 1) m[k] = k + 5;   /* write by runtime k */ \
                 idx = 2; o = m[idx];                          /* read by runtime idx */ \
                 $display(\"%0d %0d %0d %0d r=%0d\", m[0], m[1], m[2], m[3], o); $finish; \
               end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // m[k]=k+5 → 5 6 7 8; m[idx=2] = 7.
    assert_eq!(out, "5 6 7 8 r=7\n");
}

// ── multi-dimensional UNPACKED array (V2001): `reg [7:0] g[0:1][0:2]` is a 2×3
//    array of 8-bit words. elaborate flattens row-major (i*ncols+j) onto the
//    existing single-word memory model — no frozen-IR change. Read AND write. ──

#[test]
fn array_2d_const_index_rw() {
    let src = "module t; reg [7:0] g[0:1][0:2]; \
               initial begin \
                 g[0][0] = 8'd5; g[1][2] = 8'd9; g[1][0] = 8'd7; \
                 $display(\"%0d %0d %0d\", g[0][0], g[1][0], g[1][2]); $finish; \
               end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "5 7 9\n");
}

#[test]
fn const_eval_power_operator() {
    // REMAINING_WORK item: `parameter/localparam = 2**N` must fold (was silently 0,
    // underflowing the range to a 1-bit net). W = 2**4 = 16 → reg [15:0] holds 0xABCD.
    let src = "module t; localparam W = 2**4; reg [W-1:0] r; \
               initial begin r = 16'hABCD; $display(\"%0d %0h\", W, r); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "16 abcd\n");
}

#[test]
fn const_eval_arith_shift_operators() {
    // `<<<`/`>>>` (arith shift) fold in const exprs (unsigned elaboration domain).
    let src = "module t; localparam A = 3 <<< 2; localparam B = 256 >>> 3; \
               initial begin $display(\"%0d %0d\", A, B); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // 3<<2 = 12 ; 256>>3 = 32.
    assert_eq!(out, "12 32\n");
}

#[test]
fn unsigned_wide_arithmetic_128bit() {
    // REMAINING_WORK item: 128-bit unsigned add with a carry across the 64-bit word
    // boundary must NOT truncate to the low 64 bits. a=b=2^64 → a+b = 2^65.
    let src = "module t; reg [127:0] a, b, c; \
               initial begin a = 128'h1_0000_0000_0000_0000; b = a; c = a + b; \
                 $display(\"%0h\", c); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // 2*2^64 = 0x2 followed by 16 hex zeros.
    assert_eq!(out, "20000000000000000\n");
}

#[test]
fn unsigned_wide_multiply_96bit() {
    // 96-bit multiply whose product exceeds 64 bits: 2^40 * 2^40 = 2^80.
    let src = "module t; reg [95:0] a, b, c; \
               initial begin a = 96'h100_0000_0000; b = a; c = a * b; \
                 $display(\"%0h\", c); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // 2^40 * 2^40 = 2^80 = 0x1 followed by 20 hex zeros.
    assert_eq!(out, "100000000000000000000\n");
}

#[test]
fn always_comb_tracks_array_index_signal() {
    // REMAINING_WORK item: `always_comb y = mem[sel]` — changing ONLY sel must
    // re-fire the block (the array WORD index signal belongs to the comb read-set).
    // Regression for stale combinational output (collect_expr_reads ignored Signal.word).
    let src = "module t; reg [7:0] mem[0:3]; reg [1:0] sel; reg [7:0] y; \
               always_comb y = mem[sel]; \
               initial begin mem[0]=8'd10; mem[1]=8'd20; mem[2]=8'd30; mem[3]=8'd40; \
                 sel=0; #1 $display(\"%0d\", y); \
                 sel=2; #1 $display(\"%0d\", y); \
                 sel=3; #1 $display(\"%0d\", y); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "10\n30\n40\n");
}

#[test]
fn array_nonzero_base_index_normalized() {
    // REMAINING_WORK item: `reg [7:0] mem[4:7]` — index 4..7 maps to words 0..3
    // (subtract the dim's lower bound), no aliasing onto a 0-based window, no OOR.
    let src = "module t; reg [7:0] mem[4:7]; integer i; \
               initial begin for (i=4;i<=7;i=i+1) mem[i] = i*10; \
                 $display(\"%0d %0d %0d %0d\", mem[4], mem[5], mem[6], mem[7]); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "40 50 60 70\n");
}

#[test]
fn array_descending_base_index_normalized() {
    // `reg [7:0] mem[7:4]` (descending, non-zero min): indices 4..7 round-trip.
    let src = "module t; reg [7:0] mem[7:4]; integer i; \
               initial begin for (i=4;i<=7;i=i+1) mem[i] = i+100; \
                 $display(\"%0d %0d\", mem[4], mem[7]); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "104 107\n");
}

#[test]
fn array_2d_nonzero_base_index_normalized() {
    // 2-D non-zero base `g[1:2][1:3]`: each dim normalized by its lower bound.
    let src = "module t; reg [7:0] g[1:2][1:3]; integer i, j; \
               initial begin for (i=1;i<=2;i=i+1) for (j=1;j<=3;j=j+1) g[i][j] = i*10+j; \
                 $display(\"%0d %0d %0d %0d\", g[1][1], g[1][3], g[2][1], g[2][3]); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "11 13 21 23\n");
}

#[test]
fn array_x_index_read_x_write_noop() {
    // REMAINING_WORK item: an X/Z array index reads all-X (not word 0) and its write
    // is a no-op — consistent with the out-of-range word semantics.
    let src = "module t; reg [7:0] m[0:3]; reg [1:0] xi; \
               initial begin \
                 m[0] = 8'd7; m[1] = 8'd42; xi = 2'bx0;  /* xi unknown (bit1 = x) */ \
                 m[xi] = 8'd99;                           /* X-index write → no-op */ \
                 $display(\"%0d %0d %b\", m[0], m[1], m[xi]); /* word0 NOT read; X → xxxxxxxx */ \
                 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "7 42 xxxxxxxx\n");
}

#[test]
fn packed_nonzero_base_bit_select() {
    // REMAINING_WORK: `reg [7:4]` (4-bit, indices 4..7) — bit-select must normalize
    // by lsb=4: v[7]=MSB(internal 3), v[4]=LSB(internal 0). Was returning X (raw 7,4).
    let src = "module t; reg [7:4] v; initial begin v = 4'b1001; \
               $display(\"%0d %0d\", v[7], v[4]); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "1 1\n");
}

#[test]
fn packed_nonzero_base_mid_bits() {
    // reg [3:1]: 3'b101 → bit3(MSB)=1, bit2=0, bit1(LSB)=1 (normalized by lsb=1).
    let src = "module t; reg [3:1] v; initial begin v = 3'b101; \
               $display(\"%0d %0d %0d\", v[3], v[2], v[1]); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "1 0 1\n");
}

#[test]
fn packed_ascending_bit_select() {
    // reg [0:7] (ascending): index 0 = MSB. 8'b1000_0000 → r[0]=1, r[7]=0 (lsb-i).
    let src = "module t; reg [0:7] r; initial begin r = 8'b1000_0000; \
               $display(\"%0d %0d\", r[0], r[7]); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "1 0\n");
}

#[test]
fn packed_nonzero_base_part_select() {
    // reg [7:4]; v=4'b1010 (idx7→1,6→0,5→1,4→0). v[6:5] = internal[2:1] = 2'b01 = 1.
    let src = "module t; reg [7:4] v; reg [1:0] p; initial begin v = 4'b1010; \
               p = v[6:5]; $display(\"%0d\", p); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "1\n");
}

#[test]
fn packed_zero_base_unchanged() {
    // REGRESSION: `reg [7:0]` (lsb=0) bit-select is unchanged (no normalization).
    let src = "module t; reg [7:0] r; initial begin r = 8'b1010_0001; \
               $display(\"%0d %0d %0d\", r[0], r[5], r[7]); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "1 1 1\n");
}

#[test]
fn oob_emits_run_range_diagnostic() {
    // REMAINING_WORK: an out-of-range array word access now EMITS a runtime
    // E-RUN-RANGE diagnostic (the access is still recovered: read X / write dropped).
    let src = "module t; reg [7:0] m[0:3]; integer i; \
               initial begin i=9; m[i]=8'd7; $display(\"%0d\", m[i]); $finish; end endmodule";
    let ir = build(src);
    let sink = DiagSink::default();
    let res = simulate(&ir, &sink, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish); // OOR is recovered, run finishes
    let diags = sink.0.borrow();
    assert!(
        diags.iter().any(|d| d.contains("out of range")),
        "expected an E-RUN-RANGE diagnostic, got: {diags:?}"
    );
}

#[test]
fn array_oob_word_read_is_x_write_ignored() {
    // REMAINING_WORK item: an out-of-range array WORD index reads all-X and a write
    // is IGNORED — not clamped to the last element (which silently returned/corrupted
    // a valid neighbor). E-RUN-RANGE semantics.
    let src = "module t; reg [7:0] m[0:3]; integer i; \
               initial begin \
                 m[3] = 8'd33; i = 9; \
                 m[i] = 8'd77;                     /* OOR write → ignored, m[3] intact */ \
                 $display(\"%0d %b\", m[3], m[i]);  /* m[3]=33 ; OOR read → xxxxxxxx */ \
                 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "33 xxxxxxxx\n");
}

#[test]
fn array_2d_runtime_fill() {
    // grid[i][j] = i*10 + j over a nested loop with RUNTIME indices, read back.
    let src = "module t; reg [7:0] g[0:1][0:2]; integer i, j; \
               initial begin \
                 for (i = 0; i < 2; i = i + 1) \
                   for (j = 0; j < 3; j = j + 1) g[i][j] = i*10 + j; \
                 $display(\"%0d %0d %0d %0d %0d %0d\", \
                   g[0][0], g[0][1], g[0][2], g[1][0], g[1][1], g[1][2]); $finish; \
               end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // i*10+j → 0 1 2 / 10 11 12
    assert_eq!(out, "0 1 2 10 11 12\n");
}

#[test]
fn array_2d_element_bit_select_read() {
    // g[i][j][k] = bit k of the 8-bit element at [i][j]. (n == D+1)
    let src = "module t; reg [7:0] g[0:1][0:1]; \
               initial begin \
                 g[1][1] = 8'b1010_0001; \
                 $display(\"%0d %0d %0d\", g[1][1][0], g[1][1][5], g[1][1][7]); $finish; \
               end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // bit0=1, bit5=1, bit7=1
    assert_eq!(out, "1 1 1\n");
}

#[test]
fn array_2d_element_bit_write() {
    // g[i][j][k] = b writes a single bit of element [i][j]. (write n == D+1)
    let src = "module t; reg [7:0] g[0:1][0:1]; \
               initial begin \
                 g[0][1] = 8'd0; g[0][1][0] = 1'b1; g[0][1][3] = 1'b1; \
                 $display(\"%0d\", g[0][1]); $finish; \
               end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // bits 0 and 3 set → 0b1001 = 9
    assert_eq!(out, "9\n");
}

#[test]
fn array_1d_element_bit_select_unchanged() {
    // REGRESSION: `mem[i][j]` on a 1D array is bit j of word i — must still work
    // (the unified chain handler subsumes the old single-dim path).
    let src = "module t; reg [7:0] m[0:3]; \
               initial begin m[2] = 8'b0000_0100; \
                 $display(\"%0d %0d\", m[2][2], m[2][0]); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "1 0\n");
}

#[test]
fn array_2d_partial_slice_scalar_rhs_rejected() {
    // Phase-1.x ②: `g[i] = <array>` is a real ARRAY ASSIGNMENT now, but a
    // partial slice fed a SCALAR stays loud — NOT a silent word write.
    let diags = elaborate_diags("module t; reg [7:0] g[0:1][0:2]; initial g[0] = 8'd5; endmodule");
    assert!(
        diags.iter().any(|d| d.contains("non-array value")),
        "expected scalar-into-slice rejection, got: {diags:?}"
    );
}

#[test]
fn array_3d_runtime_fill() {
    // 3-D array, strides 4/2/1; element encodes its coordinate i*100+j*10+k.
    let src = "module t; reg [7:0] g[0:1][0:1][0:1]; integer i, j, k; \
               initial begin \
                 for (i=0;i<2;i=i+1) for (j=0;j<2;j=j+1) for (k=0;k<2;k=k+1) g[i][j][k]=i*100+j*10+k; \
                 $display(\"%0d %0d %0d %0d\", g[0][0][0], g[0][1][1], g[1][0][1], g[1][1][1]); $finish; \
               end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "0 11 101 111\n");
}

#[test]
fn array_2d_element_part_select_write() {
    // `g[i][j][7:4] = …` part-selects WITHIN a 2-D element; the low nibble survives.
    let src = "module t; reg [7:0] g[0:1][0:1]; \
               initial begin g[1][0] = 8'hAB; g[1][0][7:4] = 4'h3; \
                 $display(\"%0h %0h\", g[1][0], g[1][0][7:4]); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // high nibble A→3, low nibble B kept → 0x3B ; read-back of [7:4] = 3.
    assert_eq!(out, "3b 3\n");
}

#[test]
fn array_1d_element_part_select_write() {
    // 1-D element part-select write `m[i][3:0] = …` (newly symmetric with reads).
    let src = "module t; reg [7:0] m[0:3]; \
               initial begin m[2] = 8'hF0; m[2][3:0] = 4'hA; \
                 $display(\"%0h\", m[2]); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "fa\n");
}

#[test]
fn array_2d_element_indexed_part_write() {
    // `g[i][j][k+:4] = …` indexed-part write into a 2-D element.
    let src = "module t; reg [7:0] g[0:1][0:1]; integer k; \
               initial begin g[0][1] = 8'h00; k = 4; g[0][1][k+:4] = 4'hF; \
                 $display(\"%0h\", g[0][1]); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "f0\n");
}

#[test]
fn array_2d_nba_clocked_write() {
    // 2-D element written nonblocking under @(posedge clk) behaves like flat mem.
    let src = "module t; reg clk; reg [7:0] g[0:1][0:1]; integer n; \
               always @(posedge clk) g[1][1] <= g[1][1] + 8'd1; \
               initial begin clk=0; g[1][1]=8'd0; \
                 for (n=0;n<3;n=n+1) begin #5 clk=1; #5 clk=0; end \
                 $display(\"%0d\", g[1][1]); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // three rising edges → incremented to 3.
    assert_eq!(out, "3\n");
}

#[test]
fn array_2d_over_index_read_rejected() {
    // `g[i][j][k][m]` over-indexes a 2-D 8-bit element (bit-of-bit) → must reject
    // LOUDLY, SYMMETRIC with the write path (no silent X on the read side).
    let diags = elaborate_diags(
        "module t; reg [7:0] g[0:1][0:1]; reg b; initial b = g[1][1][3][0]; endmodule",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.contains("bit-select then bit-select")),
        "expected over-index read rejection, got: {diags:?}"
    );
}

#[test]
fn array_2d_signed_element() {
    // A `signed` element sign-extends like the equivalent 1-D signed reg.
    let src = "module t; reg signed [7:0] g[0:1][0:1]; reg signed [15:0] r; \
               initial begin g[1][1] = -8'sd5; r = g[1][1]; $display(\"%0d\", r); $finish; end \
               endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "-5\n");
}

#[test]
fn array_2d_wide_element_part_select() {
    // 64-bit element (multi-chunk word): a part-select crossing the 32-bit chunk
    // boundary writes/reads correctly — flatten composes with the chunk machinery.
    let src = "module t; reg [63:0] g[0:1][0:1]; \
               initial begin g[1][0] = 64'd0; g[1][0][39:24] = 16'hBEEF; \
                 $display(\"%0h %0h\", g[1][0], g[1][0][39:24]); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // bits [39:24] = 0xBEEF → word 0x0000_00BE_EF00_0000.
    assert_eq!(out, "beef000000 beef\n");
}

// ── named block with block-local declarations (sweep gap 16): locals are
//    hoisted to module nets so references inside the block resolve. ────────────

#[test]
fn named_block_local_declarations() {
    let src = "module t; integer s; reg [7:0] r; \
               initial begin: acc_blk integer i; integer acc; \
                 acc = 0; for (i = 1; i <= 5; i = i + 1) acc = acc + i; s = acc; \
                 begin: inner reg [7:0] x; reg [7:0] y; x = 10; y = 5; r = x + y; end \
                 $display(\"s=%0d r=%0d\", s, r); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // sum 1..5 = 15; nested-block locals x=10,y=5 → r=15.
    assert_eq!(out, "s=15 r=15\n");
}

// ── continuous assign with a delay `assign #d y = a`: the value propagates
//    AFTER d ticks, not immediately (certification BLOCKER-1). Transport delay. ─

#[test]
fn continuous_assign_delay_propagates_after_d() {
    let src = "module t; reg [3:0] a; wire [3:0] y; \
               assign #5 y = a; \
               initial begin a = 7; \
                 #2 $display(\"t2 y=%0d\", y);   /* not propagated yet */ \
                 #4 $display(\"t6 y=%0d\", y);   /* propagated at t=5 */ \
                 a = 3; \
                 #6 $display(\"t12 y=%0d\", y);  /* new value at t=11 */ \
                 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // y reads high-Z until the delayed driver propagates at t=5, then 7; a=3 at
    // t=6 → y=3 at t=11 (seen at t=12). `%0d` of an all-Z field prints `z` per
    // §21.2.1.2. (vita reads the not-yet-propagated net as Z [LRM unconnected-net];
    // iverilog reads X here — impl-defined for the pre-propagation window. The
    // delay-propagation behavior under test, t6=7/t12=3, is identical.)
    assert_eq!(out, "t2 y=z\nt6 y=7\nt12 y=3\n");
}

// ── bare @(sig) any-change wait blocks until the NEXT change after it arms
//    (no spurious t=0 trigger from another initial's X→init settle). ──────────

#[test]
fn bare_event_control_no_phantom_t0() {
    let src = "module t; reg sig; \
               initial begin sig = 0; #3 sig = 1; #3 sig = 0; end \
               initial begin @(sig); $display(\"c1=%0d\", $time); \
                             @(sig); $display(\"c2=%0d\", $time); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // sig set to 0 at t0 (before @ arms) is NOT a trigger; first change t=3, then t=6.
    assert_eq!(out, "c1=3\nc2=6\n");
}

// ── `reg unsigned` keyword parses + `%0h`/`%0b`/`%0o` suppress leading zeros. ─

#[test]
fn unsigned_keyword_and_min_width_radix() {
    let src = "module t; reg unsigned [7:0] v; \
               initial begin v = 8'd5; \
                 $display(\"%0h %0b %0o | %h %b\", v, v, v, v, v); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // %0* strip leading zeros (5/101/5); plain %h/%b keep full width (05/00000101).
    assert_eq!(out, "5 101 5 | 05 00000101\n");
}

// ── net-declaration initializer is an implicit continuous assign (a driver):
//    `wire x = a & b;` continuously tracks a & b, like `assign x = a & b`. A reg
//    initializer stays a one-time value (sweep gap: decl-init cont-assign). ─────

#[test]
fn net_decl_initializer_is_continuous_assign() {
    let src = "module t; reg [3:0] a, b; \
               wire [3:0] g = a & b; wire [3:0] o = a | b; \
               reg [3:0] r = 4'd9; \
               initial begin a = 4'b1100; b = 4'b1010; \
                 #1 $display(\"g=%b o=%b r=%0d\", g, o, r); \
                 a = 4'b1111; \
                 #1 $display(\"g=%b o=%b\", g, o); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // g=a&b, o=a|b track continuously; r keeps its one-time init 9. After a=1111:
    // g = 1111 & 1010 = 1010, o = 1111 | 1010 = 1111.
    assert_eq!(out, "g=1000 o=1110 r=9\ng=1010 o=1111\n");
}

// ── word-level 4-state bitwise across the 64-bit word boundary: an X in the high
//    word must propagate per-bit (NOT / AND), proving the word-parallel path keeps
//    IEEE x-semantics past bit 63. ──────────────────────────────────────────────

#[test]
fn reduction_wide_xz_word_boundary() {
    // 71-bit vector, all 0 except bit 70 = X. &v sees known-0 → 0 (0 dominates x);
    // |v sees no known-1 but an unknown → x. Masked high bits must not skew either.
    let src = "module t; reg [70:0] v; \
               initial begin v = 71'd0; v[70] = 1'bx; \
                 $display(\"%b %b\", &v, |v); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "0 x\n");
}

#[test]
fn bitwise_wide_xz_word_boundary() {
    // a is 70-bit, all 0 except bit 65 = X. ~a: definite bits → 1, bit65 → x.
    let src = "module t; reg [69:0] a, r; \
               initial begin a = 70'd0; a[65] = 1'bx; r = ~a; \
                 $display(\"%b\", r[66:64]); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // r[66]=1, r[65]=x, r[64]=1.
    assert_eq!(out, "1x1\n");
}

// ── SV `typedef enum {…} name;` — labels become integer constants (0,1,2,…); the
//    enum-typed variable is a 32-bit int; `c = GREEN` assigns 1. Explicit `=expr`
//    sets the running counter (BLUE follows GREEN). ─────────────────────────────

#[test]
fn enum_labels_are_integer_constants() {
    let src = "module t; typedef enum {RED, GREEN, BLUE} color_t; color_t c; \
               initial begin c = GREEN; $display(\"%0d\", c); \
                 c = BLUE; $display(\"%0d\", c); \
                 c = RED;  $display(\"%0d\", c); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "1\n2\n0\n");
}

// ── SV `typedef struct packed {…} name;` — members lay out MSB-first into one
//    flat vector; `s.field` is a constant part-select. First member = high bits. ──

#[test]
fn packed_struct_unknown_field_is_loud() {
    // `s.zzz` where zzz is not a member must be rejected (parse/elaborate error or
    // None IR), NEVER silently accepted as a value. Guards against the desugar
    // quietly falling through to a bogus hierarchical reference.
    let src = "module t; typedef struct packed { logic [7:0] a; } pkt_t; pkt_t s; \
               initial begin s.zzz = 8'h1; $display(\"%h\", s.zzz); $finish; end endmodule";
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    let sink = DiagSink::default();
    let ir = elaborate::elaborate(&su.expect("su"), &sink);
    let elab_err = sink
        .0
        .borrow()
        .iter()
        .any(|d| d.starts_with("Error") || d.starts_with("Fatal"));
    assert!(
        !pe.is_empty() || elab_err || ir.is_none(),
        "unknown struct field s.zzz was silently accepted (parse_ok={}, elab_err={}, ir_some={})",
        pe.is_empty(),
        elab_err,
        ir.is_some()
    );
}

#[test]
fn packed_struct_field_access() {
    // a=[7:0] (high), b=[3:0] (low). total=12. s = {a,b}.
    let src = "module t; typedef struct packed { logic [7:0] a; logic [3:0] b; } pkt_t; \
               pkt_t s; \
               initial begin s.a = 8'hAB; s.b = 4'h5; \
                 $display(\"%h %h %h\", s.a, s.b, s); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    // s.a=AB, s.b=5, whole 12-bit s = AB5.
    assert_eq!(out, "ab 5 ab5\n");
}

#[test]
fn packed_struct_whole_write_field_read() {
    // writing the whole vector then reading fields back: 12'hC34 → a=C3, b=4.
    let src = "module t; typedef struct packed { logic [7:0] a; logic [3:0] b; } pkt_t; \
               pkt_t s; \
               initial begin s = 12'hC34; \
                 $display(\"%h %h\", s.a, s.b); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "c3 4\n");
}

// ── SV `typedef <type> name;` plain alias — `byte_t x;` declares an 8-bit var;
//    width truncation applies exactly as for the underlying type. ──────────────

#[test]
fn typedef_alias_resolves_underlying_width() {
    // byte_t = logic[7:0]: 16'hABCD truncates to 0xCD. nib_t = reg[3:0]: 8'd29 → 13.
    let src = "module t; typedef logic [7:0] byte_t; typedef reg [3:0] nib_t; \
               byte_t x; nib_t y; \
               initial begin x = 16'hABCD; y = 8'd29; \
                 $display(\"%h %0d\", x, y); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "cd 13\n");
}

#[test]
fn enum_explicit_value_advances_counter() {
    // A=0, B=5 (explicit), C=6 (counter resumes from B+1).
    let src = "module t; typedef enum {A, B = 5, C} e_t; e_t v; \
               initial begin v = A; $display(\"%0d\", v); \
                 v = B; $display(\"%0d\", v); \
                 v = C; $display(\"%0d\", v); $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "0\n5\n6\n");
}

// ── P2-12: `time` type = 64-bit unsigned variable (IEEE 1800 §6.11) ──────────

#[test]
fn time_type_is_64bit_unsigned_var() {
    // Scaled $time lands in it; -1 wraps unsigned under %0d; a full 64-bit hex
    // literal round-trips. Expectations = live iverilog output.
    let (ir, opts) = build_timescaled(
        "`timescale 1ns/1ps\nmodule top; time t; initial begin #5 t = $time; \
         $display(\"a=%0d\", t); t = -1; $display(\"b=%0d\", t); \
         t = 64'hFFFF_FFFF_FFFF_FFFF; $display(\"c=%0h\", t); $finish; end endmodule\n",
    );
    let (_res, out) = simulate_capture(&ir, opts);
    assert_eq!(out, "a=5\nb=18446744073709551615\nc=ffffffffffffffff\n");
}

// ── format_version 4: runtime #delay (ExprId amount) ─────────────────────────

#[test]
fn runtime_delay_variable_scales_by_module_timescale() {
    // iverilog (probed live): `#d` with d=3 under 1ns/1ps advances 3000 ticks;
    // `#(d*2)` adds 6000; real `#r` (1.5) adds 1500; X delay adds 0.
    let (ir, opts) = build_timescaled(
        "`timescale 1ns/1ps\nmodule top; integer d; reg [7:0] xd; real r; \
         initial begin \
           d = 3; \
           #d $display(\"a=%0d\", $time); \
           #(d*2) $display(\"b=%0d\", $time); \
           r = 1.5; \
           #r $display(\"c=%0d\", $time); \
           xd = 8'hxx; \
           #xd $display(\"d=%0d\", $time); \
           $finish; \
         end endmodule\n",
    );
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "a=3\nb=9\nc=10\nd=10\n");
    assert_eq!(res.sim_time, 10500);
}

#[test]
fn runtime_delay_loop_terminates() {
    // P1-3 regression shape: `forever #d` with a runtime d must actually
    // advance time (the old #0 degrade spun the delta limit).
    let (ir, opts) = build_timescaled(
        "module top; integer d; reg clk; integer n; \
         initial begin d = 2; clk = 0; n = 0; end \
         always @(posedge clk) begin n = n + 1; if (n == 5) $finish; end \
         initial forever #d clk = ~clk; \
         endmodule\n",
    );
    let (res, _out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(res.sim_time, 18, "5 posedges at #2 toggles end at t=18");
}

// ── format_version 4: $dumpflush / $dumplimit ────────────────────────────────

#[test]
fn dumplimit_stops_dump_with_comment() {
    let dir = std::env::temp_dir().join(format!("vita_dl_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let vcd = dir.join("lim.vcd");
    let src = format!(
        "module top; reg [7:0] a; integer i; \
         initial begin \
           $dumpfile(\"{}\"); $dumpvars; $dumplimit(300); \
           a = 0; \
           for (i = 0; i < 200; i = i + 1) #1 a = a + 1; \
           $finish; \
         end endmodule\n",
        vcd.display()
    );
    let (ir, opts) = build_timescaled(&src);
    let (res, _out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    let body = std::fs::read_to_string(&vcd).expect("vcd written");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        body.contains("$comment Dump limit reached $end"),
        "limit comment expected, got:\n{body}"
    );
    assert!(
        body.len() < 1200,
        "dump must stop near the byte budget (got {} bytes)",
        body.len()
    );
}

#[test]
fn dumpflush_is_accepted_and_vcd_complete() {
    let dir = std::env::temp_dir().join(format!("vita_df_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let vcd = dir.join("fl.vcd");
    let src = format!(
        "module top; reg a; \
         initial begin \
           $dumpfile(\"{}\"); $dumpvars; \
           a = 0; #1 a = 1; \
           $dumpflush; \
           #1 a = 0; \
           $finish; \
         end endmodule\n",
        vcd.display()
    );
    let (ir, opts) = build_timescaled(&src);
    let (res, _out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    let body = std::fs::read_to_string(&vcd).expect("vcd written");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(body.contains("#2"), "post-flush changes must still dump");
}

// ── force/release semantics (format_version 4 follow-up) ────────────────────

#[test]
fn force_release_net_and_variable() {
    // Mirrors the live iverilog probe byte-for-byte: a forced net ignores its
    // driver, a forced reg ignores procedural assigns; release restores the
    // net's driver but a variable KEEPS the forced value until reassigned.
    let (ir, opts) = build_timescaled(
        "module top; wire w; reg a; assign w = a; reg r; \
         initial begin \
           a = 0; r = 0; \
           #1 $display(\"t1 w=%b r=%b\", w, r); \
           force w = 1'b1; \
           force r = 1'b1; \
           #1 $display(\"t2 w=%b r=%b\", w, r); \
           a = 1; r = 0; \
           #1 $display(\"t3 w=%b r=%b\", w, r); \
           release w; \
           release r; \
           #1 $display(\"t4 w=%b r=%b\", w, r); \
           r = 0; \
           #1 $display(\"t5 r=%b\", r); \
           $finish; \
         end endmodule\n",
    );
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(
        out,
        "t1 w=0 r=0\nt2 w=1 r=1\nt3 w=1 r=1\nt4 w=1 r=1\nt5 r=0\n"
    );
}

#[test]
fn force_blocks_nba_and_re_force_wins() {
    let (ir, opts) = build_timescaled(
        "module top; reg [3:0] q; reg clk; \
         always @(posedge clk) q <= q + 1; \
         initial begin \
           q = 0; clk = 0; \
           force q = 4'd9; \
           #1 clk = 1; #1 clk = 0; \
           $display(\"a q=%0d\", q); \
           force q = 4'd5; \
           $display(\"b q=%0d\", q); \
           release q; \
           #1 clk = 1; #1 clk = 0; \
           $display(\"c q=%0d\", q); \
           $finish; \
         end endmodule\n",
    );
    let (_res, out) = simulate_capture(&ir, opts);
    // NBA increments are swallowed while forced; re-force overrides; after
    // release the next posedge increments from the kept value (5 -> 6).
    assert_eq!(out, "a q=9\nb q=5\nc q=6\n");
}

#[test]
fn blocking_intra_assignment_delay_captures_now_writes_later() {
    // iverilog (probed live): `a = #3 b;` evaluates b NOW (5), suspends 3,
    // then writes — a cross-process `#1 b=77` during the suspension must NOT
    // leak into the captured value. $display runs after the write at t=3.
    let ir = build(
        "module tb; reg [7:0] a, b; \
           initial begin \
             b = 8'd5; \
             a = #3 b; \
             $display(\"t=%0d a=%0d b=%0d\", $time, a, b); \
             $finish; \
           end \
           initial #1 b = 8'd77; \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "t=3 a=5 b=77\n");
    assert_eq!(res.sim_time, 3);
}

#[test]
fn blocking_intra_assignment_zero_and_runtime_delay() {
    // `#0` form reschedules in the inactive region (write still lands at t=0);
    // a RUNTIME delay expr (format_version 4) works in the intra form too.
    let ir = build(
        "module tb; reg [7:0] a, c; integer d; \
           initial begin \
             a = #0 8'd9; \
             d = 4; \
             c = #(d) 8'd3; \
             $display(\"t=%0d a=%0d c=%0d\", $time, a, c); \
             $finish; \
           end \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "t=4 a=9 c=3\n");
}

#[test]
fn force_expression_reevaluates_continuously() {
    // IEEE 1364 §9.3.2: a force with an EXPRESSION RHS behaves as a continuous
    // assignment — operand changes re-evaluate and re-pin the target. iverilog
    // diverges here BY ITS OWN ADMISSION ("sorry: ... evaluated once", probed
    // live), so this pins hand-computed IEEE semantics, not iverilog parity:
    //   t2: w = 0^1 = 1, r = 0^1 = 1
    //   t3: a=1 ⇒ both re-evaluate to 1^1 = 0   (sample-once would keep 1)
    //   t4: released — the net snaps to its driver (0), the variable keeps 0.
    let ir = build(
        "module tb; reg a, b; wire w; reg r; \
           assign w = 1'b0; \
           initial begin \
             a = 0; b = 1; r = 0; \
             #1 force w = a ^ b; force r = a ^ b; \
             #1 $display(\"t2 w=%b r=%b\", w, r); \
             a = 1; \
             #1 $display(\"t3 w=%b r=%b\", w, r); \
             release w; release r; \
             #1 $display(\"t4 w=%b r=%b\", w, r); \
             a = 0; \
             #1 $display(\"t5 w=%b r=%b\", w, r); \
             $finish; \
           end \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // t5: released force must NOT fire again (a back to 0): w stays 0 (driver),
    // r keeps its last value 0.
    assert_eq!(out, "t2 w=1 r=1\nt3 w=0 r=0\nt4 w=0 r=0\nt5 w=0 r=0\n");
}

#[test]
fn force_volatile_time_rhs_reevaluates_every_delta() {
    // C-FORCE-REEVAL-p2 TEETH: a `force q = $time;` RHS reads ZERO design nets,
    // yet IEEE 1364 §9.3.2 makes it a continuous assignment — it MUST re-render
    // every delta as time advances. A net-sensitivity reeval optimization that
    // skipped forces whose source nets did not change would FREEZE this force =
    // silent-wrong. The volatile guard (any $time/$random leaf, or zero net
    // reads ⇒ ALWAYS-REEVAL) keeps it live. Hand-IEEE oracle (iverilog re-
    // evaluates a force-RHS once, so it cannot oracle this): q tracks $time.
    let (ir, opts) = build_timescaled(
        "module tb; reg [7:0] q; reg clk; \
           initial begin \
             clk = 0; q = 0; \
             force q = $time; \
             #1 clk = ~clk; \
             $display(\"t1 q=%0d\", q); \
             #1 clk = ~clk; \
             $display(\"t2 q=%0d\", q); \
             #1 clk = ~clk; \
             $display(\"t3 q=%0d\", q); \
             release q; \
             #1 clk = ~clk; \
             $display(\"t4 q=%0d\", q); \
             $finish; \
           end \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // The force re-pin lands in `propagate_changes` AFTER the same-process
    // blocking display, so each `$display` reads the value forced one delta
    // earlier: t1 sees the time-0 pin (0), t2 the time-1 pin (1), t3 the
    // time-2 pin (2). After `release` the value freezes at its last forced
    // pin (the time-2 value, 2) — the teeth is that q KEEPS ADVANCING while
    // forced (0→1→2) despite its RHS reading no design net at all.
    assert_eq!(out, "t1 q=0\nt2 q=1\nt3 q=2\nt4 q=2\n");
}

#[test]
fn force_chain_downstream_force_tracks_upstream_repin() {
    // C-FORCE-REEVAL-p2 CHAIN TEETH: `force x = src; force y = x + 1;` is a
    // force-feeds-force chain — force y's RHS reads net x, which is force x's
    // TARGET. When `src` changes, force x re-pins net x; that re-pin must in turn
    // re-trigger force y so y tracks (x + 1). IEEE 1364 §9.3.2 makes both forces
    // continuous assignments, so the chain settles like `assign x = src; assign
    // y = x + 1;` (iverilog evaluates a force-RHS once, so it cannot oracle this;
    // hand-derived).
    //
    // The defect this guards against: the net-sensitivity reeval ran ONCE per
    // `propagate_changes` and the dirty list it triggered off was then consumed
    // by the sweep — so force x's re-pin dirtied net x but force y was never
    // re-selected, FREEZING y at its stale value (silent-wrong). The fixpoint
    // loop re-selects forces fed by nets a force re-pin changed, within the same
    // call, so y tracks x.
    let ir = build(
        "module tb; reg [7:0] src; wire [7:0] x; wire [7:0] y; \
           initial begin \
             src = 5; \
             force x = src; \
             force y = x + 1; \
             #1 $display(\"t1 x=%0d y=%0d\", x, y); \
             src = 7; \
             #1 $display(\"t2 x=%0d y=%0d\", x, y); \
             #1 $display(\"t3 x=%0d y=%0d\", x, y); \
             $finish; \
           end \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    // t1: x=src=5, y=x+1=6. t2: src→7 re-pins x=7, and the SAME reeval pass
    // re-triggers force y ⇒ y=8 (NOT frozen at 6). t3: steady, x=7 y=8.
    assert_eq!(out, "t1 x=5 y=6\nt2 x=7 y=8\nt3 x=7 y=8\n");
}

#[test]
fn immediate_assert_actions_follow_verilog_truthiness() {
    // Oracle (iverilog -g2012, probed live → p1 f2 f3 f4 p5): an X condition
    // FAILS the assert (if-(x) takes else per IEEE 1800 §16.3), pass/else
    // actions are plain statements, and a PASSING assert with no action block
    // is silent (its synthesized default $error must not run).
    let ir = build(
        "module tb; reg a; reg b; reg [1:0] c; \
           initial begin \
             a = 1; \
             assert (a) $display(\"p1\"); else $display(\"f1\"); \
             a = 0; \
             assert (a) $display(\"p2\"); else $display(\"f2\"); \
             assert (b) $display(\"p3\"); else $display(\"f3\"); \
             assert (a == 0); \
             assert (a | b) else $display(\"f4\"); \
             c = 2'b10; \
             assert (c[1] & ~c[0]) $display(\"p5\"); \
             $finish; \
           end \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "p1\nf2\nf3\nf4\np5\n");
}

#[test]
fn disable_enclosing_block_is_break() {
    // Oracle (iverilog, probed live): `disable L` from inside terminates the
    // whole named block L — loop AND tail are abandoned (break idiom).
    let ir = build(
        "module tb; integer i; \
           initial begin : L \
             for (i = 0; i < 10; i = i + 1) begin \
               if (i == 3) disable L; \
               $display(\"i=%0d\", i); \
             end \
             $display(\"tail\"); \
           end \
           initial #1 begin $display(\"done\"); $finish; end \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "i=0\ni=1\ni=2\ndone\n");
}

#[test]
fn disable_loop_body_block_is_continue() {
    // Oracle (iverilog, probed live): disabling the per-iteration named block
    // skips the REST of that iteration only — the for loop keeps stepping.
    let ir = build(
        "module tb; integer i; \
           initial begin \
             for (i = 0; i < 5; i = i + 1) begin : ITER \
               if (i == 2) disable ITER; \
               $display(\"i=%0d\", i); \
             end \
             $display(\"end\"); \
             $finish; \
           end \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "i=0\ni=1\ni=3\ni=4\nend\n");
}

#[test]
fn disable_outer_from_inner_skips_both_tails() {
    // Oracle (iverilog, probed live): `disable OUTER` from INNER abandons the
    // inner remainder AND the outer remainder in one jump.
    let ir = build(
        "module tb; \
           initial begin \
             begin : OUTER \
               begin : INNER \
                 disable OUTER; \
                 $display(\"x1\"); \
               end \
               $display(\"x2\"); \
             end \
             $display(\"after\"); \
             $finish; \
           end \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "after\n");
}

#[test]
fn proc_assign_pins_and_deassign_holds() {
    // Oracle (iverilog, probed live): while `assign q = 42` is active an
    // ordinary procedural write is overridden; `deassign` HOLDS the value
    // (variable semantics); afterwards ordinary writes work again.
    let (ir, opts) = build_timescaled(
        "module tb; reg [7:0] q; \
           initial begin \
             q = 8'd1; \
             $display(\"t0 q=%0d\", q); \
             assign q = 8'd42; \
             #1 q = 8'd5; \
             $display(\"t1 q=%0d\", q); \
             deassign q; \
             $display(\"t2 q=%0d\", q); \
             q = 8'd7; \
             $display(\"t3 q=%0d\", q); \
             $finish; \
           end \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "t0 q=1\nt1 q=42\nt2 q=42\nt3 q=7\n");
}

#[test]
fn force_overrides_assign_release_resumes_it() {
    // Oracle (iverilog, probed live): force WINS over an active proc-assign;
    // release hands control BACK to the assign (latent re-pin); deassign then
    // frees the variable for ordinary writes.
    let (ir, opts) = build_timescaled(
        "module tb; reg [7:0] q; \
           initial begin \
             assign q = 8'd10; \
             $display(\"a q=%0d\", q); \
             force q = 8'd20; \
             $display(\"b q=%0d\", q); \
             release q; \
             $display(\"c q=%0d\", q); \
             deassign q; \
             q = 8'd30; \
             $display(\"d q=%0d\", q); \
             $finish; \
           end \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "a q=10\nb q=20\nc q=10\nd q=30\n");
}

#[test]
fn proc_assign_expression_reevaluates_continuously() {
    // IEEE 1364 §9.3.1: a proc-assign with an expression RHS behaves as a
    // continuous assignment until deassigned. iverilog DIVERGES by its own
    // admission ("sorry: ... evaluated once"), so this lane is pinned by hand
    // against the LRM (the force-expression precedent); const-rhs lanes keep
    // iverilog differential parity.
    let (ir, opts) = build_timescaled(
        "module tb; reg [7:0] y; reg [7:0] a; \
           initial begin \
             a = 8'd1; \
             assign y = a + 8'd1; \
             $display(\"p y=%0d\", y); \
             #1 a = 8'd9; \
             #1 $display(\"q y=%0d\", y); \
             $finish; \
           end \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, opts);
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "p y=2\nq y=10\n");
}

#[test]
fn nba_transport_basic_lands_in_nba_region() {
    // Oracle (iverilog, probed live): `q <= #3 v` lands in the NBA region of
    // t=3 — an ACTIVE-region read at t=3 still sees the old value (all four
    // prints show q=1; the write applies after the last print).
    let ir = build(
        "module tb; reg [7:0] q; \
           initial begin \
             q = 8'd1; \
             q <= #3 8'd9; \
             $display(\"t%0d q=%0d\", $time, q); \
             #1 $display(\"t%0d q=%0d\", $time, q); \
             #1 $display(\"t%0d q=%0d\", $time, q); \
             #1 $display(\"t%0d q=%0d\", $time, q); \
             #1 $finish; \
           end \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "t0 q=1\nt1 q=1\nt2 q=1\nt3 q=1\n");
}

#[test]
fn nba_transport_overlapping_activations_carry_own_values() {
    // Oracle (iverilog, probed live): the transport case that forced the v5
    // shape — three activations are IN FLIGHT at once and each delivers ITS
    // OWN captured d (a static capture net would deliver the latest d).
    // $finish is at #13, NOT #12: a same-tick tie between an Active-region
    // $finish and a due transport update is a tool-divergence zone (vvp
    // applies the update slot-top; the LRM puts it in the NBA region AFTER
    // active, so finish wins) — the design sidesteps the tie.
    let ir = build(
        "module tb; reg clk; reg [7:0] d, q; \
           initial begin clk = 0; d = 8'd1; end \
           always #1 clk = ~clk; \
           always @(posedge clk) begin \
             q <= #3 d; \
             d <= d + 8'd1; \
           end \
           initial #13 $finish; \
           always @(q) $display(\"t%0d q=%0d\", $time, q); \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "t4 q=1\nt6 q=2\nt8 q=3\nt10 q=4\nt12 q=5\n");
}

#[test]
fn nba_zero_delay_keeps_statement_order() {
    // Oracle (iverilog, probed live): `<= #0` joins the SAME tick's NBA queue
    // in statement order — the later plain `<=` wins.
    let ir = build(
        "module tb; reg [7:0] q; \
           initial begin \
             q = 8'd1; \
             q <= #0 8'd5; \
             q <= 8'd7; \
             #1 $display(\"t1 q=%0d\", q); \
             $finish; \
           end \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "t1 q=7\n");
}

#[test]
fn nba_transport_index_sampled_at_schedule() {
    // Oracle (iverilog, probed live): `mem[i] <= #2 v` freezes the index at
    // schedule time — a later i change must not move the target.
    let ir = build(
        "module tb; reg [7:0] mem [0:3]; integer i; \
           initial begin \
             mem[0] = 8'd0; mem[1] = 8'd0; mem[2] = 8'd0; mem[3] = 8'd0; \
             i = 1; \
             mem[i] <= #2 8'd42; \
             i = 3; \
             #3 $display(\"m1=%0d m3=%0d\", mem[1], mem[3]); \
             $finish; \
           end \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "m1=42 m3=0\n");
}

#[test]
fn named_event_trigger_wakes_waiter() {
    // Oracle (iverilog, probed live): `->e` wakes every `@(e)` waiter at the
    // trigger time; an event has no value and no latch.
    let ir = build(
        "module tb; event e; \
           initial begin \
             #1 -> e; \
             #2 -> e; \
             #2 $finish; \
           end \
           always @(e) $display(\"t%0d fired\", $time); \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "t1 fired\nt3 fired\n");
}

#[test]
fn named_event_in_mixed_sensitivity_list() {
    // Oracle (iverilog, probed live): `@(posedge clk or e)` wakes on both.
    // $finish sits at t8 (no clk edge): a same-tick tie between an Active
    // $finish and a same-slot edge wake is a tool-ordering divergence zone
    // (vvp runs the woken waiter first; our region order runs finish first).
    let ir = build(
        "module tb; event e; reg clk; \
           initial clk = 0; \
           always #1 clk = ~clk; \
           initial begin \
             #4 -> e; \
             #4 $finish; \
           end \
           always @(posedge clk or e) $display(\"t%0d wake\", $time); \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "t1 wake\nt3 wake\nt4 wake\nt5 wake\nt7 wake\n");
}

#[test]
fn named_event_trigger_before_arm_is_lost() {
    // Oracle (iverilog, probed live): a trigger fired BEFORE the waiter arms
    // is LOST (events are instantaneous, never latched) — only t1 is caught.
    let ir = build(
        "module tb; event e; \
           initial begin \
             -> e; \
             #1 -> e; \
             #1 $finish; \
           end \
           initial begin \
             #0 @(e) $display(\"t%0d caught\", $time); \
           end \
         endmodule",
    );
    let (res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(res.finish_reason, FinishReason::Finish);
    assert_eq!(out, "t1 caught\n");
}
