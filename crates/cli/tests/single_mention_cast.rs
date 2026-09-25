//! Single-mention 2-state coercion and sign extension in the cast and bind lanes.
//!
//! The engine walks the expression DAG as a TREE, so every node that names an
//! operand evaluates it again: a user call runs its body (and its `$display`s) once
//! per mention and `$random` draws once per mention. The 2-state coercion used to be
//! a per-bit `Concat` of `CaseEq(Select(e, i), 1'b1)` (one mention per result bit)
//! and a signed widening built `extend_to`'s `Select{Bit}` sign fill (a second
//! mention). Measured on the pre-change binary: `int'(f())` with a `$display` in
//! `f` called it 8 times and printed `000000fd`; `int'($random)` drew 32 times;
//! `int'($random * 1.0)` drew 4 times through the real→int composition.
//!
//! The lanes now build `SysFunc TwoState` (x/z→0, one mention) and the ternary
//! `$signed(1'b1 ? $signed(e) : <n-bit signed 0>)` for a signed widening of an
//! operand that may not be repeated (one mention, 4-state preserved); a real
//! operand that may not be repeated converts through `SysFunc RealToInt`.
//!
//! Every expected line is live iverilog 13 output (verilator 5.052 agrees except
//! where its `$random` stream or 2-state storage differs, noted per test), unless
//! the test says otherwise.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `src` and return its display lines (diagnostics and the end banner dropped).
fn run_with(src: &str, args: &[&str]) -> Vec<String> {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_single_mention_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{text}");
    text.lines()
        .filter(|l| {
            !(l.starts_with("warning[")
                || l.starts_with("note[")
                || l.starts_with("simulation ended")
                || l.starts_with("errors="))
        })
        .map(str::to_owned)
        .collect()
}

fn run(src: &str) -> Vec<String> {
    run_with(src, &[])
}

fn check(src: &str, want: &[&str]) {
    let got = run(src);
    assert_eq!(got, want, "full output:\n{}", got.join("\n"));
}

/// One `f` line per cast, as both oracles print. Before: `int'(f())` 8 calls and
/// `000000fd`, `longint'` 8 calls, `byte'` 8, `int'(int'(f()))` 8 and `000000fd`,
/// `16'(f())` `00fd`, `64'(fi())` `00000000fffffffd`, `shortint'(f4())` 4 calls,
/// `integer'(f())` `000000fd`.
#[test]
fn a_call_operand_is_evaluated_once_and_sign_extended() {
    check(
        r#"module top;
  function logic signed [7:0] f(); $display("f"); return -3; endfunction
  function int fi(); $display("fi"); return -3; endfunction
  function logic [3:0] f4(); $display("f4"); return 4'b1010; endfunction
  int r; longint rl; byte rb; logic [15:0] r16; shortint s;
  initial begin
    r = int'(f());       $display("int'(f)=%h", r);
    rl = longint'(f());  $display("longint'(f)=%h", rl);
    rb = byte'(f());     $display("byte'(f)=%h", rb);
    r = int'(int'(f())); $display("int'(int'(f))=%h", r);
    r16 = 16'(f());      $display("16'(f)=%h", r16);
    rl = 64'(fi());      $display("64'(fi)=%h", rl);
    s = shortint'(f4()); $display("shortint'(f4)=%h", s);
    r = integer'(f());   $display("integer'(f)=%h", r);
    #1 $finish;
  end
endmodule
"#,
        &[
            "f",
            "int'(f)=fffffffd",
            "f",
            "longint'(f)=fffffffffffffffd",
            "f",
            "byte'(f)=fd",
            "f",
            "int'(int'(f))=fffffffd",
            "f",
            "16'(f)=fffd",
            "fi",
            "64'(fi)=fffffffffffffffd",
            "f4",
            "shortint'(f4)=000a",
            "f",
            "integer'(f)=fffffffd",
        ],
    );
}

