//! ENG-1c: a delayed (or `#0`) continuous assignment decides IEEE 1800-2017 §10.3.3 on the value its
//! left-hand side will HOLD — the store's own conversion, then its resize to the lvalue width
//! (`State::store_image`) — not on its right-hand side. Pinned on native, vm and interp; one cell
//! per change also through the staged flow.
//!
//! * C1 (the store image): an integral destination compares and times the bits the store keeps
//!   (`wire [3:0] y; assign #4 y = a8;` with `a8` 1 then 17 inside the delay is no change of `y`;
//!   a real rhs rounds at the net width and sign, or at the lvalue width for a concatenation); a
//!   real destination compares reals (`int` 3 then 5 into a `real` is 3.0 then 5.0).
//! * C2 (step b, ieee1800-2017.txt:10366-10367) on a real destination under IEEE 754 equality:
//!   `+0.0` and `-0.0` are one value; identical NaN bits are one value too (the recorded split
//!   Q-NANDCA). An integral destination keeps the bitwise test.
//! * C3 (step c, :10368): a new value equal to the one the left-hand side holds deschedules the
//!   pending write and schedules nothing; before the first landing a real destination holds `+0.0`.
//! * C4 (:10341-10345): a real destination falls on nonzero -> zero (either sign) and rises otherwise;
//!   the turn-off delay is never selected (a real holds no z; z -> 0.0 falls: no oracle, O1).
//! * C5: the time-0 copy-net repair does not re-write a delayed driver; its time-0 value is its own
//!   decision.
//! * Round 2: a word of a `real` array is "constant" as the store reads it (`ra[1]` of `real
//!   ra[1:2]`, `ra[1-1]`); the image of a 2-state destination has no x / z (IEEE 1800-2017 §6.11.3);
//!   a destination a continuous assignment may not write (a dynamic array element, §6.21,
//!   ieee1800-2017.txt:5436) keeps the right-hand-side decision; a `release` restores the last
//!   landed image (Q-RELDELAY).
//! * Round 3 (cost): the decision runs once per right-hand-side change, keyed on the whole value, and
//!   the image's 2-state x / z -> 0 is masked to each 2-state chunk (bit 63 included).
//!
//! Oracles: Icarus Verilog 13.0 `-g2012` (iv), sv2v 0.0.13 -> iverilog (s2v), Verilator 5.052
//! `--binary` (vl). Each cell's comment carries the oracles' raw lines and PRE's (vita at §4.5.622).
//! vl runs every header `always` once at time 0 (a "t0 artifact" these pins drop) and has no step
//! (c); iverilog rises on every real transition (disqualified for real rise / fall) and stores a
//! `-0.0` LITERAL as +0.0 (so every -0.0 here is `$bitstoreal(64'h8000000000000000)`); s2v cannot
//! parse a `#(r, f)` delay on an assign. An integral destination's rise / fall is iverilog's per-bit
//! atomic-at-max, vl's the vector rule (recorded split, ROADMAP §2). `-0.0` renders as `-0.000000`
//! (§4.5.619).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn scratch(tag: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dav_{tag}_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn vita(d: &std::path::Path, args: &[&str]) -> (String, String, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .current_dir(d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

/// The program's own stdout lines (no `simulation ended` line).
fn lines(out: &str) -> Vec<String> {
    out.lines()
        .filter(|l| !l.starts_with("simulation ended"))
        .map(str::to_string)
        .collect()
}

