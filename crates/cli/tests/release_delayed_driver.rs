//! Q-RELDELAY: a `release` of a net or variable driven by a DELAYED continuous assignment restores the
//! driver's output at once (IEEE 1800-2017 §10.6.2: the released net "shall immediately be assigned the
//! value determined by the drivers of the net"). A delayed driver's output is written only at its landing
//! tick, so before this row the settle the release re-dirties could not restore it and the forced value
//! stayed until the next landing. The release now writes each delayed driver's last landed value, or the
//! owed `x` before its first landing, through the store funnel, in both kernels
//! (`Scheduler::released_driver_outputs`), only when the target was forced, and only onto the released
//! net: a chunk of the driver's lvalue on another net (a co-member of a concatenation) is skipped.
//!
//! Oracles: Icarus Verilog 13.0 `-g2012` (iverilog), sv2v 0.0.13 -> iverilog (sv2v), Verilator 5.052
//! `--binary --timing` (verilator). Every in-row cell's whole output equals iverilog's unless its comment
//! says otherwise (E48 and P01: the IEEE-literal side of a recorded split, = verilator; M37: no oracle);
//! each comment names the tools that differ and PRE's lines (PRE = 73d22f5e). Every test runs on native,
//! vm and interp; two also run the staged chain.
//!
//! Never pinned: the Q-RELIMM lines (a read right after the release of an UNDELAYED driver's target
//! sees the forced value until the settle: E42 E45 E50, E51 / R03 R04 R06–R08 `t=7imm`, R12 R13 R15
//! R24), E10 t=6 (Q-FORCEPORT), E40 (a non-constant select in a continuous assignment's lhs, outside
//! the language: Q-CALHSEL), E52 z (recorded split), R28 t=1 / t=3 and S13 t=2 / t=3 (N-R01), R02 t=0 /
//! t=3 and every co-member value line before the first landing (S01 S03 S17 S18 V01 V03 V06 W01 W03
//! W04 A20: Q-DCAPARTX; and N-R01's 64-bit settle image on a co-member, lens-sound cell E1: `real r;
//! assign #5 {z, y} = r;` with 40-bit z, y keeps z = 16 zeros + 24 x, = PRE, iverilog x(40)), M37
//! after its release (no oracle), and every call-count line (R29, R30, O3a `F`: Q-RELEVAL,
//! Q-DCASETTLE).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn scratch(tag: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_rdd_{tag}_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// `vita <args>` in `dir`: its stdout lines (no `simulation ended` line), after a clean exit.
fn vita_lines(dir: &std::path::Path, args: &[&str], what: &str) -> Vec<String> {
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run vita");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let code = out.status.code().unwrap_or(-1);
    assert_eq!(
        code,
        0,
        "{what}: exit {code}\n{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    stdout
        .lines()
        .filter(|l| !l.starts_with("simulation ended"))
        .map(str::to_string)
        .collect()
}

/// `src` on one backend.
fn run_on(src: &str, backend: &str) -> Vec<String> {
    let d = scratch(backend);
    std::fs::write(d.join("t.sv"), src).unwrap();
    let lines = vita_lines(&d, &["--backend", backend, "t.sv"], backend);
    let _ = std::fs::remove_dir_all(&d);
    lines
}

/// `src` on one backend: its stderr, after a clean exit.
fn stderr_on(src: &str, backend: &str) -> String {
    let d = scratch(backend);
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["--backend", backend, "t.sv"])
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(0), "{backend}: exit\n{stderr}");
    stderr
}

/// Every backend prints exactly `want`.
fn pin(src: &str, want: &[&str]) {
    for be in ["native", "vm", "interp"] {
        assert_eq!(run_on(src, be), want, "backend {be}");
    }
}

/// On every backend, the lines starting with one of `prefixes` are exactly `want`.
fn pin_lines(src: &str, prefixes: &[&str], want: &[&str]) {
    for be in ["native", "vm", "interp"] {
        let got: Vec<String> = run_on(src, be)
            .into_iter()
            .filter(|l| prefixes.iter().any(|p| l.starts_with(p)))
            .collect();
        assert_eq!(got, want, "backend {be}");
    }
}

/// On every backend, the `field=` value of each line starting with `prefix`, in order, is exactly `want`
/// (a line that prints a pinned net beside a co-member value that is never pinned).
fn pin_field(src: &str, prefix: &str, field: &str, want: &[&str]) {
    let key = format!("{field}=");
    for be in ["native", "vm", "interp"] {
        let got: Vec<String> = run_on(src, be)
            .into_iter()
            .filter(|l| l.starts_with(prefix))
            .filter_map(|l| {
                l.split_whitespace()
                    .find_map(|t| t.strip_prefix(key.as_str()).map(str::to_string))
            })
            .collect();
        assert_eq!(got, want, "backend {be}");
    }
}

/// The staged chain `vcmp` -> `velab` -> `vrun` prints exactly `want`.
fn pin_staged(src: &str, want: &[&str]) {
    let d = scratch("staged");
    std::fs::write(d.join("t.sv"), src).unwrap();
    vita_lines(&d, &["vcmp", "-o", "t.vu", "t.sv"], "vcmp");
    vita_lines(&d, &["velab", "-o", "t.velab", "t.vu"], "velab");
    let got = vita_lines(&d, &["vrun", "t.velab"], "vrun");
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(got, want, "staged");
}

/// In-row (wrong -> right): net #3, rhs 8 -> 4 bits. iverilog = sv2v = verilator = the pin. PRE: `S t=6 y=0010`
/// | `S t=8 y=0010` | `S t=11 y=0010` | `S t=20 y=0001` | `A end`. Kills mutant(s) M1 M2.
/// Cell `U01_rel_delayed_wide_dropbits`.
#[test]
fn u01_rel_delayed_wide_dropbits() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 8'h11; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "S t=6 y=0010",
            "S t=8 y=0001",
            "S t=11 y=0001",
            "S t=20 y=0001",
            "A end",
        ],
    );
}

/// In-row (wrong -> right): net #3 same width, rhs change. iverilog = sv2v = verilator = the pin. PRE:
/// `S t=6 y=0010` | `S t=8 y=0010` | `S t=11 y=0010` | `S t=20 y=0011` | `A end`. Kills mutant(s) M1 M2.
/// Cell `U03_rel_delayed_samewidth_change`.
#[test]
fn u03_rel_delayed_samewidth_change() {
    pin(
        r#"module top;
  logic [3:0] a = 4'h1; wire [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 4'h3; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "S t=6 y=0010",
            "S t=8 y=0001",
            "S t=11 y=0001",
            "S t=20 y=0011",
            "A end",
        ],
    );
}

/// In-row (wrong -> right): logic var #3. iverilog = sv2v = verilator = the pin. PRE: `S t=6 y=0010` |
/// `S t=8 y=0010` | `S t=11 y=0010` | `S t=20 y=0001` | `A end`. Kills mutant(s) M1 M2.
/// Cell `U04_rel_delayed_logicvar_dropbits`.
#[test]
fn u04_rel_delayed_logicvar_dropbits() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; logic [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 8'h11; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "S t=6 y=0010",
            "S t=8 y=0001",
            "S t=11 y=0001",
            "S t=20 y=0001",
            "A end",
        ],
    );
}

/// In-row (wrong -> right): net #(3,2). iverilog = verilator = the pin. sv2v: `(no sv2v->iverilog binary)` |
/// `s2v.err:t.sv:3:13: Parse error: unexpected token ',' (Sym_comma)`. PRE: `S t=6 y=0010` | `S t=8 y=0010` |
/// `S t=11 y=0010` | `S t=20 y=0001` | `A end`. Kills mutant(s) M1 M2.
/// Cell `U05_rel_delayed_rf_dropbits`.
#[test]
fn u05_rel_delayed_rf_dropbits() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #(3,2) y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 8'h11; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "S t=6 y=0010",
            "S t=8 y=0001",
            "S t=11 y=0001",
            "S t=20 y=0001",
            "A end",
        ],
    );
}

/// In-row (wrong -> right): real rhs -> 4-bit net. iverilog = sv2v = verilator = the pin. PRE: `S t=6 y=0010` |
/// `S t=8 y=0010` | `S t=11 y=0010` | `S t=20 y=0001` | `A end`. Kills mutant(s) M1 M2.
/// Cell `U06_rel_delayed_real_rhs_sameimage`.
#[test]
fn u06_rel_delayed_real_rhs_sameimage() {
    pin(
        r#"module top;
  real r = 1.0; wire [3:0] y;
  assign #3 y = r;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 r = 1.25; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "S t=6 y=0010",
            "S t=8 y=0001",
            "S t=11 y=0001",
            "S t=20 y=0001",
            "A end",
        ],
    );
}

/// In-row (wrong -> right): net #3, no rhs change. iverilog = sv2v = verilator = the pin. PRE: `S t=6 y=0010` |
/// `S t=8 y=0010` | `S t=11 y=0010` | `S t=20 y=0010` | `A end`. Kills mutant(s) M1 M2.
/// Cell `U07_rel_delayed_samewidth_nochange`.
#[test]
fn u07_rel_delayed_samewidth_nochange() {
    pin(
        r#"module top;
  logic [3:0] a = 4'h1; wire [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 4'h1; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "S t=6 y=0010",
            "S t=8 y=0001",
            "S t=11 y=0001",
            "S t=20 y=0001",
            "A end",
        ],
    );
}

/// In-row (wrong -> right): pending value at release. iverilog = sv2v = verilator = the pin. PRE:
/// `S t=6 y=0010` | `S t=8 y=0010` | `S t=10 y=0011` | `A end`. Kills mutant(s) M1 M2 M3.
/// Cell `U08_rel_delayed_pending`.
#[test]
fn u08_rel_delayed_pending() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 a = 8'h03; $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #2 $display("S t=10 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0010", "S t=8 y=0001", "S t=10 y=0011", "A end"],
    );
}

/// In-row (wrong -> right): net #0. iverilog = sv2v = verilator = the pin. PRE: `S t=6 y=0010` | `S t=8 y=0010`
/// | `S t=11 y=0010` | `S t=20 y=0001` | `A end`. Kills mutant(s) M1 M2.
/// Cell `U10_rel_delayed_d0_dropbits`.
#[test]
fn u10_rel_delayed_d0_dropbits() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #0 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 8'h11; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "S t=6 y=0010",
            "S t=8 y=0001",
            "S t=11 y=0001",
            "S t=20 y=0001",
            "A end",
        ],
    );
}

/// In-row (wrong -> right): wire [3:0] #3 y = a. iverilog = verilator = the pin. sv2v:
/// `(no sv2v->iverilog binary)` | `s2v.err:t.sv:3:14: Parse error: unexpected token '#' (Sym_pound)`. PRE:
/// `S t=6 y=0010` | `S t=8 y=0010` | `S t=11 y=0010` | `S t=20 y=0001` | `A end`. Kills mutant(s) M1 M2.
/// Cell `U11_rel_netdecl_delay_dropbits`.
#[test]
fn u11_rel_netdecl_delay_dropbits() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01;
  wire [3:0] #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 8'h11; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "S t=6 y=0010",
            "S t=8 y=0001",
            "S t=11 y=0001",
            "S t=20 y=0001",
            "A end",
        ],
    );
}

/// In-row (wrong -> right): #3 part-select + plain part-select. iverilog = sv2v = verilator = the pin. PRE:
/// `S t=6 w=00000010` | `S t=8 w=00000010` | `S t=11 w=00000010` | `S t=20 w=00000001` | `A end`. Kills
/// mutant(s) M1 M2.
/// Cell `U12_rel_delayed_sel_dropbits`.
#[test]
fn u12_rel_delayed_sel_dropbits() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [7:0] w;
  assign w[7:4] = 4'h0;
  assign #3 w[3:0] = a;
  initial begin
    #5 force w = 8'h02; #1 $display("S t=6 w=%b", w);
    #1 release w; #1 $display("S t=8 w=%b", w);
    #3 $display("S t=11 w=%b", w);
    #4 a = 8'h11; #5 $display("S t=20 w=%b", w); $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "S t=6 w=00000010",
            "S t=8 w=00000001",
            "S t=11 w=00000001",
            "S t=20 w=00000001",
            "A end",
        ],
    );
}

