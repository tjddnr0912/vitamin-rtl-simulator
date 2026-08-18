//! `repeat (<count>)` inside a SUSPENDABLE task — the count is folded through the
//! same funnel every select bound and replication count uses, and the CLASSIFIER
//! that decides whether the task may be lifted asks with the LOWERING's spelling.
//!
//! External aes_top round-3 §3.5. The reporter measured `repeat (64)` working and
//! `repeat (4*16)` / `repeat (LP)` / `repeat (m_n)` rejected, and — because the
//! diagnostic listed causes that were all frame-LOCAL — re-wrote a correct
//! `localparam` twice looking for a hazard that was not there.
//!
//! ORACLE: iverilog 13.0 runs every count form; each expected value below is its
//! output. The two that stay loud are pinned for the OPPOSITE reason: a count SV
//! truncates (`4'd15 + 4'd1` is 0 at four bits, not 16) must never be folded by the
//! width-unlimited domain, and a count read from a variable genuinely needs the
//! runtime counter this subset cannot host.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_rcf_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// A suspendable task (it has `@(posedge clk)` inside the `repeat`), so the count
/// decides whether the task can be lifted at all.
fn design(count: &str) -> String {
    format!(
        "module tb;\n\
         \x20 localparam int LP = 64;\n\
         \x20 logic clk = 0; int m_n; int c;\n\
         \x20 always #1 clk = ~clk;\n\
         \x20 task automatic t; begin c = 0; repeat ({count}) begin @(posedge clk); c = c + 1; end end endtask\n\
         \x20 initial begin m_n = 3; #0; m_n = 64; t(); $display(\"c=%0d\", c); $finish; end\n\
         endmodule\n"
    )
}

#[test]
fn folded_constant_counts_run_the_iverilog_number_of_times() {
    // Every expectation is iverilog 13.0's output for the same source.
    for (count, want) in [
        ("64", "c=64"),        // literal — worked before, must not move
        ("4*16", "c=64"),      // §3.5: no `Binary` arm existed in the v1 folder
        ("LP", "c=64"),        // a localparam: needs the full const domain
        ("LP/2", "c=32"),      // …and its operator set
        ("$clog2(LP)", "c=6"), // …and its system functions
    ] {
        let out = run(&design(count));
        assert!(
            out.contains(want),
            "repeat ({count}) should run the iverilog count.\nwant {want}\ngot:\n{out}"
        );
        assert!(
            !out.contains("E3009"),
            "repeat ({count}) must not be loud:\n{out}"
        );
    }
}

#[test]
fn a_fill_literal_count_is_one_iteration_and_the_classifier_agrees() {
    // §11.6: `'1` is self-determined to ONE bit ⇒ exactly one iteration (iverilog
    // agrees). THIS is the discriminator for the two-spellings defect: `lower_repeat`
    // has always unrolled a fill literal, but the classifier's own copy of the
    // question had no fill arm, so it saw "not a constant the unroller consumes",
    // flagged the timing, and rejected a task the lowering would have built fine.
    let out = run(&design("'1"));
    assert!(out.contains("c=1"), "`repeat ('1)` runs once:\n{out}");
    assert!(
        !out.contains("E3009"),
        "`repeat ('1)` must not be loud:\n{out}"
    );
    // `'0` is zero iterations — the same arm, other side.
    let z = run(&design("'0"));
    assert!(z.contains("c=0"), "`repeat ('0)` runs zero times:\n{z}");
}

