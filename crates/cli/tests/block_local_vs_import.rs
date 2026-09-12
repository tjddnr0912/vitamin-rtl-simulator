//! A module-process block-local whose bare name matches an IMPORTED package variable
//! is its OWN variable, not the package's storage (§2 Scoping, ROADMAP §5.2 row 3).
//!
//! ## What was wrong
//!
//! `import pk::*` / `import pk::pv` binds the package variable by inserting the
//! `$pkg$pk` net id VERBATIM into the importing module's `symbols`
//! (`package.rs:1288-1291` and `:1406-1408`), with the alias recorded in
//! `pkg_var_aliases`. There is exactly ONE such net per package variable for the whole
//! design (`lib.rs:1180`), so that alias is shared by every importer and every
//! instance.
//!
//! A plain, initializer-free, non-`automatic` block-local in a module PROCESS is given
//! its own `$blk$<lo>` net by exactly one admission term, `shadows_module`
//! (`block_local_class.rs:398`, `frames_reserve.rs:180`, `block_local/hoist.rs`);
//! `admit_static_plain` is deliberately false on that feed
//! (`block_local_class.rs:284-286`). That term read `gather_local_decl_names`, which is
//! the module's OWN declarations only — an import alias was never in it. So
//!
//! ```text
//! package pk; integer pv = 5; endpackage
//! module top; import pk::*;
//!  initial begin
//!   begin : blk integer pv; pv = 99; $display("inside: pv=%0d pkpv=%0d", pv, pk::pv); end
//!  end
//! endmodule
//! ```
//!
//! printed `inside: pv=99 pkpv=99` where both oracles print `inside: pv=99 pkpv=5`:
//! the block-local's write landed on the PACKAGE's storage. Because the net is global,
//! the corruption left the module — a different module importing the same package read
//! 99, a second instance of the writer read the other instance's value, and a
//! continuous `assign w = pv;` read 99. Seventeen readout cells over twelve designs.
//!
//! ## The fix
//!
//! `names_with_pkg_var_aliases` (`block_local/mod.rs`) returns `names` ∪ the bare names
//! `pkg_var_aliases` bound at the current scope, and it is handed ONLY to
//! `compute_scoped_block_locals` (`instance.rs`, both feeds, and the `iface_inst.rs`
//! mirror) and to the `shadows_module` twin in `block_local/hoist.rs`. An imported name
//! occupies the module's bare-name namespace exactly as a declared net does, which is
//! the precondition `shadows_module` exists to detect.
//!
//! Two traps the shape avoids, both pinned by tests below: the set is NOT written back
//! into `names` (that same set is `apply_import_consts`'s `local_names`, where
//! membership SUPPRESSES the import — see `an_import_still_binds_without_a_block_local`
//! and every `pv=99` cell, which need the import to be bound), and it is NOT fed to
//! `compute_per_entry_block_locals`, which reads its names with the opposite polarity.
//!
//! ## Louds this LIFTS (loud -> correct, never the reverse)
//!
//! Once the block-local owns its own net, four guards that fired only because it did
//! not stop firing, and each cell lands on the oracle value:
//! `pv` read (a1) or written (a3) AFTER the block resolves to the import again;
//! an `automatic` declarator no longer "collides with an existing net" (c1);
//! an initialized declarator is no longer a read-before-assign of a shared net (c3);
//! a narrower declarator is no longer two types on one net (c4).
//!
//! That also retires a misworded diagnostic: c3/c4 said "a same-named block-local in
//! ANOTHER BLOCK" when the collision was with the import alias. After the fix an import
//! can no longer reach either message, so the wording is accurate for every remaining
//! trigger (a genuine second block) and no wording change is needed.
//!
//! ## Oracles
//!
//! Every value pinned here was measured three-way against iverilog 13 (`-g2012` +
//! `vvp -n`) and verilator 5.052 (`--binary --timing`). Both agree on every cell except
//! the two noted in their tests: the `automatic` declarator (iverilog: "sorry:
//! Overriding the default variable lifetime is not yet supported") and the
//! explicit-import-plus-local-declaration error (verilator answers the local instead of
//! refusing; iverilog refuses, and vita is pinned to iverilog).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_blvi_{}_{n}", std::process::id()));
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

