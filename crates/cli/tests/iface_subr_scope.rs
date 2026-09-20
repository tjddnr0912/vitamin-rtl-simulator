//! §3.b `iface-subr`, the NAME-RESOLUTION half — which binding a name in an
//! interface body resolves to, and in what order it may be used.
//!
//! Split out of `iface_subr.rs` at the 1000-line policy. That file pins the row's
//! VALUES (a declared routine runs, and where); this one pins the rules that decide
//! WHICH declaration answers a name and WHEN a name exists:
//!
//! * IEEE §26.3 — an explicit import of a name the scope declares itself is an error,
//!   over both halves of the §3.13 routine name space (a task and a function share
//!   one). Two tables answer it — the constant-function table and the runtime routine
//!   tables — and exactly one of them may say so, which some cells pin by COUNT.
//! * IEEE §26.4 — a COMPILATION-UNIT (`$unit`) import is an OUTER scope that a local
//!   declaration SHADOWS. It is not the §26.3 error, and telling the two apart is the
//!   `is_cu` discriminator both lanes now thread.
//! * an interface MODPORT shares that name space too.
//! * IEEE §6.10 — a name declared later in the scope is not declared at this point.
//!
//! Every cell here has a MODULE twin wherever a module can express the shape, because
//! the machinery is shared and the module lane is where a regression would land first.
//!
//! ## Oracles
//!
//! Measured three-way against iverilog 13.0 (`-g2012` + `vvp -n`) and verilator 5.052
//! (`--binary --timing`). The §26.3 and §6.10 cells are oracle SPLITS — iverilog
//! rejects, verilator answers the local — so they pin the refusal, never a value; each
//! test's doc names both positions. The §26.4 cells are not splits: both oracles run
//! them and agree, so those pin values.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ifacesubrsc_{}_{n}", std::process::id()));
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

/// The two diagnostics row `iface-subr` removed. Passed as `absent` by every value cell.
const GONE: &[&str] = &["VITA-E3009", "VITA-E3010"];

// ------------------------------- an explicit import colliding with a declaration

/// The refusal every cell in this section pins. IEEE §26.3: an explicit import of a
/// name the importing scope declares itself is an error.
const IMPORT_CONFLICT: &str = "conflicts with a local declaration of the same name";

/// A run that must FAIL carrying every `want` phrase, and must never crash.
fn loud(src: &str, want: &[&str]) {
    let (o, code) = run(src);
    assert_ne!(code, Some(0), "expected a loud refusal:\n{o}");
    for w in want {
        assert!(o.contains(w), "expected {w:?} in:\n{o}");
    }
    assert!(!o.contains("panicked"), "{o}");
}

/// [`loud`] plus the COUNT: `phrase` appears exactly once and the run reports exactly
/// one error. Two lanes answer the import/declaration collision — the constant-function
/// table's and the routine tables' — and only one of them may SAY so; the round-2 build
/// printed the line twice and `errors=1` became `errors=2` in both the interface and
/// the module lane.
fn loud_once(src: &str, phrase: &str) {
    let (o, code) = run(src);
    assert_ne!(code, Some(0), "expected a loud refusal:\n{o}");
    assert_eq!(
        o.matches(phrase).count(),
        1,
        "expected {phrase:?} EXACTLY once in:\n{o}"
    );
    assert!(o.contains("errors=1 "), "expected `errors=1` in:\n{o}");
    assert!(!o.contains("panicked"), "{o}");
}

/// d03 / s27: an EXPLICIT `import pk::g` beside a declared FUNCTION `g`. iverilog
/// rejects ("'g' has already been imported into this scope from package 'pk'");
/// verilator answers the local (`R=1040`). Loud is the honest answer to an oracle
/// split, and the CONSTANT-function lane has refused this spelling since
/// `iface-pkg-routine`; round-2 soundness S-1 added the runtime lane's own guard, so
/// the refusal now also covers the table the call actually reads.
///
/// Pinned with the COUNT (round-3 differential R2-2): a FUNCTION collision is the one
/// spelling BOTH lanes can see, so exactly one of them speaks.
#[test]
fn an_explicit_import_colliding_with_a_declared_function_is_loud_once() {
    loud_once(
        "package pk; function automatic int g(input int a); return a + 4; endfunction endpackage\n\
         interface ifc; import pk::g; int r;\n\
         \x20 function automatic int g(input int a); return a + 1000; endfunction\n\
         \x20 initial begin r = g(40); $display(\"R=%0d\", r); end\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        IMPORT_CONFLICT,
    );
}

