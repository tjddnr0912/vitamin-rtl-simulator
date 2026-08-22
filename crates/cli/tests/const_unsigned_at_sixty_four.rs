//! The constant domain lost UNSIGNEDNESS at exactly 64 bits (IEEE 1800 §11.6.1).
//!
//! `eval_const_env_at` normalises a value by masking it to the context width with
//! the context's sign — but masking is disabled at 64 bits and beyond
//! (`masking = ctx_w > 0 && ctx_w < 64`), because there the i64 IS the width. Below
//! 64 an unsigned value comes back zero-extended, so every signed i64 operation
//! below happened to agree with the unsigned reading; at 64 it keeps its top bit
//! and the SIGN-SENSITIVE operations read it as negative.
//!
//! Measured on both oracles: `localparam L = ((64'd1 - 64'd2) > 64'd0) ? 111 : 222;`
//! is 111 and vita folded 222 — while the 63-bit twin (§4.5.347 closed it) and the
//! RUNTIME spelling of the same text were both already right, which is what made
//! this a constant-domain defect and not a comparison defect.
//!
//! Every expected value below was measured live on iverilog 13.0 and verilator
//! 5.050, which agree on all of them.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_u64ctx_{}_{n}", std::process::id()));
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
        out.status.code(),
    )
}

/// Fold `expr` BOTH as a `localparam` (the constant domain) and inline (the runtime
/// lowering). The runtime column is the internal discriminator: it was already
/// right for every cell here, so a test that pins only the constant one would not
/// show that the two spellings had disagreed.
fn both(expr: &str) -> (String, Option<i32>) {
    run(&format!(
        "module top;\n  localparam integer C = {expr};\n\
         \x20 initial begin $display(\"C=%0d R=%0d\", C, {expr}); $finish; end\nendmodule\n"
    ))
}

fn assert_both(expr: &str, want: i64) {
    let (out, code) = both(expr);
    assert_eq!(code, Some(0), "`{expr}`: nonzero exit;\n{out}");
    assert!(
        out.contains(&format!("C={want} R={want}")),
        "`{expr}`: want {want} from BOTH the constant and the runtime spelling;\n{out}"
    );
}

#[test]
fn an_unsigned_ordering_comparison_at_sixty_four_bits() {
    // The reported shape and its three siblings. `64'd1 - 64'd2` is the 64-bit
    // all-ones pattern, which is ABOVE zero unsigned and below it signed.
    assert_both("((64'd1 - 64'd2) > 64'd0)", 1);
    assert_both("((64'd1 - 64'd2) >= 64'd0)", 1);
    assert_both("((64'd1 - 64'd2) < 64'd0)", 0);
    assert_both("((64'd1 - 64'd2) <= 64'd0)", 0);
    assert_both("(64'hFFFFFFFF00000000 > 64'd0)", 1);
    assert_both("(64'hFFFFFFFF00000000 < 64'd1)", 0);
    assert_both("(64'hFFFFFFFFFFFFFFFF > 64'h7FFFFFFFFFFFFFFF)", 1);
}

#[test]
fn the_width_neighbours_pin_the_boundary() {
    // ⚠️ 63 bits was ALREADY right (masking zero-extends there) and is pinned so a
    // future change cannot drag it along; 32 bits is the ordinary case.
    assert_both("((63'd1 - 63'd2) > 63'd0)", 1);
    assert_both("((32'd1 - 32'd2) > 32'd0)", 1);
    // Mixed widths still size against each other, so the 64-bit operand wins.
    assert_both("((64'd1 - 64'd2) > 63'd0)", 1);
}

#[test]
fn above_sixty_four_bits_keeps_the_pre_slice_answer() {
    // ⚠️ A DELIBERATE DECLINE, pinned as a RESIDUE and not as support. The rule is
    // `w == 64`, not `w >= 64`: above 64 bits the i64 has ALREADY truncated the
    // value, so neither reading is the language's. Measured on both oracles, the
    // two directions disagree about which guess is better —
    //
    //   `(64'hFFFF_FFFF_FFFF_FFFF + 65'd1) > 64'hFFFF_FFFF_FFFF_FFFF` is 1, and the
    //   unsigned reading of the truncation answers 2;
    //   `((65'd1 - 65'd2) > 65'd0)` is 1, and the signed reading answers 0.
    //
    // Neither dominates, which is the definition of a guess, so >64 bits keeps the
    // pre-slice signed reading and stays ROADMAP §2's. The RUNTIME column below is
    // the oracle-correct one, which is exactly what makes this a recorded residue
    // rather than a claim of support.
    let (out, code) = both("((65'd1 - 65'd2) > 65'd0)");
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(
        out.contains("C=0 R=1"),
        "65 bits: the constant domain keeps its pre-slice answer while the runtime \
         lowering is right — both oracles say 1;\n{out}"
    );

    let (out, code) = both("((64'hFFFFFFFFFFFFFFFF + 65'd1) > 64'hFFFFFFFFFFFFFFFF)");
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(
        out.contains("C=1"),
        "the carry shape is right under the signed reading — moving >64 to unsigned \
         would break it, which is why the boundary is `== 64`;\n{out}"
    );
}