/// A clean run (exit 0) whose output contains every `want` line, verbatim as observed,
/// and none of `absent` (the pre-fix wrong text, so a silent regression cannot pass).
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

/// A run that must FAIL with the given diagnostic CODE (not its wording).
fn loud(src: &str, code_str: &str) {
    let (o, code) = run(src);
    assert_ne!(code, Some(0), "expected a loud refusal:\n{o}");
    assert!(o.contains(code_str), "expected {code_str:?} in:\n{o}");
}

const PK: &str = "package pk; integer pv = 5; endpackage\n";

// ------------------------------------------------- the two binder spellings

/// a2 — wildcard import. Oracles: `inside: pv=99 pkpv=5`. Was `pkpv=99`.
#[test]
fn a_wildcard_import_keeps_the_package_value() {
    lines(
        &format!(
            "{PK}module top; import pk::*;\n initial begin\n  begin : blk integer pv; pv = 99; \
             $display(\"inside: pv=%0d pkpv=%0d\", pv, pk::pv); end\n  #1 $finish;\n end\nendmodule\n"
        ),
        &["inside: pv=99 pkpv=5"],
        &["pkpv=99"],
    );
}

/// b4 — EXPLICIT `import pk::pv`. Oracles: `expl: pv=99 pkpv=5`. Was `pkpv=99`.
/// The explicit bind is a second insert site (`package.rs:1406-1408`), so it needs its
/// own cell.
#[test]
fn an_explicit_import_keeps_the_package_value() {
    lines(
        &format!(
            "{PK}module top; import pk::pv;\n initial begin\n  begin : blk integer pv; pv = 99; \
             $display(\"expl: pv=%0d pkpv=%0d\", pv, pk::pv); end\n  #1 $finish;\n end\nendmodule\n"
        ),
        &["expl: pv=99 pkpv=5"],
        &["pkpv=99"],
    );
}

// ------------------------------------------------------------- leak scope

/// b1 — a DIFFERENT module importing the same package. Oracles:
/// `wr-in: pkpv=5` / `obs: pv=5 pkpv=5`. Was `99` on all three readouts: the write
/// crossed a module boundary, because `pkg_vars` is one net for the whole design.
#[test]
fn the_write_does_not_reach_another_importing_module() {
    lines(
        &format!(
            "{PK}module obs; import pk::*; initial begin #2 \
             $display(\"obs: pv=%0d pkpv=%0d\", pv, pk::pv); end endmodule\n\
             module wr; import pk::*; initial begin begin : blk integer pv; pv = 99; \
             $display(\"wr-in: pkpv=%0d\", pk::pv); end end endmodule\n\
             module top; wr u1(); obs u2(); initial #5 $finish; endmodule\n"
        ),
        &["wr-in: pkpv=5", "obs: pv=5 pkpv=5"],
        &["pkpv=99"],
    );
}

/// b2 — two INSTANCES of the writer module. Oracles: `wr1: pkpv=5` / `wr2: pkpv=5` /
/// `top: pkpv=5`. Was `91` / `92` / `92` — each instance overwrote the shared net and
/// the next one observed it.
#[test]
fn two_instances_do_not_share_the_block_local() {
    lines(
        &format!(
            "{PK}module wr(input integer tag); import pk::*; initial begin #tag; \
             begin : blk integer pv; pv = 90+tag; $display(\"wr%0d: pkpv=%0d\", tag, pk::pv); end \
             end endmodule\n\
             module top; wr u1(1); wr u2(2); import pk::*; initial begin \
             #3 $display(\"top: pkpv=%0d\", pk::pv); #1 $finish; end endmodule\n"
        ),
        &["wr1: pkpv=5", "wr2: pkpv=5", "top: pkpv=5"],
        &["pkpv=91", "pkpv=92"],
    );
}

