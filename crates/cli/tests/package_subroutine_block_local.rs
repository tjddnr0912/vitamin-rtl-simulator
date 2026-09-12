//! Same-named sibling block-locals inside a PACKAGE `task`/`function` body are two
//! variables (§2 Scoping, ROADMAP §5.2 row 3).
//!
//! ## What was wrong
//!
//! §4.5.480/482 gave a MODULE-declared subroutine body's block-locals the `$blk$<lo>`
//! scoping a module process's get, by feeding `for_each_subroutine_body(&module.body,
//! ..)` into `compute_scoped_block_locals`. A PACKAGE routine is not in `module.body`:
//! `elaborate_package` clones its AST into `pkg_funcs`/`pkg_tasks`, and
//! `apply_import_routines` later injects that clone into the CALLER MODULE's
//! `func_table`/`task_table`, where the module's own reservers lower it. The
//! classification, however, ran BEFORE that injection and only over `module.body`, so a
//! package routine's block spans were never in `scoped_block_locals`, no `$blk$`
//! segment existed, and both declarators flattened onto one bare-named net:
//!
//! ```text
//! package pk;
//!   task t;
//!     begin : b1 int x = 44; $display("A=%0d", x); end
//!     begin : b2 int x;      $display("B=%0d", x); end
//!   endtask
//! endpackage
//! ```
//!
//! printed `A=44 B=44`, where both oracles print `A=44 B=0`. The init+init twin
//! printed `A=55 B=55` (the second initializer overwriting the shared net before the
//! first block ran) where both oracles print `A=44 B=55`, and a package function used
//! in a continuous assign read `W=89` where both oracles read `W=45`.
//!
//! ## The fix
//!
//! `compute_scoped_block_locals` gains an `extra_bodies` parameter: subroutine bodies
//! lowered under this module that are not reachable from `module.body`. A new step
//! (3.6a) in `elaborate_instance`, placed right after the import-routine step that
//! binds them, redoes the classification with every `rtn_pkg`-keyed routine body fed
//! to the SAME `gather_auto_block_locals` walk with the SAME arguments the module's
//! own subroutine feed passes (`admit_static_plain = true`). No new walker and no new
//! `AdmitReason`. `rtn_pkg` is the exact key set, so a module-local routine is never
//! fed twice — a second feed of one body would count each declaring span twice and
//! make a lone declaration look like a colliding pair.
//!
//! The names set stays the CALLER MODULE's declared names, not the package's. A
//! package-level variable is not a net of this module, so a sibling block-local
//! shadowing one is admitted and scoped; that is what both oracles say
//! (`a_sibling_pair_shadowing_a_package_variable`, where the package variable also
//! keeps its own value).
//!
//! Step (3.6a) also runs `check_block_local_scope_leaks` on the same bodies — the gate
//! a module-declared routine gets in step (3.5). Without it the nested shape (an outer
//! block-local read after an inner same-named, initializer-free declaration) was
//! SILENT in a package and LOUD in the module twin.
//!
//! The scoped call spelling `pk::g()` with no `import` binds its callee through
//! `inject_pkg_callees` during body lowering, long after step (3.6a), so it kept the
//! pre-existing flattened value until §2 Scoping queue row 1 fed that body to the same
//! joint gather at the injection funnel (`Elaborator::feed_scoped_block_locals`).
//! `a_scoped_call_spelling_is_covered` here pins the oracle value; the full census of
//! that spelling is `crates/cli/tests/pkg_scoped_call_block_local.rs`.
//!
//! ## Oracles
//!
//! Every value here was measured three-way against iverilog 13 (`-g2012` + `vvp -n`)
//! and verilator 5.x (`--binary --timing`); both agree on every line pinned below.
//! iverilog emits an advisory "Static variable initialization requires explicit
//! lifetime in this context." for these designs and still prints the values.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pkgbl_{}_{n}", std::process::id()));
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

/// A clean run (exit 0) whose output contains every `want` line, verbatim as observed.
fn lines(src: &str, want: &[&str]) {
    let (o, code) = run(src);
    assert_eq!(code, Some(0), "expected a clean run:\n{o}");
    for w in want {
        assert!(o.contains(w), "expected {w:?} in:\n{o}");
    }
}

