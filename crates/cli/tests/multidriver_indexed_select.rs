//! Q-MDIDXDOWN: the continuous-assignment multi-driver check (`check_whole_net_multidriver`, E3001) counts
//! each driver as the set of storage bits its store WRITES, `sim_ir::const_chunk_written_bits` (both stores are
//! pinned against it in `sim-engine`, from SV source). An indexed part-select whose base is the select's TOP storage bit (`-:` on a
//! descending net, `+:` on an ascending one) writes `[off-w+1, off+1)`; the check took `[off, off+w)` for
//! every kind, an interval shifted up by `w-1`, so a real overlap through such a select was missed (the two
//! drivers ran unresolved) and a disjoint pair was refused. The interval is also clipped to the net's bits
//! (IEEE 1800-2017 §11.5.1: only in-range bits are written), so selects that meet only outside the net run,
//! and a constant offset is read as the store evaluates it (one self-determined expression: its widest
//! operand's width, signed only if every operand is, IEEE 1800-2017 §11.8; a string literal is the unsigned
//! integer it is, §5.9), with no arithmetic of the check's own. Measured on the review
//! cells of rounds 3 to 5: a real overlap at an in-range bit is E3001, like its spelled twin where the check
//! reads both (g03: `y[P +: 2]` is refused while `y[P:P+1]` on an ascending net is skipped) — row 60's policy:
//! a partial overlap is refused, not resolved; a disjoint pair, or one meeting only out of range, runs with the
//! oracles' value or, where they refuse, crash or truncate the index, the §11.5.1 value. Not covered: an offset
//! the check does not read is still skipped as dynamic (queued, Q-MDCONSTOFF): a multiply, a unary minus, a
//! concatenation, a conditional, a function call or cast, a `$clog2` whose argument is not one non-negative
//! numeric constant within 64 bits (a string, a negative signed value, an expression), a narrow signed sum on a
//! `[N:0]` net, and, on a net whose low bound is not 0 or that ascends, every offset that does not reach the
//! check as one constant of at most 64 bits (a literal, a parameter or a sum of sized literals the front end
//! folds is read there; a string literal, a `$clog2`, a literal past 64 bits or another sum is not).
//!
//! Oracles: Icarus Verilog 13.0 `-g2012` (iverilog), sv2v 0.0.13 -> iverilog (sv2v), Verilator 5.052
//! `--binary --timing` (verilator, 2-state: z reads 0). PRE = 73d22f5e. Every test runs on native, vm and
//! interp.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// `src` on one backend: (exit code, stdout, stderr).
fn run_on(src: &str, backend: &str) -> (Option<i32>, String, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_mdis_{backend}_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["--backend", backend, "t.sv"])
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// On every backend: a clean exit, and the stdout lines (no `simulation ended` line) are exactly `want`.
fn pin(src: &str, want: &[&str]) {
    for be in ["native", "vm", "interp"] {
        let (code, stdout, stderr) = run_on(src, be);
        assert_eq!(code, Some(0), "backend {be}: exit\n{stdout}{stderr}");
        let got: Vec<&str> = stdout
            .lines()
            .filter(|l| !l.starts_with("simulation ended"))
            .collect();
        assert_eq!(got, want, "backend {be}");
    }
}

/// On every backend: a clean exit, and the stdout lines starting with one of `prefixes` are exactly `want`.
fn pin_lines(src: &str, prefixes: &[&str], want: &[&str]) {
    for be in ["native", "vm", "interp"] {
        let (code, stdout, stderr) = run_on(src, be);
        assert_eq!(code, Some(0), "backend {be}: exit\n{stdout}{stderr}");
        let got: Vec<&str> = stdout
            .lines()
            .filter(|l| prefixes.iter().any(|p| l.starts_with(p)))
            .collect();
        assert_eq!(got, want, "backend {be}");
    }
}

/// On every backend: exit 1 with `E3001` (multiple overlapping continuous assignments), nothing simulated.
fn e3001(src: &str) {
    for be in ["native", "vm", "interp"] {
        let (code, stdout, stderr) = run_on(src, be);
        let all = format!("{stdout}{stderr}");
        assert_eq!(code, Some(1), "backend {be}: exit\n{all}");
        assert!(
            all.contains("[VITA-E3001]"),
            "backend {be}: no E3001\n{all}"
        );
        assert!(!stdout.contains("A end"), "backend {be}: simulated\n{all}");
    }
}

/// A real overlap through `-:` on a descending net: `y[3 -: 2]` = bits 3..2 and `y[2]` share bit 2.
/// Spelled twin `y[3:2]` = E3001 on PRE. iverilog = sv2v `S t=1 y=zz1xzz` | `S t=2 y=zz11zz`; verilator
/// `001000` | `001100`. PRE: `S t=1 y=zz11zz` | `S t=2 y=zz11zz` (the overlap missed: each driver wrote its
/// own interval, the shared bit unresolved). Lens-diff cell E05. Kills MD1.
#[test]
fn e05_minus_idx_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = 2'b10; reg b = 1'b1; wire [5:0] y;
  assign y[3 -: 2] = a;
  assign y[2] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 2'b11; #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
    );
}

/// A real overlap through `+:` on an ascending net: `y[2 +: 2]` on `[0:5]` = y[2], y[3] (storage bits 3..2),
/// shared with `y[3]`. iverilog = sv2v `S t=1 y=zz1xzz`; verilator `001100`. PRE `S t=1 y=zz11zz` (missed).
/// Lens-diff cell G01. Kills MD1.
#[test]
fn g01_asc_plus_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = 2'b10; reg b = 1'b1;
  wire [0:5] y;
  assign y[2 +: 2] = a;
  assign y[3] = b;
  initial begin #1 $display("S t=1 y=%b", y); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// A real overlap through `-:` on a descending net with a nonzero LSB: `y[5 -: 2]` on `[7:2]` and `y[4]`.
/// iverilog = sv2v `S t=1 y=zz1xzz`; verilator `001100`. PRE `S t=1 y=zz11zz` (missed). Lens-diff cell G06.
/// Kills MD1.
#[test]
fn g06_nz_desc_minus_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = 2'b10; reg b = 1'b1;
  wire [7:2] y;
  assign y[5 -: 2] = a;
  assign y[4] = b;
  initial begin #1 $display("S t=1 y=%b", y); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// Control (PRE right, must not move): a real overlap through `-:` on an ascending net (`y[3 -: 2]` on
/// `[0:5]` = y[2], y[3], a select whose base is the LOW storage bit) with `y[2]`: E3001 on PRE too.
/// iverilog = sv2v `S t=1 y=zz10zz`; verilator `001000`. Lens-diff cell G03. Kills MD2.
#[test]
fn g03_asc_minus_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = 2'b10; reg b = 1'b1;
  wire [0:5] y;
  assign y[3 -: 2] = a;
  assign y[2] = b;
  initial begin #1 $display("S t=1 y=%b", y); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// Control (PRE right, must not move): `-:` on an ascending net with a disjoint `y[4]` runs. iverilog = sv2v
/// = the pin; verilator `001010`. PRE = the pin. Lens-diff cell G04. Kills MD2.
#[test]
fn g04_asc_minus_disjoint_runs() {
    pin(
        r#"module top;
  reg [1:0] a = 2'b10; reg b = 1'b1;
  wire [0:5] y;
  assign y[3 -: 2] = a;
  assign y[4] = b;
  initial begin #1 $display("S t=1 y=%b", y); $display("A end"); $finish; end
endmodule
"#,
        &["S t=1 y=zz101z", "A end"],
    );
}

/// A disjoint pair through `-:` on a descending net: `y[1 -: 2]` = bits 1..0 beside `y[5:2]`. iverilog =
/// sv2v = verilator = the pin. PRE: E3001 (the interval taken as [1, 3), a false overlap). Lens-diff cell
/// E09. Kills MD1.
#[test]
fn e09_minus_idx_disjoint_runs() {
    pin(
        r#"module top;
  reg [1:0] a = 2'b01; reg [3:0] b = 4'b1100; wire [5:0] y;
  assign y[1 -: 2] = a;
  assign y[5:2] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y=110001", "A end"],
    );
}

/// A disjoint pair through `+:` on an ascending net: `y[2 +: 2]` on `[0:5]` beside `y[0:1]`. iverilog = sv2v
/// = the pin; verilator `011000` (2-state). PRE: E3001 (false overlap). D7 cell G2_G01asc_disjoint_false_e3001.
/// Kills MD1.
#[test]
fn g01_asc_plus_disjoint_runs() {
    pin(
        r#"module top;
  reg [1:0] a = 2'b10; wire [0:5] y;
  assign y[2 +: 2] = a;
  assign y[0:1] = 2'b01;
  initial begin #1 $display("S t=1 y=%b", y); $display("A end"); $finish; end
endmodule
"#,
        &["S t=1 y=0110zz", "A end"],
    );
}

