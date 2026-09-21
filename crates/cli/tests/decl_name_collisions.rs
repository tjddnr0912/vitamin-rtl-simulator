//! A ROUTINE declared twice, or declared with the name of another declaration of the
//! same scope — refused (IEEE 1800-2017 §3.13).
//!
//! ROADMAP §2 rows P20 (two `function`/`task` declarations of one name in a module,
//! interface, package or transparent `generate` region) and P21 (a routine against a
//! net, variable, parameter, localparam, port, genvar, instance name or named block,
//! in either declaration order). The other binder pairs, the refusal SENTENCE, the
//! modport call and the negative pins live in `decl_name_collisions_kinds.rs`.
//!
//! Every refusing cell below is a design BOTH oracles reject and vita ran at exit 0 —
//! the PRE values are recorded per cell. Every accepting cell is a design both oracles
//! run, and its `$display` text is pinned verbatim: this rule is a REJECT gate, so the
//! controls are what stop it widening.
//!
//! ORACLES: iverilog 13.0 (`-g2012`), Verilator 5.052 (`--binary --timing`).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns `(exit_code, stdout+stderr)`. The code matters as much as the text: a
/// refusal that did not set the exit status would let a CI script run the design.
fn run_args(src: &str, extra: &[&str]) -> (i32, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dnc_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vita"));
    for a in extra {
        cmd.arg(a);
    }
    let out = cmd
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&d);
    (out.status.code().unwrap_or(-1), text)
}

fn run(src: &str) -> (i32, String) {
    run_args(src, &[])
}

/// The §3.13 refusal, pinned by CODE plus the three things a reader needs: the
/// duplicated NAME, both BINDERS in declaration order, and a second location for the
/// first declaration. `first`/`dup` are the article-carrying words ("a function",
/// "an instance"), so a pair printed in the wrong order fails here.
fn assert_collision(name: &str, src: &str, ident: &str, unit: &str, first: &str, dup: &str) {
    let (rc, out) = run(src);
    assert_eq!(rc, 1, "{name}: a refused design must exit 1:\n{out}");
    assert!(
        out.contains("VITA-E3009"),
        "{name}: the refusal is E3009:\n{out}"
    );
    // Two declarations of ONE kind get their own sentence — "both times as a port",
    // not "as a port and as a port".
    let want = if first == dup {
        format!("`{ident}` is declared twice in this {unit}, both times as {dup}")
    } else {
        format!("`{ident}` is declared twice in this {unit}: as {first} and as {dup}")
    };
    assert!(out.contains(&want), "{name}: expected `{want}`:\n{out}");
    assert!(
        out.contains("the first declaration of that name is here"),
        "{name}: a note points at the FIRST declaration:\n{out}"
    );
}

/// A design both oracles RUN. `needles` are the observed `$display` lines.
fn assert_runs(name: &str, src: &str, needles: &[&str]) {
    let (rc, out) = run(src);
    assert_eq!(rc, 0, "{name}: this design is legal and must run:\n{out}");
    for n in needles {
        assert!(out.contains(n), "{name}: expected `{n}`:\n{out}");
    }
}

// ───────────────────────────── P20: a routine declared twice ─────────────────────

const P20_A: &str = "module top;\n\
     \x20 function int f(input int a); return a + 4; endfunction\n\
     \x20 function int f(input int a); return a + 9; endfunction\n\
     \x20 int r;\n\
     \x20 initial begin r = f(40); $display(\"RD=%0d\", r); #1 $finish; end\n\
     endmodule\n";

#[test]
fn two_functions_of_one_name_in_a_module_are_refused() {
    // PRE printed `RD=49` (the SECOND body) after `W3056 … first declaration used` —
    // a false sentence over a silent-wrong. iverilog "'f' has already been declared in
    // this scope."; verilator "Duplicate declaration of function: 'f'".
    assert_collision("p20_a", P20_A, "f", "module", "a function", "a function");
}