/// A clean run whose output contains `want` and does NOT contain any of `absent`.
fn lines_without(src: &str, want: &[&str], absent: &[&str]) {
    let (o, code) = run(src);
    assert_eq!(code, Some(0), "expected a clean run:\n{o}");
    for w in want {
        assert!(o.contains(w), "expected {w:?} in:\n{o}");
    }
    for a in absent {
        assert!(!o.contains(a), "did NOT expect {a:?} in:\n{a:?} / {o}");
    }
}

/// A run that must FAIL with the given diagnostic CODE (not its wording).
fn loud(src: &str, code_str: &str) {
    let (o, code) = run(src);
    assert_ne!(code, Some(0), "expected a loud refusal:\n{o}");
    assert!(o.contains(code_str), "expected {code_str:?} in:\n{o}");
}

// ---------------------------------------------------------------- fixed cells

/// `task`, one initializer-bearing sibling beside an initializer-free one.
/// Oracles: `A=44 B=0`. Was `A=44 B=44`.
#[test]
fn a_package_task_init_then_plain_sibling() {
    lines_without(
        "package pk;\n  task t;\n    begin : b1 int x = 44; $display(\"A=%0d\", x); end\n\
             begin : b2 int x; $display(\"B=%0d\", x); end\n  endtask\nendpackage\n\
         module top; import pk::*; initial begin t(); $finish; end endmodule\n",
        &["A=44", "B=0"],
        &["B=44"],
    );
}

/// `task`, BOTH siblings initialized. Oracles: `A=44 B=55`. Was `A=55 B=55`.
#[test]
fn a_package_task_two_initialized_siblings() {
    lines_without(
        "package pk;\n  task t;\n    begin : b1 int x = 44; $display(\"A=%0d\", x); end\n\
             begin : b2 int x = 55; $display(\"B=%0d\", x); end\n  endtask\nendpackage\n\
         module top; import pk::*; initial begin t(); $finish; end endmodule\n",
        &["A=44", "B=55"],
        &["A=55"],
    );
}

/// `task`, NEITHER sibling initialized; the second assigns before reading.
/// Oracles: `A=0 B=9`. This was already correct at HEAD (by cancellation — the first
/// block's read precedes the second block's write) and must not move.
#[test]
fn a_package_task_two_plain_siblings() {
    lines(
        "package pk;\n  task t;\n    begin : b1 int x; $display(\"A=%0d\", x); end\n\
             begin : b2 int x; x = 9; $display(\"B=%0d\", x); end\n  endtask\nendpackage\n\
         module top; import pk::*; initial begin t(); $finish; end endmodule\n",
        &["A=0", "B=9"],
    );
}

/// THREE siblings, 44 / 55 / initializer-free. Oracles: `A=44 B=55 C=0`.
/// Was `A=55 B=55 C=55`.
#[test]
fn a_package_task_three_siblings() {
    lines_without(
        "package pk;\n  task t;\n    begin : b1 int x = 44; $display(\"A=%0d\", x); end\n\
             begin : b2 int x = 55; $display(\"B=%0d\", x); end\n\
             begin : b3 int x; $display(\"C=%0d\", x); end\n  endtask\nendpackage\n\
         module top; import pk::*; initial begin t(); $finish; end endmodule\n",
        &["A=44", "B=55", "C=0"],
        &["C=55"],
    );
}

/// `task automatic`, init + initializer-free. Oracles: `A=44 B=0`. Was `A=44 B=44`.
/// The queue row's claim that "the automatic twin is correct" is refuted by this cell;
/// the automatic init+init twin below IS the one that was already correct.
#[test]
fn an_automatic_package_task_init_then_plain_sibling() {
    lines_without(
        "package pk;\n  task automatic t;\n    begin : b1 int x = 44; $display(\"A=%0d\", x); end\n\
             begin : b2 int x; $display(\"B=%0d\", x); end\n  endtask\nendpackage\n\
         module top; import pk::*; initial begin t(); $finish; end endmodule\n",
        &["A=44", "B=0"],
        &["B=44"],
    );
}

/// `task automatic`, BOTH siblings initialized. Oracles: `A=44 B=55`. Correct at HEAD;
/// a must-not-move control for the fix above.
#[test]
fn an_automatic_package_task_two_initialized_siblings_stays_correct() {
    lines(
        "package pk;\n  task automatic t;\n    begin : b1 int x = 44; $display(\"A=%0d\", x); end\n\
             begin : b2 int x = 55; $display(\"B=%0d\", x); end\n  endtask\nendpackage\n\
         module top; import pk::*; initial begin t(); $finish; end endmodule\n",
        &["A=44", "B=55"],
    );
}

