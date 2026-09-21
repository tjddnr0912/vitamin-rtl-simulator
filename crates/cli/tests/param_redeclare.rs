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
fn a_duplicate_inside_one_generate_block_is_refused() {
    // The GENERATE half of the same rule (IEEE §27.3: the block is ONE declarative
    // scope). vita bound the second declaration and printed `g Q=1 / g Q=2` at exit 0;
    // iverilog 13.0 "e09.sv:7: error: 'Q' has already been declared in this scope. :
    // It was declared here as a parameter." and verilator 5.052 "%Error: e09.sv:7:18:
    // Duplicate declaration of signal: 'Q'".
    assert_refused(
        "generate-for block",
        "module top;\n\
         \x20 genvar i;\n\
         \x20 generate for (i = 0; i < 2; i = i + 1) begin : g\n\
         \x20   localparam Q = i;\n\
         \x20   localparam Q = i + 1;\n\
         \x20   initial $display(\"g Q=%0d\", Q);\n\
         \x20 end endgenerate\n\
         \x20 initial begin #1 $finish; end\n\
         endmodule\n",
        "Q",
    );
    // The two unrolled iterations re-walk the same item list, and each GenPhase walks
    // it again; the NAME-span dedupe must make that exactly one report.
    let (_rc, out) = run("module top;\n\
         \x20 genvar i;\n\
         \x20 generate for (i = 0; i < 2; i = i + 1) begin : g\n\
         \x20   localparam Q = i;\n\
         \x20   localparam Q = i + 1;\n\
         \x20 end endgenerate\n\
         \x20 initial begin #1 $finish; end\n\
         endmodule\n");
    assert_eq!(
        out.matches("duplicate declaration of").count(),
        1,
        "one source defect, one report — not one per iteration or per phase:\n{out}"
    );
    // A conditional generate block is the same declarative region.
    assert_refused(
        "generate-if block",
        "module top;\n\
         \x20 generate if (1) begin : g\n\
         \x20   localparam int R = 1;\n\
         \x20   localparam int R = 2;\n\
         \x20 end endgenerate\n\
         \x20 initial begin #1 $finish; end\n\
         endmodule\n",
        "R",
    );
}

#[test]
fn a_duplicate_in_a_transparent_generate_region_is_refused() {
    // The FOURTH declarative region that holds `ParamDecl`s, and the one no walk
    // reached: an UNLABELLED `generate … endgenerate` has no scope of its own (IEEE
    // §27.2), so its names ARE the module's — but they sit inside
    // `ModuleItem::Generate`, which the module-body filter skipped, while the §27.3
    // per-scope walk runs on the labelled path only. vita printed `Q=2` at exit 0.
    // iverilog 13.0 "r243.sv:4: error: 'Q' has already been declared in this scope. :
    // It was declared here as a parameter."; verilator 5.052 "%Error: r243.sv:4:20:
    // Duplicate declaration of signal: 'Q'".
    assert_refused(
        "transparent region, two declarations",
        "module top;\n\
         \x20 generate\n\
         \x20   localparam int Q = 1;\n\
         \x20   localparam int Q = 2;\n\
         \x20 endgenerate\n\
         \x20 initial begin $display(\"Q=%0d\", Q); #1 $finish; end\n\
         endmodule\n",
        "Q",
    );
    // FLATTENING the region into the module's own sequence (rather than adding a
    // fourth call site) is what makes the CROSS-REGION pairs refusable, and both
    // oracles reject both of them. Header parameter + region `localparam`: iverilog
    // points its note at line 1, the header — so does vita.
    assert_refused(
        "header parameter + region localparam",
        "module top #(parameter int P = 1);\n\
         \x20 generate\n\
         \x20   localparam int P = 2;\n\
         \x20 endgenerate\n\
         \x20 initial begin $display(\"P=%0d\", P); #1 $finish; end\n\
         endmodule\n",
        "P",
    );
    // Body `localparam` + region `localparam`.
    assert_refused(
        "body localparam + region localparam",
        "module top;\n\
         \x20 localparam int P = 1;\n\
         \x20 generate\n\
         \x20   localparam int P = 2;\n\
         \x20 endgenerate\n\
         \x20 initial begin $display(\"P=%0d\", P); #1 $finish; end\n\
         endmodule\n",
        "P",
    );
}