#[test]
fn two_functions_of_one_name_in_an_interface_are_refused() {
    // The interface twin. PRE: `W3056 … [in top.w]` then `RD=49`.
    assert_collision(
        "p20_b",
        "interface ifc;\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 function int f(input int a); return a + 9; endfunction\n\
         endinterface\n\
         module top;\n\
         \x20 ifc w();\n\
         \x20 int r;\n\
         \x20 initial begin r = w.f(40); $display(\"RD=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        "f",
        "interface",
        "a function",
        "a function",
    );
}

#[test]
fn two_functions_of_one_name_in_a_package_are_refused() {
    // The package lane had NO diagnostic at all: PRE printed `RD=49` silently.
    assert_collision(
        "p20_c",
        "package pk;\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 function int f(input int a); return a + 9; endfunction\n\
         endpackage\n\
         module top;\n\
         \x20 import pk::*;\n\
         \x20 int r;\n\
         \x20 initial begin r = f(40); $display(\"RD=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        "f",
        "package",
        "a function",
        "a function",
    );
}

#[test]
fn two_tasks_of_one_name_in_a_module_are_refused() {
    // PRE: `W3056 task … first declaration used` then `RD=49`.
    assert_collision(
        "p20_d",
        "module top;\n\
         \x20 int r;\n\
         \x20 task t(input int a); r = a + 4; endtask\n\
         \x20 task t(input int a); r = a + 9; endtask\n\
         \x20 initial begin t(40); $display(\"RD=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        "t",
        "module",
        "a task",
        "a task",
    );
}

#[test]
fn a_function_and_a_task_of_one_name_are_refused() {
    // The two tables never compared, so PRE said nothing and printed `RD=0`: the task
    // never ran at all — `f(40)` took the FUNCTION and discarded its value. verilator
    // "Unsupported in C: Task has the same name as function: 'f'".
    assert_collision(
        "p20_e",
        "module top;\n\
         \x20 int r;\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 task f(input int a); r = a + 9; endtask\n\
         \x20 initial begin r = 0; f(40); $display(\"RD=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        "f",
        "module",
        "a function",
        "a task",
    );
}

#[test]
fn two_functions_in_a_transparent_generate_region_are_refused() {
    // A region has no scope of its own (§27.2), so these two ARE module declarations.
    // PRE: `W3056 … [in top]` then `RD=49`.
    assert_collision(
        "p20_g",
        "module top;\n\
         \x20 int r;\n\
         \x20 generate\n\
         \x20   function int f(input int a); return a + 4; endfunction\n\
         \x20   function int f(input int a); return a + 9; endfunction\n\
         \x20 endgenerate\n\
         \x20 initial begin r = f(40); $display(\"RD=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        "f",
        "module",
        "a function",
        "a function",
    );
}

#[test]
fn two_tasks_of_one_name_in_a_package_are_refused() {
    // PRE printed `RD=9` silently — the LAST body.
    assert_collision(
        "p20_i",
        "package pk;\n\
         \x20 task t(output int a); a = 4; endtask\n\
         \x20 task t(output int a); a = 9; endtask\n\
         endpackage\n\
         module top;\n\
         \x20 import pk::*;\n\
         \x20 int r;\n\
         \x20 initial begin t(r); $display(\"RD=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        "t",
        "package",
        "a task",
        "a task",
    );
}

#[test]
fn a_unit_scope_function_beside_a_module_one_still_runs() {
    // THE control for P20: §26.4 makes a `$unit` declaration a SHADOW, not a duplicate,
    // and both oracles print `RD=49` (the module's own `f`). A design-wide name set, or
    // a walk that counted the parser's prepended `$unit` items, would have refused it.
    assert_runs(
        "p20_f",
        "function int f(input int a); return a + 4; endfunction\n\
         module top;\n\
         \x20 function int f(input int a); return a + 9; endfunction\n\
         \x20 int r;\n\
         \x20 initial begin r = f(40); $display(\"RD=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        &["RD=49"],
    );
}

