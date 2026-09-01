//! A module-scope enum label was invisible to everything declared after it.
//!
//! Labels bound at phase (3c), AFTER the whole body-parameter walk, so a `localparam`
//! written under a `typedef enum` could not name one: `typedef enum logic [31:0] {EA =
//! 32'hAB34} e_t; localparam logic [31:0] Q = EA;` was `E3009 undefined name 'EA'`
//! where both oracles print 43828. The PACKAGE spelling of the identical text folded,
//! because a package's labels are bound before any module body binds — one rule, two
//! answers, decided by which file the enum lives in. ROADMAP §2 row 11.
//!
//! ⭐ The row recorded only the label→localparam direction; it is broader. A
//! param→label→param CHAIN (`localparam A = 3; typedef enum {L = A + 1} e_t;
//! localparam B = L + 1;`) is `A=3 L=4 B=5` in both oracles and was loud on `B` here,
//! and a label is equally invisible to a later WIDTH, a later `generate if`, and a
//! later typedef's label.
//!
//! The fix binds labels in the SAME decl-order walk the body parameters use, as a
//! quiet pre-pass.
//!
//! ⚠️ …and keeps the after-the-walk pass, which is why `a_forward_reference_is_left_
//! alone` below still runs. A label whose value names a LATER parameter is an ORACLE
//! SPLIT — iverilog refuses to bind it, verilator folds it — so the pre-pass SKIPS
//! what it cannot fold rather than reporting, and the second pass decides that shape
//! exactly as before. Running twice is monotone: the first pass only adds bindings
//! that were going to be made anyway, and `restore_params` unwinds in reverse.
//!
//! Values pinned to iverilog 13.0 and verilator 5.050 unless noted.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_eldo_{}_{n}", std::process::id()));
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

/// The row's own cell, and the package twin it disagreed with, side by side.
#[test]
fn a_label_is_visible_to_a_localparam_declared_under_it() {
    let (o, ok) = run("module top;\n  \
           typedef enum logic [31:0] { EA = 32'hAB34 } e_t;\n  \
           localparam logic [31:0] Q = EA;\n  \
           initial begin $display(\"OUT=%0d\", Q); $finish; end\n\
         endmodule\n");
    assert!(ok, "was E3009 undefined name `EA`:\n{o}");
    assert!(o.contains("OUT=43828"), "{o}");

    // The package spelling always folded; both must now give one answer.
    let (o, ok) = run("package pk;\n  \
           typedef enum logic [31:0] { EA = 32'hAB34 } e_t;\n\
         endpackage\n\
         module top;\n  localparam logic [31:0] Q = pk::EA;\n  \
           initial begin $display(\"OUT=%0d\", Q); $finish; end\n\
         endmodule\n");
    assert!(ok, "{o}");
    assert!(o.contains("OUT=43828"), "{o}");
}

/// ⭐ The chain the row did not mention: a label that READS an earlier parameter and
/// is read by a later one. It exercises both directions of the walk in one design, so
/// a fix that binds labels first (rather than in order) fails the `L` half.
#[test]
fn a_param_label_param_chain_folds_in_declaration_order() {
    let (o, ok) = run("module top;\n  \
           localparam int A = 3;\n  \
           typedef enum logic [7:0] { L = A + 1 } e_t;\n  \
           localparam int B = L + 1;\n  \
           initial begin $display(\"OUT=A=%0d L=%0d B=%0d\", A, L, B); $finish; end\n\
         endmodule\n");
    assert!(ok, "{o}");
    assert!(o.contains("OUT=A=3 L=4 B=5"), "{o}");
}

/// Every other consumer that sits after the typedef: a declaration WIDTH, a
/// `generate if`, a second typedef's label, and the implicit-value counter.
#[test]
fn a_label_reaches_every_later_consumer() {
    for (what, src) in [
        (
            "declaration width",
            "module top;\n  typedef enum logic [7:0] { W8 = 8'd8 } e_t;\n  \
               logic [W8-1:0] v;\n  \
               initial begin v = 8'hA5; $display(\"OUT=%0d %h\", $bits(v), v); $finish; end\n\
             endmodule\n",
        ),
        (
            "generate condition",
            "module top;\n  typedef enum logic [7:0] { G = 8'd8 } e_t;\n  \
               generate if (G == 8) begin : y initial $display(\"OUT=8 a5\"); end\n  \
               else begin : n initial $display(\"OUT=BAD\"); end endgenerate\n  \
               initial begin #1 $finish; end\n\
             endmodule\n",
        ),
        (
            "a later typedef's label",
            "module top;\n  typedef enum logic [7:0] { A = 8'd3 } a_t;\n  \
               typedef enum logic [7:0] { B = A + 8'd5 } b_t;\n  \
               initial begin $display(\"OUT=%0d a5\", B); $finish; end\n\
             endmodule\n",
        ),
        (
            "implicit-value counter",
            "module top;\n  typedef enum logic [7:0] { P, Q, R } e_t;\n  \
               localparam int S = R + 6;\n  \
               initial begin $display(\"OUT=%0d a5\", S); $finish; end\n\
             endmodule\n",
        ),
    ] {
        let (o, ok) = run(src);
        assert!(ok, "{what}:\n{o}");
        assert!(o.contains("OUT=8"), "{what}:\n{o}");
    }
}

