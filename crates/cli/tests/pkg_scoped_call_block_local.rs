//! §2 Scoping queue row 1 — a package subroutine called by its SCOPED spelling
//! (`pk::g()`) gets the same block-local classification an imported one gets.
//!
//! ## The defect
//!
//! §4.5.485 added step (3.6a) in `elaborate_instance`: once the imported package
//! `task`/`function` bodies are bound to this instance, the block-local scoping map is
//! recomputed with those bodies fed to the same walk, so two same-named sibling
//! block-locals in a package routine get distinct `$blk$<lo>` nets instead of
//! flattening onto one.
//!
//! That step keys on `rtn_pkg`, which holds the routines an `import` bound — bare
//! names only (`package.rs`, its four `insert` sites). A call written `pk::g()` whose
//! routine is NOT imported never gets into it: `inline_pkg_function` reserves and
//! lowers that body's frame ON DEMAND in pass 7, long after (3.6a) has run. The pair
//! stayed flattened and the value was silently wrong — `Z=88` where both iverilog 13
//! and verilator 5.052 print `Z=44`, exit 0, no diagnostic.
//!
//! The census behind this file measured the trigger to be per-ROUTINE membership in
//! `rtn_pkg`, not "no import": `pk::g()` beside `import pk::g;` or `import pk::*;` is
//! CORRECT (the map is span-keyed, so an import anywhere in the module fixes every
//! call spelling of that body), while `pk::g()` beside an import of a DIFFERENT
//! routine of the same package is still wrong even though `rtn_pkg` is non-empty.
//!
//! Two more feeds were measured on the same funnel:
//!
//! - a TRANSITIVE callee of an IMPORTED root. `inject_pkg_callees` binds `pk::h` under
//!   a `::` key at step (3.6), before (3.6a), but (3.6a) iterated `rtn_pkg` keys and
//!   `rtn_pkg` never holds a `::` key — so `import pk::g;` where `g` calls `h` and `h`
//!   holds the sibling pair printed `Z=88`.
//! - the `check_block_local_scope_leaks` half of (3.6a). The nested shape (an outer
//!   block-local read after an inner same-named, initializer-free declaration) was
//!   SILENT on the scoped spelling and LOUD on both the module-declared twin and the
//!   `import pk::g` twin of the identical body.
//!
//! ## The fix
//!
//! `compute_scoped_block_locals` is split into its phase-(1) gather and its
//! phase-(2)/(3) classification, and the gather is kept on the Elaborator
//! (`scoped_gather`). At the ONE injection funnel every uncovered spelling passes —
//! `inline_pkg_function`'s `frame_idx` miss branch — `feed_scoped_block_locals` adds
//! the body to that gather, re-classifies jointly (candidacy is a joint property of
//! every fed body, so a per-body computation is not equivalent), installs the entries
//! for spans inside this body, and runs the scope-leak gate. Step (3.6a) additionally
//! unions the `::` keys of `func_table`/`task_table` into its own key set, deduped on
//! the BODY SPAN so a routine reachable both ways is fed exactly once.
//!
//! The funnel is the right site rather than a pre-scan of `module.body` for a
//! `pkg::name` reference: two live cells below (a CLASS method body and an INTERFACE
//! body holding the call) contain no such reference in `module.body` at all.
//!
//! ## Oracles
//!
//! Every value pinned here was measured three-way against iverilog 13 (`-g2012` +
//! `vvp -n`) and verilator 5.052 (`--binary --timing`); both agree on every line.
//!
//! ## Residues, measured and NOT closed here
//!
//! - a scoped TASK call `pk::t()` is a PARSE error `VITA-E2002` (iverilog also rejects
//!   it; verilator accepts). Loud, one oracle — ROADMAP §3.
//! - a scoped call whose TRANSITIVE package callee holds any block-local is LOUD
//!   `VITA-E3010 undeclared net/variable top.$func$pk::g.x` (the callee is injected
//!   after the step-6.5 frame barrier, so it is never reserved as a frame). Both
//!   oracles print 44 — ROADMAP §3.
//! - an INTERFACE body applies no package ROUTINE import, so `import pk::g;` + `g()`
//!   inside an interface is `VITA-E3010 call to undeclared function` — ROADMAP §3.
//! - the nested scope-leak shape itself stays LOUD on every spelling (both oracles
//!   print a value); this slice only moves the scoped spelling from SILENT into that
//!   loud, which is up the accuracy ladder — ROADMAP §2.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pkgscoped_{}_{n}", std::process::id()));
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