/// `function int`, init + initializer-free. Oracles: `A=44 B=0`. Was `A=44 B=44`.
#[test]
fn a_package_function_init_then_plain_sibling() {
    lines_without(
        "package pk;\n  function int f;\n    begin : b1 int x = 44; $display(\"A=%0d\", x); end\n\
             begin : b2 int x; $display(\"B=%0d\", x); end\n    f = 0;\n  endfunction\nendpackage\n\
         module top; import pk::*; int z; initial begin z = f(); $finish; end endmodule\n",
        &["A=44", "B=0"],
        &["B=44"],
    );
}

/// `function automatic int`, init + initializer-free. Oracles: `A=44 B=0`.
/// Was `A=44 B=44`.
#[test]
fn an_automatic_package_function_init_then_plain_sibling() {
    lines_without(
        "package pk;\n  function automatic int f;\n\
             begin : b1 int x = 44; $display(\"A=%0d\", x); end\n\
             begin : b2 int x; $display(\"B=%0d\", x); end\n    f = 0;\n  endfunction\nendpackage\n\
         module top; import pk::*; int z; initial begin z = f(); $finish; end endmodule\n",
        &["A=44", "B=0"],
        &["B=44"],
    );
}

/// Siblings with DIFFERENT names are not a colliding pair. Oracles: `A=44 B=0`.
/// Correct at HEAD; must not move.
#[test]
fn a_package_task_with_differently_named_locals_stays_correct() {
    lines(
        "package pk;\n  task t;\n    begin : b1 int p = 44; $display(\"A=%0d\", p); end\n\
             begin : b2 int q; $display(\"B=%0d\", q); end\n  endtask\nendpackage\n\
         module top; import pk::*; initial begin t(); $finish; end endmodule\n",
        &["A=44", "B=0"],
    );
}

/// A sibling block-local SHADOWING a package-level variable. Oracles: `A=44 B=0 P=3` —
/// the two siblings are distinct AND the package variable keeps its own value. This is
/// the cell that decides the names set: it matches only when `module_names` stays the
/// CALLER MODULE's declared names (a package variable is not among them, so the pair
/// is admitted and scoped). Was `A=44 B=44 P=3`.
#[test]
fn a_sibling_pair_shadowing_a_package_variable() {
    lines_without(
        "package pk;\n  int x = 3;\n  task t;\n\
             begin : b1 int x = 44; $display(\"A=%0d\", x); end\n\
             begin : b2 int x; $display(\"B=%0d\", x); end\n  endtask\nendpackage\n\
         module top; import pk::*;\n  initial begin t(); $display(\"P=%0d\", pk::x); $finish; end\n\
         endmodule\n",
        &["A=44", "B=0", "P=3"],
        &["B=44"],
    );
}

/// The SAME package task called from a SECOND module. Each instance classifies the
/// injected body itself, so both see the scoped pair. Oracles: `A=44 B=0` twice.
/// Was `A=44 B=44` in both.
#[test]
fn a_package_task_called_from_two_modules() {
    lines_without(
        "package pk;\n  task t;\n    begin : b1 int x = 44; $display(\"A=%0d\", x); end\n\
             begin : b2 int x; $display(\"B=%0d\", x); end\n  endtask\nendpackage\n\
         module m1; import pk::*; initial begin $display(\"m1\"); t(); end endmodule\n\
         module top; import pk::*; m1 u1();\n\
           initial begin $display(\"top\"); t(); #1 $finish; end\nendmodule\n",
        &["m1", "top", "A=44", "B=0"],
        &["B=44"],
    );
}

