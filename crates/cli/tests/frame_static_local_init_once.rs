//! A FRAMED subroutine's STATIC local initializer runs ONCE, not on every activation
//! (§2 Scoping, ROADMAP §5.2 row 2).
//!
//! ## What was wrong
//!
//! On the FRAME route (a hierarchical `u.t()`, every `function`, a `task automatic`)
//! `frames_body.rs`'s `lower_frame_func_body` / `lower_frame_task_body` called
//! `emit_frame_local_inits` INSIDE the body, with no lifetime gate, so a STATIC local
//! carrying an initializer was re-initialized on every call:
//!
//! ```text
//! function int f; int c = 100; c = c + 1; f = c; endfunction   // u.f() twice
//! ```
//!
//! returned `101 101` where both oracles return `101 102`. The nested-block half asked
//! the same wrong question — `stmt_main.rs`'s `let emit_block_inits = self.in_frame_body;`
//! asks "am I in a frame body" where IEEE 1800 §6.21 / §13.4.1 ask "is THIS declarator
//! automatic" — so a STATIC task's loop-body local re-ran its initializer every
//! iteration, printing `100` six times for the sequence both oracles print as
//! `100 101 102 103 104 105`.
//!
//! The storage was never the problem: `sim-engine/src/state/mod.rs`'s `frame_slot_auto`
//! already keeps a static slot in the persistent slab, which is why an initializer-FREE
//! static local retained correctly (`1 2 3`) throughout.
//!
//! ## The fix
//!
//! The emission point follows the declarator's EFFECTIVE lifetime — its own
//! `automatic`/`static` keyword when it carries one, else the subroutine's default
//! (`FuncMeta.is_automatic`, carried into lowering as `frame_body_auto`). An AUTOMATIC
//! declarator keeps the per-activation, per-block-entry emission unchanged. A STATIC one
//! is hoisted into `emit_frame_static_prologue`: a once-only region at the top of the
//! body behind a new frame-local flag slot (`reserve_frame_static_guard`'s `$sinit$`),
//! which lives in the same persistent static slab as the locals it guards. That is the
//! frame twin of the INLINE route's `first_call` gate in `hoist_inline_task_locals` —
//! which is exactly why the inline route was already right for this shape.
//!
//! An initializer that reads a net OUTSIDE the frame is NOT hoisted and keeps the
//! pre-slice per-activation emission, because the two oracles disagree about when a
//! static initializer observes module state (see `Oracles` below), and a third answer is
//! worse than either. A static initializer calling a net-writing function stays LOUD
//! (E3009), as it was.
//!
//! ## Oracles
//!
//! Every value below was measured three-way against iverilog 13 (`-g2012` + `vvp -n`)
//! and verilator 5.052 (`--binary --timing`). iverilog additionally emits one
//! `warning: Static variable initialization requires explicit lifetime in this context.`
//! per static-init declaration; it is a warning, not a rejection. The two oracles agree
//! on every value pinned here EXCEPT the two ordering cells:
//!
//! * `int n; initial n = 9;` read by a static local's initializer — iverilog prints
//!   `c=0` (the static initializer precedes the `initial`), verilator prints `c=9`.
//! * the same with a module-scope `int n = 9;` — iverilog `c=0`, verilator `c=9`.
//!
//! Those two keep vita's pre-slice answer (`c=9`), pinned here so the split is visible
//! if either side is ever revisited.
//!
//! One more family is 1-ORACLE: verilator refuses any design whose static local initializer
//! reads a FORMAL argument with `%Error-UNSUPPORTED: Static variable initializer`, so the
//! three `keeps_the_pre_slice_answer` / `hoists_nothing` cases below carry the iverilog 13
//! line only, and vita's pinned value is its PRE-slice one, not iverilog's.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fsli_{}_{n}", std::process::id()));
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

/// A clean run (exit 0) whose `$display` lines, in order, are exactly `want`.
///
/// The whole SEQUENCE is the assertion: this row is about how many times a value is
/// re-initialized, and a `contains` check on one line cannot see a repeat.
fn seq(src: &str, want: &[&str]) {
    let (o, code) = run(src);
    assert_eq!(code, Some(0), "expected a clean run:\n{o}");
    let got: Vec<&str> = o
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with('#'))
        .collect();
    assert_eq!(got, want, "display sequence mismatch in:\n{o}");
}

