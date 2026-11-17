//! §5.2 Q-WREXT: the write-extent view (`elaborate/src/wext.rs`, `Sidecars::write_view`) as
//! the end phase builds it, one design per writer kind and per extent kind. Each pin is the
//! view's whole dump (`WriteView::render`): one `W` line per write site (writer, form,
//! statement, chunk, net, extent, `via`), one `N` line per net it names (with its
//! declaration fact `decl`, Q-WREXT C2).
//!
//! The view is a fact about vita's own finished IR, not a language value, so no oracle
//! decides these lines. What stands behind an extent: `Bits` is
//! `sim_ir::const_chunk_written_bits`'s answer, and that kernel is pinned against what the
//! engine's stores write by `sim-engine`'s `const_chunk_written_bits_is_what_the_stores_write`
//! (the anchor) ; a `geo[..]` extent is the geometry constant the lowering recorded,
//! anchored the same way by `geometry_constant_is_what_the_stores_write` ; the view adds no
//! arithmetic. Which process an inlined task's write belongs
//! to, and that a function writes in its frame, is how the IR lowers them (premise judge 1
//! K5, W/mdrv/premise/REPORT.txt), recorded as is.
//!
//! No user-visible output moves (the view has no production reader): the record lanes
//! compare every stdout, stderr and exit code with PRE (Q-WREXT build r1, B/run_record.sh).
//! The designs are elaborated without a span resolver, so no `@file:line:col` is printed.
//!
//! §5.2 Q-WEXTPOS C1, THE STAND-DOWN (`wext.rs`): the `wa_` pins. In an elaboration that
//! held a provisional constant (`$bits` of a hierarchical name) every select chunk is
//! `undecided`, a whole-net chunk and an array word keep their extent ; the controls hold
//! no provisional constant and dump as PRE. Each expected text is PRE's dump (wext-dump
//! c9cb441b built from main's crates, W/wextpos/ground2/cells/<id>/dump.out, its `@` and
//! `RESULT` dropped) with that rule applied. The five cells premise judge 1 named for the
//! text (k2m, k2n, k2o quiet and legal, k1k the reach-loss witness, k2e m4's killer) also
//! run the built `vita` and compare its whole stdout, stderr and exit code with PRE
//! 86fa310d's (W/wextpos/ground/cells/<id>/pre.out, pre.err): this row moves no text.

use diag::{LogEvent, LogSink};
use std::collections::BTreeMap;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

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

static NEXT: AtomicUsize = AtomicUsize::new(0);

/// PRE 86fa310d's stderr for every `pre_text` cell (no `timescale ; rc 0).
const PRE_ERR: &str = "warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; \
                       assuming the 1ns/1ns base\nerrors=0 warnings=1 notes=0\n";

/// The built `vita` on `src` (as `t.sv`, in a fresh directory) prints `out` on stdout,
/// `PRE_ERR` on stderr and exits 0: PRE's whole text.
fn pre_text(src: &str, out: &str) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_wext_view_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("temp dir");
    std::fs::write(d.join("t.sv"), src).expect("write t.sv");
    let r = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(String::from_utf8_lossy(&r.stdout), out, "stdout");
    assert_eq!(String::from_utf8_lossy(&r.stderr), PRE_ERR, "stderr");
    assert_eq!(r.status.code(), Some(0), "exit code");
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
WEXT writes=2 bits=2 geo=0 word=0 undecided=0 poison=0
W C0:assign cont - c0 n0:t.v bits[5:2] via=-
W P0:always_comb blk s0 c0 n0:t.v bits[3:0] via=direct
N n0:t.v Logic w=8 alen=1 dims=n decl=Module/Logic/Internal
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
WEXT writes=6 bits=6 geo=0 word=0 undecided=0 poison=0
W P0:always_ff nba s0 c0 n1:t.v bits[7:4] via=direct
W P1:initial blk s1 c0 n2:t.w bits[7:0] via=direct
W P1:initial force s2 c0 n3:t.r bits[7:0] via=direct
W P1:initial release s3 c0 n3:t.r bits[7:0] via=direct
W P1:initial force s4 c0 n1:t.v bits[7:0] via=direct
W P1:initial release s5 c0 n1:t.v bits[7:0] via=direct
N n1:t.v Logic w=8 alen=1 dims=n decl=Module/Logic/Internal
N n2:t.w Logic w=8 alen=1 dims=n decl=Module/Logic/Internal
N n3:t.r Logic w=8 alen=1 dims=n decl=Module/Logic/Internal
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
WEXT writes=3 bits=3 geo=0 word=0 undecided=0 poison=0
W C0:port cont - c0 n2:t.u.x bits[3:0] via=-
W C1:port cont - c0 n0:t.v bits[3:0] via=-
W C2:assign cont - c0 n3:t.u.y bits[3:0] via=-
N n0:t.v Logic w=8 alen=1 dims=n decl=Module/Logic/Internal
N n2:t.u.x Logic w=4 alen=1 dims=n decl=AnsiPort/Logic/Input
N n3:t.u.y Logic w=4 alen=1 dims=n decl=AnsiPort/Logic/Output
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
WEXT writes=3 bits=3 geo=0 word=0 undecided=0 poison=0
W C0:assign cont - c0 n2:t.g bits[1:1] via=-
W C1:net_init cont - c0 n4:t.w bits[3:0] via=-
W P0:var_init blk s0 c0 n5:t.d bits[3:0] via=direct
N n2:t.g Logic w=4 alen=1 dims=n decl=Module/Logic/Internal
N n4:t.w Wire w=4 alen=1 dims=n decl=Module/Wire/Internal
N n5:t.d Logic w=4 alen=1 dims=n decl=Module/Logic/Internal
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
WEXT writes=5 bits=5 geo=0 word=0 undecided=0 poison=0
W P0:always_comb blk s2 c0 n6:t.__taskarg_tk_x_6 bits[3:0] via=task
W P0:always_comb blk s3 c0 n0:t.v bits[3:0] via=task
W P1:always_comb blk s4 c0 n3:t.d bits[0:0] via=direct
W F0:func:.t.fA blk s0 c0 n1:t.w bits[5:2] via=direct
W F0:func:.t.fA blk s1 c0 n5:t.$func$fA.fA bits[0:0] via=direct
N n0:t.v Logic w=8 alen=1 dims=n decl=Module/Logic/Internal
N n1:t.w Logic w=8 alen=1 dims=n decl=Module/Logic/Internal
N n3:t.d Logic w=1 alen=1 dims=n decl=Module/Logic/Internal
N n5:t.$func$fA.fA Reg w=1 alen=1 dims=n decl=-
N n6:t.__taskarg_tk_x_6 Logic w=4 alen=1 dims=n decl=-
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
WEXT writes=6 bits=2 geo=0 word=1 undecided=3 poison=0
W P0:always_comb blk s0 c0 n0:t.m word via=direct
W P1:always_comb blk s1 c0 n1:t.v undecided via=direct
W P2:always_comb blk s2 c0 n5:t.__forvar_k_149 bits[31:0] via=direct
W P2:always_comb blk s3 c0 n2:t.u undecided via=direct
W P2:always_comb blk s4 c0 n5:t.__forvar_k_149 bits[31:0] via=direct
W P3:initial blk s5 c0 n1:t.v undecided via=direct
N n0:t.m Logic w=8 alen=4 dims=n decl=Module/Logic/Internal
N n1:t.v Logic w=8 alen=1 dims=n decl=Module/Logic/Internal
N n2:t.u Logic w=8 alen=1 dims=n decl=Module/Logic/Internal
N n5:t.__forvar_k_149 Reg w=32 alen=1 dims=n decl=-
",
    );
}

