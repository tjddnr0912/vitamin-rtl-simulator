//! A process-header LEVEL list that names a constant (ROADMAP_ARCHIVE §4.5.529; the
//! residues are ROADMAP §2 Delays/events): `always @(K)`, `always @(K or clk)`,
//! `always @(K[0] or clk)`, `always @(p::C or clk)`.
//!
//! The header is armed before time 0, so the time-0 settle hands every constant term
//! a change: the process runs ONCE at time 0 (after the `initial` statements of time
//! 0) and then on every change of its live terms. vita refused the all-constant list
//! (E3009) and, beside a live term, DROPPED the constant together with its time-0 run
//! — at exit 0. `const_level_header.rs` now keeps the header process and replaces
//! the constant terms with one edge on an internal time-0 pulse net; an all-constant
//! list is the pulse alone (admitted since a `$finish` ends the run at the end of its
//! time step, so the one run survives a `$finish` reaching time 0).
//!
//! Every expected text below is iverilog 13.0 (`-g2012`) and verilator 5.052
//! (`--binary --timing`) printing the same lines, unless a test says otherwise. The
//! common deviations, never a disagreement about WHEN the process runs:
//! - an uninitialized `reg clk` prints `x` in iverilog and `0` in 2-state verilator
//!   (vita is 4-state and prints iverilog's `x`);
//! - several processes woken in one time step print in a different order in the two
//!   oracles; those cells compare the sorted lines.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_clet0_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let mut all = String::from_utf8_lossy(&out.stdout).into_owned();
    all.push_str(&String::from_utf8_lossy(&out.stderr));
    let mut s = String::new();
    for l in all.lines().filter(|l| {
        !l.starts_with("simulation ended")
            && !l.starts_with("errors=")
            && !l.contains("W-PP-TIMESCALE-DEFAULT")
    }) {
        s.push_str(l);
        s.push('\n');
    }
    (s, out.status.success())
}

fn run(src: &str) -> String {
    let (s, ok) = vita(src);
    assert!(ok, "expected exit 0, got:\n{s}");
    s
}

fn loud(src: &str, needle: &str) {
    let (s, ok) = vita(src);
    assert!(!ok, "expected a loud reject, got exit 0:\n{s}");
    assert!(s.contains(needle), "expected `{needle}` in:\n{s}");
}

fn expect(src: &str, want: &str) {
    assert_eq!(run(src), want, "design:\n{src}");
}

fn check(cells: &[(&str, &str)]) {
    for (src, want) in cells {
        expect(src, want);
    }
}

/// Order-free comparison for cells whose oracles print one time step in two orders.
fn check_sorted(src: &str, want: &str) {
    let mut got: Vec<String> = run(src).lines().map(str::to_string).collect();
    let mut want: Vec<String> = want.lines().map(str::to_string).collect();
    got.sort();
    want.sort();
    assert_eq!(got, want, "design:\n{src}");
}

/// `module top;` + `decls` + one `always` + a `#5` watchdog.
fn alone(decls: &str, always: &str) -> String {
    format!(
        "{decls}module top;\n{always}\n  initial begin #5 $display(\"DONE\"); $finish; end\nendmodule\n"
    )
}

/// `module top;` + `decls` + one `always` beside an uninitialized `reg clk` rising at 1
/// and falling at 2.
fn mixed(decls: &str, always: &str) -> String {
    format!(
        "{decls}module top;\n  reg clk;\n{always}\n  \
         initial begin #1 clk = 1; #1 clk = 0; #3 $display(\"DONE\"); $finish; end\nendmodule\n"
    )
}

const MIX: &str = "MIX at 0 clk=x\nMIX at 1 clk=1\nMIX at 2 clk=0\nDONE\n";

/// The shared package of the `p::C` cells, and its `clk` timeline (0, 1@1, 0@2, 1@3).
const PKG: &str =
    "package p; localparam int C = 5; typedef enum {E0, E1} e_t; logic v; endpackage\n";

fn pkg_cell(pkg: &str, body: &str) -> String {
    format!(
        "{pkg}module top;\n  reg clk = 0; initial begin #1 clk = 1; #1 clk = 0; #1 clk = 1; end\n\
         {body}\n  initial begin #5 $display(\"DONE at %0t\", $time); $finish; end\nendmodule\n"
    )
}

// ── every constant kind, alone ──────────────────────────────────────────────────

/// A list of constants only. Both oracles run the process once at time 0 and never
/// again (text per cell below), and so does vita: the list's sensitivity is the
/// time-0 pulse alone. Admitted since a `$finish` ends the run at the END of its time
/// step, so a `$finish` reaching time 0 no longer erases the run
/// (`const_level_event_order.rs` measures the channels).
#[test]
fn a_constant_alone_runs_once_at_time_zero() {
    let k = "  localparam int K = 99;\n";
    let cells = [
        // g1 a01: `LVL at 0 K=99` / `DONE`
        (
            alone("", &format!("{k}  always @(K) $display(\"LVL at %0t K=%0d\", $time, K);")),
            "LVL at 0 K=99\nDONE\n",
        ),
        // g1 a02: a constant whose value is 0 still runs
        (
            alone("", "  localparam int Z = 0;\n  always @(Z) $display(\"LVL at %0t Z=%0d\", $time, Z);"),
            "LVL at 0 Z=0\nDONE\n",
        ),
        // g1 a03: `bit`
        (
            alone("", "  localparam bit B = 1'b1;\n  always @(B) $display(\"LVL at %0t B=%0d\", $time, B);"),
            "LVL at 0 B=1\nDONE\n",
        ),
        // g1 a04: `parameter`
        (
            alone("", "  parameter int P = 7;\n  always @(P) $display(\"LVL at %0t P=%0d\", $time, P);"),
            "LVL at 0 P=7\nDONE\n",
        ),
        // g1 a06: enum label
        (
            alone(
                "",
                "  typedef enum int {EA = 0, EB = 5} e_t;\n  always @(EB) $display(\"LVL at %0t EB=%0d\", $time, EB);",
            ),
            "LVL at 0 EB=5\nDONE\n",
        ),
        // g1 a08: generate-block localparam
        (
            alone(
                "",
                "  if (1) begin : gb\n    localparam int L = 5;\n    always @(L) $display(\"GB %m at %0t L=%0d\", $time, L);\n  end",
            ),
            "GB top.gb at 0 L=5\nDONE\n",
        ),
        // g1 a09: `$unit` localparam
        (
            alone(
                "localparam int CU = 4;\n",
                "  always @(CU) $display(\"LVL at %0t CU=%0d\", $time, CU);",
            ),
            "LVL at 0 CU=4\nDONE\n",
        ),
        // g1 a10: real
        (
            alone("", "  localparam real R = 1.5;\n  always @(R) $display(\"LVL at %0t R=%0.2f\", $time, R);"),
            "LVL at 0 R=1.50\nDONE\n",
        ),
        // g1 a11: string
        (
            alone("", "  parameter string S = \"hi\";\n  always @(S) $display(\"LVL at %0t S=%s\", $time, S);"),
            "LVL at 0 S=hi\nDONE\n",
        ),
        // g1 b07: two constants, no live term: `MIX at 0 clk=x` (verilator `clk=0`) / `DONE`
        (
            mixed(
                "",
                "  localparam int K = 99;\n  localparam int K2 = 3;\n  always @(K or K2) $display(\"MIX at %0t clk=%b\", $time, clk);",
            ),
            "MIX at 0 clk=x\nDONE\n",
        ),
    ];
    let cells: Vec<(&str, &str)> = cells.iter().map(|(s, w)| (s.as_str(), *w)).collect();
    check(&cells);
}

