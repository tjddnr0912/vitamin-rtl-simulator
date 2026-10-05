//! ROADMAP §2 🆕 U, cut U-a (§4.5.591): an import leaves ONE current binding per key.
//!
//! A constant can live in one of several elaborate maps — `params` (≤64-bit, with its
//! declared range), `wide_param_bits` (>64-bit), a package-variable alias — and the
//! readers do not agree on which to ask first: the run-time name route, the bit domain
//! and a `localparam int` ask the >64-bit map first; the generate-if/-case condition
//! and a declared range ask `params` first (PRE with a stale narrow `P = 3` beside the
//! wide `P`: `xw small` from the generate-if, `xw A=9` from `localparam int A = P`).
//! Every import arm that unbound or replaced a wildcard's binding cleared only ITS
//! map, so a key could hold two declarations at once and each reader answered a
//! different one. U-a clears every map wherever the survivor is the wider binding or
//! none:
//!
//! - an AMBIGUOUS name (one name from two wildcard imports with no item of the scope
//!   between them, §26.3) is unbound in every map — loud at the use site, as iverilog
//!   and sv2v refuse it. An import has a position: with an item between, a reference
//!   there bound the first package's name, which every oracle answers. There those two
//!   imports keep their pre-U-a binding, which differs by scope: a MODULE (interface,
//!   instance) keeps the first package's binding, as the oracles do; a PACKAGE falls
//!   back to its pre-U-a per-import state, where the LATER package wins — right only
//!   where the two values agree (`pr A=3 B=5`, `pnw A=3 B=9`, all three oracles `B=3`;
//!   pinned KNOWN-WRONG below);
//! - an EXPLICIT import that wins a wildcard (§26.3), or that replaces a `$unit`
//!   explicit import (an outer scope), takes the loser's binding out of every map;
//! - a package body keeps ONE wildcard-origin / explicit-import state, as a module
//!   does, so the ambiguity of two ADJACENT wildcard imports is seen; across an item
//!   it binds as described in the first bullet (the later package wins);
//! - two §26.3 conflicts become loud: an explicit import of a name the scope declares
//!   as an enum LABEL, and two explicit imports of one VALUE-carrying name (a
//!   constant, a >64-bit constant, a variable) from two packages in one scope. A TYPE
//!   or FUNCTION name imported twice still takes the second import in silence
//!   (`import pa::t; import pb::t;` → `d19 b=8`, `import pa::f; import pb::f;` → `d20
//!   f=5`, where iverilog says "'t' has already been imported into this scope from
//!   package 'pa'." and sv2v "import of pb::t conflicts with prior import of pa::t").
//!
//! The other half (U-b: a genvar, an enum label or an explicit NARROW import that
//! replaces a >64-bit binding by a narrower one) is HELD behind §2 🆕 AE: the corrected
//! narrow value reaches the constant interpreter where the wide one was refused
//! (`Gw_cae`, `Iwn_kae`, `Ewi_cae` below must stay loud). Its witnesses are pinned
//! KNOWN-WRONG at today's lines, with every oracle's raw line beside them.
//!
//! Oracles, run on each design as written: iverilog 13.0 (`-g2012`, `vvp -n`), sv2v
//! 0.0.13 → iverilog 13.0, verilator 5.052 (`--binary --timing`). Verilator binds the
//! FIRST import of a name, explicit or wildcard: `import pb::P; import pa::*;` answers
//! the explicit `P` (as iverilog and sv2v do), `import pa::*; import pb::P;` the
//! wildcard's, and of two wildcards the first package's where iverilog and sv2v refuse
//! the name. §26.3 lets an explicit import win wherever it is written and makes two
//! wildcards ambiguous, so verilator is set aside on this import-precedence axis. That
//! rests on these measurements and on its self-contradiction on the regression guard
//! `p1_an_explicit_import_beside_a_wildcard_one` (`bits=32 hi=1`), not on a disqualifier
//! registered in ROADMAP §0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Run {
    out: String,
    err: String,
    code: i32,
}

impl Run {
    /// The `$display` lines, sorted, without the simulator's trailer.
    fn lines(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self
            .out
            .lines()
            .filter(|l| !l.starts_with("simulation ended"))
            .collect();
        v.sort_unstable();
        v
    }
}

fn run(src: &str) -> Run {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_iobk_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    Run {
        out: String::from_utf8_lossy(&out.stdout).into_owned(),
        err: String::from_utf8_lossy(&out.stderr).into_owned(),
        code: out.status.code().unwrap_or(-1),
    }
}

/// Exit 0 and exactly these `$display` lines (sorted).
fn check(src: &str, want: &[&str]) {
    let r = run(src);
    assert_eq!(r.code, 0, "stderr:\n{}\nsrc:\n{src}", r.err);
    let mut want = want.to_vec();
    want.sort_unstable();
    assert_eq!(r.lines(), want, "stdout:\n{}\nsrc:\n{src}", r.out);
}

/// Exit 1, `phrase` in the diagnostics, and no `$display` line ran.
fn loud(src: &str, phrase: &str) {
    let r = run(src);
    assert_eq!(
        r.code, 1,
        "stdout:\n{}\nstderr:\n{}\nsrc:\n{src}",
        r.out, r.err
    );
    assert!(
        r.err.contains(phrase),
        "expected {phrase:?} in:\n{}\nsrc:\n{src}",
        r.err
    );
    assert!(r.lines().is_empty(), "stdout:\n{}\nsrc:\n{src}", r.out);
    assert!(!r.err.contains("panicked"), "{}", r.err);
}

const PA_N_PB_W: &str = "package pa; localparam P = 3; endpackage\n\
     package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage\n";

const UNDECLARED_P: &str = "undeclared net/variable `top.P`";
const LOCAL_LABEL: &str = "has already been imported into this scope (explicit import from \
                           package `pk`) and is also declared locally";

// ── Ambiguity: one name from two wildcard imports, one of them >64-bit ─────────────

