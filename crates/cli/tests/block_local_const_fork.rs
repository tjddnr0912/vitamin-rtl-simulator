//! round-19 BL1: an `automatic` block-local under a `fork` whose initializer folds
//! to a CONSTANT and which is NEVER reassigned in the block is byte-identical to a
//! static net holding that constant — concurrency-immune (every activation reads the
//! same constant off one shared flattened net). Such a local was rejected E3009 by
//! the block-local per-entry-lifetime gate ("v1 flattens block-locals to one static
//! net"); it is now supported (the folded constant already rides `net.init`).
//!
//! No external oracle — iverilog 13.0 / verilator reject `automatic` lifetime
//! override (`sorry: Overriding the default variable lifetime`). Reference behavior
//! is the ALREADY-WORKING boundary: a STATIC const-init fork arm and an
//! `automatic` assigned-then-read (no initializer) fork arm both run today. So BL1's
//! gap was ONLY the `automatic` + constant-initializer form.
//!
//! §4.5.248 SUPERSEDES the "stays loud" half of this file. BL1's rule was
//! "concurrency-immune because the value is a constant nobody writes"; the real
//! invariant is ONE LIVE ACTIVATION OF THE BLOCK, and that holds for every arm of a
//! fork the process reaches once — an `initial` with no loop above it. So a NON-const
//! init and a REASSIGNED-after-init local under such a fork are supported now, with
//! their values pinned below. What stays loud is a fork that can be SPAWNED MORE THAN
//! ONCE (inside a loop, or in a repeating process): two live activations genuinely
//! cannot share one flattened net.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns (combined stdout+stderr, process_success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_blcf_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (text, out.status.success())
}

fn loud(src: &str) -> bool {
    let (o, ok) = run(src);
    !ok && o.contains("E3009")
}

// ── supported (loud → correct-support) ──────────────────────────────────────