/// In-row (wrong -> right): rhs change while forced (same image). iverilog = sv2v = verilator = the pin. PRE:
/// `S t=6 y=0010` | `S t=8 y=0010` | `S t=11 y=0001` | `A end`. Kills mutant(s) M1 M2.
/// Cell `P03_force_samimage_change_then_rel`.
#[test]
fn p03_force_samimage_change_then_rel() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 a = 8'h11; $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0010", "S t=8 y=0001", "S t=11 y=0001", "A end"],
    );
}

/// In-row (wrong -> right): logic var, read t=8 t=9 t=10. iverilog = sv2v = verilator = the pin. PRE:
/// `S t=6 y=0010` | `S t=8 y=0010` | `S t=9 y=0010` | `S t=10 y=0010` | `A end`. Kills mutant(s) M1 M2.
/// Cell `P07_force_var_rel_pending`.
#[test]
fn p07_force_var_rel_pending() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; logic [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #1 $display("S t=9 y=%b", y); #1 $display("S t=10 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "S t=6 y=0010",
            "S t=8 y=0001",
            "S t=9 y=0001",
            "S t=10 y=0001",
            "A end",
        ],
    );
}

/// In-row (wrong -> right): #0 driver, force+release at t=0. iverilog = sv2v = verilator = the pin. PRE:
/// `S t=1 y=0010` | `A end`. Kills mutant(s) M1 M2.
/// Cell `E02_rel_t0_d0`.
#[test]
fn e02_rel_t0_d0() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #0 y = a;
  initial begin
    force y = 4'h2; release y; #1 $display("S t=1 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y=0001", "A end"],
    );
}

/// In-row (wrong -> right): landing due at the release tick (release scheduled first), @(y). iverilog = sv2v =
/// the pin. verilator: `W t=0 y=0000` | `W t=2 y=0010` | `W t=7 y=0011` | `S t=8 y=0011` | `A end`. PRE:
/// `W t=2 y=0010` | `S t=8 y=0010` | `A end`. Kills mutant(s) M1 M2.
/// Cell `E03_samestep_relfirst`.
#[test]
fn e03_samestep_relfirst() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  always @(y) $display("W t=%0t y=%b", $time, y);
  initial begin
    #2 force y = 4'h2; #2 a = 8'h03; #3 release y; #1 $display("S t=8 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["W t=2 y=0010", "W t=7 y=0011", "S t=8 y=0011", "A end"],
    );
}

/// In-row (wrong -> right): same, update scheduled first. iverilog = sv2v = the pin. verilator: `W t=0 y=0000`
/// | `W t=2 y=0010` | `W t=7 y=0011` | `S t=8 y=0011` | `A end`. PRE: `W t=2 y=0010` | `S t=8 y=0010` |
/// `A end`. Kills mutant(s) M1 M2.
/// Cell `E04_samestep_updfirst`.
#[test]
fn e04_samestep_updfirst() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  always @(y) $display("W t=%0t y=%b", $time, y);
  initial begin
    #2 force y = 4'h2; #2 a = 8'h03; #0; #3 release y; #1 $display("S t=8 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["W t=2 y=0010", "W t=7 y=0011", "S t=8 y=0011", "A end"],
    );
}

/// In-row (wrong -> right): assign #3 {y,z}; release y while z forced. iverilog = sv2v = verilator = the pin.
/// PRE: `S t=6 y=0111 z=1000` | `S t=8 y=0111 z=1000` | `S t=11 y=0111 z=1000` | `A end`. Kills mutant(s) M1
/// M2 M19e.
/// Cell `E06_cat_lhs_partial_release`.
#[test]
fn e06_cat_lhs_partial_release() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h12; wire [3:0] y, z;
  assign #3 {y, z} = a;
  initial begin
    #5 force y = 4'h7; force z = 4'h8; #1 $display("S t=6 y=%b z=%b", y, z);
    #1 release y; #1 $display("S t=8 y=%b z=%b", y, z);
    #2 release z; #1 $display("S t=11 y=%b z=%b", y, z);
    $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "S t=6 y=0111 z=1000",
            "S t=8 y=0001 z=1000",
            "S t=11 y=0001 z=0010",
            "A end",
        ],
    );
}

/// In-row (wrong -> right): and #3 gate output. iverilog = sv2v = verilator = the pin. PRE: `S t=6 y=0` |
/// `S t=8 y=0` | `S t=11 y=0` | `A end`. Kills mutant(s) M1 M2.
/// Cell `E07_gate_and_delay`.
#[test]
fn e07_gate_and_delay() {
    pin(
        r#"module top;
  logic a = 1'b1, b = 1'b1; wire y;
  and #3 g(y, a, b);
  initial begin
    #5 force y = 1'b0; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0", "S t=8 y=1", "S t=11 y=1", "A end"],
    );
}

/// In-row (wrong -> right): #(3,2,4), driver output z. iverilog = the pin. sv2v: `(no sv2v->iverilog binary)` |
/// `s2v.err:t.sv:3:13: Parse error: unexpected token ',' (Sym_comma)`. verilator: `S t=6 y=0010` |
/// `S t=8 y=0000` | `S t=20 y=0001` | `A end`. PRE: `S t=6 y=0010` | `S t=8 y=0010` | `S t=20 y=0001` |
/// `A end`. Kills mutant(s) M1 M2.
/// Cell `E09_rf3_z_driver`.
#[test]
fn e09_rf3_z_driver() {
    pin(
        r#"module top;
  logic [3:0] a = 4'h1; logic en = 1'b0; wire [3:0] y;
  assign #(3,2,4) y = en ? a : 4'bz;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #2 en = 1'b1; #10 $display("S t=20 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0010", "S t=8 y=zzzz", "S t=20 y=0001", "A end"],
    );
}

/// In-row (wrong -> right): downstream assign z = y + 1. iverilog = sv2v = verilator = the pin. PRE:
/// `S t=6 y=0010 z=0011` | `S t=8 y=0010 z=0011` | `A end`. Kills mutant(s) M1 M2.
/// Cell `E12_downstream_ca`.
#[test]
fn e12_downstream_ca() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y, z;
  assign #3 y = a;
  assign z = y + 4'd1;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b z=%b", y, z);
    #1 release y; #1 $display("S t=8 y=%b z=%b", y, z);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0010 z=0011", "S t=8 y=0001 z=0010", "A end"],
    );
}

/// In-row (wrong -> right): force/release twice, change between. iverilog = sv2v = verilator = the pin. PRE:
/// `S t=8 y=0010` | `S t=14 y=0111` | `S t=15 y=0111` | `A end`. Kills mutant(s) M1 M2.
/// Cell `E13_reforce_twice`.
#[test]
fn e13_reforce_twice() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #2 release y; #1 $display("S t=8 y=%b", y);
    a = 8'h04; #1 force y = 4'h7; #5 $display("S t=14 y=%b", y);
    release y; #1 $display("S t=15 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=8 y=0001", "S t=14 y=0111", "S t=15 y=0100", "A end"],
    );
}

/// In-row (wrong -> right): driver value xz01. iverilog = sv2v = the pin. verilator: `S t=6 y=0010` |
/// `S t=8 y=0001` | `A end`. PRE: `S t=6 y=0010` | `S t=8 y=0010` | `A end`. Kills mutant(s) M1 M2.
/// Cell `E14_xz_driver_value`.
#[test]
fn e14_xz_driver_value() {
    pin(
        r#"module top;
  logic [7:0] a = 8'b0000_xz01; wire [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0010", "S t=8 y=xz01", "A end"],
    );
}

/// In-row (wrong -> right): non-sole #5 part-select, release before landing. iverilog = sv2v = the pin.
/// verilator: `S t=3 w=00000000` | `S t=6 w=00000001` | `A end`. PRE: `S t=3 w=00000010` | `S t=6 w=00000001` |
/// `A end`. Kills mutant(s) M1 M2 M3 M6.
/// Cell `E23_partsel_nonsole_before_first`.
#[test]
fn e23_partsel_nonsole_before_first() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [7:0] w;
  assign w[7:4] = 4'h0;
  assign #5 w[3:0] = a;
  initial begin
    #1 force w = 8'h22; #1 release w; #1 $display("S t=3 w=%b", w);
    #3 $display("S t=6 w=%b", w);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=3 w=0000xxxx", "S t=6 w=00000001", "A end"],
    );
}

/// In-row (wrong -> right): release inside a task. iverilog = sv2v = verilator = the pin. PRE: `S t=6 y=0010` |
/// `S t=8 y=0010` | `A end`. Kills mutant(s) M1 M2.
/// Cell `E24_task_release`.
#[test]
fn e24_task_release() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  task rel; release y; endtask
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 rel; #1 $display("S t=8 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0010", "S t=8 y=0001", "A end"],
    );
}

/// In-row (wrong -> right): interface net #3, force i.y. iverilog = sv2v = verilator = the pin. PRE:
/// `S t=6 y=0010` | `S t=8 y=0010` | `A end`. Kills mutant(s) M1 M2.
/// Cell `E25_iface_net`.
#[test]
fn e25_iface_net() {
    pin(
        r#"interface ifc;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
endinterface
module top;
  ifc i();
  initial begin
    #5 force i.y = 4'h2; #1 $display("S t=6 y=%b", i.y);
    #1 release i.y; #1 $display("S t=8 y=%b", i.y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0010", "S t=8 y=0001", "A end"],
    );
}

/// In-row (wrong -> right): #(3,2) pending at release. iverilog = verilator = the pin. sv2v:
/// `(no sv2v->iverilog binary)` | `s2v.err:t.sv:3:13: Parse error: unexpected token ',' (Sym_comma)`. PRE:
/// `S t=6 y=0010` | `S t=8 y=0010` | `S t=10 y=0011` | `A end`. Kills mutant(s) M1 M2 M3.
/// Cell `E26_rf_pending`.
#[test]
fn e26_rf_pending() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #(3,2) y = a;
  initial begin
    #5 force y = 4'h2; #1 a = 8'h03; $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #2 $display("S t=10 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0010", "S t=8 y=0001", "S t=10 y=0011", "A end"],
    );
}

/// In-row (wrong -> right): #3, a=03; release y same step. iverilog = sv2v = verilator = the pin. PRE:
/// `S t=6 y=0010` | `S t=8 y=0010` | `S t=11 y=0011` | `A end`. Kills mutant(s) M1 M2.
/// Cell `E28_change_then_release_same_step`.
#[test]
fn e28_change_then_release_same_step() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 a = 8'h03; release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0010", "S t=8 y=0001", "S t=11 y=0011", "A end"],
    );
}

/// In-row (wrong -> right): assign #(d) runtime delay. iverilog = sv2v = verilator = the pin. PRE:
/// `S t=6 y=0010` | `S t=8 y=0010` | `S t=11 y=0010` | `S t=20 y=0011` | `A end`. Kills mutant(s) M1 M2.
/// Cell `E30_runtime_delay`.
#[test]
fn e30_runtime_delay() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y; int d = 3;
  assign #(d) y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #1 a = 8'h03; #8 $display("S t=20 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "S t=6 y=0010",
            "S t=8 y=0001",
            "S t=11 y=0001",
            "S t=20 y=0011",
            "A end",
        ],
    );
}

/// In-row (wrong -> right): 100-bit net. iverilog = sv2v = verilator = the pin. PRE:
/// `S t=6 y=0000000000000000000000002` | `S t=8 y=0000000000000000000000002` | `A end`. Kills mutant(s) M1 M2.
/// Cell `E32_wide_net`.
#[test]
fn e32_wide_net() {
    pin(
        r#"module top;
  logic [99:0] a = 100'h1; wire [99:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 100'h2; #1 $display("S t=6 y=%h", y);
    #1 release y; #1 $display("S t=8 y=%h", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "S t=6 y=0000000000000000000000002",
            "S t=8 y=0000000000000000000000001",
            "A end",
        ],
    );
}