#[test]
fn a_transparent_region_beside_a_labelled_block_still_runs() {
    // THE control for the flatten: a LABELLED block is a scope of its own (§27.3),
    // so the same name in it is shadowing, not a duplicate — the flatten must not
    // pull it in. Both oracles run this and print `gQ=2` then `Q=1`.
    assert_runs(
        "region + labelled block, same name",
        "module top;\n\
         \x20 generate\n\
         \x20   localparam int Q = 1;\n\
         \x20 endgenerate\n\
         \x20 generate if (1) begin : g\n\
         \x20   localparam int Q = 2;\n\
         \x20   initial $display(\"gQ=%0d\", Q);\n\
         \x20 end endgenerate\n\
         \x20 initial begin $display(\"Q=%0d\", Q); #1 $finish; end\n\
         endmodule\n",
        &["gQ=2", "Q=1"],
    );
    // Distinct names in one region: legal, all three tools `A=1 B=2`.
    assert_runs(
        "region, distinct names",
        "module top;\n\
         \x20 generate\n\
         \x20   localparam int A = 1;\n\
         \x20   localparam int B = 2;\n\
         \x20 endgenerate\n\
         \x20 initial begin $display(\"A=%0d B=%0d\", A, B); #1 $finish; end\n\
         endmodule\n",
        &["A=1 B=2"],
    );
}

#[test]
fn a_duplicate_in_a_package_body_is_refused() {
    // The PACKAGE half (IEEE §26.2). A package has no parameter port list, so
    // `bind_params` never runs for one and its own loop guarded parameter-vs-VARIABLE
    // only: vita printed `P=2` at exit 0. iverilog 13.0 "e08.sv:4: error: 'P' has
    // already been declared in this scope."; verilator 5.052 "%Error: e08.sv:4:13:
    // Duplicate declaration of signal: 'P'".
    assert_refused(
        "package body",
        "package pk;\n\
         \x20 parameter P = 1;\n\
         \x20 parameter P = 2;\n\
         endpackage\n\
         module top; initial begin $display(\"P=%0d\", pk::P); #1 $finish; end endmodule\n",
        "P",
    );
    // …and through the IMPORT spelling, which is how the census found it.
    assert_refused(
        "package body, imported",
        "package pk;\n\
         \x20 parameter int P = 3;\n\
         \x20 parameter int P = 7;\n\
         endpackage\n\
         module top; import pk::*;\n\
         \x20 initial begin $display(\"P=%0d\", P); #1 $finish; end\n\
         endmodule\n",
        "P",
    );
    // CONTROL: distinct names, and a `parameter` beside a `localparam` and a variable,
    // all legal. All three tools: `P=3 Q=4`.
    assert_runs(
        "package body, no collision",
        "package pk;\n\
         \x20 parameter int P = 3;\n\
         \x20 localparam int Q = 4;\n\
         \x20 int v;\n\
         endpackage\n\
         module top;\n\
         \x20 initial begin $display(\"P=%0d Q=%0d\", pk::P, pk::Q); #1 $finish; end\n\
         endmodule\n",
        &["P=3 Q=4"],
    );
}

#[test]
fn one_localparam_per_generate_iteration_still_runs() {
    // THE control for the generate half: a loop body declaring ONE `localparam` is the
    // normal idiom, and each iteration is its own scope instance — not a duplicate.
    // A nested block may re-use the name (a deeper scope), and the two arms of a
    // generate-if may each declare it. All three tools: `g Q=1 / g Q=2 / a R=5 / c R=7`.
    assert_runs(
        "one per iteration",
        "module top;\n\
         \x20 genvar i;\n\
         \x20 generate for (i = 0; i < 2; i = i + 1) begin : g\n\
         \x20   localparam Q = i + 1;\n\
         \x20   initial $display(\"g Q=%0d\", Q);\n\
         \x20 end endgenerate\n\
         \x20 generate if (1) begin : a\n\
         \x20   localparam R = 5;\n\
         \x20   initial $display(\"a R=%0d\", R);\n\
         \x20 end else begin : b\n\
         \x20   localparam R = 6;\n\
         \x20 end endgenerate\n\
         \x20 generate if (1) begin : c\n\
         \x20   localparam R = 7;\n\
         \x20   initial $display(\"c R=%0d\", R);\n\
         \x20 end endgenerate\n\
         \x20 initial #1 $finish;\n\
         endmodule\n",
        &["g Q=1", "g Q=2", "a R=5", "c R=7"],
    );
    // A NESTED generate block re-declaring the enclosing block's name is §27.3
    // shadowing, exactly as it is against a module header. All three tools print the
    // inner `9` and the outer `0` / `1`.
    assert_runs(
        "nested block shadow",
        "module top;\n\
         \x20 genvar i;\n\
         \x20 generate for (i = 0; i < 2; i = i + 1) begin : g\n\
         \x20   localparam Q = i;\n\
         \x20   if (1) begin : h\n\
         \x20     localparam Q = 9;\n\
         \x20     initial $display(\"h Q=%0d\", Q);\n\
         \x20   end\n\
         \x20   initial $display(\"g Q=%0d\", Q);\n\
         \x20 end endgenerate\n\
         \x20 initial #1 $finish;\n\
         endmodule\n",
        &["h Q=9", "g Q=0", "g Q=1"],
    );
}

