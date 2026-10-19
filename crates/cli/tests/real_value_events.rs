//! ENG-1a: real-valued events, pinned on native, vm and interp.
//!
//! E1-a. A store into a real-valued net whose old and new bits are `+0.0` and `-0.0` (the one
//! pair of different bit patterns IEEE 754 calls equal) is a VALUE change and not an EVENT:
//! the bits are stored, continuous assigns and live forces reading the net re-evaluate, the
//! dump and `$monitor` record it and a `wait (expr)` is asked again, but no `@`, `@*`,
//! `always_comb` / `always_latch` or static deferred assertion is woken by it — at time 0
//! included, in a heap element (`real d[]`) too. IEEE 1800-2017 §9.4.2 ("any change in the
//! value of the expression") with footnote 12 (a real is an IEEE 754 double), §10.6.2 (a force
//! RHS is treated as a continuous assignment), §9.4.3 (`wait` blocks "until that condition
//! becomes true").
//!
//! Oracles: Icarus Verilog 13.0 `-g2012` (iv), sv2v 0.0.13 -> iverilog (s2v), Verilator 5.052
//! `--binary` (vl). Each cell's comment names the lines that decide it. vl runs every header
//! `always` once at time 0 (a "t0 artifact" its lines carry and these pins drop) and re-runs
//! `always_comb` / `always @*` on a signed-zero store (a recorded split: iv + the LRM decide);
//! iverilog re-checks a `wait` only on an operand event (a recorded split: vl + §9.4.3 decide).
//! A `-0.0` renders with its sign (`-0` / `-0.000000`, §4.5.619), as iv, s2v and vl print it.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn scratch(tag: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_rve_{tag}_{}_{n}", std::process::id()));
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

/// Every backend prints exactly `want`.
fn pin(src: &str, want: &[&str]) {
    for be in ["native", "vm", "interp"] {
        assert_eq!(run_on(src, be), want, "backend {be}");
    }
}

/// Control. iv = s2v = PRE (a store of the same NaN bits changes nothing); vl DIDNOTCONVERGE (C++ `!=` on NaN, disqualified).
/// Cell `R01_nan_same`.
#[test]
fn r01_same_nan_bits_is_no_change() {
    pin(
        "module top;\n\
  real r = 0.0; int n = 0;\n\
  always @(r) begin n++; $display(\"W t=%0t bits=%h n=%0d\", $time, $realtobits(r), n); end\n\
  initial begin #2 r = $bitstoreal(64'h7ff8000000000000); #3 r = $bitstoreal(64'h7ff8000000000000); #3 r = 1.0; #2 $display(\"A end n=%0d\", n); $finish; end\n\
endmodule\n\
",
        &[
            "W t=2 bits=7ff8000000000000 n=1",
            "W t=8 bits=3ff0000000000000 n=2",
            "A end n=2",
        ],
    );
}

/// Control (mutant M5). iv = s2v = PRE: a NaN payload change is an event (a NaN is never `==`); vl DIDNOTCONVERGE.
/// Cell `R02_nan_payload`.
#[test]
fn r02_nan_payload_change_is_an_event() {
    pin(
        "module top;\n\
  real r = 0.0; int n = 0;\n\
  always @(r) begin n++; $display(\"W t=%0t bits=%h n=%0d\", $time, $realtobits(r), n); end\n\
  initial begin #2 r = $bitstoreal(64'h7ff8000000000000); #3 r = $bitstoreal(64'h7ff8000000000001); #3 r = 1.0; #2 $display(\"A end n=%0d\", n); $finish; end\n\
endmodule\n\
",
        &[
            "W t=2 bits=7ff8000000000000 n=1",
            "W t=5 bits=7ff8000000000001 n=2",
            "W t=8 bits=3ff0000000000000 n=3",
            "A end n=3",
        ],
    );
}

/// iv = s2v "W t=8 n=1"; vl "W t=0 (its time-0 run of always) | W t=8". PRE "W t=2 | W t=5 | W t=8".
/// §9.4.2 (ieee1800-2017.txt:9511) + footnote 12 (:4409): IEEE 754 equality, +0.0 == -0.0.
/// Cell `R03_negzero_back`.
#[test]
fn r03_signed_zero_back_and_forth_wakes_nothing() {
    pin(
        "module top;\n\
  real r = 0.0; int n = 0;\n\
  always @(r) begin n++; $display(\"W t=%0t bits=%h n=%0d\", $time, $realtobits(r), n); end\n\
  initial begin #2 r = $bitstoreal(64'h8000000000000000); #3 r = 0.0; #3 r = 1.0; #2 $display(\"A end n=%0d\", n); $finish; end\n\
endmodule\n\
",
        &[
            "W t=8 bits=3ff0000000000000 n=1",
            "A end n=1",
        ],
    );
}