/// In-row (wrong -> right): pending at release, inertial cancel after. iverilog = sv2v = the pin. verilator:
/// `W t=0 y=0000` | `W t=3 y=0001` | `W t=5 y=0010` | `W t=7 y=0001` | `S t=9 y=0001` | `S t=12 y=0001` |
/// `A end`. PRE: `W t=3 y=0001` | `W t=5 y=0010` | `S t=9 y=0010` | `W t=11 y=0001` | `S t=12 y=0001` |
/// `A end`. Kills mutant(s) M1 M2 M3.
/// Cell `E33_rel_pending_cancel`.
#[test]
fn e33_rel_pending_cancel() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  always @(y) $display("W t=%0t y=%b", $time, y);
  initial begin
    #5 force y = 4'h2; #1 a = 8'h03; #1 release y; #1 a = 8'h01; #1 $display("S t=9 y=%b", y);
    #3 $display("S t=12 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "W t=3 y=0001",
            "W t=5 y=0010",
            "W t=7 y=0001",
            "S t=9 y=0001",
            "S t=12 y=0001",
            "A end",
        ],
    );
}

/// In-row (wrong -> right): two delayed part-select drivers. iverilog = sv2v = verilator = the pin. PRE:
/// `S t=6 w=00100010` | `S t=8 w=00100010` | `A end`. Kills mutant(s) M1 M2 M4.
/// Cell `E34_two_partsel_both_delayed`.
#[test]
fn e34_two_partsel_both_delayed() {
    pin(
        r#"module top;
  logic [3:0] a = 4'h1, b = 4'h5; wire [7:0] w;
  assign #3 w[3:0] = a;
  assign #2 w[7:4] = b;
  initial begin
    #5 force w = 8'h22; #1 $display("S t=6 w=%b", w);
    #1 release w; #1 $display("S t=8 w=%b", w);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 w=00100010", "S t=8 w=01010001", "A end"],
    );
}

/// In-row (wrong -> right): bufif1 #3, output z. iverilog = the pin. sv2v: `(no sv2v->iverilog binary)` |
/// `s2v.err:t.sv:3:3: Parse error: missing expected `endmodule``. verilator: `(no verilator binary)` |
/// `%Error-UNSUPPORTED: t.sv:5:14: Unsupported tristate construct (not in propagation graph): VARREF 'y'` |
/// `%Error-UNSUPPORTED: t.sv:6:16: Unsupported tristate construct (not in propagation graph): VARREF 'y'` |
/// `%Error: Exiting due to 2 error(s)`. PRE: `S t=6 y=0` | `S t=8 y=0` | `S t=13 y=1` | `A end`. Kills
/// mutant(s) M1 M2.
/// Cell `E35_bufif_delay_z`.
#[test]
fn e35_bufif_delay_z() {
    pin(
        r#"module top;
  logic a = 1'b1, en = 1'b0; wire y;
  bufif1 #3 g(y, a, en);
  initial begin
    #5 force y = 1'b0; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    en = 1'b1; #5 $display("S t=13 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0", "S t=8 y=z", "S t=13 y=1", "A end"],
    );
}

/// In-row (wrong -> right): landing while forced, release after. iverilog = sv2v = verilator = the pin. PRE:
/// `S t=10 y=0010` | `S t=11 y=0010` | `A end`. Kills mutant(s) M1 M2.
/// Cell `E36_landing_under_force`.
#[test]
fn e36_landing_under_force() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; a = 8'h03; #5 $display("S t=10 y=%b", y);
    release y; #1 $display("S t=11 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=10 y=0010", "S t=11 y=0011", "A end"],
    );
}

/// In-row (wrong -> right): logic var, landing at the release tick. iverilog = sv2v = the pin. verilator:
/// `W t=0 y=0000` | `W t=2 y=0010` | `W t=7 y=0011` | `S t=8 y=0011` | `A end`. PRE: `W t=2 y=0010` |
/// `S t=8 y=0010` | `A end`. Kills mutant(s) M1 M2.
/// Cell `E37_var_samestep_relfirst`.
#[test]
fn e37_var_samestep_relfirst() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; logic [3:0] y;
  assign #3 y = a;
  always @(y) $display("W t=%0t y=%b", $time, y);
  initial begin
    #2 force y = 4'h2; #2 a = 8'h03; #3 release y; #1 $display("S t=8 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["W t=2 y=0010", "W t=7 y=0011", "S t=8 y=0011", "A end"],
    );
}

/// In-row (wrong -> right): @(y) events at release. iverilog = sv2v = the pin. verilator: `W t=0 y=0000` |
/// `W t=3 y=0001` | `W t=5 y=0010` | `W t=7 y=0001` | `A end`. PRE: `W t=3 y=0001` | `W t=5 y=0010` | `A end`.
/// Kills mutant(s) M1 M2.
/// Cell `E39_evt_count`.
#[test]
fn e39_evt_count() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  always @(y) $display("W t=%0t y=%b", $time, y);
  initial begin
    #5 force y = 4'h2; #2 release y; #4
    $display("A end"); $finish;
  end
endmodule
"#,
        &["W t=3 y=0001", "W t=5 y=0010", "W t=7 y=0001", "A end"],
    );
}

/// In-row (wrong -> right): read right after release (same process). iverilog = sv2v = verilator = the pin.
/// PRE: `S t=6 y=0010` | `S t=7imm y=0010` | `S t=8 y=0010` | `A end`. Kills mutant(s) M1 M2.
/// Cell `E41_imm_read_delayed`.
#[test]
fn e41_imm_read_delayed() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; $display("S t=7imm y=%b", y); #1 $display("S t=8 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0010", "S t=7imm y=0001", "S t=8 y=0001", "A end"],
    );
}

/// In-row (wrong -> right): same, logic var delayed. iverilog = sv2v = verilator = the pin. PRE: `S t=6 y=0010`
/// | `S t=7imm y=0010` | `S t=8 y=0010` | `A end`. Kills mutant(s) M1 M2.
/// Cell `E43_imm_read_var_delayed`.
#[test]
fn e43_imm_read_var_delayed() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; logic [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; $display("S t=7imm y=%b", y); #1 $display("S t=8 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0010", "S t=7imm y=0001", "S t=8 y=0001", "A end"],
    );
}

/// In-row (wrong -> right): release; #0 read. iverilog = sv2v = verilator = the pin. PRE: `S t=6 y=0010` |
/// `S t=7d0 y=0010` | `S t=8 y=0010` | `A end`. Kills mutant(s) M1 M2.
/// Cell `E44_d0_read_after_release`.
#[test]
fn e44_d0_read_after_release() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #0 $display("S t=7d0 y=%b", y); #1 $display("S t=8 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0010", "S t=7d0 y=0001", "S t=8 y=0001", "A end"],
    );
}

/// In-row (wrong -> right): other process same step after release. iverilog = sv2v = verilator = the pin. PRE:
/// `S t=7other y=0010` | `S t=8 y=0010` | `A end`. Kills mutant(s) M1 M2.
/// Cell `E46_other_proc_same_step`.
#[test]
fn e46_other_proc_same_step() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #2 release y; #1 $display("S t=8 y=%b", y);
    $display("A end"); $finish;
  end
  initial begin #6; #1 $display("S t=7other y=%b", y); end
endmodule
"#,
        &["S t=7other y=0001", "S t=8 y=0001", "A end"],
    );
}

/// In-row (wrong -> right): release; a=03 same step, @(y). iverilog = sv2v = the pin. verilator: `W t=0 y=0000`
/// | `W t=3 y=0001` | `W t=5 y=0010` | `W t=7 y=0001` | `S t=8 y=0001` | `W t=10 y=0011` | `S t=11 y=0011` |
/// `A end`. PRE: `W t=3 y=0001` | `W t=5 y=0010` | `S t=8 y=0010` | `W t=10 y=0011` | `S t=11 y=0011` |
/// `A end`. Kills mutant(s) M1 M2.
/// Cell `E49_d3_release_then_change_events`.
#[test]
fn e49_d3_release_then_change_events() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  always @(y) $display("W t=%0t y=%b", $time, y);
  initial begin
    #5 force y = 4'h2; #2 release y; a = 8'h03; #1 $display("S t=8 y=%b", y); #3 $display("S t=11 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "W t=3 y=0001",
            "W t=5 y=0010",
            "W t=7 y=0001",
            "S t=8 y=0001",
            "W t=10 y=0011",
            "S t=11 y=0011",
            "A end",
        ],
    );
}

/// In-row (wrong -> right): sole #5 driver, force and release before the first landing, read right after the
/// release. iverilog = sv2v = the pin. verilator: `S t=2imm y=0000` | `S t=3 y=0000` | `S t=6 y=0001` |
/// `A end`. PRE: `S t=2imm y=0010` | `S t=3 y=xxxx` | `S t=6 y=0001` | `A end`. Kills mutant(s) M1 M2 M3 M6.
/// Cell `R01_sole_before_first_imm`.
#[test]
fn r01_sole_before_first_imm() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #5 y = a;
  initial begin
    #1 force y = 4'h2; #1 release y; $display("S t=2imm y=%b", y); #1 $display("S t=3 y=%b", y);
    #3 $display("S t=6 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=2imm y=xxxx", "S t=3 y=xxxx", "S t=6 y=0001", "A end"],
    );
}

/// In-row (wrong -> right): generate loop: one #3 driver per bit. iverilog = sv2v = verilator = the pin. PRE:
/// `S t=7imm w=1010` | `S t=8 w=1010` | `A end`. Kills mutant(s) M1 M2 M4.
/// Cell `R05_genvar_bits_delayed`.
#[test]
fn r05_genvar_bits_delayed() {
    pin(
        r#"module top;
  logic [3:0] a = 4'h5; wire [3:0] w;
  for (genvar g = 0; g < 4; g++) begin : G
    assign #3 w[g] = a[g];
  end
  initial begin
    #5 force w = 4'ha; #2 release w; $display("S t=7imm w=%b", w); #1 $display("S t=8 w=%b", w);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=7imm w=0101", "S t=8 w=0101", "A end"],
    );
}

/// In-row (wrong -> right): net #3 with $dumpvars (text lines only). iverilog = sv2v = the pin. verilator:
/// `-Info: t.sv:5: $dumpvar ignored, as Verilated without --trace` | `S t=8 y=0001` | `S t=14 y=0011` |
/// `A end`. PRE: `S t=8 y=0010` | `S t=14 y=0011` | `A end`. Kills mutant(s) M1 M2. Text lines only (the VCD is
/// compared by the row's run, not here).
/// Cell `R17_vcd_delayed_release`.
#[test]
fn r17_vcd_delayed_release() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  initial begin
    $dumpfile("d.vcd"); $dumpvars(0, top);
    #5 force y = 4'h2; #2 release y; #1 $display("S t=8 y=%b", y);
    #2 a = 8'h13; #4 $display("S t=14 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=8 y=0001", "S t=14 y=0011", "A end"],
    );
}

/// In-row (wrong -> right): release twice. iverilog = sv2v = the pin. verilator: `W t=0 y=0000` |
/// `W t=3 y=0001` | `W t=5 y=0010` | `W t=7 y=0001` | `S t=9 y=0001` | `W t=10 y=0011` | `S t=11 y=0011` |
/// `A end`. PRE: `W t=3 y=0001` | `W t=5 y=0010` | `S t=9 y=0010` | `W t=10 y=0011` | `S t=11 y=0011` |
/// `A end`. Kills mutant(s) M1 M2.
/// Cell `R20_release_twice`.
#[test]
fn r20_release_twice() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  always @(y) $display("W t=%0t y=%b", $time, y);
  initial begin
    #5 force y = 4'h2; #2 release y; a = 8'h03; #1 release y; #1 $display("S t=9 y=%b", y);
    #2 $display("S t=11 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "W t=3 y=0001",
            "W t=5 y=0010",
            "W t=7 y=0001",
            "S t=9 y=0001",
            "W t=10 y=0011",
            "S t=11 y=0011",
            "A end",
        ],
    );
}

