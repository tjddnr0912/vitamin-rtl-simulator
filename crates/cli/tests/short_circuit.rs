//! IEEE 1800 §11.4.7: `&&` and `||` evaluate their RIGHT operand only when the
//! LEFT one did not already determine the result. vita evaluated both, always.
//!
//! The generic evaluator's arm read
//!
//! ```text
//!     LogAnd | LogOr => {
//!         let l = self.eval(lhs);
//!         let r = self.eval(rhs);        // <- unconditional
//!         ...
//! ```
//!
//! which is the ordinary eager tree walk. The `Ternary` arm four lines above it
//! had already solved the same problem the right way — it reaches an arm through
//! a closure, so an arm runs only where the closure is called — and that is the
//! shape this now copies.
//!
//! ## What was actually wrong, measured against BOTH oracles
//!
//! The operator's own RESULT was correct in all 47 census cells, including every
//! x/z row. The damage was entirely in state the skipped operand should never
//! have touched:
//!
//! ```text
//!   int cnt; 0 && (c.bump()==1);   then c.cnt   vita 1  ivl 0  verilator 0
//!            1 || (c.bump()==1);   then c.cnt   vita 1  ivl 0  verilator 0
//!            0 && ($random != 0);  then $random vita -1064739199  ivl 303379748
//!            0 && (mem4[9] != 0);               vita E4002, exit 1   both: exit 0
//!            0 && f(1)   where f $displays      vita prints, ivl+verilator do not
//! ```
//!
//! The `$random` row is the sharpest: vita's CONTROL draw is byte-identical to
//! iverilog's (the two share an LCG stream), so vita's test draw is provably the
//! SECOND draw of that same stream while iverilog's is still the first. A skipped
//! operand had consumed a random number.
//!
//! ## The gate predicate is a TRUTH VALUE, not a bit
//!
//! The deciding question is `truthiness(lhs)`, which for a vector is the
//! reduction-or: `||` is decided by ANY set bit and `&&` by ALL bits zero. So
//!
//! ```text
//!   4'b01x0 || f()    SKIPS   (a set bit determines it — both oracles skip)
//!   4'b00x0 || f()    RUNS    (Tri::Unknown — both oracles run it)
//! ```
//!
//! and an x/z left operand never licenses a skip on its own. Writing the
//! predicate over planes instead of `Tri` gets exactly these two rows wrong in
//! opposite directions, which is why they are both pinned below.
//!
//! ## The two compiled lanes decline instead
//!
//! `wprog` (tier-3) and `native_eval` (the VM fast path) have no control flow and
//! evaluate both operands always. That is the same VALUE — the truth table is
//! total — but not the same DIAGNOSTICS, because an admitted `LoadIdx` reports an
//! out-of-range element read where it happens. Both now decline an `&&`/`||`
//! whose RIGHT operand compiled to such an op, which is character-for-character
//! the guard their `Ternary` arms already carried, asked of the compiled OPS
//! rather than of the expression shape. Only the right operand is guarded: the
//! left runs on every path, so a report from it is not a divergence.
//!
//! ⚠️ The whole 6,240-test suite passed both before and after this change. Nothing
//! in it evaluated a side-effecting `&&` operand, which is why these cells exist.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run a design and return its stdout+stderr plus the exit code.
fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sc_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(d.join("t.sv"), src).expect("write design");
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

/// How many times the probe function announced itself.
fn calls(out: &str) -> usize {
    out.matches("EVAL").count()
}

/// A function that says when it runs. `integer`, so it stays inside the
/// frame-call subset (an outer-net write would be a separate E3009).
const PROBE: &str = "  function integer f(input integer x); \
                     begin $display(\"EVAL%0d\", x); f = x; end endfunction\n";

fn design(body: &str) -> String {
    format!("`timescale 1ns/1ns\nmodule tb;\n  integer r;\n{PROBE}  initial begin\n{body}    $finish;\n  end\nendmodule\n")
}

// ── the two deciding rows ──────────────────────────────────────────────────