/// A run that fails elaboration with `code` in its diagnostics.
fn loud(src: &str, code: &str) {
    let (o, rc) = run(src);
    assert_eq!(rc, Some(1), "expected a loud rejection:\n{o}");
    assert!(o.contains(code), "expected {code:?} in:\n{o}");
}

// ─────────────────────────── the row: static, WITH initializer ───────────────────────────

#[test]
fn a_framed_function_static_local_initializes_once() {
    // both oracles: #101 #102 #103
    seq(
        "module m;\n\
           function int f; int c = 100; c = c + 1; f = c; endfunction\n\
         endmodule\n\
         module top; m u();\n\
           initial begin $display(\"#%0d\", u.f()); $display(\"#%0d\", u.f()); \
           $display(\"#%0d\", u.f()); #1 $finish; end\n\
         endmodule\n",
        &["#101", "#102", "#103"],
    );
}

#[test]
fn a_hierarchical_static_task_static_local_initializes_once() {
    // both oracles: #45 #46 #47
    seq(
        "module m;\n\
           task t; int a = 45; $display(\"#%0d\", a); a = a + 1; endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin u.t(); u.t(); u.t(); #1 $finish; end\n\
         endmodule\n",
        &["#45", "#46", "#47"],
    );
}

#[test]
fn an_explicit_task_static_keeps_its_initializer_once() {
    // both oracles: #45 #46 #47
    seq(
        "module m;\n\
           task static t; int a = 45; $display(\"#%0d\", a); a = a + 1; endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin u.t(); u.t(); u.t(); #1 $finish; end\n\
         endmodule\n",
        &["#45", "#46", "#47"],
    );
}

#[test]
fn a_function_void_static_local_initializes_once() {
    // both oracles: #101 #102 #103
    seq(
        "module m;\n\
           function void g; int c = 100; c = c + 1; $display(\"#%0d\", c); endfunction\n\
         endmodule\n\
         module top; m u();\n\
           initial begin u.g(); u.g(); u.g(); #1 $finish; end\n\
         endmodule\n",
        &["#101", "#102", "#103"],
    );
}

#[test]
fn a_same_module_function_call_takes_the_frame_route_and_is_fixed_too() {
    // A FUNCTION always routes `frame`, even called from its own module's process —
    // which is why the row saw a function carry the defect where the inlined task did
    // not. both oracles: #101 #102 #103
    seq(
        "module m;\n\
           function int f; int c = 100; c = c + 1; f = c; endfunction\n\
           initial begin $display(\"#%0d\", f()); $display(\"#%0d\", f()); \
           $display(\"#%0d\", f()); #1 $finish; end\n\
         endmodule\n",
        &["#101", "#102", "#103"],
    );
}

#[test]
fn a_labelled_block_local_static_initializer_runs_once() {
    // both oracles: #45 #46 #47
    seq(
        "module m;\n\
           task t; begin : b1 int a = 45; $display(\"#%0d\", a); a = a + 1; end endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin u.t(); u.t(); u.t(); #1 $finish; end\n\
         endmodule\n",
        &["#45", "#46", "#47"],
    );
}

#[test]
fn an_unlabelled_block_local_static_initializer_runs_once() {
    // both oracles: #45 #46 #47
    seq(
        "module m;\n\
           task t; begin int a = 45; $display(\"#%0d\", a); a = a + 1; end endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin u.t(); u.t(); u.t(); #1 $finish; end\n\
         endmodule\n",
        &["#45", "#46", "#47"],
    );
}

#[test]
fn a_static_initializer_reading_an_earlier_sibling_local_runs_once() {
    // The different-NAME control, wrong in the same way: both oracles
    // #7 #8 / #17 #108 / #27 #208.
    seq(
        "module m;\n\
           task t; int p = 7; int q = p + 1;\n\
             $display(\"#%0d\", p); $display(\"#%0d\", q); p = p + 10; q = q + 100;\n\
           endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin u.t(); u.t(); u.t(); #1 $finish; end\n\
         endmodule\n",
        &["#7", "#8", "#17", "#108", "#27", "#208"],
    );
}

#[test]
fn a_delay_in_the_body_does_not_change_the_once_rule() {
    // both oracles: #45 #46 #47
    seq(
        "module m;\n\
           task t; int a = 45; #1 $display(\"#%0d\", a); a = a + 1; endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin u.t(); u.t(); u.t(); #1 $finish; end\n\
         endmodule\n",
        &["#45", "#46", "#47"],
    );
}

