//! §4.5.598: a VARIABLE driven by a continuous `assign` takes no other driver — not a
//! second continuous `assign`, not a procedural write, not a declaration initializer.
//!
//! IEEE 1800-2017 §6.5: "it shall be an error to have multiple continuous assignments
//! or a mixture of procedural and continuous assignments writing to any term in the
//! expansion of a written longest static prefix of a logic variable"; §10.3.2: a
//! variable written by a continuous assignment shall not be initialized in its
//! declaration nor written by a procedural assignment. A NET takes any number of
//! continuous drivers and resolves them (§6.6); a variable does not.
//!
//! vita checked the mixture only when an `always_comb` / `always_ff` / `always_latch`
//! was one of the writers (`elaborate/src/multidriver.rs` Rule B). Everything else ran
//! at exit 0: two whole `assign`s on a `logic` were resolved like a `wire` (`y=x` on a
//! conflict), and an `assign` beside a plain `always`, an `initial`, a `final` or an
//! initializer was settled by whichever writer ran last. Rule D now reports each one as
//! `VITA-E3001`, once per variable, where iverilog 13.0 reports most of them: the
//! second `assign`, the procedure, or the declaration.
//!
//! Oracles, one shape per file. iverilog 13.0 (`-g2012`), verilator 5.052 (`--lint-only
//! -Wall --timing`; `--binary --timing` for values), sv2v 0.0.13 → iverilog 13.0:
//!
//! ```text
//!   two whole assigns on `logic y` (n_logic_cc_int, also `assign y = a & b, y = a | b;`)
//!     iverilog   d_cc_int.sv:5: error: Variable 'y' cannot have multiple drivers.
//!     verilator  %Warning-MULTIDRIVEN: d_cc_int.sv:3:9: Bit [0] of signal 't.y' have
//!                multiple combinational drivers. This can cause performance degradation.
//!     vita PRE   y=x / y=0, exit 0
//!   the same on `output logic y` (n_logic_bus, tri-state `en ? d : 1'bz` drivers)
//!     iverilog   n_logic_bus.sv:3: error: Variable 'y' cannot have multiple drivers.
//!     verilator  --lint-only: nothing; --binary: y=1 y=0 y=0 y=0 (2-state, no z)
//!     vita PRE   y=x y=0 y=z y=0, exit 0
//!   assign + always @(*) (n_logic_mix port, n_logic_mix_int body), assign + always @(a or b)
//!     iverilog   n_logic_mix.sv:3: error: Cannot perform procedural assignment to
//!                variable 'y' because it is also continuously assigned.
//!     verilator  %Warning-MULTIDRIVEN: n_logic_mix.sv:1:52: Bit [0] of signal 'y' have
//!                multiple combinational drivers. This can cause performance degradation.
//!     vita PRE   y=0 y=0 y=1 (iverilog-of-sv2v y=x y=0 y=1, verilator y=1 y=0 y=1)
//!   assign + always @(posedge clk) y <= ~a
//!     iverilog   the "Cannot perform procedural assignment" error
//!     verilator  %Error-BLKANDNBLK: d_mix_nba.sv:3:9: Unsupported: Blocking and
//!                non-blocking assignments to potentially overlapping bits of same packed
//!                variable: 't.y'
//!   assign + initial y = 0;  logic y = 1'b0 + assign;  output logic y = 1'b0 + assign
//!     iverilog   the "Cannot perform procedural assignment" error
//!     verilator  %Error-CONTASSINIT: d_init.sv:3:13: Continuous assignment to variable
//!                with initial value: 'y'
//!   assign + initial assign y = 0; (a procedural continuous assignment)
//!     iverilog   the "Cannot perform procedural assignment" error, twice
//!     verilator  %Error-CONTASSINIT: o_proc_assign.sv:5:48: Continuous assignment to
//!                variable with initial value: 'y'
//!   assign + final y = 0;
//!     iverilog   the "Cannot perform procedural assignment" error
//!     verilator  nothing (runs y=1 y=0)
//!   and g1 (y, a, b); + assign y = c;   and g1 (…); + and g2 (…);   gate + initial
//!     iverilog   d_gate.sv:4: error: Variable 'y' cannot be driven by a primitive or
//!                continuous assignment with non-default strength.
//!     verilator  %Warning-MULTIDRIVEN: d_gate.sv:3:9: Bit [0] of signal 't.y' …
//!                (gate + initial: %Error-CONTASSINIT: b27_gate_plus_initial.sv:4:11:
//!                Continuous assignment to variable with initial value: 'y')
//!   assign + always_latch (d_mix_latch; also on an output port)
//!     iverilog   d_mix_latch.sv:5: error: Cannot perform procedural assignment to
//!                variable 'y' because it is also continuously assigned.
//!     verilator  nothing (runs y=0 y=1)
//!   two assigns + always_latch (p11), initializer + assign + always_latch (p12),
//!   assign + always_latch + initial (q11)
//!     iverilog   p11_latch_two_assign.sv:4: error: Variable 'y' cannot have multiple
//!                drivers. / p11_latch_two_assign.sv:5: error: Cannot perform procedural
//!                assignment to variable 'y' because it is also continuously assigned.
//!                (p12, q11: the "Cannot perform procedural assignment" error, twice)
//!     verilator  p11 nothing (runs y=0); p12 %Error-CONTASSINIT:
//!                p12_latch_init_assign.sv:2:13: …; q11 %Error-CONTASSINIT:
//!                q11_latch_assign_initial.sv:5:11: …
//!     vita PRE   W3060 only, exit 0 (p11 `y=x`)
//! ```
//!
//! `n_logic_bus` is E3001 by owner ruling (2026-10-06): IEEE §6.5 / §10.3.2 and iverilog
//! reject it, and verilator is not an x/z oracle. `assign` + `final` and `assign` +
//! `always_latch` are E3001 on IEEE §6.5 and iverilog against verilator's silence, under
//! the same ruling ("loud where IEEE says error"; E over W). Rule B's `always_latch`
//! warning W3060 stands down whenever a continuous `assign` drives the variable, and stays
//! for the latch pairs with no `assign`: two `always_latch`, `always_latch` + `initial`,
//! and an initializer + `always_latch` (iverilog runs all three, verilator reports no
//! MULTIDRIVEN).
//!
//! Accepted, and pinned to the oracles' values: the same tri-state bus on a `wire`
//! (`y=x1x0` — iverilog and sv2v → iverilog; verilator `y=1110`), a single `assign`,
//! disjoint partial assigns (`y=01 y=10`, all three), a partial `assign` beside a
//! procedural write of the other bit (`y=11 y=00`, iverilog and verilator), `force`
//! over an `assign` (IEEE §10.6.2; `y=1 y=0 y=1 y=0`, iverilog and verilator), and a
//! block-local or `for`-loop variable named like the assigned variable (`A1 x=00 M1
//! x=11 M0 x=22` and `L y=00 L y=01 M y=11`, all three; verilator
//! `%Warning-VARHIDDEN`).
//!
//! Out of the rule, each a ROADMAP §3.b row: a select, element or member write
//! (`mdrv-partial`), a write through a task or a hierarchical name (`mdrv-ca-call`), a
//! port binding, a generate block or an interface body (`mdrv-ca-port`), and an
//! unpacked array (E3009 from `cont_array.rs`, `mdrv-ca-array`).
//!
//! The assertion-action cells (`mdrv-assert-local`) are at the end of the file.
//!
//! Every test counts the EXACT set of codes the run prints, under `-Wno-W1017`.

