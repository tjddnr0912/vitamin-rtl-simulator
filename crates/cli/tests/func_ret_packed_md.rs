//! §3 ⑤ⓖ: a function whose RETURN type has more than one packed dimension —
//! `typedef logic [N-1:0][W-1:0] t; function t f(…)`, the inline `function logic
//! [A-1:0][B-1:0] f`, and the implicit `function [1:0][3:0] f`. Every cell here was
//! E2002 at parse before this slice, and the corpus row `ibex` stopped on it
//! (`prim_lfsr.sv:392` / `:401`, which the first test reproduces).
//!
//! The return is declared FLAT (`range` is the product range, `ret_packed` keeps the
//! dims) and the function name — the body's return variable, IEEE §13.4.1 — is bound
//! in the parser's multi-dimensional packed rewrite exactly like a multi-dimensional
//! packed formal (§4.5.418). Two shapes the flat twin cannot carry stay loud:
//!
//! - an inner-dimension select on the return variable that is not proven inside its
//!   dimension at parse time — outside it, the flat twin reads a neighbouring
//!   element's bits where IEEE §7.4.6 reads x and a write changes nothing;
//! - a select on a CALL of such a function (`f(x)[1]`, a vita extension): verilator
//!   names the outer element, the flat value would give its bit.
//!
//! Values: iverilog 13.0 and verilator 5.052 unless a comment names the one that ran
//! (iverilog has no streaming operator, rejects a runtime index into a non-final
//! packed dimension, and aborts on some multi-dimensional lvalues).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_frpm_{}_{n}", std::process::id()));
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

fn prints(src: &str, tag: &str, want: &[&str]) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{out}");
    let got: Vec<&str> = out.lines().filter(|l| l.starts_with(tag)).collect();
    assert_eq!(got, want, "{out}");
}

fn is_loud(src: &str, needle: &str) -> String {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(1), "expected a refusal\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in\n{out}");
    out
}

const RET_VAR_SELECT: &str = "for every select after the first, and an index without an \
                              arithmetic, bitwise, shift or conditional operator for the first";
const RET_UNFOLDED: &str = "a return type whose dimension bounds are decimal literals";
const CALL_SELECT: &str = "whose return type has more than one packed dimension, is \
                           unsupported in v1";

// ───────────────────────────── values ─────────────────────────────

#[test]
fn the_prim_lfsr_permutation_matches_both_oracles() {
    // ibex's `prim_lfsr` non-linear output, reduced: a generate-local typedef whose
    // dims follow per-instance parameters, `lrotcol` (a local plus the formal's
    // selects) and `revcol` (a streaming reverse). Instances at 16 and 32 bits, and a
    // third that takes the other branch. verilator, and sv2v 0.0.13 → iverilog 13.0.
    let src = "module perm #(parameter int LfsrDw = 16, parameter bit NonLinearOut = 1)\n\
  (input logic [LfsrDw-1:0] in, output logic [LfsrDw-1:0] out);\n\
  localparam int NumSboxes = LfsrDw / 4;\n\
  localparam int LfsrIdxDw = $clog2(LfsrDw);\n\
  if (NonLinearOut) begin : gen_out_non_linear\n\
    logic [3:0][NumSboxes-1:0][LfsrIdxDw-1:0] matrix_indices, matrix_rotrev_indices;\n\
    typedef logic [NumSboxes-1:0][LfsrIdxDw-1:0] matrix_col_t;\n\
    function automatic matrix_col_t lrotcol(matrix_col_t col, integer shift);\n\
      matrix_col_t out;\n\
      for (int k = 0; k < NumSboxes; k++) out[(k + shift) % NumSboxes] = col[k];\n\
      return out;\n\
    endfunction : lrotcol\n\
    function automatic matrix_col_t revcol(matrix_col_t col);\n\
      return {<<LfsrIdxDw{col}};\n\
    endfunction : revcol\n\
    always_comb for (int k = 0; k < LfsrDw; k++) matrix_indices[k / NumSboxes][k % NumSboxes] = LfsrIdxDw'(k);\n\
    always_comb begin : p_rotrev\n\
      matrix_rotrev_indices[0] = matrix_indices[0];\n\
      matrix_rotrev_indices[1] = lrotcol(matrix_indices[1], NumSboxes/2);\n\
      matrix_rotrev_indices[2] = revcol(matrix_indices[2]);\n\
      matrix_rotrev_indices[3] = revcol(lrotcol(matrix_indices[3], 1));\n\
    end\n\
    always_comb for (int k = 0; k < LfsrDw; k++) out[k] = in[matrix_rotrev_indices[k % 4][k / 4]];\n\
  end else begin : gen_out_passthru\n\
    assign out = in;\n\
  end\n\
endmodule\n\
module tb;\n\
  logic [15:0] i16, o16; logic [31:0] i32, o32; logic [7:0] i8, o8;\n\
  perm #(.LfsrDw(16)) u16 (.in(i16), .out(o16));\n\
  perm #(.LfsrDw(32)) u32 (.in(i32), .out(o32));\n\
  perm #(.LfsrDw(8), .NonLinearOut(0)) u8 (.in(i8), .out(o8));\n\
  initial begin\n\
    i16 = 16'h1234; i32 = 32'h89abcdef; i8 = 8'h5a;\n\
    #1 $display(\"LF o16=%h o32=%h o8=%h\", o16, o32, o8);\n\
    i16 = 16'hbeef; i32 = 32'h0f1e2d3c;\n\
    #1 $display(\"LF o16=%h o32=%h\", o16, o32);\n\
    $finish;\n\
  end\n\
endmodule\n";
    prints(
        src,
        "LF",
        &["LF o16=2f00 o32=ff16b715 o8=5a", "LF o16=bdf7 o32=2edfd120"],
    );
}