/// Narrow first, wide second. The wide arm marked the name ambiguous and removed only
/// the wide binding; the narrow `P = 3` stayed in `params` and answered (`aa P=3`).
/// - iverilog: `error: Ambiguous use of 'P'. It is exported by both 'pa' and by 'pb'.`
/// - sv2v: `identifier "P" ambiguously refers to the definitions in any of pa, pb`
/// - verilator: `aa P=3` (binds the first wildcard; disqualified, see the header).
#[test]
fn a_narrow_then_wide_wildcard_pair_is_ambiguous() {
    loud(
        &format!(
            "{PA_N_PB_W}module top;\n  import pa::*;\n  import pb::*;\n  \
             initial #1 $display(\"aa P=%0d\", P);\n  initial #100 $finish;\nendmodule\n"
        ),
        UNDECLARED_P,
    );
}

/// Wide first, narrow second. The narrow arm unbound only `params`; the wide entry
/// stayed and both the name route and the bit domain answered it (`sel s0=1 s64=1
/// lo=9`; the value twin `val P=18446744073709551625 K=9 …`).
/// - iverilog: `error: Ambiguous use of 'P'. It is exported by both 'pb' and by 'pa'.`
/// - sv2v: `identifier "P" ambiguously refers to the definitions in any of pa, pb`
/// - verilator: `sel s0=1 s64=1 lo=9`.
#[test]
fn a_wide_then_narrow_wildcard_pair_is_ambiguous() {
    loud(
        &format!(
            "{PA_N_PB_W}module top;\n  import pb::*;\n  import pa::*;\n  \
             initial #1 $display(\"sel s0=%b s64=%b lo=%0d\", P[0], P[64], P[3:0]);\n  \
             initial #100 $finish;\nendmodule\n"
        ),
        UNDECLARED_P,
    );
    loud(
        &format!(
            "{PA_N_PB_W}module top;\n  import pb::*;\n  import pa::*;\n  \
             localparam int K = P;\n  \
             initial #1 $display(\"val P=%0d K=%0d\", P, K);\n  initial #100 $finish;\nendmodule\n"
        ),
        "undefined name `P` is not a constant",
    );
}

/// A wildcard >64-bit constant and a wildcard VARIABLE of one name, in both orders. The
/// variable arm unbound `params` and the alias but left the wide constant (`w
/// W=18446744073709551625`); the wide arm left the variable alias (`w W=5`).
/// - iverilog: `error: Ambiguous use of 'W'. It is exported by both 'pa' and by 'pb'.`
///   (both orders)
/// - sv2v: `identifier "W" ambiguously refers to the definitions in any of pa, pb`
/// - verilator: `w W=18446744073709551625`, `w W=5` (the first wildcard).
#[test]
fn a_wildcard_variable_beside_a_wildcard_wide_constant_is_ambiguous() {
    let wide = "localparam [64:0] W = 65'h1_0000_0000_0000_0009;";
    let var = "logic [7:0] W = 8'd5;";
    for (a, b) in [(wide, var), (var, wide)] {
        loud(
            &format!(
                "package pa; {a} endpackage\npackage pb; {b} endpackage\nmodule top;\n  \
                 import pa::*;\n  import pb::*;\n  initial #1 $display(\"w W=%0d\", W);\n  \
                 initial #100 $finish;\nendmodule\n"
            ),
            "undeclared net/variable `top.W`",
        );
    }
}

/// The ambiguous name read by a constant function: PRE interpreted the stale narrow
/// `P` (`pae W=3`); it is ambiguous, so it is loud (a climb onto 🆕 AE's lane, not a
/// value).
/// - iverilog: `error: Ambiguous use of 'P'. It is exported by both 'pa' and by 'pb'.`
/// - sv2v: `identifier "P" ambiguously refers to the definitions in any of pa, pb`
/// - verilator: `pae W=x`.
#[test]
fn an_ambiguous_name_read_by_a_constant_function_is_loud() {
    loud(
        &format!(
            "{PA_N_PB_W}module top;\n  import pa::*;\n  import pb::*;\n  \
             function automatic integer fae(input integer x); logic [7:0] t; fae = x + t; \
             endfunction\n  localparam integer W = fae(P);\n  \
             initial #3 $display(\"pae W=%0d\", W);\n  initial #100 $finish;\nendmodule\n"
        ),
        "undefined name `P` is not a constant",
    );
}

// ── §26.3 position: an item between two wildcard imports ───────────────────────────

/// An import has a POSITION (§26.3): a reference written between two wildcard imports
/// binds the first package's name, and the second import no longer makes it ambiguous.
/// U-a's first cut cleared every map at import time, so all of these were loud
/// (vita applies a scope's imports before its items); they keep PRE's line, which
/// every oracle prints. Module body, narrow/wide and constant/variable in both orders,
/// and a third wildcard after the second.
/// - iverilog, sv2v → iverilog, verilator: `rp A=3 P=3`; `rw A=18446744073709551625
///   P=18446744073709551625`; `vw A=5 W=5`; `wv A=18446744073709551625
///   W=18446744073709551625`; `n08 A=3 P=3`.
#[test]
fn a_reference_between_two_wildcard_imports_keeps_the_first_binding() {
    let tail = "  initial #100 $finish;\nendmodule\n";
    check(
        &format!(
            "{PA_N_PB_W}module top;\n  import pa::*;\n  localparam int A = P;\n  import pb::*;\n  \
             initial #1 $display(\"rp A=%0d P=%0d\", A, P);\n{tail}"
        ),
        &["rp A=3 P=3"],
    );
    check(
        &format!(
            "{PA_N_PB_W}module top;\n  import pb::*;\n  localparam [64:0] A = P;\n  \
             import pa::*;\n  initial #1 $display(\"rw A=%0d P=%0d\", A, P);\n{tail}"
        ),
        &["rw A=18446744073709551625 P=18446744073709551625"],
    );
    let wide = "localparam [64:0] W = 65'h1_0000_0000_0000_0009;";
    let var = "logic [7:0] W = 8'd5;";
    check(
        &format!(
            "package pa; {var} endpackage\npackage pb; {wide} endpackage\nmodule top;\n  \
             import pa::*;\n  wire [7:0] A = W;\n  import pb::*;\n  \
             initial #1 $display(\"vw A=%0d W=%0d\", A, W);\n{tail}"
        ),
        &["vw A=5 W=5"],
    );
    check(
        &format!(
            "package pa; {wide} endpackage\npackage pb; {var} endpackage\nmodule top;\n  \
             import pa::*;\n  localparam [64:0] A = W;\n  import pb::*;\n  \
             initial #1 $display(\"wv A=%0d W=%0d\", A, W);\n{tail}"
        ),
        &["wv A=18446744073709551625 W=18446744073709551625"],
    );
    check(
        &format!(
            "{PA_N_PB_W}package pc; localparam [64:0] P = 65'h1_0000_0000_0000_000b; \
             endpackage\nmodule top;\n  import pa::*;\n  localparam int A = P;\n  \
             import pb::*;\n  import pc::*;\n  \
             initial #1 $display(\"n08 A=%0d P=%0d\", A, P);\n{tail}"
        ),
        &["n08 A=3 P=3"],
    );
}

