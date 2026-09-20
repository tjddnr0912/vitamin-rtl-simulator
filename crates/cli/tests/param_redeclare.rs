//! A parameter name declared twice in one module or interface scope is refused.
//!
//! ROADMAP §2 🆕 L ⓢ ("a header parameter redeclared in the body answers the body
//! declaration"). `module top #(parameter int P = 3); parameter int P = 7;` printed
//! `P=7` at exit 0. Both oracles REJECT it — iverilog 13.0 "'P' has already been
//! declared in this scope. : It was declared here as a parameter.", verilator 5.052
//! "%Error: Duplicate declaration of signal: 'P'" — because IEEE 1800-2017 §6.20.1 /
//! §23.2.3 make the parameter port list and the module body ONE declarative scope.
//!
//! The same census found three more shapes with the same verdict from both oracles and
//! the same silent accept in vita: two declarations inside one ANSI header; two body
//! declarations in a module with no ANSI header (IEEE 1364-2005 §12.2 makes the FIRST
//! of those the parameter port list); and every shape again inside an `interface`. One
//! walk over the declaration sequence answers all four — `elaborate/param_dup.rs`.
//!
//! ⚠️ The CONTROLS below are the point of this file, not decoration. Each is a design
//! both oracles ACCEPT, and each looks like a redeclaration from one angle:
//!   * a generate-block `localparam` of a header parameter's name (§27.3 is a nested
//!     scope: `gP=7` inside, `P=3` outside, in all three tools);
//!   * an override — `#(.P(9))`, which changes one declaration and does not add one;
//!   * a package or `$unit` constant of the same name, which is import shadowing;
//!   * a body parameter whose name belongs to a DIFFERENT module's header;
//!   * the parser's own desugars: a header ARRAY parameter emits a `ParamDecl` AND a
//!     const-array `NetVar` twin, and a `parameter type T` emits the `T$w` / `T$s`
//!     carriers — neither is a user duplicate.
//!
//! ORACLES: iverilog 13.0 (`-g2012`), verilator 5.052 (`--binary --timing`). The
//! type-parameter cell is verilator-only: iverilog rejects a body `parameter type` as a
//! syntax error ("Invalid module item"), which is a refusal but not this one.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns `(exit_code, stdout+stderr)`. The code matters as much as the text: a
/// refusal that did not set the exit status would let a CI script run the design.
fn run(src: &str) -> (i32, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_prdc_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
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
    let _ = std::fs::remove_dir_all(&d);
    (out.status.code().unwrap_or(-1), text)
}

/// The refusal is pinned by CODE and by the two things a reader needs — the duplicated
/// NAME and a second location for the first declaration — never by the whole sentence.
fn assert_refused(name: &str, src: &str, ident: &str) {
    let (rc, out) = run(src);
    assert_eq!(rc, 1, "{name}: a refused design must exit 1:\n{out}");
    assert!(
        out.contains("VITA-E3009"),
        "{name}: refusal is E3009:\n{out}"
    );
    assert!(
        out.contains("duplicate declaration of") && out.contains(&format!("`{ident}`")),
        "{name}: the refusal names `{ident}`:\n{out}"
    );
    assert!(
        out.contains("the first declaration of that name is here"),
        "{name}: a note points at the FIRST declaration:\n{out}"
    );
}

fn assert_runs(name: &str, src: &str, needles: &[&str]) {
    let (rc, out) = run(src);
    assert_eq!(rc, 0, "{name}: this design is legal and must run:\n{out}");
    for n in needles {
        assert!(out.contains(n), "{name}: expected `{n}`:\n{out}");
    }
}

