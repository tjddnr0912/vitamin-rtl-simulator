//! P0 >64-bit truncation cluster (REMAINING_WORK 2026-06-10 P0-1~4).
//!
//! Root cause under test: `Value::to_u64`/`to_u128` returned the truncated low
//! word(s) for values wider than the lane instead of `None`, so relational
//! compares, shift amounts, array/word indices, part-select offsets and
//! `$clog2` silently used a wrong small number. Each test pins the exact
//! IEEE-correct behavior (iverilog agrees; see differential.rs companion).

use diag::{LogEvent, LogSink};
use sim_engine::{simulate_capture, SimOpts};

#[derive(Default)]
struct DiagSink(std::cell::RefCell<Vec<String>>);
impl LogSink for DiagSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::Diagnostic(d) = e {
            self.0
                .borrow_mut()
                .push(format!("{:?}: {}", d.severity, d.message));
        }
    }
}

/// lex → parse → elaborate → simulate, returning captured `$display` stdout.
fn run(src: &str) -> String {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = DiagSink::default();
    let ir = elaborate::elaborate(&su.expect("source unit"), &sink);
    let hard: Vec<String> = sink
        .0
        .borrow()
        .iter()
        .filter(|d| d.starts_with("Error") || d.starts_with("Fatal"))
        .cloned()
        .collect();
    assert!(hard.is_empty(), "elaborate errors: {hard:?}");
    let (_res, out) = simulate_capture(&ir.expect("ir"), SimOpts::default());
    out
}

