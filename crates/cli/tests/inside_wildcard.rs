//! `inside` (IEEE 1800-2017 §11.4.13) compares each VALUE element with wildcard equality
//! `==?` (§11.4.6): the element's x/z/? bits are don't-cares, the left operand's are
//! not. ROADMAP §2 🆕 S — an external report (`4'b1100 inside {4'b1?00}` printed `x`,
//! `if` took the else branch) reproduced at 00c3d76d.
//!
//! The parser emits `BinOp::InsideEq` per value element; elaborate's `inside_value_cmp`
//! builds the `==?` compare for a constant element with x/z bits, refuses a compound
//! x/z element, and keeps `==` for everything else (byte-identical IR). Constant
//! contexts read one wildcard routine in both constant domains (`const_wildcard_i64`,
//! `const_wide::fold_region`), which the `==?` operator shares.
//!
//! Oracles, recorded per test: verilator 5.052 (2-state cells only — it reads an x/z
//! sign bit as 0 and refuses a 4-state non-constant element), sv2v 0.0.13 → iverilog
//! 13.0 (4-state; it spells a constant x/z element `(v | wild) == pattern` and a
//! variable one as a runtime formula, and it crashes on a fill element), and iverilog
//! 13.0's own `==?` on the single-element twin of a cell (iverilog itself refuses
//! `inside`). Census: s580 REPORT (§2 🆕 S).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Run {
    out: String,
    err: String,
    code: i32,
}

impl Run {
    /// The `$display` lines, without the simulator's trailer.
    fn lines(&self) -> Vec<&str> {
        self.out
            .lines()
            .filter(|l| !l.starts_with("simulation ended"))
            .collect()
    }
}

fn run(src: &str, args: &[&str]) -> Run {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_inside_wild_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    Run {
        out: String::from_utf8_lossy(&out.stdout).into_owned(),
        err: String::from_utf8_lossy(&out.stderr).into_owned(),
        code: out.status.code().unwrap_or(-1),
    }
}

const F01: &str = r#"`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v = 4'b1100; $display("a %b", v inside {4'b1100});
    v = 4'b1100; $display("b %b", v inside {4'b1?00});
    v = 4'b1000; $display("c %b", v inside {4'b1?00});
    v = 4'b0100; $display("d %b", v inside {4'b1?00});
    v = 4'b1100; $display("e %b", v inside {4'b0000, 4'b1?00});
    v = 4'b0101; $display("f %b", v inside {[4'd1:4'd7]});
    v = 4'b1100; if (v inside {4'b1?00}) $display("g then"); else $display("g else");
    v = 4'b1100; $display("h %b", v ==? 4'b1?00);
    v = 4'bx100; $display("i %b", v inside {4'b1?00});
    $finish;
  end
endmodule
"#;
const F04: &str = r#"`timescale 1ns/1ns
module t;
  logic signed [7:0] s8; logic [7:0] u8; logic signed [3:0] s4; integer i32;
  function automatic logic signed [7:0] g8(); return -8'sd4; endfunction
  function automatic logic signed [3:0] g4(); return -4'sd4; endfunction
  initial begin
    s8 = -8'sd4;      $display("ss-narrow-pat-msb1 %b", s8 inside {4'sb1?00});
    s8 = -8'sd4;      $display("ss-narrow-pat-msbq %b", s8 inside {4'sb?100});
    s8 = 8'sb01010100; $display("ss-narrow-pat-msbq-pos %b", s8 inside {4'sb?100});
    s8 = -8'sd4;      $display("ss-narrow-pat-msbx %b", s8 inside {4'sbx100});
    s8 = 8'sb01010100; $display("ss-narrow-pat-msbz %b", s8 inside {4'sbz100});
    s8 = 8'sb00001100; $display("ss-narrow-pat-ext-ones %b", s8 inside {4'sb1?00});
    s4 = -4'sd4;      $display("ss-narrow-lhs-sext %b", s4 inside {8'sb11111?00});
    s4 = -4'sd4;      $display("ss-narrow-lhs-zero-pat %b", s4 inside {8'sb00001?00});
    s4 = -4'sd4;      $display("ss-narrow-lhs-wild-msb %b", s4 inside {8'sb?1111100});
    i32 = -4;         $display("ss-integer %b", i32 inside {4'sb1?00});
    s8 = -8'sd4;      $display("su-pattern-unsigned %b", s8 inside {4'b1?00});
    s4 = -4'sd4;      $display("su-wide-unsigned-pat %b", s4 inside {8'b11111?00});
    u8 = 8'b11111100; $display("us-lhs-unsigned %b", u8 inside {4'sb1?00});
    s8 = 8'sb00001100; $display("su-zero-ext-hit %b", s8 inside {4'b1?00});
    $display("call-lhs %b", g8() inside {4'sb1?00});
    $display("call-lhs-narrow %b", g4() inside {8'sb1111_1?00});
    s8 = -8'sd4;      $display("select-lhs-unsigned %b", s8[3:0] inside {8'sb1111_1?00});
    s8 = -8'sd4;      $display("signed-select-lhs %b", $signed(s8[3:0]) inside {8'sb1111_1?00});
    $finish;
  end
endmodule
"#;
const F05: &str = r#"`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v = 4'b1x00; $display("lhs-x-at-wild %b", v inside {4'b1?00});
    v = 4'b1z00; $display("lhs-z-at-wild %b", v inside {4'b1?00});
    v = 4'bx100; $display("lhs-x-compared %b", v inside {4'b1?00});
    v = 4'bz100; $display("lhs-z-compared %b", v inside {4'b1?00});
    v = 4'bx100; $display("lhs-x-under-msb-wild %b", v inside {4'b?100});
    v = 4'b1x00; $display("lhs-x-under-x-digit %b", v inside {4'b1x00});
    v = 4'bxxxx; $display("lhs-all-x-all-wild %b", v inside {4'b????});
    v = 4'bzzzz; $display("lhs-all-z-all-z %b", v inside {4'bzzzz});
    v = 4'bx110; $display("lhs-x-definite-miss %b", v inside {4'b0?00});
    v = 4'bx100; $display("lhs-x-plain-element %b", v inside {4'b0100});
    $finish;
  end
endmodule
"#;
const F06: &str = r#"`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v = 4'b0101; $display("wild-then-range %b", v inside {4'b1?00, [4'd1:4'd7]});
    v = 4'b1000; $display("range-then-wild %b", v inside {[4'd1:4'd7], 4'b1?00});
    v = 4'b0011; $display("neither %b", v inside {[4'd1:4'd2], 4'b1?00});
    v = 4'b1100; $display("not-hit %b", !(v inside {4'b1?00}));
    v = 4'b0100; $display("not-miss %b", !(v inside {4'b1?00}));
    v = 4'b0110; $display("third-element %b", v inside {4'b0000, 4'b0001, 4'b01?0});
    v = 4'b0111; $display("all-miss %b", v inside {4'b01?0, 4'b0000});
    v = 4'bx100; $display("miss-or-x %b", v inside {4'b0000, 4'b1?00});
    v = 4'bx100; $display("one-or-x %b", v inside {4'b?100, 4'b1?00});
    v = 4'bx100; $display("x-or-miss %b", v inside {4'b1?00, 4'b0110});
    v = 4'bx100; $display("not-x %b", !(v inside {4'b1?00}));
    v = 4'b1100; $display("fill-x-element %b", v inside {4'b0000, 'x});
    v = 4'b1100; $display("concat-of-insides %b", {v inside {4'b1?00}, v inside {4'b0?00}});
    v = 4'b0100; $display("arith %0d", (v inside {4'b0?00}) + 2'd1);
    $finish;
  end