/// In-row (wrong -> right): assign #3 {y, v}: net y and logic variable v, released one at a time. iverilog =
/// sv2v = verilator = the pin. PRE: `S t=7imm y=0111 v=1000` | `S t=8 y=0111 v=1000` | `S t=8imm y=0111 v=1000`
/// | `A end`. Kills mutant(s) M1 M2 M19e.
/// Cell `R21_cat_var_net_release`.
#[test]
fn r21_cat_var_net_release() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h12; wire [3:0] y; logic [3:0] v;
  assign #3 {y, v} = a;
  initial begin
    #5 force y = 4'h7; force v = 4'h8; #2 release v; $display("S t=7imm y=%b v=%b", y, v);
    #1 $display("S t=8 y=%b v=%b", y, v); release y; $display("S t=8imm y=%b v=%b", y, v);
    $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "S t=7imm y=0111 v=0010",
            "S t=8 y=0111 v=0010",
            "S t=8imm y=0001 v=0010",
            "A end",
        ],
    );
}

/// In-row (wrong -> right): child-local force / release of its own #3 output net. iverilog = sv2v = verilator =
/// the pin. PRE: `S t=7imm o=0010` | `S t=8 o=0010` | `S t=9 p=0010` | `A end`. Kills mutant(s) M1 M2.
/// Cell `R23_child_local_force_delayed`.
#[test]
fn r23_child_local_force_delayed() {
    pin(
        r#"module child(input logic [7:0] i, output wire [3:0] o);
  assign #3 o = i;
  initial begin #5 force o = 4'h2; #2 release o; $display("S t=7imm o=%b", o); #1 $display("S t=8 o=%b", o); end
endmodule
module top;
  logic [7:0] a = 8'h01; wire [3:0] p;
  child u(.i(a), .o(p));
  initial begin #9 $display("S t=9 p=%b", p); $display("A end"); $finish; end
endmodule
"#,
        &["S t=7imm o=0001", "S t=8 o=0001", "S t=9 p=0001", "A end"],
    );
}

/// Control (PRE = the pin; the row must not move it): net plain (control). iverilog = sv2v = verilator = the
/// pin.
/// Cell `U02_rel_plain_wide_dropbits`.
#[test]
fn u02_rel_plain_wide_dropbits() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 8'h11; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "S t=6 y=0010",
            "S t=8 y=0001",
            "S t=11 y=0001",
            "S t=20 y=0001",
            "A end",
        ],
    );
}

/// Control (PRE = the pin; the row must not move it): release before first landing, sole. iverilog = sv2v = the
/// pin. verilator: `S t=3 y=0000` | `S t=6 y=0001` | `A end`.
/// Cell `P02_rel_before_first_landing`.
#[test]
fn p02_rel_before_first_landing() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #5 y = a;
  initial begin
    #1 force y = 4'h2; #1 release y; #1 $display("S t=3 y=%b", y);
    #3 $display("S t=6 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=3 y=xxxx", "S t=6 y=0001", "A end"],
    );
}

/// Control (PRE = the pin; the row must not move it): force value = new rhs. iverilog = sv2v = verilator = the
/// pin.
/// Cell `P06_force_eq_newrhs_then_rel`.
#[test]
fn p06_force_eq_newrhs_then_rel() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h3; #1 a = 8'h03; $display("S t=6 y=%b", y);
    #6 release y; #1 $display("S t=13 y=%b", y);
    #3 $display("S t=16 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0011", "S t=13 y=0011", "S t=16 y=0011", "A end"],
    );
}

/// Control (PRE = the pin; the row must not move it): force+release at t=0, sole, before landing. iverilog =
/// sv2v = the pin. verilator: `S t=1 y=0000` | `S t=4 y=0001` | `A end`.
/// Cell `E01_rel_t0_sole_x`.
#[test]
fn e01_rel_t0_sole_x() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  initial begin
    force y = 4'h2; release y; #1 $display("S t=1 y=%b", y);
    #3 $display("S t=4 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y=xxxx", "S t=4 y=0001", "A end"],
    );
}

/// Control (PRE = the pin; the row must not move it): release of an unforced net, pending landing. iverilog =
/// sv2v = the pin. verilator: `W t=0 y=0000` | `W t=3 y=0001` | `S t=7 y=0001` | `W t=8 y=0011` |
/// `S t=10 y=0011` | `A end`.
/// Cell `E05_rel_unforced_pending`.
#[test]
fn e05_rel_unforced_pending() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  always @(y) $display("W t=%0t y=%b", $time, y);
  initial begin
    #5 a = 8'h03; #1 release y; #1 $display("S t=7 y=%b", y);
    #3 $display("S t=10 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "W t=3 y=0001",
            "S t=7 y=0001",
            "W t=8 y=0011",
            "S t=10 y=0011",
            "A end",
        ],
    );
}

/// Control (PRE = the pin; the row must not move it): legal deassign. iverilog = sv2v = verilator = the pin.
/// Cell `E20_deassign_control`.
#[test]
fn e20_deassign_control() {
    pin(
        r#"module top;
  logic [3:0] y = 4'h1;
  initial begin
    #5 assign y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 deassign y; #1 $display("S t=8 y=%b", y);
    y = 4'h3; #1 $display("S t=9 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0010", "S t=8 y=0010", "S t=9 y=0011", "A end"],
    );
}

/// Control (PRE = the pin; the row must not move it): plain CA to logic var. iverilog = verilator = the pin.
/// sv2v: `S t=6 y=0010` | `S t=8 y=0010` | `S t=11 y=0010` | `A end`.
/// Cell `E21_plain_var_control`.
#[test]
fn e21_plain_var_control() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; logic [3:0] y;
  assign y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0010", "S t=8 y=0001", "S t=11 y=0001", "A end"],
    );
}

/// Control (PRE = the pin; the row must not move it): var without CA, force/release. iverilog = sv2v =
/// verilator = the pin.
/// Cell `E22_var_no_ca_control`.
#[test]
fn e22_var_no_ca_control() {
    pin(
        r#"module top;
  logic [3:0] y = 4'h1;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    y = 4'h3; #1 $display("S t=9 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0010", "S t=8 y=0010", "S t=9 y=0011", "A end"],
    );
}

/// Control (PRE = the pin; the row must not move it): #0, a=03; release y same step. iverilog = sv2v =
/// verilator = the pin.
/// Cell `E27_d0_samestep_change`.
#[test]
fn e27_d0_samestep_change() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #0 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 a = 8'h03; release y; #1 $display("S t=8 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6 y=0010", "S t=8 y=0011", "A end"],
    );
}

/// Control (PRE = the pin; the row must not move it): logic var, release before landing. iverilog = sv2v = the
/// pin. verilator: `S t=3 y=0000` | `S t=6 y=0001` | `A end`.
/// Cell `E38_var_before_first`.
#[test]
fn e38_var_before_first() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; logic [3:0] y;
  assign #5 y = a;
  initial begin
    #1 force y = 4'h2; #1 release y; #1 $display("S t=3 y=%b", y);
    #3 $display("S t=6 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=3 y=xxxx", "S t=6 y=0001", "A end"],
    );
}

/// Control (PRE = the pin; the row must not move it): force value = the landed output 0001, release: no new
/// event at the release. iverilog = sv2v = the pin. verilator: `W t=0 y=0000` | `W t=3 y=0001` | `S t=8 y=0001`
/// | `A end`.
/// Cell `R19_release_forced_eq_output_noevent`.
#[test]
fn r19_release_forced_eq_output_noevent() {
    pin(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  always @(y) $display("W t=%0t y=%b", $time, y);
  initial begin
    #5 force y = 4'h1; #2 release y; #1 $display("S t=8 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["W t=3 y=0001", "S t=8 y=0001", "A end"],
    );
}

/// ENG-1c rebase guard: the t=20 line of `U01_rel_delayed_wide_dropbits` (net #3, rhs 8 -> 4 bits; the value a
/// later rhs change lands after the release). iverilog = sv2v = verilator = the pin. PRE: `S t=20 y=0001`.
/// Cell `U01_rel_delayed_wide_dropbits`, line `S t=20`.
#[test]
fn u01_rel_delayed_wide_dropbits_t20_line() {
    pin_lines(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 8'h11; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=20 "],
        &["S t=20 y=0001"],
    );
}

/// ENG-1c rebase guard: the t=20 line of `U04_rel_delayed_logicvar_dropbits` (logic var #3; the value a later
/// rhs change lands after the release). iverilog = sv2v = verilator = the pin. PRE: `S t=20 y=0001`.
/// Cell `U04_rel_delayed_logicvar_dropbits`, line `S t=20`.
#[test]
fn u04_rel_delayed_logicvar_dropbits_t20_line() {
    pin_lines(
        r#"module top;
  logic [7:0] a = 8'h01; logic [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 8'h11; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=20 "],
        &["S t=20 y=0001"],
    );
}

/// ENG-1c rebase guard: the t=20 line of `U05_rel_delayed_rf_dropbits` (net #(3,2); the value a later rhs
/// change lands after the release). iverilog = verilator = the pin. sv2v: (none). PRE: `S t=20 y=0001`.
/// Cell `U05_rel_delayed_rf_dropbits`, line `S t=20`.
#[test]
fn u05_rel_delayed_rf_dropbits_t20_line() {
    pin_lines(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #(3,2) y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 8'h11; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=20 "],
        &["S t=20 y=0001"],
    );
}

/// ENG-1c rebase guard: the t=20 line of `U06_rel_delayed_real_rhs_sameimage` (real rhs -> 4-bit net; the value
/// a later rhs change lands after the release). iverilog = sv2v = verilator = the pin. PRE: `S t=20 y=0001`.
/// Cell `U06_rel_delayed_real_rhs_sameimage`, line `S t=20`.
#[test]
fn u06_rel_delayed_real_rhs_sameimage_t20_line() {
    pin_lines(
        r#"module top;
  real r = 1.0; wire [3:0] y;
  assign #3 y = r;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 r = 1.25; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=20 "],
        &["S t=20 y=0001"],
    );
}

/// ENG-1c rebase guard: the t=20 line of `U10_rel_delayed_d0_dropbits` (net #0; the value a later rhs change
/// lands after the release). iverilog = sv2v = verilator = the pin. PRE: `S t=20 y=0001`.
/// Cell `U10_rel_delayed_d0_dropbits`, line `S t=20`.
#[test]
fn u10_rel_delayed_d0_dropbits_t20_line() {
    pin_lines(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #0 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 8'h11; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=20 "],
        &["S t=20 y=0001"],
    );
}

/// ENG-1c rebase guard: the t=20 line of `U11_rel_netdecl_delay_dropbits` (wire [3:0] #3 y = a; the value a
/// later rhs change lands after the release). iverilog = verilator = the pin. sv2v: (none). PRE:
/// `S t=20 y=0001`.
/// Cell `U11_rel_netdecl_delay_dropbits`, line `S t=20`.
#[test]
fn u11_rel_netdecl_delay_dropbits_t20_line() {
    pin_lines(
        r#"module top;
  logic [7:0] a = 8'h01;
  wire [3:0] #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 8'h11; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=20 "],
        &["S t=20 y=0001"],
    );
}

/// ENG-1c rebase guard: the t=20 line of `U12_rel_delayed_sel_dropbits` (#3 part-select + plain part-select;
/// the value a later rhs change lands after the release). iverilog = sv2v = verilator = the pin. PRE:
/// `S t=20 w=00000001`.
/// Cell `U12_rel_delayed_sel_dropbits`, line `S t=20`.
#[test]
fn u12_rel_delayed_sel_dropbits_t20_line() {
    pin_lines(
        r#"module top;
  logic [7:0] a = 8'h01; wire [7:0] w;
  assign w[7:4] = 4'h0;
  assign #3 w[3:0] = a;
  initial begin
    #5 force w = 8'h02; #1 $display("S t=6 w=%b", w);
    #1 release w; #1 $display("S t=8 w=%b", w);
    #3 $display("S t=11 w=%b", w);
    #4 a = 8'h11; #5 $display("S t=20 w=%b", w); $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=20 "],
        &["S t=20 w=00000001"],
    );
}

/// Lines `S t=8` only: delayed + plain part-selects, read right after release. Its `S t=7imm` line (plain bits
/// w[7:4] still forced until the settle) is Q-RELIMM: never pinned. iverilog = sv2v = verilator = the pin. PRE:
/// `S t=8 w=00000010`.
/// Cell `E51_imm_read_mixed_partsel`.
#[test]
fn e51_imm_read_mixed_partsel_lines() {
    pin_lines(
        r#"module top;
  logic [7:0] a = 8'h01; wire [7:0] w;
  assign w[7:4] = 4'h0;
  assign #3 w[3:0] = a;
  initial begin
    #5 force w = 8'h22; #2 release w; $display("S t=7imm w=%b", w); #1 $display("S t=8 w=%b", w);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=8 "],
        &["S t=8 w=00000001"],
    );
}

