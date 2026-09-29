//! §3 ⑤ⓐ: a multi-dimensional packed parameter written as a POSITIONAL assignment
//! pattern — `parameter logic [15:0][3:0] PRINCE_SHIFT_ROWS64 = '{4'hF, 4'hA, …}` — is
//! the concatenation of its items, each sized to one element of the first dimension. It
//! was E3009 (`package parameter `PRINCE_SHIFT_ROWS64` value is not a foldable
//! constant`), which stopped the corpus row `ibex` twice in `prim_cipher_pkg` (the
//! permutation and its `_INV` twin).
//!
//! IEEE 1800-2017 §10.9.1: the items are the elements of the first dimension, the left
//! bound's first, and each is assigned to its element; the element at the left bound
//! holds the highest bits whichever way the dimension runs (§7.4.1). The parser declares
//! such a parameter flat, so it rewrites the value to `{W'(e0), …}` (`W` = the element's
//! width, a nested positional pattern recursing over the remaining dimensions, a fill
//! item the sized literal it denotes) — the spelling whose fold the pre-slice binary
//! already got right at every parameter binder — when every item is a literal: sized, or
//! unsized below 2^31 with no `x` / `z`, or `-` of such a decimal. An item that names a
//! constant, computes, casts, calls, or is a string or a real keeps the refusal: the fold
//! behind it is shared code with measured defects this slice does not own (the refusal
//! tests at the end).
//!
//! Oracles: verilator 5.052 (`--binary --timing`) and sv2v 0.0.13 → iverilog 13.0
//! (iverilog alone: "packed array parameters are not supported yet"). sv2v turns a
//! positional pattern into an UNSIZED concatenation, so it is an oracle only where every
//! item already has the element's width; the 4-state cells are sv2v → iverilog on the
//! hand-spelled `{W'(e0), …}`, where verilator zero-extends an `x` sign bit.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_mdpp_{}_{n}", std::process::id()));
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

/// The design's own lines, without vita's warnings and end-of-run lines.
fn prints(src: &str, want: &[&str]) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{out}");
    let got: Vec<&str> = out
        .lines()
        .filter(|l| {
            !l.starts_with("warning[")
                && !l.starts_with("simulation ended")
                && !l.starts_with("errors=")
        })
        .collect();
    assert_eq!(got, want, "{out}");
}

fn is_loud(src: &str, needle: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(1), "expected a refusal\n{out}");
    assert!(out.contains(needle), "expected `{needle}` in\n{out}");
}

/// A module declaring `decl` and printing `$display("V %h", <name>)`.
fn one(decl: &str, name: &str) -> String {
    format!("module t;\n  {decl}\n  initial $display(\"V %h\", {name});\nendmodule\n")
}

// ───────────────────────────── values ─────────────────────────────

/// The corpus row `ibex`, reduced: `prim_cipher_pkg`'s nibble permutation and its
/// inverse, applied by the package's own `prince_shiftrows_64bit`. All three tools:
/// the permuted word, then the original back.
#[test]
fn the_ibex_prince_shift_rows_permutation_folds() {
    prints(
        r#"
package prim_cipher_pkg;
  parameter logic [15:0][3:0] PRINCE_SHIFT_ROWS64 = '{4'hF, 4'hA, 4'h5, 4'h0,
                                                      4'hB, 4'h6, 4'h1, 4'hC,
                                                      4'h7, 4'h2, 4'hD, 4'h8,
                                                      4'h3, 4'hE, 4'h9, 4'h4};
  parameter logic [15:0][3:0] PRINCE_SHIFT_ROWS64_INV = '{4'hF, 4'h2, 4'h5, 4'h8,
                                                          4'hB, 4'hE, 4'h1, 4'h4,
                                                          4'h7, 4'hA, 4'hD, 4'h0,
                                                          4'h3, 4'h6, 4'h9, 4'hC};
  function automatic logic [63:0] prince_shiftrows_64bit(logic [63:0] state_in,
                                                         logic [15:0][3:0] shifts);
    logic [63:0] state_out;
    for (int k = 0; k < 64/4; k++) begin
      state_out[k*4 +: 4] = state_in[shifts[k]*4 +: 4];
    end
    return state_out;
  endfunction