#[test]
fn the_two_shapes_this_rule_does_not_see_are_refused_by_its_siblings() {
    // (1) CONVERTED, same design, flipped expectation. A module instantiated ONLY under
    // `generate if (0)` is never elaborated, and this walk ran from `bind_params` — i.e.
    // once per INSTANCE — so nothing reached its declarations and vita printed `TOP=ok`
    // at exit 0. iverilog "p209.sv:2: error: 'P' has already been declared in this
    // scope."; verilator "%Error: p209.sv:2:17: Duplicate declaration of signal: 'P'".
    // The walk is per DEFINITION now (`driver.rs::run`), so root selection — auto-top,
    // `--top`, or a false generate — cannot decide whether a declaration is legal.
    // `decl_name_collisions_kinds.rs` carries the `--top` twins.
    let (rc, out) = run("module dead #(parameter int P = 3);\n\
         \x20 parameter int P = 7;\n\
         \x20 initial $display(\"D=%0d\", P);\n\
         endmodule\n\
         module top;\n\
         \x20 generate if (0) begin : g dead d(); end endgenerate\n\
         \x20 initial begin $display(\"TOP=ok\"); #1 $finish; end\n\
         endmodule\n");
    assert_eq!(
        rc, 1,
        "the definition is checked even with no instance:\n{out}"
    );
    assert!(
        out.contains("duplicate declaration of parameter `P`"),
        "{out}"
    );
    assert!(
        !out.contains("TOP=ok"),
        "and the design does not run:\n{out}"
    );
    // (2) CONVERTED, same design, flipped expectation. A duplicate VARIABLE in a named
    // block is a different name space with its own binder, and it printed `x=3` at exit
    // 0 — iverilog "m04.sv:5: error: 'x' has already been declared in this scope. : It
    // was declared here as a variable."; verilator "%Error: m04.sv:5:13: Duplicate
    // declaration of signal: 'x'". That binder is `hoist.rs`'s flatten, whose
    // skip-if-present could not tell a second declarator of ONE block from the same
    // name in ANOTHER block; a per-block set of flatten keys separates them, and this
    // file's own rule is still `ModuleItem::Param` only.
    // `decl_name_collisions_kinds.rs` carries the unnamed-block and comma twins and the
    // sibling-block / module-shadow controls.
    let (rc, out) = run("module top;\n\
         \x20 initial begin : blk\n\
         \x20   integer x;\n\
         \x20   integer x;\n\
         \x20   x = 3;\n\
         \x20   $display(\"x=%0d\", x);\n\
         \x20 end\n\
         \x20 initial #1 $finish;\n\
         endmodule\n");
    assert_eq!(rc, 1, "the sibling rule refuses it:\n{out}");
    assert!(
        out.contains("net/variable `top.x` redeclared (duplicate declaration)"),
        "…with `add_net`'s sentence, not this file's:\n{out}"
    );
    assert!(
        !out.contains("duplicate declaration of parameter"),
        "the parameter rule must not claim a variable:\n{out}"
    );
    assert!(!out.contains("x=3"), "and the block does not run:\n{out}");
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
fn a_parameter_and_a_net_of_one_name_are_a_sibling_rule() {
    // CONVERTED, same design, flipped expectation. This pinned `r=7` and said the
    // parameter-vs-NET collision was a neighbouring class left alone on purpose —
    // a different binder (`add_net`) and a different funnel. That class is closed now:
    // `decl_collide.rs` owns every pair of DIFFERENT binders (IEEE §3.13) and refuses
    // this one per definition. THIS file's gate is still `ModuleItem::Param` only, and
    // its own sentence still names the §6.20.1 / §27.2 / §26.2 scope rule — the pin
    // below is what keeps the two rules from merging into one message.
    let (rc, out) = run("module tb; localparam N = 7; logic [3:0] N;\n\
         \x20 initial $display(\"r=%0d\", N);\n\
         endmodule\n");
    assert_eq!(rc, 1, "the sibling rule refuses it:\n{out}");
    assert!(
        out.contains("`N` is declared twice in this module: as a localparam and as a variable"),
        "…and it is the §3.13 sentence, not this file's:\n{out}"
    );
    assert!(
        !out.contains("duplicate declaration of parameter"),
        "the parameter-vs-parameter sentence must not claim this pair:\n{out}"
    );
}
