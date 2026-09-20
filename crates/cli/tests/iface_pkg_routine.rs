//! §3.b `iface-pkg-routine` — an interface body's `import pk::g;` binds ROUTINES, and a
//! bare call inside an interface body resolves in the INTERFACE's scope, not the
//! parent module's.
//!
//! ## The defect
//!
//! An interface instance is flattened INSIDE the parent module's Nets phase
//! (`elaborate/src/instance.rs` pass 4c; pass 8 for a generate-nested one), with the
//! PARENT's `func_table` / `task_table` / `rtn_pkg` / `frame_idx` / `const_func_table`
//! live. `elaborate_iface_instances` (`elaborate/src/iface_inst.rs`) applied the
//! interface's own imports through `apply_import_consts` only — CONSTANTS, TYPES and
//! VARIABLES — and never called `apply_import_routines` nor
//! `apply_import_const_funcs`. Two measured consequences:
//!
//! * every bare routine call in an interface body was `VITA-E3010 call to undeclared
//!   function/task` where both oracles print a value, and a `localparam` folded through
//!   an imported constant function was `VITA-E3009 … has no constant-fold arm`;
//! * worse, and silently: a bare `g()` in the interface body resolved to the PARENT's
//!   `g`. `interface ifc; import pk::g; … r = g(40);` inside a module that declares its
//!   own `function int g` returning `a + 1000` printed `I=1040` at exit 0 where both
//!   oracles print `I=44`. The same with `import pk::*`, and with the parent importing
//!   a DIFFERENT package's `g` (`I=39` vs 44).
//!
//! ## The fix
//!
//! `iface_inst.rs` takes the enclosing module's whole routine scope (`RoutineScope`:
//! the two routine tables, their declaring-scope and call-shape sidecars, the frame-id
//! maps, the two constant-function tables and `scope_imports`) at the `cur_prefix`
//! window entry, leaving the interface instance's own empty one; applies the
//! interface's imports for constant functions and for routines; feeds the imported
//! routine bodies to the block-local classifier through the shared
//! `imported_routine_bodies` collector (the module lane's step 3.6a calls the same
//! one); runs step (6.5)'s `lower_frame_funcs()` after the interface's nets exist and
//! before its Logic loop; and restores the module's scope at the window exit.
//! `wire_ports` stays outside that window — a header-port connection's actual is a
//! PARENT expression. A `function`/`task` DECLARED in an interface body joined the
//! same tables one row later (`iface-subr`, `crates/cli/tests/iface_subr.rs`); the
//! two lanes share `func_table`/`task_table`, so the boundary cell below is pinned
//! here too.
//!
//! ## Oracles
//!
//! Every value pinned here was measured three-way against iverilog 13.0 (`-g2012` +
//! `vvp -n`) and verilator 5.052 (`--binary --timing`). Both agree on every value pin
//! except the two marked verilator-only, where iverilog rejects the design's SYNTAX
//! (not its value) — each names the rejection at its pin.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ifacepkgrtn_{}_{n}", std::process::id()));
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

/// A run that must FAIL with the given diagnostic CODE (not its wording), plus the one
/// phrase of the message that carries the reason.
fn loud(src: &str, code_str: &str, want: &[&str], absent: &[&str]) {
    let (o, code) = run(src);
    assert_ne!(code, Some(0), "expected a loud refusal:\n{o}");
    assert!(o.contains(code_str), "expected {code_str:?} in:\n{o}");
    for w in want {
        assert!(o.contains(w), "expected {w:?} in:\n{o}");
    }
    for a in absent {
        assert!(!o.contains(a), "did NOT expect {a:?} in:\n{o}");
    }
}

/// The census package every cell imports from. `g` reads a package localparam, `h`
/// calls a same-package sibling, `bl` holds a named block-local, `lp` a loop, `tk` an
/// output formal, `tkd` an output formal plus a delay, `rv` a package VARIABLE.
const PK: &str = r#"package pk;
  int pv = 40;
  localparam int PC = 4;
  typedef logic [7:0] byte_t;
  function automatic int g(input int a); return a + PC; endfunction
  function automatic int h(input int a); return g(a) * 2; endfunction
  function int bl(input int a); begin : b int t; t = a * 2; t = t + 1; return t; end endfunction
  function automatic int lp(input int n); int s; s = 0; for (int i = 0; i < n; i++) s += i; return s; endfunction
  task automatic tk(input int a, output int o); o = a + 100; endtask
  task automatic tkd(input int a, output int o); int t; t = a; #1 o = t + 200; endtask
  function automatic int rv(input int a); return a + pv; endfunction