#[test]
fn two_differently_named_routines_still_run() {
    // The `_ctl` twins of p20_a / p20_d / p20_g, one design: rename one of each pair and
    // all three tools print `RD=44`.
    assert_runs(
        "p20_a_ctl",
        "module top;\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 function int g(input int a); return a + 9; endfunction\n\
         \x20 int r;\n\
         \x20 initial begin r = f(40); $display(\"RD=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        &["RD=44"],
    );
    assert_runs(
        "p20_d_ctl",
        "module top;\n\
         \x20 int r;\n\
         \x20 task t(input int a); r = a + 4; endtask\n\
         \x20 task u(input int a); r = a + 9; endtask\n\
         \x20 initial begin t(40); $display(\"RD=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        &["RD=44"],
    );
    assert_runs(
        "p20_g_ctl",
        "module top;\n\
         \x20 int r;\n\
         \x20 generate\n\
         \x20   function int f(input int a); return a + 4; endfunction\n\
         \x20   function int g(input int a); return a + 9; endfunction\n\
         \x20 endgenerate\n\
         \x20 initial begin r = f(40); $display(\"RD=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        &["RD=44"],
    );
}

#[test]
fn an_interface_and_a_package_with_two_distinct_routines_still_run() {
    // The `_ctl` twins of p20_b / p20_c / p20_i.
    assert_runs(
        "p20_b_ctl",
        "interface ifc;\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 function int g(input int a); return a + 9; endfunction\n\
         endinterface\n\
         module top;\n\
         \x20 ifc w();\n\
         \x20 int r;\n\
         \x20 initial begin r = w.f(40); $display(\"RD=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        &["RD=44"],
    );
    assert_runs(
        "p20_c_ctl",
        "package pk;\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 function int g(input int a); return a + 9; endfunction\n\
         endpackage\n\
         module top;\n\
         \x20 import pk::*;\n\
         \x20 int r;\n\
         \x20 initial begin r = f(40); $display(\"RD=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        &["RD=44"],
    );
    assert_runs(
        "p20_i_ctl",
        "package pk;\n\
         \x20 task t(output int a); a = 4; endtask\n\
         \x20 task u(output int a); a = 9; endtask\n\
         endpackage\n\
         module top;\n\
         \x20 import pk::*;\n\
         \x20 int r;\n\
         \x20 initial begin t(r); $display(\"RD=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        &["RD=4"],
    );
}

// ──────────────── P21: a routine against another binder of the same scope ─────────

/// `module top; <decl> function int f…; initial r = f(40);` — every P21 cell shares
/// this shape, and every one of them printed `O=44` on PRE: the call always took the
/// function and the other declaration silently kept its own storage.
fn p21(decl: &str) -> String {
    format!(
        "module top;\n\
         \x20 {decl}\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 int r;\n\
         \x20 initial begin r = f(40); $display(\"O=%0d\", r); #1 $finish; end\n\
         endmodule\n"
    )
}

#[test]
fn a_net_and_a_function_of_one_name_are_refused() {
    // verilator "Unsupported in C: Function has the same name as variable: 'f'".
    assert_collision(
        "p21_a",
        &p21("wire f;"),
        "f",
        "module",
        "a net",
        "a function",
    );
}

#[test]
fn a_variable_and_a_function_of_one_name_are_refused() {
    assert_collision(
        "p21_b",
        &p21("logic f;"),
        "f",
        "module",
        "a variable",
        "a function",
    );
}

#[test]
fn a_parameter_and_a_function_of_one_name_are_refused() {
    // verilator "… same name as parameter: 'f'".
    assert_collision(
        "p21_c",
        &p21("parameter int f = 5;"),
        "f",
        "module",
        "a parameter",
        "a function",
    );
}

#[test]
fn a_localparam_and_a_function_of_one_name_are_refused() {
    assert_collision(
        "p21_d",
        &p21("localparam int f = 5;"),
        "f",
        "module",
        "a localparam",
        "a function",
    );
}