/// g1 a05 / a07: an all-constant list in each instance / generate copy. Both oracles
/// run each copy once at time 0 (`C top.u0 at 0 P=3` / `C top.u1 at 0 P=1`, verilator
/// in the other order; `G top.gl[0] at 0 g=0` / `G top.gl[1] at 0 g=1`).
#[test]
fn a_constant_alone_in_each_copy_runs_each_copy_once() {
    expect(
        "module child #(parameter int P = 1);\n\
           always @(P) $display(\"C %m at %0t P=%0d\", $time, P);\n\
         endmodule\n\
         module top;\n  child #(.P(3)) u0();\n  child u1();\n\
           initial begin #5 $display(\"DONE\"); $finish; end\nendmodule\n",
        "C top.u0 at 0 P=3\nC top.u1 at 0 P=1\nDONE\n",
    );
    expect(
        &alone(
            "",
            "  for (genvar g = 0; g < 2; g++) begin : gl\n    always @(g) $display(\"G %m at %0t g=%0d\", $time, g);\n  end",
        ),
        "G top.gl[0] at 0 g=0\nG top.gl[1] at 0 g=1\nDONE\n",
    );
}

// ── beside a live term ──────────────────────────────────────────────────────────

/// The silent-wrong: vita printed `MIX at 1` / `MIX at 2` only. Every cell: iverilog
/// `MIX at 0 clk=x` / 1 / 2 / `DONE`, verilator the same with `clk=0` at 0.
#[test]
fn a_constant_beside_a_live_term_adds_the_time_zero_run() {
    let k = "  localparam int K = 99;\n";
    let cells = [
        // g1 b01
        mixed("", &format!("{k}  always @(K or clk) $display(\"MIX at %0t clk=%b\", $time, clk);")),
        // g1 b05: the constant written second
        mixed("", &format!("{k}  always @(clk or K) $display(\"MIX at %0t clk=%b\", $time, clk);")),
        // g1 b06: comma list
        mixed("", &format!("{k}  always @(K, clk) $display(\"MIX at %0t clk=%b\", $time, clk);")),
        // g1 b08: two constants and a live term
        mixed(
            "",
            &format!("{k}  localparam int K2 = 3;\n  always @(K or K2 or clk) $display(\"MIX at %0t clk=%b\", $time, clk);"),
        ),
        // g1 b12: enum label
        mixed(
            "",
            "  typedef enum int {EA = 0, EB = 5} e_t;\n  always @(EB or clk) $display(\"MIX at %0t clk=%b\", $time, clk);",
        ),
        // g1 b14: real
        mixed(
            "",
            "  localparam real R = 1.5;\n  always @(R or clk) $display(\"MIX at %0t clk=%b\", $time, clk);",
        ),
        // g1 b15: `$unit`
        mixed(
            "localparam int CU = 4;\n",
            "  always @(CU or clk) $display(\"MIX at %0t clk=%b\", $time, clk);",
        ),
    ];
    let cells: Vec<(&str, &str)> = cells.iter().map(|s| (s.as_str(), MIX)).collect();
    check(&cells);
    // g1 b03: `reg clk = 0;` — both oracles print `MIX at 0 clk=0`.
    check(&[(
        "module top;\n  localparam int K = 99;\n  reg clk = 0;\n\
           always @(K or clk) $display(\"MIX at %0t clk=%b\", $time, clk);\n\
           initial begin #1 clk = 1; #1 clk = 0; #3 $display(\"DONE\"); $finish; end\nendmodule\n",
        "MIX at 0 clk=0\nMIX at 1 clk=1\nMIX at 2 clk=0\nDONE\n",
    )]);
    // g1 b04: the live term is a continuously assigned wire (iverilog `clk=x` at 0,
    // verilator `clk=0`).
    check(&[(
        "module top;\n  localparam int K = 99;\n  reg r;\n  wire clk;\n  assign clk = r;\n\
           always @(K or clk) $display(\"MIX at %0t clk=%b\", $time, clk);\n\
           initial begin #1 r = 1; #1 r = 0; #3 $display(\"DONE\"); $finish; end\nendmodule\n",
        MIX,
    )]);
}

/// g1 b13 / b16: a genvar and a per-instance parameter beside a live term. Each copy
/// runs at 0, 1 and 2 in both oracles, in a different order within a step.
#[test]
fn a_genvar_or_override_beside_a_live_term_runs_each_copy() {
    check_sorted(
        &mixed(
            "",
            "  for (genvar g = 0; g < 2; g++) begin : gl\n    always @(g or clk) $display(\"MIX %m at %0t clk=%b\", $time, clk);\n  end",
        ),
        "MIX top.gl[0] at 0 clk=x\nMIX top.gl[1] at 0 clk=x\nMIX top.gl[1] at 1 clk=1\n\
         MIX top.gl[0] at 1 clk=1\nMIX top.gl[1] at 2 clk=0\nMIX top.gl[0] at 2 clk=0\nDONE\n",
    );
    check_sorted(
        "module child #(parameter int P = 1) (input clk);\n\
           always @(P or clk) $display(\"MIX %m at %0t clk=%b P=%0d\", $time, clk, P);\n\
         endmodule\n\
         module top;\n  reg clk;\n  child #(.P(3)) u0(clk);\n  child u1(clk);\n\
           initial begin #1 clk = 1; #1 clk = 0; #3 $display(\"DONE\"); $finish; end\nendmodule\n",
        "MIX top.u0 at 0 clk=x P=3\nMIX top.u1 at 0 clk=x P=1\nMIX top.u1 at 1 clk=1 P=1\n\
         MIX top.u0 at 1 clk=1 P=3\nMIX top.u1 at 2 clk=0 P=1\nMIX top.u0 at 2 clk=0 P=3\nDONE\n",
    );
}