/// m03: the MODULE twin of the cell above, also pinned by COUNT — the doubling was
/// never interface-specific (the constant lane and the runtime lane are shared code),
/// so the module lane is where a re-introduced duplicate would show first.
#[test]
fn the_module_twin_of_the_function_collision_is_loud_once() {
    loud_once(
        "package pk; function automatic int g(input int a); return a + 4; endfunction endpackage\n\
         module ifc; import pk::g; int r;\n\
         \x20 function automatic int g(input int a); return a + 1000; endfunction\n\
         \x20 initial begin r = g(40); $display(\"R=%0d\", r); end\n\
         endmodule\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        IMPORT_CONFLICT,
    );
}

/// s26 — the TASK spelling, and round-2 soundness S-1's BLOCKING cell. The runtime
/// explicit-import arm inserted unconditionally, and `local_const_funcs` is filled
/// from `ModuleItem::Func` only, so no guard saw a declared TASK: the package's `t`
/// displaced the declaration and the design printed `R=44` — a value NEITHER oracle
/// produces (verilator `R=1040`, iverilog rejects). PRE was loud, so that was a step
/// DOWN the ladder on a shape this row opened.
/// Pinned by COUNT too: the constant lane structurally cannot see a TASK collision, so
/// the runtime lane is the only speaker and must speak exactly once.
#[test]
fn an_explicit_import_colliding_with_a_declared_task_is_loud_once() {
    loud_once(
        "package pk; task automatic t(input int a, output int o); o = a + 4; endtask endpackage\n\
         interface ifc; import pk::t; int r;\n\
         \x20 task automatic t(input int a, output int o); o = a + 1000; endtask\n\
         \x20 initial begin t(40, r); $display(\"R=%0d\", r); end\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        IMPORT_CONFLICT,
    );
}

/// s26m — the MODULE twin of the cell above. It is the control that says the defect
/// was never interface-specific: the module lane printed the same `R=44` before AND
/// after row `iface-subr`, a pre-existing silent-wrong that the shared guard climbs to
/// honest-loud. Both lanes must answer the same way, which is why both are pinned.
#[test]
fn the_module_twin_of_the_import_task_collision_is_loud_too() {
    loud(
        "package pk; task automatic t(input int a, output int o); o = a + 4; endtask endpackage\n\
         module ifc; import pk::t; int r;\n\
         \x20 task automatic t(input int a, output int o); o = a + 1000; endtask\n\
         \x20 initial begin t(40, r); $display(\"R=%0d\", r); end\n\
         endmodule\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["VITA-E3009", IMPORT_CONFLICT],
    );
}

/// s28: CROSS-NAMESPACE — a package FUNCTION `g` imported beside a declared TASK `g`.
/// IEEE §3.13 gives tasks and functions ONE name space, so the guard asks both tables
/// in one test. Before it, the two rows of one name reached the frame reserver and the
/// message named a synthesized net (`top.i.$func$g.a redeclared`) instead of the
/// collision the user wrote (round-2 soundness S-2). iverilog rejects; verilator
/// `R=1040`.
#[test]
fn an_imported_function_colliding_with_a_declared_task_is_loud() {
    loud(
        "package pk; function automatic int g(input int a); return a + 4; endfunction endpackage\n\
         interface ifc; import pk::g; int r;\n\
         \x20 task automatic g(input int a, output int o); o = a + 1000; endtask\n\
         \x20 initial begin g(40, r); $display(\"R=%0d\", r); end\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["VITA-E3009", IMPORT_CONFLICT],
    );
}

/// s30: the other half of the same name space — a package TASK `t` imported beside a
/// declared FUNCTION `t`. The constant lane cannot reach this one at all (it returns
/// early when the package has no FUNCTION of the name), so it is the cell that proves
/// the runtime guard is doing the work rather than riding the constant one.
#[test]
fn an_imported_task_colliding_with_a_declared_function_is_loud() {
    loud(
        "package pk; task automatic t(input int a, output int o); o = a + 4; endtask endpackage\n\
         interface ifc; import pk::t; int r;\n\
         \x20 function automatic int t(input int a); return a + 1000; endfunction\n\
         \x20 initial begin r = t(40); $display(\"R=%0d\", r); end\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["VITA-E3009", IMPORT_CONFLICT],
    );
}