endpackage
module t;
  logic [63:0] d, r, b;
  initial begin
    d = 64'h0123456789abcdef;
    r = prim_cipher_pkg::prince_shiftrows_64bit(d, prim_cipher_pkg::PRINCE_SHIFT_ROWS64);
    b = prim_cipher_pkg::prince_shiftrows_64bit(r, prim_cipher_pkg::PRINCE_SHIFT_ROWS64_INV);
    $display("R1 %h %h", prim_cipher_pkg::PRINCE_SHIFT_ROWS64, prim_cipher_pkg::PRINCE_SHIFT_ROWS64_INV);
    $display("R2 %h %h", r, b);
    $display("R3 %h %h", prim_cipher_pkg::PRINCE_SHIFT_ROWS64[1], prim_cipher_pkg::PRINCE_SHIFT_ROWS64_INV[14]);
  end
endmodule
"#,
        &[
            "R1 fa50b61c72d83e94 f258be147ad0369c",
            "R2 05af49e38d27c16b 0123456789abcdef",
            "R3 9 2",
        ],
    );
}

/// Every parameter binder: `$unit`, a package `parameter` and `localparam` (read
/// scoped, selected, and inside the package's own function), an interface header and
/// body, a module header default (taken and overridden) and body, and a generate
/// block. verilator and sv2v → iverilog.
#[test]
fn every_parameter_binder_reads_the_pattern() {
    prints(
        r#"
parameter logic [3:0][3:0] UP = '{4'hF, 4'hA, 3'h7, -1};
package p;
  parameter logic [3:0][3:0] PP = '{4'hF, 4'hA, 3'h7, -1};
  localparam logic [1:0][7:0] PL = '{8'h12, 8'h34};
  function automatic logic [15:0] pf(input int k);
    return PP + k;
  endfunction
endpackage
interface ifc #(parameter logic [3:0][3:0] IP = '{4'hF, 4'hA, 3'h7, -1}) ();
  localparam logic [1:0][7:0] IL = '{8'h12, 8'h34};
  logic [15:0] iw;
  assign iw = IP ^ IL;
endinterface
module m #(parameter logic [3:0][3:0] HP = '{4'hF, 4'hA, 3'h7, -1},
           parameter logic [1:0][7:0] HQ = '{8'h12, 8'h34})
          (output logic [15:0] o1, output logic [15:0] o2);
  localparam logic [3:0][3:0] BL = '{4'hF, 4'hA, 3'h7, -1};
  parameter logic [1:0][7:0] BP = '{8'h12, 8'h34};
  assign o1 = HP ^ BL;
  assign o2 = HQ ^ BP;
endmodule
module t;
  logic [15:0] a1, a2, b1, b2;
  m u1 (a1, a2);
  m #(.HP(16'h1234)) u2 (b1, b2);
  ifc i1 ();
  for (genvar g = 0; g < 2; g++) begin : gb
    localparam logic [3:0][3:0] GL = '{4'hF, 4'hA, 3'h7, -1};
    logic [15:0] w;
    assign w = GL + g;
  end
  initial begin
    #1;
    $display("U1 %h", UP);
    $display("P1 %h %h %h", p::PP, p::PL, p::PP[1]);
    $display("P2 %h", p::pf(1));
    $display("I1 %h", i1.iw);
    $display("M1 %h %h %h %h", a1, a2, u1.HP, u1.BP);
    $display("M2 %h %h %h", b1, b2, u2.HP);
    $display("G1 %h %h", gb[0].w, gb[1].w);
  end
endmodule
"#,
        &[
            "U1 fa7f",
            "P1 fa7f 1234 7",
            "P2 fa80",
            "I1 e84b",
            "M1 0000 0000 fa7f 1234",
            "M2 e84b 0000 1234",
            "G1 fa7f fa80",
        ],
    );
}