#[test]
fn a_count_sv_truncates_runs_the_truncated_number_of_times() {
    // INVERTED 2026-08-19, and STRONGER. This used to assert the refusal plus "and
    // never the unlimited-domain value 16". The count now reaches the runtime counter,
    // which evaluates it at its SELF width (§11.6) — so `4'd15 + 4'd1` is 0 at four
    // bits, exactly iverilog's answer. The old negative assertion is kept: 16 is what
    // the width-unlimited fold would say, and it must still never appear.
    let out = run(&design("4'd15+4'd1"));
    assert!(
        out.contains("c=0"),
        "SV truncates to 0 at four bits:\n{out}"
    );
    assert!(
        !out.contains("c=16"),
        "…never the unlimited-domain value:\n{out}"
    );
    // The wider spelling of the same rule (ROADMAP §2, fixed by this slice): 300 at
    // eight bits is 44.
    let w = run(&design("8'd200+8'd100"));
    assert!(w.contains("c=44"), "8-bit wrap, iverilog's answer:\n{w}");
    assert!(!w.contains("c=300"), "…not the unwrapped 300:\n{w}");
}

#[test]
fn a_variable_count_runs_with_a_per_activation_counter() {
    // INVERTED 2026-08-19. A runtime count needed a counter net, and that net was
    // MODULE-scope — which concurrent activations of a suspendable task would share,
    // so the task was refused. The reservation pass now puts a FRAME-LOCAL counter
    // aside for each such `repeat`, which removes the hazard instead of diagnosing it.
    let out = run(&design("m_n"));
    assert!(out.contains("c=64"), "a module-net count now runs:\n{out}");
    assert!(!out.contains("E3009"), "no longer loud:\n{out}");
}

#[test]
fn two_concurrent_activations_do_not_share_the_counter() {
    // THE discriminator for "frame-local", and the only one: with a module-scope
    // counter both activations decrement the same net, so the counts cross. Two forked
    // calls with different counts must each finish their own (iverilog: 3 and 5).
    let out = run(
        "module tb;\n         \x20 logic clk = 0; int n1 = 3, n2 = 5; int a, b;\n         \x20 always #1 clk = ~clk;\n         \x20 task automatic t(input int n, output int o);\n         \x20   begin o = 0; repeat (n) begin @(posedge clk); o = o + 1; end end\n         \x20 endtask\n         \x20 initial fork t(n1, a); t(n2, b); join_none\n         \x20 initial begin #20; $display(\"c=%0d %0d\", a, b); $finish; end\n         endmodule\n",
    );
    assert!(
        out.contains("c=3 5"),
        "each activation must count its own (iverilog: 3 5):\n{out}"
    );
}

#[test]
fn a_non_repeat_cause_does_not_blame_the_repeat() {
    // Control: the message is only useful if it DISCRIMINATES. A nonblocking assign to
    // a frame-local is a different rejecting predicate and must be named as itself.
    let out = run("module tb; logic clk=0; always #1 clk=~clk;\n\
         \x20 task automatic t; int loc; begin loc <= 1; @(posedge clk); end endtask\n\
         \x20 initial begin t(); $finish; end endmodule\n");
    assert!(out.contains("E3009"), "still loud:\n{out}");
    assert!(
        out.contains("nonblocking assign"),
        "the message must name the real construct:\n{out}"
    );
    assert!(
        !out.contains("`repeat`"),
        "…and must NOT mention `repeat`, which is not why this was rejected:\n{out}"
    );
}

#[test]
fn a_variable_count_outside_a_frame_still_runs_the_runtime_counter() {
    // Anti-vacuity for the unchanged half: the runtime-counter desugar is untouched,
    // so a variable count in an `initial` still runs (iverilog: c=5).
    let out = run("module tb; int n, c;\n\
         \x20 initial begin n = 5; c = 0; repeat (n) c = c + 1; $display(\"c=%0d\", c); end\n\
         endmodule\n");
    assert!(
        out.contains("c=5"),
        "runtime counter path unchanged:\n{out}"
    );
}