// ── constant selects (a constant head) ───────────────────────────────────────

/// A select of a constant is still a constant. Was E3009 ("single-bit level" or
/// "bare signal name") in every cell. Beside a live term it runs at time 0, and alone
/// it runs once at time 0 (the pulse is its whole sensitivity).
#[test]
fn a_select_of_a_constant_runs_once_at_time_zero() {
    let k = "  localparam int K = 99;\n";
    let cells = [
        // g2 a01 `K[0]`, a02 `K[2]` (bit value 0), a03 `K[3:0]`, a04 `K[0+:2]`
        (alone("", &format!("{k}  always @(K[0]) $display(\"A01 at %0t\", $time);")), "A01 at 0\nDONE\n"),
        (alone("", &format!("{k}  always @(K[2]) $display(\"A02 at %0t\", $time);")), "A02 at 0\nDONE\n"),
        (alone("", &format!("{k}  always @(K[3:0]) $display(\"A03 at %0t\", $time);")), "A03 at 0\nDONE\n"),
        (alone("", &format!("{k}  always @(K[0+:2]) $display(\"A04 at %0t\", $time);")), "A04 at 0\nDONE\n"),
        // g2 a08: `K[g]` under a generate loop
        (
            alone(
                "",
                &format!(
                    "{k}  genvar g;\n  for (g = 0; g < 3; g = g + 1) begin : G\n    always @(K[g]) $display(\"A08 g=%0d at %0t\", g, $time);\n  end"
                ),
            ),
            "A08 g=0 at 0\nA08 g=1 at 0\nA08 g=2 at 0\nDONE\n",
        ),
        // g2 a05 `@(K[0] or clk)` and a06 `@(clk or K[2])` (verilator `clk=0` at 0)
        (
            mixed("", &format!("{k}  always @(K[0] or clk) $display(\"A05 at %0t clk=%b\", $time, clk);")),
            "A05 at 0 clk=x\nA05 at 1 clk=1\nA05 at 2 clk=0\nDONE\n",
        ),
        (
            mixed("", &format!("{k}  always @(clk or K[2]) $display(\"A06 at %0t clk=%b\", $time, clk);")),
            "A06 at 0 clk=x\nA06 at 1 clk=1\nA06 at 2 clk=0\nDONE\n",
        ),
    ];
    let cells: Vec<(&str, &str)> = cells.iter().map(|(s, w)| (s.as_str(), *w)).collect();
    check(&cells);
    // g2 a09: `@(K[g] or clk)` under a generate loop — every copy at 0, 1, 2.
    check_sorted(
        &mixed(
            "",
            &format!(
                "{k}  genvar g;\n  for (g = 0; g < 3; g = g + 1) begin : G\n    always @(K[g] or clk) $display(\"A09 g=%0d at %0t\", g, $time);\n  end"
            ),
        ),
        "A09 g=0 at 0\nA09 g=1 at 0\nA09 g=2 at 0\nA09 g=2 at 1\nA09 g=1 at 1\nA09 g=0 at 1\n\
         A09 g=2 at 2\nA09 g=1 at 2\nA09 g=0 at 2\nDONE\n",
    );
}

// ── package constants ─────────────────────────────────────────────────────

