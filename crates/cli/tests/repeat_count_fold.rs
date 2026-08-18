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
fn a_count_sv_truncates_is_never_folded_by_the_unlimited_domain() {
    // `4'd15 + 4'd1` is ZERO at four bits (iverilog runs the body zero times); the
    // width-unlimited i64 domain would say 16. Folding it would be a WRONG NON-ZERO
    // count — worse than the refusal. `const_bound_u32`'s width-exactness guard
    // declines, which leaves it on the runtime-counter path: loud here, not wrong.
    let out = run(&design("4'd15+4'd1"));
    assert!(
        out.contains("E3009"),
        "a width-truncating count must stay OUT of the strong domain:\n{out}"
    );
    assert!(
        !out.contains("c=16"),
        "…and must never produce the unlimited-domain value 16:\n{out}"
    );
}

#[test]
fn a_variable_count_stays_loud_and_the_message_names_the_repeat() {
    // A module net read at run time genuinely needs the shared `$repeat_cnt$` counter,
    // which concurrent activations would corrupt. Still loud — but the diagnostic must
    // say WHICH construct, because "a frame-local unpacked ARRAY / a `wait`/`repeat`
    // reading a frame-local" describes neither a localparam nor a module net, and that
    // is what sent the reporter after the wrong fix twice.
    let out = run(&design("m_n"));
    assert!(out.contains("E3009"), "variable count stays loud:\n{out}");
    assert!(
        out.contains("`repeat`"),
        "the message must name the `repeat` as the cause:\n{out}"
    );
    assert!(
        out.contains("repeat (LP)"),
        "…and must say a folded constant count IS supported, so the reader stops \
         rewriting correct constants:\n{out}"
    );
    // The count must NOT be silently folded to the net's declaration-time value.
    assert!(
        !out.contains("c="),
        "no value may be produced at all:\n{out}"
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
fn a_constant_count_above_the_unroll_cap_stays_on_the_runtime_path() {
    // DISCRIMINATOR for the unroll cap (`REPEAT_UNROLL_CAP` = 1024). A folded count
    // is only unrolled while the cap allows it; above it the desugar falls back to the
    // shared counter net, which this subset cannot host — so the task stays loud.
    // iverilog runs it (c=2048), so this is an honest-loud boundary, not a value gap:
    // without the cap the elaborator would emit 2048 copies of the body.
    let out = run(&design("LP*32")); // 2048
    assert!(
        out.contains("E3009"),
        "a count over the unroll cap must not be unrolled:\n{out}"
    );
    assert!(!out.contains("c=2048"), "…and must not run:\n{out}");
    // Directly under the cap it DOES fold and run — so the assertion above is about the
    // cap, not about `LP*32` being unfoldable.
    let ok = run(&design("LP*16")); // 1024, exactly at the cap
    assert!(ok.contains("c=1024"), "at the cap it still unrolls:\n{ok}");
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