#[test]
fn each_instance_keeps_its_own_static_local() {
    // both oracles: #45 #45 #46 #46 — per-INSTANCE static storage, not per-module.
    seq(
        "module m;\n\
           task t; int a = 45; $display(\"#%0d\", a); a = a + 1; endtask\n\
         endmodule\n\
         module top; m u1(); m u2();\n\
           initial begin u1.t(); u2.t(); u1.t(); u2.t(); #1 $finish; end\n\
         endmodule\n",
        &["#45", "#45", "#46", "#46"],
    );
}

#[test]
fn two_caller_modules_share_one_callee_instance_static() {
    // both oracles: #45 #46 #45 #46 — u and u2 are two instances, each retaining.
    seq(
        "module m;\n\
           task t; int a = 45; $display(\"#%0d\", a); a = a + 1; endtask\n\
         endmodule\n\
         module w(); endmodule\n\
         module top; m u(); m u2();\n\
           initial begin u.t(); u.t(); u2.t(); u2.t(); #1 $finish; end\n\
         endmodule\n",
        &["#45", "#46", "#45", "#46"],
    );
}

#[test]
fn a_four_state_packed_static_local_initializes_once() {
    // both oracles: #45 #46 #47
    seq(
        "module m;\n\
           task t; logic [7:0] a = 45; $display(\"#%0d\", a); a = a + 1; endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin u.t(); u.t(); u.t(); #1 $finish; end\n\
         endmodule\n",
        &["#45", "#46", "#47"],
    );
}

#[test]
fn a_four_state_integer_static_local_initializes_once() {
    // both oracles: #45 #46 #47
    seq(
        "module m;\n\
           task t; integer a = 45; $display(\"#%0d\", a); a = a + 1; endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin u.t(); u.t(); u.t(); #1 $finish; end\n\
         endmodule\n",
        &["#45", "#46", "#47"],
    );
}

#[test]
fn a_package_function_hits_the_same_frame_root() {
    // both oracles: #201 #202. The package FUNCTION frames like a module function.
    seq(
        "package pk;\n\
           function int f; int c = 200; c = c + 1; f = c; endfunction\n\
         endpackage\n\
         module top; import pk::*;\n\
           initial begin $display(\"#%0d\", f()); $display(\"#%0d\", f()); #1 $finish; end\n\
         endmodule\n",
        &["#201", "#202"],
    );
}

#[test]
fn a_static_task_loop_body_initializer_runs_once_across_every_iteration() {
    // both oracles: #100 #101 #102 #103 #104 #105 over two three-iteration calls.
    // §4.5.189's per-ENTRY block-local init rule is AUTOMATIC-only.
    seq(
        "module m;\n\
           task t; int k;\n\
             for (k = 0; k < 3; k = k + 1) begin int z = 100; $display(\"#%0d\", z); z = z + 1; end\n\
           endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin u.t(); u.t(); #1 $finish; end\n\
         endmodule\n",
        &["#100", "#101", "#102", "#103", "#104", "#105"],
    );
}

#[test]
fn two_init_bearing_sibling_block_locals_each_initialize_once() {
    // both oracles: #44 #55 #45 #56 — the `$blk$`-scoped pair (§4.5.480) keeps two
    // separate nets AND each keeps its own once-only initializer.
    seq(
        "module m;\n\
           task t;\n\
             begin : b1 int x = 44; $display(\"#%0d\", x); x = x + 1; end\n\
             begin : b2 int x = 55; $display(\"#%0d\", x); x = x + 1; end\n\
           endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin u.t(); u.t(); #1 $finish; end\n\
         endmodule\n",
        &["#44", "#55", "#45", "#56"],
    );
}

// ─────────────────────────── controls that MUST NOT move ───────────────────────────

#[test]
fn an_initializer_free_static_task_local_still_retains() {
    // both oracles: #1 #2 #3 — the storage was always right; only the emission moved.
    seq(
        "module m;\n\
           task t; int s; s = s + 1; $display(\"#%0d\", s); endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin u.t(); u.t(); u.t(); #1 $finish; end\n\
         endmodule\n",
        &["#1", "#2", "#3"],
    );
}