/// A disjoint pair through `-:` on `[7:2]`: `y[5 -: 2]` beside `y[7:6]`. iverilog = sv2v = the pin;
/// verilator `011000` (2-state). PRE: E3001 (false overlap). D7 cell G2_G06nz_disjoint_false_e3001. Kills MD1.
#[test]
fn g06_nz_desc_minus_disjoint_runs() {
    pin(
        r#"module top;
  reg [1:0] a = 2'b10; wire [7:2] y;
  assign y[5 -: 2] = a;
  assign y[7:6] = 2'b01;
  initial begin #1 $display("S t=1 y=%b", y); $display("A end"); $finish; end
endmodule
"#,
        &["S t=1 y=0110zz", "A end"],
    );
}

const D09: &str = r#"module top;
  reg [7:0] a = 8'b10_0110_01; wire [5:0] y; wire [3:0] z;
  assign #5 {y[4 +: 2], z, y[1 -: 2]} = a;
  assign y[3:2] = 2'b11;
  always @(y) $display("Y t=%0t y=%b", $time, y);
  always @(z) $display("Z t=%0t z=%b", $time, z);
  initial begin
    #6 force y = 6'b000000; #1 release y; $display("S t=7imm y=%b z=%b", y, z);
    #1 $display("S t=8 y=%b z=%b", y, z);
    $display("A end"); $finish;
  end
endmodule
"#;

/// Indexed part-select chunks of a delayed concatenation, disjoint from `y[3:2]`: no E3001 (PRE: E3001, a
/// false overlap), and the lines equal to iverilog are pinned: `Y t=7 y=101101` | `S t=8 y=101101 z=0110`.
/// Its whole output is byte-identical to its spelled twin `{y[5:4], z, y[1:0]}`
/// (`d09_spelled_twin_lines`) on every backend. iverilog = sv2v: `Y t=0 y=xx11xx` | `Z t=5 z=0110` |
/// `Y t=5 y=101101` | `Y t=6 y=000000` | `S t=7imm y=101101 z=0110` | `Y t=7 y=101101` |
/// `S t=8 y=101101 z=0110`; verilator the same after its 2-state t=0 lines (`Z t=0 z=0000` | `Y t=0 y=001100`).
/// vita: `Y t=0 y=zz11zz` | `Y t=5 y=101101` | `Z t=5 z=0110` | `Y t=6 y=000000` | `S t=7imm y=100001 z=0110` |
/// `Y t=7 y=101101` | `S t=8 y=101101 z=0110`. Never pinned, three classes: t=0 `zz11zz` vs `xx11xx`, the owed
/// x of a delayed concatenation co-member before its first landing (Q-DCAPARTX / N-R01); `S t=7imm` `100001`
/// vs `101101`, the undelayed driver's bits `y[3:2]` read forced right after the release (Q-RELIMM); the
/// order of `Y t=5` / `Z t=5`, two processes woken in one time step, an order IEEE 1800-2017 §4.7 leaves
/// unspecified (not a defect). Lens-diff cell D09. Kills MD1 MD2.
#[test]
fn d09_indexed_partsel_chunks_lines() {
    pin_lines(
        D09,
        &["Y t=7 ", "S t=8 "],
        &["Y t=7 y=101101", "S t=8 y=101101 z=0110"],
    );
}

/// D09's spelled twin `{y[5:4], z, y[1:0]}` (the D7 control): the same pinned lines and the same three
/// never-pinned classes. iverilog = D09's iverilog text. PRE: `S t=7imm y=000000 z=0110` | `Y t=7 y=001100` |
/// `S t=8 y=001100 z=0110` (the release restored no delayed driver: Q-RELDELAY's own defect). Cell d09sp.
#[test]
fn d09_spelled_twin_lines() {
    pin_lines(
        &D09.replace("{y[4 +: 2], z, y[1 -: 2]}", "{y[5:4], z, y[1:0]}"),
        &["Y t=7 ", "S t=8 "],
        &["Y t=7 y=101101", "S t=8 y=101101 z=0110"],
    );
}

/// A real overlap through `-:` (`y[3 -: 2]`, delayed) with `y[2]`, forced to the value the oracles resolve
/// and released. iverilog = sv2v: `Y t=3 y=1x00` | `S t=4 y=1x00` | `S t=6imm y=1x00` | `S t=7 y=1x00` |
/// `S t=10 y=1x00`; verilator `1000` throughout. PRE: `Y t=0 y=z100` | `Y t=3 y=1000` | `S t=4 y=1000` |
/// `Y t=5 y=1x00` | `S t=6imm y=1x00` | `Y t=6 y=1100` | ... (the overlap missed; `S t=6imm` equal to
/// iverilog only because PRE's release did not restore the delayed driver, so the forced `1x00` stayed until
/// the settle). Q-RELDELAY's restore writes the delayed driver's own image: `S t=6imm y=1000`. Right on PRE
/// only through PRE's non-restore; now E3001, like its spelled twin `y[3:2]`. Lens-diff cell F01. Kills MD1.
#[test]
fn f01_minus_idx_overlap_force_eq_resolved_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = 2'b10; reg b = 1'b1; reg [1:0] c = 2'b00; wire [3:0] y;
  assign #3 y[3 -: 2] = a;
  assign y[2] = b;
  assign y[1:0] = c;
  always @(y) $display("Y t=%0t y=%b", $time, y);
  initial begin
    #4 $display("S t=4 y=%b", y);
    #1 force y = 4'b1x00; #1 release y; $display("S t=6imm y=%b", y);
    #1 $display("S t=7 y=%b", y); #3 $display("S t=10 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
    );
}

/// A real overlap through `-:` (`y[3 -: 2]` = `1z`, delayed) with `y[2]` = 0, forced to `1000` (= the
/// resolution) and released: no event on `y` after the release. iverilog = sv2v = verilator:
/// `C n=0 l2=0 l3=0 np2=0 nn2=0 np3=0 nn3=0 first=-1`; PRE the same (its release restored nothing). Q-RELDELAY's
/// restore writes the delayed driver's own image over the missed overlap: `C n=1 ... first=6`. Right on PRE
/// only through PRE's non-restore; now E3001, like its spelled twin `y[3:2]` (D7 Q3v2c, E3001 on PRE). D7 cell
/// Q3v2_a1z_b0_f1000. Kills MD1.
#[test]
fn q3v2_a1z_b0_f1000_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = 2'b1z; reg b = 1'b0;
  reg [1:0] c = 2'b00; wire [3:0] y; wire y2, y3; reg en = 0;
  integer n = 0, l2 = 0, l3 = 0, np2 = 0, nn2 = 0, np3 = 0, nn3 = 0, first = -1;
  assign #3 y[3 -: 2] = a;
  assign y[2] = b;
  assign y[1:0] = c;
  assign y2 = y[2];
  assign y3 = y[3];
  always @(y) if (en) begin n = n + 1; if (first < 0) first = $time; end
  always @(y[2]) if (en) l2 = l2 + 1;
  always @(y[3]) if (en) l3 = l3 + 1;
  always @(posedge y2) if (en) np2 = np2 + 1;
  always @(negedge y2) if (en) nn2 = nn2 + 1;
  always @(posedge y3) if (en) np3 = np3 + 1;
  always @(negedge y3) if (en) nn3 = nn3 + 1;
  initial begin
    #5
    force y = 4'b1000;
    #1 en = 1; release y;
    #4 $display("C n=%0d l2=%0d l3=%0d np2=%0d nn2=%0d np3=%0d nn3=%0d first=%0d", n, l2, l3, np2, nn2, np3, nn3, first);
    $display("A end"); $finish;
  end
endmodule
"#,
    );
}