/// Lines `S t=8`, `S t=12` only: delayed part-select w[P+1:P-2] (parameter expression) beside a plain
/// part-select. A legal constant select (IEEE 1800-2017 Table 10-1): no select exclusion. Its `S t=7imm` line
/// is Q-RELIMM: never pinned. iverilog = sv2v = verilator = the pin. PRE: `S t=8 w=00000010` |
/// `S t=12 w=00000010`. Kills mutant(s) M9.
/// Cell `R03_const_param_expr_sel`.
#[test]
fn r03_const_param_expr_sel_lines() {
    pin_lines(
        r#"module top;
  parameter P = 2;
  logic [7:0] a = 8'h01; wire [7:0] w;
  assign w[7:4] = 4'h0;
  assign #3 w[P+1:P-2] = a;
  initial begin
    #5 force w = 8'h22; #2 release w; $display("S t=7imm w=%b", w); #1 $display("S t=8 w=%b", w);
    #4 $display("S t=12 w=%b", w);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=8 ", "S t=12 "],
        &["S t=8 w=00000001", "S t=12 w=00000001"],
    );
}

/// Lines `S t=8`, `S t=12` only: delayed indexed part-select w[P-2 +: 4] beside a plain part-select. A legal
/// constant indexed select (Table 10-1): no select exclusion. Its `S t=7imm` line is Q-RELIMM: never pinned.
/// iverilog = sv2v = verilator = the pin. PRE: `S t=8 w=00000010` | `S t=12 w=00000010`. Kills mutant(s) M9.
/// Cell `R04_const_indexed_param`.
#[test]
fn r04_const_indexed_param_lines() {
    pin_lines(
        r#"module top;
  parameter P = 2;
  logic [7:0] a = 8'h01; wire [7:0] w;
  assign w[7:4] = 4'h0;
  assign #3 w[P-2 +: 4] = a;
  initial begin
    #5 force w = 8'h22; #2 release w; $display("S t=7imm w=%b", w); #1 $display("S t=8 w=%b", w);
    #4 $display("S t=12 w=%b", w);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=8 ", "S t=12 "],
        &["S t=8 w=00000001", "S t=12 w=00000001"],
    );
}

/// Lines `S t=8`, `S t=12` only: delayed part-select w[L-1:0], L = $clog2(16). Its `S t=7imm` line is Q-RELIMM:
/// never pinned. iverilog = sv2v = verilator = the pin. PRE: `S t=8 w=00000010` | `S t=12 w=00000010`.
/// Cell `R06_const_clog2_sel`.
#[test]
fn r06_const_clog2_sel_lines() {
    pin_lines(
        r#"module top;
  localparam L = $clog2(16);
  logic [7:0] a = 8'h01; wire [7:0] w;
  assign w[7:4] = 4'h0;
  assign #3 w[L-1:0] = a;
  initial begin
    #5 force w = 8'h22; #2 release w; $display("S t=7imm w=%b", w); #1 $display("S t=8 w=%b", w);
    #4 $display("S t=12 w=%b", w);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=8 ", "S t=12 "],
        &["S t=8 w=00000001", "S t=12 w=00000001"],
    );
}

/// Lines `S t=8`, `S t=12` only: delayed part-select w[f(3):0], f a constant function. Its `S t=7imm` line is
/// Q-RELIMM: never pinned. iverilog = sv2v = verilator = the pin. PRE: `S t=8 w=00000010` |
/// `S t=12 w=00000010`.
/// Cell `R07_const_fn_sel`.
#[test]
fn r07_const_fn_sel_lines() {
    pin_lines(
        r#"module top;
  function automatic int f(input int x); return x; endfunction
  logic [7:0] a = 8'h01; wire [7:0] w;
  assign w[7:4] = 4'h0;
  assign #3 w[f(3):0] = a;
  initial begin
    #5 force w = 8'h22; #2 release w; $display("S t=7imm w=%b", w); #1 $display("S t=8 w=%b", w);
    #4 $display("S t=12 w=%b", w);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=8 ", "S t=12 "],
        &["S t=8 w=00000001", "S t=12 w=00000001"],
    );
}

/// Lines `S t=8`, `S t=12` only: delayed part-select w[B:0], B a localparam expression. Its `S t=7imm` line is
/// Q-RELIMM: never pinned. iverilog = sv2v = verilator = the pin. PRE: `S t=8 w=00000010` |
/// `S t=12 w=00000010`.
/// Cell `R08_const_localparam_bit`.
#[test]
fn r08_const_localparam_bit_lines() {
    pin_lines(
        r#"module top;
  localparam int B = 1 + 2;
  logic [7:0] a = 8'h01; wire [7:0] w;
  assign w[7:4] = 4'h0;
  assign #3 w[B:0] = a;
  initial begin
    #5 force w = 8'h22; #2 release w; $display("S t=7imm w=%b", w); #1 $display("S t=8 w=%b", w);
    #4 $display("S t=12 w=%b", w);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=8 ", "S t=12 "],
        &["S t=8 w=00000001", "S t=12 w=00000001"],
    );
}

/// Lines `S t=2imm` only: real rhs into a 100-bit net, sole #5 driver, force t=1 / release t=2. The release
/// writes the owed x at the lvalue width without evaluating the rhs (iverilog: all x). Its t=1 / t=3 lines are
/// N-R01 (the settle sizes the owed x of a real rhs at 64 bits): never pinned. iverilog = sv2v = the pin.
/// verilator: `S t=2imm y=0000000000000000000000000`. PRE: `S t=2imm y=0000000000000000000000002`.
/// Cell `R28_real_wide_before_first`.
#[test]
fn r28_real_wide_before_first_lines() {
    pin_lines(
        r#"module top;
  real r = 1.0; wire [99:0] y;
  assign #5 y = r;
  initial begin
    #1 $display("S t=1 y=%h", y);
    force y = 100'h2; #1 release y; $display("S t=2imm y=%h", y); #1 $display("S t=3 y=%h", y);
    #3 $display("S t=6 y=%h", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=2imm "],
        &["S t=2imm y=xxxxxxxxxxxxxxxxxxxxxxxxx"],
    );
}

/// Lines `W t=7`, `S t=8` only: #0, a=03; release same step, @(y). Recorded split (IEEE-literal side adopted):
/// the release writes the driver's last landed value 0001 at once (IEEE 1800-2017 Sec. 10.6.2 "immediately"),
/// then the #0 update lands 0011. verilator prints the same t=7 pair; iverilog and sv2v print `W t=7 y=0011`
/// only. Its `W t=0` line is pre-existing: never pinned. verilator = the pin. iverilog: `W t=7 y=0011` |
/// `S t=8 y=0011`. sv2v: `W t=7 y=0011` | `S t=8 y=0011`. PRE: `W t=7 y=0011` | `S t=8 y=0011`.
/// Cell `E48_d0_samestep_events`.
#[test]
fn e48_d0_samestep_events_lines() {
    pin_lines(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #0 y = a;
  always @(y) $display("W t=%0t y=%b", $time, y);
  initial begin
    #5 force y = 4'h2; #2 a = 8'h03; release y; #1 $display("S t=8 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["W t=7 ", "S t=8 "],
        &["W t=7 y=0001", "W t=7 y=0011", "S t=8 y=0011"],
    );
}

/// Lines `W t=2`, `W t=5`, `S t=6` only: non-sole #5 part-select, release of an UNFORCED net before the first
/// landing, @(w). A release of an UNFORCED net is a no-op (no `W t=2` event; iverilog, sv2v and verilator have
/// none): the release decides nothing unless the target is pinned. Kills that gate's mutant M8 together with
/// S13; M8 turns equivalent only once BOTH Q-DCAPARTX (here: the non-sole owed x at t=0) and N-R01 (S13: the
/// sole owed x at the lvalue width) land. Its t=0 / t=3 lines (vita
/// `0000zzzz`, iverilog `0000xxxx`) are Q-DCAPARTX: never pinned. iverilog = sv2v = verilator = the pin. PRE:
/// `W t=5 w=00000001` | `S t=6 w=00000001`. Kills mutant(s) M8.
/// Cell `R02_nonsole_unforced_release_event`.
#[test]
fn r02_nonsole_unforced_release_event_lines() {
    pin_lines(
        r#"module top;
  logic [7:0] a = 8'h01; wire [7:0] w;
  assign w[7:4] = 4'h0;
  assign #5 w[3:0] = a;
  always @(w) $display("W t=%0t w=%b", $time, w);
  initial begin
    #2 release w; #1 $display("S t=3 w=%b", w); #3 $display("S t=6 w=%b", w);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["W t=2 ", "W t=5 ", "S t=6 "],
        &["W t=5 w=00000001", "S t=6 w=00000001"],
    );
}

/// Lines `W` only: E39 with an effectful delayed rhs (`assign #3 y = f(a)`, `f` prints `F`), `@(y)`, force at
/// 5, release at 7. The release restores the landed 0001 and wakes `@(y)` at t=7. iverilog = sv2v = the pin.
/// verilator: `W t=0 y=0000` | `W t=3 y=0001` | `W t=5 y=0010` | `W t=7 y=0001`. PRE: `W t=3 y=0001` |
/// `W t=5 y=0010`. The `F` (call) lines are never pinned: every settle evaluates a delayed rhs (Q-DCASETTLE),
/// so vita calls `f` 12 times here (PRE 11), iverilog and sv2v once, verilator 3 times. native falls back to vm
/// (W4030) on this design.
/// Probe `O3a_e39_display_fn` (Q-RELDELAY r1, O3).
#[test]
fn o3a_effectful_delayed_rhs_event_lines() {
    pin_lines(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  function automatic [3:0] f(input [7:0] x); $display("F t=%0t x=%b", $time, x); return x[3:0]; endfunction
  assign #3 y = f(a);
  always @(y) $display("W t=%0t y=%b", $time, y);
  initial begin
    #5 force y = 4'h2; #2 release y; #4
    $display("A end"); $finish;
  end
endmodule
"#,
        &["W "],
        &["W t=3 y=0001", "W t=5 y=0010", "W t=7 y=0001"],
    );
}

/// The staged chain (`vita vcmp` -> `velab` -> `vrun`) prints the one-shot lines of
/// `U01_rel_delayed_wide_dropbits` (= iverilog). PRE (one-shot): `S t=6 y=0010` | `S t=8 y=0010` |
/// `S t=11 y=0010` | `S t=20 y=0001` | `A end`.
/// Cell `U01_rel_delayed_wide_dropbits`, staged.
#[test]
fn u01_rel_delayed_wide_dropbits_staged() {
    pin_staged(
        r#"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 8'h11; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "S t=6 y=0010",
            "S t=8 y=0001",
            "S t=11 y=0001",
            "S t=20 y=0001",
            "A end",
        ],
    );
}

/// The staged chain (`vita vcmp` -> `velab` -> `vrun`) prints the one-shot lines of
/// `E06_cat_lhs_partial_release` (= iverilog). PRE (one-shot): `S t=6 y=0111 z=1000` | `S t=8 y=0111 z=1000` |
/// `S t=11 y=0111 z=1000` | `A end`.
/// Cell `E06_cat_lhs_partial_release`, staged.
#[test]
fn e06_cat_lhs_partial_release_staged() {
    pin_staged(
        r#"module top;
  logic [7:0] a = 8'h12; wire [3:0] y, z;
  assign #3 {y, z} = a;
  initial begin
    #5 force y = 4'h7; force z = 4'h8; #1 $display("S t=6 y=%b z=%b", y, z);
    #1 release y; #1 $display("S t=8 y=%b z=%b", y, z);
    #2 release z; #1 $display("S t=11 y=%b z=%b", y, z);
    $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "S t=6 y=0111 z=1000",
            "S t=8 y=0001 z=1000",
            "S t=11 y=0001 z=0010",
            "A end",
        ],
    );
}

// ---- Q-RELDELAY round 2 (K1d): the release writes only the released net ----