/// A clean run (exit 0) whose output contains every `want` line and none of `absent`.
fn lines(src: &str, want: &[&str], absent: &[&str]) {
    let (o, code) = run(src);
    assert_eq!(code, Some(0), "expected a clean run:\n{o}");
    for w in want {
        assert!(o.contains(w), "expected {w:?} in:\n{o}");
    }
    for a in absent {
        assert!(!o.contains(a), "did NOT expect {a:?} in:\n{o}");
    }
}

/// A clean run whose output holds `want` exactly `n` times (two instances printing the
/// same line — `contains` alone cannot tell one from two).
fn lines_n(src: &str, want: &str, n: usize, absent: &[&str]) {
    let (o, code) = run(src);
    assert_eq!(code, Some(0), "expected a clean run:\n{o}");
    assert_eq!(
        o.matches(want).count(),
        n,
        "expected {n}x {want:?} in:\n{o}"
    );
    for a in absent {
        assert!(!o.contains(a), "did NOT expect {a:?} in:\n{o}");
    }
}

/// A run that must FAIL with the given diagnostic CODE (not its wording).
fn loud(src: &str, code_str: &str, want: &[&str]) {
    let (o, code) = run(src);
    assert_ne!(code, Some(0), "expected a loud refusal:\n{o}");
    assert!(o.contains(code_str), "expected {code_str:?} in:\n{o}");
    for w in want {
        assert!(o.contains(w), "expected {w:?} in:\n{o}");
    }
}

// ------------------------------------------------- cells this slice moves (was wrong)

/// The row's own cell: `pk::g()` with no import at all. Oracles `Z=44`; was `Z=88`.
#[test]
fn scoped_call_no_import_sibling_pair() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
module top; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=44"],
        &["Z=88"],
    );
}

/// `rtn_pkg` is NON-empty here (a DIFFERENT routine of the same package is imported)
/// and the cell was still wrong — the trigger is per-ROUTINE membership, not
/// emptiness. Oracles `Z=45`; was `Z=89`.
#[test]
fn scoped_call_beside_an_import_of_another_routine() {
    lines(
        r#"package pk;
  function int other; other = 1; endfunction
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
module top; import pk::other; int z;
  initial begin z = pk::g() + other(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=45"],
        &["Z=89"],
    );
}

/// The call site is a continuous assign, not a process. Oracles `W=44`; was `W=88`.
#[test]
fn scoped_call_in_a_continuous_assign() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
module top; wire [31:0] w = pk::g();
  initial begin #1 $display("W=%0d", w); $finish; end
endmodule
"#,
        &["W=44"],
        &["W=88"],
    );
}

/// The call site sits under a `generate if`. Oracles `Z=44`; was `Z=88`.
#[test]
fn scoped_call_inside_a_generate_if() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
module top; int z;
  generate if (1) begin : gb
    initial begin z = pk::g(); $display("Z=%0d", z); end
  end endgenerate
  initial begin #1 $finish; end
endmodule
"#,
        &["Z=44"],
        &["Z=88"],
    );
}

/// Both siblings are initializer-FREE (the `admit_static_plain` rule). Oracles `Z=44`;
/// was `Z=88`.
#[test]
fn scoped_call_both_siblings_initializer_free() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x; x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
module top; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=44"],
        &["Z=88"],
    );
}

/// Both siblings carry an initializer (44 and 55). Oracles `Z=99`; was `Z=110`.
#[test]
fn scoped_call_both_siblings_initialized() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x = 55; g = g + x; end
  endfunction
endpackage
module top; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=99"],
        &["Z=110"],
    );
}

/// `function automatic`. Oracles `Z=44`; was `Z=88`.
#[test]
fn scoped_call_function_automatic() {
    lines(
        r#"package pk;
  function automatic int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
module top; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=44"],
        &["Z=88"],
    );
}

