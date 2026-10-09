//! §4.5.607 (§5.2 row 65) — a constant indexed part-select whose width is 0 or less has no
//! value (IEEE 1800-2017 §11.5.1), so it is refused where it is elaborated rather than read as
//! a default. §4.5.601 (V1) refuses it where the select is LOWERED; this file adds the selects
//! only the CONSTANT domain reads, where the width is the declaring scope's own value, and
//! pins the row's cells and its must-not-move clause. Oracles beside each cell: iverilog 13.0, sv2v 0.0.13 -> iverilog 13.0, verilator
//! 5.052 (`--binary`). PRE is vita at e474c89c (V1, md5 33a48de6) and at 73c918a6 (md5
//! a4bbc405); the two agree on every cell here.
//!
//! Open beside this row (ROADMAP §2, measured, not changed here): a zero width in the
//! UNSELECTED arm of a constant `?:` that only the constant domain reads (a parameter value,
//! a declaration bound, a generate condition, an enum value, an override) still folds the
//! live arm — the constant domain never folds the dead one — a routine in a package nothing
//! references is never elaborated, and text folded away from its declaring scope keeps the
//! earlier default (the claim is opt-in).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// stdout without the `simulation ended` line, success, stderr
fn run_raw(src: &str) -> (String, bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_zwi_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout);
    let s: Vec<&str> = so
        .lines()
        .filter(|l| !l.starts_with("simulation ended"))
        .collect();
    (
        s.join("\n"),
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// One refusal: exactly one error, with this code at this `line:col`, holding this text.
fn refused(cells: &[(&str, &str, &str, &str, &str)]) {
    let mut bad = Vec::new();
    for (name, src, at, code, text) in cells {
        let (out, ok, err) = run_raw(src);
        let head = format!(".sv:{at}: error[VITA-{code}]");
        let line = err
            .lines()
            .find(|l| l.contains("error[VITA-"))
            .unwrap_or("");
        let pass = !ok
            && err.matches("error[VITA-").count() == 1
            && line.contains(&head)
            && line.contains(text);
        if !pass {
            bad.push(format!("{name}: stdout {out:?}\nstderr {err}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n---\n"));
}

/// Runs, and prints exactly this.
fn prints(cells: &[(&str, &str, &str)]) {
    let mut bad = Vec::new();
    for (name, src, want) in cells {
        let (out, ok, err) = run_raw(src);
        if !(ok && out.trim_end() == *want) {
            bad.push(format!("{name}: stdout {out:?}\nstderr {err}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n---\n"));
}

/// A declaration bound holding an indexed part-select of width 0 or less: both constant folds
/// decline it, and with every name a constant the bound check said nothing — the declaration
/// got a SILENT one-bit (or 32-bit, or 8-element) default at exit 0 where every oracle that
/// parses it refuses. Each lane that reaches `check_const_range_bound`: a variable, a net, a
/// typedef, a function return and a function local, an ANSI port, a generate-block
/// declaration, a typed localparam, an unpacked dimension, an md-parameter element, a >64-bit
/// parameter, `-:`, a literal width, a negative width, a select inside the bound, the LSB.
#[test]
fn declaration_bound_of_width_zero_or_less_is_refused() {
    refused(&[
        // z.decl_range: iverilog refuses; sv2v refuses; verilator refuses; PRE runs, b=1
        (
            "z.decl_range",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  logic [P[0 +: N]:0] x;
  initial #1 $display("A zdr b=%0d", $bits(x));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:17",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // y.decl_range_lit: iverilog refuses; sv2v refuses; verilator refuses; PRE runs, b=1
        (
            "y.decl_range_lit",
            r#"module m;
  localparam logic [7:0] P = 8'h03;
  logic [P[0 +: 0]:0] x;
  initial #1 $display("A ydl b=%0d", $bits(x));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:17",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // y.decl_range_minus: iverilog refuses; sv2v refuses; verilator refuses; PRE runs, b=1
        (
            "y.decl_range_minus",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  logic [P[1 -: N]:0] x;
  initial #1 $display("A ydm b=%0d", $bits(x));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:17",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // y.decl_range_vec: iverilog refuses; sv2v refuses; verilator refuses; PRE runs, b=1
        (
            "y.decl_range_vec",
            r#"module m #(parameter int N = 0);
  localparam int L = 3;
  logic [L[0 +: N]:0] x;
  initial #1 $display("A ydv b=%0d", $bits(x));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:17",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // y.port_range_zw: iverilog refuses; sv2v refuses; verilator refuses; PRE runs, b=1
        (
            "y.port_range_zw",
            r#"module sub #(parameter int N = 0)();
  localparam logic [7:0] P = 8'h03;
  logic [P[0 +: N]:0] x;
  initial #1 $display("A ypz b=%0d", $bits(x));
endmodule
module m; sub s(); endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:17",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // w.ansi_port_range: iverilog refuses; sv2v refuses; verilator refuses; PRE runs, b=1
        (
            "w.ansi_port_range",
            r#"module sub #(parameter int N = 0, parameter logic [7:0] P = 8'h03)(input logic [P[0 +: N]:0] i);
  initial #1 $display("A wap b=%0d", $bits(i));
endmodule
module m; logic [3:0] w = 4'h5; sub s(.i(w)); endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "1:88",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // w.decl_range_md: iverilog sorry; sv2v refuses; verilator refuses; PRE runs, b=1
        (
            "w.decl_range_md",
            r#"module m #(parameter int N = 0);
  localparam logic [1:0][3:0] P = 8'h33;
  logic [P[1][0 +: N]:0] x;
  initial #1 $display("A wdm b=%0d", $bits(x));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:20",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // w.decl_range_neg: iverilog b=4; sv2v b=4; verilator refuses; PRE runs, b=1
        (
            "w.decl_range_neg",
            r#"module m #(parameter int N = -1);
  localparam logic [7:0] P = 8'h03;
  logic [P[0 +: N]:0] x;
  initial #1 $display("A wdn b=%0d", $bits(x));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:17",
            "E3009",
            "the width of this indexed part-select is negative",
        ),
        // w.decl_range_wide: iverilog refuses; sv2v refuses; verilator refuses; PRE runs, b=1
        (
            "w.decl_range_wide",
            r#"module m #(parameter int N = 0);
  localparam logic [99:0] P = 100'h3;
  logic [P[0 +: N]:0] x;
  initial #1 $display("A wdw b=%0d", $bits(x));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:17",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // w.fn_local_range: iverilog refuses; sv2v refuses; verilator refuses; PRE runs, r=1
        (
            "w.fn_local_range",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  function automatic int f(); logic [P[0 +: N]:0] t; t = '1; return t; endfunction
  initial #1 $display("A wfl r=%0d", f());
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:45",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // w.fn_ret_range: iverilog refuses; sv2v refuses; verilator refuses; PRE runs, r=1
        (
            "w.fn_ret_range",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  function automatic logic [P[0 +: N]:0] f(); return '1; endfunction
  initial #1 $display("A wfr r=%0d", f());
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:36",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // w.gen_lp_range: iverilog refuses; sv2v refuses; verilator refuses; PRE runs, b=1
        (
            "w.gen_lp_range",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  if (1) begin : g logic [P[0 +: N]:0] x; initial #1 $display("A wgl b=%0d", $bits(x)); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:34",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // w.lp_typed_range: iverilog refuses; sv2v refuses; verilator refuses; PRE runs, Q=4294967295 b=32
        (
            "w.lp_typed_range",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  localparam logic [P[0 +: N]:0] Q = '1;
  initial #1 $display("A wlr Q=%0d b=%0d", Q, $bits(Q));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:28",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // w.typedef_range: iverilog refuses; sv2v refuses; verilator refuses; PRE runs, b=1
        (
            "w.typedef_range",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  typedef logic [P[0 +: N]:0] t_t;
  t_t x;
  initial #1 $display("A wtr b=%0d", $bits(x));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:25",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // w.unpacked_dim: iverilog refuses; sv2v refuses; verilator refuses; PRE runs, n=8
        (
            "w.unpacked_dim",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  logic [7:0] x [P[0 +: N]:0];
  initial #1 $display("A wud n=%0d", $size(x));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:25",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // w.wire_range: iverilog refuses; sv2v refuses; verilator refuses; PRE runs, b=1 x=1
        (
            "w.wire_range",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  wire [P[0 +: N]:0] x = '1;
  initial #1 $display("A wwr b=%0d x=%0d", $bits(x), x);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:16",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // v.decl_range_nested: iverilog refuses; sv2v refuses; verilator refuses; PRE runs, b=1
        (
            "v.decl_range_nested",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  logic [P[0 +: N] + 2:0] x;
  initial #1 $display("A vdn b=%0d", $bits(x));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:17",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // v.decl_range_lsb: iverilog refuses; sv2v refuses; verilator refuses; PRE runs, b=8
        (
            "v.decl_range_lsb",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  logic [7:P[0 +: N]] x;
  initial #1 $display("A vdl b=%0d", $bits(x));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:19",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
    ]);
}

/// Loud on PRE with "the part-select `P[…]` has no constant-fold arm" — the arm exists; the
/// WIDTH is why it declined. Named now with the indexed-width funnel's text (a parameter value,
/// a header parameter, a generate condition, init or step).
#[test]
fn constant_value_names_the_zero_width() {
    refused(&[
        // z.lp_init: iverilog refuses; sv2v refuses; verilator refuses; PRE refused E3009
        (
            "z.lp_init",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'hA5;
  localparam logic [7:0] X = P[0 +: N];
  initial #1 $display("A zlp X=%h", X);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:30",
            "E3009",
            "parameter `X` value is not a constant: the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // z.lp_init_md: iverilog sorry; sv2v refuses; verilator refuses; PRE refused E3009
        (
            "z.lp_init_md",
            r#"module m #(parameter int N = 0);
  localparam logic [1:0][3:0] P = 8'hA5;
  localparam logic [7:0] X = P[1][0 +: N];
  initial #1 $display("A zlm X=%h", X);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:30",
            "E3009",
            "parameter `X` value is not a constant: the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // y.lp_tern_live0: iverilog refuses; sv2v refuses; verilator refuses; PRE refused E3009
        (
            "y.lp_tern_live0",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'hA5;
  localparam logic [7:0] X = (N == 0) ? P[0 +: N] : 8'h33;
  initial #1 $display("A ytl X=%h", X);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:30",
            "E3009",
            "parameter `X` value is not a constant: the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // z.gen_cond: iverilog refuses; sv2v refuses; verilator refuses; PRE refused E3010
        (
            "z.gen_cond",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'hA5;
  wire [7:0] o;
  if (P[0 +: N] == 0) begin : g assign o = 8'h22; end else begin : h assign o = 8'h33; end
  initial #1 $display("A zgn o=%h", o);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "4:7",
            "E3010",
            "generate-if condition is not a constant: the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // y.gcase_sel: iverilog refuses; sv2v refuses; verilator refuses; PRE refused E3010
        (
            "y.gcase_sel",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'hA5;
  wire [7:0] o;
  case (P[0 +: N]) 0: begin : z assign o = 8'h22; end default: begin : d assign o = 8'h33; end endcase
  initial #1 $display("A ygs o=%h", o);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "4:9",
            "E3010",
            "generate-case scrutinee is not a constant: the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // y.gfor_bound: iverilog refuses; sv2v refuses; verilator refuses; PRE refused E3010
        (
            "y.gfor_bound",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  logic [7:0] o = 0;
  for (genvar g = 0; g < P[0 +: N]; g++) begin : gl initial o[g] = 1'b1; end
  initial #1 $display("A ygb o=%h", o);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "4:22",
            "E3010",
            "generate-for condition is not a constant: the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // v.lp_nested: iverilog refuses; sv2v refuses; verilator refuses; PRE refused E3009
        (
            "v.lp_nested",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'hA5;
  localparam int X = P[0 +: N] + 1;
  initial #1 $display("A vln X=%0d", X);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:22",
            "E3009",
            "parameter `X` value is not a constant: the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // u.hdr_param: iverilog refuses; sv2v refuses; verilator refuses; PRE refused E3009
        (
            "u.hdr_param",
            r#"module m #(parameter int N = 0, parameter logic [7:0] P = 8'hA5, parameter logic [7:0] X = P[0 +: N]);
  initial #1 $display("A uhp X=%h", X);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "1:92",
            "E3009",
            "parameter `X` value is not a constant: the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // u.gfor_init: iverilog refuses; sv2v refuses; verilator refuses; PRE refused E3010
        (
            "u.gfor_init",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'h00;
  logic [7:0] o = 0;
  for (genvar g = P[0 +: N]; g < 2; g++) begin : gl initial o[g] = 1'b1; end
  initial #1 $display("A ugi o=%h", o);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "4:19",
            "E3010",
            "generate-for init is not a constant: the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // u.gfor_step: iverilog refuses; sv2v refuses; verilator refuses; PRE refused E3010
        (
            "u.gfor_step",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'h01;
  logic [7:0] o = 0;
  for (genvar g = 0; g < 2; g = g + P[0 +: N]) begin : gl initial o[g] = 1'b1; end
  initial #1 $display("A ugs o=%h", o);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "4:33",
            "E3010",
            "generate-for step is not a constant: the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
    ]);
}

/// A generate-case label whose indexed part-select has a width of 0 or less, read before the
/// match: both folds decline it, and PRE skipped it and took the default arm at exit 0 (all
/// three oracles refuse). A label after the match is not read: `must_not_move`'s
/// `u.gcase_label_after`.
#[test]
fn generate_case_label_read_before_the_match_is_refused() {
    refused(&[
        // u.gen_case_item: iverilog refuses; sv2v refuses; verilator refuses; PRE runs, o=33
        (
            "u.gen_case_item",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'hA5;
  wire [7:0] o;
  case (1) P[0 +: N]: begin : a assign o = 8'h22; end default: begin : d assign o = 8'h33; end endcase
  initial #1 $display("A ugc o=%h", o);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "4:12",
            "E3010",
            "generate-case label is not a constant: the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
    ]);
}

/// The row's own cells, refused by the §4.5.601 indexed-width funnel wherever the select is
/// lowered — a constant-false procedural branch and an uncalled function included (D3:
/// independent of row 17). Pinned here so the row's exit is one file.
#[test]
fn row_cells_refused_at_the_runtime_funnel() {
    refused(&[
        // x.near.revi_vec: iverilog refuses; sv2v refuses; verilator refuses; PRE refused E3009
        (
            "x.near.revi_vec",
            r#"// plain-vector twin of revi_s1: an indexed part-select whose named width folds to 0
module m;
  localparam int H = 1, L = 2; logic [7:0] v;
  initial begin v = 8'hA5; #1 $display("A revivec p=%h", v[L +: (H-L+1)]); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "4:65",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // x.near.revi_lit: iverilog refuses; sv2v refuses; verilator refuses; PRE refused E3009
        (
            "x.near.revi_lit",
            r#"// literal twin: an indexed member part-select of width 0 written as a literal
module m;
  typedef struct packed {logic [3:0] a; logic [3:0] b;} s_t;
  s_t v;
  initial begin v = 8'hA5; #1 $display("A revilit p=%h", v.a[2 +: 0]); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "5:67",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // x.near.revi_s1: iverilog refuses; sv2v refuses; verilator refuses; PRE refused E3009
        (
            "x.near.revi_s1",
            r#"// R05 mechanism twin: an indexed member part-select whose named width folds to 0 (a reversed range spelled +:)
module m;
  typedef struct packed {logic [3:0] a; logic [3:0] b;} s_t;
  localparam int H = 1, L = 2; s_t v;
  initial begin v = 8'hA5; #1 $display("A revi p=%h", v.a[L +: (H-L+1)]); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "5:64",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // rv.zw_tern: iverilog refuses; sv2v refuses; verilator refuses; PRE refused E3009
        (
            "rv.zw_tern",
            r#"// zero-width indexed select in the dead arm of a constant ternary
module m #(parameter int N = 0);
  logic [7:0] v; logic [7:0] o;
  initial v = 8'hA5;
  assign o = (N > 0) ? v[0 +: N] : 8'h33;
  initial #1 $display("A zwtern o=%h", o);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "5:31",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // rv.zw_fn: iverilog refuses; sv2v refuses; verilator refuses; PRE refused E3009
        (
            "rv.zw_fn",
            r#"// zero-width indexed select inside a function that is never called
module m #(parameter int N = 0);
  logic [7:0] v;
  function automatic logic [7:0] f(input logic [7:0] x); return x[0 +: N]; endfunction
  initial begin v = 8'hA5; #1 $display("A zwfn v=%h", v); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "4:72",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // rv.zw_vecmem: iverilog sorry; sv2v refuses; verilator refuses; PRE refused E3009
        (
            "rv.zw_vecmem",
            r#"// ZW lanes beyond the matrix: md parameter element select and a function-return select with a zero indexed width (live)
module m;
  localparam int N = 0;
  localparam logic [1:0][3:0] P = 8'hA5;
  function automatic logic [7:0] f(); return 8'hA5; endfunction
  logic [7:0] a, b;
  initial begin a = P[1][0 +: N]; b = f() ; #1 $display("A zwvm a=%h", a); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "7:31",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // rv.zw_procif: iverilog o=xx; sv2v o=xx; verilator refuses; PRE refused E3009
        (
            "rv.zw_procif",
            r#"// zero-width indexed select in a constant-false PROCEDURAL if (S_COUNT=1-style configuration)
module m #(parameter int N = 0);
  logic [7:0] v; logic [7:0] o;
  initial v = 8'hA5;
  always @* begin o = 8'h11; if (N > 0) o = v[0 +: N]; end
  initial #1 $display("A zwproc o=%h", o);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "5:52",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // rv.zw_clog: iverilog o=xx; sv2v o=xx; verilator refuses; PRE refused E3009
        (
            "rv.zw_clog",
            r#"// the alexforencich shape: CL = $clog2(S_COUNT) = 0 for S_COUNT=1; select [i*CL +: CL] guarded by S_COUNT > 1
module m #(parameter int S_COUNT = 1);
  localparam int CL = $clog2(S_COUNT);
  logic [7:0] v; logic [7:0] o;
  initial v = 8'hA5;
  always @* begin o = 8'h44; if (S_COUNT > 1) o = v[0 +: CL]; end
  initial #1 $display("A zwclog o=%h", o);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "6:58",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // r2.zw_procif2: iverilog o=xx; sv2v o=xx; verilator refuses; PRE refused E3009
        (
            "r2.zw_procif2",
            r#"// re-measure of rv.zw_procif without the time-0 race: v changes at #1 (after every process is armed), read at #3
module m #(parameter int N = 0);
  logic [7:0] v; logic [7:0] o;
  initial begin v = 8'h00; #1 v = 8'hA5; end
  always @* begin o = 8'h11; if (N > 0) o = v[0 +: N]; end
  initial #3 $display("A zwproc2 o=%h", o);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "5:52",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // r2.zw_clog2: iverilog o=xx; sv2v o=xx; verilator refuses; PRE refused E3009
        (
            "r2.zw_clog2",
            r#"// re-measure of rv.zw_clog without the time-0 race (S_COUNT=1: CL = $clog2(1) = 0), v changes at #1
module m #(parameter int S_COUNT = 1);
  localparam int CL = $clog2(S_COUNT);
  logic [7:0] v; logic [7:0] o;
  initial begin v = 8'h00; #1 v = 8'hA5; end
  always @* begin o = 8'h44; if (S_COUNT > 1) o = v[0 +: CL]; end
  initial #3 $display("A zwclog2 o=%h", o);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "6:58",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
    ]);
}

/// MUST NOT MOVE: a select in a constant-false GENERATE branch, a zero-trip generate loop or an
/// unselected generate-case arm is never elaborated (all four tools print the live arm); a
/// LEGAL width in a dead procedural branch (vita = verilator; iverilog's `xx` is its
/// `always @*` sensitivity); the legal-width controls; every channel that makes a zero
/// DEFAULT legal before the select is read — `#()`, `defparam`, an instance array, two
/// instances, an interface, a genvar — so the new bound check never reads a default; a
/// generate-case label after the match, which iverilog and sv2v never read (verilator
/// refuses it: a split, so it keeps running); and a label read before the match that vita
/// cannot fold for another reason (a constant function with a `case`: all four take the
/// default).
#[test]
fn must_not_move() {
    prints(&[
        // rv.zw_genif: iverilog o=22; sv2v o=22; verilator o=22; PRE runs, o=22
        (
            "rv.zw_genif",
            r#"// zero-width indexed select in a constant-false GENERATE if
module m #(parameter int N = 0);
  logic [7:0] v; logic [7:0] o;
  initial v = 8'hA5;
  if (N > 0) begin : g assign o = v[0 +: N]; end else begin : h assign o = 8'h22; end
  initial #1 $display("A zwgen o=%h", o);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "A zwgen o=22",
        ),
        // r2.zw_procif_ctl: iverilog o=xx; sv2v o=xx; verilator o=11; PRE runs, o=11
        (
            "r2.zw_procif_ctl",
            r#"// control: the same block with a LEGAL width (N=2 in the dead branch guard N>2): is iverilog's xx about the zero width or the time-0 race?
module m #(parameter int N = 2);
  logic [7:0] v; logic [7:0] o;
  initial v = 8'hA5;
  always @* begin o = 8'h11; if (N > 2) o = v[0 +: N]; end
  initial #1 $display("A zwctl o=%h", o);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "A zwctl o=11",
        ),
        // z.gen_for0: iverilog o=22; sv2v o=22; verilator o=22; PRE runs, o=22
        (
            "z.gen_for0",
            r#"module m #(parameter int N = 0);
  logic [7:0] v; logic [7:0] o;
  initial v = 8'hA5;
  assign o = 8'h22;
  for (genvar g = 0; g < N; g++) begin : gl initial $display("A zgf x=%h", v[0 +: N]); end
  initial #1 $display("A zgf o=%h", o);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "A zgf o=22",
        ),
        // z.gen_case: iverilog o=22; sv2v o=22; verilator o=22; PRE runs, o=22
        (
            "z.gen_case",
            r#"module m #(parameter int N = 0);
  logic [7:0] v; logic [7:0] o;
  initial v = 8'hA5;
  case (N) 0: begin : z assign o = 8'h22; end default: begin : d assign o = v[0 +: N]; end endcase
  initial #1 $display("A zgc o=%h", o);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "A zgc o=22",
        ),
        // z.gen_inst_false: iverilog q=22; sv2v q=22; verilator q=22; PRE runs, q=22
        (
            "z.gen_inst_false",
            r#"module sub #(parameter int N = 0)(output logic [7:0] q); logic [7:0] v = 8'hA5; assign q = v[0 +: N]; endmodule
module m #(parameter int N = 0);
  wire [7:0] q;
  if (N > 0) begin : g sub #(.N(N)) s(.q(q)); end else begin : h assign q = 8'h22; end
  initial #1 $display("A zgi q=%h", q);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "A zgi q=22",
        ),
        // y.decl_range_ctl: iverilog b=4; sv2v b=4; verilator b=4; PRE runs, b=4
        (
            "y.decl_range_ctl",
            r#"module m #(parameter int N = 2);
  localparam logic [7:0] P = 8'h03;
  logic [P[0 +: N]:0] x;
  initial #1 $display("A ydc b=%0d", $bits(x));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "A ydc b=4",
        ),
        // y.lp_init_ctl: iverilog X=05; sv2v X=05; verilator X=05; PRE runs, X=05
        (
            "y.lp_init_ctl",
            r#"module m #(parameter int N = 4);
  localparam logic [7:0] P = 8'hA5;
  localparam logic [7:0] X = P[0 +: N];
  initial #1 $display("A ylc X=%h", X);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "A ylc X=05",
        ),
        // y.lp_tern_ctl: iverilog X=05; sv2v X=05; verilator X=05; PRE runs, X=05
        (
            "y.lp_tern_ctl",
            r#"module m #(parameter int N = 4);
  localparam logic [7:0] P = 8'hA5;
  localparam logic [7:0] X = (N > 0) ? P[0 +: N] : 8'h33;
  initial #1 $display("A ytc X=%h", X);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "A ytc X=05",
        ),
        // y.port_decl_range: iverilog b=4; sv2v b=4; verilator b=4; PRE runs, b=4
        (
            "y.port_decl_range",
            r#"module sub #(parameter int N = 0)(input logic [8'h03 >> N:0] i); initial #1 $display("A ypr b=%0d", $bits(i)); endmodule
module m;
  localparam logic [7:0] P = 8'h03;
  logic [3:0] w = 4'h5;
  sub #(.N(0)) s(.i(w));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "A ypr b=4",
        ),
        // h.port_ovr: iverilog b=4; sv2v b=4; verilator b=4; PRE runs, b=4
        (
            "h.port_ovr",
            r#"module sub #(parameter int N = 0, parameter logic [7:0] P = 8'h03)(input logic [P[0 +: N]:0] i);
  initial #1 $display("A hpo b=%0d", $bits(i));
endmodule
module top; logic [3:0] w = 4'h5; sub #(.N(2)) s(.i(w)); initial #5 $finish; endmodule
"#,
            "A hpo b=4",
        ),
        // h.body_ovr: iverilog b=4; sv2v b=4; verilator b=4; PRE runs, b=4
        (
            "h.body_ovr",
            r#"module sub #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  logic [P[0 +: N]:0] x;
  initial #1 $display("A hbo b=%0d", $bits(x));
endmodule
module top; sub #(.N(2)) s(); initial #5 $finish; endmodule
"#,
            "A hbo b=4",
        ),
        // h.typedef_ovr: iverilog b=4; sv2v b=4; verilator b=4; PRE runs, b=4
        (
            "h.typedef_ovr",
            r#"module sub #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  typedef logic [P[0 +: N]:0] t_t;
  t_t x;
  initial #1 $display("A hto b=%0d", $bits(x));
endmodule
module top; sub #(.N(2)) s(); initial #5 $finish; endmodule
"#,
            "A hto b=4",
        ),
        // h.fnret_ovr: iverilog r=15; sv2v r=15; verilator r=15; PRE runs, r=15
        (
            "h.fnret_ovr",
            r#"module sub #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  function automatic logic [P[0 +: N]:0] f(); return '1; endfunction
  initial #1 $display("A hfo r=%0d", f());
endmodule
module top; sub #(.N(2)) s(); initial #5 $finish; endmodule
"#,
            "A hfo r=15",
        ),
        // h.genvar_range: iverilog g=1 b=2 g=2 b=4; sv2v g=1 b=2 g=2 b=4; verilator g=1 b=2 g=2 b=4; PRE runs, g=1 b=2 g=2 b=4
        (
            "h.genvar_range",
            r#"module top;
  localparam logic [7:0] P = 8'h03;
  for (genvar g = 1; g < 3; g++) begin : gl logic [P[0 +: g]:0] x; initial #1 $display("A hgr g=%0d b=%0d", g, $bits(x)); end
  initial #5 $finish;
endmodule
"#,
            "A hgr g=1 b=2\nA hgr g=2 b=4",
        ),
        // h.inst_array_ovr: iverilog b=4 b=4; sv2v b=4 b=4; verilator b=4 b=4; PRE runs, b=4 b=4
        (
            "h.inst_array_ovr",
            r#"module sub #(parameter int N = 0)(input logic [8'h03 >> 0:0] i);
  localparam logic [7:0] P = 8'h03;
  logic [P[0 +: N]:0] x;
  initial #1 $display("A hia b=%0d", $bits(x));
endmodule
module top; logic [3:0] w = 4'h5; sub #(.N(2)) s[1:0] (.i(w)); initial #5 $finish; endmodule
"#,
            "A hia b=4\nA hia b=4",
        ),
        // h.defparam_ovr: iverilog b=4; sv2v b=4; verilator b=4; PRE runs, b=4
        (
            "h.defparam_ovr",
            r#"module sub #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  logic [P[0 +: N]:0] x;
  initial #1 $display("A hdp b=%0d", $bits(x));
endmodule
module top; sub s(); defparam s.N = 2; initial #5 $finish; endmodule
"#,
            "A hdp b=4",
        ),
        // h.lp_ovr: iverilog X=05; sv2v X=05; verilator X=05; PRE runs, X=05
        (
            "h.lp_ovr",
            r#"module sub #(parameter int N = 0);
  localparam logic [7:0] P = 8'hA5;
  localparam logic [7:0] X = P[0 +: N];
  initial #1 $display("A hlo X=%h", X);
endmodule
module top; sub #(.N(4)) s(); initial #5 $finish; endmodule
"#,
            "A hlo X=05",
        ),
        // h.two_inst: iverilog N=2 b=4 N=1 b=2; sv2v N=2 b=4 N=1 b=2; verilator N=1 b=2 N=2 b=4; PRE runs, N=2 b=4 N=1 b=2
        (
            "h.two_inst",
            r#"module sub #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  logic [P[0 +: N]:0] x;
  initial #1 $display("A h2i N=%0d b=%0d", N, $bits(x));
endmodule
module top; sub #(.N(2)) a(); sub #(.N(1)) b(); initial #5 $finish; endmodule
"#,
            "A h2i N=2 b=4\nA h2i N=1 b=2",
        ),
        // h.iface_ovr: iverilog b=4; sv2v parse-error; verilator b=4; PRE runs, b=4
        (
            "h.iface_ovr",
            r#"interface ifc #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  logic [P[0 +: N]:0] x;
endinterface
module top; ifc #(.N(2)) i(); initial #1 $display("A hif b=%0d", $bits(i.x)); initial #5 $finish; endmodule
"#,
            "A hif b=4",
        ),
        // h.gen_false_inst: iverilog top; sv2v top; verilator top; PRE runs, top
        (
            "h.gen_false_inst",
            r#"module sub #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  logic [P[0 +: N]:0] x;
  initial #1 $display("A hgf b=%0d", $bits(x));
endmodule
module top #(parameter int K = 0); if (K > 0) begin : g sub #(.N(K)) s(); end initial #1 $display("A hgf top"); initial #5 $finish; endmodule
"#,
            "A hgf top",
        ),
        // u.gcase_label_after: iverilog o=22; sv2v o=22; verilator refuses; PRE runs, o=22
        (
            "u.gcase_label_after",
            r#"module m #(parameter int N = 0);
  localparam logic [7:0] P = 8'hA5;
  wire [7:0] o;
  case (1) 1: begin : a assign o = 8'h22; end P[0 +: N]: begin : b assign o = 8'h44; end default: begin : d assign o = 8'h33; end endcase
  initial #1 $display("A uga o=%h", o);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "A uga o=22",
        ),
        // u.gcase_label_cfn: iverilog o=33; sv2v o=33; verilator o=33; PRE runs, o=33
        (
            "u.gcase_label_cfn",
            r#"module m;
  function automatic int fc(input int k); case (k) 0: return 5; default: return 6; endcase endfunction
  wire [7:0] o;
  case (1) fc(0): begin : a assign o = 8'h22; end default: begin : d assign o = 8'h33; end endcase
  initial #1 $display("A ugf o=%h", o);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "A ugf o=33",
        ),
    ]);
}

/// Loud, with exactly PRE's diagnostics: each line `line:col: error[…] text`, nothing added.
fn loud_as_pre(cells: &[(&str, &str, &[&str])]) {
    let mut bad = Vec::new();
    for (name, src, want) in cells {
        let (out, ok, err) = run_raw(src);
        let got: Vec<&str> = err.lines().filter(|l| l.contains("error[VITA-")).collect();
        let pass = !ok
            && got.len() == want.len()
            && got.iter().zip(want.iter()).all(|(g, w)| g.contains(w));
        if !pass {
            bad.push(format!("{name}: stdout {out:?}\nstderr {err}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n---\n"));
}

/// Review round 1 F1: a zero or negative width is claimed only where it is the value the
/// select's DECLARING scope gives. A package routine's or typedef's text, a `$unit` routine,
/// and a module typedef or routine read inside a generate block or loop that redeclares one
/// of its names are folded at the user's prefix (ROADMAP §5.2 rows 46, 68, 71), where the
/// user's own `N = 0` binds; round 1 refused every one of these legal designs with a false
/// "zero". Each prints PRE's value, which all three oracles print. The generate-local and
/// genvar controls print too.
#[test]
fn a_select_read_away_from_its_declaring_scope_keeps_pre() {
    prints(&[
        // f.pkfn_imp_ok: iverilog r=1; sv2v r=1; verilator r=1; PRE r=1
        (
            "f.pkfn_imp_ok",
            r#"package pk;
  localparam int N = 2; localparam logic [7:0] P = 8'h03;
  function automatic int pf(); logic [P[0 +: N]:0] t; t = 1'b1; return t; endfunction
endpackage
module top #(parameter int N = 0);
  import pk::pf;
  localparam logic [7:0] P = 8'h03;
  initial #1 $display("A f_pki r=%0d", pf());
  initial #5 $finish;
endmodule
"#,
            "A f_pki r=1",
        ),
        // f.pkfn_wild_ok: iverilog r=1; sv2v r=1; verilator r=1; PRE r=1
        (
            "f.pkfn_wild_ok",
            r#"package pk;
  localparam int N = 2; localparam logic [7:0] P = 8'h03;
  function automatic int pf(); logic [P[0 +: N]:0] t; t = 1'b1; return t; endfunction
endpackage
module top;
  import pk::*;
  localparam int N = 0;
  initial #1 $display("A f_pkw r=%0d", pf());
  initial #5 $finish;
endmodule
"#,
            "A f_pkw r=1",
        ),
        // f.pkfn_scoped_ok: iverilog r=1; sv2v r=1; verilator r=1; PRE r=1
        (
            "f.pkfn_scoped_ok",
            r#"package pk;
  localparam int N = 2; localparam logic [7:0] P = 8'h03;
  function automatic int pf(); logic [P[0 +: N]:0] t; t = 1'b1; return t; endfunction
endpackage
module top;
  localparam int N = 0;
  localparam logic [7:0] P = 8'h03;
  initial #1 $display("A f_pks r=%0d", pk::pf());
  initial #5 $finish;
endmodule
"#,
            "A f_pks r=1",
        ),
        // f.pkfn_ret_ok: iverilog r=1; sv2v r=1; verilator r=1; PRE r=1
        (
            "f.pkfn_ret_ok",
            r#"package pk;
  localparam int N = 2; localparam logic [7:0] P = 8'h03;
  function automatic logic [P[0 +: N]:0] pf(); return 1'b1; endfunction
endpackage
module top;
  import pk::pf;
  localparam int N = 0;
  localparam logic [7:0] P = 8'h03;
  initial #1 $display("A f_pkr r=%0d", pf());
  initial #5 $finish;
endmodule
"#,
            "A f_pkr r=1",
        ),
        // f.pktask_ok: iverilog r=1; sv2v r=1; verilator r=1; PRE r=1
        (
            "f.pktask_ok",
            r#"package pk;
  localparam int N = 2; localparam logic [7:0] P = 8'h03;
  task automatic pt(output int r); logic [P[0 +: N]:0] v; v = 1'b1; r = v; endtask
endpackage
module top;
  import pk::pt;
  localparam int N = 0;
  localparam logic [7:0] P = 8'h03;
  int r;
  initial begin #1 pt(r); $display("A f_pkt r=%0d", r); end
  initial #5 $finish;
endmodule
"#,
            "A f_pkt r=1",
        ),
        // g.pkfn_const_ctx: iverilog K=1; sv2v K=1; verilator K=1; PRE K=1
        (
            "g.pkfn_const_ctx",
            r#"package pk;
  localparam int N = 2; localparam logic [7:0] P = 8'h03;
  function automatic int pf(); logic [P[0 +: N]:0] t; t = 1'b1; return t; endfunction
endpackage
module top;
  import pk::pf;
  localparam int N = 0;
  localparam logic [7:0] P = 8'h03;
  localparam int K = pf();
  initial #1 $display("A g_pcc K=%0d", K);
  initial #5 $finish;
endmodule
"#,
            "A g_pcc K=1",
        ),
        // f.pkg_td_imp_ok: iverilog x=1; sv2v x=1; verilator x=1; PRE x=1
        (
            "f.pkg_td_imp_ok",
            r#"package pk;
  localparam int N = 2; localparam logic [7:0] P = 8'h03;
  typedef logic [P[0 +: N]:0] t_t;
endpackage
module top;
  import pk::t_t;
  localparam int N = 0;
  localparam logic [7:0] P = 8'h03;
  t_t x = 1'b1;
  initial #1 $display("A f_ptok x=%0d", x);
  initial #5 $finish;
endmodule
"#,
            "A f_ptok x=1",
        ),
        // g.inline_fn_ok: iverilog o=1; sv2v o=1; verilator o=1; PRE o=1
        (
            "g.inline_fn_ok",
            r#"module top;
  localparam logic [7:0] P = 8'h03;
  localparam int N = 2;
  function logic [P[0 +: N]:0] f(input logic [P[0 +: N]:0] a); f = a; endfunction
  if (1) begin : g
    localparam int N = 0;
    wire [7:0] o = f(4'd1);
    initial #1 $display("A g_inlok o=%0d", o);
  end
  initial #5 $finish;
endmodule
"#,
            "A g_inlok o=1",
        ),
        // g.inline_task_ok: iverilog r=1; sv2v r=1; verilator r=1; PRE r=1
        (
            "g.inline_task_ok",
            r#"module top;
  localparam logic [7:0] P = 8'h03;
  localparam int N = 2;
  task t(output int r); logic [P[0 +: N]:0] v; v = 1'b1; r = v; endtask
  if (1) begin : g
    localparam int N = 0;
    int r;
    initial begin #1 t(r); $display("A g_itok r=%0d", r); end
  end
  initial #5 $finish;
endmodule
"#,
            "A g_itok r=1",
        ),
        // g.td_gfor_ok: iverilog x=1; sv2v x=1; verilator x=1; PRE x=1
        (
            "g.td_gfor_ok",
            r#"module top;
  localparam logic [7:0] P = 8'h03;
  localparam int N = 2;
  typedef logic [P[0 +: N]:0] t_t;
  for (genvar N = 0; N < 1; N++) begin : g
    t_t x = 1'b1;
    initial #1 $display("A g_tdf x=%0d", x);
  end
  initial #5 $finish;
endmodule
"#,
            "A g_tdf x=1",
        ),
        // g.td_genblk_ok: iverilog x=1; sv2v x=1; verilator x=1; PRE x=1
        (
            "g.td_genblk_ok",
            r#"module top;
  localparam logic [7:0] P = 8'h03;
  localparam int N = 2;
  typedef logic [P[0 +: N]:0] t_t;
  if (1) begin : g
    localparam int N = 0;
    t_t x = 1'b1;
    initial #1 $display("A g_tdg x=%0d", x);
  end
  initial #5 $finish;
endmodule
"#,
            "A g_tdg x=1",
        ),
        // f.unitfn_ok: iverilog r=1; sv2v refuses; verilator r=1; PRE r=1
        (
            "f.unitfn_ok",
            r#"localparam int N = 2;
localparam logic [7:0] P = 8'h03;
function automatic int uf(); logic [P[0 +: N]:0] t; t = 1'b1; return t; endfunction
module top #(parameter int N = 0);
  initial #1 $display("A f_unit r=%0d", uf());
  initial #5 $finish;
endmodule
"#,
            "A f_unit r=1",
        ),
        // d.s_pkgfn_loc_unused_p2m0: iverilog r=42; sv2v r=42; verilator r=42; PRE r=42
        (
            "d.s_pkgfn_loc_unused_p2m0",
            r#"package p;
  localparam logic [7:0] P = 8'h03;
  localparam int N = 2;
  function automatic int f(input int a);
    logic [P[0 +: N]:0] t;
    t = a[3:0];
    return a + 1;
  endfunction
endpackage
module top;
  localparam logic [7:0] P = 8'h03;
  localparam int N = 0;
  initial #1 $display("A s r=%0d", p::f(41));
  initial #5 $finish;
endmodule
"#,
            "A s r=42",
        ),
        // d.s_pkgfn_two_callers: iverilog N=4 r=42 N=0 r=42; sv2v N=4 r=42 N=0 r=42; verilator N=0 r=42 N=4 r=42; PRE N=4 r=42 N=0 r=42
        (
            "d.s_pkgfn_two_callers",
            r#"package p;
  localparam logic [7:0] P = 8'h03;
  localparam int N = 2;
  function automatic int f(input int a);
    logic [P[0 +: N]:0] t;
    t = a[3:0];
    return a + 1;
  endfunction
endpackage
module sub #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  initial #1 $display("A s N=%0d r=%0d", N, p::f(41));
endmodule
module top; sub #(.N(4)) a(); sub #(.N(0)) b(); initial #5 $finish; endmodule
"#,
            "A s N=4 r=42\nA s N=0 r=42",
        ),
        // d.s_pkgfn_imp_onlyN_m0: iverilog r=42; sv2v r=42; verilator r=42; PRE r=42
        (
            "d.s_pkgfn_imp_onlyN_m0",
            r#"package p;
  localparam logic [7:0] P = 8'h03;
  localparam int N = 2;
  function automatic int f(input int a);
    logic [P[0 +: N]:0] t;
    t = a[3:0];
    return a + 1;
  endfunction
endpackage
module top;
  import p::*;
  localparam int N = 0;
  initial #1 $display("A s r=%0d", f(41));
  initial #5 $finish;
endmodule
"#,
            "A s r=42",
        ),
        // d.s_pkg_typedef_val1_m0: iverilog x=1; sv2v x=1; verilator x=1; PRE x=1
        (
            "d.s_pkg_typedef_val1_m0",
            r#"package p;
  localparam logic [7:0] P = 8'h03;
  localparam int N = 2;
  typedef logic [P[0 +: N]:0] t;
endpackage
module top;
  localparam logic [7:0] P = 8'h03;
  localparam int N = 0;
  p::t x;
  initial begin x = 1; #1 $display("A s x=%0d", x); end
  initial #5 $finish;
endmodule
"#,
            "A s x=1",
        ),
        // d.s_typedef_gen_shadow_v1: iverilog x=1; sv2v x=1; verilator x=1; PRE x=1
        (
            "d.s_typedef_gen_shadow_v1",
            r#"module m;
  localparam logic [7:0] P = 8'h03;
  localparam int N = 2;
  typedef logic [P[0 +: N]:0] t;
  if (1) begin : g
    localparam int N = 0;
    t x;
    initial begin x = 1; #1 $display("A s x=%0d", x); end
  end
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "A s x=1",
        ),
        // d.s_pkgfn_ret_p2m0_one: iverilog r=1; sv2v r=1; verilator r=1; PRE r=1
        (
            "d.s_pkgfn_ret_p2m0_one",
            r#"package p;
  localparam logic [7:0] P = 8'h03;
  localparam int N = 2;
  function automatic logic [P[0 +: N]:0] f(); return 1; endfunction
endpackage
module top;
  localparam logic [7:0] P = 8'h03;
  localparam int N = 0;
  initial #1 $display("A s r=%0d", p::f());
  initial #5 $finish;
endmodule
"#,
            "A s r=1",
        ),
        // d.s_pkgfn_loc_p2m0_one: iverilog r=1; sv2v r=1; verilator r=1; PRE r=1
        (
            "d.s_pkgfn_loc_p2m0_one",
            r#"package p;
  localparam logic [7:0] P = 8'h03;
  localparam int N = 2;
  function automatic int f(); logic [P[0 +: N]:0] t; t = 1; return t; endfunction
endpackage
module top;
  localparam logic [7:0] P = 8'h03;
  localparam int N = 0;
  initial #1 $display("A s r=%0d", p::f());
  initial #5 $finish;
endmodule
"#,
            "A s r=1",
        ),
        // s3.gen_local_ok: iverilog b=4; sv2v b=4; verilator b=4; PRE b=4
        (
            "s3.gen_local_ok",
            r#"module top;
  localparam logic [7:0] P = 8'h03;
  localparam int N = 0;
  if (1) begin : g
    localparam int N = 2;
    logic [P[0 +: N]:0] x;
    initial #1 $display("A s3e b=%0d", $bits(x));
  end
  initial #5 $finish;
endmodule
"#,
            "A s3e b=4",
        ),
        // s3.gfor_genvar_ok_outer0: iverilog g=1 b=2 g=2 b=4; sv2v g=1 b=2 g=2 b=4; verilator g=1 b=2 g=2 b=4; PRE g=1 b=2 g=2 b=4
        (
            "s3.gfor_genvar_ok_outer0",
            r#"module top;
  localparam logic [7:0] P = 8'h03;
  localparam int g = 0;
  for (genvar g = 1; g < 3; g++) begin : gl
    logic [P[0 +: g]:0] x;
    initial #1 $display("A s3f g=%0d b=%0d", g, $bits(x));
  end
  initial #5 $finish;
endmodule
"#,
            "A s3f g=1 b=2\nA s3f g=2 b=4",
        ),
    ]);
}

/// The declaring scope's own binding is claimed: a package constant, imported or scoped; an
/// enum label; a generate block's own constant read by a declaration in that block; a case
/// arm's; a typedef declared in an enclosing generate block that the inner block does not
/// redeclare; and a genvar read inside its own loop. All three oracles refuse each.
#[test]
fn a_generate_local_constant_or_genvar_is_claimed_in_its_own_text() {
    refused(&[
        // t.imp_const_zero: iverilog refuses; sv2v refuses; verilator refuses; PRE b=1 (a wildcard-imported package constant: no binding of this instance, alike throughout the module)
        (
            "t.imp_const_zero",
            r#"package p;
  localparam int Z = 0;
endpackage
module m;
  import p::*;
  localparam logic [7:0] P = 8'h03;
  logic [P[0 +: Z]:0] x;
  initial #1 $display("A tiz b=%0d", $bits(x));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "7:17",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // t.scoped_const_zero: iverilog refuses; sv2v refuses; verilator refuses; PRE b=1 (a `pkg::` constant)
        (
            "t.scoped_const_zero",
            r#"package p;
  localparam int Z = 0;
endpackage
module m;
  localparam logic [7:0] P = 8'h03;
  logic [P[0 +: p::Z]:0] x;
  initial #1 $display("A tsz b=%0d", $bits(x));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "6:17",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // m.enum_zero_mod: iverilog refuses; sv2v refuses; verilator refuses; PRE x=1 (an enum label: bound in `params`)
        (
            "m.enum_zero_mod",
            r#"module top;
  localparam logic [7:0] P = 8'h03;
  typedef enum int {Z = 0, O = 1} e_t;
  logic [P[0 +: Z]:0] x = 1'b1;
  initial #1 $display("A mez x=%0d", x);
  initial #5 $finish;
endmodule
"#,
            "4:17",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // m.enum_zero_label: iverilog refuses; sv2v refuses; verilator refuses; PRE o=22 (an enum label: bound in `params`)
        (
            "m.enum_zero_label",
            r#"module top;
  localparam logic [7:0] P = 8'hA5;
  typedef enum int {Z = 0, O = 1} e_t;
  wire [7:0] o;
  case (1) P[0 +: Z]: begin : a assign o = 8'h11; end default: begin : d assign o = 8'h22; end endcase
  initial #1 $display("A mezl o=%h", o);
  initial #5 $finish;
endmodule
"#,
            "5:12",
            "E3010",
            "generate-case label is not a constant: the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // d.g_for_g0: iverilog refuses; sv2v refuses; verilator refuses; PRE g=0 b=1 g=1 b=2
        (
            "d.g_for_g0",
            r#"module m;
  localparam logic [7:0] P = 8'h03;
  for (genvar g = 0; g < 2; g++) begin : gl logic [P[0 +: g]:0] x; initial #1 $display("A dgg0 g=%0d b=%0d", g, $bits(x)); end
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:59",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // s3.gen_local_decl: iverilog refuses; sv2v refuses; verilator refuses; PRE b=1
        (
            "s3.gen_local_decl",
            r#"module top;
  localparam logic [7:0] P = 8'h03;
  localparam int N = 2;
  if (1) begin : g
    localparam int N = 0;
    logic [P[0 +: N]:0] x;
    initial #1 $display("A s3a b=%0d", $bits(x));
  end
  initial #5 $finish;
endmodule
"#,
            "6:19",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // s3.gen_local_td_nested: iverilog refuses; sv2v refuses; verilator refuses; PRE b=1
        (
            "s3.gen_local_td_nested",
            r#"module top;
  localparam logic [7:0] P = 8'h03;
  localparam int N = 2;
  if (1) begin : g1
    localparam int N = 0;
    typedef logic [P[0 +: N]:0] t_t;
    if (1) begin : g2
      t_t x;
      initial #1 $display("A s3b b=%0d", $bits(x));
    end
  end
  initial #5 $finish;
endmodule
"#,
            "6:27",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // s3.case_arm_local: iverilog refuses; sv2v refuses; verilator refuses; PRE b=1
        (
            "s3.case_arm_local",
            r#"module top #(parameter int K = 1);
  localparam logic [7:0] P = 8'h03;
  case (K) 1: begin : c localparam int N = 0; logic [P[0 +: N]:0] x; initial #1 $display("A s3g b=%0d", $bits(x)); end default: ; endcase
  initial #5 $finish;
endmodule
"#,
            "3:61",
            "E3009",
            "the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
        // d.k_gcase_g0: iverilog refuses; sv2v refuses; verilator refuses; PRE g=0 o=33 g=1 o=22
        (
            "d.k_gcase_g0",
            r#"module m;
  localparam logic [7:0] P = 8'hA5;
  for (genvar g = 0; g < 2; g++) begin : gl
    wire [7:0] o;
    case (1) P[0 +: g]: begin : a assign o = 8'h22; end default: begin : d assign o = 8'h33; end endcase
    initial #1 $display("A dkg0 g=%0d o=%h", g, o);
  end
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "5:14",
            "E3010",
            "generate-case label is not a constant: the width of this indexed part-select is zero; an indexed part-select width must be a positive constant (IEEE §11.5.1)",
        ),
    ]);
}

/// Review round 1 F2 (ruled): a sized signed width that is negative is refused, as §4.5.601
/// refuses `f[0 +: -4'sd2]` at run time (IEEE §11.5.1: a positive width). iverilog and sv2v
/// read the unsigned bit pattern; verilator accepts `-4'sd1` but refuses `4'shF`, the same
/// 4-bit value, so it answers this axis two ways and is disqualified; there is no oracle.
#[test]
fn a_negative_sized_signed_width_is_refused() {
    refused(&[
        // d.w_s4_neg8: iverilog b=4; sv2v b=4; verilator b=4; PRE b=1
        (
            "d.w_s4_neg8",
            r#"module m;
  localparam logic [31:0] P = 32'h3;
  logic [P[0 +: -4'sd8]:0] x;
  initial #1 $display("A dwws4neg b=%0d", $bits(x));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:17",
            "E3009",
            "the width of this indexed part-select is negative",
        ),
        // d.ng_n4_1_p3: iverilog b=4; sv2v b=4; verilator b=4; PRE b=1
        (
            "d.ng_n4_1_p3",
            r#"module m;
  localparam logic [31:0] P = 32'h3;
  logic [P[0 +: -4'sd1]:0] x;
  initial #1 $display("A ng b=%0d", $bits(x));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "3:17",
            "E3009",
            "the width of this indexed part-select is negative",
        ),
    ]);
}

/// Review round 1 N1: a design an earlier error refused gets no zero claim — an x/z
/// parameter binds a recovery 0, and a refused multi-level `defparam` leaves the child's
/// default in place. A width a call or `$bits` computes is not claimed either. PRE's
/// diagnostics, and only those.
#[test]
fn no_claim_after_an_error_or_through_a_call() {
    loud_as_pre(&[
        // v.decl_range_call: iverilog refuses; sv2v refuses; verilator refuses; PRE 4:10: error[VITA-E3009]
        // (a width a call computes is not claimed: the callee's text binds its own names)
        (
            "v.decl_range_call",
            r#"module m;
  function automatic int w(input int n); return n - 1; endfunction
  localparam logic [7:0] P = 8'h03;
  logic [P[0 +: w(1)]:0] x;
  initial #1 $display("A vdc b=%0d", $bits(x));
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            &["4:10: error[VITA-E3009] E-ELAB-UNSUPPORTED: a function call that does not fold to a constant is not allowed in a constant range bound (the function is undefined, or its body is outside the constant-function subset)"],
        ),
        // t.bits_width_lp2: iverilog refuses; sv2v refuses; verilator refuses; PRE as below
        // (a width `$bits` computes is not claimed: its argument's type may be another scope's text)
        (
            "t.bits_width_lp2",
            r#"module m;
  localparam logic [7:0] P = 8'hA5;
  localparam logic [7:0] Q = 8'h00;
  localparam logic [7:0] X = P[0 +: $bits(Q) - 8];
  initial #1 $display("A tbw2 X=%h", X);
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            &["4:30: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `X` value is not a constant: the part-select `P[…]` has no constant-fold arm"],
        ),
        // e.x_param: iverilog refuses; sv2v refuses; verilator refuses; PRE E3009 parameter `N` value is not a constant: 4'bxxxx h
        (
            "e.x_param",
            r#"module top;
  localparam logic [7:0] P = 8'h03;
  localparam logic [3:0] N = 4'bxxxx;
  logic [P[0 +: N]:0] x;
  initial #1 $display("A ex b=%0d", $bits(x));
  initial #5 $finish;
endmodule
"#,
            &["3:30: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `N` value is not a constant: 4'bxxxx has no constant-fold arm"],
        ),
        // e.x_lpval: iverilog refuses; sv2v refuses; verilator refuses; PRE E3009 parameter `N` value is not a constant: 4'bxxxx h
        (
            "e.x_lpval",
            r#"module top;
  localparam logic [7:0] P = 8'h03;
  localparam logic [3:0] N = 4'bxxxx;
  localparam logic [7:0] X = P[0 +: N];
  initial #1 $display("A exv X=%h", X);
  initial #5 $finish;
endmodule
"#,
            &["3:30: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `N` value is not a constant: 4'bxxxx has no constant-fold arm", "4:30: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `X` value is not a constant: the part-select `P[…]` has no constant-fold arm"],
        ),
        // e.x_gcase: iverilog refuses; sv2v refuses; verilator refuses; PRE E3009 parameter `N` value is not a constant: 4'bxxxx h
        (
            "e.x_gcase",
            r#"module top;
  localparam logic [7:0] P = 8'hA5;
  localparam logic [3:0] N = 4'bxxxx;
  wire [7:0] o;
  case (1) P[0 +: N]: begin : a assign o = 8'h11; end default: begin : d assign o = 8'h22; end endcase
  initial #1 $display("A exg o=%h", o);
  initial #5 $finish;
endmodule
"#,
            &["3:30: error[VITA-E3009] E-ELAB-UNSUPPORTED: parameter `N` value is not a constant: 4'bxxxx has no constant-fold arm"],
        ),
        // a.dp2: iverilog b=4; sv2v b=4; verilator b=4; PRE E3009 defparam: only a direct-child `instance.param` t
        (
            "a.dp2",
            r#"module sub #(parameter int N = 0);
  localparam logic [7:0] P = 8'h03;
  logic [P[0 +: N]:0] x;
  initial #1 $display("A s_dp2 b=%0d", $bits(x));
endmodule
module mid; sub s(); endmodule
module top; mid m(); defparam m.s.N = 2; initial #5 $finish; endmodule
"#,
            &["7:8: error[VITA-E3009] E-ELAB-UNSUPPORTED: defparam: only a direct-child `instance.param` target is supported (a multi-level path is a follow-on)"],
        ),
    ]);
}

/// Runs, and claims no zero or negative width (its value is not pinned: PRE's is wrong too).
fn no_width_claim(cells: &[(&str, &str)]) {
    let mut bad = Vec::new();
    for (name, src) in cells {
        let (out, _, err) = run_raw(src);
        if err.contains("indexed part-select is zero")
            || err.contains("indexed part-select is negative")
        {
            bad.push(format!("{name}: stdout {out:?}\nstderr {err}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n---\n"));
}

/// Review round 2 F2: the fold reads a bare name from `params` alone, so a claim needs that
/// entry to be the name's innermost binding of ANY kind. A generate block's own `real` or
/// `string` constant, a wider-than-64-bit one, or a genvar named like an outer constant is
/// what the text names; the fold walked past the first two to the module's `int R = 0` and
/// round 2 refused designs all three oracles run. An exact-integer `real` is bound in `params`
/// too, on its own key, so the entry the fold reads must not also be another kind's (round 3).
/// Each prints PRE's value.
#[test]
fn an_inner_binding_of_another_kind_keeps_pre() {
    prints(&[
        // q.real_cmp_label: iverilog o=22; sv2v o=22; verilator o=22; PRE o=22
        (
            "q.real_cmp_label",
            r#"module top;
  localparam int R = 0;
  localparam logic [7:0] P = 8'hA5;
  if (1) begin : g
    localparam real R = 1.5;
    wire [7:0] o;
    case (3) P[0 +: ((R > 1) ? 2 : 0)]: begin : a assign o = 8'h11; end default: begin : d assign o = 8'h22; end endcase
    initial #1 $display("A qrl o=%h", o);
  end
  initial #5 $finish;
endmodule
"#,
            "A qrl o=22",
        ),
        // q.real_label_hdr: iverilog o=22; sv2v o=22; verilator o=22; PRE o=22
        (
            "q.real_label_hdr",
            r#"module m #(parameter int R = 0);
  localparam logic [7:0] P = 8'hA5;
  if (1) begin : g
    localparam real R = 1.5;
    wire [7:0] o;
    case (3) P[0 +: ((R > 1) ? 2 : 0)]: begin : a assign o = 8'h11; end default: begin : d assign o = 8'h22; end endcase
    initial #1 $display("A qrlh o=%h", o);
  end
endmodule
module top; m u(); initial #5 $finish; endmodule
"#,
            "A qrlh o=22",
        ),
        // x.exact_real_label: iverilog o=22; sv2v o=22; verilator o=22; PRE o=22 (an exact-integer
        // `localparam real N = 2` is ALSO in `params` on the same key: its integer copy reads
        // `N / 4` as 0, the real reads 0.5 — the same-key half of the rule)
        (
            "x.exact_real_label",
            r#"module top;
  localparam real N = 2;
  localparam logic [7:0] P = 8'hA5;
  wire [7:0] o;
  case (3) P[0 +: ((N / 4 > 0) ? 2 : 0)]: begin : a assign o = 8'h11; end default: begin : d assign o = 8'h22; end endcase
  initial #1 $display("A xerl o=%h", o);
  initial #5 $finish;
endmodule
"#,
            "A xerl o=22",
        ),
        // x.exact_real_label_gen: iverilog o=22; sv2v o=22; verilator o=22; PRE o=22 (an exact-integer
        // `localparam real N = 2` is ALSO in `params` on the same key: its integer copy reads
        // `N / 4` as 0, the real reads 0.5 — the same-key half of the rule)
        (
            "x.exact_real_label_gen",
            r#"module top;
  localparam logic [7:0] P = 8'hA5;
  if (1) begin : g
    localparam real N = 2;
    wire [7:0] o;
    case (3) P[0 +: ((N / 4 > 0) ? 2 : 0)]: begin : a assign o = 8'h11; end default: begin : d assign o = 8'h22; end endcase
    initial #1 $display("A xerg o=%h", o);
  end
  initial #5 $finish;
endmodule
"#,
            "A xerg o=22",
        ),
        // m.str_label_nomatch: iverilog o=22; sv2v o=22; verilator refuses; PRE o=22
        (
            "m.str_label_nomatch",
            r#"module top;
  localparam int N = 0;
  localparam logic [7:0] P = 8'hA5;
  if (1) begin : g
    localparam string N = "\002";
    wire [7:0] o;
    case (3) P[0 +: N]: begin : a assign o = 8'h11; end default: begin : d assign o = 8'h22; end endcase
    initial #1 $display("A msn o=%h", o);
  end
  initial #5 $finish;
endmodule
"#,
            "A msn o=22",
        ),
        // w.wide_label_blk: iverilog o=11; sv2v o=11; verilator o=11; PRE o=11
        (
            "w.wide_label_blk",
            r#"module top;
  localparam int N = 0;
  localparam logic [7:0] P = 8'hA5;
  if (1) begin : g
    localparam logic [99:0] N = 100'd2;
    wire [7:0] o;
    case (1) P[0 +: N]: begin : a assign o = 8'h11; end default: begin : d assign o = 8'h22; end endcase
    initial #1 $display("A wwl o=%h", o);
  end
  initial #5 $finish;
endmodule
"#,
            "A wwl o=11",
        ),
        // w.wide_decl_blk: iverilog x=1 b=4; sv2v x=1 b=4; verilator x=1 b=4; PRE x=1 b=4
        (
            "w.wide_decl_blk",
            r#"module top;
  localparam int N = 0;
  localparam logic [7:0] P = 8'h03;
  if (1) begin : g
    localparam logic [99:0] N = 100'd2;
    logic [P[0 +: N]:0] x = 1'b1;
    initial #1 $display("A wwd x=%0d b=%0d", x, $bits(x));
  end
  initial #5 $finish;
endmodule
"#,
            "A wwd x=1 b=4",
        ),
        // m.genvar_shadow_label: iverilog o=22; sv2v o=22; verilator o=22; PRE o=22
        (
            "m.genvar_shadow_label",
            r#"module top;
  localparam int N = 0;
  localparam logic [7:0] P = 8'hA5;
  for (genvar N = 2; N < 3; N++) begin : gl
    wire [7:0] o;
    case (3) P[0 +: N]: begin : a assign o = 8'h11; end default: begin : d assign o = 8'h22; end endcase
    initial #1 $display("A mgs o=%h", o);
  end
  initial #5 $finish;
endmodule
"#,
            "A mgs o=22",
        ),
    ]);
}

/// …and a net, variable or `real` constant of the generate block that shadows an outer
/// `N = 0` is not read as that zero (all three oracles refuse each design, for the net in a
/// constant expression or the real width; PRE's silent value is not pinned).
#[test]
fn an_inner_net_or_variable_is_not_read_as_an_outer_zero() {
    no_width_claim(&[
        // k.net_shadow: iverilog refuses; sv2v refuses; verilator refuses; PRE x=1
        (
            "k.net_shadow",
            r#"module top;
  localparam int N = 0;
  localparam logic [7:0] P = 8'h03;
  if (1) begin : g
    wire [3:0] N = 4'd2;
    logic [P[0 +: N]:0] x = 1'b1;
    initial #1 $display("A kns x=%0d", x);
  end
  initial #5 $finish;
endmodule
"#,
        ),
        // m.var_shadow_label: iverilog refuses; sv2v refuses; verilator refuses; PRE o=22
        (
            "m.var_shadow_label",
            r#"module top;
  localparam int N = 0;
  localparam logic [7:0] P = 8'hA5;
  if (1) begin : g
    int N = 2;
    wire [7:0] o;
    case (3) P[0 +: N]: begin : a assign o = 8'h11; end default: begin : d assign o = 8'h22; end endcase
    initial #1 $display("A mvs o=%h", o);
  end
  initial #5 $finish;
endmodule
"#,
        ),
        // w.real_label_blk: iverilog refuses; sv2v refuses; verilator refuses; PRE o=22
        (
            "w.real_label_blk",
            r#"module top;
  localparam int N = 0;
  localparam logic [7:0] P = 8'hA5;
  if (1) begin : g
    localparam real N = 2.0;
    wire [7:0] o;
    case (1) P[0 +: N]: begin : a assign o = 8'h11; end default: begin : d assign o = 8'h22; end endcase
    initial #1 $display("A wrl o=%h", o);
  end
  initial #5 $finish;
endmodule
"#,
        ),
    ]);
}