/// The position rule through every binder that keeps an import list: a header
/// parameter or a port between a header import and a body import, an item or a
/// generate-if in a bare `generate` region (not a scope, §27.3), an interface body and
/// an instance array.
/// - iverilog, sv2v → iverilog, verilator: `hp X=3 P=3`; `hw big`, `hw
///   X=18446744073709551625 P=18446744073709551625`; `n06 P=3 b=3`; `n05 A=3 P=3`;
///   `gi g`, `gi P=3`; `inw top.x A=3 P=3`; `anw top.u[0] A=3 P=3`, `anw top.u[1]
///   A=3 P=3`.
#[test]
fn the_position_rule_holds_through_header_ports_generate_interface_and_array() {
    check(
        &format!(
            "{PA_N_PB_W}module top import pa::*; #(parameter int X = P) ();\n  import pb::*;\n  \
             initial #1 $display(\"hp X=%0d P=%0d\", X, P);\n  initial #100 $finish;\nendmodule\n"
        ),
        &["hp X=3 P=3"],
    );
    check(
        &format!(
            "{PA_N_PB_W}module top import pb::*; #(parameter [64:0] X = P) ();\n  \
             import pa::*;\n  \
             if (P > 65'd100) begin : big initial #1 $display(\"hw big\"); end\n  \
             else begin : sm initial #1 $display(\"hw small\"); end\n  \
             initial #1 $display(\"hw X=%0d P=%0d\", X, P);\n  initial #100 $finish;\nendmodule\n"
        ),
        &["hw X=18446744073709551625 P=18446744073709551625", "hw big"],
    );
    check(
        &format!(
            "{PA_N_PB_W}module m import pa::*; (input logic [P-1:0] a);\n  import pb::*;\n  \
             initial #1 $display(\"n06 P=%0d b=%0d\", P, $bits(a));\nendmodule\n\
             module top;\n  logic [2:0] x;\n  m u (.a(x));\n  initial #100 $finish;\nendmodule\n"
        ),
        &["n06 P=3 b=3"],
    );
    check(
        &format!(
            "{PA_N_PB_W}module top;\n  import pa::*;\n  \
             generate localparam int A = P; import pb::*; endgenerate\n  \
             initial #1 $display(\"n05 A=%0d P=%0d\", A, P);\n  initial #100 $finish;\nendmodule\n"
        ),
        &["n05 A=3 P=3"],
    );
    check(
        &format!(
            "{PA_N_PB_W}module top;\n  import pa::*;\n  \
             generate if (P > 1) begin : g initial #1 $display(\"gi g\"); end endgenerate\n  \
             import pb::*;\n  initial #1 $display(\"gi P=%0d\", P);\n  initial #100 $finish;\n\
             endmodule\n"
        ),
        &["gi P=3", "gi g"],
    );
    check(
        &format!(
            "{PA_N_PB_W}interface ifc;\n  import pa::*;\n  localparam int A = P;\n  \
             import pb::*;\n  initial #1 $display(\"inw %m A=%0d P=%0d\", A, P);\n\
             endinterface\nmodule top; ifc x (); initial #100 $finish; endmodule\n"
        ),
        &["inw top.x A=3 P=3"],
    );
    check(
        &format!(
            "{PA_N_PB_W}module c;\n  import pa::*;\n  localparam int A = P;\n  import pb::*;\n  \
             initial #1 $display(\"anw %m A=%0d P=%0d\", A, P);\nendmodule\n\
             module top; c u [1:0] (); initial #100 $finish; endmodule\n"
        ),
        &["anw top.u[0] A=3 P=3", "anw top.u[1] A=3 P=3"],
    );
}

/// A PACKAGE body with an item between two wildcard imports: PRE kept one state per
/// import there, so the second import bound the name as if alone; that is kept
/// exactly. Right where the readers reach the first binding (the wide `A` and `B`
/// below) or where both packages agree on the value.
/// - iverilog, sv2v → iverilog, verilator: `pwn A=18446744073709551625
///   B=18446744073709551625`; `n03 A=3 B=3`.
#[test]
fn a_package_item_between_two_wildcard_imports_keeps_the_per_import_binding() {
    check(
        &format!(
            "{PA_N_PB_W}package pc;\n  import pb::*;\n  localparam [64:0] A = P;\n  \
             import pa::*;\n  localparam [64:0] B = P;\nendpackage\nmodule top;\n  \
             initial #1 $display(\"pwn A=%0d B=%0d\", pc::A, pc::B);\n  initial #100 $finish;\n\
             endmodule\n"
        ),
        &["pwn A=18446744073709551625 B=18446744073709551625"],
    );
    check(
        "package pa; localparam P = 3; endpackage\npackage pb; localparam P = 3; endpackage\n\
         package pc;\n  import pa::*;\n  localparam int A = P;\n  import pb::*;\n  \
         localparam int B = P;\nendpackage\nmodule top;\n  \
         initial #1 $display(\"n03 A=%0d B=%0d\", pc::A, pc::B);\n  initial #100 $finish;\n\
         endmodule\n",
        &["n03 A=3 B=3"],
    );
}