/// The CALLING module declares its own `int x = 7`. The package locals must not touch
/// it and must not be routed onto it. Oracles `Z=44 X=7`; was `Z=88 X=7`.
#[test]
fn scoped_call_with_a_module_net_of_the_same_name() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
module top; int x = 7; int z;
  initial begin z = pk::g(); $display("Z=%0d X=%0d", z, x); $finish; end
endmodule
"#,
        &["Z=44 X=7"],
        &["Z=88"],
    );
}

/// The secondary feed: `import pk::g;` where `g` calls sibling `h` and `h` holds the
/// pair. `h` is bound under the `pk::h` key, which `rtn_pkg` never holds. Oracles
/// `Z=44`; was `Z=88`.
#[test]
fn transitive_callee_of_an_imported_root() {
    lines(
        r#"package pk;
  function int h;
    begin : b1 int x = 44; h = x; end
    begin : b2 int x; h = h + x; end
  endfunction
  function int g; g = h(); endfunction
endpackage
module top; import pk::g; int z;
  initial begin z = g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=44"],
        &["Z=88"],
    );
}

/// Two instances of the scoped-calling module — the per-instance feed set must not
/// leak between them. Oracles print `Z=44` twice; was `Z=88` twice.
#[test]
fn scoped_call_from_two_instances() {
    lines_n(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
module sub; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); end
endmodule
module top; sub u0(); sub u1();
  initial begin #1 $finish; end
endmodule
"#,
        "Z=44",
        2,
        &["Z=88"],
    );
}

/// The call lives in a CLASS method body, so `module.body` holds no `pkg::name`
/// reference — the cell a pre-scan of the module body would miss. Oracles `Z=44`;
/// was `Z=88`.
#[test]
fn scoped_call_from_a_class_method() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
class C; function int f; f = pk::g(); endfunction endclass
module top; C c; int z;
  initial begin c = new(); z = c.f(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=44"],
        &["Z=88"],
    );
}

/// The call lives in an INTERFACE body — the second cell with no `pkg::name` reference
/// in `module.body`, and the interface lane runs no (3.6a) step of its own. Oracles
/// `Z=44`; was `Z=88`.
#[test]
fn scoped_call_from_an_interface_body() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
interface ifc; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); end
endinterface
module top; ifc i();
  initial begin #1 $finish; end
endmodule
"#,
        &["Z=44"],
        &["Z=88"],
    );
}

/// Module `a` imports and module `b` scoped-calls the SAME routine. `a` was already
/// correct; `b` was not. Oracles `A=44 B=44`; was `A=44 B=88`.
#[test]
fn scoped_and_imported_callers_in_one_design() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
module a; import pk::g; int z; initial begin z = g(); $display("A=%0d", z); end endmodule
module b; int z; initial begin z = pk::g(); $display("B=%0d", z); end endmodule
module top; a ua(); b ub(); initial begin #1 $finish; end endmodule
"#,
        &["A=44", "B=44"],
        &["B=88"],
    );
}

/// The body is fed once and both call sites divert to the one frame. Oracles
/// `Z1=44 Z2=44`; was `Z1=88 Z2=88`.
#[test]
fn two_scoped_call_sites_in_one_process() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
module top; int z1, z2;
  initial begin z1 = pk::g(); z2 = pk::g(); $display("Z1=%0d Z2=%0d", z1, z2); $finish; end
endmodule
"#,
        &["Z1=44 Z2=44"],
        &["Z1=88"],
    );
}

/// A module-declared function uses the SAME block and local names. It was correct
/// before and must stay correct — it must not be fed twice. Oracles `Z=44 Y=11`;
/// was `Z=88 Y=11`.
#[test]
fn scoped_call_beside_a_module_routine_with_the_same_block_names() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
module top;
  function int m;
    begin : b1 int x = 11; m = x; end
    begin : b2 int x; m = m + x; end
  endfunction
  int z, y;
  initial begin z = pk::g(); y = m(); $display("Z=%0d Y=%0d", z, y); $finish; end
endmodule
"#,
        &["Z=44 Y=11"],
        &["Z=88"],
    );
}

