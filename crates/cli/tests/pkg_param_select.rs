//! A BIT / PART / INDEXED-PART select of a **package** constant.
//!
//! §4.5.363 opened the constant-domain select of a parameter and left one base
//! shape declined: the package one. The queue recorded the prerequisite as *"the
//! package const table has to carry declared width provenance"*, and that is
//! exactly what was missing — `pkg_const_meta` records widths, but it mixes
//! DECLARED ones with widths INFERRED from a folded value, so no consumer could
//! ask it the only question that matters here (*"is this width a declared
//! fact?"*). `pkg_const_range` is that table, filled by the same
//! `param_decl_range_opt` the module-scope twin uses.
//!
//! The census found **five** lanes behind the one queue line, not one:
//!
//! | lane | module scope | `pk::W` / bare-imported |
//! |---|---|---|
//! | constant width or bound | correct | **silent 1** (`$size` gave `x`) |
//! | runtime read, zero-LSB declaration | correct | correct |
//! | runtime read, `[39:8]` | correct | **silent 171** (raw internal bits) |
//! | runtime read, `[0:31]` | correct | **loud** |
//! | intra-package sibling (`parameter Q = W[7:0];`) | — | **loud** |
//!
//! and all three SPELLINGS of the same select — `pk::W[m:l]`, the bare name after
//! `import pk::*`, the bare name after `import pk::W` — behaved identically, so
//! they are one gap, not three.
//!
//! Every value here is pinned to LIVE iverilog 13.0, and every one of them is also
//! verilator 5.050's answer.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pkgsel_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr),
        out.status.code(),
    )
}

/// `parameter [31:0] W = 32'hAB34` in a package; `W[7:0]` is `0x34` = 52.
const PKG: &str = "package pk;\n\
   parameter  [31:0]  W = 32'hAB34;\n\
   parameter  [39:8]  B = 32'hAB34;\n\
   parameter  [0:31]  A = 32'hAB34;\n\
   localparam [31:0]  L = 32'hAB34;\n\
 endpackage\n";

// ── the constant-width lane, one cell per spelling ───────────────────────────