#[test]
fn an_initializer_free_static_function_local_still_retains() {
    // both oracles: #1 #2 #3
    seq(
        "module m;\n\
           function int f; int c; c = c + 1; f = c; endfunction\n\
         endmodule\n\
         module top; m u();\n\
           initial begin $display(\"#%0d\", u.f()); $display(\"#%0d\", u.f()); \
           $display(\"#%0d\", u.f()); #1 $finish; end\n\
         endmodule\n",
        &["#1", "#2", "#3"],
    );
}

#[test]
fn an_inlined_task_and_a_framed_function_without_initializers_are_unchanged() {
    // both oracles: #1 #2 (task, inlined) then #1 #2 (function, framed).
    seq(
        "module m;\n\
           task t; int s; s = s + 1; $display(\"#%0d\", s); endtask\n\
           function int f; int c; c = c + 1; f = c; endfunction\n\
           initial begin t(); t(); $display(\"#%0d\", f()); $display(\"#%0d\", f()); #1 $finish; end\n\
         endmodule\n",
        &["#1", "#2", "#1", "#2"],
    );
}

#[test]
fn an_automatic_task_initializer_free_local_still_resets_per_call() {
    // both oracles: #1 #1 #1 — a per-call window, nothing to retain.
    seq(
        "module m;\n\
           task automatic t; int s; s = s + 1; $display(\"#%0d\", s); endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin u.t(); u.t(); u.t(); #1 $finish; end\n\
         endmodule\n",
        &["#1", "#1", "#1"],
    );
}

#[test]
fn an_automatic_task_local_with_an_initializer_re_initializes_every_call() {
    // both oracles: #45 #45 #45 — the AUTOMATIC arm is untouched.
    seq(
        "module m;\n\
           task automatic t; int a = 45; $display(\"#%0d\", a); a = a + 1; endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin u.t(); u.t(); u.t(); #1 $finish; end\n\
         endmodule\n",
        &["#45", "#45", "#45"],
    );
}

#[test]
fn an_automatic_function_local_with_an_initializer_re_initializes_every_call() {
    // both oracles: #101 #101
    seq(
        "module m;\n\
           function automatic int f; int c = 100; c = c + 1; f = c; endfunction\n\
         endmodule\n\
         module top; m u();\n\
           initial begin $display(\"#%0d\", u.f()); $display(\"#%0d\", u.f()); #1 $finish; end\n\
         endmodule\n",
        &["#101", "#101"],
    );
}

#[test]
fn an_automatic_loop_body_initializer_still_re_runs_every_iteration() {
    // both oracles: #100 six times — §4.5.189's own cell, unmoved.
    seq(
        "module m;\n\
           task automatic t; int k;\n\
             for (k = 0; k < 3; k = k + 1) begin int z = 100; $display(\"#%0d\", z); z = z + 1; end\n\
           endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin u.t(); u.t(); #1 $finish; end\n\
         endmodule\n",
        &["#100", "#100", "#100", "#100", "#100", "#100"],
    );
}

#[test]
fn the_inlined_task_route_keeps_its_own_first_call_gate() {
    // both oracles: #45 #46 #47 — a same-module task call inlines, and
    // `hoist_inline_task_locals`' `first_call` already emitted the init once.
    seq(
        "module m;\n\
           task t; int a = 45; $display(\"#%0d\", a); a = a + 1; endtask\n\
           initial begin t(); t(); t(); #1 $finish; end\n\
         endmodule\n",
        &["#45", "#46", "#47"],
    );
}

#[test]
fn an_inlined_task_with_a_delay_keeps_its_once_only_initializer() {
    // both oracles: #45 #46 #47 — a `#` does NOT frame a task (§4.5.482).
    seq(
        "module m;\n\
           task t; int a = 45; #1 $display(\"#%0d\", a); a = a + 1; endtask\n\
           initial begin t(); t(); t(); #1 $finish; end\n\
         endmodule\n",
        &["#45", "#46", "#47"],
    );
}

#[test]
fn an_imported_package_task_inlines_and_keeps_its_once_only_initializer() {
    // both oracles: #101 #102 #103
    seq(
        "package pk;\n\
           task t; int c = 100; c = c + 1; $display(\"#%0d\", c); endtask\n\
         endpackage\n\
         module top; import pk::*;\n\
           initial begin t(); t(); t(); #1 $finish; end\n\
         endmodule\n",
        &["#101", "#102", "#103"],
    );
}

