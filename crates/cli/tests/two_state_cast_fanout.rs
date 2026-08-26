//! Round-35/36 — a 2-state cast named its operand once per bit of the WRONG WIDTH.
//!
//! `int'(e)` is lowered by `coerce_two_state` into a `Concat` of one
//! `CaseEq(Select(e, i), 1'b1)` per bit it covers, and the engine walks that DAG as a
//! TREE. So a 2-state prim cast multiplies the operand's evaluation cost by the width
//! the coercion is applied at. Two separate defects lived on that sentence:
//!
//! * **Round 35 — it was applied at all.** No guard, so an operand that provably
//!   cannot carry an x or z was coerced anyway, and NESTING multiplied
//!   (`int'(int'(x))` = 1024). Fixed by `expr_may_be_unknown`.
//! * **Round 36 — it was applied at the TARGET's width.** A WIDENING cast resized
//!   first and coerced the resized value, so `int'(nb)` over a 4-bit `nb` paid 32
//!   terms for 4 bits of operand. The extension bits are provably no-ops (see
//!   `expr_cast::lower_prim_cast`), so the coercion now runs at the OPERAND's width
//!   and the extension is applied to the coerced value.
//!
//! Counted by putting a `$display` inside the operand — the numbers are exact, not
//! approximate:
//!
//! | cast | operand evals, pre-35 | pre-36 | POST | iverilog 13 |
//! |---|---|---|---|---|
//! | none | 1 | 1 | 1 | 1 |
//! | `byte'` (32-bit operand, narrowing) | 8 | 8 | 8 | 1 |
//! | `int'` (32-bit operand, same width) | 32 | 32 | 32 | 1 |
//! | `longint'` (32-bit operand, widening) | 64 | 64 | **32** | 1 |
//! | `int'(int'(x))` | **1024** | 32 | 32 | 1 |
//! | `int'` of a 4-bit operand | 32 | 32 | **4** | 1 |
//! | `longint'` of a 4-bit operand | 64 | 64 | **4** | 1 |
//!
//! ⭐ The discriminator for building a coercion at all is 2-state-ness, not width:
//! `integer'` and `int'` are both 32-bit and signed, and differed by 27× in wall
//! clock — `integer` is 4-state, so no coercion is built for it.
//!
//! Round 36's motivating measurement, on the reporter's own repro (a
//! `function automatic` called from a continuous `assign`, whose body says
//! `if (k <= int'(nb))` with `nb` 4 bits wide), release, 5,000 clocked iterations,
//! foreground: **6.95 s → 0.685 s = 10.1×**, output byte-identical. The reporter's
//! own control — replacing `int'(nb)` by the hand-written `{28'd0, nb}` — put the
//! same file at 25× of the frame-call gap, which is what identified the cast rather
//! than the frame call or the 128-bit part-select as the cost.
//!
//! ⚠️ **This still does NOT make the evaluation count correct.** A single
//! `int'(f())` names `f()` 32 times against iverilog's 1, because a `Call` is
//! conservatively "may be unknown" and an `int` is 32 bits wide. What round 35
//! removed is the MULTIPLICATION under nesting; what round 36 removed is paying for
//! bits the operand does not have. Both are moves up the ladder on a count that was
//! already wrong, never a regression — the residue is recorded in ROADMAP §2 rather
//! than left implicit. Values are unaffected: 64 cast cells over x/z-carrying
//! operands print byte-identically pre-35, pre-36, POST and under live iverilog 13.
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
/// `g` returns `int`, so its result is 32 bits wide: casts to `int` are same-width,
/// to `byte`/`shortint` narrowing, to `longint` widening.
fn ping_source(expr: &str) -> String {
    format!(
        "module tb;\n\
           function automatic int g(input int x); $display(\"ping\"); g = x + 1; endfunction\n\
           int r;\n\
           initial begin r = {expr}; $display(\"done %0d\", r); end\n\
         endmodule\n"
    )
}

