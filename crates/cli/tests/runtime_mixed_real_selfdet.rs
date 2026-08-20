//! §2 — a REAL operand must not widen its integral sibling.
//!
//! IEEE 1800 §11.8.1: when one operand of an arithmetic operator is real the
//! other is CONVERTED, and the conversion boundary is self-determined — the
//! integral side is read at its OWN width and converted afterwards. The engine
//! was handing both operands the binary's context width, and `sim_ir::selfwidth`
//! gives a real constant `{width: 64}`, so a mixed binary's context was 64 and
//! the integral side evaluated there:
//!
//!   `logic signed [3:0] s = -8;`
//!   `1.0 + (-s)`     → the negate ran at 64 bits ⇒ 9,   both oracles −7
//!   `1.0 + (s + s)`  → the sum ran at 64 bits   ⇒ −15,  both oracles 1
//!
//! The integral subtree alone was always right (`$display("%0d", -s)` is −8) and
//! an explicit `$itor(-s)` was right too — only the IMPLICIT conversion was
//! wrong. The same source text folded as a `localparam` has been correct since
//! §4.5.343, so one expression had two answers inside one design.
//!
//! The predicate is STATIC (`sim_ir::realness`, shared with elaborate and
//! memoized beside the width table) rather than read off the evaluated values:
//! deciding afterwards would evaluate the operands twice in the mixed case,
//! which draws `$random` twice and moves the RNG sequence.
//!
//! Oracle: iverilog 13.0 and verilator 5.050 agree on every value here.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_backend(src: &str, backend: Option<&str>) -> (String, Option<i32>, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_rmrs_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vita"));
    if let Some(b) = backend {
        cmd.arg("--backend").arg(b);
    }
    let out = cmd.arg(&path).output().expect("run vita");
    let _ = std::fs::remove_file(&path);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Evaluate `expr` at RUNTIME with `s` a 4-bit signed −8, and print it as a real
/// (`%f` — a `$rtoi` here would truncate to 32 bits and hide the very width this
/// file is about).
fn rt(expr: &str, want: &str) {
    let src = format!(
        "module top;\n\
         logic signed [3:0] s; logic [3:0] u; real a, r;\n\
         initial begin s = -8; u = 4'd8; a = 1.0;\n\
         r = {expr};\n\
         $display(\"R=%f\", r); #1 $finish; end\n\
         endmodule\n"
    );
    for backend in [None, Some("interp"), Some("vm"), Some("native")] {
        let (out, code, err) = run_backend(&src, backend);
        assert_eq!(code, Some(0), "`{expr}` on {backend:?}, stderr:\n{err}");
        assert!(
            out.contains(&format!("R={want}")),
            "`{expr}` on {backend:?} want R={want}; got:\n{out}"
        );
    }
}

/// The headline: the integral operand keeps its own width across the conversion
/// boundary, for every arithmetic operator and in both operand orders.
#[test]
fn a_real_sibling_does_not_widen_the_integral_operand() {
    rt("1.0 + (-s)", "-7.000000");
    rt("1.0 + (s + s)", "1.000000");
    rt("1.0 - (s + s)", "1.000000");
    rt("2.0 * (-s)", "-16.000000");
    rt("(-s) / 2.0", "-4.000000");
    rt("(-s) + 1.0", "-7.000000");
    // the real can be a param, a net or a literal — all three are the same rule
    rt("a + (-s)", "-7.000000");
    // …and an UNSIGNED narrow operand wraps the same way
    rt("1.0 + (u + u)", "1.000000");
    // A NEGATED REAL is the mirror shape: without the unary arm in the shared
    // rule the node stops being real and the integral sibling widens again.
    rt("(-a) + (s + s)", "-1.000000");
}

/// A `real d[]` ELEMENT is real even though its net kind is `DynArray` — the
/// sidecar that makes the engine store those elements as f64 is what says so, and
/// it has to reach the width table (`build_full`) as well as the shared rule.
/// Both halves are invisible to a library-level test, where every sidecar is
/// empty by construction, so this one runs through the CLI.
#[test]
fn a_real_dynamic_array_element_is_real() {
    let (out, code, err) = run_backend(
        "module top;\n\
         logic signed [3:0] s; real d[]; real r;\n\
         initial begin s = -8; d = new[2]; d[0] = 0.5; d[1] = 0.5;\n\
         r = d[0] + (-s);\n\
         $display(\"R=%f\", r); #1 $finish; end\n\
         endmodule\n",
        None,
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("R=-7.500000"),
        "a real array element must not widen its integral sibling; got:\n{out}"
    );
    // The elaborate-side gate reads the SAME rule — it must still refuse the
    // element in a concatenation (this is what separates "the rule lost the
    // sidecar" from "only the engine's copy lost it").
    let (_, code, err) = run_backend(
        "module top;\n\
         real d[];\n\
         initial begin d = new[1]; d[0] = 0.5;\n\
         $display(\"%b\", {d[0], 1'b0}); #1 $finish; end\n\
         endmodule\n",
        None,
    );
    assert_eq!(
        code,
        Some(1),
        "real element in a concat must stay loud:\n{err}"
    );
    assert!(err.contains("real"), "{err}");
}