endpackage
"#;

fn with_pk(tail: &str) -> String {
    format!("{PK}\n{tail}")
}

// ----------------------------------------------------- the import binds a routine

/// c1: the row's own cell — an explicit `import pk::g;` in an interface body, called
/// from the interface's own `initial`. Was `VITA-E3010`; both oracles print 44.
#[test]
fn iface_explicit_import_function() {
    lines(
        &with_pk(
            "interface ifc; int r; import pk::g; initial begin r = g(40); $display(\"V=%0d\", r); end endinterface\n\
             module top; ifc i(); initial #2 $finish; endmodule\n",
        ),
        &["V=44"],
        &["VITA-E3010"],
    );
}

/// c2: the WILDCARD spelling of c1. Both oracles print 44.
#[test]
fn iface_wildcard_import_function() {
    lines(
        &with_pk(
            "interface ifc; int r; import pk::*; initial begin r = g(40); $display(\"V=%0d\", r); end endinterface\n\
             module top; ifc i(); initial #2 $finish; endmodule\n",
        ),
        &["V=44"],
        &["VITA-E3010"],
    );
}

/// c3: an imported TASK with an output formal — the frame lane's copy-out path, which
/// `lower_frame_funcs()` is what reserves. Both oracles print 140.
#[test]
fn iface_import_task_output_formal() {
    lines(
        &with_pk(
            "interface ifc; int r; import pk::tk; initial begin tk(40, r); $display(\"V=%0d\", r); end endinterface\n\
             module top; ifc i(); initial #2 $finish; endmodule\n",
        ),
        &["V=140"],
        &["VITA-E3010"],
    );
}

/// c4: an imported function holding a NAMED block-local (`begin : b int t;`) — the
/// shape the block-local classifier has to see the imported body for. Both oracles
/// print 41.
#[test]
fn iface_import_function_block_local() {
    lines(
        &with_pk(
            "interface ifc; int r; import pk::bl; initial begin r = bl(20); $display(\"V=%0d\", r); end endinterface\n\
             module top; ifc i(); initial #2 $finish; endmodule\n",
        ),
        &["V=41"],
        &["VITA-E3010"],
    );
}

/// c5: an imported function that calls a same-package SIBLING (`h` calls `g`), so the
/// transitive-callee injection has to reach this scope's tables too. Both oracles
/// print 48.
#[test]
fn iface_import_function_transitive_callee() {
    lines(
        &with_pk(
            "interface ifc; int r; import pk::h; initial begin r = h(20); $display(\"V=%0d\", r); end endinterface\n\
             module top; ifc i(); initial #2 $finish; endmodule\n",
        ),
        &["V=48"],
        &["VITA-E3010"],
    );
}

/// c6: an imported function whose body reads a package VARIABLE (`pv`), so the body
/// still resolves in the PACKAGE's scope from inside an interface. Both oracles print
/// 44.
#[test]
fn iface_import_function_reads_pkg_var() {
    lines(
        &with_pk(
            "interface ifc; int r; import pk::rv; initial begin r = rv(4); $display(\"V=%0d\", r); end endinterface\n\
             module top; ifc i(); initial #2 $finish; endmodule\n",
        ),
        &["V=44"],
        &["VITA-E3010"],
    );
}

/// c14: an imported function with a LOOP — not reducible to an expression, so it must
/// take the frame route `lower_frame_funcs()` reserves. Both oracles print 45.
#[test]
fn iface_import_function_loop_body() {
    lines(
        &with_pk(
            "interface ifc; int r; import pk::lp; initial begin r = lp(10); $display(\"V=%0d\", r); end endinterface\n\
             module top; ifc i(); initial #2 $finish; endmodule\n",
        ),
        &["V=45"],
        &["VITA-E3010"],
    );
}

/// c15: an imported TASK carrying a DELAY — a suspendable frame task, and the time it
/// finishes at is pinned beside its value. Both oracles print `V=204 T=1`.
#[test]
fn iface_import_task_with_delay() {
    lines(
        &with_pk(
            "interface ifc; int r; import pk::tkd; initial begin tkd(4, r); $display(\"V=%0d T=%0t\", r, $time); end endinterface\n\
             module top; ifc i(); initial #3 $finish; endmodule\n",
        ),
        &["V=204 T=1"],
        &["VITA-E3010"],
    );
}