/// The round-36 shape: the operand is **4 bits wide**, so every 2-state cast of it is
/// a WIDENING one and the target width is the thing that used to be paid for.
/// `n` is unsigned, `sn` signed — both 4-state, so the coercion is genuinely built.
fn narrow_ping_source(expr: &str) -> String {
    format!(
        "module tb;\n\
           function automatic logic [3:0] n();         $display(\"ping\"); n  = 4'b1x01; endfunction\n\
           function automatic logic signed [3:0] sn(); $display(\"ping\"); sn = -4'sd3;  endfunction\n\
           longint r;\n\
           initial begin r = {expr}; $display(\"done %0d\", r); end\n\
         endmodule\n"
    )
}

fn count_pings(src: &str) -> usize {
    let (out, code) = run(src);
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

fn pings(expr: &str) -> usize {
    count_pings(&ping_source(expr))
}

fn narrow_pings(expr: &str) -> usize {
    count_pings(&narrow_ping_source(expr))
}

/// The load-bearing round-35 assertion: NESTING no longer multiplies. Pinned as a
/// VALUE, not as "fewer than before" — a bound that cannot silently drift back.
#[test]
fn a_nested_two_state_cast_does_not_multiply_the_operand() {
    assert_eq!(pings("g(1)"), 1, "an uncast call is evaluated once");
    // A single SAME-WIDTH cast still fans out to that width: a `Call` is
    // conservatively "may be unknown", so its coercion is still built, and `int` is
    // exactly as wide as `g`'s return. Round 36 does not touch this cell — there is
    // no resize in front of the coercion to reorder. This is the residue, pinned so
    // that closing it is a deliberate change and not a surprise.
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
    assert_eq!(pings("bit'(g(1))"), 1, "a 1-bit 2-state cast is one term");
}

/// ⚠️ The soundness half. The guard is only sound if `expr_may_be_unknown` never
/// answers "known" for something that can carry an x or z — otherwise the coercion is
/// skipped and the x leaks through the 2-state cast, which is a silent-wrong.
///
/// Every value here is pinned against LIVE iverilog 13.0, and every operand is
/// x/z-carrying in a different way: a whole 4-state net, its negation (which forces
/// the widening `Concat[Replicate(sign), e]` path that the `Replicate` arm
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
    // ⚠️ Round 36 re-measured every field of this line after moving the coercion in
    // front of the extension: unchanged, which is the whole claim of that change.
    assert!(
        out.contains(
            "A=ffffffffffffff8a B=ffffffffffffff8a C=ffffffffffffff8a \
             D=0000000000000000 E=000000000000000a F=000000000000000a"
        ),
        "an x/z operand must still be coerced:\n{out}"
    );
}

/// ROUND 36, the headline: a widening 2-state cast costs the OPERAND's width.
///
/// ⚠️ `longint'(g(1))` moved **64 → 32** and that is the point of the change, not a
/// loosened bound. `g` returns 32 bits; the 32 extension bits `longint'` adds are a
/// zero- or sign-fill, and coercing a fill bit is the identity, so paying 64 terms
/// for them was pure waste. `int'(g(1))` (same width) and `byte'(g(1))`
/// (narrowing, already at the smaller width) are unmoved, which is what shows the
/// change is the WIDENING arm and nothing else.
///
/// ⚠️ `longint'(int'(g(1)))` is still **64**, and it is worth saying why it did not
/// move with its neighbour: the inner cast leaves a `Concat` of `CaseEq`, which is
/// known by construction, so NO coercion is built for the outer cast — it takes the
/// plain `extend_to` path, and `extend_to` derives its sign fill from the value it is
/// extending, naming that 32-term inner concat a second time. That doubling is a
/// property of sign extension in this IR, not of the coercion, so it is out of this
/// change's scope and pinned here rather than glossed.
#[test]
fn a_widening_two_state_cast_costs_the_operands_width() {
    assert_eq!(
        pings("longint'(g(1))"),
        32,
        "was 64: 32 fill bits were paid for"
    );
    assert_eq!(
        pings("byte'(g(1))"),
        8,
        "narrowing is already the smaller width"
    );
    assert_eq!(pings("shortint'(g(1))"), 16, "narrowing, unmoved");
    assert_eq!(
        pings("longint'(int'(g(1)))"),
        64,
        "no coercion is built; `extend_to`'s sign fill names the inner concat twice"
    );
    assert_eq!(
        pings("longint'(byte'(g(1)))"),
        16,
        "same shape one width down: 8-term inner concat, named twice"
    );
}

