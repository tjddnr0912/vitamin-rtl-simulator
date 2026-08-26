//! Round-35 R1/R4 — a 2-state cast named its operand once per DECLARED BIT.
//!
//! `int'(e)` is lowered by `coerce_two_state` into a `Concat` of one
//! `CaseEq(Select(e, i), 1'b1)` per target bit, and the engine walks that DAG as a
//! TREE. So an unguarded 2-state prim cast multiplied the operand's evaluation cost
//! by the cast's declared width, and nesting multiplied again. Counted by putting a
//! `$display` inside the operand — the numbers are exact, not approximate:
//!
//! | cast | operand evaluations, PRE | POST | iverilog 13 |
//! |---|---|---|---|
//! | none | 1 | 1 | 1 |
//! | `byte'` | 8 | 8 | 1 |
//! | `int'` | 32 | 32 | 1 |
//! | `longint'` | 64 | 64 | 1 |
//! | `int'(int'(x))` | **1024** | **32** | 1 |
//!
//! ⭐ The discriminator is 2-state-ness, not width: `integer'` and `int'` are both
//! 32-bit and signed, and differed by 27× in wall clock — `integer` is 4-state, so
//! no coercion is built for it at all.
//!
//! The fix is the guard the SIBLING coercion site already had. `inline_fn.rs`'s
//! formal binding calls the same `coerce_two_state` and gates it on
//! `expr_may_be_unknown`, with a comment recording the same measurement ("42.7x on a
//! `longint` one, 23x `.velab` growth, and nesting multiplies it"). `lower_prim_cast`
//! simply never got it. Build the per-bit coercion only where the operand can
//! actually carry an x or z.
//!
//! ⚠️ **This does NOT make the evaluation count correct.** A single `int'(f())` still
//! names `f()` 32 times against iverilog's 1, because a `Call` is conservatively "may
//! be unknown". What the guard removes is the MULTIPLICATION under nesting. That is a
//! move up the ladder on a count that was already wrong, never a regression — and the
//! residue is recorded in ROADMAP §2 rather than left implicit. Values are unaffected
//! either way: 64 cast cells over x/z-carrying operands print byte-identically PRE,
//! POST and under live iverilog 13.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_castfan_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr),
        out.status.code(),
    )
}

/// One `$display` per evaluation of the operand, so the ping count IS the fan-out.
fn ping_source(expr: &str) -> String {
    format!(
        "module tb;\n\
           function automatic int g(input int x); $display(\"ping\"); g = x + 1; endfunction\n\
           int r;\n\
           initial begin r = {expr}; $display(\"done %0d\", r); end\n\
         endmodule\n"
    )
}

fn pings(expr: &str) -> usize {
    let (out, code) = run(&ping_source(expr));
    assert_eq!(code, Some(0), "{out}");
    // The value is checked per-case rather than here: a 1-bit cast (`logic'`)
    // legitimately truncates `g(1) == 2` to 0, and a helper that demanded one answer
    // would have to exclude the very cell that proves 4-state casts do not fan out.
    assert!(
        out.contains("done "),
        "the run must reach the display:\n{out}"
    );
    out.lines().filter(|l| l.trim() == "ping").count()
}

/// The load-bearing assertion: NESTING no longer multiplies. Pinned as a VALUE, not
/// as "fewer than before" — a bound that cannot silently drift back.
#[test]
fn a_nested_two_state_cast_does_not_multiply_the_operand() {
    assert_eq!(pings("g(1)"), 1, "an uncast call is evaluated once");
    // A single cast still fans out to the declared width: a `Call` is conservatively
    // "may be unknown", so its coercion is still built. This is the residue, pinned
    // so that closing it is a deliberate change and not a surprise.
    assert_eq!(pings("int'(g(1))"), 32);
    // …but the OUTER cast of a nested pair sees a `Concat` of `CaseEq`, which is
    // known by construction, so it does not rebuild. 32, not 32×32.
    assert_eq!(pings("int'(int'(g(1)))"), 32);
    assert_eq!(pings("int'(int'(int'(g(1))))"), 32);
}