#[test]
fn writes_through_the_function_name_and_its_dimension_queries() {
    // The return variable written by element (a runtime OUTER index, a literal inner
    // one) and queried with `$size` / `$bits` — its dims are literals; `rot` and `rev`
    // return a type whose dims are NAMES and never select the return variable (a local,
    // and a streaming reverse). The call widened into a 16-bit variable. verilator
    // (iverilog: the same, except `rev`).
    let src = "module tb;\n\
  localparam int A = 3, B = 4;\n\
  typedef logic [A-1:0][B-1:0] col_t;\n\
  typedef logic [2:0][3:0] lit_t;\n\
  function automatic col_t rot(col_t c, integer sh);\n\
    col_t o;\n\
    for (int k = 0; k < A; k++) o[(k + sh) % A] = c[k];\n\
    return o;\n\
  endfunction\n\
  function automatic logic [2:0][3:0] swp(logic [A-1:0][B-1:0] c);\n\
    for (int k = 0; k < A; k++) swp[k] = c[A-1-k];\n\
    swp[1][0] = ~swp[1][0];\n\
  endfunction\n\
  function automatic lit_t st(col_t c);\n\
    st = c; st[0] = 4'hf; st[2][3:2] = 2'b01;\n\
  endfunction\n\
  function automatic logic [2:0][3:0] q(input int u);\n\
    q = '0; q[0] = $size(q); q[1] = $bits(q); q[2] = $size(q, 2);\n\
  endfunction\n\
  function automatic col_t rev(col_t c);\n\
    return {<<B{c}};\n\
  endfunction\n\
  col_t x; logic [15:0] wide;\n\
  initial begin\n\
    x = 12'h5a3; wide = rot(x, 2);\n\
    $display(\"W rot=%h swp=%h st=%h q=%h rev=%h\", rot(x, 1), swp(x), st(x), q(0), rev(x));\n\
    $display(\"W wide=%h bits=%0d\", wide, $bits(rot(x, 0)));\n\
    $finish;\n\
  end\n\
endmodule\n";
    prints(
        src,
        "W ",
        &[
            "W rot=a35 swp=3b5 st=5af q=4c3 rev=3a5",
            "W wide=035a bits=12",
        ],
    );
}