#[test]
fn a_port_and_a_function_of_one_name_are_refused() {
    // The PORT is declared in the header, so the walk must reach `module.ports` and not
    // only the body. verilator "… same name as port: 'f'".
    assert_collision(
        "p21_e",
        "module sub(input wire f);\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 int r;\n\
         \x20 initial begin r = f(40); $display(\"O=%0d\", r); end\n\
         endmodule\n\
         module top;\n\
         \x20 wire w = 1'b0;\n\
         \x20 sub u(.f(w));\n\
         \x20 initial begin #1 $finish; end\n\
         endmodule\n",
        "f",
        "module",
        "a port",
        "a function",
    );
}

#[test]
fn a_genvar_and_a_function_of_one_name_are_refused() {
    // A genvar lives only in `genvar_decls`, which no other binder reads.
    assert_collision(
        "p21_f",
        &p21("genvar f;"),
        "f",
        "module",
        "a genvar",
        "a function",
    );
}

#[test]
fn an_instance_name_and_a_function_of_one_name_are_refused() {
    // verilator "… same name as instance: 'f'".
    assert_collision(
        "p21_g",
        "module sub; endmodule\n\
         module top;\n\
         \x20 sub f();\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 int r;\n\
         \x20 initial begin r = f(40); $display(\"O=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        "f",
        "module",
        "an instance",
        "a function",
    );
}

#[test]
fn a_named_block_label_and_a_function_of_one_name_are_refused() {
    // verilator "Unsupported in C: Block has the same name as function: 'f'". Only a
    // block written DIRECTLY as a procedural block's body counts — see the control
    // `a_block_local_and_a_sibling_block_still_run` for what must not.
    assert_collision(
        "p21_h",
        "module top;\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 int r;\n\
         \x20 initial begin : f\n\
         \x20   r = 7;\n\
         \x20 end\n\
         \x20 initial begin r = f(40); $display(\"O=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        "f",
        "module",
        "a function",
        "a named block",
    );
}

#[test]
fn a_variable_and_a_task_of_one_name_are_refused() {
    // The TASK half of the same rule — the two tables are separate, so both need it.
    assert_collision(
        "p21_i",
        "module top;\n\
         \x20 logic t;\n\
         \x20 int r;\n\
         \x20 task t(input int a); r = a + 4; endtask\n\
         \x20 initial begin t(40); $display(\"O=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        "t",
        "module",
        "a variable",
        "a task",
    );
}

#[test]
fn a_variable_and_a_function_of_one_name_in_an_interface_are_refused() {
    assert_collision(
        "p21_j",
        "interface ifc;\n\
         \x20 logic f;\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         endinterface\n\
         module top;\n\
         \x20 ifc w();\n\
         \x20 int r;\n\
         \x20 initial begin r = w.f(40); $display(\"O=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        "f",
        "interface",
        "a variable",
        "a function",
    );
}

#[test]
fn the_declaration_order_is_not_a_discriminator() {
    // The ROUTINE first, the other binder second. Both oracles reject these exactly as
    // they reject p21_c / p21_b, and the refusal must name the pair in SOURCE order —
    // a walk keyed on "a routine was registered first" would have missed them entirely,
    // because routines register at step (3.5) and nets only at step (4).
    assert_collision(
        "p21_k",
        "module top;\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 parameter int f = 5;\n\
         \x20 int r;\n\
         \x20 initial begin r = f(40); $display(\"O=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        "f",
        "module",
        "a function",
        "a parameter",
    );
    assert_collision(
        "p21_l",
        "module top;\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 logic f;\n\
         \x20 int r;\n\
         \x20 initial begin r = f(40); $display(\"O=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        "f",
        "module",
        "a function",
        "a variable",
    );
}