/// `pk::q` is imported and holds the same block/local names; `pk::g` is scoped-called.
/// The imported half was correct in the same design and must stay so — this is the
/// control for the dedupe. Oracles `Z=44 Y=11`; was `Z=88 Y=11`.
#[test]
fn scoped_call_beside_an_imported_twin_with_the_same_local_names() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
  function int q;
    begin : b1 int x = 11; q = x; end
    begin : b2 int x; q = q + x; end
  endfunction
endpackage
module top; import pk::q; int z, y;
  initial begin z = pk::g(); y = q(); $display("Z=%0d Y=%0d", z, y); $finish; end
endmodule
"#,
        &["Z=44 Y=11"],
        &["Z=88"],
    );
}

/// An unrelated package's routine is imported, so `rtn_pkg` is non-empty with a key
/// from a DIFFERENT package. Oracles `Z=45`; was `Z=89`.
#[test]
fn scoped_call_with_an_unrelated_package_imported() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
package qk;
  function int f; f = 1; endfunction
endpackage
module top; import qk::f; int z;
  initial begin z = pk::g() + f(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=45"],
        &["Z=89"],
    );
}

/// The call site is an `always_comb`. Oracles `Z=44`; was `Z=88`.
#[test]
fn scoped_call_inside_always_comb() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
module top; int z; logic c = 0;
  always_comb z = pk::g() + c;
  initial begin #1 $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=44"],
        &["Z=88"],
    );
}

/// THREE same-named siblings — the magnitude scaled with the sibling count. Oracles
/// `Z=44`; was `Z=132`.
#[test]
fn scoped_call_three_same_named_siblings() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
    begin : b3 int x; g = g + x; end
  endfunction
endpackage
module top; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=44"],
        &["Z=132"],
    );
}

/// Two modules both scoped-call; one of them also declares a colliding `int x = 7`.
/// Oracles `A=44 X=7` and `B=44`; was `A=88 X=7` and `B=88`.
#[test]
fn two_scoped_calling_modules_one_with_a_colliding_net() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
module a; int x = 7; int z; initial begin z = pk::g(); $display("A=%0d X=%0d", z, x); end endmodule
module b; int z; initial begin z = pk::g(); $display("B=%0d", z); end endmodule
module top; a ua(); b ub(); initial begin #1 $finish; end endmodule
"#,
        &["A=44 X=7", "B=44"],
        &["A=88", "B=88"],
    );
}

// ------------------------------- the nested scope-leak shape, and why it stays SILENT here

/// The scoped spelling of the nested scope-leak shape stays SILENT — round 2 removed the
/// scope-leak gate from the on-demand feed.
///
/// An earlier round ran `check_block_local_scope_leaks` there, which moved this cell
/// SILENT -> LOUD and gave the scoped spelling parity with its module and import twins. That
/// parity was measured to cost more than it buys on a body whose inner block-local differs in
/// WIDTH from the outer one (`SCRATCH/review6/soundness-r2/d6.sv`, pinned below as
/// `package_body_with_a_narrower_inner_block_local_stays_correct`): the gate's predicate keys
/// on a NAME collision, and that cell is `Z=300` in PRE and in both oracles. Gating the
/// scoped spelling made it LOUD — correct -> loud, which the accuracy ladder forbids. Three
/// attempts to narrow the predicate each produced a new defect on the same axis, so the
/// narrowing was reverted whole (CLAUDE.md D8) and the scoped spelling is left ungated, which
/// is PRE byte for byte.
///
/// The cost is this cell: the PRE value `Z=14` where both oracles print `Z=7`, a pre-existing
/// SILENT on this spelling while the two twins below stay LOUD. ROADMAP §2 row — closing it
/// needs the gate to resolve the BINDING instead of keying on the name, which is the same
/// prerequisite the inline-fold lane needs (round-2 review S-R2-1).
#[test]
fn scoped_call_nested_scope_leak_stays_silent() {
    lines(
        r#"package pk;
  function int g;
    begin : o int x = 7;
      begin : i int x; g = x; end
      g = g + x;
    end
  endfunction
endpackage
module top; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=14"],
        &["VITA-E3009"],
    );
}