/// c16: a GENERATE-nested interface instance (`top.gb.i`), which is flattened in the
/// parent's pass 8 rather than pass 4c. Both oracles print 44.
#[test]
fn iface_in_generate_import_function() {
    lines(
        &with_pk(
            "interface ifc; int r; import pk::g; initial begin r = g(40); $display(\"V=%0d\", r); end endinterface\n\
             module top; generate if (1) begin : gb ifc i(); end endgenerate initial #2 $finish; endmodule\n",
        ),
        &["V=44"],
        &["VITA-E3010"],
    );
}

/// c17: a HEADER import (`interface ifc import pk::g; #(parameter …)`), which binds
/// before the header's own parameter defaults. Both oracles print 44.
#[test]
fn iface_header_import_function() {
    lines(
        &with_pk(
            "interface ifc import pk::g; #(parameter int N = 40); int r; initial begin r = g(N); $display(\"V=%0d\", r); end endinterface\n\
             module top; ifc i(); initial #2 $finish; endmodule\n",
        ),
        &["V=44"],
        &["VITA-E3010"],
    );
}

/// c19: the value is read from OUTSIDE, through the interface member (`i.r`), so the
/// call's result has to land in the interface instance's own net. Both oracles print
/// 44.
#[test]
fn iface_import_function_member_read_from_parent() {
    lines(
        &with_pk(
            "interface ifc; int r; import pk::g; initial r = g(40); endinterface\n\
             module top; ifc i(); initial begin #1 $display(\"V=%0d\", i.r); #1 $finish; end endmodule\n",
        ),
        &["V=44"],
        &["VITA-E3010"],
    );
}

/// c21: the CONSTANT half — a `localparam` in an interface body folded through an
/// imported constant function. Was `VITA-E3009 … has no constant-fold arm`; both
/// oracles print 44. The module twin (`module_twin_localparam_const_function`) already
/// folded before this slice and is pinned below unchanged.
#[test]
fn iface_localparam_imported_const_function() {
    lines(
        &with_pk(
            "interface ifc; import pk::g; localparam int W = g(40); int r; initial begin r = W; $display(\"V=%0d\", r); end endinterface\n\
             module top; ifc i(); initial #2 $finish; endmodule\n",
        ),
        &["V=44"],
        &["VITA-E3009", "VITA-E3010"],
    );
}

/// c22: the call sits in an `always_comb`, not an `initial` — a different lowering
/// funnel for the same table. Both oracles print 44.
#[test]
fn iface_import_function_in_always_comb() {
    lines(
        &with_pk(
            "interface ifc; int a, r; import pk::g; always_comb r = g(a); initial begin a = 40; #1 $display(\"V=%0d\", r); end endinterface\n\
             module top; ifc i(); initial #2 $finish; endmodule\n",
        ),
        &["V=44"],
        &["VITA-E3010"],
    );
}

/// c9: TWO instances of one parameterised interface, so the per-instance tables must be
/// taken and restored per instance rather than once per interface. Both oracles print
/// `V40=44` and `V6=10`.
#[test]
fn iface_two_instances_each_import() {
    lines(
        &with_pk(
            "interface ifc #(parameter int A = 1); int r; import pk::g; initial begin r = g(A); $display(\"V%0d=%0d\", A, r); end endinterface\n\
             module top; ifc #(.A(40)) i(); ifc #(.A(6)) j(); initial #2 $finish; endmodule\n",
        ),
        &["V40=44", "V6=10"],
        &["VITA-E3010"],
    );
}

// ------------------------------------ the silent-wrong: the parent's routine answered

/// c10: the interface imports `pk::g` (+PC = +4) and the PARENT declares its own
/// `function int g` (+1000). PRE printed `I=1040` at exit 0; both oracles print
/// `I=44 M=1040`. The parent's own call must still reach the parent's `g`.
#[test]
fn iface_import_wins_over_parent_function() {
    lines(
        &with_pk(
            "interface ifc; int r; import pk::g; initial begin r = g(40); $display(\"I=%0d\", r); end endinterface\n\
             module top; function int g(input int a); return a + 1000; endfunction int q; ifc i(); initial begin q = g(40); $display(\"M=%0d\", q); #2 $finish; end endmodule\n",
        ),
        &["I=44", "M=1040"],
        &["I=1040"],
    );
}