#[test]
fn a_design_that_reads_both_objects_is_refused_too() {
    // PRE was SELF-CONSISTENTLY wrong: `call=44 net=z` and `call=44 const=5` — the call
    // took the function and the hierarchical read took the other declaration, i.e. one
    // name resolved to two different objects in one design. Pinned because that is the
    // shape a value-only differential can miss.
    assert_collision(
        "p21_m",
        "module top;\n\
         \x20 wire f;\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 int r;\n\
         \x20 initial begin r = f(40); $display(\"call=%0d net=%0b\", r, top.f); #1 $finish; end\n\
         endmodule\n",
        "f",
        "module",
        "a net",
        "a function",
    );
    assert_collision(
        "p21_n",
        "module top;\n\
         \x20 localparam int f = 5;\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 int r;\n\
         \x20 initial begin r = f(40); $display(\"call=%0d const=%0d\", r, top.f); #1 $finish; end\n\
         endmodule\n",
        "f",
        "module",
        "a localparam",
        "a function",
    );
}

#[test]
fn one_renamed_declaration_makes_every_p21_shape_run() {
    // The eight `_ctl` cells: rename the non-routine declaration and all three tools
    // print `O=44`. These are the controls that say the gate is keyed on the NAME.
    for (tag, decl) in [
        ("p21_a_ctl", "wire g;"),
        ("p21_c_ctl", "parameter int g = 5;"),
        ("p21_f_ctl", "genvar g;"),
        ("p21_i_ctl", "logic u;"),
    ] {
        assert_runs(tag, &p21(decl), &["O=44"]);
    }
    assert_runs(
        "p21_e_ctl",
        "module sub(input wire g);\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 int r;\n\
         \x20 initial begin r = f(40); $display(\"O=%0d\", r); end\n\
         endmodule\n\
         module top;\n\
         \x20 wire w = 1'b0;\n\
         \x20 sub u(.g(w));\n\
         \x20 initial begin #1 $finish; end\n\
         endmodule\n",
        &["O=44"],
    );
    assert_runs(
        "p21_g_ctl",
        "module sub; endmodule\n\
         module top;\n\
         \x20 sub g();\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 int r;\n\
         \x20 initial begin r = f(40); $display(\"O=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        &["O=44"],
    );
    assert_runs(
        "p21_h_ctl",
        "module top;\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 int r;\n\
         \x20 initial begin : g\n\
         \x20   r = 7;\n\
         \x20 end\n\
         \x20 initial begin r = f(40); $display(\"O=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        &["O=44"],
    );
    assert_runs(
        "p21_j_ctl",
        "interface ifc;\n\
         \x20 logic g;\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         endinterface\n\
         module top;\n\
         \x20 ifc w();\n\
         \x20 int r;\n\
         \x20 initial begin r = w.f(40); $display(\"O=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        &["O=44"],
    );
}

// ───────────── review round 1: the binders the first collector missed ───────────

#[test]
fn a_typedef_or_a_class_of_a_routine_name_is_refused() {
    // F4. A type NAME shares the region's name space with a subroutine. vita printed
    // `O=44`; verilator "Unsupported in C: Function has the same name as TYPEDEF 'f':
    // 'f'" / "… as CLASS 'f': 'f'", and iverilog cannot even parse the function header
    // after either declaration (rc=4 "syntax error"). Controls (`typedef int g;` /
    // `class g; endclass`) run in all three.
    assert_collision(
        "b1_typedef",
        "module top;\n\
         \x20 typedef int f;\n\
         \x20 function int f(); return 44; endfunction\n\
         \x20 initial $display(\"O=%0d\", f());\n\
         endmodule\n",
        "f",
        "module",
        "a typedef",
        "a function",
    );
    assert_collision(
        "b2_class",
        "module top;\n\
         \x20 class f; endclass\n\
         \x20 function int f(); return 44; endfunction\n\
         \x20 initial $display(\"O=%0d\", f());\n\
         endmodule\n",
        "f",
        "module",
        "a class",
        "a function",
    );
    assert_runs(
        "b1_ctl",
        "module top;\n\
         \x20 typedef int g;\n\
         \x20 function int f(); return 44; endfunction\n\
         \x20 initial $display(\"O=%0d\", f());\n\
         endmodule\n",
        &["O=44"],
    );
    assert_runs(
        "b2_ctl",
        "module top;\n\
         \x20 class g; endclass\n\
         \x20 function int f(); return 44; endfunction\n\
         \x20 initial $display(\"O=%0d\", f());\n\
         endmodule\n",
        &["O=44"],
    );
}

