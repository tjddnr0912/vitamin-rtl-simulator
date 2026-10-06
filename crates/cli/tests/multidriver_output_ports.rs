//! IEEE 1800 §9.2.2.2–§9.2.2.4 reach an OUTPUT PORT that is a variable exactly as
//! they reach a body variable — and vita checked only the body.
//!
//! §23.2.2.3: an `output` port written with an explicit data type and no net type
//! (`output logic y`, `output reg y`, `output int y`, a typedef / enum / struct type)
//! defaults to a VARIABLE. So `output logic y` written by two `always_comb` is the
//! same two-driver design as `logic y;` written by two `always_comb`. The body form
//! was `VITA-E3001`; the port form ran at exit 0, because the check collected only
//! body `NetVarDecl`s. Under a testbench (`md u1 (.y(y))` below) the port form
//! printed `y=1` where iverilog 13.0 prints `y=0`: the two writers resolve by order.
//!
//! Every port cell here is paired with its internal-variable twin, and the pair must
//! give the same code, severity and clause; only the message subject differs
//! (`output port `y` (a variable, IEEE §23.2.2.3)` vs `variable `y``).
//!
//! Oracles, measured one shape per file. iverilog 13.0 (`-g2012`) says nothing on
//! any of these and runs them. verilator 5.052 `--lint-only -Wall --timing`:
//!
//! ```text
//!   md_comb   output logic y; two always_comb
//!     %Warning-MULTIDRIVEN: md_comb.sv:3:17: Variable written to in always_comb also
//!       written by other process (IEEE 1800-2023 9.2.2.2): 'y'
//!   f_multi   output logic q; always_ff + always @(*)
//!     %Warning-MULTIDRIVEN: f_multi.sv:3:26: Variable also written to in always_ff
//!       (IEEE 1800-2023 9.2.2.4): 'q'
//!   md_star   output logic y; two always @(*)
//!     %Warning-MULTIDRIVENPROC: md_star.sv:3:17: Variable written to in always block
//!       also written by another always block: 'y'        (no 9.2.2.x clause)
//!   output logic y = 1'b0; always_comb y = a;
//!     %Warning-MULTIDRIVEN: p_init_comb.sv:2:15: Variable written to in always_comb
//!       also written by other process (IEEE 1800-2023 9.2.2.2): 'y'
//!   output logic [3:0] q = 4'd0; always_ff q <= d;
//!     %Warning-PROCASSINIT: p_init_ff.sv:1:71: Procedural assignment to declaration
//!       with initial value: 'q'                          (no MULTIDRIVEN)
//!   output logic y; two always_latch
//!     (nothing)
//!   output reg / int / bit [3:0] / enum / packed struct / typedef vector /
//!   `parameter type T` / continuation (`output logic z, y`) / non-ANSI `output reg y;`
//!   / non-ANSI `output logic y;`, each with two always_comb
//!     %Warning-MULTIDRIVEN: …: Variable written to in always_comb also written by
//!       other process (IEEE 1800-2023 9.2.2.2): 'y'
//! ```
//!
//! Each internal twin draws the same verilator line. That is the module doc table in
//! `elaborate/src/multidriver.rs`: MULTIDRIVEN ⇒ `VITA-E3001`; an initializer under
//! `always_ff`, or an `always_latch` pair, is xcelium-only (`*E,MULAXX`) ⇒ the warning
//! `VITA-W3060`; two plain `always` ⇒ silent.
//!
//! The controls are ports that are NETS, which the check does not reach. With two
//! continuous assigns (`output wire`, an implicit `output [1:0]`, `inout wire`, `inout
//! logic`), wire resolution settles them and both oracles run them. With two
//! `always_comb` writers (`output wire`, implicit `output`, `inout wire`) the write
//! itself is refused — vita `VITA-E3018`, iverilog `'y' is not a valid l-value for a
//! procedural assignment.` / `'y' is declared here as a wire.`, verilator
//! `%Warning-MULTIDRIVEN` — and the exact code set `{E3018}` is what shows the port was
//! not ALSO collected as a variable. `inout logic` and `input logic` written by two
//! `always_comb` are outside the slice and pinned as they are (exit 0, no code): vita
//! builds them as variables and the collector skips them by direction, while both
//! oracles refuse the write (iverilog the same l-value error; verilator MULTIDRIVEN on
//! the `inout`, `%Error-ASSIGNIN: Assigning to input/const variable: 'y'` on the
//! `input`).
//!
//! ## Unpacked-array output ports: E3001 by owner ruling (2026-10-06)
//!
//! `output logic [3:0] y [2]` (and non-ANSI `output reg [3:0] y [2];`) written WHOLE by
//! two `always_comb` is E3001, as its body twin `logic [3:0] y [2];` already was before
//! this check reached ports. Neither oracle rejects it, and the two disagree on the
//! value, which depends on which writer runs last:
//!
//! ```text
//!   iverilog 13.0 -g2012 + vvp            y0=3 y1=5
//!   verilator 5.052 --lint-only -Wall     %Warning-DECLFILENAME: …: Filename '…' does
//!                                         not match MODULE name: 'm'
//!                                         %Error: Exiting due to 1 warning(s)
//!                                         (no MULTIDRIVEN)
//!   verilator 5.052 --binary --timing     y0=5 y1=3
//!   vita before the port check            y0=5 y1=3, exit 0
//! ```
//!
//! A new reject of a design two simulators run is normally a stop. The owner ruled on
//! 2026-10-06 to keep E3001 here as an exception: IEEE 1800 §9.2.2.2 says the variables
//! an `always_comb` writes "shall not be written to by any other process", the body twin
//! is already E3001, and the oracles' own split shows the result is an ordering race.
//!
//! Every test counts the EXACT set of codes the run prints (`-Wno-W1017` silences the
//! no-`timescale` warning so that nothing is left out): a port whose NET is built as a
//! wire while the check calls it a variable prints `VITA-E3018` beside the expected
//! code, and that fails here.

