//! An inline (SSA-fold) call binds each actual to a formal that is a VARIABLE OF
//! ITS DECLARED TYPE (§13.4.3), and the bind is an ASSIGNMENT to it (§13.5.3).
//! The inline path substitutes the actual's ExprId for the formal's NAME, so
//! nothing about the declared type reached the body: the frame path gets the
//! same three properties for free, because its formal IS a net.
//!
//! Three properties, and they are each other's preconditions:
//!   WIDTH (§11.6.2) — a wide actual truncates, a narrow one extends by the
//!                     ACTUAL's own sign (§11.6.1)
//!   SIGN            — what makes `x/3` a signed divide in the body
//!   2-STATE (§6.11.1) — x/z store as 0 in a `bit`/`byte`/`int`/… formal
//!
//! §4.5.323 built this three times applying a SUBSET and measured a regression
//! each time, then reverted: applying a subset is worse than applying none. So
//! the whole assignment rides ONE gate — a trustworthy actual width — and every
//! shape without one keeps the pre-slice behavior verbatim.
//!
//! A 1,920-cell sweep (12 formal types x 8 bodies x 10 actuals x 2 return types)
//! measured FIXED 636, REGRESSED 0, still-wrong 8 against iverilog 13 — and the
//! 8 are a DIFFERENT funnel (the frame arg bind, see `a_frame_bound_formal_is_a_
//! separate_funnel`), unchanged by this slice. ⚠️ That sweep's actual axis had no
//! `real` actual and no hierarchical reference; adversarial review found a
//! regression in the first (now `a_real_actual_is_never_bit_coerced_into_a_2_
//! state_formal`) and a pre-existing gap in the second (ROADMAP §2).
//!
//! ORACLE: iverilog 13.0 — every value below was run through it, and the PRE
//! value in each comment was measured on a binary built from the parent commit.
//! Three rows deliberately assert something OTHER than iverilog's answer, and
//! each says so at the assert: the impure-actual value, the real-returning-call
//! value, and the class-field value are documented gaps carried by ROADMAP §2.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_args(src: &str, args: &[&str]) -> (String, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ifb_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_vita"));
    for a in args {
        c.arg(a);
    }
    let out = c
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let so = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n");
    let se = String::from_utf8_lossy(&out.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&d);
    (so, se)
}

fn run(src: &str) -> String {
    run_args(src, &[]).0
}