/// c25: the WILDCARD spelling of c10 — `import pk::*` in the interface, parent declares
/// `g`. PRE `I=1040`; both oracles `I=44 M=1040`.
#[test]
fn iface_wildcard_import_wins_over_parent_function() {
    lines(
        &with_pk(
            "interface ifc; int r; import pk::*; initial begin r = g(40); $display(\"I=%0d\", r); end endinterface\n\
             module top; function int g(input int a); return a + 1000; endfunction int q; ifc i(); initial begin q = g(40); $display(\"M=%0d\", q); #2 $finish; end endmodule\n",
        ),
        &["I=44", "M=1040"],
        &["I=1040"],
    );
}

/// c26: both scopes IMPORT a `g`, from different packages — the interface `pk::g`
/// (+4), the parent `pq::g` (-1). PRE printed the parent's for both (`I=39`); both
/// oracles print `I=44 M=39`.
#[test]
fn iface_import_wins_over_parent_import() {
    lines(
        &with_pk(
            "package pq; function automatic int g(input int a); return a - 1; endfunction endpackage\n\
             interface ifc; int r; import pk::g; initial begin r = g(40); $display(\"I=%0d\", r); end endinterface\n\
             module top; import pq::g; int q; ifc i(); initial begin q = g(40); $display(\"M=%0d\", q); #2 $finish; end endmodule\n",
        ),
        &["I=44", "M=39"],
        &["I=39"],
    );
}

// ------------------------------------------------ the import does NOT leak outward

/// c8: the INTERFACE imports `pk::g` and the PARENT calls a bare `g()`. The interface's
/// import is not visible in the enclosing module, so the parent's call stays loud —
/// named `[in top]`, and the interface's own call is no longer loud. Both oracles
/// refuse the design for the same call (iverilog: "No function named `g' found in this
/// context (top)"; verilator: "Can't find definition of task/function: 'g'").
#[test]
fn iface_import_not_visible_in_parent() {
    loud(
        &with_pk(
            "interface ifc; import pk::g; int r; initial r = g(1); endinterface\n\
             module top; int q; ifc i(); initial begin q = g(40); $display(\"V=%0d\", q); #2 $finish; end endmodule\n",
        ),
        "VITA-E3010",
        &["[in top]"],
        &["[in top.i]"],
    );
}

/// c23: two SIBLING interfaces — `ia` imports `pk::g`, `ib` does not and calls `g()`.
/// The import is scoped to `ia`, so only `top.b` is loud. Both oracles refuse the
/// design for the `ib` call alone (iverilog names the context `top.b`).
#[test]
fn iface_import_not_visible_in_sibling_iface() {
    loud(
        &with_pk(
            "interface ia; int r; import pk::g; initial begin r = g(40); $display(\"A=%0d\", r); end endinterface\n\
             interface ib; int r; initial begin r = g(1); $display(\"B=%0d\", r); end endinterface\n\
             module top; ia a(); ib b(); initial #2 $finish; endmodule\n",
        ),
        "VITA-E3010",
        &["[in top.b]"],
        &["[in top.a]"],
    );
}

/// c7: the PARENT imports `pk::g` and the interface body — which has no import of its
/// own — calls a bare `g()`. This is the ORACLE SPLIT of the slice: iverilog runs it
/// and prints `V=44`, verilator refuses it ("Can't find definition of task/function:
/// 'g'"). IEEE 1800-2017 §26.3 makes an import visible in the importing scope and in
/// the scopes lexically NESTED inside it; an `interface` declaration is not nested in
/// the module that instantiates it, so vita follows verilator here. The cell was a
/// silent `V=44` before this slice for the same reason c10 was `I=1040` — the parent's
/// table was live — so it moves with that mechanism, not on its own decision.
#[test]
fn iface_without_import_does_not_see_parent_import() {
    loud(
        &with_pk(
            "interface ifc; int r; initial begin r = g(40); $display(\"V=%0d\", r); end endinterface\n\
             module top; import pk::g; ifc i(); initial #2 $finish; endmodule\n",
        ),
        "VITA-E3010",
        &["[in top.i]"],
        &["V=44"],
    );
}

// --------------------------------------------------------------- one-oracle cells