/// The same package task called TWICE from one module. Oracles: `A=44 B=0` both times.
/// Was `A=44 B=44` both times.
#[test]
fn a_package_task_called_twice() {
    let (o, code) = run(
        "package pk;\n  task t;\n    begin : b1 int x = 44; $display(\"A=%0d\", x); end\n\
             begin : b2 int x; $display(\"B=%0d\", x); end\n  endtask\nendpackage\n\
         module top; import pk::*; initial begin t(); t(); $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0), "expected a clean run:\n{o}");
    assert_eq!(o.matches("A=44").count(), 2, "two calls expected:\n{o}");
    assert_eq!(o.matches("B=0").count(), 2, "two calls expected:\n{o}");
    assert!(!o.contains("B=44"), "leftover sibling value:\n{o}");
}

/// A package task whose two siblings are BOTH initialized, called TWICE. Both oracles
/// print `A=44 B=55` on BOTH calls: a static local with an initializer is initialized
/// once, and the retained values here are the same numbers. This is the cell that
/// touches the per-activation re-initialization class (§2, the `frames_body.rs`
/// `emit_frame_local_inits` slice); it is pinned on BOTH calls because vita already
/// matches the oracles on both.
#[test]
fn a_package_task_with_two_initialized_siblings_called_twice() {
    let (o, code) = run(
        "package pk;\n  task t;\n    begin : b1 int x = 44; $display(\"A=%0d\", x); end\n\
             begin : b2 int x = 55; $display(\"B=%0d\", x); end\n  endtask\nendpackage\n\
         module top; import pk::*; initial begin t(); t(); $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0), "expected a clean run:\n{o}");
    assert_eq!(o.matches("A=44").count(), 2, "two calls expected:\n{o}");
    assert_eq!(o.matches("B=55").count(), 2, "two calls expected:\n{o}");
    assert!(!o.contains("A=55"), "first block read the sibling:\n{o}");
}

/// A package FUNCTION driving a CONTINUOUS ASSIGN. Oracles: `W=45`. Was `W=89` — the
/// second block's `x` read the first block's 44 and added it again.
#[test]
fn a_package_function_in_a_continuous_assign() {
    lines_without(
        "package pk;\n  function int g(input int i);\n\
             begin : b1 int x = 44; g = x + i; end\n\
             begin : b2 int x; g = g + x; end\n  endfunction\nendpackage\n\
         module top; import pk::*; wire [31:0] w = g(1);\n\
           initial begin #1 $display(\"W=%0d\", w); $finish; end\nendmodule\n",
        &["W=45"],
        &["W=89"],
    );
}

/// An EXPLICIT `import pk::t;` reaches the same step (3.6a). Oracles: `A=44 B=0`.
/// Was `A=44 B=44`.
#[test]
fn an_explicitly_imported_package_task() {
    lines_without(
        "package pk;\n  task t;\n    begin : b1 int x = 44; $display(\"A=%0d\", x); end\n\
             begin : b2 int x; $display(\"B=%0d\", x); end\n  endtask\nendpackage\n\
         module top; import pk::t; initial begin t(); $finish; end endmodule\n",
        &["A=44", "B=0"],
        &["B=44"],
    );
}

/// NESTED, both declarations initialized: the inner block shadows the outer, the outer
/// read after the block is the outer's own value. Oracles: `I=8 O=7`. Was `I=8 O=8`.
#[test]
fn a_nested_package_local_pair_both_initialized() {
    lines_without(
        "package pk;\n  task t;\n    begin : o1 int x = 7;\n\
               begin : i1 int x = 8; $display(\"I=%0d\", x); end\n\
               $display(\"O=%0d\", x);\n    end\n  endtask\nendpackage\n\
         module top; import pk::*; initial begin t(); $finish; end endmodule\n",
        &["I=8", "O=7"],
        &["O=8"],
    );
}

/// NESTED with an initializer-free INNER declaration. Oracles: `I=0 O=7`; vita cannot
/// resolve the outer read past the flat table, so this must be LOUD, exactly as the
/// MODULE twin below already is. Was SILENT `I=7 O=7`. The CODE is pinned, not the
/// wording.
#[test]
fn a_nested_package_local_with_a_plain_inner_is_loud() {
    loud(
        "package pk;\n  task t;\n    begin : o1 int x = 7;\n\
               begin : i1 int x; $display(\"I=%0d\", x); end\n\
               $display(\"O=%0d\", x);\n    end\n  endtask\nendpackage\n\
         module top; import pk::*; initial begin t(); $finish; end endmodule\n",
        "VITA-E3009",
    );
}

// ------------------------------------------------------- must-not-move controls

/// The MODULE twin of the first cell: already correct at HEAD (§4.5.480/482), and the
/// module feed is untouched by this slice.
#[test]
fn the_module_twin_stays_correct() {
    lines(
        "module top;\n  task t;\n    begin : b1 int x = 44; $display(\"A=%0d\", x); end\n\
             begin : b2 int x; $display(\"B=%0d\", x); end\n  endtask\n\
           initial begin t(); $finish; end\nendmodule\n",
        &["A=44", "B=0"],
    );
}

/// The MODULE twin of the nested both-initialized cell: correct at HEAD.
#[test]
fn the_module_nested_twin_stays_correct() {
    lines(
        "module top;\n  task t;\n    begin : o1 int x = 7;\n\
               begin : i1 int x = 8; $display(\"I=%0d\", x); end\n\
               $display(\"O=%0d\", x);\n    end\n  endtask\n\
           initial begin t(); $finish; end\nendmodule\n",
        &["I=8", "O=7"],
    );
}

/// The MODULE twin of the nested plain-inner cell, in a task and in a process: LOUD at
/// HEAD and must stay loud. The code is pinned, not the wording.
#[test]
fn the_module_nested_plain_inner_twins_stay_loud() {
    loud(
        "module top;\n  task t;\n    begin : o1 int x = 7;\n\
               begin : i1 int x; $display(\"I=%0d\", x); end\n\
               $display(\"O=%0d\", x);\n    end\n  endtask\n\
           initial begin t(); end\n  initial begin #1 $finish; end\nendmodule\n",
        "VITA-E3009",
    );
    loud(
        "module top;\n  initial begin : p1 int y = 7;\n\
             begin : p2 int y; $display(\"I=%0d\", y); end\n\
             $display(\"O=%0d\", y);\n  end\n\
           initial begin #1 $finish; end\nendmodule\n",
        "VITA-E3009",
    );
}

/// A package task with a BODY-TOP static local (no block) is untouched: the counter is
/// retained across calls. Oracles: `101` then `102`. Correct at HEAD.
#[test]
fn a_package_task_body_top_static_local_stays_correct() {
    lines(
        "package pk;\n  task t; int c = 100; c = c + 1; $display(\"f=%0d\", c); endtask\n\
         endpackage\n\
         module top; import pk::*; initial begin t(); t(); $finish; end endmodule\n",
        &["f=101", "f=102"],
    );
}

/// A package `task automatic` with a body-top local re-initializes per call. Oracles:
/// `101` twice. Correct at HEAD.
#[test]
fn an_automatic_package_task_body_top_local_stays_correct() {
    let (o, code) = run(
        "package pk;\n  task automatic t; int c = 100; c = c + 1; $display(\"f=%0d\", c); endtask\n\
         endpackage\n\
         module top; import pk::*; initial begin t(); t(); $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0), "expected a clean run:\n{o}");
    assert_eq!(o.matches("f=101").count(), 2, "expected 101 twice:\n{o}");
}

/// Sibling block-locals in an INTERFACE task are LOUD (tasks in an interface are
/// outside the MVP) and stay loud — never silent.
#[test]
fn interface_task_siblings_stay_loud() {
    loud(
        "interface ib;\n  task t;\n    begin : b1 int x = 44; $display(\"A=%0d\", x); end\n\
             begin : b2 int x; $display(\"B=%0d\", x); end\n  endtask\nendinterface\n\
         module top; ib u(); initial begin u.t(); $finish; end endmodule\n",
        "VITA-E3009",
    );
}

/// Sibling block-locals in a CLASS method are LOUD and stay loud — class methods are
/// not fed to the classifier by this slice.
#[test]
fn class_method_siblings_stay_loud() {
    loud(
        "class C;\n  task t;\n    begin : b1 int x = 44; $display(\"A=%0d\", x); end\n\
             begin : b2 int x; $display(\"B=%0d\", x); end\n  endtask\nendclass\n\
         module top; C c = new(); initial begin c.t(); $finish; end endmodule\n",
        "VITA-E3010",
    );
}

/// CONVERTED PIN (§2 Scoping queue row 1) — a SCOPED call `pk::g()` with no `import`
/// binds its callee through `inject_pkg_callees` during body lowering, after step
/// (3.6a) has run. It used to keep the pre-slice flatten and this test pinned that
/// wrong value (`Z=88`) under the name `a_scoped_call_spelling_is_not_covered_yet`.
/// The body is now fed to the SAME joint gather at the injection funnel
/// (`Elaborator::feed_scoped_block_locals`), so the pin is the oracle value: both
/// iverilog 13 and verilator 5.052 print `Z=44`. The whole census of this spelling
/// lives in `crates/cli/tests/pkg_scoped_call_block_local.rs`.
#[test]
fn a_scoped_call_spelling_is_covered() {
    lines(
        "package pk;\n  function int g;\n    begin : b1 int x = 44; g = x; end\n\
             begin : b2 int x; g = g + x; end\n  endfunction\nendpackage\n\
         module top; int z;\n  initial begin z = pk::g(); $display(\"Z=%0d\", z); $finish; end\n\
         endmodule\n",
        &["Z=44"],
    );
}