/// s29: the WILDCARD arm is NOT the explicit arm and must stay silent — §26.3 says a
/// local declaration simply WINS a wildcard import, with no diagnostic. Both oracles
/// run it and print `R=1040`. This is the false-loud control for the guard above: a
/// guard written one level up would have refused this working design.
#[test]
fn a_wildcard_import_still_loses_to_a_declared_task_without_a_diagnostic() {
    lines(
        "package pk; task automatic t(input int a, output int o); o = a + 4; endtask endpackage\n\
         interface ifc; import pk::*; int r;\n\
         \x20 task automatic t(input int a, output int o); o = a + 1000; endtask\n\
         \x20 initial begin t(40, r); $display(\"R=%0d\", r); end\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["R=1040"],
        GONE,
    );
}

// ------------------------------- a $unit import is SHADOWED, never a collision

/// r01 (round-3 soundness R2-1): the same collision written at COMPILATION-UNIT
/// scope. §26.3's error is about an import IN the declaring scope; a `$unit` import is
/// an OUTER scope, and §26.4 lets the local declaration simply shadow it — both
/// oracles print `R=1040`. Both lanes chain `cu_imports` in front of the scope's own
/// imports, so without the `i < n_cu` discriminator the guard refused this legal
/// design. The MODULE lane is pinned first because that is where the guard is shared
/// code and where PRE answered `R=44`, a value neither oracle prints.
#[test]
fn a_unit_scope_import_is_shadowed_by_a_declared_task() {
    lines(
        "package pk; task automatic t(input int a, output int o); o = a + 4; endtask endpackage\n\
         import pk::t;\n\
         module top; int r;\n\
         \x20 task automatic t(input int a, output int o); o = a + 1000; endtask\n\
         \x20 initial begin t(40, r); $display(\"R=%0d\", r); #2 $finish; end\n\
         endmodule\n",
        &["R=1040"],
        GONE,
    );
}

/// r01i: the INTERFACE twin of the cell above — the same `i < n_cu` index over the
/// interface window's own chained import list. Both oracles `R=1040`.
#[test]
fn a_unit_scope_import_is_shadowed_by_an_interface_declared_task() {
    lines(
        "package pk; task automatic t(input int a, output int o); o = a + 4; endtask endpackage\n\
         import pk::t;\n\
         interface ifc; int r;\n\
         \x20 task automatic t(input int a, output int o); o = a + 1000; endtask\n\
         \x20 initial begin t(40, r); $display(\"R=%0d\", r); end\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["R=1040"],
        GONE,
    );
}

/// r02: the FUNCTION spelling, which the CONSTANT lane's own guard carried the same
/// defect for — false-loud on a legal design since `iface-pkg-routine`, and printed
/// twice once the runtime guard joined it. One `is_cu` discriminator closes both
/// lanes. Both oracles `R=1040`.
#[test]
fn a_unit_scope_import_is_shadowed_by_a_declared_function() {
    lines(
        "package pk; function automatic int g(input int a); return a + 4; endfunction endpackage\n\
         import pk::g;\n\
         module top; int r;\n\
         \x20 function automatic int g(input int a); return a + 1000; endfunction\n\
         \x20 initial begin r = g(40); $display(\"R=%0d\", r); #2 $finish; end\n\
         endmodule\n",
        &["R=1040"],
        GONE,
    );
}

// ------------------------------------ IEEE §6.10 use before declaration

/// The §6.10 refusal these cells pin. Pinned as a PHRASE, not the whole sentence: the
/// message is the module lane's and both lanes must print the same one.
const USED_BEFORE_DECL: &str = "is used before it is declared";