/// d8 — a CONTINUOUS assign reading the bare (imported) name. Oracles:
/// `cont: w=5 pkpv=5`. Was `w=99 pkpv=99`: proof there was no module-local shadow net,
/// only the one package net under both spellings.
#[test]
fn a_continuous_assign_reads_the_import_not_the_block_local() {
    lines(
        &format!(
            "{PK}module top; import pk::*;\n wire [31:0] w; assign w = pv;\n initial begin\n  \
             begin : blk integer pv; pv = 99; end\n  \
             #1 $display(\"cont: w=%0d pkpv=%0d\", w, pk::pv); $finish;\n end\nendmodule\n"
        ),
        &["cont: w=5 pkpv=5"],
        &["w=99"],
    );
}

/// f1 — reading `pk::pv` BEFORE and INSIDE the block. Oracles: `pre: pkpv=5` /
/// `in: pkpv=5`. Was `pre: pkpv=5` / `in: pkpv=99`, which is what proves this was a
/// WRITE-side aliasing defect and not a name-resolution one: the pre-block read was
/// always right.
#[test]
fn the_package_value_is_unchanged_before_and_inside_the_block() {
    lines(
        &format!(
            "{PK}module top; import pk::*;\n initial begin\n  \
             $display(\"pre: pkpv=%0d\", pk::pv);\n  \
             begin : blk integer pv; pv = 99; $display(\"in: pkpv=%0d\", pk::pv); end\n  \
             #1 $finish;\n end\nendmodule\n"
        ),
        &["pre: pkpv=5", "in: pkpv=5"],
        &["in: pkpv=99"],
    );
}

/// f3 — two SIBLING blocks each declaring the name. Oracles: `s1: pv=90 pkpv=5` /
/// `s2: pv=91 pkpv=5`. Was `pkpv=90` / `pkpv=91`. This is the cell that refutes the
/// standing rationale in `block_local_class.rs`: a sibling pair shadowing a package
/// variable is admitted and scoped only on the SUBROUTINE feed, never on the
/// module-process feed, so "two spans" did not rescue it.
#[test]
fn sibling_blocks_are_two_variables_and_neither_is_the_package() {
    lines(
        &format!(
            "{PK}module top; import pk::*;\n initial begin\n  \
             begin : b1 integer pv; pv = 90; $display(\"s1: pv=%0d pkpv=%0d\", pv, pk::pv); end\n  \
             begin : b2 integer pv; pv = 91; $display(\"s2: pv=%0d pkpv=%0d\", pv, pk::pv); end\n  \
             #1 $finish;\n end\nendmodule\n"
        ),
        &["s1: pv=90 pkpv=5", "s2: pv=91 pkpv=5"],
        &["pkpv=90", "pkpv=91"],
    );
}

/// f4 — a SECOND package variable that is not shadowed still imports. Oracles:
/// `two: pv=99 pkpv=5 qv=6`. `qv=6` was already right (the damage is name-scoped), and
/// it is the direct pin for trap 1: if the augmented set were written back into `names`
/// it would become `apply_import_consts`'s suppression set and `qv` would unbind.
#[test]
fn an_unshadowed_second_package_variable_still_imports() {
    lines(
        "package pk; integer pv = 5; integer qv = 6; endpackage\n\
         module top; import pk::*;\n initial begin\n  \
         begin : blk integer pv; pv = 99; \
         $display(\"two: pv=%0d pkpv=%0d qv=%0d\", pv, pk::pv, qv); end\n  \
         #1 $finish;\n end\nendmodule\n",
        &["two: pv=99 pkpv=5 qv=6"],
        &["pkpv=99"],
    );
}