/// `p::C` / `p::E1` / a package `parameter` in every lane. Was E3009 "must name a
/// package variable" in every cell; both oracles treat it like a local constant, alone
/// and beside a live term.
#[test]
fn a_package_constant_is_a_constant_in_every_lane() {
    let pp = "package p; parameter int C = 5; endpackage\n";
    let cells = [
        // g3 a01 header level alone, a12 enum label, a16 package `parameter`
        // both oracles: LVL at 0 / DONE at 5
        (pkg_cell(PKG, "  always @(p::C) $display(\"LVL at %0t\", $time);"), "LVL at 0\nDONE at 5\n"),
        // both oracles: EN at 0 / DONE at 5
        (pkg_cell(PKG, "  always @(p::E1) $display(\"EN at %0t\", $time);"), "EN at 0\nDONE at 5\n"),
        // both oracles: PP at 0 / DONE at 5
        (pkg_cell(pp, "  always @(p::C) $display(\"PP at %0t\", $time);"), "PP at 0\nDONE at 5\n"),
        // g3 a02 / a16b header level beside a live term
        (
            pkg_cell(PKG, "  always @(p::C or clk) $display(\"MIX at %0t clk=%b\", $time, clk);"),
            "MIX at 0 clk=0\nMIX at 1 clk=1\nMIX at 2 clk=0\nMIX at 3 clk=1\nDONE at 5\n",
        ),
        (
            pkg_cell(pp, "  always @(p::C or clk) $display(\"PPM at %0t clk=%b\", $time, clk);"),
            "PPM at 0 clk=0\nPPM at 1 clk=1\nPPM at 2 clk=0\nPPM at 3 clk=1\nDONE at 5\n",
        ),
        // g3 a11b a select of it beside a live term
        (
            pkg_cell(PKG, "  always @(p::C[0] or clk) $display(\"BSM at %0t clk=%b\", $time, clk);"),
            "BSM at 0 clk=0\nBSM at 1 clk=1\nBSM at 2 clk=0\nBSM at 3 clk=1\nDONE at 5\n",
        ),
        // g3 a03 header edge alone (never), a04 header edge beside a live edge
        (pkg_cell(PKG, "  always @(posedge p::C) $display(\"PE at %0t\", $time);"), "DONE at 5\n"),
        (
            pkg_cell(PKG, "  always @(posedge p::C or posedge clk) $display(\"PEM at %0t clk=%b\", $time, clk);"),
            "PEM at 1 clk=1\nPEM at 3 clk=1\nDONE at 5\n",
        ),
        // g3 a05 / a06 / a11 / a12b / a16c in-body: never wakes
        (pkg_cell(PKG, "  initial begin @(p::C) $display(\"IL at %0t\", $time); end"), "DONE at 5\n"),
        (pkg_cell(PKG, "  initial begin @(posedge p::C) $display(\"IE at %0t\", $time); end"), "DONE at 5\n"),
        (pkg_cell(PKG, "  initial begin @(p::C[0]) $display(\"BS at %0t\", $time); end"), "DONE at 5\n"),
        (pkg_cell(PKG, "  initial begin @(p::E1) $display(\"ENI at %0t\", $time); end"), "DONE at 5\n"),
        (pkg_cell(pp, "  initial begin @(p::C) $display(\"PPI at %0t\", $time); end"), "DONE at 5\n"),
        // g3 a05b / a06b in-body beside a live term: the live term wakes it
        (
            pkg_cell(PKG, "  initial begin @(p::C or clk) $display(\"ILM at %0t clk=%b\", $time, clk); end"),
            "ILM at 1 clk=1\nDONE at 5\n",
        ),
        (
            pkg_cell(PKG, "  initial begin @(posedge p::C or posedge clk) $display(\"IEM at %0t clk=%b\", $time, clk); end"),
            "IEM at 1 clk=1\nDONE at 5\n",
        ),
        // g3 a08 static task, a09 automatic task, a10 fork branch: never wakes
        (
            pkg_cell(
                PKG,
                "  task t; @(p::C) $display(\"T at %0t\", $time); endtask\n  initial begin t; $display(\"after t at %0t\", $time); end",
            ),
            "DONE at 5\n",
        ),
        (
            pkg_cell(
                PKG,
                "  task automatic ta; @(p::C) $display(\"TA at %0t\", $time); endtask\n  initial begin ta; $display(\"after ta at %0t\", $time); end",
            ),
            "DONE at 5\n",
        ),
        (
            pkg_cell(
                PKG,
                "  initial begin fork begin @(p::C) $display(\"F1 at %0t\", $time); end begin #1 $display(\"F2 at %0t\", $time); end join $display(\"JOIN at %0t\", $time); end",
            ),
            "F2 at 1\nDONE at 5\n",
        ),
    ];
    let cells: Vec<(&str, &str)> = cells.iter().map(|(s, w)| (s.as_str(), *w)).collect();
    check(&cells);
    // g3 a15: `@(p::C or p::v)` with `import p::v` written at 1, 2, 3. iverilog
    // `CV at 0 v=x`, verilator `CV at 0 v=0`; both then 1 / 2 / 3.
    check(&[(
        pkg_cell(
            PKG,
            "  import p::v;\n  always @(p::C or p::v) $display(\"CV at %0t v=%b\", $time, p::v);\n  initial begin #1 v = 1; #1 v = 0; #1 v = 1; end",
        )
        .as_str(),
        "CV at 0 v=x\nCV at 1 v=1\nCV at 2 v=0\nCV at 3 v=1\nDONE at 5\n",
    )]);
}

/// g3 a13 / a13b / a13f / a14 / a14b: an imported constant. The live cells lost `at
/// 0` at exit 0 and now run it; the alone cells (both oracles `IS at 0` / `IN at 0`,
/// then `DONE at 5`) run once at time 0.
#[test]
fn an_imported_constant_runs_once_at_time_zero() {
    let live = "at 0 clk=0\n{} at 1 clk=1\n{} at 2 clk=0\n{} at 3 clk=1\nDONE at 5\n";
    let lv = |tag: &str| format!("{tag} {}", live.replace("{}", tag));
    let cells = [
        (
            pkg_cell(
                PKG,
                "  import p::*; always @(C) $display(\"IS at %0t\", $time);",
            ),
            "IS at 0\nDONE at 5\n".to_string(),
        ),
        (
            pkg_cell(
                PKG,
                "  import p::*; always @(C or clk) $display(\"ISM at %0t clk=%b\", $time, clk);",
            ),
            lv("ISM"),
        ),
        (
            pkg_cell(
                PKG,
                "  import p::*; always @(E1 or clk) $display(\"IEM at %0t clk=%b\", $time, clk);",
            ),
            lv("IEM"),
        ),
        (
            pkg_cell(
                PKG,
                "  import p::C; always @(C) $display(\"IN at %0t\", $time);",
            ),
            "IN at 0\nDONE at 5\n".to_string(),
        ),
        (
            pkg_cell(
                PKG,
                "  import p::C; always @(C or clk) $display(\"INM at %0t clk=%b\", $time, clk);",
            ),
            lv("INM"),
        ),
    ];
    for (src, want) in &cells {
        expect(src, want);
    }
}

/// g3 b08 / b09 / b09b: a generate-scope `localparam K` shadowing a module net `K`.
/// The constant wins (IEEE §6.21), so a change of the NET never wakes the process —
/// b09b moves the net's only change to 4, and neither oracle prints `at 4`. b08
/// (`@(K)` alone; both oracles `GK at 0` / `DONE at 5`) runs once at time 0.
#[test]
fn a_shadowing_generate_constant_runs_once_at_time_zero() {
    let net = "  logic [7:0] K = 8'd0;\n";
    check(&[
        (
            pkg_cell(
                "",
                &format!(
                    "{net}  initial begin #1 K = 8'd1; #1 K = 8'd2; #1 K = 8'd3; end\n  if (1) begin : g localparam int K = 99; always @(K) $display(\"GK at %0t\", $time); end"
                ),
            )
            .as_str(),
            "GK at 0\nDONE at 5\n",
        ),
        (
            pkg_cell(
                "",
                &format!(
                    "{net}  initial begin #1 K = 8'd1; #1 K = 8'd2; #1 K = 8'd3; end\n  if (1) begin : g localparam int K = 99; always @(K or clk) $display(\"GKM at %0t clk=%b\", $time, clk); end"
                ),
            )
            .as_str(),
            "GKM at 0 clk=0\nGKM at 1 clk=1\nGKM at 2 clk=0\nGKM at 3 clk=1\nDONE at 5\n",
        ),
    ]);
    check(&[(
        "module top;\n  logic [7:0] K = 8'd0;\n  initial begin #4 K = 8'd7; end\n\
           reg clk = 0; initial begin #1 clk = 1; #1 clk = 0; #1 clk = 1; end\n\
           if (1) begin : g localparam int K = 99; always @(K or clk) $display(\"GKM at %0t clk=%b\", $time, clk); end\n\
           initial begin #6 $display(\"DONE at %0t\", $time); $finish; end\nendmodule\n",
        "GKM at 0 clk=0\nGKM at 1 clk=1\nGKM at 2 clk=0\nGKM at 3 clk=1\nDONE at 6\n",
    )]);
}