#[test]
fn a_fork_label_of_a_routine_name_is_refused_like_the_begin_twin() {
    // F4 / M1. The `ModuleItem::Proc` arm matched only `Stmt::Block`, so the `begin : f`
    // twin (census p21_h) shipped while `fork : f … join` stayed silent. iverilog
    // "'f' has already been declared in this scope. : It was declared here as a named
    // block."; verilator "FORK 'f' has the same name as function: 'f'".
    assert_collision(
        "g10",
        "module top;\n\
         \x20 function int f(input int a); return a + 1; endfunction\n\
         \x20 initial fork : f\n\
         \x20   #1 $display(\"done\");\n\
         \x20 join\n\
         \x20 initial begin #2 $display(\"O=%0d\", f(43)); $finish; end\n\
         endmodule\n",
        "f",
        "module",
        "a function",
        "a named block",
    );
}

#[test]
fn a_labelled_generate_blocks_label_is_a_declaration_of_the_enclosing_scope() {
    // M2. TWO questions with different answers, and the file's doc used to give only
    // one. The block's CONTENTS are its own §27.3 region — the control below runs in
    // both oracles — but its LABEL is declared in the enclosing scope: iverilog "'fn'
    // has already been declared in this scope … declared here as a function",
    // verilator "Generate block has the same name as function: 'fn'". vita printed
    // `F05 1 44`.
    assert_collision(
        "f05",
        "module top;\n\
         \x20 function int fn(input int a); return a + 1; endfunction\n\
         \x20 generate\n\
         \x20   if (1) begin : fn\n\
         \x20     wire z;\n\
         \x20   end\n\
         \x20 endgenerate\n\
         \x20 initial begin #1 $display(\"O=%0d\", fn(43)); $finish; end\n\
         endmodule\n",
        "fn",
        "module",
        "a function",
        "a generate block",
    );
    // THE control for "label only, no recursion": the block DECLARES a `wire f` of the
    // module function's name, which is legal §27.3 shadowing — `G06 44` in all three.
    assert_runs(
        "g06",
        "module top;\n\
         \x20 function int f(input int a); return a + 1; endfunction\n\
         \x20 genvar gi;\n\
         \x20 generate for (gi = 0; gi < 2; gi = gi + 1) begin : g\n\
         \x20   wire f;\n\
         \x20   assign f = 1'b1;\n\
         \x20 end endgenerate\n\
         \x20 initial begin #1 $display(\"O=%0d\", f(43)); $finish; end\n\
         endmodule\n",
        &["O=44"],
    );
}

#[test]
fn an_enum_label_of_a_routine_name_is_refused() {
    // M4. IEEE §6.19 declares an enum's labels in the scope that holds the typedef,
    // not inside the type. vita printed `G19 8 44`; iverilog "'fn' has already been
    // declared in this scope … as an enum type or value", verilator "Function has the
    // same name as ENUMITEM 'fn'".
    assert_collision(
        "g19",
        "module top;\n\
         \x20 typedef enum int { fn = 7, gg = 8 } e_t;\n\
         \x20 function int fn(input int a); return a + 1; endfunction\n\
         \x20 e_t e;\n\
         \x20 initial begin e = gg; #1 $display(\"O=%0d\", fn(43)); $finish; end\n\
         endmodule\n",
        "fn",
        "module",
        "an enum label",
        "a function",
    );
    // Control: labels that collide with nothing. All three print `E 8 44`.
    assert_runs(
        "g19_ctl",
        "module top;\n\
         \x20 typedef enum int { aa = 7, gg = 8 } e_t;\n\
         \x20 function int fn(input int a); return a + 1; endfunction\n\
         \x20 e_t e;\n\
         \x20 initial begin e = gg; #1 $display(\"E %0d %0d\", e, fn(43)); $finish; end\n\
         endmodule\n",
        &["E 8 44"],
    );
}

