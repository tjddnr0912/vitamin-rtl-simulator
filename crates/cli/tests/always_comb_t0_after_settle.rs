//! §4.5.584 — an `always_comb` / `always_latch` time-0 pass waits for its inputs.
//!
//! IEEE 1800-2017 §9.2.2.2: `always_comb` "is automatically triggered once at time zero,
//! after all initial and always procedures have been started"; §9.2.2.3 gives
//! `always_latch` the same rules (wording from secondary sources; the LRM text was not
//! read). vita ran that pass in the FIRST time-0 batch, beside the `initial` bodies, so it
//! read every net an `initial` or a continuous assign had not written yet: a W4031 / E4003
//! at time 0 that neither oracle prints, an exit 1 on a clean `assert`, an x→0→1 glitch at
//! `@(negedge y)`.
//!
//! The rule now (both executors and tier-3, one spelling each):
//! - the block is ARMED at seeding, as a level block is, so a first-batch write or the
//!   time-0 settle wakes it like any later change;
//! - a block the time-0 settle woke runs after the first batch, ONE PER BATCH, each pass
//!   followed by a continuous-assign settle, before the batch the settle's other wakes
//!   lead;
//! - its implicit pass runs at the first Inactive promotion at time 0, again one per
//!   batch with a settle after each, before the promoted `#0` resumes — as a TRIGGER:
//!   for a block idle at its top (its waiter armed; starting it consumes the waiter),
//!   or IN PLACE OF the pending wake of a block woken and not yet run (one activation,
//!   in the trigger's slot); a block running, suspended inside, or stopped at
//!   `$finish` gets none.
//!
//! A combinational UDP is not an `always_comb`: its table runs in the first batch and on
//! every input change, as before the slice.
//!
//! Oracles: Icarus Verilog 13.0 (`iverilog -g2012`, `vvp -n`) and Verilator 5.052
//! (`--binary --timing --assert`). Verilator is no oracle for an `x` (2-state: it prints
//! `00` where an unwritten variable is `xx`) nor for how many times a block runs (it
//! re-evaluates a block until it settles and re-reports a violation every evaluation —
//! twice per step, plus the step after); its REPORT PRESENCE at a time is used, and its
//! values where no `x` is involved. Every expected text below is vita's output; the oracle
//! lines quoted above it are raw (`iv:` / `vl:`), from the slice's grounding cells.
//!
//! Every design runs on native, interp and vm and must be byte-identical across them
//! (stdout, diagnostics, exit code, VCD); three also run the staged chain (`vita vcmp` →
//! `velab` → `vrun`), which must match the one-shot run.
//!
//! Splits (the oracles disagree, pinned at vita's value with both quoted), the residue
//! (both oracles silent, vita still reports — a consumer block written before the block
//! feeding it, no process write at time 0: needs an order among the time-0 passes,
//! iverilog's reverse source order or verilator's dependency order) and the processes the
//! rule must not move are pinned in `always_comb_t0_splits.rs`.
#[path = "always_comb_t0_util/mod.rs"]
mod util;
use util::*;
// ── the gap: a report or a value both oracles do not give ─────────────────────────

/// A `unique` / `priority` case in a block written before the `initial` that drives it,
/// plus an `always_latch`: one report, at the real miss (time 2), none at time 0.
const P1: &str = "module top;
  logic [1:0] s, y;
  always_comb begin
    y = 0;
    unique case (s) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
  initial begin
    s = 2'd1;
    #1 $display(\"t=%0t y=%0d\", $time, y);
    #1 s = 2'd0;
    #1 $display(\"t=%0t y=%0d\", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
";