/// A hierarchical `-:` lvalue (`u.y[3 -: 2]`, a deferred hierarchical select) overlapping the child's own
/// `y[2]`: the same check, the same interval. iverilog `S t=4 y=1xzz`. PRE: `S t=4 y=10zz` (missed). The
/// spelled twin `u.y[3:2]` is E3001 on PRE. Probe b3/ovl. Kills MD1.
#[test]
fn hier_minus_idx_overlap_is_e3001() {
    e3001(
        r#"module child;
  wire [3:0] y;
  assign y[2] = 1'b1;
endmodule
module top;
  reg [1:0] a = 2'b10;
  child u();
  assign #3 u.y[3 -: 2] = a;
  initial begin #4 $display("S t=4 y=%b", u.y); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// A hierarchical `-:` lvalue disjoint from the child's `y[2]` runs. iverilog = the pin. PRE: E3001 (false
/// overlap). Probe b3/dis. Kills MD1.
#[test]
fn hier_minus_idx_disjoint_runs() {
    pin(
        r#"module child;
  wire [3:0] y;
  assign y[2] = 1'b1;
endmodule
module top;
  reg [1:0] a = 2'b10;
  child u();
  assign #3 u.y[1 -: 2] = a;
  initial begin #4 $display("S t=4 y=%b", u.y); $display("A end"); $finish; end
endmodule
"#,
        &["S t=4 y=z110", "A end"],
    );
}

/// Two selects wholly ABOVE the MSB of `wire [3:0] y` (`y[5 -: 2]` = bits 5..4, `y[4 +: 1]` = bit 4): neither
/// writes a bit (IEEE 1800-2017 §11.5.1: only in-range bits are written), so they overlap nowhere and the net
/// stays z. iverilog = sv2v `S t=1 y=zzzz` (verilator `0010`, 2-state, writes a truncated select). PRE `zzzz`.
/// Round-3 lens-sound S1 cell K32 (round 3 refused it: the interval was not clipped). Kills MC1.
#[test]
fn k32_both_above_msb_runs() {
    pin(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0; wire [3:0] y;
  assign y[5 -: 2] = a;
  assign y[4 +: 1] = b;
  initial begin
    #1 $display("S t=%0t y=%b", $time, y);
    #1 $finish;
  end
endmodule
"#,
        &["S t=1 y=zzzz"],
    );
}

/// K32's spelled twin `y[5:4]` + `y[4]`: nothing in range, nothing written. iverilog and sv2v refuse
/// (`error: Index y[4] is out of range.`); IEEE §11.5.1 by hand: `zzzz`. PRE: E3001. Lens-sound cell K32t.
/// Kills MC1.
#[test]
fn k32t_spelled_both_above_msb_runs() {
    pin(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0; wire [3:0] y;
  assign y[5:4] = a;
  assign y[4] = b;
  initial begin
    #1 $display("S t=%0t y=%b", $time, y);
    #1 $finish;
  end
endmodule
"#,
        &["S t=1 y=zzzz"],
    );
}

/// `y[5:4]` + `y[4 +: 1]` on `wire [3:0] y`: iverilog (`warning: Part select y[5:4] is out of range.`) = sv2v
/// `S t=1 y=zzzz`. PRE: E3001. Lens-sound cell K32t2. Kills MC1.
#[test]
fn k32t2_spelled_vs_up_above_msb_runs() {
    pin(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0; wire [3:0] y;
  assign y[5:4] = a;
  assign y[4 +: 1] = b;
  initial begin
    #1 $display("S t=%0t y=%b", $time, y);
    #1 $finish;
  end
endmodule
"#,
        &["S t=1 y=zzzz"],
    );
}

/// `y[4 +: 2]` + `y[4 +: 1]` on `wire [3:0] y`: iverilog = sv2v `S t=1 y=zzzz`. PRE: E3001. Lens-sound cell
/// K32u. Kills MC1.
#[test]
fn k32u_up_up_both_above_msb_runs() {
    pin(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0; wire [3:0] y;
  assign y[4 +: 2] = a;
  assign y[4 +: 1] = b;
  initial begin
    #1 $display("S t=%0t y=%b", $time, y);
    #1 $finish;
  end
endmodule
"#,
        &["S t=1 y=zzzz"],
    );
}

/// Two selects wholly BELOW bit 0 through a signed parameter (`N = -1`: `y[N -: 2]` = bits -1..-2,
/// `y[N-1 +: 1]` = bit -2): nothing written, no overlap. iverilog = sv2v `S t=1 y=zzzz`. PRE `zzzz`. Round-3
/// lens-sound S1 cell K43 (round 3 refused it).
#[test]
fn k43_negparam_both_below_lsb_runs() {
    pin(
        r#"module top;
  localparam integer N = -1; reg [1:0] a = 2'b11; reg b = 1'b0; wire [3:0] y;
  assign y[N -: 2] = a;
  assign y[N-1 +: 1] = b;
  initial begin
    #1 $display("S t=%0t y=%b", $time, y);
    #1 $finish;
  end
endmodule
"#,
        &["S t=1 y=zzzz"],
    );
}

/// K43's spelled twin `y[N:N-1]` + `y[N-1]`: iverilog and sv2v refuse (`sorry: part selects straddling the
/// start of signal (y[(N)-('sd1)]) are not currently supported.`); IEEE §11.5.1 by hand: `zzzz`. PRE: E3001.
/// Lens-sound cell K43t.
#[test]
fn k43t_negparam_spelled_runs() {
    pin(
        r#"module top;
  localparam integer N = -1; reg [1:0] a = 2'b11; reg b = 1'b0; wire [3:0] y;
  assign y[N:N-1] = a;
  assign y[N-1] = b;
  initial begin
    #1 $display("S t=%0t y=%b", $time, y);
    #1 $finish;
  end
endmodule
"#,
        &["S t=1 y=zzzz"],
    );
}

/// `y[5 -: 3]` (bits 5..3: only y[3] is in range, = a[0]) + `y[4 +: 1]` (nothing in range): they meet only at
/// bit 4, outside the net. IEEE §11.5.1 by hand `S t=1 y=1zzz`; verilator `1000` (2-state); iverilog and sv2v
/// crash (`Assertion failed: (adr+that.size_ <= size_), function set_vec, file vvp_net.cc, line 1212.`). PRE
/// `1zzz`. Round-3 lens-sound S1 cell K38 (round 3 refused it). Kills MC1.
#[test]
fn k38_partial_above_vs_up_out_runs() {
    pin(
        r#"module top;
  reg [2:0] a = 3'b101; reg b = 1'b0; wire [3:0] y;
  assign y[5 -: 3] = a;
  assign y[4 +: 1] = b;
  initial begin
    #1 $display("S t=%0t y=%b", $time, y);
    #1 $finish;
  end
endmodule
"#,
        &["S t=1 y=1zzz"],
    );
}

/// K38's spelled twin `y[5:3]` + `y[4 +: 1]`: by hand `S t=1 y=1zzz`; verilator `1000`; iverilog and sv2v crash
/// (the same `vvp_net.cc` assertion). PRE: E3001. Lens-sound cell K38t. Kills MC1.
#[test]
fn k38t_spelled_partial_vs_up_out_runs() {
    pin(
        r#"module top;
  reg [2:0] a = 3'b101; reg b = 1'b0; wire [3:0] y;
  assign y[5:3] = a;
  assign y[4 +: 1] = b;
  initial begin
    #1 $display("S t=%0t y=%b", $time, y);
    #1 $finish;
  end
endmodule
"#,
        &["S t=1 y=1zzz"],
    );
}

/// A 65-bit constant offset `65'h1_0000_0000_0000_0003` is read whole: 2^64 + 3 is out of range, `y[.. -: 2]`
/// writes nothing, `y[2]` = b. IEEE §11.5.1 by hand `S t=1 y=z0zz` (PRE `z0zz`: the store drops the write);
/// iverilog = sv2v `S t=1 y=1xzz` after `warning: verinum::as_long() truncated 65 bits to 63, returns 3`
/// (the oracle truncates the offset); verilator refuses (`Value too wide`). Round-3 lens-sound S1 cell K36
/// (round 3 read the low word, 3, and refused it). Kills MC2.
#[test]
fn k36_wide65_offset_out_of_range_runs() {
    pin(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0; wire [3:0] y;
  assign y[65'h1_0000_0000_0000_0003 -: 2] = a;
  assign y[2] = b;
  initial begin
    #1 $display("S t=%0t y=%b", $time, y);
    #1 $finish;
  end
endmodule
"#,
        &["S t=1 y=z0zz"],
    );
}

/// K36 through `+:` at 2^64 + 2 (round-3 lens-sound cell K36u, verbatim): out of range, `y[.. +: 2]` writes
/// nothing; by hand `S t=1 y=z0zz`. iverilog = sv2v `S t=1 y=1xzz` (the offset truncated to 2, the same
/// warning). PRE = round 2 = round 3: E3001 (the low word, 2, overlaps `y[2]`).
#[test]
fn k36u_wide65_offset_up_runs() {
    pin(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0; wire [3:0] y;
  assign y[65'h1_0000_0000_0000_0002 +: 2] = a;
  assign y[2] = b;
  initial begin
    #1 $display("S t=%0t y=%b", $time, y);
    #1 $finish;
  end
endmodule
"#,
        &["S t=1 y=z0zz"],
    );
}

/// `y[7 -: 3]` on `wire [5:0] y` writes only y[5]; `y[6]` writes nothing: they meet only at the nonexistent
/// bit 6. IEEE §11.5.1 by hand `S t=1 y=1zzzzz` | `S t=2 y=0zzzzz` (PRE the same); verilator `100000` |
/// `000000`; iverilog and sv2v refuse (`error: Index y[6] is out of range.`). Round-3 lens-diff F3 cell w03
/// (round 3 refused it). Kills MC1.
#[test]
fn w03_partial_above_overlap_out_of_range_only_runs() {
    pin(
        r#"module top;
  reg [2:0] a = {3{1'b1}}; reg b = 1'b0;
  wire [5:0] y;
  assign y[7 -: 3] = a;
  assign y[6] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 0; b = 1'b1; #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y=1zzzzz", "S t=2 y=0zzzzz", "A end"],
    );
}

/// A `-:` reaching BELOW bit 0 (`y[1 -: 3]` = bits 1..-1, in range y[1:0]) overlapping `y[0]`: E3001.
/// iverilog and sv2v refuse (`sorry: part selects straddling the start of signal (y['sd1-:'sd3]) are not
/// currently supported.`); verilator drops the whole write; IEEE §11.5.1 by hand y[0] = 1 vs 0 -> x, a real
/// overlap. PRE `S t=1 y=zz10` (missed). Round-3 lens-sound cell K01 (S5 gap; an `A end` line added). Kills MLD1.
#[test]
fn k01_minus_idx_below_bit0_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [2:0] a = 3'b111; reg b = 1'b0; wire [3:0] y;
  assign y[1 -: 3] = a;
  assign y[0] = b;
  initial begin
    #1 $display("S t=%0t y=%b", $time, y);
    $display("A end"); #1 $finish;
  end
endmodule
"#,
    );
}

/// K01 through an output port connection (`.o(y[1 -: 3])`) beside `assign y[0] = b;`: E3001. iverilog and
/// sv2v refuse (straddling the start of signal); by hand y[0] = 1 vs 0 -> x. PRE `S t=1 y=zz10` (missed).
/// Round-3 lens-sound cell K30 (S5 gap; an `A end` line added). Kills MLD1.
#[test]
fn k30_port_below_bit0_overlap_is_e3001() {
    e3001(
        r#"module c3(output [2:0] o);
  assign o = 3'b111;
endmodule
module top;
  reg b = 1'b0; wire [3:0] y;
  c3 u(.o(y[1 -: 3]));
  assign y[0] = b;
  initial begin
    #1 $display("S t=%0t y=%b", $time, y);
    $display("A end"); #1 $finish;
  end
endmodule
"#,
    );
}

/// An overlap only at the `-:` BASE bit (`y[3 -: 2]` = bits 3..2, `y[3]`): E3001, like its spelled twin
/// `y[3:2]` (E3001 on PRE). iverilog = sv2v `S t=1 y=zzx0zz` | `S t=2 y=zzx1zz`; verilator `000000` |
/// `001100`. PRE: E3001 too. Probe basebit_dn (S5 gap). Kills MLD2.
#[test]
fn minus_idx_overlap_at_base_bit_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = 2'b10; reg b = 1'b0; wire [5:0] y;
  assign y[3 -: 2] = a;
  assign y[3] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 2'b01; b = 1'b1; #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
    );
}

/// `y[2 +: 3]` on `wire [10:3] y` normalizes to the signed offset -1 (bits 2..4, in range y[4:3]) and overlaps
/// `y[3]`: E3001, like its spelled twin `y[4:2]`. iverilog and sv2v refuse (`sorry: part selects straddling the
/// start of signal (y['sd2+:'sd3]) are not currently supported.`); verilator drops the write; by hand y[3] =
/// 1 vs 0 -> x. PRE = round 3 `S t=1 y=zzzzzz10` (missed: the negative offset was read as 4294967295). Lens-diff
/// cell n25. Kills MC2.
#[test]
fn n25_plus_idx_below_lsb_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [2:0] d0 = 3'b111;
  reg [0:0] d1 = 1'b0;
  wire [10:3] y;
  assign y[2 +: 3] = d0;
  assign y[3] = d1;
  initial begin
    #1 $display("S t=1 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
    );
}