#[test]
fn package_scoped_select_sizes_a_net() {
    let (out, c) = run(&format!(
        "{PKG}module top;\n\
           logic [pk::W[7:0]-1:0] v;\n\
           initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(c, Some(0));
    assert!(out.contains("BITS=52"), "pk::W[7:0] as a width:\n{out}");
}

#[test]
fn wildcard_imported_select_sizes_a_net() {
    let (out, c) = run(&format!(
        "{PKG}import pk::*;\n\
         module top;\n\
           logic [W[7:0]-1:0] v;\n\
           initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(c, Some(0));
    assert!(out.contains("BITS=52"), "bare imported W[7:0]:\n{out}");
}

#[test]
fn explicit_imported_select_sizes_a_net() {
    let (out, c) = run(&format!(
        "{PKG}import pk::W;\n\
         module top;\n\
           logic [W[7:0]-1:0] v;\n\
           initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(c, Some(0));
    assert!(
        out.contains("BITS=52"),
        "explicitly imported W[7:0]:\n{out}"
    );
}

/// The other constant consumers of the same fold. `$size` is here because its
/// failure mode was not a wrong number: an unpacked dimension of `x`.
#[test]
fn every_constant_consumer_of_the_select() {
    for (what, body) in [
        (
            "unpacked dim",
            "logic v [pk::W[7:0]-1:0];\n\
             initial begin $display(\"BITS=%0d\", $size(v)); $finish; end\n",
        ),
        (
            "replication count",
            "initial begin $display(\"BITS=%0d\", $bits({pk::W[7:0]{1'b1}})); $finish; end\n",
        ),
        (
            "localparam value",
            "localparam int Q = pk::W[7:0];\n\
             initial begin $display(\"BITS=%0d\", Q); $finish; end\n",
        ),
        (
            "generate condition",
            "generate if (pk::W[7:0] == 52) begin : g\n\
               initial begin $display(\"BITS=52\"); $finish; end\n\
             end else begin : h\n\
               initial begin $display(\"BITS=BAD\"); $finish; end\n\
             end endgenerate\n",
        ),
    ] {
        let (out, c) = run(&format!("{PKG}module top;\n{body}endmodule\n"));
        assert_eq!(c, Some(0), "{what}:\n{out}");
        assert!(out.contains("BITS=52"), "{what}:\n{out}");
    }
}

/// A generate scope and a submodule scope are separate lowering scopes; the
/// package table is reached from both.
#[test]
fn the_select_folds_in_a_nested_scope() {
    for (what, src) in [
        (
            "generate scope",
            format!(
                "{PKG}module top;\n\
                   generate if (1) begin : g\n\
                     logic [pk::W[7:0]-1:0] v;\n\
                     initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
                   end endgenerate\n\
                 endmodule\n"
            ),
        ),
        (
            "submodule scope",
            format!(
                "{PKG}module sub();\n\
                   logic [pk::W[7:0]-1:0] v;\n\
                   initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
                 endmodule\n\
                 module top; sub u(); endmodule\n"
            ),
        ),
        (
            "submodule port list",
            format!(
                "{PKG}module sub(o); output [pk::W[7:0]-1:0] o;\n\
                   initial begin $display(\"BITS=%0d\", $bits(o)); $finish; end\n\
                 endmodule\n\
                 module top; wire [200:0] w; sub u(.o(w)); endmodule\n"
            ),
        ),
        (
            "instance parameter override",
            format!(
                "{PKG}module sub #(parameter int N = 1) ();\n\
                   initial begin $display(\"BITS=%0d\", N); $finish; end\n\
                 endmodule\n\
                 module top; sub #(.N(pk::W[7:0])) u(); endmodule\n"
            ),
        ),
    ] {
        let (out, c) = run(&src);
        assert_eq!(c, Some(0), "{what}:\n{out}");
        assert!(out.contains("BITS=52"), "{what}:\n{out}");
    }
}

// ── the RUNTIME lane: the declared LSB and direction were never subtracted ───

/// ⚠️ The queue line said *"the runtime lane already prints the right value in
/// all three tools"*. That was measured on a ZERO-LSB declaration, where the
/// normalization is a no-op. `parameter [39:8] B` made `pk::B[15:8]` print **171**
/// — the raw internal bits 15..8 of the stored value — where both oracles print
/// 52, at exit 0.
#[test]
fn a_nonzero_lsb_package_param_normalizes_its_runtime_select() {
    let (out, c) = run(&format!(
        "{PKG}module top;\n\
           initial begin $display(\"V=%0d %0d\", pk::B[15:8], pk::B[23:16]); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(c, Some(0));
    assert!(out.contains("V=52 171"), "pk::B runtime select:\n{out}");
}

/// The ascending declaration's runtime select was LOUD, not merely wrong.
#[test]
fn an_ascending_package_param_reads_its_runtime_select() {
    let (out, c) = run(&format!(
        "{PKG}module top;\n\
           initial begin $display(\"V=%0d\", pk::A[24:31]); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(c, Some(0));
    assert!(
        out.contains("V=52"),
        "ascending pk::A runtime select:\n{out}"
    );
}

#[test]
fn the_bare_imported_spelling_normalizes_the_same_way() {
    let (out, c) = run(&format!(
        "{PKG}import pk::*;\n\
         module top;\n\
           initial begin $display(\"V=%0d %0d\", B[15:8], A[24:31]); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(c, Some(0));
    assert!(
        out.contains("V=52 52"),
        "bare imported runtime select:\n{out}"
    );
}

// ── the intra-package sibling lane ───────────────────────────────────────────

/// A package parameter written in terms of a SELECT of an earlier one. The
/// package's own fold makes each declared range live for the rest of the package
/// exactly as it already made `param_meta` live, then restores it.
#[test]
fn a_package_parameter_may_select_a_sibling() {
    let (out, c) = run("package pk;\n\
           parameter [31:0] W = 32'hAB34;\n\
           parameter [39:8] B = 32'hAB34;\n\
           parameter int    Q = W[7:0];\n\
           parameter int    R = B[15:8];\n\
         endpackage\n\
         module top;\n\
           initial begin $display(\"V=%0d %0d\", pk::Q, pk::R); $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("V=52 52"),
        "intra-package sibling select:\n{out}"
    );
}

/// …and the range must NOT leak out of the package: a module-scope name of the
/// same spelling with no declared range keeps declining.
#[test]
fn the_package_range_does_not_leak_to_a_module_name() {
    let (out, c) = run("package pk;\n\
           parameter [31:0] W = 32'hAB34;\n\
         endpackage\n\
         module top;\n\
           localparam W = ~8'hAB;\n\
           logic [W[7:0]-1:0] v;\n\
           initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0));
    // The module's own `W` has a VALUE-inferred width, so the fold declines and
    // the bound clamps to 1 — the pre-existing §4.5.363 residue, unchanged. What
    // must never happen is 52: that would be the package's declaration answering
    // for a different object's value.
    assert!(
        out.contains("BITS=1"),
        "module-scope W is not pk::W:\n{out}"
    );
}

// ── the provenance gate, and the bindings that could stale it ────────────────

/// ⚠️ The gate that keeps this from trading one silent-wrong for another. A
/// package parameter whose width is INFERRED from its initializer's value has no
/// `pkg_const_range` entry, so the select declines exactly as the module-scope
/// twin does — landing on iverilog's answer, which is also 1 here. (verilator
/// answers 60; the two oracles split on this shape, so it is not a target.)
#[test]
fn a_value_inferred_package_width_declines() {
    let (out, c) = run("package pk;\n\
           parameter W = ~8'hCB;\n\
         endpackage\n\
         module top;\n\
           logic [(pk::W[15:8])+8-1:0] v;\n\
           initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("BITS=1"),
        "value-inferred package width:\n{out}"
    );
}

/// A local declaration wins the import — for the RANGE as well as the value.
/// `bind_param_range` is set-or-CLEAR precisely so the two cannot separate.
#[test]
fn a_local_declaration_wins_the_imported_range() {
    let (out, c) = run("package pk;\n\
           parameter [31:0] W = 32'hAB34;\n\
         endpackage\n\
         import pk::*;\n\
         module top;\n\
           localparam [39:8] W = 32'hAB22;\n\
           logic [W[15:8]-1:0] v;\n\
           initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0));
    // 0xAB22 on `[39:8]`: declared bits 15..8 are `0x22` = 34.
    assert!(
        out.contains("BITS=34"),
        "local range wins the import:\n{out}"
    );
}

// ── shapes that must stay exactly as loud as they were ──────────────────────

#[test]
fn shapes_that_stay_loud() {
    for (what, src) in [
        (
            // A package STRING parameter has no i64 value and is not in
            // `pkg_consts`; both oracles answer 68 here, and closing that is the
            // string constant domain's gap, not this one.
            "string package parameter",
            "package pk; parameter S = \"RED\"; endpackage\n\
             module top; logic [pk::S[7:0]-1:0] v;\n\
               initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
             endmodule\n",
        ),
        (
            // Two wildcard imports make the name ambiguous (§26.8) — iverilog
            // agrees it is an error.
            "ambiguous wildcard name",
            "package pa; parameter [31:0] W = 32'hAB34; endpackage\n\
             package pb; parameter [31:0] W = 32'hAB99; endpackage\n\
             import pa::*; import pb::*;\n\
             module top; logic [W[7:0]-1:0] v;\n\
               initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
             endmodule\n",
        ),
        (
            // A package const ARRAY element in a width — `const_array_vals_of_base`
            // owns this shape and the new arm refuses it, as the module-scope arm
            // does. iverilog rejects unpacked array parameters outright.
            "package const array element",
            "package pk; parameter int ROT [0:3] = '{10,20,52,40}; endpackage\n\
             module top; logic [pk::ROT[2]-1:0] v;\n\
               initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
             endmodule\n",
        ),
    ] {
        let (out, c) = run(src);
        assert_ne!(c, Some(0), "{what} should stay loud:\n{out}");
    }
}

/// A select reaching outside the declared range reads `x` for the outside bits
/// (§11.5.1), which the integer domain cannot represent — it declines, and lands
/// on iverilog's answer.
#[test]
fn an_out_of_range_package_select_declines() {
    let (out, c) = run("package pk; parameter [7:0] W = 8'h34; endpackage\n\
         module top; logic [pk::W[15:8]+52-1:0] v;\n\
           initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
         endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("BITS=1"),
        "out-of-range package select:\n{out}"
    );
}

/// The sign bits above the declared width belong to the i64 container, not to the
/// value: `parameter signed [31:0] W = -32'sd52` selects `W[15:8]` = 255.
#[test]
fn a_signed_negative_package_param_selects_the_masked_byte() {
    let (out, c) = run(
        "package pk; parameter signed [31:0] W = -32'sd52; endpackage\n\
         module top; logic [pk::W[15:8]-203:0] v;\n\
           initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(c, Some(0));
    assert!(
        out.contains("BITS=53"),
        "signed negative package param:\n{out}"
    );
}

/// ⚠️ The diagnostic half. `W-PARSE-SELECT-BASE` says the select "applies to an
/// expression, not to a net or variable" — a false statement about `pk::W[7:0]`,
/// which is a name (§26.3) that both oracles accept. It fired on the package
/// spelling alone, so the same select drew a portability warning or nothing at all
/// depending on which of its three spellings you wrote.
#[test]
fn a_select_on_a_package_scoped_name_is_not_a_portability_warning() {
    let (out, c) = run(&format!(
        "{PKG}module top;\n\
           initial begin $display(\"V=%0d\", pk::W[7:0]); $finish; end\n\
         endmodule\n"
    ));
    assert_eq!(c, Some(0));
    assert!(out.contains("V=52"), "{out}");
    assert!(
        !out.contains("W-PARSE-SELECT-BASE"),
        "a package-scoped name must not draw the non-portable-base warning:\n{out}"
    );
}

// ── what the adversarial review measured, and what it cost ──────────────────

/// ⚠️⚠️ THE REGRESSION THIS SLICE CREATED, and the reason `bind_param_value` exists.
///
/// `param_range` is keyed exactly like `params`, and until this slice no binder could
/// rebind a key another binder had ranged — the FQ key made every scope disjoint. A
/// wildcard import breaks that: it binds a PACKAGE declaration at the MODULE's own
/// key, and a local enum label or genvar rebinds the value one phase later while the
/// package's range stayed behind. Both spellings were correct→silent-wrong at exit 0,
/// and both are pinned here at the value all three tools gave before the slice.
#[test]
fn a_local_binding_of_an_imported_name_does_not_inherit_the_package_range() {
    // The enum label: `W` is 0xDEADBEEF, read at `[15:8]` of a 32-bit label = 0xBE.
    let (out, c) = run(
        "package pk;\n           parameter [39:8] W = 32'hAB34;\n         endpackage\n         module top;\n           import pk::*;\n           typedef enum logic [31:0] { W = 32'hDEADBEEF } e_t;\n           initial begin $display(\"V=%0d\", W[15:8]); $finish; end\n         endmodule\n",
    );
    assert_eq!(c, Some(0));
    assert!(
        out.contains("V=190"),
        "enum label must not use pk::W's range:\n{out}"
    );

    // The genvar: `W` is 8, and `[11:8]` of a genvar's value is 0.
    let (out, c) = run(
        "package pk;\n           parameter [39:8] W = 32'hAB34;\n         endpackage\n         module top;\n           import pk::*;\n           genvar W;\n           generate for (W = 8; W < 9; W = W + 1) begin : g\n             initial begin $display(\"V=%0d\", W[11:8]); $finish; end\n           end endgenerate\n         endmodule\n",
    );
    assert_eq!(c, Some(0));
    assert!(
        out.contains("V=0"),
        "genvar must not use pk::W's range:\n{out}"
    );
}

/// A parenthesised base names the same object. Both oracles reject the spelling
/// outright (vita keeps the value and says so, `W-PARSE-SELECT-BASE`), so the target
/// is vita's own answer to the same select two characters away — which the first
/// version of this slice made differ.
#[test]
fn a_parenthesis_does_not_change_which_bits_a_select_names() {
    let (out, c) = run(&format!(
        "{PKG}module top;\n           initial begin $display(\"V=%0h %0h\", pk::B[15:8], (pk::B)[15:8]); $finish; end\n         endmodule\n"
    ));
    assert_eq!(c, Some(0));
    assert!(out.contains("V=34 34"), "parenthesised base:\n{out}");
}

/// A NESTED select reaches the wide bit domain, which resolves the base by NAME —
/// and that resolver declined every narrow package constant, so the last asymmetry
/// between the three spellings lived here: the bare-imported spelling folded and the
/// `pkg::` one declared one bit.
#[test]
fn a_nested_select_folds_through_every_spelling() {
    for (what, pre, base) in [
        ("package", "", "pk::W"),
        ("wildcard import", "import pk::*;\n", "W"),
        ("explicit import", "import pk::W;\n", "W"),
    ] {
        let (out, c) = run(&format!(
            "{PKG}{pre}module top;\n               logic [{base}[15:0][7:0]-1:0] v;\n               initial begin $display(\"BITS=%0d\", $bits(v)); $finish; end\n             endmodule\n"
        ));
        assert_eq!(c, Some(0), "{what}:\n{out}");
        assert!(out.contains("BITS=52"), "{what}:\n{out}");
    }
}
