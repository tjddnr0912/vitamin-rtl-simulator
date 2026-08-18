//! `default clocking` (IEEE 1800-2017 §14.12) supplies the clocking event to a
//! concurrent assertion that gives none.
//!
//! External aes_top round-3 §3.6. The block itself already PARSED — the parser even
//! built an `is_default` flag — but nothing in elaborate ever read it, so the feature
//! was a dead contract: a design could declare a default clocking, get `errors=0`,
//! and still have every assertion that relied on it rejected. The consumer side was
//! refused one layer earlier, in the parser, which reported the wrong cause (the
//! assertion is fine; whether the scope has a clock is not a parse-time fact).
//!
//! ORACLE: verilator 5.050. iverilog 13.0 cannot parse `default clocking` at all, so
//! it is not an oracle here. Every expected line below is verilator's output for the
//! same source, including the two REJECTs — verilator also refuses an unclocked
//! assertion with no default clocking, which is why removing the parse error had to
//! move that refusal rather than delete it.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dclk_{}_{n}", std::process::id()));
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

/// Fires on every `posedge clk` while `b` is 0; `b` goes 1 at t=6.
/// verilator: FAIL t=1, FAIL t=3, FAIL t=5.
const BODY: &str = "  initial begin #6; b = 1; #4 $finish; end\nendmodule\n";

fn head(extra: &str) -> String {
    format!("module tb;\n  logic clk = 0; logic a = 1, b = 0;\n  always #1 clk = ~clk;\n{extra}")
}

#[test]
fn default_clocking_supplies_the_clock_to_an_unclocked_assertion() {
    let out = run(&format!(
        "{}{}",
        head(
            "  default clocking cb @(posedge clk); endclocking\n\
             \x20 assert property (a |-> b) else $display(\"FAIL t=%0t\", $time);\n"
        ),
        BODY
    ));
    for t in ["FAIL t=1", "FAIL t=3", "FAIL t=5"] {
        assert!(out.contains(t), "verilator prints {t}:\n{out}");
    }
    // ANTI-VACUITY: it must also STOP. If the clock were never wired the output would
    // be empty and the three asserts above would be the only thing under test.
    assert!(
        !out.contains("FAIL t=7"),
        "must stop once the property holds (b=1 at t=6):\n{out}"
    );
}

#[test]
fn without_a_default_clocking_an_unclocked_assertion_is_still_loud() {
    // verilator: "Concurrent assertion has no clock" — REJECT. Removing the parse
    // error must MOVE this refusal, never delete it.
    let out = run(&format!(
        "{}{}",
        head("  assert property (a |-> b) else $display(\"FAIL t=%0t\", $time);\n"),
        BODY
    ));
    assert!(out.contains("E3009"), "must stay loud:\n{out}");
    assert!(
        out.contains("default clocking"),
        "…and the message must name the way out that the design does not have:\n{out}"
    );
    assert!(!out.contains("FAIL t="), "nothing may run:\n{out}");
}

#[test]
fn an_explicit_clocking_event_wins_over_the_default() {
    // `c2` toggles every 3 ⇒ posedges at t=3 and t=9 (verilator: FAIL t=3, FAIL t=9).
    // If the default were applied instead, the times would be the `clk` ones.
    let out = run("module tb;\n  logic clk = 0, c2 = 0; logic a = 1, b = 0;\n\
         \x20 always #1 clk = ~clk; always #3 c2 = ~c2;\n\
         \x20 default clocking cb @(posedge clk); endclocking\n\
         \x20 assert property (@(posedge c2) a |-> b) else $display(\"FAIL t=%0t\", $time);\n\
         \x20 initial begin #10 $finish; end\nendmodule\n");
    assert!(
        out.contains("FAIL t=3") && out.contains("FAIL t=9"),
        "explicit clock:\n{out}"
    );
    assert!(
        !out.contains("FAIL t=1"),
        "the default must NOT be applied:\n{out}"
    );
}

#[test]
fn a_named_property_with_no_clock_of_its_own_takes_the_default() {
    let out = run(&format!(
        "{}{}",
        head(
            "  default clocking cb @(posedge clk); endclocking\n\
             \x20 property p; a |-> b; endproperty\n\
             \x20 assert property (p) else $display(\"FAIL t=%0t\", $time);\n"
        ),
        BODY
    ));
    for t in ["FAIL t=1", "FAIL t=3", "FAIL t=5"] {
        assert!(
            out.contains(t),
            "a named property inherits it too — {t}:\n{out}"
        );
    }
}