/// Before: 32 draws, `800fde20`, next `557845aa`. Verilator's `$random` stream
/// differs, so iverilog alone is the oracle for the drawn values.
#[test]
fn a_random_operand_draws_once_through_int_cast() {
    check(
        r#"module top;
  int a, b; longint c; byte d; logic [31:0] e; shortint s;
  function automatic longint flong(input longint x); return x; endfunction
  function automatic logic [71:0] fb72(input logic signed [71:0] x); return x; endfunction
  function automatic int fint(input int x); return x; endfunction
  initial begin
    b = $random; b = $random; // skip to draw 3 (8484d609, negative)
    a = int'($random);  $display("int'($random)=%h", a);
    b = $random; $display("next=%h", b);
    #1 $finish;
  end
endmodule
"#,
        &["int'($random)=8484d609", "next=b1f05663"],
    );
}

/// Before: `00000000800fde20` (zero-extended, many draws). iverilog only (stream).
#[test]
fn a_random_operand_draws_once_through_longint_cast() {
    check(
        r#"module top;
  int b; longint c;
  function automatic longint flong(input longint x); return x; endfunction
  initial begin
    b = $random; b = $random;
    c = longint'($random); $display("longint'($random)=%h", c);
    b = $random; $display("next=%h", b);
    #1 $finish;
  end
endmodule
"#,
        &["longint'($random)=ffffffff8484d609", "next=b1f05663"],
    );
}

/// Before: `byte'($random)` drew 8 times. iverilog only (stream).
#[test]
fn a_random_operand_draws_once_through_byte_cast() {
    check(
        r#"module top;
  int b; byte d; logic [31:0] e;
  initial begin
    b = $random; b = $random;
    d = byte'($random); $display("byte'($random)=%h", d);
    b = $random; $display("next=%h", b);
    e = 32'($random); $display("32'($random)=%h", e);
    b = $random; $display("next=%h", b);
    #1 $finish;
  end
endmodule
"#,
        &[
            "byte'($random)=09",
            "next=b1f05663",
            "32'($random)=06b97b0d",
            "next=46df998d",
        ],
    );
}

/// `int'($random * 1.0)` converts through `RealToInt`, one mention. Before: 4 draws,
/// `c0895e81`, next `00f3e301`. iverilog only (stream).
#[test]
fn a_random_real_operand_draws_once_through_int_cast() {
    check(
        r#"module top;
  int a, b;
  initial begin
    a = int'($random*1.0); $display("int'($random*1.0)=%h", a);
    b = $random; $display("next=%h", b);
    #1 $finish;
  end
endmodule
"#,
        &["int'($random*1.0)=12153524", "next=c0895e81"],
    );
}

/// A real-returning call and a `$random`-bearing real product converted to a
/// ≤32-bit target, one evaluation each (`RealToInt`). Before: `int'(rf())` 8
/// calls, `byte'(rf())` 8, `int'(rp())` 5, `int'(-rp())` 8, `integer'(rf())` 8,
/// `byte'($random*0.5)` `40` next `00f3e301`. The call lines agree with
/// verilator; the `$random` line is iverilog only (stream).
#[test]
fn a_real_call_operand_is_called_once() {
    check(
        r#"module top;
  function real rf(); $display("rf"); return -300.7; endfunction
  function real rp(); $display("rp"); return 2.5; endfunction
  int a; longint l; byte b; shortint s; logic [31:0] u; int n;
  initial begin
    a = int'(rf()); $display("int'(rf)=%h", a);
    b = byte'(rf()); $display("byte'(rf)=%h", b);
    a = int'(rp()); $display("int'(rp)=%h", a);
    a = int'(-rp()); $display("int'(-rp)=%h", a);
    b = byte'($random * 0.5); n = $random; $display("byte'($random*0.5)=%h next=%h", b, n);
    a = integer'(rf()); $display("integer'(rf)=%h", a);
    #1 $finish;
  end
endmodule
"#,
        &[
            "rf",
            "int'(rf)=fffffed3",
            "rf",
            "byte'(rf)=d3",
            "rp",
            "int'(rp)=00000003",
            "rp",
            "int'(-rp)=fffffffd",
            "byte'($random*0.5)=92 next=c0895e81",
            "rf",
            "integer'(rf)=fffffed3",
        ],
    );
}

