//! ROADMAP §2 row 14 — BUILT, MEASURED, REVERTED (2026-09-01). This file pins the state
//! it was reverted TO, so the next attempt starts from measurements instead of a guess.
//!
//! **The defect.** `localparam logic signed [7:0] NM = -8'sd2; localparam logic [63:0] X
//! = NM ^ 64'h0;` is `fffffffffffffffe` in vita and `00000000000000fe` in iverilog AND
//! verilator. §11.8.2 converts a signed operand of an unsigned expression at ITS OWN
//! width. ⭐⭐ vita contradicts ITSELF: the same expression over a FUNCTION LOCAL folds
//! `00…fe`, because a local's declared width lives in the interpreter's `envw` and the
//! module-scope initializer folds through the width-UNLIMITED `const_eval_in_scope`,
//! which has no context to convert a leaf into.
//!
//! **What was built.** Route an initializer whose target has a DECLARED width/sign
//! through `eval_const_assign` — the width-aware entry the constant-function interpreter
//! already used. It works: a 44-cell census (4 declarations × 11 operators) went 27
//! divergent → 3, §4.5.366's four-operator module-scope residue closed, and the pinned
//! `>>>` self-contradiction (`18446744073709551615` constant vs `4294967295` runtime)
//! went away.
//!
//! **Why it was reverted — three review rounds, seven BLOCKING.**
//!
//! Rounds 1 and 2 were mine and were fixed: the gate has to demand provenance of every
//! LEAF as well as the target (`param_meta`'s width is a DEFAULT for an untyped parameter
//! and ABSENT for a `time` one); it must decline above the i64 lane (else it returns the
//! correct value TRUNCATED = a silent-for-silent trade); it must refuse an unsized FILL
//! operand (vita already sizes `'1` to 64 all-ones and the old fold's overflow DECLINE
//! was the only thing keeping that honest); the provenance set must answer THREE-valued
//! or an inner declaration that records nothing lets the scope walk vouch for an
//! ancestor's width; and the PACKAGE binder must not be routed, because a package's
//! stored value cannot go canonical while its consumers still fold unlimited (§2 row 26).
//!
//! ⚠️⚠️ **Round 3 found two more, both in the shared walk and both there since round 1:**
//!
//! 1. **§11.4.10 — a shift's RIGHT operand is self-determined and unsigned**, and the
//!    width-aware walk pushes the assignment context onto every leaf including that one,
//!    so a narrow SIGNED count sign-extends and the shift yields 0. 30 correct→wrong plus
//!    36 loud→wrong. ⭐ It is PRE-EXISTING and independent — the const-FUNCTION spelling
//!    is wrong today, with no routing involved — so it is its own row, and it has to be
//!    closed before this one.
//! 2. **The i64-lane bound was on the TARGET only.** A `logic signed [64:0]` LEAF whose
//!    value fits an i64 passes the provenance test and is then evaluated as a 64-bit
//!    operand, which changes what `/` and `%` answer. 83 cells.
//!
//! ⇒ the prerequisite is a width-aware walk that is correct on its own terms, and that is
//! a different slice with a different blast radius (it is shared with the constant-function
//! interpreter). Reverted rather than fixed forward a fourth time.
//!
//! The cells below are the reverted state. Every "vita" value here is KNOWN-WRONG against
//! the oracle named beside it; they are asserted so the next attempt can see instantly
//! what it moved, and so a partial fix cannot land unnoticed.
//!
//! Values pinned to iverilog 13.0 and verilator 5.050.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pdwp_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.success())
}