#[test]
fn an_ansi_header_parameter_redeclared_in_the_body_is_refused() {
    // The row itself. PRE printed `P=7` at exit 0 for all three.
    assert_refused(
        "body parameter",
        "module top #(parameter int P = 3); parameter int P = 7;\n\
         \x20 initial begin $display(\"P=%0d\", P); #1 $finish; end\n\
         endmodule\n",
        "P",
    );
    assert_refused(
        "body localparam",
        "module top #(parameter int P = 3); localparam int P = 7;\n\
         \x20 initial begin $display(\"P=%0d\", P); #1 $finish; end\n\
         endmodule\n",
        "P",
    );
    // A DIFFERENT declared type is still one name in one scope, so the type axis must
    // not be read as a discriminator — both oracles reject this exactly as above.
    assert_refused(
        "different type",
        "module top #(parameter int P = 3); parameter logic [7:0] P = 7;\n\
         \x20 initial begin $display(\"P=%0d\", P); #1 $finish; end\n\
         endmodule\n",
        "P",
    );
}

#[test]
fn a_redeclaration_inside_one_ansi_header_is_refused() {
    assert_refused(
        "header twice",
        "module top #(parameter int P = 3, parameter int P = 7);\n\
         \x20 initial begin $display(\"P=%0d\", P); #1 $finish; end\n\
         endmodule\n",
        "P",
    );
}

#[test]
fn two_body_declarations_with_no_ansi_header_are_refused() {
    // IEEE 1364-2005 §12.2: with no `#(...)` the body list IS the parameter port list,
    // so the SECOND of these is the redeclaration — `param_ports` already answers it
    // that way for overrides. Both spellings, with and without a port list, because
    // the presence of ports is what decides whether the module looks "non-ANSI".
    assert_refused(
        "non-ANSI with a port",
        "module top(a); input a; parameter P = 3; parameter P = 7;\n\
         \x20 initial begin $display(\"P=%0d\", P); #1 $finish; end\n\
         endmodule\n",
        "P",
    );
    assert_refused(
        "no ports at all",
        "module top; parameter P = 3; parameter P = 7;\n\
         \x20 initial begin $display(\"P=%0d\", P); #1 $finish; end\n\
         endmodule\n",
        "P",
    );
    assert_refused(
        "parameter then localparam",
        "module top; parameter int P = 3; localparam int P = 7;\n\
         \x20 initial begin $display(\"P=%0d\", P); #1 $finish; end\n\
         endmodule\n",
        "P",
    );
}

#[test]
fn the_interface_twin_is_refused_too() {
    // The interface window (`iface_inst.rs`) binds its own parameters; it reaches the
    // rule through the same `bind_params` the module lane calls, which is why this is
    // one gate and not two. Both oracles reject both spellings.
    assert_refused(
        "interface, body parameter",
        "interface ifc #(parameter int P = 3); parameter int P = 7;\n\
         endinterface\n\
         module top; ifc i();\n\
         \x20 initial begin $display(\"P=%0d\", i.P); #1 $finish; end\n\
         endmodule\n",
        "P",
    );
    assert_refused(
        "interface, body localparam",
        "interface ifc #(parameter int P = 3); localparam int P = 7;\n\
         endinterface\n\
         module top; ifc i();\n\
         \x20 initial begin $display(\"P=%0d\", i.P); #1 $finish; end\n\
         endmodule\n",
        "P",
    );
}

#[test]
fn an_override_does_not_rescue_a_redeclaration() {
    // `#(.P(9))` targets the HEADER declaration; the body one is still a second
    // declaration of the name. PRE printed `P=9` at exit 0 — the override channel had
    // simply routed the body declaration through the header binder. verilator reports
    // both the duplicate AND "Instance attempts to override 'P' as a parameter, but it
    // is a local parameter"; iverilog reports the duplicate.
    assert_refused(
        "override plus redeclaration",
        "module sub #(parameter int P = 3); parameter int P = 7;\n\
         \x20 initial $display(\"P=%0d\", P);\n\
         endmodule\n\
         module top; sub #(.P(9)) u(); initial #2 $finish; endmodule\n",
        "P",
    );
}