/// A constant offset `3'd1 - 3'd2` is 7 at its own 3-bit width (the store wraps it there), out of range on
/// `wire [3:0] y`: `y[.. +: 2]` writes nothing, no overlap with `y[0]`. iverilog = sv2v `S t=1 y=zzz0`
/// (verilator `1000`). PRE `zzz0`. Probe subw_neg.
#[test]
fn offset_wraps_at_its_own_width_runs() {
    pin(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0; wire [3:0] y;
  assign y[3'd1 - 3'd2 +: 2] = a;
  assign y[0] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y=zzz0", "A end"],
    );
}

// ── round 5: the offset is the store's own region fold (S1-r4 / F2, F1, string literals, `$clog2`) ──────────
// r4 read each `+` / `-` operand at its own width and sign (S1-r4 / F2) and a constant past 64 bits by its low
// word (F1); the store evaluates the whole offset as ONE self-determined expression (IEEE 1800-2017 §11.8.1-2: the
// widest operand's width, signed only if every operand is, an operand extended by the expression's sign, wrapping
// there) and a string literal as the unsigned integer it is (§5.9). The check now asks that fold
// (`sim_ir::const_chunk_written_bits`). `R4` below is round 4's B.

/// `(4'd15 + 4'd1) + 8'd0` is 16 in its one 8-bit region (r4 wrapped the inner sum at 4 bits: 0), so `y[.. -: 1]`
/// shares bit 16 with `y[16]`. PRE E3001; R4 `S t=1 y16=0 y0=z | A end`; iverilog = sv2v `S t=1 y16=x y0=z | A
/// end`. Cell ls4_out1_a01_nest_ovl16.
#[test]
fn a01_nested_sum_one_region_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0;
  wire [31:0] y;
  assign y[(4'd15 + 4'd1) + 8'd0 -: 1] = a[0];
  assign y[16] = b;
  initial begin
    #1 $display("S t=1 y16=%b y0=%b", y[16], y[0]);
    $display("A end"); $finish;
  end
endmodule
"#,
    );
}

/// The same select (bit 16) beside `y[0]`: disjoint (r4 saw bit 0). PRE = iverilog = sv2v `S t=1 y16=1 y0=0 | A
/// end`; R4 E3001. Cell ls4_out1_a02_nest_false0.
#[test]
fn a02_nested_sum_one_region_disjoint_runs() {
    pin(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0;
  wire [31:0] y;
  assign y[(4'd15 + 4'd1) + 8'd0 -: 1] = a[0];
  assign y[0] = b;
  initial begin
    #1 $display("S t=1 y16=%b y0=%b", y[16], y[0]);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y16=1 y0=0", "A end"],
    );
}

/// `4'sb1111 + 8'd0` is unsigned (one operand is), so `4'sb1111` is zero-extended: 15, the bit `y[15]` drives (r4
/// read -1). PRE E3001; R4 `S t=1 y=0zzzzzzzzzzzzzzz | A end`; iverilog = sv2v `S t=1 y=xzzzzzzzzzzzzzzz | A
/// end`. Cell ls4_out1_a03_mixsign_ovl15.
#[test]
fn a03_signed_leaf_in_unsigned_region_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0;
  wire [15:0] y;
  assign y[4'sb1111 + 8'd0 -: 1] = a[0];
  assign y[15] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
    );
}

/// `8'd20 - 4'sb1111` is 20 - 15 = 5 in an unsigned 8-bit region, shared with `y[5]` (r4 read 20 - (-1) = 21).
/// PRE E3001; R4 `S t=1 y5=0 y21=z | A end`; iverilog = sv2v `S t=1 y5=x y21=z | A end`. Cell
/// ls4_out1_a05_mixsub_ovl5.
#[test]
fn a05_unsigned_minus_signed_leaf_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0;
  wire [31:0] y;
  assign y[8'd20 - 4'sb1111 -: 1] = a[0];
  assign y[5] = b;
  initial begin
    #1 $display("S t=1 y5=%b y21=%b", y[5], y[21]);
    $display("A end"); $finish;
  end
endmodule
"#,
    );
}

/// The same select (bit 5) beside `y[21]`: disjoint. PRE = iverilog = sv2v `S t=1 y5=1 y21=0 | A end`; R4 E3001.
/// Cell ls4_out1_a05b_mixsub_false21.
#[test]
fn a05b_unsigned_minus_signed_leaf_disjoint_runs() {
    pin(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0;
  wire [31:0] y;
  assign y[8'd20 - 4'sb1111 -: 1] = a[0];
  assign y[21] = b;
  initial begin
    #1 $display("S t=1 y5=%b y21=%b", y[5], y[21]);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y5=1 y21=0", "A end"],
    );
}