/// ⚠️ PRE-identical and WRONG (ROADMAP §2 Real row): a >32-bit target of a
/// non-repeatable real keeps the multi-mention composition, because `RealToInt`
/// saturates at |x| ≥ 2^127 where the composition answers 0 like both oracles.
/// Both oracles call `rf` once and print `longint'($random*1.0)=0000000012153524
/// next=c0895e81` (iverilog; verilator's stream differs). The values below are
/// vita's, unchanged by this change: 24 calls, and a value built from several
/// draws.
#[test]
fn a_wide_real_cast_of_a_call_keeps_the_composition() {
    let mut want: Vec<&str> = vec!["rf"; 24];
    want.push("longint'(rf)=fffffffffffffed3");
    want.push("longint'($random*1.0)=ffffffff06d7cd0d next=47ecdb8f");
    check(
        r#"module top;
  function real rf(); $display("rf"); return -300.7; endfunction
  longint l; int n;
  initial begin
    l = longint'(rf()); $display("longint'(rf)=%h", l);
    l = longint'($random * 1.0); n = $random; $display("longint'($random*1.0)=%h next=%h", l, n);
    #1 $finish;
  end
endmodule
"#,
        &want,
    );
}

/// Before: `int'(int'($random))` `800fde20` next `557845aa`; `int'($signed(f()))`
/// 9 calls; `longint'(int'(f()))` 16 calls and `00000000000000fd`;
/// `longint'(byte'(f()))` 16 calls; `shortint'(f())` 8 calls and `000000fd`.
/// The `$random` line is iverilog only.
#[test]
fn nested_and_signed_call_operands_are_evaluated_once() {
    check(
        r#"module top;
  function logic signed [7:0] f(); $display("f"); return -3; endfunction
  int a, n; longint l;
  initial begin
    n = $random; n = $random;
    a = int'(int'($random)); n = $random; $display("int'(int'($random))=%h next=%h", a, n);
    a = int'($signed(f())); $display("int'($signed(f))=%h", a);
    l = longint'(int'(f())); $display("longint'(int'(f))=%h", l);
    l = longint'(byte'(f())); $display("longint'(byte'(f))=%h", l);
    a = shortint'(f()); $display("shortint'(f)=%h", a);
    #1 $finish;
  end
endmodule
"#,
        &[
            "int'(int'($random))=8484d609 next=b1f05663",
            "f",
            "int'($signed(f))=fffffffd",
            "f",
            "longint'(int'(f))=fffffffffffffffd",
            "f",
            "longint'(byte'(f))=fffffffffffffffd",
            "f",
            "shortint'(f)=fffffffd",
        ],
    );
}

/// The hazard `cast_extend_signed` used to fall back to the mirror for: a signed
/// call widened by a size cast. Before: `16'(fp(0))` `00fd`, and
/// `16'($signed(fp(0)))` called `fp` twice. The `16'(fur(0))`
/// lines wrap `$urandom`, whose stream vita does not share with either oracle, so
/// they are pinned as vita's values (identical before and after: one draw per
/// cast, the `next` draws unshifted).
#[test]
fn a_size_cast_of_a_signed_call_extends_by_its_sign_once() {
    check(
        r#"module top;
  function automatic logic signed [7:0] fp(input int x); $display("fp"); return -3 + x; endfunction
  function automatic logic signed [7:0] fur(input int x); return $urandom; endfunction
  logic [15:0] r; int u;
  initial begin
    r = 16'(fp(0)); $display("16'(fp(0))=%h", r);
    r = 16'(fur(0)); u = $urandom; $display("16'(fur(0))=%h next=%h", r, u);
    r = 16'(fur(0)); u = $urandom; $display("16'(fur(0))=%h next=%h", r, u);
    r = 16'($signed(fp(0))); $display("16'($signed(fp(0)))=%h", r);
    #1 $finish;
  end
endmodule
"#,
        &[
            "fp",
            "16'(fp(0))=fffd",
            "16'(fur(0))=0039 next=6e789e6a",
            "16'(fur(0))=0018 next=f88bb8a8",
            "fp",
            "16'($signed(fp(0)))=fffd",
        ],
    );
}