#[test]
fn a_module_level_variable_initializer_is_unaffected() {
    // both oracles: #6 #7 — the module-scope t0 init already ran once; this is the
    // contrast that shows the defect was the FRAME emission point, not the rule.
    seq(
        "module m;\n\
           int x = 5;\n\
           task t; x = x + 1; $display(\"#%0d\", x); endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin u.t(); u.t(); #1 $finish; end\n\
         endmodule\n",
        &["#6", "#7"],
    );
}

#[test]
fn a_static_initializer_calling_a_net_writing_function_stays_loud() {
    // Both oracles run the initializer once (iverilog a=1 n=0, verilator a=1 n=1 — they
    // split on the readback), and vita keeps the pre-slice E3009: the callee assigns a
    // net outside its own frame. Hoisting must not turn a loud into a value.
    loud(
        "module m;\n\
           int n;\n\
           function int f2; n = 1; f2 = 1; endfunction\n\
           task t; int a = f2(); $display(\"#%0d\", a); endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin u.t(); #1 $finish; end\n\
         endmodule\n",
        "VITA-E3009",
    );
}

// ─────────────────────────── ordering cells ───────────────────────────

#[test]
fn a_static_initializer_reading_a_sibling_declared_just_above_it_agrees() {
    // both oracles: #5 #6 — declaration order inside one activation, no split.
    seq(
        "module m;\n\
           task t; int a = 5; int b = a + 1; $display(\"#%0d\", a); $display(\"#%0d\", b); endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin u.t(); #1 $finish; end\n\
         endmodule\n",
        &["#5", "#6"],
    );
}

#[test]
fn a_static_initializer_reading_a_module_net_keeps_the_pre_slice_answer() {
    // ORACLES SPLIT: `int n; initial n = 9;` read by a static local initializer is
    // iverilog `#0 #9` twice and verilator `#9 #9` twice. vita keeps its pre-slice
    // per-activation emission (`#9 #9` twice) rather than inventing a third answer.
    seq(
        "module m;\n\
           int n;\n\
           initial n = 9;\n\
           task t; int c = n; $display(\"#%0d\", c); $display(\"#%0d\", n); endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin #1 u.t(); u.t(); #1 $finish; end\n\
         endmodule\n",
        &["#9", "#9", "#9", "#9"],
    );
}

#[test]
fn a_static_initializer_reading_a_module_scope_initialized_net_keeps_the_pre_slice_answer() {
    // ORACLES SPLIT the same way for a module-scope `int n = 9;`: iverilog `#0 #0`,
    // verilator `#9 #9`. vita keeps `#9 #9`.
    seq(
        "module m;\n\
           int n = 9;\n\
           task t; int c = n; $display(\"#%0d\", c); endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin #1 u.t(); u.t(); #1 $finish; end\n\
         endmodule\n",
        &["#9", "#9"],
    );
}

// ─────────── formals: a frame with one declined initializer hoists nothing ───────────

#[test]
fn a_static_initializer_reading_a_formal_argument_keeps_the_pre_slice_answer() {
    // A FORMAL has no value until a call binds it, so a static initializer reading one is
    // not a once-only value. Hoisting it read the FIRST call's argument, which is a third
    // answer: `#6 #7 / D=31 D=32`. Declined, so the shape keeps the per-activation emission
    // and this is byte-identical to the pre-slice binary.
    //
    // 1-oracle: iverilog 13 prints `#1 #2 / D=1 D=2` (both initializers run at t0, with the
    // formal still 0); verilator 5.052 refuses the design outright with
    // `%Error-UNSUPPORTED: Static variable initializer`. vita is neither: it pins its own
    // pre-slice per-activation answer, which this slice must not move.
    seq(
        "module m;\n\
           function int f(int k); int c = k; c = c + 1; return c; endfunction\n\
           task t(input int k); int d = k * 10; d = d + 1; $display(\"#%0d\", d); endtask\n\
         endmodule\n\
         module top; m u();\n\
           initial begin $display(\"#%0d\", u.f(5)); $display(\"#%0d\", u.f(7));\n\
             u.t(3); u.t(4); #1 $finish; end\n\
         endmodule\n",
        &["#6", "#8", "#31", "#41"],
    );
}

