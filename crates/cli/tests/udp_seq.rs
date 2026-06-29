//! YELLOW #9 (SEQUENTIAL User-Defined Primitive, IEEE 1364 §29).
//!
//! `primitive … output reg q; … table … endtable endprimitive` with edge columns
//! (`(vw)` pairs and `r f p n *` shorthands), an optional current-state column
//! (`inputs : state : next`), `-` (no-change) next-states, and an `initial q = …;`
//! power-on value. DESUGARED in the parser (extending the combinational path) into a
//! synthetic ordinary module: an `always @(i0 or i1 or …)` (all inputs as plain
//! NoEdge terms ⇒ finest AnyEdge wake) whose body is a LITERAL §29 state-table
//! evaluator — LEVEL rows first (order-independent), THEN edge rows, no-match ⇒ x
//! (never hold), `-` ⇒ an empty then-branch that OWNS its else-slot. Edge detection
//! uses one shadow reg per input (`reg __p_udp_<out>_<k>;`) and a FOLDED (z→x)
//! change guard, so z↔x swaps are not edges and do not re-evaluate the table. Pure
//! parser desugar (IR-0): no new AST node, no `.vu` schema-hash flip, no
//! `format_version` bump.
//!
//! Every expected output below is pinned to live iverilog 13.0 (UDPs are fully
//! supported). Honest-loud rejects: >1 edge column per row, vector output, edge
//! endpoint `b`, malformed edge paren, `initial` to a non-0/1/x value, three colons,
//! and an inconsistent `reg`-marked but purely combinational table.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_udpseq_{}_{n}", std::process::id()));
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

/// All `$display` lines (in order) must appear in stdout.
fn expect_lines(out: &str, lines: &[&str]) {
    for l in lines {
        assert!(out.contains(l), "missing {l:?} in:\n{out}");
    }
}

#[test]
fn udp_seq_canonical_dff() {
    // Positive-edge D flip-flop — the canonical sequential UDP. Full waveform
    // pinned to iverilog. (A lone DFF passes even with the level-precedence /
    // no-match bugs, so it is necessary but NOT sufficient — see the precedence and
    // no-match tests below.)
    let (out, err, code) = run("primitive dff(q, clk, d);\n\
           output reg q; input clk, d;\n\
           initial q = 1'b0;\n\
           table\n\
             (01) 0 : ? : 0 ;\n\
             (01) 1 : ? : 1 ;\n\
             (0?) 1 : 1 : 1 ;\n\
             (0?) 0 : 0 : 0 ;\n\
             (?0) ? : ? : - ;\n\
             ?  (??) : ? : - ;\n\
           endtable\n\
         endprimitive\n\
         module top; reg clk, d; wire q; dff u(q, clk, d);\n\
         initial begin\n\
           clk=0; d=0; #1 $display(\"t1 q=%b\", q);\n\
           d=1;     #1 $display(\"t2 q=%b\", q);\n\
           clk=1;   #1 $display(\"t3 q=%b\", q);\n\
           d=0;     #1 $display(\"t4 q=%b\", q);\n\
           clk=0;   #1 $display(\"t5 q=%b\", q);\n\
           clk=1;   #1 $display(\"t6 q=%b\", q);\n\
           d=1; clk=0; #1 $display(\"t7 q=%b\", q);\n\
           clk=1;   #1 $display(\"t8 q=%b\", q);\n\
         end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    expect_lines(
        &out,
        &[
            "t1 q=0", "t2 q=0", "t3 q=1", "t4 q=1", "t5 q=1", "t6 q=0", "t7 q=0", "t8 q=1",
        ],
    );
}