endmodule
"#;
const F09: &str = r#"`timescale 1ns/1ns
module t;
  logic [3:0] v; logic m; logic [3:0] r;
  wire w1; wire [7:0] w8; wire [3:0] y;
  assign w1 = v inside {4'b1?00};
  assign w8 = v inside {4'b1?00};
  assign y = (v inside {4'b1?00}) ? 4'd1 : 4'd2;
  always_comb m = v inside {4'b1?00};
  always @* r = (v inside {4'b1?00}) ? 4'd7 : 4'd3;
  initial begin
    v = 4'b1100; #1 $display("cont %b %b %h comb %b star %0d", w1, w8, y, m, r);
    v = 4'b0100; #1 $display("cont %b %b %h comb %b star %0d", w1, w8, y, m, r);
    $finish;
  end
endmodule
"#;
const F10: &str = r#"`timescale 1ns/1ns
package p; function automatic logic pf(input logic [3:0] a); return a inside {4'b1?00}; endfunction endpackage
module t;
  logic [3:0] v; logic r; wire wf; wire [7:0] w8;
  function automatic logic fa(input logic [3:0] a); return a inside {4'b1?00}; endfunction
  function logic fs(input logic [3:0] a); fs = a inside {4'b1?00}; endfunction
  function logic fsx(input logic [3:0] a); fsx = a inside {4'b1?00, 'x}; endfunction
  task automatic tk(input logic [3:0] a, output logic o); o = a inside {4'b1?00}; endtask
  assign wf = fa(v);
  assign w8 = fsx(v) + fs(v);
  initial begin
    v = 4'b1100; tk(v, r); #1
    $display("frame %b inlined %b task %b pkg %b cont-frame %b cont-inlined %h", fa(v), fs(v), r, p::pf(v), wf, w8);
    v = 4'b0110; tk(v, r); #1
    $display("frame %b inlined %b task %b pkg %b cont-frame %b cont-inlined %h", fa(v), fs(v), r, p::pf(v), wf, w8);
    $finish;
  end
endmodule
"#;

/// The external report's cells a–i. verilator 5.052: a1 b1 c1 d0 e1 f1 g then h1 (i is
/// 2-state-blind); sv2v 0.0.13 → iverilog 13.0: the same plus `i x`. iverilog 13.0 itself
/// refuses `inside` (`sorry: "inside" expressions not supported yet.`).
/// PRE (00c3d76d) printed: `a 1`, `b x`, `c x`, `d 0`, `e x`, `f 1`, `g else`, `h 1`, `i x`.
#[test]
fn report_r2_cells() {
    let r = run(F01, &[]);
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        ["a 1", "b 1", "c 1", "d 0", "e 1", "f 1", "g then", "h 1", "i x",],
        "stdout:\n{}",
        r.out
    );
}

/// An x, z or ? digit at any position of the element — MSB and LSB included, in binary,
/// hex, decimal and octal — is a don't-care. A narrower element zero-extends, so its
/// extension bits are COMPARED (`narrow-ext-compared`, `wide-one-hi`). verilator 5.052
/// and sv2v → iverilog 13.0 print every value here; so does iverilog's own `==?` on the
/// single-element twin of each cell.
/// PRE (00c3d76d) printed: `x-digit x`, `z-digit x`, `msb-q x`, `msb-x x`, `msb-z x`, `lsb-q x`, `all-q x`, `hex-q x`, `dec-x x`, `oct-q x`, `narrow-hit x`, `narrow-ext-compared 0`, `wide-zero-hi x`, `wide-one-hi 0`, `wide-wild-hi x`.
#[test]
fn element_wildcard_digit_positions_and_radices() {
    let r = run(
        r#"`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v = 4'b1100; $display("x-digit %b", v inside {4'b1x00});
    v = 4'b1100; $display("z-digit %b", v inside {4'b1z00});
    v = 4'b0100; $display("msb-q %b", v inside {4'b?100});
    v = 4'b1100; $display("msb-x %b", v inside {4'bx100});
    v = 4'b0100; $display("msb-z %b", v inside {4'bz100});
    v = 4'b1101; $display("lsb-q %b", v inside {4'b110?});
    v = 4'b1010; $display("all-q %b", v inside {4'b????});
    v = 4'b1100; $display("hex-q %b", v inside {4'h?});
    v = 4'b1100; $display("dec-x %b", v inside {4'dx});
    v = 4'b1100; $display("oct-q %b", v inside {4'o1?});
    v = 4'b0110; $display("narrow-hit %b", v inside {3'b1?0});
    v = 4'b1100; $display("narrow-ext-compared %b", v inside {3'b1?0});
    v = 4'b1100; $display("wide-zero-hi %b", v inside {6'b001?00});
    v = 4'b1100; $display("wide-one-hi %b", v inside {6'b101?00});
    v = 4'b1100; $display("wide-wild-hi %b", v inside {6'b??1?00});
    $finish;
  end
endmodule
"#,
        &[],
    );
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        [
            "x-digit 1",
            "z-digit 1",
            "msb-q 1",
            "msb-x 1",
            "msb-z 1",
            "lsb-q 1",
            "all-q 1",
            "hex-q 1",
            "dec-x 1",
            "oct-q 1",
            "narrow-hit 1",
            "narrow-ext-compared 0",
            "wide-zero-hi 1",
            "wide-one-hi 0",
            "wide-wild-hi 1",
        ],
        "stdout:\n{}",
        r.out
    );
}

/// The mask spans the comparison width at 33, 64, 65 and 128 bits (one and several
/// words), a 4-bit element against a wide operand compares the operand's high bits to 0,
/// and a SIZED element whose MSB is x zero-extends (only an unsized literal pads with
/// x, see `unsized_and_fill_elements`). verilator 5.052 and sv2v → iverilog 13.0 agree
/// on every line.
/// PRE (00c3d76d) printed: `w33-hit x`, `w33-miss 0`, `w33-narrow-el x`, `w64-hit x`, `w64-miss 0`, `w64-narrow-el x`, `w65-hit x`, `w65-miss 0`, `w65-narrow-el x`, `w128-hit x`, `w128-miss 0`, `w128-narrow-el x`, `sized-x-msb-zero-ext x`, `sized-x-msb-high-compared 0`.
#[test]
fn element_and_left_operand_width_ladder() {
    let r = run(
        r#"`timescale 1ns/1ns
module t;
  logic [32:0] u33; logic [63:0] u64; logic [64:0] u65; logic [127:0] u128; logic [7:0] u8;
  initial begin
    u33 = 33'b011111111111111111111111111111110; $display("w33-hit %b", u33 inside {33'b?1111111111111111111111111111111?});
    u33 = 33'b111111111111111111111111111111101; $display("w33-miss %b", u33 inside {33'b?1111111111111111111111111111111?});
    u33 = 33'b1011; $display("w33-narrow-el %b", u33 inside {4'b1?11});
    u64 = 64'b0111111111111111111111111111111111111111111111111111111111111110; $display("w64-hit %b", u64 inside {64'b?11111111111111111111111111111111111111111111111111111111111111?});
    u64 = 64'b1111111111111111111111111111111111111111111111111111111111111101; $display("w64-miss %b", u64 inside {64'b?11111111111111111111111111111111111111111111111111111111111111?});
    u64 = 64'b1011; $display("w64-narrow-el %b", u64 inside {4'b1?11});
    u65 = 65'b01111111111111111111111111111111111111111111111111111111111111110; $display("w65-hit %b", u65 inside {65'b?111111111111111111111111111111111111111111111111111111111111111?});
    u65 = 65'b11111111111111111111111111111111111111111111111111111111111111101; $display("w65-miss %b", u65 inside {65'b?111111111111111111111111111111111111111111111111111111111111111?});
    u65 = 65'b1011; $display("w65-narrow-el %b", u65 inside {4'b1?11});
    u128 = 128'b01111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111110; $display("w128-hit %b", u128 inside {128'b?111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111?});
    u128 = 128'b11111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111101; $display("w128-miss %b", u128 inside {128'b?111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111?});
    u128 = 128'b1011; $display("w128-narrow-el %b", u128 inside {4'b1?11});
    u8 = 8'b0000_0100; $display("sized-x-msb-zero-ext %b", u8 inside {4'bx100});
    u8 = 8'b1000_0100; $display("sized-x-msb-high-compared %b", u8 inside {4'bx100});
    $finish;
  end