/// `0 && f()` — the left operand decides, so `f` must not run. Value stays 0.
#[test]
fn a_false_left_operand_skips_the_right_one_of_and() {
    let (out, code) = run(&design(
        "    r = (1'b0 && f(1));\n    $display(\"R=%0d\", r);\n",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(calls(&out), 0, "f must not run:\n{out}");
    assert!(out.contains("R=0"), "value unchanged:\n{out}");
}

/// `1 || f()` — the mirror image.
#[test]
fn a_true_left_operand_skips_the_right_one_of_or() {
    let (out, code) = run(&design(
        "    r = (1'b1 || f(1));\n    $display(\"R=%0d\", r);\n",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(calls(&out), 0, "f must not run:\n{out}");
    assert!(out.contains("R=1"), "value unchanged:\n{out}");
}

/// The two rows that must STILL evaluate — a fix that skips these is wrong in
/// the other direction, and the value it would produce is also wrong.
#[test]
fn a_non_deciding_left_operand_still_evaluates_the_right_one() {
    let (out, code) = run(&design(
        "    r = (1'b1 && f(1));\n    $display(\"A=%0d\", r);\n\
         \x20   r = (1'b0 || f(1));\n    $display(\"B=%0d\", r);\n",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(calls(&out), 2, "both must run:\n{out}");
    assert!(out.contains("A=1") && out.contains("B=1"), "{out}");
}

// ── the truth-value predicate, not a bit ───────────────────────────────────

/// ⭐ A vector with x bits AND a set bit. The set bit DECIDES `||`, so the right
/// operand is skipped even though the left is not a clean 1. Both oracles skip.
#[test]
fn a_set_bit_decides_or_even_with_x_bits_present() {
    let (out, code) = run(&format!(
        "`timescale 1ns/1ns\nmodule tb;\n  integer r;\n  reg [3:0] v;\n{PROBE}  \
         initial begin\n    v = 4'b01x0;\n    r = (v || f(1));\n    \
         $display(\"R=%0d\", r);\n    $finish;\n  end\nendmodule\n"
    ));
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(calls(&out), 0, "a set bit determines `||`:\n{out}");
    assert!(out.contains("R=1"), "{out}");
}

/// ⚠️ The neighbour that must NOT be skipped: only 0 and x bits, so the truth
/// value is Unknown and the right operand decides. iverilog runs it.
#[test]
fn an_unknown_truth_value_does_not_decide_or() {
    let (out, code) = run(&format!(
        "`timescale 1ns/1ns\nmodule tb;\n  integer r;\n  reg [3:0] v;\n{PROBE}  \
         initial begin\n    v = 4'b00x0;\n    r = (v || f(1));\n    \
         $display(\"R=%0d\", r);\n    $finish;\n  end\nendmodule\n"
    ));
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(calls(&out), 1, "Tri::Unknown must evaluate the rhs:\n{out}");
    assert!(out.contains("R=1"), "{out}");
}

/// The `&&` twin of the pair above: all-zero decides, x-bearing does not.
#[test]
fn an_all_zero_vector_decides_and_but_an_x_bearing_one_does_not() {
    let (out, code) = run(&format!(
        "`timescale 1ns/1ns\nmodule tb;\n  integer r;\n  reg [3:0] v;\n{PROBE}  \
         initial begin\n    v = 4'b0000;\n    r = (v && f(1));\n    \
         v = 4'b00x0;\n    r = (v && f(2));\n    $finish;\n  end\nendmodule\n"
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(!out.contains("EVAL1"), "all-zero decides `&&`:\n{out}");
    assert!(out.contains("EVAL2"), "an x truth value does not:\n{out}");
}

/// An x left operand never licenses a skip on `&&` either — `x && 0` is 0, which
/// the right operand is needed to discover.
#[test]
fn an_x_left_operand_is_not_a_licence_to_skip() {
    let (out, code) = run(&format!(
        "`timescale 1ns/1ns\nmodule tb;\n  integer r;\n  reg a;\n{PROBE}  \
         initial begin\n    a = 1'bx;\n    r = (a && f(0));\n    \
         $display(\"R=%0d\", r);\n    $finish;\n  end\nendmodule\n"
    ));
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(calls(&out), 1, "{out}");
    assert!(
        out.contains("R=0"),
        "x && 0 is 0, and only the rhs says so:\n{out}"
    );
}

// ── every syntactic position, not just an assignment rhs ───────────────────

/// The census ran twelve positions; these are the ones that reach a different
/// lowering (a branch condition, a loop condition, a case scrutinee, a task
/// argument, a ternary condition and a nested chain each take their own path).
#[test]
fn the_skip_holds_in_every_position_that_has_its_own_lowering() {
    for (label, body) in [
        ("blocking", "    r = (1'b0 && f(1));\n"),
        ("nonblocking", "    r <= (1'b0 && f(1));\n    #1;\n"),
        ("if-cond", "    if (1'b0 && f(1)) r = 1;\n"),
        ("while-cond", "    while (1'b0 && f(1)) r = 1;\n"),
        (
            "case-scrutinee",
            "    case (1'b0 && f(1)) 1'b1: r = 1; default: r = 0; endcase\n",
        ),
        ("ternary-cond", "    r = (1'b0 && f(1)) ? 1 : 2;\n"),
        ("ternary-arm", "    r = 1'b1 ? 7 : (1'b0 && f(1));\n"),
        ("systask-arg", "    $display(\"X=%0d\", (1'b0 && f(1)));\n"),
        ("chain", "    r = (1'b0 && 1'b1 && f(1));\n"),
        ("mixed", "    r = (1'b0 && (1'b1 || f(1)));\n"),
    ] {
        let (out, code) = run(&design(body));
        assert_eq!(code, Some(0), "{label}:\n{out}");
        assert_eq!(calls(&out), 0, "{label} must skip the rhs:\n{out}");
    }
}

/// A continuous assign settles rather than executing, so it gets its own row.
///
/// ⚠️ The left operand is a `wire` driven by a constant, not a `reg` assigned in
/// an `initial`. A `reg` is x until its first write, and an x left operand does
/// NOT decide — so the settle at t=0 legitimately evaluates the right operand and
/// the cell would be measuring the x row instead of this one. That mistake made
/// this test fail first time round, on correct behaviour.
///
/// ⚠️⚠️ The oracles SPLIT on the count here and vita follows verilator: verilator
/// and vita call `f` zero times, iverilog calls it once. iverilog builds a
/// dataflow node for the expression and evaluates it once at initialisation
/// regardless — the same caching the census saw when it counted 1 call against
/// vita's 13 on a toggling driver. That is a difference about WHEN a settling
/// expression runs, not about whether `&&` short-circuits, so verilator is the
/// oracle for this row and zero is the answer.
#[test]
fn a_continuous_assign_also_skips_a_decided_right_operand() {
    let (out, code) = run(&format!(
        "`timescale 1ns/1ns\nmodule tb;\n  wire a = 1'b0;\n  wire w;\n{PROBE}  \
         assign w = a && f(1);\n  \
         initial begin\n    #10;\n    $display(\"W=%0b\", w);\n    \
         $finish;\n  end\nendmodule\n"
    ));
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(calls(&out), 0, "a is definitely 0 throughout:\n{out}");
    assert!(out.contains("W=0"), "{out}");
}

/// The neighbour that proves the row above is about the LEFT operand's truth
/// value and not about continuous assigns being lazy in general: an x-valued
/// driver settles through the right operand, exactly as the procedural x row
/// does. `reg a;` is x until something writes it.
#[test]
fn a_continuous_assign_with_an_x_driver_still_evaluates_the_right_operand() {
    let (out, code) = run(&format!(
        "`timescale 1ns/1ns\nmodule tb;\n  reg a;\n  wire w;\n{PROBE}  \
         assign w = a && f(1);\n  \
         initial begin\n    #10;\n    $display(\"W=%0b\", w);\n    \
         $finish;\n  end\nendmodule\n"
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(calls(&out) > 0, "an x driver does not decide `&&`:\n{out}");
}

// ── the three cells where the damage was a WRONG VALUE ─────────────────────

/// ⭐ The `$random` stream. A skipped operand used to consume a draw, so the NEXT
/// `$random` returned the second number of the sequence. The value asserted here
/// is iverilog's, measured — vita and iverilog share the LCG, so this is a
/// cross-tool value pin and not a self-consistency check.
#[test]
fn a_skipped_operand_does_not_consume_a_random_draw() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  integer r;\n  initial begin\n    \
         r = 0;\n    if (1'b0 && ($random != 0)) r = 1;\n    \
         $display(\"R=%0d\", $random);\n    $finish;\n  end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("R=303379748"),
        "the first draw of the stream, as iverilog reports it; \
         -1064739199 is the SECOND draw and means the skipped operand drew one:\n{out}"
    );
}

/// A class method mutating object state from a skipped operand.
#[test]
fn a_skipped_operand_does_not_mutate_an_object() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  class C;\n    int cnt = 0;\n    \
         function int bump(); cnt = cnt + 1; return cnt; endfunction\n  endclass\n  \
         C c;\n  int r;\n  initial begin\n    c = new();\n    \
         if (1'b0 && (c.bump() == 1)) r = 1;\n    \
         if (1'b1 || (c.bump() == 1)) r = 2;\n    \
         $display(\"CNT=%0d\", c.cnt);\n    $finish;\n  end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("CNT=0"), "neither call may run:\n{out}");
}

/// ⚠️ A DIAGNOSTIC APPEARING is a divergence exactly as much as one going
/// missing. An out-of-range element read guarded by `&&` used to report `E4002`
/// and exit 1 where both oracles exit 0 — so the guard idiom this exists to
/// support did not work. This is also the cell the two compiled lanes decline
/// for, so it exercises the `wprog` / `native_eval` guards rather than only the
/// generic arm.
#[test]
fn a_guarded_out_of_range_read_neither_reports_nor_fails() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  reg [7:0] mem [0:3];\n  integer i;\n  \
         reg r;\n  initial begin\n    i = 9;\n    \
         r = (i < 4) && (mem[i] != 8'h00);\n    $display(\"R=%0b\", r);\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "guarded read must not fail the run:\n{out}");
    assert!(
        !out.contains("E4002"),
        "no report from a skipped read:\n{out}"
    );
    assert!(out.contains("R=0"), "{out}");
}

/// The `||` spelling of the same guard, where the skip is on a TRUE left.
#[test]
fn an_or_guarded_out_of_range_read_is_also_silent() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  reg [7:0] mem [0:3];\n  integer i;\n  \
         reg r;\n  initial begin\n    i = 9;\n    \
         r = (i >= 4) || (mem[i] != 8'h00);\n    $display(\"R=%0b\", r);\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(!out.contains("E4002"), "{out}");
    assert!(out.contains("R=1"), "{out}");
}

/// ⚠️ The guarded read must still report when it is NOT guarded away — the two
/// lanes decline the whole program, so this checks the decline did not also
/// swallow the diagnostic the generic path owes.
#[test]
fn an_unguarded_out_of_range_read_still_reports() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  reg [7:0] mem [0:3];\n  integer i;\n  \
         reg r;\n  initial begin\n    i = 9;\n    \
         r = (i > 0) && (mem[i] != 8'h00);\n    $display(\"R=%0b\", r);\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert!(
        out.contains("E4002"),
        "the read HAPPENS here, so it must still be reported:\n{out}"
    );
    assert_ne!(code, Some(0), "{out}");
}

/// A skipped operand is skipped in a FRAME body too — the `&self` frame executor
/// is a separate walk from the module-scope one, and a fix landing only in the
/// module lane would leave every subroutine eager.
#[test]
fn a_frame_body_also_short_circuits() {
    let (out, code) = run(&format!(
        "`timescale 1ns/1ns\nmodule tb;\n  integer r;\n{PROBE}  \
         function integer g(input integer x);\n    begin\n      \
         g = (1'b0 && f(x)) ? 1 : 0;\n      for (int k = 0; k < 1; k++) g = g;\n    \
         end\n  endfunction\n  \
         initial begin\n    r = g(5);\n    $display(\"R=%0d\", r);\n    \
         $finish;\n  end\nendmodule\n"
    ));
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(calls(&out), 0, "{out}");
    assert!(out.contains("R=0"), "{out}");
}