use std::collections::BTreeSet;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run one design through one-shot `vita` with `-Wno-W1017` and return its exit code
/// and combined output.
fn run(src: &str) -> (Option<i32>, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cavd_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(d.join("t.sv"), src).expect("write design");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("-Wno-W1017")
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code(), s)
}

/// Every diagnostic code the run printed (`[VITA-…]`), as a set.
fn codes(out: &str) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for (i, _) in out.match_indices("[VITA-") {
        let rest = &out[i + 1..];
        if let Some(end) = rest.find(']') {
            set.insert(rest[..end].to_string());
        }
    }
    set
}

fn expect_codes(out: &str, want: &[&str], what: &str) {
    let want: BTreeSet<String> = want.iter().map(|c| c.to_string()).collect();
    assert_eq!(codes(out), want, "{what}: exact code set:\n{out}");
}

/// The design's own output lines: diagnostics and status lines dropped.
fn shown(out: &str) -> Vec<&str> {
    out.lines()
        .filter(|l| {
            !l.contains("[VITA-") && !l.starts_with("simulation ended") && !l.starts_with("errors=")
        })
        .collect()
}

/// The design is refused with exactly one `VITA-E3001` line, which starts with `at`
/// (`t.sv:LINE:COL: error[VITA-E3001]`) and contains every `needle`.
fn rejects(src: &str, at: &str, needles: &[&str], what: &str) {
    let (rc, out) = run(src);
    assert_eq!(rc, Some(1), "{what}: expected exit 1:\n{out}");
    expect_codes(&out, &["VITA-E3001"], what);
    let lines: Vec<&str> = out.lines().filter(|l| l.contains("[VITA-E3001]")).collect();
    assert_eq!(lines.len(), 1, "{what}: one E3001 per variable:\n{out}");
    assert!(
        lines[0].starts_with(&format!("{at}: error[VITA-E3001]")),
        "{what}: caret at {at}:\n{out}"
    );
    for n in needles {
        assert!(lines[0].contains(n), "{what}: `{n}` in:\n{out}");
    }
}

/// A clean run printing exactly `want`.
fn accepts(src: &str, want: &[&str], what: &str) {
    let (rc, out) = run(src);
    assert_eq!(rc, Some(0), "{what}: expected exit 0:\n{out}");
    expect_codes(&out, &[], what);
    assert_eq!(shown(&out), want, "{what}:\n{out}");
}