/// The MODULE-declared twin is LOUD, in PRE and after — it reaches the gate through step
/// (3.5), which this slice does not touch. Both oracles print `Z=7`.
#[test]
fn nested_scope_leak_module_twin_stays_loud() {
    loud(
        r#"module top; int z;
  function int g;
    begin : o int x = 7;
      begin : i int x; g = x; end
      g = g + x;
    end
  endfunction
  initial begin z = g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        "VITA-E3009",
        &["block-local `x` is referenced outside its `begin…end` block"],
    );
}

/// The `import pk::g` twin is LOUD too — it reaches the gate through step (3.6a), which
/// §4.5.485 shipped. Both oracles print `Z=7`.
#[test]
fn nested_scope_leak_import_twin_stays_loud() {
    loud(
        r#"package pk;
  function int g;
    begin : o int x = 7;
      begin : i int x; g = x; end
      g = g + x;
    end
  endfunction
endpackage
module top; import pk::g; int z;
  initial begin z = g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        "VITA-E3009",
        &["block-local `x` is referenced outside its `begin…end` block"],
    );
}

/// The cell that decided round 2: a package body reached by `pk::g()` whose inner
/// block-local is NARROWER than the outer one it shadows. `Z=300` in PRE and in both
/// oracles; gating the scoped spelling made it LOUD. This is the regression watchdog for the
/// scope-leak call that must NOT come back to `feed_scoped_block_locals` while the gate's
/// predicate is a bare name collision.
#[test]
fn package_body_with_a_narrower_inner_block_local_stays_correct() {
    lines(
        r#"package pk;
  function int g;
    begin : o int x = 300;
      begin : i byte x; end
      g = x;
    end
  endfunction
endpackage
module top; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); #1 $finish; end
endmodule
"#,
        &["Z=300"],
        &["VITA-E3009"],
    );
}

// ------------------------------------------- controls: correct before AND after

/// Bare call after `import pk::g` — correct before and after. Oracles `Z=44`.
#[test]
fn control_bare_call_after_explicit_import() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
module top; import pk::g; int z;
  initial begin z = g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=44"],
        &["Z=88"],
    );
}

/// Bare call after `import pk::*`. Oracles `Z=44`.
#[test]
fn control_bare_call_after_wildcard_import() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
module top; import pk::*; int z;
  initial begin z = g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=44"],
        &["Z=88"],
    );
}

/// `pk::g()` WITH `import pk::g` — already correct (the map is span-keyed). This is the
/// cell that refutes the row's "with no import" wording, and the dedupe watches it.
/// Oracles `Z=44`.
#[test]
fn control_scoped_call_with_explicit_import() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
module top; import pk::g; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=44"],
        &["Z=88"],
    );
}

/// `pk::g()` WITH `import pk::*`. Oracles `Z=44`.
#[test]
fn control_scoped_call_with_wildcard_import() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
module top; import pk::*; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=44"],
        &["Z=88"],
    );
}

/// The transitive-callee shape with NO block-local in the callee — isolates the
/// block-local as the trigger. Oracles `Z=44`.
#[test]
fn control_transitive_callee_with_no_block_local() {
    lines(
        r#"package pk;
  function int h; h = 44; endfunction
  function int g; g = h(); endfunction
endpackage
module top; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=44"],
        &[],
    );
}

/// 8-bit `[11:4]` siblings, both assigned. The old flatten happened to agree with the
/// oracle here (an accidental immunity), so this watches the fix for a value change.
/// Oracles `Z=a8`.
#[test]
fn control_multibit_non_zero_lsb_siblings() {
    lines(
        r#"package pk;
  function [11:4] g;
    begin : b1 logic [11:4] x = 8'hA5; g = x; end
    begin : b2 logic [11:4] x; x = 8'h03; g = g + x; end
  endfunction
endpackage
module top; logic [11:4] z;
  initial begin z = pk::g(); $display("Z=%0h", z); $finish; end
endmodule
"#,
        &["Z=a8"],
        &[],
    );
}

/// The identical body declared as a MODULE function — untouched by this slice.
/// Oracles `Z=44`.
#[test]
fn control_module_local_routine_same_body() {
    lines(
        r#"module top;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
  int z;
  initial begin z = g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=44"],
        &["Z=88"],
    );
}

/// ONE block-local, no sibling: it must NOT be scoped. A double feed of one body would
/// count its declaring span twice and make this lone declaration look like a pair —
/// this is the dedupe's watchdog. Oracles `Z=44`.
#[test]
fn control_single_block_local_no_sibling() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
  endfunction