/// VALUE half (mutant M2). iv = s2v = vl = PRE: the -0.0 bits propagate through `assign b = a`.
/// Cell `R04_ca_negzero`.
#[test]
fn r04_continuous_assign_still_copies_the_bits() {
    pin(
        "module top;\n\
  real a = 0.0; real b;\n\
  assign b = a;\n\
  initial begin\n\
    #5 a = $bitstoreal(64'h8000000000000000); #1 $display(\"S t=%0t a=%h b=%h\", $time, $realtobits(a), $realtobits(b));\n\
    #1 a = 2.0; #1 $display(\"S t=%0t a=%h b=%h\", $time, $realtobits(a), $realtobits(b));\n\
    $finish;\n\
  end\n\
endmodule\n\
",
        &[
            "S t=6 a=8000000000000000 b=8000000000000000",
            "S t=8 a=4000000000000000 b=4000000000000000",
        ],
    );
}

/// iv = s2v. vl RE-RUNS ("C t=5 bits=8000000000000000" twice): a recorded split; §9.2.2.2.1 + §9.4.2 754 equality decide.
/// Cell `R06_comb_negzero`.
#[test]
fn r06_always_comb_does_not_rerun() {
    pin(
        "module top;\n\
  real r = 0.0; real y;\n\
  always_comb begin y = r; $display(\"C t=%0t bits=%h\", $time, $realtobits(r)); end\n\
  initial begin #5 r = $bitstoreal(64'h8000000000000000); #3 r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 bits=0000000000000000",
            "C t=8 bits=3ff0000000000000",
            "A end",
        ],
    );
}

/// iv = s2v; vl "W t=0 | W t=8" (t0 artifact). PRE "W t=5 | W t=8".
/// Cell `R07_nba_negzero`.
#[test]
fn r07_nba_store() {
    pin(
        "module top;\n\
  real r = 0.0;\n\
  always @(r) $display(\"W t=%0t bits=%h\", $time, $realtobits(r));\n\
  initial begin #5 r <= $bitstoreal(64'h8000000000000000); #3 r <= 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=8 bits=3ff0000000000000",
            "A end",
        ],
    );
}

/// VALUE half: $monitor fires at t=5 as iv = s2v do ("M t=5 r=-0.000000", vita too since §4.5.619); vl prints
/// nothing at t=5 (its change test is 754 `==`). §21.2.3 :29009 "changes value".
/// Cell `R08b_monitor_real_f`.
#[test]
fn r08b_monitor_still_prints_the_value_change() {
    pin(
        "module top;\n\
  real r = 0.0;\n\
  initial $monitor(\"M t=%0t r=%f\", $time, r);\n\
  initial begin #5 r = $bitstoreal(64'h8000000000000000); #3 r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "M t=0 r=0.000000",
            "M t=5 r=-0.000000",
            "M t=8 r=1.000000",
            "A end",
        ],
    );
}

/// iv = s2v; vl adds its t0 line. PRE "W t=5 | W t=8".
/// Cell `R09_port_negzero`.
#[test]
fn r09_input_port() {
    pin(
        "module sub(input real x);\n\
  always @(x) $display(\"W t=%0t bits=%h\", $time, $realtobits(x));\n\
endmodule\n\
module top;\n\
  real r = 0.0;\n\
  sub u(.x(r));\n\
  initial begin #5 r = $bitstoreal(64'h8000000000000000); #3 r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=8 bits=3ff0000000000000",
            "A end",
        ],
    );
}

/// Control. iv = s2v = PRE; vl DIDNOTCONVERGE.
/// Cell `R10_ca_nan_same`.
#[test]
fn r10_assign_of_same_nan() {
    pin(
        "module top;\n\
  real a = 0.0; real b; int n = 0;\n\
  assign b = a;\n\
  always @(b) begin n++; $display(\"W t=%0t bits=%h n=%0d\", $time, $realtobits(b), n); end\n\
  initial begin #2 a = $bitstoreal(64'h7ff8000000000000); #3 a = $bitstoreal(64'h7ff8000000000000); #3 a = 1.0; #2 $display(\"A end n=%0d\", n); $finish; end\n\
endmodule\n\
",
        &[
            "W t=2 bits=7ff8000000000000 n=1",
            "W t=8 bits=3ff0000000000000 n=2",
            "A end n=2",
        ],
    );
}

/// iv = s2v = vl "W t=8". PRE "W t=5".
/// Cell `R11_wait_expr_negzero`.
#[test]
fn r11_event_list_or() {
    pin(
        "module top;\n\
  real r = 0.0;\n\
  initial begin @(r or r); $display(\"W t=%0t bits=%h\", $time, $realtobits(r)); end\n\
  initial begin #5 r = $bitstoreal(64'h8000000000000000); #3 r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=8 bits=3ff0000000000000",
            "A end",
        ],
    );
}