#[test]
fn a_static_initializer_chain_through_a_formal_keeps_the_pre_slice_answer() {
    // The TRANSITIVE case: `d` does not name the formal, but `c` does, so hoisting `d`
    // alone would read `c`'s default and produce a third answer. `c` is declined, and the
    // decline propagates to `d`.
    //
    // 1-oracle: iverilog 13 prints `#1 #2 / #2 / #2 #3 / #3`; verilator refuses
    // (`%Error-UNSUPPORTED: Static variable initializer`). Pinned at vita's pre-slice value.
    seq(
        "module m;\n\
           function int f(int k); int c = k; int d = c + 1;\n\
             c = c + 1; d = d + 1; $display(\"#%0d\", c); $display(\"#%0d\", d); return d;\n\
           endfunction\n\
         endmodule\n\
         module top; m u();\n\
           initial begin $display(\"#%0d\", u.f(5)); $display(\"#%0d\", u.f(7)); #1 $finish; end\n\
         endmodule\n",
        &["#6", "#7", "#7", "#8", "#9", "#9"],
    );
}

#[test]
fn a_mixed_frame_with_one_declined_initializer_hoists_nothing() {
    // ALL-OR-NOTHING per frame. `int a = 5;` is admissible on its own and `int b = a + k;`
    // is not. Hoisting only `a` gives `#7 #9` — `b`'s re-run initializer reads `a` AFTER the
    // body mutated it — which is neither oracle nor the pre-slice answer. The whole frame
    // stays per-activation instead.
    //
    // 1-oracle: iverilog 13 prints `#6 #7` (both initializers once, at t0); verilator refuses
    // (`%Error-UNSUPPORTED: Static variable initializer`). Pinned at vita's pre-slice `#7 #8`.
    seq(
        "module m;\n\
           function int f(int k); int a = 5; int b = a + k;\n\
             a = a + 1; b = b + 1; return b;\n\
           endfunction\n\
         endmodule\n\
         module top; m u();\n\
           initial begin $display(\"#%0d\", u.f(1)); $display(\"#%0d\", u.f(2)); #1 $finish; end\n\
         endmodule\n",
        &["#7", "#8"],
    );
}

#[test]
fn a_sibling_chain_without_a_formal_still_hoists() {
    // The control for the three above: the SAME two-declarator chain with no formal in it
    // is fully admitted and hoisted, so both initializers run once and both locals retain.
    // Both oracles: `#6 #7 / #7` then `#7 #8 / #8`.
    seq(
        "module m;\n\
           function int f; int a = 5; int b = a + 1;\n\
             a = a + 1; b = b + 1; $display(\"#%0d\", a); $display(\"#%0d\", b); return b;\n\
           endfunction\n\
         endmodule\n\
         module top; m u();\n\
           initial begin $display(\"#%0d\", u.f()); $display(\"#%0d\", u.f()); #1 $finish; end\n\
         endmodule\n",
        &["#6", "#7", "#7", "#7", "#8", "#8"],
    );
}

// ───────── granularity: an admitted declarator is hoisted even beside a declined one ─────────

#[test]
fn an_admitted_declarator_retains_beside_a_declined_sibling() {
    // `int a = 15;` is admitted; `int b = outside;` reads a module net and is declined. The
    // admitted one must still be hoisted: BOTH oracles retain `a` across the two calls
    // (iverilog 13 `mix 16001 17002`, verilator 5.052 `mix 16051 17052` — they differ only
    // on `b`'s initial value, never on whether `a` retains), and a frame-wide decline left
    // it at `16051 16051`, which is neither oracle.
    //
    // The thousands digit is what this slice fixes and what is pinned: `16` then `17`. The
    // hundreds/units digit is `b`, still on the pre-slice per-activation path because the
    // oracles SPLIT on what a static initializer reading module state observes (`b` starts
    // at 0 on iverilog and at 50 on verilator) — an open row, not this slice's.
    // `fpure` (both declarators admitted) matches both oracles exactly.
    seq(
        "module top;\n\
           int outside = 50;\n\
           function int fmix(); int a = 15; int b = outside; a=a+1; b=b+1; fmix = a*1000+b; endfunction\n\
           function int fpure(); int a = 15; int b = 20; a=a+1; b=b+1; fpure = a*1000+b; endfunction\n\
           initial begin\n\
             $display(\"#%0d\", fmix()); $display(\"#%0d\", fmix());\n\
             $display(\"#%0d\", fpure()); $display(\"#%0d\", fpure()); #1 $finish;\n\
           end\n\
         endmodule\n",
        &["#16051", "#17051", "#16021", "#17022"],
    );
}

