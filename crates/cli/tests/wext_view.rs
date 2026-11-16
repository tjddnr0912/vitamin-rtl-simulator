//! §5.2 Q-WREXT: the write-extent view (`elaborate/src/wext.rs`, `Sidecars::write_view`) as
//! the end phase builds it, one design per writer kind and per extent kind. Each pin is the
//! view's whole dump (`WriteView::render`): one `W` line per write site (writer, form,
//! statement, chunk, net, extent), one `N` line per net it names.
//!
//! The view is a fact about vita's own finished IR, not a language value, so no oracle
//! decides these lines. What stands behind an extent: `Bits` is
//! `sim_ir::const_chunk_written_bits`'s answer, and that kernel is pinned against what the
//! engine's stores write by `sim-engine`'s `const_chunk_written_bits_is_what_the_stores_write`
//! (the anchor) ; the view adds no arithmetic. Which process an inlined task's write belongs
//! to, and that a function writes in its frame, is how the IR lowers them (premise judge 1
//! K5, W/mdrv/premise/REPORT.txt), recorded as is.
//!
//! No user-visible output moves (the view has no production reader): the record lanes
//! compare every stdout, stderr and exit code with PRE (Q-WREXT build r1, B/run_record.sh).
//! The designs are elaborated without a span resolver, so no `@file:line:col` is printed.

use diag::{LogEvent, LogSink};
use std::collections::BTreeMap;

struct Quiet;
impl LogSink for Quiet {
    fn emit(&self, _: LogEvent) {}
}

/// The view's dump for one design that elaborates clean.
fn view(src: &str) -> String {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let unit = su.expect("source unit");
    let (ir, sc) = elaborate::elaborate_with_timescale(&unit, &Quiet, &BTreeMap::new(), -9);
    let ir = ir.expect("the design elaborates");
    sc.write_view.render(&ir, &sc)
}

fn pin(src: &str, want: &str) {
    let got = view(src);
    assert_eq!(got, want, "\n--- got ---\n{got}--- want ---\n{want}");
}

/// A continuous part-select and an `always_comb` part-select on one variable: two `Bits`
/// extents by the kernel, `[5:2]` and `[3:0]` (matrix cell m_a_ca_acomb_ppo's pair).
#[test]
fn w01_cont_and_comb_parts() {
    pin(
        "module t; logic [7:0] v; logic [3:0] s;
  assign v[5:2] = s;
  always_comb v[3:0] = ~s;
endmodule
",
        "\
WEXT writes=2 bits=2 word=0 undecided=0 poison=0
W C0:assign cont - c0 n0:t.v bits[5:2]
W P0:always_comb blk s0 c0 n0:t.v bits[3:0]
N n0:t.v Logic w=8 alen=1 dims=n
",
    );
}

/// The four statement forms (`blk`, `nba`, `force`, `release`) and a procedural `assign` /
/// `deassign`, which lowers to a `Force` / `Release` statement (`Sidecars::assign_ranks`
/// names those statements): the view keeps the form, a reader decides.
#[test]
fn w02_statement_forms() {
    pin(
        "module t; logic clk; logic [7:0] v, w, r;
  always_ff @(posedge clk) v[7:4] <= w[3:0];
  initial begin
    w = 8'h5a;
    force r = 8'h0f;
    release r;
    assign v = 8'h00;
    deassign v;
  end
endmodule
",
        "\
WEXT writes=6 bits=6 word=0 undecided=0 poison=0
W P0:always_ff nba s0 c0 n1:t.v bits[7:4]
W P1:initial blk s1 c0 n2:t.w bits[7:0]
W P1:initial force s2 c0 n3:t.r bits[7:0]
W P1:initial release s3 c0 n3:t.r bits[7:0]
W P1:initial force s4 c0 n1:t.v bits[7:0]
W P1:initial release s5 c0 n1:t.v bits[7:0]
N n1:t.v Logic w=8 alen=1 dims=n
N n2:t.w Logic w=8 alen=1 dims=n
N n3:t.r Logic w=8 alen=1 dims=n
",
    );
}