/// An item is sized to its element as an assignment sizes it: truncated, extended by
/// its OWN signedness (`2'sb10` → `e`, `2'b10` → `2`, `-1` → `f`), a fill at the
/// element's width. verilator; sv2v cannot size these items.
#[test]
fn each_item_is_sized_to_its_element() {
    prints(
        r#"
module t;
  localparam logic [3:0][3:0] W1 = '{5'h1F, 3'h7, 1, -1};
  localparam logic [3:0][3:0] W2 = '{2'sb10, 2'b10, 1'sb1, 1'b1};
  localparam logic [3:0][3:0] W3 = '{'1, '0, '1, '0};
  localparam logic [3:0][3:0] W5 = '{16'hFFF3, 32'd9, 8'sh80, 3'sb100};
  initial begin
    $display("W1 %h", W1);
    $display("W2 %h", W2);
    $display("W3 %h", W3);
    $display("W5 %h", W5);
  end
endmodule
"#,
        &["W1 f71f", "W2 e2f1", "W3 f0f0", "W5 390c"],
    );
}

/// The first item is the LEFT bound's element, the highest bits, in either direction
/// and at any right bound; a nested positional pattern is its element's value (down to
/// single bits), beside plain items of the element's width. verilator and sv2v →
/// iverilog.
#[test]
fn dimension_direction_and_nested_patterns() {
    prints(
        r#"
module t;
  localparam logic [0:3][3:0] D1 = '{4'h1, 4'h2, 4'h3, 4'h4};
  localparam logic [3:0][0:3] D2 = '{4'h1, 4'h2, 4'h3, 4'h4};
  localparam logic [4:1][3:0] D3 = '{4'h1, 4'h2, 4'h3, 4'h4};
  localparam logic [1:0][1:0][3:0] D4 = '{'{4'h1, 4'h2}, '{4'h3, 4'h4}};
  localparam logic [1:0][1:0][3:0] D5 = '{8'hAB, 8'hCD};
  localparam logic [1:0][1:0][3:0] D6 = '{'{4'h1, 4'h2}, 8'hCD};
  localparam logic [1:0][3:0] D7 = '{'{1'b1, 1'b0, 1'b1, 1'b0}, '{0, 1, 0, 1}};
  localparam logic [2:0][1:0] D8 = '{2'b01, 2'b10, 2'b11};
  localparam logic [1:0][2:0][1:0] D9 = '{'{2'd1, 2'd2, 2'd3}, '{2'd0, 2'd1, 2'd2}};
  localparam logic [1:0][0:0] DA = '{1'b1, 1'b0};
  localparam logic [0:0][3:0] DB = '{4'h9};
  initial begin
    $display("D1 %h %h %h", D1, D1[0], D1[3]);
    $display("D2 %h %h %b", D2, D2[0], D2[3]);
    $display("D3 %h %h %h", D3, D3[1], D3[4]);
    $display("D4 %h %h %h", D4, D4[1], D4[0][1]);
    $display("D5 %h %h", D5, D5[0][0]);
    $display("D6 %h", D6);
    $display("D7 %h", D7);
    $display("D8 %h", D8);
    $display("D9 %h %h", D9, D9[1][0]);
    $display("DA %b", DA);
    $display("DB %h", DB);
  end
endmodule
"#,
        &[
            "D1 1234 1 4",
            "D2 1234 4 0001",
            "D3 1234 4 1",
            "D4 1234 12 3",
            "D5 abcd d",
            "D6 12cd",
            "D7 a5",
            "D8 1b",
            "D9 6c6 3",
            "DA 10",
            "DB 9",
        ],
    );
}