/// c24: the interface instance is passed to a child module through an interface port
/// (`module sub(ifc p)`), and the value is read as `p.r`. verilator-ONLY: iverilog 13
/// rejects the `module sub(ifc p);` header outright ("syntax error … Errors in port
/// declarations"), which is a syntax refusal, not a disagreement about the value.
/// verilator prints `V=44`.
#[test]
fn iface_import_function_through_iface_port() {
    lines(
        "package pk;\n  localparam int PC = 4;\n  function automatic int g(input int a); return a + PC; endfunction\nendpackage\n\
         interface ifc; int r; import pk::g; modport mp(input r); initial r = g(40); endinterface\n\
         module sub(ifc p); initial begin #1 $display(\"V=%0d\", p.r); end endmodule\n\
         module top; ifc i(); sub s(i); initial #2 $finish; endmodule\n",
        &["V=44"],
        &["VITA-E3010"],
    );
}

/// c20: an imported `void` function with an OUTPUT formal, called as a statement — the
/// other half of the copy-out route c3 exercises. verilator-ONLY: iverilog 13 rejects a
/// function with a non-input port ("Function pk.fo port o is not an input port"), which
/// is a declaration refusal, not a disagreement about the value. verilator prints
/// `V=42`.
#[test]
fn iface_import_void_function_output_formal() {
    lines(
        &format!(
            "{}{}",
            PK.replace(
                "endpackage",
                "  function automatic void fo(input int a, output int o); o = a * 3; endfunction\nendpackage"
            ),
            "\ninterface ifc; int r; import pk::fo; initial begin fo(14, r); $display(\"V=%0d\", r); end endinterface\n\
             module top; ifc i(); initial #2 $finish; endmodule\n",
        ),
        &["V=42"],
        &["VITA-E3010"],
    );
}

// ------------------------------------------ cells that must NOT move (control twins)

/// c11: a HEADER wildcard import feeding a header parameter DEFAULT — the constant half
/// that already worked. Both oracles print 44 before and after.
#[test]
fn iface_header_import_constant_unchanged() {
    lines(
        &with_pk(
            "interface ifc import pk::*; #(parameter int N = PC); int r; initial begin r = N * 11; $display(\"V=%0d\", r); end endinterface\n\
             module top; ifc i(); initial #2 $finish; endmodule\n",
        ),
        &["V=44"],
        &["VITA-E3010"],
    );
}

/// c12: an imported TYPE used to declare an interface member. Both oracles print 44
/// before and after.
#[test]
fn iface_import_typedef_unchanged() {
    lines(
        &with_pk(
            "interface ifc; import pk::byte_t; byte_t r; initial begin r = 8'h2c; $display(\"V=%0d\", r); end endinterface\n\
             module top; ifc i(); initial #2 $finish; endmodule\n",
        ),
        &["V=44"],
        &["VITA-E3010"],
    );
}

/// c13: an imported package VARIABLE read from an interface body. Both oracles print 44
/// before and after.
#[test]
fn iface_import_pkg_var_unchanged() {
    lines(
        &with_pk(
            "interface ifc; import pk::pv; int r; initial begin r = pv + 4; $display(\"V=%0d\", r); end endinterface\n\
             module top; ifc i(); initial #2 $finish; endmodule\n",
        ),
        &["V=44"],
        &["VITA-E3010"],
    );
}

/// c18: the SCOPED spelling `pk::g(40)` with no import at all, which reserved its frame
/// from inside this same window before the slice. Both oracles print 44 before and
/// after.
#[test]
fn iface_scoped_call_unchanged() {
    lines(
        &with_pk(
            "interface ifc; int r; initial begin r = pk::g(40); $display(\"V=%0d\", r); end endinterface\n\
             module top; ifc i(); initial #2 $finish; endmodule\n",
        ),
        &["V=44"],
        &["VITA-E3010"],
    );
}

/// m1: the MODULE twin of c1 — the lane this slice does not touch. Both oracles print
/// 44 before and after.
#[test]
fn module_twin_import_function() {
    lines(
        &with_pk(
            "module top; int r; import pk::g; initial begin r = g(40); $display(\"V=%0d\", r); #2 $finish; end endmodule\n",
        ),
        &["V=44"],
        &["VITA-E3010"],
    );
}