// ── what the time-0 run writes, and when ───────────────────────────────────────

/// g1 d02s / d03s: the time-0 run's blocking and non-blocking writes land (`x=x at 1`
/// before). g1 d04s: it runs after every `initial` statement of time 0. x1: an
/// `initial` written BELOW the `always` that changes the live term at time 0 does
/// not add a second run. The all-constant twins d02 / d03 / d04 (both oracles
/// `x=100 at 1`, `x=101 at 1`, `I1` / `I2` / `A at 0`) write and order the same.
#[test]
fn the_time_zero_run_writes_and_orders_like_the_oracles() {
    let w = |sens: &str, stmt: &str| {
        format!(
            "module top;\n  localparam int K = 99;\n  reg [31:0] x;\n  reg clk;\n  always @({sens}) {stmt}\n\
               initial #1 $display(\"x=%0d at %0t\", x, $time);\n\
               initial begin #5 clk = 1; #4 clk = 0; #6 $display(\"DONE\"); $finish; end\nendmodule\n"
        )
    };
    let o = |sens: &str| {
        format!(
            "module top;\n  localparam int K = 99;\n  reg clk;\n  initial $display(\"I1 at %0t\", $time);\n\
               always @({sens}) $display(\"A at %0t\", $time);\n  initial $display(\"I2 at %0t\", $time);\n\
               initial begin #5 clk = 1; #4 clk = 0; #6 $display(\"DONE\"); $finish; end\nendmodule\n"
        )
    };
    let cells = [
        (w("K", "x = K + 1;"), "x=100 at 1\nDONE\n"),
        (w("K or clk", "x = K + 1;"), "x=100 at 1\nDONE\n"),
        (w("K", "x <= K + 2;"), "x=101 at 1\nDONE\n"),
        (w("K or clk", "x <= K + 2;"), "x=101 at 1\nDONE\n"),
        (o("K"), "I1 at 0\nI2 at 0\nA at 0\nDONE\n"),
        (o("K or clk"), "I1 at 0\nI2 at 0\nA at 0\nA at 5\nA at 9\nDONE\n"),
        (
            "module top;\n  localparam int K = 99;\n  reg clk;\n\
               always @(K or clk) $display(\"MIX at %0t clk=%b\", $time, clk);\n  initial clk = 0;\n\
               initial begin #1 clk = 1; #1 clk = 0; #3 $display(\"DONE\"); $finish; end\nendmodule\n"
                .to_string(),
            "MIX at 0 clk=0\nMIX at 1 clk=1\nMIX at 2 clk=0\nDONE\n",
        ),
    ];
    let cells: Vec<(&str, &str)> = cells.iter().map(|(s, w)| (s.as_str(), *w)).collect();
    check(&cells);
}

/// x2 / x3: the live term changes at time 0 in the NBA region (`initial clk <= 0;`) or
/// after `#0` (`initial #0 clk = 0;`). The oracles SPLIT: iverilog runs the process
/// twice at time 0 (`MIX at 0 clk=x` / `MIX at 0 clk=0` / 1 / 2 / `DONE`), verilator
/// once (`MIX at 0 clk=0` / 1 / 2 / `DONE`). vita_pre printed verilator's text only
/// because it dropped the constant's time-0 run; the lane lands on iverilog's.
#[test]
fn a_live_term_written_later_in_time_zero_follows_iverilog() {
    let want = "MIX at 0 clk=x\nMIX at 0 clk=0\nMIX at 1 clk=1\nMIX at 2 clk=0\nDONE\n";
    for init in ["initial clk <= 0;", "initial #0 clk = 0;"] {
        let src = format!(
            "module top;\n  localparam int K = 99;\n  reg clk;\n\
               always @(K or clk) $display(\"MIX at %0t clk=%b\", $time, clk);\n  {init}\n\
               initial begin #1 clk = 1; #1 clk = 0; #3 $display(\"DONE\"); $finish; end\nendmodule\n"
        );
        assert_eq!(run(&src), want, "design:\n{src}");
    }
}

// ── body shapes ─────────────────────────────────────────────────────────────────