/// 4-state operands through 2-state casts: unchanged, both oracles.
#[test]
fn x_and_z_operands_coerce_before_they_extend() {
    check(
        r#"module top;
  logic signed [7:0] sx = 8'b1x0z0101; logic [7:0] ux = 8'b1x0z0101;
  logic signed [7:0] sz = 8'bx1010101;
  int a; longint b; byte c; shortint d;
  initial begin
    a = int'(sx); $display("int'(sx)=%h", a);
    b = longint'(sx); $display("longint'(sx)=%h", b);
    a = int'(ux); $display("int'(ux)=%h", a);
    c = byte'(sx); $display("byte'(sx)=%h", c);
    a = int'(sz); $display("int'(sz)=%h", a);
    d = shortint'(sz); $display("shortint'(sz)=%h", d);
    a = int'(int'(sx)); $display("int'(int'(sx))=%h", a);
    #1 $finish;
  end
endmodule
"#,
        &[
            "int'(sx)=ffffff85",
            "longint'(sx)=ffffffffffffff85",
            "int'(ux)=00000085",
            "byte'(sx)=85",
            "int'(sz)=00000055",
            "shortint'(sz)=0055",
            "int'(int'(sx))=ffffff85",
        ],
    );
}

/// 4-state actuals bound to 2-state formals (the inline bind lane): unchanged,
/// both oracles.
#[test]
fn x_and_z_actuals_coerce_through_two_state_formals() {
    check(
        r#"module top;
  logic signed [7:0] sx = 8'b1x0z0101; logic [3:0] u4 = 4'b1x01; logic signed [3:0] s4 = 4'b1x01;
  function automatic int fi(input int x); return x; endfunction
  function automatic longint fl(input longint x); return x; endfunction
  function automatic bit [15:0] b16(input bit [15:0] x); return x; endfunction
  function automatic bit signed [15:0] bs16(input bit signed [15:0] x); return x; endfunction
  function automatic byte fb(input byte x); return x; endfunction
  int a; longint b; logic [15:0] c; byte d;
  initial begin
    a = fi(sx); $display("fi(sx)=%h", a);
    b = fl(sx); $display("fl(sx)=%h", b);
    c = b16(u4); $display("b16(u4)=%h", c);
    c = bs16(s4); $display("bs16(s4)=%h", c);
    c = b16(s4); $display("b16(s4)=%h", c);
    d = fb(sx); $display("fb(sx)=%h", d);
    a = fi(s4); $display("fi(s4)=%h", a);
    #1 $finish;
  end
endmodule
"#,
        &[
            "fi(sx)=ffffff85",
            "fl(sx)=ffffffffffffff85",
            "b16(u4)=0009",
            "bs16(s4)=fff9",
            "b16(s4)=fff9",
            "fb(sx)=85",
            "fi(s4)=fffffff9",
        ],
    );
}