#[test]
fn a_constant_count_above_the_unroll_cap_runs_on_the_runtime_counter() {
    // INVERTED 2026-08-19. The unroll cap (1024) still applies — a count above it is
    // NOT unrolled — but the runtime counter it falls back to is now a per-activation
    // FRAME-LOCAL, so the suspendable task hosts it instead of refusing. iverilog runs
    // 2048 and so does this.
    let out = run(&design("LP*32")); // 2048
    assert!(
        out.contains("c=2048"),
        "must run the iverilog count:\n{out}"
    );
    assert!(!out.contains("E3009"), "no longer loud:\n{out}");
    // At the cap it is UNROLLED instead — a different mechanism reaching the same
    // number, which is what keeps the cap itself under test.
    let ok = run(&design("LP*16")); // 1024
    assert!(ok.contains("c=1024"), "at the cap it unrolls:\n{ok}");
}

#[test]
fn the_general_backstop_names_its_own_class_not_the_repeat() {
    // The THIRD rejecting predicate — the body is not a leaf non-suspending frame body.
    // Reached today by a `return` inside a fork arm (measured with a `panic!` probe:
    // three suite designs land here, all in `cli::fork_in_frame`). It cannot name one
    // construct the way the other two do, but it must still say WHICH class it is, and
    // it must not borrow another predicate's noun.
    let out = run(
        "module tb;\n         \x20 logic clk = 0; always #5 clk = ~clk;\n         \x20 task automatic other; @(posedge clk); endtask\n         \x20 task automatic run;\n         \x20   @(posedge clk);\n         \x20   fork begin @(posedge clk); return; end other(); join\n         \x20   $display(\"after join\");\n         \x20 endtask\n         \x20 initial begin run(); $finish; end\n         endmodule\n",
    );
    assert!(out.contains("E3009"), "still loud:\n{out}");
    assert!(
        out.contains("not a leaf"),
        "the backstop must name its own class:\n{out}"
    );
    // Only the CAUSE clause is under test — the trailing "Supported here: …" list
    // legitimately mentions a nonblocking assign (to a MODULE net, which is allowed).
    let cause = out.split(". Supported here").next().unwrap_or(&out);
    assert!(
        !cause.contains("`repeat`") && !cause.contains("local to this task"),
        "…and must not borrow another predicate's noun:\n{cause}"
    );
}

#[test]
fn a_narrow_signed_count_keeps_its_sign() {
    // DISCRIMINATOR for the self-width seal's `!signed` guard. The seal ZERO-extends,
    // which is right for an unsigned count and would turn `-4'sd1` (4'b1111) into 15.
    // A negative count runs the body zero times (§12.7.3) and iverilog agrees, so the
    // signed case must be left alone and let the signed 32-bit counter's `> 0` test
    // answer. `4'sd3` in the same shape proves the arm is not simply skipped.
    for (expr, want) in [("-4'sd1", "c=0"), ("-3'sd2", "c=0"), ("4'sd3", "c=3")] {
        let out = run(&format!(
            "module tb; int c;\n             \x20 initial begin c=0; repeat ({expr}) c=c+1; $display(\"c=%0d\", c); end\n             endmodule\n"
        ));
        assert!(
            out.contains(want),
            "`repeat ({expr})` must give {want}:\n{out}"
        );
    }
}

#[test]
fn a_nested_runtime_repeat_gets_its_own_counter() {
    // DISCRIMINATOR for the reservation walking INTO a `repeat`'s body: the inner one
    // needs its own frame-local counter, and if the walk stops at the outer `repeat`
    // the inner falls to the module net — which the lowering-site backstop then
    // refuses, so the failure is loud rather than wrong. Either way this design must
    // run and count 2 x 2 (iverilog: 4).
    let out = run(
        "module tb; logic clk = 0; int n = 2, c; always #1 clk = ~clk;\n         \x20 task automatic t;\n         \x20   begin c = 0; repeat (n) repeat (n) begin @(posedge clk); c = c + 1; end end\n         \x20 endtask\n         \x20 initial begin t(); $display(\"c=%0d\", c); $finish; end\n         endmodule\n",
    );
    assert!(
        out.contains("c=4"),
        "nested runtime repeats each count (iverilog: 4):\n{out}"
    );
    assert!(!out.contains("E3009"), "…and neither is refused:\n{out}");
}