#[test]
fn a_signed_sixty_four_bit_comparison_stays_signed() {
    // ⚠️ THE OTHER HALF. The rule is keyed on the operand pair's SIGNEDNESS, not
    // on the width, so a signed pair keeps the signed reading.
    assert_both("((64'sd1 - 64'sd2) > 64'sd0)", 0);
    assert_both("(-64'sd1 < 64'sd0)", 1);
    // §11.6.1: ONE unsigned operand makes the whole comparison unsigned.
    assert_both("((64'sd1 - 64'sd2) > 64'd0)", 1);
}

#[test]
fn equality_and_bit_pattern_operators_are_untouched() {
    // `==`/`!=` compare the pattern, so they were right and must not move; the
    // same holds for `+` and `<<`.
    assert_both("((64'd1 - 64'd2) == 64'hFFFFFFFFFFFFFFFF)", 1);
    assert_both("((64'd1 - 64'd2) != 64'd0)", 1);
    assert_both("(64'hFFFFFFFFFFFFFFFF + 64'd1)", 0);
    // ⚠️ Not `assert_both`: the `integer` target truncates the constant column to
    // 32 bits while the runtime column keeps 64 — both oracles print exactly this.
    let (out, code) = both("(64'hFFFFFFFF00000000 << 4)");
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(
        out.contains("C=0 R=18446744004990074880"),
        "`<<` is a bit-pattern op and is untouched;\n{out}"
    );
}

#[test]
fn an_arithmetic_shift_is_logical_at_an_unsigned_context() {
    // ⚠️ §11.4.10: `>>>` is arithmetic ONLY when its left operand is signed, and
    // §11.6.1 has already converted that operand to the expression's type — which
    // is unsigned here. Both oracles take the logical branch even for a
    // DECLARED-SIGNED left operand, so `>>>` joins `>>` in the unsigned route.
    //
    // ⚠️⚠️ This was not a fix looking for a defect: the comparison redirect above
    // UNMASKED it. With `>>>` left on the signed path, 14 measured cells went from
    // correct to silently wrong — an adversarial-review find, and the reason the
    // predicate's own doc now lists both shifts.
    assert_both("((64'hFFFFFFFFFFFFFFFF >>> 60) > 64'd100)", 0);
    assert_both("((64'hFFFFFFFF00000000 >>> 32) > 64'd100)", 1);
    let (out, code) = run("module top;\n  parameter longint signed P = -100;\n\
         \x20 localparam integer C = ((P >>> 60) > 64'd100);\n\
         \x20 initial begin $display(\"C=%0d\", C); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(
        out.contains("C=0"),
        "a declared-signed left operand is still converted by the unsigned context \
         (both oracles);\n{out}"
    );

    // ⚠️ THE RESIDUE, pinned honestly: at MODULE scope a `localparam` initializer
    // folds through the width-unlimited `const_eval_in_scope`, which never reaches
    // this rule — so the same `>>>` is still wrong there while the runtime spelling
    // is right. ROADMAP §2 owns it; its prerequisite is the AST self-width pass.
    let (out, code) = run(
        "module top;\n  localparam longint unsigned M = (64'hFFFFFFFF00000000 >>> 32);\n\
         \x20 initial begin $display(\"M=%0d R=%0d\", M, (64'hFFFFFFFF00000000 >>> 32));\n\
         \x20 $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(
        out.contains("M=18446744073709551615 R=4294967295"),
        "both oracles say 4294967295 for BOTH columns — the constant one is the \
         recorded module-scope residue;\n{out}"
    );
}

