//! A cast is a SELF-DETERMINED boundary (IEEE 1800-2017 §11.8.1): `8'(a*a)`
//! multiplies two 8-bit operands no matter how wide the expression around it is.
//!
//! `lower_size_cast` used to return its operand node RAW whenever the cast width
//! already equalled the operand's width — and a bare `Binary`/`Unary`/`Ternary`
//! node is CONTEXT-determined to the engine, so an enclosing width propagated
//! straight through the cast and re-ran the whole operation wider.
//! `logic [15:0] r; r = 8'(a*a)` with `a = 8'hff` printed `fe01` where iverilog
//! prints `0001`. A 4,032-cell sweep (binary / unary / ternary operators x
//! operand pairs x cast widths {2,4,8,16} x 8 destinations) measured **634 wrong
//! cells, every one of them with the destination wider than the cast** — silent,
//! at exit 0.
//!
//! The fix makes the tail uniform: the cast always ends in `$signed`/`$unsigned`,
//! which is this repo's seal primitive (the index seal in `packed.rs` uses it for
//! the same reason) because the engine evaluates a `$signed`/`$unsigned` operand
//! SELF-determined and only then resizes to the context. The other two arms were
//! already sealed — `extend_to` builds a `Concat`, `select_low` a `Select`, both
//! self-determined — so the uniform tail is a no-op on them.
//!
//! Second defect, same two lines: the signedness a cast INHERITS from its operand
//! came from `expr_self_signed`, elaborate's own hand-written mirror. It reads
//! UNSIGNED where the canonical rule says signed at every leaf whose sign lives
//! in a sidecar rather than a net flag — a user function's declared return, a
//! class field, an element-typed array reduction — and at the whole int-returning
//! system-function family it does not list (`$clog2`, `$countones`, `$random`,
//! `$fopen`, `$test$plusargs`, `$value$plusargs`, `$cast`, the file-read family,
//! `$dist_*`, `.len()`/`.compare()`/the string→int methods, `$itor`/`$bitstoreal`
//! and the real-math ids). It reads SIGNED where the rule says unsigned at exactly
//! one id, `$stime`. It now comes from the CANONICAL `sim_ir::selfwidth` rule, the
//! same answer the engine's width table will give.
//!
//! Third, found while measuring the second: `ast_ctx_signed` called an UNPACKED
//! ARRAY ELEMENT read unsigned because the AST spells it `BitSelect`, exactly like
//! a packed bit-select. An element carries its own declared sign.
//!
//! ⚠️ Three things the sign fix must NOT do, all measured on earlier cuts and all
//! guarded now: seal on a width `ir_bits_of` could not answer or answered from a
//! class-field HANDLE net; adopt a canonical "signed" for an operand
//! `extend_to`'s sign fill would then evaluate a second time; and answer "cannot
//! resolve" for an array element, which drops the §4.5.212 context width.
//!
//! ORACLE: iverilog 13.0, with verilator 5.050 as the second oracle where it
//! applies. Every expected value below was run through them; the two rows where
//! the oracles disagree say so and name the clause that decides.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_args(src: &str, args: &[&str]) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_scseal_{}_{n}", std::process::id()));
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
    let r = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::remove_dir_all(&d);
    r
}

fn run(src: &str) -> String {
    run_args(src, &[])
}

// ── the headline ────────────────────────────────────────────────────────────

