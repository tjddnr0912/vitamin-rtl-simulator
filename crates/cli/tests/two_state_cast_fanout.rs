//! Round-35/36 — a 2-state cast named its operand once per bit of the WRONG WIDTH.
//!
//! `int'(e)` was lowered by `coerce_two_state` into a `Concat` of one
//! `CaseEq(Select(e, i), 1'b1)` per bit it covers, and the engine walks that DAG as a
//! TREE. So a 2-state prim cast multiplied the operand's evaluation cost by the width
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
//! | cast | operand evals, pre-35 | pre-36 | round 36 | single-mention | iverilog 13 |
//! |---|---|---|---|---|---|
//! | none | 1 | 1 | 1 | 1 | 1 |
//! | `byte'` (32-bit operand, narrowing) | 8 | 8 | 8 | **1** | 1 |
//! | `int'` (32-bit operand, same width) | 32 | 32 | 32 | **1** | 1 |
//! | `longint'` (32-bit operand, widening) | 64 | 64 | 32 | **1** | 1 |
//! | `int'(int'(x))` | 1024 | 32 | 32 | **1** | 1 |
//! | `int'` of a 4-bit operand | 32 | 32 | 4 | **1** | 1 |
//! | `longint'` of a 4-bit operand | 64 | 64 | 4 | **1** | 1 |
//!
//! The single-mention column: the coercion is now `SysFunc TwoState` (x/z→0, the
//! operand's own width and sign) and a signed widening is the single-mention
//! ternary `$signed(1'b1 ? $signed(e) : <n-bit signed 0>)`, so every cell names
//! its operand once, as iverilog 13 and verilator 5 do. The pins below that
//! asserted the old per-bit counts were CONVERTED to the oracle count (1); none
//! was deleted.
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
//! Values over x/z-carrying operands are pinned against live iverilog 13 below and
//! are unchanged by the single-mention spelling.
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
    // A `Call` is conservatively "may be unknown", so the coercion is still built,
    // but `TwoState` names it once. Converted pin: was 32 (one mention per bit),
    // iverilog 13 and verilator 5 print one `ping`.
    assert_eq!(pings("int'(g(1))"), 1);
    // The OUTER cast of a nested pair sees `TwoState`, which is known by
    // construction, so it does not rebuild. Converted pins: were 32.
    assert_eq!(pings("int'(int'(g(1)))"), 1);
    assert_eq!(pings("int'(int'(int'(g(1))))"), 1);
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

/// A widening, narrowing or nested 2-state cast names its operand ONCE.
///
/// Converted pins (old → new, new = iverilog 13 and verilator 5): `longint'(g(1))`
/// 32 → 1, `byte'(g(1))` 8 → 1, `shortint'(g(1))` 16 → 1, `longint'(int'(g(1)))`
/// 64 → 1, `longint'(byte'(g(1)))` 16 → 1. The last two used to double because the
/// outer cast built `extend_to`'s `Select{Bit}` sign fill over the inner coercion;
/// a signed operand that may not be repeated is now extended by the single-mention
/// ternary instead.
#[test]
fn a_widening_two_state_cast_costs_the_operands_width() {
    assert_eq!(pings("longint'(g(1))"), 1, "was 32");
    assert_eq!(pings("byte'(g(1))"), 1, "was 8");
    assert_eq!(pings("shortint'(g(1))"), 1, "was 16");
    assert_eq!(pings("longint'(int'(g(1)))"), 1, "was 64");
    assert_eq!(pings("longint'(byte'(g(1)))"), 1, "was 16");
}

/// The reporting shape: a NARROW 4-state operand. The reporter's `int'(nb)` with
/// `nb` 4 bits wide is exactly `int'(n())` here.
///
/// Converted pins (old → new, new = iverilog 13 and verilator 5): every count
/// 4 → 1. The signed operand is now extended by its canonical sign through the
/// single-mention ternary, so its value moved too: `int'(sn())` printed `13`
/// (zero-extended −3) and now prints `-3`, both oracles' value.
#[test]
fn a_narrow_operand_no_longer_pays_the_targets_width() {
    assert_eq!(narrow_pings("int'(n())"), 1, "was 4");
    assert_eq!(narrow_pings("longint'(n())"), 1, "was 4");
    assert_eq!(narrow_pings("shortint'(n())"), 1, "was 4");
    assert_eq!(narrow_pings("int'(sn())"), 1, "was 4");
    assert_eq!(narrow_pings("longint'(sn())"), 1, "was 4");
    for (expr, want) in [
        ("int'(n())", "done 9"),
        ("longint'(n())", "done 9"),
        ("int'(sn())", "done -3"),
        ("longint'(sn())", "done -3"),
    ] {
        let (out, code) = run(&narrow_ping_source(expr));
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains(want), "`{expr}` expected `{want}`:\n{out}");
    }
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
    // ⚠️ `B` is iverilog 13.0's own answer and the load-bearing cell. `A` is the
    // oracle value too since the placeholder's declared shape is recorded when it
    // is created (`hier_leaf_shape.rs`): iverilog 13.0 and verilator 5.052 both
    // print `fffffffffffffffd`; it was `000000000000fffd`, the hierarchical-
    // placeholder sign gap `cast_extend_signed`'s doc describes. `E` is the same
    // operand at the fabricated width itself, where no resize happens at all.
    assert!(
        out.contains("A=fffffffffffffffd B=0000001234567800 E=34567800"),
        "a fabricated width must not be frozen into the low half:\n{out}"
    );
    // The declared width of `u1.w40` is now known, so the FABRICATED-width order
    // is kept reachable through an instance inside a generate scope, a path the
    // declaration walk declines: `B` and `E` are iverilog 13.0's and verilator
    // 5.052's values; `A` there is still the placeholder sign gap (both oracles
    // print `fffffffffffffffd`), pinned as vita's value.
    let (out, code) = run(&src
        .replace("sub u1();", "if (1) begin : g sub u1(); end")
        .replace("u1.", "g.u1."));
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("A=000000000000fffd B=0000001234567800 E=34567800"),
        "a fabricated width must not be frozen into the low half:\n{out}"
    );
}