/// Control: `wait (r == 0.0)` after 1.0 -> -0.0 — all four "W t=5".
/// Cell `R14_wait_real_level`.
#[test]
fn r14_wait_condition_true_on_negative_zero() {
    pin(
        "module top;\n\
  real r = 1.0;\n\
  initial begin #1; wait (r == 0.0); $display(\"W t=%0t\", $time); end\n\
  initial begin #5 r = $bitstoreal(64'h8000000000000000); #3 r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=5",
            "A end",
        ],
    );
}

/// vl "W t=8"; iv / s2v "sorry: Unpacked structs not supported". Hand = vl (§9.4.2, 754 equality). PRE "W t=5 | W t=8".
/// Cell `R15_struct_real_negzero`.
#[test]
fn r15_unpacked_struct_real_member() {
    pin(
        "module top;\n\
  typedef struct { real r; int i; } s_t;\n\
  s_t s;\n\
  initial begin s.r = 0.0; s.i = 0; end\n\
  initial forever begin @(s.r); $display(\"W t=%0t\", $time); end\n\
  initial begin #5 s.r = $bitstoreal(64'h8000000000000000); #3 s.r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=8",
            "A end",
        ],
    );
}

/// iv = s2v; vl "W t=0 | W t=8". realtime is real (§6.12).
/// Cell `R16_realtime_negzero`.
#[test]
fn r16_realtime() {
    pin(
        "module top;\n\
  realtime r = 0.0;\n\
  always @(r) $display(\"W t=%0t\", $time);\n\
  initial begin #5 r = $bitstoreal(64'h8000000000000000); #3 r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=8",
            "A end",
        ],
    );
}

/// iv = s2v "C t=8"; vl "C t=0" only (no re-run at all). PRE "C t=5 | C t=8".
/// Cell `R17_star_real_negzero`.
#[test]
fn r17_always_star() {
    pin(
        "module top;\n\
  real r = 0.0, y;\n\
  always @* begin y = r; $display(\"C t=%0t\", $time); end\n\
  initial begin #5 r = $bitstoreal(64'h8000000000000000); #3 r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "C t=8",
            "A end",
        ],
    );
}

/// Control (mutant M6): 1.0 -> -1.0 wakes on all three (vl adds its t0 line).
/// Cell `R18_sign_flip_one`.
#[test]
fn r18_sign_flip_of_a_nonzero_is_an_event() {
    pin(
        "module top;\n\
  real r = 1.0;\n\
  always @(r) $display(\"W t=%0t r=%f\", $time, r);\n\
  initial begin #5 r = -1.0; #3 r = 2.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W t=5 r=-1.000000", "W t=8 r=2.000000", "A end"],
    );
}

/// iv = s2v = vl "W t=9" (iv/s2v warn "procedural continuous assignments are not yet fully supported"). PRE "W t=5 | W t=9".
/// Cell `R20_force_negzero`.
#[test]
fn r20_force() {
    pin(
        "module top;\n\
  real r = 0.0;\n\
  always @(r) $display(\"W t=%0t bits=%h\", $time, $realtobits(r));\n\
  initial begin #5 force r = $bitstoreal(64'h8000000000000000); #3 release r; #1 r = 1.0; #1 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=9 bits=3ff0000000000000",
            "A end",
        ],
    );
}

/// `r = z` (int 0 -> +0.0) over -0.0: iv = s2v "W t=8"; vl adds its t0 line. PRE "W t=2 | W t=5 | W t=8".
/// Cell `R22_int_to_real_zero`.
#[test]
fn r22_int_to_real_conversion_store() {
    pin(
        "module top;\n\
  real r = 0.0; int z = 0;\n\
  always @(r) $display(\"W t=%0t bits=%h\", $time, $realtobits(r));\n\
  initial begin #2 r = $bitstoreal(64'h8000000000000000); #3 r = z; #3 r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=8 bits=3ff0000000000000",
            "A end",
        ],
    );
}

/// VALUE half (mutant M11). PRE = vl "W t=5": `wait ((1.0 / r) < 0.0)` became true at t=5, §9.4.3 (:9687) "remain blocked
/// until that condition becomes true". iv = s2v "W t=8" (iverilog re-checks a wait only on an operand EVENT): a recorded split.
/// Cell `R23_wait_recip_negzero`.
#[test]
fn r23_wait_expr_is_rechecked_on_the_value_change() {
    pin(
        "module top;\n\
  real r = 0.0;\n\
  initial begin #1; wait ((1.0 / r) < 0.0); $display(\"W t=%0t\", $time); end\n\
  initial begin #5 r = -0.0; #3 r = -1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W t=5", "A end"],
    );
}

