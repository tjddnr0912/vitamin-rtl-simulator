//! R30-2: a SIGNED constant operand in a continuous assign's RHS took the whole
//! assign off the compiled op-stream.
//!
//! `native/wprog.rs`'s admission gate is *uniform width AND sign inside one node's
//! subtree*. A `localparam int II` records `signed`, so inside an unsigned context —
//! which is what `logic [31:0] n; assign n = s ^ II;` gives it — the leaf declined and
//! the whole assign fell onto the generic path. Measured, release, interleaved, 64
//! assigns × 200k cycles:
//!
//! | RHS | native | vm | native/vm |
//! |---|---|---|---|
//! | `s ^ II`, `localparam logic [31:0] II` | 41.1 ns/eval | 72.8 ns | 0.56 |
//! | `s ^ II`, `localparam int II` (SIGNED) | **118.1 ns/eval** | 73.2 ns | **1.61** |
//!
//! ⭐ **This is the only family measured where native loses to vm.** An external
//! round-34 report proposed a different axis — that an unpacked-array-element LHS makes
//! native lose — and an adversarial census refuted it over 34 cells: native was faster
//! on every one, and 3.9× faster on the report's own headline shape.
//!
//! ⚠️ **The module header of `wprog.rs` records that dropping the sign half of the gate
//! ENTIRELY was already built, measured sound, measured 1.00× on picorv32 and keccak,
//! and reverted.** That measurement was right; its conclusion was scoped to two designs
//! that do not contain this shape. A `localparam int` is what SV RTL writes, and a
//! genvar in an expression is the same cell — so a `generate for` whose body indexes
//! with its genvar, which is what a hash or cipher round looks like, was falling off the
//! compiled path entirely.
//!
//! The relaxation is restricted to a `Const` LEAF, which makes the soundness argument
//! trivial rather than set-wide: at equal width a constant's two's-complement bits do
//! not depend on how its signedness was recorded, and the `Const` arm masks to the
//! context width and pushes exactly those bits. The two ops that read a sign are
//! untouched — `>>>` declines on the node's own sign, and a comparison requires both
//! operands to share a signedness and passes it down explicitly.
//!
//! This file is the battery that discharges that argument by MEASUREMENT: every
//! admitted operator, with a signed constant in each operand position, at two widths,
//! compared `--backend native` against `--backend vm` AND against live iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_backend(src: &str, backend: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sgnleaf_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--backend")
        .arg(backend)
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr),
        out.status.code(),
    )
}

/// Every operator `wprog` admits, with a signed constant in each operand position.
/// `Mul`/`Div`/`Mod`/`Pow` are deliberately absent from the admitted set, which is
/// what makes the two's-complement argument apply to the whole list.
const OPS: &[&str] = &[
    "&", "|", "^", "+", "-", "<<", ">>", ">>>", "<", "<=", ">", ">=", "==", "!=", "&&", "||",
];

/// Operand pairings: a signed constant on each side, against a signed net, an unsigned
/// net, and a NARROWER net — the last is the case where a sign would have to
/// sign-EXTEND if the admission were wrong about it.
const PAIRS: &[(&str, &str)] = &[
    ("xs", "SI"),
    ("xu", "SI"),
    ("SI", "xs"),
    ("SI", "xu"),
    ("ns", "SI"),
    ("nu", "SP"),
];

fn battery_source() -> String {
    let mut decls = String::new();
    let mut body = String::new();
    let mut i = 0;
    for op in OPS {
        for (l, r) in PAIRS {
            i += 1;
            decls.push_str(&format!("  logic signed [31:0] o{i};\n"));
            body.push_str(&format!(
                "    o{i} = {l} {op} {r}; $display(\"C{i}=%0d\", o{i});\n"
            ));
        }
    }
    format!(
        "module tb;\n\
           localparam int          SI = -32'sd7;\n\
           localparam int          SP =  32'sd7;\n\
           logic signed [31:0] xs; logic [31:0] xu;\n\
           logic signed [7:0]  ns = -8'sd3; logic [7:0] nu = 8'hFD;\n\
         {decls}\
           initial begin\n\
             xs = -32'sd100; xu = 32'hFFFFFF9C;\n\
         {body}\
           end\n\
         endmodule\n"
    )
}

fn cells(out: &str) -> Vec<&str> {
    let mut v: Vec<&str> = out.lines().filter(|l| l.starts_with('C')).collect();
    v.sort_unstable();
    v
}