#[test]
fn a_block_before_its_driver_reports_only_the_real_miss() {
    // iv: t=1 y=1 / WARNING: a_uc_combfirst.sv:5: value is unhandled for priority or
    //     unique case statement / Time: 2  Scope: top / t=3 y=0
    // vl: t=1 y=1 / [2] %Error: a_uc_combfirst.sv:5: Assertion failed in top: unique case,
    //     but none matched for '2'h0' / t=3 y=0      (first report at [2])
    // PRE: W4031 [in top] [at time 0] and [at time 2].
    let r = sim(P1);
    assert_out(&r, "t=1 y=1\nt=3 y=0\n", 4);
    assert_eq!(r.diags, ["5:12 W4031 [in top] [at time 2]"]);
    assert_eq!(r.code, 0);
    staged_matches(P1, &r);

    // `priority case`. iv: WARNING …:5 … Time: 2  Scope: top; vl: [2] %Error: …:5:
    // Assertion failed in top: priority case, but non-match found for '2'h0'.
    let r = sim(&P1.replace("unique case", "priority case"));
    assert_out(&r, "t=1 y=1\nt=3 y=0\n", 4);
    assert_eq!(r.diags, ["5:14 W4031 [in top] [at time 2]"]);

    // `always_latch` (no default assignment, so `y` holds 1). iv: t=1 y=1 / WARNING
    // a_lc_combfirst.sv:4 … Time: 2  Scope: top / t=3 y=1; vl: t=1 y=1 / [2] %Error: …:4:
    // Assertion failed in top: unique case, but none matched for '2'h0' / t=3 y=1.
    let latch = "module top;
  logic [1:0] s, y;
  always_latch begin
    unique case (s) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
  initial begin
    s = 2'd1;
    #1 $display(\"t=%0t y=%0d\", $time, y);
    #1 s = 2'd0;
    #1 $display(\"t=%0t y=%0d\", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
";
    let r = sim(latch);
    assert_out(&r, "t=1 y=1\nt=3 y=1\n", 4);
    assert_eq!(r.diags, ["4:12 W4031 [in top] [at time 2]"]);

    // A single `unique if`. iverilog rejects it (`syntax error`), so verilator alone:
    // t=1 y=1 / [2] %Error: sa_ui_combfirst.sv:5: Assertion failed in top: 'unique if'
    // statement violated / t=3 y=0.
    let ui = P1.replace(
        "unique case (s) 2'd1: y = 1; 2'd2: y = 2; endcase",
        "unique if (s == 2'd1) y = 1;",
    );
    let r = sim(&ui);
    assert_out(&r, "t=1 y=1\nt=3 y=0\n", 4);
    assert_eq!(r.diags, ["5:12 W4031 [in top] [at time 2]"]);
}

/// The same block in a child, its input a port the testbench drives.
const P2: &str = "module top;
  logic [1:0] s, y;
  leaf u(.s(s), .y(y));
  initial begin
    s = 2'd1;
    #1 $display(\"t=%0t y=%0d\", $time, y);
    #1 s = 2'd0;
    #1 $display(\"t=%0t y=%0d\", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
module leaf(input wire [1:0] s, output logic [1:0] y);
  always_comb begin
    y = 0;
    unique case (s) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
endmodule
";

#[test]
fn a_block_behind_a_port_reports_only_the_real_miss() {
    // iv: t=1 y=1 / WARNING: b_uc_w_d1_tbfirst.sv:16: … / Time: 2  Scope: top.u / t=3 y=0
    // vl: t=1 y=1 / [2] %Error: b_uc_w_d1_tbfirst.sv:16: Assertion failed in top.u: unique
    //     case, but none matched for '2'h0' / t=3 y=0
    // PRE: W4031 [in top.u] [at time 0] and [at time 2].
    let r = sim(P2);
    assert_out(&r, "t=1 y=1\nt=3 y=0\n", 4);
    assert_eq!(r.diags, ["16:12 W4031 [in top.u] [at time 2]"]);
    staged_matches(P2, &r);

    // Three ports deep, the leaf written first. iv: WARNING b_uc_v_d3_leaffirst.sv:4 …
    // Time: 2  Scope: top.u.u.u; vl: [2] %Error: …:4: Assertion failed in top.u.u.u: ….
    let deep = "module leaf(input logic [1:0] s, output logic [1:0] y);
  always_comb begin
    y = 0;
    unique case (s) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
endmodule
module m1(input logic [1:0] s, output logic [1:0] y);
  leaf u(.s(s), .y(y));
endmodule
module m2(input logic [1:0] s, output logic [1:0] y);
  m1 u(.s(s), .y(y));
endmodule
module top;
  logic [1:0] s, y;
  m2 u(.s(s), .y(y));
  initial begin
    s = 2'd1;
    #1 $display(\"t=%0t y=%0d\", $time, y);
    #1 s = 2'd0;
    #1 $display(\"t=%0t y=%0d\", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
";
    let r = sim(deep);
    assert_out(&r, "t=1 y=1\nt=3 y=0\n", 4);
    assert_eq!(r.diags, ["4:12 W4031 [in top.u.u.u] [at time 2]"]);

    // A block → port → block chain. iv: WARNING q_hchain.sv:11 … Time: 2  Scope: top.u
    // (only); vl: first report [2] %Error: q_hchain.sv:11: Assertion failed in top.u: ….
    let hchain = "module top;
  logic [1:0] s, x, y;
  always_comb x = s;
  leafh u(.s(x), .y(y));
  initial begin s = 2'd1; #2 s = 2'd0; #1 $finish; end
  initial #100 $finish;
endmodule
module leafh(input logic [1:0] s, output logic [1:0] y);
  always_comb begin
    y = 0;
    unique case (s) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
endmodule
";
    let r = sim(hchain);
    assert_out(&r, "", 3);
    assert_eq!(r.diags, ["11:12 W4031 [in top.u] [at time 2]"]);

    // Generate-scope blocks. iv: WARNING q_gen_comb_first.sv:6 … Time: 2  Scope: top.g[0]
    // (only); vl: first report [2] %Error: …:6: Assertion failed in top.g[0]: ….
    let gen = "module top;
  logic [1:0] s [2], y [2];
  for (genvar i = 0; i < 2; i++) begin : g
    always_comb begin
      y[i] = 0;
      unique case (s[i]) 2'd1: y[i] = 1; 2'd2: y[i] = 2; endcase
    end
  end
  initial begin s[0] = 2'd1; s[1] = 2'd2; #2 s[0] = 2'd0; #1 $finish; end
  initial #100 $finish;
endmodule
";
    let r = sim(gen);
    assert_out(&r, "", 3);
    assert_eq!(r.diags, ["6:14 W4031 [in top.g[0]] [at time 2]"]);
}

