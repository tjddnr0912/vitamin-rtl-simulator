//! `wprog` refused any expression whose self-width differed from its context, so a
//! narrow value meeting a wide one fell off the compiled backend entirely.
//!
//! ## What the census found
//!
//! An execution-weighted census of `wprog` declines (instrumentation measured and
//! reverted, not committed) on the three corpus designs that have NO frame bodies —
//! the designs a frame arena would do nothing for:
//!
//! ```text
//! darkriscv   325k declined requests    241266 (74%)  CMP: operand WIDTH 3 vs 32
//!                                        24162        CMP: operand WIDTH 4 vs 32
//!                                        15442        Signal  w=32 sw.w=1
//!                                         7924        LogNot  w=32 sw.w=1
//! serv, picorv32: the same two families are the top buckets.
//! ```
//!
//! ⭐ The work-list was much smaller than the request counts suggest: the compile cache
//! runs `compile` once per `(eid, w, signed)`, so 241k requests is 7 distinct expressions.
//!
//! ## What changed
//!
//! A node narrower than its context is now admitted, and HOW depends on the LRM's sizing
//! rule rather than on the width:
//!
//! * **self-determined** (a leaf, `Select`, `Concat`/`Replicate`, and every one-bit result
//!   — comparisons, `&&`/`||`, `!`, the reductions): compile at its OWN width and extend.
//!   Sign-extension is `WOp::Sext`, which calls `value::resize_word` — the same function
//!   `Value::resize`'s ≤64-bit arm uses — under exactly `resize_keep_sign`'s condition
//!   (`value.signed && ctx_signed`). Zero-extension emits no op at all: every stack value
//!   is already masked to its own width.
//! * **context-determined** (`~`, unary `-`, the bitwise binaries, `+`/`-`, the shifts,
//!   `?:`): computed at the CONTEXT width, so it simply proceeds.
//!
//! and a comparison's operands are sized to `max(self-width)` with their pair signedness
//! (§11.8.1), which is `eval_binary_ctx`'s comparison arm verbatim.
//!
//! ⚠️⚠️ **THE TRAP, and it bit me.** `sw.width < w` does NOT mean "compute at `sw.width`
//! and extend" — that is true only for a self-determined node. My first version applied it
//! to everything, and `logic [7:0] s = v[8:11] + 4'd1` with the select all-ones became
//! **0** (15 + 1 folded at four bits) instead of **16**. `ascending_part_select_in_arith`
//! caught it. That is why the classification is an `_`-free match: a new operator must not
//! inherit a sizing rule from a catch-all.
//!
//! Truncation (`sw.width > w`) still declines everywhere.
//!
//! ## What it bought
//!
//! `darkriscv -6.2%`, `serv -2.7%`, `picorv32 -1.8%`; `aes`/`keccak`/`keccak-arr` flat
//! because their cost is inside frame bodies, which this does not touch. Every pinned
//! corpus digest unchanged.
//!
//! The bulk of the verification is in `sim-engine`'s own differential battery
//! (`s2_wprog_matches_generic_eval_on_admitted_corpus_trees`), which grew from 7,960 to
//! 8,225 admitted trees and gained a widening sweep of 45,180 programs, every one
//! value-identical to the generic evaluator. What this file adds is the end-to-end half:
//! the shapes where the two halves of the rule give DIFFERENT answers, pinned against
//! iverilog.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run under every backend that ships and require identical output. The VM and the
/// interpreter do NOT use `wprog` at all, so they are a built-in oracle for this change:
/// a compiled program that disagreed with the generic evaluator shows up here even
/// without an external tool.
fn agrees_across_backends(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_wpw_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(d.join("t.sv"), src).expect("write design");
    let mut first: Option<String> = None;
    for be in ["native", "vm", "interp"] {
        let out = Command::new(env!("CARGO_BIN_EXE_vita"))
            .args(["t.sv", "--backend", be])
            .current_dir(&d)
            .output()
            .expect("run vita");
        let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
        s.push_str(&String::from_utf8_lossy(&out.stderr));
        match &first {
            None => first = Some(s),
            Some(f) => assert_eq!(f, &s, "backend {be} diverged"),
        }
    }
    let _ = std::fs::remove_dir_all(&d);
    first.unwrap()
}