endmodule
"#,
        &[],
    );
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        [
            "w33-hit 1",
            "w33-miss 0",
            "w33-narrow-el 1",
            "w64-hit 1",
            "w64-miss 0",
            "w64-narrow-el 1",
            "w65-hit 1",
            "w65-miss 0",
            "w65-narrow-el 1",
            "w128-hit 1",
            "w128-miss 0",
            "w128-narrow-el 1",
            "sized-x-msb-zero-ext 1",
            "sized-x-msb-high-compared 0",
        ],
        "stdout:\n{}",
        r.out
    );
}

/// IEEE 1800-2017 §11.4.5 (which §11.4.6 applies to `==?`): operands of unequal width
/// sign-extend when BOTH are signed, zero-extend otherwise. A signed pattern's x/z sign
/// bit therefore extends as a don't-care (`ss-narrow-pat-msbq`, `-msbx`, `-msbz`), and a
/// signed left operand narrower than a signed pattern sign-extends (`ss-narrow-lhs-*`,
/// `call-lhs-narrow`, `signed-select-lhs`); one unsigned side keeps zero extension
/// (`su-*`, `us-*`, `select-lhs-unsigned`). iverilog 13.0's own `==?` and sv2v →
/// iverilog print every line. verilator 5.052 agrees except where the sign bit is x/z
/// (it reads it as 0 and prints 0 for `-msbq`, `-msbq-pos`, `-msbx`, `-msbz`) and on
/// `su-wide-unsigned-pat` (verilator 1; iverilog 0, as §11.4.5's zero extension gives).
/// PRE (00c3d76d) printed: `ss-narrow-pat-msb1 x`, `ss-narrow-pat-msbq x`, `ss-narrow-pat-msbq-pos x`, `ss-narrow-pat-msbx x`, `ss-narrow-pat-msbz x`, `ss-narrow-pat-ext-ones 0`, `ss-narrow-lhs-sext x`, `ss-narrow-lhs-zero-pat 0`, `ss-narrow-lhs-wild-msb x`, `ss-integer x`, `su-pattern-unsigned 0`, `su-wide-unsigned-pat 0`, `us-lhs-unsigned 0`, `su-zero-ext-hit x`, `call-lhs x`, `call-lhs-narrow x`, `select-lhs-unsigned 0`, `signed-select-lhs x`.
#[test]
fn signed_comparison_extends_by_sign() {
    let r = run(F04, &[]);
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        [
            "ss-narrow-pat-msb1 1",
            "ss-narrow-pat-msbq 1",
            "ss-narrow-pat-msbq-pos 1",
            "ss-narrow-pat-msbx 1",
            "ss-narrow-pat-msbz 1",
            "ss-narrow-pat-ext-ones 0",
            "ss-narrow-lhs-sext 1",
            "ss-narrow-lhs-zero-pat 0",
            "ss-narrow-lhs-wild-msb 1",
            "ss-integer 1",
            "su-pattern-unsigned 0",
            "su-wide-unsigned-pat 0",
            "us-lhs-unsigned 0",
            "su-zero-ext-hit 1",
            "call-lhs 1",
            "call-lhs-narrow 1",
            "select-lhs-unsigned 0",
            "signed-select-lhs 1",
        ],
        "stdout:\n{}",
        r.out
    );
}

/// Only the ELEMENT's x/z bits are don't-cares: a left-operand x/z under a wildcard bit
/// is covered, one under a compared bit is `x` unless another compared bit differs.
/// sv2v → iverilog 13.0 and iverilog's own `==?` print every line (verilator is 2-state
/// and is not an oracle for an x operand).
/// PRE (00c3d76d) printed: `lhs-x-at-wild x`, `lhs-z-at-wild x`, `lhs-x-compared x`, `lhs-z-compared x`, `lhs-x-under-msb-wild x`, `lhs-x-under-x-digit x`, `lhs-all-x-all-wild x`, `lhs-all-z-all-z x`, `lhs-x-definite-miss 0`, `lhs-x-plain-element x`.
#[test]
fn left_operand_x_is_not_a_wildcard() {
    let r = run(F05, &[]);
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        [
            "lhs-x-at-wild 1",
            "lhs-z-at-wild 1",
            "lhs-x-compared x",
            "lhs-z-compared x",
            "lhs-x-under-msb-wild 1",
            "lhs-x-under-x-digit 1",
            "lhs-all-x-all-wild 1",
            "lhs-all-z-all-z 1",
            "lhs-x-definite-miss 0",
            "lhs-x-plain-element x",
        ],
        "stdout:\n{}",
        r.out
    );
}

/// The parser's `||` fold over the elements is §11.4.13's result rule — any 1 → 1, else
/// any x → x, else 0 — with wildcard elements beside ranges and plain values, under `!`,
/// in a concatenation and in arithmetic. sv2v → iverilog 13.0 prints every line that
/// holds no fill; verilator 5.052 prints the fill line `fill-x-element 1` (sv2v crashes
/// on a fill element: `Wildcard.hs:96 Non-exhaustive patterns`).
/// PRE (00c3d76d) printed: `wild-then-range 1`, `range-then-wild x`, `neither 0`, `not-hit x`, `not-miss 1`, `third-element x`, `all-miss 0`, `miss-or-x x`, `one-or-x x`, `x-or-miss x`, `not-x x`, `fill-x-element x`, `concat-of-insides x0`, `arith x`.
#[test]
fn several_elements_ranges_and_negation() {
    let r = run(F06, &[]);
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        [
            "wild-then-range 1",
            "range-then-wild 1",
            "neither 0",
            "not-hit 0",
            "not-miss 1",
            "third-element 1",
            "all-miss 0",
            "miss-or-x x",
            "one-or-x 1",
            "x-or-miss x",
            "not-x x",
            "fill-x-element 1",
            "concat-of-insides 10",
            "arith 2",
        ],
        "stdout:\n{}",
        r.out
    );
}