/// Bodies that cannot suspend the process, each measured with both oracles running it
/// at 0, 5 and 9 (`clk` rises at 5, falls at 9): a function call (i04), `$finish`
/// (i12), `-> ev` (i16), and `for` / `if` / `case` (i17).
///
/// `fork … join_none` (i08 with `#1` inside, i14 with `@(e2)` inside) runs at 0 in
/// both oracles too (iverilog `P at 0 clk=x`, verilator `clk=0`), but every fork is
/// BACK ON vita_pre's ROUTE: a `join_none` child against a `disable` of the block
/// splits the oracles at time 0 and left a line neither prints, so the time-0 run
/// is dropped again, as vita_pre did.
#[test]
fn a_body_that_cannot_suspend_runs_at_time_zero() {
    let m = |decls: &str, body: &str, tail: &str| {
        format!(
            "module top;\n  localparam int K = 99;\n  reg clk;\n{decls}  always @(K or clk) {body}\n{tail}\
               initial begin #5 clk = 1; #4 clk = 0; #6 $display(\"DONE\"); $finish; end\nendmodule\n"
        )
    };
    let e2 = "  initial begin #3 e2 = 1; #2 clk = 1; #2 e2 = 0; #2 clk = 0; #2 e2 = 1; #4 $display(\"DONE\"); $finish; end\n";
    let cells = [
        // i04: iverilog `F at 0 x=100 clk=x`, verilator `clk=0`
        (
            m(
                "  reg [31:0] x;\n  function int f(input int a); f = a + 1; endfunction\n",
                "begin x = f(K); $display(\"F at %0t x=%0d clk=%b\", $time, x, clk); end",
                "",
            ),
            "F at 0 x=100 clk=x\nF at 5 x=100 clk=1\nF at 9 x=100 clk=0\nDONE\n".to_string(),
        ),
        // i12: `END at 0` in both
        (m("", "$finish;", "  final $display(\"END at %0t\", $time);\n"), "END at 0\n".to_string()),
        // i08: both oracles `P at 0` (iverilog `clk=x`, verilator `clk=0`) / `FK at 1`
        // / 5 / 6 / 9 / 10; vita_pre's text (no time-0 run)
        (
            m(
                "",
                "begin fork #1 $display(\"FK at %0t\", $time); join_none $display(\"P at %0t clk=%b\", $time, clk); end",
                "",
            ),
            "P at 5 clk=1\nFK at 6\nP at 9 clk=0\nFK at 10\nDONE\n".to_string(),
        ),
        // i17: both oracles print exactly this
        (
            m(
                "  reg [3:0] i;\n",
                "begin for (i = 0; i < 2; i = i + 1) $display(\"L at %0t i=%0d\", $time, i); \
                 if (clk) $display(\"H at %0t\", $time); else $display(\"NH at %0t\", $time); \
                 case (clk) 1'b1: $display(\"C1 at %0t\", $time); default: $display(\"CD at %0t\", $time); endcase end",
                "",
            ),
            "L at 0 i=0\nL at 0 i=1\nNH at 0\nCD at 0\nL at 5 i=0\nL at 5 i=1\nH at 5\nC1 at 5\n\
             L at 9 i=0\nL at 9 i=1\nNH at 9\nCD at 9\nDONE\n"
                .to_string(),
        ),
    ];
    for (src, want) in &cells {
        assert_eq!(run(src), *want, "design:\n{src}");
    }
    // i14: `fork @(e2) … join_none` (e2 rises at 3, falls at 7, rises at 11); both
    // oracles `P at 0` / `FE at 3` / 5 / 7 / 9 / 11; vita_pre's text (no time-0 run).
    let src = format!(
        "module top;\n  localparam int K = 99;\n  reg clk;\n  reg e2;\n\
           always @(K or clk) begin fork @(e2) $display(\"FE at %0t\", $time); join_none $display(\"P at %0t clk=%b\", $time, clk); end\n{e2}endmodule\n"
    );
    assert_eq!(
        run(&src),
        "P at 5 clk=1\nFE at 7\nP at 9 clk=0\nFE at 11\nDONE\n"
    );
    // i16: `-> ev` wakes `always @(ev)` at 0 / 5 / 9 — iverilog prints TR before EV
    // in each step, verilator EV before TR.
    check_sorted(
        &m(
            "  event ev;\n",
            "begin -> ev; $display(\"TR at %0t clk=%b\", $time, clk); end",
            "  always @(ev) $display(\"EV at %0t\", $time);\n",
        ),
        "TR at 0 clk=x\nEV at 0\nTR at 5 clk=1\nEV at 5\nTR at 9 clk=0\nEV at 9\nDONE\n",
    );
}

/// SPLIT shapes that keep the header lane, pinned UNCHANGED from vita_pre (which prints
/// verilator's text in each). iverilog also runs the body at time 0; verilator does
/// not:
/// - g1 d01s `#2` body: iverilog `D at 2 clk=x` / 7 / 11, verilator 7 / 11;
/// - i01 static task with `#1`: iverilog `T at 1 clk=x` / 6 / 10, verilator 6 / 10;
/// - i03 automatic task with `#1`: iverilog `TA at 1` / 6 / 10, verilator 6 / 10;
/// - i05 `x = #1 K`: iverilog `B at 1` / 6 / 10, verilator 6 / 10;
/// - i09 `fork … join` with timing: iverilog from 1, verilator from 6;
/// - i11 `wait fork`: iverilog `WF at 0 clk=x` / 5 / 9, verilator 5 / 9;
/// - i15 `fork … join` without timing: iverilog `C2`,`C1`,`P at 0` / 5 / 9, verilator
///   5 / 9.
#[test]
fn a_body_that_can_suspend_keeps_the_header_lane() {
    let m = |decls: &str, body: &str| {
        format!(
            "module top;\n  localparam int K = 99;\n  reg clk;\n{decls}  always @(K or clk) {body}\n\
               initial begin #5 clk = 1; #4 clk = 0; #6 $display(\"DONE\"); $finish; end\nendmodule\n"
        )
    };
    let cells = [
        (m("", "begin #2 $display(\"D at %0t clk=%b\", $time, clk); end"), "D at 7 clk=1\nD at 11 clk=0\nDONE\n"),
        (
            m("  task t; begin #1 $display(\"T at %0t clk=%b\", $time, clk); end endtask\n", "t;"),
            "T at 6 clk=1\nT at 10 clk=0\nDONE\n",
        ),
        (
            m("  task automatic ta; begin #1 $display(\"TA at %0t clk=%b\", $time, clk); end endtask\n", "ta;"),
            "TA at 6 clk=1\nTA at 10 clk=0\nDONE\n",
        ),
        (
            m("  reg [31:0] x;\n", "begin x = #1 K; $display(\"B at %0t x=%0d clk=%b\", $time, x, clk); end"),
            "B at 6 x=99 clk=1\nB at 10 x=99 clk=0\nDONE\n",
        ),
        (
            m(
                "",
                "begin fork #1 $display(\"FJ1 at %0t\", $time); #2 $display(\"FJ2 at %0t\", $time); join $display(\"P at %0t clk=%b\", $time, clk); end",
            ),
            "FJ1 at 6\nFJ2 at 7\nP at 7 clk=1\nFJ1 at 10\nFJ2 at 11\nP at 11 clk=0\nDONE\n",
        ),
        (m("", "begin wait fork; $display(\"WF at %0t clk=%b\", $time, clk); end"), "WF at 5 clk=1\nWF at 9 clk=0\nDONE\n"),
        (
            m(
                "",
                "begin fork $display(\"C1 at %0t\", $time); $display(\"C2 at %0t\", $time); join $display(\"P at %0t clk=%b\", $time, clk); end",
            ),
            "C1 at 5\nC2 at 5\nP at 5 clk=1\nC1 at 9\nC2 at 9\nP at 9 clk=0\nDONE\n",
        ),
    ];
    let cells: Vec<(&str, &str)> = cells.iter().map(|(s, w)| (s.as_str(), *w)).collect();
    check(&cells);
    // k02: a task without timing that enables a task with `#1` — refused transitively.
    // iverilog `T at 0 clk=x` / `I at 1 clk=x` / 5 / 6 / 9 / 10, verilator from 5.
    check(&[(
        m(
            "  task inner; begin #1 $display(\"I at %0t clk=%b\", $time, clk); end endtask\n  \
             task t; begin $display(\"T at %0t clk=%b\", $time, clk); inner; end endtask\n",
            "t;",
        )
        .as_str(),
        "T at 5 clk=1\nI at 6 clk=1\nT at 9 clk=0\nI at 10 clk=0\nDONE\n",
    )]);
}