#[test]
fn every_return_spelling_sign_and_state() {
    // Ascending dims, a signed typedef (sign-extended at the call), 2-state typedef
    // and inline `bit` returns fed x/z, the implicit `[1:0][3:0]` and `signed` forms,
    // and `reg`. Both oracles.
    let src = "module tb;\n\
  typedef logic [0:2][0:3] asc_t;\n\
  typedef logic signed [1:0][3:0] sg_t;\n\
  typedef bit [1:0][3:0] b_t;\n\
  function automatic asc_t fa(asc_t c); fa = c; fa[0] = 4'h9; fa[2][0] = 1'b1; endfunction\n\
  function automatic sg_t fs(logic [7:0] v); fs = v; endfunction\n\
  function automatic b_t fb(logic [7:0] v); fb = v; endfunction\n\
  function automatic [1:0][3:0] fi(input [7:0] v); fi = v; fi[1] = 4'h7; endfunction\n\
  function signed [1:0][3:0] fis(input [7:0] v); fis = v; endfunction\n\
  function automatic reg [1:0][3:0] fr(input [7:0] v); fr = v; fr[0][3] = 1'b0; endfunction\n\
  function automatic bit [1:0][3:0] fbi(input logic [7:0] v); fbi = v; endfunction\n\
  logic [15:0] w; integer n; logic [7:0] bx = 8'b1x0z_0110;\n\
  initial begin\n\
    $display(\"S fa=%h\", fa(12'h5a3));\n\
    w = fs(8'h9c); n = fs(8'h9c);\n\
    $display(\"S fs_ext=%h fs_int=%0d fs_lt=%0d\", w, n, fs(8'h9c) < 0);\n\
    $display(\"S fb=%b fbi=%b\", fb(bx), fbi(bx));\n\
    w = fis(8'h9c);\n\
    $display(\"S fi=%h fis_ext=%h fr=%h\", fi(8'h3c), w, fr(8'h3c));\n\
    $display(\"S bits=%0d %0d %0d\", $bits(fa(0)), $bits(fs(0)), $bits(fi(0)));\n\
    $finish;\n\
  end\n\
endmodule\n";
    prints(
        src,
        "S ",
        &[
            "S fa=9ab",
            "S fs_ext=ff9c fs_int=-100 fs_lt=1",
            "S fb=10000110 fbi=10000110",
            "S fi=7c fis_ext=ff9c fr=34",
            "S bits=12 8 8",
        ],
    );
}

#[test]
fn package_class_interface_and_generate_routes() {
    // A package function called scoped and imported, a package parameter in the dims
    // of a whole-value return (`pw`), a top-level class method, a module-local class
    // method, an interface function, and a function local to a generate block. Both
    // oracles.
    let src = "package pk;\n\
  parameter int N = 3;\n\
  typedef logic [2:0][3:0] pc_t;\n\
  function automatic pc_t prot(pc_t c);\n\
    int j;\n\
    for (int k = 0; k < N; k++) begin j = (k + 1) % N; prot[j] = c[k]; end\n\
  endfunction\n\
  function automatic logic [1:0][2:0] pinl(logic [5:0] v);\n\
    pinl = v; pinl[1][0] = ~pinl[1][0];\n\
  endfunction\n\
  function automatic logic [1:0][N-1:0] pw(logic [2*N-1:0] v); pw = ~v; endfunction\n\
endpackage\n\
interface ifc;\n\
  function automatic logic [1:0][3:0] fi(logic [7:0] v); fi = v; fi[1] = ~fi[1]; endfunction\n\
endinterface\n\
typedef logic [1:0][3:0] cc_t;\n\
class C;\n\
  function cc_t sw(cc_t v); sw[0] = v[1]; sw[1] = v[0]; endfunction\n\
  function automatic logic [1:0][3:0] inl(logic [7:0] v); inl = v; inl[1][3:2] = 2'b01; endfunction\n\
endclass\n\
module tb;\n\
  import pk::*;\n\
  class K;\n\
    function cc_t km(logic [7:0] v); km = v; km[0][3] = 1'b1; endfunction\n\
  endclass\n\
  if (1) begin : gb\n\
    function automatic cc_t gf(logic [7:0] v); gf = v; gf[0] = 4'h9; endfunction\n\
    initial #1 $display(\"R g=%h\", gf(8'h3c));\n\
  end\n\
  ifc i();\n\
  C c; K k;\n\
  initial begin\n\
    c = new; k = new;\n\
    $display(\"R prot=%h pkprot=%h pinl=%h pw=%h\", prot(12'h5a3), pk::prot(12'h123), pinl(6'b101101), pw(6'b101101));\n\
    $display(\"R csw=%h cinl=%h km=%h fi=%h\", c.sw(8'h3c), c.inl(8'h3c), k.km(8'h34), i.fi(8'h3c));\n\
    #2 $finish;\n\
  end\n\
endmodule\n";
    prints(
        src,
        "R ",
        &[
            "R prot=a35 pkprot=231 pinl=25 pw=12",
            "R csw=c3 cinl=7c km=3c fi=cc",
            "R g=39",
        ],
    );
}