/// The representation itself, spelled by hand: one call, one draw, and an x or z
/// MSB extends as x like the plain assignment. iverilog 13 (verilator is 2-state
/// here and its `$random` stream differs, so it is not an oracle for this cell).
#[test]
fn the_hand_spelled_ternary_extends_once_and_keeps_x() {
    check(
        r#"module top;
  function logic signed [7:0] f(); $display("f"); return -3; endfunction
  logic signed [7:0] sx = 8'b1x0z0101; logic signed [7:0] sz = 8'bx1010101;
  int a, b; longint c; logic [31:0] l; logic [63:0] l64;
  initial begin
    a = $signed(1'b1 ? $signed(f()) : 32'sd0); $display("tern(f)=%h", a);
    l = $signed(1'b1 ? sx : 32'sd0); $display("tern(sx)=%h", l);
    l = sx; $display("assign(sx)=%h", l);
    l = $signed(1'b1 ? sz : 32'sd0); $display("tern(sz)=%h", l);
    l = sz; $display("assign(sz)=%h", l);
    b = $random; b = $random;
    c = $signed(1'b1 ? $random : 64'sd0); $display("tern($random)=%h", c);
    b = $random; $display("next=%h", b);
    l64 = $signed(1'b1 ? sx : 64'sd0); $display("tern64(sx)=%h", l64);
    l = $unsigned(1'b1 ? $unsigned(sx) : 32'd0); $display("uternU(sx)=%h", l);
    #1 $finish;
  end
endmodule
"#,
        &[
            "f",
            "tern(f)=fffffffd",
            "tern(sx)=ffffffX5",
            "assign(sx)=ffffffX5",
            "tern(sz)=xxxxxxX5",
            "assign(sz)=xxxxxxX5",
            "tern($random)=ffffffff8484d609",
            "next=b1f05663",
            "tern64(sx)=ffffffffffffffX5",
            "uternU(sx)=000000X5",
        ],
    );
}

/// Already one draw before this change; pinned so it stays. iverilog only.
#[test]
fn random_actuals_bound_to_wide_formals_draw_once() {
    check(
        r#"module top;
  int b; longint c; logic [71:0] w;
  function automatic longint flong(input longint x); return x; endfunction
  function automatic logic [71:0] fb72(input logic signed [71:0] x); return x; endfunction
  initial begin
    b = $random; b = $random;
    c = flong($random); $display("flong($random)=%h", c);
    b = $random; $display("next=%h", b);
    w = fb72($random); $display("fb72($random)=%h", w);
    b = $random; $display("next=%h", b);
    #1 $finish;
  end
endmodule
"#,
        &[
            "flong($random)=ffffffff8484d609",
            "next=b1f05663",
            "fb72($random)=000000000006b97b0d",
            "next=46df998d",
        ],
    );
}

/// `fb72($random)` with a NEGATIVE draw (`c0895e81`, the second draw) sign-extends
/// into the 72-bit signed formal with one draw. iverilog only (stream).
#[test]
fn a_negative_random_actual_sign_extends_into_a_wide_formal() {
    check(
        r#"module top;
  function automatic logic [71:0] fb72(input logic signed [71:0] x); return x; endfunction
  function automatic longint flong(input longint x); return x; endfunction
  logic [71:0] b; longint a; int n;
  initial begin
    n = $random;
    b = fb72($random); n = $random; $display("fb72($random)=%h next=%h", b, n);
    n = $random;
    a = flong($random); n = $random; $display("flong($random)=%h next=%h", a, n);
    #1 $finish;
  end
endmodule
"#,
        &[
            "fb72($random)=ffffffffffc0895e81 next=8484d609",
            "flong($random)=0000000006b97b0d next=46df998d",
        ],
    );
}

