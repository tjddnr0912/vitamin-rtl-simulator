//! IEEE §12.5: the case expression is evaluated ONCE. vita evaluated it once per
//! ARM TESTED — and, when there was no arm to test, not at all.
//!
//! `lower_case` builds the cascade from one shared `scrut_id`, which is right in
//! the arena and wrong in the evaluator: a shared node is walked as a TREE, so
//! the scrutinee ran again for every `CaseEq`. Measured against BOTH oracles,
//! vita was wrong in two directions at once:
//!
//! ```text
//!   case (f(n)) 0: 1: 2: 3: default:    n=3    vita E×4   ivl E×1   verilator E×1
//!   casez / casex, one arm before a hit        vita E×2   ivl E×1   verilator E×1
//!   0,1,2: / 3,4,5:   (multi-label)            vita E×6   ivl E×1   verilator E×1
//!   case (f(n)) default:  (no Match arm)       vita E×0   ivl E×1   verilator E×1
//! ```
//!
//! ⭐ The last row is why this is not a repetition count. With no Match arm there
//! is no comparison, so the scrutinee was never evaluated AT ALL — a `$display`
//! in it did not print, and the `E4002` an out-of-range read owes did not fire.
//! One fix closes both directions: capture the scrutinee into a temp before the
//! cascade, and compare the temp.
//!
//! ## What the capture has to carry
//!
//! The cascade sizes every `CaseEq(scrutinee, label)` pair from the scrutinee, so
//! the temp is not a neutral container:
//!
//! * **Signedness** — an unsigned capture of a signed scrutinee makes every pair
//!   unsigned, which silently takes a different arm (`case (s) -1:` with signed
//!   `s = 4'hF`). `fresh_case_tmp` takes it as a parameter for that reason.
//! * **4-state** — `casez`/`casex` match on the scrutinee's own x/z bits, so the
//!   capture net is `Reg`; a 2-state one would coerce them to 0 first.
//! * **Width** — taken after §12.5's common-maximum pass, so a fill-bearing
//!   selector is captured at the width the labels forced it to.
//!
//! ## Scope
//!
//! ⚠️ MODULE-SCOPE bodies only. A frame body cannot write a module net —
//! `frame_write_lvalue` is `&self` and reaches only the activation window — so
//! the capture there has to be a frame LOCAL, which needs the reservation pass
//! `repeat` already has (`frame_repeat_cnt`). Every cell that measured a
//! difference is module-scope. The frame half is the follow-on, and it carries a
//! large perf win with it: `run_frame_call` on `bench/keccak`'s `keccak_f.sv` is
//! 63.8% of the run and 79.7% of THAT is branch conditions, which is two `case`
//! statements of 24 and 25 arms re-evaluating `x + 5*y` up to 25 times a call.
//!
//! ⚠️ Real and string scrutinees are skipped too: the capture net would need
//! `NetKind::Real` / `String`, and neither shows a measured difference (both
//! oracles agree with vita on `case (r)`, and iverilog rejects `case (s)`).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run a design and return its stdout+stderr with the timescale note dropped.
fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cso_{}_{n}", std::process::id()));
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
fn evals(out: &str) -> usize {
    out.matches("EVAL").count()
}

/// A scrutinee that says when it runs, wrapped around `n`.
const PROBE: &str = "  function integer f(input integer x); \
                     begin $display(\"EVAL%0d\", x); f = x; end endfunction\n";