/// KNOWN-WRONG (PRE kept, not U-a's): the same package shape where the second
/// package's value reaches the reader. PRE's per-import state bound the second `P`
/// over the first, and a later reference answers it; §26.3 binds `P` to the first
/// package at the earlier reference.
/// - iverilog, sv2v → iverilog, verilator: `pnw A=3 B=3`; `pr A=3 B=3`.
#[test]
fn known_wrong_a_package_reference_between_two_wildcard_imports() {
    let pc = |pb_p: &str, tag: &str| {
        format!(
            "package pa; localparam P = 3; endpackage\npackage pb; {pb_p} endpackage\n\
             package pc;\n  import pa::*;\n  localparam int A = P;\n  import pb::*;\n  \
             localparam int B = P;\nendpackage\nmodule top;\n  \
             initial #1 $display(\"{tag} A=%0d B=%0d\", pc::A, pc::B);\n  \
             initial #100 $finish;\nendmodule\n"
        )
    };
    check(
        &pc("localparam [64:0] P = 65'h1_0000_0000_0000_0009;", "pnw"),
        &["pnw A=3 B=9"],
    );
    check(&pc("localparam P = 5;", "pr"), &["pr A=3 B=5"]);
}

/// KNOWN-WRONG (PRE kept, not U-a's): the position rule asks whether ANY item of the
/// scope lies between the two imports, not whether a reference to the name does. An
/// unrelated item keeps PRE's binding, where the name is ambiguous.
/// - iverilog: `error: Ambiguous use of 'P'. It is exported by both 'pa' and by 'pb'.`
/// - sv2v: `identifier "P" ambiguously refers to the definitions in any of pa, pb`
/// - verilator: `n04 P=3` (the first import).
#[test]
fn known_wrong_an_unrelated_item_between_two_wildcard_imports() {
    check(
        &format!(
            "{PA_N_PB_W}module top;\n  import pa::*;\n  wire w;\n  import pb::*;\n  \
             initial #1 $display(\"n04 P=%0d\", P);\n  initial #100 $finish;\nendmodule\n"
        ),
        &["n04 P=3"],
    );
}

// ── An explicit import wins a wildcard in every map ────────────────────────────────

/// `import pa::*; import pb::P;` — the explicit >64-bit `P` wins (§26.3), but the
/// losing wildcard's `P = 3` stayed in `params`, and the integer fold read it: the
/// generate-if took `small`, the `[P[3:0]:0]` width was 4 bits.
/// - iverilog, sv2v → iverilog: `gif big`; `wid b=10`.
/// - verilator: `gif small`; `wid b=4` (binds the first import, the wildcard;
///   disqualified).
#[test]
fn an_explicit_wide_import_unbinds_the_losing_wildcard_narrow() {
    check(
        &format!(
            "{PA_N_PB_W}module top;\n  import pa::*;\n  import pb::P;\n  \
             if (P > 65'd100) begin : t initial #1 $display(\"gif big\"); end\n  \
             else begin : e initial #1 $display(\"gif small\"); end\n  \
             logic [P[3:0]:0] v;\n  initial #1 $display(\"wid b=%0d\", $bits(v));\n  \
             initial #100 $finish;\nendmodule\n"
        ),
        &["gif big", "wid b=10"],
    );
}

/// The same pair through three more binders: a `$unit` wildcard under a module
/// explicit import (X3), an interface body (IFnw) and an instance array (IAi).
/// - iverilog, sv2v → iverilog: `W=10000000000000007 bits_v=8 eq=1`;
///   `ifnw top.x P=18446744073709551625 bv=10 b=65`; `gif top.u[0].t big`,
///   `gif top.u[1].t big`.
/// - verilator: `W=10000000000000007 bits_v=8 eq=1`; `ifnw top.x P=3 bv=4 b=32`;
///   `gif top.u[0].e small`, `gif top.u[1].e small`.
#[test]
fn the_explicit_wide_import_wins_through_unit_interface_and_instance_array() {
    check(
        "package pa; localparam W = 5; endpackage\n\
         package pb; localparam [64:0] W = 65'h1_0000_0000_0000_0007; endpackage\n\
         import pa::*;\nmodule X3;\n  import pb::W;\n  wire [W[3:0]:0] v;\n  initial begin\n    \
         $display(\"W=%h bits_v=%0d eq=%b\", W, $bits(v), (W == 65'h1_0000_0000_0000_0007));\n    \
         #1 $finish;\n  end\nendmodule\n",
        &["W=10000000000000007 bits_v=8 eq=1"],
    );
    check(
        &format!(
            "{PA_N_PB_W}interface ifc;\n  import pa::*;\n  import pb::P;\n  \
             logic [P[3:0]:0] v;\n  \
             initial #1 $display(\"ifnw %m P=%0d bv=%0d b=%0d\", P, $bits(v), $bits(P));\n\
             endinterface\nmodule top;\n  ifc x ();\n  initial #100 $finish;\nendmodule\n"
        ),
        &["ifnw top.x P=18446744073709551625 bv=10 b=65"],
    );
    check(
        &format!(
            "{PA_N_PB_W}module c;\n  import pa::*;\n  import pb::P;\n  \
             if (P > 65'd100) begin : t initial #1 $display(\"gif %m big\"); end\n  \
             else begin : e initial #1 $display(\"gif %m small\"); end\nendmodule\n\
             module top;\n  c u [1:0] ();\n  initial #100 $finish;\nendmodule\n"
        ),
        &["gif top.u[0].t big", "gif top.u[1].t big"],
    );
}