/// Other declarations of the type: a multi-dimensional typedef (local, signed, a
/// package's), the implicit and `signed` implicit spellings, `reg`, a 100-bit element,
/// and a parenthesised item. verilator and sv2v → iverilog where it can size them.
#[test]
fn typedefs_implicit_signed_and_reg_declarations() {
    prints(
        r#"
package q;
  typedef logic [3:0][7:0] w_t;
  parameter w_t QW = '{8'h11, 8'h22, 8'h33, 8'h44};
endpackage
module t;
  typedef logic [3:0][7:0] w_t;
  typedef logic signed [1:0][3:0] sw_t;
  localparam w_t PW = '{8'h11, 8'h22, 8'h33, 8'h44};
  localparam sw_t PSW = '{4'hF, 4'h0};
  localparam q::w_t PQ = '{8'h01, 8'h02, 8'h03, 8'h04};
  parameter signed [1:0][3:0] S1 = '{-1, 1};
  parameter [1:0][3:0] S2 = '{4'hA, 4'hB};
  localparam reg [1:0][3:0] S3 = '{4'hC, 4'hD};
  localparam logic [1:0][99:0] S4 = '{-1, 100'h5};
  localparam logic [1:0][7:0] S5 = '{(8'h12), (-1)};
  initial begin
    $display("T1 %h %h", PW, PW[1]);
    $display("T5 %h %0d %0d", PSW, PSW, PSW < 0);
    $display("T6 %h %h %h", PQ, PQ[2], q::QW[3]);
    $display("S1 %h %0d %0d", S1, S1, S1 < 0);
    $display("S2 %h %h", S2, S3);
    $display("S4 %h", S4);
    $display("S5 %h", S5);
  end
endmodule
"#,
        &[
            "T1 11223344 33",
            "T5 f0 -16 1",
            "T6 01020304 02 11",
            "S1 f1 -15 1",
            "S2 ab cd",
            "S4 fffffffffffffffffffffffff0000000000000000000000005",
            "S5 12ff",
        ],
    );
}