#[test]
fn the_constant_function_lane() {
    // Called in a `localparam` and a generate-if condition. verilator (iverilog
    // aborts on the packed localparam). The body shapes the constant interpreter
    // declines stay loud exactly as their one-dimensional twins do.
    let src = "module tb;\n\
  typedef logic [2:0][3:0] col_t;\n\
  typedef logic signed [1:0][3:0] sg_t;\n\
  function col_t stat(col_t c); return c ^ 12'h00f; endfunction\n\
  function automatic col_t nm(col_t c); nm = c; nm[1] = 4'h0; endfunction\n\
  function automatic sg_t sg(logic [7:0] v); return v; endfunction\n\
  localparam col_t P3 = stat(12'h5a3);\n\
  localparam logic [3:0] E3 = P3[0];\n\
  localparam col_t P4 = nm(12'h5a3);\n\
  localparam int S1 = sg(8'hf0);\n\
  localparam logic [15:0] S2 = sg(8'hf0);\n\
  if (stat(12'h5a3) == 12'h5ac) begin : g_t initial $display(\"K gen-if taken\"); end\n\
  initial begin #1 $display(\"K P3=%h E3=%h P4=%h S1=%0d S2=%h\", P3, E3, P4, S1, S2); $finish; end\n\
endmodule\n";
    prints(
        src,
        "K ",
        &["K gen-if taken", "K P3=5ac E3=c P4=503 S1=-16 S2=fff0"],
    );
}

#[test]
fn outer_runtime_index_recursion_non_ansi_and_three_dimensions() {
    // An outer index out of range writes nothing (3 and -1), a recursive call, the
    // non-ANSI formal spelling, a `pkg::t` return, three packed dimensions, and a
    // continuous assign of a call. Both oracles.
    let src = "package pk;\n  typedef logic [1:0][3:0] pc_t;\nendpackage\n\
module tb;\n\
  typedef logic [2:0][3:0] col_t;\n\
  typedef logic [1:0][2:0][3:0] c3_t;\n\
  function automatic col_t d0(col_t c, int k); d0 = c; d0[k] = 4'hf; endfunction\n\
  function automatic col_t rec(col_t c, int n);\n\
    if (n == 0) return c;\n\
    return rec({c[1:0], c[2]}, n - 1);\n\
  endfunction\n\
  function automatic col_t nonansi;\n\
    input col_t c;\n\
    nonansi = c; nonansi[2] = 4'h0;\n\
  endfunction\n\
  function automatic pk::pc_t fp(logic [7:0] v); fp = v; fp[1] = ~fp[1]; endfunction\n\
  function automatic c3_t f3(c3_t c);\n\
    f3 = c; f3[1][2] = 4'h7; f3[0][1][3] = 1'b1; f3[1][0][1:0] = 2'b10;\n\
  endfunction\n\
  col_t w; assign w = d0(12'h5a3, 1);\n\
  initial begin\n\
    #1;\n\
    $display(\"D d0=%h %h %h rec=%h\", d0(12'h5a3, 0), d0(12'h5a3, 3), d0(12'h5a3, -1), rec(12'h5a3, 2));\n\
    $display(\"D nonansi=%h fp=%h f3=%h w=%h\", nonansi(12'h5a3), fp(8'h3c), f3(24'h123456), w);\n\
    $finish;\n\
  end\n\
endmodule\n";
    prints(
        src,
        "D ",
        &[
            "D d0=5af 5a3 5a3 rec=35a",
            "D nonansi=0a3 fp=cc f3=7224d6 w=5f3",
        ],
    );
}

