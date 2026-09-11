//! IEEE 1800 §11.6 — a PACKAGE CONSTANT READ, `pk::K`, is a primary, so it fixes its
//! own width exactly as the wildcard-imported bare `K` does. `wide_top_is_self_determined`
//! listed `Ident` and not `PkgScoped`, so every consumer of that predicate refused the
//! scoped spelling and the value folded at the leaf's own default instead
//! (`#(.P(pk::PW))` printed the default's width; >64 bits printed E3009).
//!
//! Oracles: every expectation below was measured 3-way (vita / iverilog 13 `-g2012`+`vvp`
//! / verilator 5.052 `--binary --timing`); the cells that are NOT 2-oracle, and the
//! residues this slice deliberately leaves at their pre-slice answer, say so in place.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_psos_{}_{n}", std::process::id()));
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

const LEAF: &str = "module leaf #(parameter P = 1);\n  initial $display(\"leaf bits=%0d val=%h\", $bits(P), P);\nendmodule\n";

/// Build one census cell: a package body, then `LEAF`, then the body of `module t`.
fn cell(pkg: &str, body: &str) -> String {
    format!("package pk; {pkg} endpackage\n{LEAF}module t;\n  import pk::*;\n{body}  initial #1 $finish;\nendmodule\n")
}

fn check(pkg: &str, body: &str, want: &[&str]) {
    let (o, c) = run(&cell(pkg, body));
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(lines(&o), want, "{o}");
}

/// The row's own cell and its width bands: the scoped spelling now binds what the bare
/// wildcard-imported twin of the SAME declaration binds, which is both oracles' answer.
/// The leaf default is `parameter P = 1` except in `base`, which keeps `1'b0` — PRE the
/// scoped cell printed the DEFAULT's width (1, 8, 32), never a fixed lane.
#[test]
fn a_scoped_package_constant_override_binds_its_declared_width() {
    let both = ["leaf bits=36 val=800000001", "leaf bits=36 val=800000001"];
    let twins = "  leaf #(.P(pk::PW)) u1();\n  leaf #(.P(PW)) u2();\n";
    // base — leaf default `1'b0`; PRE `bits=1 val=1`.
    let (o, c) = run(&format!(
        "package pk; parameter logic [35:0] PW = 36'h8_0000_0001; endpackage\nmodule leaf #(parameter P = 1'b0);\n  initial $display(\"leaf bits=%0d val=%h\", $bits(P), P);\nendmodule\nmodule t;\n  import pk::*;\n{twins}  initial #1 $finish;\nendmodule\n"
    ));
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(lines(&o), both, "{o}");
    // B1 — a ≤32-bit package constant: PRE the VALUE fitted and only the width was wrong.
    check(
        "parameter logic [7:0]  PW = 8'hA5;",
        twins,
        &["leaf bits=8 val=a5", "leaf bits=8 val=a5"],
    );
    // B2 — 33..64. PRE `bits=32 val=00000001`: both columns wrong.
    check("parameter logic [35:0] PW = 36'h8_0000_0001;", twins, &both);
    // C1 / C2 — a SIGNED negative package constant, wide and narrow.
    check(
        "parameter logic signed [35:0] PW = -36'sd5;",
        twins,
        &["leaf bits=36 val=ffffffffb", "leaf bits=36 val=ffffffffb"],
    );
    check(
        "parameter logic signed [7:0] PW = -8'sd5;",
        twins,
        &["leaf bits=8 val=fb", "leaf bits=8 val=fb"],
    );
    // D1 / D3 — the package declaration's own form is irrelevant: untyped `parameter`
    // and untyped `localparam` carry the initializer's width just as the typed one does.
    check("parameter PW = 36'h8_0000_0001;", twins, &both);
    check("localparam PW = 36'h8_0000_0001;", twins, &both);
}

/// PRE, a >64-bit package constant was LOUD on the scoped spelling (E3009 "the override
/// of parameter `P` is not a constant") and correct on the bare twin. Two faces of one
/// defect: it went SILENT instead when the value happened to fit an i64 (`L_PS`).
#[test]
fn a_wide_scoped_package_constant_is_no_longer_refused() {
    // B3a — scoped instance alone; PRE rc=1 with E3009.
    check(
        "parameter logic [71:0] PW = 72'h11_2233_4455_6677_8899;",
        "  leaf #(.P(pk::PW)) u1();\n",
        &["leaf bits=72 val=112233445566778899"],
    );
    // L_PU — >64 and NOT representable in an i64: the loud face.
    check(
        "parameter logic [71:0] PU = 72'hFF_0000_0000_0000_0001;",
        "  leaf #(.P(pk::PU)) s(); leaf #(.P(PU)) b();\n",
        &[
            "leaf bits=72 val=ff0000000000000001",
            "leaf bits=72 val=ff0000000000000001",
        ],
    );
    // L_PS — >64 and representable: the silent face, PRE `bits=32 val=fffffffd`.
    check(
        "parameter logic signed [71:0] PS = -72'sd3;",
        "  leaf #(.P(pk::PS)) s(); leaf #(.P(PS)) b();\n",
        &[
            "leaf bits=72 val=fffffffffffffffffd",
            "leaf bits=72 val=fffffffffffffffffd",
        ],
    );
}