/// The folded value reaches every constant consumer: a generate condition, a
/// `localparam`, a range bound, an instance override, case items, comparisons and a
/// concatenation. verilator and sv2v → iverilog.
#[test]
fn the_folded_value_reaches_constant_consumers() {
    prints(
        r#"
module c #(parameter logic [7:0] Q = 8'h0) (output logic [7:0] o);
  assign o = Q;
endmodule
module t;
  localparam logic [3:0][3:0] P = '{4'h3, 4'h2, 4'h1, 4'h0};
  localparam int L = P[2] + 1;
  logic [P[3]:0] w;
  logic [7:0] co;
  c #(.Q(P[3:2])) u (co);
  if (P[1] == 4'h1) begin : g1
    initial $display("C1 then");
  end else begin : g2
    initial $display("C1 else");
  end
  initial begin
    w = '1;
    #1;
    $display("C2 %0d %0d %0d", L, $bits(w), $bits(P));
    $display("C3 %h", co);
    case (4'h2)
      P[2]: $display("C4 hit2");
      P[1]: $display("C4 hit1");
      default: $display("C4 none");
    endcase
    $display("C5 %0d %0d", P == 16'h3210, P[0] < P[3]);
    $display("C6 %h", {P, P[1]});
  end
endmodule
"#,
        &[
            "C1 then",
            "C2 3 4 16",
            "C3 32",
            "C4 hit2",
            "C5 1 1",
            "C6 32101",
        ],
    );
}

/// 4-state items in an element wider than 64 bits keep their unknowns: `2'sbx1` is
/// sign-extended with its `x`, `1'bx` / `2'bz1` zero-extended, a fill is the fill at the
/// element's width. The oracle is sv2v → iverilog on the hand-spelled `{64'(e), …}`
/// (verilator zero-extends the `x` sign bit).
#[test]
fn four_state_items_in_a_wide_element() {
    prints(
        r#"
package p;
  parameter logic [1:0][63:0] K3 = '{2'sbx1, 64'h5};
  parameter logic [2:0][63:0] K4 = '{1'bx, 2'bz1, 64'hFFFF_FFFF_FFFF_FFFE};
endpackage
module t;
  localparam logic [1:0][63:0] K5 = '{2'sbx1, 64'h5};
  localparam logic [1:0][63:0] K6 = '{'x, 'z};
  initial $display("K3 %h", p::K3);
  initial $display("K4 %h", p::K4);
  initial $display("K5 %h", K5);
  initial $display("K6 %h", K6);
endmodule
"#,
        &[
            "K3 xxxxxxxxxxxxxxxX0000000000000005",
            "K4 000000000000000X000000000000000Zfffffffffffffffe",
            "K5 xxxxxxxxxxxxxxxX0000000000000005",
            "K6 xxxxxxxxxxxxxxxxzzzzzzzzzzzzzzzz",
        ],
    );
}

/// Unsized literals below 2^31 in elements wider than 32 bits: an unsigned one is
/// zero-extended, a signed or decimal one sign-extended. verilator and sv2v → iverilog.
#[test]
fn unsized_literals_below_two_to_the_31_in_a_wide_element() {
    prints(
        r#"
module t;
  localparam logic [1:0][63:0] C1 = '{'h7FFF_FFFF, 'sh7FFF_FFFF};
  localparam logic [1:0][63:0] C2 = '{'o17777777777, 'd2147483647};
  localparam logic [1:0][63:0] C3 = '{-2147483647, 2147483647};
  localparam logic [1:0][63:0] C4 = '{'sb1, 'sb0111};
  initial begin
    $display("C1 %h", C1);
    $display("C2 %h", C2);
    $display("C3 %h", C3);
    $display("C4 %h", C4);
  end
endmodule
"#,
        &[
            "C1 000000007fffffff000000007fffffff",
            "C2 000000007fffffff000000007fffffff",
            "C3 ffffffff80000001000000007fffffff",
            "C4 00000000000000010000000000000007",
        ],
    );
}

/// A header default that every instance overrides is never folded, so a pattern the
/// rewrite leaves alone (three items for two elements) still runs, as before and as in
/// both oracles; so does a `defparam` over a rewritten default.
#[test]
fn an_overridden_default_is_never_folded() {
    prints(
        r#"
module m #(parameter logic [1:0][3:0] P = '{4'h1, 4'h2, 4'h3}) ();
  initial $display("E4 %h", P);
endmodule
module n #(parameter logic [1:0][3:0] P = '{4'h1, 4'h2}) ();
  initial $display("E7 %h", P);
endmodule
module t;
  m #(.P(8'h12)) u();
  n v();
  defparam v.P = 8'h34;
endmodule
"#,
        &["E4 12", "E7 34"],
    );
}

// ───────────────────────────── refusals ─────────────────────────────

const NOT_CONST: &str = "value is not a constant";

/// An item count that is not the first dimension's element count: both oracles reject
/// the design ("Assignment pattern missed initializing elements", "with too many
/// elements"); vita keeps its refusal.
#[test]
fn a_wrong_item_count_stays_refused() {
    is_loud(
        &one("localparam logic [3:0][3:0] E = '{4'h1, 4'h2};", "E"),
        NOT_CONST,
    );
    is_loud(
        &one(
            "localparam logic [3:0][3:0] E = '{4'h1, 4'h2, 4'h3, 4'h4, 4'h5};",
            "E",
        ),
        NOT_CONST,
    );
}

/// Shapes the rewrite leaves to elaborate's refusal, each measured: a dimension bound
/// that names a constant (verilator `123` / `abc`), a keyed pattern and a keyed nested
/// one (`5555`, `1123`), a one-dimensional parameter (`b`, also iverilog), and a
/// streaming item (`0800`).
#[test]
fn shapes_outside_the_rewrite_stay_refused() {
    is_loud(
        "module t;\n  localparam int N = 3;\n  localparam logic [N-1:0][3:0] E = '{4'h1, 4'h2, 4'h3};\n  initial $display(\"V %h\", E);\nendmodule\n",
        NOT_CONST,
    );
    is_loud(
        "module m #(parameter int N = 2, parameter logic [N-1:0][3:0] P = '{4'h1, 4'h2}) ();\n  initial $display(\"V %h\", P);\nendmodule\nmodule t;\n  m u1();\nendmodule\n",
        NOT_CONST,
    );
    is_loud(
        &one("localparam logic [3:0][3:0] E = '{default: 4'h5};", "E"),
        NOT_CONST,
    );
    is_loud(
        &one(
            "localparam logic [1:0][1:0][3:0] E = '{'{default: 4'h1}, '{4'h2, 4'h3}};",
            "E",
        ),
        NOT_CONST,
    );
    is_loud(
        &one("localparam logic [3:0] E = '{1'b1, 1'b0, 1'b1, 1'b1};", "E"),
        NOT_CONST,
    );
    is_loud(
        &one("localparam logic [1:0][7:0] E = '{{<<{4'h1}}, 8'h0};", "E"),
        NOT_CONST,
    );
}

/// A 2-state (`bit`) element stays refused: the parameter lane keeps no 2-state
/// identity, so an `x` item above 64 bits would stay `x` (the one-dimensional
/// `localparam bit [127:0] B = {64'hx, 64'h1}` prints `x` in vita and verilator, `0` in
/// iverilog, which IEEE 1800-2017 §6.11.3 backs). Its values match verilator (`a115`)
/// once supported; the test pins the refusal until then.
#[test]
fn a_two_state_element_stays_refused() {
    is_loud(
        &one(
            "localparam bit [3:0][3:0] E = '{4'b1010, 2'b01, 1'b1, 4'h5};",
            "E",
        ),
        NOT_CONST,
    );
    is_loud(
        "typedef bit [1:0][3:0] v_t;\nmodule t;\n  localparam v_t E = '{4'h1, 4'h2};\n  initial $display(\"V %h\", E);\nendmodule\n",
        NOT_CONST,
    );
}

/// A constant-function call (verilator `65660914`) and a real (`23e7`), in a module and in
/// a package, are not literals and keep the refusal; an `x` item in an element that fits
/// 64 bits is rewritten and refused by the fold, like the one-dimensional
/// `localparam logic [15:0] P = 16'hx0z5`.
#[test]
fn items_the_fold_cannot_carry_stay_refused() {
    is_loud(
        "module t;\n  function automatic int f(input int k); return k + 100; endfunction\n  localparam logic [1:0][7:0] E = '{f(1), f(2)};\n  initial $display(\"V %h\", E);\nendmodule\n",
        NOT_CONST,
    );
    is_loud(
        &one(
            "localparam logic [3:0][3:0] E = '{1.5, 2.5, -1.5, 7.49};",
            "E",
        ),
        NOT_CONST,
    );
    is_loud(
        "package p;\n  function automatic int pf(input int k); return k * 3; endfunction\n  parameter logic [1:0][7:0] E = '{pf(1), pf(2)};\nendpackage\nmodule t;\n  initial $display(\"V %h\", p::E);\nendmodule\n",
        "package parameter `E` value is not a foldable constant",
    );
    is_loud(
        &one("localparam logic [1:0][3:0] E = '{4'bx01z, 4'h1};", "E"),
        NOT_CONST,
    );
}

/// Unsized literals outside the leaf rule keep the pattern's refusal. `'bx` / `'hz` into a
/// 64-bit element: the fold pads the unknown to 32 bits only (`00000000xxxxxxxx`), where
/// iverilog and sv2v → iverilog pad it to the element (`xxxxxxxxxxxxxxxx`) — wrong in a
/// plain parameter and in a procedural assignment too (docs/PROBE_CATALOG.md).
/// `2147483648`, `4294967295` and `'sd2147483648` are an oracle split: iverilog grows the
/// literal to hold its value, as vita's fold does (`0000000080000000`), verilator reads
/// 32 signed bits (`ffffffff80000000`, `ffffffffffffffff`).
#[test]
fn unsized_literals_outside_the_leaf_rule_stay_refused() {
    is_loud(
        &one("localparam logic [1:0][63:0] E = '{'bx, 'hz};", "E"),
        NOT_CONST,
    );
    is_loud(
        &one(
            "localparam logic [1:0][63:0] E = '{64'h1, 'bz1 | 64'h0};",
            "E",
        ),
        NOT_CONST,
    );
    is_loud(
        &one(
            "localparam logic [1:0][1:0][63:0] E = '{'{'hx, 64'h1}, '{64'h2, 64'h3}};",
            "E",
        ),
        NOT_CONST,
    );
    is_loud(
        &one("localparam logic [1:0][63:0] E = '{2147483648, 1};", "E"),
        NOT_CONST,
    );
    is_loud(
        &one(
            "localparam logic [1:0][63:0] E = '{1, 4294967295 + 0};",
            "E",
        ),
        NOT_CONST,
    );
    is_loud(
        &one("localparam logic [1:0][63:0] E = '{'sd2147483648, 1};", "E"),
        NOT_CONST,
    );
}

/// Items that are not literals keep the refusal, measured each: names (`'{A, B, C, -A}`,
/// verilator `4d1c`), a header default reading the header (`'{A, A + 1}`, verilator `12`
/// and `56`), an instance override carrying `x` into an item — vita's override binder
/// binds `'bx` as 0 (ROADMAP §2 row 15), where verilator and iverilog print
/// `00000000xxxxxxxx0000000000000001` —, operators (`000f`), strings (`6163`), signing
/// casts and a concatenation (`e3f2`), a package constant and a negated sized literal
/// (`14ff`).
#[test]
fn items_other_than_literals_stay_refused() {
    is_loud(
        "module t;\n  localparam int A = 20;\n  localparam logic signed [2:0] B = 3'sb101;\n  localparam C = 5'd17;\n  localparam logic [3:0][3:0] E = '{A, B, C, -A};\n  initial $display(\"V %h\", E);\nendmodule\n",
        NOT_CONST,
    );
    is_loud(
        "module m #(parameter int A = 1, parameter logic [1:0][3:0] P = '{A, A + 1}) ();\n  initial $display(\"V %h\", P);\nendmodule\nmodule t;\n  m #(.A(5)) u();\nendmodule\n",
        NOT_CONST,
    );
    is_loud(
        "module m #(parameter N = 0, parameter logic [1:0][63:0] P = '{N, 64'h1}) ();\n  initial $display(\"V %h\", P);\nendmodule\nmodule t;\n  m #(.N('bx)) u();\nendmodule\n",
        NOT_CONST,
    );
    is_loud(
        &one(
            "localparam logic [3:0][3:0] E = '{4'h1 + 4'hF, 4'hF + 1, 4'h8 << 1, ~4'h0};",
            "E",
        ),
        NOT_CONST,
    );
    is_loud(
        &one("localparam logic [1:0][7:0] E = '{\"a\", \"bc\"};", "E"),
        NOT_CONST,
    );
    is_loud(
        &one(
            "localparam logic [3:0][3:0] E = '{$signed(2'b10), $unsigned(-2'sd1), 2'sb01 * 2'sb11, {1'b1, 1'b0}};",
            "E",
        ),
        NOT_CONST,
    );
    is_loud(
        "package p;\n  localparam int PA = 20;\nendpackage\nmodule t;\n  localparam logic [1:0][7:0] E = '{p::PA, (-8'sd1)};\n  initial $display(\"V %h\", E);\nendmodule\n",
        NOT_CONST,
    );
}

/// `string` takes no packed dimensions: all three oracles reject
/// `localparam string [1:0][3:0] P`, and the pattern spelling keeps its refusal.
#[test]
fn a_string_prefix_keeps_the_refusal() {
    is_loud(
        &one("localparam string [1:0][3:0] E = '{4'h1, 4'h2};", "E"),
        NOT_CONST,
    );
}

/// A pattern as an instance's override actual is not rewritten (the parser does not know
/// the target's type there): verilator `78`, vita refuses the override.
#[test]
fn a_pattern_override_actual_stays_refused() {
    is_loud(
        "module m #(parameter logic [1:0][3:0] P = '{4'h1, 4'h2}) ();\n  initial $display(\"V %h\", P);\nendmodule\nmodule t;\n  m #(.P('{4'h7, 4'h8})) u();\nendmodule\n",
        "the override of parameter `P` is not a constant",
    );
}