// --------------------------------------------------------- block position

/// d2 — an `always_ff` body, not an `initial`. Oracles: `ff: pv=99 pkpv=5`.
#[test]
fn an_always_ff_block_local_keeps_the_package_value() {
    lines(
        &format!(
            "{PK}module top; import pk::*; logic clk=0; always #1 clk=~clk;\n \
             always_ff @(posedge clk) begin : blk integer pv; pv = 99; \
             $display(\"ff: pv=%0d pkpv=%0d\", pv, pk::pv); end\n initial #3 $finish;\nendmodule\n"
        ),
        &["ff: pv=99 pkpv=5"],
        &["pkpv=99"],
    );
}

/// d6 — a named block nested two deep. Oracles: `deep: pv=99 pkpv=5`.
#[test]
fn a_doubly_nested_block_keeps_the_package_value() {
    lines(
        &format!(
            "{PK}module top; import pk::*;\n initial begin : o1 begin : o2 \
             begin : blk integer pv; pv = 99; \
             $display(\"deep: pv=%0d pkpv=%0d\", pv, pk::pv); end end #1 $finish; end\nendmodule\n"
        ),
        &["deep: pv=99 pkpv=5"],
        &["pkpv=99"],
    );
}

/// d7 — an UNNAMED `begin ... end`. Both oracles accept a declaration in one.
/// Oracles: `unnamed: pv=99 pkpv=5`.
#[test]
fn an_unnamed_block_keeps_the_package_value() {
    lines(
        &format!(
            "{PK}module top; import pk::*;\n initial begin\n  \
             begin integer pv; pv = 99; $display(\"unnamed: pv=%0d pkpv=%0d\", pv, pk::pv); end\n  \
             #1 $finish;\n end\nendmodule\n"
        ),
        &["unnamed: pv=99 pkpv=5"],
        &["pkpv=99"],
    );
}

// ------------------------------------------------- package variable types

/// c5 — package variable declared `int`. Oracles: `c5: pv=99 pkpv=5`.
#[test]
fn an_int_package_variable_keeps_its_value() {
    lines(
        "package pk; int pv = 5; endpackage\n\
         module top; import pk::*;\n initial begin\n  \
         begin : blk int pv; pv = 99; $display(\"c5: pv=%0d pkpv=%0d\", pv, pk::pv); end\n  \
         #1 $finish;\n end\nendmodule\n",
        &["c5: pv=99 pkpv=5"],
        &["pkpv=99"],
    );
}

/// c8 — package variable declared `logic [7:0]`. Oracles: `c8: pv=99 pkpv=5`.
#[test]
fn a_packed_vector_package_variable_keeps_its_value() {
    lines(
        "package pk; logic [7:0] pv = 5; endpackage\n\
         module top; import pk::*;\n initial begin\n  \
         begin : blk logic [7:0] pv; pv = 99; $display(\"c8: pv=%0d pkpv=%0d\", pv, pk::pv); end\n  \
         #1 $finish;\n end\nendmodule\n",
        &["c8: pv=99 pkpv=5"],
        &["pkpv=99"],
    );
}

// ------------------------------------------------------ lifted loud cells

/// a1 — the AFTER-BLOCK read, the shape ROADMAP §5.2 row 3 was filed on. It did NOT
/// reproduce as a silent 99: before the fix it was a loud E3009 ("block-local `pv` is
/// referenced outside its `begin…end` block"), because the scope-leak gate fires on the
/// NAME regardless of which net it resolves to. Now that the block-local owns a `$blk$`
/// net the gate has nothing to guard and the outer reference resolves to the import:
/// `after: pv=5 pkpv=5`, both oracles. Loud -> correct.
#[test]
fn a_read_after_the_block_resolves_to_the_import() {
    lines(
        &format!(
            "{PK}module top; import pk::*;\n initial begin\n  \
             begin : blk integer pv; pv = 99; end\n  \
             $display(\"after: pv=%0d pkpv=%0d\", pv, pk::pv);\n  #1 $finish;\n end\nendmodule\n"
        ),
        &["after: pv=5 pkpv=5"],
        &["E3009", "pv=99"],
    );
}