/// An unsized literal whose leftmost digit is x/z pads with that x/z to the width of the
/// expression (§5.7.1), so against a 36-bit operand `'bx1` matches on bit 0 alone:
/// iverilog 13.0's own `w ==? 'bx1` and verilator 5.052 print 1 (sv2v → iverilog prints
/// 0 because sv2v rewrites the literal into a 32-bit mask — not an oracle here). A known
/// leftmost digit pads with 0 (`unsized-known-msb-wide-lhs`). A fill element is sized to
/// the left operand; `'x`/`'z` match everything (verilator 5.052 and iverilog's `==?`
/// twin: 1), `'1` stays an ordinary compare.
/// PRE (00c3d76d) printed: `unsized-known-msb x`, `unsized-x-msb-hit x`, `unsized-x-msb-miss 0`, `unsized-q x`, `unsized-hex-q x`, `unsized-x-msb-wide-lhs 0`, `unsized-known-msb-wide-lhs 0`, `fill-x x`, `fill-z x`, `fill-x-second x`, `fill-1-miss 0`, `fill-1-hit 1`.
#[test]
fn unsized_and_fill_elements() {
    let r = run(
        r#"`timescale 1ns/1ns
module t;
  logic [3:0] v; logic [35:0] w;
  initial begin
    v = 4'b1100; $display("unsized-known-msb %b", v inside {'b1?00});
    v = 4'b0011; $display("unsized-x-msb-hit %b", v inside {'bx1});
    v = 4'b0010; $display("unsized-x-msb-miss %b", v inside {'bx1});
    v = 4'b1100; $display("unsized-q %b", v inside {'b?});
    v = 4'b1100; $display("unsized-hex-q %b", v inside {'h?});
    w = 36'hF_0000_0001; $display("unsized-x-msb-wide-lhs %b", w inside {'bx1});
    w = 36'hF_0000_0001; $display("unsized-known-msb-wide-lhs %b", w inside {'b1x1});
    v = 4'b1010; $display("fill-x %b", v inside {'x});
    v = 4'b1010; $display("fill-z %b", v inside {'z});
    v = 4'b0000; $display("fill-x-second %b", v inside {4'b0001, 'x});
    v = 4'b0111; $display("fill-1-miss %b", v inside {'1});
    v = 4'b1111; $display("fill-1-hit %b", v inside {'1});
    $finish;
  end
endmodule
"#,
        &[],
    );
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        [
            "unsized-known-msb 1",
            "unsized-x-msb-hit 1",
            "unsized-x-msb-miss 0",
            "unsized-q 1",
            "unsized-hex-q 1",
            "unsized-x-msb-wide-lhs 1",
            "unsized-known-msb-wide-lhs 0",
            "fill-x 1",
            "fill-z 1",
            "fill-x-second 1",
            "fill-1-miss 0",
            "fill-1-hit 1",
        ],
        "stdout:\n{}",
        r.out
    );
}

