//! The KIND half of `decl_name_collisions.rs`, split out at the repo's 1000-line
//! policy: one name, two declarations, two different binders (IEEE 1800-2017 §3.13).
//!
//! ROADMAP §2 rows P25(b) (a genvar against a parameter or a net, and a parameter
//! against a net), P25(a) (a duplicate parameter in a scope no binder reaches), X7
//! (the refusal SENTENCE per region), P22 (a call on a MODPORT name) and the P26
//! residue (a second declarator in a STATIC task body) — plus the negative pins for
//! everything that must NOT become loud. The routine rows P20 and P21 are in the
//! sibling file.
//!
//! Every refusing cell below is a design BOTH oracles reject and vita ran at exit 0 —
//! the values are the PRE binary's, recorded per cell. Every accepting cell is a design
//! both oracles run, and its `$display` text is pinned verbatim: this rule is a REJECT
//! gate, so the controls are what stop it widening.
//!
//! ORACLES: iverilog 13.0 (`-g2012`), Verilator 5.052 (`--binary --timing`). Two axes
//! carry only ONE oracle and are pinned as such rather than as parity:
//!   * `module dead #(parameter int P = 3); parameter int P = 7;` with no instance —
//!     iverilog rejects (it elaborates an uninstantiated module as its own top),
//!     verilator accepts (it never looks at the unit). IEEE declaration legality does
//!     not depend on instantiation, so vita follows iverilog;
//!   * `import pk::mp;` beside `modport mp` with NO call — iverilog rejects the import,
//!     verilator accepts. vita accepts, and that cell is pinned ACCEPTING so the call
//!     refusal cannot creep back to the declaration.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns `(exit_code, stdout+stderr)`. The code matters as much as the text: a
/// refusal that did not set the exit status would let a CI script run the design.
fn run_args(src: &str, extra: &[&str]) -> (i32, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dnc2_{}_{n}", std::process::id()));
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

/// The §6.20.1 / §27.2 duplicate-PARAMETER refusal (`param_dup.rs`), which keeps its
/// own sentence. `rule` is the scope clause this shape must quote.
fn assert_dup_param(name: &str, src: &str, ident: &str, rule: &str, extra: &[&str]) {
    let (rc, out) = run_args(src, extra);
    assert_eq!(rc, 1, "{name}: a refused design must exit 1:\n{out}");
    assert!(
        out.contains("VITA-E3009"),
        "{name}: the refusal is E3009:\n{out}"
    );
    let want = format!("duplicate declaration of parameter `{ident}`");
    assert!(out.contains(&want), "{name}: expected `{want}`:\n{out}");
    assert!(
        out.contains(rule),
        "{name}: expected the rule `{rule}`:\n{out}"
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

// ────────────────── P25(b): a genvar, a parameter and a net share one space ───────

#[test]
fn a_genvar_and_a_parameter_of_one_name_are_refused() {
    // PRE printed `Q=3` for all four spellings — the genvar simply was not a
    // declaration to any other binder. iverilog "'Q' has already been declared in this
    // scope."; verilator "Duplicate declaration of signal: 'Q'".
    assert_collision(
        "p25_b2",
        "module top;\n\
         \x20 genvar Q;\n\
         \x20 parameter int Q = 3;\n\
         \x20 initial begin $display(\"Q=%0d\", Q); #1 $finish; end\n\
         endmodule\n",
        "Q",
        "module",
        "a genvar",
        "a parameter",
    );
    assert_collision(
        "p25_b6",
        "module top;\n\
         \x20 genvar Q;\n\
         \x20 localparam int Q = 3;\n\
         \x20 initial begin $display(\"Q=%0d\", Q); #1 $finish; end\n\
         endmodule\n",
        "Q",
        "module",
        "a genvar",
        "a localparam",
    );
}

#[test]
fn a_genvar_and_a_localparam_in_a_transparent_region_are_refused() {
    // The region is flattened into the module's own sequence, exactly as its parameter
    // declarations are — so the pair is visible at module scope. PRE: `Q=3`.
    assert_collision(
        "p25_b1",
        "module top;\n\
         \x20 generate\n\
         \x20   genvar Q;\n\
         \x20   localparam int Q = 3;\n\
         \x20 endgenerate\n\
         \x20 initial begin $display(\"Q=%0d\", Q); #1 $finish; end\n\
         endmodule\n",
        "Q",
        "module",
        "a genvar",
        "a localparam",
    );
}

#[test]
fn a_genvar_and_a_net_of_one_name_are_refused() {
    // PRE printed `Q=z`: the wire existed and the genvar existed, separately.
    assert_collision(
        "p25_b3",
        "module top;\n\
         \x20 genvar Q;\n\
         \x20 wire Q;\n\
         \x20 initial begin $display(\"Q=%0b\", Q); #1 $finish; end\n\
         endmodule\n",
        "Q",
        "module",
        "a genvar",
        "a net",
    );
}

#[test]
fn a_genvar_declared_twice_is_refused() {
    // `add_net` never sees a genvar, so nothing compared these two. PRE: `TOP=ok`.
    assert_collision(
        "p25_b4",
        "module top;\n\
         \x20 genvar Q;\n\
         \x20 genvar Q;\n\
         \x20 initial begin $display(\"TOP=ok\"); #1 $finish; end\n\
         endmodule\n",
        "Q",
        "module",
        "a genvar",
        "a genvar",
    );
}

#[test]
fn a_parameter_and_a_net_of_one_name_are_refused() {
    // The cell `param_redeclare.rs` recorded as the NEIGHBOURING class and left alone:
    // `localparam int Q = 1; wire Q;`, PRE `Q=1`. `add_net`'s guard is net-vs-net only
    // and the parameter walk is parameter-vs-parameter only, so neither binder could
    // have reported it — which is why it needs this third walk rather than a widening
    // of either.
    assert_collision(
        "p25_b5",
        "module top;\n\
         \x20 localparam int Q = 1;\n\
         \x20 wire Q;\n\
         \x20 initial begin $display(\"Q=%0d\", Q); #1 $finish; end\n\
         endmodule\n",
        "Q",
        "module",
        "a localparam",
        "a net",
    );
}

#[test]
fn one_renamed_declaration_makes_every_p25b_shape_run() {
    // The five `_ctl` cells. `Q=z` in the b3 twin is the undriven-wire reading vita and
    // iverilog agree on (verilator reads `0`); it is unrelated to this rule and pinned
    // as vita's own value.
    assert_runs(
        "p25_b1_ctl",
        "module top;\n\
         \x20 generate\n\
         \x20   genvar G;\n\
         \x20   localparam int Q = 3;\n\
         \x20 endgenerate\n\
         \x20 initial begin $display(\"Q=%0d\", Q); #1 $finish; end\n\
         endmodule\n",
        &["Q=3"],
    );
    assert_runs(
        "p25_b3_ctl",
        "module top;\n\
         \x20 genvar G;\n\
         \x20 wire Q;\n\
         \x20 initial begin $display(\"Q=%0b\", Q); #1 $finish; end\n\
         endmodule\n",
        &["Q=z"],
    );
    assert_runs(
        "p25_b4_ctl",
        "module top;\n\
         \x20 genvar Q;\n\
         \x20 genvar R;\n\
         \x20 initial begin $display(\"TOP=ok\"); #1 $finish; end\n\
         endmodule\n",
        &["TOP=ok"],
    );
    assert_runs(
        "p25_b5_ctl",
        "module top;\n\
         \x20 localparam int Q = 1;\n\
         \x20 wire W;\n\
         \x20 initial begin $display(\"Q=%0d\", Q); #1 $finish; end\n\
         endmodule\n",
        &["Q=1"],
    );
    assert_runs(
        "p25_b6_ctl",
        "module top;\n\
         \x20 genvar G;\n\
         \x20 localparam int Q = 3;\n\
         \x20 initial begin $display(\"Q=%0d\", Q); #1 $finish; end\n\
         endmodule\n",
        &["Q=3"],
    );
}

// ─────────── P25(a): a duplicate parameter in a scope no binder ever reached ──────

const P25_A1: &str = "module dead #(parameter int P = 3);\n\
     \x20 parameter int P = 7;\n\
     \x20 initial $display(\"dead P=%0d\", P);\n\
     endmodule\n\
     module top;\n\
     \x20 generate if (0) begin : g\n\
     \x20   dead u();\n\
     \x20 end endgenerate\n\
     \x20 initial begin $display(\"TOP=ok\"); #1 $finish; end\n\
     endmodule\n";

const SEC_6201: &str = "a parameter port list and the module body are ONE declarative scope";

#[test]
fn a_module_instantiated_only_under_a_false_generate_is_still_checked() {
    // The row `param_redeclare.rs` recorded as "this rule still does not see": the check
    // ran from `bind_params`, i.e. per INSTANCE, and `collect_instantiated` descends a
    // generate-if without evaluating it — so `dead` is neither a root nor elaborated,
    // and nothing walked its declarations. PRE printed `TOP=ok` at exit 0; iverilog
    // "p209.sv:2: error: 'P' has already been declared in this scope."; verilator
    // "Duplicate declaration of signal: 'P'".
    assert_dup_param("p25_a1", P25_A1, "P", SEC_6201, &[]);
}

#[test]
fn pinning_the_top_does_not_hide_the_duplicate() {
    // `--top top` was the shape that survived even the accidental auto-top immunity:
    // with it, PRE was silent for BOTH the generate-if module and the never-instantiated
    // one. The check is per DEFINITION now, so the root selection cannot decide it.
    assert_dup_param("p25_a1 --top", P25_A1, "P", SEC_6201, &["--top", "top"]);
    assert_dup_param(
        "p25_a2 --top",
        "module dead #(parameter int P = 3);\n\
         \x20 parameter int P = 7;\n\
         \x20 initial $display(\"dead P=%0d\", P);\n\
         endmodule\n\
         module top;\n\
         \x20 initial begin $display(\"TOP=ok\"); #1 $finish; end\n\
         endmodule\n",
        "P",
        SEC_6201,
        &["--top", "top"],
    );
}

#[test]
fn the_staged_velab_lane_refuses_it_too() {
    // The grounding measured the STAGED shape as well, and it was the one that showed
    // the defect was root selection and not auto-top: `vcmp` -> `velab --top top` ->
    // `vrun` printed `TOP=ok` at exit 0 for p25_a1, and `velab --top dead` refused the
    // same file. The check runs before any root is picked now, so the staged lane
    // refuses it whichever unit is pinned. Driven through the library entry points, not
    // argv, because the staged binaries need `--features separate-bins` to exist at all.
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dnc2_stg_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let sv = d.join("t.sv");
    let vu = d.join("t.vu");
    let velab = d.join("t.velab");
    std::fs::write(&sv, P25_A1).unwrap();
    let path = |p: &std::path::Path| p.to_string_lossy().into_owned();
    assert_eq!(
        cli::run_vcmp(&[path(&sv)], Some(&path(&vu)), &cli::VitaOpts::default()),
        cli::EXIT_OK,
        "the duplicate is an ELABORATE-stage defect, so vcmp still succeeds"
    );
    let opts = cli::VitaOpts {
        tops: vec!["top".to_string()],
        ..cli::VitaOpts::default()
    };
    let code = cli::run_velab(&path(&vu), &path(&velab), &opts);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(
        code,
        cli::EXIT_USER_ERROR,
        "`velab --top top` must refuse the duplicate in `dead` (PRE returned EXIT_OK)"
    );
}

#[test]
fn a_never_instantiated_interface_is_checked_too() {
    // An interface is never a root, so it was never elaborated at all and PRE printed
    // `TOP=ok`. ONE oracle: iverilog rejects, verilator accepts (it never looks at the
    // unit). Declaration legality does not depend on instantiation.
    assert_dup_param(
        "p25_a4",
        "interface dead_if #(parameter int P = 3);\n\
         \x20 parameter int P = 7;\n\
         \x20 initial $display(\"dead_if P=%0d\", P);\n\
         endinterface\n\
         module top;\n\
         \x20 initial begin $display(\"TOP=ok\"); #1 $finish; end\n\
         endmodule\n",
        "P",
        "a parameter port list and the interface body are ONE declarative scope",
        &[],
    );
}

#[test]
fn a_live_generate_instance_is_still_refused_and_still_says_6201() {
    // p25_a3: the shape that ALREADY worked, and the control for the move of the check
    // from `bind_params` to the definition — it must not become silent, and its sentence
    // must not change.
    assert_dup_param(
        "p25_a3",
        "module dead #(parameter int P = 3);\n\
         \x20 parameter int P = 7;\n\
         \x20 initial $display(\"dead P=%0d\", P);\n\
         endmodule\n\
         module top;\n\
         \x20 generate if (1) begin : g\n\
         \x20   dead u();\n\
         \x20 end endgenerate\n\
         \x20 initial begin $display(\"TOP=ok\"); #1 $finish; end\n\
         endmodule\n",
        "P",
        SEC_6201,
        &[],
    );
}

#[test]
fn two_distinct_parameters_in_an_uninstantiated_module_still_run() {
    // The `_ctl` twin of p25_a1 / p25_a2: distinct names, no refusal, `TOP=ok`. Without
    // it, "check every definition" could have become "refuse every definition".
    assert_runs(
        "p25_a1_ctl",
        "module dead #(parameter int P = 3);\n\
         \x20 parameter int Q = 7;\n\
         \x20 initial $display(\"dead P=%0d Q=%0d\", P, Q);\n\
         endmodule\n\
         module top;\n\
         \x20 generate if (0) begin : g\n\
         \x20   dead u();\n\
         \x20 end endgenerate\n\
         \x20 initial begin $display(\"TOP=ok\"); #1 $finish; end\n\
         endmodule\n",
        &["TOP=ok"],
    );
    assert_runs(
        "p25_a4_ctl",
        "interface dead_if #(parameter int P = 3);\n\
         \x20 parameter int Q = 7;\n\
         endinterface\n\
         module top;\n\
         \x20 initial begin $display(\"TOP=ok\"); #1 $finish; end\n\
         endmodule\n",
        &["TOP=ok"],
    );
}

// ─────────────────────────────── X7: the refusal SENTENCE ────────────────────────

const SEC_272: &str = "a `generate … endgenerate` region with no block label is TRANSPARENT";

#[test]
fn a_transparent_region_duplicate_does_not_blame_a_parameter_port_list() {
    // x7_a: the module has no `#(...)` at all, and PRE told the reader its "parameter
    // port list and the module body" collided. The region's own rule (§27.2) is what
    // put the two declarations in one scope, so it is the rule the refusal quotes.
    assert_dup_param(
        "x7_a",
        "module top;\n\
         \x20 generate\n\
         \x20   localparam int Q = 1;\n\
         \x20   localparam int Q = 2;\n\
         \x20 endgenerate\n\
         \x20 initial begin $display(\"Q=%0d\", Q); #1 $finish; end\n\
         endmodule\n",
        "Q",
        SEC_272,
        &[],
    );
    let (_rc, out) = run("module top;\n\
         \x20 generate\n\
         \x20   localparam int Q = 1;\n\
         \x20   localparam int Q = 2;\n\
         \x20 endgenerate\n\
         \x20 initial begin $display(\"Q=%0d\", Q); #1 $finish; end\n\
         endmodule\n");
    assert!(
        !out.contains("parameter port list"),
        "x7_a must not name a construct the file does not contain:\n{out}"
    );
}

#[test]
fn an_interface_duplicate_says_interface_and_not_module() {
    // x7_b / x7_f: the §6.20.1 sentence is shared by the module and interface lanes and
    // said "the module body" in both. The unit kind is now a parameter of the sentence.
    assert_dup_param(
        "x7_f",
        "interface ifc #(parameter int P = 3);\n\
         \x20 parameter int P = 7;\n\
         endinterface\n\
         module top;\n\
         \x20 ifc w();\n\
         \x20 initial begin $display(\"TOP=ok\"); #1 $finish; end\n\
         endmodule\n",
        "P",
        "a parameter port list and the interface body are ONE declarative scope",
        &[],
    );
    assert_dup_param(
        "x7_b",
        "interface ifc;\n\
         \x20 generate\n\
         \x20   localparam int Q = 1;\n\
         \x20   localparam int Q = 2;\n\
         \x20 endgenerate\n\
         endinterface\n\
         module top;\n\
         \x20 ifc w();\n\
         \x20 initial begin $display(\"TOP=ok\"); #1 $finish; end\n\
         endmodule\n",
        "Q",
        SEC_272,
        &[],
    );
}

#[test]
fn the_labelled_block_and_package_sentences_are_unchanged() {
    // x7_c / x7_d: the two sentences that were already right. A labelled generate block
    // is a declarative region of its own (§27.3) and a package body is one too (§26.2);
    // neither may be replaced by the transparent-region sentence.
    assert_dup_param(
        "x7_c",
        "module top;\n\
         \x20 genvar i;\n\
         \x20 generate\n\
         \x20   for (i = 0; i < 2; i = i + 1) begin : g\n\
         \x20     localparam int Q = i;\n\
         \x20     localparam int Q = i + 1;\n\
         \x20   end\n\
         \x20 endgenerate\n\
         \x20 initial begin $display(\"TOP=ok\"); #1 $finish; end\n\
         endmodule\n",
        "Q",
        "a generate block is ONE declarative scope (IEEE 1800-2017 §27.3)",
        &[],
    );
    assert_dup_param(
        "x7_d",
        "package pk;\n\
         \x20 parameter int P = 1;\n\
         \x20 parameter int P = 2;\n\
         endpackage\n\
         module top;\n\
         \x20 import pk::*;\n\
         \x20 initial begin $display(\"P=%0d\", P); #1 $finish; end\n\
         endmodule\n",
        "P",
        "a package body is ONE declarative scope (IEEE 1800-2017 §26.2)",
        &[],
    );
}

// ───────────────────── P22: a call whose dotted name is a MODPORT ────────────────

#[test]
fn a_call_on_a_modport_name_is_refused() {
    // p22_b: `w.mp(40)` where `mp` is a MODPORT of `w`'s interface AND a wildcard-
    // imported function. Both oracles resolve the dotted name to the modport and reject
    // the call — verilator "Found definition of 'w.mp' as a MODPORT but expected a
    // task/function", iverilog "No function named `w.mp' found in this context (top)".
    // vita took the imported function and printed `R=44`.
    let (rc, out) = run("package pk;\n\
         \x20 function int mp(input int a); return a + 4; endfunction\n\
         endpackage\n\
         interface ifc;\n\
         \x20 import pk::*;\n\
         \x20 logic [3:0] x;\n\
         \x20 modport mp (input x);\n\
         endinterface\n\
         module top;\n\
         \x20 ifc w();\n\
         \x20 int r;\n\
         \x20 initial begin r = w.mp(40); $display(\"R=%0d\", r); #1 $finish; end\n\
         endmodule\n");
    assert_eq!(rc, 1, "p22_b: the call must be refused:\n{out}");
    assert!(
        out.contains("VITA-E3009") && out.contains("`w.mp` names a modport of interface `ifc`"),
        "p22_b: the refusal names the dotted call and the interface:\n{out}"
    );
    assert!(
        !out.contains("R=44"),
        "p22_b: the call must not run:\n{out}"
    );
}

#[test]
fn an_explicit_import_of_the_modport_name_is_refused_at_the_call_too() {
    // p22_a: the same call with `import pk::mp;`. iverilog rejects at the MODPORT line
    // (an explicit import of a name the interface declares) and verilator at the CALL;
    // both reject the file, so refusing the call is a rung up either way.
    let (rc, out) = run("package pk;\n\
         \x20 function int mp(input int a); return a + 4; endfunction\n\
         endpackage\n\
         interface ifc;\n\
         \x20 import pk::mp;\n\
         \x20 logic [3:0] x;\n\
         \x20 modport mp (input x);\n\
         endinterface\n\
         module top;\n\
         \x20 ifc w();\n\
         \x20 int r;\n\
         \x20 initial begin r = w.mp(40); $display(\"R=%0d\", r); #1 $finish; end\n\
         endmodule\n");
    assert_eq!(rc, 1, "p22_a: the call must be refused:\n{out}");
    assert!(
        out.contains("`w.mp` names a modport of interface `ifc`"),
        "p22_a:\n{out}"
    );
}

#[test]
fn the_declaration_pair_alone_is_not_refused() {
    // p22_b2 — BOTH oracles run this: a wildcard import beside a `modport` of one of the
    // package's names does not collide at the declaration. p22_a2 is the explicit-import
    // twin, which only iverilog rejects; vita follows verilator there. Both are pinned
    // ACCEPTING so the CALL refusal can never migrate to the declaration.
    for (tag, imp) in [("p22_b2", "import pk::*;"), ("p22_a2", "import pk::mp;")] {
        assert_runs(
            tag,
            &format!(
                "package pk;\n\
                 \x20 function int mp(input int a); return a + 4; endfunction\n\
                 endpackage\n\
                 interface ifc;\n\
                 \x20 {imp}\n\
                 \x20 logic [3:0] x;\n\
                 \x20 modport mp (input x);\n\
                 endinterface\n\
                 module top;\n\
                 \x20 ifc w();\n\
                 \x20 initial begin $display(\"R=ok\"); #1 $finish; end\n\
                 endmodule\n"
            ),
            &["R=ok"],
        );
    }
}

#[test]
fn a_call_on_a_non_modport_interface_routine_still_runs() {
    // p22_b_ctl: the modport is named `mq`, so `w.mp(40)` still resolves to the imported
    // function and prints `R=44` — the control that says the refusal is keyed on the
    // MEMBER name and not on "the receiver is an interface instance". (iverilog cannot
    // call an interface-instance function at all, so verilator is the oracle here.)
    assert_runs(
        "p22_b_ctl",
        "package pk;\n\
         \x20 function int mp(input int a); return a + 4; endfunction\n\
         endpackage\n\
         interface ifc;\n\
         \x20 import pk::*;\n\
         \x20 logic [3:0] x;\n\
         \x20 modport mq (input x);\n\
         endinterface\n\
         module top;\n\
         \x20 ifc w();\n\
         \x20 int r;\n\
         \x20 initial begin r = w.mp(40); $display(\"R=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        &["R=44"],
    );
}

// ─────────────── P26 residue: a second declarator in a STATIC task body ──────────

#[test]
fn a_duplicate_local_in_a_static_task_body_is_refused() {
    // p26_f / p26_t: `hoist_one_inline_local` skips a name already in `symbols` so that
    // ONE static task inlined at N call sites shares one net (§6.21) — and that skip
    // swallowed a SECOND DECLARATOR in one body. PRE ran the task and printed `x=43`:
    // the LAST declarator's initializer won, and with no initializers at all the two
    // names simply collapsed onto one net. Both oracles reject; the framed twin
    // (`task automatic`) was already loud with this exact sentence.
    for (tag, body) in [
        ("p26_f", "int x = 1;\n\x20   int x = 3;\n\x20   r = a + x;"),
        (
            "p26_t",
            "int x;\n\x20   int x;\n\x20   x = 3;\n\x20   r = a + x;",
        ),
    ] {
        let (rc, out) = run(&format!(
            "module top;\n\
             \x20 int r;\n\
             \x20 task tk(input int a);\n\
             \x20   {body}\n\
             \x20 endtask\n\
             \x20 initial begin tk(40); $display(\"x=%0d\", r); #1 $finish; end\n\
             endmodule\n"
        ));
        assert_eq!(rc, 1, "{tag}: must be refused:\n{out}");
        assert!(
            out.contains("VITA-E3009") && out.contains("redeclared (duplicate declaration)"),
            "{tag}: the `add_net` sentence:\n{out}"
        );
        assert!(
            !out.contains("x=43"),
            "{tag}: the task must not run:\n{out}"
        );
    }
}

#[test]
fn one_static_task_inlined_twice_still_shares_one_local() {
    // THE control for the per-inlining set: the skip it narrows is what makes a static
    // local ONE variable across calls (§6.21), and both oracles agree — the second call
    // sees the first call's value and the initializer does NOT re-run. `x=1` then `x=2`
    // (not `x=1` twice) is the whole point.
    assert_runs(
        "static local retention",
        "module top;\n\
         \x20 task tk;\n\
         \x20   int x = 0;\n\
         \x20   x = x + 1;\n\
         \x20   $display(\"x=%0d\", x);\n\
         \x20 endtask\n\
         \x20 initial begin tk(); tk(); #1 $finish; end\n\
         endmodule\n",
        &["x=1", "x=2"],
    );
    // …and the p26_f / p26_t `_ctl` twins: two DIFFERENT names in one body.
    assert_runs(
        "p26_f_ctl",
        "module top;\n\
         \x20 int r;\n\
         \x20 task tk(input int a);\n\
         \x20   int x = 1;\n\
         \x20   int y = 3;\n\
         \x20   r = a + x + y - 4;\n\
         \x20 endtask\n\
         \x20 initial begin tk(40); $display(\"x=%0d\", r); #1 $finish; end\n\
         endmodule\n",
        &["x=40"],
    );
}

// ──────────────────── negative pins: what must NOT become loud ───────────────────

#[test]
fn a_non_ansi_port_and_its_own_net_declaration_still_run() {
    // `module top(a); input a; wire a;` is ONE declaration spelled in two items, and
    // both oracles RUN it — measured: iverilog prints `A=z`, verilator `A=0` (the
    // undriven-wire split). A port-vs-net arm in the pair table would have refused every
    // non-ANSI module in the corpus.
    assert_runs(
        "non-ANSI port + net",
        "module top(a);\n\
         \x20 input a;\n\
         \x20 wire a;\n\
         \x20 initial begin $display(\"A=%0b\", a); #1 $finish; end\n\
         endmodule\n",
        &["A=z"],
    );
    assert_runs(
        "non-ANSI port + reg",
        "module top(a, b);\n\
         \x20 input a;\n\
         \x20 output b;\n\
         \x20 wire a;\n\
         \x20 reg b;\n\
         \x20 initial begin b = 1'b1; $display(\"A=%0b B=%0b\", a, b); #1 $finish; end\n\
         endmodule\n",
        &["A=z B=1"],
    );
}

#[test]
fn the_parsers_own_desugars_are_still_not_duplicates() {
    // A header ARRAY parameter emits a `ParamDecl` in `module.params` AND a
    // `const_param` `NetVar` twin at the front of the body, which reads as
    // parameter-vs-net; a `parameter type T` emits the `T$w` / `T$s` carriers, which
    // read as a name declared several times. Both would have gone loud here.
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
fn a_duplicate_local_in_one_block_is_refused_on_the_flatten_path() {
    // The FLATTEN twin of the static-task residue above, and the shape the P26 row kept
    // open. `hoist.rs`'s "skip re-creating it rather than erroring redeclared" exists so
    // two SEQUENTIAL blocks reusing one temp name share one net; it asks `symbols`, and
    // by the time a SECOND DECLARATOR OF THE SAME BLOCK arrives the first has already
    // written it — so the duplicate was swallowed and the block ran. Measured on PRE and
    // on POST-without-this-guard: `x=3` at exit 0, named and unnamed block alike, where
    // iverilog says "'x' has already been declared in this scope. : It was declared here
    // as a variable." and verilator "Duplicate declaration of signal: 'x'".
    //
    // The grounding cell p26_q was loud for a DIFFERENT reason (its read-before-assign
    // guard fired), which is what kept the class looking closed.
    for (tag, head) in [("named block", "begin : blk"), ("unnamed block", "begin")] {
        let (rc, out) = run(&format!(
            "module top;\n\
             \x20 initial {head}\n\
             \x20   integer x;\n\
             \x20   integer x;\n\
             \x20   x = 3;\n\
             \x20   $display(\"x=%0d\", x);\n\
             \x20 end\n\
             \x20 initial #1 $finish;\n\
             endmodule\n"
        ));
        assert_eq!(rc, 1, "{tag}: must be refused:\n{out}");
        assert!(
            out.contains("VITA-E3009")
                && out.contains("net/variable `top.x` redeclared (duplicate declaration)"),
            "{tag}: the `add_net` sentence, on the flattened key:\n{out}"
        );
        assert!(
            !out.contains("x=3"),
            "{tag}: the block must not run:\n{out}"
        );
    }
    // A comma declaration is two declarators of one block too.
    let (rc, out) = run("module top;\n\
         \x20 initial begin : blk\n\
         \x20   integer x, y;\n\
         \x20   integer x;\n\
         \x20   x = 3; y = 4;\n\
         \x20   $display(\"x=%0d y=%0d\", x, y);\n\
         \x20 end\n\
         \x20 initial #1 $finish;\n\
         endmodule\n");
    assert_eq!(rc, 1, "comma declaration:\n{out}");
    assert!(
        out.contains("net/variable `top.x` redeclared (duplicate declaration)"),
        "…and it names `x`, not `y`:\n{out}"
    );
}

#[test]
fn a_block_local_and_a_sibling_block_still_run() {
    // p26_k / p26_l / p26_m — and THE control for the per-block set above: a block
    // label is collected ONLY for a block written directly as a procedural block's
    // body, and a block-local variable is a different name space with its own binder.
    // Two SIBLING blocks may reuse a name (the flatten's deliberate coalesce, which the
    // per-block set must not touch), a fork arm may declare one, and a block-local may
    // shadow a module net — all three run in both oracles.
    assert_runs(
        "p26_k",
        "module top;\n\
         \x20 initial begin\n\
         \x20   begin : b1 int x = 1; $display(\"b1 x=%0d\", x); end\n\
         \x20   begin : b2 int x = 3; $display(\"b2 x=%0d\", x); end\n\
         \x20   #1 $finish;\n\
         \x20 end\n\
         endmodule\n",
        &["b1 x=1", "b2 x=3"],
    );
    assert_runs(
        "p26_l",
        "module top;\n\
         \x20 initial begin\n\
         \x20   fork : fb\n\
         \x20     begin int x = 1; $display(\"f x=%0d\", x); end\n\
         \x20   join\n\
         \x20   #1 $finish;\n\
         \x20 end\n\
         endmodule\n",
        &["f x=1"],
    );
    assert_runs(
        "p26_m",
        "module top;\n\
         \x20 logic [7:0] x;\n\
         \x20 initial begin x = 8'hAA; end\n\
         \x20 initial begin : b\n\
         \x20   int x = 3;\n\
         \x20   #1 $display(\"blk x=%0d mod x=%0h\", x, top.x);\n\
         \x20   #1 $finish;\n\
         \x20 end\n\
         endmodule\n",
        &["blk x=3 mod x=aa"],
    );
}

#[test]
fn a_generate_block_may_shadow_a_module_declaration() {
    // A LABELLED generate block is a declarative region of its own (§27.3), so a
    // function or a localparam it declares SHADOWS the module's — both oracles run
    // this, and the transparent-region flatten must not pull a labelled block in.
    assert_runs(
        "labelled block shadow",
        "module top;\n\
         \x20 localparam int Q = 1;\n\
         \x20 generate if (1) begin : g\n\
         \x20   localparam int Q = 2;\n\
         \x20   initial $display(\"gQ=%0d\", Q);\n\
         \x20 end endgenerate\n\
         \x20 initial begin $display(\"Q=%0d\", Q); #1 $finish; end\n\
         endmodule\n",
        &["gQ=2", "Q=1"],
    );
}

#[test]
fn one_source_defect_reports_once_however_many_instances() {
    // The definition-level cadence is what makes this true by construction: `sub a();
    // sub b();` of a module with a name declared twice reports ONE defect.
    let (rc, out) = run("module sub;\n\
         \x20 wire f;\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         endmodule\n\
         module top; sub a(); sub b(); initial #1 $finish; endmodule\n");
    assert_eq!(rc, 1, "still refused:\n{out}");
    assert_eq!(
        out.matches("is declared twice in this").count(),
        1,
        "two instances of one bad module report once:\n{out}"
    );
}

#[test]
fn two_duplicated_names_report_twice() {
    // The counterpart: the walk reports per NAME, so two independent defects are two
    // reports — and in SOURCE order, not in the map's order.
    let (rc, out) = run("module top;\n\
         \x20 wire f;\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 genvar Q;\n\
         \x20 localparam int Q = 3;\n\
         \x20 initial #1 $finish;\n\
         endmodule\n");
    assert_eq!(rc, 1, "refused:\n{out}");
    assert_eq!(
        out.matches("is declared twice in this").count(),
        2,
        "two duplicated names, two reports:\n{out}"
    );
    let f_at = out.find("`f` is declared twice").expect("f reported");
    let q_at = out.find("`Q` is declared twice").expect("Q reported");
    assert!(f_at < q_at, "reports come in source order:\n{out}");
}

#[test]
fn a_name_declared_three_times_reports_once() {
    // A non-ANSI header name, its `PortDecl` and a function: three declarations of one
    // name, two of which are the SAME declaration in two items. One report, naming the
    // port and the function.
    let (rc, out) = run("module top(f);\n\
         \x20 input f;\n\
         \x20 function int f(input int a); return a + 4; endfunction\n\
         \x20 initial #1 $finish;\n\
         endmodule\n");
    assert_eq!(rc, 1, "refused:\n{out}");
    assert_eq!(
        out.matches("is declared twice in this").count(),
        1,
        "one name, one report:\n{out}"
    );
    assert!(
        out.contains("as a port and as a function"),
        "the pair is port + function:\n{out}"
    );
}
