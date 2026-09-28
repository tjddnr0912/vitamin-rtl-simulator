//! §3 ⑤ⓗ: a `typedef enum` declared INSIDE a generate block. Its labels are
//! declarations of that block (IEEE 1800-2017 §6.19 with §27.3), and elaborate never
//! bound them there: every read was E3010 undeclared, and a read of a label that
//! shared its name with an outer constant or net took the OUTER object in silence.
//! The corpus row `ibex` stopped on it (`ibex_multdiv_fast.sv:268`, `gen_mult_fast`),
//! which the first test reduces.
//!
//! The labels now bind at the block's own key in every generate phase, beside the
//! block's parameters — for a CARRIED typedef (`elaborate/src/gen_enum.rs`): written
//! directly in the block, literal base and values, no label read above it, and every
//! label name declared once in the block. Any other generate typedef keeps its labels
//! unbound, as before. Three neighbours were in the same path and are closed with it:
//!
//! - `.name()` keyed its synthetic function by the TYPE name, so two sibling blocks
//!   declaring one type name with different labels shared the first block's function;
//! - the parser's parse-time tables (constant folds, multi-dimensional packed
//!   parameter selects, struct and enum variables) did not see a label as a
//!   declaration, so an outer constant of its name answered a fold inside the block;
//! - a generate branch written without `begin … end` had no parser scope, so what a
//!   declaration there hid stayed hidden for the rest of the module.
//!
//! Oracles: verilator 5.052 (`--binary --timing`) and sv2v 0.0.13 → iverilog 13.0.
//! iverilog cannot read the original: it does not bind a generate block's enum label
//! (`Unable to bind wire/reg/memory`, then a segfault). Where sv2v cannot lower a
//! construct (the enum methods), verilator is the one that ran, and the comment says so.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_gbel_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let all =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (all, out.status.code())
}

/// The design's own lines that start with `tag` (`""` = all of them), without vita's
/// warnings and end-of-run lines.
fn prints(src: &str, tag: &str, want: &[&str]) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{out}");
    let got: Vec<&str> = out
        .lines()
        .filter(|l| {
            l.starts_with(tag)
                && !l.starts_with("warning[")
                && !l.starts_with("simulation ended")
                && !l.starts_with("errors=")
        })
        .collect();
    assert_eq!(got, want, "{out}");
}

fn is_loud(src: &str, needle: &str) -> String {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(1), "expected a refusal\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in\n{out}");
    out
}

// ───────────────────────────── values ─────────────────────────────