/// The rule reaches the override through every CHANNEL and every import form, one
/// forwarding level included — all measured identical and matching both oracles.
#[test]
fn every_channel_carries_the_scoped_width() {
    let both = ["leaf bits=36 val=800000001", "leaf bits=36 val=800000001"];
    const PKG: &str = "parameter logic [35:0] PW = 36'h8_0000_0001;";
    // H1 — `defparam`, the late channel.
    check(
        PKG,
        "  leaf u1();\n  leaf u2();\n  defparam u1.P = pk::PW;\n  defparam u2.P = PW;\n",
        &both,
    );
    // I1 — forwarded one level through an untyped middle parameter.
    let (o, c) = run(&format!(
        "package pk; {PKG} endpackage\n{LEAF}module mid #(parameter Q = 1);\n  leaf #(.P(Q)) l();\nendmodule\nmodule t;\n  import pk::*;\n  mid #(.Q(pk::PW)) m1();\n  mid #(.Q(PW))     m2();\n  initial #1 $finish;\nendmodule\n"
    ));
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(lines(&o), both, "{o}");
    // F1 — an EXPLICIT single-name import instead of the wildcard; unchanged answer.
    let (o, c) = run(&format!(
        "package pk; {PKG} endpackage\n{LEAF}module t;\n  import pk::PW;\n  leaf #(.P(pk::PW)) u1();\n  leaf #(.P(PW)) u2();\n  initial #1 $finish;\nendmodule\n"
    ));
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(lines(&o), both, "{o}");
}

/// The wrapper controls that were ALREADY correct PRE and must not move: a concat, a
/// full-width part-select and a concat with a literal all route the same `pk::PW` leaf
/// through a node kind the predicate already admitted.
#[test]
fn the_wrapper_controls_do_not_move() {
    const P36: &str = "parameter logic [35:0] PW = 36'h8_0000_0001;";
    const P72: &str = "parameter logic [71:0] PW = 72'h11_2233_4455_6677_8899;";
    // P1 / P2 — `{pk::PW}`.
    check(
        P36,
        "  leaf #(.P({pk::PW})) u1();\n",
        &["leaf bits=36 val=800000001"],
    );
    check(
        P72,
        "  leaf #(.P({pk::PW})) u1();\n",
        &["leaf bits=72 val=112233445566778899"],
    );
    // P3 / P4 — `pk::PW[w-1:0]`.
    check(
        P36,
        "  leaf #(.P(pk::PW[35:0])) u1();\n",
        &["leaf bits=36 val=800000001"],
    );
    check(
        P72,
        "  leaf #(.P(pk::PW[71:0])) u1();\n",
        &["leaf bits=72 val=112233445566778899"],
    );
    // E5 — `{pk::PW, 1'b0}`, scoped and bare identical PRE and POST.
    check(
        P36,
        "  leaf #(.P({pk::PW, 1'b0})) u1();\n  leaf #(.P({PW, 1'b0})) u2();\n",
        &["leaf bits=37 val=1000000002", "leaf bits=37 val=1000000002"],
    );
}