const TWO_CA: &str = "is driven by more than one continuous `assign`";
const TWO_GATE: &str =
    "is driven by more than one continuous driver (an `assign` or a gate output)";
const CA_PROC: &str = "is driven by a continuous `assign` AND written by";
const CA_INIT: &str = "has a declaration initializer AND is driven by a continuous `assign`";

// ── two continuous assigns on a variable ─────────────────────────────────────────

#[test]
fn n_logic_cc_int_two_assigns_on_a_body_variable() {
    // The blog's `vchk/n_logic_cc_int.sv`, verbatim.
    rejects(
        "module n_logic_cc_int (input logic a, b, output logic o);
    logic y;
    assign y = a & b;
    assign y = a | b;
    assign o = y;
endmodule
",
        "t.sv:4:5",
        &["variable `y`", TWO_CA, "(IEEE §6.5)", "[in n_logic_cc_int]"],
        "n_logic_cc_int",
    );
}

#[test]
fn n_logic_bus_two_tristate_assigns_on_an_output_port() {
    // The blog's `chk/n_logic_bus.sv`, verbatim. E3001 by owner ruling (2026-10-06):
    // IEEE §6.5 / §10.3.2 and iverilog ("Variable 'y' cannot have multiple drivers.")
    // reject it; verilator, which reports nothing, is not an x/z oracle.
    rejects(
        "module n_logic_bus (input logic en0, en1, d0, d1, output logic y);
    assign y = en0 ? d0 : 1'bz;
    assign y = en1 ? d1 : 1'bz;
endmodule
",
        "t.sv:3:5",
        &[
            "output port `y` (a variable, IEEE §23.2.2.3)",
            TWO_CA,
            "declare it a net (`wire`)",
        ],
        "n_logic_bus",
    );
}

#[test]
fn two_assigns_on_an_output_port_under_a_testbench() {
    // PRE: `y=x` / `y=0` at exit 0 — wire resolution of a variable. iverilog
    // `d_cc_port.sv:3: error: Variable 'y' cannot have multiple drivers.`, verilator
    // `%Warning-MULTIDRIVEN: … Bit [0] of signal 't.y' have multiple combinational drivers.`
    rejects(
        "module m (input logic a, b, output logic y);
  assign y = a & b;
  assign y = a | b;
endmodule
module t;
  logic a = 1, b = 0; wire y;
  m u (.a(a), .b(b), .y(y));
  initial begin #1 $display(\"y=%b\", y); a = 0; #1 $display(\"y=%b\", y); $finish; end
endmodule
",
        "t.sv:3:3",
        &["output port `y`", TWO_CA, "[in t.u]"],
        "port, two assigns",
    );
}

#[test]
fn two_drivers_in_one_assign_item_a_gate_and_a_struct_variable() {
    // One `assign` item with two left-hand sides naming `y` is two drivers (iverilog
    // `Variable 'y' cannot have multiple drivers.`, verilator MULTIDRIVEN; PRE `y=x`).
    rejects(
        "module t;
  logic a = 1, b = 0;
  logic y;
  assign y = a & b, y = a | b;
  initial begin #1 $display(\"y=%b\", y); $finish; end
endmodule
",
        "t.sv:4:3",
        &["variable `y`", TWO_CA],
        "one item, two lhs",
    );
    // A gate output is a continuous driver (iverilog `Variable 'y' cannot be driven by a
    // primitive or continuous assignment with non-default strength.`, verilator
    // MULTIDRIVEN; PRE `y=x`).
    rejects(
        "module t;
  logic a = 1, b = 0, c = 1;
  logic y;
  and g1 (y, a, b);
  assign y = c;
  initial begin #1 $display(\"y=%b\", y); $finish; end
endmodule
",
        "t.sv:5:3",
        &["variable `y`", TWO_GATE, "keep one driver"],
        "gate + assign",
    );
    // Two gates (iverilog the same error at both gates, verilator `%Warning-MULTIDRIVEN:
    // b28_gate_pair.sv:2:19: …`; PRE `y=1`).
    rejects(
        "module t;
  logic a = 1, b = 1;
  logic y;
  and g1 (y, a, b);
  or g2 (y, a, b);
  initial begin #1 $display(\"y=%b\", y); $finish; end
endmodule
",
        "t.sv:5:3",
        &["variable `y`", TWO_GATE],
        "two gates",
    );
    // A gate beside a procedural writer or an initializer (iverilog the primitive-driver
    // error, verilator `%Error-CONTASSINIT`; PRE `y=0` / `y=1`).
    rejects(
        "module t;
  logic a = 1, b = 1;
  logic y;
  and g (y, a, b);
  initial y = 1'b0;
  initial begin #1 $display(\"y=%b\", y); $finish; end
endmodule
",
        "t.sv:5:3",
        &[
            "variable `y`",
            "is driven by a gate output AND written by `initial`",
            "keep the gate",
        ],
        "gate + initial",
    );
    rejects(
        "module t;
  logic a = 1, b = 1;
  logic y = 1'b0;
  and g (y, a, b);
  initial begin #1 $display(\"y=%b\", y); $finish; end
endmodule
",
        "t.sv:3:3",
        &[
            "variable `y`",
            "has a declaration initializer AND is driven by a gate output",
        ],
        "gate + initializer",
    );
    // A packed-struct variable (iverilog `Variable 'y' cannot have multiple drivers.`,
    // verilator `%Warning-MULTIDRIVEN: … Bits [7:0] of signal 't.y'`; PRE `y=XX`).
    rejects(
        "module t;
  typedef struct packed { logic [3:0] hi; logic [3:0] lo; } s_t;
  logic [7:0] a = 8'h12, b = 8'h34;
  s_t y;
  assign y = a;
  assign y = b;
  initial begin #1 $display(\"y=%h\", y); $finish; end
endmodule
",
        "t.sv:6:3",
        &["variable `y`", TWO_CA],
        "struct variable",
    );
}