/// iv = s2v "C t=0 | W t=3 | C t=3"; vl "C t=0 | W t=0 | W t=3" (t0 artifact). PRE "W t=0 | C t=0 | C t=0 | W t=3 | C t=3".
/// Cell `R24_t0_negzero_event`.
#[test]
fn r24_time_zero_store_is_no_event() {
    pin(
        "module top;\n\
  real r = 0.0; real y;\n\
  always @(r) $display(\"W t=%0t\", $time);\n\
  always_comb begin y = r; $display(\"C t=%0t\", $time); end\n\
  initial r = $bitstoreal(64'h8000000000000000);\n\
  initial begin #3 r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["C t=0", "W t=3", "C t=3", "A end"],
    );
}

/// Control: 2.0 -> -0.0 at time 0 IS a change; all four agree.
/// Cell `R25_t0_negzero_then_real`.
#[test]
fn r25_time_zero_real_change_then_signed_zero() {
    pin(
        "module top;\n\
  real r = 0.0; logic a = 0;\n\
  always @(r or a) $display(\"W t=%0t a=%0d\", $time, a);\n\
  initial begin r = 2.0; #0 r = $bitstoreal(64'h8000000000000000); end\n\
  initial begin #3 a = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W t=0 a=0", "W t=0 a=0", "W t=3 a=1", "A end"],
    );
}

/// iv = vl = PRE (right only by rounding: the element holder is integral until ENG-1b).
/// Cell `A01_elem_negzero`.
#[test]
fn a01_element_signed_zero() {
    pin(
        "module top;\n\
  real ra [2] = '{0.0, 0.0};\n\
  initial begin @(ra[1]); $display(\"W t=%0t bits=%h\", $time, $realtobits(ra[1])); end\n\
  initial begin #5 ra[1] = $bitstoreal(64'h8000000000000000); #3 ra[1] = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=8 bits=3ff0000000000000",
            "A end",
        ],
    );
}

/// Control = vl (iv / s2v: unpacked structs unsupported). The member is its own real net.
/// Cell `A05_struct_real`.
#[test]
fn a05_struct_member_control() {
    pin(
        "module top;\n\
  typedef struct { real r; int i; } s_t;\n\
  s_t s;\n\
  initial begin s.r = 0.0; s.i = 0; end\n\
  initial forever begin @(s.r); $display(\"W t=%0t r=%f\", $time, s.r); end\n\
  initial begin #5 s.r = 0.25; #1 s.i = 3; #2 s.r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W t=5 r=0.250000", "W t=8 r=1.000000", "A end"],
    );
}

/// Control: iv = vl = PRE.
/// Cell `A07_elem_wait`.
#[test]
fn a07_element_wait_control() {
    pin(
        "module top;\n\
  real ra [2] = '{0.0, 0.0};\n\
  initial begin wait (ra[1] > 0.1); $display(\"W t=%0t r=%f\", $time, ra[1]); end\n\
  initial begin #5 ra[1] = 0.25; #3 ra[1] = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W t=5 r=0.250000", "A end"],
    );
}

/// HEAP twin (mutant M9). Hand (LRM): the element is 754-equal at t=5, so no re-run; 1.0 at t=8 is a change. Both tools never re-run
/// on the element: iv "C t=0 | C t=0 | A end" (sorry: constant selects in always_* ...), vl "C t=0 | A end". PRE "C t=0 | C t=0 | C t=5 | C t=8".
/// Cell `R12_dyn_real_comb`.
#[test]
fn r12_dynamic_array_element_in_always_comb() {
    pin(
        "module top;\n\
  real d [];\n\
  real y;\n\
  initial begin d = new[2]; d[0] = 0.0; d[1] = 0.0; end\n\
  always_comb begin y = d[1]; $display(\"C t=%0t\", $time); end\n\
  initial begin #5 d[1] = $bitstoreal(64'h8000000000000000); #3 d[1] = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0",
            "C t=0",
            "C t=8",
            "A end",
        ],
    );
}

/// HEAP VALUE half (mutant M15): PRE = vl "W t=5" (§9.4.3); iv "A end".
/// Cell `E51_heap_comb_and_wait`.
#[test]
fn e51_heap_value_change_rechecks_a_wait() {
    pin(
        "module top;\n\
  real d []; real y;\n\
  initial begin d = new[2]; d[0] = 0.0; d[1] = 0.0; end\n\
  always_comb y = d[0];\n\
  initial begin #1; wait ((1.0 / d[1]) < 0.0); $display(\"W t=%0t\", $time); end\n\
  initial begin #5 d[1] = $bitstoreal(64'h8000000000000000); #3 d[1] = -1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=5",
            "A end",
        ],
    );
}

