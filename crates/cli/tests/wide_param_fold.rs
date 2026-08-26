//! Wide (>64-bit) `localparam` values built from CARRY-FREE operators.
//!
//! External aes_top round-3 §3.3. `localparam logic [127:0] K = 128'h…` worked and
//! `= {8'he1, 120'h0}` did not — `E3009 … not a foldable constant expression` — which
//! is the spelling every crypto IP actually uses for a round constant or a mask.
//!
//! The >64-bit domain (`wide_param_bits`, `ir::BitPacked`, `resize_bits`) was already
//! standing on the DECLARATION path and its entry condition was already right; what
//! was missing is arms. `fold_init` had three: a literal, a fill literal, and a
//! parenthesised one.
//!
//! ADMISSION IS "CARRY-FREE", not "whatever was convenient". Concat, replication, a
//! size cast, a constant LOGICAL shift, `&`/`|`/`^` and `~` each decide a result bit
//! from operand bits at known positions. `+`/`-`/`*` need a carry chain across 128
//! bits and `>>>` reads the sign bit; implementing either here would be a second
//! spelling of the engine's arithmetic, and a subtly wrong one is a silent wrong
//! PARAMETER — P0-5. Those stay loud, and the last three tests pin that.
//!
//! ORACLE: iverilog 13.0. Every expected value below is its output for the same
//! source, `%032h`.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_wpf_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// `A` is a wide parameter so the name-resolving arm is exercised too.
fn design(expr: &str) -> String {
    format!(
        "module tb;\n\
         \x20 localparam logic [127:0] A = 128'h0123456789abcdef_fedcba9876543210;\n\
         \x20 localparam logic [127:0] C = 128'hffffffffffffffff_0000000000000000;\n\
         \x20 localparam logic [127:0] B = {expr};\n\
         \x20 initial $display(\"B=%032h\", B);\n\
         endmodule\n"
    )
}

#[test]
fn carry_free_operators_fold_to_the_iverilog_value() {
    for (expr, want) in [
        // the two the report named
        ("{8'he1, 120'h0}", "e1000000000000000000000000000000"),
        ("128'(8'he1) << 120", "e1000000000000000000000000000000"),
        // the rest of the carry-free set
        ("{16{8'hAB}}", "abababababababababababababababab"),
        ("~A", "fedcba98765432100123456789abcdef"),
        ("A >> 8", "000123456789abcdeffedcba98765432"),
        ("A << 4", "123456789abcdeffedcba98765432100"),
        ("A | 128'h1", "0123456789abcdeffedcba9876543211"),
        ("A & 128'hff", "00000000000000000000000000000010"),
        (
            "A ^ {2{64'hf0f0f0f0f0f0f0f0}}",
            "f1d3b597795b3d1f0e2c4a6886a4c2e0",
        ),
        // a literal still folds exactly as before — this arm must not have moved
        ("128'h1", "00000000000000000000000000000001"),
    ] {
        let out = run(&design(expr));
        assert!(
            out.contains(&format!("B={want}")),
            "`{expr}` must fold to iverilog's value B={want}:\n{out}"
        );
    }
}

#[test]
fn unknown_bits_ride_through_the_placement_arms() {
    // Concat MOVES bits, it does not read them, so an x survives at its position —
    // iverilog prints the same. (The value-reading arms decline on unknowns instead;
    // see the test below.)
    let out = run(&design("{4'hx, 124'h0}"));
    assert!(
        out.contains("B=x0000000000000000000000000000000"),
        "an x must survive a concat at its own position:\n{out}"
    );
}

#[test]
fn a_bitwise_operator_on_unknown_bits_declines_instead_of_guessing() {
    // A 4-state `&`/`|`/`^`/`~` table belongs in ONE place and it is not the constant
    // folder. Declining leaves the caller's loud reject — never a guessed bit in a
    // parameter, which nothing downstream could tell from a real value.
    let out = run(&design("~{4'hx, 124'h0}"));
    assert!(out.contains("E3009"), "must decline, not guess:\n{out}");
}

