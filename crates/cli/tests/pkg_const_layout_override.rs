//! §2 "Index sealing" — an ASCENDING (`[0:35]`) or NON-ZERO-LSB (`[39:4]`) constant used
//! as a parameter OVERRIDE binds its DECLARED width, in both lanes.
//!
//! `narrow_param_bits` / `pkg_const_narrow_bits` (the module and package resolvers of a
//! narrow constant's bits) decline such a declaration, because the wide bit domain
//! indexes positionally from 0 and carries no direction — a real hazard for a consumer
//! that reads BIT POSITIONS (§4.5.363). An override binding reads no bit position: it
//! takes a WIDTH and a SIGN and coerces an already-folded value to them, and
//! `param_decl_range_opt` records the width as `|msb − lsb| + 1`, which is
//! direction- and offset-independent. The decline therefore threw away a declared width,
//! and the leaf fell back to the value-inferred 32 — which TRUNCATED any value needing
//! more than 32 bits. See `crates/elaborate/src/const_decl_width.rs`.
//!
//! Oracles: every expectation below was measured 3-way (vita / iverilog 13 `-g2012` +
//! `vvp` / verilator 5.052 `--binary --timing`) and both oracles agree on every pinned
//! cell. The cells that are an oracle SPLIT, and the residues this slice deliberately
//! leaves at their pre-slice answer, say so in place and are not pinned.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pclo_{}_{n}", std::process::id()));
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
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

fn lines(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter(|l| !l.starts_with("simulation ended") && !l.contains("VITA-W1017"))
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with("errors="))
        .collect()
}

/// `parameter P = 8` — the UNTYPED, UNRANGED leaf whose width §6.20.2 takes from the
/// final override value. That is the only target shape this row is about; a declared
/// type or range on the leaf wins over the override and was already correct.
const LEAF: &str =
    "module c #(parameter TAG=\"?\", parameter P = 8)();\n  initial $display(\"%s bits=%0d val=%h\", TAG, $bits(P), P);\nendmodule\n";

/// A package `pk`, then `LEAF`, then the body of `module top`.
fn cell(pkg: &str, body: &str) -> String {
    format!("package pk;\n{pkg}endpackage\n{LEAF}module top;\n  import pk::*;\n{body}  initial #1 $finish;\nendmodule\n")
}

fn check(pkg: &str, body: &str, want: &[&str]) {
    let (o, c) = run(&cell(pkg, body));
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(lines(&o), want, "{o}");
}

/// The row's own cell, both spellings and both non-`[w-1:0]` layouts. PRE every one of
/// these printed `bits=32 val=00000abc` / `00000def`; both oracles print 36.
#[test]
fn a_ascending_and_offset_package_constants_bind_their_declared_width() {
    check(
        "  localparam logic [0:35] PA = 36'hABC;\n  localparam logic [39:4] PB = 36'hDEF;\n  localparam logic [35:0] PN = 36'h123;\n",
        "  c #(.TAG(\"A_scoped\"), .P(pk::PA)) a();\n  c #(.TAG(\"A_import\"), .P(PA)) b();\n  c #(.TAG(\"B_scoped\"), .P(pk::PB)) d();\n  c #(.TAG(\"B_import\"), .P(PB)) e();\n  c #(.TAG(\"N_scoped\"), .P(pk::PN)) f();\n",
        &[
            "A_scoped bits=36 val=000000abc",
            "A_import bits=36 val=000000abc",
            "B_scoped bits=36 val=000000def",
            "B_import bits=36 val=000000def",
            // CONTROL — `[35:0]`, correct PRE and byte-identical POST.
            "N_scoped bits=36 val=000000123",
        ],
    );
}