/// ROUND 36 on the reporting shape: a NARROW 4-state operand, which is where the
/// target-vs-operand asymmetry is largest. The reporter's `int'(nb)` with `nb` 4 bits
/// wide is exactly `int'(n())` here.
///
/// ⚠️ The signed operand costs the same 4 and not 5 (4 value bits + 1 coerced sign
/// bit). That is not the reordering being clever — it is the PRE-EXISTING sign
/// decision showing through: `cast_extend_signed` falls back to the mirror for an
/// operand it may not name twice, and the mirror calls a `Call`-rooted operand
/// unsigned, so the fill is a literal `1'b0` and there is no sign bit to coerce. The
/// value that follows from that (`int'(sn())` = `0000000d` where iverilog 13 gives
/// `fffffffd`) is a pre-existing gap recorded in ROADMAP §2, measured identical
/// before and after this change — the cell is here so a future fix to the SIGN moves
/// this count to 5 loudly rather than silently.
#[test]
fn a_narrow_operand_no_longer_pays_the_targets_width() {
    assert_eq!(narrow_pings("int'(n())"), 4, "was 32");
    assert_eq!(narrow_pings("longint'(n())"), 4, "was 64");
    assert_eq!(narrow_pings("shortint'(n())"), 4, "was 16");
    assert_eq!(
        narrow_pings("int'(sn())"),
        4,
        "was 32; fill is 1'b0, see above"
    );
    assert_eq!(narrow_pings("longint'(sn())"), 4, "was 64");
}

/// The value half of the round-36 equivalence argument, both signednesses, every
/// cell measured three ways (pre-36 binary / POST / live iverilog 13.0) and identical
/// in all three. These are the designs that would VIOLATE the argument if it were
/// wrong: an x in the sign bit of a signed operand, an x in the top bit of an
/// unsigned one, a 1-bit operand, a same-width cast, a narrowing cast, and a z fill.
#[test]
fn coercing_before_extending_is_the_same_value_both_signednesses() {
    let src = "module tb;\n\
                 logic signed [7:0] ss, p;\n\
                 logic [7:0] us;\n\
                 logic b1;\n\
                 logic signed [3:0] s4, n4;\n\
                 logic [63:0] wide;\n\
                 initial begin\n\
                   ss = 8'bx101_0011; us = 8'bx101_0011; b1 = 1'bx;\n\
                   s4 = 4'bz011; n4 = -4'sd3; p = 8'b1101_00x1;\n\
                   wide = 64'hFEDC_BA98_7654_321x;\n\
                   $display(\"W1=%h W2=%h W3=%h\", int'(ss), longint'(ss), byte'(s4));\n\
                   $display(\"W4=%h W5=%h\", int'(us), longint'(us));\n\
                   $display(\"W6=%h W7=%h\", int'(b1), byte'(b1));\n\
                   $display(\"E1=%h E2=%h E3=%h\", byte'(ss), byte'(us), bit'(b1));\n\
                   $display(\"N1=%h N2=%h N3=%h\", byte'(wide), int'(wide), bit'(ss));\n\
                   $display(\"Z1=%h Z2=%h\", int'(s4), longint'(-ss));\n\
                   $display(\"X1=%h X2=%h\", longint'(int'(ss)), int'(byte'(us)));\n\
                   $display(\"X3=%h X4=%h\", int'(-us), int'(~ss));\n\
                   $display(\"A1=%h A2=%h A3=%h\", int'(8'bxxxx_xxxx), int'(8'bzzzz_zzzz), \
                     longint'(4'bz1z0));\n\
                   $display(\"S1=%0d S2=%0d\", int'(p), longint'(p));\n\
                   $display(\"S4=%0d S5=%0d\", int'(n4), longint'(n4));\n\
                 end\n\
               endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    for line in [
        "W1=00000053 W2=0000000000000053 W3=03",
        "W4=00000053 W5=0000000000000053",
        "W6=00000000 W7=00",
        "E1=53 E2=53 E3=0",
        "N1=10 N2=76543210 N3=1",
        "Z1=00000003 Z2=0000000000000000",
        "X1=0000000000000053 X2=00000053",
        "X3=00000000 X4=0000002c",
        "A1=00000000 A2=00000000 A3=0000000000000004",
        // ⚠️ The sign-bit cells. `p`'s sign bit is a KNOWN 1 with an x in the middle,
        // so a coerce-then-replicate that got the order wrong would print a positive
        // number here; `n4` is a signed NET (repeatable, so `cast_extend_signed` does
        // adopt its sign) widening from 4 bits, which is the coerced-sign-fill path.
        "S1=-47 S2=-47",
        "S4=-3 S5=-3",
    ] {
        assert!(out.contains(line), "expected `{line}` in:\n{out}");
    }
}