/// `if`, `?:`, a `case` item expression, an immediate `assert`, `while` and `for`
/// conditions. verilator 5.052 and sv2v → iverilog 13.0 print every line.
/// PRE (00c3d76d) printed: `if else`, `ternary 0X`, `case default`, `assert fail`, `while 1000`, `for 0`.
#[test]
fn procedural_contexts() {
    let r = run(
        r#"`timescale 1ns/1ns
module t;
  logic [3:0] v; integer n;
  initial begin
    v = 4'b1100;
    if (v inside {4'b1?00}) $display("if then"); else $display("if else");
    $display("ternary %h", (v inside {4'b1?00}) ? 8'd1 : 8'd2);
    case (1'b1) (v inside {4'b1?00}): $display("case item"); default: $display("case default"); endcase
    assert (v inside {4'b1?00}) $display("assert pass"); else $display("assert fail");
    v = 4'b1000; while (v inside {4'b1?00}) v = v + 4'd1; $display("while %b", v);
    n = 0; for (v = 4'b1000; v inside {4'b1?0?}; v = v + 4'd1) n = n + 1; $display("for %0d", n);
    $finish;
  end
endmodule
"#,
        &[],
    );
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        [
            "if then",
            "ternary 01",
            "case item",
            "assert pass",
            "while 1001",
            "for 2",
        ],
        "stdout:\n{}",
        r.out
    );
}

/// `wait (v inside {4'b1?00})` wakes when `v` becomes `4'b1000`. verilator 5.052 prints
/// both lines (sv2v cannot parse `fork … join_none`).
/// PRE (00c3d76d) printed: `wait end`.
#[test]
fn wait_condition_wakes() {
    let r = run(
        r#"`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v = 4'b0000;
    fork begin wait (v inside {4'b1?00}); $display("wait woke %0t", $time); end join_none
    #1 v = 4'b1000;
    #2 $display("wait end");
    $finish;
  end
endmodule
"#,
        &[],
    );
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        ["wait woke 1", "wait end",],
        "stdout:\n{}",
        r.out
    );
}

/// A continuous assign into a 1-bit and an 8-bit net, a `?:` in a continuous assign,
/// `always_comb` and `always @*`. verilator 5.052 and sv2v → iverilog 13.0 print both
/// lines.
/// PRE (00c3d76d) printed: `cont x 0000000x X comb x star X`, `cont 0 00000000 2 comb 0 star 3`.
#[test]
fn continuous_and_combinational_contexts() {
    let r = run(F09, &[]);
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        [
            "cont 1 00000001 1 comb 1 star 7",
            "cont 0 00000000 2 comb 0 star 3",
        ],
        "stdout:\n{}",
        r.out
    );
}

/// The same comparison in every subroutine lane `--obs-dir` reports: an automatic
/// function (`route: frame`), a static function (`route: inlined`), a task (`frame`), a
/// package function (`frame`), a function in a continuous assign (`frame`) and static
/// functions inlined into a continuous assign, one with a `'x` element. verilator 5.052
/// prints both lines.
/// PRE (00c3d76d) printed: `frame x inlined x task x pkg x cont-frame x cont-inlined xx`, `frame 0 inlined 0 task 0 pkg 0 cont-frame 0 cont-inlined xx`.
#[test]
fn subroutine_lanes() {
    let r = run(F10, &[]);
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        [
            "frame 1 inlined 1 task 1 pkg 1 cont-frame 1 cont-inlined 02",
            "frame 0 inlined 0 task 0 pkg 0 cont-frame 0 cont-inlined 01",
        ],
        "stdout:\n{}",
        r.out
    );
}

/// A class method body and a queue element as the left operand. verilator 5.052 prints
/// the line (sv2v cannot parse `new` / `[$]`).
/// PRE (00c3d76d) printed: `method x queue-elem x`.
#[test]
fn class_method_and_queue_element() {
    let r = run(
        r#"`timescale 1ns/1ns
class K; function logic m(input logic [3:0] a); return a inside {4'b1?00}; endfunction endclass
module t;
  logic [3:0] q [$];
  initial begin
    K k; k = new;
    q.push_back(4'b1100);
    $display("method %b queue-elem %b", k.m(4'b1000), q[0] inside {4'b1?00});
    $finish;
  end
endmodule
"#,
        &[],
    );
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(r.lines(), ["method 1 queue-elem 1",], "stdout:\n{}", r.out);
}

/// `assert property (@(posedge clk) v inside {4'b1?00})` holds for `v = 4'b1100`:
/// verilator 5.052 and sv2v → iverilog 13.0 print only `property end`.
/// PRE (00c3d76d) printed: `property fail 1`, `property fail 3`, `property fail 5`, `property end`.
#[test]
fn concurrent_assertion_holds() {
    let r = run(
        r#"`timescale 1ns/1ns
module t;
  logic [3:0] v; logic clk = 0;
  always #1 clk = ~clk;
  assert property (@(posedge clk) v inside {4'b1?00}) else $display("property fail %0t", $time);
  initial begin v = 4'b1100; #6 $display("property end"); $finish; end
endmodule
"#,
        &[],
    );
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(r.lines(), ["property end",], "stdout:\n{}", r.out);
}

/// Typed and untyped `localparam` initializers, a range bound, an instance parameter
/// override, a generate `if` and a generate-`for` `if`. verilator 5.052 and sv2v →
/// iverilog 13.0 print every line. On PRE the design was refused (E3009 / E3010 "has no
/// constant-fold arm" for each initializer, condition and override), except the range
/// bound, which PRE sized `[0:0]` silently — `$bits` 1 where both oracles print 2.
/// PRE (00c3d76d) printed: nothing — the design was refused (see the doc above).
#[test]
fn constant_contexts_fold_the_wildcard() {
    let r = run(
        r#"`timescale 1ns/1ns
module m #(parameter logic P = 1'b0) (); initial $display("override %b", P); endmodule
module t;
  localparam logic [3:0] PV = 4'b1100;
  localparam logic L1 = (4'b1100 inside {4'b1?00});
  localparam L2 = (4'b1000 inside {4'b1?00});
  localparam L4 = PV inside {4'b0?00};
  logic [(4'b1100 inside {4'b1?00}) : 0] wb;
  m #(.P(4'b1100 inside {4'b1?00})) u();
  if (PV inside {4'b1?00}) begin : g initial $display("gen-if then"); end
  else begin : g2 initial $display("gen-if else"); end
  for (genvar i = 0; i < 2; i++) begin : gl
    if ((PV + i) inside {4'b110?}) begin : h initial $display("gen-for %0d in", i); end
    else begin : k initial $display("gen-for %0d out", i); end
  end
  initial begin #1 $display("params %b %b %b bits %0d", L1, L2, L4, $bits(wb)); $finish; end
endmodule
"#,
        &[],
    );
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        [
            "gen-if then",
            "gen-for 0 in",
            "gen-for 1 in",
            "override 1",
            "params 1 1 0 bits 2",
        ],
        "stdout:\n{}",
        r.out
    );
}

/// CONTROL — passes on PRE as well. Elements with no x/z bit keep exactly the `==`
/// they had: variable elements (4-state and 2-state), a continuous assign, a function,
/// string and real sets (§11.4.13 compares a non-integral element with `==`), a signed
/// negative element, a negated range, `'1`/`'0` fills, class handles against `null`,
/// a generate `if` on a string parameter, a constraint set with ranges (every draw
/// lands in the set) and `randomize() with`, and a `case` item. PRE prints the same
/// lines; the elaborated IR of this design is byte-identical PRE vs POST (REPORT).
#[test]
fn elements_without_x_z_are_unchanged() {
    let r = run(
        r#"`timescale 1ns/1ns
class C; rand bit [3:0] x; constraint c { x inside {4'd3, 4'd5, [4'd10:4'd12]}; } endclass
class H; int a; endclass
module t;
  logic [3:0] v, e; bit [3:0] be; string s; real r; logic signed [7:0] s8;
  localparam logic [3:0] PV = 4'd12;
  localparam LQ = PV inside {4'd12, 4'd3};
  localparam string MODE = "B";
  wire w = v inside {4'd1, e, [4'd8:4'd9]};
  if (PV inside {4'd12}) begin : g initial $display("gen %b", LQ); end
  if (MODE inside {"A", "B"}) begin : gs initial $display("gen-string"); end
  function automatic logic f(input logic [3:0] a); return a inside {4'd2, 4'd4}; endfunction
  initial begin
    C o; H h, h2; int ok, okall, inset;
    o = new; h = new; h2 = h;
    v = 4'd4; e = 4'd9; be = 4'd4; s = "ab"; r = 1.5; s8 = -8'sd4;
    #1 $display("plain %b var %b cont %b fn %b str %b real %b neg %b not-range %b",
                v inside {4'd4}, v inside {e, be}, w, f(v), s inside {"ab", "cd"},
                r inside {1.5, [2.0:3.0]}, s8 inside {-8'sd4, 8'sd3}, !(v inside {[0:3]}));
    $display("fill-1-0 %b handle %b %b", v inside {'1, '0}, h inside {null, h2}, h2 inside {null});
    okall = 1; inset = 1;
    for (int i = 0; i < 20; i++) begin
      ok = o.randomize(); okall = okall & ok;
      inset = inset & (o.x == 3 || o.x == 5 || (o.x >= 10 && o.x <= 12));
    end
    $display("constraint ok %0d in-set %0d", okall, inset);
    ok = o.randomize() with { x inside {4'd11}; }; $display("with ok %0d x %0d", ok, o.x);
    case (1'b1) (v inside {4'd4, 4'd5}): $display("case item"); default: $display("case default"); endcase
    #1 $finish;
  end
endmodule
"#,
        &[],
    );
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        [
            "gen 1",
            "gen-string",
            "plain 1 var 1 cont 0 fn 1 str 1 real 1 neg 1 not-range 1",
            "fill-1-0 0 handle 1 0",
            "constraint ok 1 in-set 1",
            "with ok 1 x 11",
            "case item",
        ],
        "stdout:\n{}",
        r.out
    );
}

/// RESIDUE — pinned at the observed value, not the oracle's. An element whose x/z bits
/// arrive at RUN time (a 4-state variable holding them, a function returning a pattern,
/// a constant `4'd1/4'd0`) is still compared with `==`, so a match reads `x`. vita has
/// no IR primitive for a runtime don't-care mask (the reason `lower_wildcard_eq` refuses
/// a non-constant pattern). sv2v → iverilog 13.0 prints `var-x-element 1`,
/// `var-z-element 1`, `var-all-x-element 1`, `var-known-element 1`,
/// `var-x-element-miss 0`, `call-element 1`, `div-by-zero-element 1`; iverilog's own
/// `v ==? e` prints 1; verilator 5.052 refuses a 4-state non-constant element. PRE
/// prints the same lines as POST.
#[test]
fn runtime_x_z_element_residue() {
    let r = run(
        r#"`timescale 1ns/1ns
module t;
  logic [3:0] v, e;
  function automatic logic [3:0] cf(); return 4'b1?00; endfunction
  initial begin
    v = 4'b1100; e = 4'b1x00; $display("var-x-element %b", v inside {e});
    v = 4'b1000; e = 4'b1z00; $display("var-z-element %b", v inside {e});
    v = 4'b1100; e = 4'bxxxx; $display("var-all-x-element %b", v inside {e});
    v = 4'b1100; e = 4'b1100; $display("var-known-element %b", v inside {e});
    v = 4'b0100; e = 4'b1x00; $display("var-x-element-miss %b", v inside {e});
    v = 4'b1100; $display("call-element %b", v inside {cf()});
    v = 4'b1100; $display("div-by-zero-element %b", v inside {4'd1 / 4'd0});
    $finish;
  end
endmodule
"#,
        &[],
    );
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        [
            "var-x-element x",
            "var-z-element x",
            "var-all-x-element x",
            "var-known-element 1",
            "var-x-element-miss 0",
            "call-element x",
            "div-by-zero-element x",
        ],
        "stdout:\n{}",
        r.out
    );
}

/// The `==?` operator shares the builder (`wildcard_cmp_ids`), so its sign and unsized
/// extension moved with it: before, a signed pattern zero-extended, a signed narrower
/// left operand zero-extended, and an unsized x literal stopped at 32 bits. iverilog
/// 13.0's own `==?` prints every line (it is the native 4-state oracle for `==?`);
/// sv2v → iverilog agrees.
/// PRE (00c3d76d) printed: `weq-narrow-pat-msb1 0`, `weq-narrow-pat-msbq 0`, `weq-narrow-pat-msbq-pos 0`, `weq-narrow-pat-known 0`, `weq-narrow-lhs-sext 0`, `weq-narrow-lhs-zero-pat 1`, `weq-narrow-pat-ext-ones 1`, `weq-integer 0`, `weq-ne 1`, `weq-unsized-x-msb-wide 0`, `weq-same-width 1`, `weq-unsigned-pat 0`.
#[test]
fn wildcard_eq_operator_extends_by_sign_and_unsized_x() {
    let r = run(
        r#"`timescale 1ns/1ns
module t;
  logic signed [7:0] s8; logic signed [3:0] s4; integer i32; logic [35:0] w;
  initial begin
    s8 = -8'sd4;       $display("weq-narrow-pat-msb1 %b", s8 ==? 4'sb1?00);
    s8 = -8'sd4;       $display("weq-narrow-pat-msbq %b", s8 ==? 4'sb?100);
    s8 = 8'sb01010100; $display("weq-narrow-pat-msbq-pos %b", s8 ==? 4'sb?100);
    s8 = -8'sd4;       $display("weq-narrow-pat-known %b", s8 ==? 4'sb1100);
    s4 = -4'sd4;       $display("weq-narrow-lhs-sext %b", s4 ==? 8'sb11111?00);
    s4 = -4'sd4;       $display("weq-narrow-lhs-zero-pat %b", s4 ==? 8'sb00001?00);
    s8 = 8'sb00001100; $display("weq-narrow-pat-ext-ones %b", s8 ==? 4'sb1?00);
    i32 = -4;          $display("weq-integer %b", i32 ==? 4'sb1?00);
    s8 = -8'sd4;       $display("weq-ne %b", s8 !=? 4'sb1?00);
    w = 36'hF_0000_0001; $display("weq-unsized-x-msb-wide %b", w ==? 'bx1);
    s8 = -8'sd4;       $display("weq-same-width %b", s8 ==? 8'sb11111?00);
    s4 = -4'sd4;       $display("weq-unsigned-pat %b", s4 ==? 8'b11111?00);
    $finish;
  end
endmodule
"#,
        &[],
    );
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        [
            "weq-narrow-pat-msb1 1",
            "weq-narrow-pat-msbq 1",
            "weq-narrow-pat-msbq-pos 1",
            "weq-narrow-pat-known 1",
            "weq-narrow-lhs-sext 1",
            "weq-narrow-lhs-zero-pat 0",
            "weq-narrow-pat-ext-ones 0",
            "weq-integer 1",
            "weq-ne 0",
            "weq-unsized-x-msb-wide 1",
            "weq-same-width 1",
            "weq-unsigned-pat 0",
        ],
        "stdout:\n{}",
        r.out
    );
}