/// Co-member observer, x/z-free: `assign #5 {y, z[1:0]}` beside `assign z[3:2]`, force / release y; `@(z)`
/// prints the time only. the release names one net of a delayed concatenation driver that also drives an
/// unforced co-member, before the first landing; the release writes the owed x onto the released net only
/// (round 1 also wrote it onto the co-member, an event no tool has). iverilog = sv2v = verilator = the pin. PRE
/// = the pin. Round-1 POST: `E t=2` | `E t=5` | `A end`. Kills mutant(s) M17h M17e M17n.
/// Cell `V10a_window_observer_time_only`.
#[test]
fn v10a_window_observer_time_only() {
    pin(
        r#"module top;
  logic [1:0] a = 2'b10;
  wire [1:0] y; wire [3:0] z;
  assign #5 {y, z[1:0]} = {a, a};
  assign z[3:2] = 2'b01;
  initial begin #1; forever begin @(z); $display("E t=%0t", $time); end end
  initial begin
    #1 force y = 2'b11;
    #1 release y;
    #6 $display("A end"); $finish;
  end
endmodule
"#,
        &["E t=5", "A end"],
    );
}

/// Co-member observer, x/z-free: `assign #5 {y[1:0], z}` beside `assign y[3:2]`, force / release y; `@(z)`
/// counts the changes. the release names one net of a delayed concatenation driver that also drives an unforced
/// co-member, before the first landing; the release writes the owed x onto the released net only (round 1 also
/// wrote it onto the co-member, an event no tool has). iverilog = sv2v = verilator = the pin. PRE = the pin.
/// Round-1 POST: `C n=2` | `A end`. Kills mutant(s) M17h M17e M17n.
/// Cell `V10b_window_observer_count`.
#[test]
fn v10b_window_observer_count() {
    pin(
        r#"module top;
  logic [1:0] a = 2'b10;
  wire [3:0] y; wire [1:0] z;
  integer n = 0;
  assign #5 {y[1:0], z} = {a, a};
  assign y[3:2] = 2'b01;
  initial begin #1; forever begin @(z); n = n + 1; end end
  initial begin
    #1 force y = 4'b1111;
    #1 release y;
    #6 $display("C n=%0d", n);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["C n=1", "A end"],
    );
}

