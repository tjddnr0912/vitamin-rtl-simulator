//! §4.5.601 — a part-select width, an indexed part-select width or a replication count the
//! engine cannot reduce is refused (E3009), never read as one bit, zero repetitions or the
//! whole net.
//!
//! The engine folds those three constant edges with one shallow fold (`Const`, the `Add`/
//! `Sub` of a `[m:l]` width tree, `$clog2` of a `Const`) and defaults where it has no
//! answer: a read selects one bit, a replication repeats zero times, a write spans the net
//! from the low bound up. What reached it (row-9 census, VeeR EL2; the oracle-cell
//! sources):
//!
//! * arithmetic over a select of a parameter DECLARED wider than 64 bits — a member of a
//!   packed-struct parameter (`pt.HI`, the parser's `pt[15:8]`) or a select of a plain
//!   72-bit parameter (`WP[7:0]`) in `f[pt.HI-1:0]`, `f[0 +: pt.HI+1]`, `{(pt.HI+1){…}}`.
//!   The bound walk has no self width for such a select (`select_base_at_declared`
//!   declines past 64 bits: the walk also folds other scopes' text at this prefix,
//!   §4.5.560), so the bound did not fold. EL2's fetch address read `00000000` for
//!   `80000000`;
//! * a call the constant interpreter declines (ROADMAP §2 🆕 AC), e.g. a `case` body;
//! * an x-valued count or width (both oracles refuse one), and a bound over a const-array
//!   element whose width the bound walk does not know.
//!
//! Round 2 (both review lenses): the gate asks whether the engine reads the edge's VALUE,
//! not only whether it reduces — a subtraction the fold saturates (an ascending net's
//! tree, now built in the net's direction; out-of-order bounds; a negative count) is
//! refused; a target width sized early, before the deferred pass, is refused when it
//! differs from what was assumed; a >64-bit parameter with a non-zero low bound is
//! refused in widths and counts. Only a NEGATIVE LITERAL bound (`x[-1:0]`, pinned in
//! `const_fold_bounds.rs`) keeps its old one-bit reading: an oracle split (iverilog and
//! sv2v refuse it as out of order, verilator reads a two-bit select).
//!
//! Round 3 decides every edge instead (`elaborate/src/edge_gate.rs`): its IEEE value, from
//! the constant domain or from the lowered tree's own region, handed to the engine as a
//! tree it reads right or as a `Const`; the values are pinned in `constant_edge_decision.rs`.
//! What stays refused here is what has no decided value: arithmetic over a >64-bit
//! parameter's bits (ROADMAP §2 V1b — giving the select a self width is the
//! declaring-scope stage's work), a declined call, an x value. Cells whose true width is
//! 1 or true count 0 were right by accident and are refused too (`accidental_*` below).
//!
//! ORACLES: verilator 5.052 (`--binary`) and sv2v 0.0.13 -> iverilog 13.0 (`-g2012` +
//! `vvp`) for every cell; iverilog 13.0 directly rejects `pt.HI` in a constant expression
//! ("A hierarchical reference (`pt.HI') is not allowed in a constant expression.") and
//! agrees with the other two on the plain 72-bit / 128-bit / package cells. Each REFUSED pin
//! keeps the oracles' value beside it.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_raw(src: &str) -> (String, bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_wsce_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut s = String::new();
    for l in so.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    (
        s,
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The census cell: an 80-bit packed-struct parameter `pt` (HI=5, LO=2) and a plain
/// 72-bit parameter `WP` (5), a 32-bit all-ones `f`, and one `$display`.
fn cell(pre: &str, decls: &str, body: &str, disp: &str) -> String {
    format!(
        "{pre}typedef struct packed {{ logic [63:0] PAD; logic [7:0] HI; logic [7:0] LO; }} prm_t;\n\
         module top #(parameter prm_t pt = 80'h0000000000000000_05_02, \
         parameter logic [71:0] WP = 72'h00_0000_0000_0000_0005);\n\
         \x20 logic [31:0] f, r, a;\n  integer i, n;\n{decls}\
         \x20 initial begin\n    f = 32'hffff_ffff; r = 0; a = 0; n = 0;\n{body}\n    #1 {disp}\n    $finish;\n  end\nendmodule\n"
    )
}

fn proc_cell(body: &str) -> String {
    cell("", "", body, "$display(\"r=%h\", r);")
}

fn runs(src: &str, want: &str) {
    let (out, ok, err) = run_raw(src);
    assert!(ok, "expected success, stderr:\n{err}\nsource:\n{src}");
    assert_eq!(out.trim_end(), want, "source:\n{src}");
}

fn refused(src: &str, needle: &str) {
    let (out, ok, err) = run_raw(src);
    assert!(!ok, "expected a refusal, got:\n{out}\nsource:\n{src}");
    assert!(
        err.contains("error[VITA-E3009]") && err.contains(needle),
        "unexpected diagnostic:\n{err}\nsource:\n{src}"
    );
    assert!(
        out.is_empty() || !out.contains("r="),
        "a refused design printed:\n{out}"
    );
}

const WIDE_PT: &str = "it reads `pt`, a parameter wider than 64 bits";
const PART: &str = "the width of this part-select does not fold to a constant";
const IDX: &str = "the width of this indexed part-select does not fold to a constant";
const REP: &str = "this replication count does not fold to a constant";

// ── V1, read lanes: REFUSED (PRE: silent 1 bit / 0 repetitions) ──────────────────────

/// REFUSED. `f[pt.HI-1:0]`: PRE `r=00000001`; verilator, sv2v `r=0000001f`.
#[test]
fn refused_msb_minus_member() {
    let src = proc_cell("    r = f[pt.HI-1:0];");
    refused(&src, PART);
    refused(&src, WIDE_PT);
}

/// REFUSED. `f[0 +: pt.HI+1]`: PRE `r=00000001`; verilator, sv2v `r=0000003f`.
#[test]
fn refused_indexed_width_member() {
    refused(&proc_cell("    r = f[0 +: pt.HI+1];"), IDX);
}

/// REFUSED. `f[31 -: pt.HI+1]`: PRE `r=00000001`; verilator, sv2v `r=0000003f`.
#[test]
fn refused_indexed_down_width_member() {
    refused(&proc_cell("    r = f[31 -: pt.HI+1];"), IDX);
}

/// REFUSED. `{(pt.HI+1){1'b1}}`: PRE `r=00000000`; verilator, sv2v `r=0000003f`.
#[test]
fn refused_replication_count_member() {
    let src = proc_cell("    r = {(pt.HI+1){1'b1}};");
    refused(&src, REP);
    refused(&src, WIDE_PT);
}

/// REFUSED. `f[WP[7:0]+1:0]` over a plain 72-bit parameter: PRE `r=00000001`; iverilog,
/// verilator, sv2v `r=0000007f`.
#[test]
fn refused_plain_72_bit_select() {
    refused(
        &proc_cell("    r = f[WP[7:0]+1:0];"),
        "it reads `WP`, a parameter wider than 64 bits",
    );
}

/// REFUSED. `{(WP[7:0]+1){1'b1}}`: PRE `r=00000000`; iverilog, verilator, sv2v `r=0000003f`.
#[test]
fn refused_plain_72_bit_replication() {
    refused(&proc_cell("    r = {(WP[7:0]+1){1'b1}};"), REP);
}

/// REFUSED. `f[pt.HI*2:pt.LO]`: PRE `r=00000001`; verilator, sv2v `r=000001ff`.
#[test]
fn refused_product_of_members() {
    refused(&proc_cell("    r = f[pt.HI*2:pt.LO];"), WIDE_PT);
}

/// REFUSED. A 128-bit localparam whose value does not fit 64 bits (the EL2 shape: its
/// value lives in the wide map): `f[BIG[7:0]+1:0]` PRE `r=00000001`, `{(BIG[7:0]+1){1'b1}}`
/// PRE `r=00000000`; iverilog, verilator, sv2v `r=0000007f`, `r=0000003f`.
#[test]
fn refused_value_wider_than_64_bits() {
    let d = "  localparam logic [127:0] BIG = 128'h0000_0000_0000_0001_0000_0000_0000_0005;\n";
    let p = "$display(\"r=%h\", r);";
    let big = "it reads `BIG`, a parameter wider than 64 bits";
    refused(&cell("", d, "    r = f[BIG[7:0]+1:0];", p), big);
    refused(&cell("", d, "    r = {(BIG[7:0]+1){1'b1}};", p), big);
}

/// REFUSED. A 72-bit localparam: `f[LP[7:0]+1:0]` PRE `r=00000001`; iverilog, verilator,
/// sv2v `r=0000007f`.
#[test]
fn refused_wide_localparam() {
    refused(
        &cell(
            "",
            "  localparam logic [71:0] LP = 72'h05;\n",
            "    r = f[LP[7:0]+1:0];",
            "$display(\"r=%h\", r);",
        ),
        "it reads `LP`",
    );
}

/// REFUSED, every lowering position of one select: a function body (automatic and static),
/// a task body, a net declaration assignment, a continuous assign (LSB arithmetic), an
/// `always_comb`, a `$display` argument, a generate block, a ternary arm and a continuous
/// replication. PRE `r=00000001` (`r=1`, `r=00000000` for the replication); verilator,
/// sv2v `r=0000001f` (`03ffffff` for the LSB cells, `1f` for `$display`, `0000003f` for
/// the replication).
#[test]
fn refused_in_every_lowering_position() {
    let p = "$display(\"r=%h\", r);";
    let w = "$display(\"r=%h\", w);";
    let cells = [
        cell(
            "",
            "  function automatic logic [31:0] g(input logic [31:0] x); return x[pt.HI-1:0]; endfunction\n",
            "    r = g(f);",
            p,
        ),
        cell(
            "",
            "  function logic [31:0] g(input logic [31:0] x); g = x[pt.HI-1:0]; endfunction\n",
            "    r = g(f);",
            p,
        ),
        cell(
            "",
            "  task automatic t(input logic [31:0] x, output logic [31:0] y); y = x[pt.HI-1:0]; endtask\n",
            "    t(f, r);",
            p,
        ),
        cell("", "  wire [31:0] w = f[pt.HI-1:0];\n", "", w),
        cell("", "  wire [31:0] w;\n  assign w = f[31:pt.HI+1];\n", "", w),
        cell(
            "",
            "  logic [31:0] c;\n  always_comb c = f[31:pt.HI+1];\n",
            "",
            "$display(\"r=%h\", c);",
        ),
        cell("", "", "", "$display(\"r=%h\", f[pt.HI-1:0]);"),
        cell(
            "",
            "  wire [31:0] w;\n  if (1) begin : gb\n    assign w = f[pt.HI-1:0];\n  end\n",
            "",
            w,
        ),
        cell("", "", "    r = n == 0 ? f[pt.HI-1:0] : 32'd0;", p),
        cell("", "  wire [31:0] w;\n  assign w = {(pt.HI+1){1'b1}};\n", "", w),
    ];
    for src in &cells {
        refused(src, WIDE_PT);
    }
}

/// REFUSED, every spelling of the selected object: a hierarchical net `u.f[…]` (the
/// deferred select), an interface member `ii.f[…]`, an unpacked-array element
/// `mem[2][…]`, outer elements of a multi-dim packed array `m[pt.HI-3:0]`, a generate
/// loop's `f[pt.HI-1+i:i]`, and a string target `{(pt.HI-3){"ab"}}`. PRE `r=00000001`
/// (`r=00000001 00000001` for the loop, `s=` for the string); verilator, sv2v
/// `r=0000001f`, `r=0000001f`, `r=0000001f`, `r=00000fff`, `r=0000001f 0000001f`,
/// `s=abab`.
#[test]
fn refused_in_every_select_spelling() {
    let p = "$display(\"r=%h\", r);";
    let cells = [
        cell(
            "module sub; logic [31:0] f = 32'hffff_ffff; endmodule\n",
            "  sub u();\n",
            "    #1 r = u.f[pt.HI-1:0];",
            p,
        ),
        cell(
            "interface ifc; logic [31:0] f = 32'hffff_ffff; endinterface\n",
            "  ifc ii();\n",
            "    #1 r = ii.f[pt.HI-1:0];",
            p,
        ),
        cell(
            "",
            "  logic [31:0] mem [0:3];\n",
            "    mem[2] = 32'hffff_ffff; r = mem[2][pt.HI-1:0];",
            p,
        ),
        cell(
            "",
            "  logic [7:0][3:0] m = 32'hffff_ffff;\n",
            "    #1 r = m[pt.HI-3:0];",
            p,
        ),
        cell(
            "",
            "  wire [31:0] w [0:1];\n  for (genvar k = 0; k < 2; k++) begin : g\n    assign w[k] = f[pt.HI-1+k:k];\n  end\n",
            "",
            "$display(\"r=%h %h\", w[0], w[1]);",
        ),
        cell(
            "",
            "  string s;\n",
            "    s = {(pt.HI-3){\"ab\"}};",
            "$display(\"s=%s\", s);",
        ),
    ];
    for src in &cells {
        refused(src, WIDE_PT);
    }
}

/// REFUSED, the remaining write lanes: a hierarchical target `u.f[pt.HI-1:0] = '1;`, a
/// continuous assign's indexed target `w[0 +: pt.HI+1]`, and an `always_ff` nonblocking
/// replication. PRE `r=00000001`, `r=00000001`, `r=00000000`; verilator, sv2v
/// `r=0000001f`, `r=0000003f`, `r=0000003f`.
#[test]
fn refused_in_the_remaining_write_lanes() {
    refused(
        &cell(
            "module sub; logic [31:0] g = 0; endmodule\n",
            "  sub u();\n",
            "    u.g[pt.HI-1:0] = '1;",
            "$display(\"r=%h\", u.g);",
        ),
        PART,
    );
    refused(
        &cell(
            "",
            "  wire [31:0] w;\n  assign w[0 +: pt.HI+1] = '1;\n  assign w[31:pt.HI+1] = '0;\n",
            "",
            "$display(\"r=%h\", w);",
        ),
        IDX,
    );
    refused(
        &cell(
            "",
            "  logic clk = 0;\n  logic [31:0] q;\n  always_ff @(posedge clk) q <= {(pt.HI+1){1'b1}};\n",
            "    #1 clk = 1;",
            "$display(\"r=%h\", q);",
        ),
        REP,
    );
}

// ── V1, write lanes: REFUSED (PRE: the write spanned the net, or one bit) ────────────

/// REFUSED. `a[0 +: pt.HI+1] = '1;` PRE `r=00000001`, `a[pt.HI-1:0] = '1;` and its NBA twin
/// PRE `r=00000001`, `a[31 -: pt.HI+1] = '1;` PRE `r=00000001`; verilator, sv2v
/// `r=0000003f`, `r=0000001f`, `r=0000001f`, `r=fc000000`.
#[test]
fn refused_procedural_writes() {
    let p = "$display(\"r=%h\", a);";
    refused(&cell("", "", "    a[0 +: pt.HI+1] = '1;", p), IDX);
    refused(&cell("", "", "    a[pt.HI-1:0] = '1;", p), PART);
    refused(&cell("", "", "    a[pt.HI-1:0] <= '1;", p), PART);
    refused(&cell("", "", "    a[31 -: pt.HI+1] = '1;", p), IDX);
}

/// REFUSED. A continuous-assign target `w[31:pt.HI+1]` beside `w[pt.HI:0]`: PRE
/// `r=00000040`; verilator, sv2v `r=ffffffc0`.
#[test]
fn refused_continuous_assign_target() {
    refused(
        &cell(
            "",
            "  wire [31:0] w;\n  assign w[31:pt.HI+1] = '1;\n  assign w[pt.HI:0] = '0;\n",
            "",
            "$display(\"r=%h\", w);",
        ),
        PART,
    );
}

// ── right by accident under the fallback: REFUSED as well ────────────────────────────

/// REFUSED, right by accident on PRE: the true width is 1 (`f[pt.HI-5:0]`, PRE and both
/// oracles `r=00000001`), the true count is 0 inside a concatenation
/// (`{8'hA5, {(pt.HI-5){1'b1}}}`, PRE and both oracles `r=000000a5`), the write reaches
/// the net's top bit (`a[31:pt.HI+1] = f[31:6];`, PRE and both oracles `r=ffffffc0`), and
/// an output-port actual whose net is otherwise 0 (`.q(w[pt.HI-1:0])`, PRE and both
/// oracles `r=00000015`). The fallback cannot tell these from the wrong ones.
#[test]
fn accidental_right_cells_are_refused() {
    refused(&proc_cell("    r = f[pt.HI-5:0];"), PART);
    refused(&proc_cell("    r = {8'hA5, {(pt.HI-5){1'b1}}};"), REP);
    refused(
        &cell(
            "",
            "",
            "    a[31:pt.HI+1] = f[31:6];",
            "$display(\"r=%h\", a);",
        ),
        PART,
    );
    refused(
        &cell(
            "module src(output logic [4:0] q); assign q = 5'h15; endmodule\n",
            "  wire [31:0] w;\n  assign w[31:5] = '0;\n  src u (.q(w[pt.HI-1:0]));\n",
            "",
            "$display(\"r=%h\", w);",
        ),
        PART,
    );
}

// ── 🆕 AC: a call the constant interpreter declines, same sinks: REFUSED ─────────────

const CASE_FNS: &str =
    "  function automatic int g(input int a); case (a) 2: g = 7; default: g = 1; endcase endfunction\n\
     \x20 function automatic int g0(input int a); case (a) 2: g0 = 0; default: g0 = 1; endcase endfunction\n";

/// REFUSED. A `case`-bodied function in a count or bound: `{g(2){1'b1}}` PRE `r=00000000`,
/// `f[0 +: g(2)]` PRE `r=00000001`, `f[g(2):0]` PRE `r=00000001`, `f[g(2)+1:0]` PRE
/// `r=00000001`, `f[11:g(2)]` PRE `r=00000001`, `a[g(2):0] = '1;` PRE `r=00000001`;
/// iverilog, verilator, sv2v `r=0000007f`, `0000007f`, `000000ff`, `000001ff`,
/// `0000001f`, `000000ff`. `f[g0(2):0]` (true width 1) is right by accident on PRE and
/// refused too.
#[test]
fn refused_declined_constant_call() {
    let p = "$display(\"r=%h\", r);";
    let arm = "`g(…)` has no constant-fold arm";
    refused(&cell("", CASE_FNS, "    r = {g(2){1'b1}};", p), REP);
    refused(&cell("", CASE_FNS, "    r = {g(2){1'b1}};", p), arm);
    refused(&cell("", CASE_FNS, "    r = f[0 +: g(2)];", p), IDX);
    refused(&cell("", CASE_FNS, "    r = f[g(2):0];", p), PART);
    refused(&cell("", CASE_FNS, "    r = f[g(2)+1:0];", p), PART);
    refused(&cell("", CASE_FNS, "    r = f[11:g(2)];", p), PART);
    refused(
        &cell(
            "",
            CASE_FNS,
            "    a[g(2):0] = '1;",
            "$display(\"r=%h\", a);",
        ),
        PART,
    );
    refused(&cell("", CASE_FNS, "    r = f[g0(2):0];", p), "`g0(…)`");
}

// ── must stay byte-identical ─────────────────────────────────────────────────────────

/// Bare members and bare selects fold (the i64 tier answers a select with no arithmetic
/// above it), as do a declaration range over a member, a 128-bit parameter's select in a
/// declaration range, `for` / `repeat` bounds, a member read through an `int` localparam,
/// the member's value, and an indexed select whose BASE (not width) holds the arithmetic.
/// verilator and sv2v agree on every value; iverilog on the plain-parameter ones.
#[test]
fn controls_unchanged() {
    runs(&proc_cell("    r = f[pt.HI:pt.LO];"), "r=0000000f");
    runs(&proc_cell("    r = {pt.HI{1'b1}};"), "r=0000001f");
    runs(&proc_cell("    r = f[WP[7:0]:0];"), "r=0000003f");
    runs(
        &cell(
            "",
            "  logic [pt.HI-1:0] d;\n",
            "    d = '1; r = d;",
            "$display(\"r=%h\", r);",
        ),
        "r=0000001f",
    );
    runs(
        &cell(
            "",
            "  parameter logic [127:0] K = 128'h0000_0000_0000_0000_0000_0000_DD00_0000;\n  wire [K[31:24]-1:0] w;\n",
            "",
            "$display(\"r=%0d\", $bits(w));",
        ),
        "r=221",
    );
    runs(
        &cell(
            "",
            "  logic [31:pt.HI+1] d;\n",
            "",
            "$display(\"r=%0d\", $bits(d));",
        ),
        "r=26",
    );
    runs(
        &proc_cell("    for (i = 0; i < pt.HI+1; i++) n++; r = n;"),
        "r=00000006",
    );
    runs(&proc_cell("    repeat (pt.HI+1) n++; r = n;"), "r=00000006");
    runs(
        &cell(
            "",
            "  localparam int H = pt.HI;\n",
            "    r = f[H-1:0];",
            "$display(\"r=%h\", r);",
        ),
        "r=0000001f",
    );
    runs(&proc_cell("    r = pt.HI + 1;"), "r=00000006");
    runs(&proc_cell("    r = f[pt.HI+1 -: 3];"), "r=00000007");
    runs(&proc_cell("    r = f[pt.HI+1];"), "r=00000001");
}

/// A package's >64-bit constant is selected through its named scope and folds (§4.5.560):
/// `f[pk::P[7:0]+1:0]` `r=0000007f`, all three oracles.
#[test]
fn package_wide_select_folds() {
    runs(
        &cell(
            "package pk; localparam logic [71:0] P = 72'h05; endpackage\n",
            "",
            "    r = f[pk::P[7:0]+1:0];",
            "$display(\"r=%h\", r);",
        ),
        "r=0000007f",
    );
}

/// The same five shapes over a 16-bit struct and a 16-bit parameter fold, as on PRE:
/// `r=0000001f`, `0000003f`, `0000003f`, `0000007f`, `000001ff` (verilator, sv2v; iverilog
/// on the plain-parameter cell).
#[test]
fn narrow_twins_unchanged() {
    let narrow = |body: &str| {
        format!(
            "typedef struct packed {{ logic [7:0] HI; logic [7:0] LO; }} prm_t;\n\
             module top #(parameter prm_t pt = 16'h05_02, parameter logic [15:0] WP = 16'h0005);\n\
             \x20 logic [31:0] f, r;\n  initial begin\n    f = 32'hffff_ffff;\n{body}\n    #1 $display(\"r=%h\", r);\n  end\nendmodule\n"
        )
    };
    runs(&narrow("    r = f[pt.HI-1:0];"), "r=0000001f");
    runs(&narrow("    r = f[0 +: pt.HI+1];"), "r=0000003f");
    runs(&narrow("    r = {(pt.HI+1){1'b1}};"), "r=0000003f");
    runs(&narrow("    r = f[WP[7:0]+1:0];"), "r=0000007f");
    runs(&narrow("    r = f[pt.HI*2:pt.LO];"), "r=000001ff");
}

/// A hierarchical parameter in a bound is patched by the deferred pass and then folds — the
/// gate runs after it. verilator `r=0000001f`, `r=0000003f`; iverilog and sv2v refuse the
/// hierarchical reference in a constant expression.
#[test]
fn hierarchical_parameter_bound_folds_after_the_deferred_pass() {
    let sub = "module sub; parameter W = 5; endmodule\n";
    let p = "$display(\"r=%h\", r);";
    runs(
        &cell(sub, "  sub u();\n", "    r = f[u.W-1:0];", p),
        "r=0000001f",
    );
    runs(
        &cell(sub, "  sub u();\n", "    r = {(u.W+1){1'b1}};", p),
        "r=0000003f",
    );
}

// ── round 2: what the review lenses found next to the class ─────────────────────────

/// A plain module whose `initial` displays one line — the lens cells' shape.
fn module(decls: &str, body: &str, disp: &str) -> String {
    format!("module top;\n{decls}  initial begin\n{body}\n    #1 {disp}\n    $finish;\n  end\nendmodule\n")
}

/// An ASCENDING net's part-select whose bounds fold only after lowering (`$size`,
/// `$high`, generate-block localparams): the width tree was `(msb - lsb) + 1`, which the
/// engine's saturating subtraction read as one bit. It is `(lsb - msb) + 1` now. PRE
/// `r=00000000`, `g=02000000`, and a false E3001 for the continuous twin; iverilog,
/// sv2v, verilator `r=00000004`, `g=3e000000`, `w=2a000000`, and `g=2a000000` for a task
/// output (PRE `g=02000000`); sv2v and verilator for the generate-block cell, where
/// iverilog refuses `gb.B`.
#[test]
fn ascending_net_bounds_that_fold_late_are_right() {
    let arr = "  logic [7:0] arr [6];\n  logic [0:31] g;\n  logic [31:0] r;\n";
    let p = "$display(\"r=%h\", r);";
    runs(
        &module(arr, "    g = 32'h89ab_cdef;\n    r = g[2:$size(arr)];", p),
        "r=00000004",
    );
    runs(
        &module(arr, "    g = 32'h89ab_cdef;\n    r = g[2:$high(arr)+1];", p),
        "r=00000004",
    );
    runs(
        &module(
            "  if (1) begin : gb localparam int A = 2; localparam int B = 6; end\n  logic [0:31] g;\n  logic [31:0] r;\n",
            "    g = 32'h89ab_cdef;\n    r = g[gb.A:gb.B];",
            p,
        ),
        "r=00000004",
    );
    runs(
        &module(
            arr,
            "    g = 32'h0;\n    g[2:$size(arr)] = 5'h1f;",
            "$display(\"g=%h\", g);",
        ),
        "g=3e000000",
    );
    runs(
        &module(
            "  logic [7:0] arr [6];\n  wire [0:31] w;\n  assign w[2:$size(arr)] = 5'h15;\n  assign w[0:1] = 2'b00;\n  assign w[7:31] = 25'h0;\n",
            "",
            "$display(\"w=%h\", w);",
        ),
        "w=2a000000",
    );
    runs(
        &module(
            "  logic [7:0] arr [6];\n  logic [0:31] g;\n  task automatic t(output logic [4:0] y); y = 5'h15; endtask\n",
            "    g = 32'h0;\n    t(g[2:$size(arr)]);",
            "$display(\"g=%h\", g);",
        ),
        "g=2a000000",
    );
}

/// REFUSED. A tree the engine reduces only by saturating its subtraction: a descending
/// net's bounds out of order (`f[2:$size(arr)]`, PRE `r=00000001`; iverilog and sv2v
/// "part select f[2:6] is out of order.", verilator `r=00000017`) and a negative count
/// (`{8'hA5, {($size(arr)-7){1'b1}}}`, PRE `r=000000a5`; iverilog and sv2v
/// "Concatenation repeat may not be negative (-1).", verilator "Replication value of < 0
/// or X/Z not legal").
#[test]
fn saturated_widths_and_counts_are_refused() {
    let d = "  logic [7:0] arr [6];\n  logic [31:0] f, r;\n";
    let p = "$display(\"r=%h\", r);";
    refused(
        &module(d, "    f = 32'h89ab_cdef;\n    r = f[2:$size(arr)];", p),
        "the width of this part-select is negative: its bounds are out of order",
    );
    refused(
        &module(
            d,
            "    f = 32'h89ab_cdef;\n    r = {8'hA5, {($size(arr)-7){1'b1}}};",
            p,
        ),
        "this replication count is negative",
    );
}

/// A target width that folds only after the deferred hierarchical pass no longer sizes
/// anything as one bit (round 3): a fill is spelled for the run to size, and a capture
/// holds the right-hand side at its own width. PRE `a=00000001` for `a[gb.L-1:0] = '1;`,
/// `a[gb.L-1:0] = #1 f;` and `a[u.W-1:0] = '1;`; sv2v and verilator `a=0000001f`,
/// `a=0000000f`, verilator `a=0000001f` (iverilog refuses a hierarchical reference in a
/// constant expression).
///
/// A task output copied to a part-select target whose width does not fold is refused
/// like any other write (`t(a[pt.HI-1:0])`: PRE `a=00000015`, the whole net from bit 0;
/// sv2v `a=ffff0015`): the copy-out lvalue lives in the task-call side table, outside
/// the statement arena, and the decision walks it too.
#[test]
fn a_target_width_read_before_it_folds_is_sized_right() {
    refused(
        "typedef struct packed { logic [63:0] PAD; logic [7:0] HI; logic [7:0] LO; } prm_t;\n\
         module top #(parameter prm_t pt = 80'h0000000000000000_05_02);\n\
         \x20 logic [31:0] a;\n\
         \x20 task automatic t(output logic [4:0] y); y = 5'h15; endtask\n\
         \x20 initial begin\n    a = 32'hffff_0000;\n    t(a[pt.HI-1:0]);\n    #1 $display(\"a=%h\", a);\n  end\nendmodule\n",
        WIDE_PT,
    );
    let gb = "  logic [31:0] f, a;\n  if (1) begin : gb localparam int L = 5; end\n";
    let p = "$display(\"a=%h\", a);";
    runs(
        &module(
            gb,
            "    f = 32'h89ab_cdef; a = 0;\n    a[gb.L-1:0] = '1;",
            p,
        ),
        "a=0000001f",
    );
    runs(
        &module(
            gb,
            "    f = 32'h89ab_cdef; a = 0;\n    a[gb.L-1:0] = #1 f;",
            p,
        ),
        "a=0000000f",
    );
    runs(
        &format!(
            "module sub; parameter W = 5; endmodule\n{}",
            module(
                "  logic [31:0] a;\n  sub u();\n",
                "    a = 0;\n    #1 a[u.W-1:0] = '1;",
                p
            )
        ),
        "a=0000001f",
    );
}

/// The read of the same late-folding width needs nothing sized early and runs:
/// `r = f[gb.L-1:0];` and a non-fill target `a[gb.L-1:0] = 32'hffff_ffff;` (sv2v and
/// verilator `r=0000000f`, `a=0000001f`). So does a fill whose lowering is already as
/// wide as the final target — `'0` (zero at every width) and `n ? 5'd0 : '1` (the fill
/// takes its 5-bit sibling's width) — and a task output copied to such a target, which
/// the run sizes from the folded width: verilator `a=ffffffe0`, `a=0000001f`,
/// `a=ffff0015`; sv2v `a=ffff0015` for the last (iverilog refuses `gb.L` in a constant).
#[test]
fn a_late_folding_width_that_sizes_nothing_early_runs() {
    let gb = "  logic [31:0] a;\n  integer n;\n  if (1) begin : gb localparam int L = 5; end\n";
    let p = "$display(\"a=%h\", a);";
    runs(
        &module(gb, "    a = '1;\n    a[gb.L-1:0] = '0;", p),
        "a=ffffffe0",
    );
    runs(
        &module(gb, "    a = 0; n = 0;\n    a[gb.L-1:0] = n ? 5'd0 : '1;", p),
        "a=0000001f",
    );
    runs(
        &format!(
            "module top;\n  logic [31:0] a;\n  if (1) begin : gb localparam int L = 5; end\n\
             \x20 task automatic t(output logic [4:0] y); y = 5'h15; endtask\n\
             \x20 initial begin\n    a = 32'hffff_0000;\n    t(a[gb.L-1:0]);\n    #1 {p}\n  end\nendmodule\n"
        ),
        "a=ffff0015",
    );
    late_folding_reads_run();
}

fn late_folding_reads_run() {
    let gb = "  logic [31:0] f, r, a;\n  if (1) begin : gb localparam int L = 5; end\n";
    runs(
        &module(
            gb,
            "    f = 32'h89ab_cdef; r = 0;\n    r = f[gb.L-1:0];",
            "$display(\"r=%h\", r);",
        ),
        "r=0000000f",
    );
    runs(
        &module(
            gb,
            "    a = 0;\n    a[gb.L-1:0] = 32'hffff_ffff;",
            "$display(\"a=%h\", a);",
        ),
        "a=0000001f",
    );
}

/// The split the gate keeps, and only that one: a NEGATIVE LITERAL bound (`f[-1:0]`)
/// reads the one bit at the right bound as before (PRE `r=00000001`; iverilog and sv2v
/// "part select f[-1:0] is out of order.", verilator a two-bit select, `r=00000003`).
/// Any other folded width the engine would misread is refused with its width — over
/// vita's limit through a wide parameter (`f[WP[71:40]:0]`, 1048578 bits, PRE
/// `r=00000001`; iverilog, sv2v, verilator `r=89abcdef`) — while the literal twin
/// `f[1048577:0]`, a tree the engine reads exactly, runs (`r=89abcdef` = all three).
#[test]
fn only_a_negative_literal_bound_keeps_its_old_reading() {
    let p = "$display(\"r=%h\", r);";
    let d = "  logic [31:0] f, r;\n";
    runs(
        &module(d, "    f = 32'h89ab_cdef;\n    r = f[-1:0];", p),
        "r=00000001",
    );
    refused(
        &format!(
            "module top #(parameter logic [71:0] WP = {{32'h0010_0001, 40'h0}});\n\
             \x20 logic [31:0] f, r;\n  initial begin\n    f = 32'h89ab_cdef;\n    r = f[WP[71:40]:0];\n    #1 {p}\n  end\nendmodule\n"
        ),
        "the width of this part-select (1048578 bits) exceeds vita's limit of 1048576 bits",
    );
    runs(
        &module(d, "    f = 32'h89ab_cdef;\n    r = f[1048577:0];", p),
        "r=89abcdef",
    );
}

/// REFUSED (ROADMAP §2 "A bare >64-bit non-zero-LSB parameter select reads
/// positionally"): a width or count over a >64-bit parameter declared `[79:8]`, whose
/// selects vita reads by position from bit 0. `f[P[15:8]:0]` PRE `r=00000001`,
/// `f[0 +: P[15:8]]` PRE `r=00000000`, `repeat (P[15:8])` PRE `r=00000000`; iverilog,
/// sv2v, verilator `r=0000000f`, `r=00000007`, `r=00000003`.
#[test]
fn a_wide_parameter_with_a_non_zero_low_bound_is_refused_in_widths_and_counts() {
    let hdr = "module top #(parameter logic [79:8] P = {8'd5, 56'd0, 8'd3});\n  logic [31:0] f, r;\n  initial begin\n    f = 32'h89ab_cdef; r = 0;\n";
    let tail = "\n    #1 $display(\"r=%h\", r);\n  end\nendmodule\n";
    let nz = "a parameter wider than 64 bits declared [79:8]";
    refused(&format!("{hdr}    r = f[P[15:8]:0];{tail}"), nz);
    refused(&format!("{hdr}    r = f[0 +: P[15:8]];{tail}"), nz);
    refused(&format!("{hdr}    repeat (P[15:8]) r++;{tail}"), nz);
}

/// The refusal names the cause read after the deferred pass and through casts and
/// concatenations: an operator over a hierarchical name (`f[u.W*2-1:0]`), a wide
/// parameter under a concatenation (`f[{4'b0, WP[3:0]}:0]`), an x-valued parameter
/// (`WP = {64'h0, 8'bxxxx_0101}`).
#[test]
fn the_refusal_names_the_cause() {
    let p = "$display(\"r=%h\", r);";
    refused(
        &format!(
            "module sub; parameter W = 5; endmodule\n{}",
            module(
                "  logic [31:0] f, r;\n  sub u();\n",
                "    f = 32'h89ab_cdef;\n    #1 r = f[u.W*2-1:0];",
                p
            )
        ),
        "it applies `*` to `u.W`, a hierarchical name",
    );
    refused(
        &proc_cell("    r = f[{4'b0, WP[3:0]}:0];"),
        "it reads `WP`, a parameter wider than 64 bits",
    );
    refused(
        &format!(
            "module top #(parameter logic [71:0] WP = {{64'h0, 8'bxxxx_0101}});\n\
             \x20 logic [31:0] f, r;\n  initial begin\n    f = 32'h89ab_cdef;\n    r = f[WP[7:0]+1:0];\n    #1 {p}\n  end\nendmodule\n"
        ),
        "its value has x or z bits",
    );
}
