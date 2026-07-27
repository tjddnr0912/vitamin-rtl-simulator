//! A generate control expression that mentions a real was loud:
//! `generate if (R/2 > 2)` and `for (i = 0; i < R/2; …)` both reported "is not a
//! constant" because the fold only ever ran in the integer domain, where a real
//! parameter has no value. iverilog takes the then-branch and runs three
//! iterations respectively.
//!
//! The condition now folds through `const_truth_in_scope`: integer domain first,
//! and if that declines and the expression mentions a real, the REAL domain — with
//! the truth value taken there. That ordering is the point. `R/2 > 2` is a real
//! comparison whose 1-bit result is what crosses into the integer world;
//! converting `R` to an integer first would decide the branch on 2 instead of
//! 2.5, which is the leaf-conversion mistake the real domain exists to avoid.
//! Every value pinned to LIVE iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_grc_{}_{n}", std::process::id()));
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

/// A generate-if whose condition is a real comparison, both polarities, and a
/// generate-for whose bound is a real expression.
#[test]
fn generate_control_folds_in_the_real_domain() {
    let (out, c) = run(
        "module m;\n  localparam real R = 5.0;\n  localparam real T = 1.5;\n\
           generate if (R/2 > 2) begin : g1 initial $display(\"A=then\"); end\n\
           else begin : h1 initial $display(\"A=else\"); end endgenerate\n\
           generate if (T > 2.0) begin : g2 initial $display(\"B=then\"); end\n\
           else begin : h2 initial $display(\"B=else\"); end endgenerate\n\
           genvar i;\n\
           generate for (i = 0; i < R/2; i = i + 1) begin : f\n\
             initial $display(\"C=%0d\", i); end endgenerate\n\
           initial #1 $finish;\nendmodule\n",
    );
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    // 2.5 > 2 is TRUE — deciding on a truncated 2 would take the else branch.
    assert!(
        out.contains("A=then"),
        "real comparison is true; got:\n{out}"
    );
    assert!(
        out.contains("B=else"),
        "real comparison is false; got:\n{out}"
    );
    // i < 2.5 runs for i = 0,1,2 — a truncated bound would stop at 2.
    for w in ["C=0", "C=1", "C=2"] {
        assert!(out.contains(w), "expected `{w}`; got:\n{out}");
    }
    assert!(!out.contains("C=3"), "bound must not over-run; got:\n{out}");
}

/// The fractional part must actually decide the branch — this is the case a
/// leaf conversion would get wrong, since 2.5 and 2 disagree against `> 2`.
#[test]
fn the_fractional_part_decides_the_branch() {
    for (val, want) in [("2.5", "then"), ("2.0", "else"), ("1.9", "else")] {
        let (out, c) = run(&format!(
            "module m;\n  localparam real R = {val};\n\
               generate if (R > 2) begin : g initial $display(\"P=then\"); end\n\
               else begin : h initial $display(\"P=else\"); end endgenerate\n\
               initial #1 $finish;\nendmodule\n"
        ));
        assert_eq!(c, Some(0), "R = {val}; got:\n{out}");
        assert!(out.contains(&format!("P={want}")), "R = {val}; got:\n{out}");
    }
}

/// An all-integer control expression must be untouched — the integer domain is
/// still tried first, so nothing about ordinary generate code changes.
#[test]
fn integer_control_expressions_are_unaffected() {
    let (out, c) = run("module m;\n  localparam int N = 3;\n  genvar i;\n\
           generate if (N > 2) begin : g initial $display(\"I=then\"); end\n\
           else begin : h initial $display(\"I=else\"); end endgenerate\n\
           generate for (i = 0; i < N; i = i + 1) begin : f\n\
             initial $display(\"J=%0d\", i); end endgenerate\n\
           initial #1 $finish;\nendmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    assert!(out.contains("I=then"), "integer if; got:\n{out}");
    for w in ["J=0", "J=1", "J=2"] {
        assert!(out.contains(w), "expected `{w}`; got:\n{out}");
    }
    assert!(!out.contains("J=3"), "integer bound; got:\n{out}");
}

/// A genuinely non-constant condition must still be LOUD — the real fallback
/// must not turn "cannot fold" into a guess.
#[test]
fn a_non_constant_condition_is_still_loud() {
    let (out, c) = run("module m;\n  logic v;\n\
           generate if (v) begin : g initial $display(\"X\"); end endgenerate\n\
           initial #1 $finish;\nendmodule\n");
    assert_ne!(c, Some(0), "a signal condition must stay loud; got:\n{out}");
}
