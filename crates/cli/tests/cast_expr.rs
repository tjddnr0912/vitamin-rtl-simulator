//! SV static cast `casting_type'(expr)` (IEEE 1800 §6.24) — end-to-end.
//!
//! Three families, ALL iverilog-13-pinned (the values below are the live
//! `iverilog -g2012` outputs):
//!  - SIZE cast `N'(e)`: width N, signedness INHERITED from the operand
//!    (sign-extend iff operand signed; result compares/prints signed iff operand
//!    signed). `8'(s8=-1)`=-1, `8'(8'hFF)`=255, `12'(s8)`=fff, `12'(u8)`=0ff.
//!  - TYPE cast `int'/byte'/…'(e)`: width/sign/state from the named type. 2-state
//!    targets (int/byte/shortint/longint/bit) coerce X/Z→0 PER BIT; 4-state
//!    (integer/logic/reg/time) preserve. Integer narrowing wraps two's-complement.
//!    real→integral ROUNDS HALF AWAY FROM ZERO (not $rtoi truncation).
//!  - SIGNING cast `signed'/unsigned'(e)`: PRESERVE width, flip sign, preserve X/Z.
//!
//! Lowered entirely to existing IR (IR-0; format_version unchanged). real→longint/
//! time decomposes the trunc-toward-zero integer into hi/lo 32-bit words in the
//! real domain (bit-exact vs iverilog). Class/typedef `name'(e)` down-casts stay
//! LOUD (no oracle; a `Derived'(base)` needs a runtime type check = `$cast`).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cast_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

fn line(src_body: &str, want: &str) {
    let src = format!("module t;\n  initial begin\n{src_body}\n  end\nendmodule\n");
    let (out, err, code) = run(&src);
    assert_eq!(code, Some(0), "expected exit 0.\nsrc:\n{src}\n{err}{out}");
    assert!(
        out.lines().any(|l| l == want),
        "expected a line `{want}`.\nsrc:\n{src}\ngot:\n{out}{err}"
    );
}

// Like `line`, but `decls` are emitted at MODULE scope (functions, localparams)
// before the `initial` block holding `src_body`.
fn line_mod(decls: &str, src_body: &str, want: &str) {
    let src = format!("module t;\n{decls}\n  initial begin\n{src_body}\n  end\nendmodule\n");
    let (out, err, code) = run(&src);
    assert_eq!(code, Some(0), "expected exit 0.\nsrc:\n{src}\n{err}{out}");
    assert!(
        out.lines().any(|l| l == want),
        "expected a line `{want}`.\nsrc:\n{src}\ngot:\n{out}{err}"
    );
}

fn loud(src_body: &str) {
    let src = format!("module t;\n  initial begin\n{src_body}\n  end\nendmodule\n");
    let (out, err, code) = run(&src);
    assert_ne!(
        code,
        Some(0),
        "must be loud, not silent.\nsrc:\n{src}\n{out}{err}"
    );
    assert!(
        format!("{err}{out}").contains("VITA-E"),
        "expected a loud E-diagnostic.\nsrc:\n{src}\n{out}{err}"
    );
}

#[test]
fn size_cast_inherits_operand_signedness() {
    // 8'(signed -1)=-1, 8'(unsigned 255)=255; widening sign-extends iff signed.
    line(
        "    byte s8 = -1; logic [7:0] u8 = 8'hFF;\n\
         \x20   $display(\"%0d %0d %h %h %0d\", 8'(s8), 8'(u8), 12'(s8), 12'(u8), (8'(s8) < 0));",
        "-1 255 fff 0ff 1",
    );
    // truncating a wider signed/unsigned operand keeps the inherited sign.
    line(
        "    int w = 16'shABCD;\n\
         \x20   $display(\"%h\", 8'(16'hABCD));",
        "cd",
    );
}

#[test]
fn type_cast_int_family_width_sign_and_wrap() {
    // int' zero-/sign-extends and stays 32-bit signed; byte'/shortint' wrap.
    line(
        "    $display(\"%0d %0d %0d %0d %0d\", int'(8'hFF), byte'(255), byte'(256), byte'(128), shortint'(70000));",
        "255 -1 0 -128 4464",
    );
    line("    $display(\"%0d\", shortint'(32768));", "-32768");
    line(
        "    $display(\"%0d %0d\", longint'(-1), integer'(8'hFF));",
        "-1 255",
    );
}