endpackage
module top; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=44"],
        &[],
    );
}

/// Siblings declaring DIFFERENT names — no collision to resolve. Oracles `Z=44`.
#[test]
fn control_siblings_with_different_local_names() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int y; g = g + y; end
  endfunction
endpackage
module top; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=44"],
        &[],
    );
}

/// Sibling `for (int i …)` loop locals. Oracles `Z=23`.
#[test]
fn control_sibling_for_loop_locals() {
    lines(
        r#"package pk;
  function int g;
    int s = 0;
    begin : b1 for (int i = 0; i < 3; i++) s = s + i; end
    begin : b2 for (int i = 0; i < 2; i++) s = s + 10; end
    g = s;
  endfunction
endpackage
module top; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=23"],
        &[],
    );
}

/// The module twin of the colliding-net cell. Oracles `Z=44 X=7`.
#[test]
fn control_module_routine_siblings_with_a_module_net() {
    lines(
        r#"module top;
  int x = 7;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
  int z;
  initial begin z = g(); $display("Z=%0d X=%0d", z, x); $finish; end
endmodule
"#,
        &["Z=44 X=7"],
        &[],
    );
}

/// The import twin of the colliding-net cell. Oracles `Z=44 X=7`.
#[test]
fn control_imported_routine_siblings_with_a_module_net() {
    lines(
        r#"package pk;
  function int g;
    begin : b1 int x = 44; g = x; end
    begin : b2 int x; g = g + x; end
  endfunction
endpackage
module top; import pk::g; int x = 7; int z;
  initial begin z = g(); $display("Z=%0d X=%0d", z, x); $finish; end
endmodule
"#,
        &["Z=44 X=7"],
        &[],
    );
}

// ------------------------------- residues around the shared scope-leak gate (NOT fixed here)
//
// The gate `check_block_local_scope_leaks` rejects a reference to a name OUTSIDE the block
// that declares it, because vita's flat per-body local table would resolve that reference to
// the inner declaration. Its predicate is a NAME collision, so it also rejects bodies where
// the flatten is byte-correct — a false loud in the module and import lanes, measured below.
//
// Three rounds of this slice tried to narrow the predicate ("the inner declaration is never
// referenced and carries no initializer", then that plus a geometry match against the
// resolved outer twin). Each attempt produced a new defect on the same axis: a §11.6.1
// context-width hijack through the inline lane's name-keyed `scope.dims` lookup, then a
// correct -> loud regression on a package body whose inner local is narrower, then a missed
// outer twin whenever the routine body's ROOT statement is the enclosing block. Three
// blockers on one axis is the signal that the axis is wrong (CLAUDE.md D8), so the whole
// narrowing was reverted: `block_local/gate.rs`, `hoist.rs` and `mod.rs` are byte-identical
// to main and the cells below are byte-identical to PRE.
//
// What the gate actually needs is to resolve the BINDING a post-block reference takes rather
// than key on the name — the same prerequisite the inline-fold lane's context lookup needs.
// ROADMAP §2/§3 rows. The cells below pin the observed behaviour with the oracle value in
// the comment, so the residue is visible and closing it is a deliberate change.

/// FALSE LOUD, module lane: the inner `x` is declared and nothing more — never referenced
/// inside its own block, no initializer — so nothing reaches the coalesced net through it and
/// the outer read is byte-correct. Both iverilog 13 and verilator 5.052 print `Z=7`; vita
/// rejects. PRE-identical.
#[test]
fn inert_inner_decl_module_lane_is_falsely_loud() {
    loud(
        r#"module top; int z;
  function int g;
    begin : o int x = 7;
      begin : i int x; end
      g = x;
    end
  endfunction
  initial begin z = g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        "VITA-E3009",
        &["block-local `x` is referenced outside its `begin…end` block"],
    );
}

/// The same false loud on the IMPORT lane (the body reaches the gate through step (3.6a)).
/// Both oracles print `Z=7`. PRE-identical.
#[test]
fn inert_inner_decl_import_lane_is_falsely_loud() {
    loud(
        r#"package pk;
  function int g;
    begin : o int x = 7;
      begin : i int x; end
      g = x;
    end
  endfunction
endpackage
module top; import pk::g; int z;
  initial begin z = g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        "VITA-E3009",
        &["block-local `x` is referenced outside its `begin…end` block"],
    );
}