/// Co-member observer, x/z-free: `assign #5 {y, z[1:0]}` beside `assign z[3:2]`, force / release y; the first
/// change of z. the release names one net of a delayed concatenation driver that also drives an unforced
/// co-member, before the first landing; the release writes the owed x onto the released net only (round 1 also
/// wrote it onto the co-member, an event no tool has). iverilog = sv2v = verilator = the pin. PRE = the pin.
/// Round-1 POST: `F t=2` | `A end`. Kills mutant(s) M17h M17e M17n.
/// Cell `V10c_window_observer_first_change`.
#[test]
fn v10c_window_observer_first_change() {
    pin(
        r#"module top;
  logic [1:0] a = 2'b10;
  wire [1:0] y; wire [3:0] z; wire q;
  assign #5 {y, z[1:0]} = {a, a};
  assign z[3:2] = 2'b01;
  assign q = (z[1:0] === 2'bzz);
  initial begin #1; @(z); $display("F t=%0t", $time); end
  initial begin
    #1 force y = 2'b11;
    #1 release y;
    #6 $display("A end"); $finish;
  end
endmodule
"#,
        &["F t=5", "A end"],
    );
}

/// Variable target, net co-member, x/z-free observer: `assign #5 {v, z[1:0]}`, force / release v. the release
/// names one net of a delayed concatenation driver that also drives an unforced co-member, before the first
/// landing; the release writes the owed x onto the released net only (round 1 also wrote it onto the co-member,
/// an event no tool has). iverilog = sv2v = verilator = the pin. PRE = the pin. Round-1 POST: `E t=2` | `E t=5`
/// | `A end`. Kills mutant(s) M17h M17e M17n.
/// Cell `W02_var_target_xzfree_observer`.
#[test]
fn w02_var_target_xzfree_observer() {
    pin(
        r#"module top;
  logic [1:0] a = 2'b10;
  logic [1:0] v; wire [3:0] z;
  assign #5 {v, z[1:0]} = {a, a};
  assign z[3:2] = 2'b01;
  initial begin #1; forever begin @(z); $display("E t=%0t", $time); end end
  initial begin
    #1 force v = 2'b11;
    #1 release v;
    #6 $display("A end"); $finish;
  end
endmodule
"#,
        &["E t=5", "A end"],
    );
}

/// Co-member bound to a child input port, observed by the child: `assign #5 {y, p[1:0]}`, force / release y.
/// the release names one net of a delayed concatenation driver that also drives an unforced co-member, before
/// the first landing; the release writes the owed x onto the released net only (round 1 also wrote it onto the
/// co-member, an event no tool has). verilator differs on x (2-state). iverilog = sv2v = the pin. verilator:
/// `S t=2imm y=00` | `C t=5` | `S t=8 y=10 p=0110` | `A end`. PRE: `S t=2imm y=11` | `C t=5` |
/// `S t=8 y=10 p=0110` | `A end`. Round-1 POST: `S t=2imm y=xx` | `C t=2` | `C t=5` | `S t=8 y=10 p=0110` |
/// `A end`. Kills mutant(s) M17h M17e M17n.
/// Cell `W04_port_comember_child_observer`.
#[test]
fn w04_port_comember_child_observer() {
    pin(
        r#"module child(input wire [3:0] i);
  initial begin #1; forever begin @(i); $display("C t=%0t", $time); end end
endmodule
module top;
  logic [1:0] a = 2'b10;
  wire [1:0] y; wire [3:0] p;
  assign #5 {y, p[1:0]} = {a, a};
  assign p[3:2] = 2'b01;
  child u(.i(p));
  initial begin
    #1 force y = 2'b11;
    #1 release y; $display("S t=2imm y=%b", y);
    #6 $display("S t=8 y=%b p=%b", y, p);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=2imm y=xx", "C t=5", "S t=8 y=10 p=0110", "A end"],
    );
}

/// Lines `Y t=`, `Z t=2`, `S t=8` only: concatenation co-member before the first landing:
/// `assign #5 {y, z[1:0]}` beside `assign z[3:2]`, force / release y. the release names one net of a delayed
/// concatenation driver that also drives an unforced co-member, before the first landing; the release writes
/// the owed x onto the released net only (round 1 also wrote it onto the co-member, an event no tool has). The
/// `Z t=0` and z value lines (vita z, iverilog x) are Q-DCAPARTX: never pinned. iverilog = sv2v = the pin.
/// verilator: `Y t=0 y=00` | `Y t=1 y=11` | `Y t=2 y=00` | `Y t=5 y=10` | `S t=8 y=10 z=0110`. PRE:
/// `Y t=1 y=11` | `Y t=5 y=10` | `S t=8 y=10 z=0110`. Round-1 POST: `Y t=1 y=11` | `Z t=2 z=01xx` |
/// `Y t=2 y=xx` | `Y t=5 y=10` | `S t=8 y=10 z=0110`. Kills mutant(s) M17h M17e M17n.
/// Cell `S01_cat_comember_owedx_event`.
#[test]
fn s01_cat_comember_owedx_event_lines() {
    pin_lines(
        r#"module top;
  logic [1:0] a = 2'b10;
  wire [1:0] y; wire [3:0] z;
  assign #5 {y, z[1:0]} = {a, a};
  assign z[3:2] = 2'b01;
  always @(z) $display("Z t=%0t z=%b", $time, z);
  always @(y) $display("Y t=%0t y=%b", $time, y);
  initial begin
    #1 force y = 2'b11;
    #1 release y; $display("S t=2imm y=%b z=%b", y, z);
    #1 $display("S t=3 y=%b z=%b", y, z);
    #5 $display("S t=8 y=%b z=%b", y, z);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["Y t=", "Z t=2", "S t=8"],
        &[
            "Y t=1 y=11",
            "Y t=2 y=xx",
            "Y t=5 y=10",
            "S t=8 y=10 z=0110",
        ],
    );
}

/// Stderr only, on native, vm and interp: the release of one net of a delayed concatenation driver (the S01
/// program) prints no `W4020` (`W-RUN-DYN-DEGRADE`). The co-member's chunk is skipped by the release; on native
/// a skipped chunk must also stay out of the dynamic-write fallback, which would print `warning[VITA-W4020]
/// W-RUN-DYN-DEGRADE: unsupported dyn lvalue shape (write ignored) [at time 2]` with stdout unchanged (lens
/// mutant LM1). POST r2 native / vm / interp: `W1017` only (`errors=0 warnings=1`). Kills mutant(s) LM1.
/// Cell `S01_cat_comember_owedx_event`.
#[test]
fn s01_cat_comember_release_no_dyn_degrade_warning() {
    let src = r#"module top;
  logic [1:0] a = 2'b10;
  wire [1:0] y; wire [3:0] z;
  assign #5 {y, z[1:0]} = {a, a};
  assign z[3:2] = 2'b01;
  always @(z) $display("Z t=%0t z=%b", $time, z);
  always @(y) $display("Y t=%0t y=%b", $time, y);
  initial begin
    #1 force y = 2'b11;
    #1 release y; $display("S t=2imm y=%b z=%b", y, z);
    #1 $display("S t=3 y=%b z=%b", y, z);
    #5 $display("S t=8 y=%b z=%b", y, z);
    $display("A end"); $finish;
  end
endmodule
"#;
    for be in ["native", "vm", "interp"] {
        let err = stderr_on(src, be);
        assert!(
            !err.contains("W4020") && !err.contains("W-RUN-DYN-DEGRADE"),
            "backend {be}: stderr\n{err}"
        );
    }
}

/// Lines `Y t=2`, `Z t=2`, `S t=8` only: target with a second driver, whole co-member: `assign #5 {y[1:0], z}`
/// beside `assign y[3:2]`, force / release y. the release names one net of a delayed concatenation driver that
/// also drives an unforced co-member, before the first landing; the release writes the owed x onto the released
/// net only (round 1 also wrote it onto the co-member, an event no tool has). The z value lines before the
/// first landing are Q-DCAPARTX: never pinned. iverilog = sv2v = the pin. verilator: `Y t=2 y=0100` |
/// `S t=8 y=0110 z=10`. PRE: `Y t=2 y=0111` | `S t=8 y=0110 z=10`. Round-1 POST: `Z t=2 z=xx` | `Y t=2 y=01xx`
/// | `S t=8 y=0110 z=10`. Kills mutant(s) M17h M17e M17n.
/// Cell `S17_cat_target_nonsole_comember_whole`.
#[test]
fn s17_cat_target_nonsole_comember_whole_lines() {
    pin_lines(
        r#"module top;
  logic [1:0] a = 2'b10;
  wire [3:0] y; wire [1:0] z;
  assign #5 {y[1:0], z} = {a, a};
  assign y[3:2] = 2'b01;
  always @(z) $display("Z t=%0t z=%b", $time, z);
  always @(y) $display("Y t=%0t y=%b", $time, y);
  initial begin
    #1 force y = 4'b1111;
    #1 release y; $display("S t=2imm y=%b z=%b", y, z);
    #6 $display("S t=8 y=%b z=%b", y, z);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["Y t=2", "Z t=2", "S t=8"],
        &["Y t=2 y=01xx", "S t=8 y=0110 z=10"],
    );
}

/// Lines `Q t=2`, `S t=8` only: a net downstream of the co-member: `assign q = (z[1:0] === 2'bzz)`, force /
/// release y. the release names one net of a delayed concatenation driver that also drives an unforced
/// co-member, before the first landing; the release writes the owed x onto the released net only (round 1 also
/// wrote it onto the co-member, an event no tool has); round 1 moved q at t=2. iverilog = sv2v = verilator =
/// the pin. PRE = the pin. Round-1 POST: `Q t=2 q=0` | `S t=8 y=10 z=0110 q=0`. Kills mutant(s) M17h M17e M17n.
/// Cell `S18_cat_comember_downstream_change`.
#[test]
fn s18_cat_comember_downstream_change_lines() {
    pin_lines(
        r#"module top;
  logic [1:0] a = 2'b10;
  wire [1:0] y; wire [3:0] z; wire q;
  assign #5 {y, z[1:0]} = {a, a};
  assign z[3:2] = 2'b01;
  assign q = (z[1:0] === 2'bzz);
  always @(q) $display("Q t=%0t q=%b", $time, q);
  initial begin
    #1 force y = 2'b11;
    #1 release y;
    #1 $display("S t=3 y=%b z=%b q=%b", y, z, q);
    #5 $display("S t=8 y=%b z=%b q=%b", y, z, q);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["Q t=2", "S t=8"],
        &["S t=8 y=10 z=0110 q=0"],
    );
}

/// Lines `Y t=2`, `Z t=2`, `S t=8` only: the released net is the co-member side: `assign #5 {y, z[1:0]}`, force
/// / release z. the release names one net of a delayed concatenation driver that also drives an unforced
/// co-member (here y), before the first landing; the release writes the owed x onto the released net only
/// (round 1 also wrote it onto the co-member, an event no tool has). Its t=0 and t=2 z value bits beside the
/// released chunk are Q-DCAPARTX. iverilog = sv2v = the pin. verilator: `Z t=2 z=0100` | `S t=8 y=10 z=0110`.
/// PRE: `Z t=2 z=0111` | `S t=8 y=10 z=0110`. Round-1 POST: `Z t=2 z=01xx` | `Y t=2 y=xx` |
/// `S t=8 y=10 z=0110`. Kills mutant(s) M17h M17e M17n.
/// Cell `V03_target_z_comember_y`.
#[test]
fn v03_target_z_comember_y_lines() {
    pin_lines(
        r#"module top;
  logic [1:0] a = 2'b10;
  wire [1:0] y; wire [3:0] z;
  assign #5 {y, z[1:0]} = {a, a};
  assign z[3:2] = 2'b01;
  always @(z) $display("Z t=%0t z=%b", $time, z);
  always @(y) $display("Y t=%0t y=%b", $time, y);
  initial begin
    #1 force z = 4'b1111;
    #1 release z; $display("S t=2imm y=%b z=%b", y, z);
    #1 $display("S t=3 y=%b z=%b", y, z);
    #5 $display("S t=8 y=%b z=%b", y, z);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["Y t=2", "Z t=2", "S t=8"],
        &["Z t=2 z=01xx", "S t=8 y=10 z=0110"],
    );
}

/// Lines `Z t=2`, `U t=2`, `Y t=2` only: three-chunk lvalue: `assign #5 {y, z[1:0], u[3:2]}`, two co-members
/// with second drivers, force / release y. the release names one net of a delayed concatenation driver that
/// also drives an unforced co-member, before the first landing; the release writes the owed x onto the released
/// net only (round 1 also wrote it onto the co-member, an event no tool has). iverilog = sv2v = the pin.
/// verilator: `Y t=2 y=00`. PRE: (none). Round-1 POST: `Z t=2 z=01xx` | `U t=2 u=xx10` | `Y t=2 y=xx`. Kills
/// mutant(s) M17h M17e M17n.
/// Cell `V06_three_member`.
#[test]
fn v06_three_member_lines() {
    pin_lines(
        r#"module top;
  logic [5:0] a = 6'b101101;
  wire [1:0] y; wire [3:0] z; wire [3:0] u;
  assign #5 {y, z[1:0], u[3:2]} = a;
  assign z[3:2] = 2'b01;
  assign u[1:0] = 2'b10;
  always @(z) $display("Z t=%0t z=%b", $time, z);
  always @(u) $display("U t=%0t u=%b", $time, u);
  always @(y) $display("Y t=%0t y=%b", $time, y);
  initial begin
    #1 force y = 2'b11;
    #1 release y; $display("S t=2imm y=%b z=%b u=%b", y, z, u);
    #1 $display("S t=3 y=%b z=%b u=%b", y, z, u);
    #5 $display("S t=8 y=%b z=%b u=%b", y, z, u);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["Z t=2", "U t=2", "Y t=2"],
        &["Y t=2 y=xx"],
    );
}

/// Lines `V t=`, `Z t=2`, `S t=8` only: variable target, net co-member: `assign #5 {v, z[1:0]}`, force /
/// release v. the release names one net of a delayed concatenation driver that also drives an unforced
/// co-member, before the first landing; the release writes the owed x onto the released net only (round 1 also
/// wrote it onto the co-member, an event no tool has). The z value lines are Q-DCAPARTX: never pinned. iverilog
/// = sv2v = the pin. verilator: `V t=0 v=00` | `V t=1 v=11` | `V t=2 v=00` | `V t=5 v=10` |
/// `S t=8 v=10 z=0110`. PRE: `V t=1 v=11` | `V t=5 v=10` | `S t=8 v=10 z=0110`. Round-1 POST: `V t=1 v=11` |
/// `Z t=2 z=01xx` | `V t=2 v=xx` | `V t=5 v=10` | `S t=8 v=10 z=0110`. Kills mutant(s) M17h M17e M17n.
/// Cell `W01_var_target_net_comember_owedx`.
#[test]
fn w01_var_target_net_comember_owedx_lines() {
    pin_lines(
        r#"module top;
  logic [1:0] a = 2'b10;
  logic [1:0] v; wire [3:0] z;
  assign #5 {v, z[1:0]} = {a, a};
  assign z[3:2] = 2'b01;
  always @(z) $display("Z t=%0t z=%b", $time, z);
  always @(v) $display("V t=%0t v=%b", $time, v);
  initial begin
    #1 force v = 2'b11;
    #1 release v; $display("S t=2imm v=%b z=%b", v, z);
    #1 $display("S t=3 v=%b z=%b", v, z);
    #5 $display("S t=8 v=%b z=%b", v, z);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["V t=", "Z t=2", "S t=8"],
        &[
            "V t=1 v=11",
            "V t=2 v=xx",
            "V t=5 v=10",
            "S t=8 v=10 z=0110",
        ],
    );
}

/// Lines `Y t=2`, `E t=` only: the released net twice in the lvalue around a co-member:
/// `assign #5 {y[3:2], z[1:0], y[1:0]}`, force / release y. the release names one net of a delayed
/// concatenation driver that also drives an unforced co-member, before the first landing; the release writes
/// the owed x onto the released net only (round 1 also wrote it onto the co-member, an event no tool has); both
/// y chunks take the owed x. iverilog = sv2v = the pin. verilator: `Y t=2 y=0000` | `E t=5`. PRE: `E t=5`.
/// Round-1 POST: `E t=2` | `Y t=2 y=xxxx` | `E t=5`. Kills mutant(s) M17h M17e M17n M18.
/// Cell `W03_target_twice_comember_owedx`.
#[test]
fn w03_target_twice_comember_owedx_lines() {
    pin_lines(
        r#"module top;
  logic [1:0] a = 2'b10;
  wire [3:0] y; wire [3:0] z;
  assign #5 {y[3:2], z[1:0], y[1:0]} = {a, a, a};
  assign z[3:2] = 2'b01;
  initial begin #1; forever begin @(z); $display("E t=%0t", $time); end end
  always @(y) $display("Y t=%0t y=%b", $time, y);
  initial begin
    #1 force y = 4'b1111;
    #1 release y; $display("S t=2imm y=%b z=%b", y, z);
    #1 $display("S t=3 y=%b z=%b", y, z);
    #5 $display("S t=8 y=%b z=%b", y, z);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["Y t=2", "E t="],
        &["Y t=2 y=xxxx", "E t=5"],
    );
}

/// The `y` value of each `S t=` line only: co-member feeding downstream logic (`assign q = ^z`,
/// `@(posedge z[0])`), force / release y before the first landing. The released net reads the owed x at t=3.
/// The z and q value lines before the first landing (vita z, iverilog x) are Q-DCAPARTX: never pinned. Kills no
/// mutant (round 1 already printed these y values; its z value line is what round 1 moved). iverilog = sv2v =
/// the pin. verilator: `S t=… y=00` | `S t=… y=10`. PRE: `S t=… y=11` | `S t=… y=10`.
/// Cell `S03_cat_comember_downstream`.
#[test]
fn s03_cat_comember_downstream_y() {
    pin_field(
        r#"module top;
  logic [1:0] a = 2'b10;
  wire [1:0] y; wire [3:0] z; wire q;
  assign #5 {y, z[1:0]} = {a, a};
  assign z[3:2] = 2'b01;
  assign q = ^z;
  always @(q) $display("Q t=%0t q=%b", $time, q);
  always @(posedge z[0]) $display("P t=%0t z0 posedge", $time);
  initial begin
    #1 force y = 2'b11;
    #1 release y;
    #1 $display("S t=3 y=%b z=%b q=%b", y, z, q);
    #5 $display("S t=8 y=%b z=%b q=%b", y, z, q);
    $display("A end"); $finish;
  end
endmodule
"#,
        "S t=",
        "y",
        &["xx", "10"],
    );
}

/// Lines `Y t=`, `Z t=2` and the `y` value of each `S t=` line only: co-member in the MOST significant chunk:
/// `assign #5 {z[1:0], y} = a` beside `assign z[3:2]`, force / release y before the first landing. The slice
/// for y is the low four bits of the owed x; no `Z t=2` event. The z value bits before the first landing are
/// Q-DCAPARTX: never pinned. iverilog = sv2v = the pin. verilator: `Y t=0 y=0000` | `Y t=1 y=1111` |
/// `Y t=2 y=0000` | `Y t=5 y=0110` | `S t=… y=0000` | `S t=… y=0000` | `S t=… y=0110`. PRE: `Y t=1 y=1111` |
/// `Y t=5 y=0110` | `S t=… y=1111` | `S t=… y=1111` | `S t=… y=0110`. Round-1 POST: `Y t=1 y=1111` |
/// `Y t=2 y=xxxx` | `Z t=2 z=01xx` | `Y t=5 y=0110` | `S t=… y=xxxx` | `S t=… y=xxxx` | `S t=… y=0110`. Kills
/// mutant(s) M17h M17e M17n.
/// Cell `A20_owedx_comember_msb_nonsole`.
#[test]
fn a20_owedx_comember_msb_nonsole_y() {
    let src = r#"module top;
  wire [3:0] y; wire [3:0] z; reg [5:0] a;
  assign #5 {z[1:0], y} = a;
  assign z[3:2] = 2'b01;
  always @(y) $display("Y t=%0t y=%b", $time, y);
  always @(z) $display("Z t=%0t z=%b", $time, z);
  initial begin
    a = 6'b10_0110;
    #1 force y = 4'hF;
    #1 release y; $display("S t=%0timm y=%b z=%b", $time, y, z);
    #1 $display("S t=%0t y=%b z=%b", $time, y, z);
    #5 $display("S t=%0t y=%b z=%b", $time, y, z);
    #1 $display("A end"); $finish;
  end
endmodule
"#;
    pin_lines(
        src,
        &["Y t=", "Z t=2"],
        &["Y t=1 y=1111", "Y t=2 y=xxxx", "Y t=5 y=0110"],
    );
    pin_field(src, "S t=", "y", &["xxxx", "xxxx", "0110"]);
}

/// Lines `S t=7imm`, `S t=8` only: the released net twice in its own lvalue: `assign #3 {y[3:0], y[7:4]} = a`,
/// force / release y after the landing. Every chunk is on y, so the skip is empty (the plain funnel write).
/// iverilog = sv2v = verilator = the pin. PRE: `S t=7imm y=00` | `S t=8 y=00`. Kills mutant(s) M18.
/// Cell `L36_same_net_twice_in_lhs`.
#[test]
fn l36_same_net_twice_in_lhs_lines() {
    pin_lines(
        r#"module top;
  logic [7:0] a = 8'h5a; wire [7:0] y;
  assign #3 {y[3:0], y[7:4]} = a;
  initial begin
    #5 force y = 8'h00; #1 $display("S t=6 y=%h", y);
    #1 release y; $display("S t=7imm y=%h", y); #1 $display("S t=8 y=%h", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=7imm", "S t=8"],
        &["S t=7imm y=a5", "S t=8 y=a5"],
    );
}

/// Lines `S t=7imm`, `S t=8`, `S t=10` only: two nets of one delayed concatenation, both forced, released in
/// one step: `assign #3 {y, z} = a`. Each release writes only its own net's slice of the landed value. iverilog
/// = sv2v = verilator = the pin. PRE: `S t=7imm y=0111 z=1000` | `S t=8 y=0111 z=1000` |
/// `S t=10 y=0011 z=0100`. Kills mutant(s) M19e.
/// Cell `P08_concat_two_released_same_step`.
#[test]
fn p08_concat_two_released_same_step_lines() {
    pin_lines(
        r#"module top;
  logic [7:0] a = 8'h12; wire [3:0] y, z;
  assign #3 {y, z} = a;
  initial begin
    #5 force y = 4'h7; force z = 4'h8; #1 a = 8'h34; #1 release y; release z; $display("S t=7imm y=%b z=%b", y, z);
    #1 $display("S t=8 y=%b z=%b", y, z); #2 $display("S t=10 y=%b z=%b", y, z);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=7imm", "S t=8", "S t=10"],
        &[
            "S t=7imm y=0001 z=0010",
            "S t=8 y=0001 z=0010",
            "S t=10 y=0011 z=0100",
        ],
    );
}

/// Real right-hand side into a concatenation, release after the landing: `assign #5 {y, z} = r`, r = 6.0. The
/// slice is taken from the converted landing (`y=01`), not from the raw 64-bit real; verilator adds a `Y t=0`
/// (2-state). iverilog = sv2v = the pin. verilator: `Y t=0 y=00` | `Y t=5 y=01` | `S t=6 y=11 z=10` |
/// `Y t=6 y=11` | `S t=7imm y=01 z=10` | `Y t=7 y=01` | `S t=8 y=01 z=10` | `A end`. PRE: `Y t=5 y=01` |
/// `S t=6 y=11 z=10` | `Y t=6 y=11` | `S t=7imm y=11 z=10` | `S t=8 y=11 z=10` | `A end`.
/// Cell `W05_real_rhs_concat_landed`.
#[test]
fn w05_real_rhs_concat_landed() {
    pin(
        r#"module top;
  real r = 6.0;
  wire [1:0] y; wire [1:0] z;
  assign #5 {y, z} = r;
  always @(y) $display("Y t=%0t y=%b", $time, y);
  initial begin
    #6 force y = 2'b11; $display("S t=6 y=%b z=%b", y, z);
    #1 release y; $display("S t=7imm y=%b z=%b", y, z);
    #1 $display("S t=8 y=%b z=%b", y, z);
    $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "Y t=5 y=01",
            "S t=6 y=11 z=10",
            "Y t=6 y=11",
            "S t=7imm y=01 z=10",
            "Y t=7 y=01",
            "S t=8 y=01 z=10",
            "A end",
        ],
    );
}

/// Control: a SOLE delayed concatenation driver, release of y before the first landing:
/// `assign #5 {y, z} = {a, a}`. z already holds x from the settle; no co-member event. iverilog and sv2v print
/// the two t=5 lines in the other order (harness-format). iverilog: `Y t=1 y=11` | `S t=2imm y=xx z=xx` |
/// `Y t=2 y=xx` | `Y t=5 y=10` | `Z t=5 z=10` | `S t=8 y=10 z=10` | `A end`. sv2v: `Y t=1 y=11` |
/// `S t=2imm y=xx z=xx` | `Y t=2 y=xx` | `Y t=5 y=10` | `Z t=5 z=10` | `S t=8 y=10 z=10` | `A end`. verilator:
/// `Z t=0 z=00` | `Y t=0 y=00` | `Y t=1 y=11` | `S t=2imm y=00 z=00` | `Y t=2 y=00` | `Z t=5 z=10` |
/// `Y t=5 y=10` | `S t=8 y=10 z=10` | `A end`. PRE: `Y t=1 y=11` | `S t=2imm y=11 z=xx` | `Y t=2 y=xx` |
/// `Z t=5 z=10` | `Y t=5 y=10` | `S t=8 y=10 z=10` | `A end`.
/// Cell `S16_cat_comember_sole_owedx`.
#[test]
fn s16_cat_comember_sole_owedx() {
    pin(
        r#"module top;
  logic [1:0] a = 2'b10;
  wire [1:0] y; wire [1:0] z;
  assign #5 {y, z} = {a, a};
  always @(z) $display("Z t=%0t z=%b", $time, z);
  always @(y) $display("Y t=%0t y=%b", $time, y);
  initial begin
    #1 force y = 2'b11;
    #1 release y; $display("S t=2imm y=%b z=%b", y, z);
    #6 $display("S t=8 y=%b z=%b", y, z);
    $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "Y t=1 y=11",
            "S t=2imm y=xx z=xx",
            "Y t=2 y=xx",
            "Z t=5 z=10",
            "Y t=5 y=10",
            "S t=8 y=10 z=10",
            "A end",
        ],
    );
}