#[test]
fn a_whole_assign_beside_a_concat_or_delayed_assign_is_reported_once() {
    // PRE reported these through the flat overlap check, with no location (`error[VITA-
    // E3001] E-ELAB-MULTIDRIVER: net `t.y` driven by multiple overlapping continuous
    // assignments`). Rule D reports the variable at its second `assign` and the overlap
    // check stands down for that net: one line, not two. iverilog `Variable 'y' cannot
    // have multiple drivers.` on both.
    rejects(
        "module t;
  logic [1:0] a = 2'b10;
  logic y, z;
  assign {y, z} = a;
  assign y = 1'b0;
  initial begin #1 $display(\"y=%b z=%b\", y, z); $finish; end
endmodule
",
        "t.sv:5:3",
        &["variable `y`", TWO_CA],
        "concat + whole",
    );
    rejects(
        "module t;
  logic a = 1, b = 0;
  logic y;
  assign #1 y = a;
  assign y = b;
  initial begin #2 $display(\"y=%b\", y); $finish; end
endmodule
",
        "t.sv:5:3",
        &["variable `y`", TWO_CA],
        "delayed + whole",
    );
}

#[test]
fn a_reg_with_two_assigns_is_one_e3001() {
    // A `reg` takes one continuous assign as a `logic` does (§4.5.600), so two are Rule
    // D's E3001 alone — one diagnostic per variable. Until §4.5.600 the run also printed
    // E3018 once per assign (IEEE 1364's rule). iverilog `d_reg_cc.sv:5: error: Variable
    // 'y' cannot have multiple drivers.`
    let (rc, out) = run("module t;
  logic a = 1, b = 0;
  reg y;
  assign y = a & b;
  assign y = a | b;
  initial begin #1 $display(\"y=%b\", y); $finish; end
endmodule
");
    assert_eq!(rc, Some(1), "{out}");
    expect_codes(&out, &["VITA-E3001"], "reg, two assigns");
}

// ── a continuous assign beside a procedural write ────────────────────────────────

#[test]
fn n_logic_mix_assign_and_always_star_on_an_output_port() {
    // The blog's `chk/n_logic_mix.sv`, verbatim.
    rejects(
        "module n_logic_mix (input logic a, b, output logic y);
    assign y = a & b;
    always @(*) y = a | b;
endmodule
",
        "t.sv:3:5",
        &[
            "output port `y` (a variable, IEEE §23.2.2.3)",
            CA_PROC,
            "`always`",
            "(IEEE §6.5, §10.3.2)",
        ],
        "n_logic_mix",
    );
}

#[test]
fn n_logic_mix_int_assign_and_always_star_on_a_body_variable() {
    // The blog's `vchk/n_logic_mix_int.sv`, verbatim.
    rejects(
        "module n_logic_mix_int (input logic a, b, output logic o);
    logic y;
    assign y = a & b;
    always @(*) y = a | b;
    assign o = y;
endmodule
",
        "t.sv:4:5",
        &["variable `y`", CA_PROC, "`always`"],
        "n_logic_mix_int",
    );
}

#[test]
fn an_assign_beside_any_plain_process() {
    let with = |proc: &str| {
        format!(
            "module t;
  logic clk = 0, a = 1, b = 0;
  logic y;
  assign y = a;
  {proc}
  initial begin #1 $display(\"y=%b\", y); clk = 1; a = 0; #1 $display(\"y=%b\", y); $finish; end
endmodule
"
        )
    };
    // Each: iverilog `Cannot perform procedural assignment to variable 'y' because it is
    // also continuously assigned.`; PRE ran every one at exit 0.
    for (proc, what, label) in [
        // verilator %Warning-MULTIDRIVEN
        ("always @* y = a | b;", "`always`", "always @*"),
        (
            "always @(a or b) y = a | b;",
            "`always`",
            "always @(a or b)",
        ),
        // verilator %Error-BLKANDNBLK
        ("always @(posedge clk) y <= ~a;", "`always`", "always NBA"),
        // verilator %Error-CONTASSINIT
        ("initial y = 0;", "`initial`", "initial"),
        // verilator %Error-CONTASSINIT (a procedural continuous assignment)
        (
            "initial begin #1 assign y = 0; end",
            "`initial`",
            "procedural assign",
        ),
        // verilator: nothing
        ("final y = 0;", "`final`", "final"),
    ] {
        rejects(
            &with(proc),
            "t.sv:5:3",
            &["variable `y`", CA_PROC, what],
            label,
        );
    }
}

#[test]
fn an_assign_beside_an_always_star_on_a_non_ansi_port() {
    // iverilog `Cannot perform procedural assignment to variable 'y' …`, verilator
    // MULTIDRIVEN; PRE `y=1`.
    rejects(
        "module m (a, y);
  input a;
  output logic y;
  assign y = a;
  always @* y = ~a;
endmodule
module t;
  logic a = 1; wire y;
  m u (.a(a), .y(y));
  initial begin #1 $display(\"y=%b\", y); $finish; end
endmodule
",
        "t.sv:5:3",
        &["output port `y`", CA_PROC, "[in t.u]"],
        "non-ANSI port",
    );
}

#[test]
fn an_assertion_action_writing_the_variable_is_a_procedural_writer() {
    // A module-level `assert property` is a process; its action writes `x` directly.
    // No oracle runs the item (iverilog `sorry: concurrent_assertion_item not
    // supported.`; verilator --lint-only is silent and --binary prints `A1 x=00`), so
    // this is IEEE §6.5 alone — the same writer as `initial x = 8'h00;`.
    rejects(
        "module t;
  logic clk = 0;
  logic [7:0] a = 8'h11;
  logic [7:0] x;
  assign x = a;
  a1: assert property (@(posedge clk) 1'b0) else x = 8'h00;
  initial begin #1 clk = 1; #1 clk = 0; #1 $display(\"A1 x=%h\", x); $finish; end
endmodule
",
        "t.sv:6:7",
        &["variable `x`", CA_PROC, "`initial`"],
        "assertion action write",
    );
}

#[test]
fn an_assign_beside_an_always_latch_is_rule_d_not_the_latch_warning() {
    // Rule B's latch branch warned (W3060) and stopped there, so a continuous `assign`
    // beside an `always_latch` ran at exit 0 — and so did two `assign`s, an initializer
    // or an `initial` beside it. Rule D owns them now; one E3001 line each.
    let latch = |items: &str, decl: &str| {
        format!(
            "module t;
  logic en = 1, d = 0, a = 1;
  {decl}
{items}
  initial begin #1 $display(\"y=%b\", y); $finish; end
endmodule
"
        )
    };
    // d_mix_latch: iverilog `Cannot perform procedural assignment to variable 'y'
    // because it is also continuously assigned.`; verilator nothing.
    rejects(
        &latch("  assign y = a;\n  always_latch if (en) y = d;", "logic y;"),
        "t.sv:5:3",
        &["variable `y`", CA_PROC, "`always_latch`"],
        "assign + always_latch",
    );
    // p11: iverilog `Variable 'y' cannot have multiple drivers.` and the procedural
    // error; verilator nothing; PRE `y=x`, W3060 only.
    rejects(
        &latch(
            "  assign y = 1'b0;\n  assign y = 1'b1;\n  always_latch if (en) y = d;",
            "logic y;",
        ),
        "t.sv:5:3",
        &["variable `y`", TWO_CA],
        "two assigns + always_latch",
    );
    // p12: iverilog the procedural error twice, verilator `%Error-CONTASSINIT`.
    rejects(
        &latch(
            "  assign y = a;\n  always_latch if (en) y = d;",
            "logic y = 1'b0;",
        ),
        "t.sv:5:3",
        &["variable `y`", CA_PROC, "`always_latch`"],
        "initializer + assign + always_latch",
    );
    // q11: iverilog the procedural error twice, verilator `%Error-CONTASSINIT`. The
    // first procedural writer in body order is named.
    rejects(
        &latch(
            "  assign y = a;\n  always_latch if (en) y = d;\n  initial y = 1'b1;",
            "logic y;",
        ),
        "t.sv:5:3",
        &["variable `y`", CA_PROC, "`always_latch`"],
        "assign + always_latch + initial",
    );
    // On an output port: iverilog the procedural error, verilator nothing.
    rejects(
        "module m (input logic en, d, a, output logic y);
  assign y = a;
  always_latch if (en) y = d;
endmodule
module t;
  logic en = 1, d = 0, a = 1; wire y;
  m u (.en(en), .d(d), .a(a), .y(y));
  initial begin #1 $display(\"y=%b\", y); $finish; end
endmodule
",
        "t.sv:3:3",
        &["output port `y`", CA_PROC, "`always_latch`", "[in t.u]"],
        "port, assign + always_latch",
    );
}

#[test]
fn a_latch_pair_with_no_assign_keeps_its_warning() {
    // Unchanged (Rule B / Rule C): iverilog runs each (`y=1`, `y=0`, `y=0`) and
    // verilator reports no MULTIDRIVEN (PROCASSINIT on the initializer).
    for (decl, items, want) in [
        (
            "logic y;",
            "  always_latch if (en) y = d;\n  always_latch if (!en) y = a;",
            "is written by `always_latch` AND by `always_latch`",
        ),
        (
            "logic y;",
            "  always_latch if (!en) y = d;\n  initial y = 1'b0;",
            "is written by `always_latch` AND by `initial`",
        ),
        (
            "logic y = 1'b0;",
            "  always_latch if (!en) y = d;",
            "has a declaration initializer AND is written by `always_latch`",
        ),
    ] {
        let src = format!(
            "module t;
  logic en = 1, d = 1, a = 0;
  {decl}
{items}
  initial begin #1 $display(\"y=%b\", y); $finish; end
endmodule
"
        );
        let (rc, out) = run(&src);
        assert_eq!(rc, Some(0), "{out}");
        expect_codes(&out, &["VITA-W3060"], want);
        assert!(out.contains(want), "{want}:\n{out}");
    }
}

// ── a declaration initializer beside a continuous assign ─────────────────────────

#[test]
fn an_initializer_beside_an_assign() {
    // iverilog `d_init.sv:3: error: Cannot perform procedural assignment to variable 'y'
    // because it is also continuously assigned.`, verilator `%Error-CONTASSINIT:
    // d_init.sv:3:13: Continuous assignment to variable with initial value: 'y'`; PRE
    // `y=1 y=0`.
    rejects(
        "module t;
  logic a = 1;
  logic y = 1'b0;
  assign y = a;
  initial begin #1 $display(\"y=%b\", y); a = 0; #1 $display(\"y=%b\", y); $finish; end
endmodule
",
        "t.sv:3:3",
        &["variable `y`", CA_INIT, "(IEEE §10.3.2)"],
        "body initializer",
    );
    // An ANSI output port's default is its initializer: iverilog `d_port_default.sv:1:
    // error: Cannot perform procedural assignment …`, verilator `%Error-CONTASSINIT:
    // d_port_default.sv:1:43: …`; PRE `y=1 y=0`.
    rejects(
        "module m (input logic a, output logic y = 1'b0);
  assign y = a;
endmodule
module t;
  logic a = 1; wire y;
  m u (.a(a), .y(y));
  initial begin #1 $display(\"y=%b\", y); a = 0; #1 $display(\"y=%b\", y); $finish; end
endmodule
",
        "t.sv:1:26",
        &["output port `y`", CA_INIT],
        "port default",
    );
    // The corpus cell `AI/s594__g__c2__Sdly_L` (excluded from the manifest as
    // oracle-rejects): iverilog `Sdly_L.sv:4: error: Cannot perform procedural
    // assignment to variable 'w' because it is also continuously assigned.`, verilator
    // `%Error-CONTASSINIT: Sdly_L.sv:4:13: …`.
    rejects(
        "`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  logic w = 0;
  assign #((X + 2'b00) - 8'd250) w = 1'b1;
  initial begin #1 $display(\"t1 w=%b\", w); #2 $display(\"t3 w=%b\", w); end
  initial #20 $finish;
endmodule
",
        "t.sv:4:3",
        &["variable `w`", CA_INIT],
        "Sdly_L",
    );
}

// ── accepted ─────────────────────────────────────────────────────────────────────

#[test]
fn n_wire_bus_two_assigns_on_a_net_resolve() {
    // The blog's `chk/n_wire_bus.sv` module under a testbench. iverilog and sv2v →
    // iverilog `y=x1x0`; verilator (2-state) `y=1110`.
    accepts(
        "module n_wire_bus (input logic en0, en1, d0, d1, output wire y);
    assign y = en0 ? d0 : 1'bz;
    assign y = en1 ? d1 : 1'bz;
endmodule
module t;
  logic en0 = 1, en1 = 1, d0 = 0, d1 = 1;
  wire y;
  logic [3:0] s;
  n_wire_bus u (.en0(en0), .en1(en1), .d0(d0), .d1(d1), .y(y));
  initial begin
    #1 s[3] = y; en0 = 0;
    #1 s[2] = y; en0 = 1;
    #1 s[1] = y; en1 = 0;
    #1 s[0] = y;
    $display(\"y=%b\", s);
    $finish;
  end
endmodule
",
        &["y=x1x0"],
        "n_wire_bus",
    );
}

#[test]
fn one_assign_and_partial_assigns_are_single_drivers() {
    // All three oracles: `y=1` / `y=0`.
    accepts(
        "module t;
  logic a = 1;
  logic y;
  assign y = a;
  initial begin #1 $display(\"y=%b\", y); a = 0; #1 $display(\"y=%b\", y); $finish; end
endmodule
",
        &["y=1", "y=0"],
        "one assign",
    );
    // Disjoint bits. All three oracles: `y=01` / `y=10`.
    accepts(
        "module t;
  logic a = 1, b = 0;
  logic [1:0] y;
  assign y[0] = a;
  assign y[1] = b;
  initial begin #1 $display(\"y=%b\", y); a = 0; b = 1; #1 $display(\"y=%b\", y); $finish; end
endmodule
",
        &["y=01", "y=10"],
        "disjoint partial assigns",
    );
    // A partial assign beside a procedural write of the other bit. iverilog and
    // verilator `y=11` / `y=00` (sv2v → iverilog `y=1x` / `y=00`).
    accepts(
        "module t;
  logic a = 1, b = 0;
  logic [1:0] y;
  assign y[0] = a;
  always @* y[1] = b;
  initial begin #1 b = 1; #1 $display(\"y=%b\", y); a = 0; b = 0; #1 $display(\"y=%b\", y); $finish; end
endmodule
",
        &["y=11", "y=00"],
        "partial assign + partial procedural",
    );
}

#[test]
fn force_over_an_assign_is_not_a_driver() {
    // IEEE §10.6.2: a `force` overrides a continuous assignment to a variable until its
    // `release`. iverilog and verilator `y=1 y=0 y=1 y=0` (sv2v → iverilog, which
    // turns `y` into a net, `y=x y=0 y=0 y=0`).
    accepts(
        "module t;
  logic a = 1;
  logic y;
  assign y = a;
  initial begin #1 $display(\"y=%b\", y); force y = 0; #1 $display(\"y=%b\", y); release y; #1 $display(\"y=%b\", y); a = 0; #1 $display(\"y=%b\", y); $finish; end
endmodule
",
        &["y=1", "y=0", "y=1", "y=0"],
        "force",
    );
}

#[test]
fn a_block_local_or_loop_variable_of_the_same_name_is_not_the_variable() {
    // All three oracles print these lines (verilator `%Warning-VARHIDDEN`).
    accepts(
        "module t;
  logic [7:0] a = 8'h11;
  logic [7:0] x;
  assign x = a;
  initial begin : b
    logic [7:0] x;
    x = 8'h00;
    #1 $display(\"A1 x=%h\", x);
  end
  initial begin
    #2 $display(\"M1 x=%h\", t.x);
    a = 8'h22;
    #1 $display(\"M0 x=%h\", t.x);
    $finish;
  end
endmodule
",
        &["A1 x=00", "M1 x=11", "M0 x=22"],
        "block-local shadow",
    );
    accepts(
        "module t;
  logic [7:0] a = 8'h11;
  logic [7:0] y;
  assign y = a;
  initial begin
    for (logic [7:0] y = 0; y < 2; y++) $display(\"L y=%h\", y);
    #1 $display(\"M y=%h\", y);
    $finish;
  end
endmodule
",
        &["L y=00", "L y=01", "M y=11"],
        "for-loop variable",
    );
}

// ── assertion-action locals (ROADMAP §3.b mdrv-assert-local) ─────────────────────
//
// v1 flattens a block-local onto the module variable of the same name. A local declared
// in an assertion's action block is such a local, and the multidriver walks descend into
// action blocks (`stmt_writes_whole_ident`) while the shadow guard of Rules A and B does
// not (`declares_local_named(…, false)`). Rule D opts in to the descent; Rules A and B
// do not, because there the descent turns a false E3001 into a silent wrong value. These
// cells pin each lane as it is, PRE = POST.

#[test]
fn rule_d_lane_an_action_block_local_hides_the_process() {
    // Concurrent: verilator and sv2v → iverilog `A1 x=11` / `A0 x=22` (iverilog
    // `sorry: concurrent_assertion_item not supported.`). Without the opt-in this was
    // a false E3001.
    accepts(
        "module t;
  logic clk = 0;
  logic [7:0] a = 8'h11;
  logic [7:0] x;
  assign x = a;
  a1: assert property (@(posedge clk) 1'b0) else begin logic [7:0] x; x = '0; end
  initial begin
    #1 clk = 1; #1 clk = 0;
    #1 $display(\"A1 x=%h\", x);
    a = 8'h22;
    #1 $display(\"A0 x=%h\", x);
    $finish;
  end
endmodule
",
        &["A1 x=11", "A0 x=22"],
        "concurrent action local, assign",
    );
    // Deferred: the same lines (iverilog `sorry: Deferred assertions are not
    // supported.`); vita adds its W3056 for the inline action.
    let (rc, out) = run("module t;
  logic [7:0] a = 8'h11;
  logic [7:0] x;
  assign x = a;
  initial begin
    #1 assert final (a == 8'h00) else begin logic [7:0] x; x = '0; end
    #1 $display(\"A1 x=%h\", x);
    a = 8'h22;
    #1 $display(\"A0 x=%h\", x);
    $finish;
  end
endmodule
");
    assert_eq!(rc, Some(0), "{out}");
    expect_codes(&out, &["VITA-W3056"], "deferred action local, assign");
    assert_eq!(shown(&out), ["A1 x=11", "A0 x=22"], "{out}");
}

#[test]
fn rule_d_lane_the_flattened_local_still_leaks_at_a_finer_probe() {
    // KNOWN WRONG, pinned as it is (mdrv-assert-local): the action's local write lands
    // on the module `x` for the rest of the step. verilator `--binary` prints
    // `[0] chg x=11`, `L x=5a`, `S x=11`, `A1 x=11`, `[3] chg x=22`, `A0 x=22`: no
    // change of `x` at time 1. (vita's missing `[0] chg x=11` is the time-0 split, not
    // this defect; iverilog of sv2v prints no `[0]` line either.)
    let (rc, out) = run("module t;
  logic clk = 0;
  logic [7:0] a = 8'h11;
  logic [7:0] x;
  assign x = a;
  a1: assert property (@(posedge clk) 1'b0) else begin logic [7:0] x; x = 8'h5a; $display(\"L x=%h\", x); end
  always @(x) $display(\"[%0t] chg x=%h\", $time, x);
  initial begin
    #1 clk = 1; #0 $display(\"S x=%h\", x); #1 clk = 0;
    #1 $display(\"A1 x=%h\", x);
    a = 8'h22;
    #1 $display(\"A0 x=%h\", x);
    $finish;
  end
endmodule
");
    assert_eq!(rc, Some(0), "{out}");
    expect_codes(&out, &[], "concurrent, finer probe");
    assert_eq!(
        shown(&out),
        [
            "L x=5a",
            "[1] chg x=5a",
            "S x=11",
            "A1 x=11",
            "[3] chg x=22",
            "A0 x=22"
        ],
        "{out}"
    );
    // Deferred: the action reads the module value back (`L x=11`; verilator `L x=5a`).
    let (rc, out) = run("module t;
  logic [7:0] a = 8'h11;
  logic [7:0] x;
  assign x = a;
  always @(x) $display(\"[%0t] chg x=%h\", $time, x);
  initial begin
    #1 assert final (a == 8'h00) else begin logic [7:0] x; x = 8'h5a; $display(\"L x=%h\", x); end
    #0 $display(\"S x=%h\", x);
    #1 $display(\"A1 x=%h\", x);
    a = 8'h22;
    #1 $display(\"A0 x=%h\", x);
    $finish;
  end
endmodule
");
    assert_eq!(rc, Some(0), "{out}");
    expect_codes(&out, &[], "deferred, finer probe");
    assert_eq!(
        shown(&out),
        [
            "[1] chg x=5a",
            "S x=11",
            "L x=11",
            "A1 x=11",
            "[2] chg x=22",
            "A0 x=22"
        ],
        "{out}"
    );
}

#[test]
fn rules_a_b_lane_the_action_block_local_stays_as_it_was() {
    // `always_comb` beside the action-block local: a FALSE E3001 (Rule B) — verilator
    // and sv2v → iverilog print `A1 x=11` / `A0 x=22`. Kept loud: opting Rules A and B
    // into the descent would run it, and the `always @*` twin below shows what it would
    // then print.
    let comb = |assertion: &str| {
        format!(
            "module t;
  logic clk = 0;
  logic [7:0] a = 8'h11;
  logic [7:0] x;
  {assertion}
endmodule
"
        )
    };
    let (rc, out) = run(&comb(
        "always_comb x = a;
  a1: assert property (@(posedge clk) 1'b0) else begin logic [7:0] x; x = '0; end
  initial begin #1 clk = 1; #1 clk = 0; #1 $display(\"A1 x=%h\", x); a = 8'h22; #1 $display(\"A0 x=%h\", x); $finish; end",
    ));
    assert_eq!(rc, Some(1), "{out}");
    expect_codes(
        &out,
        &["VITA-E3001"],
        "always_comb, concurrent action local",
    );
    assert!(
        out.contains("variable `x` is written by `always_comb` AND by `initial`"),
        "{out}"
    );
    let (rc, out) = run(&comb(
        "always_comb x = a;
  initial begin #1 assert final (a == 8'h00) else begin logic [7:0] x; x = '0; end
    #1 $display(\"A1 x=%h\", x); a = 8'h22; #1 $display(\"A0 x=%h\", x); $finish; end",
    ));
    assert_eq!(rc, Some(1), "{out}");
    expect_codes(
        &out,
        &["VITA-E3001", "VITA-W3056"],
        "always_comb, deferred action local",
    );
    // KNOWN WRONG, pinned as it is: the `always @*` twin has no inferring procedure, so
    // no rule reports it, and the local's `'0` lands on the module `x`: `A1 x=00` where
    // verilator prints `A1 x=11` (sv2v → iverilog `A1 x=xx`) — the value a Rules A/B
    // opt-in would turn the E3001 above into.
    let (rc, out) = run(&comb(
        "always @* x = a;
  a1: assert property (@(posedge clk) 1'b0) else begin logic [7:0] x; x = '0; end
  initial begin #1 clk = 1; #1 clk = 0; #1 $display(\"A1 x=%h\", x); a = 8'h22; #1 $display(\"A0 x=%h\", x); $finish; end",
    ));
    assert_eq!(rc, Some(0), "{out}");
    expect_codes(&out, &[], "always @*, concurrent action local");
    assert_eq!(shown(&out), ["A1 x=00", "A0 x=22"], "{out}");
}