/// A module's explicit >64-bit import replaces a `$unit` EXPLICIT narrow import of
/// the name (an outer scope): the narrow `P = 3` stayed in `params`, and the
/// generate-if and the declared range read it. Alone, and after two ambiguous `$unit`
/// wildcards (the explicit `pn::P` re-bound the name there).
/// - iverilog, sv2v → iverilog, verilator: `n01 big`, `n01 P=18446744073709551625
///   b=10`; `m13 big`, `m13 P=18446744073709551625 b=10`.
#[test]
fn an_explicit_wide_import_replaces_a_unit_explicit_narrow_one() {
    let body = |tag: &str| {
        format!(
            "module top;\n  import pw::P;\n  \
             if (P > 65'd100) begin : big initial #1 $display(\"{tag} big\"); end\n  \
             else begin : sm initial #1 $display(\"{tag} small\"); end\n  \
             logic [P[3:0]:0] v;\n  initial #1 $display(\"{tag} P=%0d b=%0d\", P, $bits(v));\n  \
             initial #100 $finish;\nendmodule\n"
        )
    };
    let pw = "package pw; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage\n";
    check(
        &format!(
            "package pn; localparam P = 3; endpackage\n{pw}import pn::P;\n{}",
            body("n01")
        ),
        &["n01 P=18446744073709551625 b=10", "n01 big"],
    );
    check(
        &format!(
            "package pa; localparam P = 1; endpackage\npackage pb; localparam P = 2; endpackage\n\
             package pn; localparam P = 3; endpackage\n{pw}\
             import pa::*;\nimport pb::*;\nimport pn::P;\n{}",
            body("m13")
        ),
        &["m13 P=18446744073709551625 b=10", "m13 big"],
    );
}

/// q1g split. Without a generate-case the explicit wide `P` now answers the
/// generate-if too (`@ big`, which PRE dropped). With the `case (P)`, the case is
/// loud: a >64-bit generate-case scrutinee is refused by design, exactly as its plain
/// twin `import pb::P; case (P) …` (no wildcard) already was — PRE chose arm `h3` off
/// the losing wildcard's `P = 3`.
/// - iverilog, sv2v → iverilog: `@ P=18446744073709551625 Q=18446744073709551626
///   w=18446744073709551625 sh=16 N=5 b=65`, `@ big` (and `@ hitw` with the case).
/// - verilator: `@ P=3 Q=4 w=3 sh=0 N=5 b=32` (disqualified).
#[test]
fn a_generate_if_reads_the_explicit_wide_import_and_a_generate_case_stays_loud() {
    let head = "package pa; localparam P = 3; localparam N = 5; endpackage\n\
         package pb; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage\n\
         module top;\n  import pa::*;\n  import pb::P;\n  localparam [64:0] Q = P + 65'd1;\n  \
         wire [64:0] w = P;\n  \
         initial #1 $display(\"@ P=%0d Q=%0d w=%0d sh=%0d N=%0d b=%0d\", P, Q, w, P >> 60, N, \
         $bits(P));\n";
    let tail = "  if (P > 65'd100) begin : big initial #2 $display(\"@ big\"); end\n  \
                initial #100 $finish;\nendmodule\n";
    check(
        &format!("{head}{tail}"),
        &[
            "@ P=18446744073709551625 Q=18446744073709551626 w=18446744073709551625 sh=16 N=5 b=65",
            "@ big",
        ],
    );
    loud(
        &format!(
            "{head}  case (P)\n    \
             65'h1_0000_0000_0000_0009: begin : hw initial #2 $display(\"@ hitw\"); end\n    \
             3: begin : h3 initial #2 $display(\"@ hit3\"); end\n    \
             default: begin : dd initial #2 $display(\"@ dflt\"); end\n  endcase\n{tail}"
        ),
        "generate-case scrutinee is not a constant",
    );
}

/// The explicit wide import read by a constant function: PRE interpreted the losing
/// wildcard's `P = 3` (`pae W=3`); it now refuses the 65-bit argument, the decline the
/// plain twin already gives (§2 🆕 AE's lane: a climb, never a value).
/// - iverilog, sv2v → iverilog, verilator: `pae W=x`.
#[test]
fn the_explicit_wide_import_read_by_a_constant_function_is_loud() {
    loud(
        &format!(
            "{PA_N_PB_W}module top;\n  import pa::*;\n  import pb::P;\n  \
             function automatic integer fae(input integer x); logic [7:0] t; fae = x + t; \
             endfunction\n  localparam integer W = fae(P);\n  \
             initial #3 $display(\"pae W=%0d\", W);\n  initial #100 $finish;\nendmodule\n"
        ),
        "`fae(…)` has no constant-fold arm",
    );
}

/// An explicit VARIABLE import over a wildcard >64-bit constant: the variable arm
/// unbound `params` and the alias but left the wide constant, which the name route
/// answered (`ev W=18446744073709551625 b=8`).
/// - iverilog, sv2v → iverilog: `ev W=5 b=8`.
/// - verilator: `ev W=18446744073709551625 b=65` (disqualified).
#[test]
fn an_explicit_variable_import_unbinds_the_losing_wildcard_wide() {
    check(
        "package pa; localparam [64:0] W = 65'h1_0000_0000_0000_0009; endpackage\n\
         package pb; logic [7:0] W = 8'd5; endpackage\n\
         module top;\n  import pa::*;\n  import pb::W;\n  \
         initial #1 $display(\"ev W=%0d b=%0d\", W, $bits(W));\n  initial #100 $finish;\n\
         endmodule\n",
        &["ev W=5 b=8"],
    );
}

// ── A package body is one import scope ─────────────────────────────────────────────