/// One compound x/z element per design: the wildcard bits are not one constant `Const`
/// node here, so `==` would read them as x and `==?` cannot be built — loud, never `==`.
/// Every one of these printed `x` on PRE except the replication, which printed `0`
/// because bit 2 happened to differ (with `v = 4'b0000` it prints `x` where both oracles
/// print 1). verilator 5.052 and iverilog's own `==?` print 1 for all but the
/// replication (0).
#[test]
fn compound_x_z_element_is_loud() {
    for el in [
        "{2'b1?, 2'b00}",
        "P | 4'b000x",
        "4'(4'b1?00)",
        "1'b1 ? 4'b1?00 : 4'b0000",
        "{2{2'b?0}}",
        "~4'b0?11",
        "$unsigned(4'b1?00)",
        "{p, 2'b?0}",
    ] {
        let src = format!(
            "`timescale 1ns/1ns\nmodule t;\n  logic [3:0] v; logic [1:0] p; \
             localparam logic [3:0] P = 4'b1100;\n  initial begin v = 4'b1100; p = 2'b11; \
             $display(\"r %b\", v inside {{{el}}}); $finish; end\nendmodule\n"
        );
        let r = run(&src, &[]);
        assert_eq!(r.code, 1, "`{el}` must be refused:\n{}{}", r.out, r.err);
        assert!(
            r.err.contains(
                "error[VITA-E3009] E-ELAB-UNSUPPORTED: an `inside` element that builds \
                            an x/z/? literal into a larger expression is not supported"
            ),
            "`{el}` wrong refusal:\n{}",
            r.err
        );
        assert!(!r.out.contains("r "), "`{el}` printed a value:\n{}", r.out);
    }
}

/// Refusals kept or added, each a shape with no constant value. A constant x/z element in
/// a constraint (verilator 5.052 cannot solve it either: `randomize()` returns 0); a
/// constant-function body comparing against an x/z pattern (PRE and e5147442 the same
/// refusal; both oracles 1); a range bound and an array dimension on a wildcard whose
/// pattern is not one literal or whose value is x — PRE and e5147442 made the net ONE bit
/// silently and `$size` x, sv2v → iverilog 2 / x; a generate-`case` item of the same kind
/// — PRE and e5147442 took `default` silently, sv2v → iverilog takes the item, verilator
/// refuses an x/? label there; an unsized x pattern against an absolute hierarchical left
/// operand, whose width — and so the pattern's padding — is not known at lowering. The
/// needles are the refusal's class, not its full text.
#[test]
fn shapes_without_a_constant_value_are_loud() {
    let cases = [
        (
            "class C; rand bit [3:0] x; constraint c { x inside {4'b1?00}; } endclass\nmodule t;\n  initial begin C o; int ok; o = new; ok = o.randomize(); $display(\"ok %0d x %b\", ok, o.x); $finish; end\nendmodule\n",
            "[VITA-E3009]",
            "unsupported constraint expression form",
        ),
        (
            "module t;\n  function automatic logic cf(input logic [3:0] a); return a inside {4'b1?00}; endfunction\n  localparam logic L = cf(4'b1100);\n  initial begin $display(\"L %b\", L); $finish; end\nendmodule\n",
            "[VITA-E3009]",
            "parameter `L` value is not a constant",
        ),
        (
            "module t;\n  logic [(4'b1100 inside {{2'b1?, 2'b00}}) : 0] wb;\n  initial begin $display(\"B %0d\", $bits(wb)); $finish; end\nendmodule\n",
            "[VITA-E3009]",
            "is not allowed in a constant range bound",
        ),
        (
            "module t;\n  logic ad [(4'bx100 inside {4'b1?00}) : 0];\n  initial begin $display(\"A %0d\", $size(ad)); $finish; end\nendmodule\n",
            "[VITA-E3009]",
            "is not allowed in a constant range bound",
        ),
        (
            "module t;\n  logic [(4'b1100 ==? {2'b1?, 2'b00}) : 0] wq;\n  initial begin $display(\"Q %0d\", $bits(wq)); $finish; end\nendmodule\n",
            "[VITA-E3009]",
            "is not allowed in a constant range bound",
        ),
        (
            "module t;\n  case (1'b1)\n    (4'b1100 inside {{2'b1?, 2'b00}}): begin : gi initial $display(\"item\"); end\n    default: begin : gd initial $display(\"default\"); end\n  endcase\n  initial #1 $finish;\nendmodule\n",
            "[VITA-E3010]",
            "generate-case item is not a constant",
        ),
        (
            "module late; logic [35:0] u36 = 36'hF_0000_0001; endmodule\nmodule t;\n  late uL();\n  initial begin #1 $display(\"H %b\", t.uL.u36 inside {'bx1}); $finish; end\nendmodule\n",
            "[VITA-E3009]",
            "unsized pattern whose leftmost digit is x/z",
        ),
    ];
    for (src, code, needle) in cases {
        let r = run(src, &[]);
        assert_eq!(r.code, 1, "must be refused:\n{src}\n{}{}", r.out, r.err);
        assert!(
            r.err.contains(code) && r.err.contains(needle),
            "{src}\n{}",
            r.err
        );
        assert!(r.lines().is_empty(), "{src} printed a value:\n{}", r.out);
    }
}