#[test]
fn type_cast_two_state_coerces_xz_four_state_preserves() {
    // int'(8'hAX) masks the X nibble to 0 (per-bit 2-state coercion).
    line(
        "    logic [7:0] xv = 8'hAX; $display(\"%h\", int'(xv));",
        "000000a0",
    );
    line(
        "    logic [7:0] xv = 8'hAX; $display(\"%h\", byte'(xv));",
        "a0",
    );
    line("    $display(\"%b %b\", bit'(1'bx), bit'(1'bz));", "0 0");
    // integer (4-state) keeps x in the LSB.
    line("    $display(\"%b\", logic'(1'bx));", "x");
    line("    $display(\"%b\", logic'(1'bz));", "z");
}

#[test]
fn one_bit_cast_takes_lsb() {
    line(
        "    $display(\"%b %b %b\", logic'(2'b10), bit'(2'b11), reg'(2'b11));",
        "0 1 1",
    );
}

#[test]
fn signing_cast_preserves_width_and_xz() {
    line(
        "    logic [3:0] f = 4'hF; $display(\"%0d %0d\", signed'(f), $bits(signed'(f)));",
        "-1 4",
    );
    line(
        "    byte s8 = -1; $display(\"%0d %0d\", unsigned'(s8), $bits(unsigned'(s8)));",
        "255 8",
    );
    line("    $display(\"%0d\", unsigned'(-1));", "4294967295");
    line(
        "    logic [7:0] xv = 8'hAX; $display(\"%h %h\", signed'(xv), unsigned'(xv));",
        "ax ax",
    );
}

#[test]
fn real_to_int_rounds_half_away_from_zero() {
    // NOT $rtoi truncation: 2.5→3, -2.5→-3, 0.5→1, -0.5→-1, 3.5→4, -3.5→-4.
    line(
        "    $display(\"%0d %0d %0d %0d %0d %0d\", int'(2.5), int'(-2.5), int'(0.5), int'(-0.5), int'(3.5), int'(-3.5));",
        "3 -3 1 -1 4 -4",
    );
    line(
        "    $display(\"%0d %0d %0d %0d\", int'(3.9), int'(3.4), int'(-3.4), int'(-3.6));",
        "4 3 -3 -4",
    );
}

#[test]
fn real_cast_of_integral_and_time() {
    line(
        "    $display(\"%0.1f %0.1f\", real'(3), real'(-5));",
        "3.0 -5.0",
    );
    line("    $display(\"%0d\", time'(-1));", "18446744073709551615");
}

#[test]
fn cast_binds_tighter_than_binary_ops() {
    // 8'(a) + b parses/evaluates as (8'(a)) + b, not 8'(a + b).
    line(
        "    logic [7:0] a = 8'hF0; int b = 1; $display(\"%0d\", 8'(a) + b);",
        "241",
    );
}

#[test]
fn class_or_typedef_name_cast_is_loud() {
    // No oracle for a class cast and typedef-name casts are out of v1 scope → loud.
    loud("    int x; x = my_unknown_t'(3);");
}

#[test]
fn real_to_longint_and_time_cast_round_half_away() {
    // real→longint/time decomposes the trunc-toward-zero integer into hi/lo 32-bit
    // words in the real domain ({hi,lo}, IR-0). Round HALF AWAY FROM ZERO, wide
    // magnitude preserved, negatives correct. All values are iverilog-13 outputs.
    line(
        "    real r;\n\
         \x20   r=3.7;  $display(\"%0d\", longint'(r));\n\
         \x20   r=-2.5; $display(\"%0d\", longint'(r));\n\
         \x20   r=1234567890123.0;  $display(\"%0d\", longint'(r));\n\
         \x20   r=-1234567890123.0; $display(\"%0d\", longint'(r));\n\
         \x20   r=-0.4; $display(\"%0d\", longint'(r));\n\
         \x20   r=4294967296.5; $display(\"%0d\", longint'(r));\n\
         \x20   r=100.0; $display(\"%0d\", time'(r));",
        "4",
    );
    // negative wide magnitude exact (the trunc-toward-zero fix, not floor).
    line(
        "    real r=-1234567890123.0; $display(\"%0d\", longint'(r));",
        "-1234567890123",
    );
    // |x| < 0.5 rounds to 0 even when negative (the off-by-one the floor path hit).
    line("    real r=-0.4; $display(\"%0d\", longint'(r));", "0");
    // wide value beyond 32-bit magnitude is preserved (32-bit $rtoi would lose it).
    line(
        "    real r=4294967296.5; $display(\"%0d\", longint'(r));",
        "4294967297",
    );
    // HUNT SW: an exactly-representable ODD integer in [2^52,2^53) (f64 ulp=1.0)
    // must NOT be perturbed by the round step — a `+0.5` pre-add would round it to
    // even (off by one). 2^53-1 = 9007199254740991 stays exact (both signs).
    line(
        "    real r=9007199254740991.0; $display(\"%0d\", longint'(r));",
        "9007199254740991",
    );
    line(
        "    real r=-9007199254740991.0; $display(\"%0d\", longint'(r));",
        "-9007199254740991",
    );
}