/// a3 — a WRITE to the bare name after the block updates the package variable, which is
/// the oracles' proof that the outer name is the import again: `afterwrite: pv=7
/// pkpv=7`. Was the same E3009. Loud -> correct.
#[test]
fn a_write_after_the_block_updates_the_package_variable() {
    lines(
        &format!(
            "{PK}module top; import pk::*;\n initial begin\n  \
             begin : blk integer pv; pv = 99; end\n  pv = 7;\n  \
             $display(\"afterwrite: pv=%0d pkpv=%0d\", pv, pk::pv);\n  #1 $finish;\n end\nendmodule\n"
        ),
        &["afterwrite: pv=7 pkpv=7"],
        &["E3009"],
    );
}

/// c1 — an `automatic` declarator. Was E3009 "collides with an existing net of the same
/// name"; the collision was with the import alias, and there is none now:
/// `c1: pv=99 pkpv=5`. Loud -> correct.
///
/// SINGLE ORACLE: verilator only. iverilog 13 refuses the design outright ("sorry:
/// Overriding the default variable lifetime is not yet supported"), so it cannot answer
/// here; verilator prints the pinned line, and it is the same value the whole
/// non-`automatic` family lands on.
#[test]
fn an_automatic_declarator_no_longer_collides_with_the_import() {
    lines(
        &format!(
            "{PK}module top; import pk::*;\n initial begin\n  \
             begin : blk automatic integer pv; pv = 99; \
             $display(\"c1: pv=%0d pkpv=%0d\", pv, pk::pv); end\n  #1 $finish;\n end\nendmodule\n"
        ),
        &["c1: pv=99 pkpv=5"],
        &["E3009"],
    );
}

/// c3 — an INITIALIZED declarator read before it is reassigned. Was E3009
/// read-before-assign of a shared flattened net (and the message named "a same-named
/// block-local in another block", which there never was). Oracles: `c3: pv=99 pkpv=5`.
/// Loud -> correct.
#[test]
fn an_initialized_declarator_is_no_longer_a_shared_net_read() {
    lines(
        &format!(
            "{PK}module top; import pk::*;\n initial begin\n  \
             begin : blk integer pv = 3; pv = pv+96; \
             $display(\"c3: pv=%0d pkpv=%0d\", pv, pk::pv); end\n  #1 $finish;\n end\nendmodule\n"
        ),
        &["c3: pv=99 pkpv=5"],
        &["E3009"],
    );
}

/// c4 — a NARROWER declarator than the package variable. Was E3009 "different
/// width/signedness than a same-named block-local in another block" — again the
/// collision was the import, not another block. Oracles: `c4: pv=99 pkpv=5`.
/// Loud -> correct.
#[test]
fn a_narrower_declarator_no_longer_shares_a_net_with_the_import() {
    lines(
        "package pk; logic [31:0] pv = 5; endpackage\n\
         module top; import pk::*;\n initial begin\n  \
         begin : blk logic [7:0] pv; pv = 8'd99; \
         $display(\"c4: pv=%0d pkpv=%0d\", pv, pk::pv); end\n  #1 $finish;\n end\nendmodule\n",
        &["c4: pv=99 pkpv=5"],
        &["E3009"],
    );
}

// ----------------------------------------------------------------- controls

/// b3 — NO import, `pk::pv` used only qualified. The isolating control: with no import
/// there is no module-scope `pv` symbol, the block-local always minted its own net, and
/// this cell was correct before the fix. `noimp: pv=99 pkpv=5`, byte-identical.
#[test]
fn without_an_import_the_block_local_was_always_its_own() {
    lines(
        &format!(
            "{PK}module top;\n initial begin\n  \
             begin : blk integer pv; pv = 99; \
             $display(\"noimp: pv=%0d pkpv=%0d\", pv, pk::pv); end\n  #1 $finish;\n end\nendmodule\n"
        ),
        &["noimp: pv=99 pkpv=5"],
        &["pkpv=99"],
    );
}