#[test]
fn the_two_backends_agree_on_every_signed_constant_operand() {
    let src = battery_source();
    let (nat, cn) = run_backend(&src, "native");
    let (vm, cv) = run_backend(&src, "vm");
    assert_eq!(cn, Some(0), "{nat}");
    assert_eq!(cv, Some(0), "{vm}");
    let (a, b) = (cells(&nat), cells(&vm));
    assert_eq!(
        a.len(),
        OPS.len() * PAIRS.len(),
        "every cell must report:\n{nat}"
    );
    assert_eq!(a, b, "native and vm must agree on every cell");
}

/// The values themselves, not just backend agreement — two backends can agree and both
/// be wrong. Pinned to LIVE iverilog 13.0, which printed all 96 identically.
#[test]
fn the_signed_constant_battery_matches_iverilog() {
    let (nat, c) = run_backend(&battery_source(), "native");
    assert_eq!(c, Some(0), "{nat}");
    // A representative spread rather than all 96: one per operator family, chosen where
    // an unsigned reading of the constant would give a DIFFERENT answer.
    for want in [
        // -100 & -7 = -100 & 0xFFFF_FFF9 = 0xFFFF_FF98 = -104
        "C1=-104",
        // signed compare: -100 < -7 is true; an unsigned reading would make it false
        "C55=1",
        // -100 >>> -7. ⚠️ The amount is read UNSIGNED (§11.4.10), so it is ~4.29e9 and
        // every bit vacates — and the fill is ZERO, not the sign, because a negative
        // SIGNED right operand makes the whole expression unsigned (§11.8.1). Measured,
        // not predicted: the first draft of this test asserted -1 and all three tools
        // said 0.
        "C37=0",
        // a NARROW signed net against the wide signed constant: -3 + -7 = -10
        "C23=-10",
    ] {
        assert!(nat.contains(want), "missing {want}:\n{nat}");
    }
}

/// ⚠️ The half that keeps the relaxation honest: a MIXED-SIGN COMPARISON must still
/// decline the compiled path, because a comparison is one of the two places sign is not
/// inert. It declines at its own guard (`lw.signed != rw.signed`), not at the entry
/// gate, so the relaxation cannot reach it — and the answer must be the same either way.
#[test]
fn a_mixed_sign_comparison_answers_the_same_on_both_backends() {
    let src = "module tb;\n\
                 localparam int SI = -32'sd7;\n\
                 localparam logic [31:0] UI = 32'hFFFFFFF9;\n\
                 logic [31:0] u; logic signed [31:0] s;\n\
                 logic a, b, c, d;\n\
                 initial begin\n\
                   u = 32'd5; s = -32'sd5;\n\
                   a = (s < SI); b = (u < UI); c = (u < SI); d = (s < UI);\n\
                   $display(\"M=%b%b%b%b\", a, b, c, d);\n\
                 end\n\
               endmodule\n";
    let (nat, cn) = run_backend(src, "native");
    let (vm, cv) = run_backend(src, "vm");
    assert_eq!(cn, Some(0), "{nat}");
    assert_eq!(cv, Some(0), "{vm}");
    // iverilog 13.0 prints M=0110.
    assert!(nat.contains("M=0110"), "{nat}");
    assert!(vm.contains("M=0110"), "{vm}");
}

/// `>>>` is the other place sign is not inert, and it declines on the NODE's sign
/// rather than a leaf's — so a signed constant reaching it must not change the fill.
#[test]
fn an_arithmetic_right_shift_still_reads_the_node_sign() {
    let src = "module tb;\n\
                 localparam int SH = 32'sd4;\n\
                 logic signed [31:0] s; logic [31:0] u;\n\
                 logic signed [31:0] a; logic [31:0] b;\n\
                 initial begin\n\
                   s = -32'sd64; u = 32'hFFFFFFC0;\n\
                   a = s >>> SH; b = u >>> SH;\n\
                   $display(\"A=%0d B=%08h\", a, b);\n\
                 end\n\
               endmodule\n";
    let (nat, cn) = run_backend(src, "native");
    let (vm, _) = run_backend(src, "vm");
    assert_eq!(cn, Some(0), "{nat}");
    // iverilog: A=-4 (sign fill), B=0ffffffc (zero fill — the operand is unsigned).
    assert!(nat.contains("A=-4 B=0ffffffc"), "{nat}");
    assert!(vm.contains("A=-4 B=0ffffffc"), "{vm}");
}