#[test]
fn const_fork_watchdog_now_runs() {
    // The BL1 repro: a detached watchdog arm with a constant timeout. It elaborates
    // and simulates; `$finish` fires at time 1 and the `#5_000_000` watchdog never
    // matures (correct — a detached join_none arm killed by $finish), so "late" is
    // NOT printed and there is no E3009.
    let (o, ok) = run("module top;\n\
         initial begin\n\
           fork\n\
             begin\n\
               automatic int unsigned timeout_ns = 5_000_000;\n\
               #(timeout_ns * 1ns);\n\
               $display(\"late\");\n\
             end\n\
           join_none\n\
           $display(\"started\");\n\
           #1 $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(!o.contains("E3009"), "unexpected E3009:\n{o}");
    assert!(o.contains("started"), "sim did not run:\n{o}");
    assert!(!o.contains("late"), "watchdog matured but should not:\n{o}");
}

#[test]
fn const_fork_value_used() {
    // The constant's VALUE is actually available and correct inside the arm.
    let (o, ok) = run("module top;\n\
         initial begin\n\
           fork\n\
             begin\n\
               automatic int W = 8;\n\
               logic [7:0] x;\n\
               x = W - 5;\n\
               if (x == 3) $display(\"PASS\");\n\
             end\n\
           join_none\n\
           #1 $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("PASS"), "const value not usable in arm:\n{o}");
}

#[test]
fn const_fork_width_bound_stays_loud() {
    // ORTHOGONAL limitation (NOT what BL1 addresses): using a block-local `automatic`
    // const as a packed-range BOUND (`logic[W-1:0]`) is rejected by the pre-existing
    // "a reference to net/variable is not allowed in a constant range bound" rule — a
    // block-local is a runtime net, not an elaboration-time constant, so its use as a
    // width stays loud regardless of the per-entry-lifetime gate. Kept as a boundary
    // pin so a future BL1 change cannot silently start using a net as a range bound.
    assert!(loud(
        "module top;\n\
         initial begin\n\
           fork\n\
             begin\n\
               automatic int W = 8;\n\
               logic [W-1:0] x;\n\
               x = 3;\n\
               if (x == 3) $display(\"PASS\");\n\
             end\n\
           join_none\n\
           #1 $finish;\n\
         end\n\
         endmodule\n"
    ));
}

#[test]
fn const_fork_time_literal_delay() {
    // A time-literal fold (`#(t*1ns)`) is also const-immune; two arms with different
    // constant timeouts elaborate under one `join_none` fork.
    let (o, ok) = run("module top;\n\
         initial begin\n\
           fork\n\
             begin automatic int a = 3; #(a * 1ns); $display(\"A@%0t\", $time); end\n\
             begin automatic int b = 2; #(b * 1ns); $display(\"B@%0t\", $time); end\n\
           join_none\n\
           #5 $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(!o.contains("E3009"), "unexpected E3009:\n{o}");
    assert!(
        o.contains("A@") && o.contains("B@"),
        "arms did not fire:\n{o}"
    );
}

// ── §4.5.248: single-spawn fork arms — now supported, values pinned ─────────

#[test]
fn nonconst_init_in_a_single_spawn_fork_arm_runs() {
    // The initializer reads a module net, so BL1's constant argument does not apply —
    // but this `fork` is reached exactly once, so the arm has exactly one activation
    // and the flattened net holds exactly one value. Was E3009.
    let (o, ok) = run("module top;\n\
         logic [31:0] some_net;\n\
         initial begin\n\
           some_net = 7;\n\
           fork\n\
             begin\n\
               automatic int x = some_net;\n\
               #(x * 1ns);\n\
               $display(\"x=%0d\", x);\n\
             end\n\
           join_none\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("x=7"),
        "the init must observe the net's value:\n{o}"
    );
}

#[test]
fn a_written_block_local_in_a_single_spawn_fork_arm_runs() {
    // Constant initializer, then REASSIGNED — the standard watchdog shape
    // (`automatic int t = D; void'($value$plusargs(…, t)); #(t*1ns);`) in miniature.
    // One activation ⇒ the write is this activation's own. Was E3009.
    let (o, ok) = run("module top;\n\
         initial begin\n\
           fork\n\
             begin\n\
               automatic int c = 0;\n\
               c = c + 1;\n\
               $display(\"c=%0d\", c);\n\
             end\n\
           join_none\n\
           #1 $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("c=1"), "the write must land:\n{o}");
}

#[test]
fn a_copy_out_into_a_single_spawn_fork_local_lands() {
    // The write arrives through an `inout` formal's copy-out rather than an assignment
    // — the shape BL1's adversarial review used to argue the local was NOT immune. It
    // is not immune; it does not need to be, because there is one activation.
    let (o, ok) = run("module top;\n\
         function automatic int f(inout int io); io = io + 1; return io; endfunction\n\
         int y;\n\
         initial begin\n\
           fork\n\
             begin\n\
               automatic int c = 5;\n\
               y = f(c);\n\
               #10 $display(\"c=%0d y=%0d\", c, y);\n\
             end\n\
           join_none\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("c=6 y=6"), "inout copy-out:\n{o}");
}

#[test]
fn an_assert_action_write_in_a_single_spawn_fork_arm_lands() {
    // The write is in a deferred assertion's else-action. Same argument.
    let (o, ok) = run("module top;\n\
         logic x;\n\
         initial begin\n\
           x = 1'b0;\n\
           fork\n\
             begin\n\
               automatic int c = 5;\n\
               assert #0 (x) else c = 0;\n\
               #10 $display(\"c=%0d\", c);\n\
             end\n\
           join_none\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(
        o.contains("c=0"),
        "the failing assertion's action must write c:\n{o}"
    );
}

// ── still loud: a fork that can be SPAWNED MORE THAN ONCE ───────────────────

#[test]
fn a_fork_inside_a_loop_stays_loud() {
    // Iteration N+1 spawns a second arm while N's may still be live (`join_none`), so
    // two activations would share one flattened net. This is the case the old blanket
    // `under_fork` rejection was really protecting against.
    assert!(loud(
        "module top;\n\
         initial begin\n\
           for (int i = 0; i < 2; i++)\n\
             fork begin automatic int c = i; #2 $display(\"c=%0d\", c); end join_none\n\
           #10 $finish;\n\
         end\n\
         endmodule\n"
    ));
}

#[test]
fn a_fork_in_a_repeating_process_stays_loud() {
    // An `always` re-runs, so the same spawn point fires again while the previous
    // arm may still be live. The local is REASSIGNED, so BL1's separate
    // constant-is-immune exemption (a value every activation agrees on) does not
    // apply — this is the multiplicity gate itself.
    assert!(loud(
        "module top;\n\
         logic clk = 0; always #5 clk = ~clk;\n\
         always @(posedge clk)\n\
           fork begin automatic int c = 3; c = c + 1; #1 $display(\"c=%0d\", c); end join_none\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
    // A non-constant init in the same shape — BL1 never covered this one either.
    assert!(loud(
        "module top;\n\
         logic clk = 0; always #5 clk = ~clk;\n\
         int g = 4;\n\
         always @(posedge clk)\n\
           fork begin automatic int c = g; #1 $display(\"c=%0d\", c); end join_none\n\
         initial #40 $finish;\n\
         endmodule\n"
    ));
}

#[test]
fn a_fork_under_a_loop_under_a_fork_stays_loud() {
    // The OUTER fork is single-spawn, but a `forever` inside its arm re-reaches the
    // INNER fork — so `fork_multi` must propagate through the loop, not be cleared by
    // the outer fork's single-spawn verdict.
    assert!(loud(
        "module top;\n\
         initial fork\n\
           forever begin\n\
             fork begin automatic int c = 1; #1 $display(\"c=%0d\", c); end join_none\n\
             #3;\n\
           end\n\
         join_none\n\
         initial #9 $finish;\n\
         endmodule\n"
    ));
}

#[test]
fn a_static_decl_init_reading_an_automatic_sibling_stays_loud() {
    // §6.21: the static initializer runs at time 0, the `automatic` is initialized on
    // block entry — so the read has no value and a copy-out back into it is overwritten
    // by that entry initialization. Lifting the fork restriction UNCOVERED this (the
    // identical non-fork shape was silently printing the pre-copy-out value), so it is
    // pinned here as the loud it should always have been.
    assert!(loud(
        "module top;\n\
         function automatic int f(inout int io); io = io + 1; return io; endfunction\n\
         initial begin\n\
           fork\n\
             begin\n\
               automatic int c = 5;\n\
               begin\n\
                 int z = f(c);\n\
                 $display(\"%0d %0d\", z, c);\n\
               end\n\
             end\n\
           join_none\n\
           #1 $finish;\n\
         end\n\
         endmodule\n"
    ));
    // The NON-fork spelling of the same mistake — this one was silent-wrong before.
    assert!(loud(
        "module top;\n\
         function automatic int f(inout int io); io = io + 1; return io; endfunction\n\
         initial begin\n\
           begin automatic int c = 5; int z = f(c); $display(\"%0d %0d\", z, c); end\n\
           $finish;\n\
         end\n\
         endmodule\n"
    ));
}

// ── regression: the existing NON-fork const-init automatic (§4.5.213-D) ──────

#[test]
fn nonfork_const_init_automatic_still_works() {
    // A non-fork automatic-with-const-init block-local is handled by the pre-existing
    // per-entry path; the BL1 exception must not perturb it.
    let (o, ok) = run("module top;\n\
         initial begin\n\
           begin\n\
             automatic int lim = 20;\n\
             $display(\"lim=%0d\", lim);\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(ok, "expected clean sim, got:\n{o}");
    assert!(o.contains("lim=20"), "per-entry const init broke:\n{o}");
}