/// vl minus its t0 line: "S t=6 y=8000000000000000 | W t=8 y=3ff0000000000000"; iv "S t=6 y=0000000000000000" (no heap CA). PRE adds "W t=5".
/// Cell `E50_heap_ca_negzero`.
#[test]
fn e50_heap_element_through_an_assign() {
    pin(
        "module top;\n\
  real d []; real y;\n\
  initial begin d = new[2]; d[0] = 0.0; d[1] = 0.0; end\n\
  assign y = d[1];\n\
  always @(y) $display(\"W t=%0t y=%h\", $time, $realtobits(y));\n\
  initial begin #5 d[1] = $bitstoreal(64'h8000000000000000); #1 $display(\"S t=%0t y=%h\", $time, $realtobits(y)); #2 d[1] = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "S t=6 y=8000000000000000",
            "W t=8 y=3ff0000000000000",
            "A end",
        ],
    );
}

/// Mutant M13. vl minus its t0 line. §10.6.2 (:10683) the force RHS "shall be treated just as a continuous assignment".
/// iv = s2v "sorry: procedural continuous assignments ... evaluated once" -> y=0000000000000000 (disqualified). PRE adds "W t=5".
/// Cell `E01_force_expr_real`.
#[test]
fn e01_force_expression_reevaluates_on_the_value_change() {
    pin(
        "module top;\n\
  real r = 0.0; real y;\n\
  initial force y = r * 1.0;\n\
  always @(y) $display(\"W t=%0t y=%h\", $time, $realtobits(y));\n\
  initial begin #5 r = $bitstoreal(64'h8000000000000000); #1 $display(\"S t=%0t r=%h y=%h\", $time, $realtobits(r), $realtobits(y)); #2 r = 2.0; #1 $display(\"S t=%0t y=%h\", $time, $realtobits(y)); $finish; end\n\
endmodule\n\
",
        &[
            "S t=6 r=8000000000000000 y=8000000000000000",
            "W t=8 y=4000000000000000",
            "S t=9 y=4000000000000000",
        ],
    );
}

/// Mutant M13. PRE = vl; iv = s2v "W t=0 y=0 | A end" (force RHS evaluated once). y is logic: its 0 -> 1 is an event.
/// Cell `E27_force_expr_logic_reader`.
#[test]
fn e27_force_expression_feeding_a_logic_reader() {
    pin(
        "module top;\n\
  real r = 0.0; logic y;\n\
  initial force y = (1.0 / r) < 0.0;\n\
  always @(y) $display(\"W t=%0t y=%b\", $time, y);\n\
  initial begin #5 r = $bitstoreal(64'h8000000000000000); #3 r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=0 y=0",
            "W t=5 y=1",
            "W t=8 y=0",
            "A end",
        ],
    );
}

/// iv = s2v; vl "S t=6 y=8000000000000000 y2=... y3=..." (re-runs every block): recorded split.
/// Cell `E02_comb_noside`.
#[test]
fn e02_comb_star_and_level_writers() {
    pin(
        "module top;\n\
  real r = 0.0; real y, y2, y3;\n\
  always_comb y = r;\n\
  always @* y2 = r;\n\
  always @(r) y3 = r;\n\
  initial begin #5 r = $bitstoreal(64'h8000000000000000); #1 $display(\"S t=%0t y=%h y2=%h y3=%h\", $time, $realtobits(y), $realtobits(y2), $realtobits(y3)); #2 r = 2.0; #1 $display(\"S t=%0t y=%h\", $time, $realtobits(y)); $finish; end\n\
endmodule\n\
",
        &[
            "S t=6 y=0000000000000000 y2=0000000000000000 y3=0000000000000000",
            "S t=9 y=4000000000000000",
        ],
    );
}

/// iv = s2v; vl adds its t0 line.
/// Cell `E07_task_out_negzero`.
#[test]
fn e07_task_output_copy_out() {
    pin(
        "module top;\n\
  real r = 0.0;\n\
  task automatic t(output real o); o = $bitstoreal(64'h8000000000000000); endtask\n\
  always @(r) $display(\"W t=%0t bits=%h\", $time, $realtobits(r));\n\
  initial begin #5 t(r); #1 $display(\"S t=%0t bits=%h\", $time, $realtobits(r)); #2 r = 1.0; #1 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "S t=6 bits=8000000000000000",
            "W t=8 bits=3ff0000000000000",
            "A end",
        ],
    );
}