/// A packed-array element and an array-of-struct member: `flatten_word`'s offset tree (a
/// stride product and a bounds guard) is not read by the kernel ; its value with every
/// index a constant leaf is the geometry constant the lowering records (Q-WREXT C2,
/// `geo[..]`, pinned against both stores by sim-engine's
/// `geometry_constant_is_what_the_stores_write`) ; a genvar bit-select is one constant per
/// iteration, decided by the kernel ; a genvar EXPRESSION index stays a tree, `undecided`
/// (premise judge 1 N5: reach, not soundness ; the geometry home reads leaves only).
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
WEXT writes=6 bits=2 geo=2 word=0 undecided=2 poison=0
W C0:assign cont - c0 n2:t.v bits[0:0] via=-
W C1:assign cont - c0 n3:t.x undecided via=-
W C2:assign cont - c0 n2:t.v bits[1:1] via=-
W C3:assign cont - c0 n3:t.x undecided via=-
W P0:always_comb blk s0 c0 n0:t.p geo[15:8] via=direct
W P1:always_comb blk s1 c0 n1:t.q geo[7:4] via=direct
N n0:t.p Logic w=16 alen=1 dims=n decl=Module/Logic/Internal
N n1:t.q Logic w=16 alen=1 dims=n decl=Module/Logic/Internal
N n2:t.v Logic w=4 alen=1 dims=n decl=Module/Logic/Internal
N n3:t.x Logic w=4 alen=1 dims=n decl=Module/Logic/Internal
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
WEXT writes=3 bits=3 geo=0 word=0 undecided=0 poison=0
W P0:initial blk s0 c0 n3:t.u.v bits[3:0] via=direct
W P0:initial blk s1 c0 n0:t.a bits[3:0] via=direct
W P0:initial blk s1 c1 n1:t.b bits[1:0] via=direct
N n0:t.a Logic w=4 alen=1 dims=n decl=Module/Logic/Internal
N n1:t.b Logic w=4 alen=1 dims=n decl=Module/Logic/Internal
N n3:t.u.v Logic w=8 alen=1 dims=n decl=Module/Logic/Internal
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
WEXT writes=5 bits=5 geo=0 word=0 undecided=0 poison=0
W P0:always_comb blk s4 c0 n3:t.d bits[0:0] via=direct
W F0:func:.t.fB blk s0 c0 n0:t.v bits[1:0] via=direct
W F0:func:.t.fB blk s1 c0 n0:t.v bits[3:2] via=direct
W F0:func:.t.fB blk s2 c0 n5:t.$func$fB.fB bits[0:0] via=direct
W F1:task:.t.tD blk s3 c0 n1:t.w bits[7:4] via=direct
N n0:t.v Logic w=8 alen=1 dims=n decl=Module/Logic/Internal
N n1:t.w Logic w=8 alen=1 dims=n decl=Module/Logic/Internal
N n3:t.d Logic w=1 alen=1 dims=n decl=Module/Logic/Internal
N n5:t.$func$fB.fB Reg w=1 alen=1 dims=n decl=-
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
WEXT writes=6 bits=6 geo=0 word=0 undecided=0 poison=0
W P0:always_comb blk s5 c0 n3:t.d bits[0:0] via=direct
W F0:func:.t.fI blk s0 c0 n6:t.$func$fI.fI bits[0:0] via=direct
W F1:func:.t.fO blk s1 c0 n9:t.$func$fO.r bits[0:0] via=direct
W F1:func:.t.fO blk s2 c0 n0:t.v bits[0:0] via=direct
W F1:func:.t.fO blk s3 c0 n8:t.$func$fO.fO bits[0:0] via=direct
W F2:task:.t.tW blk s4 c0 n1:t.w bits[3:0] via=direct
N n0:t.v Logic w=8 alen=1 dims=n decl=Module/Logic/Internal
N n1:t.w Logic w=8 alen=1 dims=n decl=Module/Logic/Internal
N n3:t.d Logic w=1 alen=1 dims=n decl=Module/Logic/Internal
N n6:t.$func$fI.fI Reg w=1 alen=1 dims=n decl=-
N n8:t.$func$fO.fO Reg w=1 alen=1 dims=n decl=-
N n9:t.$func$fO.r Logic w=1 alen=1 dims=n decl=-
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
WEXT writes=2 bits=2 geo=0 word=0 undecided=0 poison=0
W F0:task:.t.tA blk s0 c0 n0:t.w bits[7:4] via=direct
W F1:task:.t.tB blk s1 c0 n0:t.w bits[3:0] via=direct
N n0:t.w Logic w=8 alen=1 dims=n decl=Module/Logic/Internal
",
    );
}