/// The OTHER five consumers of `wide_top_is_self_determined`, one cell each.
///
/// `40'(pk::PW)` and `40'(pk::PS)` are the size-cast operand widen; `pk::PW | pk::PZ`
/// and `pk::PW ^ 36'h1` are the extension-invariant bitwise tree. All four moved to the
/// bare twin's answer and both oracles agree. `$clog2`, a range bound, an index and a
/// replicate count were spelling-independent before and after (no gate on that route).
#[test]
fn the_other_consumers_of_the_predicate() {
    const PK: &str = "parameter logic [35:0] PW = 36'h8_0000_0001; parameter logic [35:0] PZ = 36'h0_F0F0_F0F0; parameter logic [7:0] P8 = 8'd12;";
    // K1 — casts, `$clog2`, the bitwise tree and a net range bound in one design.
    check(
        PK,
        "  leaf #(.P(40'(pk::PW))) c1();  leaf #(.P(40'(PW))) c2();\n  leaf #(.P($clog2(pk::PW))) c3(); leaf #(.P($clog2(PW))) c4();\n  leaf #(.P(pk::PW | pk::PZ)) c5(); leaf #(.P(PW | PZ)) c6();\n  logic [pk::P8-1:0] ns; logic [P8-1:0] nb;\n  initial $display(\"net scoped=%0d bare=%0d\", $bits(ns), $bits(nb));\n",
        &[
            "net scoped=12 bare=12",
            // c1 was `bits=32 val=00000001` PRE; c2 was already 40.
            "leaf bits=40 val=0800000001",
            "leaf bits=40 val=0800000001",
            // c3/c4 — `$clog2` has no self-determination gate; unmoved.
            "leaf bits=32 val=00000024",
            "leaf bits=32 val=00000024",
            // c5 was `bits=32 val=f0f0f0f1` PRE; c6 was already 36.
            "leaf bits=36 val=8f0f0f0f1",
            "leaf bits=36 val=8f0f0f0f1",
        ],
    );
    // A WIDENING cast of a signed negative package constant, a NARROWING cast, and the
    // bare twin of each. ⚠️ The narrowing `8'(...)` pair is a 1-ORACLE cell: vita and
    // iverilog print `bits=8 val=01`, verilator prints `bits=32 val=00000001` for BOTH
    // spellings. It is here as a non-move control, not as an adjudicated answer.
    let (o, c) = run(
        "package pk;\n  parameter logic [35:0] PW = 36'h8_0000_0001;\n  parameter logic signed [35:0] PS = -36'sd5;\nendpackage\nmodule leaf #(parameter P = 1);\n  initial $display(\"%m bits=%0d val=%h\", $bits(P), P);\nendmodule\nmodule t;\n  import pk::*;\n  leaf #(.P(40'(pk::PW))) c40s(); leaf #(.P(40'(PW))) c40b();\n  leaf #(.P(8'(pk::PW)))  c8s();  leaf #(.P(8'(PW)))  c8b();\n  leaf #(.P(40'(pk::PS))) cSs();  leaf #(.P(40'(PS))) cSb();\n  initial #1 $finish;\nendmodule\n",
    );
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "t.c40s bits=40 val=0800000001", // PRE 32/00000001
            "t.c40b bits=40 val=0800000001",
            "t.c8s bits=8 val=01", // unmoved, 1-oracle
            "t.c8b bits=8 val=01",
            "t.cSs bits=40 val=fffffffffb", // PRE 32/fffffffb
            "t.cSb bits=40 val=fffffffffb",
        ],
        "{o}"
    );
    // The bitwise tree over three tops. ⭐ `pk::PW & ~pk::PZ` WAS the recorded residue
    // here — a `~` inside the tree makes it non-extension-invariant, so the whole
    // override falls to `override_self_meta`, the second rung. That rung now certifies a
    // `PkgScoped` leaf under a qualified key, so `as` prints the `bits=36 val=800000001`
    // both oracles print, the same as its bare twin `ab`.
    let (o, c) = run(
        "package pk;\n  parameter logic [35:0] PW = 36'h8_0000_0001;\n  parameter logic [35:0] PZ = 36'h0_F0F0_F0F0;\nendpackage\nmodule leaf #(parameter P = 1);\n  initial $display(\"%m bits=%0d val=%h\", $bits(P), P);\nendmodule\nmodule t;\n  import pk::*;\n  leaf #(.P(pk::PW | pk::PZ))   os(); leaf #(.P(PW | PZ))   ob();\n  leaf #(.P(pk::PW & ~pk::PZ))  as(); leaf #(.P(PW & ~PZ))  ab();\n  leaf #(.P(pk::PW ^ 36'h1))    xs(); leaf #(.P(PW ^ 36'h1)) xb();\n  initial #1 $finish;\nendmodule\n",
    );
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "t.os bits=36 val=8f0f0f0f1", // PRE 32/f0f0f0f1
            "t.ob bits=36 val=8f0f0f0f1",
            "t.as bits=36 val=800000001", // PRE 32/00000001; now = `ab` and both oracles
            "t.ab bits=36 val=800000001",
            "t.xs bits=36 val=800000000", // PRE 32/00000000
            "t.xb bits=36 val=800000000",
        ],
        "{o}"
    );
    // The two backstop consumers (`param_bits_at_declared`, `wide_param_const_in_scope`)
    // are reached from a localparam INITIALIZER declared wider than the package
    // constant. Measured spelling-independent both PRE and POST, at both width bands —
    // another lane answers first — and equal to both oracles. Non-move controls.
    for (decl, want) in [
        (
            "39",
            "decl scoped bits=40 val=0800000001 / bare bits=40 val=0800000001",
        ),
        ("127", "decl scoped bits=128 val=00000000000000000000000800000001 / bare bits=128 val=00000000000000000000000800000001"),
    ] {
        let (o, c) = run(&format!(
            "package pk;\n  parameter logic [35:0] PW = 36'h8_0000_0001;\nendpackage\nmodule t;\n  import pk::*;\n  localparam logic [{decl}:0] Qs = pk::PW;\n  localparam logic [{decl}:0] Qb = PW;\n  initial $display(\"decl scoped bits=%0d val=%h / bare bits=%0d val=%h\", $bits(Qs), Qs, $bits(Qb), Qb);\n  initial #1 $finish;\nendmodule\n"
        ));
        assert_eq!(c, Some(0), "{o}");
        assert_eq!(lines(&o), [want], "{o}");
    }
    // A range bound, an index and a replicate count — `selfdet_bits_i64`'s route.
    // Spelling-independent PRE and POST, both oracles agree. Non-move control.
    let (o, c) = run(
        "package pk;\n  parameter logic [7:0] P8 = 8'd12;\nendpackage\nmodule t;\n  import pk::*;\n  logic [pk::P8-1:0] ns;\n  logic [P8-1:0] nb;\n  logic [31:0] v = 32'h0000_1000;\n  initial begin\n    $display(\"bound  scoped=%0d bare=%0d\", $bits(ns), $bits(nb));\n    $display(\"index  scoped=%b bare=%b\", v[pk::P8], v[P8]);\n    $display(\"repl   scoped bits=%0d val=%h\", $bits({pk::P8{1'b1}}), {pk::P8{1'b1}});\n    $display(\"repl   bare   bits=%0d val=%h\", $bits({P8{1'b1}}), {P8{1'b1}});\n  end\n  initial #1 $finish;\nendmodule\n",
    );
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "bound  scoped=12 bare=12",
            "index  scoped=1 bare=1",
            "repl   scoped bits=12 val=fff",
            "repl   bare   bits=12 val=fff",
        ],
        "{o}"
    );
}