#[test]
fn carrying_operators_fold_in_the_wide_lane() {
    // ⭐ The admission boundary this file used to state was "carry-free only": `+`, `*`
    // and `>>>` refused, "rather than grow a 128-bit adder / sign-aware shifter inside
    // the elaborator". That was a statement about RISK, and the risk is payable — the
    // limbs are the same `BitPacked` the engine uses and the operand-extension rule is
    // written once, at the single place both operands reach a common width. Values
    // measured on iverilog 13.0 AND verilator 5.050.
    for (expr, want) in [
        ("A + 1", "0123456789abcdeffedcba9876543211"),
        ("A * 2", "02468acf13579bdffdb97530eca86420"),
        ("A >>> 4", "00123456789abcdeffedcba987654321"),
    ] {
        let out = run(&design(expr));
        assert!(out.contains(want), "`{expr}` should fold to {want}:\n{out}");
    }
    // ⚠️ Division and modulus WERE still out, for a stated reason — "a wide divide is
    // a different algorithm" and a second, subtly wrong spelling of it produces a
    // silent wrong PARAMETER. Round 34 removed the second spelling instead of writing
    // one: the engine's limb `mw_divmod` moved into `sim-ir` and both domains call it.
    // Values are live iverilog 13.0 AND an independent Python golden.
    for (expr, want) in [
        ("A / 3", "00611722833944a554f43e32d21c10b0"),
        ("A % 3", "00000000000000000000000000000000"),
    ] {
        let out = run(&design(expr));
        assert!(out.contains(want), "`{expr}` should fold to {want}:\n{out}");
    }
    // What has no value still stays loud: §11.4.3 makes `x / 0` an `x`, and iverilog
    // and vita's own runtime both give x there — verilator's 0 is a 2-state artifact.
    for expr in ["A / (C - C)", "A % (C - C)"] {
        let out = run(&design(expr));
        assert!(out.contains("E3009"), "`{expr}` must stay loud:\n{out}");
    }
}

#[test]
fn a_replication_wider_than_a_net_may_be_declines() {
    // Fail-closed cap: `{N{…}}` with a hostile N must not allocate before the caller
    // ever sees it. `MAX_NET_WIDTH` is the width a declaration already lives under, so
    // anything past it cannot land anywhere legal.
    let out = run(&design("{100000000{8'hAB}}"));
    assert!(
        out.contains("E3009"),
        "hostile replication must decline:\n{out}"
    );
}

#[test]
fn a_narrow_declaration_is_untouched_by_the_wide_lane() {
    // The wide lane is entered only when the i64 fold has already declined AND the
    // declared width is over 64. A 32-bit parameter written with the same operators
    // keeps its integer identity — it is still usable as a width and a bound.
    let out = run("module tb;\n\
         \x20 localparam logic [31:0] W = {8'h00, 8'h00, 8'h00, 8'h20};\n\
         \x20 logic [W-1:0] v;\n\
         \x20 initial $display(\"W=%08h w=%0d\", W, $bits(v));\n\
         endmodule\n");
    assert!(
        out.contains("W=00000020"),
        "narrow concat still folds:\n{out}"
    );
    // The value keeps its INTEGER identity: still usable as a declared width (iverilog
    // agrees, w=32). The wide lane is entered only past 64 bits, so this never touched
    // it — and if it ever did, the width would come back as something else.
    assert!(
        out.contains("w=32"),
        "…and is still usable as a width:\n{out}"
    );
}

#[test]
fn the_name_arm_resolves_the_name_that_was_written() {
    // DISCRIMINATOR for the resolver: the design has TWO wide parameters, so a lookup
    // that returns "some wide parameter" instead of THIS one produces a different
    // value rather than an error. Every other test here names only `A`.
    let out = run(&design("~C"));
    assert!(
        out.contains("B=0000000000000000ffffffffffffffff"),
        "`~C` must fold C, not the other wide parameter (iverilog agrees):\n{out}"
    );
}

#[test]
fn a_bitwise_operator_takes_the_wider_operand_width() {
    // DISCRIMINATOR for §11.4.8: the result width is max(lhs, rhs), and the NARROW
    // side must be extended. Written narrow-on-the-LEFT on purpose — with the wide
    // operand on the left, taking the left width alone happens to give the same
    // answer, so that spelling cannot tell the two rules apart. iverilog agrees.
    let out = run(&design("8'hff | A"));
    assert!(
        out.contains("B=0123456789abcdeffedcba98765432ff"),
        "the 8-bit operand must be extended to 128, not the other way round:\n{out}"
    );
}

#[test]
fn unknown_bits_decline_in_every_value_reading_arm_not_just_not() {
    // DISCRIMINATOR for the `&`/`|`/`^` half of the unknown rule — the `~` test above
    // covers only the unary arm. iverilog folds this (to `x1234…`); vita declines
    // rather than carry a second 4-state table, and that refusal is the thing under
    // test: a mutant that drops the check produces a plausible NUMBER here.
    let out = run(&design("A | {4'hx, 124'h0}"));
    assert!(
        out.contains("E3009"),
        "a bitwise op on unknowns must decline:\n{out}"
    );
    assert!(!out.contains("B="), "…and must not produce a value:\n{out}");
}

#[test]
fn an_unsized_fill_literal_has_no_self_width_and_declines() {
    // §5.7.1 makes `'1` CONTEXT-determined, so inside a concat — where each part is
    // self-determined (§11.4.12) — it has no width at all. Only `fold_init`, which
    // knows the target, may fold one. iverilog does produce a value here, but the
    // construct is not well-formed (a concat operand must be sized), so this pins the
    // REFUSAL, not iverilog's number: a mutant that gives the fill a 1-bit self width
    // would silently shift every other part of the concat by 7 places.
    let out = run(&design("{'1, 120'h0}"));
    assert!(
        out.contains("E3009"),
        "an unsized fill in a concat must decline:\n{out}"
    );
}