#[test]
fn the_inner_selects_a_parse_time_proof_admits() {
    // A literal inner index, part and indexed part inside the dimension, an outer
    // runtime index beside them, and an outer part that runs off the vector (its
    // in-range element is written). verilator (iverilog rejects a runtime index into a
    // non-final packed dimension).
    let src = "module tb;\n\
  function automatic logic [2:0][3:0] a(int i); a = 12'h5a3; a[i][0] = 1'b1; endfunction\n\
  function automatic logic [2:0][3:0] b(int i); b = 12'h5a3; b[1][3:0] = 4'h1; endfunction\n\
  function automatic logic [2:0][3:0] c(int i); c = 12'h5a3; c[1][0 +: 4] = 4'h1; c[2][3 -: 2] = 2'b11; endfunction\n\
  function automatic logic [2:0][3:0] d(int i); d = 12'h5a3; d[1][2] = 1'b1; d[i][1] = 1'b0; endfunction\n\
  function automatic logic [2:0][3:0] e(int i); e = 12'h5a3; e[i +: 2] = 8'h00; e[2][1] = e[0][0]; endfunction\n\
  initial begin\n\
    $display(\"I a=%h b=%h c=%h d=%h e=%h\", a(1), b(0), c(0), d(0), e(1));\n\
    $display(\"I a3=%h d3=%h e2=%h\", a(3), d(3), e(2));\n\
    $finish;\n\
  end\n\
endmodule\n";
    prints(
        src,
        "I ",
        &["I a=5b3 b=513 c=d13 d=5e1 e=203", "I a3=5a3 d3=5e3 e2=2a3"],
    );
}

#[test]
fn a_block_local_named_like_the_function_shadows_the_return_variable() {
    // Inside `blk` the name is the 8-bit local; the chain there is NOT rewritten
    // against the return type's dims. Both oracles.
    let src = "module tb;\n\
  typedef logic [2:0][3:0] col_t;\n\
  function automatic col_t f(col_t c);\n\
    logic [7:0] t;\n\
    begin : blk\n\
      logic [7:0] f;\n\
      f = 8'h81; f[1] = 1'b1; f[6:5] = 2'b11; t = f;\n\
    end\n\
    return {c[2], t};\n\
  endfunction\n\
  initial begin $display(\"B f=%h\", f(12'h5a3)); $finish; end\n\
endmodule\n";
    prints(src, "B ", &["B f=5e3"]);
}

// ───────────────────────────── refusals ─────────────────────────────