#[test]
fn a_narrow_signed_leaf_is_reinterpreted_by_the_unsigned_context() {
    // ⚠️ §11.6.1: in a 64-bit UNSIGNED context a narrow signed operand is
    // reinterpreted unsigned and ZERO-extended. The walk skips `leaf_into_ctx`
    // wherever masking is off, so without this the leaf arrived sign-extended and
    // the unsigned route read the extension as magnitude — an adversarial-review
    // find that reached a `$bits` part-select width as loud → silently wrong.
    let (out, code) = run("module top;\n  parameter logic signed [7:0] P = -100;\n\
         \x20 localparam integer C = ((P / 8'sd3) > 64'd100);\n\
         \x20 localparam integer D = ((P / 8'sd3) > 63'd100);\n\
         \x20 initial begin $display(\"C=%0d D=%0d\", C, D); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(
        out.contains("C=0 D=0"),
        "−100 reads as the unsigned byte 156, so 156/3 = 52 is NOT above 100; the \
         63-bit sibling already answered that way (both oracles);\n{out}"
    );
}

#[test]
fn every_consumer_of_the_comparison_follows() {
    // The comparison's result is a 1-bit 0/1, so a wrong fold propagates into
    // whatever reads it — pinned because these are the shapes a design notices.
    assert_both("(((64'd1 - 64'd2) > 64'd0) ? 7 : 9)", 7);
    assert_both("(((64'd1 - 64'd2) > 64'd0) && 1'b1)", 1);
    assert_both("(!((64'd1 - 64'd2) > 64'd0))", 0);

    // A generate-if condition is the consumer with teeth: the wrong fold
    // ELABORATES THE OTHER BRANCH.
    let (out, code) = run("module top;\n  integer C;\n\
         \x20 generate if ((64'd1 - 64'd2) > 64'd0) begin : g initial C = 1;\n\
         \x20 end else begin : h initial C = 0; end endgenerate\n\
         \x20 initial begin #1 $display(\"C=%0d\", C); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(
        out.contains("C=1"),
        "the unsigned branch is elaborated;\n{out}"
    );
}

#[test]
fn a_parameter_carries_the_unsignedness_too() {
    // Not just literals: the rule reads the operands' declared signedness.
    let (out, code) = run(
        "module top;\n  localparam [63:0] A = 64'd1;\n  localparam [63:0] B = 64'd2;\n\
         \x20 localparam longint unsigned U = 64'd1;\n\
         \x20 localparam integer C = ((A - B) > 64'd0);\n\
         \x20 localparam integer D = ((U - 64'd2) > 64'd0);\n\
         \x20 initial begin $display(\"C=%0d D=%0d\", C, D); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "got:\n{out}");
    assert!(out.contains("C=1 D=1"), "both oracles;\n{out}");
}

#[test]
fn division_modulus_and_logical_shift_follow_the_same_rule() {
    // ⚠️ `/`, `%` and a logical `>>` are the SIGN-SENSITIVE members of the two
    // other arms, and they take the same rule — but only where the WIDTH-AWARE
    // walk owns the node. A const-function body is such a place; a module-scope
    // `localparam` is not (it folds width-unlimited through `const_eval_in_scope`,
    // which is ROADMAP §2's recorded prerequisite and still wrong there).
    //
    // `>>` was not merely wrong before — it was LOUD: `const_binop` declines a
    // logical shift of a negative value because the answer depends on a width it
    // cannot see. Here the width is known.
    let (out, code) = run(
        "module top;\n\
         \x20 function automatic longint unsigned fm(input longint unsigned x); fm = x % 64'd10; endfunction\n\
         \x20 function automatic longint unsigned fd(input longint unsigned x); fd = x / 64'd2; endfunction\n\
         \x20 function automatic longint unsigned fs(input longint unsigned x); fs = x >> 32; endfunction\n\
         \x20 localparam integer A = fm(64'hFFFFFFFFFFFFFFFF);\n\
         \x20 localparam integer B = fd(64'hFFFFFFFFFFFFFFFF);\n\
         \x20 localparam integer C = fs(64'hFFFFFFFF00000000);\n\
         \x20 initial begin $display(\"A=%0d B=%0d C=%0d\", A, B, C); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "`>>` used to be a LOUD reject here;\n{out}");
    assert!(
        out.contains("A=5 B=-1 C=-1"),
        "both oracles; B and C print as 32-bit signed because the localparam \
         target is `integer`;\n{out}"
    );
}