/// The declaration-shape axis: ascending, offset, both-at-once, signed, `bit`, and a
/// package `parameter` beside a package `localparam`. PRE the six non-`[w-1:0]` shapes
/// printed 32; the `int` / `integer` / `[35:0]` / `[0:0]` controls printed what they
/// print now. Both oracles: 36 / 36 / 36 / 36 / 36 / 36 / 32 / 32 / 36 / 1.
#[test]
fn b_every_declared_layout_binds_its_own_width() {
    check(
        "  localparam logic [35:0]        A_desc  = 36'hABC;\n\
         \x20 localparam logic [0:35]        A_asc   = 36'hABC;\n\
         \x20 localparam logic [39:4]        A_lo4   = 36'hABC;\n\
         \x20 localparam logic [4:39]        A_asc4  = 36'hABC;\n\
         \x20 localparam logic [0:0]         A_b00   = 1'b1;\n\
         \x20 localparam logic signed [0:35] A_sasc  = 36'hABC;\n\
         \x20 localparam bit [0:35]          A_basc  = 36'hABC;\n\
         \x20 localparam int                 A_int   = 36'hABC;\n\
         \x20 localparam integer             A_integ = 36'hABC;\n\
         \x20 parameter  logic [0:35]        A_pasc  = 36'hABC;\n\
         \x20 parameter  logic [39:4]        A_plo4  = 36'hABC;\n",
        "  c #(.TAG(\"desc \"), .P(pk::A_desc )) i1();\n\
         \x20 c #(.TAG(\"asc  \"), .P(pk::A_asc  )) i2();\n\
         \x20 c #(.TAG(\"lo4  \"), .P(pk::A_lo4  )) i3();\n\
         \x20 c #(.TAG(\"asc4 \"), .P(pk::A_asc4 )) i4();\n\
         \x20 c #(.TAG(\"b00  \"), .P(pk::A_b00  )) i5();\n\
         \x20 c #(.TAG(\"sasc \"), .P(pk::A_sasc )) i7();\n\
         \x20 c #(.TAG(\"basc \"), .P(pk::A_basc )) i8();\n\
         \x20 c #(.TAG(\"int  \"), .P(pk::A_int  )) i9();\n\
         \x20 c #(.TAG(\"integ\"), .P(pk::A_integ)) i10();\n\
         \x20 c #(.TAG(\"pasc \"), .P(pk::A_pasc )) i11();\n\
         \x20 c #(.TAG(\"plo4 \"), .P(pk::A_plo4 )) i12();\n",
        &[
            "desc  bits=36 val=000000abc", // CONTROL
            "asc   bits=36 val=000000abc",
            "lo4   bits=36 val=000000abc",
            "asc4  bits=36 val=000000abc",
            "b00   bits=1 val=1", // CONTROL — `[0:0]` is lo 0 and descending-equivalent
            "sasc  bits=36 val=000000abc",
            "basc  bits=36 val=000000abc",
            "int   bits=32 val=00000abc", // CONTROL — a declared TYPE, 32 in all 3 tools
            "integ bits=32 val=00000abc", // CONTROL
            "pasc  bits=36 val=000000abc",
            "plo4  bits=36 val=000000abc",
        ],
    );
}

/// A ONE-BIT declaration whose LSB is not 0: `logic [7:7]`. The width is 1 whatever the
/// offset, and PRE this printed `bits=32 val=00000001` — the offset alone, with no width
/// disagreement anywhere, was enough to lose it. Both oracles: `bits=1 val=1`.
#[test]
fn c_one_bit_at_a_non_zero_offset_stays_one_bit() {
    check(
        "  localparam logic [7:7] A_b77 = 1'b1;\n  localparam logic [0:0] A_b00 = 1'b1;\n",
        "  c #(.TAG(\"b77\"), .P(pk::A_b77)) i1();\n  c #(.TAG(\"b00\"), .P(pk::A_b00)) i2();\n",
        &[
            "b77 bits=1 val=1",
            "b00 bits=1 val=1", // CONTROL
        ],
    );
}