/// Two wildcard imports in a PACKAGE body, each with its own state: neither saw the
/// other, and the second package's `P` answered (`pp Z=5`, `pp
/// Z=18446744073709551625`). One state per package body sees the ambiguity. A name
/// only one package offers, a qualified reference and an explicit import keep their
/// values.
/// - iverilog: `error: Ambiguous use of 'P'. It is exported by both 'pa' and by 'pb'.`
///   (both); `ppnu Z=3`; `pp Z=18446744073709551628`; `ppen Z=5`.
/// - sv2v: `identifier "P" ambiguously refers to the definitions in any of pa, pb`
///   (both); `ppnu Z=3`; `pp Z=18446744073709551628`; `ppen Z=5`.
/// - verilator: `pp Z=3`, `pp Z=3`; `ppnu Z=3`; `pp Z=18446744073709551628`;
///   `ppen Z=3` (disqualified).
#[test]
fn a_package_body_sees_an_ambiguity_between_its_wildcard_imports() {
    let ref_z = |pb_p: &str, z: &str| {
        format!(
            "package pa; localparam P = 3; endpackage\npackage pb; {pb_p} endpackage\n\
             package pc;\n  import pa::*;\n  import pb::*;\n  {z}\nendpackage\n\
             module top;\n  initial #1 $display(\"pp Z=%0d\", pc::Z);\n  initial #100 $finish;\n\
             endmodule\n"
        )
    };
    let unfoldable = "package parameter `Z` value is not a foldable constant";
    loud(
        &ref_z("localparam P = 5;", "localparam int Z = P;"),
        unfoldable,
    );
    loud(
        &ref_z(
            "localparam [64:0] P = 65'h1_0000_0000_0000_0009;",
            "localparam [64:0] Z = P;",
        ),
        unfoldable,
    );
    check(
        "package pa; localparam P = 3; localparam A = 1; endpackage\n\
         package pb; localparam P = 5; localparam B = 2; endpackage\n\
         package pc;\n  import pa::*;\n  import pb::*;\n  localparam int Z = A + B;\nendpackage\n\
         module top;\n  initial #1 $display(\"ppnu Z=%0d\", pc::Z);\n  initial #100 $finish;\n\
         endmodule\n",
        &["ppnu Z=3"],
    );
    check(
        &ref_z(
            "localparam [64:0] P = 65'h1_0000_0000_0000_0009;",
            "localparam [64:0] Z = pa::P + pb::P;",
        ),
        &["pp Z=18446744073709551628"],
    );
    check(
        "package pa; localparam P = 3; endpackage\npackage pb; localparam P = 5; endpackage\n\
         package pc;\n  import pa::*;\n  import pb::P;\n  localparam int Z = P;\nendpackage\n\
         module top;\n  initial #1 $display(\"ppen Z=%0d\", pc::Z);\n  initial #100 $finish;\n\
         endmodule\n",
        &["ppen Z=5"],
    );
}

// ── §26.3: an explicit import that collides ────────────────────────────────────────

/// An explicit import of a name the scope declares as an enum LABEL (§6.19 declares a
/// label in the scope holding its typedef): PRE bound both, and the readers answered
/// the import (`E1=18446744073709551616 K=0` for a 65-bit `E1`) or the label (`ewn
/// E1=1 K=1`, `pel Z=1`), at exit 0. Module body, a `generate … endgenerate` region
/// (not a scope, §27.2) and a package body.
/// - iverilog: `error: 'E1' has already been imported into this scope from package
///   'pk'.` (all four)
/// - sv2v: `declaration of E1 conflicts with prior import of pk::E1` (all four)
/// - verilator: the label (`E1=1 K=1`, `ewn E1=1 K=1`, `egrx E1=1 K=1`, `pel Z=1`).
#[test]
fn an_explicit_import_of_a_local_enum_label_is_loud() {
    loud(
        "package pk; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage\n\
         module top;\n  import pk::E1;\n  typedef enum {E0, E1} e_t;\n  localparam int K = E1;\n  \
         initial #1 $display(\"val E1=%0d K=%0d\", E1, K);\n  initial #100 $finish;\nendmodule\n",
        LOCAL_LABEL,
    );
    loud(
        "package pk; localparam E1 = 7; endpackage\n\
         module top;\n  import pk::E1;\n  typedef enum {E0, E1} e_t;\n  localparam int K = E1;\n  \
         initial #1 $display(\"ewn E1=%0d K=%0d\", E1, K);\n  initial #100 $finish;\nendmodule\n",
        LOCAL_LABEL,
    );
    loud(
        "package pk; localparam E1 = 7; endpackage\n\
         module top;\n  import pk::E1;\n  generate typedef enum {E0, E1} e_t; endgenerate\n  \
         localparam int K = E1;\n  initial #1 $display(\"egrx E1=%0d K=%0d\", E1, K);\n  \
         initial #100 $finish;\nendmodule\n",
        LOCAL_LABEL,
    );
    loud(
        "package pk; localparam E1 = 7; endpackage\n\
         package pc;\n  import pk::E1;\n  typedef enum {E0, E1} e_t;\n  localparam int Z = E1;\n\
         endpackage\nmodule top;\n  initial #1 $display(\"pel Z=%0d\", pc::Z);\n  \
         initial #100 $finish;\nendmodule\n",
        LOCAL_LABEL,
    );
}

/// The label rule's edges, each kept on its route: a WILDCARD import loses to the
/// local label (§26.3, already right); a `$unit` label under a module explicit import
/// is an outer scope the import shadows; and a typedef inside an UNLABELLED
/// `begin … end` of a generate region is an oracle split, left as it was.
/// - iverilog, sv2v → iverilog, verilator: `lwc E1=1 K=1`; `ecul E1=7 K=7`.
/// - `begin … end`: iverilog `error: 'E1' has already been imported into this scope
///   from package 'pk'.` (+ `warning: Anachronistic use of begin/end to surround
///   generate schemes.`), sv2v → iverilog `egbx E1=7 K=7`, verilator `egbx E1=1 K=1`.
#[test]
fn the_label_rule_keeps_its_edges() {
    check(
        "package pk; localparam E1 = 7; endpackage\n\
         module top;\n  import pk::*;\n  typedef enum {E0, E1} e_t;\n  localparam int K = E1;\n  \
         initial #1 $display(\"lwc E1=%0d K=%0d\", E1, K);\n  initial #100 $finish;\nendmodule\n",
        &["lwc E1=1 K=1"],
    );
    check(
        "package pk; localparam E1 = 7; endpackage\ntypedef enum {E0, E1} e_t;\n\
         module top;\n  import pk::E1;\n  localparam int K = E1;\n  \
         initial #1 $display(\"ecul E1=%0d K=%0d\", E1, K);\n  initial #100 $finish;\nendmodule\n",
        &["ecul E1=7 K=7"],
    );
    check(
        "package pk; localparam E1 = 7; endpackage\n\
         module top;\n  import pk::E1;\n  generate begin typedef enum {E0, E1} e_t; end endgenerate\n  \
         localparam int K = E1;\n  initial #1 $display(\"egbx E1=%0d K=%0d\", E1, K);\n  \
         initial #100 $finish;\nendmodule\n",
        &["egbx E1=7 K=7"],
    );
}