// ── the trap: context-determined operators must NOT fold at their own width ──

/// ⭐ THE CELL THE FIRST VERSION FAILED. Each of these operators OVERFLOWS its operands'
/// width, so folding it narrow and then extending gives a different answer from computing
/// it at the assignment's width — which is what §11.6.1 requires.
///
/// All values pinned against iverilog 13 (verilator agrees):
/// `A` 4'hF + 4'd1 = 16 at eight bits, 0 at four.
/// `B` 4'h0 - 4'd1 = 255 at eight bits, 15 at four.
/// `C` 4'h9 << 2 = 36 at eight bits, 4 at four.
/// `D` ~4'h0 = 255 at eight bits, 15 at four.
/// `E` `-s` with `s = -4'sd1` next to an UNSIGNED `8'd0`: the `+` is unsigned, so `s`
///     ZERO-extends to 8'h0F and `-0x0F` is 0xF1 = 241 — not 1, and not 255. ⚠️ I guessed
///     1 here and the oracle said 241; every number in this file is measured, not derived.
/// `F` the ternary takes the context on BOTH branches.
#[test]
fn a_context_determined_operator_is_computed_at_the_context_width() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         logic [3:0] a, b; logic signed [3:0] s; logic c;\n  \
         logic [7:0] ra, rb, rc, rd, re, rf;\n  \
         initial begin\n    \
         a = 4'hF; b = 4'd1; s = -4'sd1; c = 1'b1;\n    \
         ra = a + b;\n    rb = 4'h0 - b;\n    rc = 4'h9 << 2;\n    \
         rd = ~4'h0;\n    re = -s + 8'd0;\n    rf = c ? (a + b) : 8'd7;\n    \
         $display(\"A=%0d B=%0d C=%0d D=%0d E=%0d F=%0d\", ra, rb, rc, rd, re, rf);\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert!(
        out.contains("A=16 B=255 C=36 D=255 E=241 F=16"),
        "each of these is a DIFFERENT number when folded at the operand width:\n{out}"
    );
}

/// The ascending-part-select spelling of the same trap, kept here because it is the exact
/// design that failed: `v[8:11]` on a `[0:15]` net is four all-one bits, and `+ 4'd1` is
/// 16 in the 8-bit lvalue.
#[test]
fn a_narrow_select_feeding_an_add_still_widens_before_adding() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  logic [0:15] v; logic [7:0] s;\n  \
         initial begin v = 16'h00FF; s = v[8:11] + 4'd1; \
         $display(\"S=%0d\", s); $finish; end\nendmodule\n",
    );
    assert!(out.contains("S=16"), "{out}");
}

// ── the other half: self-determined nodes ARE folded then extended ────────

/// A self-determined node keeps its own width and is converted on the way out. These are
/// the shapes that would be WRONG under the context-determined rule: a comparison is one
/// bit however wide its context, a concat is the sum of its parts, a reduction is one bit.
#[test]
fn a_self_determined_node_keeps_its_own_width() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         logic [3:0] a, b; logic [31:0] p, q, r, u;\n  \
         initial begin\n    a = 4'hF; b = 4'h3;\n    \
         p = (a > b);\n    q = {a, b};\n    r = &a;\n    u = (a && b);\n    \
         $display(\"P=%0d Q=%0h R=%0d U=%0d\", p, q, r, u);\n    $finish;\n  end\nendmodule\n",
    );
    assert!(out.contains("P=1 Q=f3 R=1 U=1"), "{out}");
}

/// ⭐ SIGN EXTENSION is the one conversion this change emits, and it fires only when the
/// value AND its context are both signed — `resize_keep_sign`'s rule, not a new one.
///
/// `S` a signed −1 assigned to a signed 32-bit lvalue sign-extends to −1.
/// `U` ⚠️ the same value assigned to an UNSIGNED 32-bit lvalue is **4294967295**, not 15:
///     the assignment's context signedness is the RIGHT-hand side's, not the lvalue's, so
///     it still sign-extends. I guessed 15; all three tools say 4294967295.
/// `V` an unsigned 4'hF assigned to a signed lvalue is 15 — zero-extended, because the
///     value is unsigned. That is the pair `U`/`V` this cell exists for: the fill follows
///     the VALUE, and only then is the result reinterpreted.
#[test]
fn sign_extension_needs_the_value_and_the_context_to_agree() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         logic signed [3:0] s; logic [3:0] u;\n  \
         logic signed [31:0] rs, rv; logic [31:0] ru;\n  \
         initial begin\n    s = -4'sd1; u = 4'hF;\n    \
         rs = s; ru = s; rv = u;\n    \
         $display(\"S=%0d U=%0d V=%0d\", rs, ru, rv);\n    $finish;\n  end\nendmodule\n",
    );
    assert!(out.contains("S=-1 U=4294967295 V=15"), "{out}");
}