#[test]
fn udp_seq_level_over_edge_order_independent() {
    // KILLER: a level row and an edge row both match the post-edge vector; the LEVEL
    // row wins regardless of textual order. Edge says capture d=1 ⇒ 1; level (clk=1,
    // d=1) says 0. q must be 0 in BOTH textual orders (a flop-inference lowering or
    // edge-first cascade would give 1). q0=x because the FIRST settling transition
    // (clk:x→0, d:x→1) matches no row and clobbers the initial 0 to x.
    let body = |rows: &str| {
        format!(
            "primitive p(q, clk, d);\n\
               output reg q; input clk, d;\n\
               initial q = 1'b0;\n\
               table {rows} endtable\n\
             endprimitive\n\
             module top; reg clk,d; wire q; p u(q,clk,d);\n\
             initial begin clk=0; d=1; #1 $display(\"v0 q=%b\",q); clk=1; #1 $display(\"v1 q=%b\",q); end endmodule\n"
        )
    };
    let (out_a, _, ca) = run(&body("(01) 1 : ? : 1 ;  1 1 : ? : 0 ;"));
    let (out_b, _, cb) = run(&body("1 1 : ? : 0 ;  (01) 1 : ? : 1 ;"));
    assert_eq!(ca, Some(0));
    assert_eq!(cb, Some(0));
    expect_lines(&out_a, &["v0 q=x", "v1 q=0"]);
    expect_lines(&out_b, &["v0 q=x", "v1 q=0"]);
}