/// ⭐ THE VALUE, not only `$bits`. A 36-bit constant that does not fit 32 bits had its
/// top four bits DELETED and its sign flipped by the value-inferred width:
/// PRE `bits=32 val=edcba987 dec=-305419897`, both oracles
/// `bits=36 val=fedcba987 dec=68414056839`. The signed twin kept its value and lost its
/// width (PRE `bits=32 val=fffffffb`).
#[test]
fn d_a_value_wider_than_32_bits_is_no_longer_truncated() {
    let src = "package pk;\n\
        \x20 localparam logic [0:35]        BIG  = 36'hFEDCBA987;\n\
        \x20 localparam logic [35:0]        BIGN = 36'hFEDCBA987;\n\
        \x20 localparam logic signed [0:35] SNEG = -36'sd5;\n\
        endpackage\n\
        module c #(parameter TAG=\"?\", parameter P = 8)();\n\
        \x20 initial $display(\"%s bits=%0d val=%h dec=%0d\", TAG, $bits(P), P, P);\n\
        endmodule\n\
        module top;\n\
        \x20 c #(.TAG(\"BIG_asc \"), .P(pk::BIG )) a();\n\
        \x20 c #(.TAG(\"BIG_desc\"), .P(pk::BIGN)) b();\n\
        \x20 c #(.TAG(\"SNEG    \"), .P(pk::SNEG)) d();\n\
        \x20 initial #1 $finish;\n\
        endmodule\n";
    let (o, c) = run(src);
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "BIG_asc  bits=36 val=fedcba987 dec=68414056839",
            // CONTROL — the `[35:0]` twin of the same value, correct PRE and POST.
            "BIG_desc bits=36 val=fedcba987 dec=68414056839",
            "SNEG     bits=36 val=ffffffffb dec=-5",
        ],
        "{o}"
    );
}

/// The MODULE lane: a parent-scope `parameter` / `localparam` with the same declarations.
/// Cell-for-cell identical to the package lane PRE (all 32) and POST (all 36) — the two
/// resolvers are twins called from the same consumer arms, which is why one slice closes
/// both spellings.
#[test]
fn e_module_lane_twin_binds_the_declared_width() {
    let src = "module c #(parameter TAG=\"?\", parameter P = 8)();\n\
        \x20 initial $display(\"%s bits=%0d val=%h\", TAG, $bits(P), P);\n\
        endmodule\n\
        module top #(parameter logic [0:35] MA = 36'hABC,\n\
        \x20            parameter logic [39:4] MB = 36'hDEF,\n\
        \x20            parameter logic [35:0] MN = 36'h123)();\n\
        \x20 localparam logic [0:35] LA = 36'hABC;\n\
        \x20 localparam logic [39:4] LB = 36'hDEF;\n\
        \x20 c #(.TAG(\"mod_MA\"), .P(MA)) a();\n\
        \x20 c #(.TAG(\"mod_MB\"), .P(MB)) b();\n\
        \x20 c #(.TAG(\"mod_MN\"), .P(MN)) d();\n\
        \x20 c #(.TAG(\"mod_LA\"), .P(LA)) e();\n\
        \x20 c #(.TAG(\"mod_LB\"), .P(LB)) f();\n\
        \x20 initial #1 $finish;\n\
        endmodule\n";
    let (o, c) = run(src);
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "mod_MA bits=36 val=000000abc",
            "mod_MB bits=36 val=000000def",
            "mod_MN bits=36 val=000000123", // CONTROL
            "mod_LA bits=36 val=000000abc",
            "mod_LB bits=36 val=000000def",
        ],
        "{o}"
    );
}