/// ⚠️ Sign-extending an X or Z sign bit fills the new bits with X/Z — `resize_word`'s rule,
/// and the one a second spelling would most likely get wrong by filling with 0 or 1.
#[test]
fn an_unknown_sign_bit_fills_the_extension_with_unknown() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         logic signed [3:0] s; logic signed [11:0] r;\n  \
         initial begin s = 4'sbx011; r = s; $display(\"R=%b\", r); $finish; end\nendmodule\n",
    );
    assert!(out.contains("R=xxxxxxxxx011"), "{out}");
}

// ── comparisons: unequal width and mixed signedness ───────────────────────

/// The census's top bucket: operands of unequal width. §11.8.1 sizes both to
/// `max(self-width)` with their PAIR signedness, so a signed operand meeting an unsigned
/// one is compared UNSIGNED — `A` and `B` are the pair that separates the two rules.
///
/// `A` −1 as a 4-bit signed vs an unsigned 8-bit 1: the pair is unsigned, so the −1
///     zero-extends to 15 and 15 > 1.
/// `B` the same against a SIGNED 8-bit 1: the pair is signed, −1 sign-extends, −1 < 1.
/// `C`/`D` a 3-bit value against a 32-bit one, the shape that is 74% of darkriscv's
///     declined requests.
#[test]
fn comparison_operands_size_to_the_wider_with_pair_signedness() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  \
         logic signed [3:0] s; logic [7:0] u; logic signed [7:0] su;\n  \
         logic [2:0] n; logic [31:0] w;\n  \
         initial begin\n    s = -4'sd1; u = 8'd1; su = 8'sd1; n = 3'd5; w = 32'd5;\n    \
         $display(\"A=%0d B=%0d C=%0d D=%0d\", (s > u), (s > su), (n == w), (n < w));\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert!(out.contains("A=1 B=0 C=1 D=0"), "{out}");
}

// ── the pre-existing gap this slice's fuzz found, pinned as it stands ─────

/// ⚠️ NOT FIXED, and NOT this slice's: pinned so a later change to the comparison's
/// operand sizing cannot move it silently in either direction.
///
/// §11.8.1 makes a comparison unsigned when EITHER operand is, and that unsignedness
/// propagates DOWN into both operands — which demotes a `>>>` inside one of them to a
/// logical shift. vita does not propagate it:
///
/// ```text
///   reg signed [7:0] b = 8'shB3;   (b >>> 4) > 8'd100
///     vita 1     iverilog 0     verilator 0        (exit 0, no diagnostic)
/// ```
///
/// ⭐ It ISOLATES: the same expression against a SIGNED `8'sd100` is 0 in all three, and
/// `$signed(b >>> 4)` is −5 in all three. So it is neither the shift nor the sign of `b` —
/// it is the direction the comparison propagates signedness.
///
/// ⚠️ All three vita backends agree, so it lives in the shared sizing path and has
/// nothing to do with `wprog`. Recorded as ROADMAP §2 row 🆕 A. It was found by this
/// slice's differential lens, which hit the family five times in 1,130 fuzzed designs.
#[test]
fn a_comparison_does_not_yet_push_its_unsignedness_into_its_operands() {
    let out = agrees_across_backends(
        "`timescale 1ns/1ns\nmodule t;\n  reg signed [7:0] b;\n  \
         initial begin b = 8'shB3;\n    \
         $display(\"A=%0d B=%0d C=%0d\", \
         (b >>> 4) > 8'd100, (b >>> 4) > 8'sd100, $signed(b >>> 4));\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert!(
        out.contains("A=1 B=0 C=-5"),
        "A is vita's WRONG answer (both oracles say 0) and is pinned as such; \
         B and C already match both oracles:\n{out}"
    );
}