/// iv "C t=0 | S t=6 bits=8000000000000000 y=0000000000000000 | W t=8 | C t=8" (resume order at t=8 differs: a
/// pre-existing order split, PRE has vita's order too); vl "... y=8000000000000000 | W t=8" (never re-runs the comb).
/// Cell `E08_array_whole_assign`.
#[test]
fn e08_whole_array_assign() {
    pin(
        "module top;\n\
  real ra [2] = '{0.0, 0.0}; real y;\n\
  always_comb begin y = ra[1]; $display(\"C t=%0t\", $time); end\n\
  initial forever begin @(ra[1]); $display(\"W t=%0t\", $time); end\n\
  initial begin #5 ra = '{0.0, $bitstoreal(64'h8000000000000000)}; #1 $display(\"S t=%0t bits=%h y=%h\", $time, $realtobits(ra[1]), $realtobits(y)); #2 ra = '{0.0, 1.0}; #1 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0",
            "S t=6 bits=8000000000000000 y=0000000000000000",
            "C t=8",
            "W t=8",
            "A end",
        ],
    );
}

/// iv = s2v: `r = 1'bx` stores +0.0 over -0.0, no event; vl adds its t0 line.
/// Cell `E14_x_to_real`.
#[test]
fn e14_x_to_real_conversion() {
    pin(
        "module top;\n\
  real r = 1.0;\n\
  always @(r) $display(\"W t=%0t bits=%h\", $time, $realtobits(r));\n\
  initial begin #2 r = $bitstoreal(64'h8000000000000000); #3 r = 1'bx; #1 $display(\"S t=%0t bits=%h\", $time, $realtobits(r)); #2 r = 1.0; #1 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=2 bits=8000000000000000",
            "S t=6 bits=0000000000000000",
            "W t=8 bits=3ff0000000000000",
            "A end",
        ],
    );
}

/// iv = s2v; vl "L t=0" only.
/// Cell `E17_latch_negzero`.
#[test]
fn e17_always_latch() {
    pin(
        "module top;\n\
  real r = 0.0; real y; logic en = 1;\n\
  always_latch begin if (en) y = r; $display(\"L t=%0t\", $time); end\n\
  initial begin #5 r = $bitstoreal(64'h8000000000000000); #3 r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "L t=0",
            "L t=8",
            "A end",
        ],
    );
}

/// Hand (§16.4.3: a static deferred assertion is an always_comb; 754 equality). vl "P t=0 | P t=5 | P t=5 | P t=8 | P t=8 ...";
/// iv "sorry: Deferred assertions are not supported"; s2v drops it ("A end").
/// Cell `E18_item_deferred_assert`.
#[test]
fn e18_static_deferred_assertion() {
    pin(
        "module top;\n\
  real r = 0.0;\n\
  a1: assert #0 (r == 0.0 || r == 1.0) $display(\"P t=%0t\", $time); else $display(\"F t=%0t\", $time);\n\
  initial begin #5 r = $bitstoreal(64'h8000000000000000); #3 r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "P t=0",
            "P t=8",
            "A end",
        ],
    );
}

/// iv = s2v; vl adds its t0 line.
/// Cell `E19_out_port_negzero`.
#[test]
fn e19_output_port() {
    pin(
        "module sub(output real o);\n\
  initial begin o = 0.0; #5 o = $bitstoreal(64'h8000000000000000); #3 o = 1.0; end\n\
endmodule\n\
module top;\n\
  real w;\n\
  sub u(.o(w));\n\
  always @(w) $display(\"W t=%0t bits=%h\", $time, $realtobits(w));\n\
  initial begin #10 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W t=8 bits=3ff0000000000000", "A end"],
    );
}

/// iv = s2v; vl adds its t0 line.
/// Cell `E20_ca_chain_event`.
#[test]
fn e20_assign_chain() {
    pin(
        "module top;\n\
  real a = 0.0; real b;\n\
  assign b = a;\n\
  always @(b) $display(\"W t=%0t bits=%h\", $time, $realtobits(b));\n\
  initial begin #5 a = $bitstoreal(64'h8000000000000000); #3 a = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=8 bits=3ff0000000000000",
            "A end",
        ],
    );
}

/// Control: the logic y changes 0 -> 1 at t=5 (the value moved). PRE = vl; iv = s2v omit "W t=0 y=0" (a pre-existing
/// time-0 split of an assign over a real, unchanged by this rule).
/// Cell `E21_ca_to_logic_event`.
#[test]
fn e21_assign_to_logic_control() {
    pin(
        "module top;\n\
  real a = 0.0; logic y;\n\
  assign y = (1.0 / a) < 0.0;\n\
  always @(y) $display(\"W t=%0t y=%b\", $time, y);\n\
  initial begin #5 a = $bitstoreal(64'h8000000000000000); #3 a = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=0 y=0",
            "W t=5 y=1",
            "W t=8 y=0",
            "A end",
        ],
    );
}