/// `src` on one backend: its stdout lines, after a clean exit.
fn run_on(src: &str, backend: &str) -> Vec<String> {
    let d = scratch(backend);
    std::fs::write(d.join("t.sv"), src).unwrap();
    let (out, err, code) = vita(&d, &["--backend", backend, "t.sv"]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(code, 0, "{backend}: exit {code}\n{out}{err}");
    lines(&out)
}

/// `src` through the staged flow (vcmp -> velab -> vrun): its stdout lines.
fn run_staged(src: &str) -> Vec<String> {
    let d = scratch("staged");
    std::fs::write(d.join("t.sv"), src).unwrap();
    for args in [
        &["vcmp", "t.sv", "-o", "t.vu"][..],
        &["velab", "t.vu", "-o", "t.velab"][..],
    ] {
        let (out, err, code) = vita(&d, args);
        assert_eq!(code, 0, "{args:?}: exit {code}\n{out}{err}");
    }
    let (out, err, code) = vita(&d, &["vrun", "t.velab"]);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(code, 0, "vrun: exit {code}\n{out}{err}");
    lines(&out)
}

/// Every backend prints exactly `want`.
fn pin(src: &str, want: &[&str]) {
    for be in ["native", "vm", "interp"] {
        assert_eq!(run_on(src, be), want, "backend {be}");
    }
}

/// [`pin`], and the staged flow prints it too.
fn pin_staged(src: &str, want: &[&str]) {
    pin(src, want);
    assert_eq!(run_staged(src), want, "staged vcmp -> velab -> vrun");
}

/// [`pin`] without the `W t=0 ` line. A delayed PART-SELECT driver of a net with another driver
/// owes no initial x in vita (`b=Z`; iverilog and sv2v `b=X`, verilator 0): the queued row
/// Q-DCAPARTX, not this rule, so no pin freezes that line.
fn pin_after_t0(src: &str, want: &[&str]) {
    for be in ["native", "vm", "interp"] {
        let got: Vec<String> = run_on(src, be)
            .into_iter()
            .filter(|l| !l.starts_with("W t=0 "))
            .collect();
        assert_eq!(got, want, "backend {be}");
    }
}

/// Every backend writes `r.vcd` whose body (from `$enddefinitions`) is `want`.
fn pin_vcd(src: &str, want: &[&str]) {
    for be in ["native", "vm", "interp"] {
        let d = scratch(be);
        std::fs::write(d.join("t.sv"), src).unwrap();
        let (out, err, code) = vita(&d, &["--backend", be, "t.sv"]);
        assert_eq!(code, 0, "{be}: {out}{err}");
        let vcd = std::fs::read_to_string(d.join("r.vcd")).expect("r.vcd");
        let _ = std::fs::remove_dir_all(&d);
        let body: Vec<&str> = vcd
            .lines()
            .skip_while(|l| !l.starts_with("$enddefinitions"))
            .collect();
        assert_eq!(body, want, "backend {be}");
    }
}

/// `src` is refused on every backend: exit 1 and a diagnostic line that starts with `at` and
/// carries `diag`.
fn refused(src: &str, at: &str, diag: &str) {
    for be in ["native", "vm", "interp"] {
        let d = scratch(be);
        std::fs::write(d.join("t.sv"), src).unwrap();
        let (out, err, code) = vita(&d, &["--backend", be, "t.sv"]);
        let _ = std::fs::remove_dir_all(&d);
        assert_eq!(code, 1, "{be}: exit {code}\n{out}{err}");
        let all = format!("{out}{err}");
        assert!(
            all.lines().any(|l| l.starts_with(at) && l.contains(diag)),
            "{be}: no `{at}…{diag}` line\n{all}"
        );
    }
}

/// C2 (b): -0.0 at t=5 is the value already scheduled (+0.0); nothing is rescheduled and `b` stays +0.0.
/// Cell `P14_ca_delay`. Oracles: iv "S t=7 bits=0000000000000000 | W t=10 bits=3ff0000000000000 | A end"; s2v "S t=7 bits=0000000000000000 | W t=10 bits=3ff0000000000000 | A end"; vl "W t=0 bits=0000000000000000 | S t=7 bits=0000000000000000 | W t=10 bits=3ff0000000000000 | A end".
/// PRE "S t=7 bits=8000000000000000 | W t=10 bits=3ff0000000000000 | A end".
#[test]
fn p14_negzero_does_not_differ_from_scheduled_zero() {
    pin_staged(
        r##"module top;
  real a = 0.0; real b;
  assign #1 b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #5 a = $bitstoreal(64'h8000000000000000); #2 $display("S t=%0t bits=%h", $time, $realtobits(b)); #2 a = 1.0; #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=7 bits=0000000000000000"##,
            r##"W t=10 bits=3ff0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C2 (b): as P14 with an expression rhs.
/// Cell `D02_ca_delay_mul`. Oracles: iv "S t=7 bits=0000000000000000 | S t=11 bits=3ff0000000000000 | A end"; s2v "S t=7 bits=0000000000000000 | S t=11 bits=3ff0000000000000 | A end"; vl "S t=7 bits=0000000000000000 | S t=11 bits=3ff0000000000000 | A end".
/// PRE "S t=7 bits=8000000000000000 | S t=11 bits=3ff0000000000000 | A end".
#[test]
fn d02_rhs_expression() {
    pin(
        r##"module top;
  real a = 0.0; real b;
  assign #1 b = a * 1.0;
  initial begin #5 a = $bitstoreal(64'h8000000000000000); #2 $display("S t=%0t bits=%h", $time, $realtobits(b)); #2 a = 1.0; #2 $display("S t=%0t bits=%h", $time, $realtobits(b)); $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=7 bits=0000000000000000"##,
            r##"S t=11 bits=3ff0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C3 (c): 1.0 then -0.0 before 1.0 lands: -0.0 equals `b` = +0.0, the pending write is descheduled and nothing is scheduled (vl has no step c).
/// Cell `D03_ca_delay_back_and_forth`. Oracles: iv "S t=9 bits=0000000000000000 | A end"; s2v "S t=9 bits=0000000000000000 | A end"; vl "S t=9 bits=8000000000000000 | A end".
/// PRE "S t=9 bits=8000000000000000 | A end".
#[test]
fn d03_supersede_to_negzero_equals_lhs() {
    pin(
        r##"module top;
  real a = 0.0; real b;
  assign #2 b = a;
  initial begin #5 a = 1.0; #1 a = $bitstoreal(64'h8000000000000000); #3 $display("S t=%0t bits=%h", $time, $realtobits(b)); $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=9 bits=0000000000000000"##, r##"A end"##],
    );
}

/// C2 (b): the delayed assign sits inside a submodule, its output a port.
/// Cell `D05_ca_delay_port`. Oracles: iv "S t=7 bits=0000000000000000 | A end"; s2v "S t=7 bits=0000000000000000 | A end"; vl "S t=7 bits=0000000000000000 | A end".
/// PRE "S t=7 bits=8000000000000000 | A end".
#[test]
fn d05_delayed_assign_behind_a_port() {
    pin(
        r##"module sub(input real i, output real o); assign #1 o = i; endmodule
module top;
  real a = 0.0; real b;
  sub u(.i(a), .o(b));
  initial begin #5 a = $bitstoreal(64'h8000000000000000); #2 $display("S t=%0t bits=%h", $time, $realtobits(b)); $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=7 bits=0000000000000000"##, r##"A end"##],
    );
}

/// C2 (b): `assign #0`.
/// Cell `D06_ca_delay0`. Oracles: iv "S t=7 bits=0000000000000000 | W t=8 bits=3ff0000000000000 | A end"; s2v "S t=7 bits=0000000000000000 | W t=8 bits=3ff0000000000000 | A end"; vl "W t=0 bits=0000000000000000 | S t=7 bits=0000000000000000 | W t=8 bits=3ff0000000000000 | A end".
/// PRE "S t=7 bits=8000000000000000 | W t=8 bits=3ff0000000000000 | A end".
#[test]
fn d06_delay0() {
    pin(
        r##"module top;
  real a = 0.0; real b;
  assign #0 b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #5 a = $bitstoreal(64'h8000000000000000); #2 $display("S t=%0t bits=%h", $time, $realtobits(b)); #1 a = 1.0; #1 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=7 bits=0000000000000000"##,
            r##"W t=8 bits=3ff0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C2: `wait ((1.0 / b) < 0.0)` holds until -1.0 lands at t=9 (`b` never takes the -0.0).
/// Cell `D07_ca_delay_wait`. Oracles: iv "W t=9 | A end"; s2v "W t=9 | A end"; vl "W t=9 | A end".
/// PRE "W t=6 | A end".
#[test]
fn d07_wait_on_delayed_net() {
    pin(
        r##"module top;
  real a = 0.0; real b;
  assign #1 b = a;
  initial begin #1; wait ((1.0 / b) < 0.0); $display("W t=%0t", $time); end
  initial begin #5 a = $bitstoreal(64'h8000000000000000); #3 a = -1.0; #3 $display("A end"); $finish; end
endmodule
"##,
        &[r##"W t=9"##, r##"A end"##],
    );
}

/// C3 (c): as D03 with a waiter on `b`.
/// Cell `D12_ca_delay_supersede_negzero_wake`. Oracles: iv "S t=10 bits=0000000000000000 | A end"; s2v "S t=10 bits=0000000000000000 | A end"; vl "W t=0 bits=0000000000000000 | S t=10 bits=8000000000000000 | A end".
/// PRE "S t=10 bits=8000000000000000 | A end".
#[test]
fn d12_supersede_to_negzero_no_wake() {
    pin(
        r##"module top;
  real a = 0.0; real b;
  assign #2 b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #5 a = 1.0; #1 a = $bitstoreal(64'h8000000000000000); #4 $display("S t=%0t bits=%h", $time, $realtobits(b)); $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=10 bits=0000000000000000"##, r##"A end"##],
    );
}

/// C2 (b): an `always_comb` reading the delayed net sees no -0.0.
/// Cell `D15_ca_delay_feeds_comb`. Oracles: iv "C t=0 bits=0000000000000000 | S t=7 bits=0000000000000000 | C t=9 bits=3fe0000000000000 | A end"; s2v "C t=0 bits=0000000000000000 | S t=7 bits=0000000000000000 | C t=9 bits=3fe0000000000000 | A end"; vl "C t=0 bits=0000000000000000 | C t=0 bits=0000000000000000 | C t=0 bits=0000000000000000 | C t=1 bits=0000000000000000 | C t=1 bits=0000000000000000 | C t=5 bits=0000000000000000 | C t=5 bits=0000000000000000 | S t=7 bits=0000000000000000 | C t=7 bits=0000000000000000 | C t=7 bits=0000000000000000 | C t=8 bits=0000000000000000 | C t=8 bits=0000000000000000 | C t=8 bits=0000000000000000 | C t=9 bits=3fe0000000000000 | C t=9 bits=3fe0000000000000 | A end | C t=10 bits=3fe0000000000000 | C t=10 bits=3fe0000000000000".
/// PRE "C t=0 bits=0000000000000000 | S t=7 bits=8000000000000000 | C t=9 bits=3fe0000000000000 | A end".
#[test]
fn d15_delayed_net_feeds_comb() {
    pin(
        r##"module top;
  real a = 0.0; real b; real x;
  assign #1 b = a;
  always_comb begin x = b; $display("C t=%0t bits=%h", $time, $realtobits(x)); end
  initial begin #5 a = $bitstoreal(64'h8000000000000000); #2 $display("S t=%0t bits=%h", $time, $realtobits(b)); #1 a = 0.5; #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"C t=0 bits=0000000000000000"##,
            r##"S t=7 bits=0000000000000000"##,
            r##"C t=9 bits=3fe0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C4: 1.0 -> 2.0 rises (t=11), 2.0 -> 0.0 falls (t=17), 0.0 -> -0.0 is no change, -0.0 -> 0.5 rises (t=26); iverilog rises on every real transition (disqualified for real rise / fall).
/// Cell `D16_ca_risefall_real`. Oracles: iv "W t=6 bits=3ff0000000000000 | W t=11 bits=4000000000000000 | W t=16 bits=0000000000000000 | W t=26 bits=3fe0000000000000 | A end"; s2v "(no sv2v->iverilog binary)"; vl "W t=0 bits=0000000000000000 | W t=6 bits=3ff0000000000000 | W t=11 bits=4000000000000000 | W t=17 bits=0000000000000000 | W t=26 bits=3fe0000000000000 | A end".
/// PRE "W t=6 bits=3ff0000000000000 | W t=12 bits=4000000000000000 | W t=17 bits=0000000000000000 | W t=27 bits=3fe0000000000000 | A end".
#[test]
fn d16_rise_fall_on_a_real() {
    pin_staged(
        r##"module top;
  real a = 0.0; real b;
  assign #(1,2) b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #5 a = 1.0; #5 a = 2.0; #5 a = 0.0; #5 a = $bitstoreal(64'h8000000000000000); #5 a = 0.5; #5 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=6 bits=3ff0000000000000"##,
            r##"W t=11 bits=4000000000000000"##,
            r##"W t=17 bits=0000000000000000"##,
            r##"W t=26 bits=3fe0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C2 (b) at time 0: the initializer's -0.0 does not differ from the +0.0 the first settle scheduled.
/// Cell `D17_ca_delay_negzero_init`. Oracles: iv "S t=3 bits=0000000000000000 | W t=6 bits=3ff0000000000000 | A end"; s2v "S t=3 bits=0000000000000000 | W t=6 bits=3ff0000000000000 | A end"; vl "W t=0 bits=0000000000000000 | S t=3 bits=8000000000000000 | W t=6 bits=3ff0000000000000 | A end".
/// PRE "S t=3 bits=8000000000000000 | W t=6 bits=3ff0000000000000 | A end".
#[test]
fn d17_negzero_initializer_under_scheduled_zero() {
    pin(
        r##"module top;
  real a = $bitstoreal(64'h8000000000000000); real b;
  assign #1 b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #3 $display("S t=%0t bits=%h", $time, $realtobits(b)); #2 a = 1.0; #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=3 bits=0000000000000000"##,
            r##"W t=6 bits=3ff0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C2: `$monitor` records no t=6 line (iv / s2v cannot monitor the expression).
/// Cell `D19_ca_delay_monitor`. Oracles: iv "SORRY: t.sv:4: currently only simple signals or constant expressions may be passed to $monitor. | NOTE: You can work around this by assigning the desired expression to an |       intermediate net (using a continuous assignment) and passing that net |       to $monitor."; s2v "SORRY: s2v.v:5: currently only simple signals or constant expressions may be passed to $monitor. | NOTE: You can work around this by assigning the desired expression to an |       intermediate net (using a continuous assignment) and passing that net |       to $monitor."; vl "M t=0 bits=0000000000000000 | M t=9 bits=3fe0000000000000 | A end".
/// PRE "M t=0 bits=0000000000000000 | M t=6 bits=8000000000000000 | M t=9 bits=3fe0000000000000 | A end".
#[test]
fn d19_monitor_sees_no_negzero_store() {
    pin(
        r##"module top;
  real a = 0.0; real b;
  assign #1 b = a;
  initial $monitor("M t=%0t bits=%h", $time, $realtobits(b));
  initial begin #5 a = $bitstoreal(64'h8000000000000000); #3 a = 0.5; #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"M t=0 bits=0000000000000000"##,
            r##"M t=9 bits=3fe0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// control: an undelayed assign stores -0.0 (E1-a: a value change, no event).
/// Cell `D04_ca_nodelay_control`. Oracles: iv "S t=7 bits=8000000000000000 | A end"; s2v "S t=7 bits=8000000000000000 | A end"; vl "S t=7 bits=8000000000000000 | A end".
/// PRE "S t=7 bits=8000000000000000 | A end".
#[test]
fn d04_undelayed_control() {
    pin(
        r##"module top;
  real a = 0.0; real b;
  assign b = a;
  initial begin #5 a = $bitstoreal(64'h8000000000000000); #2 $display("S t=%0t bits=%h", $time, $realtobits(b)); $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=7 bits=8000000000000000"##, r##"A end"##],
    );
}

/// control: nonzero values schedule and land as before.
/// Cell `D08_ca_delay_nonzero_control`. Oracles: iv "W t=6 bits=3fe0000000000000 | W t=8 bits=bfe0000000000000 | A end"; s2v "W t=6 bits=3fe0000000000000 | W t=8 bits=bfe0000000000000 | A end"; vl "W t=0 bits=0000000000000000 | W t=6 bits=3fe0000000000000 | W t=8 bits=bfe0000000000000 | A end".
/// PRE "W t=6 bits=3fe0000000000000 | W t=8 bits=bfe0000000000000 | A end".
#[test]
fn d08_nonzero_control() {
    pin(
        r##"module top;
  real a = 0.0; real b;
  assign #1 b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #5 a = 0.5; #2 a = -0.5; #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=6 bits=3fe0000000000000"##,
            r##"W t=8 bits=bfe0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// control: a real rhs into an `int` destination, unchanged values.
/// Cell `D10_ca_delay_real_to_int`. Oracles: iv "W t=1 b=0 | S t=7 b=0 | W t=9 b=2 | S t=10 b=2 | A end"; s2v "W t=1 b=0 | S t=7 b=0 | W t=9 b=2 | S t=10 b=2 | A end"; vl "W t=0 b=0 | S t=7 b=0 | W t=9 b=2 | S t=10 b=2 | A end".
/// PRE "S t=7 b=0 | W t=9 b=2 | S t=10 b=2 | A end".
#[test]
fn d10_real_to_int_destination_control() {
    pin(
        r##"module top;
  real a = 0.0; int b;
  assign #1 b = a;
  always @(b) $display("W t=%0t b=%0d", $time, b);
  initial begin #5 a = $bitstoreal(64'h8000000000000000); #2 $display("S t=%0t b=%0d", $time, b); #1 a = 2.0; #2 $display("S t=%0t b=%0d", $time, b); $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=7 b=0"##,
            r##"W t=9 b=2"##,
            r##"S t=10 b=2"##,
            r##"A end"##,
        ],
    );
}

/// control: a return to the value driven before lands as before.
/// Cell `D11_ca_delay_return_to_driven`. Oracles: iv "S t=10 bits=0000000000000000 | A end"; s2v "S t=10 bits=0000000000000000 | A end"; vl "W t=0 bits=0000000000000000 | S t=10 bits=0000000000000000 | A end".
/// PRE "S t=10 bits=0000000000000000 | A end".
#[test]
fn d11_return_to_driven_control() {
    pin(
        r##"module top;
  real a = 0.0; real b;
  assign #2 b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #5 a = 1.0; #1 a = 0.0; #4 $display("S t=%0t bits=%h", $time, $realtobits(b)); $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=10 bits=0000000000000000"##, r##"A end"##],
    );
}

/// recorded split Q-NANDCA: identical NaN bits are the same value for (b), so the NaN lands once; IEEE 754 NaN != NaN would reschedule it on every settle pass. Since r3 the repeat (`a * c` keeps the bits) ends at the raw-rhs skip before (b), so mutant N5 no longer reaches it here: `d13c_identical_nan_bits_equal_the_left_hand_side` observes N5.
/// Cell `D13_ca_delay_nan_identical`. Oracles: iv "S t=7 bits=0000000000000000 | W t=8 bits=7ff8000000000001 | S t=9 bits=7ff8000000000001 | A end"; s2v "S t=7 bits=0000000000000000 | W t=8 bits=7ff8000000000001 | S t=9 bits=7ff8000000000001 | A end"; vl "W t=0 bits=0000000000000000 | %Error-DIDNOTCONVERGE: Active region did not converge after '--converge-limit' of 10000 tries | Aborting...".
/// PRE "S t=7 bits=7ff8000000000001 | W t=7 bits=7ff8000000000001 | S t=9 bits=7ff8000000000001 | A end".
#[test]
fn d13_identical_nan_control() {
    pin(
        r##"module top;
  real a = 0.0; real c = 1.0; real b;
  assign #2 b = a * c;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #5 a = $bitstoreal(64'h7ff8000000000001); #1 c = 2.0; #1 $display("S t=%0t bits=%h", $time, $realtobits(b)); #2 $display("S t=%0t bits=%h", $time, $realtobits(b)); $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=7 bits=7ff8000000000001"##,
            r##"W t=7 bits=7ff8000000000001"##,
            r##"S t=9 bits=7ff8000000000001"##,
            r##"A end"##,
        ],
    );
}

/// recorded split Q-NANDCA at (c) (mutant N5): the rhs returns to the landed NaN bits while 1.0 is pending, so (c) deschedules 1.0 and schedules nothing; the cancelled write's tick ends the run (12), as for any (c) cancel (cell `n5/k1_cancel_end_time_control`: iv, s2v and vita `F t=12`). IEEE 754 NaN != NaN would schedule the NaN again (`F t=13`).
/// Cell `n5/c1_back_to_landed_nan`. Oracles: iv "W t=3 bits=7ff8000000000001 | W t=13 bits=7ff8000000000001 | F t=13 bits=7ff8000000000001"; s2v "W t=3 bits=7ff8000000000001 | W t=13 bits=7ff8000000000001 | F t=13 bits=7ff8000000000001"; vl "W t=0 bits=0000000000000000 | %Error-DIDNOTCONVERGE: Active region did not converge after '--converge-limit' of 10000 tries | Aborting...".
/// PRE "W t=3 bits=7ff8000000000001 | F t=13 bits=7ff8000000000001".
#[test]
fn d13c_identical_nan_bits_equal_the_left_hand_side() {
    pin(
        r##"module top;
  real a = 0.0; real b;
  assign #2 b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #1 a = $bitstoreal(64'h7ff8000000000001); #9 a = 1.0; #1 a = $bitstoreal(64'h7ff8000000000001); end
  final $display("F t=%0t bits=%h", $time, $realtobits(b));
endmodule
"##,
        &[
            r##"W t=3 bits=7ff8000000000001"##,
            r##"F t=12 bits=7ff8000000000001"##,
        ],
    );
}

/// control: NaNs with different payloads are different values.
/// Cell `D14_ca_delay_nan_payload`. Oracles: iv "W t=6 bits=7ff8000000000001 | W t=8 bits=7ff8000000000002 | S t=9 bits=7ff8000000000002 | A end"; s2v "W t=6 bits=7ff8000000000001 | W t=8 bits=7ff8000000000002 | S t=9 bits=7ff8000000000002 | A end"; vl "W t=0 bits=0000000000000000 | %Error-DIDNOTCONVERGE: Active region did not converge after '--converge-limit' of 10000 tries | Aborting...".
/// PRE "W t=6 bits=7ff8000000000001 | W t=8 bits=7ff8000000000002 | S t=9 bits=7ff8000000000002 | A end".
#[test]
fn d14_nan_payload_control() {
    pin(
        r##"module top;
  real a = 0.0; real b;
  assign #1 b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #5 a = $bitstoreal(64'h7ff8000000000001); #2 a = $bitstoreal(64'h7ff8000000000002); #2 $display("S t=%0t bits=%h", $time, $realtobits(b)); $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=6 bits=7ff8000000000001"##,
            r##"W t=8 bits=7ff8000000000002"##,
            r##"S t=9 bits=7ff8000000000002"##,
            r##"A end"##,
        ],
    );
}

/// control: -0.0 then back to a nonzero value.
/// Cell `D18_ca_delay_negzero_then_back`. Oracles: iv "S t=10 bits=0000000000000000 | A end"; s2v "S t=10 bits=0000000000000000 | A end"; vl "W t=0 bits=0000000000000000 | S t=10 bits=0000000000000000 | A end".
/// PRE "S t=10 bits=0000000000000000 | A end".
#[test]
fn d18_negzero_then_back_control() {
    pin(
        r##"module top;
  real a = 0.0; real b;
  assign #2 b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #5 a = $bitstoreal(64'h8000000000000000); #1 a = 0.0; #4 $display("S t=%0t bits=%h", $time, $realtobits(b)); $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=10 bits=0000000000000000"##, r##"A end"##],
    );
}

/// control (mutant M22): an integral destination keeps the bitwise (b).
/// Cell `D21_ca_delay_inertial_int_twin`. Oracles: iv "W t=2 b=0000000000000000 | S t=10 b=0000000000000000 | A end"; s2v "W t=2 b=0000000000000000 | S t=10 b=0000000000000000 | A end"; vl "W t=0 b=0000000000000000 | S t=10 b=0000000000000000 | A end".
/// PRE "W t=2 b=0000000000000000 | S t=10 b=0000000000000000 | A end".
#[test]
fn d21_int_inertial_twin_control() {
    pin(
        r##"module top;
  logic [63:0] a = 0; logic [63:0] b;
  assign #2 b = a;
  always @(b) $display("W t=%0t b=%h", $time, b);
  initial begin #5 a = 1; #1 a = 0; #4 $display("S t=%0t b=%h", $time, b); $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=2 b=0000000000000000"##,
            r##"S t=10 b=0000000000000000"##,
            r##"A end"##,
        ],
    );
}

/// control (mutant M22): a 64-bit integral destination whose sign bit flips over zero is a change (754 equality is for reals only).
/// Cell `I02_ca_delay_int64_signbit`. Oracles: iv "W t=1 b=0000000000000000 | W t=6 b=8000000000000000 | A end"; s2v "W t=1 b=0000000000000000 | W t=6 b=8000000000000000 | A end"; vl "W t=0 b=0000000000000000 | W t=6 b=8000000000000000 | A end".
/// PRE "W t=1 b=0000000000000000 | W t=6 b=8000000000000000 | A end".
#[test]
fn i02_int64_sign_bit_is_a_change() {
    pin(
        r##"module top;
  logic [63:0] a = 0; logic [63:0] b;
  assign #1 b = a;
  always @(b) $display("W t=%0t b=%h", $time, b);
  initial begin #5 a = 64'h8000000000000000; #3 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=1 b=0000000000000000"##,
            r##"W t=6 b=8000000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C3 (c) before any landing: the lhs of a real `assign #0` holds +0.0 until its first landing, so the initializer's -0.0 schedules nothing.
/// Cell `D22_ca_delay0_negzero_init_t0`. Oracles: iv "S t=1 bits=0000000000000000 | W t=3 bits=3ff0000000000000 | A end"; s2v "S t=1 bits=0000000000000000 | W t=3 bits=3ff0000000000000 | A end"; vl "W t=0 bits=0000000000000000 | S t=1 bits=8000000000000000 | W t=3 bits=3ff0000000000000 | A end".
/// PRE "S t=1 bits=8000000000000000 | W t=3 bits=3ff0000000000000 | A end".
#[test]
fn d22_delay0_negzero_initializer_lhs_undriven() {
    pin(
        r##"module top;
  real a = $bitstoreal(64'h8000000000000000); real b;
  assign #0 b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #1 $display("S t=%0t bits=%h", $time, $realtobits(b)); #2 a = 1.0; #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=1 bits=0000000000000000"##,
            r##"W t=3 bits=3ff0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C3 (c) before any landing: 1.0 is cancelled by -0.0, which equals the undriven lhs (+0.0).
/// Cell `D23_ca_delay_cancel_to_negzero_undriven`. Oracles: iv "S t=6 bits=0000000000000000 | A end"; s2v "S t=6 bits=0000000000000000 | A end"; vl "W t=0 bits=0000000000000000 | S t=6 bits=8000000000000000 | A end".
/// PRE "S t=6 bits=8000000000000000 | A end".
#[test]
fn d23_cancel_to_negzero_before_any_landing() {
    pin(
        r##"module top;
  real a = 0.0; real b;
  assign #3 b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #1 a = 1.0; #1 a = $bitstoreal(64'h8000000000000000); #4 $display("S t=%0t bits=%h", $time, $realtobits(b)); $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=6 bits=0000000000000000"##, r##"A end"##],
    );
}

/// C3 (c) bookkeeping (mutant M28): after (c) cancels 1.0, a later 1.0 schedules again.
/// Cell `D24_ca_delay_cancel_then_same_again`. Oracles: iv "W t=10 bits=3ff0000000000000 | S t=12 bits=3ff0000000000000 | A end"; s2v "W t=10 bits=3ff0000000000000 | S t=12 bits=3ff0000000000000 | A end"; vl "W t=0 bits=0000000000000000 | W t=10 bits=3ff0000000000000 | S t=12 bits=3ff0000000000000 | A end".
/// PRE "W t=10 bits=3ff0000000000000 | S t=12 bits=3ff0000000000000 | A end".
#[test]
fn d24_cancel_then_same_value_again() {
    pin(
        r##"module top;
  real a = 0.0; real b;
  assign #2 b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #5 a = 1.0; #1 a = $bitstoreal(64'h8000000000000000); #2 a = 1.0; #4 $display("S t=%0t bits=%h", $time, $realtobits(b)); $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=10 bits=3ff0000000000000"##,
            r##"S t=12 bits=3ff0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C3 (c) bookkeeping (mutant M28): (c) against a landed value, then the cancelled value again.
/// Cell `D25_ca_delay_landed_cancel_then_same`. Oracles: iv "W t=7 bits=3ff0000000000000 | W t=12 bits=4000000000000000 | S t=14 bits=4000000000000000 | A end"; s2v "W t=7 bits=3ff0000000000000 | W t=12 bits=4000000000000000 | S t=14 bits=4000000000000000 | A end"; vl "W t=0 bits=0000000000000000 | W t=7 bits=3ff0000000000000 | W t=12 bits=4000000000000000 | S t=14 bits=4000000000000000 | A end".
/// PRE "W t=7 bits=3ff0000000000000 | W t=12 bits=4000000000000000 | S t=14 bits=4000000000000000 | A end".
#[test]
fn d25_landed_cancel_then_same_value_again() {
    pin(
        r##"module top;
  real a = 0.0; real b;
  assign #2 b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #5 a = 1.0; #3 a = 2.0; #1 a = 1.0; #1 a = 2.0; #4 $display("S t=%0t bits=%h", $time, $realtobits(b)); $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=7 bits=3ff0000000000000"##,
            r##"W t=12 bits=4000000000000000"##,
            r##"S t=14 bits=4000000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C2 (b) (mutant M20): +0.0 is scheduled over b = 1.0, then -0.0 arrives: the same value as the scheduled one, so the t=7 landing stands.
/// Cell `D26_ca_delay_negzero_under_scheduled_zero`. Oracles: iv "W t=2 bits=3ff0000000000000 | W t=7 bits=0000000000000000 | S t=10 bits=0000000000000000 | A end"; s2v "W t=2 bits=3ff0000000000000 | W t=7 bits=0000000000000000 | S t=10 bits=0000000000000000 | A end"; vl "W t=0 bits=0000000000000000 | W t=2 bits=3ff0000000000000 | W t=7 bits=0000000000000000 | S t=10 bits=0000000000000000 | A end".
/// PRE "W t=2 bits=3ff0000000000000 | W t=8 bits=8000000000000000 | S t=10 bits=8000000000000000 | A end".
#[test]
fn d26_negzero_under_scheduled_zero_keeps_the_schedule() {
    pin(
        r##"module top;
  real a = 1.0; real b;
  assign #2 b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #5 a = 0.0; #1 a = $bitstoreal(64'h8000000000000000); #4 $display("S t=%0t bits=%h", $time, $realtobits(b)); $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=2 bits=3ff0000000000000"##,
            r##"W t=7 bits=0000000000000000"##,
            r##"S t=10 bits=0000000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C2 (b) (mutant M20, second killer): -0.0 is scheduled over b = 1.0, then +0.0 arrives: the same value as the scheduled one, so -0.0 lands at t=7 (a bitwise (b) rescheduled +0.0 for t=8).
/// Cell `M20b_negzero_scheduled_then_poszero`. Oracles: iv "W t=2 bits=3ff0000000000000 | W t=7 bits=8000000000000000 | S t=10 bits=8000000000000000 | A end"; s2v "W t=2 bits=3ff0000000000000 | W t=7 bits=8000000000000000 | S t=10 bits=8000000000000000 | A end"; vl "W t=0 bits=0000000000000000 | W t=2 bits=3ff0000000000000 | W t=7 bits=8000000000000000 | S t=10 bits=8000000000000000 | A end".
/// PRE "W t=2 bits=3ff0000000000000 | W t=8 bits=0000000000000000 | S t=10 bits=0000000000000000 | A end".
#[test]
fn m20b_poszero_under_scheduled_negzero_keeps_the_schedule() {
    pin(
        r##"module top;
  real a = 1.0; real b;
  assign #2 b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #5 a = $bitstoreal(64'h8000000000000000); #1 a = 0.0; #4 $display("S t=%0t bits=%h", $time, $realtobits(b)); $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=2 bits=3ff0000000000000"##,
            r##"W t=7 bits=8000000000000000"##,
            r##"S t=10 bits=8000000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C2 (b): the dump records no t=6 value for `b`: the delayed assign scheduled nothing (`a` itself is recorded `r-0` at #5, §4.5.619).
/// Cell `D20_ca_delay_vcd`. VCD body from `$enddefinitions`: iv "$enddefinitions $end / $comment Show the parameter values. $end / $dumpall / $end / #0 / $dumpvars / r0 " / r0 ! / $end / #5 / r-0 ! / #8 / r0.5 ! / #9 / r0.5 " / #10"; s2v "$enddefinitions $end / $comment Show the parameter values. $end / $dumpall / $end / #0 / $dumpvars / r0 " / r0 ! / $end / #5 / r-0 ! / #8 / r0.5 ! / #9 / r0.5 " / #10"; vl writes no dump.
/// PRE "$enddefinitions $end / $dumpvars / r0 ! / r0 " / $end / #0 / #5 / r-0 ! / #6 / r-0 " / #8 / r0.5 ! / #9 / r0.5 "".
#[test]
fn d20_dump_has_no_delayed_negzero_record() {
    pin_vcd(
        r##"module top;
  real a = 0.0; real b;
  assign #1 b = a;
  initial begin $dumpfile("r.vcd"); $dumpvars(0, top); end
  initial begin #5 a = $bitstoreal(64'h8000000000000000); #3 a = 0.5; #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"$enddefinitions $end"##,
            r##"$dumpvars"##,
            r##"r0 !"##,
            r##"r0 ""##,
            r##"$end"##,
            r##"#0"##,
            r##"#5"##,
            r##"r-0 !"##,
            r##"#8"##,
            r##"r0.5 !"##,
            r##"#9"##,
            r##"r0.5 ""##,
        ],
    );
}

/// C3 (c): 1.0 at #5 then -0.0 at #6 cancels the pending 1.0; `b` gets no record until 0.5 lands at #12.
/// Cell `V02_vcd_dca_negzero`. VCD body from `$enddefinitions`: iv "$enddefinitions $end / $comment Show the parameter values. $end / $dumpall / $end / #0 / $dumpvars / r0 " / r0 ! / $end / #5 / r1 ! / #6 / r-0 ! / #10 / r0.5 ! / #12 / r0.5 " / #13"; s2v "$enddefinitions $end / $comment Show the parameter values. $end / $dumpall / $end / #0 / $dumpvars / r0 " / r0 ! / $end / #5 / r1 ! / #6 / r-0 ! / #10 / r0.5 ! / #12 / r0.5 " / #13"; vl writes no dump.
/// PRE "$enddefinitions $end / $dumpvars / r0 ! / r0 " / $end / #0 / #5 / r1 ! / #6 / r-0 ! / #8 / r-0 " / #10 / r0.5 ! / #12 / r0.5 "".
#[test]
fn v02_dump_of_a_cancelled_write() {
    pin_vcd(
        r##"module top;
  real a = 0.0; real b;
  assign #2 b = a;
  initial begin $dumpfile("r.vcd"); $dumpvars(0, top); end
  initial begin #5 a = 1.0; #1 a = $bitstoreal(64'h8000000000000000); #4 a = 0.5; #3 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"$enddefinitions $end"##,
            r##"$dumpvars"##,
            r##"r0 !"##,
            r##"r0 ""##,
            r##"$end"##,
            r##"#0"##,
            r##"#5"##,
            r##"r1 !"##,
            r##"#6"##,
            r##"r-0 !"##,
            r##"#10"##,
            r##"r0.5 !"##,
            r##"#12"##,
            r##"r0.5 ""##,
        ],
    );
}

/// C1: a real rhs into a 1-bit destination rounds, then rises / falls on that bit.
/// Cell `T08_dca_bit_lhs_real`. Oracles: iv "W t=4 b=0 | W t=6 b=1 | W t=14 b=0 | W t=21 b=1 | A end"; s2v "(no sv2v->iverilog binary)"; vl "W t=0 b=0 | W t=6 b=1 | W t=14 b=0 | W t=21 b=1 | A end".
/// PRE "W t=4 b=0 | W t=6 b=1 | W t=14 b=0 | W t=24 b=1 | A end".
#[test]
fn t08_real_rhs_into_one_bit() {
    pin(
        r##"module top;
  real r = 0.0; logic b;
  assign #(1,4) b = r;
  always @(b) $display("W t=%0t b=%b", $time, b);
  initial begin #5 r = 0.75; #5 r = 0.4; #10 r = 0.9; #10 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=4 b=0"##,
            r##"W t=6 b=1"##,
            r##"W t=14 b=0"##,
            r##"W t=21 b=1"##,
            r##"A end"##,
        ],
    );
}

/// C3 (c) on constant words of a real array, at time 0.
/// Cell `B03b_dca_fullarr_negzero_init`. Oracles: iv "S t=3 bits=0000000000000000 | A end"; s2v "S t=3 bits=0000000000000000 | A end"; vl "S t=3 bits=8000000000000000 | A end".
/// PRE "S t=3 bits=8000000000000000 | A end".
#[test]
fn b03b_array_words_negzero_initializer() {
    pin(
        r##"module top;
  real a = $bitstoreal(64'h8000000000000000); real ra [0:1];
  assign #1 ra[0] = a;
  assign #1 ra[1] = a;
  initial begin #3 $display("S t=%0t bits=%h", $time, $realtobits(ra[0])); #2 a = 1.0; #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=3 bits=0000000000000000"##, r##"A end"##],
    );
}

/// C2 (b) on constant words of a real array.
/// Cell `B03c_dca_fullarr_negzero_later`. Oracles: iv "S t=8 bits=0000000000000000 | A end"; s2v "S t=8 bits=0000000000000000 | A end"; vl "S t=8 bits=0000000000000000 | A end".
/// PRE "S t=8 bits=8000000000000000 | A end".
#[test]
fn b03c_array_words_negzero_later() {
    pin(
        r##"module top;
  real a = 0.0; real ra [0:1];
  assign #2 ra[0] = a;
  assign #2 ra[1] = a;
  initial begin #5 a = $bitstoreal(64'h8000000000000000); #3 $display("S t=%0t bits=%h", $time, $realtobits(ra[0])); #1 a = 1.0; #3 $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=8 bits=0000000000000000"##, r##"A end"##],
    );
}

/// C1: 3.0 -> 3.2 rounds to the same `int` 3: no reschedule, 3 lands at t=14.
/// Cell `T09_dca_int_lhs_real_single_resched`. Oracles: iv "W t=4 b=1 | W t=14 b=3 | A end b=3"; s2v "W t=4 b=1 | W t=14 b=3 | A end b=3"; vl "W t=0 b=0 | W t=4 b=1 | W t=14 b=3 | A end b=3".
/// PRE "W t=4 b=1 | W t=16 b=3 | A end b=3".
#[test]
fn t09_real_rhs_same_integer_no_reschedule() {
    pin(
        r##"module top;
  real r = 1.0; int b;
  assign #4 b = r;
  always @(b) $display("W t=%0t b=%0d", $time, b);
  initial begin #10 r = 3.0; #2 r = 3.2; #10 $display("A end b=%0d", b); $finish; end
endmodule
"##,
        &[r##"W t=4 b=1"##, r##"W t=14 b=3"##, r##"A end b=3"##],
    );
}

/// C4 on the runtime delay lane `#(dr, df)`: 2.0 -> 0.0 falls (t=18).
/// Cell `B10_dca_runtime_risefall`. Oracles: iv "W t=6 bits=3ff0000000000000 | W t=11 bits=4000000000000000 | W t=16 bits=0000000000000000 | W t=26 bits=3fe0000000000000 | A end"; s2v "(no sv2v->iverilog binary)"; vl "W t=0 bits=0000000000000000 | W t=6 bits=3ff0000000000000 | W t=11 bits=4000000000000000 | W t=18 bits=0000000000000000 | W t=26 bits=3fe0000000000000 | A end".
/// PRE "W t=6 bits=3ff0000000000000 | W t=13 bits=4000000000000000 | W t=18 bits=0000000000000000 | W t=28 bits=3fe0000000000000 | A end".
#[test]
fn b10_runtime_rise_fall_on_a_real() {
    pin(
        r##"module top;
  real a = 0.0; real b; int dr = 1, df = 3;
  assign #(dr, df) b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #5 a = 1.0; #5 a = 2.0; #5 a = 0.0; #5 a = $bitstoreal(64'h8000000000000000); #5 a = 0.5; #5 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=6 bits=3ff0000000000000"##,
            r##"W t=11 bits=4000000000000000"##,
            r##"W t=18 bits=0000000000000000"##,
            r##"W t=26 bits=3fe0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C3 (c) on the runtime delay lane `#(d)`.
/// Cell `B09_dca_runtime_delay`. Oracles: iv "S t=10 bits=0000000000000000 | W t=13 bits=3fe0000000000000 | A end"; s2v "S t=10 bits=0000000000000000 | W t=13 bits=3fe0000000000000 | A end"; vl "W t=0 bits=0000000000000000 | S t=10 bits=8000000000000000 | W t=13 bits=3fe0000000000000 | A end".
/// PRE "S t=10 bits=8000000000000000 | W t=13 bits=3fe0000000000000 | A end".
#[test]
fn b09_runtime_delay_cancel_to_negzero() {
    pin(
        r##"module top;
  real a = 0.0; real b; int d = 2;
  assign #(d) b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #5 a = 1.0; #1 a = $bitstoreal(64'h8000000000000000); #4 $display("S t=%0t bits=%h", $time, $realtobits(b)); #1 a = 0.5; #3 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=10 bits=0000000000000000"##,
            r##"W t=13 bits=3fe0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C3 (c) into an interface member.
/// Cell `B12_dca_interface_lhs`. Oracles: iv "S t=10 bits=0000000000000000 | A end"; s2v "S t=10 bits=0000000000000000 | A end"; vl "W t=0 bits=0000000000000000 | S t=10 bits=8000000000000000 | A end".
/// PRE "S t=10 bits=8000000000000000 | A end".
#[test]
fn b12_interface_member_destination() {
    pin(
        r##"interface ifc; real b; endinterface
module top;
  ifc i();
  real a = 0.0;
  assign #2 i.b = a;
  always @(i.b) $display("W t=%0t bits=%h", $time, $realtobits(i.b));
  initial begin #5 a = 1.0; #1 a = $bitstoreal(64'h8000000000000000); #4 $display("S t=%0t bits=%h", $time, $realtobits(i.b)); $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=10 bits=0000000000000000"##, r##"A end"##],
    );
}

/// C1: a real rhs into an `int` under `#(1,4)` takes the per-bit delay on the rounded value (iverilog's `t=4 b=0` is its x for a 2-state `int`).
/// Cell `B04_dca_int_lhs_risefall`. Oracles: iv "W t=4 b=0 | W t=6 b=1 | W t=14 b=0 | W t=21 b=2 | W t=34 b=0 | A end"; s2v "(no sv2v->iverilog binary)"; vl "W t=0 b=0 | W t=6 b=1 | W t=14 b=0 | W t=21 b=2 | W t=34 b=0 | A end".
/// PRE "W t=6 b=1 | W t=14 b=0 | W t=24 b=2 | W t=34 b=0 | A end".
#[test]
fn b04_real_rhs_into_int_rise_fall() {
    pin(
        r##"module top;
  real r = 0.0; int b;
  assign #(1,4) b = r;
  always @(b) $display("W t=%0t b=%0d", $time, b);
  initial begin #5 r = 1.0; #5 r = 0.3; #10 r = 2.0; #10 r = -0.4; #10 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=6 b=1"##,
            r##"W t=14 b=0"##,
            r##"W t=21 b=2"##,
            r##"W t=34 b=0"##,
            r##"A end"##,
        ],
    );
}

/// C3 (c): a submodule's delayed output port drives a word of a real array.
/// Cell `B17_dca_port_out_elem`. Oracles: iv "S t=9 bits=0000000000000000 | A end"; s2v "S t=9 bits=0000000000000000 | A end"; vl "W t=0 bits=0000000000000000 | S t=9 bits=0000000000000000 | A end".
/// PRE "S t=9 bits=8000000000000000 | A end".
#[test]
fn b17_port_output_into_a_real_word() {
    pin(
        r##"module sub(input real i, output real o); assign #2 o = i; endmodule
module top;
  real a = 0.0; real rb [0:1];
  sub u(.i(a), .o(rb[1]));
  always @(rb[1]) $display("W t=%0t bits=%h", $time, $realtobits(rb[1]));
  initial begin #5 a = $bitstoreal(64'h8000000000000000); #4 $display("S t=%0t bits=%h", $time, $realtobits(rb[1])); $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=9 bits=0000000000000000"##, r##"A end"##],
    );
}

/// C2 (b) under `#0`: -0.0 then +0.0 in one step, then 1.0 then -0.0: `b` ends +0.0 (= vl; iv / s2v land the last -0.0 of the step).
/// Cell `B08_dca0_toggle_same_step`. Oracles: iv "W t=6 bits=8000000000000000 | S t=8 bits=8000000000000000 | A end"; s2v "W t=6 bits=8000000000000000 | S t=8 bits=8000000000000000 | A end"; vl "W t=0 bits=0000000000000000 | S t=8 bits=0000000000000000 | A end".
/// PRE "S t=8 bits=8000000000000000 | A end".
#[test]
fn b08_delay0_toggle_in_one_step() {
    pin(
        r##"module top;
  real a = 0.0; real b;
  assign #0 b = a;
  always @(b) $display("W t=%0t bits=%h", $time, $realtobits(b));
  initial begin #5 a = $bitstoreal(64'h8000000000000000); a = 0.0; #1 a = 1.0; a = $bitstoreal(64'h8000000000000000); #2 $display("S t=%0t bits=%h", $time, $realtobits(b)); $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=8 bits=0000000000000000"##, r##"A end"##],
    );
}

/// C3 (c) into an unpacked struct's real member (iv / s2v cannot run it; vl has no step c: hand, §10.3.3).
/// Cell `B18_dca_struct_member_lhs`. Oracles: iv "(no iverilog binary)"; s2v "(no sv2v->iverilog binary)"; vl "S t=10 bits=8000000000000000 | A end".
/// PRE "S t=10 bits=8000000000000000 | A end".
#[test]
fn b18_struct_member_destination() {
    pin(
        r##"module top;
  typedef struct { real x; int n; } st_t;
  st_t s;
  real a = 0.0;
  assign #2 s.x = a;
  initial begin #5 a = 1.0; #1 a = $bitstoreal(64'h8000000000000000); #4 $display("S t=%0t bits=%h", $time, $realtobits(s.x)); $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=10 bits=0000000000000000"##, r##"A end"##],
    );
}

/// C1: 0.0 then 0.4 inside the delay round to the same `int` 0: the t=13 landing stands (a raw-rhs (b) rescheduled it to t=14).
/// Cell `T05_dca_int_lhs_real_cancel`. Oracles: iv "W t=3 b=1 | S t=8 b=1 | W t=13 b=0 | A end b=0"; s2v "W t=3 b=1 | S t=8 b=1 | W t=13 b=0 | A end b=0"; vl "W t=0 b=0 | W t=3 b=1 | S t=8 b=1 | W t=13 b=0 | A end b=0".
/// PRE "W t=3 b=1 | S t=8 b=1 | W t=14 b=0 | A end b=0".
#[test]
fn t05_real_rhs_into_int_cancel() {
    pin(
        r##"module top;
  real r = 1.0; int b;
  assign #3 b = r;
  always @(b) $display("W t=%0t b=%0d", $time, b);
  initial begin #5 r = 2.0; #1 r = 1.2; #2 $display("S t=%0t b=%0d", $time, b); #2 r = 0.0; #1 r = 0.4; #5 $display("A end b=%0d", b); $finish; end
endmodule
"##,
        &[
            r##"W t=3 b=1"##,
            r##"S t=8 b=1"##,
            r##"W t=13 b=0"##,
            r##"A end b=0"##,
        ],
    );
}

/// C1: a real rhs into `logic [3:0]` under `#(1,4)`: 0 -> 2 rises at t=21.
/// Cell `T04_dca_lv4_lhs_real_risefall`. Oracles: iv "W t=4 b=0 | W t=6 b=1 | W t=14 b=0 | W t=21 b=2 | W t=34 b=0 | A end"; s2v "(no sv2v->iverilog binary)"; vl "W t=0 b=0 | W t=6 b=1 | W t=14 b=0 | W t=21 b=2 | W t=34 b=0 | A end".
/// PRE "W t=4 b=0 | W t=6 b=1 | W t=14 b=0 | W t=24 b=2 | W t=34 b=0 | A end".
#[test]
fn t04_real_rhs_into_logic4_rise_fall() {
    pin(
        r##"module top;
  real r = 0.0; logic [3:0] b;
  assign #(1,4) b = r;
  always @(b) $display("W t=%0t b=%0d", $time, b);
  initial begin #5 r = 1.0; #5 r = 0.3; #10 r = 2.0; #10 r = -0.4; #10 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=4 b=0"##,
            r##"W t=6 b=1"##,
            r##"W t=14 b=0"##,
            r##"W t=21 b=2"##,
            r##"W t=34 b=0"##,
            r##"A end"##,
        ],
    );
}

/// C3 (c) on a real-array word whose first drive is -0.0 under `#(2,5)`.
/// Cell `S2k_elem_risefall_from_zero`. Oracles: iv "S t=10 r0=0000000000000000 | A end"; s2v "(no sv2v->iverilog binary)"; vl "(no verilator binary) | %Error: Internal Error: t.sv:4:23: ../V3Number.cpp:2640: Number operation called with non-double argument: '32'h0"".
/// PRE "S t=10 r0=8000000000000000 | A end".
#[test]
fn s2k_real_word_first_drive_negzero() {
    pin(
        r##"module top;
  real a; real b = 0.5; real ra [0:1];
  assign #(2,5) ra[0] = a;
  assign #(2,5) ra[1] = b;
  initial begin #1 a = $bitstoreal(64'h8000000000000000); #9 $display("S t=%0t r0=%h", $time, $realtobits(ra[0])); #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=10 r0=0000000000000000"##, r##"A end"##],
    );
}

/// C2 (b): r goes to -0.0 inside one step; c's `#0` copy keeps its +0.0.
/// Cell `S1a_t5_same_seq`. Oracles: iv "S t=6 c=0000000000000000 r=8000000000000000 | A end"; s2v "S t=6 c=0000000000000000 r=8000000000000000 | A end"; vl "S t=6 c=3ff0000000000000 r=3ff0000000000000 | A end".
/// PRE "S t=6 c=8000000000000000 r=8000000000000000 | A end".
#[test]
fn s1a_t5_negzero_product_after_landing() {
    pin(
        r##"module top;
  real a = 0.0; real a1, a2, r, c;
  assign #0 a1 = a;
  assign #0 a2 = a1;
  assign r = (a - a1) * (a2 > 0.5 ? -1.0 : 1.0);
  assign #0 c = r;
  initial begin #5 a = 1.0; #1 $display("S t=%0t c=%h r=%h", $time, $realtobits(c), $realtobits(r)); #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=6 c=0000000000000000 r=8000000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C1 through a net-declaration delay `wire [7:0] #(2,5) w = a;`: 0 -> 3 rises at t=22.
/// Cell `F2e3_decl_assign_delay_real_rhs`. Oracles: iv "W t=5 w=1 | W t=15 w=0 | W t=22 w=3 | A end"; s2v "(no sv2v->iverilog binary)"; vl "W t=0 w=0 | W t=2 w=1 | W t=15 w=0 | W t=22 w=3 | A end".
/// PRE "W t=5 w=1 | W t=15 w=0 | W t=25 w=3 | A end".
#[test]
fn f2e3_net_declaration_delay_real_rhs() {
    pin(
        r##"module top;
  real a = 1.0;
  wire [7:0] #(2,5) w = a;
  always @(w) $display("W t=%0t w=%0d", $time, w);
  initial begin #10 a = 0.4; #10 a = 3.0; #10 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=5 w=1"##,
            r##"W t=15 w=0"##,
            r##"W t=22 w=3"##,
            r##"A end"##,
        ],
    );
}

/// C3 (c) on the runtime lane `#(dv)`.
/// Cell `S2e_runtime_delay_cancel_negzero`. Oracles: iv "S t=11 b=0000000000000000 | W t=15 b=4000000000000000 | A end"; s2v "S t=11 b=0000000000000000 | W t=15 b=4000000000000000 | A end"; vl "W t=0 b=0000000000000000 | S t=11 b=8000000000000000 | W t=15 b=4000000000000000 | A end".
/// PRE "S t=11 b=8000000000000000 | W t=15 b=4000000000000000 | A end".
#[test]
fn s2e_runtime_delay_cancel_to_negzero() {
    pin(
        r##"module top;
  real a = 0.0; int dv = 2; real b;
  assign #(dv) b = a;
  always @(b) $display("W t=%0t b=%h", $time, $realtobits(b));
  initial begin #5 a = 1.0; #1 a = $bitstoreal(64'h8000000000000000); #5 $display("S t=%0t b=%h", $time, $realtobits(b)); #2 a = 2.0; #4 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=11 b=0000000000000000"##,
            r##"W t=15 b=4000000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C3 (c) before any landing: a whole real's first drive -0.0 equals its +0.0.
/// Cell `S2b_whole_d2_negzero_first`. Oracles: iv "S t=3 b=0000000000000000 | A end"; s2v "S t=3 b=0000000000000000 | A end"; vl "S t=3 b=8000000000000000 | A end".
/// PRE "S t=3 b=8000000000000000 | A end".
#[test]
fn s2b_whole_first_drive_negzero() {
    pin(
        r##"module top;
  real a; real b;
  assign #2 b = a;
  initial begin a = $bitstoreal(64'h8000000000000000); #3 $display("S t=%0t b=%h", $time, $realtobits(b)); #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=3 b=0000000000000000"##, r##"A end"##],
    );
}

/// C1: a real rhs into `logic [7:0]` under `#(2,5)`.
/// Cell `S2c_int_lhs_real_rhs_risefall`. Oracles: iv "W t=5 b=1 | W t=15 b=0 | W t=22 b=3 | W t=35 b=0 | W t=42 b=255 | A end"; s2v "(no sv2v->iverilog binary)"; vl "W t=0 b=0 | W t=2 b=1 | W t=15 b=0 | W t=22 b=3 | W t=35 b=0 | W t=42 b=255 | A end".
/// PRE "W t=5 b=1 | W t=15 b=0 | W t=25 b=3 | W t=35 b=0 | W t=42 b=255 | A end".
#[test]
fn s2c_real_rhs_into_logic8_rise_fall() {
    pin(
        r##"module top;
  real a = 1.0; logic [7:0] b;
  assign #(2,5) b = a;
  always @(b) $display("W t=%0t b=%0d", $time, b);
  initial begin #10 a = 0.4; #10 a = 3.0; #10 a = 0.0; #10 a = -0.6; #10 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=5 b=1"##,
            r##"W t=15 b=0"##,
            r##"W t=22 b=3"##,
            r##"W t=35 b=0"##,
            r##"W t=42 b=255"##,
            r##"A end"##,
        ],
    );
}

/// C3 (c) before any landing on a constant real-array word.
/// Cell `S2b_elem_d2_negzero_first`. Oracles: iv "S t=3 r0=0000000000000000 | A end"; s2v "S t=3 r0=0000000000000000 | A end"; vl "S t=3 r0=8000000000000000 | A end".
/// PRE "S t=3 r0=8000000000000000 | A end".
#[test]
fn s2b_word_first_drive_negzero() {
    pin(
        r##"module top;
  real a; real b = 0.5; real ra [0:1];
  assign #2 ra[0] = a;
  assign #2 ra[1] = b;
  initial begin a = $bitstoreal(64'h8000000000000000); #3 $display("S t=%0t r0=%h", $time, $realtobits(ra[0])); #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=3 r0=0000000000000000"##, r##"A end"##],
    );
}

/// C3 (c) (mutant M21): a constant -0.0 under `#1` into a whole real.
/// Cell `F2a_whole_const_negzero_d1`. Oracles: iv "S t=2 r0=0000000000000000 | A end"; s2v "S t=2 r0=0000000000000000 | A end"; vl "S t=2 r0=8000000000000000 | A end".
/// PRE "S t=2 r0=8000000000000000 | A end".
#[test]
fn f2a_whole_constant_negzero() {
    pin(
        r##"module top;
  real r0;
  assign #1 r0 = $bitstoreal(64'h8000000000000000);
  initial begin #2 $display("S t=%0t r0=%h", $time, $realtobits(r0)); #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=2 r0=0000000000000000"##, r##"A end"##],
    );
}

/// C3 (c) word arm (mutant N4): a constant -0.0 under `#1` into a constant word of a real array.
/// Cell `F2a_elem_const_negzero_d1`. Oracles: iv "S t=2 r0=0000000000000000 | A end"; s2v "S t=2 r0=0000000000000000 | A end"; vl "S t=2 r0=8000000000000000 | A end".
/// PRE "S t=2 r0=8000000000000000 | A end".
#[test]
fn f2a_word_constant_negzero() {
    pin(
        r##"module top;
  real ra [0:1];
  assign #1 ra[0] = $bitstoreal(64'h8000000000000000);
  assign #1 ra[1] = 0.5;
  initial begin #2 $display("S t=%0t r0=%h", $time, $realtobits(ra[0])); #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=2 r0=0000000000000000"##, r##"A end"##],
    );
}

/// C4 + C3 (mutant M28): 1.0 -> 0.0 then back to 1.0 inside the fall delay cancels; later falls and rises on the whole value.
/// Cell `S2f_risefall_real_cancel_back`. Oracles: iv "W t=2 b=3ff0000000000000 | W t=12 b=0000000000000000 | W t=15 b=3ff0000000000000 | W t=25 b=0000000000000000 | W t=38 b=3fe0000000000000 | A end"; s2v "(no sv2v->iverilog binary)"; vl "W t=0 b=0000000000000000 | W t=2 b=3ff0000000000000 | W t=28 b=0000000000000000 | W t=38 b=3fe0000000000000 | A end".
/// PRE "W t=5 b=3ff0000000000000 | W t=28 b=0000000000000000 | W t=41 b=3fe0000000000000 | A end".
#[test]
fn s2f_rise_fall_cancel_and_back() {
    pin(
        r##"module top;
  real a = 1.0; real b;
  assign #(2,5) b = a;
  always @(b) $display("W t=%0t b=%h", $time, $realtobits(b));
  initial begin #10 a = 0.0; #3 a = 1.0; #10 a = 0.0; #10 a = $bitstoreal(64'h8000000000000000); #3 a = 0.5; #10 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=2 b=3ff0000000000000"##,
            r##"W t=28 b=0000000000000000"##,
            r##"W t=38 b=3fe0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C3 (c) at time 0 into a struct member (iv / s2v cannot run it; vl has no step c: hand, §10.3.3).
/// Cell `S2g_struct_member_delayed`. Oracles: iv "(no iverilog binary)"; s2v "(no sv2v->iverilog binary)"; vl "S t=1 sr=8000000000000000 | A end".
/// PRE "S t=1 sr=8000000000000000 | A end".
#[test]
fn s2g_struct_member_delay0() {
    pin(
        r##"module top;
  typedef struct { real r; int i; } s_t;
  s_t s;
  real a = $bitstoreal(64'h8000000000000000);
  assign #0 s.r = a;
  initial begin #1 $display("S t=%0t sr=%h", $time, $realtobits(s.r)); #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=1 sr=0000000000000000"##, r##"A end"##],
    );
}

/// C3 (c) at time 0 (mutant M21): `assign #0` of a -0.0 initializer into a whole real.
/// Cell `S2a_whole_d0_negzero_init`. Oracles: iv "S t=1 r0=0000000000000000 r1=3fe0000000000000 | A end"; s2v "S t=1 r0=0000000000000000 r1=3fe0000000000000 | A end"; vl "S t=1 r0=8000000000000000 r1=3fe0000000000000 | A end".
/// PRE "S t=1 r0=8000000000000000 r1=3fe0000000000000 | A end".
#[test]
fn s2a_whole_delay0_negzero_initializer() {
    pin_staged(
        r##"module top;
  real a = $bitstoreal(64'h8000000000000000); real b = 0.5; real r0, r1;
  assign #0 r0 = a;
  assign #0 r1 = b;
  initial begin #1 $display("S t=%0t r0=%h r1=%h", $time, $realtobits(r0), $realtobits(r1)); #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=1 r0=0000000000000000 r1=3fe0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C3 (c) word arm at time 0 (mutant N4).
/// Cell `S2a_elem_d0_negzero_init`. Oracles: iv "S t=1 r0=0000000000000000 r1=3fe0000000000000 | A end"; s2v "S t=1 r0=0000000000000000 r1=3fe0000000000000 | A end"; vl "S t=1 r0=8000000000000000 r1=3fe0000000000000 | A end".
/// PRE "S t=1 r0=8000000000000000 r1=3fe0000000000000 | A end".
#[test]
fn s2a_word_delay0_negzero_initializer() {
    pin(
        r##"module top;
  real a = $bitstoreal(64'h8000000000000000); real b = 0.5; real ra [0:1];
  assign #0 ra[0] = a;
  assign #0 ra[1] = b;
  initial begin #1 $display("S t=%0t r0=%h r1=%h", $time, $realtobits(ra[0]), $realtobits(ra[1])); #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=1 r0=0000000000000000 r1=3fe0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C2 (b): -0.0 after a landed 1.0 is a change and lands at t=9; the later +0.0 is the same value, so `o` keeps -0.0.
/// Cell `S2h_port_out_delayed_negzero`. Oracles: iv "W t=6 o=3ff0000000000000 | W t=9 o=8000000000000000 | S t=14 o=8000000000000000 | A end"; s2v "W t=6 o=3ff0000000000000 | W t=9 o=8000000000000000 | S t=14 o=8000000000000000 | A end"; vl "W t=0 o=0000000000000000 | W t=6 o=3ff0000000000000 | W t=9 o=8000000000000000 | S t=14 o=8000000000000000 | A end".
/// PRE "W t=6 o=3ff0000000000000 | W t=9 o=8000000000000000 | S t=14 o=0000000000000000 | A end".
#[test]
fn s2h_port_output_negzero_lands() {
    pin(
        r##"module m(input real x, output real y);
  assign #1 y = x;
endmodule
module top;
  real a = 0.0; real o;
  m u(.x(a), .y(o));
  always @(o) $display("W t=%0t o=%h", $time, $realtobits(o));
  initial begin #5 a = 1.0; #3 a = $bitstoreal(64'h8000000000000000); #3 a = 0.0; #3 $display("S t=%0t o=%h", $time, $realtobits(o)); #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=6 o=3ff0000000000000"##,
            r##"W t=9 o=8000000000000000"##,
            r##"S t=14 o=8000000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C1: 0.3 then -0.2 inside the delay round to the same `int` 0: the t=27 landing stands.
/// Cell `S2d_int_lhs_real_rhs_inertial`. Oracles: iv "W t=5 b=1 | W t=27 b=0 | A end"; s2v "W t=5 b=1 | W t=27 b=0 | A end"; vl "W t=0 b=0 | W t=5 b=1 | W t=27 b=0 | A end".
/// PRE "W t=5 b=1 | W t=29 b=0 | A end".
#[test]
fn s2d_real_rhs_into_int_inertial() {
    pin(
        r##"module top;
  real a = 1.2; int b;
  assign #5 b = a;
  always @(b) $display("W t=%0t b=%0d", $time, b);
  initial begin #10 a = 2.0; #2 a = 1.4; #10 a = 0.3; #2 a = -0.2; #10 $display("A end"); $finish; end
endmodule
"##,
        &[r##"W t=5 b=1"##, r##"W t=27 b=0"##, r##"A end"##],
    );
}

/// C2 / C5 control: `c = r * 1.0` is no copy net; its `#0` decision keeps +0.0.
/// Cell `S1a_ctl_computed`. Oracles: iv "S t=1 c=0000000000000000 r=8000000000000000 | A end"; s2v "S t=1 c=0000000000000000 r=8000000000000000 | A end"; vl "S t=1 c=3ff0000000000000 r=0000000000000000 | A end".
/// PRE "S t=1 c=8000000000000000 r=8000000000000000 | A end".
#[test]
fn s1a_ctl_computed_copy() {
    pin(
        r##"module top;
  real a = 1.0; real a1, a2, r, c;
  assign #0 a1 = a;
  assign #0 a2 = a1;
  assign r = (a - a1) * (a2 > 0.5 ? -1.0 : 1.0);
  assign #0 c = r * 1.0;
  initial begin #1 $display("S t=%0t c=%h r=%h", $time, $realtobits(c), $realtobits(r)); #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=1 c=0000000000000000 r=8000000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C1: 2.0 -> 1.0 -> 2.6 into `logic [7:0]`: per-bit on the rounded values (vl's vector rule differs: recorded split, ROADMAP §2).
/// Cell `F2c_int_lhs_real_rhs_nz_nz`. Oracles: iv "W t=5 b=2 | W t=15 b=1 | W t=22 b=3 | A end"; s2v "(no sv2v->iverilog binary)"; vl "W t=0 b=0 | W t=2 b=2 | W t=12 b=1 | W t=22 b=3 | A end".
/// PRE "W t=5 b=2 | W t=15 b=1 | W t=25 b=3 | A end".
#[test]
fn f2c_real_rhs_into_logic8_nonzero_steps() {
    pin(
        r##"module top;
  real a = 2.0; logic [7:0] b;
  assign #(2,5) b = a;
  always @(b) $display("W t=%0t b=%0d", $time, b);
  initial begin #10 a = 1.0; #10 a = 2.6; #10 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=5 b=2"##,
            r##"W t=15 b=1"##,
            r##"W t=22 b=3"##,
            r##"A end"##,
        ],
    );
}

/// control (recorded split, ROADMAP §2): an integral first drive takes iverilog's per-bit atomic-at-max delay; verilator's vector rule lands at t=2.
/// Cell `T07_dca_int_lhs_real_samelhs`. Oracles: iv "W t=13 b=1 | A end b=1"; s2v "(no sv2v->iverilog binary)"; vl "W t=0 b=0 | W t=2 b=1 | A end b=1".
/// PRE "W t=13 b=1 | A end b=1".
#[test]
fn t07_first_drive_atomic_at_max_control() {
    pin(
        r##"module top;
  real r = 1.0; int b;
  assign #(2,6) b = r;
  always @(b) $display("W t=%0t b=%0d", $time, b);
  initial begin #5 r = 0.2; #2 r = 1.3; #10 $display("A end b=%0d", b); $finish; end
endmodule
"##,
        &[r##"W t=13 b=1"##, r##"A end b=1"##],
    );
}

/// C5 (mutants N3, N3n): the time-0 copy-net repair re-evaluated `c = r` (-0.0) over the (c)-kept +0.0 landing.
/// Cell `S1a_noinit_const`. Oracles: iv "S t=1 c=0000000000000000 r=8000000000000000 | A end"; s2v "S t=1 c=0000000000000000 r=8000000000000000 | A end"; vl "S t=1 c=8000000000000000 r=8000000000000000 | A end".
/// PRE "S t=1 c=8000000000000000 r=8000000000000000 | A end".
#[test]
fn s1a_t0_repair_does_not_rewrite_a_delayed_copy() {
    pin(
        r##"module top;
  real a1, a2, r, c;
  assign #0 a1 = 1.0;
  assign #0 a2 = a1;
  assign r = (1.0 - a1) * (a2 > 0.5 ? -1.0 : 1.0);
  assign #0 c = r;
  initial begin #1 $display("S t=%0t c=%h r=%h", $time, $realtobits(c), $realtobits(r)); #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=1 c=0000000000000000 r=8000000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C5 (mutants N3, N3n): as S1a_noinit_const with an initializer behind the chain.
/// Cell `S1a_t0_repair_after_b`. Oracles: iv "S t=1 c=0000000000000000 r=8000000000000000 | A end"; s2v "S t=1 c=0000000000000000 r=8000000000000000 | A end"; vl "S t=1 c=3ff0000000000000 r=0000000000000000 | A end".
/// PRE "S t=1 c=8000000000000000 r=8000000000000000 | A end".
#[test]
fn s1a_t0_repair_after_initializer() {
    pin_staged(
        r##"module top;
  real a = 1.0; real a1, a2, r, c;
  assign #0 a1 = a;
  assign #0 a2 = a1;
  assign r = (a - a1) * (a2 > 0.5 ? -1.0 : 1.0);
  assign #0 c = r;
  initial begin #1 $display("S t=%0t c=%h r=%h", $time, $realtobits(c), $realtobits(r)); #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=1 c=0000000000000000 r=8000000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C1: the real rhs rounds at the net width, the store keeps the low 4 bits: 1.0 -> 17.0 inside the delay is no change of b[3:0].
/// Cell `Q1a_sel_real_b`. Oracles: iv "W t=0 b=X | W t=4 b=0 | W t=9 b=1 | A end b=1"; s2v "W t=0 b=X | W t=4 b=0 | W t=9 b=1 | A end b=1"; vl "W t=0 b=0 | W t=9 b=1 | A end b=1".
/// PRE "W t=4 b=0 | W t=11 b=1 | A end b=1".
#[test]
fn q1a_real_rhs_into_part_select() {
    pin_after_t0(
        r##"module top;
  wire [7:0] b; real r = 0.0;
  assign b[7:4] = 4'h0;
  assign #4 b[3:0] = r;
  always @(b) $display("W t=%0t b=%0d", $time, b);
  initial begin #5 r = 1.0; #2 r = 17.0; #10 $display("A end b=%0d", b); $finish; end
endmodule
"##,
        &[r##"W t=4 b=0"##, r##"W t=9 b=1"##, r##"A end b=1"##],
    );
}

/// C1: 1.0 -> 16.0 is 0001 -> 0000 on the kept bits: a fall (t=11).
/// Cell `Q1b_sel_real_rf`. Oracles: iv "W t=0 b=X | W t=4 b=1 | W t=11 b=0 | A end b=0"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:4:13: Parse error: unexpected token ',' (Sym_comma)"; vl "W t=0 b=0 | W t=4 b=1 | W t=11 b=0 | A end b=0".
/// PRE "W t=4 b=1 | W t=14 b=0 | A end b=0".
#[test]
fn q1b_real_rhs_into_part_select_rise_fall() {
    pin_after_t0(
        r##"module top;
  wire [7:0] b; real r = 1.0;
  assign b[7:4] = 4'h0;
  assign #(4,1) b[3:0] = r;
  always @(b) $display("W t=%0t b=%0d", $time, b);
  initial begin #10 r = 16.0; #10 $display("A end b=%0d", b); $finish; end
endmodule
"##,
        &[r##"W t=4 b=1"##, r##"W t=11 b=0"##, r##"A end b=0"##],
    );
}

/// C1 / O2 (pre-existing): an 8-bit rhs into b[3:0]: 1 -> 17 keeps 0001.
/// Cell `Q1c_sel_int_wide_b`. Oracles: iv "W t=0 b=X | W t=4 b=0 | W t=9 b=1 | A end b=1"; s2v "W t=0 b=X | W t=4 b=0 | W t=9 b=1 | A end b=1"; vl "W t=0 b=0 | W t=9 b=1 | A end b=1".
/// PRE "W t=4 b=0 | W t=11 b=1 | A end b=1".
#[test]
fn q1c_wide_integral_rhs_into_part_select() {
    pin_after_t0(
        r##"module top;
  wire [7:0] b; logic [7:0] a = 0;
  assign b[7:4] = 4'h0;
  assign #4 b[3:0] = a;
  always @(b) $display("W t=%0t b=%0d", $time, b);
  initial begin #5 a = 1; #2 a = 17; #10 $display("A end b=%0d", b); $finish; end
endmodule
"##,
        &[r##"W t=4 b=0"##, r##"W t=9 b=1"##, r##"A end b=1"##],
    );
}

/// C1 / O2: 1 -> 16 is a fall on the kept bits.
/// Cell `Q1d_sel_int_wide_rf`. Oracles: iv "W t=0 b=X | W t=4 b=1 | W t=11 b=0 | A end b=0"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:4:13: Parse error: unexpected token ',' (Sym_comma)"; vl "W t=0 b=0 | W t=4 b=1 | W t=11 b=0 | A end b=0".
/// PRE "W t=4 b=1 | W t=14 b=0 | A end b=0".
#[test]
fn q1d_wide_integral_rhs_into_part_select_rise_fall() {
    pin_after_t0(
        r##"module top;
  wire [7:0] b; logic [7:0] a = 1;
  assign b[7:4] = 4'h0;
  assign #(4,1) b[3:0] = a;
  always @(b) $display("W t=%0t b=%0d", $time, b);
  initial begin #10 a = 16; #10 $display("A end b=%0d", b); $finish; end
endmodule
"##,
        &[r##"W t=4 b=1"##, r##"W t=11 b=0"##, r##"A end b=0"##],
    );
}

/// C1: -1.0 -> 15.0 on a signed net's select: both keep 1111.
/// Cell `Q1k_sel_real_signed_net_b`. Oracles: iv "W t=0 b=X | W t=4 b=0 | W t=9 b=15 | A end b=15"; s2v "W t=0 b=X | W t=4 b=0 | W t=9 b=15 | A end b=15"; vl "W t=0 b=0 | W t=9 b=15 | A end b=15".
/// PRE "W t=4 b=0 | W t=11 b=15 | A end b=15".
#[test]
fn q1k_real_rhs_into_signed_net_select() {
    pin_after_t0(
        r##"module top;
  wire signed [7:0] b; real r = 0.0;
  assign b[7:4] = 4'h0;
  assign #4 b[3:0] = r;
  always @(b) $display("W t=%0t b=%0d", $time, b);
  initial begin #5 r = -1.0; #2 r = 15.0; #10 $display("A end b=%0d", b); $finish; end
endmodule
"##,
        &[r##"W t=4 b=0"##, r##"W t=9 b=15"##, r##"A end b=15"##],
    );
}

/// C1 into a select of a `logic` variable (the t=0 x is the variable's own: = iv).
/// Cell `Q1l_sel_real_int_var_rf`. Oracles: iv "W t=0 b=X | W t=4 b=1 | W t=11 b=0 | A end b=0"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:3:13: Parse error: unexpected token ',' (Sym_comma)"; vl "W t=0 b=0 | W t=4 b=1 | W t=11 b=0 | A end b=0".
/// PRE "W t=0 b=X | W t=4 b=1 | W t=14 b=0 | A end b=0".
#[test]
fn q1l_real_rhs_into_variable_select() {
    pin(
        r##"module top;
  logic [7:0] b; real r = 1.0;
  assign #(4,1) b[3:0] = r;
  assign b[7:4] = 4'h0;
  always @(b) $display("W t=%0t b=%0d", $time, b);
  initial begin #10 r = 16.0; #10 $display("A end b=%0d", b); $finish; end
endmodule
"##,
        &[
            r##"W t=0 b=X"##,
            r##"W t=4 b=1"##,
            r##"W t=11 b=0"##,
            r##"A end b=0"##,
        ],
    );
}

/// C1 with a ternary real rhs.
/// Cell `Q1m_sel_real_ternary_rf`. Oracles: iv "W t=0 b=X | W t=4 b=1 | W t=11 b=0 | A end b=0"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:4:13: Parse error: unexpected token ',' (Sym_comma)"; vl "W t=0 b=0 | W t=4 b=1 | W t=11 b=0 | A end b=0".
/// PRE "W t=4 b=1 | W t=14 b=0 | A end b=0".
#[test]
fn q1m_real_ternary_rhs_into_select() {
    pin_after_t0(
        r##"module top;
  wire [7:0] b; real r = 1.0; logic s = 1;
  assign b[7:4] = 4'h0;
  assign #(4,1) b[3:0] = s ? r : 2.0;
  always @(b) $display("W t=%0t b=%0d", $time, b);
  initial begin #10 r = 16.0; #10 $display("A end b=%0d", b); $finish; end
endmodule
"##,
        &[r##"W t=4 b=1"##, r##"W t=11 b=0"##, r##"A end b=0"##],
    );
}

/// C1 / O2: 8'h11 -> 8'h01 keeps 0001 (no change), 8'h10 falls at t=21.
/// Cell `Q1s_sel_int_wide_rf_tf`. Oracles: iv "W t=0 b=X | W t=4 b=1 | W t=21 b=0 | A end b=0"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:4:13: Parse error: unexpected token ',' (Sym_comma)"; vl "W t=0 b=0 | W t=4 b=1 | W t=21 b=0 | A end b=0".
/// PRE "W t=4 b=1 | W t=24 b=0 | A end b=0".
#[test]
fn q1s_wide_integral_rhs_select_rise_fall() {
    pin_after_t0(
        r##"module top;
  wire [7:0] b; logic [7:0] a = 8'h11;
  assign b[7:4] = 4'h0;
  assign #(4,1) b[3:0] = a;
  always @(b) $display("W t=%0t b=%0d", $time, b);
  initial begin #10 a = 8'h01; #10 a = 8'h10; #10 $display("A end b=%0d", b); $finish; end
endmodule
"##,
        &[r##"W t=4 b=1"##, r##"W t=21 b=0"##, r##"A end b=0"##],
    );
}

/// C1 / O2 (pre-existing, three oracles agree): `wire [3:0] y; assign #4 y = a8;` 1 -> 17 is no change of y.
/// Cell `Q1e_whole_int_wide_b`. Oracles: iv "W t=4 y=0 | W t=9 y=1 | A end y=1"; s2v "W t=4 y=0 | W t=9 y=1 | A end y=1"; vl "W t=0 y=0 | W t=9 y=1 | A end y=1".
/// PRE "W t=4 y=0 | W t=11 y=1 | A end y=1".
#[test]
fn q1e_wide_integral_rhs_into_narrow_net() {
    pin_staged(
        r##"module top;
  wire [3:0] y; logic [7:0] a = 0;
  assign #4 y = a;
  always @(y) $display("W t=%0t y=%0d", $time, y);
  initial begin #5 a = 1; #2 a = 17; #10 $display("A end y=%0d", y); $finish; end
endmodule
"##,
        &[r##"W t=4 y=0"##, r##"W t=9 y=1"##, r##"A end y=1"##],
    );
}

/// C1 / O2: 1 -> 16 into 4 bits is a fall (t=11).
/// Cell `Q1f_whole_int_wide_rf`. Oracles: iv "W t=4 y=1 | W t=11 y=0 | A end y=0"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:3:13: Parse error: unexpected token ',' (Sym_comma)"; vl "W t=0 y=0 | W t=4 y=1 | W t=11 y=0 | A end y=0".
/// PRE "W t=4 y=1 | W t=14 y=0 | A end y=0".
#[test]
fn q1f_wide_integral_rhs_rise_fall() {
    pin(
        r##"module top;
  wire [3:0] y; logic [7:0] a = 1;
  assign #(4,1) y = a;
  always @(y) $display("W t=%0t y=%0d", $time, y);
  initial begin #10 a = 16; #10 $display("A end y=%0d", y); $finish; end
endmodule
"##,
        &[r##"W t=4 y=1"##, r##"W t=11 y=0"##, r##"A end y=0"##],
    );
}

/// C1: a real rhs into `{c, d}` rounds at the lvalue width: 1.0 -> 257.0 keeps 00000001.
/// Cell `Q1g_concat_real_b`. Oracles: iv "W t=4 cd=0 | W t=9 cd=1 | A end cd=1"; s2v "W t=4 cd=0 | W t=9 cd=1 | A end cd=1"; vl "W t=0 cd=0 | W t=9 cd=1 | A end cd=1".
/// PRE "W t=4 cd=0 | W t=11 cd=1 | A end cd=1".
#[test]
fn q1g_real_rhs_into_concatenation() {
    pin(
        r##"module top;
  wire [3:0] c, d; real r = 0.0;
  assign #4 {c, d} = r;
  always @(c or d) $display("W t=%0t cd=%0d", $time, {c, d});
  initial begin #5 r = 1.0; #2 r = 257.0; #10 $display("A end cd=%0d", {c, d}); $finish; end
endmodule
"##,
        &[r##"W t=4 cd=0"##, r##"W t=9 cd=1"##, r##"A end cd=1"##],
    );
}

/// C1: 1.0 -> 256.0 into 8 bits is a fall.
/// Cell `Q1h_concat_real_rf`. Oracles: iv "W t=4 cd=1 | W t=11 cd=0 | A end cd=0"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:3:13: Parse error: unexpected token ',' (Sym_comma)"; vl "W t=0 cd=0 | W t=4 cd=1 | W t=11 cd=0 | A end cd=0".
/// PRE "W t=4 cd=1 | W t=14 cd=0 | A end cd=0".
#[test]
fn q1h_real_rhs_into_concatenation_rise_fall() {
    pin(
        r##"module top;
  wire [3:0] c, d; real r = 1.0;
  assign #(4,1) {c, d} = r;
  always @(c or d) $display("W t=%0t cd=%0d", $time, {c, d});
  initial begin #10 r = 256.0; #10 $display("A end cd=%0d", {c, d}); $finish; end
endmodule
"##,
        &[r##"W t=4 cd=1"##, r##"W t=11 cd=0"##, r##"A end cd=0"##],
    );
}

/// C1: a real rhs into `wire [3:0]`.
/// Cell `Q1j_whole_int_real_ctl_b`. Oracles: iv "W t=4 y=0 | W t=9 y=1 | A end y=1"; s2v "W t=4 y=0 | W t=9 y=1 | A end y=1"; vl "W t=0 y=0 | W t=9 y=1 | A end y=1".
/// PRE "W t=4 y=0 | W t=11 y=1 | A end y=1".
#[test]
fn q1j_real_rhs_into_narrow_net() {
    pin(
        r##"module top;
  wire [3:0] y; real r = 0.0;
  assign #4 y = r;
  always @(y) $display("W t=%0t y=%0d", $time, y);
  initial begin #5 r = 1.0; #2 r = 17.0; #10 $display("A end y=%0d", y); $finish; end
endmodule
"##,
        &[r##"W t=4 y=0"##, r##"W t=9 y=1"##, r##"A end y=1"##],
    );
}

/// C1 / O2: `y = a + b` evaluated at 8 bits: 1 -> 17 keeps 0001.
/// Cell `Q1q_whole_int_wide_expr_b`. Oracles: iv "W t=4 y=0 | W t=9 y=1 | A end y=1"; s2v "W t=4 y=0 | W t=9 y=1 | A end y=1"; vl "W t=0 y=0 | W t=9 y=1 | A end y=1".
/// PRE "W t=4 y=0 | W t=11 y=1 | A end y=1".
#[test]
fn q1q_wide_expression_rhs() {
    pin(
        r##"module top;
  wire [3:0] y; logic [7:0] a = 0, b = 0;
  assign #4 y = a + b;
  always @(y) $display("W t=%0t y=%0d", $time, y);
  initial begin #5 a = 1; #2 b = 16; #10 $display("A end y=%0d", y); $finish; end
endmodule
"##,
        &[r##"W t=4 y=0"##, r##"W t=9 y=1"##, r##"A end y=1"##],
    );
}

/// C1 / O2: an x in a bit the store drops is no change.
/// Cell `Q1r_whole_int_wide_x_upper_b`. Oracles: iv "W t=4 y=0000 | W t=9 y=0001 | A end y=0001"; s2v "W t=4 y=0000 | W t=9 y=0001 | A end y=0001"; vl "W t=0 y=0000 | W t=9 y=0001 | A end y=0001".
/// PRE "W t=4 y=0000 | W t=11 y=0001 | A end y=0001".
#[test]
fn q1r_wide_rhs_x_in_dropped_bits() {
    pin(
        r##"module top;
  wire [3:0] y; logic [7:0] a = 0;
  assign #4 y = a;
  always @(y) $display("W t=%0t y=%b", $time, y);
  initial begin #5 a = 8'h01; #2 a = 8'bx0000001; #10 $display("A end y=%b", y); $finish; end
endmodule
"##,
        &[
            r##"W t=4 y=0000"##,
            r##"W t=9 y=0001"##,
            r##"A end y=0001"##,
        ],
    );
}

/// C1 / O2 through a submodule's narrow output port.
/// Cell `Q1t_port_narrow_out_b`. Oracles: iv "W t=4 y=0 | W t=9 y=1 | A end y=1"; s2v "W t=4 y=0 | W t=9 y=1 | A end y=1"; vl "W t=0 y=0 | W t=9 y=1 | A end y=1".
/// PRE "W t=4 y=0 | W t=11 y=1 | A end y=1".
#[test]
fn q1t_wide_rhs_through_narrow_output_port() {
    pin(
        r##"module sub(input [7:0] i, output [3:0] o); assign #4 o = i; endmodule
module top;
  wire [3:0] y; logic [7:0] a = 0;
  sub u(.i(a), .o(y));
  always @(y) $display("W t=%0t y=%0d", $time, y);
  initial begin #5 a = 1; #2 a = 17; #10 $display("A end y=%0d", y); $finish; end
endmodule
"##,
        &[r##"W t=4 y=0"##, r##"W t=9 y=1"##, r##"A end y=1"##],
    );
}

/// C1 / O2 through a net-declaration assignment `wire [3:0] #4 y = a;`.
/// Cell `Q1u_decl_assign_narrow_b`. Oracles: iv "W t=4 y=0 | W t=9 y=1 | A end y=1"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:3:14: Parse error: unexpected token '#' (Sym_pound)"; vl "W t=0 y=0 | W t=9 y=1 | A end y=1".
/// PRE "W t=4 y=0 | W t=11 y=1 | A end y=1".
#[test]
fn q1u_net_declaration_assign_narrow() {
    pin(
        r##"module top;
  logic [7:0] a = 0;
  wire [3:0] #4 y = a;
  always @(y) $display("W t=%0t y=%0d", $time, y);
  initial begin #5 a = 1; #2 a = 17; #10 $display("A end y=%0d", y); $finish; end
endmodule
"##,
        &[r##"W t=4 y=0"##, r##"W t=9 y=1"##, r##"A end y=1"##],
    );
}

/// C1 / O2: `assign #(4,1) v = y + 1;` (32-bit rhs, 4-bit v): 15 + 1 is 0 on the kept bits, a fall at t=11.
/// Cell `Q1v_counter_plus1_rf`. Oracles: iv "W t=4 v=1 | W t=11 v=0 | W t=24 v=4 | A end v=4"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:3:13: Parse error: unexpected token ',' (Sym_comma)"; vl "W t=0 v=0 | W t=4 v=1 | W t=11 v=0 | W t=24 v=4 | A end v=4".
/// PRE "W t=4 v=1 | W t=14 v=0 | W t=24 v=4 | A end v=4".
#[test]
fn q1v_int_literal_widens_rhs() {
    pin(
        r##"module top;
  wire [3:0] v; logic [3:0] y = 0;
  assign #(4,1) v = y + 1;
  always @(v) $display("W t=%0t v=%0d", $time, v);
  initial begin #10 y = 15; #10 y = 3; #10 $display("A end v=%0d", v); $finish; end
endmodule
"##,
        &[
            r##"W t=4 v=1"##,
            r##"W t=11 v=0"##,
            r##"W t=24 v=4"##,
            r##"A end v=4"##,
        ],
    );
}

/// control: a rhs narrower than the lvalue zero-extends (unchanged).
/// Cell `Q1o_whole_int_narrow_rhs_rf`. Oracles: iv "W t=4 y=1 | W t=11 y=0 | W t=24 y=2 | A end y=2"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:3:13: Parse error: unexpected token ',' (Sym_comma)"; vl "W t=0 y=0 | W t=4 y=1 | W t=11 y=0 | W t=24 y=2 | A end y=2".
/// PRE "W t=4 y=1 | W t=11 y=0 | W t=24 y=2 | A end y=2".
#[test]
fn q1o_narrow_rhs_zero_extends_control() {
    pin(
        r##"module top;
  wire [7:0] y; logic [3:0] a = 1;
  assign #(4,1) y = a;
  always @(y) $display("W t=%0t y=%0d", $time, y);
  initial begin #10 a = 0; #10 a = 2; #10 $display("A end y=%0d", y); $finish; end
endmodule
"##,
        &[
            r##"W t=4 y=1"##,
            r##"W t=11 y=0"##,
            r##"W t=24 y=2"##,
            r##"A end y=2"##,
        ],
    );
}

/// control (recorded split, ROADMAP §2): a signed narrow rhs sign-extends; -1 -> 1 rises / falls per bit (iverilog), vl's vector rule lands at t=24.
/// Cell `Q1p_whole_int_signed_ext_rf`. Oracles: iv "W t=4 y=1 | W t=14 y=255 | W t=21 y=1 | A end y=1"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:3:13: Parse error: unexpected token ',' (Sym_comma)"; vl "W t=0 y=0 | W t=4 y=1 | W t=14 y=255 | W t=24 y=1 | A end y=1".
/// PRE "W t=4 y=1 | W t=14 y=255 | W t=21 y=1 | A end y=1".
#[test]
fn q1p_narrow_signed_rhs_sign_extends_control() {
    pin(
        r##"module top;
  wire [7:0] y; logic signed [3:0] a = 1;
  assign #(4,1) y = a;
  always @(y) $display("W t=%0t y=%0d", $time, y);
  initial begin #10 a = -1; #10 a = 1; #10 $display("A end y=%0d", y); $finish; end
endmodule
"##,
        &[
            r##"W t=4 y=1"##,
            r##"W t=14 y=255"##,
            r##"W t=21 y=1"##,
            r##"A end y=1"##,
        ],
    );
}

/// C1 + C4: an `int` rhs into a real converts first; 3 -> 5 rises (t=11), 5 -> 0 falls (t=24), 0 -> -2 rises, -2 -> 6 rises (t=41).
/// Cell `Q2a_rdest_int_nz_nz_rf`. Oracles: iv "W t=6 b=3.000000 | W t=11 b=5.000000 | W t=21 b=0.000000 | W t=31 b=-2.000000 | W t=41 b=6.000000 | A end"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:3:13: Parse error: unexpected token ',' (Sym_comma)"; vl "W t=0 b=0.000000 | W t=6 b=3.000000 | W t=11 b=5.000000 | W t=24 b=0.000000 | W t=31 b=-2.000000 | W t=41 b=6.000000 | A end".
/// PRE "W t=6 b=3.000000 | W t=14 b=5.000000 | W t=24 b=0.000000 | W t=31 b=-2.000000 | W t=44 b=6.000000 | A end".
#[test]
fn q2a_int_rhs_into_real_rise_fall() {
    pin(
        r##"module top;
  int i = 0; real b;
  assign #(1,4) b = i;
  always @(b) $display("W t=%0t b=%f", $time, b);
  initial begin #5 i = 3; #5 i = 5; #10 i = 0; #10 i = -2; #10 i = 6; #10 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=6 b=3.000000"##,
            r##"W t=11 b=5.000000"##,
            r##"W t=24 b=0.000000"##,
            r##"W t=31 b=-2.000000"##,
            r##"W t=41 b=6.000000"##,
            r##"A end"##,
        ],
    );
}

/// C1: 3 -> 4'bx011 converts to the same 3.0: no change, 3.0 lands at t=9.
/// Cell `Q2b_rdest_x_bits_b`. Oracles: iv "W t=9 b=3.000000 | A end b=3.000000"; s2v "W t=9 b=3.000000 | A end b=3.000000"; vl "W t=0 b=0.000000 | W t=9 b=3.000000 | A end b=3.000000".
/// PRE "W t=11 b=3.000000 | A end b=3.000000".
#[test]
fn q2b_x_bits_read_zero_in_a_real() {
    pin(
        r##"module top;
  logic [3:0] i = 0; real b;
  assign #4 b = i;
  always @(b) $display("W t=%0t b=%f", $time, b);
  initial begin #5 i = 3; #2 i = 4'bx011; #10 $display("A end b=%f", b); $finish; end
endmodule
"##,
        &[r##"W t=9 b=3.000000"##, r##"A end b=3.000000"##],
    );
}

/// C1 + C4: 5 -> 'x converts to 0.0, a fall (t=14); the first drive 5 rises (t=1).
/// Cell `Q2c_rdest_to_x_rf`. Oracles: iv "W t=1 b=5.000000 | W t=11 b=0.000000 | A end b=0.000000"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:3:13: Parse error: unexpected token ',' (Sym_comma)"; vl "W t=0 b=0.000000 | W t=1 b=5.000000 | W t=14 b=0.000000 | A end b=0.000000".
/// PRE "W t=4 b=5.000000 | W t=11 b=0.000000 | A end b=0.000000".
#[test]
fn q2c_all_x_into_a_real_falls() {
    pin(
        r##"module top;
  logic [3:0] i = 5; real b;
  assign #(1,4) b = i;
  always @(b) $display("W t=%0t b=%f", $time, b);
  initial begin #10 i = 4'bxxxx; #10 $display("A end b=%f", b); $finish; end
endmodule
"##,
        &[
            r##"W t=1 b=5.000000"##,
            r##"W t=14 b=0.000000"##,
            r##"A end b=0.000000"##,
        ],
    );
}

/// O1 (no oracle, hand-IEEE): 5 -> 'z under `#(1,4,2)` converts to 0.0 and takes the fall delay (t=14); see the comment.
/// Cell `Q2d_rdest_to_z_rft`. Oracles: iv "W t=1 b=5.000000 | W t=11 b=0.000000 | A end b=0.000000"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:3:13: Parse error: unexpected token ',' (Sym_comma)"; vl "W t=0 b=0.000000 | W t=1 b=5.000000 | A end b=5.000000".
/// PRE "W t=4 b=5.000000 | W t=12 b=0.000000 | A end b=0.000000".
/// NO ORACLE (O1, recorded in ROADMAP §2). iverilog lands 0.0 at t=11 (it rises on every real transition:
/// disqualified for real rise / fall, D16 Q2a); verilator never lands the z (keeps 5.0: disqualified). PRE took the
/// turn-off delay (t=12, its per-bit `-> z` arm). Hand-IEEE: §10.3.3 (ieee1800-2017.txt:10343-10345) names "a
/// transition to z" of the right-hand side; a real holds no z, and the value the left-hand side takes is 0.0 (the
/// store converts z bits to 0, §6.12.2), so the transition decided on the stored value is nonzero -> zero: the
/// falling delay (t=14). The value-based reading is the rule every other real-destination cell follows.
#[test]
fn q2d_all_z_into_a_real_falls() {
    pin(
        r##"module top;
  logic [3:0] i = 5; real b;
  assign #(1,4,2) b = i;
  always @(b) $display("W t=%0t b=%f", $time, b);
  initial begin #10 i = 4'bzzzz; #10 $display("A end b=%f", b); $finish; end
endmodule
"##,
        &[
            r##"W t=1 b=5.000000"##,
            r##"W t=14 b=0.000000"##,
            r##"A end b=0.000000"##,
        ],
    );
}

/// C4: a first drive of 5 into a real rises (t=1).
/// Cell `Q2e_rdest_int_first_rf`. Oracles: iv "W t=1 b=5.000000 | A end b=5.000000"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:3:13: Parse error: unexpected token ',' (Sym_comma)"; vl "(no verilator binary) | %Error: Internal Error: t.sv:3:19: ../V3Number.cpp:2640: Number operation called with non-double argument: '32'h0"".
/// PRE "W t=4 b=5.000000 | A end b=5.000000".
#[test]
fn q2e_int_first_drive_into_a_real_rises() {
    pin(
        r##"module top;
  int i = 5; real b;
  assign #(1,4) b = i;
  always @(b) $display("W t=%0t b=%f", $time, b);
  initial begin #10 $display("A end b=%f", b); $finish; end
endmodule
"##,
        &[r##"W t=1 b=5.000000"##, r##"A end b=5.000000"##],
    );
}

/// C1 control: x bits read 0, `b` ends 0.0 with no event.
/// Cell `Q2i_rdest_x_whole_b`. Oracles: iv "A end b=0.000000"; s2v "A end b=0.000000"; vl "W t=0 b=0.000000 | A end b=0.000000".
/// PRE "A end b=0.000000".
#[test]
fn q2i_x_values_into_a_real() {
    pin(
        r##"module top;
  logic [3:0] i = 0; real b;
  assign #4 b = i;
  always @(b) $display("W t=%0t b=%f", $time, b);
  initial begin #5 i = 2; #2 i = 4'bx010; #1 i = 4'bxxxx; #10 $display("A end b=%f", b); $finish; end
endmodule
"##,
        &[r##"A end b=0.000000"##],
    );
}

/// C1: 2^53+1 -> 2^53 are one double: no change, the value lands at t=9.
/// Cell `Q2j_rdest_wide_int_b`. Oracles: iv "W t=9 b=9007199254740992.000000 | A end b=9007199254740992.000000"; s2v "W t=9 b=9007199254740992.000000 | A end b=9007199254740992.000000"; vl "W t=0 b=0.000000 | W t=9 b=9007199254740992.000000 | A end b=9007199254740992.000000".
/// PRE "W t=11 b=9007199254740992.000000 | A end b=9007199254740992.000000".
#[test]
fn q2j_two_ints_one_double() {
    pin(
        r##"module top;
  logic [63:0] i = 0; real b;
  assign #4 b = i;
  always @(b) $display("W t=%0t b=%f", $time, b);
  initial begin #5 i = 64'd9007199254740993; #2 i = 64'd9007199254740992; #10 $display("A end b=%f", b); $finish; end
endmodule
"##,
        &[
            r##"W t=9 b=9007199254740992.000000"##,
            r##"A end b=9007199254740992.000000"##,
        ],
    );
}

/// C4: 5 -> 0 falls (t=14); -0 is the integer 0, no change.
/// Cell `Q2k_rdest_int_neg_zero_rf`. Oracles: iv "W t=1 b=4014000000000000 | W t=11 b=0000000000000000 | A end"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:3:13: Parse error: unexpected token ',' (Sym_comma)"; vl "W t=0 b=0000000000000000 | W t=1 b=4014000000000000 | W t=14 b=0000000000000000 | A end".
/// PRE "W t=4 b=4014000000000000 | W t=14 b=0000000000000000 | A end".
#[test]
fn q2k_int_zero_into_a_real() {
    pin(
        r##"module top;
  int i = 5; real b;
  assign #(1,4) b = i;
  always @(b) $display("W t=%0t b=%h", $time, $realtobits(b));
  initial begin #10 i = 0; #10 i = -0; #10 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=1 b=4014000000000000"##,
            r##"W t=14 b=0000000000000000"##,
            r##"A end"##,
        ],
    );
}

/// control: a signed 8-bit -1 converts to -1.0.
/// Cell `Q2l_rdest_signed8_to_real_b`. Oracles: iv "W t=9 b=-1.000000 | A end b=-1.000000"; s2v "W t=9 b=-1.000000 | A end b=-1.000000"; vl "W t=0 b=0.000000 | W t=9 b=-1.000000 | A end b=-1.000000".
/// PRE "W t=9 b=-1.000000 | A end b=-1.000000".
#[test]
fn q2l_signed_rhs_into_a_real_control() {
    pin(
        r##"module top;
  logic signed [7:0] i = 0; real b;
  assign #4 b = i;
  always @(b) $display("W t=%0t b=%f", $time, b);
  initial begin #5 i = -1; #10 $display("A end b=%f", b); $finish; end
endmodule
"##,
        &[r##"W t=9 b=-1.000000"##, r##"A end b=-1.000000"##],
    );
}

/// C5 control: `assign #0 c = r;` with `real r = 0.5` lands 0.5 with no time-0 event.
/// Cell `Q4a_t0_real_copy0_init`. Oracles: iv "S t=1 c=0.500000 | A end"; s2v "S t=1 c=0.500000 | A end"; vl "W t=0 c=0.000000 | W t=0 c=0.500000 | S t=1 c=0.500000 | A end".
/// PRE "S t=1 c=0.500000 | A end".
#[test]
fn q4a_t0_real_delay0_copy_control() {
    pin(
        r##"module top;
  real r = 0.5; real c;
  assign #0 c = r;
  always @(c) $display("W t=%0t c=%f", $time, c);
  initial begin #1 $display("S t=1 c=%f", c); #1 $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=1 c=0.500000"##, r##"A end"##],
    );
}

/// C5 control: the integral twin.
/// Cell `Q4b_t0_int_copy0_init`. Oracles: iv "S t=1 w=0 | A end"; s2v "S t=1 w=0 | A end"; vl "L t=0 w=0 | S t=1 w=0 | A end".
/// PRE "S t=1 w=0 | A end".
#[test]
fn q4b_t0_int_delay0_copy_control() {
    pin(
        r##"module top;
  reg r = 0; wire w;
  assign #0 w = r;
  always @(w) $display("L t=%0t w=%b", $time, w);
  always @(negedge w) $display("N t=%0t", $time);
  initial begin #1 $display("S t=1 w=%b", w); #1 $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=1 w=0"##, r##"A end"##],
    );
}

/// C3 base: a real destination reads 0.0 before its first landing (its x drive stores +0.0).
/// Cell `Q4c_rdest_owes_x`. Oracles: iv "S t=1 c=0.000000 | W t=3 c=2.500000 | S t=4 c=2.500000 | A end"; s2v "S t=1 c=0.000000 | W t=3 c=2.500000 | S t=4 c=2.500000 | A end"; vl "W t=0 c=0.000000 | S t=1 c=0.000000 | W t=3 c=2.500000 | S t=4 c=2.500000 | A end".
/// PRE "S t=1 c=0.000000 | W t=3 c=2.500000 | S t=4 c=2.500000 | A end".
#[test]
fn q4c_real_destination_before_its_landing() {
    pin(
        r##"module top;
  real r = 2.5; real c;
  assign #3 c = r;
  always @(c) $display("W t=%0t c=%f", $time, c);
  initial begin #1 $display("S t=1 c=%f", c); #3 $display("S t=4 c=%f", c); #1 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=1 c=0.000000"##,
            r##"W t=3 c=2.500000"##,
            r##"S t=4 c=2.500000"##,
            r##"A end"##,
        ],
    );
}

/// C3 (c) at time 0: `real r = -0.0; assign #0 c = r;` leaves c +0.0.
/// Cell `Q4d2_t0_negzero_copy0`. Oracles: iv "S t=1 c=0000000000000000 | A end"; s2v "S t=1 c=0000000000000000 | A end"; vl "W t=0 c=0000000000000000 | S t=1 c=8000000000000000 | A end".
/// PRE "S t=1 c=8000000000000000 | A end".
#[test]
fn q4d2_t0_negzero_delay0_copy() {
    pin(
        r##"module top;
  real r = $bitstoreal(64'h8000000000000000); real c;
  assign #0 c = r;
  always @(c) $display("W t=%0t c=%h", $time, $realtobits(c));
  initial begin #1 $display("S t=1 c=%h", $realtobits(c)); #1 $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=1 c=0000000000000000"##, r##"A end"##],
    );
}

/// C5 control: an integral `#0` chain.
/// Cell `Q4f_t0_int_copy0_chain`. Oracles: iv "S t=1 b=0101 | A end"; s2v "S t=1 b=0101 | A end"; vl "W t=0 b=0000 | S t=1 b=0000 | P t=1 | W t=1 b=0101 | A end".
/// PRE "S t=1 b=0101 | A end".
#[test]
fn q4f_t0_int_delay0_chain_control() {
    pin(
        r##"module top;
  reg [3:0] s = 4'b0101; wire [3:0] a, b;
  assign #0 a = s;
  assign #0 b = a;
  always @(b) $display("W t=%0t b=%b", $time, b);
  always @(posedge b[0]) $display("P t=%0t", $time);
  initial begin #1 $display("S t=1 b=%b", b); #1 $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=1 b=0101"##, r##"A end"##],
    );
}

/// C5 control: a real `#0` chain, then -0.0 lands as a change from 0.5.
/// Cell `Q4g2_t0_real_copy0_chain_init`. Oracles: iv "S t=1 c=3fe0000000000000 | W t=2 c=8000000000000000 | S t=3 c=8000000000000000 a=8000000000000000 | A end"; s2v "S t=1 c=3fe0000000000000 | W t=2 c=8000000000000000 | S t=3 c=8000000000000000 a=8000000000000000 | A end"; vl "W t=0 c=0000000000000000 | S t=1 c=0000000000000000 | W t=1 c=3fe0000000000000 | S t=3 c=3fe0000000000000 a=8000000000000000 | W t=3 c=8000000000000000 | A end".
/// PRE "S t=1 c=3fe0000000000000 | W t=2 c=8000000000000000 | S t=3 c=8000000000000000 a=8000000000000000 | A end".
#[test]
fn q4g2_t0_real_delay0_chain_control() {
    pin(
        r##"module top;
  real r = 0.5; real a, c;
  assign #0 a = r;
  assign #0 c = a;
  always @(c) $display("W t=%0t c=%h", $time, $realtobits(c));
  initial begin #1 $display("S t=1 c=%h", $realtobits(c)); #1 r = $bitstoreal(64'h8000000000000000); #1 $display("S t=3 c=%h a=%h", $realtobits(c), $realtobits(a)); #1 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=1 c=3fe0000000000000"##,
            r##"W t=2 c=8000000000000000"##,
            r##"S t=3 c=8000000000000000 a=8000000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C5 control: a landed `#0` copy, then a computed -0.0 equal to it.
/// Cell `Q4j_t0_copy0_landed_then_negzero`. Oracles: iv "S t=2 c=0000000000000000 b=0000000000000000 | A end"; s2v "S t=2 c=0000000000000000 b=0000000000000000 | A end"; vl "S t=2 c=0000000000000000 b=0000000000000000 | A end".
/// PRE "S t=2 c=0000000000000000 b=0000000000000000 | A end".
#[test]
fn q4j_t0_delay0_landed_then_negzero_control() {
    pin(
        r##"module top;
  real a = 1.0; real b, c;
  assign b = a - 1.0;
  assign #0 c = b;
  initial begin #1 a = 1.0; #1 $display("S t=2 c=%h b=%h", $realtobits(c), $realtobits(b)); #1 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=2 c=0000000000000000 b=0000000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C5 control: S1a's integral twin.
/// Cell `Q4k_t0_repair_int_landed_chain_twin`. Oracles: iv "S t=1 c=1000 r=1000 | A end"; s2v "S t=1 c=1000 r=1000 | A end"; vl "S t=1 c=1000 r=1000 | A end".
/// PRE "S t=1 c=1000 r=1000 | A end".
#[test]
fn q4k_t0_repair_int_chain_control() {
    pin(
        r##"module top;
  logic [3:0] a1, a2, r, c;
  assign #0 a1 = 4'd1;
  assign #0 a2 = a1;
  assign r = (4'd1 - a1) ^ (a2 > 0 ? 4'd8 : 4'd0);
  assign #0 c = r;
  initial begin #1 $display("S t=1 c=%b r=%b", c, r); #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=1 c=1000 r=1000"##, r##"A end"##],
    );
}

/// C3 (c) word arm: `assign #1 ra[0] = a;` with `real a = -0.0`.
/// Cell `Q4m_word_negzero_init_d1`. Oracles: iv "S t=3 r0=0000000000000000 r1=3fe0000000000000 | A end"; s2v "S t=3 r0=0000000000000000 r1=3fe0000000000000 | A end"; vl "W t=0 r0=0000000000000000 | S t=3 r0=8000000000000000 r1=3fe0000000000000 | A end".
/// PRE "S t=3 r0=8000000000000000 r1=3fe0000000000000 | A end".
#[test]
fn q4m_word_negzero_initializer_delay1() {
    pin(
        r##"module top;
  real ra [0:1]; real a = $bitstoreal(64'h8000000000000000);
  assign #1 ra[0] = a;
  assign #1 ra[1] = 0.5;
  always @(ra[0]) $display("W t=%0t r0=%h", $time, $realtobits(ra[0]));
  initial begin #3 $display("S t=3 r0=%h r1=%h", $realtobits(ra[0]), $realtobits(ra[1])); #1 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=3 r0=0000000000000000 r1=3fe0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// C3 premise: the (c) word arm assumes this assign is the element's only driver; the E3018 gate guarantees it (iverilog and sv2v refuse too).
/// Cell `Q4l_word_two_drivers`. Oracles: iv "(no iverilog binary) | t.sv:4: error: Variable 'ra' cannot have multiple drivers. | 1 error(s) during elaboration."; s2v "(no sv2v->iverilog binary) | s2v.c.out:s2v.v:6: error: Variable 'ra' cannot have multiple drivers. | s2v.c.out:1 error(s) during elaboration."; vl "S t=3 r0=2.000000".
/// PRE "t.sv:4:3: error[VITA-E3018] E-ELAB-LVALUE-KIND: variable `top.ra` is driven by more than one continuous `assign`; v1 accepts that on a variable that is not `logic` only when each drives a different whole element of an unpacked array, every element once (IEEE 1800 §6.5: one continuous driver per element) [in top]".
#[test]
fn q4l_two_drivers_of_one_real_word_refused() {
    refused(
        r##"module top;
  real ra [0:1]; real a = 1.0, b = 2.0;
  assign #1 ra[0] = a;
  assign #1 ra[0] = b;
  assign ra[1] = 0.0;
  initial begin #3 $display("S t=3 r0=%f", ra[0]); $finish; end
endmodule
"##,
        r##"t.sv:4:3: "##,
        r##"error[VITA-E3018]"##,
    );
}

/// C4 on the runtime lane `#(dr, df)`: int 3 -> 5 rises (t=11).
/// Cell `Q5a_rdest_runtime_rf_int`. Oracles: iv "W t=6 b=3.000000 | W t=11 b=5.000000 | W t=21 b=0.000000 | A end"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:3:14: Parse error: unexpected token ',' (Sym_comma)"; vl "W t=0 b=0.000000 | W t=6 b=3.000000 | W t=11 b=5.000000 | W t=24 b=0.000000 | A end".
/// PRE "W t=6 b=3.000000 | W t=14 b=5.000000 | W t=24 b=0.000000 | A end".
#[test]
fn q5a_runtime_rise_fall_int_into_real() {
    pin(
        r##"module top;
  int dr = 1, df = 4; int i = 0; real b;
  assign #(dr, df) b = i;
  always @(b) $display("W t=%0t b=%f", $time, b);
  initial begin #5 i = 3; #5 i = 5; #10 i = 0; #10 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=6 b=3.000000"##,
            r##"W t=11 b=5.000000"##,
            r##"W t=24 b=0.000000"##,
            r##"A end"##,
        ],
    );
}

/// C4 with `#(1,4,2)`: the turn-off delay is never selected for a real.
/// Cell `Q5b_rdest_three_delays_real`. Oracles: iv "W t=6 b=3.000000 | W t=11 b=5.000000 | W t=21 b=0.000000 | A end"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:3:13: Parse error: unexpected token ',' (Sym_comma)"; vl "W t=0 b=0.000000 | W t=6 b=3.000000 | W t=11 b=5.000000 | W t=24 b=0.000000 | A end".
/// PRE "W t=6 b=3.000000 | W t=14 b=5.000000 | W t=24 b=0.000000 | A end".
#[test]
fn q5b_three_delays_on_a_real() {
    pin(
        r##"module top;
  real r = 0.0; real b;
  assign #(1,4,2) b = r;
  always @(b) $display("W t=%0t b=%f", $time, b);
  initial begin #5 r = 3.0; #5 r = 5.0; #10 r = 0.0; #10 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=6 b=3.000000"##,
            r##"W t=11 b=5.000000"##,
            r##"W t=24 b=0.000000"##,
            r##"A end"##,
        ],
    );
}

/// control (Q-NANDCA): a NaN from 0/0 through `#2` lands once and is not rescheduled when `z` is rewritten with the same value.
/// Cell `Q8a_nan_same_bits_delayed`. Oracles (aarch64): iv "W t=2 b=7ff8000000000000 | A end n=1"; s2v "W t=2 b=7ff8000000000000 | A end n=1"; vl "%Error-DIDNOTCONVERGE: Settle region did not converge after '--converge-limit' of 10000 tries | Aborting...".
/// PRE "W t=2 b=7ff8000000000000 | A end n=1".
///
/// The NaN's sign bit is not pinned: vita's real arithmetic hands back the CPU's default NaN, so
/// 0/0 is `7ff8000000000000` on aarch64 and `fff8000000000000` on x86-64 (CI's ubuntu-latest and
/// RHEL9 runners) — a pre-existing defect against CONTRIBUTING's Linux = macOS output guarantee,
/// queued as ROADMAP §5.2 Q-NANCANON. Until it lands, the landed value is pinned exactly on every
/// bit but bit 63 and must be a NaN; Q-NANCANON restores the exact line `W t=2 b=7ff8000000000000`.
#[test]
fn q8a_nan_from_zero_by_zero_control() {
    let src = r##"module top;
  real z = 0.0; real r; real b; int n = 0;
  assign r = z / z;
  assign #2 b = r;
  always @(b) begin n = n + 1; $display("W t=%0t b=%h", $time, $realtobits(b)); end
  initial begin #3 z = 0.0; #3 $display("A end n=%0d", n); $finish; end
endmodule
"##;
    for be in ["native", "vm", "interp"] {
        let got = run_on(src, be);
        assert_eq!(got.len(), 2, "backend {be}: {got:?}");
        let hex = got[0]
            .strip_prefix("W t=2 b=")
            .unwrap_or_else(|| panic!("backend {be}: {got:?}"));
        let bits =
            u64::from_str_radix(hex, 16).unwrap_or_else(|e| panic!("backend {be}: {hex}: {e}"));
        assert!(
            f64::from_bits(bits).is_nan(),
            "backend {be}: {hex} is not a NaN"
        );
        assert_eq!(
            bits & !(1u64 << 63),
            0x7ff8_0000_0000_0000,
            "backend {be}: {hex} differs from the quiet NaN 7ff8000000000000 outside the sign bit"
        );
        assert_eq!(got[1], "A end n=1", "backend {be}");
    }
}

// ── Round 2 ──────────────────────────────────────────────────────────────────────────────────────
// D3: the (c) word arm reads a constant word as the store does. D4: the store image carries a
// 2-state destination's x / z -> 0. D5: a destination a continuous assignment may not write keeps
// the right-hand-side decision (BASE's line). The release cells: Q-RELDELAY (§4.5.624) restores the
// last landed store image. "BASE" is vita at §4.5.626, the commit this round builds on.

/// D3 (F-A): `real ra[1:2]`, `ra[1]` lowers to the index `1 - 1`; the (c) word arm reads it as the store does (`offset_value`), so -0.0 under the delay equals the +0.0 the word holds: no event.
/// Cell `N06a_arr_lsb1_negzero`. Oracles: iv "S t=2 r1=0000000000000000 r2=3fe0000000000000 | A end"; s2v "S t=2 r1=0000000000000000 r2=3fe0000000000000 | A end"; vl "S t=2 r1=8000000000000000 r2=3fe0000000000000 | A end".
/// PRE "S t=2 r1=8000000000000000 r2=3fe0000000000000 | A end"; BASE (§4.5.626) "S t=2 r1=8000000000000000 r2=3fe0000000000000 | A end".
#[test]
fn d3_word_index_normalized_by_a_nonzero_low_bound() {
    pin_staged(
        r##"module top;
  real ra [1:2];
  assign #1 ra[1] = $bitstoreal(64'h8000000000000000);
  assign #1 ra[2] = 0.5;
  initial begin #2 $display("S t=%0t r1=%h r2=%h", $time, $realtobits(ra[1]), $realtobits(ra[2])); #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=2 r1=0000000000000000 r2=3fe0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// D3 (F-A): `ra[1-1]` is the same constant word; the (c) word arm folds it.
/// Cell `N06e_arr_expr_idx_negzero`. Oracles: iv "S t=2 r0=0000000000000000 r1=3fe0000000000000 | A end"; s2v "S t=2 r0=0000000000000000 r1=3fe0000000000000 | A end"; vl "S t=2 r0=8000000000000000 r1=3fe0000000000000 | A end".
/// PRE "S t=2 r0=8000000000000000 r1=3fe0000000000000 | A end"; BASE (§4.5.626) "S t=2 r0=8000000000000000 r1=3fe0000000000000 | A end".
#[test]
fn d3_word_index_written_as_an_expression() {
    pin(
        r##"module top;
  real ra [0:1];
  assign #1 ra[1-1] = $bitstoreal(64'h8000000000000000);
  assign #1 ra[2-1] = 0.5;
  initial begin #2 $display("S t=%0t r0=%h r1=%h", $time, $realtobits(ra[0]), $realtobits(ra[1])); #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=2 r0=0000000000000000 r1=3fe0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// D3 (F-A): the (c) decline on `ra[1]` of `real ra[1:2]` cancels a pending write.
/// Cell `N06f_arr_lsb1_cancel`. Oracles: iv "S t=6 bits=0000000000000000 | A end"; s2v "S t=6 bits=0000000000000000 | A end"; vl "W t=0 bits=0000000000000000 | S t=6 bits=8000000000000000 | A end".
/// PRE "S t=6 bits=8000000000000000 | A end"; BASE (§4.5.626) "S t=6 bits=8000000000000000 | A end".
#[test]
fn d3_word_index_normalized_cancel() {
    pin(
        r##"module top;
  real a = 0.0; real ra [1:2];
  assign #3 ra[1] = a;
  assign #3 ra[2] = 0.5;
  always @(ra[1]) $display("W t=%0t bits=%h", $time, $realtobits(ra[1]));
  initial begin #1 a = 1.0; #1 a = $bitstoreal(64'h8000000000000000); #4 $display("S t=%0t bits=%h", $time, $realtobits(ra[1])); $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=6 bits=0000000000000000"##, r##"A end"##],
    );
}

/// D3 (F-A): a `#0` copy of -0.0 into `ra[1]` of `real ra[1:2]` at time 0 leaves the word +0.0.
/// Cell `P19_arr_lsb1_t0_copy`. Oracles: iv "S t=1 r1=0000000000000000 r2=3fe0000000000000 | A end"; s2v "S t=1 r1=0000000000000000 r2=3fe0000000000000 | A end"; vl "S t=1 r1=8000000000000000 r2=3fe0000000000000 | A end".
/// PRE "S t=1 r1=8000000000000000 r2=3fe0000000000000 | A end"; BASE (§4.5.626) "S t=1 r1=8000000000000000 r2=3fe0000000000000 | A end".
#[test]
fn d3_word_index_normalized_delay0_copy_at_time_0() {
    pin(
        r##"module top;
  real a = $bitstoreal(64'h8000000000000000); real b = 0.5; real ra [1:2];
  assign #0 ra[1] = a;
  assign #0 ra[2] = b;
  initial begin #1 $display("S t=%0t r1=%h r2=%h", $time, $realtobits(ra[1]), $realtobits(ra[2])); #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"S t=1 r1=0000000000000000 r2=3fe0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// D4 (F-C): `int y; assign #(1,4) y = a;` with `a` 5 then 4'bx: the store keeps 0 (2-state), so the decision sees 5 -> 0, a fall (t=14), then 0 -> 5 a rise (t=21).
/// Cell `N03_2state_x_rf`. Oracles: iv "W t=4 y=5 | W t=14 y=0 | W t=21 y=5 | A end y=5"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:3:13: Parse error: unexpected token ',' (Sym_comma)"; vl "W t=0 y=0 | W t=1 y=5 | W t=14 y=0 | W t=21 y=5 | A end y=5".
/// PRE "W t=4 y=5 | W t=11 y=0 | W t=24 y=5 | A end y=5"; BASE (§4.5.626) "W t=4 y=5 | W t=11 y=0 | W t=24 y=5 | A end y=5".
#[test]
fn d4_two_state_int_takes_x_as_zero_rise_fall() {
    pin_staged(
        r##"module top;
  logic [3:0] a = 4'd5; int y;
  assign #(1,4) y = a;
  always @(y) $display("W t=%0t y=%0d", $time, y);
  initial begin #10 a = 4'bxxxx; #10 a = 4'd5; #10 $display("A end y=%0d", y); $finish; end
endmodule
"##,
        &[
            r##"W t=4 y=5"##,
            r##"W t=14 y=0"##,
            r##"W t=21 y=5"##,
            r##"A end y=5"##,
        ],
    );
}

/// D4 (F-C): the same on `bit [3:0] y`.
/// Cell `N03b_2state_bit_x_rf`. Oracles: iv "W t=4 y=5 | W t=14 y=0 | W t=21 y=5 | A end y=5"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:3:13: Parse error: unexpected token ',' (Sym_comma)"; vl "W t=0 y=0 | W t=1 y=5 | W t=14 y=0 | W t=21 y=5 | A end y=5".
/// PRE "W t=4 y=5 | W t=11 y=0 | W t=24 y=5 | A end y=5"; BASE (§4.5.626) "W t=4 y=5 | W t=11 y=0 | W t=24 y=5 | A end y=5".
#[test]
fn d4_two_state_bit_vector_takes_x_as_zero() {
    pin(
        r##"module top;
  logic [3:0] a = 4'd5; bit [3:0] y;
  assign #(1,4) y = a;
  always @(y) $display("W t=%0t y=%0d", $time, y);
  initial begin #10 a = 4'bxxxx; #10 a = 4'd5; #10 $display("A end y=%0d", y); $finish; end
endmodule
"##,
        &[
            r##"W t=4 y=5"##,
            r##"W t=14 y=0"##,
            r##"W t=21 y=5"##,
            r##"A end y=5"##,
        ],
    );
}

/// D4 (F-C): `bit [3:0] y; assign #4 y = a;` with `a` 5 -> 4'b010x -> 4'b0100 a unit later: 4'b010x stores 0100, so the second change is no change of `y` (step b) and 0100 lands at t=14.
/// Cell `N03c_2state_x_uniform_b`. Oracles: iv "W t=4 y=5 | W t=14 y=4 | A end y=4"; s2v "W t=4 y=5 | W t=15 y=4 | A end y=4"; vl "W t=0 y=0 | W t=4 y=5 | W t=14 y=4 | A end y=4".
/// PRE "W t=4 y=5 | W t=15 y=4 | A end y=4"; BASE (§4.5.626) "W t=4 y=5 | W t=15 y=4 | A end y=4".
#[test]
fn d4_two_state_x_bit_is_the_value_it_stores() {
    pin(
        r##"module top;
  logic [3:0] a = 4'd5; bit [3:0] y;
  assign #4 y = a;
  always @(y) $display("W t=%0t y=%0d", $time, y);
  initial begin #10 a = 4'b010x; #1 a = 4'b0100; #10 $display("A end y=%0d", y); $finish; end
endmodule
"##,
        &[r##"W t=4 y=5"##, r##"W t=14 y=4"##, r##"A end y=4"##],
    );
}

/// D4 (F-C): the same on `int y`.
/// Cell `N03d_2state_int_x_uniform_b`. Oracles: iv "W t=4 y=5 | W t=14 y=4 | A end y=4"; s2v "W t=4 y=5 | W t=15 y=4 | A end y=4"; vl "W t=0 y=0 | W t=4 y=5 | W t=14 y=4 | A end y=4".
/// PRE "W t=4 y=5 | W t=15 y=4 | A end y=4"; BASE (§4.5.626) "W t=4 y=5 | W t=15 y=4 | A end y=4".
#[test]
fn d4_two_state_int_x_bit_is_the_value_it_stores() {
    pin(
        r##"module top;
  logic [31:0] a = 32'd5; int y;
  assign #4 y = a;
  always @(y) $display("W t=%0t y=%0d", $time, y);
  initial begin #10 a = 32'b010x; #1 a = 32'd4; #10 $display("A end y=%0d", y); $finish; end
endmodule
"##,
        &[r##"W t=4 y=5"##, r##"W t=14 y=4"##, r##"A end y=4"##],
    );
}

/// D4 control: a 4-state `logic [3:0] y` keeps the x bit, so 4'b010x -> 4'b0100 is a change (t=15, iverilog and sv2v).
/// Cell `N03e_4state_ctl_uniform_b`. Oracles: iv "W t=4 y=0101 | W t=15 y=0100 | A end y=4"; s2v "W t=4 y=0101 | W t=15 y=0100 | A end y=4"; vl "W t=0 y=0000 | W t=4 y=0101 | W t=14 y=0100 | A end y=4".
/// PRE "W t=4 y=0101 | W t=15 y=0100 | A end y=4"; BASE (§4.5.626) "W t=4 y=0101 | W t=15 y=0100 | A end y=4".
#[test]
fn d4_four_state_control_keeps_the_x() {
    pin(
        r##"module top;
  logic [3:0] a = 4'd5; logic [3:0] y;
  assign #4 y = a;
  always @(y) $display("W t=%0t y=%b", $time, y);
  initial begin #10 a = 4'b010x; #1 a = 4'b0100; #10 $display("A end y=%0d", y); $finish; end
endmodule
"##,
        &[r##"W t=4 y=0101"##, r##"W t=15 y=0100"##, r##"A end y=4"##],
    );
}

/// D5 (F-D): a dynamic array element is not a legal continuous-assignment target (IEEE 1800-2017 §6.21, ieee1800-2017.txt:5436); vita runs it and decides on the right-hand side as before ENG-1c (BASE's line), not on the store image. Its refusal is queued.
/// Cell `P01b_dyn_real_elem_negzero`. Oracles: iv "(no iverilog binary)"; s2v "(no sv2v->iverilog binary) | s2v.err:sv2v: t.sv:2:25: Parse error: expected comma or end of declarations | s2v.err:  error, called at src/Language/SystemVerilog/Parser/ParseDecl.hs:693:26 in main:Language.SystemVerilog.Parser.ParseDecl"; vl "(no verilator binary) | %Error: t.sv:3:13: Dynamically-sized variable not allowed in continuous assignment (IEEE 1800-2023 6.21): 'dr' | %Error-CONTASSINIT: t.sv:4:17: Continuous assignment to variable with initial value: 'dr' | %Error: Exiting due to 2 error(s)".
/// PRE "S t=6 bits=8000000000000000 | A end"; BASE (§4.5.626) "S t=6 bits=8000000000000000 | A end".
#[test]
fn d5_dynamic_array_element_keeps_the_right_hand_side_decision() {
    pin_staged(
        r##"module top;
  real a = 0.0; real dr [];
  assign #3 dr[0] = a;
  initial begin dr = new[2]; #1 a = 1.0; #1 a = $bitstoreal(64'h8000000000000000); #4 $display("S t=%0t bits=%h", $time, $realtobits(dr[0])); $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=6 bits=8000000000000000"##, r##"A end"##],
    );
}

/// D5 (F-D): two delayed drivers of one dynamic element (no gate sees them): BASE's line; the (c) arm's "one driver per element" premise does not hold here, so it is not applied.
/// Cell `P27b_dyn_real_two_ca_premise`. Oracles: none (vita-only cell).
/// PRE "S t=21 d0=1.000000 | A end"; BASE (§4.5.626) "S t=21 d0=1.000000 | A end".
#[test]
fn d5_two_continuous_drivers_of_one_dynamic_element() {
    pin(
        r##"module top;
  real a = 0.0, b = 0.0; real dr [];
  assign #2 dr[0] = a;
  assign #3 dr[0] = b;
  initial begin dr = new[1]; #5 a = 1.0; #5 b = 2.0; #5 a = 5.0; #1 a = 1.0; #5 $display("S t=%0t d0=%f", $time, dr[0]); $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=21 d0=1.000000"##, r##"A end"##],
    );
}

/// Q-RELDELAY interplay: release restores the last landed store image; a later change only in bits the store drops is no change (iverilog, sv2v, verilator).
/// Cell `U01_rel_delayed_wide_dropbits`. Oracles: iv "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end"; s2v "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end"; vl "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end".
/// PRE "S t=6 y=0010 | S t=8 y=0010 | S t=11 y=0010 | S t=20 y=0001 | A end"; BASE (§4.5.626) "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end".
#[test]
fn rel_u01_dropped_bits_change_after_release() {
    pin_staged(
        r##"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 8'h11; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"##,
        &[
            r##"S t=6 y=0010"##,
            r##"S t=8 y=0001"##,
            r##"S t=11 y=0001"##,
            r##"S t=20 y=0001"##,
            r##"A end"##,
        ],
    );
}

/// Q-RELDELAY interplay: U01 on a `logic` variable.
/// Cell `U04_rel_delayed_logicvar_dropbits`. Oracles: iv "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end"; s2v "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end"; vl "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end".
/// PRE "S t=6 y=0010 | S t=8 y=0010 | S t=11 y=0010 | S t=20 y=0001 | A end"; BASE (§4.5.626) "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end".
#[test]
fn rel_u04_logic_variable() {
    pin(
        r##"module top;
  logic [7:0] a = 8'h01; logic [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 8'h11; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"##,
        &[
            r##"S t=6 y=0010"##,
            r##"S t=8 y=0001"##,
            r##"S t=11 y=0001"##,
            r##"S t=20 y=0001"##,
            r##"A end"##,
        ],
    );
}

/// Q-RELDELAY interplay: U01 under `#(3,2)`.
/// Cell `U05_rel_delayed_rf_dropbits`. Oracles: iv "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:3:13: Parse error: unexpected token ',' (Sym_comma)"; vl "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end".
/// PRE "S t=6 y=0010 | S t=8 y=0010 | S t=11 y=0010 | S t=20 y=0001 | A end"; BASE (§4.5.626) "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end".
#[test]
fn rel_u05_rise_fall() {
    pin(
        r##"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #(3,2) y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 8'h11; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"##,
        &[
            r##"S t=6 y=0010"##,
            r##"S t=8 y=0001"##,
            r##"S t=11 y=0001"##,
            r##"S t=20 y=0001"##,
            r##"A end"##,
        ],
    );
}

/// Q-RELDELAY interplay: a real right-hand side 1.0 -> 1.25 rounds to the same 4-bit image.
/// Cell `U06_rel_delayed_real_rhs_sameimage`. Oracles: iv "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end"; s2v "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end"; vl "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end".
/// PRE "S t=6 y=0010 | S t=8 y=0010 | S t=11 y=0010 | S t=20 y=0001 | A end"; BASE (§4.5.626) "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end".
#[test]
fn rel_u06_real_rhs_same_image() {
    pin(
        r##"module top;
  real r = 1.0; wire [3:0] y;
  assign #3 y = r;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 r = 1.25; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"##,
        &[
            r##"S t=6 y=0010"##,
            r##"S t=8 y=0001"##,
            r##"S t=11 y=0001"##,
            r##"S t=20 y=0001"##,
            r##"A end"##,
        ],
    );
}

/// Q-RELDELAY interplay: U01 under `#0`.
/// Cell `U10_rel_delayed_d0_dropbits`. Oracles: iv "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end"; s2v "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end"; vl "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end".
/// PRE "S t=6 y=0010 | S t=8 y=0010 | S t=11 y=0010 | S t=20 y=0001 | A end"; BASE (§4.5.626) "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end".
#[test]
fn rel_u10_delay0() {
    pin(
        r##"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #0 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 8'h11; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"##,
        &[
            r##"S t=6 y=0010"##,
            r##"S t=8 y=0001"##,
            r##"S t=11 y=0001"##,
            r##"S t=20 y=0001"##,
            r##"A end"##,
        ],
    );
}

/// Q-RELDELAY interplay: U01 as `wire [3:0] #3 y = a;`.
/// Cell `U11_rel_netdecl_delay_dropbits`. Oracles: iv "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end"; s2v "(no sv2v->iverilog binary) | s2v.err:t.sv:3:14: Parse error: unexpected token '#' (Sym_pound)"; vl "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end".
/// PRE "S t=6 y=0010 | S t=8 y=0010 | S t=11 y=0010 | S t=20 y=0001 | A end"; BASE (§4.5.626) "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | S t=20 y=0001 | A end".
#[test]
fn rel_u11_net_declaration_delay() {
    pin(
        r##"module top;
  logic [7:0] a = 8'h01;
  wire [3:0] #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y);
    #4 a = 8'h11; #5 $display("S t=20 y=%b", y); $display("A end"); $finish;
  end
endmodule
"##,
        &[
            r##"S t=6 y=0010"##,
            r##"S t=8 y=0001"##,
            r##"S t=11 y=0001"##,
            r##"S t=20 y=0001"##,
            r##"A end"##,
        ],
    );
}

/// Q-RELDELAY interplay: a delayed part-select driver of a two-driver net.
/// Cell `U12_rel_delayed_sel_dropbits`. Oracles: iv "S t=6 w=00000010 | S t=8 w=00000001 | S t=11 w=00000001 | S t=20 w=00000001 | A end"; s2v "S t=6 w=00000010 | S t=8 w=00000001 | S t=11 w=00000001 | S t=20 w=00000001 | A end"; vl "S t=6 w=00000010 | S t=8 w=00000001 | S t=11 w=00000001 | S t=20 w=00000001 | A end".
/// PRE "S t=6 w=00000010 | S t=8 w=00000010 | S t=11 w=00000010 | S t=20 w=00000001 | A end"; BASE (§4.5.626) "S t=6 w=00000010 | S t=8 w=00000001 | S t=11 w=00000001 | S t=20 w=00000001 | A end".
#[test]
fn rel_u12_part_select_driver() {
    pin(
        r##"module top;
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
"##,
        &[
            r##"S t=6 w=00000010"##,
            r##"S t=8 w=00000001"##,
            r##"S t=11 w=00000001"##,
            r##"S t=20 w=00000001"##,
            r##"A end"##,
        ],
    );
}

/// C1 (no force): `a` 01 then 11 inside `#4`: the image 0001 is unchanged, so 0001 lands at t=14 (PRE and BASE re-scheduled on the raw change: 0000 at t=15).
/// Cell `P01_pulse_raw_change_noforce`. Oracles: iv "S t=13 y=0000 | S t=15 y=0001 | S t=17 y=0001 | A end"; s2v "S t=13 y=0000 | S t=15 y=0001 | S t=17 y=0001 | A end"; vl "S t=13 y=0000 | S t=15 y=0001 | S t=17 y=0001 | A end".
/// PRE "S t=13 y=0000 | S t=15 y=0000 | S t=17 y=0001 | A end"; BASE (§4.5.626) "S t=13 y=0000 | S t=15 y=0000 | S t=17 y=0001 | A end".
#[test]
fn rel_p01_no_force_dropped_bits_inside_the_delay() {
    pin(
        r##"module top;
  logic [7:0] a = 8'h00; wire [3:0] y;
  assign #4 y = a;
  initial begin
    #10 a = 8'h01; #2 a = 8'h11; #1 $display("S t=13 y=%b", y);
    #2 $display("S t=15 y=%b", y); #2 $display("S t=17 y=%b", y); $display("A end"); $finish;
  end
endmodule
"##,
        &[
            r##"S t=13 y=0000"##,
            r##"S t=15 y=0001"##,
            r##"S t=17 y=0001"##,
            r##"A end"##,
        ],
    );
}

/// Q-RELDELAY interplay: a same-image change while forced, then release: 0001 at once and after.
/// Cell `P03_force_samimage_change_then_rel`. Oracles: iv "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | A end"; s2v "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | A end"; vl "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | A end".
/// PRE "S t=6 y=0010 | S t=8 y=0010 | S t=11 y=0001 | A end"; BASE (§4.5.626) "S t=6 y=0010 | S t=8 y=0001 | S t=11 y=0001 | A end".
#[test]
fn rel_p03_same_image_change_while_forced() {
    pin(
        r##"module top;
  logic [7:0] a = 8'h01; wire [3:0] y;
  assign #3 y = a;
  initial begin
    #5 force y = 4'h2; #1 a = 8'h11; $display("S t=6 y=%b", y);
    #1 release y; #1 $display("S t=8 y=%b", y);
    #3 $display("S t=11 y=%b", y); $display("A end"); $finish;
  end
endmodule
"##,
        &[
            r##"S t=6 y=0010"##,
            r##"S t=8 y=0001"##,
            r##"S t=11 y=0001"##,
            r##"A end"##,
        ],
    );
}

// ── Round 3 ──────────────────────────────────────────────────────────────────────────────────────
// The decision runs once per right-hand-side change (a right-hand side equal to the last decided one
// returns after one compare) and the 2-state x / z -> 0 of the image runs a word at a time, masked to
// each 2-state chunk; these pin what that mask and that key must keep. Cells E/r3/pins/p1-p3.

/// Round 3: a 2-state chunk's x / z -> 0 is masked to its own bits: the 4-state chunk `p` beside the
/// `bit` chunk `q` in one word keeps its z and x (unmasked, z -> 0 cleared p to 1111; clearing the
/// value bits of every unknown made z an x).
/// Cell `p1_concat_4state_beside_2state`. Oracles: iv "Q t=4 q=0101 | P t=4 p=0101 | P t=14 p=zzzz | P t=24 p=xxxx | A end p=xxxx q=0101" (before the `always @(q)` line was dropped; same-time order is not pinned); vl (2-state: no z / x) "P t=0 p=0000 | P t=4 p=0101 | A end p=0101 q=0101".
/// PRE "P t=4 p=0101 | P t=14 p=zzzz | P t=24 p=xxxx | A end p=xxxx q=0101"; BASE (§4.5.626) the same.
#[test]
fn r3_two_state_mask_keeps_a_four_state_neighbour() {
    pin(
        r##"module top;
  logic [7:0] a = 8'h55; logic [3:0] p; bit [3:0] q;
  assign #4 {p, q} = a;
  always @(p) $display("P t=%0t p=%b", $time, p);
  initial begin #10 a = 8'bzzzz_0101; #10 a = 8'bxxxx_0101; #10 $display("A end p=%b q=%b", p, q); $finish; end
endmodule
"##,
        &[
            r##"P t=4 p=0101"##,
            r##"P t=14 p=zzzz"##,
            r##"P t=24 p=xxxx"##,
            r##"A end p=xxxx q=0101"##,
        ],
    );
}

/// Round 3: bit 63 of a 64-bit 2-state destination is cleared too: `longint y; assign #4 y = a;` with `a`
/// 5 -> {1'bx, 63'd4} -> 4 a unit later stores 4 then 4, so the second change is no change and 4 lands at
/// t=14 (the 64-bit twin of `d4_two_state_int_x_bit_is_the_value_it_stores`).
/// Cell `p2_longint_x_bit63`. Oracles: iv "W t=4 y=5 | W t=14 y=4 | A end y=4"; vl "W t=0 y=0 | W t=4 y=5 | W t=14 y=4 | A end y=4".
/// PRE "W t=4 y=5 | W t=15 y=4 | A end y=4"; BASE (§4.5.626) the same.
#[test]
fn r3_two_state_x_at_bit_63_is_the_value_it_stores() {
    pin(
        r##"module top;
  logic [63:0] a = 64'd5; longint y;
  assign #4 y = a;
  always @(y) $display("W t=%0t y=%0d", $time, y);
  initial begin #10 a = {1'bx, 63'd4}; #1 a = 64'd4; #10 $display("A end y=%0d", y); $finish; end
endmodule
"##,
        &[r##"W t=4 y=5"##, r##"W t=14 y=4"##, r##"A end y=4"##],
    );
}

/// Round 3: the key of "the right-hand side last decided" is the whole value: 0 -> 4'bxxxx keeps the
/// value words (all 0) and changes only the unknown plane, and is a change.
/// Cell `p3_zero_to_all_x`. Oracles: iv "W t=4 y=0000 | W t=14 y=xxxx | W t=24 y=0000 | A end y=0000"; vl (2-state) "W t=0 y=0000 | A end y=0000".
/// PRE "W t=4 y=0000 | W t=14 y=xxxx | W t=24 y=0000 | A end y=0000"; BASE (§4.5.626) the same.
#[test]
fn r3_zero_to_all_x_is_a_change() {
    pin(
        r##"module top;
  logic [3:0] a = 4'd0; logic [3:0] y;
  assign #4 y = a;
  always @(y) $display("W t=%0t y=%b", $time, y);
  initial begin #10 a = 4'bxxxx; #10 a = 4'b0000; #10 $display("A end y=%b", y); $finish; end
endmodule
"##,
        &[
            r##"W t=4 y=0000"##,
            r##"W t=14 y=xxxx"##,
            r##"W t=24 y=0000"##,
            r##"A end y=0000"##,
        ],
    );
}