/// RECORDED RESIDUES — these lines are pinned at their UNCHANGED answer so a later slice
/// sees them move. None of them is a pass.
///
/// - ⭐ E2 `~pk::PW` and E3 `pk::PW + 1` are CLOSED (§2 "Index sealing" ⓒ). The second
///   rung got exactly the qualified `ConstWidths` key this note predicted:
///   `declared_override_widths::names` now collects a `PkgScoped` leaf, certifies it
///   through `pkg_const_narrow_bits` under `"{pkg}::{name}"`, and
///   `ctx_width_names_are_evident` reads it back there. E2 is now `bits=36
///   val=7fffffffe` — both oracles. E3 is now `bits=36 val=800000002`, verilator's
///   answer; iverilog says `bits=37` (its documented bound-`+` max+1
///   self-contradiction, non-evidence).
/// - L_PA `logic [11:4] PA`: a non-zero declared LSB makes `pkg_const_narrow_bits` and
///   `narrow_param_bits` decline, so BOTH spellings print 32 where both oracles print
///   `bits=8 val=a5`. A pre-existing ROADMAP §2 class, upstream of this predicate — and
///   the CONTROL for the two closures above: the new qualified key routes a `PkgScoped`
///   leaf to `pkg_const_narrow_bits`, which still refuses a non-zero LSB, so this line is
///   byte-identical PRE→POST. Re-measured: vita `t.s bits=32 val=000000a5` /
///   `t.b bits=32 val=000000a5`, both oracles `bits=8 val=a5`.
#[test]
fn the_second_rung_and_the_non_zero_lsb_class_are_unmoved() {
    const PK: &str = "parameter logic [35:0] PW = 36'h8_0000_0001;";
    check(
        PK,
        "  leaf #(.P(~pk::PW)) u1();\n  leaf #(.P(~PW)) u2();\n",
        &["leaf bits=36 val=7fffffffe", "leaf bits=36 val=7fffffffe"],
    );
    check(
        PK,
        "  leaf #(.P(pk::PW + 1)) u1();\n  leaf #(.P(PW + 1)) u2();\n",
        &["leaf bits=36 val=800000002", "leaf bits=36 val=800000002"],
    );
    check(
        "parameter logic [11:4] PA = 8'hA5;",
        "  leaf #(.P(pk::PA)) s(); leaf #(.P(PA)) b();\n",
        &["leaf bits=32 val=000000a5", "leaf bits=32 val=000000a5"],
    );
}