#[test]
fn an_immediate_assert_in_a_block_fails_only_on_a_real_violation() {
    // No violation at all. iv: t=1 y=1, run rc=0, no ERROR; vl: t=1 y=1, run rc=0.
    // PRE: rc=1, `error[VITA-E4003] E-RUN-USER-ERROR: Assertion failed [in top] [at time 0]`.
    let clean = "module top;
  logic [1:0] s, y;
  always_comb begin
    y = s;
    assert (s == 2'd1 || s == 2'd2);
  end
  initial begin
    s = 2'd1;
    #1 $display(\"t=%0t y=%0d\", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
";
    let r = sim(clean);
    assert_out(&r, "t=1 y=1\n", 2);
    assert!(r.diags.is_empty(), "{r:?}");
    assert_eq!(r.code, 0);
    // A real violation at time 2. iv: t=1 y=1 / ERROR: q_assert_combfirst.sv:5: / Time: 2
    // Scope: top; vl: t=1 y=1 / [2] %Error: q_assert_combfirst.sv:5: Assertion failed in
    // top: 'assert' failed.  PRE: E4003 at time 0 and at time 2.
    let viol = clean.replace("#1 $finish;\n  end", "#1 s = 2'd0;\n    #1 $finish;\n  end");
    let r = sim(&viol);
    assert_eq!(r.diags, ["5:5 E4003 [in top] [at time 2]"]);
    assert_eq!(r.code, 1);
}

