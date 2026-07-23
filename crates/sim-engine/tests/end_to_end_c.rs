//! End-to-end sim-engine tests: build a SimIr via the real lex → parse →
//! elaborate pipeline, simulate it, and assert on captured $display output and
//! the generated VCD file.

use sim_engine::{simulate, simulate_capture, FinishReason, SimOpts};

#[path = "end_to_end_util/mod.rs"]
mod util;
#[allow(unused_imports)]
use util::*;

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

// E2. `**` with a real operand is SUPPORTED (IEEE 1800 §11.4.9: real result =
// pow(base, exp)) — it desugars to the $pow sysfunc (libm::pow). Was previously a
// §6.2 ElabUnsupported reject. iverilog-pinned values.
#[test]
fn real_power_supported_via_pow() {
    let diags = elaborate_diags(
        "module t; real r; initial begin r = 2.0 ** 3; $display(\"%g\", r); end endmodule",
    );
    assert!(
        !diags.iter().any(|d| d.contains("power (**) not defined")),
        "real ** must no longer be rejected, got: {diags:?}"
    );
    // real base/int exp, int base/real exp, fractional exp, negative base — all
    // fold via libm::pow (iverilog: 8, 8, 3, -8).
    let ir = build(
        "module t; initial begin \
         $display(\"%g %g %g %g\", 2.0**3, 2**3.0, 9.0**0.5, (-2.0)**3); \
         $finish; end endmodule",
    );
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "8 8 3 -8\n");
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
    // y reads X (driven-unknown) until the delayed driver propagates at t=5, then
    // 7; a=3 at t=6 → y=3 at t=11 (seen at t=12). `%0d` of an all-X field prints
    // `x` per §21.2.1.2. A continuous-assign driver (even a delayed one) drives
    // its net from t=0, so before its first output the net is X, NOT the
    // unconnected-net Z — iverilog-pinned (the driver's output register inits to
    // X; the delay applies to transitions). This previously read Z (the net was
    // left at its undriven wire default until the delayed write landed).
    assert_eq!(out, "t2 y=x\nt6 y=7\nt12 y=3\n");
}

#[test]
fn delayed_driver_initial_x_propagates_before_first_output() {
    // A DELAYED gate's output holds X (driven-unknown) during [0, d), and that X
    // must PROPAGATE through a downstream (undelayed) cont-assign — the fix drives
    // the initial X inside the settle fixpoint, so `d = g` sees g==X (not the
    // stale Z default). `and #4 (g,a,1)` with a=1 ⇒ g=X until t=4 then 1; d
    // follows g. iverilog-pinned: `t2 g=x d=x`, `t6 g=1 d=1`.
    let src = "module t; reg a; wire g; wire d; \
               and #4 (g, a, 1'b1); assign d = g; \
               initial begin a = 1; \
                 #2 $display(\"t2 g=%b d=%b\", g, d); \
                 #4 $display(\"t6 g=%b d=%b\", g, d); \
                 $finish; end endmodule";
    let ir = build(src);
    let (_res, out) = simulate_capture(&ir, SimOpts::default());
    assert_eq!(out, "t2 g=x d=x\nt6 g=1 d=1\n");
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