#[test]
fn five_admitted_kinds_retain_beside_one_declined_sibling() {
    // A parameter, an enum label, a real and a string initializer are all admitted; only
    // `int c = $bits(wide);` is declined (a system function is not on the allowlist). Each
    // admitted declarator keeps its own once-only initializer.
    //
    // Both oracles: `4819 5921 7023`. vita is `4819 5920 7021` — the five admitted ones
    // retain (that is the whole `+1102`-per-call step) and `c` alone is one short each call,
    // because it re-initializes. A frame-wide decline printed `4819 4819 4819`.
    seq(
        "module top;\n\
           typedef enum int { EL = 7 } e_t;\n\
           parameter int PP = 3;\n\
           logic [11:0] wide;\n\
           function int f();\n\
             int a = PP; int b = EL; int c = $bits(wide); real r = 2.5; string s = \"ab\";\n\
             a = a + 1; b = b + 1; c = c + 1; r = r + 1.0;\n\
             f = a*1000 + b*100 + c + int'(r) + s.len();\n\
           endfunction\n\
           initial begin $display(\"#%0d\", f()); $display(\"#%0d\", f());\n\
             $display(\"#%0d\", f()); #1 $finish; end\n\
         endmodule\n",
        &["#4819", "#5920", "#7021"],
    );
}

#[test]
fn an_admitted_declarator_retains_beside_both_a_net_read_and_a_formal_read() {
    // Three declarators: `int adm = 10 + 5;` admitted, `int dec_out = outside;` declined
    // (module net), `int dec_arg = arg;` declined (formal). `adm` retains — the `10000`
    // place steps 16 -> 17.
    //
    // 1-oracle: iverilog 13 `160101 170202`; verilator 5.052 refuses the design
    // (`%Error-UNSUPPORTED: Static variable initializer`) because of the formal read. vita
    // is `165102 175102`: `adm` retains, the two declined ones do not. A frame-wide decline
    // printed `165102 165102`.
    seq(
        "module top;\n\
           int outside = 50;\n\
           function int f(input int arg);\n\
             int adm = 10 + 5; int dec_out = outside; int dec_arg = arg;\n\
             adm = adm + 1; dec_out = dec_out + 1; dec_arg = dec_arg + 1;\n\
             f = adm*10000 + dec_out*100 + dec_arg;\n\
           endfunction\n\
           initial begin $display(\"#%0d\", f(1)); $display(\"#%0d\", f(1)); #1 $finish; end\n\
         endmodule\n",
        &["#165102", "#175102"],
    );
}

#[test]
fn a_retained_counter_beside_a_net_read_index() {
    // The realistic shape. `int count = 0;` must retain across calls; `int idx =
    // outside_net;` is declined. vita prints `#107 #207` — `count` steps 1 -> 2 — which is
    // verilator 5.052's line exactly; iverilog 13 prints `100 200` (its static initializer
    // sees `outside_net` as 0), the same oracle split on the declined declarator's value.
    // Both oracles agree `count` retains, and the pre-slice binary printed `107 107`.
    seq(
        "module top;\n\
           int outside_net = 7;\n\
           function int f(); int count = 0; int idx = outside_net; count++; f = count*100 + idx; endfunction\n\
           initial begin $display(\"#%0d\", f()); $display(\"#%0d\", f()); #1 $finish; end\n\
         endmodule\n",
        &["#107", "#207"],
    );
}

#[test]
fn a_declined_initializer_reading_an_admitted_local_declines_the_whole_frame() {
    // The ONE shape the frame-wide escape hatch exists for. `int a = 15;` is admissible and
    // `int b = a + outside;` is not; hoisting `a` alone would let `b`'s per-activation
    // re-run read `a` AFTER the body mutated it, so the frame keeps the pre-slice emission
    // whole and this is byte-identical to the pre-slice binary (`16066 16066`).
    //
    // iverilog 13 `16016 17017`, verilator 5.052 `16066 17067` — both retain, and vita does
    // not. Measured on a build with the frame-wide return removed, THIS design prints
    // `16066 17067`, verilator verbatim; so for a net-read sibling the hatch is a
    // CONSERVATIVE choice, not a correctness one. It is kept because the same rule covers
    // the FORMAL-read sibling (`int b = a + k`), where the same mutant measured `7 9`, an
    // answer neither oracle gives. Narrowing it needs the open oracle-split row resolved.
    seq(
        "module top;\n\
           int outside = 50;\n\
           function int f(); int a = 15; int b = a + outside; a=a+1; b=b+1; f = a*1000+b; endfunction\n\
           initial begin $display(\"#%0d\", f()); $display(\"#%0d\", f()); #1 $finish; end\n\
         endmodule\n",
        &["#16066", "#16066"],
    );
}