/// `a*a` = 65025 needs 16 bits; the cast says 8, so the product is 8-bit and the
/// answer is `01`. The destination being 16 bits wide must not reopen the cast.
#[test]
fn a_narrowing_cast_is_not_reopened_by_a_wider_destination() {
    let o = run(r#"module t;
  logic [7:0] a = 8'hFF;
  logic [15:0] r16; logic [7:0] r8; logic [3:0] r4;
  initial begin
    r8  = 8'(a*a);  $display("%h", r8);
    r16 = 8'(a*a);  $display("%h", r16);
    r16 = 8'(a+a);  $display("%h", r16);
    r4  = 4'(a+1);  $display("%h", r4);
    r16 = 4'(a+1);  $display("%h", r16);
    #1 $finish;
  end
endmodule
"#);
    // iverilog: 01 / 0001 / 00fe / 0 / 0000.  PRE: 01 / fe01 / 01fe / 0 / 0010
    // — the destination-width == cast-width rows were already right, which is
    // exactly why nothing in the suite noticed the other three.
    assert_eq!(o, "01\n0001\n00fe\n0\n0000");
}

/// Every position that pushes a context width into its operand leaked; the sweep
/// found no other kind. The four self-determined positions in the next test were
/// right before this slice and are here so an over-eager seal cannot pass either.
#[test]
fn the_seal_holds_in_every_context_determined_consumer() {
    let o = run(
        r#"module sub(input [15:0] p); initial #2 $display("PORT %h", p); endmodule
module t;
  logic [7:0] a = 8'hFF;
  logic [15:0] u16;
  wire  [15:0] cw; assign cw = 8'(a*a);
  sub up(.p(8'(a*a)));
  function automatic [15:0] fid(input [15:0] x); fid = x; endfunction
  initial begin
    u16 <= 8'(a*a); #1;      $display("NBA  %h", u16);
    $display("CONT %h", cw);
    u16 = 8'(a*a) + 16'd0;   $display("ADD  %h", u16);
    u16 = 8'(a*a) | 16'h0;   $display("OR   %h", u16);
    u16 = fid(8'(a*a));      $display("ARG  %h", u16);
    u16 = a[0] ? 8'(a*a) : 16'd7; $display("TERN %h", u16);
    case (4'(a+1))
      16'd16:  $display("CASE %h", 16'hdead);
      16'd0:   $display("CASE %h", 16'h0000);
      default: $display("CASE %h", 16'hbeef);
    endcase
    $display("CMP  %h", {15'b0, (4'(a+1) == 16'd16)});
    #3 $finish;
  end
endmodule
"#,
    );
    // iverilog, all eight.  PRE: fe01 / fe01 / fe01 / fe01 / fe01 / fe01 /
    // dead / 0001 — i.e. every one of them wrong.
    assert_eq!(
        o,
        "NBA  0001\nCONT 0001\nADD  0001\nOR   0001\nARG  0001\n\
         TERN 0001\nCASE 0000\nCMP  0000\nPORT 0001"
    );
}

/// ANTI-VACUITY. A concat part, a replication value, a `$display` argument and an
/// array index are self-determined positions — the cast was already correct there
/// before this slice. They stay pinned so a seal that changed the VALUE (rather
/// than only the boundary) cannot pass.
#[test]
fn self_determined_consumers_are_byte_identical() {
    let o = run(r#"module t;
  logic [7:0] a = 8'hFF;
  logic [15:0] u16; logic [15:0] mem [0:255];
  initial begin
    mem[1] = 16'hcafe;
    u16 = {8'b0, 8'(a*a)};  $display("%h", u16);
    u16 = {2{8'(a*a)}};     $display("%h", u16);
    $display("%h", 8'(a*a));
    u16 = mem[8'(a*a)];     $display("%h", u16);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "0001\n0101\n01\ncafe"); // PRE identical (iverilog too)
}

// ── the inherited sign ──────────────────────────────────────────────────────

/// An UNSIGNED cast result whose top bit is set must ZERO-extend. `a + 10` =
/// 265, low four bits `1001`; the cast is unsigned, so the 16-bit destination
/// sees `0009` and not `fff9`. Kills a seal hard-wired to `$signed`.
#[test]
fn an_unsigned_cast_result_zero_extends() {
    let o = run(r#"module t;
  logic [7:0] a = 8'hFF;
  logic [15:0] u16; logic signed [15:0] s16;
  initial begin
    u16 = 4'(a + 8'd10); $display("%h", u16);
    s16 = 4'(a + 8'd10); $display("%h", s16);
    #1 $finish;
  end
endmodule
"#);
    // iverilog: 0009 both — the DESTINATION's signedness does not enter the rhs.
    assert_eq!(o, "0009\n0009"); // PRE: 0019 0019
}

/// A SIGNED cast result sign-extends into either destination. Kills a seal
/// hard-wired to `$unsigned`.
#[test]
fn a_signed_cast_result_sign_extends_into_both_destinations() {
    let o = run(r#"module t;
  logic signed [3:0] sb = -4'sd5;
  logic signed [7:0] sa = -8'sd3;
  logic [15:0] u16; logic signed [15:0] s16;
  initial begin
    s16 = 4'(sb + 4'sd0); $display("%h", s16);
    u16 = 4'(sb + 4'sd0); $display("%h", u16);
    s16 = 3'(sa >>> 1);   $display("%h", s16);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "fffb\nfffb\nfffe"); // PRE identical (iverilog too)
}

/// The mirror's blind spot: a user function's DECLARED return sign, on the two
/// arms that do NOT build a sign fill. (The widening arm is the next test — it
/// cannot take this fix, and the reason is a hazard, not an oversight.)
#[test]
fn a_signed_function_return_is_the_operand_sign_where_it_is_not_repeated() {
    let o = run(r#"module t;
  function automatic signed [3:0] fs(input d); fs = -4'sd3; endfunction
  function automatic signed [3:0] fh(input d); fh = -4'sd2; endfunction
  logic signed [15:0] s16;
  initial begin
    s16 = 4'(fs(0) + 4'sd0); $display("%h", s16);
    s16 = 2'(fh(0) + 4'sd0); $display("%h", s16);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "fffd\nfffe"); // iverilog; PRE: fffd / 0002
}

/// The other blind-spot family: the int-returning system functions the mirror
/// simply does not list. Both oracles agree. `$clog2(300) = 9 = 1001` and
/// `$countones(8'hff) = 8 = 1000` — four-bit values whose top bit is set, and
/// the cast inherits `integer`'s signedness, so a wider destination sign-extends.
#[test]
fn an_int_returning_system_function_is_signed_under_a_cast() {
    let o = run(r#"module t;
  logic signed [15:0] s16;
  initial begin
    s16 = 4'($clog2(300));       $display("%h", s16);
    s16 = 4'($countones(8'hFF)); $display("%h", s16);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "fff9\nfff8"); // iverilog AND verilator; PRE: 0009 / 0008
}

/// The WIDENING arm still takes the canonical sign when the operand IS
/// repeatable — the impure-operand guard must not swallow the pure half.
/// `$clog2(300) = 9`, `-9 = …10111` at 32 bits, low 40 sign-extended.
#[test]
fn a_widening_cast_of_a_repeatable_operand_still_takes_the_canonical_sign() {
    let o = run(r#"module t;
  logic [63:0] r;
  initial begin
    r = 40'(-$clog2(300)); $display("%h", r);
    #1 $finish;
  end
endmodule
"#);
    // iverilog AND verilator. PRE: 00000000fffffff7.
    assert_eq!(o, "fffffffffffffff7");
}

/// When the guard declines, it must hand back THE MIRROR's answer, not `false`.
/// `$signed(f())` is signed to both, so the row is PRE == POST == oracle — its
/// job is to fail if the fallback is ever hard-wired to unsigned.
#[test]
fn the_impure_guard_falls_back_to_the_mirror_not_to_unsigned() {
    let o = run(r#"module t;
  function automatic signed [3:0] fp(input d); fp = -4'sd3; endfunction
  logic [63:0] r;
  initial begin
    r = 16'($signed(fp(0))); $display("%h", r);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "fffffffffffffffd"); // iverilog; PRE identical
}

/// A class field lowers to `Signal{net: 32-bit HANDLE net}` with its real width
/// in a sidecar, so `ir_bits_of` answers a FABRICATED `Some(32)` — a `Some`, which
/// is why "decline when `ir_bits_of` is `None`" was not enough. At cast width
/// exactly 32 that put the operand on the same-width arm and the seal then
/// evaluated it at the FIELD's width, truncating the context-widened operation:
/// `0000` for `0100`, and the signed row even flipped sign to `ffc8`.
/// (No iverilog oracle for classes. Hand-IEEE: `c.u8 + ua` is unsigned, its
/// context is `max(8, 32) = 32`, so 253 + 3 = 256; the signed row is 100 + 100.)
#[test]
fn a_class_field_operand_never_seals_on_the_handle_width() {
    let o = run(r#"module t;
  class C; logic [7:0] u8; logic signed [7:0] s8; endclass
  C c; logic [3:0] ua = 4'd3; logic signed [7:0] sp = 8'sd100;
  logic [63:0] r;
  initial begin
    c = new(); c.u8 = 8'hFD; c.s8 = 8'sd100;
    #1;
    r = 32'(c.u8 + ua); $display("%h", r);
    r = 32'(c.s8 + sp); $display("%h", r);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "0000000000000100\n00000000000000c8"); // PRE identical
}

/// An UNPACKED ARRAY ELEMENT read is spelled `BitSelect` in the AST, exactly like
/// a packed bit-select — but §5.4.1's "a select is unsigned" is about the select.
/// An element carries its own declared sign. Row 2 is the fix (`27` → `fff7`);
/// row 1 is the guard against over-correcting: answering "I cannot resolve this
/// leaf" instead of the element's sign drops §4.5.212's context width and cost
/// **170 cells** of a 1,560-cell element matrix, this row among them.
#[test]
fn an_unpacked_array_element_carries_its_own_sign() {
    let o = run(r#"module t;
  logic signed [3:0] e4 [0:3];
  logic [63:0] r;
  initial begin
    e4[0] = -4'sd3;
    #1;
    r = 7'(e4[0] + 4'h3);   $display("%h", r);
    r = 8'(e4[0] * 4'sh3);  $display("%h", r);
    #1 $finish;
  end
endmodule
"#);
    // iverilog. Row 1 PRE-identical (an unsigned sibling makes the whole
    // expression unsigned, §11.8.1, so the old blanket answer was right there);
    // row 2 PRE: 0000000000000027.
    assert_eq!(o, "0000000000000010\nfffffffffffffff7");
}

/// ANTI-VACUITY for the element rule's DECLINING half — the previous test only
/// pinned rows where the select IS an element, so nothing stopped the helper
/// from claiming every `BitSelect`. §5.4.1 keeps all three of these UNSIGNED: a
/// part-select and a bit-select of a packed SIGNED vector, and a bit-select taken
/// OF an element (whose base is the element read, not the array ident).
/// `8'shF9` is `1111_1001`, so a signed reading of any of them would show up as
/// `ffff…` instead of the small positive value.
#[test]
fn a_packed_select_stays_unsigned_even_on_a_signed_base() {
    let o = run(r#"module t;
  logic signed [7:0] sv;
  logic signed [7:0] am [0:3];
  logic [63:0] r;
  initial begin
    sv = 8'shF9; am[0] = 8'shF9;
    #1;
    r = 16'(sv[3:0]    + 4'sh0); $display("%h", r);
    r = 16'(sv[3]      + 4'sh0); $display("%h", r);
    r = 16'(am[0][3]   + 4'sh0); $display("%h", r);
    r = 16'(am[0][3:1] + 4'sh0); $display("%h", r);
    #1 $finish;
  end
endmodule
"#);
    // iverilog; PRE identical on all four (this is the half the rule must NOT move).
    assert_eq!(
        o,
        "0000000000000009\n0000000000000001\n\
         0000000000000001\n0000000000000004"
    );
}

/// The declined branch still stamps `$signed` when the operand is signed — the
/// tail SHAPE is the pre-slice one, the sign INPUT is the canonical rule, and for
/// an `ir_bits_of`-declining operand that is a fix in its own right.
/// (`ir_bits_of` returns `None` for the array reductions as well as for strings
/// and placeholders. iverilog parses the queue but rejects the method — "Method
/// sum is not a queue method" — so this row is hand-IEEE: `-2 + -1 = -3`, a
/// signed `byte`, sign-extended.)
#[test]
fn a_declined_cast_still_inherits_a_signed_operand() {
    let o = run(r#"module t;
  byte signed q[$];
  logic [63:0] r;
  initial begin
    q.push_back(-2); q.push_back(-1);
    #1;
    r = 8'(q.sum()); $display("%h", r);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "fffffffffffffffd"); // PRE: 00000000000000fd
}

/// ⚠️ THE WIDENING ARM CANNOT TAKE THE SIGN FIX FOR AN IMPURE OPERAND, and this
/// test exists to keep it that way. `extend_to`'s sign fill is
/// `Select{Bit, base: e}` — it names the operand a SECOND time; the zero fill is
/// a constant and does not. So adopting a canonical "signed" the mirror called
/// unsigned makes an impure operand run twice. Measured on the first cut of this
/// slice: `fp` was called TWICE, and `16'(fur(0))` (a `$urandom` wrapper) built
/// its value out of the sign bit of one draw and the low bits of the NEXT,
/// shifting every later draw. Both are ladder violations, so the widening arm
/// keeps the mirror's answer whenever the operand is not repeatable.
///
/// The cost is stated, not hidden: these three rows are still the PRE values and
/// iverilog says `fffd` / `fffd` / `fffffffd`. Closing them needs an extension
/// that names its operand once (or a callee-purity predicate) — ROADMAP §2.
#[test]
fn a_widening_cast_never_evaluates_an_impure_operand_twice() {
    let o = run(r#"module t;
  function automatic signed [3:0] fp(input d); $display("CALL"); fp = -4'sd3; endfunction
  function automatic signed [3:0] fs(input d); fs = -4'sd3; endfunction
  logic signed [15:0] s16; logic [31:0] u32;
  initial begin
    s16 = 16'(fp(0));         $display("%h", s16);
    s16 = 16'(fs(0) + 4'sd0); $display("%h", s16);
    u32 = int'(fs(0));        $display("%h", u32);
    #1 $finish;
  end
endmodule
"#);
    // ONE "CALL" line is the whole point of the test.
    assert_eq!(o, "CALL\n000d\n000d\n0000000d");
}

/// `$stime` is the ONE id where the mirror said signed and the canonical rule
/// says unsigned, and the two oracles SPLIT on it — so the standard decides.
/// IEEE 1364-2005 §17.7.2: "$stime … returns an **unsigned** integer that is a
/// 32-bit time". verilator 5.050 agrees; iverilog 13.0 returns a signed
/// `integer` and prints `ffff8000`. vita already read `$stime` as unsigned
/// EVERYWHERE ELSE (`q = $stime` at t=2^31 gives `0000000080000000` in PRE too),
/// so PRE was self-inconsistent and only the cast site matched iverilog.
#[test]
fn stime_is_unsigned_under_a_cast_like_everywhere_else() {
    let o = run(r#"`timescale 1ns/1ns
module t;
  logic signed [31:0] s32;
  initial begin
    #32768;
    s32 = 16'($stime); $display("%h", s32);
    $finish;
  end
endmodule
"#);
    assert_eq!(o, "00008000"); // verilator + §17.7.2; iverilog: ffff8000 (PRE matched it)
}

/// A class field's sign is a vita SIDECAR, not its handle net's flag — the same
/// blind spot, on the axis iverilog cannot judge. Hand-IEEE: `4'(c.sb + 0)` is a
/// 4-bit SIGNED `1101`, so a 16-bit destination reads `fffd`. (PRE: `000d`.)
#[test]
fn a_class_field_sign_reaches_the_cast() {
    let o = run(r#"module t;
  class C; logic signed [3:0] sb; endclass
  C c; logic signed [15:0] s16;
  initial begin
    c = new; c.sb = -4'sd3;
    s16 = 4'(c.sb + 4'sd0); $display("%h", s16);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "fffd");
}

/// NOT-YET-KNOWABLE is not UNSIGNED **and it is not SIGNED either** — a deferred
/// hierarchical reference is still a placeholder while the cast is lowered, so
/// the canonical rule declines and the old mirror answers. Both polarities are
/// pinned because only pinning one lets the opposite default live: `**` and the
/// shift family take their sign from the BASE alone (Table 11-21), so a SIGNED
/// base with a placeholder exponent needs `signed` and an UNSIGNED base needs
/// `unsigned`. `(-2)**3 = -8 = 1000` sign-extends to `f8`; `9**3 = 729` truncates
/// to `d9` and must NOT sign-extend.
#[test]
fn a_placeholder_operand_keeps_the_pre_canonical_answer() {
    let o = run(r#"module sub; logic [3:0] k; initial k = 4'd3; endmodule
module t;
  sub u1();
  logic signed [3:0] sa = -4'sd2;
  logic [3:0] ua = 4'd9;
  logic signed [15:0] s16;
  initial begin
    #1;
    s16 = 8'(sa ** u1.k);  $display("%h", s16);
    s16 = 8'(ua ** u1.k);  $display("%h", s16);
    s16 = 8'(sa >>> u1.k); $display("%h", s16);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "fff8\n00d9\nffff"); // iverilog; PRE: fff8 / 02d9 / ffff
}

/// The two shapes where `ir_bits_of` cannot answer and the caller FABRICATES 32.
/// Sealing on a fabricated width is a rung down in both directions, so the seal
/// is declined there — these rows are the guard.
///   - hierarchical: `w` defaults to 32, so only `N == 32` reaches the same-width
///     arm, and there the pre-seal `return e` was LOAD-BEARING (the engine's
///     post-resolve width table sign-extends). A seal stamped `$unsigned`:
///     `0000fffd` for the oracles' `fffffffd`.
///   - `string`: a string net's table width is 0, so the `Concat` the widening
///     arm builds has a CANONICAL self-width of `N−31`, and `$unsigned` evaluates
///     its operand at exactly that width — `61626364` became `164` over the whole
///     band `33 ≤ N ≤ 63`. (iverilog rejects `N'(string)`; hand-IEEE and PRE both
///     say the bytes survive.)
#[test]
fn a_fabricated_operand_width_declines_the_seal() {
    let o = run(r#"module sub; logic signed [15:0] s = -16'sd3; endmodule
module t;
  sub u1();
  string st = "abcd";
  logic [31:0] r; logic [63:0] w;
  initial begin
    #1;
    r = 32'(u1.s); $display("%h", r);
    w = 40'(st);   $display("%h", w);
    w = 48'(st);   $display("%h", w);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(
        o,
        "fffffffd\n0000000061626364\n0000000061626364" // = PRE; iverilog on row 1
    );
}

// ── width axes ──────────────────────────────────────────────────────────────

/// Above 64 bits and across the word boundary — the seal must not depend on the
/// value fitting one lane.
#[test]
fn the_seal_holds_above_the_word_boundary() {
    let o = run(r#"module t;
  logic [99:0] w100;
  logic [99:0] wa = 100'h3_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF;
  initial begin
    w100 = 33'(wa + 100'd1); $display("%h", w100);
    w100 = 64'(wa * 100'd3); $display("%h", w100);
    w100 = 65'(wa + 100'd1); $display("%h", w100);
    #1 $finish;
  end
endmodule
"#);
    // iverilog. PRE leaked the carry out of every one of the three.
    assert_eq!(
        o,
        "0000000000000000000000000\n\
         000000000fffffffffffffffd\n\
         0000000000000000000000000"
    );
}

/// A nested cast: the inner boundary must survive the outer one's context.
#[test]
fn a_nested_cast_seals_at_both_levels() {
    let o = run(r#"module t;
  logic [7:0] a = 8'hFF;
  logic [15:0] u16;
  initial begin
    u16 = 8'(4'(a+1) + 8'd1);  $display("%h", u16);
    u16 = 16'(8'(a*a) + 16'd1); $display("%h", u16);
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(o, "0001\n0002"); // iverilog; PRE: 0011 / fe02
}

/// The seal is a new `SysFunc` node in the arena, so the three execution backends
/// must still agree on it (the tier-3 W-program declines a `SysFunc` and falls
/// back to the generic evaluator — that is a coverage question, not a value one).
#[test]
fn the_three_backends_agree_on_a_sealed_cast() {
    let src = r#"module t;
  logic [7:0] a; logic [15:0] u16; logic signed [15:0] s16;
  logic signed [3:0] sb;
  initial begin
    a = 8'hFF; sb = -4'sd5;
    u16 = 8'(a*a);        $display("%h", u16);
    u16 = 4'(a + 8'd10);  $display("%h", u16);
    s16 = 4'(sb + 4'sd0); $display("%h", s16);
    #1 $finish;
  end
endmodule
"#;
    let want = "0001\n0009\nfffb";
    assert_eq!(run_args(src, &["--backend", "interp"]), want);
    assert_eq!(run_args(src, &["--backend", "bytecode"]), want);
    assert_eq!(run_args(src, &["--backend", "native"]), want);
}