/// The row itself, with the FUNCTION-LOCAL twin that proves the machinery exists and is
/// right. Both oracles answer `00…fe` for both columns; vita splits them.
#[test]
fn a_module_scope_name_does_not_convert_at_its_declared_width_but_a_local_does() {
    let (o, ok) = run("module top;\n  \
           function automatic logic [63:0] f();\n    \
             logic signed [7:0] L;\n    L = -8'sd2;\n    f = L ^ 64'h0;\n  \
           endfunction\n  \
           localparam logic [63:0] XF = f();\n  \
           localparam logic signed [7:0] NM = -8'sd2;\n  \
           localparam logic [63:0] XM = NM ^ 64'h0;\n  \
           initial begin $display(\"OUT func=%h mod=%h\", XF, XM); $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    // KNOWN-WRONG on the `mod` column: both oracles give 00000000000000fe for BOTH.
    assert!(
        o.contains("OUT func=00000000000000fe mod=fffffffffffffffe"),
        "the split is the row; closing it makes both columns 00…fe:\n{o}"
    );
}

/// ⭐ THE PREREQUISITE, and the reason this row is not just "route the initializer":
/// §11.4.10 makes a shift's RIGHT operand self-determined and unsigned, and the
/// width-aware walk does not — so a narrow SIGNED count is sign-extended into the
/// enclosing context and `16'hFF01 << 3'b101` becomes 0.
///
/// ⚠️⚠️ This is PRE-EXISTING and reachable TODAY through a constant FUNCTION, with no
/// routing involved, which is what makes it a row of its own rather than a property of
/// the reverted change. Both oracles: `e020`.
#[test]
fn a_signed_shift_count_is_sign_extended_by_the_width_aware_walk() {
    let (o, ok) = run("module top;\n  \
           function automatic logic [15:0] f();\n    \
             logic signed [2:0] C;\n    logic [15:0] B;\n    \
             C = -3'sd3; B = 16'hFF01;\n    f = B << C;\n  \
           endfunction\n  \
           localparam logic [15:0] FN = f();\n  \
           initial begin $display(\"OUT=%h\", FN); $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    // KNOWN-WRONG: `3'b101` is the count 5, so both oracles give e020.
    assert!(
        o.contains("OUT=0000"),
        "closing §11.4.10 in the width-aware walk makes this e020:\n{o}"
    );
}

/// The §4.5.366 residue the routing also closed, pinned in its reverted state: at module
/// scope `/`, `%` and `>>>` over a 64-bit UNSIGNED declaration lose the sign, while `>>`
/// is already right. All four are iverilog's in the fixed state
/// (`5 1844674407370955161 1152921504606846975 1152921504606846975`).
#[test]
fn the_sixty_four_bit_unsigned_operators_still_lose_their_sign_at_module_scope() {
    let (o, ok) = run("module top;\n  \
           localparam [63:0] P = 64'hFFFFFFFFFFFFFFFF % 64'd10;\n  \
           localparam [63:0] Q = 64'hFFFFFFFFFFFFFFFF / 64'd10;\n  \
           localparam [63:0] R = 64'hFFFFFFFFFFFFFFFF >> 4;\n  \
           localparam [63:0] S = 64'hFFFFFFFFFFFFFFFF >>> 4;\n  \
           initial begin $display(\"OUT=%0d %0d %0d %0d\", P, Q, R, S); $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    assert!(
        o.contains("OUT=18446744073709551615 0 1152921504606846975 18446744073709551615"),
        "KNOWN-WRONG in three of four columns; `>>` is already right:\n{o}"
    );
}

/// ⚠️ The cells a fix must NOT move, so a future attempt cannot pass by making every
/// narrow signed name zero-extend. A shift takes its result's signedness from its LEFT
/// operand alone (Table 11-21) and a self-determined signed LITERAL keeps its own sign —
/// all three tools agree on every column here, today.
#[test]
fn a_shift_and_a_literal_keep_the_sign() {
    let (o, ok) = run("module top;\n  \
           localparam logic signed [7:0] NM = -8'sd2;\n  \
           localparam logic [63:0] A = NM << 0;\n  \
           localparam logic [63:0] B = NM >>> 0;\n  \
           localparam logic [63:0] C = (-8'sd2) ^ 64'h0;\n  \
           initial begin $display(\"OUT=%h %h %h\", A, B, C); $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    assert!(
        o.contains("OUT=fffffffffffffffe fffffffffffffffe fffffffffffffffe"),
        "all three tools agree here:\n{o}"
    );
}

/// …and the UNSIGNED declaration, which is correct today and stays correct: the missing
/// conversion is driven by the leaf's declared SIGN, not by its width.
#[test]
fn an_unsigned_declaration_is_already_correct() {
    let (o, ok) = run("module top;\n  localparam logic [7:0] U = 8'hFE;\n  \
           localparam logic [63:0] X = U ^ 64'h0;\n  \
           localparam logic [63:0] Y = U << 0;\n  \
           initial begin $display(\"OUT=%h %h\", X, Y); $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    assert!(o.contains("OUT=00000000000000fe 00000000000000fe"), "{o}");
}