/// Round 2 (R2-1). A CONSTANT `inside` element with x/z bits compares the left operand
/// at the comparison's width, `max(L(lhs), L(pattern))` (§11.6.1 Table 11-21): the carry
/// of `4'd15 + 4'd1`, the shift of `4'b1000 << 1` and the `~` run at the pattern's width.
/// Localparams (typed and untyped), a range bound, a `?:` bound, generate `if` and an
/// instance override. sv2v 0.0.13 → iverilog 13.0 and verilator 5.052 print every line.
/// e5147442 read the left operand at its own width and printed `gen-a else`, `gen-b then`,
/// `override W=8`, `L 0 0 0 0 1 1 0 0 0 0 0`, `bits 1 8` — `L5`, `L6`, `gen-b`, the `?:` bound
/// and the override were RIGHT on PRE (00c3d76d), which refused every other line here
/// (E3009 / E3010 "has no constant-fold arm").
#[test]
fn constant_wildcard_reads_the_left_operand_at_the_common_width() {
    let r = run(
        r#"`timescale 1ns/1ns
module m #(parameter W = 0) (); initial #1 $display("override W=%0d", W); endmodule
module t;
  localparam [3:0] A = 4'd15;
  localparam [3:0] U4 = 4'b1100;
  localparam L1 = (4'd15 + 4'd1) inside {5'b1?000};
  localparam L2 = (~4'b0000) inside {5'b1111?};
  localparam L3 = (A + 4'd1) inside {5'b1?000};
  localparam L4 = (4'b1000 << 1) inside {5'b1?000};
  localparam L5 = (4'd15 + 4'd1) inside {5'b0?000};
  localparam L6 = (4'hF << 1) inside {8'b0000_111?};
  localparam L7 = (~4'b0011) inside {8'b1111_11?0};
  localparam L8 = (U4 + 4'd4) inside {8'b0001_0?00};
  localparam L9 = (-(4'd4)) inside {8'b1111_1?00};
  localparam L10 = (4'd4 - 4'd5) inside {8'b1111_111?};
  localparam bit LB = (4'd0 - 4'd1) inside {5'b1111?};
  logic [((4'd15 + 4'd1) inside {5'b1?000}) : 0] rb;
  logic [((4'd15 + 4'd1) inside {8'b0000_?000}) ? 7 : 3 : 0] ab;
  if ((4'd15 + 4'd1) inside {5'b1?000}) begin : g1 initial #1 $display("gen-a then"); end
  else begin : g1e initial #1 $display("gen-a else"); end
  if ((4'd15 + 4'd1) inside {5'b0?000}) begin : g2 initial #1 $display("gen-b then"); end
  else begin : g2e initial #1 $display("gen-b else"); end
  m #(.W(((4'd15 + 4'd1) inside {8'b0000_?000}) ? 8 : 2)) u();
  initial begin
    #2 $display("L %b %b %b %b %b %b %b %b %b %b %b", L1, L2, L3, L4, L5, L6, L7, L8, L9, L10, LB);
    $display("bits %0d %0d", $bits(rb), $bits(ab));
    $finish;
  end
endmodule
"#,
        &[],
    );
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        [
            "gen-a then",
            "gen-b else",
            "override W=2",
            "L 1 1 1 1 0 0 1 1 1 1 1",
            "bits 2 4",
        ],
        "stdout:\n{}",
        r.out
    );
}

/// Round 2 (R2-1). The `==?` OPERATOR in a constant context reads the same routine:
/// `Q1`–`Q3` at the common width, `Q4`/`Q5` a signed pattern against a signed byte (sign
/// extension, an x sign bit a don't-care), `Q6` two wildcard compares under `||`, `Q7`
/// `!=?`; `I1` the signed `inside` twin and `I2` a two-element set. iverilog 13.0's own
/// `==?` prints the Q line; sv2v → iverilog prints all of it (verilator 5.052 reads the x
/// sign bit of `Q4` / `I1` as 0). On PRE (00c3d76d) and e5147442 the `==?` cells were
/// silently wrong — lens probes: `(4'd15+4'd1) ==? 5'b1?000` 0, `S ==? 4'sb?100` 0,
/// `S2 ==? 4'sb1?00` 1 — and `Q6`, `I1`, `I2` were E3009.
#[test]
fn constant_wildcard_eq_operator_and_sets() {
    let r = run(
        r#"`timescale 1ns/1ns
module t;
  localparam signed [7:0] S = 8'sd84;
  localparam signed [7:0] S2 = 8'sd12;
  localparam logic [3:0] PV = 4'b1100;
  localparam Q1 = (4'd15 + 4'd1) ==? 5'b1?000;
  localparam Q2 = (~4'b0000) ==? 5'b1111?;
  localparam Q3 = (4'd15 + 4'd1) ==? 5'b0?000;
  localparam Q4 = S ==? 4'sb?100;
  localparam Q5 = S2 ==? 4'sb1?00;
  localparam Q6 = (PV ==? 4'b0?00) || (PV ==? 4'b1?00);
  localparam Q7 = (4'd15 + 4'd1) !=? 8'b0001_?000;
  localparam I1 = S inside {4'sb?100};
  localparam I2 = PV inside {4'b0?00, 4'b1?00};
  logic [((4'd15 + 4'd1) ==? 5'b1?000) : 0] rbq;
  if ((4'd15 + 4'd1) ==? 5'b1?000) begin : gq initial #1 $display("gen-q then"); end
  else begin : gqe initial #1 $display("gen-q else"); end
  initial begin
    #2 $display("Q %b %b %b %b %b %b %b I %b %b bits %0d", Q1, Q2, Q3, Q4, Q5, Q6, Q7, I1, I2, $bits(rbq));
    $finish;
  end
endmodule
"#,
        &[],
    );
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        ["gen-q then", "Q 1 1 0 1 0 1 0 I 1 1 bits 2",],
        "stdout:\n{}",
        r.out
    );
}

/// Round 2 (R2-2, R2-6). A definite known-bit mismatch is the `==?` answer (0) — a
/// signed, an unsized and a 100-bit operand — and a range bound / array dimension on an
/// admitted wildcard takes its width. sv2v → iverilog 13.0 and verilator 5.052 print every
/// value. PRE (00c3d76d) printed `gen else`, `L 0 0 0 bits 1 1 1 size x` (the three bounds
/// one bit and the array size x, silently); e5147442 refused `L1`, `L2` and the generate
/// `if` (E3009 / E3010) — a correct → loud the round-1 wide-domain decline caused.
#[test]
fn constant_definite_mismatch_and_bounds() {
    let r = run(
        r#"`timescale 1ns/1ns
module t;
  localparam [99:0] W = 100'd2;
  localparam L1 = 4'b0100 inside {4'sb1?00};
  localparam L2 = 4'b0100 inside {'b1?00};
  localparam L3 = W inside {4'b000?};
  logic [(4'b1100 inside {4'sb1?00}) : 0] rbs;
  logic [(4'b1100 inside {'b1?00}) : 0] rbu;
  logic [(4'b1100 inside {4'b0000, 4'b1?00}) : 0] rb2;
  logic ad [(4'b1100 inside {4'b1?00}) : 0];
  if (4'b0100 inside {4'sb1?00}) begin : g initial #1 $display("gen then"); end
  else begin : ge initial #1 $display("gen else"); end
  initial begin
    #2 $display("L %b %b %b bits %0d %0d %0d size %0d", L1, L2, L3, $bits(rbs), $bits(rbu), $bits(rb2), $size(ad));
    $finish;
  end
endmodule
"#,
        &[],
    );
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        ["gen else", "L 0 0 0 bits 2 2 2 size 2",],
        "stdout:\n{}",
        r.out
    );
}

/// Round 2 (R2-3). A left operand with no width at lowering (an absolute hierarchical
/// path) takes `(lhs | W) ==/!= (P | W)`, which the engine sizes and signs once the path
/// resolves. iverilog 13.0's own `==?` (sv2v → iverilog for the `inside` spelling) prints
/// every line; verilator 5.052 differs only on the x sign bit / x left operand (2-state)
/// and on `s4 inside {8'b1111_1?00}` (its own sign defect, iverilog 0). PRE (00c3d76d)
/// refused the `==?` line and printed `x`/`0` for the `inside` ones; e5147442 refused
/// all of them ("left operand of unsizable width").
#[test]
fn absolute_hierarchical_left_operand() {
    let r = run(
        r#"`timescale 1ns/1ns
module late;
  logic [7:0] u8 = 8'b0101_0100; logic signed [7:0] s8 = -8'sd4; logic signed [3:0] s4 = -4'sd4;
  logic [7:0] ux = 8'b0x01_x100;
endmodule
module t;
  logic [7:0] u8n;
  late uL();
  wire a1 = t.uL.u8 inside {4'b?100};
  wire a2 = t.u8n inside {4'b?100};
  initial begin
    u8n = 8'b0000_0100; #1;
    $display("cont %b %b", a1, a2);
    $display("hier %b %b %b", t.uL.u8 inside {4'b?100}, t.u8n inside {4'b?100}, t.uL.u8 inside {8'b0101_0100});
    $display("signed %b %b %b %b", t.uL.s8 inside {4'sb?100}, t.uL.s8 inside {4'sb1?00}, t.uL.s4 inside {8'sb1111_1?00}, t.uL.s4 inside {8'b1111_1?00});
    $display("xz %b %b %b", t.uL.ux inside {8'b0?01_?100}, t.uL.ux inside {8'b0101_?100}, t.uL.ux inside {8'b0111_?100});
    $display("weq %b %b", t.uL.u8 ==? 4'b?100, t.uL.s8 !=? 4'sb1?00);
    $finish;
  end
endmodule
"#,
        &[],
    );
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        [
            "cont 0 1",
            "hier 0 1 1",
            "signed 1 1 1 0",
            "xz 1 x 0",
            "weq 0 0",
        ],
        "stdout:\n{}",
        r.out
    );
}

/// Round 2 (R2-4). A `let` that names an unsized x literal pads it to the expression's
/// width exactly as the literal written in place does (§5.7.1, §11.12): the fact is
/// recorded where the literal is lowered, not read from the element's source text.
/// iverilog cannot run `let`; its own `v36 ==? 'bx1` and `v8 ==? 4'bx100` print 1 and 1,
/// hand-IEEE agrees. e5147442 printed `let-unsized 0 direct 1` — one design, two answers —
/// and `let-weq 0`; PRE (00c3d76d) printed `let-unsized 0 direct 0`, `let-sized x direct x`,
/// `let-weq 0`.
#[test]
fn let_reference_to_an_unsized_x_literal() {
    let r = run(
        r#"`timescale 1ns/1ns
module t;
  let LU = 'bx1;
  let LS = 4'bx100;
  logic [35:0] v36; logic [7:0] v8;
  initial begin
    v36 = 36'hF_0000_0001; v8 = 8'b0000_0100;
    $display("let-unsized %b direct %b", v36 inside {LU}, v36 inside {'bx1});
    $display("let-sized %b direct %b", v8 inside {LS}, v8 inside {4'bx100});
    $display("let-weq %b", v36 ==? LU);
    $finish;
  end
endmodule
"#,
        &[],
    );
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        [
            "let-unsized 1 direct 1",
            "let-sized 1 direct 1",
            "let-weq 1",
        ],
        "stdout:\n{}",
        r.out
    );
}

/// Round 2 (R2-5). `8'(a + (b ==? 4'b1x0x))`: the size-cast lowering used to map every
/// binary operator before looking at it, and `==?` has no `ir::BinOp` twin, so a DEBUG
/// build panicked (`WildEq must be lowered via lower_wildcard_eq`, rc 101) while release
/// threw the mapping away. cli tests run the debug binary. iverilog 13.0 and verilator
/// 5.052: `sc 00000010 00000001 00000010`; release PRE (00c3d76d) printed the first two and
/// `xxxxxxxx` for the `inside` spelling; the e5147442 debug binary panicked.
#[test]
fn wildcard_eq_under_a_size_cast_debug_build() {
    let r = run(
        r#"`timescale 1ns/1ns
module t;
  logic [7:0] a; logic [3:0] b;
  initial begin
    a = 8'd1; b = 4'b1000;
    $display("sc %b %b %b", 8'(a + (b ==? 4'b1x0x)), 8'(a + (b !=? 4'b1x0x)), 8'(a + (b inside {4'b1x0x})));
    $finish;
  end
endmodule
"#,
        &[],
    );
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert_eq!(
        r.lines(),
        ["sc 00000010 00000001 00000010",],
        "stdout:\n{}",
        r.out
    );
}

/// The comparison is IR, so the three executors print the same bytes — the pinned
/// designs above, run under `--backend interp` and `--backend vm` against the default.
#[test]
fn backends_agree() {
    for src in [F01, F04, F05, F06, F09, F10] {
        let native = run(src, &[]);
        assert_eq!(native.code, 0, "{}", native.err);
        for be in ["interp", "vm"] {
            let r = run(src, &["--backend", be]);
            assert_eq!(r.code, 0, "{be}: {}", r.err);
            assert_eq!(r.out, native.out, "--backend {be} differs from native");
        }
    }
}