#[test]
fn a_select_the_flat_twin_cannot_carry_on_the_return_variable_is_loud() {
    // An inner-dimension select outside its dimension reads a neighbouring element's
    // bits in the flat twin (the formal twin of this select does, PROBE_CATALOG), a
    // name there can read a loop variable the parse-time fold cannot see (`for (int L
    // …)` beside `localparam int L`), and an operator in the first index is evaluated
    // at the offset arithmetic's 32 bits instead of its own width (`f[q + 1'b1]` with
    // `q = 2'd3` is element 0 for both oracles).
    for sel in [
        "f[1][j] = 1'b1;",
        "f[1][4] = 1'b1;",
        "f[1][-1] = 1'b1;",
        "f[1][L] = 1'b1;",
        "f[1][L-1] = 1'b1;",
        "f[1][2'd1] = 1'b1;",
        "f[1][5:4] = 2'b11;",
        "f[1][L:0] = 3'b111;",
        "f[1][3 +: 2] = 2'b11;",
        "f[1][j +: 1] = 1'b1;",
        "f[q + 1'b1] = 4'hf;",
        "f[j - 1] = 4'hf;",
        "f[1'b1 ? j : 0] = 4'hf;",
        "f[j +: L] = 8'hff;",
        "f[L:0] = 12'h0;",
        "f['1] = 4'hf;",
        "f[0] = f['1];",
        "f[0] = {3'b000, f[1][j]};",
    ] {
        let src = format!(
            "module tb;\n  localparam int L = 2;\n  logic [1:0] q = 2'd3;\n  \
             function automatic logic [2:0][3:0] f(int j);\n    f = 12'h5a3;\n    {sel}\n  \
             endfunction\n  initial begin $display(\"r=%h\", f(2)); $finish; end\nendmodule\n"
        );
        is_loud(&src, RET_VAR_SELECT);
    }
    // A dimension bound that is a NAME is re-evaluated at every select, where a local,
    // loop variable, class parameter or enum label of that name would win, and the
    // parser's constant table does not follow all of them: every select and dimension
    // query on that return variable is refused — an overridable parameter, a
    // `localparam`, a package constant alike.
    for (decl, dim) in [
        ("parameter int N = 4", "N-1"),
        ("parameter int N = 4; localparam int L = 4", "L-1"),
        ("parameter int N = 4; localparam int L = 3", "L"),
    ] {
        for body in ["f[1] = 4'hf;", "f[1][2] = 1'b1;", "f[0] = $size(f);"] {
            let src = format!(
                "module m #({decl});\n  function automatic logic [2:0][{dim}:0] f(int i);\n    \
                 f = 0; {body}\n  endfunction\n  initial $display(\"r=%h\", f(0));\nendmodule\n\
                 module tb; m u(); initial #1 $finish; endmodule\n"
            );
            is_loud(&src, RET_UNFOLDED);
        }
    }
    // …while the whole return variable still works there (both oracles `3c`).
    prints(
        "module m #(parameter int N = 4);\n  function automatic logic [1:0][N-1:0] f(logic [7:0] v);\n    \
         f = v;\n  endfunction\n  initial $display(\"U f=%h\", f(8'h3c));\nendmodule\n\
         module tb; m u(); initial #1 $finish; endmodule\n",
        "U ",
        &["U f=3c"],
    );
}

#[test]
fn a_select_on_a_call_is_loud_on_every_route() {
    // verilator names the outer ELEMENT (`f(12'h5a3)[1]` is `a`); the flat value's
    // bit 1 is `1`. iverilog rejects every select on a call. A `let` over the call (with
    // arguments, without, or choosing between two calls with `?:`), or the call passed
    // through a `let`'s formal, selects from the same flat value.
    let src = "package pk;\n\
  typedef logic [2:0][3:0] pc_t;\n\
  function automatic pc_t pf(pc_t v); return v; endfunction\n\
endpackage\n\
module sub;\n\
  typedef logic [2:0][3:0] sc_t;\n\
  function automatic sc_t hf(sc_t v); return v; endfunction\n\
endmodule\n\
interface ifc;\n\
  function automatic logic [1:0][3:0] fi(logic [7:0] v); return v; endfunction\n\
endinterface\n\
typedef logic [2:0][3:0] col_t;\n\
class C;\n\
  function col_t m(col_t v); return v; endfunction\n\
endclass\n\
module tb;\n\
  import pk::*;\n\
  class K;\n\
    function col_t km(col_t v); return v; endfunction\n\
  endclass\n\
  function automatic col_t f(col_t v); return v; endfunction\n\
  function automatic logic [3:0] g(col_t v); return f(v)[1]; endfunction\n\
  let lf(x) = f(x);\n\
  let id(x) = x;\n\
  let lt(c, x) = c ? f(x) : f(x);\n\
  let g0 = f(12'h5a3);\n\
  if (1) begin : gb\n\
    function automatic col_t gf(col_t v); return v; endfunction\n\
    initial #1 $display(\"g=%h\", gf(12'h5a3)[1]);\n\
  end\n\
  sub u();\n\
  ifc i();\n\
  C c; K k;\n\
  initial begin\n\
    c = new; k = new;\n\
    $display(\"a=%h\", f(12'h5a3)[1]);\n\
    $display(\"b=%h\", pk::pf(12'h5a3)[1]);\n\
    $display(\"c=%h\", pf(12'h5a3)[1]);\n\
    $display(\"d=%h\", c.m(12'h5a3)[1]);\n\
    $display(\"e=%h\", u.hf(12'h5a3)[1]);\n\
    $display(\"h=%h\", k.km(12'h5a3)[1]);\n\
    $display(\"i=%h\", i.fi(8'h3c)[1]);\n\
    $display(\"j=%h %h\", f(12'h5a3)[1:0], f(12'h5a3)[0 +: 2]);\n\
    $display(\"p=%h\", (f(12'h5a3))[1]);\n\
    $display(\"r=%h\", g(12'h5a3));\n\
    $display(\"l=%h %h %h %h\", lf(12'h5a3)[1], id(f(12'h5a3))[1], lt(1'b1, 12'h5a3)[1], g0[1]);\n\
    #2 $finish;\n\
  end\n\
endmodule\n";
    let out = is_loud(src, CALL_SELECT);
    for callee in ["`f`", "`pf`", "`m`", "`hf`", "`km`", "`fi`", "`gf`"] {
        let n = out
            .lines()
            .filter(|l| l.contains(CALL_SELECT) && l.contains(&format!("call to {callee}")))
            .count();
        assert!(n > 0, "no refusal for {callee}\n{out}");
    }
    // a, b, c, d, e, h, i, j×2, p, g's body, four `let`s, and the generate block.
    assert_eq!(
        out.lines().filter(|l| l.contains(CALL_SELECT)).count(),
        16,
        "{out}"
    );
}