/// `y` settles once at time 0: no x→0 negedge, one `@(y)` wake.
const P10: &str = "module top;
  logic s, y;
  int nneg = 0, npos = 0, nany = 0;
  always @(negedge y) begin nneg++; $display(\"NEG t=%0t\", $time); end
  always @(posedge y) begin npos++; $display(\"POS t=%0t\", $time); end
  always @(y) nany++;
  initial begin wait (y === 1'b0); $display(\"WAIT0 released t=%0t\", $time); end
  always_comb y = (s === 1'b1);
  initial begin
    s = 1'b1;
    #1 $display(\"t=%0t y=%b neg=%0d pos=%0d any=%0d\", $time, y, nneg, npos, nany);
    $finish;
  end
  initial #100 $finish;
endmodule
";

#[test]
fn a_block_makes_no_time_zero_glitch_for_edge_and_level_watchers() {
    // iv: POS t=0 / t=1 y=1 neg=0 pos=1 any=1
    // vl: WAIT0 released t=0 / POS t=0 / t=1 y=1 neg=0 pos=1 any=1 (WAIT0: 2-state `y`
    //     starts 0, no oracle)
    // PRE: NEG t=0 / WAIT0 released t=0 / POS t=0 / t=1 y=1 neg=1 pos=1 any=2
    let r = sim(P10);
    assert_out(&r, "POS t=0\nt=1 y=1 neg=0 pos=1 any=1\n", 1);
    assert!(r.diags.is_empty(), "{r:?}");
    staged_matches(P10, &r);
    // The waveform shows the same single settle — `y` goes x→1 at time 0, with no 0 in
    // between — and is byte-identical across the three executors.
    let dumped = P10.replace(
        "  initial #100 $finish;",
        "  initial begin $dumpfile(\"d.vcd\"); $dumpvars(0, top); end\n  initial #100 $finish;",
    );
    let (r, vcd) = sim_vcd(&dumped);
    assert_out(&r, "POS t=0\nt=1 y=1 neg=0 pos=1 any=1\n", 1);
    let vcd = String::from_utf8(vcd.expect("d.vcd written")).unwrap();
    let id = vcd
        .lines()
        .find(|l| l.starts_with("$var") && l.contains(" y $end"))
        .and_then(|l| l.split_whitespace().nth(3))
        .expect("y's VCD id")
        .to_string();
    let y_changes: Vec<&str> = vcd
        .lines()
        .filter(|l| l.len() > 1 && l[1..] == *id && !l.starts_with('$'))
        .collect();
    assert_eq!(y_changes, [format!("x{id}"), format!("1{id}")], "{vcd}");
}

#[test]
fn a_first_slice_read_sees_the_block_unrun_and_a_hash0_read_sees_it_run() {
    // iv: r0 y=xx / r1 y=01 / r2 y=01 / r3 y=01;  vl: r0 y=00 (2-state) / r1..r3 y=01
    // PRE: W4031 [at time 0] and r0 y=00 — a value neither oracle prints.
    let src = "module top;
  logic [1:0] s, y;
  always_comb begin
    y = 0;
    unique case (s) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
  initial begin
    s = 2'd1;
    $display(\"r0 y=%b\", y);
    #0 $display(\"r1 y=%b\", y);
    #0 $display(\"r2 y=%b\", y);
    #0 $display(\"r3 y=%b\", y);
  end
  initial #1 $finish;
endmodule
";
    let r = sim(src);
    assert_out(&r, "r0 y=xx\nr1 y=01\nr2 y=01\nr3 y=01\n", 1);
    assert!(r.diags.is_empty(), "{r:?}");
}

/// A block whose input settles from a constant (a continuous assign, a constant port
/// tie) runs, alone, after the first batch — so a reader that batch woke reads its output
/// through a continuous assign or a port. Holding it at the front of the reader's batch
/// lost that hop (`v=xx`, `o=x`), which is why the passes take one batch each.
#[test]
fn a_reader_the_first_batch_woke_reads_the_block_through_an_assign_or_a_port() {
    // iv: b2 t=0 v=01 y=01 / e t=1 v=01 y=01;  vl: identical;  PRE: identical.
    let ca = "module top;
  wire [1:0] w;
  logic [1:0] y;
  wire [1:0] v;
  logic t;
  assign w = 2'd1;
  assign v = y;
  always_comb y = w;
  initial t = 1'b1;
  always @(t) $display(\"b2 t=%0t v=%b y=%b\", $time, v, y);
  initial #1 begin $display(\"e t=%0t v=%b y=%b\", $time, v, y); $finish; end
endmodule
";
    let r = sim(ca);
    assert_out(&r, "b2 t=0 v=01 y=01\ne t=1 v=01 y=01\n", 1);
    // iv: b2 t=0 o=1 / e t=1 o=1;  vl: identical;  PRE: identical.
    let port = "module top;
  wire o;
  logic t;
  child u(.a(1'b0), .o(o));
  initial t = 1'b1;
  always @(t) $display(\"b2 t=%0t o=%b\", $time, o);
  initial #1 begin $display(\"e t=%0t o=%b\", $time, o); $finish; end
endmodule
module child(input logic a, output logic o);
  always_comb o = ~a;
endmodule
";
    let r = sim(port);
    assert_out(&r, "b2 t=0 o=1\ne t=1 o=1\n", 1);
}

/// A combinational UDP is a primitive with an output from the start: its table runs in
/// the first time-0 batch (an `initial`) and on every input change (`always @(inputs)`),
/// never through `always_comb`'s implicit pass. As the late `always_comb` pass it read
/// `x` in every reader before the first promotion when its input was set only by a
/// declaration initializer (review r1 D1–D4). Every value here equals PRE's.
const INV: &str = "primitive inv(o, a); output o; input a; table 0:1; 1:0; endtable endprimitive\n";

#[test]
fn a_combinational_udp_evaluates_in_the_first_batch() {
    let b2 = |decl: &str, inst: &str| {
        format!(
            "{INV}module top;
  wire o;
  logic t;
{decl}
  {inst}
  initial t = 1'b1;
  always @(t) $display(\"b2 t=%0t o=%b\", $time, o);
  initial #1 begin $display(\"e t=%0t o=%b\", $time, o); $finish; end
endmodule
"
        )
    };
    // Input from a declaration initializer, read by a batch-2 process.
    // iv: b2 t=0 o=1 / e t=1 o=1;  vl: identical;  PRE: identical;  r1-A: b2 t=0 o=x.
    let r = sim(&b2("  logic k = 1'b0;", "inv u(o, k);"));
    assert_out(&r, "b2 t=0 o=1\ne t=1 o=1\n", 1);
    // …through a continuous assign. iv: b2 t=0 o=1;  vl: same;  PRE: same;  r1-A: o=x.
    let r = sim(&b2("  logic k = 1'b0;\n  wire kw = k;", "inv u(o, kw);"));
    assert_out(&r, "b2 t=0 o=1\ne t=1 o=1\n", 1);
    // …a constant tie (control). iv: b2 t=0 o=1;  vl: same;  PRE = r1-A: same.
    let r = sim(&b2("", "inv u(o, 1'b0);"));
    assert_out(&r, "b2 t=0 o=1\ne t=1 o=1\n", 1);
    // …a localparam (control). iv: b2 t=0 o=0;  vl: same;  PRE = r1-A: same.
    let r = sim(&b2("  localparam logic P = 1'b1;", "inv u(o, P);"));
    assert_out(&r, "b2 t=0 o=0\ne t=1 o=0\n", 1);
    // In a child, its output read through the child's port.
    // iv: b2 t=0 o=0 / e t=1 o=0;  vl: same;  PRE: same;  r1-A: b2 t=0 o=x.
    let child = format!(
        "{INV}module ch(output wire o);
  logic k = 1'b1;
  inv u(o, k);
endmodule
module top;
  wire o;
  logic t;
  ch c(.o(o));
  initial t = 1'b1;
  always @(t) $display(\"b2 t=%0t o=%b\", $time, o);
  initial #1 begin $display(\"e t=%0t o=%b\", $time, o); $finish; end
endmodule
"
    );
    assert_out(&sim(&child), "b2 t=0 o=0\ne t=1 o=0\n", 1);
    // A flop sampling it on a time-0 posedge. iv: t=1 q=1 o=1;  vl: same;  PRE: same;
    // r1-A: t=1 q=x o=1 (the flop captured the unrun table's x).
    let ff = format!(
        "{INV}module top;
  wire o;
  logic k = 1'b0;
  logic clk, q;
  inv u(o, k);
  always @(posedge clk) q <= o;
  initial begin clk = 1'b1; #1 $display(\"t=%0t q=%b o=%b\", $time, q, o); $finish; end
endmodule
"
    );
    assert_out(&sim(&ff), "t=1 q=1 o=1\n", 1);
    // Fed through a gate primitive, sampled the same way (control).
    // iv: t=1 n=0 o=1 qf=1;  vl: same;  PRE = r1-A: same.
    let gate = format!(
        "{INV}module top;
  reg k = 1'b1;
  wire n, o, q;
  reg clk;
  reg qf;
  not g(n, k);
  inv u(o, n);
  always @(posedge clk) qf <= o;
  initial begin clk = 1'b1; #1 $display(\"t=%0t n=%b o=%b qf=%b\", $time, n, o, qf); $finish; end
endmodule
"
    );
    assert_out(&sim(&gate), "t=1 n=0 o=1 qf=1\n", 1);
    // A `#0` reader (control). iv: z t=0 o=1;  vl: same;  PRE = r1-A: same.
    let hash0 = format!(
        "{INV}module top;
  wire o;
  logic k = 1'b0;
  inv u(o, k);
  initial begin #0 $display(\"z t=%0t o=%b\", $time, o); #1 $display(\"e t=%0t o=%b\", $time, o); $finish; end
endmodule
"
    );
    assert_out(&sim(&hash0), "z t=0 o=1\ne t=1 o=1\n", 1);
    // RESIDUE, pre-existing: the first slice of an `initial` (a parent process, so ahead
    // of the child's table in the first batch) reads the table unrun.
    // iv: i0 t=0 o=1;  vl: i0 t=0 o=1;  PRE: i0 t=0 o=x;  r1-A: o=x — unchanged.
    let first_slice = format!(
        "{INV}module top;
  wire o;
  logic k = 1'b0;
  inv u(o, k);
  initial begin $display(\"i0 t=%0t o=%b\", $time, o); #1 $display(\"e t=%0t o=%b\", $time, o); $finish; end
endmodule
"
    );
    assert_out(&sim(&first_slice), "i0 t=0 o=x\ne t=1 o=1\n", 1);
}

/// The implicit pass is a TRIGGER: it reaches a block only while the block is idle at
/// its top (its static waiter armed), and starting any activation consumes the waiter,
/// re-armed when the body returns. A body that stopped at `$finish`, or that is
/// suspended inside a timing control, gets no second activation (review r1 F1, F1b:
/// r1-A re-entered both).
#[test]
fn the_implicit_pass_reaches_only_a_block_idle_at_its_top() {
    // `initial a = 1;` declared first; the block finishes on its batch-2 run.
    // iv: C t=0 a=1 (once);  vl: C t=0 a=1 (once);  PRE: same;  r1-A: C t=0 a=1 twice.
    let fin = "module top;
  logic a, y;
  initial a = 1;
  always_comb begin y = a; $display(\"C t=%0t a=%b\", $time, a); if (a) $finish; end
endmodule
";
    assert_out(&sim(fin), "C t=0 a=1\n", 0);
    // Without the `$finish` the implicit pass does run (the count split): iv: C t=0 a=1
    // twice;  vl: once;  PRE: once.
    let nofin = "module top;
  logic a, y;
  initial a = 1;
  always_comb begin y = a; $display(\"C t=%0t a=%b\", $time, a); end
  initial #5 $finish;
endmodule
";
    assert_out(&sim(nofin), "C t=0 a=1\nC t=0 a=1\n", 5);
    // The `$finish` body's side effect, and `final`: iv: FIN t=0 / final;  vl: same;
    // PRE: same;  r1-A: FIN t=0 twice.
    let side = "module top;
  logic a, y;
  int fd;
  initial a = 1;
  always_comb begin y = a; if (a) begin $display(\"FIN t=%0t\", $time); $finish; end end
  final $display(\"final\");
endmodule
";
    let r = sim(side);
    assert_eq!(
        r.out, "FIN t=0\nfinal\nsimulation ended (Finish) at time 0\n",
        "{r:?}"
    );
    // Suspended inside its body (a `#1` / a `wait`, which vita and verilator accept in
    // an `always_comb` and iverilog rejects; verilator re-runs a block that writes `n`
    // until it settles, so it is no oracle for the count). One activation at a time, as
    // PRE: PRE prints `C t=0 a=x n=1` (it ran before the `initial`), vita now `a=0`
    // (verilator's first line); the rest is PRE's text. r1-A: `C t=0 … n=1`, `C t=0 …
    // n=2`, both `D` lines at n=2 — a second activation over the suspended one.
    let delay = "module top;
  logic a, y; int n;
  always_comb begin n = n + 1; y = a; $display(\"C t=%0t a=%b n=%0d\", $time, a, n); #1; $display(\"D t=%0t n=%0d\", $time, n); end
  initial begin a = 0; #5 a = 1; #5 $finish; end
endmodule
";
    assert_out(
        &sim(delay),
        "C t=0 a=0 n=1\nD t=1 n=1\nC t=5 a=1 n=2\nD t=6 n=2\n",
        10,
    );
    let wait = "module top;
  logic a, b, y; int n;
  always_comb begin n = n + 1; y = a; $display(\"C t=%0t a=%b n=%0d\", $time, a, n); wait (b); $display(\"D t=%0t n=%0d\", $time, n); end
  initial begin a = 0; b = 0; #5 a = 1; #2 b = 1; #5 $finish; end
endmodule
";
    assert_out(&sim(wait), "C t=0 a=0 n=1\nD t=7 n=1\n", 12);
    // The implicit pass itself suspends; a change while it is suspended starts nothing
    // (its waiter was consumed when it started). PRE: C t=0 a=x n=1 / D t=2 n=1 (=
    // vita);  r1-A: a second activation at t=1 (`C t=1 a=1 n=2`) from the live seeding
    // waiter.
    let refire = "module top;
  logic a; int n;
  always_comb begin n = n + 1; $display(\"C t=%0t a=%b n=%0d\", $time, a, n); #2; $display(\"D t=%0t n=%0d\", $time, n); end
  initial begin #1 a = 1; #5 $finish; end
endmodule
";
    assert_out(&sim(refire), "C t=0 a=x n=1\nD t=2 n=1\n", 6);
}

#[test]
fn a_block_fed_through_a_port_from_a_declaration_initialised_block_is_silent() {
    // Top `always_comb x = a;` (`a = 2'd1` by its declaration) → port → child `unique
    // case`. iv: L t=0 s=01 / L t=2 s=00 / WARNING rh_declinit_port_child.sv:14 … Time: 2
    // Scope: top.u;  vl: L t=0 s=01 / L t=2 s=00 / [2] %Error: …:14: Assertion failed in
    // top.u: …;  PRE: L t=0 s=xx / W4031 [in top.u] [at time 0] / L t=0 s=01 / ….
    // The child's block runs once at time 0, as on both oracles: the top block's
    // implicit pass settles the port, which wakes the child (its waiter consumed), so
    // the child's own implicit pass runs in place of that pending wake — one run
    // (r1-A, which delivered the pass and the wake both, printed `L t=0 s=01` twice).
    let src = "module top;
  logic [1:0] a = 2'd1;
  logic [1:0] x;
  wire [1:0] y;
  always_comb x = a;
  leafh u(.s(x), .y(y));
  initial begin #2 a = 2'd0; #1 $finish; end
  initial #100 $finish;
endmodule
module leafh(input logic [1:0] s, output logic [1:0] y);
  always_comb begin
    $display(\"L t=%0t s=%b\", $time, s);
    y = 0;
    unique case (s) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
endmodule
";
    let r = sim(src);
    assert_out(&r, "L t=0 s=01\nL t=2 s=00\n", 3);
    assert_eq!(r.diags, ["14:12 W4031 [in top.u] [at time 2]"]);
}

/// Two blocks the time-0 settle wakes, the producer written first: run in one batch the
/// consumer read `ao_w` / its port before the settle carried the producer's write.
#[test]
fn settle_woken_blocks_chained_through_an_assign_or_a_port_run_one_at_a_time() {
    // iv: t=1 y=1 (no WARNING);  vl: t=1 y=1 (no %Error);  PRE: W4031 [in top] [at time 0].
    let ca = "module top;
  wire [1:0] w;
  wire w2;
  logic [1:0] a_o;
  wire [1:0] ao_w;
  logic [1:0] y;
  assign w = 2'd1;
  assign w2 = 1'b1;
  assign ao_w = a_o;
  always_comb a_o = w;
  always_comb begin
    y = 2'd0;
    unique case (ao_w & {w2, w2})
      2'd1: y = 2'd1;
      2'd2: y = 2'd2;
    endcase
  end
  initial begin #1 $display(\"t=1 y=%0d\", y); #1 $finish; end
endmodule
";
    let r = sim(ca);
    assert_out(&r, "t=1 y=1\n", 2);
    assert!(r.diags.is_empty(), "{r:?}");
    // Parent block → port → child block, both settle-woken (the child by its constant
    // `k` tie). iv: t=1 y=1;  vl: t=1 y=1;  PRE: W4031 [in top.u] [at time 0].
    let port = "module top;
  wire [1:0] w;
  logic [1:0] a_o;
  assign w = 2'd1;
  always_comb a_o = w;
  child u(.s(a_o), .k(1'b1));
  initial begin #1 $display(\"t=1 y=%0d\", u.y); #1 $finish; end
endmodule
module child(input logic [1:0] s, input logic k);
  logic [1:0] y;
  always_comb begin
    y = 2'd0;
    unique case (s & {k, k})
      2'd1: y = 2'd1;
      2'd2: y = 2'd2;
    endcase
  end
endmodule
";
    let r = sim(port);
    assert_out(&r, "t=1 y=1\n", 2);
    assert!(r.diags.is_empty(), "{r:?}");
}

#[test]
fn with_no_first_batch_the_settle_woken_blocks_still_run_one_at_a_time() {
    // No `initial` at all: the settle's wakes ARE the first batch, and the two blocks it
    // wakes (`a_o = w`, then the consumer of `ao_w = a_o`) run one per batch with a settle
    // between. iv: F y=1 (no WARNING);  vl: F y=1 (no %Error);  PRE: W4031 [at time 0].
    let src = "module top;
  wire [1:0] w;
  wire w2;
  logic [1:0] a_o;
  wire [1:0] ao_w;
  logic [1:0] y;
  assign w = 2'd1;
  assign w2 = 1'b1;
  assign ao_w = a_o;
  always_comb a_o = w;
  always_comb begin
    y = 2'd0;
    unique case (ao_w & {w2, w2})
      2'd1: y = 2'd1;
      2'd2: y = 2'd2;
    endcase
  end
  final $display(\"F y=%0d\", y);
endmodule
";
    let r = sim(src);
    assert_eq!(
        r.out, "F y=1\nsimulation ended (Quiescent) at time 0\n",
        "{r:?}"
    );
    assert!(r.diags.is_empty(), "{r:?}");
    // …and a level process the settle woke beside a block still waits for it.
    // iv: r t=0 w=01 v=xx y=xx (the reader first);  vl: r t=0 w=01 v=01 y=01;  PRE: as vl.
    // Kept at verilator's (PRE's) side of that split, in both text orders.
    for (a, b) in [
        (
            "always_comb y = w;",
            "always @(w) $display(\"r t=%0t w=%b v=%b y=%b\", $time, w, v, y);",
        ),
        (
            "always @(w) $display(\"r t=%0t w=%b v=%b y=%b\", $time, w, v, y);",
            "always_comb y = w;",
        ),
    ] {
        let src = format!(
            "module top;
  wire [1:0] w;
  logic [1:0] y;
  wire [1:0] v;
  assign w = 2'd1;
  assign v = y;
  {a}
  {b}
  final $display(\"F v=%b y=%b\", v, y);
endmodule
"
        );
        let r = sim(&src);
        assert_eq!(
            r.out, "r t=0 w=01 v=01 y=01\nF v=01 y=01\nsimulation ended (Quiescent) at time 0\n",
            "{src}"
        );
    }
}

// ── implementation pins (no second oracle: verilator does not converge on `n++`) ─────

#[test]
fn a_block_armed_at_seeding_runs_once_per_later_change() {
    // Its implicit pass consumes the seeding arm and its return re-arms it: one waiter,
    // not two (with a second waiter interp/vm printed `C t=5` twice and `n=3`). iv: C t=0 a=0 / C t=5 a=1 /
    // n=2;  vl: DIDNOTCONVERGE (`n` is read and written by the block).
    let src = "module top;
  logic a = 1'b0;
  logic y;
  int n;
  always_comb begin y = a; n++; $display(\"C t=%0t a=%b\", $time, a); end
  initial begin #5 a = 1; #1 $display(\"n=%0d\", n); $finish; end
endmodule
";
    let r = sim(src);
    assert_out(&r, "C t=0 a=0\nC t=5 a=1\nn=2\n", 6);
    // A `#0` landing and the implicit pass share the promotion. The landing wakes the
    // block first (its waiter is consumed: woken, not yet run), so the implicit pass runs
    // in place of that pending wake: one pass, with the landed value. A count
    // split, iverilog-only (verilator is no count oracle, and it aborts here):
    // iv: C t=0 w=0 / C t=0 w=0 / C t=5 w=1 / n=3;  vl: C t=0 w=0 twice, then
    // DIDNOTCONVERGE;  PRE: C t=0 w=x / C t=0 w=0 / C t=5 w=1 / n=3 (a value neither
    // prints);  r1-A (the pass delivered regardless): iverilog's text.
    let landing = "module top;
  logic s;
  wire w;
  logic y;
  int n;
  assign #0 w = s;
  always_comb begin y = w; n++; $display(\"C t=%0t w=%b\", $time, w); end
  initial begin s = 1'b0; #5 s = 1'b1; #1 $display(\"n=%0d\", n); $finish; end
endmodule
";
    let r = sim(landing);
    assert_out(&r, "C t=0 w=0\nC t=5 w=1\nn=2\n", 6);
}