/// Four arms, the fourth matches: the scrutinee ran four times.
#[test]
fn a_plain_case_evaluates_its_scrutinee_once() {
    let (out, code) = run(&format!(
        "`timescale 1ns/1ns\nmodule tb;\n  integer n;\n{PROBE}  \
         initial begin n = 3;\n    case (f(n))\n      0: $display(\"a0\");\n      \
         1: $display(\"a1\");\n      2: $display(\"a2\");\n      3: $display(\"a3\");\n      \
         default: $display(\"ad\");\n    endcase\n    $finish;\n  end\nendmodule\n"
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("a3"), "still takes the right arm:\n{out}");
    assert_eq!(evals(&out), 1, "once, not once per arm tested:\n{out}");
}

/// ⭐ NO Match arm at all. This is the direction a "count the repeats" fix would
/// have missed: there is nothing to compare against, so the scrutinee was never
/// evaluated and its effects never happened.
#[test]
fn a_default_only_case_still_evaluates_its_scrutinee() {
    let (out, code) = run(&format!(
        "`timescale 1ns/1ns\nmodule tb;\n  integer n;\n{PROBE}  \
         initial begin n = 9;\n    case (f(n)) default: $display(\"ad\"); endcase\n    \
         $finish;\n  end\nendmodule\n"
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("ad"), "{out}");
    assert_eq!(evals(&out), 1, "zero before this fix:\n{out}");
}

/// Falling through every arm to `default` is still one evaluation.
#[test]
fn a_case_that_matches_nothing_evaluates_its_scrutinee_once() {
    let (out, code) = run(&format!(
        "`timescale 1ns/1ns\nmodule tb;\n  integer n;\n{PROBE}  \
         initial begin n = 9;\n    case (f(n))\n      0: $display(\"a0\");\n      \
         1: $display(\"a1\");\n    default: $display(\"ad\");\n    endcase\n    \
         $finish;\n  end\nendmodule\n"
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("ad"), "{out}");
    assert_eq!(evals(&out), 1, "{out}");
}

/// Multi-label arms (`0,1,2:`) are several COMPARISONS in one arm, so they were
/// several evaluations — six here before the fix.
#[test]
fn multi_label_arms_share_one_evaluation() {
    let (out, code) = run(&format!(
        "`timescale 1ns/1ns\nmodule tb;\n  integer n;\n{PROBE}  \
         initial begin n = 5;\n    case (f(n))\n      0, 1, 2: $display(\"a012\");\n      \
         3, 4, 5: $display(\"a345\");\n    default: $display(\"ad\");\n    endcase\n    \
         $finish;\n  end\nendmodule\n"
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("a345"), "{out}");
    assert_eq!(evals(&out), 1, "{out}");
}

/// `casez` and `casex` go through the same cascade and had the same defect.
#[test]
fn casez_and_casex_evaluate_their_scrutinee_once() {
    for (kw, labels, want) in [
        (
            "casez",
            "4'b000?: $display(\"a0\");\n      4'b001?: $display(\"a1\");",
            "a1",
        ),
        (
            "casex",
            "4'b0xxx: $display(\"a0\");\n      4'b1x1x: $display(\"a1\");",
            "a1",
        ),
    ] {
        let val = if kw == "casez" { "4'b0011" } else { "4'b1010" };
        let (out, code) = run(&format!(
            "`timescale 1ns/1ns\nmodule tb;\n  reg [3:0] n;\n  \
             function [3:0] f(input [3:0] x); begin $display(\"EVAL%0d\", x); f = x; end \
             endfunction\n  initial begin n = {val};\n    {kw} (f(n))\n      {labels}\n    \
             default: $display(\"ad\");\n    endcase\n    $finish;\n  end\nendmodule\n"
        ));
        assert_eq!(code, Some(0), "{kw}:\n{out}");
        assert!(out.contains(want), "{kw} takes the right arm:\n{out}");
        assert_eq!(evals(&out), 1, "{kw}:\n{out}");
    }
}

/// A nested `case` in a matched arm: one evaluation each, not two each.
#[test]
fn a_nested_case_evaluates_each_scrutinee_once() {
    let (out, code) = run(&format!(
        "`timescale 1ns/1ns\nmodule tb;\n  integer n, m;\n{PROBE}  \
         initial begin n = 1; m = 2;\n    case (f(n))\n      0: $display(\"o0\");\n      \
         1: case (f(m))\n           0: $display(\"i0\");\n           2: $display(\"i2\");\n         \
         default: $display(\"id\");\n         endcase\n    default: $display(\"od\");\n    \
         endcase\n    $finish;\n  end\nendmodule\n"
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("i2"), "{out}");
    assert_eq!(evals(&out), 2, "one outer, one inner:\n{out}");
}

/// ⚠️ The capture carries SIGNEDNESS, and this is the cell that proves it. The
/// collective rule (§12.5 / §11.8.1) makes the whole comparison unsigned because
/// `4'hF` is unsigned, so `4'hF` is the arm — an unsigned capture would agree
/// here by luck, and `$signed(s)` against a lone signed label is the half that
/// separates them.
#[test]
fn the_capture_keeps_the_scrutinee_signedness() {
    let (out, code) = run("`timescale 1ns/1ns\nmodule tb;\n  reg signed [3:0] s;\n  \
         initial begin s = 4'hF;\n    \
         case (s) -1: $display(\"neg1\"); 4'hF: $display(\"hexF\"); default: $display(\"D\"); \
         endcase\n    \
         case ($signed(s)) -1: $display(\"s_neg1\"); default: $display(\"s_D\"); endcase\n    \
         $finish;\n  end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("hexF"), "collectively unsigned:\n{out}");
    assert!(
        out.contains("s_neg1"),
        "signed against a signed label:\n{out}"
    );
    // ⚠️ NOT `!contains("neg1")`: `s_neg1` contains it. The first case must not
    // take the `-1` arm, which is what an unsigned capture would have done.
    assert!(!out.lines().any(|l| l.trim() == "neg1"), "{out}");
}

/// ⚠️ The capture is 4-STATE. `casez`/`casex` match against the scrutinee's own
/// x/z bits, so a 2-state capture would zero them before the first test.
#[test]
fn the_capture_keeps_x_and_z_bits() {
    let (out, code) = run("`timescale 1ns/1ns\nmodule tb;\n  reg [3:0] n;\n  \
         initial begin n = 4'b1x0z;\n    \
         casez (n) 4'b1??0: $display(\"A\"); 4'b1x0z: $display(\"B\"); default: $display(\"D\"); \
         endcase\n    \
         casex (n) 4'b1xxx: $display(\"X1\"); default: $display(\"XD\"); endcase\n    \
         $finish;\n  end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains('A') && out.contains("X1"),
        "iverilog agrees:\n{out}"
    );
}

/// A scrutinee wider than one word still captures whole.
#[test]
fn a_wide_scrutinee_captures_at_its_own_width() {
    let (out, code) = run("`timescale 1ns/1ns\nmodule tb;\n  reg [99:0] w;\n  \
         initial begin w = 100'h3_0000_0000_0000_0000_0000_0002;\n    \
         case (w) 100'h1: $display(\"one\"); \
         100'h3_0000_0000_0000_0000_0000_0002: $display(\"big\"); \
         default: $display(\"D\"); endcase\n    $finish;\n  end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("big"), "{out}");
}

/// §12.5's common-maximum pass runs BEFORE the capture, so a bare-fill selector
/// is captured at the width its labels forced on it. `case ('1) 8'h01: '1:`
/// takes the `'1` arm in both oracles; a capture taken at the pre-widening
/// 1-bit width would send it to `default`.
#[test]
fn a_fill_selector_captures_after_the_common_width_pass() {
    let (out, code) = run("`timescale 1ns/1ns\nmodule tb;\n  \
         initial begin case ('1) 8'h01: $display(\"one\"); '1: $display(\"fill\"); \
         default: $display(\"D\"); endcase $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("fill"), "{out}");
}

/// The capture owns the scrutinee's reads now, so a runtime range report has to
/// keep pointing at the SCRUTINEE rather than at the `case` keyword — and it
/// fires ONCE. (Measured: this design reported at col 45 before, which is not
/// `mem[9]` either; the capture's own span puts it on the read.)
#[test]
fn an_out_of_range_scrutinee_reports_once_at_the_read() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  reg [7:0] mem [0:3];\n  integer i;\n  \
         initial begin\n    for (i = 0; i < 4; i = i + 1) mem[i] = i[7:0];\n    \
         case (mem[9]) 8'd0: $display(\"z\"); default: $display(\"d\"); endcase\n    \
         $finish;\n  end\nendmodule\n",
    );
    assert_ne!(code, Some(0), "an out-of-range read is E4002:\n{out}");
    assert_eq!(
        out.matches("E-RUN-RANGE").count(),
        1,
        "one read, one report:\n{out}"
    );
    assert!(
        out.contains("t.sv:7:11"),
        "the caret is on `mem[9]`:\n{out}"
    );
}

/// The synthetic capture net must not appear in a waveform. It shares the
/// `$ia_tmp$` sigil precisely so the existing VCD filter covers it — §4.5.374
/// paid once for a new sigil slipping past that filter.
#[test]
fn the_capture_net_stays_out_of_the_vcd() {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cso_vcd_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(
        d.join("t.sv"),
        "`timescale 1ns/1ns\nmodule tb;\n  reg clk = 0; reg [1:0] st = 0; reg [7:0] o;\n  \
         always #5 clk = ~clk;\n  \
         always @(posedge clk) begin\n    case (st + 2'd1)\n      2'd0: o <= 8'hA0;\n      \
         2'd1: o <= 8'hA1;\n    default: o <= 8'hFF;\n    endcase\n    st <= st + 2'd1;\n  \
         end\n  initial begin $dumpfile(\"w.vcd\"); $dumpvars(0, tb); #40 $finish; end\n\
         endmodule\n",
    )
    .expect("write design");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stdout)
    );
    let vcd = std::fs::read_to_string(d.join("w.vcd")).expect("vcd");
    let _ = std::fs::remove_dir_all(&d);
    assert!(
        !vcd.contains("ia_tmp"),
        "the case capture net leaked into the VCD:\n{vcd}"
    );
}

/// An `always_comb` holding a `case` must still wake on the scrutinee. The
/// capture inserts a write between the read and the tests, and the sensitivity
/// list is collected from the AST — this is the cell that says the two did not
/// drift apart.
#[test]
fn an_always_comb_case_keeps_its_sensitivity() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  reg [1:0] sel = 0;\n  reg [7:0] o;\n  \
         always_comb case (sel) 2'd0: o = 8'd10; 2'd1: o = 8'd11; 2'd2: o = 8'd12; \
         default: o = 8'd99; endcase\n  \
         initial begin #1 $display(\"%0d\", o); sel = 2'd2; #1 $display(\"%0d\", o); \
         sel = 2'd3; #1 $display(\"%0d\", o); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    let vals: Vec<&str> = out
        .lines()
        .filter(|l| l.trim().parse::<u32>().is_ok())
        .collect();
    assert_eq!(vals, vec!["10", "12", "99"], "iverilog agrees:\n{out}");
}

/// Two instances of a module holding one `case` get their own capture net —
/// elaboration is per-instance, so this is a property of how nets are minted
/// rather than something the lowering arranges. Pinned because a shared capture
/// would make the second instance read the first one's scrutinee.
#[test]
fn two_instances_do_not_share_a_capture() {
    let (out, code) = run("`timescale 1ns/1ns\n\
         module m(input [1:0] s, output reg [7:0] o);\n  \
         always @* case (s) 2'd0: o = 8'd10; 2'd1: o = 8'd11; default: o = 8'd99; endcase\n\
         endmodule\n\
         module tb;\n  reg [1:0] x = 2'd0, y = 2'd1;\n  wire [7:0] a, b;\n  \
         m u0(.s(x), .o(a));\n  m u1(.s(y), .o(b));\n  \
         initial begin #1 $display(\"a=%0d b=%0d\", a, b); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("a=10 b=11"), "each instance its own:\n{out}");
}

/// ⚠️ A frame body is NOT hoisted, and this cell says so rather than leaving it
/// to the implementation to remember. `frame_write_lvalue` reaches only the
/// activation window, so a module-net capture would not be writable there; the
/// scrutinee is still re-evaluated per arm inside a function. Nothing observable
/// changes for a side-effect-free scrutinee, which is why this asserts the
/// ANSWER and not a count — when the follow-on lands, this test keeps passing.
#[test]
fn a_case_inside_a_frame_body_still_answers_correctly() {
    let (out, code) = run("`timescale 1ns/1ns\nmodule tb;\n  integer i, r;\n  \
         function automatic integer sel(input integer x);\n    integer j;\n    \
         begin sel = 0; for (j = 0; j < 1; j = j + 1) \
         case (x) 0: sel = 10; 1: sel = 11; 2: sel = 12; 3: sel = 13; default: sel = 99; \
         endcase end\n  endfunction\n  \
         initial begin for (i = 0; i < 4; i = i + 1) begin r = sel(i); $display(\"%0d\", r); \
         end $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    let vals: Vec<&str> = out
        .lines()
        .filter(|l| l.trim().parse::<u32>().is_ok())
        .collect();
    assert_eq!(vals, vec!["10", "11", "12", "13"], "{out}");
}
