//! §4.5.584, the other half of `always_comb_t0_after_settle.rs` (whose module doc states
//! the rule and the oracles: Icarus Verilog 13.0 `-g2012`, Verilator 5.052 `--binary
//! --timing --assert`; verilator is no oracle for an `x` nor for a run count).
//!
//! Here: the SPLITS — the oracles disagree; each pin is vita's value with both oracles'
//! raw lines quoted above it — the RESIDUE — both oracles silent at time 0 and vita still
//! reporting, every case a consumer block written before the block feeding it with no
//! process write at time 0, which needs an ORDER among the time-0 passes (iverilog's
//! reverse source order or verilator's dependency order; ordering by either oracle is
//! the never-chased split) — and the processes the rule must not move.
#[path = "always_comb_t0_util/mod.rs"]
mod util;
use util::*;
// ── splits: pinned at vita's value, both oracles quoted ────────────────────────────

#[test]
fn split_a_constant_block_read_in_the_first_slice_is_unrun() {
    // Forced by the rule (the first slice runs before any time-0 pass). iv: i0 y=xx /
    // i1 y=xx / i2 y=11 / i3 y=11;  vl: i0..i3 y=11;  PRE: i0..i3 y=11 (= vl). vita now
    // matches neither on i1 (iverilog runs the pass after the first `#0` batch).
    let src = "module top;
  logic [1:0] y;
  always_comb y = 2'd3;
  initial begin
    $display(\"i0 y=%b\", y);
    #0 $display(\"i1 y=%b\", y);
    #0 $display(\"i2 y=%b\", y);
    #0 $display(\"i3 y=%b\", y);
  end
  initial #1 $finish;
endmodule
";
    let r = sim(src);
    assert_out(&r, "i0 y=xx\ni1 y=11\ni2 y=11\ni3 y=11\n", 1);
    // The same with an `always_latch` (iverilog rejects a constant latch: `always_latch
    // process has no event control`) and with a second block reading a declaration
    // initializer. s583_v03 vl: init t=0 y=10 (×3);  iv: y=xx / y=xx / y=10.
    // s583_v01 vl: y=11 z=1 (×3);  iv: y=xx z=x / y=xx z=x / y=11 z=1.  PRE = vl on both.
    let latch = "module top;
  logic [1:0] y;
  logic en = 1;
  always_latch if (en) y = 2'd2;
  initial begin
    $display(\"init t=%0t y=%b\", $time, y);
    #0 $display(\"init#0 t=%0t y=%b\", $time, y);
    #0 $display(\"init#0#0 t=%0t y=%b\", $time, y);
  end
  initial #1 $finish;
endmodule
";
    let r = sim(latch);
    assert_out(&r, "init t=0 y=xx\ninit#0 t=0 y=10\ninit#0#0 t=0 y=10\n", 1);
    let two = "module top;
  logic [1:0] y;
  logic a = 1;
  logic z;
  always_comb y = 2'd3;
  always_comb z = a;
  initial begin
    $display(\"init t=%0t y=%b z=%b\", $time, y, z);
    #0 $display(\"init#0 t=%0t y=%b z=%b\", $time, y, z);
    #0 $display(\"init#0#0 t=%0t y=%b z=%b\", $time, y, z);
  end
  initial #1 $finish;
endmodule
";
    let r = sim(two);
    assert_out(
        &r,
        "init t=0 y=xx z=x\ninit#0 t=0 y=11 z=1\ninit#0#0 t=0 y=11 z=1\n",
        1,
    );
}

#[test]
fn split_a_settle_woken_block_runs_twice_at_time_zero() {
    // Woken by the settle, then its implicit pass. A constant net: iv: C t=0 w=11 /
    // WARNING k1_ca_settled_miss.sv:7 … Time: 0 / C t=0 w=11 / WARNING … Time: 0 /
    // e t=1 y=0;  vl: C t=0 w=11 / [0] %Error: …:7: Assertion failed in top: unique case,
    // but none matched for '2'h3' / e t=1 y=0;  PRE: one C line, one W4031 (= vl).
    let k1 = "module top;
  wire [1:0] w = 2'd3;
  logic [1:0] y;
  always_comb begin
    $display(\"C t=%0t w=%b\", $time, w);
    y = 0;
    unique case (w) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
  initial #1 begin $display(\"e t=%0t y=%0d\", $time, y); $finish; end
endmodule
";
    let r = sim(k1);
    assert_out(&r, "C t=0 w=11\nC t=0 w=11\ne t=1 y=0\n", 1);
    assert_eq!(
        r.diags,
        [
            "7:12 W4031 [in top] [at time 0]",
            "7:12 W4031 [in top] [at time 0]"
        ]
    );
    // `~a` of a declaration-initialised variable: iverilog runs it ONCE here (iv: C t=0
    // w=1 / C t=5 w=0 / n=2; vl: DIDNOTCONVERGE after C t=0 w=1; PRE: as iv) — vita runs
    // the settle-woken pass and the implicit pass: twice, matching neither.
    let d1b = "module top;
  logic a = 1'b0;
  wire w;
  logic y;
  int n;
  assign w = ~a;
  always_comb begin y = w; n++; $display(\"C t=%0t w=%b\", $time, w); end
  initial begin #5 a = 1; #1 $display(\"n=%0d\", n); $finish; end
endmodule
";
    let r = sim(d1b);
    assert_out(&r, "C t=0 w=1\nC t=0 w=1\nC t=5 w=0\nn=3\n", 6);
}

#[test]
fn split_a_finish_in_the_first_batch_still_lets_the_block_run_twice() {
    // iv: $finish called at 0 / C t=0 s=1 / F t=0 y=1;  vl: C t=0 s=1 / F t=0 y=1;
    // PRE: C t=0 s=x / C t=0 s=1 / F t=0 y=1.  The `$finish` ends the run at the step's
    // stable point, after the woken pass and the implicit pass (both oracles run it once).
    let src = "module top;
  logic s, y;
  always_comb begin y = s; $display(\"C t=%0t s=%b\", $time, s); end
  initial begin s = 1'b1; $finish; end
  final $display(\"F t=%0t y=%b\", $time, y);
endmodule
";
    let r = sim(src);
    assert_eq!(
        r.out, "C t=0 s=1\nC t=0 s=1\nF t=0 y=1\nsimulation ended (Finish) at time 0\n",
        "{r:?}"
    );
    // `$stop` and `$fatal` end the run at the statement, before any pass. `$stop`: iv:
    // $stop called at 0 / F t=0 y=x (= vita);  vl: C t=0 s=1 / F t=0 y=1.  `$fatal`: iv:
    // FATAL … / C t=0 s=1 / F t=0 y=1, rc=1;  vl: %Fatal, no C, no F, rc=1.
    let r = sim(&src.replace("$finish;", "$stop;"));
    assert_eq!(
        r.out, "F t=0 y=x\nsimulation ended (Stop) at time 0\n",
        "{r:?}"
    );
    let r = sim(&src.replace("$finish;", "$fatal(1, \"boom\");"));
    assert_eq!(
        r.out, "F t=0 y=x\nsimulation ended (Error) at time 0\n",
        "{r:?}"
    );
    assert_eq!(r.diags, ["4:27 F4004 [in top] [at time 0]"]);
    assert_eq!(r.code, 1);
}

#[test]
fn split_order_of_the_implicit_pass_against_hash0_and_hierarchical_reads() {
    // The implicit pass leads the first promotion, so it runs BEFORE the `#0`
    // continuation. iv: I0 … / I1 t=0 s=01 w=01 / C t=0 w=01 / I2 t=0 s=01 w=01 /
    // C t=0 w=01 / t=1 y=1;  vl: … C t=0 w=01 / I2 … (once);  PRE: C t=0 w=xx + W4031.
    let src = "module top;
  logic [1:0] s, y;
  wire [1:0] w;
  assign w = s;
  initial begin
    $display(\"I0 t=%0t s=%b w=%b\", $time, s, w);
    s = 2'd1;
    $display(\"I1 t=%0t s=%b w=%b\", $time, s, w);
    #0 $display(\"I2 t=%0t s=%b w=%b\", $time, s, w);
    #1 $display(\"t=%0t y=%0d\", $time, y);
    #1 $finish;
  end
  always_comb begin
    $display(\"C t=%0t w=%b\", $time, w);
    y = 0;
    unique case (w) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
  initial #100 $finish;
endmodule
";
    let r = sim(src);
    assert_out(
        &r,
        "I0 t=0 s=xx w=xx\nI1 t=0 s=01 w=01\nC t=0 w=01\nC t=0 w=01\nI2 t=0 s=01 w=01\nt=1 y=1\n",
        2,
    );
    assert!(r.diags.is_empty(), "{r:?}");
    // A parent reading a child's constant block. Hierarchically (row7_comb): iv: r=xx /
    // r#0=xx;  vl: r=ee / r#0=ee;  PRE: r=xx / r#0=ee.  Through an output port: iv: r=xx /
    // r#0=xx / r#00=ee;  vl: ee ×3;  PRE: xx / ee / ee.  Both = PRE.
    let hier = "module top;
  logic [7:0] r;
  child u1();
  initial begin r = u1.s; $display(\"r=%h\", r); #0 $display(\"r#0=%h\", u1.s); end
  initial #10 $finish;
endmodule
module child;
  logic [7:0] s;
  always_comb s = 8'hEE;
endmodule
";
    assert_out(&sim(hier), "r=xx\nr#0=ee\n", 10);
    let port = "module top;
  wire [7:0] w;
  child u(.s(w));
  initial begin
    $display(\"r t=%0t w=%h\", $time, w);
    #0 $display(\"r#0 t=%0t w=%h\", $time, w);
    #0 $display(\"r#00 t=%0t w=%h\", $time, w);
    #1 $display(\"e t=%0t w=%h\", $time, w);
    $finish;
  end
endmodule
module child(output logic [7:0] s);
  always_comb s = 8'hEE;
endmodule
";
    assert_out(
        &sim(port),
        "r t=0 w=xx\nr#0 t=0 w=ee\nr#00 t=0 w=ee\ne t=1 w=ee\n",
        1,
    );
}

#[test]
fn split_a_batch_two_reader_of_a_declaration_initialised_block() {
    // The block has no settle wake; its only time-0 pass is the implicit one. iv: b2 t=0
    // y=x;  vl: b2 t=0 y=1;  PRE: y=1 (= vl) — now iverilog's.
    let src = "module top;
  logic a = 1'b0;
  logic y;
  logic t;
  always_comb y = ~a;
  initial t = 1'b1;
  always @(t) $display(\"b2 t=%0t y=%b\", $time, y);
  initial #1 begin $display(\"e t=%0t y=%b\", $time, y); $finish; end
endmodule
";
    assert_out(&sim(src), "b2 t=0 y=x\ne t=1 y=1\n", 1);
    // Armed at seeding: the first batch's `s = 1` wakes the block ahead of the reader it
    // also wakes. iv: b2 t=0 s=1 y=x (the reader first);  vl: b2 t=0 s=1 y=1;  PRE: = vl.
    let armed = "module top;
  logic s, y;
  always_comb y = s;
  initial s = 1'b1;
  always @(s) $display(\"b2 t=%0t s=%b y=%b\", $time, s, y);
  initial #1 begin $display(\"e t=%0t y=%b\", $time, y); $finish; end
endmodule
";
    assert_out(&sim(armed), "b2 t=0 s=1 y=1\ne t=1 y=1\n", 1);
}

#[test]
fn split_writers_the_rule_leaves_as_they_were() {
    // `initial #0 s = 1;`: iv: C t=0 s=01 (silent);  vl: C t=0 s=00 / [0] %Error (2-state
    // miss);  PRE: C t=0 s=xx / W4031 at time 0 / C t=0 s=01 — unchanged (= vl's report).
    let hash0 = "module top;
  logic [1:0] s, y;
  always_comb begin
    $display(\"C t=%0t s=%b\", $time, s);
    unique case (s) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
  initial #0 s = 2'd1;
  initial begin #2 s = 2'd0; #1 $finish; end
  initial #100 $finish;
endmodule
";
    let r = sim(hash0);
    assert_out(&r, "C t=0 s=xx\nC t=0 s=01\nC t=2 s=00\n", 3);
    assert_eq!(
        r.diags,
        [
            "5:12 W4031 [in top] [at time 0]",
            "5:12 W4031 [in top] [at time 2]"
        ]
    );
}

/// A forward chain (producer written first) from a source no time-0 event reaches — a
/// declaration initializer or a constant block. The producer's pass wakes the middle
/// block, whose own pass then runs in that wake's place (its trigger's slot), so the
/// consumer two hops down reads the settled middle value. With the middle's pass dropped
/// and its woken run queued behind every later pass, the consumer read `xx`: an E4003 at
/// time 0 and exit 1 where both oracles exit 0 (review r2 R2-1..R2-3). The report itself
/// splits: iverilog runs the passes in reverse order and reports; verilator is silent.
#[test]
fn split_a_forward_chain_from_a_quiet_source_reports_nothing_at_time_zero() {
    // iv: ERROR: tg10_assert_chain3_fwd.sv:8: A t=0 m=xx / Time: 0  Scope: top /
    //     t=1 y=1, run rc=0;  vl: t=1 y=1, run rc=0;  PRE: t=1 y=1, rc 0 (= vl).
    let decl = "module top;
  logic [1:0] src = 2'd1;
  logic [1:0] p, m, y;
  always_comb p = src;
  always_comb m = p;
  always_comb begin
    y = m;
    assert (m == 2'd1) else $error(\"A t=%0t m=%b\", $time, m);
  end
  initial #1 $display(\"t=%0t y=%0d\", $time, y);
  initial #5 $finish;
endmodule
";
    let r = sim(decl);
    assert_out(&r, "t=1 y=1\n", 5);
    assert!(r.diags.is_empty(), "{r:?}");
    assert_eq!(r.code, 0);
    // A constant block as the source (`always_comb p = CFG;`, no read set).
    // iv: ERROR: tg13_constcomb_chain3_assert.sv:8: A t=0 m=xx / Time: 0, run rc=0;
    // vl: t=1 y=1, run rc=0;  PRE: t=1 y=1, rc 0.
    let konst = decl
        .replace(
            "  logic [1:0] src = 2'd1;\n",
            "  localparam logic [1:0] CFG = 2'd1;\n",
        )
        .replace("always_comb p = src;", "always_comb p = CFG;")
        .replace("assert (m == 2'd1)", "assert (m == CFG)");
    let r = sim(&konst);
    assert_out(&r, "t=1 y=1\n", 5);
    assert!(r.diags.is_empty(), "{r:?}");
    assert_eq!(r.code, 0);
    // Four blocks, a `unique case` at the end. iv: WARNING: tg6_chain4_fwd.sv:8: value
    // is unhandled … / Time: 0  Scope: top / t=1 y=1;  vl: t=1 y=1;  PRE: t=1 y=1.
    let four = "module top;
  logic [1:0] src = 2'd1;
  logic [1:0] p, m1, m2, y;
  always_comb p = src;
  always_comb m1 = p;
  always_comb m2 = m1;
  always_comb begin
    unique case (m2)
      2'd1: y = 2'd1;
      2'd2: y = 2'd2;
    endcase
  end
  initial #1 $display(\"t=%0t y=%0d\", $time, y);
  initial #5 $finish;
endmodule
";
    let r = sim(four);
    assert_out(&r, "t=1 y=1\n", 5);
    assert!(r.diags.is_empty(), "{r:?}");
    // The woken block runs once, in its trigger's slot (a count split): iv: Y t=0 x=xx /
    // Y t=0 x=01;  vl: Y t=0 x=01;  PRE: Y t=0 x=01.
    let once = "module top;
  logic [1:0] s = 2'd1;
  logic [1:0] x, y;
  always_comb x = s;
  always_comb begin y = x; $display(\"Y t=%0t x=%b\", $time, x); end
  initial #1 $finish;
endmodule
";
    assert_out(&sim(once), "Y t=0 x=01\n", 1);
}

// ── the residue: both oracles silent at time 0, vita reports ───────────────────────

#[test]
fn residue_a_consumer_block_before_its_producer_block_still_reports_at_time_zero() {
    // `a = 0` by declaration, A (`unique case ({a, b})`) written before B (`b = a`). iv:
    // B t=0 / A t=0 ab=00 / A t=5 ab=10 / WARNING q_chaind_A_B.sv:6 … Time: 5 / B t=5 /
    // A t=5 ab=11 / t=6 y=3 (iverilog reports the consumer-first read itself at t=5; its
    // time-0 silence is its reverse order of the time-0 passes);  vl: B t=0 / A t=0 ab=00 /
    // A t=5 ab=11 … (no report);  PRE: the same text as below.
    let src = "module top;
  logic a = 1'b0, b;
  logic [1:0] y;
  always_comb begin
    $display(\"A t=%0t ab=%b%b\", $time, a, b);
    unique case ({a, b}) 2'b00: y = 0; 2'b11: y = 3; endcase
  end
  always_comb begin $display(\"B t=%0t\", $time); b = a; end
  initial begin
    #5 a = 1'b1;
    #1 $display(\"t=%0t y=%0d\", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
";
    let r = sim(src);
    assert_out(
        &r,
        "A t=0 ab=0x\nB t=0\nA t=0 ab=00\nA t=5 ab=10\nB t=5\nA t=5 ab=11\nt=6 y=3\n",
        7,
    );
    assert_eq!(
        r.diags,
        [
            "6:12 W4031 [in top] [at time 0]",
            "6:12 W4031 [in top] [at time 5]"
        ]
    );
    // Through a middle block (A, Bc `c = b`, Bb `b = a`). iv: Bb t=0 / Bc t=0 / A t=0
    // ac=00 … WARNING q_chain3_A_Bc_Bb.sv:6 … Time: 5;  vl: Bc t=0 / Bb t=0 / A t=0 ac=00
    // (no report);  PRE: the same text as below.
    let chain3 = "module top;
  logic a = 1'b0, b, c;
  logic [1:0] y;
  always_comb begin
    $display(\"A t=%0t ac=%b%b\", $time, a, c);
    unique case ({a, c}) 2'b00: y = 0; 2'b11: y = 3; endcase
  end
  always_comb begin $display(\"Bc t=%0t\", $time); c = b; end
  always_comb begin $display(\"Bb t=%0t\", $time); b = a; end
  initial begin
    #5 a = 1'b1;
    #1 $display(\"t=%0t y=%0d\", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
";
    let r = sim(chain3);
    assert_out(
        &r,
        "A t=0 ac=0x\nBc t=0\nBb t=0\nBc t=0\nA t=0 ac=00\nA t=5 ac=10\nBb t=5\nBc t=5\nA t=5 ac=11\nt=6 y=3\n",
        7,
    );
    assert_eq!(r.diags[0], "6:12 W4031 [in top] [at time 0]");
    // Two settle-woken blocks, the consumer (through `ao_w = a_o`) written first.
    // iv: t=1 y=1 (no WARNING);  vl: t=1 y=1;  PRE: W4031 at time 0 — unchanged.
    let settle = "module top;
  wire [1:0] w;
  wire w2;
  logic [1:0] a_o;
  wire [1:0] ao_w;
  logic [1:0] y;
  assign w = 2'd1;
  assign w2 = 1'b1;
  assign ao_w = a_o;
  always_comb begin
    y = 2'd0;
    unique case (ao_w & {w2, w2})
      2'd1: y = 2'd1;
      2'd2: y = 2'd2;
    endcase
  end
  always_comb a_o = w;
  initial begin #1 $display(\"t=1 y=%0d\", y); #1 $finish; end
endmodule
";
    let r = sim(settle);
    assert_out(&r, "t=1 y=1\n", 2);
    assert_eq!(r.diags, ["12:12 W4031 [in top] [at time 0]"]);
}

// ── must not move ──────────────────────────────────────────────────────────────

#[test]
fn processes_the_rule_must_not_move() {
    // A self-timed clock (now `Initial`, was `Comb`): iv and vl: t=22 n=2 clk=0.
    let clk = "module top;
  logic clk; int n = 0;
  always #5 clk = ~clk;
  always @(posedge clk) n++;
  initial clk = 0;
  initial begin #22 $display(\"t=%0t n=%0d clk=%b\", $time, n, clk); $finish; end
  initial #100 $finish;
endmodule
";
    assert_out(&sim(clk), "t=22 n=2 clk=0\n", 22);
    // A clock generator read by a batch-2 and a `#0` reader, before and after them.
    // iv and vl: b2 t=0 clk=0 / z t=0 clk=0 / e t=12 clk=0.
    let gen = "module top;
  logic clk;
  logic t;
  always begin clk = 1'b0; #5 clk = 1'b1; #5; end
  initial t = 1'b1;
  always @(t) $display(\"b2 t=%0t clk=%b\", $time, clk);
  initial #0 $display(\"z t=%0t clk=%b\", $time, clk);
  initial #12 begin $display(\"e t=%0t clk=%b\", $time, clk); $finish; end
endmodule
";
    assert_out(&sim(gen), "b2 t=0 clk=0\nz t=0 clk=0\ne t=12 clk=0\n", 12);
    // `initial s <= 1;`: every tool reports at time 0. iv: C t=0 s=xx / WARNING … Time: 0 /
    // C t=0 s=01 / …;  vl: C t=0 s=00 / [0] %Error … / C t=0 s=01 / ….
    let nba = "module top;
  logic [1:0] s, y;
  always_comb begin
    $display(\"C t=%0t s=%b\", $time, s);
    unique case (s) 2'd1: y = 1; 2'd2: y = 2; endcase
  end
  initial s <= 2'd1;
  initial begin #2 s = 2'd0; #1 $finish; end
  initial #100 $finish;
endmodule
";
    let r = sim(nba);
    assert_out(&r, "C t=0 s=xx\nC t=0 s=01\nC t=2 s=00\n", 3);
    assert_eq!(
        r.diags,
        [
            "5:12 W4031 [in top] [at time 0]",
            "5:12 W4031 [in top] [at time 2]"
        ]
    );
    // A real miss on a constant: iv: WARNING q_constuc_comb.sv:3 … Time: 0;  vl: [0]
    // %Error: …:3: Assertion failed in top: unique case, but none matched for '2'h3'.
    let constuc = "module top;
  logic [1:0] y;
  always_comb begin y = 0; unique case (2'd3) 2'd1: y = 1; endcase end
  initial #1 $finish;
endmodule
";
    let r = sim(constuc);
    assert_out(&r, "", 1);
    assert_eq!(r.diags, ["3:35 W4031 [in top] [at time 0]"]);
}