/// iv "C t=0 | C t=8"; vl "C t=0".
/// Cell `E24_nba_elem_comb`.
#[test]
fn e24_nba_element() {
    pin(
        "module top;\n\
  real ra [2] = '{0.0, 0.0}; real y;\n\
  always_comb begin y = ra[1]; $display(\"C t=%0t\", $time); end\n\
  initial begin #5 ra[1] <= $bitstoreal(64'h8000000000000000); #3 ra[1] <= 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0",
            "C t=8",
            "A end",
        ],
    );
}

/// vl "W t=8"; iv / s2v cannot parse `iff` here.
/// Cell `E26_iff_real`.
#[test]
fn e26_iff_on_a_real() {
    pin(
        "module top;\n\
  real r = 0.0; logic en = 1;\n\
  initial forever begin @(r iff en); $display(\"W t=%0t\", $time); end\n\
  initial begin #5 r = $bitstoreal(64'h8000000000000000); #3 r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=8",
            "A end",
        ],
    );
}

/// iv = s2v "S t=6 z=0"; vl "S t=6 z=1" (re-runs the comb): recorded split.
/// Cell `E28_ca_in_assign_comb_logic`.
#[test]
fn e28_comb_logic_of_a_real() {
    pin(
        "module top;\n\
  real r = 0.0; logic z;\n\
  always_comb z = (1.0 / r) < 0.0;\n\
  initial begin #5 r = $bitstoreal(64'h8000000000000000); #1 $display(\"S t=%0t z=%b\", $time, z); #2 r = -1.0; #1 $display(\"S t=%0t z=%b\", $time, z); $finish; end\n\
endmodule\n\
",
        &[
            "S t=6 z=0",
            "S t=9 z=1",
        ],
    );
}

/// Hand; vl "C t=0" only; iv / s2v: unpacked structs unsupported.
/// Cell `E33_comb_struct_arr_member`.
#[test]
fn e33_comb_struct_array_member() {
    pin(
        "module top;\n\
  typedef struct { real r; int i; } s_t;\n\
  s_t sa [2]; real y;\n\
  initial begin sa[0].r = 0.0; sa[1].r = 0.0; sa[0].i = 0; sa[1].i = 0; end\n\
  always_comb begin y = sa[1].r; $display(\"C t=%0t\", $time); end\n\
  initial begin #5 sa[1].r = $bitstoreal(64'h8000000000000000); #3 sa[1].r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0",
            "C t=8",
            "A end",
        ],
    );
}

/// iv = s2v; vl "C t=0".
/// Cell `E46_func_ret_real_comb`.
#[test]
fn e46_function_of_a_real_in_comb() {
    pin(
        "module top;\n\
  real r = 0.0; real y;\n\
  function automatic real f(input real a); return a * 1.0; endfunction\n\
  always_comb begin y = f(r); $display(\"C t=%0t\", $time); end\n\
  initial begin #5 r = $bitstoreal(64'h8000000000000000); #3 r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0",
            "C t=8",
            "A end",
        ],
    );
}

/// Control: iv = s2v = PRE.
/// Cell `E15_nan_sign_flip`.
#[test]
fn e15_nan_sign_flip_is_an_event() {
    pin(
        "module top;\n\
  real r = 0.0;\n\
  always @(r) $display(\"W t=%0t bits=%h\", $time, $realtobits(r));\n\
  initial begin #2 r = $bitstoreal(64'h7ff8000000000001); #3 r = $bitstoreal(64'hfff8000000000001); #3 r = 1.0; #1 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=2 bits=7ff8000000000001",
            "W t=5 bits=fff8000000000001",
            "W t=8 bits=3ff0000000000000",
            "A end",
        ],
    );
}

/// Control: iv = s2v = PRE.
/// Cell `E22_decl_init_negzero`.
#[test]
fn e22_declaration_initialiser_control() {
    pin(
        "module top;\n\
  real r = $bitstoreal(64'h8000000000000000);\n\
  always @(r) $display(\"W t=%0t bits=%h\", $time, $realtobits(r));\n\
  always_comb $display(\"C t=%0t bits=%h\", $time, $realtobits(r));\n\
  initial begin #3 r = 1.0; #1 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0 bits=8000000000000000",
            "W t=3 bits=3ff0000000000000",
            "C t=3 bits=3ff0000000000000",
            "A end",
        ],
    );
}

/// VALUE half: PRE = vl "W t=5" (the bits changed and the condition became true, §9.4.3); iv = s2v "W t=8".
/// Cell `E23_wait_only_change`.
#[test]
fn e23_wait_on_the_bits() {
    pin(
        "module top;\n\
  real r = 0.0; logic done = 0;\n\
  initial begin wait ($realtobits(r) != 0); $display(\"W t=%0t\", $time); end\n\
  initial begin #5 r = $bitstoreal(64'h8000000000000000); #3 r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=5",
            "A end",
        ],
    );
}