/// P0-1: unsigned relational on 65..128-bit operands must compare the FULL
/// value (2^64 > 1), not just word 0 (which is 0 here).
#[test]
fn wide_unsigned_relational_compares_full_value() {
    let out = run(r#"
module t;
  reg [127:0] a, b;
  initial begin
    a = 128'h1_0000_0000_0000_0000; // 2^64: word0 = 0
    b = 128'h1;
    $display("gt=%b lt=%b ge=%b le=%b", a > b, a < b, a >= b, a <= b);
  end
endmodule
"#);
    assert_eq!(out.trim(), "gt=1 lt=0 ge=1 le=0");
}

/// P0-1 (multi-word): the compare must also be exact beyond the 128-bit
/// arithmetic lane — word-wise, any width.
#[test]
fn very_wide_relational_256bit() {
    let out = run(r#"
module t;
  reg [255:0] a, b;
  initial begin
    a = 256'h1_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000; // 2^192
    b = 256'd5;
    $display("gt=%b lt=%b", a > b, a < b);
  end
endmodule
"#);
    assert_eq!(out.trim(), "gt=1 lt=0");
}

/// P0-1 (signed): a negative 128-bit signed value is LESS than +1; sign-aware
/// compare must not read only the low word (all-ones low word looks huge).
#[test]
fn wide_signed_relational_sign_aware() {
    let out = run(r#"
module t;
  reg signed [127:0] a, b;
  initial begin
    a = 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff; // -1
    b = 128'sd1;
    $display("lt=%b gt=%b", a < b, a > b);
    a = 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_fffe; // -2
    b = 128'hffff_ffff_ffff_ffff_ffff_ffff_ffff_ffff; // -1
    $display("lt2=%b", a < b);
  end
endmodule
"#);
    assert_eq!(out.trim(), "lt=1 gt=0\nlt2=1");
}

/// P0-2: a shift amount held in a >64-bit signal whose value exceeds u64 must
/// shift everything out (logical → 0), not be truncated to word 0 (= no shift).
#[test]
fn shift_amount_wider_than_64_bits_shifts_out() {
    let out = run(r#"
module t;
  reg [7:0] x, l, r;
  reg [127:0] s;
  initial begin
    x = 8'hFF;
    s = 128'h1_0000_0000_0000_0000; // 2^64: word0 = 0
    l = x << s;
    r = x >> s;
    $display("l=%h r=%h", l, r);
  end
endmodule
"#);
    assert_eq!(out.trim(), "l=00 r=00");
}

/// P0-2 (arith fill): `>>>` of a negative signed value by an over-u64 amount
/// fills with the sign bit (all ones), not "no shift".
#[test]
fn arith_shift_right_huge_amount_fills_sign() {
    let out = run(r#"
module t;
  reg signed [7:0] x, y;
  reg [127:0] s;
  initial begin
    x = -8'sd2; // 8'b1111_1110: distinguishes sign-fill (all ones) from no-shift
    s = 128'h1_0000_0000_0000_0000;
    y = x >>> s;
    $display("y=%b", y);
  end
endmodule
"#);
    assert_eq!(out.trim(), "y=11111111");
}

/// P0-3: unary minus must produce the full-width two's complement — `-1` in
/// 128 bits is 32 f's, not zeros in the upper word.
#[test]
fn negate_wide_value_full_width() {
    let out = run(r#"
module t;
  reg [127:0] n, m;
  initial begin
    n = -128'd1;
    m = -(128'h1_0000_0000_0000_0000); // -(2^64)
    $display("n=%h", n);
    $display("m=%h", m);
  end
endmodule
"#);
    assert_eq!(
        out.trim(),
        "n=ffffffffffffffffffffffffffffffff\nm=ffffffffffffffff0000000000000000"
    );
}

/// P0-4 (array word index): an index whose value exceeds u32 keeps its LOW 32
/// BITS — `2^32 + 1` reads and writes element 1, exactly as both oracles do.
///
/// This asserted the opposite until §4.5.310 ("must NOT wrap to a small
/// element"), on the P0-4 reading that an `as u32` truncation is a silent
/// value corruption. That reading is right about the CAUSE — the original
/// defect was a Rust cast nobody chose — and wrong about the ANSWER: iverilog
/// 13 and verilator 5.050 both index an unpacked array by the low 32 bits of
/// the index read as a 32-bit integer. What P0-4 actually forbids is the
/// truncation happening by accident; it happens deliberately now, in
/// `elaborate::seal_index_unsigned`, and an x/z above bit 31 still poisons the
/// index (`cli/tests/array_word_index_domain.rs`).
///
/// The sibling below (`select_offset_beyond_u64_is_x`) is UNCHANGED and is the
/// contrast that makes this one a decision: a packed part-select offset is not
/// an array word, the two oracles disagree about it, and vita stays with
/// iverilog there.
#[test]
fn array_index_beyond_u32_keeps_its_low_32_bits() {
    let out = run(r#"
module t;
  reg [7:0] mem [0:3];
  reg [63:0] idx;
  reg [7:0] v;
  initial begin
    mem[1] = 8'h55;
    idx = 64'h1_0000_0001; // 2^32 + 1 -> element 1 in both oracles
    v = mem[idx];
    $display("v=%h", v);
    mem[idx] = 8'hAA;       // lands on mem[1]
    $display("m1=%h", mem[1]);
  end
endmodule
"#);
    assert_eq!(out.trim(), "v=55\nm1=aa");
}

/// P0-4 (part-select offset): an indexed part-select whose offset exceeds the
/// u64 lane is out-of-range → X, not a silent low-word offset.
#[test]
fn select_offset_beyond_u64_is_x() {
    let out = run(r#"
module t;
  reg [255:0] big;
  reg [127:0] off;
  reg [7:0] w;
  initial begin
    big = 256'h5a;
    off = 128'h1_0000_0000_0000_0000; // word0 = 0: truncation would read big[7:0]
    w = big[off +: 8];
    $display("w=%h", w);
  end
endmodule
"#);
    assert_eq!(out.trim(), "w=xx");
}

/// P0-4 ($clog2): must use the full value — $clog2(2^64) = 64, $clog2(2^64+1) = 65.
#[test]
fn clog2_of_wide_value_is_exact() {
    let out = run(r#"
module t;
  reg [127:0] a, b;
  initial begin
    a = 128'h1_0000_0000_0000_0000;
    b = 128'h1_0000_0000_0000_0001;
    $display("ca=%0d cb=%0d", $clog2(a), $clog2(b));
  end
endmodule
"#);
    assert_eq!(out.trim(), "ca=64 cb=65");
}

/// %c keeps printing the LOW byte even when the value is wide with high bits
/// set (IEEE: low 8 bits) — must not degrade to NUL under the stricter lane.
#[test]
fn percent_c_uses_low_byte_of_wide_value() {
    let out = run(r#"
module t;
  reg [127:0] ch;
  initial begin
    ch = 128'hdead_beef_0000_0000_0000_0000_0000_0041; // low byte = 'A'
    $display("c=%c", ch);
  end
endmodule
"#);
    assert_eq!(out.trim(), "c=A");
}
