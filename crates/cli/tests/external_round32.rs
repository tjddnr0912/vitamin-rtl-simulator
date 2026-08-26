//! External report round-32 (hash_top) and aes_top 2026-08-24, re-triaged at HEAD.
//!
//! Each test names the report item it closes. Every expectation here was measured against
//! iverilog before it was written down — the reports were filed against `6c4be81`, four
//! slices back, so "still reproduces" was established rather than assumed.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r32_{}_{n}", std::process::id()));
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
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

// ── N32-1: `$bits(<expr>)` as a packed declaration bound ─────────────────────

/// The silent-wrong the report led with. `wire [$bits(8'h00)-1:0] c;` declared a ONE-BIT
/// net, so `c <= 8'hA5` truncated to `1` at exit 0 in every backend — and on a PORT it
/// did that across a module boundary. The same call answered 8 at runtime, 8 as an
/// unpacked dimension, and E3009 as a parameter value: one source line, three answers.
/// iverilog: `c=8 d=16 e=6 PORT_W=8`.
#[test]
fn bits_of_an_expression_folds_in_a_packed_declaration_bound() {
    let (o, e, code) = run("module sub (input logic [$bits(8'h00)-1:0] p);\n\
           initial #2 $display(\"PORT_W=%0d\", $bits(p));\n\
         endmodule\n\
         module tb;\n\
           logic [7:0] s8 = 8'hA5;\n\
           wire  [$bits(8'h00)-1:0]      c_net;\n\
           logic [$bits({s8,s8})-1:0]    d_var;\n\
           wire  [$bits({3{2'b01}})-1:0] e_repl;\n\
           assign c_net = 8'hA5;\n\
           sub u(.p(8'hA5));\n\
           initial #1 $display(\"c=%0d d=%0d e=%0d\",\n\
             $bits(c_net), $bits(d_var), $bits(e_repl));\n\
           initial #9 $finish;\n\
         endmodule\n");
    assert_eq!(code, Some(0), "must run: {o}{e}");
    assert!(
        o.contains("c=8 d=16 e=6"),
        "literal / concat / replication:\n{o}"
    );
    assert!(
        o.contains("PORT_W=8"),
        "a port bound must not truncate:\n{o}"
    );
}

/// The other half, and the more important one: a `$bits` shape the constant domain still
/// cannot see must be LOUD, not a silent 1-bit net. Before this it silently declared one
/// bit even for `$bits(<undeclared name>)`.
#[test]
fn an_unfoldable_bits_bound_is_loud_not_one_bit() {
    for arg in ["mem[0][1]", "no_such_name"] {
        let (o, e, code) = run(&format!(
            "module tb;\n  logic [7:0] mem [0:3];\n\
               wire [$bits({arg})-1:0] w;\n\
               initial begin #1 $display(\"VAL=%0d\", $bits(w)); $finish; end\n\
             endmodule\n"
        ));
        let all = format!("{o}{e}");
        assert_ne!(code, Some(0), "`$bits({arg})` must be loud:\n{all}");
        assert!(
            !o.contains("VAL=1"),
            "must not silently declare 1 bit:\n{all}"
        );
    }
}