/// ⚠️ The pre-pass must not answer a question the second pass owns. A label naming a
/// LATER parameter is an ORACLE SPLIT — iverilog: *"Unable to bind wire/reg/memory
/// `LP`"*, verilator: 5 — so vita's answer is UNCHANGED by this slice, and it must
/// stay a value rather than becoming a diagnostic from the pre-pass.
#[test]
fn a_forward_reference_is_left_alone() {
    let (o, ok) = run("module top;\n  \
           typedef enum logic [31:0] { X = LP } e_t;\n  \
           localparam logic [31:0] LP = 32'd5;\n  \
           initial begin $display(\"OUT=%0d\", X); $finish; end\n\
         endmodule\n");
    assert!(
        ok,
        "the pre-pass must SKIP what it cannot fold, not report:\n{o}"
    );
    assert!(o.contains("OUT=5"), "{o}");
}

/// …and a label that folds nowhere is still loud, ONCE. Two passes over the same
/// typedef must not report the same label twice, and §6.19's range check — which the
/// quiet pass also skips — must still fire exactly once.
#[test]
fn an_unfoldable_or_out_of_range_label_reports_exactly_once() {
    for src in [
        "module top;\n  typedef enum logic [7:0] { X = NOPE } e_t;\n  \
           initial begin $display(\"OUT=%0d\", X); $finish; end\nendmodule\n",
        "module top;\n  typedef enum logic [3:0] { BIG = 8'd200 } e_t;\n  \
           initial begin $display(\"OUT=%0d\", BIG); $finish; end\nendmodule\n",
    ] {
        let (o, ok) = run(src);
        assert!(!ok, "must be loud:\n{o}");
        assert_eq!(
            o.matches("error[VITA-").count(),
            1,
            "reported by both passes:\n{o}"
        );
    }
}

/// ⚠️⚠️ THE SOUNDNESS ARGUMENT. Two bindings are safe only if they AGREE — three
/// review findings, two lenses, one shape: a consumer that folds BETWEEN the passes
/// keeps pass 1's answer while everything after keeps pass 2's, so one label reads two
/// values in one run at exit 0. All three cells were `E3009` before the slice and must
/// stay loud.
///
/// 1. `{A = LP, B}` — skipping an unfoldable `A` skipped the `next` counter, so `B`
///    bound 0 and `localparam Q = B;` folded 0 while `B` itself printed 6.
/// 2. `enum logic [W-1:0] {A = -8'sd2}` with `W` below — `enum_base_width` was unknown
///    in pass 1, the mask never ran, and `Q` folded the raw `-2` as `fffe` where pass 2
///    makes `A` 254.
/// 3. `import pk::*` beside a body `localparam` of the same name — the value resolved
///    to the IMPORT in pass 1 and to the local declaration in pass 2. A gate cannot see
///    this one (the fold SUCCEEDS, with a different answer), so pass 2 verifies pass 1.
#[test]
fn a_label_that_would_bind_differently_in_the_two_passes_stays_loud() {
    for (what, src) in [
        (
            "the implicit-value counter after a skipped label",
            "module top;\n  typedef enum { A = LP, B } e_t;\n  \
               localparam LP = 5;\n  localparam Q = B;\n  \
               initial begin $display(\"OUT=%0d %0d\", B, Q); $finish; end\n\
             endmodule\n",
        ),
        (
            "an enum base whose width is not yet a fact",
            "module top;\n  typedef enum logic [W-1:0] { A = -8'sd2 } e_t;\n  \
               localparam W = 8;\n  localparam logic [15:0] Q = A;\n  \
               initial begin $display(\"OUT=%0d %h\", A, Q); $finish; end\n\
             endmodule\n",
        ),
        (
            "a name that resolves to a different declaration in each pass",
            "package pk; parameter int K = 7; endpackage\n\
             module top;\n  import pk::*;\n  \
               typedef enum logic [31:0] { A = K } e_t;\n  \
               localparam int K = 100;\n  localparam logic [31:0] Q = A;\n  \
               initial begin $display(\"OUT=%0d %0d\", A, Q); $finish; end\n\
             endmodule\n",
        ),
    ] {
        let (o, ok) = run(src);
        assert!(!ok, "{what}: one label would read two values:\n{o}");
    }
}

/// …and the CONTROLS: move the declaration the pre-pass could not see ABOVE the
/// typedef and every one of them folds. Without these the test above would pass for a
/// pre-pass that never binds anything.
#[test]
fn the_same_three_shapes_fold_once_the_declaration_precedes_the_typedef() {
    for (what, src, want) in [
        (
            "counter",
            "module top;\n  localparam LP = 5;\n  \
               typedef enum { A = LP, B } e_t;\n  localparam Q = B;\n  \
               initial begin $display(\"OUT=%0d %0d\", B, Q); $finish; end\n\
             endmodule\n",
            "OUT=6 6",
        ),
        (
            "enum base width",
            "module top;\n  localparam W = 8;\n  \
               typedef enum logic [W-1:0] { A = -8'sd2 } e_t;\n  \
               localparam logic [15:0] Q = A;\n  \
               initial begin $display(\"OUT=%0d %h\", A, Q); $finish; end\n\
             endmodule\n",
            "OUT=254 00fe",
        ),
    ] {
        let (o, ok) = run(src);
        assert!(ok, "{what}:\n{o}");
        assert!(o.contains(want), "{what}: want {want}\n{o}");
    }
}

/// A label SHADOWS a wildcard-imported constant of the same name for everything
/// declared after it — the binding order change is observable here, and verilator
/// agrees (iverilog rejects the shadow outright, so it is not an oracle for this
/// cell).
#[test]
fn a_label_shadows_an_imported_constant_for_later_declarations() {
    let (o, ok) = run("package pk; parameter int EA = 99; endpackage\n\
         module top;\n  import pk::*;\n  \
           typedef enum logic [31:0] { EA = 32'd7 } e_t;\n  \
           localparam int Q = EA;\n  \
           initial begin $display(\"OUT=%0d\", Q); $finish; end\n\
         endmodule\n");
    assert!(ok, "{o}");
    assert!(o.contains("OUT=7"), "{o}");
}