/// Operands whose width `ir_bits_of` does not know (`q.sum()`) or knows from a
/// hierarchical placeholder. The hierarchical lines agree with both oracles.
/// ⚠️ The `q.sum` lines are PRE-identical and WRONG (verilator 5.052 prints
/// `fffffffffffffffd`; iverilog 13 rejects `q.sum()` on a queue): the width is
/// fabricated, so the single-mention ternary may not be used, and `extend_to`'s
/// sign fill would name the non-repeatable operand twice, so the mirror's
/// unsigned answer is kept (ROADMAP §2).
#[test]
fn fabricated_width_operands_extend_by_their_sign() {
    check(
        r#"module sub;
  logic signed [15:0] s = -16'sd3;
  logic signed [7:0] s8 = 8'sb1x0z0101;
endmodule
module top;
  sub u1();
  byte signed q[$];
  logic [63:0] a; logic [31:0] b;
  initial begin
    q.push_back(-8'sd3); q.push_back(8'sd0);
    #1;
    a = longint'(q.sum()); $display("longint'(q.sum)=%h", a);
    a = 64'(q.sum()); $display("64'(q.sum)=%h", a);
    b = 32'(u1.s); $display("32'(u1.s)=%h", b);
    a = 64'(u1.s); $display("64'(u1.s)=%h", a);
    a = longint'(u1.s); $display("longint'(u1.s)=%h", a);
    a = longint'(u1.s8); $display("longint'(u1.s8)=%h", a);
    b = int'(u1.s8); $display("int'(u1.s8)=%h", b);
    #1 $finish;
  end
endmodule
"#,
        &[
            "longint'(q.sum)=00000000000000fd",
            "64'(q.sum)=00000000000000fd",
            "32'(u1.s)=fffffffd",
            "64'(u1.s)=fffffffffffffffd",
            "longint'(u1.s)=fffffffffffffffd",
            "longint'(u1.s8)=ffffffffffffff85",
            "int'(u1.s8)=ffffff85",
        ],
    );
}

/// A different defect, NOT closed here (ROADMAP §2): a context-determined operand
/// of a prim cast is evaluated self-determined. Pinned at the pre-change values so
/// this change is seen not to move it. Oracles (iverilog 13 and verilator 5.052):
/// `int'(-u4)=fffffff6`, `int'(-f4)=fffffff6`, `int'(~u4)=fffffff5`,
/// `int'(u4+8)=00000012`, `int'(f4+8)=00000012`.
#[test]
fn context_determined_operands_are_unchanged() {
    check(
        r#"module top;
  logic [3:0] u4 = 4'b1010; logic signed [3:0] s4 = 4'sb1010;
  function logic [3:0] f4(); return 4'b1010; endfunction
  int a; logic [15:0] r16;
  initial begin
    a = int'(-u4);  $display("int'(-u4)=%h", a);
    a = int'(-f4()); $display("int'(-f4)=%h", a);
    a = int'(-s4);  $display("int'(-s4)=%h", a);
    r16 = 16'(-u4); $display("16'(-u4)=%h", r16);
    a = int'(~u4);  $display("int'(~u4)=%h", a);
    a = int'(u4 + 4'd8); $display("int'(u4+8)=%h", a);
    a = int'(f4() + 4'd8); $display("int'(f4+8)=%h", a);
    #1 $finish;
  end
endmodule
"#,
        &[
            "int'(-u4)=00000006",
            "int'(-f4)=00000006",
            "int'(-s4)=00000006",
            "16'(-u4)=fff6",
            "int'(~u4)=00000005",
            "int'(u4+8)=00000002",
            "int'(f4+8)=00000002",
        ],
    );
}

/// Widening casts over NETS in continuous assigns and an `always_comb`, on every
/// backend. iverilog 13 (verilator agrees except `y4`, where it is 2-state). Before:
/// `y3`/`c1` `000000fd` and `y4` `00fd` (the call operand zero-extended).
#[test]
fn widening_casts_over_nets_agree_on_every_backend() {
    let src = r#"module top;
  logic signed [7:0] s;
  logic [3:0] u;
  function automatic logic signed [7:0] fs(input logic signed [7:0] x); return x; endfunction
  wire [31:0] y1 = int'(s);
  wire [63:0] y2 = longint'(s);
  wire [31:0] y3 = int'(fs(s));
  wire [15:0] y4 = 16'(fs(s));
  wire [31:0] y5 = int'(u);
  logic [31:0] c1; logic [63:0] c2;
  always_comb begin c1 = int'(fs(s)); c2 = longint'(s); end
  initial begin
    s = -8'sd3; u = 4'b1x01; #1;
    $display("y1=%h y2=%h y3=%h y4=%h y5=%h c1=%h c2=%h", y1, y2, y3, y4, y5, c1, c2);
    s = 8'sb1x0z0101; #1;
    $display("y1=%h y2=%h y3=%h y4=%h y5=%h c1=%h c2=%h", y1, y2, y3, y4, y5, c1, c2);
    s = 8'sd5; #1;
    $display("y1=%h y2=%h y3=%h y4=%h y5=%h c1=%h c2=%h", y1, y2, y3, y4, y5, c1, c2);
    #1 $finish;
  end
endmodule
"#;
    let want = [
        "y1=fffffffd y2=fffffffffffffffd y3=fffffffd y4=fffd y5=00000009 c1=fffffffd c2=fffffffffffffffd",
        "y1=ffffff85 y2=ffffffffffffff85 y3=ffffff85 y4=ffX5 y5=00000009 c1=ffffff85 c2=ffffffffffffff85",
        "y1=00000005 y2=0000000000000005 y3=00000005 y4=0005 y5=00000009 c1=00000005 c2=0000000000000005",
    ];
    for backend in ["interp", "vm", "native"] {
        let got = run_with(src, &["--backend", backend]);
        assert_eq!(got, want, "backend {backend}:\n{}", got.join("\n"));
    }
}