#[test]
fn a_bare_name_that_is_not_a_property_is_a_clocked_boolean() {
    // `assert property (a)` parses to a property INSTANCE; there is no `property a`,
    // and with a default clocking in scope verilator reads it as a clocked boolean.
    // This elaborator already ran the explicit spelling `assert property (@(clk) a)`,
    // so the only thing that was missing is the implicit clock.
    let out = run(
        "module tb;\n  logic clk = 0; logic a = 0;\n  always #1 clk = ~clk;\n\
         \x20 default clocking cb @(posedge clk); endclocking\n\
         \x20 assert property (a) else $display(\"FAIL t=%0t\", $time);\n\
         \x20 initial begin #6; a = 1; #4 $finish; end\nendmodule\n",
    );
    for t in ["FAIL t=1", "FAIL t=3", "FAIL t=5"] {
        assert!(out.contains(t), "clocked boolean — {t}:\n{out}");
    }
    assert!(
        !out.contains("FAIL t=7"),
        "…and it stops when `a` goes 1:\n{out}"
    );
}

#[test]
fn without_a_default_clocking_a_bare_name_keeps_the_unknown_property_message() {
    // CONTROL for the arm above: the re-interpretation is gated on a default clocking
    // existing, so a design with no clock to fall back on keeps the diagnostic that
    // names both ways out. Without this the arm would swallow a genuine typo.
    let out = run(
        "module tb;\n  logic clk = 0; logic a = 0;\n  always #1 clk = ~clk;\n\
         \x20 assert property (a) else $display(\"FAIL t=%0t\", $time);\n\
         \x20 initial begin #6; a = 1; #4 $finish; end\nendmodule\n",
    );
    assert!(out.contains("E3009"), "still loud:\n{out}");
    assert!(
        out.contains("unknown property"),
        "the typo-naming message is the right answer with no default clocking:\n{out}"
    );
}

#[test]
fn an_unsupported_clocking_event_does_not_become_an_invisible_default() {
    // The default is recorded only AFTER the edge-list check, so a `default clocking`
    // this subset rejects (a level event) cannot silently arm assertions with a clock
    // the engine never wired: the block is reported, and the assertion then reports
    // that it has no clock. Two loud diagnostics, no silent run.
    let out = run(&format!(
        "{}{}",
        head(
            "  default clocking cb @(clk); endclocking\n\
             \x20 assert property (a |-> b) else $display(\"FAIL t=%0t\", $time);\n"
        ),
        BODY
    ));
    assert!(!out.contains("FAIL t="), "nothing may run:\n{out}");
    assert!(out.contains("E3009"), "loud:\n{out}");
}

#[test]
fn the_default_does_not_leak_into_a_sibling_module() {
    // `default_clocking` is MODULE-LOCAL and `lower_clocking_blocks` clears it per
    // module. Every other test here has one module, so none of them can see a leak:
    // this is the only shape where forgetting the clear is observable. The child has
    // no default clocking of its own, so its unclocked assertion must be LOUD — if the
    // parent's clock leaked, it would silently run on the wrong scope's clock instead.
    let out = run(
        "module child;\n  logic a = 1, b = 0;\n         \x20 assert property (a |-> b) else $display(\"CHILD t=%0t\", $time);\n         endmodule\n         module tb;\n  logic clk = 0; logic a = 1, b = 0;\n  always #1 clk = ~clk;\n         \x20 default clocking cb @(posedge clk); endclocking\n         \x20 child u();\n         \x20 assert property (a |-> b) else $display(\"TB t=%0t\", $time);\n         \x20 initial begin #6 $finish; end\nendmodule\n",
    );
    assert!(
        out.contains("E3009"),
        "the child's assertion must be loud:\n{out}"
    );
    assert!(
        !out.contains("CHILD t="),
        "the parent's default clocking must NOT reach the child:\n{out}"
    );
}

// ── §16.15 `default disable iff` — the second scope-level default ──────────────