#[test]
fn an_instance_name_and_a_net_of_one_name_are_refused() {
    // M5. The closed list answered this pair "not measured, so not refused"; it is
    // measured now and both oracles reject — iverilog "'u1' has already been declared
    // in this scope. … as a net", verilator "Instance has the same name as variable:
    // 'u1'". vita printed `F07B 1 0`.
    assert_collision(
        "f07b",
        "module sub(input x, output y); assign y = ~x; endmodule\n\
         module top;\n\
         \x20 reg a; wire b; wire u1;\n\
         \x20 assign u1 = 1'b0;\n\
         \x20 sub u1(a, b);\n\
         \x20 initial begin a = 1'b0; #1 $display(\"O %b %b\", b, u1); $finish; end\n\
         endmodule\n",
        "u1",
        "module",
        "a net",
        "an instance",
    );
    // Control: the instance and the net have different names — `I 1 0` in all three.
    assert_runs(
        "f07",
        "module sub(input x, output y); assign y = ~x; endmodule\n\
         module top;\n\
         \x20 reg a; wire b; wire u2;\n\
         \x20 assign u2 = 1'b0;\n\
         \x20 sub u1(a, b);\n\
         \x20 initial begin a = 1'b0; #1 $display(\"I %b %b\", b, u2); $finish; end\n\
         endmodule\n",
        &["I 1 0"],
    );
}

#[test]
fn a_user_identifier_holding_a_dollar_is_judged_like_any_other() {
    // F2 / F3 / M3. `synthesized()` used to skip every name containing a `$`, which
    // IEEE §5.6 allows a USER to write after the first character. The blanket skip
    // turned the W3056 routine warning into NO diagnostic at all on a wrong answer,
    // and left the shipped net-vs-routine and genvar-vs-parameter pairs silent.
    // All three cells: iverilog "'f$1' has already been declared in this scope.",
    // verilator "Duplicate declaration of function: 'f$1'" / "Function has the same
    // name as variable: 'f$1'" / "Duplicate declaration of signal: 'Q$a'".
    assert_collision(
        "c1_dollar_rtn",
        "module top;\n\
         \x20 function int f$1(); return 44; endfunction\n\
         \x20 function int f$1(); return 49; endfunction\n\
         \x20 initial $display(\"RD=%0d\", f$1());\n\
         endmodule\n",
        "f$1",
        "module",
        "a function",
        "a function",
    );
    assert_collision(
        "b6_dollar",
        "module top;\n\
         \x20 wire f$1;\n\
         \x20 function int f$1(); return 44; endfunction\n\
         \x20 initial $display(\"O=%0d\", f$1());\n\
         endmodule\n",
        "f$1",
        "module",
        "a net",
        "a function",
    );
    assert_collision(
        "c2_dollar_genvar",
        "module top;\n\
         \x20 genvar Q$a;\n\
         \x20 localparam int Q$a = 3;\n\
         \x20 initial $display(\"Q=%0d\", Q$a);\n\
         endmodule\n",
        "Q$a",
        "module",
        "a genvar",
        "a localparam",
    );
    // THE control: two DIFFERENT `$`-bearing user names must still run — `RD=44` in
    // all three tools.
    assert_runs(
        "c1_ctl",
        "module top;\n\
         \x20 function int f$1(); return 44; endfunction\n\
         \x20 function int g$1(); return 49; endfunction\n\
         \x20 initial $display(\"RD=%0d\", f$1());\n\
         endmodule\n",
        &["RD=44"],
    );
}