/// The same false loud where the shadowed outer binding is a function-scope variable read
/// BEFORE the block, module lane. Both oracles `Z=7`. PRE-identical.
#[test]
fn inert_inner_decl_outer_var_read_before_module_lane_is_falsely_loud() {
    loud(
        r#"module top; int z;
  function int g;
    int x = 7;
    g = x;
    begin : i int x; end
  endfunction
  initial begin z = g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        "VITA-E3009",
        &["block-local `x` is referenced outside its `begin…end` block"],
    );
}

/// Same, import lane. Both oracles `Z=7`. PRE-identical.
#[test]
fn inert_inner_decl_outer_var_read_before_import_lane_is_falsely_loud() {
    loud(
        r#"package pk;
  function int g;
    int x = 7;
    g = x;
    begin : i int x; end
  endfunction
endpackage
module top; import pk::g; int z;
  initial begin z = g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        "VITA-E3009",
        &["block-local `x` is referenced outside its `begin…end` block"],
    );
}

/// The SCOPED spelling of the same body is not gated at all (see
/// `scoped_call_nested_scope_leak_stays_silent` for why), and its flatten happens to be
/// byte-correct: `Z=7`, which is what both oracles print. Correct in PRE and after — this is
/// the cell that shows the module and import rejections above are false.
#[test]
fn inert_inner_decl_scoped_lane_is_correct() {
    lines(
        r#"package pk;
  function int g;
    begin : o int x = 7;
      begin : i int x; end
      g = x;
    end
  endfunction
endpackage
module top; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=7"],
        &["VITA-E3009"],
    );
}

/// The scoped lane with the inert declaration inside a `for` loop body. Both oracles `Z=7`.
#[test]
fn inert_inner_decl_in_a_for_loop_scoped_lane() {
    lines(
        r#"package pk;
  function int g;
    begin : o int x = 7;
      for (int k = 0; k < 2; k++) begin : i int x; end
      g = x;
    end
  endfunction
endpackage
module top; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=7"],
        &["VITA-E3009"],
    );
}

/// The scoped lane with the shadowed outer binding a FORMAL. Both oracles `Z=7`.
#[test]
fn inert_inner_decl_outer_formal_scoped_lane() {
    lines(
        r#"package pk;
  function int g(input int x);
    begin : i int x; end
    g = x;
  endfunction
endpackage
module top; int z;
  initial begin z = pk::g(7); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=7"],
        &["VITA-E3009"],
    );
}

/// RESIDUE, scoped lane: one reference to the name inside the block makes the inner
/// declaration a real shadow, and the scoped spelling is ungated, so the flatten's value is
/// what comes out — `Z=1` where both oracles print `Z=7`. Pinned as observed, ROADMAP §2.
/// Its module-lane twin below is LOUD, which is the correct-or-loud answer for the shape.
#[test]
fn inner_decl_referenced_inside_its_block_scoped_lane_is_silent() {
    lines(
        r#"package pk;
  function int g;
    begin : o int x = 7;
      begin : i int x; x = 1; end
      g = x;
    end
  endfunction
endpackage
module top; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=1"],
        &["VITA-E3009"],
    );
}

/// The module-lane twin of the cell above: LOUD, in PRE and after. Both oracles `Z=7`, so the
/// shape is a ROADMAP §3 row (loud -> supported), not a silent one.
#[test]
fn inner_decl_referenced_inside_its_block_module_lane_is_loud() {
    loud(
        r#"module top; int z;
  function int g;
    begin : o int x = 7;
      begin : i int x; x = 1; end
      g = x;
    end
  endfunction
  initial begin z = g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        "VITA-E3009",
        &["block-local `x` is referenced outside its `begin…end` block"],
    );
}

