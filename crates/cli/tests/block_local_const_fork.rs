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
//! correct-or-loud: a NON-const init under a fork (init reads a module net) and a
//! REASSIGNED-after-init local under a fork (`automatic int c=0; c=c+1;`) genuinely
//! need per-activation storage a shared module net cannot provide, so they stay loud.
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

// ── correct-or-loud (MUST stay loud) ────────────────────────────────────────

#[test]
fn nonconst_fork_stays_loud() {
    // The initializer reads a module net → not a compile-time constant → the arm
    // genuinely needs fresh per-activation storage, which a shared flattened net
    // cannot provide under concurrency. Stays E3009.
    assert!(loud(
        "module top;\n\
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
           #1 $finish;\n\
         end\n\
         endmodule\n"
    ));
}

#[test]
fn reassigned_const_fork_stays_loud() {
    // Constant initializer but REASSIGNED in the body — concurrency would alias the
    // mutable value across activations on one shared net. Stays E3009.
    assert!(loud(
        "module top;\n\
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
         endmodule\n"
    ));
}

#[test]
fn fork_inout_call_writes_const_stays_loud() {
    // CRITICAL-1 (adversarial): a function/method CALL in an EXPRESSION can WRITE the
    // const-init local via an `output`/`inout` copy-out. `y = f(c)` where `f` has an
    // `inout int io` mutates `c`, so it is NOT concurrency-immune — the write-scanner
    // must inspect the call in the rhs (it previously only saw lvalue roots and
    // statement-level task-call args, and silently ACCEPTED this → aliased net). Stays
    // E3009.
    assert!(loud(
        "module top;\n\
         function automatic int f(inout int io); io = io + 1; return io; endfunction\n\
         int y;\n\
         initial begin\n\
           fork\n\
             begin\n\
               automatic int c = 5;\n\
               y = f(c);\n\
               #10 $display(\"%0d\", c);\n\
             end\n\
           join_none\n\
           #1 $finish;\n\
         end\n\
         endmodule\n"
    ));
}

#[test]
fn fork_assert_action_writes_const_stays_loud() {
    // CRITICAL-2 (adversarial): an assert ACTION block can WRITE the const-init local.
    // `assert #0 (x) else c = 0;` writes `c` in the fail action — the write-scanner
    // must recurse into a `DeferredAssert`'s then/else statements (previously swallowed
    // by the `_ => false` arm → silently accepted). Stays E3009.
    assert!(loud(
        "module top;\n\
         logic x;\n\
         initial begin\n\
           x = 1'b1;\n\
           fork\n\
             begin\n\
               automatic int c = 5;\n\
               assert #0 (x) else c = 0;\n\
               #10 $display(\"%0d\", c);\n\
             end\n\
           join_none\n\
           #1 $finish;\n\
         end\n\
         endmodule\n"
    ));
}

#[test]
fn fork_nested_decl_init_call_writes_stays_loud() {
    // "Also fix": a NESTED-block decl-init `int z = f(c);` writes `c` via `f`'s inout
    // copy-out. The Block/Fork arm previously walked `stmts` but NOT nested `decls`, so
    // this decl-init write was missed. Stays E3009.
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