/// The other half of the discriminator: a 4-state cast of the same width and sign
/// builds no coercion at all, and never did. Present so the pair above cannot be
/// read as "casts are expensive" when the real rule is "2-state casts are".
#[test]
fn a_four_state_cast_of_the_same_width_never_fanned_out() {
    assert_eq!(pings("integer'(g(1))"), 1);
    assert_eq!(pings("logic'(g(1))"), 1);
    assert_eq!(pings("24'(g(1))"), 1, "a SIZE cast is not a 2-state cast");
    assert_eq!(pings("signed'(g(1))"), 1, "nor is a SIGNING cast");
}

/// ⚠️ The soundness half. The guard is only sound if `expr_may_be_unknown` never
/// answers "known" for something that can carry an x or z — otherwise the coercion is
/// skipped and the x leaks through the 2-state cast, which is a silent-wrong.
///
/// Every value here is pinned against LIVE iverilog 13.0, and every operand is
/// x/z-carrying in a different way: a whole 4-state net, its negation (which forces
/// the widening `Concat[Replicate(sign), e]` path that the new `Replicate` arm
/// governs), a part-select of one, and a literal with an x digit.
#[test]
fn an_unknown_operand_is_still_coerced_through_every_two_state_cast() {
    let src = "module tb;\n\
                 logic signed [7:0] w;\n\
                 logic [63:0] a, b, c, d, e, f;\n\
                 initial begin\n\
                   w = 8'b1x0z_1010;\n\
                   a = byte'(w); b = int'(w); c = longint'(w);\n\
                   d = int'(-w); e = int'(w[3:0]); f = int'(8'hxA);\n\
                   $display(\"A=%h B=%h C=%h D=%h E=%h F=%h\", a, b, c, d, e, f);\n\
                 end\n\
               endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    // ⚠️ MEASURED against live iverilog 13.0, not predicted — the first draft of this
    // test asserted a hand-derived line and every field of it was wrong. The x/z
    // digits of `w` coerce to 0 (`8'b1x0z_1010` → `8'h8a`) and the result then
    // SIGN-extends, which is why the high halves are `ff…` rather than zeros. Had the
    // guard wrongly skipped the coercion, these would contain `x` digits instead.
    assert!(
        out.contains(
            "A=ffffffffffffff8a B=ffffffffffffff8a C=ffffffffffffff8a \
             D=0000000000000000 E=000000000000000a F=000000000000000a"
        ),
        "an x/z operand must still be coerced:\n{out}"
    );
}

/// The widening path. `extend_to` builds `Concat[Replicate(sign bit), e]`, and
/// `expr_may_be_unknown` had no `Replicate` arm — it fell into the catch-all
/// `_ => true`, so before that arm was added EVERY widening 2-state cast rebuilt the
/// coercion the guard was meant to skip. Measured on the reporting shape, adding the
/// arm took one repro from 65.2 s to 0.12 s (542×) with byte-identical output.
///
/// ⚠️ MEASURED RESIDUE, pinned rather than glossed: a widening cast over a CALL still
/// fans out to the wider width (`longint'(int'(g(1)))` names `g` 64 times, not 32),
/// because the sign bit is selected from the operand and the operand is a `Call`,
/// which is conservatively unknown. The nesting collapse above is what shipped; this
/// is what did not.
#[test]
fn a_widening_cast_over_a_call_still_fans_out_to_the_wider_width() {
    assert_eq!(pings("longint'(g(1))"), 64);
    assert_eq!(pings("longint'(int'(g(1)))"), 64);
    let src = "module tb;\n\
                 byte b; logic signed [63:0] x, y;\n\
                 initial begin\n\
                   b = -8'sd3;\n\
                   x = longint'(b); y = longint'(-b);\n\
                   $display(\"X=%0d Y=%0d\", x, y);\n\
                 end\n\
               endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    // iverilog 13.0: sign-extension survives the skip.
    assert!(out.contains("X=-3 Y=3"), "{out}");
}