/// KNOWN-WRONG, not §2 🆕 U (a parser residue): an explicit import written in a
/// `generate … endgenerate` region beside a `$unit` `typedef enum` label of the same
/// name. The parser copies the `$unit` typedef into the module unless a module item
/// shadows a label, and its shadow scan does not look inside a generate region, so the
/// label is copied in and answers. The label is an OUTER-scope declaration, never the
/// §26.3 conflict: the explicit-import check leaves it out (a refusal here would say
/// "declared locally" of a design all three tools run), so this keeps today's value.
/// - iverilog, sv2v → iverilog, verilator: `eclg E1=7 K=7`.
#[test]
fn known_wrong_a_unit_label_beside_a_generate_region_import() {
    check(
        "package pk; localparam E1 = 7; endpackage\ntypedef enum {E0, E1} e_t;\n\
         module top;\n  generate import pk::E1; endgenerate\n  localparam int K = E1;\n  \
         initial #1 $display(\"eclg E1=%0d K=%0d\", E1, K);\n  initial #100 $finish;\nendmodule\n",
        &["eclg E1=1 K=1"],
    );
}

/// Two explicit imports of one name from two packages in ONE scope: PRE bound both —
/// the narrow into `params`, the wide into `wide_param_bits` — and each reader picked
/// one (`ee P=18446744073709551625 b=65`, `ee P=18446744073709551625 b=32`, `eeg
/// small`; interface `ifee P=18446744073709551625 b=65`; package `peenw
/// Z=18446744073709551625`).
/// - iverilog: `error: 'P' has already been imported into this scope from package
///   'pa'.` (`'pb'` for the reversed pair)
/// - sv2v: `import of pb::P conflicts with prior import of pa::P` (reversed: `import
///   of pa::P conflicts with prior import of pb::P`)
/// - verilator: `ee P=3 b=32`, `ee P=18446744073709551625 b=65`, `eeg small`, `ifee
///   P=3 b=32`, `peenw Z=3`.
#[test]
fn two_explicit_imports_of_one_name_from_two_packages_are_loud() {
    let body = "  initial #1 $display(\"ee P=%0d b=%0d\", P, $bits(P));\n  initial #100 $finish;\n";
    loud(
        &format!("{PA_N_PB_W}module top;\n  import pa::P;\n  import pb::P;\n{body}endmodule\n"),
        "`P` has already been imported into this scope from package `pa` (explicit import \
         from package `pb`)",
    );
    loud(
        &format!("{PA_N_PB_W}module top;\n  import pb::P;\n  import pa::P;\n{body}endmodule\n"),
        "`P` has already been imported into this scope from package `pb` (explicit import \
         from package `pa`)",
    );
    loud(
        &format!(
            "{PA_N_PB_W}module top;\n  import pa::P;\n  import pb::P;\n  \
             if (P > 65'd100) begin : big initial #1 $display(\"eeg big\"); end else begin : sm \
             initial #1 $display(\"eeg small\"); end\n  initial #100 $finish;\nendmodule\n"
        ),
        "from package `pa` (explicit import from package `pb`)",
    );
    loud(
        &format!(
            "{PA_N_PB_W}interface ifc;\n  import pa::P;\n  import pb::P;\n  int k;\n  \
             initial begin k = P; #1 $display(\"ifee P=%0d b=%0d\", P, $bits(P)); end\n\
             endinterface\nmodule top;\n  ifc i();\n  initial #100 $finish;\nendmodule\n"
        ),
        "from package `pa` (explicit import from package `pb`)",
    );
    loud(
        &format!(
            "{PA_N_PB_W}package pc;\n  import pa::P;\n  import pb::P;\n  \
             localparam [64:0] Z = P;\nendpackage\nmodule top;\n  \
             initial #1 $display(\"peenw Z=%0d\", pc::Z);\n  initial #100 $finish;\nendmodule\n"
        ),
        "from package `pa` (explicit import from package `pb`)",
    );
}

/// The explicit-pair rule's edges: the same package twice is one import (all three
/// tools run it), and a `$unit` explicit import is an OUTER scope that a module's own
/// explicit import of the name replaces, not a collision.
/// - iverilog, sv2v → iverilog, verilator: `ees P=3`; `cuenw P=18446744073709551625
///   K=9 b=65`.
#[test]
fn the_explicit_pair_rule_keeps_its_edges() {
    check(
        "package pa; localparam P = 3; endpackage\n\
         module top;\n  import pa::P;\n  import pa::P;\n  initial #1 $display(\"ees P=%0d\", P);\n  \
         initial #100 $finish;\nendmodule\n",
        &["ees P=3"],
    );
    check(
        &format!(
            "{PA_N_PB_W}import pa::P;\nmodule top;\n  import pb::P;\n  localparam int K = P;\n  \
             initial #1 $display(\"cuenw P=%0d K=%0d b=%0d\", P, K, $bits(P));\n  \
             initial #100 $finish;\nendmodule\n"
        ),
        &["cuenw P=18446744073709551625 K=9 b=65"],
    );
}

// ── HELD: U-b, the wide→narrow half (§2 🆕 AE) ─────────────────────────────────────