use std::collections::BTreeSet;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run one design through one-shot `vita` with `-Wno-W1017` and return its exit code
/// and combined output.
fn run(src: &str, extra: &[&str]) -> (Option<i32>, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_mdop_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    let f = d.join("t.sv");
    std::fs::write(&f, src).expect("write design");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("-Wno-W1017")
        .args(extra)
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

/// The run printed exactly these codes and no other.
fn expect_codes(out: &str, want: &[&str], what: &str) {
    let want: BTreeSet<String> = want.iter().map(|c| c.to_string()).collect();
    assert_eq!(codes(out), want, "{what}: exact code set:\n{out}");
}

/// The multi-driver diagnostic lines (`VITA-E3001` / `VITA-W3060`) the run printed.
fn driver_lines(out: &str) -> Vec<&str> {
    out.lines()
        .filter(|l| l.contains("[VITA-E3001]") || l.contains("[VITA-W3060]"))
        .collect()
}

/// The port design and its internal-variable twin: same exit code, exactly the code
/// `code` and nothing else, one diagnostic line each, and each subject as it should
/// read. Returns the port's line.
fn twin(port: &str, internal: &str, name: &str, code: &str, rc: i32, tail: &str) -> String {
    let (prc, pout) = run(port, &[]);
    let (irc, iout) = run(internal, &[]);
    expect_codes(&pout, &[code], "port");
    expect_codes(&iout, &[code], "twin");
    let (pl, il) = (driver_lines(&pout), driver_lines(&iout));
    assert_eq!(pl.len(), 1, "port: exactly one driver diagnostic:\n{pout}");
    assert_eq!(il.len(), 1, "twin: exactly one driver diagnostic:\n{iout}");
    assert_eq!(prc, Some(rc), "port exit code:\n{pout}");
    assert_eq!(irc, Some(rc), "twin exit code:\n{iout}");
    let want_port = format!("output port `{name}` (a variable, IEEE §23.2.2.3) {tail}");
    let want_int = format!("variable `{name}` {tail}");
    assert!(
        pl[0].contains(&want_port),
        "port: wanted `{want_port}`:\n{pout}"
    );
    assert!(
        il[0].contains(&want_int),
        "twin: wanted `{want_int}`:\n{iout}"
    );
    pl[0].to_string()
}

/// No diagnostic at all, and exit 0.
fn expect_silent(src: &str, what: &str) {
    let (rc, out) = run(src, &[]);
    expect_codes(&out, &[], what);
    assert_eq!(rc, Some(0), "{what}:\n{out}");
}

// ── The reported designs, verbatim ───────────────────────────────────────────

#[test]
fn md_comb_two_always_comb_on_an_output_port_is_an_error() {
    let line = twin(
        "module md_comb (input logic a, b, output logic y, z);\n    \
         always_comb y = a & b;\n    always_comb y = a | b;\nendmodule\n",
        "module md_comb_int (input logic a, b, output logic o);\n    logic y;\n    \
         always_comb y = a & b;\n    always_comb y = a | b;\n    assign o = y;\nendmodule\n",
        "y",
        "VITA-E3001",
        1,
        "is written by `always_comb` AND by `always_comb`, which is two drivers on one \
         variable (IEEE §9.2.2.2)",
    );
    // At the first `always_comb`, as the twin is.
    assert!(line.starts_with("t.sv:2:5: error[VITA-E3001]"), "{line}");
}

#[test]
fn f_multi_always_ff_and_always_star_on_an_output_port_is_an_error() {
    twin(
        "module f_multi (input logic clk, d, rst, output logic q);\n    \
         always_ff @(posedge clk) q <= d;\n    always @(*) if (rst) q = 1'b0;\nendmodule\n",
        "module f_multi_int (input logic clk, d, rst, output logic o);\n    logic q;\n    \
         always_ff @(posedge clk) q <= d;\n    always @(*) if (rst) q = 1'b0;\n    \
         assign o = q;\nendmodule\n",
        "q",
        "VITA-E3001",
        1,
        "is written by `always_ff` AND by `always`, which is two drivers on one variable \
         (IEEE §9.2.2.4)",
    );
}

/// Two plain `always @(*)`: no §9.2.2.x clause reaches them (`always @*` "permits
/// multiple processes to write to the same variable", §9.2.2.2.2), so the port is as
/// silent as its twin always was.
#[test]
fn md_star_two_always_star_on_an_output_port_is_accepted() {
    expect_silent(
        "module md_star (input logic a, b, output logic y, z);\n    \
         always @(*) y = a & b;\n    always @(*) y = a | b;\nendmodule\n",
        "md_star",
    );
}

/// The port module under a testbench: the run stops in the instance, where it used
/// to print `y=1` (iverilog 13.0: `y=0`).
#[test]
fn an_instantiated_port_module_is_checked_per_instance() {
    let (rc, out) = run(
        "module md(input logic a, b, output logic y);\n  always_comb y = a & b;\n  \
         always_comb y = a | b;\nendmodule\nmodule tb;\n  logic a = 0, b = 1;\n  wire y;\n  \
         md u1 (.a(a), .b(b), .y(y));\n  initial begin #1 $display(\"y=%b\", y); $finish; \
         end\nendmodule\n",
        &["--top", "tb"],
    );
    assert_eq!(rc, Some(1), "{out}");
    expect_codes(&out, &["VITA-E3001"], "instantiated");
    let lines = driver_lines(&out);
    assert_eq!(lines.len(), 1, "{out}");
    assert!(lines[0].ends_with("[in tb.u1]"), "{out}");
    assert!(!out.contains("y="), "no value is printed:\n{out}");
}

// ── The port-shape matrix, each against its internal twin ────────────────────

#[test]
fn port_initializer_and_always_comb_is_an_error_like_its_twin() {
    let line = twin(
        "module t(input logic a, output logic y = 1'b0);\n  always_comb y = a;\nendmodule\n",
        "module t(input logic a, output logic o);\n  logic y = 1'b0;\n  always_comb y = a;\n  \
         assign o = y;\nendmodule\n",
        "y",
        "VITA-E3001",
        1,
        "has a declaration initializer AND is written by `always_comb`, which is two \
         drivers on one variable (IEEE §9.2.2.2)",
    );
    // At the port declaration, as the twin's is at its declaration.
    assert!(line.starts_with("t.sv:1:25: error[VITA-E3001]"), "{line}");
}

#[test]
fn port_initializer_and_always_ff_is_a_warning_like_its_twin() {
    twin(
        "module t(input logic clk, input logic [3:0] d, output logic [3:0] q = 4'd0);\n  \
         always_ff @(posedge clk) q <= d;\nendmodule\n",
        "module t(input logic clk, input logic [3:0] d, output logic [3:0] o);\n  \
         logic [3:0] q = 4'd0;\n  always_ff @(posedge clk) q <= d;\n  assign o = q;\n\
         endmodule\n",
        "q",
        "VITA-W3060",
        0,
        "has a declaration initializer AND is written by `always_ff`; xcelium rejects this \
         as two drivers (*E,MULAXX, IEEE §9.2.2.4)",
    );
}

#[test]
fn always_latch_pair_on_an_output_port_is_a_warning_like_its_twin() {
    twin(
        "module t(input logic en, d, e, output logic y);\n  always_latch if (en) y = d;\n  \
         always_latch if (!en) y = e;\nendmodule\n",
        "module t(input logic en, d, e, output logic o);\n  logic y;\n  \
         always_latch if (en) y = d;\n  always_latch if (!en) y = e;\n  assign o = y;\n\
         endmodule\n",
        "y",
        "VITA-W3060",
        0,
        "is written by `always_latch` AND by `always_latch`; xcelium rejects this as two \
         drivers (*E,MULAXX, IEEE §9.2.2.3)",
    );
}

/// Every explicit-data-type spelling is a variable, classified by the kind the port's
/// net is built with (`port_net_kind`) — the typedef, enum, struct and type-parameter
/// ports included. Each is E3001 with two `always_comb`, as verilator's MULTIDRIVEN.
#[test]
fn every_explicit_data_type_output_port_is_a_variable() {
    let comb2 = "  always_comb y = a & b;\n  always_comb y = a | b;\nendmodule\n";
    let cases: Vec<(&str, String)> = vec![
        (
            "reg",
            format!("module t(input logic a, b, output reg y);\n{comb2}"),
        ),
        (
            "int",
            format!("module t(input int a, b, output int y);\n{comb2}"),
        ),
        (
            "bit vector",
            format!("module t(input bit [3:0] a, b, output bit [3:0] y);\n{comb2}"),
        ),
        (
            "typedef vector",
            format!("typedef logic [3:0] v_t;\nmodule t(input v_t a, b, output v_t y);\n{comb2}"),
        ),
        (
            "packed struct",
            "typedef struct packed { logic p; logic q; } s_t;\n\
             module t(input logic a, b, output s_t y);\n  always_comb y = {a, b};\n  \
             always_comb y = {b, a};\nendmodule\n"
                .to_string(),
        ),
        (
            "enum",
            "typedef enum logic [1:0] {A, B, C} e_t;\n\
             module t(input logic a, output e_t y);\n  always_comb y = A;\n  \
             always_comb y = C;\nendmodule\n"
                .to_string(),
        ),
        (
            "type parameter",
            format!(
                "module t #(parameter type T = logic [3:0]) (input T a, b, output T y);\n{comb2}"
            ),
        ),
        (
            "continuation",
            format!("module t(input logic a, b, output logic z, y);\n  assign z = a;\n{comb2}"),
        ),
        (
            "non-ANSI output reg",
            format!("module t(a, b, y);\n  input a, b;\n  output reg y;\n{comb2}"),
        ),
        (
            "non-ANSI output logic",
            format!("module t(a, b, y);\n  input logic a, b;\n  output logic y;\n{comb2}"),
        ),
    ];
    for (what, src) in cases {
        let (rc, out) = run(&src, &[]);
        expect_codes(&out, &["VITA-E3001"], what);
        let lines = driver_lines(&out);
        assert_eq!(rc, Some(1), "{what}:\n{out}");
        assert_eq!(lines.len(), 1, "{what}:\n{out}");
        assert!(
            lines[0].contains("[VITA-E3001]")
                && lines[0].contains(
                    "output port `y` (a variable, IEEE §23.2.2.3) is written by \
                     `always_comb` AND by `always_comb`"
                )
                && lines[0].contains("(IEEE §9.2.2.2)"),
            "{what}:\n{out}"
        );
    }
}

/// The split non-ANSI form (`output y; reg y;`) was already checked through its `reg`
/// declaration; it is still ONE diagnostic, about the variable.
#[test]
fn the_split_non_ansi_form_reports_once() {
    let (rc, out) = run(
        "module t(a, b, y);\n  input a, b;\n  output y;\n  reg y;\n  always_comb y = a & b;\n  \
         always_comb y = a | b;\nendmodule\n",
        &[],
    );
    assert_eq!(rc, Some(1), "{out}");
    expect_codes(&out, &["VITA-E3001"], "split");
    let lines = driver_lines(&out);
    assert_eq!(lines.len(), 1, "{out}");
    assert!(lines[0].contains("variable `y` is written by"), "{out}");
}

// ── More writer pairs on output ports ────────────────────────────────────────

/// A continuous `assign` and an `always_comb` on one output port. Both oracles reject
/// it: iverilog 13.0 `t.sv:3: error: Cannot perform procedural assignment to variable
/// 'y' because it is also continuously assigned.` / `Elaboration failed`; verilator
/// 5.052 `%Warning-MULTIDRIVEN: t.sv:3:15: Variable written to in always_comb also
/// written by other process (IEEE 1800-2023 9.2.2.2): 'y'`. Before the port check vita
/// ran it at exit 0; the body twin was already E3001. (`assign` + `always @*` on a
/// port stays exit 0, as before: no §9.2.2.x clause reaches a plain `always`.)
#[test]
fn a_continuous_assign_and_always_comb_on_an_output_port_is_an_error() {
    twin(
        "module t(input logic a, b, output logic y);\n  assign y = a;\n  \
         always_comb y = b;\nendmodule\n",
        "module t(input logic a, b, output logic o);\n  logic y;\n  assign y = a;\n  \
         always_comb y = b;\n  assign o = y;\nendmodule\n",
        "y",
        "VITA-E3001",
        1,
        "is written by `always_comb` AND by a continuous `assign`, which is two drivers on \
         one variable (IEEE §9.2.2.2)",
    );
}

/// Owner ruling 2026-10-06 (module doc): an unpacked-array output port written whole by
/// two `always_comb` is E3001, as its body twin. Oracles on this design (the testbench
/// reads `y` at #1): iverilog 13.0 `y0=3 y1=5`; verilator 5.052 `--binary` `y0=5 y1=3`;
/// verilator `--lint-only -Wall` `%Warning-DECLFILENAME: …: Filename '…' does not match
/// MODULE name: 'm'` / `%Error: Exiting due to 1 warning(s)`, no MULTIDRIVEN. vita
/// before the port check: `y0=5 y1=3`, exit 0.
#[test]
fn an_unpacked_array_output_port_written_whole_twice_is_an_error_by_owner_ruling() {
    let tb = "module tb;\n  logic [3:0] a = 4'd3, c = 4'd5;\n  logic [3:0] y [2];\n  \
              m u(.a(a), .c(c), .y(y));\n  initial begin #1 $display(\"y0=%0d y1=%0d\", y[0], \
              y[1]); #100 $finish; end\nendmodule\n";
    let line = twin(
        &format!(
            "module m(input logic [3:0] a, c, output logic [3:0] y [2]);\n  \
             always_comb y = '{{a, c}};\n  always_comb y = '{{c, a}};\nendmodule\n{tb}"
        ),
        &format!(
            "module m(input logic [3:0] a, c, output logic [3:0] yo [2]);\n  \
             logic [3:0] y [2];\n  always_comb y = '{{a, c}};\n  always_comb y = '{{c, a}};\n  \
             assign yo = y;\nendmodule\n{}",
            tb.replace(".y(y)", ".yo(y)")
        ),
        "y",
        "VITA-E3001",
        1,
        "is written by `always_comb` AND by `always_comb`, which is two drivers on one \
         variable (IEEE §9.2.2.2)",
    );
    assert!(line.ends_with("[in tb.u]"), "{line}");
}

/// The non-ANSI spelling of the same ruling: `output reg [3:0] y [2];`. Oracles:
/// iverilog 13.0 `y0=3 y1=5`; verilator 5.052 `--binary` `y0=5 y1=3`; verilator
/// `--lint-only -Wall` `%Warning-DECLFILENAME: …: Filename '…' does not match MODULE
/// name: 'm'` / `%Error: Exiting due to 1 warning(s)`, no MULTIDRIVEN. vita before the
/// port check: `y0=5 y1=3`, exit 0.
#[test]
fn a_non_ansi_unpacked_array_output_reg_written_whole_twice_is_an_error_by_owner_ruling() {
    let (rc, out) = run(
        "module m(a, c, y);\n  input [3:0] a, c;\n  output reg [3:0] y [2];\n  \
         always_comb y = '{a, c};\n  always_comb y = '{c, a};\nendmodule\n\
         module tb;\n  logic [3:0] a = 4'd3, c = 4'd5;\n  logic [3:0] y [2];\n  \
         m u(.a(a), .c(c), .y(y));\n  initial begin #1 $display(\"y0=%0d y1=%0d\", y[0], \
         y[1]); #100 $finish; end\nendmodule\n",
        &[],
    );
    assert_eq!(rc, Some(1), "{out}");
    expect_codes(&out, &["VITA-E3001"], "non-ANSI unpacked");
    let lines = driver_lines(&out);
    assert_eq!(lines.len(), 1, "{out}");
    assert!(
        lines[0].starts_with("t.sv:4:3: error[VITA-E3001]")
            && lines[0].contains(
                "output port `y` (a variable, IEEE §23.2.2.3) is written by `always_comb` AND \
                 by `always_comb`, which is two drivers on one variable (IEEE §9.2.2.2)"
            )
            && lines[0].ends_with("[in tb.u]"),
        "{out}"
    );
    assert!(!out.contains("y0="), "no value is printed:\n{out}");
}

// ── Controls: ports that are nets, and ports the check does not reach ────────

/// Two continuous assigns: legal on a net, settled by wire resolution, run by both
/// oracles.
#[test]
fn net_ports_with_two_continuous_assigns_stay_silent() {
    for (what, port) in [
        ("output wire", "output wire y"),
        ("implicit output", "output [1:0] y"),
        ("inout wire", "inout wire [1:0] y"),
        ("inout logic", "inout logic [1:0] y"),
    ] {
        expect_silent(
            &format!(
                "module t(input logic a, b, en, {port});\n  \
                 assign y = en ? {{a, b}} : 2'bzz;\n  assign y = en ? 2'bzz : {{b, a}};\n\
                 endmodule\n"
            ),
            what,
        );
    }
}

/// Two `always_comb` on a port that is a NET: the write itself is E3018, and the exact
/// set `{E3018}` shows the port was not also collected as a variable (module doc).
#[test]
fn net_ports_with_two_always_comb_are_e3018_only() {
    for (what, port) in [
        ("output wire", "output wire y"),
        ("implicit output", "output y"),
        ("inout wire", "inout wire y"),
    ] {
        let (rc, out) = run(
            &format!(
                "module t(input logic a, b, {port});\n  always_comb y = a & b;\n  \
                 always_comb y = a | b;\nendmodule\n"
            ),
            &[],
        );
        expect_codes(&out, &["VITA-E3018"], what);
        assert_eq!(rc, Some(1), "{what}:\n{out}");
    }
}

/// Outside the slice, pinned as it is: an `inout logic` or `input logic` port written by
/// two `always_comb` runs at exit 0 with no diagnostic. vita builds both as variables and
/// the check skips them by direction; both oracles refuse the write (module doc).
#[test]
fn inout_and_input_logic_ports_get_no_multidriver_diagnostic() {
    for (what, port) in [
        ("inout logic", "inout logic y"),
        ("input logic", "input logic y"),
    ] {
        expect_silent(
            &format!(
                "module t(input logic a, b, {port});\n  always_comb y = a & b;\n  \
                 always_comb y = a | b;\nendmodule\n"
            ),
            what,
        );
    }
}