#[test]
fn a_select_through_a_chain_of_lets_is_loud() {
    // Seventeen lets, each wrapping the next, the last over the call (verilator `2`,
    // the element; the flat value's bit is `0`). The expansion has no depth bound, so
    // neither has the refusal.
    let mut lets = String::from("  let l0(x) = f(x);\n");
    for k in 1..17 {
        lets.push_str(&format!("  let l{k}(x) = l{}(x);\n", k - 1));
    }
    let src = format!(
        "module tb;\n  function automatic logic [1:0][3:0] f(logic [7:0] v); return v; endfunction\n\
         {lets}  initial begin $display(\"c=%h\", l16(8'h21)[1]); $finish; end\nendmodule\n"
    );
    is_loud(&src, CALL_SELECT);
}

#[test]
fn the_call_refusal_is_keyed_on_the_name() {
    // A one-dimensional `f` elsewhere in the design shares the refusal of the
    // multi-dimensional `f` in `other`: the callee is not resolved at the select.
    // Every design with such a return was a parse error before, so this refuses
    // nothing that ran.
    is_loud(
        "module other;\n  function automatic logic [1:0][3:0] f(logic [7:0] v); return v; endfunction\nendmodule\n\
         module tb;\n  function automatic logic [11:0] f(logic [11:0] v); return v; endfunction\n  \
         other o();\n  initial begin $display(\"a=%h\", f(12'h5a3)[7:4]); $finish; end\nendmodule\n",
        CALL_SELECT,
    );
    // The control: with no multi-dimensional return in the design, the same select
    // keeps its value (verilator `a`; the W2004 warning says it is not portable).
    prints(
        "module tb;\n  function automatic logic [11:0] f(logic [11:0] v); return v; endfunction\n  \
         initial begin $display(\"N a=%h\", f(12'h5a3)[7:4]); $finish; end\nendmodule\n",
        "N ",
        &["N a=a"],
    );
}

#[test]
fn an_unpacked_array_typedef_return_stays_loud() {
    // The return would be one element. iverilog rejects the shape; verilator
    // returns the array (`22`).
    is_loud(
        "module tb;\n  typedef logic [7:0] a_t [0:1];\n  function a_t f(input int u); \
         f[0] = 8'h11; f[1] = 8'h22; endfunction\n  initial begin a_t v; v = f(0); \
         $display(\"%h\", v[1]); $finish; end\nendmodule\n",
        "a function return type other than an unpacked-array typedef",
    );
}