/// Control (mutant M14): a 64-bit INTEGRAL sign bit over zero is an event on all three tools. iv orders "C" before "L" at
/// t=5 and t=8 (a pre-existing resume-order split; PRE has this order).
/// Cell `I01_logic64_signbit`.
#[test]
fn i01_integral_sign_bit_is_an_event() {
    pin(
        "module top;\n\
  logic [63:0] v = 64'h0; longint l = 0; logic [63:0] y;\n\
  always @(v) $display(\"W t=%0t v=%h\", $time, v);\n\
  always @(l) $display(\"L t=%0t l=%h\", $time, l);\n\
  always_comb begin y = v; $display(\"C t=%0t\", $time); end\n\
  initial begin #5 v = 64'h8000000000000000; l = 64'h8000000000000000; #3 v = 64'h0; l = 0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "C t=0",
            "W t=5 v=8000000000000000",
            "L t=5 l=8000000000000000",
            "C t=5",
            "W t=8 v=0000000000000000",
            "L t=8 l=0000000000000000",
            "C t=8",
            "A end",
        ],
    );
}

/// Row-73 lens twin (lens-diff-r6). Hand = iv rule: no wake at the -0.0 store.
/// Cell `V00s_real_wait_bits_sens`.
#[test]
fn row73_v00s_wait_on_a_real() {
    pin(
        "module top;\n\
  real r = 0.0;\n\
  initial begin\n\
    @(r);\n\
    $display(\"W t=%0t r=%f bits=%h\", $time, r, $realtobits(r));\n\
  end\n\
  initial begin #5 r = $bitstoreal(64'h8000000000000000); $display(\"S t=%0t bits=%h\", $time, $realtobits(r)); #3 r = 1.0; end\n\
  initial #20 $display(\"A end\");\n\
endmodule\n\
",
        &[
            "S t=5 bits=8000000000000000",
            "W t=8 r=1.000000 bits=3ff0000000000000",
            "A end",
        ],
    );
}

/// Row-73 lens twin (lens-diff-r5): 1.0 -> 0.0 is an event, 0.0 -> -0.0 is not.
/// Cell `X16s_real_negzero_sens`.
#[test]
fn row73_x16s_zero_then_negative_zero() {
    pin(
        "module top;\n\
  real r = 1.0;\n\
  always @(r) $display(\"W t=%0t r=%f\", $time, r);\n\
  initial begin #5 r = 0.0; #1 r = -0.0; end\n\
  initial #20 $display(\"A end\");\n\
endmodule\n\
",
        &["W t=5 r=0.000000", "A end"],
    );
}

/// Row-73 lens twin (lens-diff-r6).
/// Cell `V06s_real_array_elem_sens`.
#[test]
fn row73_v06s_element_wait() {
    pin(
        "module top;\n\
  real ra [2];\n\
  initial begin\n\
    @(ra[0]);\n\
    $display(\"W t=%0t bits=%h\", $time, $realtobits(ra[0]));\n\
  end\n\
  initial begin ra[0] = 0.0; ra[1] = 0.0; #5 ra[0] = $bitstoreal(64'h8000000000000000); #3 ra[0] = 1.0; end\n\
  initial #20 $display(\"A end\");\n\
endmodule\n\
",
        &[
            "W t=8 bits=3ff0000000000000",
            "A end",
        ],
    );
}

/// VALUE half (mutant M2): the dump records the t=5 store, "#5 / r-0 !" as iv = s2v write it (vita since §4.5.619;
/// before, "r0"); vl records nothing at t=5. §21.7 :29954 "records the values of the variables that change".
/// Cell `R05_vcd_negzero`.
#[test]
fn r05_dump_records_the_value_change() {
    let src = "module top;\n\
  real r = 0.0;\n\
  initial begin $dumpfile(\"r.vcd\"); $dumpvars(0, top); end\n\
  initial begin #5 r = $bitstoreal(64'h8000000000000000); #3 r = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
";
    for be in ["native", "vm", "interp"] {
        let d = scratch("r05");
        std::fs::write(d.join("t.sv"), src).unwrap();
        let (out, err, code) = vita(&d, &["--backend", be, "t.sv"]);
        assert_eq!(code, 0, "{be}: {out}{err}");
        let vcd = std::fs::read_to_string(d.join("r.vcd")).expect("r.vcd");
        let _ = std::fs::remove_dir_all(&d);
        let body: Vec<&str> = vcd
            .lines()
            .skip_while(|l| !l.starts_with("$enddefinitions"))
            .collect();
        assert_eq!(
            body,
            [
                "$enddefinitions $end",
                "$dumpvars",
                "r0 !",
                "$end",
                "#0",
                "#5",
                "r-0 !",
                "#8",
                "r1 !"
            ],
            "backend {be}"
        );
    }
}