/// A SIZE CAST of such a name, `40'(pk::PA)`. The width is the cast's size and the sign
/// is the operand's — the rule the wide channel already applies to the `[35:0]` twin
/// (`40'(pk::PN)`, correct PRE), unreachable for the ascending source only because its
/// operand declined. PRE all three ascending/offset cells printed 32. Both oracles: 40.
#[test]
fn f_a_size_cast_of_such_a_name_binds_the_cast_size() {
    let src = "package pk;\n\
        \x20 localparam logic [0:35]        PA   = 36'hABC;\n\
        \x20 localparam logic [39:4]        PB   = 36'hDEF;\n\
        \x20 localparam logic [35:0]        PN   = 36'h123;\n\
        \x20 localparam logic signed [0:35] SA   = -36'sd5;\n\
        \x20 localparam logic signed [35:0] SN   = -36'sd5;\n\
        endpackage\n\
        module c #(parameter TAG=\"?\", parameter P = 8)();\n\
        \x20 initial $display(\"%s bits=%0d val=%h\", TAG, $bits(P), P);\n\
        endmodule\n\
        module top;\n\
        \x20 import pk::*;\n\
        \x20 c #(.TAG(\"c40_PA\"), .P(40'(pk::PA))) a();\n\
        \x20 c #(.TAG(\"c40_PB\"), .P(40'(pk::PB))) b();\n\
        \x20 c #(.TAG(\"c40_PN\"), .P(40'(pk::PN))) d();\n\
        \x20 c #(.TAG(\"c40_im\"), .P(40'(PA)))     e();\n\
        \x20 c #(.TAG(\"c40_SA\"), .P(40'(pk::SA))) f();\n\
        \x20 c #(.TAG(\"c40_SN\"), .P(40'(pk::SN))) g();\n\
        \x20 initial #1 $finish;\n\
        endmodule\n";
    let (o, c) = run(src);
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "c40_PA bits=40 val=0000000abc",
            "c40_PB bits=40 val=0000000def",
            "c40_PN bits=40 val=0000000123", // CONTROL
            "c40_im bits=40 val=0000000abc",
            // The SIGN travels with the operand: an ascending signed source now
            // sign-extends to 40 exactly as its descending twin below always did.
            "c40_SA bits=40 val=fffffffffb",
            "c40_SN bits=40 val=fffffffffb", // CONTROL
        ],
        "{o}"
    );
}

/// A NESTED package constant and a module-local untyped ALIAS of one — the two indirect
/// spellings. `pk2::X` re-declares `[0:35]` (PRE 32), `pk2::Y` re-declares `[35:0]` (PRE
/// 36, control: ascending-ness does not contaminate the VALUE, only the leaf layout), and
/// `localparam L = pk::PA` forwards through the untyped-alias arm (PRE 32). Both oracles:
/// 36 for all three.
#[test]
fn g_nested_package_and_untyped_alias_spellings() {
    let src = "package pk;\n  localparam logic [0:35] PA = 36'hABC;\nendpackage\n\
        package pk2;\n  import pk::*;\n  localparam logic [0:35] X = pk::PA;\n  localparam logic [35:0] Y = pk::PA;\nendpackage\n\
        module c #(parameter TAG=\"?\", parameter P = 8)();\n\
        \x20 initial $display(\"%s bits=%0d val=%h\", TAG, $bits(P), P);\n\
        endmodule\n\
        module top;\n\
        \x20 import pk::*;\n\
        \x20 localparam L = pk::PA;\n\
        \x20 localparam logic [35:0] LT = pk::PA;\n\
        \x20 c #(.TAG(\"nested_X \"), .P(pk2::X)) a();\n\
        \x20 c #(.TAG(\"nested_Y \"), .P(pk2::Y)) b();\n\
        \x20 c #(.TAG(\"modlocal \"), .P(L))      d();\n\
        \x20 c #(.TAG(\"modlocalT\"), .P(LT))     e();\n\
        \x20 initial begin $display(\"Lbits=%0d LTbits=%0d\", $bits(L), $bits(LT)); #1 $finish; end\n\
        endmodule\n";
    let (o, c) = run(src);
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            // `$bits` of the alias in the PARENT was already 36 in all three tools PRE —
            // pinned as the control that says the defect was on the BINDING, not the fold.
            "Lbits=36 LTbits=36",
            "nested_X  bits=36 val=000000abc",
            "nested_Y  bits=36 val=000000abc", // CONTROL
            "modlocal  bits=36 val=000000abc",
            "modlocalT bits=36 val=000000abc", // CONTROL
        ],
        "{o}"
    );
}