// ───── an UNSELECTED declarator in a `$blk$`-scoped block is claimed by exactly one emitter ─────

#[test]
fn an_unselected_declarator_beside_a_scoped_sibling_keeps_its_initializer() {
    // `int x = 7;` collides with a second block's `x`, so it earns a `$blk$<lo>` scope;
    // `int y = x + 1;` in the same block collides with nothing and is UNSELECTED. The
    // admission pass and the per-activation emitter once asked the same name-resolving
    // predicate under DIFFERENT prefixes — admission under the per-declaration
    // `block_local_scope_prefix` (which omits the segment for an unselected declaration) and
    // emission under the `Stmt::Block` wrap (which applies it to the whole body). `x` was
    // invisible to the first and visible to the second, so `y` was claimed by NEITHER and
    // its initializer was never lowered: `17099 27099`, wrong on the first call where even
    // the pre-slice binary was right.
    //
    // Both oracles: `17107 27107`. Admission now resolves under the wrap prefix, so `y` is
    // admissible (it reads `x`, which is hoisted, and the prologue emits in declaration
    // order), and the skip is membership in what the prologue actually emitted.
    seq(
        "module top;\n\
           function int f;\n\
             int r;\n\
             begin int x = 7; int y = x + 1; x = x + 10; r = x*1000 + y; end\n\
             begin int x = 99; r = r + x; end\n\
             f = r;\n\
           endfunction\n\
           initial begin $display(\"#%0d\", f()); $display(\"#%0d\", f()); #1 $finish; end\n\
         endmodule\n",
        &["#17107", "#27107"],
    );
}

#[test]
fn an_unselected_declarator_that_reads_nothing_still_hoists_and_retains() {
    // The same shape with `int y = 8;`, which reads no sibling. It must hoist and retain —
    // both oracles `17107 27107` — so the fix for the reading case cannot be "decline every
    // unselected declarator in a scoped block".
    seq(
        "module top;\n\
           function int f;\n\
             int r;\n\
             begin int x = 7; int y = 8; x = x + 10; r = x*1000 + y; end\n\
             begin int x = 99; r = r + x; end\n\
             f = r;\n\
           endfunction\n\
           initial begin $display(\"#%0d\", f()); $display(\"#%0d\", f()); #1 $finish; end\n\
         endmodule\n",
        &["#17107", "#27107"],
    );
}

#[test]
fn the_same_pair_without_a_name_collision_is_unaffected() {
    // The control that isolates the mechanism: rename the second block's `x` to `z` and no
    // block earns a `$blk$` scope at all, so the two prefixes cannot diverge. Both oracles
    // `17107 27107`.
    seq(
        "module top;\n\
           function int f;\n\
             int r;\n\
             begin int x = 7; int y = x + 1; x = x + 10; r = x*1000 + y; end\n\
             begin int z = 99; r = r + z; end\n\
             f = r;\n\
           endfunction\n\
           initial begin $display(\"#%0d\", f()); $display(\"#%0d\", f()); #1 $finish; end\n\
         endmodule\n",
        &["#17107", "#27107"],
    );
}

#[test]
fn two_scoped_blocks_each_with_an_unselected_reader_all_retain() {
    // Both blocks declare `x` (so both are scoped) and each carries its own unselected `y`
    // reading its own block's `x`. Every one of the four declarators is hoisted, under its
    // own block's prefix, and `y` resolves to ITS block's `x` — not the other one's.
    // Both oracles: `18299 28499`.
    seq(
        "module top;\n\
           function int f;\n\
             int r;\n\
             begin int x = 7;  int y = x + 1; x = x + 10; r = x*1000 + y; end\n\
             begin int x = 99; int y = x + 2; x = x + 20; r = r + x*10 + y; end\n\
             f = r;\n\
           endfunction\n\
           initial begin $display(\"#%0d\", f()); $display(\"#%0d\", f()); #1 $finish; end\n\
         endmodule\n",
        &["#18299", "#28499"],
    );
}