/// A task enable that resolves to a module-local task whose body cannot suspend,
/// transitively. Both oracles run the process at 0, 5 and 9 (iverilog `clk=x` at 0,
/// verilator `clk=0`); vita dropped the time-0 run (i02 / k01).
/// - k01 static task without timing: `T at 0` / 5 / 9;
/// - k03 automatic task without timing: `TA at 0` / 5 / 9;
/// - k04 a task writing its `output` formal: `O at 0 x=100` / 5 / 9;
/// - k07 a task enabling a task without timing: `T`, `I` at 0 / 5 / 9.
#[test]
fn a_task_enable_that_cannot_suspend_runs_at_time_zero() {
    let m = |decls: &str, body: &str| {
        format!(
            "module top;\n  localparam int K = 99;\n  reg clk;\n{decls}  always @(K or clk) {body}\n\
               initial begin #5 clk = 1; #4 clk = 0; #6 $display(\"DONE\"); $finish; end\nendmodule\n"
        )
    };
    let cells = [
        (
            m("  task t; begin $display(\"T at %0t clk=%b\", $time, clk); end endtask\n", "t;"),
            "T at 0 clk=x\nT at 5 clk=1\nT at 9 clk=0\nDONE\n",
        ),
        (
            m("  task automatic ta; begin $display(\"TA at %0t clk=%b\", $time, clk); end endtask\n", "ta;"),
            "TA at 0 clk=x\nTA at 5 clk=1\nTA at 9 clk=0\nDONE\n",
        ),
        (
            m(
                "  reg [31:0] x;\n  task t(output [31:0] o); begin o = K + 1; end endtask\n",
                "begin t(x); $display(\"O at %0t x=%0d clk=%b\", $time, x, clk); end",
            ),
            "O at 0 x=100 clk=x\nO at 5 x=100 clk=1\nO at 9 x=100 clk=0\nDONE\n",
        ),
        (
            m(
                "  task inner; begin $display(\"I at %0t clk=%b\", $time, clk); end endtask\n  \
                 task t; begin $display(\"T at %0t clk=%b\", $time, clk); inner; end endtask\n",
                "t;",
            ),
            "T at 0 clk=x\nI at 0 clk=x\nT at 5 clk=1\nI at 5 clk=1\nT at 9 clk=0\nI at 9 clk=0\nDONE\n",
        ),
    ];
    let cells: Vec<(&str, &str)> = cells.iter().map(|(s, w)| (s.as_str(), *w)).collect();
    check(&cells);
    // k08: a task IMPORTED from a package is not module-local and keeps the header
    // lane, unchanged from vita_pre (both oracles print `PT at 0 v=99` / 5 / 9 —
    // recorded residue).
    check(&[(
        "package p; task t(input int v); $display(\"PT at %0t v=%0d\", $time, v); endtask endpackage\n\
         module top;\n  import p::t;\n  localparam int K = 99;\n  reg clk;\n  always @(K or clk) t(K);\n\
           initial begin #5 clk = 1; #4 clk = 0; #6 $display(\"DONE\"); $finish; end\nendmodule\n",
        "PT at 5 v=99\nPT at 9 v=99\nDONE\n",
    )]);
}

/// A constant select or `p::C` on a list the time-0 lane DECLINES is held aside like a
/// bare constant — dropped beside a live term — where it was refused as a net. Both
/// kinds of list are oracle SPLITS:
/// - j01 `@(K[0] or clk)`, j02 `@(p::C or clk)`, j03 `@(K[3:0] or clk)` with a `#1`
///   body (clk rises at 5, falls at 9): iverilog `J at 1 clk=x` / 6 / 10, verilator
///   6 / 10. vita lands on verilator, like the bare `@(K or clk)` (g1 d01s).
/// - j04 `@(K[0] or posedge clk)`, j05 `@(p::C or posedge clk)` (clk 1@1, 0@2, 1@3):
///   iverilog `J at 1` / `J at 3`, verilator also `J at 0 clk=0`. vita lands on
///   iverilog, like the bare `@(K or posedge clk)` (g1 c01).
#[test]
fn a_constant_select_or_package_term_on_a_declined_list_is_dropped() {
    let d = |decls: &str, sens: &str| {
        format!(
            "{decls}module top;\n  localparam int K = 99;\n  reg clk;\n\
               always @({sens}) begin #1 $display(\"J at %0t clk=%b\", $time, clk); end\n\
               initial begin #5 clk = 1; #4 clk = 0; #6 $display(\"DONE\"); $finish; end\nendmodule\n"
        )
    };
    let e = |decls: &str, sens: &str| {
        format!(
            "{decls}module top;\n  localparam int K = 99;\n  reg clk;\n\
               always @({sens}) $display(\"J at %0t clk=%b\", $time, clk);\n\
               initial begin #1 clk = 1; #1 clk = 0; #1 clk = 1; #2 $display(\"DONE\"); $finish; end\nendmodule\n"
        )
    };
    let pc = "package p; localparam int C = 5; endpackage\n";
    let vl = "J at 6 clk=1\nJ at 10 clk=0\nDONE\n";
    let iv = "J at 1 clk=1\nJ at 3 clk=1\nDONE\n";
    let cells = [
        (d("", "K[0] or clk"), vl),
        (d(pc, "p::C or clk"), vl),
        (d("", "K[3:0] or clk"), vl),
        (e("", "K[0] or posedge clk"), iv),
        (e(pc, "p::C or posedge clk"), iv),
    ];
    let cells: Vec<(&str, &str)> = cells.iter().map(|(s, w)| (s.as_str(), *w)).collect();
    check(&cells);
}