/// An instance's output port driving a part of the parent's variable: a `port` row whose
/// chunk is on the parent's net, decided `[3:0]`; the child's own `assign` is an `assign` row.
#[test]
fn w03_port_hookup() {
    pin(
        "module c(input logic [3:0] x, output logic [3:0] y); assign y = ~x; endmodule
module t; logic [7:0] v; logic [3:0] s;
  c u(.x(s), .y(v[3:0]));
endmodule
",
        "\
WEXT writes=3 bits=3 word=0 undecided=0 poison=0
W C0:port cont - c0 n2:t.u.x bits[3:0]
W C1:port cont - c0 n0:t.v bits[3:0]
W C2:assign cont - c0 n3:t.u.y bits[3:0]
N n0:t.v Logic w=8 alen=1 dims=n
N n2:t.u.x Logic w=4 alen=1 dims=n
N n3:t.u.y Logic w=4 alen=1 dims=n
",
    );
}

/// A gate primitive's output (an `assign` row), a `wire` declaration initializer (a
/// `net_init` row) and a variable's non-constant declaration initializer (the synthesized
/// `var_init` process).
#[test]
fn w04_gate_net_init_var_init() {
    pin(
        "module t; logic a, b; logic [3:0] g; logic [3:0] s;
  and ga(g[1], a, b);
  wire [3:0] w = s;
  logic [3:0] d = s;
endmodule
",
        "\
WEXT writes=3 bits=3 word=0 undecided=0 poison=0
W C0:assign cont - c0 n2:t.g bits[1:1]
W C1:net_init cont - c0 n4:t.w bits[3:0]
W P0:var_init blk s0 c0 n5:t.d bits[3:0]
N n2:t.g Logic w=4 alen=1 dims=n
N n4:t.w Wire w=4 alen=1 dims=n
N n5:t.d Logic w=4 alen=1 dims=n
",
    );
}

/// K5: a static task called from `always_comb` is inlined, so its write on `v` is a
/// statement of the CALLER's process (`P..:always_comb`) ; a function writing `v` is a frame
/// body (`F0:func`), not attributed to the process that calls it.
#[test]
fn w05_task_inline_function_frame() {
    pin(
        "module t; logic [7:0] v, w; logic [3:0] s; logic d;
  task tk(input logic [3:0] x); v[3:0] = x; endtask
  function automatic logic fA(input logic [3:0] x); w[5:2] = x; return 1'b1; endfunction
  always_comb tk(s);
  always_comb d = fA(s);
endmodule
",
        "\
WEXT writes=5 bits=5 word=0 undecided=0 poison=0
W P0:always_comb blk s2 c0 n6:t.__taskarg_tk_x_6 bits[3:0]
W P0:always_comb blk s3 c0 n0:t.v bits[3:0]
W P1:always_comb blk s4 c0 n3:t.d bits[0:0]
W F0:func:.t.fA blk s0 c0 n1:t.w bits[5:2]
W F0:func:.t.fA blk s1 c0 n5:t.$func$fA.fA bits[0:0]
N n0:t.v Logic w=8 alen=1 dims=n
N n1:t.w Logic w=8 alen=1 dims=n
N n3:t.d Logic w=1 alen=1 dims=n
N n5:t.$func$fA.fA Reg w=1 alen=1 dims=n
N n6:t.__taskarg_tk_x_6 Logic w=4 alen=1 dims=n
",
    );
}