#[test]
fn udp_seq_nomatch_on_fully_defined_edge_is_x() {
    // No-match ⇒ x, even on a FULLY-DEFINED falling edge (not just x-involving
    // transitions). Only `(01) 1` is covered; a `clk:1→0` fall matches nothing ⇒ x.
    let (out, err, code) = run(
        "primitive p(q, clk, d);\n\
           output reg q; input clk, d;\n\
           initial q = 1'b1;\n\
           table (01) 1 : ? : 1 ; endtable\n\
         endprimitive\n\
         module top; reg clk,d; wire q; p u(q,clk,d);\n\
         initial begin clk=1; d=1; #1 $display(\"n0 q=%b\",q); clk=0; #1 $display(\"n1 q=%b\",q); end endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    expect_lines(&out, &["n0 q=x", "n1 q=x"]);
}

#[test]
fn udp_seq_d_latch_transparency_and_hold() {
    // Level-sensitive D-latch: transparent while ena=1, holds while ena=0.
    let (out, err, code) = run("primitive dlatch(q, ena, d);\n\
           output reg q; input ena, d;\n\
           table\n\
             1 0 : ? : 0 ;\n\
             1 1 : ? : 1 ;\n\
             0 ? : ? : - ;\n\
             1 x : ? : - ;\n\
           endtable\n\
         endprimitive\n\
         module top; reg ena,d; wire q; dlatch u(q,ena,d);\n\
         initial begin\n\
           ena=1; d=0; #1 $display(\"L0 q=%b\",q);\n\
           d=1;        #1 $display(\"L1 q=%b\",q);\n\
           ena=0;      #1 $display(\"L2 q=%b\",q);\n\
           d=0;        #1 $display(\"L3 hold q=%b\",q);\n\
           ena=1;      #1 $display(\"L4 q=%b\",q);\n\
         end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    expect_lines(
        &out,
        &["L0 q=0", "L1 q=1", "L2 q=1", "L3 hold q=1", "L4 q=0"],
    );
}

#[test]
fn udp_seq_shorthand_equals_explicit() {
    // `r f p n *` are byte-identical to the explicit `(01) (10) … (??)` forms.
    let body = |c0: &str, c1: &str, c2: &str| {
        format!(
            "primitive p(q, clk, d);\n\
               output reg q; input clk, d;\n\
               initial q = 1'b0;\n\
               table\n\
                 {c0} 0 : ? : 0 ;\n\
                 {c0} 1 : ? : 1 ;\n\
                 {c1} ? : ? : - ;\n\
                 ? {c2} : ? : - ;\n\
               endtable\n\
             endprimitive\n\
             module top; reg clk,d; wire q; p u(q,clk,d);\n\
             initial begin clk=0;d=1;#1 clk=1;#1 $display(\"S q=%b\",q); d=0;#1 $display(\"S2 q=%b\",q); clk=0;#1 clk=1;#1 $display(\"S3 q=%b\",q); end endmodule\n"
        )
    };
    let (out_s, _, cs) = run(&body("r", "f", "*"));
    let (out_e, _, ce) = run(&body("(01)", "(10)", "(??)"));
    assert_eq!(cs, Some(0));
    assert_eq!(ce, Some(0));
    expect_lines(&out_s, &["S q=1", "S2 q=1", "S3 q=0"]);
    // explicit must equal shorthand
    assert_eq!(
        out_s
            .lines()
            .filter(|l| l.starts_with("S"))
            .collect::<Vec<_>>(),
        out_e
            .lines()
            .filter(|l| l.starts_with("S"))
            .collect::<Vec<_>>()
    );
}

#[test]
fn udp_seq_dash_retains_exactly() {
    // `-` (no-change) retains 0, 1, AND x exactly across non-matching transitions.
    let (out, err, code) = run("primitive p(q, clk, d);\n\
           output reg q; input clk, d;\n\
           table\n\
             (01) 0 : ? : 0 ;\n\
             (01) 1 : ? : 1 ;\n\
             (?0) ? : ? : - ;\n\
             ?  (??) : ? : - ;\n\
           endtable\n\
         endprimitive\n\
         module top; reg clk,d; wire q; p u(q,clk,d);\n\
         initial begin\n\
           clk=0; d=1; #1 $display(\"D0 q=%b\",q);\n\
           clk=1; #1 $display(\"D1 q=%b\",q);\n\
           clk=0; #1 $display(\"D2 q=%b\",q);\n\
           d=0;   #1 $display(\"D3 q=%b\",q);\n\
           clk=1; #1 $display(\"D4 q=%b\",q);\n\
           clk=0; #1 $display(\"D5 q=%b\",q);\n\
         end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    // q starts x; first edge captures 1; holds 1 across data; captures 0; holds 0.
    expect_lines(
        &out,
        &["D0 q=x", "D1 q=1", "D2 q=1", "D3 q=1", "D4 q=0", "D5 q=0"],
    );
}

#[test]
fn udp_seq_state_column_t_flip_flop() {
    // State-column wildcard vs literal: a T flip-flop toggles on the STORED state.
    let (out, err, code) = run("primitive tff(q, clk, t);\n\
           output reg q; input clk, t;\n\
           initial q = 1'b0;\n\
           table\n\
             (01) 1 : 0 : 1 ;\n\
             (01) 1 : 1 : 0 ;\n\
             (01) 0 : ? : - ;\n\
             (?0) ? : ? : - ;\n\
             ?  (??) : ? : - ;\n\
           endtable\n\
         endprimitive\n\
         module top; reg clk,t; wire q; tff u(q,clk,t);\n\
         initial begin\n\
           clk=0; t=1; #1;\n\
           repeat(6) begin clk=1; #1 $display(\"T q=%b\",q); clk=0; #1; end\n\
         end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    expect_lines(
        &out,
        &["T q=1", "T q=0", "T q=1", "T q=0", "T q=1", "T q=0"],
    );
}

#[test]
fn udp_seq_jk_flip_flop_multi_input_and_state() {
    // JK flip-flop: multiple inputs + the state column (classic sequential UDP).
    let (out, err, code) = run("primitive jk(q, clk, j, k);\n\
           output reg q; input clk, j, k;\n\
           initial q = 1'b0;\n\
           table\n\
             (01) 0 0 : ? : - ;\n\
             (01) 0 1 : ? : 0 ;\n\
             (01) 1 0 : ? : 1 ;\n\
             (01) 1 1 : 0 : 1 ;\n\
             (01) 1 1 : 1 : 0 ;\n\
             (?0) ? ? : ? : - ;\n\
             ?  (??) ? : ? : - ;\n\
             ?  ? (??) : ? : - ;\n\
           endtable\n\
         endprimitive\n\
         module top; reg clk,j,k; wire q; jk u(q,clk,j,k);\n\
         task tick; begin clk=1; #1; clk=0; #1; end endtask\n\
         initial begin\n\
           clk=0; j=0; k=0; #1;\n\
           j=1; k=0; tick; $display(\"J set  q=%b\",q);\n\
           j=0; k=0; tick; $display(\"J hold q=%b\",q);\n\
           j=0; k=1; tick; $display(\"J rst  q=%b\",q);\n\
           j=1; k=1; tick; $display(\"J tog1 q=%b\",q);\n\
           j=1; k=1; tick; $display(\"J tog2 q=%b\",q);\n\
         end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    expect_lines(
        &out,
        &[
            "J set  q=1",
            "J hold q=1",
            "J rst  q=0",
            "J tog1 q=1",
            "J tog2 q=0",
        ],
    );
}

#[test]
fn udp_seq_instance_shift_register_output_first() {
    // Two DFF instances chained into a shift register (output-first port order).
    let (out, err, code) = run("primitive dff(q, clk, d);\n\
           output reg q; input clk, d;\n\
           initial q = 1'b0;\n\
           table\n\
             (01) 0 : ? : 0 ;\n\
             (01) 1 : ? : 1 ;\n\
             (?0) ? : ? : - ;\n\
             ? (??) : ? : - ;\n\
           endtable\n\
         endprimitive\n\
         module top; reg clk,d; wire q1,q2;\n\
           dff a(q1, clk, d);\n\
           dff b(q2, clk, q1);\n\
         initial begin\n\
           clk=0; d=1; #1; clk=1; #1 $display(\"R1 q1=%b q2=%b\",q1,q2);\n\
           clk=0; #1; clk=1; #1 $display(\"R2 q1=%b q2=%b\",q1,q2);\n\
           d=0; clk=0; #1; clk=1; #1 $display(\"R3 q1=%b q2=%b\",q1,q2);\n\
         end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    expect_lines(&out, &["R1 q1=1 q2=0", "R2 q1=1 q2=1", "R3 q1=0 q2=1"]);
}

#[test]
fn udp_seq_initial_survives_across_holds() {
    // `initial q=1` survives the first x→known input settling iff hold rows cover
    // those transitions; otherwise it clobbers to x (t0-settling replay, no special
    // initial immunity). Here the `(x?)`/`? x` hold rows preserve the 1.
    let (out, err, code) = run("primitive p(q, clk, d);\n\
           output reg q; input clk, d;\n\
           initial q = 1'b1;\n\
           table\n\
             (01) 0 : ? : 0 ;\n\
             (01) 1 : ? : 1 ;\n\
             (0?) ? : ? : - ;\n\
             (x?) ? : ? : - ;\n\
             ?  x  : ? : - ;\n\
             (?0) ? : ? : - ;\n\
             ?  (??) : ? : - ;\n\
           endtable\n\
         endprimitive\n\
         module top; reg clk,d; wire q; p u(q,clk,d);\n\
         initial begin\n\
           #1 $display(\"I0 q=%b\",q);\n\
           clk=0; #1 $display(\"I1 q=%b\",q);\n\
           d=0;   #1 $display(\"I2 q=%b\",q);\n\
           clk=1; #1 $display(\"I3 q=%b\",q);\n\
         end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    expect_lines(&out, &["I0 q=1", "I1 q=1", "I2 q=1", "I3 q=0"]);
}

#[test]
fn udp_seq_z_input_folds_to_x_for_edges() {
    // z folds to x: clk:0→z is `(0x)` (covered by `p`), NOT `(01)`, so a strict-rising
    // `(01)` capture does not fire on 0→z (output holds via the `*` hold).
    let (out, err, code) = run("primitive p(q, clk, d);\n\
           output reg q; input clk, d;\n\
           initial q = 1'b0;\n\
           table\n\
             (01) 1 : ? : 1 ;\n\
             p    0 : ? : 0 ;\n\
             (?0) ? : ? : - ;\n\
             ? (??) : ? : - ;\n\
           endtable\n\
         endprimitive\n\
         module top; reg clk,d; wire q; p u(q,clk,d);\n\
         initial begin clk=0; d=1; #1; clk=1'bz; #1 $display(\"Z1 q=%b\",q); end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    expect_lines(&out, &["Z1 q=x"]);
}

#[test]
fn udp_seq_zx_swap_is_not_an_edge_and_holds() {
    // A z↔x swap on an input is NOT a folded change: it must neither fire an edge nor
    // re-evaluate the table. q holds across d:z→x (a naive `prev !== cur` guard plus
    // the no-match else would clobber it to x → silent-wrong).
    let (out, err, code) = run("primitive p(q, clk, d);\n\
           output reg q; input clk, d;\n\
           initial q = 1'b1;\n\
           table\n\
             (01) ? : ? : 0 ;\n\
             1 1 : ? : 1 ;\n\
           endtable\n\
         endprimitive\n\
         module top; reg clk, d; wire q; p u(q, clk, d);\n\
         initial begin\n\
           clk=0; #1; d=1'bz; #1 $display(\"P0 q=%b\",q);\n\
           clk=1; #1 $display(\"P1 q=%b\",q);\n\
           d=1'bx; #1 $display(\"P2 q=%b\",q);\n\
         end endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    // P0=x (clk:x→0 with no matching row), P1=0 (rising clk, d=z=? matches → 0),
    // P2=0 (d:z→x folds to no change ⇒ hold, NOT a no-match clobber to x).
    expect_lines(&out, &["P0 q=x", "P1 q=0", "P2 q=0"]);
}

#[test]
fn udp_seq_star_holds_across_data_toggles() {
    // `*`/`(??)` is an EDGE spec (any change), not a level wildcard. Here `? * : ? : -`
    // holds q across every data toggle once captured.
    let (out, err, code) = run(
        "primitive p(q, clk, d);\n\
           output reg q; input clk, d;\n\
           initial q = 1'b0;\n\
           table\n\
             (01) 0 : ? : 0 ;\n\
             (01) 1 : ? : 1 ;\n\
             (?0) ? : ? : - ;\n\
             ?  *  : ? : - ;\n\
           endtable\n\
         endprimitive\n\
         module top; reg clk,d; wire q; p u(q,clk,d);\n\
         initial begin\n\
           clk=0; d=1; #1; clk=1; #1 $display(\"X1 q=%b\",q);\n\
           d=0; #1 $display(\"X2 q=%b\",q); d=1; #1 $display(\"X3 q=%b\",q); d=1'bx; #1 $display(\"X4 q=%b\",q);\n\
         end endmodule\n",
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    expect_lines(&out, &["X1 q=1", "X2 q=1", "X3 q=1", "X4 q=1"]);
}

// ─────────────────────────── honest-loud rejects ───────────────────────────

fn rejects(src: &str, needle: &str) {
    let (_out, err, code) = run(src);
    assert_ne!(
        code,
        Some(0),
        "expected loud reject, succeeded. stderr:\n{err}"
    );
    assert!(err.contains(needle), "wrong reject reason; stderr:\n{err}");
}

#[test]
fn udp_seq_two_edge_columns_is_loud() {
    rejects(
        "primitive p(q,clk,d); output reg q; input clk,d;\n\
           table (01) (01) : ? : 1 ; endtable endprimitive\n\
         module top; reg clk,d; wire q; p u(q,clk,d); initial $display(\"x\"); endmodule\n",
        "at most one edge column per UDP table row",
    );
}

#[test]
fn udp_seq_vector_output_is_loud() {
    rejects(
        "primitive p(q,d); output reg [1:0] q; input d;\n\
           table 0:?:0; endtable endprimitive\n\
         module top; reg d; wire [1:0] q; p u(q,d); initial $display(\"x\"); endmodule\n",
        "scalar (1-bit) UDP output",
    );
}

#[test]
fn udp_seq_edge_endpoint_b_is_loud() {
    rejects(
        "primitive p(q,clk); output reg q; input clk;\n\
           table (b1):?:1; endtable endprimitive\n\
         module top; reg clk; wire q; p u(q,clk); initial $display(\"x\"); endmodule\n",
        "UDP edge endpoint (0 1 x ?)",
    );
}

#[test]
fn udp_seq_malformed_edge_paren_is_loud() {
    rejects(
        "primitive p(q,clk); output reg q; input clk;\n\
           table (011):?:1; endtable endprimitive\n\
         module top; reg clk; wire q; p u(q,clk); initial $display(\"x\"); endmodule\n",
        "two-symbol edge pair (vw) closed by ')'",
    );
}

#[test]
fn udp_seq_initial_non_01x_is_loud() {
    rejects(
        "primitive p(q,clk,d); output reg q; input clk,d;\n\
           initial q = 1'bz;\n\
           table (01) ?:?:1; endtable endprimitive\n\
         module top; reg clk,d; wire q; p u(q,clk,d); initial $display(\"x\"); endmodule\n",
        "UDP initial value of 0, 1, or x",
    );
}

#[test]
fn udp_seq_three_colons_is_loud() {
    rejects(
        "primitive p(q,clk,d); output reg q; input clk,d;\n\
           table (01) ?:?:?:1; endtable endprimitive\n\
         module top; reg clk,d; wire q; p u(q,clk,d); initial $display(\"x\"); endmodule\n",
        "at most two colons in a UDP table row",
    );
}

#[test]
fn udp_seq_reg_but_combinational_table_is_loud() {
    // `output reg` (a sequential marker) but a table with NO sequential content
    // (no edge / no `-` / no state column / no two-colon row) is internally
    // inconsistent ⇒ loud (iverilog actually ASSERTION-CRASHES on this form).
    rejects(
        "primitive p(q,a,b); output reg q; input a,b;\n\
           table 0 0:0; 1 1:1; endtable endprimitive\n\
         module top; reg a,b; wire q; p u(q,a,b); initial $display(\"x\"); endmodule\n",
        "sequential UDP table",
    );
}

#[test]
fn udp_seq_wire_output_with_sequential_table_is_loud() {
    // The MIRROR inconsistency (caught by the independent adversarial review): a
    // plain `wire` output (no `reg`) with a SEQUENTIAL table (two-colon / edge /
    // `-` / state) requires a `reg` output per IEEE §29.7. iverilog aborts; vita
    // must loud-reject, not silently synthesize a working DFF.
    rejects(
        "primitive p(q, clk, d); output q; input clk, d;\n\
           table (01) 0:?:0; (01) 1:?:1; (?0) ?:?:-; ? (??):?:-; endtable endprimitive\n\
         module top; reg clk,d; wire q; p u(q,clk,d);\n\
         initial begin clk=0;d=1;#1 clk=1;#1 $display(\"q=%b\",q); end endmodule\n",
        "`reg` output",
    );
}

#[test]
fn udp_seq_initial_without_reg_is_loud() {
    // `initial OUT=…;` is NOT a `reg` declaration — a sequential UDP still requires
    // the output declared `reg` (iverilog rejects `output q; initial q=…;`).
    rejects(
        "primitive p(q, clk, d); output q; initial q=1'b0; input clk, d;\n\
           table (01) 0:?:0; (01) 1:?:1; endtable endprimitive\n\
         module top; reg clk,d; wire q; p u(q,clk,d);\n\
         initial begin clk=0;d=1;#1 clk=1;#1 $display(\"q=%b\",q); end endmodule\n",
        "`reg` output",
    );
}
