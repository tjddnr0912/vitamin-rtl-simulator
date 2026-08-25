//! Round-33's diagnostic axis: **the message named the DECLARED thing and not the
//! REJECTED one, and the caret stood where the cause was not.**
//!
//! Six of that report's eleven items were one defect seen from six sites. The caret
//! came from the elaborator's ambient span, which during module elaboration is the
//! MODULE HEADER — so every constant-fold rejection in one module printed the same
//! `file:line:col`, at a line holding no parameter, and the only way to find the
//! culprit was to comment declarations out one at a time. The text stopped at "value
//! is not a foldable constant expression", so three declarations rejected for three
//! unrelated reasons were indistinguishable; none of the words `real`, `string` or
//! `replication` appeared in any of them, and when the cause was an UNDEFINED NAME the
//! name was in hand and thrown away.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r33d_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// The caret goes on the INITIALIZER, so two rejections in one module are two
/// different lines — and each names its own cause.
#[test]
fn a_constant_fold_rejection_points_at_the_declaration_and_says_why() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  localparam int P = 1;\n  \
         localparam int Q = P + NOPE;\n  localparam string S = \"abc\";\n  \
         localparam int Q2 = S.len2();\n  \
         initial begin #1 $display(\"%0d %0d\", Q, Q2); $finish; end\nendmodule\n",
    );
    assert_ne!(code, Some(0), "{out}");
    // Two DIFFERENT lines — the defect was that both printed `module tb;`.
    assert!(
        out.contains("t.sv:4:"),
        "the first must anchor at line 4:\n{out}"
    );
    assert!(out.contains("t.sv:6:"), "the second at line 6:\n{out}");
    // …and the undefined name is named, having been in hand all along.
    assert!(out.contains("`NOPE`"), "the name must appear:\n{out}");
}

/// A `real` in an integer constant context is called a real.
#[test]
fn a_real_cause_is_named_as_a_real() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule t;\n  localparam real R = 5.0;\n  \
         localparam M = R/2.0;\n  initial $display(\"%0d\", M);\nendmodule\n",
    );
    assert_ne!(code, Some(0));
    assert!(
        out.contains("is a real, which has no integral constant value"),
        "{out}"
    );
}

/// ⭐ A missing package file is ONE error that names the package. It used to be seven
/// `E2002`s — the first at the `::`, the rest at perfectly correct declarations
/// further down that only failed because the tf-port list never closed — with the
/// package's name in none of them.
#[test]
fn a_missing_package_is_one_error_that_names_it() {
    let (out, code) = run("`timescale 1ns/1ns\nmodule tb;\n  \
         function automatic int f(input q::mode_e m);\n    int unsigned k;\n    \
         logic [1023:0] buf2;\n    f = 1;\n  endfunction\n  \
         initial begin #1 $display(\"R=%0d\", 1); $finish; end\nendmodule\n");
    assert_ne!(code, Some(0));
    assert_eq!(
        out.matches("error[VITA-").count(),
        1,
        "exactly one error, not a cascade:\n{out}"
    );
    assert!(
        out.contains("`q::mode_e`"),
        "it must name the package:\n{out}"
    );
}

/// An enum method rejection describes enum methods — the old text described
/// hierarchical function calls, a different feature entirely, and carried no location.
#[test]
fn an_enum_method_rejection_describes_enum_methods() {
    let (out, code) = run("`timescale 1ns/1ns\nmodule tb;\n  parameter K = 9;\n  \
         typedef enum logic [7:0] { A = K, B } e_t;\n  e_t x;\n  \
         initial begin #1 x = A; $display(\"R=%0d %s\", x, x.name()); $finish; end\nendmodule\n");
    assert_ne!(code, Some(0));
    assert!(out.contains("enum method"), "{out}");
    assert!(out.contains("t.sv:6:"), "it must carry a location:\n{out}");
    assert!(
        !out.contains("hierarchical function call"),
        "the misleading wording came back:\n{out}"
    );
}

/// A non-constant bound is reported ONCE, with the caret on the bound. The range is
/// folded by more than one pass, so it used to print twice — both times on the
/// `logic` keyword rather than the `$clog2`.
#[test]
fn a_non_constant_bound_is_reported_once_at_the_bound() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  int n = 8;\n  logic [$clog2(n)-1:0] v;\n  \
         initial begin #1 $display(\"R=%0d\", $bits(v)); $finish; end\nendmodule\n",
    );
    assert_ne!(code, Some(0));
    assert_eq!(
        out.matches("error[VITA-").count(),
        1,
        "exactly one, not one per pass:\n{out}"
    );
    assert!(
        out.contains("t.sv:4:10"),
        "the caret is on `$clog2`:\n{out}"
    );
}

/// ⭐ An unconnected child INPUT warns. The asymmetry ran OPPOSITE to the consequence:
/// a dangling output DISCARDS a value, a dangling input MANUFACTURES one (`z` at time
/// 0) and propagates it — and only the first was warned about.
#[test]
fn an_unconnected_child_input_warns() {
    let (out, code) = run("`timescale 1ns/1ns\n\
         module leaf(input logic a, input logic b, output logic y); assign y = a & b; endmodule\n\
         module tb; logic y;\n  leaf u(.a(1'b1), .y(y));\n  \
         initial begin #1 $display(\"y=%b\", y); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "a warning, not an error:\n{out}");
    assert!(out.contains("input port `b` left unconnected"), "{out}");
    // A ROOT's own ports stay silent — they are unconnected by definition.
    let (out2, code2) = run(
        "`timescale 1ns/1ns\nmodule tb(input logic a, output logic y);\n  assign y = a;\n  \
         initial begin #1 $finish; end\nendmodule\n",
    );
    assert_eq!(code2, Some(0));
    assert!(
        !out2.contains("left unconnected"),
        "root ports stay quiet:\n{out2}"
    );
}

/// ⭐ A variable driven by BOTH a declaration initializer and `always_comb` warns.
/// vita ran it at exit 0 with nothing to say while xcelium stops elaboration
/// (`*E,MULAXX`) and verilator errors (MULTIDRIVEN) — green in the development loop,
/// dead at sign-off. The simulated VALUE is unchanged.
#[test]
fn an_always_comb_variable_with_an_initializer_warns() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  logic [7:0] cnt = 0;\n  logic rdy = 1'b1;\n  \
         logic ok;\n  always_comb rdy = (cnt < 8'd128);\n  always_comb ok = 1'b1;\n  \
         initial begin #1 $display(\"R=%0b%0b\", rdy, ok); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "warn, never change the value:\n{out}");
    assert!(out.contains("R=11"), "{out}");
    assert_eq!(
        out.matches("declaration initializer AND").count(),
        1,
        "only `rdy` has both drivers — `cnt` has only an initializer and `ok` only \
         the always_comb:\n{out}"
    );
    assert!(out.contains("`rdy`"), "{out}");
}