/// An unpacked-array element is `word` (not compared bit-wise) ; a run-time index and a
/// loop index are `undecided` ; a constant bit wholly out of range writes no bit and is
/// `undecided` too (matrix p6_whole_oor's shape ; the kernel answers `None` for both).
#[test]
fn w06_word_runtime_out_of_range() {
    pin(
        "module t; logic [7:0] m [0:3]; logic [7:0] v, u; logic [1:0] i; logic [7:0] s;
  always_comb m[1] = s;
  always_comb v[i] = s[0];
  always_comb for (int k = 1; k < 8; k++) u[k] = s[k];
  initial v[9] = 1'b1;
endmodule
",
        "\
WEXT writes=6 bits=2 word=1 undecided=3 poison=0
W P0:always_comb blk s0 c0 n0:t.m word
W P1:always_comb blk s1 c0 n1:t.v undecided
W P2:always_comb blk s2 c0 n5:t.__forvar_k_149 bits[31:0]
W P2:always_comb blk s3 c0 n2:t.u undecided
W P2:always_comb blk s4 c0 n5:t.__forvar_k_149 bits[31:0]
W P3:initial blk s5 c0 n1:t.v undecided
N n0:t.m Logic w=8 alen=4 dims=n
N n1:t.v Logic w=8 alen=1 dims=n
N n2:t.u Logic w=8 alen=1 dims=n
N n5:t.__forvar_k_149 Reg w=32 alen=1 dims=n
",
    );
}

/// A packed-array element and an array-of-struct member: `flatten_word`'s offset tree (a
/// stride product and a bounds guard) is not read by the kernel, so both are `undecided` in
/// this commit (the geometry constant is Q-WREXT C2's) ; a genvar bit-select is one
/// constant per iteration, decided ; a genvar EXPRESSION index stays a tree, `undecided`
/// (premise judge 1 N5: reach, not soundness).
#[test]
fn w07_packed_element_member_genvar() {
    pin(
        "typedef struct packed { logic [3:0] a; logic [3:0] b; } st_t;
module t; logic [1:0][7:0] p; st_t [1:0] q; logic [3:0] v, x; logic [7:0] s;
  always_comb p[1] = s;
  always_comb q[0].a = s[3:0];
  for (genvar g = 0; g < 2; g++) begin : G
    assign v[g] = s[g];
    assign x[2*g+1] = s[g];
  end
endmodule
",
        "\
WEXT writes=6 bits=2 word=0 undecided=4 poison=0
W C0:assign cont - c0 n2:t.v bits[0:0]
W C1:assign cont - c0 n3:t.x undecided
W C2:assign cont - c0 n2:t.v bits[1:1]
W C3:assign cont - c0 n3:t.x undecided
W P0:always_comb blk s0 c0 n0:t.p undecided
W P1:always_comb blk s1 c0 n1:t.q undecided
N n0:t.p Logic w=16 alen=1 dims=n
N n1:t.q Logic w=16 alen=1 dims=n
N n2:t.v Logic w=4 alen=1 dims=n
N n3:t.x Logic w=4 alen=1 dims=n
",
    );
}

/// A hierarchical write lands on the child's net once the end phase patched it (the chunk
/// is resolved before the view is built) ; a concatenation lvalue is one chunk per part,
/// `c0` and `c1` of one statement.
#[test]
fn w08_hier_write_and_concat() {
    pin(
        "module c; logic [7:0] v; endmodule
module t; c u(); logic [3:0] a, b; logic [7:0] s;
  initial begin
    u.v[3:0] = s[3:0];
    {a, b[1:0]} = s[5:0];
  end
endmodule
",
        "\
WEXT writes=3 bits=3 word=0 undecided=0 poison=0
W P0:initial blk s0 c0 n3:t.u.v bits[3:0]
W P0:initial blk s1 c0 n0:t.a bits[3:0]
W P0:initial blk s1 c1 n1:t.b bits[1:0]
N n0:t.a Logic w=4 alen=1 dims=n
N n1:t.b Logic w=4 alen=1 dims=n
N n3:t.u.v Logic w=8 alen=1 dims=n
",
    );
}