/// m21: the MODULE twin of c21 — a `localparam` folded through an imported constant
/// function, which the module lane already did. Both oracles print 44 before and after.
#[test]
fn module_twin_localparam_const_function() {
    lines(
        &with_pk(
            "module top; import pk::g; localparam int W = g(40); int r; initial begin r = W; $display(\"V=%0d\", r); #2 $finish; end endmodule\n",
        ),
        &["V=44"],
        &["VITA-E3009", "VITA-E3010"],
    );
}

// ------------------------------------------------- the refusal row `iface-subr` lifted

/// The neighbouring row: a `function` DECLARED in an interface body. It was
/// `VITA-E3009 functions/tasks inside an interface are outside the MVP` plus a
/// `VITA-E3010` at the call when this file was written; row `iface-subr` registers a
/// declared routine in this window's own `func_table` and both oracles' `V=44` runs.
/// Kept here as the boundary pin of THIS row — the import lane and the declaration
/// lane share one table, so a change to either must keep this cell at 44.
#[test]
fn iface_declared_function_runs() {
    lines(
        "interface ifc; int r; function int f(input int a); return a + 4; endfunction\n\
         initial begin r = f(40); $display(\"V=%0d\", r); end endinterface\n\
         module top; ifc i(); initial #2 $finish; endmodule\n",
        &["V=44"],
        &["VITA-E3009", "VITA-E3010"],
    );
}

// ------------------------------------------------------ the instance scope is the interface's

/// Review round 1 (soundness S-1): `%m` inside an imported routine called from an
/// interface instance names THAT instance, exactly as the module lane names its own
/// (`top.u.g` beside `top.w.g`). Before the fix the interface cell printed `top.g` —
/// the parent's `inst_prefix` was still live inside the window, so the instance
/// segment was dropped from `%m` and from the OBS `subroutine_calls[].name`. This is
/// a PARITY pin between the two lanes, not an oracle pin: what `%m` prints inside a
/// package routine is a recorded oracle split (ROADMAP §2 — iverilog `pk::g`,
/// verilator `pk.g`, vita the calling instance).
#[test]
fn iface_imported_routine_percent_m_names_the_instance() {
    lines(
        "package pk;\n\
         function automatic int g(input int a); int s; s = 0; for (int i = 0; i < a; i++) s += i; $display(\"SCOPE=%m\"); return s; endfunction\n\
         endpackage\n\
         interface ifc; int r; import pk::g; initial begin r = g(3); end endinterface\n\
         module mm; import pk::g; int q; initial begin q = g(3); end endmodule\n\
         module top; ifc u(); mm w(); initial begin #1 $finish; end endmodule\n",
        &["SCOPE=top.u.g", "SCOPE=top.w.g"],
        &["SCOPE=top.g\n"],
    );
}

// ------------------------------------ the static scoped frame travels between instances

/// The package whose `stat` is NOT `automatic`: `x` is ONE variable for the whole
/// design (IEEE §6.21), so consecutive calls accumulate.
const PK_STAT: &str = r#"package pk;
  function int stat(input int a); int x; x = x + a; return x; endfunction
endpackage
"#;

/// Review round 2 (differential F1): two instances of an interface whose body makes
/// the SAME scoped call share one frame, hence one copy of the static local. Round 1
/// gave each instance its own `frame_idx` and printed `L1=10 L2=10`, a REGRESSION on a
/// cell the pre-slice build already had right. Both oracles print `L1=10 L2=20`.
#[test]
fn iface_instances_share_a_static_scoped_frame() {
    lines(
        &format!(
            "{PK_STAT}\n\
             interface ifc #(parameter int D = 1) (); int l; initial begin #D l = pk::stat(10); $display(\"L%0d=%0d\", D, l); end endinterface\n\
             module top; ifc #(1) u1(); ifc #(2) u2(); initial #5 $finish; endmodule\n"
        ),
        &["L1=10", "L2=20"],
        &["L2=10"],
    );
}

/// The same across THREE instances, each call carrying a different argument, so the
/// pin reads the accumulation rather than one repeated sum: 10, 10+20, 30+30. Round 1
/// printed `L1=10 L2=20 L3=30` (three separate `x`). Both oracles print 10 / 30 / 60.
#[test]
fn iface_instances_accumulate_in_one_static_scoped_local() {
    lines(
        &format!(
            "{PK_STAT}\n\
             interface ifc #(parameter int A = 1) (); int l; initial begin #A l = pk::stat(A*10); $display(\"L%0d=%0d\", A, l); end endinterface\n\
             module top; ifc #(1) u1(); ifc #(2) u2(); ifc #(3) u3(); initial #9 $finish; endmodule\n"
        ),
        &["L1=10", "L2=30", "L3=60"],
        &["L2=20", "L3=30"],
    );
}