/// r17 (round-3 soundness R2-2): a declared function reading a net declared BELOW it,
/// called from a decl-initializer. `check_decl_precedes_use` is keyed on `decl_pos`,
/// `decl_pos_scope` and `decl_pos_range`, and the interface window installed none of
/// them, so the gate returned at its range test for every name in an interface body —
/// vacuous. The cell then printed `R=3 L=100` at exit 0: `later` read as 0, a value
/// NEITHER oracle produces (iverilog rejects "Check for declaration after use",
/// verilator `R=303`). The module twin below is loud and always was.
#[test]
fn a_declared_function_reading_a_later_declaration_is_loud() {
    loud(
        "interface ifc;\n\
         \x20 function automatic int lp(input int n); int s; s = 0; for (int k = 0; k < n; k++) s = s + k + later; return s; endfunction\n\
         \x20 int r = lp(3);\n\
         \x20 int later = 100;\n\
         \x20 initial $display(\"R=%0d L=%0d\", r, later);\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["VITA-E3010", USED_BEFORE_DECL],
    );
}

/// r17m: the MODULE twin — loud before and after, which is what makes the interface
/// cell a lane-parity gap rather than a new rule.
#[test]
fn the_module_twin_of_the_use_before_declaration_is_loud() {
    loud(
        "module ifc;\n\
         \x20 function automatic int lp(input int n); int s; s = 0; for (int k = 0; k < n; k++) s = s + k + later; return s; endfunction\n\
         \x20 int r = lp(3);\n\
         \x20 int later = 100;\n\
         \x20 initial $display(\"R=%0d L=%0d\", r, later);\n\
         endmodule\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["VITA-E3010", USED_BEFORE_DECL],
    );
}

/// r21: the FALSE-LOUD control — the same design with `later` declared ABOVE the
/// function. All four tools print `R=303 L=100`. It is what says the gate fires on the
/// ORDER and not on the shape, and that `R=3` was never an ordering opinion.
#[test]
fn a_declaration_above_its_use_is_correct() {
    lines(
        "interface ifc;\n\
         \x20 int later = 100;\n\
         \x20 function automatic int lp(input int n); int s; s = 0; for (int k = 0; k < n; k++) s = s + k + later; return s; endfunction\n\
         \x20 int r = lp(3);\n\
         \x20 initial $display(\"R=%0d L=%0d\", r, later);\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["R=303 L=100"],
        GONE,
    );
}

/// The PROC half of the same vacuity, which predates this row entirely: an interface
/// `initial` reading two names declared below it printed `R=100 L=100` at exit 0 on
/// PRE and through round 2. Installing the tables gates every body in the window, so
/// it now carries exactly the module twin's two messages. iverilog rejects ("Could not
/// find variable ``r'' … Check for declaration after use"); verilator prints
/// `R=100 L=100`, so this is an oracle split answered with honest-loud.
#[test]
fn an_interface_process_reading_a_later_declaration_is_loud() {
    loud(
        "interface ifc;\n\
         \x20 initial r = later;\n\
         \x20 int later = 100;\n\
         \x20 int r;\n\
         \x20 initial #1 $display(\"R=%0d L=%0d\", r, later);\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["VITA-E3010", USED_BEFORE_DECL],
    );
}

/// The MODULE twin of the proc cell — byte-identical diagnostics, PRE and POST.
#[test]
fn the_module_twin_of_the_process_use_before_declaration_is_loud() {
    loud(
        "module ifc;\n\
         \x20 initial r = later;\n\
         \x20 int later = 100;\n\
         \x20 int r;\n\
         \x20 initial #1 $display(\"R=%0d L=%0d\", r, later);\n\
         endmodule\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["VITA-E3010", USED_BEFORE_DECL],
    );
}

/// s36 (round-2 soundness S-4): a MODPORT and a declared routine of the same name.
/// An interface's modports and its routines share one name space, so both oracles
/// REJECT — iverilog "'mp' has already been declared in this scope. … It was declared
/// here as a function."; verilator "MODPORT 'mp' has the same name as function: 'mp'".
/// Both namespaces only became live together in this window with this row and nothing
/// compared them, so POST ran the design and printed `R=44`. There is no module twin:
/// a module has no modport.
#[test]
fn a_modport_named_like_a_declared_routine_is_loud() {
    loud(
        "interface ifc; int r;\n\
         \x20 function automatic int mp(input int a); return a + 4; endfunction\n\
         \x20 modport mp (input r);\n\
         \x20 initial begin r = mp(40); $display(\"R=%0d\", r); end\n\
         endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &[
            "VITA-E3009",
            "modport `mp` has the same name as a function/task declared in this interface",
        ],
    );
}