/// Q-WREXT C2 geometry: a constant element of each packed shape the lowering composes — a
/// descending and an ascending outer dimension, a non-zero low bound, three dimensions, a
/// part-select of the leaf under an element, an element range of an inner dimension, an
/// array-of-struct member ; an index out of bounds and a negative literal (a unary minus,
/// not a leaf) stay `undecided`, and so does a variable index.
#[test]
fn w12_geometry_shapes() {
    pin(
        "typedef struct packed { logic [3:0] a; logic [2:0] b; logic c; } s_t;
module t; logic [7:0] s; logic [1:0] i;
  logic [3:0][7:0] a; logic [0:3][7:0] b; logic [5:2][7:0] c; logic [1:0][2:0][3:0] d;
  logic [3:0][7:0] e; logic [1:0][2:0][3:0] f; s_t [2:0] g; logic [3:0][7:0] h, k, m;
  assign a[1] = s;
  assign b[1] = s;
  assign c[3] = s;
  assign d[1][2] = s[3:0];
  assign e[2][5:2] = s[3:0];
  assign f[1][1:0] = s;
  assign g[2].b = s[2:0];
  assign h[4] = s;
  assign k[-1] = s;
  always_comb m[i] = s;
endmodule
",
        "\
WEXT writes=10 bits=0 geo=7 word=0 undecided=3 poison=0
W C0:assign cont - c0 n2:t.a geo[15:8] via=-
W C1:assign cont - c0 n3:t.b geo[23:16] via=-
W C2:assign cont - c0 n4:t.c geo[15:8] via=-
W C3:assign cont - c0 n5:t.d geo[23:20] via=-
W C4:assign cont - c0 n6:t.e geo[21:18] via=-
W C5:assign cont - c0 n7:t.f geo[19:12] via=-
W C6:assign cont - c0 n8:t.g geo[19:17] via=-
W C7:assign cont - c0 n9:t.h undecided via=-
W C8:assign cont - c0 n10:t.k undecided via=-
W P0:always_comb blk s0 c0 n11:t.m undecided via=direct
N n2:t.a Logic w=32 alen=1 dims=n decl=Module/Logic/Internal
N n3:t.b Logic w=32 alen=1 dims=n decl=Module/Logic/Internal
N n4:t.c Logic w=32 alen=1 dims=n decl=Module/Logic/Internal
N n5:t.d Logic w=24 alen=1 dims=n decl=Module/Logic/Internal
N n6:t.e Logic w=32 alen=1 dims=n decl=Module/Logic/Internal
N n7:t.f Logic w=24 alen=1 dims=n decl=Module/Logic/Internal
N n8:t.g Logic w=24 alen=1 dims=n decl=Module/Logic/Internal
N n9:t.h Logic w=32 alen=1 dims=n decl=Module/Logic/Internal
N n10:t.k Logic w=32 alen=1 dims=n decl=Module/Logic/Internal
N n11:t.m Logic w=32 alen=1 dims=n decl=Module/Logic/Internal
",
    );
}