/// A FABRICATED operand width (`ir_bits_of` answers `None`) keeps the per-bit
/// coercion, whose `Concat` carries the cast's declared width. `TwoState` passes
/// the unknown width through: a first cut made `$bits(int'(q.sum()))` an E3009,
/// `int'(qu8.sum())` `fffffffd`, and truncated `48'(int'(s))`. Values are
/// verilator 5.052's (iverilog 13 rejects `sum()` on a queue); every line is also
/// the pre-change binary's.
#[test]
fn a_fabricated_width_keeps_the_declared_cast_width() {
    check(
        r#"module top;
  byte unsigned qu8[$];
  byte signed q[$];
  logic [47:0] q48[$];
  string s = "abcd";
  int a; logic [47:0] w48; logic [39:0] w40;
  initial begin
    q.push_back(-8'sd3); q.push_back(8'sd0);
    qu8.push_back(8'hfd); qu8.push_back(8'h00);
    q48.push_back(48'h8000_0000_0001);
    $display("bits=%0d", $bits(int'(q.sum())));
    a = int'(qu8.sum()); $display("int'(qu8.sum)=%h", a);
    w48 = 48'(int'(s)); $display("48'(int'(s))=%h", w48);
    w40 = 40'(q48.sum()); $display("40'(q48.sum)=%h", w40);
    #1 $finish;
  end
endmodule
"#,
        &[
            "bits=32",
            "int'(qu8.sum)=000000fd",
            "48'(int'(s))=000061626364",
            "40'(q48.sum)=0000000001",
        ],
    );
}

/// Real → integral casts out of range. `longint'(fr(1e40))` is 0 for both
/// oracles; `RealToInt` saturates at |x| ≥ 2^127 and printed `ffffffffffffffff`
/// in a first cut, so a wider-than-32-bit target of a non-repeatable real keeps
/// the multi-mention composition. ⚠️ The `int'` lines are PRE-identical and
/// WRONG: both oracles print `00000000`, vita `ffffffff`, repeatable operand or
/// not (ROADMAP §2 Real row).
#[test]
fn an_out_of_range_real_converts_like_the_repeatable_spelling() {
    check(
        r#"module top;
  function real fr(input real x); return x; endfunction
  real r = 1e40;
  int a; longint l;
  initial begin
    l = longint'(fr(1e40)); $display("longint'(fr(1e40))=%h", l);
    a = int'(fr(1e40)); $display("int'(fr(1e40))=%h", a);
    l = longint'(r); $display("longint'(r)=%h", l);
    a = int'(r); $display("int'(r)=%h", a);
    l = longint'(fr(-300.7)); $display("longint'(fr(-300.7))=%h", l);
    #1 $finish;
  end
endmodule
"#,
        &[
            "longint'(fr(1e40))=0000000000000000",
            "int'(fr(1e40))=ffffffff",
            "longint'(r)=0000000000000000",
            "int'(r)=ffffffff",
            "longint'(fr(-300.7))=fffffffffffffed3",
        ],
    );
}