/// e2 — a MODULE-LEVEL declaration beside the import (IEEE §26.3 local-wins). Correct
/// before and after: the local name was already in `gather_local_decl_names`.
/// `modlvl: pv=99 pkpv=5`.
#[test]
fn a_module_level_declaration_wins_the_wildcard_import() {
    lines(
        &format!(
            "{PK}module top; import pk::*; integer pv;\n initial begin pv = 99; \
             $display(\"modlvl: pv=%0d pkpv=%0d\", pv, pk::pv); #1 $finish; end\nendmodule\n"
        ),
        &["modlvl: pv=99 pkpv=5"],
        &["pkpv=99"],
    );
}

/// e4 — the MECHANISM PROOF, unchanged by the fix. Adding a module-level `integer pv`
/// (which touches no package storage) already made the whole shape correct before it,
/// because that is the one thing that put the name into the shadow set. Oracles:
/// `both: pv=99 mod=1 pkpv=5`.
#[test]
fn a_module_level_twin_already_scoped_the_block_local() {
    lines(
        &format!(
            "{PK}module top; integer pv = 1; import pk::*;\n initial begin\n  \
             begin : blk integer pv; pv = 99; \
             $display(\"both: pv=%0d mod=%0d pkpv=%0d\", pv, top.pv, pk::pv); end\n  \
             #1 $finish;\n end\nendmodule\n"
        ),
        &["both: pv=99 mod=1 pkpv=5"],
        &["pkpv=99"],
    );
}

/// e3 — an EXPLICIT import plus a local declaration of the name is an error
/// (IEEE §26.3). iverilog refuses it; verilator answers the local. vita is pinned to
/// iverilog and must stay loud: the fix must not turn this into a silent answer.
#[test]
fn an_explicit_import_colliding_with_a_local_declaration_stays_loud() {
    loud(
        &format!(
            "{PK}module top; import pk::pv; integer pv;\n initial begin pv = 99; \
             $display(\"explmod: pv=%0d pkpv=%0d\", pv, pk::pv); #1 $finish; end\nendmodule\n"
        ),
        "E3009",
    );
}

/// d3 — a FUNCTION body local. Already correct (§4.5.486 gave subroutine bodies
/// frame-local storage), and the fix must leave it byte-identical: the augmented set
/// reaches `compute_scoped_block_locals`, which the subroutine feed also uses, so this
/// is the regression pin for that feed. `func: pv=99 pkpv=5`.
#[test]
fn a_function_body_local_is_unaffected() {
    lines(
        &format!(
            "{PK}module top; import pk::*;\n function integer f(); integer pv; pv = 99; \
             $display(\"func: pv=%0d pkpv=%0d\", pv, pk::pv); return pv; endfunction\n \
             initial begin void'(f()); #1 $finish; end\nendmodule\n"
        ),
        &["func: pv=99 pkpv=5"],
        &["pkpv=99"],
    );
}

/// d4 — a TASK body local. Same, already correct. `task: pv=99 pkpv=5`.
#[test]
fn a_task_body_local_is_unaffected() {
    lines(
        &format!(
            "{PK}module top; import pk::*;\n task t(); integer pv; pv = 99; \
             $display(\"task: pv=%0d pkpv=%0d\", pv, pk::pv); endtask\n \
             initial begin t(); #1 $finish; end\nendmodule\n"
        ),
        &["task: pv=99 pkpv=5"],
        &["pkpv=99"],
    );
}