/// Q-WREXT C2 declaration facts: a module variable, a net, a generate-scope variable, ANSI
/// ports (input, output variable, output net, inout), non-ANSI ports (an output completed
/// by `reg`, an input completed by a body `logic`: site Module, direction Input), a
/// type-parameter variable, and `-` for what no declaration site writes: a block local, a
/// loop variable, task formals and locals, function frame nets.
#[test]
fn w13_declaration_facts() {
    pin(
        "module u(input logic [3:0] i, output logic [3:0] o, output [3:0] w, inout [3:0] io);
  assign o = i; assign w = i;
endmodule
module n(a, b); input [3:0] a; logic [3:0] a; output reg [3:0] b; always @* b = a; endmodule
module p #(parameter type T = logic [3:0]) (input logic [3:0] x);
  T v; always_comb v = x;
endmodule
module t; logic [3:0] a, q, r, y; wire [3:0] w, io; int k;
  u uu(.i(a), .o(q), .w(w), .io(io));
  n nn(.a(a), .b(r));
  p pp(.x(a));
  for (genvar g = 0; g < 2; g++) begin : G logic [1:0] gv; always_comb gv = a[1:0]; end
  task tk(input logic [3:0] x); logic [3:0] tl; tl = x; endtask
  function automatic logic [3:0] f(input logic [3:0] x); return x; endfunction
  always_comb begin logic [3:0] bl; bl = a; end
  initial begin for (int j = 0; j < 2; j++) k = j; tk(a); y = f(a); end
endmodule
",
        "\
WEXT writes=21 bits=21 geo=0 word=0 undecided=0 poison=0
W C0:port cont - c0 n15:t.uu.i bits[3:0] via=-
W C1:port cont - c0 n1:t.q bits[3:0] via=-
W C2:port cont - c0 n4:t.w bits[3:0] via=-
W C3:port cont - c0 n18:t.uu.io bits[3:0] via=-
W C4:assign cont - c0 n16:t.uu.o bits[3:0] via=-
W C5:assign cont - c0 n17:t.uu.w bits[3:0] via=-
W C6:port cont - c0 n19:t.nn.a bits[3:0] via=-
W C7:port cont - c0 n2:t.r bits[3:0] via=-
W C8:port cont - c0 n21:t.pp.x bits[3:0] via=-
W P0:always_comb blk s1 c0 n9:t.G[0].gv bits[1:0] via=direct
W P1:always_comb blk s2 c0 n10:t.G[1].gv bits[1:0] via=direct
W P2:always_comb blk s3 c0 n7:t.bl bits[3:0] via=direct
W P3:initial blk s4 c0 n8:t.__forvar_j_754 bits[31:0] via=direct
W P3:initial blk s5 c0 n6:t.k bits[31:0] via=direct
W P3:initial blk s6 c0 n8:t.__forvar_j_754 bits[31:0] via=direct
W P3:initial blk s7 c0 n13:t.__taskarg_tk_x_13 bits[3:0] via=task
W P3:initial blk s8 c0 n14:t.$itask$tk$L.tl bits[3:0] via=task
W P3:initial blk s9 c0 n3:t.y bits[3:0] via=direct
W P4:always blk s10 c0 n20:t.nn.b bits[3:0] via=direct
W P6:always_comb blk s12 c0 n22:t.pp.v bits[3:0] via=direct
W F0:func:.t.f blk s0 c0 n12:t.$func$f.f bits[3:0] via=direct
N n1:t.q Logic w=4 alen=1 dims=n decl=Module/Logic/Internal
N n2:t.r Logic w=4 alen=1 dims=n decl=Module/Logic/Internal
N n3:t.y Logic w=4 alen=1 dims=n decl=Module/Logic/Internal
N n4:t.w Wire w=4 alen=1 dims=n decl=Module/Wire/Internal
N n6:t.k Reg w=32 alen=1 dims=n decl=Module/Int/Internal
N n7:t.bl Logic w=4 alen=1 dims=n decl=-
N n8:t.__forvar_j_754 Reg w=32 alen=1 dims=n decl=-
N n9:t.G[0].gv Logic w=2 alen=1 dims=n decl=Generate/Logic/Internal
N n10:t.G[1].gv Logic w=2 alen=1 dims=n decl=Generate/Logic/Internal
N n12:t.$func$f.f Reg w=4 alen=1 dims=n decl=-
N n13:t.__taskarg_tk_x_13 Logic w=4 alen=1 dims=n decl=-
N n14:t.$itask$tk$L.tl Logic w=4 alen=1 dims=n decl=-
N n15:t.uu.i Logic w=4 alen=1 dims=n decl=AnsiPort/Logic/Input
N n16:t.uu.o Logic w=4 alen=1 dims=n decl=AnsiPort/Logic/Output
N n17:t.uu.w Wire w=4 alen=1 dims=n decl=AnsiPort/Wire/Output
N n18:t.uu.io Wire w=4 alen=1 dims=n decl=AnsiPort/Wire/Inout
N n19:t.nn.a Logic w=4 alen=1 dims=n decl=Module/Logic/Input
N n20:t.nn.b Reg w=4 alen=1 dims=n decl=NonAnsiPort/Reg/Output
N n21:t.pp.x Logic w=4 alen=1 dims=n decl=AnsiPort/Logic/Input
N n22:t.pp.v Logic w=4 alen=1 dims=n decl=Module/Logic/Internal/tparam
",
    );
}