/// ① SIGN. The headline: the body reads the formal, and the formal's declared
/// signedness is what picks the OPERATION. Each of these three is a different
/// operator reading the same bits, so a fix that only moved the value would
/// leave at least one of them wrong.
#[test]
fn the_formals_signedness_picks_the_bodys_operation() {
    let o = run(r#"module t;
  function signed [7:0] fdiv(input signed [7:0] x); fdiv = x/3;    endfunction
  function signed [7:0] fsar(input signed [7:0] x); fsar = x>>>1;  endfunction
  function               fgt(input signed [7:0] x); fgt  = x>1;    endfunction
  initial begin
    $display("%0d %0d %0d", fdiv(8'hf7), fsar(8'hf7), fgt(8'hf7));
    #1 $finish;
  end
endmodule
"#);
    // PRE printed `82 123 1` — the body divided, shifted and compared 247.
    assert_eq!(o, "-3 -5 0");
}

/// ① SIGN, carried by the KIND rather than a `signed` keyword. `byte` is signed
/// by §6.11.1 and `byte unsigned` is not, so the two rows must split — a fix that
/// read `p.signed` alone (the syntactic qualifier) gets both wrong.
#[test]
fn a_2_state_atom_type_carries_its_own_signedness() {
    let o = run(r#"module t;
  function [7:0] fs(input byte          x); fs = x/3; endfunction
  function [7:0] fu(input byte unsigned x); fu = x/3; endfunction
  initial begin
    $display("%0d %0d", fs(8'hf7), fu(8'hf7));
    #1 $finish;
  end
endmodule
"#);
    // PRE printed `82 82`: both read 247. iverilog: -3 truncated into the
    // unsigned 8-bit return is 253, and 247/3 is 82.
    assert_eq!(o, "253 82");
}

/// ② WIDTH, narrowing. §4.5.323 left a wide/exact actual bound VERBATIM because
/// truncating it "would flip a wider-context shift" — that claim was measured
/// false here: `shft(16'hff00)` is the very shape it named, and truncation is
/// what makes it right.
#[test]
fn a_wide_actual_truncates_to_the_formals_width() {
    let o = run(r#"module t;
  function [15:0] fnar (input signed [3:0] x); fnar  = x; endfunction
  function [15:0] funar(input        [3:0] x); funar = x; endfunction
  function  [7:0] shft (input        [7:0] c); shft  = c >> 1; endfunction
  initial begin
    $display("%h %h %h", fnar(8'hf7), funar(8'hf7), shft(16'hff00));
    #1 $finish;
  end
endmodule
"#);
    // PRE printed `00f7 00f7 80` — no truncation at all, so `shft` shifted 0xff00
    // and kept the low byte of the result.
    assert_eq!(o, "0007 0007 00");
}

/// ② WIDTH, widening — and the one direction that was ALREADY right, kept as a
/// guard. The extension follows the ACTUAL's sign (§11.6.1), not the formal's:
/// a signed 8-bit actual sign-extends into an UNSIGNED 16-bit formal.
#[test]
fn a_narrow_actual_extends_by_its_own_sign_not_the_formals() {
    let o = run(r#"module t;
  function [15:0] fw(input [15:0] x); fw = x; endfunction
  function [15:0] fn(input signed [3:0] x); fn = x; endfunction
  initial begin
    $display("%h %h", fw(8'shf7), fn(4'shb));
    #1 $finish;
  end
endmodule
"#);
    // Both were already correct in PRE and must stay so: a stamp of the FORMAL's
    // sign onto the extension direction would make row 1 `00f7`.
    assert_eq!(o, "fff7 fffb");
}

/// ③ 2-STATE (§6.11.1). Both rows carry x in the actual; the formal's type is
/// the only thing that decides whether it survives.
#[test]
fn a_2_state_formal_drops_x_and_z() {
    let o = run(r#"module t;
  function  [7:0] fbit(input bit [7:0] x); fbit = x; endfunction
  function [31:0] fint(input int       x); fint = x; endfunction
  function  [7:0] flog(input logic [7:0] x); flog = x; endfunction
  initial begin
    $display("%h %h %h", fbit(8'hx7), fint(32'hxxxx_00f7), flog(8'hx7));
    #1 $finish;
  end
endmodule
"#);
    // PRE printed `x7 xxxx00f7 x7`. Row 3 is the control: a 4-state formal KEEPS
    // the x, so a coercion applied to every formal would break it.
    assert_eq!(o, "07 000000f7 x7");
}

/// The three properties interacting. A nested inline call rebinds the OUTER
/// formal's value into an inner formal: §4.5.323 round 1 applied the sign alone
/// and this shape read un-truncated high bits under the new sign.
#[test]
fn a_nested_rebind_applies_both_formals() {
    let o = run(r#"module t;
  function [15:0] inner(input signed [3:0] y); inner = y*2; endfunction
  function [15:0] outer(input signed [7:0] x); outer = inner(x); endfunction
  initial begin
    $display("%0d", outer(8'shf7));
    #1 $finish;
  end
endmodule
"#);
    // PRE printed `65518`: -9 reached `inner` un-truncated. iverilog truncates
    // -9 to 4 bits (7), then doubles it.
    assert_eq!(o, "14");
}

/// The §4.5.323 round-2 regression, permanently. Truncating a constant actual
/// FOLDS it, and a folded constant lands on §4.5.310's constant-index carve-out;
/// before §4.5.324 read that carve-out's index with its sign, this turned a loud
/// out-of-range read into a silent neighbour. Row 1 is in range (an UNSIGNED
/// return makes -1 into 255), row 2 is not and must stay loud.
#[test]
fn a_truncated_constant_actual_reaches_the_index_carve_out_safely() {
    let (o, e) = run_args(
        r#"module t;
  logic [7:0] arr [0:255];
  logic [7:0] sm  [0:7];
  integer i;
  function [7:0] idx(input signed [7:0] k); idx = k; endfunction
  initial begin
    for (i = 0; i < 256; i = i + 1) arr[i] = i[7:0];
    for (i = 0; i < 8;   i = i + 1) sm[i]  = i[7:0] | 8'hA0;
    $display("%h", arr[idx(-8'sd1)]);
    $display("%h", sm[idx(-3'sd1)]);
    #1 $finish;
  end
endmodule
"#,
        &[],
    );
    assert_eq!(o, "ff\nxx");
    assert!(e.contains("E4002"), "row 2 must stay loud, got: {e}");
}

/// Non-bit-vector formals and actuals are bound verbatim — the kind
/// discriminator has to precede any width work. §4.5.323 round 2 split the two
/// resize directions across two primitives and the narrowing one chopped a real
/// actual's IEEE-754 payload (`freal(3.7)` printed 154).
#[test]
fn a_real_or_string_bind_is_never_bit_resized() {
    let o = run(r#"module t;
  function [31:0] freal(input [15:0] x); freal = x+1;  endfunction
  function  [7:0] frl  (input real    r); frl   = r/2;  endfunction
  function integer flen(input string s); flen = s.len(); endfunction
  initial begin
    $display("%0d %0d %0d", freal(3.7), frl(9.0), flen("abcd"));
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "5 5 4");
}

/// ⚠️ THE REGRESSION THIS SLICE ALMOST SHIPPED. A signed result is safe to BUILD
/// and unsafe to CONSUME: it tells every downstream widening resize to sign-FILL,
/// and `extend_to` builds that fill as a SECOND MENTION of the operand (§4.5.320
/// S1). The 2-state coercion is the same hazard w-fold — it names its operand once
/// per bit. Stamping the formal's sign on `$random` made the widening at the
/// RETURN resize draw TWICE: the value came from the second draw and the whole
/// stream ran one ahead of iverilog's.
///
/// So an actual that cannot be repeated keeps the pre-slice bind. Row 2 is the
/// gap that buys (iverilog truncates to `ffffff81`); rows 1 and 3 are the property
/// — the stream position is iverilog's, exactly one draw per call.
#[test]
fn an_impure_actual_is_never_named_twice() {
    let o = run(r#"module t;
  function [31:0] fb  (input byte      x); fb   = x; endfunction
  function [31:0] pass(input [31:0]    x); pass = x; endfunction
  integer r1, r2, r3;
  initial begin
    r1 = pass($random);
    r2 = fb($random);
    r3 = $random;
    $display("%h %h %h", r1, r2, r3);
    $display("%h", fb(8'hx7));
    #1 $finish;
  end
endmodule
"#);
    // `12153524` and `8484d609` are iverilog's 1st and 3rd draws of the default
    // stream, and vita's raw stream was checked to match it for four draws. Row 4
    // is the other half: a REPEATABLE 2-state actual IS still coerced, so a gate
    // widened to reject everything makes it `xxxxxx07`.
    assert_eq!(o, "12153524 c0895e81 8484d609\n00000007");
}

/// The same hazard through the other impure leaf — a frame `Expr::Call`, whose
/// width MATCHES the formal's, so the bind itself resizes nothing and only the
/// stamp is added. That was enough: the widening happened downstream at the
/// return resize. Row S is the one that drew twice; row U shares the stream and
/// so reports whether row S consumed one draw or two.
#[test]
fn a_frame_call_actual_is_evaluated_once() {
    let o = run(r#"module t;
  function automatic signed [3:0] fs(input d); fs = $random; endfunction
  function [7:0] inl(input signed [3:0] x); inl = x; endfunction
  function [7:0] inu(input        [3:0] x); inu = x; endfunction
  logic [7:0] r; integer probe;
  initial begin
    #1;
    r = inl(fs(0)); probe = $random; $display("S val=%h next=%h", r, probe);
  end
  initial begin
    #2;
    r = inu(fs(0)); probe = $random; $display("U val=%h next=%h", r, probe);
    #1 $finish;
  end
endmodule
"#);
    // iverilog's, exactly. An ungated stamp printed `S val=01 next=8484d609` —
    // the value came from the SECOND draw and every later draw shifted.
    assert_eq!(o, "S val=04 next=c0895e81\nU val=09 next=b1f05663");
}

/// ⚠️ THE OTHER REGRESSION THIS SLICE ALMOST SHIPPED. The 2-state step sits
/// OUTSIDE `resize_inline_assign`, so it does not inherit that function's
/// real/string guards — and `coerce_two_state` read a `real` actual's IEEE-754
/// payload bit by bit. That is CORRECT → SILENT-WRONG: a 40-cell sweep of
/// {8 two-state formals} x {5 real actual forms} regressed, and it is the same
/// symptom §4.5.323 round 2 hit from a different door.
///
/// The guard has to be the AST-aware `cast_operand_is_real`, because a frame
/// `Expr::Call` is opaque to the IR-level `expr_is_real`: a real-returning
/// function's return var is a 64-bit `Reg` net holding the payload, not a
/// `NetKind::Real`. Row `call` is that shape, and it pins the PRE value — a real
/// actual is not CONVERTED to the formal's integer type (iverilog says 5), and
/// refusing to trade one silent-wrong for another is all this guard claims.
#[test]
fn a_real_actual_is_never_bit_coerced_into_a_2_state_formal() {
    let o = run(r#"module t;
  real r;
  function [31:0] fint(input int          x); fint = x; endfunction
  function [31:0] flog(input logic [31:0] x); flog = x; endfunction
  function [31:0] fbyt(input byte         x); fbyt = x; endfunction
  function [63:0] flng(input longint      x); flng = x; endfunction
  function automatic real rf(input d); rf = 4.0; endfunction
  function [7:0] g(input [7:0] x); g = x+1; endfunction
  initial begin
    r = 9.0; $display("%0d %0d %0d", fint(r), flog(r), fbyt(r));
    r = 3.7; $display("%0d %0d", flng(r), fbyt(r));
    $display("%0d", g(rf(0)));
    #1 $finish;
  end
endmodule
"#);
    // Rows 1-2 are iverilog's and were iverilog's in PRE too; an ungated build
    // printed `0 9 0` and `4615514078110652826 154`. Row 3 is PRE's value and
    // iverilog's is 5 (ROADMAP §2: no real→integer conversion at an inline bind).
    assert_eq!(o, "9 9 9\n4 4\n0");
}

/// The ORDER of the width resize and the 2-state coercion. Coercing FIRST loses
/// the actual's signedness — the coerced `Concat` is unsigned, so the widening
/// that follows zero-fills. Every other 2-state row here uses an equal-width
/// actual, which makes the order structurally invisible; this one does not.
///
/// ⚠️ ROUND 36 REFINEMENT, recorded here rather than left to quietly contradict the
/// name: a widening bind now DOES coerce first, for cost (see
/// `a_narrow_widening_bind_coerces_at_the_actuals_width` below). What makes that safe
/// is exactly the hazard this row names — the extension sign is read off the
/// PRE-coercion actual (`expr_self_signed(eid)`), never off the `Concat`. This row's
/// actual is a CONSTANT with no x/z, so no coercion is built for it at all and it
/// still pins the plain resize-then-seal order; the new row pins the reordered one.
#[test]
fn the_width_resize_precedes_the_2_state_coercion() {
    let o = run(r#"module t;
  function [31:0] fb(input bit [15:0] x); fb = x; endfunction
  function [31:0] fi(input int        x); fi = x; endfunction
  initial begin
    $display("%h %h", fb(8'shf7), fi(8'shf7));
    #1 $finish;
  end
endmodule
"#);
    // iverilog. Coerce-before-resize prints `000000f7` in column 1. Column 2 is
    // the anti-vacuity half: at 32 bits nothing truncates, so it must not move.
    assert_eq!(o, "0000fff7 fffffff7");
}

/// The 2-state half of the "name an impure actual only once" trigger. Its
/// sibling row uses `byte`, which is SIGNED — so `formal_signed` alone already
/// gated it and the 2-state term had zero teeth. An UNSIGNED 2-state formal is
/// the only shape that tests it, and there the coercion is the duplicator: 32
/// mentions of `$random` per call.
#[test]
fn an_unsigned_2_state_formal_also_declines_an_impure_actual() {
    let o = run(r#"module t;
  function [31:0] fbu(input bit [31:0]   x); fbu = x; endfunction
  function [31:0] fiu(input int unsigned x); fiu = x; endfunction
  integer a, b, c;
  initial begin
    a = fbu($random); b = fiu($random); c = $random;
    $display("%h %h %h", a, b, c);
    #1 $finish;
  end
endmodule
"#);
    // iverilog's first three draws, in order — one per call. Dropping the 2-state
    // term from the trigger prints `57e5f0de c03b2280 4d9533f9`.
    assert_eq!(o, "12153524 c0895e81 8484d609");
}

/// The predicate that decides whether the coercion is BUILT at all. Its default
/// is "coerce", so the rows that matter are the ones that look known and are not:
/// a zero divisor and an out-of-range constant select both produce x from
/// operands that are themselves known.
#[test]
fn the_coercion_is_built_exactly_where_x_can_arrive() {
    let o = run(r#"module t;
  logic [31:0] u4 = 32'hxxxx_beef;
  int          k2 = 32'h0000_00f7;
  logic [7:0]  d0 = 8'h00;
  logic [7:0]  n8 = 8'hf7;
  function [31:0] fi(input int      x); fi = x; endfunction
  function  [7:0] fb(input byte     x); fb = x; endfunction
  function  [7:0] fs(input shortint x); fs = x; endfunction
  initial begin
    $display("%h %h %h %h", fi(u4), fi(k2), fb(k2[7:0]), fb(u4[7:0]));
    $display("%h %h %h %h", fb(n8/d0), fb(n8[15:8]), fs({u4[7:0],8'h01}), fb(u4[7:0] === 8'hef));
    #1 $finish;
  end
endmodule
"#);
    // iverilog. PRE printed `xxxxbeef` / `xx xx` in the three x-bearing slots.
    // Row 2 col 1 is a divide by zero and col 2 an out-of-range select — both are
    // x out of known-looking operands, so a predicate that called `Div` or a
    // constant `Select` proven-known would print `xx` here.
    assert_eq!(o, "0000beef 000000f7 f7 ef\n00 00 01 01");
}

/// ⚠️ The teeth for the predicate's INTERIOR arms. The row above uses `logic`
/// operands, so its `Signal` arm answers "may be unknown" before any of these arms
/// decides — four mutations (operand-driven `Div`/`Mod`/`Pow`, a `Select` with no
/// static range check, a `Select` admitting `[i +: n]`, a `Ternary` ignoring its
/// condition) survived the whole suite against it. Every operand here is 2-state,
/// so each arm is the only thing standing between the row and an x.
#[test]
fn each_predicate_arm_decides_at_least_one_row() {
    let o = run(r#"module t;
  bit [7:0] b8 = 8'hf7, z8 = 8'h00;
  logic [7:0] lx = 8'hxx;
  bit [3:0] r2;
  localparam int C2 = 2;
  function [7:0] fb(input byte     x); fb = x; endfunction
  function [7:0] fs(input shortint x); fs = x; endfunction
  initial begin
    r2 = 2;
    $display("%h %h %h", fb(b8/z8), fb(b8%z8), fb(b8[9]));
    $display("%h %h", fb(lx[0]?b8:z8), fb(lx[0]?8'h11:8'h22));
    $display("%h %h %h", fs(b8[2 -: 4]), fs(b8[C2 -: 4]), fs(b8[r2 -: 4]));
    #1 $finish;
  end
endmodule
"#);
    // Rows 1-2 are iverilog's; PRE printed `xx xx 0X` and `xX XX`.
    //
    // ⚠️ Row 3 is a HAND-IEEE pin, not an oracle pin: `b8[2 -: 4]` reaches one bit
    // below the vector, so the select is `111x` and §6.11.1 stores it in a
    // `shortint` as `1110`. iverilog prints `0X` — it coerces a LITERAL x into a
    // 2-state formal (this file's `a_2_state_formal_drops_x_and_z` row is
    // iverilog's own answer) but not an out-of-range select's x, so it contradicts
    // itself and cannot arbitrate here.
    //
    // Row 3's first two columns are also why the `PartIdxUp/Down` rejection is not
    // about "a runtime offset" as one might assume — a runtime offset is already
    // caught by the constant fold, and columns 1-2 are constants. The real reason
    // is that the in-range test `off + width <= base_width` measures the wrong end
    // for `-:`.
    assert_eq!(o, "00 00 00\n00 00\n0e 0e 0e");
}

/// The AST actuals are threaded as a THIRD index-parallel slice, and a misaligned
/// index degrades silently: `.get(i)` yields `None`, which falls back to the weaker
/// IR-only real test — the same thing as dropping the AST half. Only a
/// MULTI-ARGUMENT call can see the false-positive direction, where argument 0 reads
/// argument 1's AST, decides "real", and skips a coercion it owed.
#[test]
fn the_ast_actual_is_matched_to_its_own_formal() {
    let o = run(r#"module t;
  logic [31:0] xn = 32'hxxxx_00f7;
  function automatic real arf(input d); arf = 4.0; endfunction
  function [63:0] two(input longint a, input longint b); two = {a[31:0], b[31:0]}; endfunction
  initial begin
    $display("%h", two(xn, arf(0)));
    #1 $finish;
  end
endmodule
"#);
    // The high half is iverilog's (x/z→0 in a `longint`); an off-by-one index makes
    // it `xxxx00f7`, which is PRE's value. The low half is the documented real→
    // integer gap — iverilog converts 4.0 and prints `00000004`.
    assert_eq!(o, "000000f700000000");
}

/// The coercion names its operand once per DECLARED bit, and a nested call feeds
/// one coercion into the next — five levels of `longint` did not finish inside a
/// 120 s cap before the predicate gated it, against 0.02 s after. This runs in
/// well under a second; if it ever hangs, that is the finding.
#[test]
fn a_nested_2_state_bind_does_not_explode() {
    let o = run(r#"module t;
  logic [63:0] acc = 0; int k;
  function [63:0] f0(input longint x); f0 = {x}; endfunction
  function [63:0] f1(input longint x); f1 = {f0(x)}; endfunction
  function [63:0] f2(input longint x); f2 = {f1(x)}; endfunction
  function [63:0] f3(input longint x); f3 = {f2(x)}; endfunction
  function [63:0] f4(input longint x); f4 = {f3(x)}; endfunction
  initial begin
    for (k = 0; k < 200; k = k + 1) acc = acc ^ f4(k[7:0]);
    $display("%h", acc);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "0000000000000000");
}

/// A class-field actual is a DOCUMENTED GAP and this row pins it, so that a
/// mutation removing the trustworthy-width gate is visible. `ir_bits_of`
/// fabricates 32 for a class field (its real width lives in a sidecar next to a
/// 32-bit handle net), and resizing on a fabricated width is a rung down —
/// §4.5.323 round 3 shipped exactly that and turned `xxc3` loose. PRE and POST
/// print the same thing; ROADMAP §2 carries the gap.
#[test]
fn a_class_field_actual_keeps_the_pre_slice_behavior() {
    let o = run(r#"module t;
  class C; byte unsigned bu = 8'hc3; byte sf = 8'hab; endclass
  C c;
  function [15:0] fc(input signed [15:0] x); fc = x; endfunction
  initial begin
    c = new();
    $display("%h %h", fc(c.bu), fc(c.sf));
    #1 $finish;
  end
endmodule
"#);
    // iverilog: `00c3 ffab`. Both PRE and POST print the below — the gate is what
    // keeps this from moving to a DIFFERENT wrong value.
    assert_eq!(o, "xxc3 xxab");
}

/// ⭐ THE OTHER PATH, AND IT IS NOW CLOSED. This cell was written as a known gap — "a
/// FRAME-bound formal has the same §11.6.1 extension defect and this slice does not touch
/// it" — pinning vitamin's wrong `000000f7` / `00f7` against iverilog's `0000fff7` /
/// `fff7` so a later fix would have a starting measurement. It got one.
///
/// The cause was that both frame funnels evaluated the actual with the FORMAL's
/// signedness as the context sign. §11.8.3 gives an assignment-like context its WIDTH but
/// not its SIGN — the right-hand expression's type is the expression's own — so
/// `8'shf7` (−9) sign-extends to `fff7` before the 16-bit unsigned formal receives it.
///
/// ⚠️ It was found by the adversarial review of an unrelated slice (`>>>`'s fill following
/// the result type). That fix made the `AShr` arm honour `ctx_signed` for the first time,
/// which removed the accidental immunity this defect had been hiding behind and turned it
/// from a wrong extension into a wrong `>>>` VALUE. Fixing the context sign closed both.
///
/// Two sites, because the funnels are separate: `eval_core`'s `Expr::Call` arm (frame
/// FUNCTION) and `exec/frame_call.rs`'s `split_in_binds` (frame TASK). Row 3 is the
/// inline path, which was already correct.
#[test]
fn a_frame_bound_formal_takes_the_actuals_own_signedness() {
    let o = run(r#"module t;
  function automatic [31:0] ff(input [15:0] x); ff = x; endfunction
  task     automatic tk(input [15:0] x); $display("%h", x); endtask
  function           inl(input [15:0] x); inl = x[15]; endfunction
  initial begin
    $display("%h", ff(8'shf7));
    tk(8'shf7);
    $display("%0d", inl(8'shf7));
    #1 $finish;
  end
endmodule
"#);
    // All three now match iverilog exactly.
    assert_eq!(o, "0000fff7\nfff7\n1");
}

/// Every backend folds the same IR, so the bind must be invisible to the choice.
#[test]
fn the_three_backends_agree_on_a_bound_formal() {
    let src = r#"module t;
  function signed [7:0] f(input signed [7:0] x); f = x/3; endfunction
  function  [7:0] g(input bit [3:0] x); g = x; endfunction
  reg clk = 0;
  always #1 clk = ~clk;
  always @(posedge clk) $display("%0d %h", f(8'hf7), g(8'hx5));
  initial #5 $finish;
endmodule
"#;
    let a = run_args(src, &["--backend", "interp"]).0;
    let b = run_args(src, &["--backend", "bytecode"]).0;
    let c = run_args(src, &["--backend", "native"]).0;
    assert_eq!(a, b);
    assert_eq!(a, c);
    assert!(a.starts_with("-3 05"), "got {a}");
}

/// ROUND 36 — a narrow actual bound to a WIDER 2-state formal coerces at the
/// ACTUAL's width, not the formal's.
///
/// `coerce_two_state` names its operand once per bit it covers and the engine walks
/// that DAG as a TREE, so binding a 4-bit actual to an `int` formal paid 32
/// evaluations for 4 bits of actual. The extension bits are a literal 0 (unsigned
/// actual) or copies of the actual's sign bit, and `CaseEq` is a per-bit function, so
/// coerce-then-extend and extend-then-coerce are the same value. Measured demand
/// across the whole `cli` suite (logged at the coercion): 21 binds reach it, 5 of
/// them widening — small, but it is the same defect the prim cast had, one call away.
///
/// ⚠️ EVERY actual here carries an x or a z, because an actual that provably cannot
/// is not coerced at all and the row would be vacuous. Verified non-vacuous by
/// instrumenting the new branch: this design fires it SEVEN times (formal/actual
/// widths 16/4 in both signednesses, 32/4 in both, 32/8, 64/8, 8/4), including the
/// signed path where the fill bit is itself coerced. `fb1(s4[0])` is the control —
/// equal width, so it does not fire. Every cell measured three ways (a pre-36
/// binary, POST, live iverilog 13.0) and identical in all three.
#[test]
fn a_narrow_widening_bind_coerces_at_the_actuals_width() {
    let o = run(r#"module t;
  logic signed [3:0] s4;
  logic        [3:0] u4;
  logic signed [7:0] s8;
  function [31:0] fb16(input bit  [15:0] x); fb16 = x; endfunction
  function [31:0] fbs (input byte        x); fbs  = x; endfunction
  function [31:0] fi  (input int         x); fi   = x; endfunction
  function [63:0] fl  (input longint     x); fl   = x; endfunction
  function [31:0] fb1 (input bit         x); fb1  = x; endfunction
  initial begin
    s4 = 4'b1x01; u4 = 4'b1x01; s8 = 8'b1x01_z011;
    $display("%h %h", fb16(s4), fb16(u4));
    $display("%h %h", fi(s4),   fi(u4));
    $display("%h %h", fl(s8),   fbs(s4));
    $display("%h %h", fb1(s4[0]), fi(s8));
    #1 $finish;
  end
endmodule
"#);
    // iverilog 13.0. The x/z digits coerce to 0 (`4'b1x01` -> `4'h9`,
    // `8'b1x01_z011` -> `8'h93`) and the result then SIGN-extends for a signed
    // actual — a coerce-first that read its sign off the coerced `Concat` would
    // print `00000009` / `00000093` in the signed columns instead.
    assert_eq!(
        o,
        "0000fff9 00000009\nfffffff9 00000009\nffffffffffffff93 fffffff9\n00000001 ffffff93"
    );
}