/// d5 — a block-local inside a GENERATE block. Already correct (the genblock scope
/// prefixes the flatten), and the hoist-side twin now consults `pkg_var_aliases`
/// through the outward scope walk, so this is the pin that the walk did not change a
/// generate body's answer. `gen: pv=99 pkpv=5`.
#[test]
fn a_generate_block_local_is_unaffected() {
    lines(
        &format!(
            "{PK}module top; import pk::*;\n genvar g;\n \
             generate for (g=0; g<1; g=g+1) begin : gb\n   \
             initial begin : blk integer pv; pv = 99; \
             $display(\"gen: pv=%0d pkpv=%0d\", pv, pk::pv); end\n end endgenerate\n \
             initial #1 $finish;\nendmodule\n"
        ),
        &["gen: pv=99 pkpv=5"],
        &["pkpv=99"],
    );
}

/// Trap 1, stated directly: with no block-local at all, the wildcard import must still
/// bind the bare name. `apply_import_consts` SUPPRESSES an import whose name is in its
/// `local_names` set, which is the same `names` the fix deliberately does not touch.
/// Oracles: `plain: pv=5 pkpv=5`.
#[test]
fn an_import_still_binds_without_a_block_local() {
    lines(
        &format!(
            "{PK}module top; import pk::*;\n initial begin \
             $display(\"plain: pv=%0d pkpv=%0d\", pv, pk::pv); #1 $finish; end\nendmodule\n"
        ),
        &["plain: pv=5 pkpv=5"],
        &[],
    );
}

// ------------------------------------------------------- the interface lane
//
// An `interface` body gets the same five block-local classifier maps as a module body
// (`iface_inst.rs`), so it has the same defect and takes the same fix. Round-1 review
// found the first cut of that mirror was DEAD CODE: the maps were computed BEFORE the
// interface's two `apply_import_consts` passes, so `pkg_var_aliases` held nothing for
// the interface scope yet and the augmentation was a guaranteed no-op. The whole
// computation now sits after both import passes, mirroring the module lane's order.
// Every cell below was measured 4-way (PRE, POST, iverilog 13, verilator 5.052); PRE
// answered `pkpv=99` / `pkpv=91` / `w=99` on the first five and both oracles keep the
// package value.

/// s3_08 — BODY `import pk::*` inside the interface. Oracles: `s3_08: pv=99 pkpv=5`.
/// Was `pkpv=99` in both PRE and the first cut of the mirror.
#[test]
fn an_interface_body_import_keeps_the_package_value() {
    lines(
        &format!(
            "{PK}interface ifc();\n  import pk::*;\n  initial begin : blk\n    integer pv;\n    \
             pv = 99;\n    $display(\"s3_08: pv=%0d pkpv=%0d\", pv, pk::pv);\n  end\nendinterface\n\
             module top;\n  ifc i1();\n  initial #1 $finish;\nendmodule\n"
        ),
        &["s3_08: pv=99 pkpv=5"],
        &["pkpv=99"],
    );
}

/// s3_11 — HEADER import (`interface ifc import pk::*; ();`), the other of the two
/// `apply_import_consts` passes the maps must now follow. Oracles:
/// `s3_11: pv=99 pkpv=5`.
#[test]
fn an_interface_header_import_keeps_the_package_value() {
    lines(
        &format!(
            "{PK}interface ifc import pk::*; ();\n  initial begin : blk\n    integer pv;\n    \
             pv = 99;\n    $display(\"s3_11: pv=%0d pkpv=%0d\", pv, pk::pv);\n  end\nendinterface\n\
             module top;\n  ifc i1();\n  initial #1 $finish;\nendmodule\n"
        ),
        &["s3_11: pv=99 pkpv=5"],
        &["pkpv=99"],
    );
}