/// Q-WREXT C2 `via`: a write in an inlined static task's statements is `task` (nested
/// tasks, the copy-in of each formal, a task local's initializer) ; a write in a deferred
/// assertion's action is `defer`, also when the action calls a task ; the caller's own
/// statements around them are `direct` ; a void function called as a statement is inlined
/// as a task ; a hierarchical task enable is its frame body (`direct`) ; a continuous
/// writer has no `via` (`-`).
#[test]
fn w14_via_intervals() {
    pin(
        "module c; logic [3:0] cv; task ht(input logic [3:0] x); cv = x; endtask endmodule
module t; c u(); logic [3:0] v, w, a, z; int n;
  task inner(input logic [3:0] x); w = x; endtask
  task outer(input logic [3:0] x); int cnt = 0; inner(x); v = x; n = cnt; endtask
  function void setz(input logic [3:0] x); z = x; endfunction
  assign a = 4'h0;
  initial begin
    v = 4'h1;
    outer(4'h7);
    w = 4'h2;
    assert final (a != 0) else outer(4'h3);
    assert final (a != 0) else z = 4'h4;
    setz(4'h5);
    u.ht(4'h6);
    n = 1;
  end
endmodule
",
        "\
WEXT writes=19 bits=19 geo=0 word=0 undecided=0 poison=0
W C0:assign cont - c0 n2:t.a bits[3:0] via=-
W P0:initial blk s0 c0 n0:t.v bits[3:0] via=direct
W P0:initial blk s1 c0 n5:t.__taskarg_outer_x_5 bits[3:0] via=task
W P0:initial blk s2 c0 n6:t.$itask$outer$L.cnt bits[31:0] via=task
W P0:initial blk s3 c0 n7:t.$itask$outer$L.__taskarg_inner_x_7 bits[3:0] via=task
W P0:initial blk s4 c0 n1:t.w bits[3:0] via=task
W P0:initial blk s5 c0 n0:t.v bits[3:0] via=task
W P0:initial blk s6 c0 n4:t.n bits[31:0] via=task
W P0:initial blk s7 c0 n1:t.w bits[3:0] via=direct
W P0:initial blk s9 c0 n5:t.__taskarg_outer_x_5 bits[3:0] via=defer
W P0:initial blk s10 c0 n7:t.$itask$outer$L.__taskarg_inner_x_7 bits[3:0] via=defer
W P0:initial blk s11 c0 n1:t.w bits[3:0] via=defer
W P0:initial blk s12 c0 n0:t.v bits[3:0] via=defer
W P0:initial blk s13 c0 n4:t.n bits[31:0] via=defer
W P0:initial blk s15 c0 n3:t.z bits[3:0] via=defer
W P0:initial blk s16 c0 n8:t.__taskarg_setz_x_8 bits[3:0] via=task
W P0:initial blk s17 c0 n3:t.z bits[3:0] via=task
W P0:initial blk s18 c0 n4:t.n bits[31:0] via=direct
W F0:task:.t.u.ht blk s19 c0 n9:t.u.cv bits[3:0] via=direct
N n0:t.v Logic w=4 alen=1 dims=n decl=Module/Logic/Internal
N n1:t.w Logic w=4 alen=1 dims=n decl=Module/Logic/Internal
N n2:t.a Logic w=4 alen=1 dims=n decl=Module/Logic/Internal
N n3:t.z Logic w=4 alen=1 dims=n decl=Module/Logic/Internal
N n4:t.n Reg w=32 alen=1 dims=n decl=Module/Int/Internal
N n5:t.__taskarg_outer_x_5 Logic w=4 alen=1 dims=n decl=-
N n6:t.$itask$outer$L.cnt Reg w=32 alen=1 dims=n decl=-
N n7:t.$itask$outer$L.__taskarg_inner_x_7 Logic w=4 alen=1 dims=n decl=-
N n8:t.__taskarg_setz_x_8 Logic w=4 alen=1 dims=n decl=-
N n9:t.u.cv Logic w=4 alen=1 dims=n decl=Module/Logic/Internal
",
    );
}

/// Q-WREXT C2 geometry through the hierarchical funnel (`flatten_word_eids`, the eid twin
/// of `flatten_word`): a constant element of a child's multi-dimensional packed net written
/// by the parent, procedurally and continuously. The ranges are the bits PRE's simulation
/// sets from x (`u.p[1][2]` 20..23, `u.q[2]` 16..23, `u.q[1]` 8..15, W/wrext/build NOTES).
#[test]
fn w15_geometry_hierarchical() {
    pin(
        "module c; logic [1:0][2:0][3:0] p; logic [3:0][7:0] q; endmodule
module t; c u();
  initial begin u.p[1][2] = 4'h3; u.q[2] = 8'h1; end
  assign u.q[1] = 8'h2;
endmodule
",
        "\
WEXT writes=3 bits=0 geo=3 word=0 undecided=0 poison=0
W C0:assign cont - c0 n1:t.u.q geo[15:8] via=-
W P0:initial blk s0 c0 n0:t.u.p geo[23:20] via=direct
W P0:initial blk s1 c0 n1:t.u.q geo[23:16] via=direct
N n0:t.u.p Logic w=24 alen=1 dims=n decl=Module/Logic/Internal
N n1:t.u.q Logic w=32 alen=1 dims=n decl=Module/Logic/Internal
",
    );
}

/// Q-WREXT C2 `via` at the deferred action's interval start: a package function first
/// called in a deferred assertion's CONDITION has its frame body lowered right there
/// (`inline_pkg_function` -> `frame_scoped_pkg_routines`), so its last statement — the
/// return-slot write `s3` — is the statement pushed immediately before the action's
/// interval (`s4`). It stays `direct` (a frame body), the action's write `defer`.
#[test]
fn w16_defer_interval_after_condition_frame() {
    pin(
        "package p;
  function automatic logic [3:0] pf(input logic [3:0] x); logic [3:0] y; y = x + 1; return y; endfunction
endpackage
module t; logic [3:0] a, v;
  initial begin
    a = 0;
    assert final (p::pf(a) != 0) else v = 4'h1;
  end
endmodule
",
        "\
WEXT writes=4 bits=4 geo=0 word=0 undecided=0 poison=0
W P0:initial blk s0 c0 n0:t.a bits[3:0] via=direct
W P0:initial blk s4 c0 n1:t.v bits[3:0] via=defer
W F0:func:.t.pf blk s2 c0 n4:t.$func$p::pf.y bits[3:0] via=direct
W F0:func:.t.pf blk s3 c0 n3:t.$func$p::pf.pf bits[3:0] via=direct
N n0:t.a Logic w=4 alen=1 dims=n decl=Module/Logic/Internal
N n1:t.v Logic w=4 alen=1 dims=n decl=Module/Logic/Internal
N n3:t.$func$p::pf.pf Reg w=4 alen=1 dims=n decl=-
N n4:t.$func$p::pf.y Logic w=4 alen=1 dims=n decl=-
",
    );
}

/// R6: fix S's own pair. `x[$bits(u.w)]` (u.w is 4 bits) on PRE dumps the stale geometry
/// `geo[65:64]` (the provisional 32's element ; the bits the engine writes are `[9:8]`,
/// PRE stdout `x4=11 x32=01`) and the literal `x[32]` `geo[65:64]` ; under the flag both
/// selects are `undecided`.
/// PRE dump (wext-dump c9cb441b, W/wextpos/ground2/cells/a1_bits_cont): `C0 .. geo[65:64]`,
/// `P0 .. geo[65:64]`.
#[test]
fn wa_a1_stand_down_pair() {
    let src = r#"module child; logic [3:0] w; endmodule
module top;
  logic [63:0][1:0] x;
  assign x[$bits(u.w)] = 2'b11;
  child u();
  initial begin
    x[32] = 2'b01;
    #1 $display("x4=%b x32=%b", x[4], x[32]);
    $finish;
  end
endmodule
"#;
    pin(
        src,
        "\
WEXT writes=2 bits=0 geo=0 word=0 undecided=2 poison=0
W C0:assign cont - c0 n0:top.x undecided via=-
W P0:initial blk s0 c0 n0:top.x undecided via=direct
N n0:top.x Logic w=128 alen=1 dims=n decl=Module/Logic/Internal
",
    );
}

/// R6: the overlap PRE's view hides. `x[$bits(u.w)]` and `x[4]` write the same element
/// (PRE stdout `x4=01 x32=xx`) ; PRE dumps `C0 .. geo[65:64]` and `P0 .. geo[9:8]`, two
/// disjoint extents ; under the flag both are `undecided`, never a false disjoint pair.
#[test]
fn wa_a4_stand_down_overlap() {
    let src = r#"module child; logic [3:0] w; endmodule
module top;
  logic [63:0][1:0] x;
  assign x[$bits(u.w)] = 2'b11;
  child u();
  initial begin
    x[4] = 2'b01;
    #1 $display("x4=%b x32=%b", x[4], x[32]);
    $finish;
  end
endmodule
"#;
    pin(
        src,
        "\
WEXT writes=2 bits=0 geo=0 word=0 undecided=2 poison=0
W C0:assign cont - c0 n0:top.x undecided via=-
W P0:initial blk s0 c0 n0:top.x undecided via=direct
N n0:top.x Logic w=128 alen=1 dims=n decl=Module/Logic/Internal
",
    );
}

/// The kernel path, low bound 1: `y[$bits(u.w)]` on `[40:1]` folds the provisional 32 in
/// `norm_sub_k` ; PRE dumps `bits[31:31]` for it and for `y[32]` ; under the flag both are
/// `undecided`.
#[test]
fn wa_w01_stand_down_kernel() {
    let src = r#"module child; logic [3:0] w; endmodule
module top;
  logic [40:1] y;
  assign y[$bits(u.w)] = 1'b1;
  child u();
  initial begin y[32] = 1'b0; #1 $display("y4=%b y32=%b", y[4], y[32]); $finish; end
endmodule
"#;
    pin(
        src,
        "\
WEXT writes=2 bits=0 geo=0 word=0 undecided=2 poison=0
W C0:assign cont - c0 n0:top.y undecided via=-
W P0:initial blk s0 c0 n0:top.y undecided via=direct
N n0:top.y Logic w=40 alen=1 dims=n decl=Module/Logic/Internal
",
    );
}

/// The geometry path, two dimensions: PRE dumps `geo[193:192]` for `x[1][$bits(u.w)]` and
/// `x[1][32]` ; under the flag both are `undecided`.
#[test]
fn wa_w06_stand_down_geometry() {
    let src = r#"module child; logic [3:0] w; endmodule
module top;
  logic [3:0][63:0][1:0] x;
  assign x[1][$bits(u.w)] = 2'b11;
  child u();
  initial begin x[1][32] = 2'b01; #1 $display("a=%b b=%b", x[1][4], x[1][32]); $finish; end
endmodule
"#;
    pin(
        src,
        "\
WEXT writes=2 bits=0 geo=0 word=0 undecided=2 poison=0
W C0:assign cont - c0 n0:top.x undecided via=-
W P0:initial blk s0 c0 n0:top.x undecided via=direct
N n0:top.x Logic w=512 alen=1 dims=n decl=Module/Logic/Internal
",
    );
}

/// Premise judge 1 line 3 (quiet legal): `y[$bits(u.w)-1:0]` stays a SELECT chunk (PRE
/// dumps `bits[3:0]` on the 32-bit `y`, not `bits[31:0]`), so it stands down to
/// `undecided`, as `y[31:4]` does. vita's text is PRE's (`y=1234567f` ; iverilog and
/// verilator printed `y=1234567f`, old output, not re-run).
#[test]
fn wa_k2m_quiet_legal_comb() {
    let src = r#"module child; logic [3:0] w; endmodule
module top;
  logic [31:0] y;
  always_comb y[$bits(u.w)-1:0] = 4'hf;
  always_comb y[31:4] = 28'h1234567;
  child u();
  initial begin #1 $display("y=%h", y); $finish; end
endmodule
"#;
    pin(
        src,
        "\
WEXT writes=2 bits=0 geo=0 word=0 undecided=2 poison=0
W P0:always_comb blk s0 c0 n0:top.y undecided via=direct
W P1:always_comb blk s1 c0 n0:top.y undecided via=direct
N n0:top.y Logic w=32 alen=1 dims=n decl=Module/Logic/Internal
",
    );
    pre_text(src, "y=1234567f\nsimulation ended (Finish) at time 1\n");
}

/// Premise judge 1 line 3, the continuous twin of k2m: both selects `undecided` ; vita's
/// text is PRE's (`y=1234567f` ; iverilog and verilator `y=1234567f`, old output).
#[test]
fn wa_k2n_quiet_legal_assign() {
    let src = r#"module child; logic [3:0] w; endmodule
module top;
  logic [31:0] y;
  assign y[$bits(u.w)-1:0] = 4'hf;
  always_comb y[31:4] = 28'h1234567;
  child u();
  initial begin #1 $display("y=%h", y); $finish; end
endmodule
"#;
    pin(
        src,
        "\
WEXT writes=2 bits=0 geo=0 word=0 undecided=2 poison=0
W C0:assign cont - c0 n0:top.y undecided via=-
W P0:always_comb blk s0 c0 n0:top.y undecided via=direct
N n0:top.y Logic w=32 alen=1 dims=n decl=Module/Logic/Internal
",
    );
    pre_text(src, "y=1234567f\nsimulation ended (Finish) at time 1\n");
}

/// Premise judge 1 line 3, the port twin: the port row's select on `y` and `y[31:4]` are
/// `undecided` ; the child's own whole-net `assign o` keeps `bits[3:0]` ; vita's text is
/// PRE's (`y=1234567f` ; iverilog and verilator `y=1234567f`, old output).
#[test]
fn wa_k2o_quiet_legal_port() {
    let src = r#"module child; logic [3:0] w; endmodule
module drv(output logic [3:0] o); assign o = 4'hf; endmodule
module top;
  logic [31:0] y;
  drv d(.o(y[$bits(u.w)-1:0]));
  always_comb y[31:4] = 28'h1234567;
  child u();
  initial begin #1 $display("y=%h", y); $finish; end
endmodule
"#;
    pin(
        src,
        "\
WEXT writes=3 bits=1 geo=0 word=0 undecided=2 poison=0
W C0:port cont - c0 n0:top.y undecided via=-
W C1:assign cont - c0 n1:top.d.o bits[3:0] via=-
W P0:always_comb blk s0 c0 n0:top.y undecided via=direct
N n0:top.y Logic w=32 alen=1 dims=n decl=Module/Logic/Internal
N n1:top.d.o Logic w=4 alen=1 dims=n decl=AnsiPort/Logic/Output
",
    );
    pre_text(src, "y=1234567f\nsimulation ended (Finish) at time 1\n");
}

/// Premise judge 1 line 4, the reach-loss witness: `$bits(u.w)` only in a `$display` raises
/// the flag for the whole elaboration, so the two real element writes PRE dumps as
/// `geo[3:2]` and `geo[2:2]` are `undecided` (reach lost, no wrong extent) ; vita's text
/// is PRE's (`b=4 x=xxxx10xx`).
#[test]
fn wa_k1k_reach_loss() {
    let src = r#"module child; logic [3:0] w; endmodule
module top;
  logic [3:0][1:0] x;
  assign x[1] = 2'b11;
  child u();
  initial begin x[1][0] = 1'b0; #1 $display("b=%0d x=%b", $bits(u.w), x); $finish; end
endmodule
"#;
    pin(
        src,
        "\
WEXT writes=2 bits=0 geo=0 word=0 undecided=2 poison=0
W C0:assign cont - c0 n0:top.x undecided via=-
W P0:initial blk s0 c0 n0:top.x undecided via=direct
N n0:top.x Logic w=8 alen=1 dims=n decl=Module/Logic/Internal
",
    );
    pre_text(src, "b=4 x=xxxx10xx\nsimulation ended (Finish) at time 1\n");
}

/// Premise judge 1 line 4, m4's behaviour killer: under the flag (`$bits(u.w)` in a
/// `$display`) a whole-net chunk reads no expression and keeps the kernel's `bits[3:0]`,
/// for the port row and the `always_comb` writer of `v` alike (PRE's dump, unchanged) ;
/// vita's text is PRE's (`b=4 v=0`).
#[test]
fn wa_k2e_whole_net_stays() {
    let src = r#"module child(output logic [3:0] o); logic [3:0] w; assign o = 4'h5; endmodule
module top;
  logic [3:0] v;
  child u(.o(v));
  always_comb v = 4'h0;
  initial begin #1 $display("b=%0d v=%h", $bits(u.w), v); $finish; end
endmodule
"#;
    pin(
        src,
        "\
WEXT writes=3 bits=3 geo=0 word=0 undecided=0 poison=0
W C0:port cont - c0 n0:top.v bits[3:0] via=-
W C1:assign cont - c0 n1:top.u.o bits[3:0] via=-
W P0:always_comb blk s0 c0 n0:top.v bits[3:0] via=direct
N n0:top.v Logic w=4 alen=1 dims=n decl=Module/Logic/Internal
N n1:top.u.o Logic w=4 alen=1 dims=n decl=AnsiPort/Logic/Output
",
    );
    pre_text(src, "b=4 v=0\nsimulation ended (Finish) at time 1\n");
}

/// An unpacked-array word stays `word` under the flag (the stand-down comes after `Word`),
/// while the select `y[3:0]` in the same design is `undecided`. PRE dump (wext-dump
/// c9cb441b, W/wextpos/build/cells/wa_word): `P0 .. m word`, `P1 .. y bits[3:0]` ; PRE
/// stdout `b=4 m1=5a y=xxxx0001`.
#[test]
fn wa_word_stays_word() {
    let src = r#"module child; logic [3:0] w; endmodule
module top;
  logic [7:0] m [0:3];
  logic [7:0] y;
  always_comb m[1] = 8'h5a;
  always_comb y[3:0] = 4'h1;
  child u();
  initial begin #1 $display("b=%0d m1=%h y=%b", $bits(u.w), m[1], y); $finish; end
endmodule
"#;
    pin(
        src,
        "\
WEXT writes=2 bits=0 geo=0 word=1 undecided=1 poison=0
W P0:always_comb blk s0 c0 n0:top.m word via=direct
W P1:always_comb blk s1 c0 n1:top.y undecided via=direct
N n0:top.m Logic w=8 alen=4 dims=n decl=Module/Logic/Internal
N n1:top.y Logic w=8 alen=1 dims=n decl=Module/Logic/Internal
",
    );
}

/// Control, flag CLEAR (R1): `x[u.P]` is a hierarchical PARAMETER, a `Signal` placeholder
/// the end phase overwrites with a `Const` (no `Const` overwritten, nothing registered) ;
/// on the two-dimensional net PRE already dumps it `undecided` (no geometry for a
/// `Signal` leaf at lowering) and the literal `x[32]` `geo[65:64]` — both as PRE.
#[test]
fn wa_w10_control_two_dims() {
    let src = r#"module child; parameter int P = 4; endmodule
module top;
  logic [63:0][1:0] x;
  assign x[u.P] = 2'b11;
  child u();
  initial begin x[32] = 2'b01; #1 $display("x4=%b x32=%b", x[4], x[32]); $finish; end
endmodule
"#;
    pin(
        src,
        "\
WEXT writes=2 bits=0 geo=1 word=0 undecided=1 poison=0
W C0:assign cont - c0 n0:top.x undecided via=-
W P0:initial blk s0 c0 n0:top.x geo[65:64] via=direct
N n0:top.x Logic w=128 alen=1 dims=n decl=Module/Logic/Internal
",
    );
}

/// Control, flag CLEAR (R1): a one-dimensional `y[u.P]` is decided by the kernel from the
/// patched parameter `Const` (`bits[4:4]`, P = 4), beside `y[4]` — as PRE.
#[test]
fn wa_k8a_control_decided() {
    let src = r#"module child; parameter int P = 4; endmodule
module top;
  logic [63:0] y;
  assign y[u.P] = 1'b1;
  child u();
  initial begin y[4] = 1'b0; #1 $display("y4=%b", y[4]); $finish; end
endmodule
"#;
    pin(
        src,
        "\
WEXT writes=2 bits=2 geo=0 word=0 undecided=0 poison=0
W C0:assign cont - c0 n0:top.y bits[4:4] via=-
W P0:initial blk s0 c0 n0:top.y bits[4:4] via=direct
N n0:top.y Logic w=64 alen=1 dims=n decl=Module/Logic/Internal
",
    );
}

/// Control, flag CLEAR (R1): `y[u.P]` on `[11:4]` with P = 6 is `bits[2:2]` and `y[9]`
/// `bits[5:5]` (PRE stdout `y=xx0xx1xx`: bit 6 set, bit 9 clear) — as PRE.
#[test]
fn wa_n3_control_low_bound() {
    let src = r#"module child; parameter int P = 6; endmodule
module top;
  logic [11:4] y;
  assign y[u.P] = 1'b1;
  child u();
  initial begin y[9] = 1'b0; #1 $display("y=%b", y); $finish; end
endmodule
"#;
    pin(
        src,
        "\
WEXT writes=2 bits=2 geo=0 word=0 undecided=0 poison=0
W C0:assign cont - c0 n0:top.y bits[2:2] via=-
W P0:initial blk s0 c0 n0:top.y bits[5:5] via=direct
N n0:top.y Logic w=8 alen=1 dims=n decl=Module/Logic/Internal
",
    );
}