// ── post-implementation adversarial-hunt regressions ──

#[test]
fn widening_a_signed_operand_sign_extends() {
    // HUNT SW-1: a type/size cast that WIDENS a signed operand must SIGN-extend,
    // not zero-extend. Verified through %0d, < 0, +1, and a nested cast.
    line(
        "    byte sb = -1; logic signed [7:0] sx = -16;\n\
         \x20   $display(\"%0d %0d %0d %0d %b\", int'(sb), longint'(sb), int'(sx), int'(sx) + 1, (int'(sx) < 0));",
        "-1 -1 -16 -15 1",
    );
    line(
        "    logic signed [7:0] sx = -16; $display(\"%h\", int'(sx));",
        "fffffff0",
    );
}

#[test]
fn extend_preserves_z_not_just_x() {
    // HUNT SW-2: widening a 4-state operand must preserve Z (a bitwise `| 0` would
    // collapse z→x). Both the retained low bits and the sign-extension fill.
    line(
        "    logic [3:0] zv = 4'bz0z1; $display(\"%b\", 8'(zv));",
        "0000z0z1",
    );
    line(
        "    logic [3:0] zv = 4'bz0z1; $display(\"%b\", integer'(zv));",
        "0000000000000000000000000000z0z1",
    );
    line(
        "    logic signed [3:0] zs = 4'bz010; $display(\"%b\", 8'(signed'(zs)));",
        "zzzzz010",
    );
}

#[test]
fn cast_of_real_returning_function_call_rounds() {
    // HUNT SW-3: a DIRECT call to a real-returning user function must be detected
    // as a real operand → round-half-away (not a raw IEEE-754 bit reinterpret).
    line_mod(
        "  function automatic real getr(); getr = 2.5; endfunction\n\
         \x20 function automatic real bigr(); bigr = 3.9; endfunction",
        "    int a; byte b; int c;\n\
         \x20   a = int'(getr()); b = byte'(getr()); c = int'(bigr());\n\
         \x20   $display(\"%0d %0d %0d\", a, b, c);",
        "3 3 4",
    );
    // 2nd-round hunt: the real-call must be detected THROUGH unary +/- and real
    // arithmetic (not just a direct call), else the raw IEEE-754 bits leak.
    line_mod(
        "  function automatic real getr(); getr = 2.5; endfunction\n\
         \x20 function automatic int geti(); geti = 7; endfunction",
        "    int iv; iv = 2;\n\
         \x20   $display(\"%0d %0d %0d %0d\", int'(-getr()), int'(+getr()), int'(getr() + iv), int'(geti()));",
        "-3 3 5 7",
    );
}

#[test]
fn real_call_through_unary_to_longint_rounds() {
    // The real→longint cast must still treat a real call reached through a unary
    // `-` as REAL (not bit-reinterpret it) — and now produce the correct rounded
    // 64-bit value: longint'(-2.5) = -3 (round half away). iverilog parity.
    let src = "module t;\n  function automatic real getr(); getr = 2.5; endfunction\n\
               \x20 initial begin longint x; x = longint'(-getr()); $display(\"%0d\", x); end\nendmodule\n";
    let (out, err, code) = run(src);
    assert_eq!(code, Some(0), "expected exit 0.\n{err}{out}");
    assert!(out.lines().any(|l| l == "-3"), "expected -3.\n{out}{err}");
}

#[test]
fn parameter_width_size_cast() {
    // HUNT LG-1: a bare param/localparam width `W'(e)` is a legal size cast (was
    // loud-rejected as a name cast). Still loud for a true typedef/class name.
    line_mod(
        "  localparam W = 12;",
        "    logic [7:0] a = 8'hAB; $display(\"%h %h\", W'(a), (W+1)'(a));",
        "0ab 00ab",
    );
}
