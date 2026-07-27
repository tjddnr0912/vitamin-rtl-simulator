//! Real-domain constant folding: `localparam real R = 2.0 + 3.0;` and friends were
//! LOUD (E3009) because `param_real_value` folded only a real LITERAL, or fell back
//! to the INTEGER const domain for a declared-`real` parameter. Real ARITHMETIC
//! reached neither — while vita's own runtime computed it correctly. `localparam
//! real` is a common idiom, so this was a visible hole.
//!
//! Two rules the design rests on, both learned the hard way:
//!
//!   * A real is converted to an integer at the CONTEXT BOUNDARY or nowhere. Doing
//!     it at a const-eval leaf destroys the value before the enclosing operator
//!     picks its domain (`R > 2` with R = 2.4 took the wrong generate branch).
//!   * §11.8.1 — an operation with ANY real operand is evaluated in the real
//!     domain, so the real fold must run BEFORE the integer fallback. Getting that
//!     order wrong was measured mid-slice: once an exactly-integral real parameter
//!     gained an i64 twin, `localparam real HALF = CLK/2;` (CLK = 5.0) became
//!     foldable by the INTEGER domain and answered 2.00 instead of 2.50.
//!
//! Every value here is pinned to LIVE iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_raw(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_rcf_{}_{n}", std::process::id()));
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
        out.status.success(),
    )
}

/// The whole operator set, plus the integer-promotion rule and a real ternary.
#[test]
fn the_real_operator_set_folds() {
    let (out, ok) = run_raw(
        "module t;\n\
           localparam real A = 3/2;          // integer division, stays 1.0\n\
           localparam real B = 3.0/2;        // real division\n\
           localparam real C = 2.0 ** 0.5;\n\
           localparam real D = 2.0 ** -1.0;\n\
           localparam real E = 0.1 + 0.2;\n\
           localparam real F = -4.0;\n\
           localparam real G = 0.0;\n\
           localparam real H = 10 / 4.0;     // int promotes (§11.8.1)\n\
           localparam real I = (2.0 > 1.0) ? 1.5 : 2.5;\n\
           localparam real J = 9.0 - 1.5;\n\
           initial $display(\"V=%0.4f %0.4f %0.4f %0.4f %0.4f %0.4f %0.4f %0.4f %0.4f %0.4f\",\n\
                            A, B, C, D, E, F, G, H, I, J);\n\
         endmodule\n",
    );
    assert!(ok, "expected success; got:\n{out}");
    assert!(
        out.contains("V=1.0000 1.5000 1.4142 0.5000 0.3000 -4.0000 0.0000 2.5000 1.5000 7.5000"),
        "real operator set; got:\n{out}"
    );
}

/// The ordering rule: a real operand anywhere puts the expression in the real
/// domain, so the integer fold must not get there first and truncate.
#[test]
fn a_real_operand_keeps_the_expression_in_the_real_domain() {
    let (out, ok) = run_raw(
        "module t;\n\
           localparam real CLK = 5.0;\n\
           localparam real HALF = CLK/2;      // 2.5, NOT the integer 5/2 = 2\n\
           localparam real ALIAS = HALF;      // real-to-real alias\n\
           localparam real CHAIN = ALIAS * 2; // 5.0\n\
           localparam real INTONLY = 7/2;     // no real operand -> integer 3 -> 3.0\n\
           initial $display(\"D=%0.2f %0.2f %0.2f %0.2f\", HALF, ALIAS, CHAIN, INTONLY);\n\
         endmodule\n",
    );
    assert!(ok, "expected success; got:\n{out}");
    assert!(
        out.contains("D=2.50 2.50 5.00 3.00"),
        "real domain wins, integer-only stays integer; got:\n{out}"
    );
}

/// A real LITERAL initializer registers NO i64 twin, so an integral use of the
/// parameter stays loud whatever its value. A twin was tried — it buys spelling
/// parity, since `localparam real R = 4;` (integer literal) DOES get one through
/// the integer path and can size `logic [R-1:0]` — but a twin also lets the
/// INTEGER const domain answer expressions that mention a real, and
/// `param_real_value` is the only site applying the §11.8.1 ordering rule. Every
/// other const-evaluation context then truncated: `generate if (R/2 > 2)` with
/// R = 5.0 took the ELSE branch and a generate-scope `localparam real X = R/2`
/// bound 2.0. Loud on one spelling beats five silent-wrongs; the asymmetry is
/// recorded in ROADMAP §3.
#[test]
fn a_real_literal_param_gets_no_integer_twin() {
    for v in ["4.0", "2.5", "-4.0", "8.5", "0.0"] {
        let (out, ok) = run_raw(&format!(
            "module t;\n  localparam real R = {v};\n  logic [R-1:0] x;\n\
               initial begin x=0; $display(\"%0d\", $bits(x)); end\nendmodule\n"
        ));
        assert!(!ok, "R = {v} in a width must stay loud; got:\n{out}");
    }
    // The parameter is still perfectly usable as a REAL — only the integral
    // capability is withheld.
    let (out, ok) = run_raw(
        "module t;\n  localparam real R = 4.0, S = R * 2.0;\n\
           initial $display(\"U=%0.2f %0.2f\", R, S);\nendmodule\n",
    );
    assert!(ok, "expected success; got:\n{out}");
    assert!(
        out.contains("U=4.00 8.00"),
        "real use is unaffected; got:\n{out}"
    );
}

/// Correct-or-loud at the edges: a real division by zero has no usable parameter
/// value, and a non-finite result must never bind silently.
#[test]
fn non_finite_real_results_stay_loud() {
    for init in ["1.0 / 0.0", "0.0 / 0.0", "-1.0 / 0.0"] {
        let (out, ok) = run_raw(&format!(
            "module t;\n  localparam real Z = {init};\n\
               initial $display(\"%0.2f\", Z);\nendmodule\n"
        ));
        assert!(!ok, "`{init}` must stay loud; got:\n{out}");
    }
}