/// `(32'hFFFF_FFFF + 32'd1) + 40'd0` is 2^32 in its 40-bit region (r4 wrapped the inner sum at 32 bits: 0): out
/// of `wire [3:0] y`, so beside `y[0]` it drives nothing. PRE = iverilog = sv2v `S t=1 y=zzz0 | A end`; R4 E3001.
/// Cell ls4_out1_a12_ovf32_false0.
#[test]
fn a12_sum_wraps_at_the_region_not_the_operand_runs() {
    pin(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0;
  wire [3:0] y;
  assign y[(32'hFFFF_FFFF + 32'd1) + 40'd0 +: 2] = a;
  assign y[0] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y=zzz0", "A end"],
    );
}

/// `4'sd7 + 4'sd1 + 0` is a signed 32-bit region (the unsized `0` is an integer): 8, so `y[.. +: 2]` (bits 9..8)
/// shares bit 8 with `y[8]` (r4 wrapped `4'sd7 + 4'sd1` at 4 bits: -8). PRE E3001; R4 `S t=1 y9=1 y8=0 y7=z | A
/// end`; iverilog = sv2v `S t=1 y9=1 y8=x y7=z | A end`. Cell ls4_out5_e07_s4mixed.
#[test]
fn e07_signed_sum_with_unsized_operand_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a; reg b;
  wire [15:0] y;
  assign y[4'sd7 + 4'sd1 + 0 +: 2] = a;
  assign y[8] = b;
  initial begin
    a = 2'b11; b = 1'b0;
    #1 $display("S t=1 y9=%b y8=%b y7=%b", y[9], y[8], y[7]);
    $display("A end"); $finish;
  end
endmodule
"#,
    );
}