/// CONTROLS that must not move — the consumers that DO read a bit position, and the
/// declared-type leaf targets.
///
/// * `pk::PA[0:3]` is a PART-SELECT of the ascending constant. It is answered by
///   `wide_name_bits`' select arms, which still route to `narrow_param_bits` /
///   `pkg_const_narrow_bits` and still decline the layout — the hazard §4.5.363 recorded
///   is real there. All three tools print `bits=4 val=0`, PRE and POST.
/// * a leaf declared `logic [7:0]` / `int` takes its own type, not the override's.
/// * the value channel of a `wire [P-1:0]` read is unaffected.
#[test]
fn h_layout_reading_and_declared_type_consumers_are_unmoved() {
    let src = "package pk;\n  localparam logic [0:35] PA = 36'hABC;\nendpackage\n\
        module c8 #(parameter TAG=\"?\", parameter logic [7:0] P = 8)();\n\
        \x20 initial $display(\"%s bits=%0d val=%h\", TAG, $bits(P), P);\n\
        endmodule\n\
        module ci #(parameter TAG=\"?\", parameter int P = 8)();\n\
        \x20 initial $display(\"%s bits=%0d val=%h\", TAG, $bits(P), P);\n\
        endmodule\n\
        module cs #(parameter TAG=\"?\", parameter P = 8)();\n\
        \x20 initial $display(\"%s bits=%0d val=%h\", TAG, $bits(P), P);\n\
        endmodule\n\
        module cw #(parameter P = 8)();\n\
        \x20 wire [P-1:0] q; assign q = 2748;\n\
        \x20 initial $display(\"tw qw=%0d\", q);\n\
        endmodule\n\
        module top;\n\
        \x20 c8 #(.TAG(\"t8 \"), .P(pk::PA))      a();\n\
        \x20 ci #(.TAG(\"ti \"), .P(pk::PA))      b();\n\
        \x20 cs #(.TAG(\"sel\"), .P(pk::PA[0:3])) d();\n\
        \x20 cw #(.P(pk::PA))                    e();\n\
        \x20 initial #1 $finish;\n\
        endmodule\n";
    let (o, c) = run(src);
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "t8  bits=8 val=bc",
            "ti  bits=32 val=00000abc",
            "sel bits=4 val=0",
            "tw qw=2748",
        ],
        "{o}"
    );
}

/// RESIDUE, pinned so a later slice sees it move: a REDUCTION of such a constant in a
/// constant range bound (`wire [(|pk::PA)+2:0]`) is still LOUD (`VITA-E3009`), in both
/// lanes, where both oracles size the net at 4. The reduction needs the operand's BITS
/// (its value), not only its width, and those come through `wide_name_bits` — the one
/// consumer whose layout decline this slice deliberately leaves in place. Closing it
/// needs the fold's own accept set, which is a separate, un-started item.
#[test]
fn i_residue_a_reduction_bound_over_such_a_constant_is_still_loud() {
    let src = "package pk;\n  localparam logic [0:3] PA = 4'b1010;\n  localparam logic [3:0] PN = 4'b1010;\nendpackage\n\
        module top;\n\
        \x20 import pk::*;\n\
        \x20 wire [(|pk::PN)+2:0] ok;\n\
        \x20 wire [(|pk::PA)+2:0] bad;\n\
        \x20 initial #1 $finish;\n\
        endmodule\n";
    let (o, c) = run(src);
    assert_eq!(c, Some(1), "{o}");
    assert!(o.contains("VITA-E3009"), "{o}");
    assert!(
        o.contains("a reduction of an operand whose width the constant domain cannot read"),
        "{o}"
    );
}

/// NOT PINNED, recorded: `#(.P(pk::PA + 0))` is an ORACLE SPLIT — iverilog 13 binds 37
/// (its known self-contradiction on `+`: it answers 36 for the bare name in the same
/// design), verilator 5.052 binds 36. vita binds verilator's 36 POST (32 PRE), which is
/// IEEE §11.6.1's `max(36, 32)`. The cell is asserted only against verilator's value, and
/// this test exists to name the split rather than to claim a 2-oracle pin.
#[test]
fn j_operator_wrapped_spelling_matches_verilator_on_a_split_cell() {
    check(
        "  localparam logic [0:35] PA = 36'hABC;\n",
        "  c #(.TAG(\"plus0\"), .P(pk::PA + 0)) a();\n",
        &["plus0 bits=36 val=000000abc"],
    );
}