/// A frame body's blocks are walked from its entry through every terminator: the write in
/// the `else` arm of a function and the write after an automatic task's `#1` (a `Delay`
/// terminator's resume block) are frame writes too.
#[test]
fn w09_frame_branches_and_resume() {
    pin(
        "module t; logic [7:0] v, w; logic [3:0] s; logic d;
  function automatic logic fB(input logic [3:0] x);
    if (x[0]) v[1:0] = x[1:0]; else v[3:2] = x[3:2];
    return 1'b0;
  endfunction
  task automatic tD(input logic [3:0] x); #1 w[7:4] = x; endtask
  always_comb d = fB(s);
  initial tD(s);
endmodule
",
        "\
WEXT writes=5 bits=5 word=0 undecided=0 poison=0
W P0:always_comb blk s4 c0 n3:t.d bits[0:0]
W F0:func:.t.fB blk s0 c0 n0:t.v bits[1:0]
W F0:func:.t.fB blk s1 c0 n0:t.v bits[3:2]
W F0:func:.t.fB blk s2 c0 n5:t.$func$fB.fB bits[0:0]
W F1:task:.t.tD blk s3 c0 n1:t.w bits[7:4]
N n0:t.v Logic w=8 alen=1 dims=n
N n1:t.w Logic w=8 alen=1 dims=n
N n3:t.d Logic w=1 alen=1 dims=n
N n5:t.$func$fB.fB Reg w=1 alen=1 dims=n
",
    );
}

/// A frame that calls a frame (a `Call` terminator's return block holds the write after the
/// call) and a frame task that waits (a `Wait` terminator's resume block).
#[test]
fn w10_frame_call_and_wait() {
    pin(
        "module t; logic [7:0] v, w; logic [3:0] s; logic d, e;
  function automatic logic fI(input logic [3:0] x); return x[0]; endfunction
  function automatic logic fO(input logic [3:0] x);
    logic r;
    r = fI(x);
    v[0] = r;
    return r;
  endfunction
  task automatic tW(input logic [3:0] x); wait (e); w[3:0] = x; endtask
  always_comb d = fO(s);
  initial tW(s);
endmodule
",
        "\
WEXT writes=6 bits=6 word=0 undecided=0 poison=0
W P0:always_comb blk s5 c0 n3:t.d bits[0:0]
W F0:func:.t.fI blk s0 c0 n6:t.$func$fI.fI bits[0:0]
W F1:func:.t.fO blk s1 c0 n9:t.$func$fO.r bits[0:0]
W F1:func:.t.fO blk s2 c0 n0:t.v bits[0:0]
W F1:func:.t.fO blk s3 c0 n8:t.$func$fO.fO bits[0:0]
W F2:task:.t.tW blk s4 c0 n1:t.w bits[3:0]
N n0:t.v Logic w=8 alen=1 dims=n
N n1:t.w Logic w=8 alen=1 dims=n
N n3:t.d Logic w=1 alen=1 dims=n
N n6:t.$func$fI.fI Reg w=1 alen=1 dims=n
N n8:t.$func$fO.fO Reg w=1 alen=1 dims=n
N n9:t.$func$fO.r Logic w=1 alen=1 dims=n
",
    );
}

/// A frame task that enables another frame task: a `Call` terminator, whose return block
/// holds the write after the enable (a function call inside a frame is an expression and
/// needs no return block, so w10 does not reach this).
#[test]
fn w11_frame_task_enables_task() {
    pin(
        "module t; logic [7:0] w; logic [3:0] s; logic e;
  task automatic tB(input logic [3:0] x); wait (e); w[3:0] = x; endtask
  task automatic tA(input logic [3:0] x); tB(x); w[7:4] = x; endtask
  initial tA(s);
endmodule
",
        "\
WEXT writes=2 bits=2 word=0 undecided=0 poison=0
W F0:task:.t.tA blk s0 c0 n0:t.w bits[7:4]
W F1:task:.t.tB blk s1 c0 n0:t.w bits[3:0]
N n0:t.w Logic w=8 alen=1 dims=n
",
    );
}
