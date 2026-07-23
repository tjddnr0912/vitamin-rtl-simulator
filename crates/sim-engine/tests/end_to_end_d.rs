//! End-to-end sim-engine tests: build a SimIr via the real lex → parse →
//! elaborate pipeline, simulate it, and assert on captured $display output and
//! the generated VCD file.

use sim_engine::{simulate_capture, FinishReason, SimOpts};

#[path = "end_to_end_util/mod.rs"]
mod util;
#[allow(unused_imports)]
use util::*;

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
    // `#(d*2)` adds 6000; real `#r` (1.5) adds 1500; X delay adds 0. At t=10.5ns
    // $time ROUNDS 10.5→11 (half-up, §20.3.1; iverilog-pinned c=d=11, NOT 10).
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
    assert_eq!(out, "a=3\nb=9\nc=11\nd=11\n");
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
