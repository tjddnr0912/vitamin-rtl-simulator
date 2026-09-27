//! ROADMAP §2 row 14 — CLOSED by §4.5.556. A module-scope initializer of 64 bits or less
//! folded through the width-UNLIMITED i64 walk (`const_eval_in_scope`), which holds a narrow
//! SIGNED leaf already sign-extended: `localparam logic signed [7:0] NM = -8'sd2; localparam
//! logic [63:0] X = NM ^ 64'h0;` was `fffffffffffffffe` where iverilog AND verilator print
//! `00000000000000fe` (§11.8.2 converts a signed operand of an unsigned expression at its own
//! width) — and vita contradicted itself, since the same expression over a FUNCTION LOCAL
//! folded `00…fe`.
//!
//! History, for the next reader of a routing change: the first attempt (2026-09-01) routed the
//! target through `eval_const_assign` and was reverted after three review rounds (a shift
//! count pushed into the context, closed as row 27; and an i64 bound on the TARGET only, which
//! evaluated a `logic signed [64:0]` leaf as a 64-bit operand). §4.5.542 made the wide walk
//! `fold_bits_at` evaluate a region the way §11.8.2 does, and §4.5.556 routes a declared target
//! of 64 bits or less whose initializer names a constant (or has an operand wider than 64
//! bits) through it — `Elaborator::param_init_at_declared_width`, in every binder at once.
//!
//! Values pinned to iverilog 13.0 and verilator 5.052.
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

/// The row itself, with the FUNCTION-LOCAL twin: both oracles answer `00…fe` for both
/// columns, and so does vita now (the module column was `ff…fe`).
#[test]
fn a_module_scope_name_converts_at_its_declared_width_like_a_local() {
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
    assert!(
        o.contains("OUT func=00000000000000fe mod=00000000000000fe"),
        "both oracles give 00…fe for both columns:\n{o}"
    );
}

/// The §4.5.366 residue, now CLOSED: at module scope `/`, `%` and `>>>` over a 64-bit
/// UNSIGNED declaration used to lose the sign (while `>>` was already right), because
/// the initializer folded in the width-unlimited lane where a 64-bit operand's top bit
/// reads as an i64 sign. The width-aware walk (`param_init_width_aware_ok`) applies
/// `const_i64_is_unsigned_at`, so all four columns are now the value BOTH oracles print.
#[test]
fn the_sixty_four_bit_unsigned_operators_keep_their_sign_at_module_scope() {
    let (o, ok) = run("module top;\n  \
           localparam [63:0] P = 64'hFFFFFFFFFFFFFFFF % 64'd10;\n  \
           localparam [63:0] Q = 64'hFFFFFFFFFFFFFFFF / 64'd10;\n  \
           localparam [63:0] R = 64'hFFFFFFFFFFFFFFFF >> 4;\n  \
           localparam [63:0] S = 64'hFFFFFFFFFFFFFFFF >>> 4;\n  \
           initial begin $display(\"OUT=%0d %0d %0d %0d\", P, Q, R, S); $finish; end\n\
         endmodule\n");
    assert!(ok, "vita failed:\n{o}");
    assert!(
        o.contains("OUT=5 1844674407370955161 1152921504606846975 1152921504606846975"),
        "all four columns are iverilog's and verilator's:\n{o}"
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