/// KNOWN-WRONG (held, U-b). A local enum label over a wildcard >64-bit constant: the
/// label rebinds `params` and leaves the wide entry, which the name route and the bit
/// domain answer first.
/// - iverilog, sv2v → iverilog, verilator: `B2D E1=1 K=1`; `val E1=1 K=1 KW=1 wv=1
///   b=32`, `pc one`; generate-region typedef `egr E1=1 K=1`.
#[test]
fn held_a_label_over_a_wildcard_wide_constant() {
    let pk = "package pk; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage\n";
    check(
        &format!(
            "{pk}module top;\n  import pk::*;\n  typedef enum {{E0, E1}} e_t;\n  \
             localparam int K = E1;\n  initial $display(\"B2D E1=%0d K=%0d\", E1, K);\n  \
             initial #100 $finish;\nendmodule\n"
        ),
        &["B2D E1=18446744073709551616 K=0"],
    );
    check(
        &format!(
            "{pk}module top;\n  import pk::*;\n  typedef enum {{E0, E1}} e_t;\n  \
             localparam int K = E1;\n  localparam [64:0] KW = E1;\n  wire [64:0] wv = E1;\n  \
             initial begin\n    \
             #1 $display(\"val E1=%0d K=%0d KW=%0d wv=%0d b=%0d\", E1, K, KW, wv, $bits(E1));\n    \
             case (E1) 0: $display(\"pc zero\"); 1: $display(\"pc one\"); default: \
             $display(\"pc def\"); endcase\n  end\n  initial #100 $finish;\nendmodule\n"
        ),
        &[
            "pc def",
            "val E1=18446744073709551616 K=0 KW=18446744073709551616 wv=18446744073709551616 b=32",
        ],
    );
    check(
        &format!(
            "{pk}module top;\n  import pk::*;\n  generate\n    typedef enum {{E0, E1}} e_t;\n  \
             endgenerate\n  localparam int K = E1;\n  \
             initial #1 $display(\"egr E1=%0d K=%0d\", E1, K);\n  initial #100 $finish;\n\
             endmodule\n"
        ),
        &["egr E1=18446744073709551616 K=0"],
    );
}

/// KNOWN-WRONG (held, U-b). An explicit NARROW import over a wildcard (or `$unit`)
/// >64-bit one: the narrow wins `params`, the wide entry stays and is read first.
/// - iverilog, sv2v → iverilog: `val P=3 K=3 KW=4 wv=3 b=32 sh=0`; `cuewn P=3 K=3
///   b=32`.
/// - verilator: `val P=18446744073709551625 K=9 KW=18446744073709551626
///   wv=18446744073709551625 b=65 sh=16` (disqualified); `cuewn P=3 K=3 b=32`.
#[test]
fn held_an_explicit_narrow_import_over_a_wide_one() {
    check(
        "package pa; localparam [64:0] P = 65'h1_0000_0000_0000_0009; endpackage\n\
         package pb; localparam P = 3; endpackage\n\
         module top;\n  import pa::*;\n  import pb::P;\n  localparam int K = P;\n  \
         localparam [64:0] KW = P + 65'd1;\n  wire [64:0] wv = P;\n  \
         initial #1 $display(\"val P=%0d K=%0d KW=%0d wv=%0d b=%0d sh=%0d\", P, K, KW, wv, \
         $bits(P), P >> 60);\n  initial #100 $finish;\nendmodule\n",
        &["val P=18446744073709551625 K=9 KW=18446744073709551626 wv=18446744073709551625 b=32 sh=16"],
    );
    check(
        &format!(
            "{PA_N_PB_W}import pb::P;\nmodule top;\n  import pa::P;\n  localparam [64:0] K = P;\n  \
             initial #1 $display(\"cuewn P=%0d K=%0d b=%0d\", P, K, $bits(P));\n  \
             initial #100 $finish;\nendmodule\n"
        ),
        &["cuewn P=18446744073709551625 K=18446744073709551625 b=32"],
    );
}

/// MUST STAY LOUD — the guard any U-b attempt has to keep (§2 🆕 AE). The corrected
/// narrow value would reach the constant interpreter, which answers a never-assigned
/// 4-state local as 0 (`W=0`/`W=1`/`W=3`), where every oracle prints `x`; today the
/// interpreter refuses the 65-bit argument the stale wide binding hands it. Through a
/// genvar override, a localparam alias of an explicit narrow import, and a label
/// override.
/// - iverilog, sv2v → iverilog, verilator: `ae top.g[0].u6 W=x`, `ae top.g[1].u6
///   W=x`; `ae top.u6 W=x`; `ae top.u6 W=x`.
#[test]
fn must_stay_loud_the_constant_interpreter_never_sees_the_narrowed_value() {
    let sae = "module sae #(parameter N = 0) ();\n  \
               function automatic integer fae(input integer x); logic [7:0] t; fae = x + t; \
               endfunction\n  localparam integer W = fae(N);\n  \
               initial #3 $display(\"ae %m W=%0d\", W);\nendmodule\n";
    let no_arm = "`fae(…)` has no constant-fold arm";
    loud(
        &format!(
            "module top;\n  localparam [64:0] i = 65'h1_0000_0000_0000_0009;\n  \
             for (genvar i = 0; i < 2; i++) begin : g\n    sae #(.N(i)) u6 ();\n  end\n  \
             initial #100 $finish;\nendmodule\n{sae}"
        ),
        no_arm,
    );
    loud(
        &format!(
            "{PA_N_PB_W}module top;\n  import pb::*;\n  import pa::P;\n  \
             localparam [64:0] KW = P;\n  sae #(.N(KW)) u6 ();\n  initial #100 $finish;\n\
             endmodule\n{sae}"
        ),
        no_arm,
    );
    loud(
        &format!(
            "package pk; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage\n\
             module top;\n  import pk::*;\n  typedef enum {{E0, E1}} e_t;\n  \
             sae #(.N(E1)) u6 ();\n  initial #100 $finish;\nendmodule\n{sae}"
        ),
        no_arm,
    );
}