/// Controls: the positions that already answered correctly must not move.
#[test]
fn the_bits_positions_that_worked_still_work() {
    let (o, e, code) = run(
        "module tb;\n  logic [7:0] s8;\n  logic [7:0] m [$bits(8'h00)];\n\
           localparam int W = $bits(8'h00);\n\
           initial begin\n\
             $display(\"VAL RT=%0d UP=%0d NAME=%0d TYPE=%0d P=%0d\",\n\
               $bits({3{2'b01}}), $size(m), $bits(s8), $bits(logic [7:0]), W);\n\
             #1 $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "{o}{e}");
    assert!(
        o.contains("VAL RT=6 UP=8 NAME=8 TYPE=8 P=8"),
        "runtime / unpacked dim / name / type / parameter:\n{o}"
    );
}

// ── N32-3: the rejection diagnostic cited a deleted rule ─────────────────────

/// The message said these calls are "supported only as the direct rhs of a blocking
/// assignment (v9)" — a rule §4.5.374 removed. Taken at face value it told users to
/// rewrite working code. The caret also sat on the statement head rather than the call.
#[test]
fn the_file_read_rejection_states_the_rule_that_is_actually_in_force() {
    let (o, e, code) = run("module tb; integer fd, a; initial begin\n\
           fd = $fopen(\"x.txt\",\"r\");\n\
           if (a &&\n\
               ($fgetc(fd) != -1)) $display(\"y\");\n\
           $finish; end\n\
         endmodule\n");
    let all = format!("{o}{e}");
    assert_ne!(
        code,
        Some(0),
        "the `&&` right operand is still refused:\n{all}"
    );
    assert!(
        !all.contains("direct rhs of a blocking assignment"),
        "the deleted rule must not be quoted:\n{all}"
    );
    assert!(
        all.contains("DIFFERENT NUMBER OF TIMES"),
        "say why this position is different:\n{all}"
    );
    assert!(
        all.contains(":4:"),
        "the caret belongs on the call (line 4), not the `if` (line 3):\n{all}"
    );
}

/// …and the positions §4.5.374 opened must keep working, which is what makes the old
/// message's advice actively harmful.
#[test]
fn the_opened_file_read_positions_still_work() {
    let (o, e, code) = run("module tb; integer fd, n, c; initial begin\n\
           fd = $fopen(\"x.txt\",\"r\");\n\
           if ($fgetc(fd) != -1) n = 1;\n\
           if (!$value$plusargs(\"N=%d\", n)) n = 2;\n\
           c = ($fgetc(fd) != -1) ? 1 : 0;\n\
           $display(\"VAL=%0d %0d\", n, c);\n\
           $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "{o}{e}");
    assert!(o.contains("VAL="), "{o}");
}

// ── aes_top §4: W2004 and the chain of indices ───────────────────────────────

/// `a[7:0][3:0]` went unwarned while vita silently returned the whole `a`. iverilog
/// states the rule exactly — "All but the final index in a chain of indices must be a
/// single value, not a range" — and that is decidable at parse time.
#[test]
fn a_range_in_a_non_final_index_warns() {
    let (o, e, _) = run("module tb;\n  logic [7:0] a = 8'h5a;\n  logic [3:0] o2;\n\
           assign o2 = a[7:0][3:0];\n\
           initial begin #1 $display(\"o2=%h\", o2); $finish; end\n\
         endmodule\n");
    assert!(
        format!("{o}{e}").contains("W2004"),
        "a part-select of a part-select is the shape iverilog rejects:\n{o}{e}"
    );
}

/// The false-positive control, and the reason a blanket "second select warns" would have
/// been worse than the gap: an ARRAY INDEX followed by a select is legal everywhere and
/// appears in every design. iverilog runs it (`VAL=a 0`).
#[test]
fn an_array_index_before_a_select_does_not_warn() {
    let (o, e, code) = run(
        "module tb;\n  logic [7:0] mem [0:3];\n  logic [3:0] o1, o2;\n\
           initial begin mem[1] = 8'h5a;\n\
             o1 = mem[1][3:0]; o2 = mem[1][2];\n\
             $display(\"VAL=%h %h\", o1, o2); #1 $finish; end\n\
         endmodule\n",
    );
    assert_eq!(code, Some(0), "{o}{e}");
    assert!(o.contains("VAL=a 0"), "values must match iverilog:\n{o}");
    assert!(
        !format!("{o}{e}").contains("W2004"),
        "the common idiom must stay silent:\n{o}{e}"
    );
}

// ── aes_top §5: two diagnostics contradicting each other ─────────────────────

/// A wide-literal override said "is not a constant; default kept" while the companion
/// check errored — so it was a constant, and no default was kept. Both halves wrong.
///
/// ⚠️ **This test used to assert the WORDING of a refusal.** Round 34 removed the
/// refusal: `bits` became one of the channels both `keeps_default_of` and
/// `has_applied_override` ask about, so the override is APPLIED. A wording assertion
/// over a limitation outlives the limitation, so it is now a VALUE assertion — which
/// is a strictly stronger statement of the same complaint (the two diagnostics
/// contradicted each other about one override, and now there are none because the
/// override lands). Pinned to live iverilog 13.0, which prints the same line.
#[test]
fn a_wide_literal_override_is_named_for_what_it_is() {
    let (o, e, code) = run(
        "module leaf #(parameter logic [127:0] K = 128'h0) (output logic [127:0] o);\n\
           assign o = K;\n\
         endmodule\n\
         module tb; logic [127:0] o;\n\
           leaf #(.K(128'hdeadbeef_00000000_00000000_00000001)) u(.o(o));\n\
           initial begin #1 $display(\"K=%032h\", o); $finish; end\n\
         endmodule\n",
    );
    let all = format!("{o}{e}");
    assert_eq!(code, Some(0), "the override applies now:\n{all}");
    assert!(
        o.contains("K=deadbeef000000000000000000000001"),
        "iverilog prints exactly this:\n{all}"
    );
    assert!(
        !all.contains("default kept") && !all.contains("WIDER than the 64-bit integer channel"),
        "neither half of the contradiction may survive:\n{all}"
    );
}