/// Lines `Y t=`, `Z t=5`, `S t=` only: control: release of y AFTER the landing: `assign #2 {y, z[1:0]}` beside
/// `assign z[3:2]`. No co-member event at the release (no `Z t=5`). The `Z t=0` line is Q-DCAPARTX: never
/// pinned. iverilog = sv2v = the pin. verilator: `Y t=0 y=00` | `Y t=2 y=10` | `Y t=4 y=11` |
/// `S t=5imm y=10 z=0110` | `Y t=5 y=10` | `S t=6 y=10 z=0110`. PRE: `Y t=2 y=10` | `Y t=4 y=11` |
/// `S t=5imm y=11 z=0110` | `S t=6 y=11 z=0110`.
/// Cell `S02_cat_comember_landed_noevent`.
#[test]
fn s02_cat_comember_landed_noevent_lines() {
    pin_lines(
        r#"module top;
  logic [1:0] a = 2'b10;
  wire [1:0] y; wire [3:0] z;
  assign #2 {y, z[1:0]} = {a, a};
  assign z[3:2] = 2'b01;
  always @(z) $display("Z t=%0t z=%b", $time, z);
  always @(y) $display("Y t=%0t y=%b", $time, y);
  initial begin
    #4 force y = 2'b11;
    #1 release y; $display("S t=5imm y=%b z=%b", y, z);
    #1 $display("S t=6 y=%b z=%b", y, z);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["Y t=", "Z t=5", "S t="],
        &[
            "Y t=2 y=10",
            "Y t=4 y=11",
            "S t=5imm y=10 z=0110",
            "Y t=5 y=10",
            "S t=6 y=10 z=0110",
        ],
    );
}

/// Lines `W t=2`, `W t=5`, `S t=6` only: a release of an UNFORCED 100-bit net with a sole `assign #5 w = r`
/// (real) before the first landing. No `W t=2` event: an unforced release writes nothing. Kills M8 together
/// with R02; it depends on N-R01 (the settle sizes the owed x of a real right-hand side at 64 bits under the
/// 100-bit lvalue, so M8's write of the lvalue-wide x is an event), not on Q-DCAPARTX: it stops killing M8 once
/// N-R01 lands. Its `S t=2imm` / `S t=3` lines are N-R01: never pinned. iverilog = sv2v = verilator = the pin.
/// PRE = the pin. Kills mutant(s) M8.
/// Cell `S13_r28_unforced_release`.
#[test]
fn s13_r28_unforced_release_lines() {
    pin_lines(
        r#"module top;
  real r = 2.0; wire [99:0] w;
  assign #5 w = r;
  always @(w) $display("W t=%0t w=%h", $time, w);
  initial begin
    #2 release w; $display("S t=2imm w=%b", w);
    #1 $display("S t=3 w=%b", w);
    #3 $display("S t=6 w=%h", w);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["W t=2", "W t=5", "S t=6"],
        &[
            "W t=5 w=0000000000000000000000002",
            "S t=6 w=0000000000000000000000002",
        ],
    );
}

/// Lines `N t=`, `W t=5`, `W t=7`, `S t=8` only: an UNDELAYED driver, force / release, `@(negedge y[0])`. The
/// release restores only delayed drivers; an undelayed one is left to the settle (no negedge at the release).
/// iverilog = sv2v = verilator = the pin. PRE = the pin. Kills mutant(s) M16.
/// Cell `S21_m16_undelayed_negedge`.
#[test]
fn s21_m16_undelayed_negedge_lines() {
    pin_lines(
        r#"module top;
  logic [3:0] a = 4'h1; wire [3:0] y;
  assign y = a;
  always @(negedge y[0]) $display("N t=%0t y=%b", $time, y);
  always @(y) $display("W t=%0t y=%b", $time, y);
  initial begin
    #5 force y = 4'h3; #2 release y; #1 $display("S t=8 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["N t=", "W t=5", "W t=7", "S t=8"],
        &["W t=5 y=0011", "W t=7 y=0001", "S t=8 y=0001"],
    );
}

/// Lines `N t=`, `W t=7`, `S t=8` only: a delayed and an undelayed part-select driver of one net, force /
/// release, `@(negedge w[0])`. Only the delayed driver's bits are restored by the release. iverilog = sv2v =
/// verilator = the pin. PRE: `S t=8 w=00100101`. Kills mutant(s) M16.
/// Cell `S22_m16_mixed_negedge`.
#[test]
fn s22_m16_mixed_negedge_lines() {
    pin_lines(
        r#"module top;
  logic [3:0] a = 4'h1; logic [3:0] b = 4'h5; wire [7:0] w;
  assign #3 w[7:4] = a;
  assign w[3:0] = b;
  always @(negedge w[0]) $display("N t=%0t w=%b", $time, w);
  always @(w) $display("W t=%0t w=%b", $time, w);
  initial begin
    #5 force w = 8'h25; #2 release w; #1 $display("S t=8 w=%b", w);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["N t=", "W t=7", "S t=8"],
        &["W t=7 w=00010101", "S t=8 w=00010101"],
    );
}

/// Lines `S t=6imm` only: no oracle: `deassign` of a forced variable that `assign #3 v = a` drives, at t=6.
/// iverilog and sv2v refuse the program
/// (`Cannot perform procedural assignment to variable 'v' because it is also continuously assigned.`),
/// verilator `%Error-CONTASSINIT`; vita accepts it (Q-DEASSIGNCA). The deassign leaves the strong force, so the
/// restore's funnel write must drop on the still-forced v: this is the one killer of M5 (the restore lifting
/// the force flags). When Q-DEASSIGNCA makes the program loud, this pin turns loud and M5 equivalent. Its t=7 /
/// t=8 lines (after the release) are never pinned (no oracle). PRE = the pin. Kills mutant(s) M5.
/// Cell `M37_deassign_forced_var_with_ca`.
#[test]
fn m37_deassign_forced_var_with_ca_line() {
    pin_lines(
        r#"module top;
  logic [7:0] a = 8'h01; logic [3:0] v;
  assign #3 v = a;
  initial begin
    #5 force v = 4'h2; #1 deassign v; $display("S t=6imm v=%b", v);
    #1 release v; $display("S t=7imm v=%b", v); #1 $display("S t=8 v=%b", v);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=6imm"],
        &["S t=6imm v=0010"],
    );
}

/// Recorded split, the IEEE-literal side (the Q5 decision, owner flagged): `assign #0 y = a` forced to 1, at
/// t=7 `a = 1; release y;`, posedge / negedge counters. At the release the `#0` driver's propagated output is
/// still the old 0 (IEEE 1800-2017 §10.6.2 "immediately", §4.9.1: the continuous assignment is its own
/// process), so y falls and rises again in the step; iverilog and sv2v count no edge pair, as PRE. Event twins
/// P12 P14. verilator = the pin. iverilog: `S t=8 y=1 np=1 nn=0` | `A end`. sv2v: `S t=8 y=1 np=1 nn=0` |
/// `A end`. PRE: `S t=8 y=1 np=1 nn=0` | `A end`.
/// Cell `P01_d0_glitch_posedge_counter`.
#[test]
fn p01_d0_glitch_posedge_counter() {
    pin(
        r#"module top;
  logic a = 1'b0; wire y; int np = 0, nn = 0;
  assign #0 y = a;
  always @(posedge y) np++;
  always @(negedge y) nn++;
  initial begin
    #5 force y = 1'b1; #2 a = 1'b1; release y; #1 $display("S t=8 y=%b np=%0d nn=%0d", y, np, nn);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=8 y=1 np=2 nn=1", "A end"],
    );
}