#[test]
fn a_duplicated_type_parameter_names_t_and_not_its_carrier() {
    // A `parameter type T` is lowered to the synthesized `T$w` / `T$s` carriers, so the
    // gate sees the duplicate under a name that is not in the source. Naming `T$w`
    // would send the reader hunting a declaration that does not exist — and the several
    // carriers of ONE declaration must say it once, not once each.
    let src = "module top #(parameter type T = int); parameter type T = byte;\n\
         \x20 T x;\n\
         \x20 initial begin x = 3; $display(\"b=%0d\", $bits(x)); #1 $finish; end\n\
         endmodule\n";
    assert_refused("type parameter", src, "T");
    let (_, out) = run(src);
    assert!(
        !out.contains("T$w") && !out.contains("T$s"),
        "the carrier name must never appear in the refusal:\n{out}"
    );
    assert_eq!(
        out.matches("duplicate declaration of").count(),
        1,
        "one user declaration, one report — not one per carrier:\n{out}"
    );
    assert!(
        out.contains("type parameter `T`"),
        "…and it is called a type parameter:\n{out}"
    );
}

#[test]
fn one_source_defect_reports_once_however_many_instances() {
    // The gate runs from `bind_params`, i.e. once per INSTANCE, while the defect is a
    // property of the source. Without the span key this printed the same duplicate
    // twice for `sub a(); sub b();` — noise that grows with instance count.
    let (rc, out) = run("module sub #(parameter int P = 3); parameter int P = 7;\n\
         \x20 initial $display(\"sP=%0d\", P);\n\
         endmodule\n\
         module top; sub a(); sub b(); initial #1 $finish; endmodule\n");
    assert_eq!(rc, 1, "still refused:\n{out}");
    assert_eq!(
        out.matches("duplicate declaration of").count(),
        1,
        "two instances of one bad module report once:\n{out}"
    );
}

#[test]
fn two_duplicated_names_report_twice() {
    // The counterpart of the test above: the dedupe is keyed on the offending
    // declaration, so it must not collapse two independent defects into one. Both
    // oracles report both.
    let (rc, out) = run(
        "module top #(parameter int P = 3, parameter int Q = 4); parameter int P = 7; parameter int Q = 8;\n\
         \x20 initial begin $display(\"%0d %0d\", P, Q); #1 $finish; end\n\
         endmodule\n",
    );
    assert_eq!(rc, 1, "refused:\n{out}");
    assert_eq!(
        out.matches("duplicate declaration of").count(),
        2,
        "two duplicated names, two reports:\n{out}"
    );
    assert!(
        out.contains("`P`") && out.contains("`Q`"),
        "both names are named:\n{out}"
    );
}

#[test]
fn a_generate_block_localparam_of_the_same_name_still_runs() {
    // §27.3: a generate block is a nested scope, so this is shadowing, not
    // redeclaration. Measured in both oracles: `gP=7` inside and `P=3` outside, one
    // design. A gate that refused this would turn a working, legal idiom loud.
    assert_runs(
        "generate shadow",
        "module top #(parameter int P = 3);\n\
         \x20 if (1) begin : g\n\
         \x20   localparam int P = 7;\n\
         \x20   initial $display(\"gP=%0d\", P);\n\
         \x20 end\n\
         \x20 initial begin $display(\"P=%0d\", P); #1 $finish; end\n\
         endmodule\n",
        &["gP=7", "P=3"],
    );
}