/// ⚠️ THE REORDER'S OWN SILENT-WRONG, caught by reasoning about the premise and then
/// RUN — the whole suite was green over it.
///
/// The round-36 equivalence argument rests on `w` being the operand's ACTUAL width.
/// `ir_bits_of` answers `None` for a deferred HIERARCHICAL reference (and for a
/// `string` net, the string-producing system functions, and the element-typed
/// `pop`/array-reduction family), and the caller then FABRICATES 32 — the same trap
/// `lower_size_cast`'s doc records for the seal. Both orders are built on that guess,
/// but they degrade differently: coerce-after takes the low `tw` bits of a concat
/// whose real width is unknown (and the engine's post-resolve width table still
/// widens it correctly), while coerce-first FREEZES the guess into the low half.
///
/// Measured on `longint'(u1.w40)` with `logic [39:0] w40` in a child instance:
/// iverilog 13.0 and the pre-36 binary both print `0000001234567800`; an unguarded
/// reorder printed `0000000034567800` — the top 8 bits of the operand deleted, exit 0.
/// So the reorder is taken only where the width is a DECLARED fact.
#[test]
fn a_width_unknown_operand_keeps_the_resize_then_coerce_order() {
    let src = "module sub;\n\
                 logic signed [15:0] s;\n\
                 logic [39:0] w40;\n\
                 initial begin s = -16'sd3; w40 = 40'h12_3456_78xz; end\n\
               endmodule\n\
               module tb;\n\
                 sub u1();\n\
                 longint a, b; int e;\n\
                 initial begin\n\
                   #1;\n\
                   a = longint'(u1.s); b = longint'(u1.w40); e = int'(u1.w40);\n\
                   $display(\"A=%h B=%h E=%h\", a, b, e);\n\
                 end\n\
               endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    // ⚠️ `B` is iverilog 13.0's own answer and the load-bearing cell. `A` is NOT:
    // iverilog gives `fffffffffffffffd`, and vita's `000000000000fffd` is the
    // pre-existing hierarchical-placeholder sign gap that `cast_extend_signed`'s doc
    // records (ROADMAP §2) — pinned as vita's value, measured identical pre-36 and
    // POST, so that closing THAT gap is a deliberate change. `E` is the same operand
    // at the fabricated width itself, where no resize happens at all.
    assert!(
        out.contains("A=000000000000fffd B=0000001234567800 E=34567800"),
        "a fabricated width must not be frozen into the low half:\n{out}"
    );
}