#[test]
fn the_ibex_multiplier_state_machine_runs_in_either_branch() {
    // `ibex_multdiv_fast`, reduced: each generate branch declares `mult_fsm_e` with
    // its own labels and steps a state machine through them. Both instances, reset
    // with a falling edge.
    let src = r#"
module mul #(parameter int K = 1) (input logic clk, input logic rst_n, output logic [1:0] st, output logic idle);
  if (K == 0) begin : g_single
    typedef enum logic { MULL, MULH } fsm_e;
    fsm_e q, d;
    always_comb begin
      d = q;
      case (q)
        MULL: d = MULH;
        MULH: d = MULL;
        default: d = MULL;
      endcase
    end
    always_ff @(posedge clk or negedge rst_n)
      if (!rst_n) q <= MULL; else q <= d;
    assign st = {1'b0, q};
    assign idle = q == MULL;
  end else begin : g_fast
    typedef enum logic [1:0] { ALBL, ALBH, AHBL, AHBH } fsm_e;
    fsm_e q, d;
    always_comb begin
      d = q;
      unique case (q)
        ALBL: d = ALBH;
        ALBH: d = AHBL;
        AHBL: d = AHBH;
        AHBH: d = ALBL;
        default: d = ALBL;
      endcase
    end
    always_ff @(posedge clk or negedge rst_n)
      if (!rst_n) q <= ALBL; else q <= d;
    assign st = q;
    assign idle = q == ALBL;
  end
endmodule
module t;
  logic clk = 0, rst_n = 1;
  logic [1:0] s0, s1; logic i0, i1;
  mul #(.K(0)) u0 (.clk(clk), .rst_n(rst_n), .st(s0), .idle(i0));
  mul #(.K(1)) u1 (.clk(clk), .rst_n(rst_n), .st(s1), .idle(i1));
  initial begin
    #1 rst_n = 0; #1 rst_n = 1;
    repeat (6) begin
      #5 clk = 1; #5 clk = 0;
      $display("P s0=%0d i0=%0d s1=%0d i1=%0d", s0, i0, s1, i1);
    end
    $finish;
  end
endmodule
"#;
    prints(
        src,
        "P ",
        &[
            "P s0=1 i0=0 s1=1 i1=0",
            "P s0=0 i0=1 s1=2 i1=0",
            "P s0=1 i0=0 s1=3 i1=0",
            "P s0=0 i0=1 s1=0 i1=1",
            "P s0=1 i0=0 s1=1 i1=0",
            "P s0=0 i0=1 s1=2 i1=0",
        ],
    );
}

#[test]
fn a_label_is_a_constant_of_its_block_at_its_declared_width() {
    // Values, a label in a localparam and in a range bound, `$bits`, a concat, a
    // negation and a comparison at the base's width — and two labels named like an
    // outer constant `A` and an outer net `N`: inside the block they are the labels,
    // outside the outer objects. PRE printed E3010 for the rest and took the outer
    // `A` / `N` inside the block without a word.
    let src = r#"
module t;
  localparam int A = 9;
  logic [7:0] N = 8'd77;
  if (1) begin : g
    typedef enum logic [2:0] { A, B = 3'd5, C, N } e_t;
    localparam int X = C + 1;
    logic [C:0] v;
    e_t s = B;
    initial begin
      #1;
      $display("G A=%0d B=%0d C=%0d N=%0d X=%0d bitsv=%0d bitsA=%0d s=%0d", A, B, C, N, X, $bits(v), $bits(A), s);
      $display("G cat=%b neg=%0d cmp=%0d", {A, C}, -C, (C > 3'd4));
    end
  end
  initial begin
    #2 $display("T A=%0d N=%0d", A, N);
  end
endmodule
"#;
    prints(
        src,
        "",
        &[
            "G A=0 B=5 C=6 N=7 X=7 bitsv=7 bitsA=3 s=5",
            "G cat=000110 neg=2 cmp=1",
            "T A=9 N=77",
        ],
    );
}

#[test]
fn a_loop_iteration_binds_its_own_labels() {
    // A label beside the genvar in a child's parameter override and in a nested
    // generate-if condition, per iteration. Both oracles. A label whose VALUE follows
    // the genvar is not carried (its value reads a name) and stays unbound.
    let src = r#"
module sub #(parameter int P = 0) (output logic [7:0] o);
  assign o = 8'(P);
endmodule
module t;
  logic [7:0] o [3];
  logic [7:0] q [3];
  for (genvar i = 0; i < 3; i++) begin : g
    typedef enum logic [3:0] { A = 4'd2, B, C } e_t;
    sub #(.P(C + 4'(i))) u (.o(o[i]));
    if (B + 4'(i) == 4) begin : hit
      assign q[i] = 8'd100 + 8'(A);
    end else begin : miss
      assign q[i] = 8'd200 + 8'(B);
    end
  end
  initial #1 $display("F o=%0d,%0d,%0d q=%0d,%0d,%0d", o[0], o[1], o[2], q[0], q[1], q[2]);
endmodule
"#;
    prints(src, "F ", &["F o=4,5,6 q=203,102,203"]);
    is_loud(
        r#"
module t;
  for (genvar i = 0; i < 2; i++) begin : g
    typedef enum logic [3:0] { A = 4'(i + 2), B } e_t;
    initial #1 $display("F %0d", B);
  end
endmodule
"#,
        "undeclared net/variable `t.g[0].B`",
    );
}

#[test]
fn routines_of_the_block_read_its_labels_and_a_module_routine_does_not() {
    let src = r#"
module t;
  localparam int A = 50;
  function automatic int mf(); return A; endfunction
  if (1) begin : g
    typedef enum logic [2:0] { A = 3'd3, B } e_t;
    function automatic logic [2:0] gf(input logic [2:0] x); return x + B; endfunction
    task automatic tk(output int r); r = A * 10; endtask
    int r;
    initial begin
      tk(r);
      $display("FN gf=%0d mf=%0d tk=%0d", gf(3'd1), mf(), r);
    end
  end
endmodule
"#;
    prints(src, "FN ", &["FN gf=5 mf=50 tk=30"]);
}

#[test]
fn case_arms_unlabelled_blocks_and_nesting() {
    // A `case` arm, an unlabelled `if` block (`genblk2`), a nested block reading its
    // parent's label.
    let src = r#"
module t #(parameter int K = 2);
  case (K)
    1: begin : c1 typedef enum logic [1:0] { X = 2'd1, Y } e_t; initial #1 $display("C1 %0d", Y); end
    2: begin : c2 typedef enum logic [2:0] { X = 3'd5, Y } e_t; initial #1 $display("C2 %0d %0d", X, Y); end
  endcase
  if (1) begin
    typedef enum { U0 = 7, U1 } u_t;
    initial #2 $display("U %0d %0d", U0, U1);
    if (1) begin : inner
      initial #3 $display("I %0d", U1 + 2);
    end
  end
endmodule
"#;
    prints(src, "", &["C2 5 6", "U 7 8", "I 10"]);
}

#[test]
fn a_typedef_in_a_generate_region_is_not_a_blocks() {
    // `generate … endgenerate` is not a scope (§27.2): its typedef's labels belong to
    // the module, whose passes do not see inside a region. Both oracles `M 1`; vita
    // keeps the path it had.
    is_loud(
        r#"
module t;
  generate
    typedef enum logic [1:0] { R0, R1, R2 } r_t;
  endgenerate
  initial #4 $display("M %0d", R1);
endmodule
"#,
        "undeclared net/variable `t.R1`",
    );
}

#[test]
fn a_struct_typedef_above_is_read_for_its_member_widths() {
    // Its members `A` / `B` are not reads of the labels, and the routine's formal `C`
    // hides the label `C` in its body. Both oracles.
    let src = r#"
module t;
  if (1) begin : g
    typedef struct packed { logic [1:0] A; logic B; } s_t;
    typedef enum logic [1:0] { A = 2'd1, C } e_t;
    function automatic int f(input int C); return C * 2; endfunction
    s_t sv;
    initial begin sv = 3'b101; #1 $display("R A=%0d C=%0d f=%0d svA=%0d", A, C, f(5), sv.A); end
  end
endmodule
"#;
    prints(src, "R ", &["R A=1 C=2 f=10 svA=2"]);
}

#[test]
fn a_loop_variable_that_shares_the_labels_key_leaves_the_label_intact() {
    // The loop inside `g` binds its genvar `i` at `g`'s key while it runs, and the
    // routine in the loop body replays it later; the label `i` is still 1 afterwards.
    // Both oracles.
    let src = r#"
module t;
  if (1) begin : g
    typedef enum logic [1:0] { i = 2'd1, j } e_t;
    for (genvar i = 0; i < 2; i++) begin : h
      function automatic int f(input int x); return x + i; endfunction
      int r;
      initial begin r = f(10); #1 $display("L i=%0d r=%0d", i, r); end
    end
    initial #2 $display("V i=%0d j=%0d", i, j);
  end
endmodule
"#;
    prints(src, "", &["L i=0 r=10", "L i=1 r=11", "V i=1 j=2"]);
}

#[test]
fn signed_int_keyword_and_two_state_bases() {
    // sv2v → iverilog prints the base-less `int` enum's labels unsigned
    // (`4294967291`); verilator prints -5, which is §6.19's `int` base.
    let src = r#"
module t;
  if (1) begin : g
    typedef enum logic signed [2:0] { SA = -3'sd2, SB, SC } s_t;
    typedef enum { IA = -5, IB } i_t;
    typedef enum bit [1:0] { BA, BB } b_t;
    typedef enum byte { YA = 8'd3, YB } y_t;
    initial #1 begin
      $display("S %0d %0d %0d cmp=%0d", SA, SB, SC, SA < SC);
      $display("I %0d %0d bits=%0d", IA, IB, $bits(IA));
      $display("B %0d %0d bits=%0d", BA, BB, $bits(BB));
      $display("Y %0d %0d bits=%0d", YA, YB, $bits(YB));
    end
  end
endmodule
"#;
    prints(
        src,
        "",
        &[
            "S -2 -1 0 cmp=1",
            "I -5 -4 bits=32",
            "B 0 1 bits=2",
            "Y 3 4 bits=8",
        ],
    );
    // A base naming a constant — the block's or the module's — reads a name, so the
    // typedef is not carried (both oracles `W 30 31` / `O 3 4`).
    for decl in [
        "localparam int W2 = 5;\n    typedef enum logic [W2-1:0] { WA = 5'd30, WB } w_t;",
        "typedef enum logic [W-1:0] { WA = W, WB } w_t;",
    ] {
        is_loud(
            &format!(
                "module t;\n  localparam int W = 3;\n  if (1) begin : g\n    {decl}\n    initial #1 $display(\"W %0d\", WB);\n  end\nendmodule\n"
            ),
            "undeclared net/variable `t.g[0].WB`",
        );
    }
}

#[test]
fn a_block_local_variable_shadows_the_label_inside_its_process() {
    let src = r#"
module t;
  if (1) begin : g
    typedef enum logic [1:0] { A, B } e_t;
    initial begin : b
      logic [7:0] A;
      A = 8'd5;
      $display("G1 A=%0d B=%0d", A, B);
    end
    initial #1 $display("G2 A=%0d", A);
  end
endmodule
"#;
    prints(src, "G", &["G1 A=5 B=1", "G2 A=0"]);
}

#[test]
fn an_event_control_on_a_label_that_shadows_a_net_never_wakes() {
    // One of the thirteen enum twins of `bare_ident_route_readers.rs`: verilator
    // never wakes on the label; iverilog, which does not see it, wakes on the net.
    let src = r#"
module top;
  logic V;
  initial begin V = 0; #2 V = 1; end
  generate if (1) begin : g
    typedef enum int { V = 99 } e_t;
    initial begin @(posedge V); $display("EDGE at %0t", $time); end
  end endgenerate
  initial #5 begin $display("DONE"); $finish; end
endmodule
"#;
    prints(src, "", &["DONE"]);
}

#[test]
fn a_task_actual_that_names_a_shadowing_label_passes_the_label() {
    // Another enum twin: the input actual `V` is the label (0x63), not the net.
    let src = r#"
module top;
  logic [7:0] V;
  initial V = 8'h10;
  generate if (1) begin : g
    typedef enum int { V = 99 } e_t;
    logic [7:0] z;
    task automatic bump(inout logic [7:0] a, input logic [7:0] b); a = a + b; endtask
    initial begin #1; z = 8'h01; bump(z, V); $display("Z=%h", z); end
  end endgenerate
  initial #5 $finish;
endmodule
"#;
    prints(src, "Z=", &["Z=64"]);
}

// ──────────────────────── the enum methods ────────────────────────

#[test]
fn name_is_the_variables_own_type_when_two_blocks_share_a_type_name() {
    // verilator (sv2v has no enum methods). Before: `B R` — block `a`'s function
    // answered for block `b`'s type — and `M R` for the module's own.
    let src = r#"
module t;
  typedef enum logic [1:0] { M0, M1, M2, M3 } e_t;
  e_t m = e_t'(2);
  if (1) begin : a
    typedef enum logic [1:0] { P, Q, R } e_t;
    e_t s = Q;
    initial #1 $display("A %s %0d %0d %0d", s.name(), s.next(), s.first(), s.last());
  end
  if (1) begin : b
    typedef enum logic [1:0] { W, X2, Y2, Z2 } e_t;
    e_t s = Y2;
    initial #2 $display("B %s %0d %0d", s.name(), s.next(), s.num());
  end
  initial #3 $display("M %s", m.name());
endmodule
"#;
    prints(src, "", &["A Q 2 0 2", "B Y2 3 4", "M M2"]);
}

// ─────────────────────── parse-time tables ────────────────────────

#[test]
fn a_label_hides_an_outer_multi_dimensional_packed_parameter() {
    // verilator. The parser rewrote `P[1]` through the module's `[1:0][3:0] P`
    // layout (`x`); the label is 3'd6, so bit 1 is 1.
    let src = r#"
module t;
  localparam logic [1:0][3:0] P = 8'hA5;
  if (1) begin : g
    typedef enum logic [2:0] { P = 3'd6, Q } e_t;
    initial #1 $display("PM %0d %b %0d", P[1], P, Q);
  end
  initial #2 $display("PO %h", P[1]);
endmodule
"#;
    prints(src, "P", &["PM 1 110 7", "PO a"]);
}

#[test]
fn a_package_typedef_bound_by_a_package_label_resolves_in_the_package() {
    // The typedef's twin names the package's own label (`p::L2`), as it names the
    // package's own constants: E3009 before, and the importer's `L2 = 7` (eight bits)
    // where the importer declares one. Both oracles: three bits.
    let pkg = r#"
package p;
  typedef enum logic [1:0] { L0, L1, L2 } e_t;
  typedef logic [L2:0] v_t;
endpackage
"#;
    prints(
        &format!(
            "{pkg}module t;\n  p::v_t x;\n  initial begin x = '1; #1 $display(\"PK bits=%0d x=%b\", $bits(x), x); end\nendmodule\n"
        ),
        "PK",
        &["PK bits=3 x=111"],
    );
    prints(
        &format!(
            "{pkg}module t;\n  localparam int L2 = 7;\n  p::v_t x;\n  initial begin x = '1; #1 $display(\"PL bits=%0d x=%b L2=%0d\", $bits(x), x, L2); end\nendmodule\n"
        ),
        "PL",
        &["PL bits=3 x=111 L2=7"],
    );
}

#[test]
fn a_value_naming_another_label_is_not_carried() {
    // `C = A + 4` reads a name (the first typedef's label, which also shadows the
    // module's `A = 2`): verilator `C=4 D=5`, and the parser folded 6 for the methods.
    // The second typedef is not carried, so its labels are E3010 as before; the first,
    // literal one binds.
    is_loud(
        r#"
module t;
  localparam int A = 2;
  if (1) begin : g
    typedef enum logic [1:0] { A, B } e_t;
    typedef enum logic [2:0] { C = A + 4, D } f_t;
    initial #1 $display("PF A=%0d C=%0d", A, C);
  end
endmodule
"#,
        "undeclared net/variable `t.g[0].C`",
    );
    prints(
        r#"
module t;
  localparam int A = 2;
  if (1) begin : g
    typedef enum logic [1:0] { A, B } e_t;
    typedef enum logic [2:0] { C = A + 4, D } f_t;
    initial #1 $display("PF A=%0d B=%0d", A, B);
  end
endmodule
"#,
        "PF",
        &["PF A=0 B=1"],
    );
    // A generate-array index naming such a label: both oracles `GI 10` (`h[0]`); the
    // parser folded the outer `A = 1` and printed 11. It is a parse error now.
    is_loud(
        r#"
module t;
  localparam int A = 1;
  if (1) begin : g
    typedef enum logic [1:0] { A, B } e_t;
    for (genvar i = 0; i < 2; i++) begin : h
      wire [7:0] w = 8'd10 + 8'(i);
    end
    initial #1 $display("GI %0d", h[A].w);
  end
endmodule
"#,
        "expected a constant generate-array index",
    );
}

#[test]
fn a_module_label_still_hides_a_wildcard_import() {
    // Control for the parser change at module scope: unchanged, both oracles.
    let src = r#"
package q;
  localparam int K = 1;
endpackage
module t;
  import q::*;
  typedef enum logic [1:0] { K, M } e_t;
  typedef enum logic [2:0] { C = K + 4, D } f_t;
  f_t v = D;
  initial #1 $display("IM K=%0d C=%0d D=%0d v=%0d", K, C, D, v);
endmodule
"#;
    prints(src, "IM", &["IM K=0 C=4 D=5 v=5"]);
}

// ───────────────────────────── refusals ─────────────────────────────

#[test]
fn a_typedef_whose_label_another_declaration_of_its_block_takes_is_not_carried() {
    // Every such pair is illegal — both oracles refuse it ("Duplicate declaration of
    // enum value", "… has the same name as ENUMITEM") — and vita never refused it:
    // before this slice the labels were unbound, and a read of the other declaration's
    // name took it. The typedef is left unbound as before (its other label `B` is
    // E3010), so no binding shares a key with the other declaration.
    for decls in [
        "localparam int A = 1;\n    typedef enum logic [1:0] { A, B } e_t;",
        "typedef enum logic [1:0] { A, B } e_t;\n    localparam int A = 1;",
        "logic [7:0] A = 8'd9;\n    typedef enum logic [1:0] { A, B } e_t;",
        "typedef enum logic [1:0] { A, B } e_t;\n    typedef enum logic [2:0] { C = 3'd4, A } f_t;",
        "typedef enum logic [1:0] { A, B } e_t;\n    function automatic int A(); return 7; endfunction",
        "typedef enum logic [1:0] { A, B } e_t;\n    sub A ();",
        "typedef enum logic [1:0] { A, B } e_t;\n    if (1) begin : A end",
        "typedef enum logic [1:0] { A, B } e_t;\n    if (0) begin : X end else begin : A end",
        "typedef enum logic [1:0] { A, B } e_t;\n    if (0) begin : X end else if (0) begin : Y end else begin : A end",
        "typedef enum logic [1:0] { A, B } e_t;\n    case (1) 1: begin : A end endcase",
        "typedef enum logic [1:0] { A, B } e_t;\n    for (genvar k = 0; k < 1; k++) begin : A end",
        "import q::A;\n    typedef enum logic [1:0] { A, B } e_t;",
    ] {
        let src = format!(
            "package q; localparam int A = 5; endpackage\nmodule t;\n  import q::A;\n  if (1) begin : g\n    {decls}\n    initial #1 $display(\"CB B=%0d\", B);\n  end\nendmodule\nmodule sub; endmodule\n"
        );
        is_loud(&src, "undeclared net/variable `t.g[0].B`");
    }
    // The loop variable is an implicit localparam of each iteration's block (§27.4):
    // verilator "Variable has same name as ENUMITEM 'i'".
    is_loud(
        "module t;\n  for (genvar i = 0; i < 2; i++) begin : g\n    typedef enum logic [2:0] { X = 3'd4, i } e_t;\n    initial #1 $display(\"R X=%0d\", X);\n  end\nendmodule\n",
        "undeclared net/variable `t.g[0].X`",
    );
    // …and in an untaken branch, which nothing elaborates, it still runs as before.
    let (out, rc) = run(
        "module t;\n  if (0) begin : g\n    localparam int A = 1;\n    typedef enum logic [1:0] { A, B } e_t;\n  end\n  initial #1 $display(\"ok\");\nendmodule\n",
    );
    assert_eq!(rc, Some(0), "{out}");
}

#[test]
fn a_typedef_whose_inputs_are_not_facts_where_it_stands_keeps_its_labels_unbound() {
    // Its value names a declaration BELOW it in its block. Both oracles resolve by
    // scope (`X=9 Y=10`, the block's `LP`); the first generate phase would have folded
    // the module's `LP = 3`. So the labels stay unbound: E3010 at a read, as before.
    is_loud(
        r#"
module t;
  localparam int LP = 3;
  if (1) begin : g
    typedef enum logic [0:3] { X = LP, Y } e_t;
    localparam int LP = 9;
    initial #1 $display("FR X=%0d Y=%0d", X, Y);
  end
endmodule
"#,
        "undeclared net/variable `t.g[0].X`",
    );
    // …and a design that never reads them runs as it did (both oracles).
    prints(
        r#"
module t;
  localparam int LP = 3;
  if (1) begin : g
    typedef enum logic [3:0] { X = LP, Y } e_t;
    localparam int LP = 9;
    e_t v;
    initial begin v = e_t'(4'd12); #1 $display("G v=%0d LP=%0d", v, LP); end
  end
endmodule
"#,
        "G ",
        &["G v=12 LP=9"],
    );
    // A value naming a LATER typedef's label, likewise — and that later typedef is
    // read above itself, so neither binds.
    prints(
        r#"
module t;
  if (1) begin : g
    typedef enum logic [3:0] { A = B2, A2 } e1_t;
    typedef enum logic [3:0] { B2 = 4'd5, B3 } e2_t;
    e1_t v;
    initial begin v = e1_t'(4'd9); #1 $display("G v=%0d", v); end
  end
endmodule
"#,
        "G ",
        &["G v=9"],
    );
    // A read ABOVE the typedef: position decides in vita, scope in the oracles.
    is_loud(
        r#"
module t;
  if (1) begin : g
    localparam int Z = B + 1;
    typedef enum logic [1:0] { A, B } e_t;
    initial #1 $display("RA Z=%0d B=%0d", Z, B);
  end
endmodule
"#,
        "generate-scope parameter `Z` value is not a constant",
    );
}

#[test]
fn a_begin_less_branch_is_a_scope_to_the_parser() {
    // `if (1) <item>` is a generate block (§27.5). What a declaration there hides —
    // a label or a net named like the module's `[1:0][3:0] P` — stayed hidden after
    // it, so the module's `P[1]` read `0` (both oracles `a`; PRE `a` for the label,
    // and `0` for the net, which was wrong before this slice).
    for decl in ["typedef enum logic [1:0] { P, Q } e_t;", "logic [7:0] P;"] {
        let src = format!(
            "module t;\n  localparam logic [1:0][3:0] P = 8'hA5;\n  if (1) {decl}\n  initial #1 $display(\"U P1=%h P0=%h\", P[1], P[0]);\nendmodule\n"
        );
        prints(&src, "U ", &["U P1=a P0=5"]);
    }
}

#[test]
fn a_block_type_whose_labels_follow_a_parameter_is_not_carried() {
    // `P = 2'(K)` reads a parameter, so the typedef is not carried: its labels stay
    // unbound (verilator `R Q 2 first=1`), and a MODULE variable of the module's `e_t`
    // still answers its methods from the module's type inside the block (verilator,
    // and PRE, `R Y num=3 first=0`).
    is_loud(
        r#"
module t #(parameter int K = 1);
  typedef enum logic [1:0] { M0, M1, M2 } e_t;
  if (1) begin : g
    typedef enum logic [1:0] { P = 2'(K), Q } e_t;
    e_t s = Q;
    initial #1 $display("R %s %0d first=%0d", s.name(), s, s.first());
  end
endmodule
"#,
        "undeclared net/variable `t.g[0].Q`",
    );
    prints(
        r#"
module t #(parameter int K = 1);
  typedef enum logic [1:0] { X, Y, Z } e_t;
  e_t a = Y;
  if (1) begin : g
    typedef enum logic [1:0] { P = 2'(K), Q } e_t;
    initial #1 $display("R %s num=%0d first=%0d", a.name(), a.num(), a.first());
  end
endmodule
"#,
        "R ",
        &["R Y num=3 first=0"],
    );
}

#[test]
fn a_bare_begin_end_inside_a_block_is_left_alone() {
    // Both oracles give a bare `begin … end` a scope of its own, and vita flattens it
    // into the block, so a typedef there is not carried and its labels are no
    // collision with the block's names. Both oracles, and PRE, `R v=6`.
    for src in [
        "module t;\n  for (genvar i = 0; i < 2; i++) begin : g\n    begin\n      typedef enum logic [7:0] {i = 5, X} e_t;\n      e_t v = e_t'(6);\n      initial #1 $display(\"R v=%0d\", v);\n    end\n  end\nendmodule\n",
        "module t;\n  if (1) begin : g\n    localparam A = 1;\n    begin\n      typedef enum logic [7:0] {A = 5, B} e_t;\n      e_t v = e_t'(6);\n      initial #1 $display(\"R v=%0d\", v);\n    end\n  end\nendmodule\n",
    ] {
        let (out, rc) = run(src);
        assert_eq!(rc, Some(0), "{out}");
        assert!(out.lines().filter(|l| l.starts_with("R ")).all(|l| l == "R v=6"), "{out}");
    }
}

#[test]
fn a_folded_type_size_is_not_a_literal() {
    // The parser folds `$bits(w_t)` to a decimal where it stands, and here `w_t`'s
    // bound names the block's `X = 8` declared below it: verilator `A=8 bits=9`, the
    // positional fold 2. The folded decimal carries the call's span, so the typedef is
    // not carried.
    is_loud(
        r#"
module t;
  localparam X = 2;
  if (1) begin : g
    typedef logic [X-1:0] w_t;
    typedef enum logic [7:0] {A = $bits(w_t)} e_t;
    localparam X = 8;
    initial #1 $display("q1c A=%0d", A);
  end
endmodule
"#,
        "undeclared net/variable `t.g[0].A`",
    );
}

#[test]
fn a_hierarchical_read_of_a_block_label_stays_loud() {
    // verilator `H 4`; sv2v → iverilog cannot bind `g.B` either.
    is_loud(
        r#"
module t;
  if (1) begin : g
    typedef enum logic [2:0] { A = 3'd3, B } e_t;
  end
  initial #1 $display("H %0d", g.B);
endmodule
"#,
        "undeclared hierarchical name `g.B`",
    );
}