/// A TERNARY with a real arm is a real-valued expression, so the taken integral
/// arm converts at the same self-determined boundary — and the node must actually
/// PRODUCE a real, because the binary arms above now trust the static claim that
/// it is one. Leaving it integral made `(sel ? 1.0 : (s+s)) - (s+s)` answer −16
/// where both oracles answer 0: before this slice BOTH sides were wrong the same
/// way and cancelled, so fixing one side alone would have been a regression.
#[test]
fn a_ternary_with_a_real_arm_produces_a_real() {
    for (setup, expr, want, fmt) in [
        ("sel = 0;", "(sel ? 1.0 : (s + s)) - (s + s)", "0", "%0d"),
        ("sel = 0;", "(sel ? 1.0 : (s + s)) + (s + s)", "0", "%0d"),
        ("sel = 0;", "(1'b0 ? 1.0 : (s + s)) - (s + s)", "0", "%0d"),
        ("sel = 0;", "(sel ? 1.0 : (s + s))", "0.000000", "%f"),
        ("sel = 1;", "(sel ? (-s) : 2.0)", "-8.000000", "%f"),
        ("sel = 1;", "(sel ? (u + u) : 2.0)", "0.000000", "%f"),
    ] {
        let src = format!(
            "module top;\n\
             logic signed [3:0] s; logic [3:0] u; bit sel; real r; integer q;\n\
             initial begin s = -8; u = 4'd8; {setup}\n\
             {} = {expr};\n\
             $display(\"R={fmt}\", {}); #1 $finish; end\n\
             endmodule\n",
            if fmt == "%f" { "r" } else { "q" },
            if fmt == "%f" { "r" } else { "q" }
        );
        let (out, code, err) = run_backend(&src, None);
        assert_eq!(code, Some(0), "`{expr}`, stderr:\n{err}");
        assert!(
            out.contains(&format!("R={want}")),
            "`{expr}` want {want}; got:\n{out}"
        );
    }
}

/// The rule reaches the COMPARISONS too, whose operands are otherwise sized
/// against each other. Without this the slice disagreed with itself inside one
/// design — `1.0 + (-s)` read `-s` at 4 bits while `1.0 > (-s)` read it at 64 —
/// and an `if` built on the comparison took the wrong branch.
#[test]
fn a_mixed_real_comparison_reads_its_integral_side_self_determined() {
    for (expr, want) in [
        ("1.0 > (-s)", 1),
        ("(-s) < 1.0", 1),
        ("(-s) == -8.0", 1),
        ("(-s) >= 1.0", 0),
        ("(s + s) == 0.0", 1),
        // integral-only comparisons keep their mutual sizing
        ("(-s) > 4'sd0", 0),
        ("(-s) > 32'sd0", 1),
    ] {
        let src = format!(
            "module top;\n\
             logic signed [3:0] s; integer q;\n\
             initial begin s = -8; q = {expr};\n\
             $display(\"R=%0d\", q); #1 $finish; end\n\
             endmodule\n"
        );
        let (out, code, err) = run_backend(&src, None);
        assert_eq!(code, Some(0), "`{expr}`, stderr:\n{err}");
        assert!(
            out.contains(&format!("R={want}")),
            "`{expr}` want {want}; got:\n{out}"
        );
    }
    // …and the control flow built on one takes the right branch.
    let (out, code, err) = run_backend(
        "module top;\n\
         logic signed [3:0] s; integer q;\n\
         initial begin s = -8; if (1.0 > (-s)) q = 111; else q = 222;\n\
         $display(\"R=%0d\", q); #1 $finish; end\n\
         endmodule\n",
        None,
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("R=111"), "got:\n{out}");
}

/// A leaf, an explicit `$itor`, and the integral subtree on its own were always
/// right — pinned so the fix cannot be "achieved" by breaking them.
#[test]
fn the_neighbours_that_were_already_right_stay_right() {
    rt("1.0 + s", "-7.000000");
    rt("1.0 + $itor(-s)", "-7.000000");
    let (out, code, err) = run_backend(
        "module top;\n\
         logic signed [3:0] s;\n\
         initial begin s = -8;\n\
         $display(\"R=%0d %0d\", -s, s + s); #1 $finish; end\n\
         endmodule\n",
        None,
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("R=-8 0"),
        "self-determined integral; got:\n{out}"
    );
}

/// A purely INTEGRAL binary must still take the context width — that is the half
/// of the rule this change must not break, and the widths where it shows.
#[test]
fn a_purely_integral_binary_still_takes_its_context() {
    for (decl, expr, want) in [
        ("integer q", "-s", "8"),
        ("logic signed [3:0] q", "-s", "-8"),
        ("integer q", "s + s", "-16"),
        ("logic signed [3:0] q", "s + s", "0"),
        ("logic signed [7:0] q", "s * 4'sd2", "-16"),
        ("integer q", "(s + s) * 2", "-32"),
    ] {
        let src = format!(
            "module top;\n\
             logic signed [3:0] s; {decl};\n\
             initial begin s = -8; q = {expr};\n\
             $display(\"R=%0d\", q); #1 $finish; end\n\
             endmodule\n"
        );
        let (out, code, err) = run_backend(&src, None);
        assert_eq!(code, Some(0), "`{decl} = {expr}`, stderr:\n{err}");
        assert!(
            out.contains(&format!("R={want}")),
            "`{decl} = {expr}` want {want}; got:\n{out}"
        );
    }
}

/// The constant domain answered this correctly first (§4.5.343). Both evaluators
/// of one source line agree now, which is the self-inconsistency that made this
/// worth fixing.
#[test]
fn the_constant_and_runtime_evaluators_agree() {
    let (out, code, err) = run_backend(
        "module top;\n\
         localparam signed [3:0] S = -8;\n\
         localparam real C1 = 1.0 + (-S);\n\
         localparam real C2 = 1.0 + (S + S);\n\
         logic signed [3:0] s; real r1, r2;\n\
         initial begin s = -8; r1 = 1.0 + (-s); r2 = 1.0 + (s + s);\n\
         $display(\"R=%f %f %f %f\", C1, r1, C2, r2); #1 $finish; end\n\
         endmodule\n",
        None,
    );
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(
        out.contains("R=-7.000000 -7.000000 1.000000 1.000000"),
        "the localparam and the runtime must agree; got:\n{out}"
    );
}