#[test]
fn default_disable_iff_suppresses_until_the_condition_clears() {
    // The scope's reset gates every assertion that does not write its own. `rst` drops
    // at t=4, so the property is only checked from t=5 on (verilator: FAIL t=5, 7, 9).
    let out = run(
        "module tb;\n  logic clk = 0, rst = 1; logic a = 1, b = 0;\n         \x20 always #1 clk = ~clk;\n         \x20 default clocking cb @(posedge clk); endclocking\n         \x20 default disable iff (rst);\n         \x20 assert property (a |-> b) else $display(\"FAIL t=%0t\", $time);\n         \x20 initial begin #4 rst = 0; #6 $finish; end\nendmodule\n",
    );
    for t in ["FAIL t=5", "FAIL t=7", "FAIL t=9"] {
        assert!(out.contains(t), "expected {t}:\n{out}");
    }
    // ANTI-VACUITY, and the whole point: the early edges must be SUPPRESSED. Without
    // this the test passes against an implementation that ignores the default entirely.
    assert!(
        !out.contains("FAIL t=1") && !out.contains("FAIL t=3"),
        "edges before the reset cleared must be suppressed:\n{out}"
    );
}

#[test]
fn an_explicit_disable_iff_wins_over_the_default() {
    // §16.15: the default applies only where the assertion gives none. Here the
    // explicit condition is 0 throughout, so nothing is suppressed even though the
    // scope default (`rst`) is high for the first four ticks. verilator agrees.
    let out = run(
        "module tb;\n  logic clk = 0, rst = 1, other = 0; logic a = 1, b = 0;\n         \x20 always #1 clk = ~clk;\n         \x20 default clocking cb @(posedge clk); endclocking\n         \x20 default disable iff (rst);\n         \x20 assert property (disable iff (other) a |-> b) else $display(\"FAIL t=%0t\", $time);\n         \x20 initial begin #4 rst = 0; #6 $finish; end\nendmodule\n",
    );
    for t in ["FAIL t=1", "FAIL t=3", "FAIL t=5"] {
        assert!(
            out.contains(t),
            "the explicit condition governs — expected {t}:\n{out}"
        );
    }
}

#[test]
fn without_a_default_disable_iff_nothing_is_suppressed() {
    // CONTROL: the same design minus the default must fire on every edge, so the test
    // above is measuring the default and not the clock.
    let out = run(
        "module tb;\n  logic clk = 0; logic a = 1, b = 0;\n         \x20 always #1 clk = ~clk;\n         \x20 default clocking cb @(posedge clk); endclocking\n         \x20 assert property (a |-> b) else $display(\"FAIL t=%0t\", $time);\n         \x20 initial begin #10 $finish; end\nendmodule\n",
    );
    for t in ["FAIL t=1", "FAIL t=3", "FAIL t=5"] {
        assert!(out.contains(t), "expected {t}:\n{out}");
    }
}

#[test]
fn the_default_reset_does_not_leak_into_a_sibling_module() {
    // Same module-locality property as `default clocking`, and the same reason it needs
    // its own test: every other case here has one module. The child has no default of
    // its own, so its assertion must fire from t=1 — if the parent's `rst` leaked, the
    // first two edges would be missing.
    let out = run(
        "module child(input logic clk);\n  logic a = 1, b = 0;\n         \x20 assert property (@(posedge clk) a |-> b) else $display(\"CHILD t=%0t\", $time);\n         endmodule\n         module tb;\n  logic clk = 0, rst = 1; logic a = 1, b = 0;\n         \x20 always #1 clk = ~clk;\n         \x20 default clocking cb @(posedge clk); endclocking\n         \x20 default disable iff (rst);\n         \x20 child u(clk);\n         \x20 initial begin #4 rst = 0; #4 $finish; end\nendmodule\n",
    );
    assert!(
        out.contains("CHILD t=1"),
        "the parent's default reset must NOT reach the child:\n{out}"
    );
}

#[test]
fn a_second_default_disable_iff_in_one_scope_is_loud() {
    // §16.15 allows exactly one per scope; verilator says so explicitly ("Only one
    // 'default disable iff' allowed per module"). vita used to accept two and keep
    // the last, which makes WHICH reset applies depend on declaration order — and is
    // the only design where the "first or last?" question is observable at all, so it
    // is also the discriminator the mutation battery asked for.
    let out = run(
        "module tb;\n  logic clk = 0, r1 = 1, r2 = 0; logic a = 1, b = 0;\n         \x20 always #1 clk = ~clk;\n         \x20 default clocking cb @(posedge clk); endclocking\n         \x20 default disable iff (r1);\n         \x20 default disable iff (r2);\n         \x20 assert property (a |-> b) else $display(\"FAIL t=%0t\", $time);\n         \x20 initial begin #4 r1 = 0; #6 $finish; end\nendmodule\n",
    );
    assert!(out.contains("E3009"), "a second one must be loud:\n{out}");
    assert!(!out.contains("FAIL t="), "…and nothing may run:\n{out}");
}