/// a01 through a hierarchical lvalue `u.y[..]`. PRE E3001; R4 `S t=1 y16=0 y0=z | A end`; iverilog = sv2v `S t=1
/// y16=x y0=z | A end`. Cell ls4_out2_b02_hier_nest_ovl16.
#[test]
fn b02_hierarchical_nested_sum_overlap_is_e3001() {
    e3001(
        r#"module child; wire [31:0] y; endmodule
module top;
  reg [1:0] a = 2'b11; reg b = 1'b0;
  child u();
  assign u.y[(4'd15 + 4'd1) + 8'd0 -: 1] = a[0];
  assign u.y[16] = b;
  initial begin
    #1 $display("S t=1 y16=%b y0=%b", u.y[16], u.y[0]);
    $display("A end"); $finish;
  end
endmodule
"#,
    );
}

/// a01 as an output-port actual `.o(y[..])`. PRE E3001; R4 `S t=1 y16=1 y0=z | A end`; iverilog = sv2v `S t=1
/// y16=x y0=z | A end`. Cell ls4_out2_b05_port_nest_ovl16.
#[test]
fn b05_port_actual_nested_sum_overlap_is_e3001() {
    e3001(
        r#"module child(output wire o); assign o = 1'b1; endmodule
module top;
  reg [1:0] a = 2'b11; reg b = 1'b0;
  wire [31:0] y;
  child c(.o(y[(4'd15 + 4'd1) + 8'd0 -: 1]));
  assign y[16] = b;
  initial begin
    #1 $display("S t=1 y16=%b y0=%b", y[16], y[0]);
    $display("A end"); $finish;
  end
endmodule
"#,
    );
}

/// `4'sb1111 + 32'd0` is 15 (zero-extended into an unsigned 32-bit region), shared with `y[15]`. PRE E3001; R4 `S
/// t=1 y31=z y15=0 | A end`; iverilog = sv2v `S t=1 y31=z y15=x | A end`. Cell ls4_out2_b10_mix_add_32u.
#[test]
fn b10_signed_leaf_in_32bit_unsigned_region_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0;
  wire [31:0] y;
  assign y[4'sb1111 + 32'd0 -: 1] = a[0];
  assign y[15] = b;
  initial begin
    #1 $display("S t=1 y31=%b y15=%b", y[31], y[15]);
    $display("A end"); $finish;
  end
endmodule
"#,
    );
}

/// a01 as a bit-select `y[(4'd15 + 4'd1) + 8'd0]`. PRE E3001; R4 `S t=1 y16=0 y0=z | A end`; iverilog = sv2v `S
/// t=1 y16=x y0=z | A end`. Cell ls4_out2_b12_nest_bit.
#[test]
fn b12_nested_sum_bit_select_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0;
  wire [31:0] y;
  assign y[(4'd15 + 4'd1) + 8'd0] = a[0];
  assign y[16] = b;
  initial begin
    #1 $display("S t=1 y16=%b y0=%b", y[16], y[0]);
    $display("A end"); $finish;
  end
endmodule
"#,
    );
}

/// `2'sd3 + 3'd1` is unsigned 3-bit: 3 + 1 = 4, shared with `y[4]` (r4 read 2'sd3 as -1: 0). PRE E3001; R4 `S t=1
/// y=zzz0zzzz | S t=2 y=zzz1zzzz | A end`; iverilog = sv2v `S t=1 y=zzzxzzzz | S t=2 y=zzzxzzzz | A end`. Cell
/// ld4_outE_e21_mix_ovl.
#[test]
fn e21_mixed_sign_sum_overlap_is_e3001() {
    e3001(
        r#"module top;

  reg [0:0] a = 1'b1; reg [0:0] b = 1'b0;
  wire [7:0] y;
  assign y[2'sd3 + 3'd1 +: 1] = a;
  assign y[4] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 1'b0; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
    );
}

/// The same select (bit 4) beside `y[0]`: disjoint. PRE = iverilog = sv2v `S t=1 y=zzz1zzz0 | S t=2 y=zzz0zzz1 |
/// A end`; R4 E3001. Cell ld4_outE_e21_mix_vs0.
#[test]
fn e21_mixed_sign_sum_disjoint_runs() {
    pin(
        r#"module top;

  reg [0:0] a = 1'b1; reg [0:0] b = 1'b0;
  wire [7:0] y;
  assign y[2'sd3 + 3'd1 +: 1] = a;
  assign y[0] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 1'b0; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y=zzz1zzz0", "S t=2 y=zzz0zzz1", "A end"],
    );
}

/// `1'sb1 + 3'd5` is unsigned 3-bit: 1 + 5 = 6, shared with `y[6]` (r4 read 1'sb1 as -1: 4). PRE E3001; R4 `S t=1
/// y=z0zzzzzz | S t=2 y=z1zzzzzz | A end`; iverilog = sv2v `S t=1 y=zxzzzzzz | S t=2 y=zxzzzzzz | A end`. Cell
/// ld4_outH_v4_1sb_plus_u_ieee6.
#[test]
fn v4_one_bit_signed_plus_unsigned_overlap_is_e3001() {
    e3001(
        r#"module top;

  reg a = 1'b1; reg b = 1'b0;
  wire [7:0] y;
  assign y[1'sb1 + 3'd5 +: 1] = a;
  assign y[6] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 1'b0; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
    );
}

/// The same select (bit 6) beside `y[4]`: disjoint. PRE = iverilog = sv2v `S t=1 y=z1z0zzzz | S t=2 y=z0z1zzzz |
/// A end`; R4 E3001. Cell ld4_outH_v4_1sb_plus_u_chk4.
#[test]
fn v4_one_bit_signed_plus_unsigned_disjoint_runs() {
    pin(
        r#"module top;

  reg a = 1'b1; reg b = 1'b0;
  wire [7:0] y;
  assign y[1'sb1 + 3'd5 +: 1] = a;
  assign y[4] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 1'b0; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y=z1z0zzzz", "S t=2 y=z0z1zzzz", "A end"],
    );
}

/// `OFF + 8'd6` with `localparam signed [3:0] OFF = -4'sd2` is unsigned 8-bit: 14 + 6 = 20, out of `wire [7:0] y`
/// (r4: -2 + 6 = 4, the bit `y[4]` drives). PRE = iverilog = sv2v `S t=1 y=zzz0zzzz | S t=2 y=zzz1zzzz | A end`;
/// R4 E3001. Cell ld4_outH_v7_s4_plus_u8_chk4.
#[test]
fn v7_signed_param_plus_unsigned_out_of_range_runs() {
    pin(
        r#"module top;
  localparam signed [3:0] OFF = -4'sd2;
  reg a = 1'b1; reg b = 1'b0;
  wire [7:0] y;
  assign y[OFF + 8'd6 +: 1] = a;
  assign y[4] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 1'b0; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y=zzz0zzzz", "S t=2 y=zzz1zzzz", "A end"],
    );
}

/// `3'sd7 + 5'd4` is unsigned 5-bit: 7 + 4 = 11, out of range (r4: -1 + 4 = 3). PRE = iverilog = sv2v `S t=1
/// y=zzzz0zzz | S t=2 y=zzzz1zzz | A end`; R4 E3001. Cell ld4_outH_v12_s3_plus_u5_chk3.
#[test]
fn v12_signed_plus_unsigned_out_of_range_runs() {
    pin(
        r#"module top;

  reg a = 1'b1; reg b = 1'b0;
  wire [7:0] y;
  assign y[3'sd7 + 5'd4 +: 1] = a;
  assign y[3] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 1'b0; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y=zzzz0zzz", "S t=2 y=zzzz1zzz", "A end"],
    );
}

/// `y[3'd1 + 2'sd3]` is bit 4 (unsigned 3-bit 1 + 3), shared with `y[4]` (r4: 1 + (-1) = 0). PRE E3001; R4 `S t=1
/// y=zzz0zzzz | S t=2 y=zzz1zzzz | A end`; iverilog = sv2v `S t=1 y=zzzxzzzz | S t=2 y=zzzxzzzz | A end`. Cell
/// ld4_outI_i09_f2_bit_vs4.
#[test]
fn i09_mixed_sign_bit_select_overlap_is_e3001() {
    e3001(
        r#"module top;

  reg [0:0] a = 1'b1; reg [0:0] b = 1'b0;
  wire [7:0] y;
  assign y[3'd1 + 2'sd3] = a;
  assign y[4] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 1'b0; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
    );
}

/// The same bit-select beside `y[0]`: disjoint. PRE = iverilog = sv2v `S t=1 y=zzz1zzz0 | S t=2 y=zzz0zzz1 | A
/// end`; R4 E3001. Cell ld4_outI_i10_f2_bit_vs0.
#[test]
fn i10_mixed_sign_bit_select_disjoint_runs() {
    pin(
        r#"module top;

  reg [0:0] a = 1'b1; reg [0:0] b = 1'b0;
  wire [7:0] y;
  assign y[3'd1 + 2'sd3] = a;
  assign y[0] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 1'b0; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y=zzz1zzz0", "S t=2 y=zzz0zzz1", "A end"],
    );
}

/// i09 spelled `y[3'd1 + 2'sd3 : 3'd1 + 2'sd3]`. PRE E3001; R4 `S t=1 y=zzz0zzzz | S t=2 y=zzz1zzzz | A end`;
/// iverilog = sv2v `S t=1 y=zzzxzzzz | S t=2 y=zzzxzzzz | A end`. Cell ld4_outI_i11_f2_sp_vs4.
#[test]
fn i11_mixed_sign_part_select_overlap_is_e3001() {
    e3001(
        r#"module top;

  reg [0:0] a = 1'b1; reg [0:0] b = 1'b0;
  wire [7:0] y;
  assign y[3'd1 + 2'sd3 : 3'd1 + 2'sd3] = a;
  assign y[4] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 1'b0; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
    );
}

/// i10 spelled `[m:l]`. PRE = iverilog = sv2v `S t=1 y=zzz1zzz0 | S t=2 y=zzz0zzz1 | A end`; R4 E3001. Cell
/// ld4_outI_i12_f2_sp_vs0.
#[test]
fn i12_mixed_sign_part_select_disjoint_runs() {
    pin(
        r#"module top;

  reg [0:0] a = 1'b1; reg [0:0] b = 1'b0;
  wire [7:0] y;
  assign y[3'd1 + 2'sd3 : 3'd1 + 2'sd3] = a;
  assign y[0] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 1'b0; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y=zzz1zzz0", "S t=2 y=zzz0zzz1", "A end"],
    );
}

/// `65'sh1_0000_0000_0000_0002` is -(2^64 - 2) (a 65-bit signed constant), out of range: the select drives
/// nothing beside `y[1]` (r4 read the low word: 2, `-: 2` = bits 2..1). Same source: cells x9_t1,
/// e10_65sneg_vs_lo. PRE = iverilog = sv2v `S t=1 y=zz0z | S t=2 y=zz1z | A end`; R4 E3001. Cell ld4_t1.
#[test]
fn t1_wide_negative_offset_out_of_range_runs() {
    pin(
        r#"module top;

  reg [1:0] a = 2'b10; reg [0:0] b = 1'b0;
  wire [3:0] y;
  assign y[65'sh1_0000_0000_0000_0002 -: 2] = a;
  assign y[1] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 2'b01; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y=zz0z", "S t=2 y=zz1z", "A end"],
    );
}

/// The same constant as both `[m:l]` bounds (cell x9_t2): out of range, drives nothing. PRE refused it as
/// overlapping too (it read the low words, 2 and 1). PRE = R4 E3001; iverilog = sv2v `S t=1 y=zz0z | S t=2 y=zz1z
/// | A end`. Cell ld4_t2.
#[test]
fn t2_wide_negative_part_select_bounds_run() {
    pin(
        r#"module top;
  reg [1:0] a = 2'b10; reg [0:0] b = 1'b0;
  wire [3:0] y;
  assign y[65'sh1_0000_0000_0000_0002 : 65'sh1_0000_0000_0000_0001] = a;
  assign y[1] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 2'b01; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y=zz0z", "S t=2 y=zz1z", "A end"],
    );
}

/// An 80-bit signed localparam -(2^64 - 2) (cell x9_t3): out of range. PRE = iverilog = sv2v `V
/// p=-18446744073709551614 | S t=1 y=zz0z | S t=2 y=zz1z | A end`; R4 E3001. Cell ld4_t3.
#[test]
fn t3_wide_negative_localparam_offset_runs() {
    pin(
        r#"module top;
  localparam logic signed [79:0] P = 80'shFFFF_FFFF_0000_0000_0000_0002;
  reg [1:0] a = 2'b10; reg [0:0] b = 1'b0;
  wire [3:0] y;
  assign y[P -: 2] = a;
  assign y[1] = b;
  initial begin
    $display("V p=%0d", P);
    #1 $display("S t=1 y=%b", y);
    a = 2'b01; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &[
            "V p=-18446744073709551614",
            "S t=1 y=zz0z",
            "S t=2 y=zz1z",
            "A end",
        ],
    );
}

/// A 100-bit signed constant whose low word is 2 and whose value is negative: out of range. PRE = iverilog = sv2v
/// `S t=1 y=zz0z | S t=2 y=zz1z | A end`; R4 E3001. Cell ld4_outI_i17_f1_100s_vs_lo.
#[test]
fn i17_hundred_bit_negative_offset_runs() {
    pin(
        r#"module top;

  reg [1:0] a = 2'b10; reg [0:0] b = 1'b0;
  wire [3:0] y;
  assign y[100'shF_FFFF_FFFF_0000_0000_0000_0002 -: 2] = a;
  assign y[1] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 2'b01; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y=zz0z", "S t=2 y=zz1z", "A end"],
    );
}

/// A 65-bit signed localparam -(2^64 - 2): out of range. PRE = iverilog = sv2v `S t=1 y=zz0z | S t=2 y=zz1z | A
/// end`; R4 E3001. Cell ld4_outI_i24_f1_genparam.
#[test]
fn i24_wide_negative_param_offset_runs() {
    pin(
        r#"module top;
  localparam logic signed [64:0] P = 65'sh1_0000_0000_0000_0002;
  reg [1:0] a = 2'b10; reg [0:0] b = 1'b0;
  wire [3:0] y;
  assign y[P -: 2] = a;
  assign y[1] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 2'b01; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y=zz0z", "S t=2 y=zz1z", "A end"],
    );
}

/// A 128-bit signed constant with low word 1 and value -(2^64 - 1): out of range. PRE read the low word (bits
/// 2..1: a false overlap with `y[1]`). PRE = R4 E3001; iverilog = sv2v `S t=1 y=zz0z | S t=2 y=zz1z | A end`.
/// Cell ld4_outE_e10_128sneg_vs_lo.
#[test]
fn e10_128bit_negative_offset_runs() {
    pin(
        r#"module top;

  reg [1:0] a = 2'b10; reg [0:0] b = 1'b0;
  wire [3:0] y;
  assign y[128'shFFFF_FFFF_FFFF_FFFF_0000_0000_0000_0001 +: 2] = a;
  assign y[1] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 2'b01; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y=zz0z", "S t=2 y=zz1z", "A end"],
    );
}

/// `y[65'sh1_0000_0000_0000_0001]` is bit -(2^64 - 1), out of range (§11.5.1: no write). PRE read the low word,
/// bit 1. No oracle: iverilog and sv2v refuse the select. PRE = R4 E3001; iverilog `t.sv:5: sorry: part selects
/// straddling the start of signal (y[65'sb10000000000000000000000000000000000000000000000000000000000000001]) are
/// not currently supported.`; sv2v `s2v.v:5: sorry: part selects straddling the start of signal
/// (y[65'sb10000000000000000000000000000000000000000000000000000000000000001]) are not currently supported.`.
/// Cell ld4_outI_i14_f1_bit_vs1.
#[test]
fn i14_wide_negative_bit_select_runs() {
    pin(
        r#"module top;

  reg [0:0] a = 1'b1; reg [0:0] b = 1'b0;
  wire [3:0] y;
  assign y[65'sh1_0000_0000_0000_0001] = a;
  assign y[1] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 1'b0; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $display("A end"); $finish;
  end
endmodule
"#,
        &["S t=1 y=zz0z", "S t=2 y=zz1z", "A end"],
    );
}

/// `y[3:3-$clog2(4)+1]` is bits 3..2 (`$clog2` is now read), shared with `y[2]`. PRE and r4 skipped the chunk
/// (silent). PRE = R4 `S t=1 y=11zz`; iverilog = sv2v `S t=1 y=1xzz`. Cell ra4_ls1d_K41t_clog2_width_spelled.
#[test]
fn k41t_clog2_in_part_select_bound_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0; wire [3:0] y;
  assign y[3:3-$clog2(4)+1] = a;
  assign y[2] = b;
  initial begin
    #1 $display("S t=%0t y=%b", $time, y);
    #1 $finish;
  end
endmodule
"#,
    );
}

/// `y[$clog2(8) -: 2]` is bits 3..2, shared with `y[2]`. PRE = R4 `S t=1 y=11zz`; iverilog = sv2v `S t=1 y=1xzz`.
/// Cell ra4_ls1d_K42_clog2_base_dn.
#[test]
fn k42_clog2_offset_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0; wire [3:0] y;
  assign y[$clog2(8) -: 2] = a;
  assign y[2] = b;
  initial begin
    #1 $display("S t=%0t y=%b", $time, y);
    #1 $finish;
  end
endmodule
"#,
    );
}

/// k41t's select with a `'1` driver. PRE = R4 `S y=11zz | A end`; iverilog = sv2v `S y=1xzz | A end`. Cell
/// x05_clogpc_ovl.
#[test]
fn x05_clog2_part_select_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [3:0] y;
  assign y[3:3-$clog2(4)+1] = a;
  assign y[2] = b;
  initial begin #1 $display("S y=%b", y); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// k42's select on `wire [7:0] y`. PRE = R4 `S y=zzzz11zz | A end`; iverilog = sv2v `S y=zzzz1xzz | A end`. Cell
/// x07_clogdn_ovl.
#[test]
fn x07_clog2_offset_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [7:0] y;
  assign y[$clog2(8) -: 2] = a;
  assign y[2] = b;
  initial begin #1 $display("S y=%b", y); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// The same select (bits 3..2) beside `y[1]`: disjoint. PRE = R4 = iverilog = sv2v `S y=zzzz110z | A end`. Cell
/// x08_clogdn_disj.
#[test]
fn x08_clog2_offset_disjoint_runs() {
    pin(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [7:0] y;
  assign y[$clog2(8) -: 2] = a;
  assign y[1] = b;
  initial begin #1 $display("S y=%b", y); $display("A end"); $finish; end
endmodule
"#,
        &["S y=zzzz110z", "A end"],
    );
}

/// `64'hFFFF_FFFF_FFFF_FFFF + 64'd2` wraps at its 64-bit region to 1: `+: 2` is bits 2..1, shared with `y[1]`.
/// PRE = R4 E3001; iverilog = sv2v `S y=zzzzz1xz | A end`. Cell w03_wrap64_ovl.
#[test]
fn w03_offset_wraps_at_64_bits_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [7:0] y;
  assign y[64'hFFFF_FFFF_FFFF_FFFF + 64'd2 +: 2] = a;
  assign y[1] = b;
  initial begin #1 $display("S y=%b", y); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// The same select beside `y[3]`: disjoint. PRE = R4 = iverilog = sv2v `S y=zzzz011z | A end`. Cell
/// w04_wrap64_disj.
#[test]
fn w04_offset_wraps_at_64_bits_disjoint_runs() {
    pin(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [7:0] y;
  assign y[64'hFFFF_FFFF_FFFF_FFFF + 64'd2 +: 2] = a;
  assign y[3] = b;
  initial begin #1 $display("S y=%b", y); $display("A end"); $finish; end
endmodule
"#,
        &["S y=zzzz011z", "A end"],
    );
}

/// `64'hFFFF_FFFF_FFFF_FFFF + 64'd3` wraps to 2: `-: 2` is bits 2..1, shared with `y[1]`. PRE `S y=zzzzz10z | A
/// end`; R4 E3001; iverilog = sv2v `S y=zzzzz1xz | A end`. Cell w06_wrap64_dn.
#[test]
fn w06_offset_wraps_at_64_bits_minus_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [7:0] y;
  assign y[64'hFFFF_FFFF_FFFF_FFFF + 64'd3 -: 2] = a;
  assign y[1] = b;
  initial begin #1 $display("S y=%b", y); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// `y["a" +: 2]` is bits 98..97 (`"a"` = 97), shared with `y[97]`. r4 skipped a string-literal offset (silent).
/// PRE E3001; R4 `S y=zz10zzz | A end`; iverilog = sv2v `S y=zz1xzzz | A end`. Cell x01_str_ovl.
#[test]
fn x01_string_offset_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [127:0] y;
  assign y["a" +: 2] = a;
  assign y[97] = b;
  initial begin #1 $display("S y=%b", y[100:94]); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// The same select beside `y[99]`: disjoint. PRE = R4 = iverilog = sv2v `S y=z011zzz | A end`. Cell x02_str_disj.
#[test]
fn x02_string_offset_disjoint_runs() {
    pin(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [127:0] y;
  assign y["a" +: 2] = a;
  assign y[99] = b;
  initial begin #1 $display("S y=%b", y[100:94]); $display("A end"); $finish; end
endmodule
"#,
        &["S y=z011zzz", "A end"],
    );
}

/// `y["a"]` is bit 97, shared with `y[97]`. PRE E3001; R4 `S y=zzz0zzz | A end`; iverilog = sv2v `S y=zzzxzzz | A
/// end`. Cell s01_bit_ovl.
#[test]
fn s01_string_bit_select_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [0:0] a = '1;
  reg b = 1'b0;
  wire [127:0] y;
  assign y["a"] = a;
  assign y[97] = b;
  initial begin #1 $display("S y=%b", y[100:94]); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// `y["b" -: 2]` is bits 98..97, shared with `y[97]`. PRE skipped it (silent) too. PRE = R4 `S y=zz10zzz | A
/// end`; iverilog = sv2v `S y=zz1xzzz | A end`. Cell s02_dn_ovl.
#[test]
fn s02_string_minus_offset_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [127:0] y;
  assign y["b" -: 2] = a;
  assign y[97] = b;
  initial begin #1 $display("S y=%b", y[100:94]); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// The same select beside `y[96]`: disjoint. PRE = R4 = iverilog = sv2v `S y=zz110zz | A end`. Cell s03_dn_disj.
#[test]
fn s03_string_minus_offset_disjoint_runs() {
    pin(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [127:0] y;
  assign y["b" -: 2] = a;
  assign y[96] = b;
  initial begin #1 $display("S y=%b", y[100:94]); $display("A end"); $finish; end
endmodule
"#,
        &["S y=zz110zz", "A end"],
    );
}

/// `y["a" + 1 +: 2]` is bits 99..98, shared with `y[99]`. PRE E3001; R4 `S y=z01zzzz | A end`; iverilog = sv2v `S
/// y=zx1zzzz | A end`. Cell s05_tree_ovl.
#[test]
fn s05_string_sum_offset_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [127:0] y;
  assign y["a" + 1 +: 2] = a;
  assign y[99] = b;
  initial begin #1 $display("S y=%b", y[100:94]); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// The same select beside `y[97]`: disjoint. PRE = R4 = iverilog = sv2v `S y=z110zzz | A end`. Cell
/// s06_tree_disj.
#[test]
fn s06_string_sum_offset_disjoint_runs() {
    pin(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [127:0] y;
  assign y["a" + 1 +: 2] = a;
  assign y[97] = b;
  initial begin #1 $display("S y=%b", y[100:94]); $display("A end"); $finish; end
endmodule
"#,
        &["S y=z110zzz", "A end"],
    );
}

/// `"\001\001"` is 257: `+: 2` is bits 258..257, shared with `y[257]`. PRE E3001; R4 `S y=zz10zzz | A end`;
/// iverilog = sv2v `S y=zz1xzzz | A end`. Cell s09_2ch_ovl.
#[test]
fn s09_two_character_string_offset_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [300:0] y;
  assign y["\001\001" +: 2] = a;
  assign y[257] = b;
  initial begin #1 $display("S y=%b", y[260:254]); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// `""` is 0: `+: 2` is bits 1..0, shared with `y[0]`. PRE E3001; R4 `S y=zzzzzz10 | A end`; iverilog = sv2v `S
/// y=zzzzzz1x | A end`. Cell w01_empty_ovl.
#[test]
fn w01_empty_string_offset_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [7:0] y;
  assign y["" +: 2] = a;
  assign y[0] = b;
  initial begin #1 $display("S y=%b", y); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// The same select alone writes bits 1..0. PRE = R4 = iverilog = sv2v `S y=zzzzzz11 | A end`. Cell
/// w02_empty_solo.
#[test]
fn w02_empty_string_offset_single_driver() {
    pin(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [7:0] y;
  assign y["" +: 2] = a;
  initial begin #1 $display("S y=%b", y); $display("A end"); $finish; end
endmodule
"#,
        &["S y=zzzzzz11", "A end"],
    );
}

/// `"a" + 4'sb1111` is unsigned (a string literal is): 97 + 15 = 112, shared with `y[112]`. PRE E3001; R4 `S
/// y=zz10zzz | A end`; iverilog = sv2v `S y=zz1xzzz | A end`. Cell w10_str_s4.
#[test]
fn w10_string_plus_signed_leaf_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [127:0] y;
  assign y["a" + 4'sb1111 +: 2] = a;
  assign y[112] = b;
  initial begin #1 $display("S y=%b", y[115:109]); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// The same select alone writes bits 113..112. PRE = R4 = iverilog = sv2v `S y=zz11zzz | A end`. Cell
/// w11_str_s4_solo.
#[test]
fn w11_string_plus_signed_leaf_single_driver() {
    pin(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [127:0] y;
  assign y["a" + 4'sb1111 +: 2] = a;
  initial begin #1 $display("S y=%b", y[115:109]); $display("A end"); $finish; end
endmodule
"#,
        &["S y=zz11zzz", "A end"],
    );
}

/// `129'sh1_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF` is -1 (§5.7.1), so `+: 2` writes bit 0 (§11.5.1), shared
/// with `y[0]`. The store drops this select (a signed value of 129 bits or more, queued as Q-IDXVALUE); the check
/// reads it as IEEE does. No oracle runs it. PRE `S y=zzz0 | A end`; R4 E3001; iverilog `t.sv:5: sorry: part
/// selects straddling the start of signal (y[129'sb1+:'sd2]) are not currently supported.`; sv2v `s2v.v:5: sorry:
/// part selects straddling the start of signal (y[129'sb1+:'sd2]) are not currently supported.`. Cell
/// x12_s129_ovl.
#[test]
fn x12_129bit_minus_one_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [3:0] y;
  assign y[129'sh1_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF +: 2] = a;
  assign y[0] = b;
  initial begin #1 $display("S y=%b", y); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// A bare `'1` index is one bit (§5.7.1, self-determined): 1, so `+: 2` is bits 2..1, shared with `y[1]`. PRE =
/// R4 E3001; iverilog `S y=zzzzzzzzzzzzz1xz | A end`; sv2v `s2v.v:5: sorry: part selects straddling the start of
/// signal (y[1'sd-1+:'sd2]) are not currently supported.`. Cell f04_fill_bare_ovl1.
#[test]
fn f04_bare_fill_offset_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [15:0] y;
  assign y['1 +: 2] = a;
  assign y[1] = b;
  initial begin #1 $display("S y=%b", y); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// `'1 + 4'd0` sizes the fill to 4 bits: 15, so `-: 2` is bits 15..14, shared with `y[14]`. PRE skipped it
/// (silent). PRE `S y=10zzzzzzzzzzzzzz | A end`; R4 E3001; iverilog = sv2v `S y=1xzzzzzzzzzzzzzz | A end`. Cell
/// f06_fill_dn_ovl14.
#[test]
fn f06_fill_in_sum_offset_overlap_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [15:0] y;
  assign y['1 + 4'd0 -: 2] = a;
  assign y[14] = b;
  initial begin #1 $display("S y=%b", y); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// `"\200"` + 15 NULs is 2^127, out of range (§11.5.1 :12384-12386: no write), so beside `y[0]` nothing overlaps.
/// PRE read the low word (0: a false overlap). iverilog and sv2v truncate the index to its low word and are
/// disqualified here (the K36u precedent). PRE E3001; R4 `S y=zzzzzzz0 | A end`; iverilog = sv2v `S y=zzzzzz1x |
/// A end`. Cell s16_b127_vs0.
#[test]
fn s16_string_past_127_bits_runs() {
    pin(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [7:0] y;
  assign y["\200\000\000\000\000\000\000\000\000\000\000\000\000\000\000\000" +: 2] = a;
  assign y[0] = b;
  initial begin #1 $display("S y=%b", y); $display("A end"); $finish; end
endmodule
"#,
        &["S y=zzzzzzz0", "A end"],
    );
}

/// `$clog2(4) - 3` is -1 in a signed 32-bit region (`$clog2` returns an integer), so `+: 2` writes bit 0 (bits
/// 0..-1), shared with `y[0]`. PRE and r4 ran it (`zzzzzzz1`, the overlap missed). iverilog and sv2v refuse the
/// select (`sorry: part selects straddling the start of signal`); the read `x[$clog2(4) - 3 +: 2]` is `1x` on
/// vita = iverilog = sv2v, the store's signed reading. PRE = R4 `S y=zzzzzzz1 | A end`; iverilog `t.sv:5: sorry:
/// part selects straddling the start of signal (y[($clog2('sd4))-('sd3)+:'sd2]) are not currently supported.`;
/// sv2v `s2v.v:5: sorry: part selects straddling the start of signal (y[-('sd1)+:'sd2]) are not currently
/// supported.`. Cell cS2_clog_minus_int_vs0.
#[test]
fn cs2_clog2_minus_integer_writes_bit_zero_is_e3001() {
    e3001(
        r#"module top;
  reg [1:0] a = '1;
  reg b = 1'b0;
  wire [7:0] y;
  assign y[$clog2(4) - 3 +: 2] = a;
  assign y[0] = b;
  initial begin #1 $display("S y=%b", y); $display("A end"); $finish; end
endmodule
"#,
    );
}

/// A string literal past 64 bits with its top bit set is the unsigned integer it is (§5.9), 2^72 - 1: out of
/// range (§11.5.1 :12384-12386: no write), so beside `y[0]` nothing overlaps. Read signed (-1) it would write bit 0,
/// a false overlap. PRE = R4 = round 5 = iverilog = sv2v `S t=1 y=zzzzzzz0 | S t=2 y=zzzzzzz1`; verilator refuses
/// (`%Error: t.sv:4:12: Value too wide for 32-bits expected in this context 72'hffffffffffffffffff`). Lens-sound r5
/// cell mcx/cm1_str72_vs0. Kills CM1 (offset_value reads a string literal wider than 64 bits signed).
#[test]
fn cm1_string_past_64_bits_top_bit_set_runs() {
    pin_lines(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0;
  wire [7:0] y;
  assign y["\377\377\377\377\377\377\377\377\377" +: 2] = a;
  assign y[0] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 2'b00; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $finish;
  end
endmodule
"#,
        &["S "],
        &["S t=1 y=zzzzzzz0", "S t=2 y=zzzzzzz1"],
    );
}

/// `$clog2` of a string literal: the check does not read it (the `$clog2` arm declines a string argument;
/// Q-MDCONSTOFF cStr), the store reads `"\005"` as 5 (§5.9), `$clog2(5)` = 3, so `+: 2` writes bits 4..3, disjoint
/// from `y[0]`; the pair runs (and still does once the check reads it). Taking the width arm's 0 for the argument
/// would count bits 1..0, a false overlap. PRE = R4 = round 5 = iverilog = sv2v `S t=1 y=zzz11zz0 | S t=2
/// y=zzz00zz1`; verilator `00011000 | 00000001` (2-state, z reads 0: disqualified). Lens-sound r5 cell
/// mcx/cm2_clogstr_vs0. Kills CM2 (offset_value's `$clog2` arm takes a string argument).
#[test]
fn cm2_clog2_of_a_string_disjoint_runs() {
    pin_lines(
        r#"module top;
  reg [1:0] a = 2'b11; reg b = 1'b0;
  wire [7:0] y;
  assign y[$clog2("\005") +: 2] = a;
  assign y[0] = b;
  initial begin
    #1 $display("S t=1 y=%b", y);
    a = 2'b00; b = 1'b1;
    #1 $display("S t=2 y=%b", y);
    $finish;
  end
endmodule
"#,
        &["S "],
        &["S t=1 y=zzz11zz0", "S t=2 y=zzz00zz1"],
    );
}