/// FIXED by this slice, not by the gate: both siblings carry an initializer, so the §2
/// Scoping classifier gives each its own `$blk$<lo>` net once the feed hands the package body
/// to it. PRE printed `Z=5`; both oracles and POST print `Z=7`.
#[test]
fn scoped_call_inner_initialized_shadow_is_fixed() {
    lines(
        r#"package pk;
  function int g;
    begin : o int x = 7;
      begin : i int x = 5; end
      g = x;
    end
  endfunction
endpackage
module top; int z;
  initial begin z = pk::g(); $display("Z=%0d", z); $finish; end
endmodule
"#,
        &["Z=7"],
        &["Z=5"],
    );
}

/// RESIDUE, the round-2 finding's shape: an inert inner `logic [7:0] v` beside an outer
/// `logic [31:0] v`, reached by the INLINE fold. vita rejects it; both oracles print
/// `V=fe01`. PRE-identical — this is the cell that a name-keyed stand-down turned into a
/// silent `V=1`, and the reason the gate must resolve the binding before it can be narrowed.
#[test]
fn inert_inner_decl_narrower_than_the_outer_is_loud() {
    loud(
        r#"module t;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF;
  function [31:0] fv;
    logic [31:0] v;
    begin
      begin : n logic [7:0] v; end
      v = a8 * b8;
    end
    fv = v;
  endfunction
  initial begin $display("V=%0h", fv()); #1 $finish; end
endmodule
"#,
        "VITA-E3009",
        &["block-local `v` is referenced outside its `begin…end` block"],
    );
}

/// RESIDUE, the round-3 finding's shape: the routine body's ROOT statement is the block
/// holding the true outer twin. vita rejects; both oracles print `V=fe01`. PRE-identical.
#[test]
fn root_block_outer_twin_is_loud() {
    loud(
        r#"module top;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF;
  function [31:0] fv;
    logic [7:0] v;
    begin : outer
      logic [31:0] v;
      begin : inner logic [7:0] v; end
      v = a8 * b8;
      fv = v;
    end
  endfunction
  initial begin $display("V=%0h", fv()); #1 $finish; end
endmodule
"#,
        "VITA-E3009",
        &["block-local `v` is referenced outside its `begin…end` block"],
    );
}

/// The SIGN mirror of the cell above — both oracles print `V=255`, and the value a name-keyed
/// stand-down produced was `V=-1`. vita rejects. PRE-identical.
#[test]
fn root_block_outer_twin_sign_is_loud() {
    loud(
        r#"module top;
  function int fv;
    logic signed [7:0] v;
    begin : outer
      logic [7:0] v;
      begin : inner logic signed [7:0] v; end
      v = 8'hFF;
      fv = v;
    end
  endfunction
  initial begin $display("V=%0d", fv()); #1 $finish; end
endmodule
"#,
        "VITA-E3009",
        &["block-local `v` is referenced outside its `begin…end` block"],
    );
}

/// The same shape in a static TASK. Both oracles print `V=fe01`; vita rejects. PRE-identical.
#[test]
fn root_block_outer_twin_task_is_loud() {
    loud(
        r#"module top;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF; logic [31:0] o;
  task tv(output logic [31:0] r);
    logic [7:0] v;
    begin : outer
      logic [31:0] v;
      begin : inner logic [7:0] v; end
      v = a8 * b8;
      r = v;
    end
  endtask
  initial begin tv(o); $display("V=%0h", o); #1 $finish; end
endmodule
"#,
        "VITA-E3009",
        &["block-local `v` is referenced outside its `begin…end` block"],
    );
}

/// Attribution control for the two cells above: the SAME module with no same-named inner
/// declaration at all. `V=1` in PRE, `V=fe01` after and in both oracles — the width is the
/// inline-fold lane's §11.6.1 assignment context for `a8 * b8` in a non-`automatic`
/// function (§4.5.491, landed in the same bundle), not anything about a block-local.
#[test]
fn control_no_name_collision_has_the_oracle_width() {
    lines(
        r#"module t;
  logic [7:0] a8 = 8'hFF, b8 = 8'hFF;
  function [31:0] fv;
    logic [31:0] v;
    begin
      begin : n logic [7:0] w; end
      v = a8 * b8;
    end
    fv = v;
  endfunction
  initial begin $display("V=%0h", fv()); #1 $finish; end
endmodule
"#,
        &["V=fe01"],
        &["VITA-E3009"],
    );
}