/// Header lists the lane does not take, pinned UNCHANGED from vita_pre:
/// - g1 c01 `@(K or posedge clk)`: iverilog `MIX at 1` / `DONE`, verilator also `MIX
///   at 0` (vita = iverilog);
/// - g1 e01 in-body `always begin @(K) … end`: iverilog `AB at 0`, verilator nothing
///   (vita = verilator);
/// - g1 b09 net-only `@(clk)` with `reg clk;`: iverilog 1 / 2, verilator 0 / 1 / 2
///   (vita = iverilog).
#[test]
fn a_list_outside_the_lane_is_unchanged() {
    let k = "  localparam int K = 99;\n";
    let cells = [
        (
            format!(
                "module top;\n{k}  reg clk;\n  always @(K or posedge clk) $display(\"MIX at %0t clk=%b\", $time, clk);\n\
                   initial begin #1 clk = 1; #1 clk = 0; #3 $display(\"DONE\"); $finish; end\nendmodule\n"
            ),
            "MIX at 1 clk=1\nDONE\n",
        ),
        (
            format!(
                "module top;\n{k}  always begin @(K) $display(\"AB at %0t\", $time); end\n\
                   initial begin #15 $display(\"DONE\"); $finish; end\nendmodule\n"
            ),
            "DONE\n",
        ),
        (
            mixed("", "  always @(clk) $display(\"MIX at %0t clk=%b\", $time, clk);"),
            "MIX at 1 clk=1\nMIX at 2 clk=0\nDONE\n",
        ),
    ];
    let cells: Vec<(&str, &str)> = cells.iter().map(|(s, w)| (s.as_str(), *w)).collect();
    check(&cells);
}

// ── the refusals that remain, and what they say ────────────────────────────────

/// An all-constant list the time-0 lane declines stays loud, and the refusal names
/// the reason for THIS block (the reverse cells, `r*` / `m02`):
/// - g1 d01 `#2` body: iverilog `D at 2` / `DONE`, verilator `DONE`;
/// - r01 `@(e2)` body: iverilog `R at 3`, verilator nothing;
/// - r05 `wait fork` body: iverilog `R at 0`, verilator nothing;
/// - r07 `wait (v)` body: iverilog `R at 3`, verilator nothing (not admitted: both
///   run `wait (1)`);
/// - r12 `@(K or posedge K2)`: iverilog nothing, verilator `R at 0`;
/// - r13 `always_ff @(K)`: both `FF at 0` (the refusal claims no split);
/// - r14 `@(K iff en)`: iverilog syntax error, verilator `IFF at 0`;
/// - r15 `import p::t;` task enable: both `PT at 0` (not a module-local task);
/// - m02 `assert property (@(K) a)`: iverilog rejects, verilator `P at 0`.
///
/// Its shadow twin keeps the §2 O sentence, and the term prints as written (j06
/// `@(K[0])`, j07 `@(p::C)`). A net-headed bit or element select keeps its own
/// refusal (per-bit level tracking is a separate feature).
#[test]
fn the_remaining_refusals_name_their_cause() {
    let one = |decls: &str, always: &str| {
        format!(
            "module top;\n  localparam int K = 99;\n{decls}  {always}\n\
               initial begin #9 $display(\"DONE\"); $finish; end\nendmodule\n"
        )
    };
    let split = "(iverilog runs such a process at time 0, verilator does not)";
    loud(
        &one("", "always @(K) begin #2 $display(\"D at %0t\", $time); end"),
        &format!("with no live term runs its process once at time 0 and never again; vita runs \
                  that only for an `always` written in the source, with level terms and no `iff` \
                  guard, whose body cannot suspend — here the body can suspend at a `#` delay {split}"),
    );
    loud(
        &one(
            "  reg e2 = 0;\n",
            "always @(K) begin @(e2) $display(\"R at %0t\", $time); end",
        ),
        &format!("here the body can suspend at an `@` event control {split}"),
    );
    loud(
        &one(
            "",
            "always @(K) begin wait fork; $display(\"R at %0t\", $time); end",
        ),
        &format!("here the body can suspend at `wait fork` {split}"),
    );
    loud(
        &one(
            "  reg v = 0;\n",
            "always @(K) begin wait (v); $display(\"R at %0t\", $time); end",
        ),
        "here the body holds `wait (…)`, which vita does not admit",
    );
    loud(
        &one(
            "  localparam int K2 = 1;\n",
            "always @(K or posedge K2) $display(\"R at %0t\", $time);",
        ),
        "here this list also has an edge term (verilator runs such a process at time 0, \
         iverilog does not)",
    );
    loud(
        &one("", "always_ff @(K) $display(\"FF at %0t\", $time);"),
        "here this block is an `always_ff`",
    );
    loud(
        &one(
            "  reg en = 1;\n",
            "always @(K iff en) $display(\"IFF at %0t\", $time);",
        ),
        "here this list carries an `iff` guard",
    );
    loud(
        "package p; task t; $display(\"PT at %0t\", $time); endtask endpackage\n\
         module top;\n  import p::t;\n  localparam int K = 99;\n  always @(K) t;\n\
           initial begin #9 $display(\"DONE\"); $finish; end\nendmodule\n",
        "here the body holds the enable `t` of a task not declared in this module",
    );
    loud(
        &one(
            "  reg a = 1;\n",
            "assert property (@(K) a) $display(\"P at %0t\", $time);",
        ),
        "here this list is the clock of an assertion, covergroup or clocking block",
    );
    loud(
        "module top;\n  logic V;\n  initial begin V = 0; #1 V = 1; end\n\
           generate if (1) begin : g\n    localparam int V = 2;\n\
             always @(V) begin #1 $display(\"HDR fired at %0t\", $time); end\n  end endgenerate\n\
           initial #3 begin $display(\"DONE\"); $finish; end\nendmodule\n",
        "which shadows the net of the same name — the level event control `@(V)` has no live term",
    );
    loud(
        "module top;\n  localparam int K = 99;\n  always @(K[0]) begin #2 $display(\"J at %0t\", $time); end\n\
           initial begin #15 $display(\"DONE\"); $finish; end\nendmodule\n",
        "a level event control on the constant `K[0]` (a parameter",
    );
    loud(
        "package p; localparam int C = 5; endpackage\nmodule top;\n\
           always @(p::C) begin #2 $display(\"J at %0t\", $time); end\n\
           initial begin #15 $display(\"DONE\"); $finish; end\nendmodule\n",
        "a level event control on the constant `p::C` (a parameter",
    );
    loud(
        &pkg_cell(
            PKG,
            "  always @(p::nope or clk) $display(\"M at %0t\", $time);",
        ),
        "must name a package variable or constant, and this name is neither",
    );
    // g2 b12 `reg [3:0] a [0:1]; always @(a[1])` — a whole 4-bit element.
    loud(
        "module top;\n  reg [3:0] a [0:1];\n  always @(a[1]) $display(\"B at %0t\", $time);\n\
           initial begin #1 a[1] = 4'd5; #1 $finish; end\nendmodule\n",
        "a level (non-edge) event control on a bit or element select is not supported",
    );
}