/// The BOUNDARY of that sharing: the parent module's own scoped `pk::stat(10)` gets
/// its own frame, so `M` reads 10 rather than continuing the interfaces' `x` at 13.
///
/// `M` is not asserted: vita prints `M=10` where both oracles print `M=13`, the recorded
/// ROADMAP §2 row "a PACKAGE routine's static local is ONE variable", which the module
/// lane owns and this row does not close. `L1`/`L2` are pinned because the pre-slice
/// build printed exactly `L1=1 L2=3 M=10`: the carry between interface instances must
/// not reach the parent's own tables. Depositing it there was measured twice — it left
/// a static root's AUTOMATIC callee behind (`E3010 call to undeclared function h` on a
/// design both oracles run), and it handed a parent's scoped call a frame an interface
/// had lowered, turning an `E3010` into a silent `V=2` where both oracles say `V=3`.
#[test]
fn parent_scoped_call_keeps_its_own_static_scoped_frame() {
    lines(
        &format!(
            "{PK_STAT}\n\
             interface ifc #(parameter int N = 1) (); int l; initial begin #N l = pk::stat(N); $display(\"L%0d=%0d\", N, l); end endinterface\n\
             module top;\n\
               ifc #(1) u1();\n\
               ifc #(2) u2();\n\
               int q;\n\
               initial begin #4 q = pk::stat(10); $display(\"M=%0d\", q); end\n\
               initial #6 $finish;\n\
             endmodule\n"
        ),
        &["L1=1", "L2=3"],
        &["L2=2"],
    );
}

/// The other side of the predicate: an AUTOMATIC package routine stays per-instance.
/// `tkd` suspends for 1 ns between reading its input and writing its output, so two
/// instances whose calls overlap in time would corrupt each other's locals if they
/// shared a frame. Both oracles print `O1=201 @1` and `O7=207 @1`.
#[test]
fn iface_instances_do_not_share_an_automatic_frame() {
    lines(
        "package pk;\n\
         task automatic tkd(input int a, output int o); int t; t = a; #1 o = t + 200; endtask\n\
         endpackage\n\
         interface ifc #(parameter int A = 1) (); import pk::tkd; int o; initial begin tkd(A, o); $display(\"O%0d=%0d @%0d\", A, o, $time); end endinterface\n\
         module top; ifc #(1) u1(); ifc #(7) u2(); initial #9 $finish; endmodule\n",
        &["O1=201 @1", "O7=207 @1"],
        &[],
    );
}

/// Review round 2 (differential G1 / soundness S2-3): an interface instance whose OWN
/// import lane bound a static callee under its `px::h` key (`import px::g;` where `g`
/// calls non-automatic `h`) and whose body also calls `px::h(100)` directly must hold
/// ONE copy of `h`'s static local, not two. The first carry adopted the sibling's
/// `px::h` frame over the one this window had already lowered `g` against, so the
/// direct call walked the carried frame (`M2=201 M3=301`) while `g` kept the
/// instance's own. The adopt now leaves a key the window already defines alone.
///
/// The values are vita's per-scope line (the round-1 build printed the same); both
/// oracles print `A2=103 M2=203 A3=206 M3=306` — ONE design-wide static local, the
/// recorded ROADMAP §2 row this slice does not close. Pinned for the two-copies
/// shape only: `M<n>` must equal `A<n> + 100` in every instance.
#[test]
fn iface_import_lane_static_callee_is_one_copy_per_instance() {
    lines(
        "package px;\n\
         function int h(input int a); int x; x = x + a; return x; endfunction\n\
         function int g(input int a); return h(a); endfunction\n\
         endpackage\n\
         interface ifc #(parameter int D = 1) ();\n\
         import px::g;\n\
         int l; int m;\n\
         initial begin #D l = g(D); m = px::h(100); $display(\"A%0d=%0d M%0d=%0d\", D, l, D, m); end\n\
         endinterface\n\
         module top; ifc #(1) u1(); ifc #(2) u2(); ifc #(3) u3(); initial #9 $finish; endmodule\n",
        &["A1=1 M1=101", "A2=2 M2=102", "A3=3 M3=103"],
        &["M2=201", "M3=301"],
    );
}