#[test]
fn shadowing_and_overriding_and_unrelated_modules_still_run() {
    // A body parameter the header does not declare — the control that says the gate is
    // keyed on the NAME and not on "the module has a header".
    assert_runs(
        "body parameter, no collision",
        "module top #(parameter int Q = 3); parameter int P = 7;\n\
         \x20 initial begin $display(\"Q=%0d P=%0d\", Q, P); #1 $finish; end\n\
         endmodule\n",
        &["Q=3 P=7"],
    );
    // An override alone: one declaration, a new value.
    assert_runs(
        "override alone",
        "module sub #(parameter int P = 3); initial $display(\"sP=%0d\", P); endmodule\n\
         module top; sub #(.P(9)) u(); initial #2 $finish; endmodule\n",
        &["sP=9"],
    );
    // `P` is a header parameter of ANOTHER module. The walk is per-`ModuleDecl`; a
    // design-wide name set would have refused this.
    assert_runs(
        "another module's header name",
        "module other #(parameter int P = 3); initial $display(\"oP=%0d\", P); endmodule\n\
         module top; localparam int P = 7; other u();\n\
         \x20 initial begin $display(\"P=%0d\", P); #1 $finish; end\n\
         endmodule\n",
        &["P=7", "oP=3"],
    );
    // A package constant of the same name is import shadowing (§26.3), which both
    // oracles accept — the local declaration wins.
    assert_runs(
        "wildcard import shadow",
        "package pk; parameter int P = 3; endpackage\n\
         module top; import pk::*; localparam int P = 7;\n\
         \x20 initial begin $display(\"P=%0d\", P); #1 $finish; end\n\
         endmodule\n",
        &["P=7"],
    );
    // …and the `$unit` twin of it.
    assert_runs(
        "unit-scope shadow",
        "parameter int P = 3;\n\
         module top; localparam int P = 7;\n\
         \x20 initial begin $display(\"P=%0d\", P); #1 $finish; end\n\
         endmodule\n",
        &["P=7"],
    );
}

#[test]
fn the_parsers_own_param_desugars_are_not_duplicates() {
    // A header ARRAY parameter is emitted as a `ParamDecl` in `module.params` AND a
    // const-array `NetVar` at the front of the body (`ParamItem::ConstArrayVar`). If
    // the walk had counted body NetVars, every array parameter in the suite would have
    // gone loud. verilator runs both of these; iverilog declines unpacked array
    // parameters outright ("sorry: … not supported yet"), so it is not the oracle here.
    assert_runs(
        "header array parameter",
        "module top #(parameter int A[2] = '{1,2});\n\
         \x20 initial begin $display(\"A=%0d %0d\", A[0], A[1]); #1 $finish; end\n\
         endmodule\n",
        &["A=1 2"],
    );
    assert_runs(
        "body array localparam",
        "module top; localparam int A[2] = '{1,2};\n\
         \x20 initial begin $display(\"A=%0d %0d\", A[0], A[1]); #1 $finish; end\n\
         endmodule\n",
        &["A=1 2"],
    );
    // A single `parameter type` emits several carriers under ONE name token; only a
    // second declaration of `T` may collide.
    assert_runs(
        "header type parameter",
        "module top #(parameter type T = int);\n\
         \x20 T x;\n\
         \x20 initial begin x = 3; $display(\"b=%0d x=%0d\", $bits(x), x); #1 $finish; end\n\
         endmodule\n",
        &["b=32 x=3"],
    );
}

#[test]
fn a_parameter_and_a_net_of_one_name_are_not_this_rule() {
    // Neighbouring class, deliberately untouched: `localparam N = 7; logic [3:0] N;` is
    // equally illegal and both oracles reject it, but it is a parameter-vs-NET
    // collision — a different binder and a different funnel (`add_net`). Pinned so the
    // next reader can see that this slice's gate is `ModuleItem::Param` only, and so a
    // future widening of it is a deliberate, measured edit rather than a side effect.
    // Today vita resolves the name to the parameter; `block_local_shadows_param.rs`
    // pins the same cell from the shadow-rule side.
    let (rc, out) = run("module tb; localparam N = 7; logic [3:0] N;\n\
         \x20 initial $display(\"r=%0d\", N);\n\
         endmodule\n");
    assert_eq!(rc, 0, "unchanged by this slice:\n{out}");
    assert!(
        out.contains("r=7"),
        "unchanged: the parameter still wins:\n{out}"
    );
}