/// s3_12 — the leak OUT of the interface: an enclosing module importing the same
/// package read the interface block-local's write. Oracles: `s3_12: pkpv=5`. Was 91.
#[test]
fn an_interface_block_local_does_not_reach_the_enclosing_module() {
    lines(
        &format!(
            "{PK}interface ifc();\n  import pk::*;\n  initial begin : blk\n    integer pv;\n    \
             pv = 91;\n  end\nendinterface\n\
             module top;\n  import pk::*;\n  ifc i1();\n  \
             initial #1 $display(\"s3_12: pkpv=%0d\", pk::pv);\n  initial #2 $finish;\nendmodule\n"
        ),
        &["s3_12: pkpv=5"],
        &["pkpv=91"],
    );
}

/// s3_13 — two INSTANCES of the interface. Oracles: `pkpv=5` on all three readouts.
/// Was `91` / `92` / `92`: each instance overwrote the one shared package net.
#[test]
fn two_interface_instances_do_not_share_the_block_local() {
    lines(
        &format!(
            "{PK}interface ifc #(parameter integer V = 0) ();\n  import pk::*;\n  \
             initial begin : blk\n    integer pv;\n    pv = V;\n    \
             $display(\"s3_13-i%0d: pv=%0d pkpv=%0d\", V, pv, pk::pv);\n  end\nendinterface\n\
             module top;\n  ifc #(.V(91)) i1();\n  ifc #(.V(92)) i2();\n  \
             initial #1 $display(\"s3_13-top: pkpv=%0d\", pk::pv);\n  initial #2 $finish;\nendmodule\n"
        ),
        &[
            "s3_13-i91: pv=91 pkpv=5",
            "s3_13-i92: pv=92 pkpv=5",
            "s3_13-top: pkpv=5",
        ],
        &["pkpv=91", "pkpv=92"],
    );
}

/// s3_16 — a CONTINUOUS assign inside the interface reading the imported bare name.
/// Oracles: `s3_16: w=5 pkpv=5`. Was `w=99 pkpv=99`.
#[test]
fn an_interface_continuous_assign_reads_the_import() {
    lines(
        &format!(
            "{PK}interface ifc();\n  import pk::*;\n  wire [31:0] w;\n  assign w = pv;\n  \
             initial begin : blk\n    integer pv;\n    pv = 99;\n  end\n  \
             initial #1 $display(\"s3_16: w=%0d pkpv=%0d\", w, pk::pv);\nendinterface\n\
             module top;\n  ifc i1();\n  initial #2 $finish;\nendmodule\n"
        ),
        &["s3_16: w=5 pkpv=5"],
        &["w=99"],
    );
}

/// Control — the interface declares the name ITSELF (no block-local). Correct before
/// and after; the reorder must not disturb the interface's own member resolution.
/// Oracles: `g1: pv=99 pkpv=5`.
#[test]
fn an_interface_own_declaration_wins_the_import() {
    lines(
        &format!(
            "{PK}interface ifc();\n  import pk::*;\n  integer pv;\n  initial begin : blk\n    \
             pv = 99;\n    $display(\"g1: pv=%0d pkpv=%0d\", pv, pk::pv);\n  end\nendinterface\n\
             module top;\n  ifc i1();\n  initial #1 $finish;\nendmodule\n"
        ),
        &["g1: pv=99 pkpv=5"],
        &["pkpv=99"],
    );
}

/// Control — an interface with NO import at all. Byte-identical across PRE and POST:
/// with no alias there is no augmentation and the block-local always minted its own
/// net. Oracles: `g2: pv=99 pkpv=5`.
#[test]
fn an_interface_without_an_import_is_unaffected() {
    lines(
        &format!(
            "{PK}interface ifc();\n  initial begin : blk\n    integer pv;\n    pv = 99;\n    \
             $display(\"g2: pv=%0d pkpv=%0d\", pv, pk::pv);\n  end\nendinterface\n\
             module top;\n  ifc i1();\n  initial #1 $finish;\nendmodule\n"
        ),
        &["g2: pv=99 pkpv=5"],
        &["pkpv=99"],
    );
}
