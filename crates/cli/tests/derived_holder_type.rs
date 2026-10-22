//! ENG-1b: a net vita DERIVES to hold a value on the design's behalf takes the TYPE of the storage
//! the value comes from or goes to — kind, width, signedness and unpacked shape — read by one
//! classifier (`held_of_storage`, elaborate `held.rs`). Producers pinned here:
//!
//! * the intra-assignment capture temps (`r = #1 e`, `= @(go) e`, `<= @(go) e`, `= repeat (n) @(e)`,
//!   the `$random` / `$dist_*` / `$cast` intercept, `$value$plusargs`): the DESTINATION's type, so
//!   `tmp = e` is the same assignment as `lhs = e` (IEEE 1800-2017 §9.4.5, Table 9-3). A
//!   hierarchical destination (`u.r = #1 x`) is recorded and retyped when the write resolves;
//! * the string-queue self-pattern snapshot (`q = '{q[1], q[0]}`): the element's type;
//! * the `@(ra[1])` select holder: the selected element's type, a `real` one a copy net at time 0
//!   (`sim_engine::alias::real_move`);
//! * the SVA sampled-value register (`$past` / `$stable` / `$changed` / `$rose` / `$fell`): the
//!   variable's type with its signedness; a constant has no register (its sampled value is
//!   itself, §16.5.1); `$rose` / `$fell` of a `real` or `string` is refused by name, once;
//! * the clocking sample / drive holders: the signal's type; a string, heap or unpacked-array item
//!   gets no holder, and the FIRST USE of `cb.x` is refused by name, at the use (an unused item runs).
//!
//! Oracles: Icarus Verilog 13.0 `-g2012` (iv), sv2v 0.0.13 -> iverilog (s2v), Verilator 5.052
//! `--binary` (vl). Each cell's comment names the lines that decide it; "hand" = the IEEE clause
//! named beside it. vl runs every header `always` once at time 0 (a "t0 artifact" its lines carry
//! and these pins drop), rounds a sampled `real` (disqualified on real SVA values) and refuses
//! `$rose` of a string; iverilog cannot parse a clocking block and drops concurrent assertions.
//! A pinned first-tick SVA line marked "Q-DSV" is a known residue: IEEE §16.5.1 makes the default
//! sampled value the DECLARATION value, vita's register starts at the type default (queued).
//! `-0.0` renders as `-0.000000` (§4.5.619). Every cell runs on native, vm and interp; the ones whose
//! holder rides the `.velab` also through the staged flow.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn scratch(tag: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dht_{tag}_{}_{n}", std::process::id()));
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

/// [`pin`], and the staged flow prints it too (the holder is an elaborate product, so it
/// rides the `.velab`).
fn pin_staged(src: &str, want: &[&str]) {
    pin(src, want);
    assert_eq!(run_staged(src), want, "staged vcmp -> velab -> vrun");
}

/// `src` is refused on every backend: exit 1 and a diagnostic line that starts with `at` and
/// carries `diag`; returns each backend's whole output.
fn refused(src: &str, at: &str, diag: &str) -> Vec<String> {
    let mut all_out = Vec::new();
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
        all_out.push(all);
    }
    all_out
}

/// [`pin`], and no backend warns beyond the missing-timescale note: an unused clocking item is
/// never sampled (PRE's commit read the handle without an index: W4020 on vm / interp only).
fn pin_quiet(src: &str, want: &[&str]) {
    for be in ["native", "vm", "interp"] {
        let d = scratch(be);
        std::fs::write(d.join("t.sv"), src).unwrap();
        let (out, err, code) = vita(&d, &["--backend", be, "t.sv"]);
        let _ = std::fs::remove_dir_all(&d);
        assert_eq!(code, 0, "{be}: exit {code}\n{out}{err}");
        assert_eq!(lines(&out), want, "backend {be}");
        assert!(
            !err.lines()
                .any(|l| l.contains("warning[") && !l.contains("W1017")),
            "{be}: unexpected warning\n{err}"
        );
    }
}

/// [`refused`], and that line is the ONLY error (`errors=1`).
fn refused_once(src: &str, at: &str, diag: &str) {
    for all in refused(src, at, diag) {
        assert!(
            all.lines().any(|l| l.starts_with("errors=1 ")),
            "not one error:\n{all}"
        );
    }
}

/// E1-b (mutant M7): iv = vl. PRE missed t=5 (the integral holder held round(0.5) == round(0.75) == 1).
/// Cell `A02_elem_half`.
#[test]
fn a02_element_holder_sees_the_real_value() {
    pin_staged(
        "module top;\n\
  real ra [2] = '{0.0, 0.0};\n\
  initial forever begin @(ra[1]); $display(\"W t=%0t r=%f\", $time, ra[1]); end\n\
  initial begin #2 ra[1] = 0.5; #3 ra[1] = 0.75; #3 ra[1] = 2.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=2 r=0.500000",
            "W t=5 r=0.750000",
            "W t=8 r=2.000000",
            "A end",
        ],
    );
}

/// E1-b: iv "W t=5 | W t=8"; vl adds its t0 run. PRE "W t=0 | W t=8" (x -> 0 of the logic holder; t=5 missed).
/// Cell `A03_elem_header`.
#[test]
fn a03_element_holder_header_no_time_zero_event() {
    pin_staged(
        "module top;\n\
  real ra [2] = '{0.0, 0.0};\n\
  always @(ra[1]) $display(\"W t=%0t r=%f\", $time, ra[1]);\n\
  initial begin #5 ra[1] = 0.25; #1 ra[0] = 3.0; #2 ra[1] = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=5 r=0.250000",
            "W t=8 r=1.000000",
            "A end",
        ],
    );
}

/// E1-b: iv = s2v = vl. PRE "W t=8".
/// Cell `A04_elem_2d`.
#[test]
fn a04_two_dimensional_element() {
    pin_staged(
        "module top;\n\
  real m [2][2];\n\
  initial begin m[0][0] = 0.0; m[0][1] = 0.0; m[1][0] = 0.0; m[1][1] = 0.0; end\n\
  initial forever begin @(m[1][0]); $display(\"W t=%0t r=%f\", $time, m[1][0]); end\n\
  initial begin #5 m[1][0] = 0.25; #1 m[0][0] = 3.0; #2 m[1][0] = 1.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=5 r=0.250000",
            "W t=8 r=1.000000",
            "A end",
        ],
    );
}

/// E1-b: vl "... | W t=8 r=-0.000000" (vita renders -0.0 as -0.000000 since §4.5.619); iv / s2v: unpacked structs unsupported.
/// Cell `E13_struct_arr_member_real`.
#[test]
fn e13_struct_array_real_member_holder() {
    pin(
        "module top;\n\
  typedef struct { real r; int i; } s_t;\n\
  s_t sa [2];\n\
  initial begin sa[0].r = 0.0; sa[1].r = 0.0; sa[0].i = 0; sa[1].i = 0; end\n\
  initial begin #1; forever begin @(sa[1].r); $display(\"W t=%0t r=%f\", $time, sa[1].r); end end\n\
  initial begin #2 sa[1].r = 0.5; #3 sa[1].r = 0.75; #3 sa[1].r = $bitstoreal(64'h8000000000000000); #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=2 r=0.500000",
            "W t=5 r=0.750000",
            "W t=8 r=-0.000000",
            "A end",
        ],
    );
}

/// E1-b + E1-a: iv = s2v "W t=3 | H t=3"; vl adds "H t=0". PRE "H t=0 | W t=3 | H t=3".
/// Cell `E38_holder_t0_init_negzero`.
#[test]
fn e38_holder_copies_a_time_zero_negative_zero() {
    pin(
        "module top;\n\
  real ra [2];\n\
  initial forever begin @(ra[1]); $display(\"W t=%0t\", $time); end\n\
  always @(ra[1]) $display(\"H t=%0t\", $time);\n\
  initial ra[1] = $bitstoreal(64'h8000000000000000);\n\
  initial begin #3 ra[1] = 0.5; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W t=3", "H t=3", "A end"],
    );
}

/// E1-b: iv = vl. PRE "W t=2 | W t=8".
/// Cell `E40_realtime_holder`.
#[test]
fn e40_realtime_element_holder() {
    pin(
        "module top;\n\
  realtime ta [2] = '{0.0, 0.0};\n\
  initial forever begin @(ta[1]); $display(\"W t=%0t\", $time); end\n\
  initial begin #2 ta[1] = 0.5; #3 ta[1] = 0.75; #3 ta[1] = 2.0; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=2",
            "W t=5",
            "W t=8",
            "A end",
        ],
    );
}

/// E1-b: iv; vl adds its t0 run. PRE "W t=0 | W t=2 | W t=8".
/// Cell `E43_holder_mixed_list`.
#[test]
fn e43_holder_in_a_mixed_list() {
    pin(
        "module top;\n\
  real ra [2] = '{0.0, 0.0}; logic b = 0;\n\
  always @(ra[1] or b) $display(\"W t=%0t\", $time);\n\
  initial begin #2 ra[1] = 0.5; #3 ra[1] = 0.75; #3 b = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &["W t=2", "W t=5", "W t=8", "A end"],
    );
}

/// E1-b capture temp (mutant M16): vl "S t=2 y=1.500000"; iv / s2v vvp "syntax error". §9.2.2 / §10.4.1: the RHS value
/// is captured, then assigned. PRE "y=2.000000" (the capture rounded it).
/// Cell `E29_intra_delay_real`.
#[test]
fn e29_intra_assignment_delay_captures_the_real() {
    pin_staged(
        "module top;\n\
  real y = 0.0, z = 0.0;\n\
  initial begin y = #2 1.5; $display(\"S t=%0t y=%f\", $time, y); z <= #1 2.25; #2 $display(\"S t=%0t z=%f\", $time, z); $finish; end\n\
endmodule\n\
",
        &[
            "S t=2 y=1.500000",
            "S t=4 z=2.250000",
        ],
    );
}

/// E1-b capture temp (mutant M16): vl; iv / s2v vvp "syntax error". PRE "y=2.000000".
/// Cell `E30_intra_event_real`.
#[test]
fn e30_intra_assignment_event_captures_the_real() {
    pin_staged(
        "module top;\n\
  real y = 0.0; logic go = 0;\n\
  initial begin y = @(go) 1.5; $display(\"S t=%0t y=%f\", $time, y); $finish; end\n\
  initial #3 go = 1;\n\
endmodule\n\
",
        &["S t=3 y=1.500000"],
    );
}

/// E1-b clocking holder (mutant M10): vl "W t=0 | W t=6" (t0 artifact); iv / s2v cannot parse the clocking block. PRE
/// "W t=2 | W t=6" (the x-initial integral holder moved at the first edge). §14.13: cb.r is the sampled value of r.
/// Cell `R19_clockvar_real`.
#[test]
fn r19_clocking_sample_of_negative_zero() {
    pin(
        "module top;\n\
  logic clk = 0; real r = 0.0;\n\
  clocking cb @(posedge clk); input r; endclocking\n\
  always @(cb.r) $display(\"W t=%0t\", $time);\n\
  initial begin #1 r = $bitstoreal(64'h8000000000000000); #1 clk = 1; #2 clk = 0; #1 r = 1.0; #1 clk = 1; #2 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=6",
            "A end",
        ],
    );
}

/// E1-b clocking holder: vl minus its t0 line. PRE read cb.r as 4609434218613702656.000000.
/// Cell `R19b_clockvar_real_poszero`.
#[test]
fn r19b_clocking_sample_reads_the_real() {
    pin_staged(
        "module top;\n\
  logic clk = 0; real r = 0.0; real y = 7.0;\n\
  clocking cb @(posedge clk); input r; endclocking\n\
  always @(cb.r) begin y = cb.r; $display(\"W t=%0t y=%f bits=%h\", $time, y, $realtobits(cb.r)); end\n\
  initial begin #1 $display(\"S t=%0t cb.r=%f\", $time, cb.r); #1 clk = 1; #2 clk = 0; #1 r = 1.5; #1 clk = 1; #1 $display(\"S t=%0t cb.r=%f y=%f\", $time, cb.r, y); #1 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "S t=1 cb.r=0.000000",
            "W t=6 y=1.500000 bits=3ff8000000000000",
            "S t=7 cb.r=1.500000 y=1.500000",
            "A end",
        ],
    );
}

/// E1-b clocking holder: vl. PRE "y=4612811918334230528.000000".
/// Cell `R19c_clockvar_real_value`.
#[test]
fn r19c_clocking_sample_value() {
    pin(
        "module top;\n\
  logic clk = 0; real r = 2.5; real y = 7.0;\n\
  clocking cb @(posedge clk); input r; endclocking\n\
  initial begin #1 clk = 1; #1 y = cb.r; $display(\"S t=%0t y=%f\", $time, y); $display(\"S t=%0t sum=%f\", $time, cb.r + 0.25); $finish; end\n\
endmodule\n\
",
        &[
            "S t=2 y=2.500000",
            "S t=2 sum=2.750000",
        ],
    );
}

/// E1-b + E1-a: vl minus its t0 line. PRE "W t=1 | W t=4 bits=c3e0000000000000 | W t=7 bits=43cff80000000000".
/// Cell `R19d_clockvar_real_negzero_after`.
#[test]
fn r19d_clocking_sample_negative_zero_after_zero() {
    pin(
        "module top;\n\
  logic clk = 0; real r = 0.0;\n\
  clocking cb @(posedge clk); input r; endclocking\n\
  always @(cb.r) $display(\"W t=%0t bits=%h\", $time, $realtobits(cb.r));\n\
  initial begin #1 clk = 1; #1 clk = 0; #1 r = $bitstoreal(64'h8000000000000000); #1 clk = 1; #1 clk = 0; #1 r = 1.0; #1 clk = 1; #1 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=7 bits=3ff0000000000000",
            "A end",
        ],
    );
}

/// E1-b clocking OUTPUT holder: vl minus its t0 line. PRE drove the integer 1 ("bits=0000000000000001").
/// Cell `E47_clockvar_output_real`.
#[test]
fn e47_clocking_output_drives_the_real() {
    pin_staged(
        "module top;\n\
  logic clk = 0; real r = 0.0;\n\
  clocking cb @(posedge clk); output r; endclocking\n\
  always @(r) $display(\"W t=%0t bits=%h\", $time, $realtobits(r));\n\
  initial begin #1 cb.r <= $bitstoreal(64'h8000000000000000); #1 clk = 1; #1 clk = 0; #1 cb.r <= 1.0; #1 clk = 1; #1 $display(\"S t=%0t bits=%h\", $time, $realtobits(r)); #1 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "W t=5 bits=3ff0000000000000",
            "S t=6 bits=3ff0000000000000",
            "A end",
        ],
    );
}

/// E1-b + E1-a, hand: the t=3 drive stores +0.0 over -0.0 (a value change, no event). vl "S t=5 bits=8000000000000000"
/// (verilator skips a 754-equal store); iv / s2v cannot parse. PRE "W t=1 | W t=3 | ... | W t=7 bits=0000000000000001".
/// Cell `E47b_clockvar_output_real_zero`.
#[test]
fn e47b_clocking_output_drive_of_positive_zero() {
    pin(
        "module top;\n\
  logic clk = 0; real r = 0.0;\n\
  clocking cb @(posedge clk); output r; endclocking\n\
  always @(r) $display(\"W t=%0t bits=%h\", $time, $realtobits(r));\n\
  initial begin #1 r = $bitstoreal(64'h8000000000000000); #1 cb.r <= 0.0; #1 clk = 1; #1 clk = 0; #1 $display(\"S t=%0t bits=%h\", $time, $realtobits(r)); #1 cb.r <= 1.0; #1 clk = 1; #1 $display(\"S t=%0t bits=%h\", $time, $realtobits(r)); #1 $display(\"A end\"); $finish; end\n\
endmodule\n\
",
        &[
            "S t=5 bits=0000000000000000",
            "W t=7 bits=3ff0000000000000",
            "S t=8 bits=3ff0000000000000",
            "A end",
        ],
    );
}

/// Row-73 lens twin (lens-sound-r6): iv = vl wake at t=5.
/// Cell `Q2s_realarr_frac_sens`.
#[test]
fn row73_q2s_element_fraction() {
    pin(
        "module top;\n\
  real ra[2] = '{0.0, 0.0};\n\
  initial begin\n\
    @(ra[1]);\n\
    $display(\"W t=%0t r=%f\", $time, ra[1]);\n\
  end\n\
  initial begin #5 ra[1] = 0.25; #3 ra[1] = 1.0; end\n\
  initial #20 $display(\"A end\");\n\
endmodule\n\
",
        &["W t=5 r=0.250000", "A end"],
    );
}

/// Row-73 lens twin (lens-sound-r6), control (0.75 rounds to 1: PRE was right by rounding).
/// Cell `Q2b_realarr_075_sens`.
#[test]
fn row73_q2b_element_three_quarters() {
    pin(
        "module top;\n\
  real ra[2] = '{0.0, 0.0};\n\
  initial begin\n\
    @(ra[1]);\n\
    $display(\"W t=%0t r=%f\", $time, ra[1]);\n\
  end\n\
  initial begin #5 ra[1] = 0.75; #3 ra[1] = 1.0; end\n\
  initial #20 $display(\"A end\");\n\
endmodule\n\
",
        &["W t=5 r=0.750000", "A end"],
    );
}

/// Row-73 lens twin (lens-sound-r6): iv "C t=5 | C t=8"; no time-0 line.
/// Cell `Q2h_realarr_frac_hdr_sens`.
#[test]
fn row73_q2h_element_header() {
    pin(
        "module top;\n\
  real ra[2] = '{0.0, 0.0};\n\
  always @(ra[1]) $display(\"C t=%0t r=%f\", $time, ra[1]);\n\
  initial begin #5 ra[1] = 0.25; #3 ra[1] = 1.0; end\n\
  initial #20 $display(\"A end\");\n\
endmodule\n\
",
        &["C t=5 r=0.250000", "C t=8 r=1.000000", "A end"],
    );
}

/// Row-73 lens twin (lens-sound-r6), hand: the wake is at t=5. PRE "A F t=8".
/// Cell `Q2n_realarr_frac_noctl`.
#[test]
fn row73_q2n_element_then_deferred_assertion() {
    pin(
        "module top;\n\
  real ra[2] = '{0.0, 0.0};\n\
  initial begin\n\
    @(ra[1]);\n\
    assert #0 (0) else $display(\"A F t=%0t\", $time);\n\
  end\n\
  initial begin #5 ra[1] = 0.25; #3 ra[1] = 1.0; end\n\
  initial #20 $display(\"A end\");\n\
endmodule\n\
",
        &["A F t=5", "A end"],
    );
}

/// F1 (E1-b-t0): the `@(ra[1])` holder is a copy net at time 0 = iv's lines (iv orders the two t=5 / t=8 wakes both ways: a §4.7 race).
/// Cell `H09_holder_nonzero_decl_init` (W/eng1/lens-diff-r1/o7/H09_holder_nonzero_decl_init). Oracles: iv "X t=5 r=0.750000 | W t=5 r=0.750000 | W t=8 r=1.000000 | X t=8 r=1.000000 | A end"; s2v "does not run ((no sv2v->iverilog binary))"; vl "W t=0 r=0.500000 | X t=5 r=0.750000 | W t=5 r=0.750000 | X t=8 r=1.000000 | W t=8 r=1.000000 | A end".
/// PRE "W t=0 r=0.500000 | A end"; r1 "W t=0 r=0.500000 | W t=5 r=0.750000 | X t=5 r=0.750000 | W t=8 r=1.000000 | X t=8 r=1.000000 | A end". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_h09_select_holder_nonzero_init_no_t0() {
    pin(
        r##"module top;
  real ra [0:1] = '{0.0, 0.5};
  always @(ra[1]) $display("W t=%0t r=%f", $time, ra[1]);
  initial forever begin @(ra[1]); $display("X t=%0t r=%f", $time, ra[1]); end
  initial begin #5 ra[1] = 0.75; #3 ra[1] = 1.0; #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=5 r=0.750000"##,
            r##"X t=5 r=0.750000"##,
            r##"W t=8 r=1.000000"##,
            r##"X t=8 r=1.000000"##,
            r##"A end"##,
        ],
    );
}

/// F1, hand = vl minus its time-0 lines (iv / s2v cannot parse the pattern) for `ra[1]`. RESIDUE `M t=0`: the 2-D word `m[1][0]` has a bounds-guarded index the copy-net word fold declines, so it is no copy net -- its integral twin `int m [0:1][0:1]` prints the same `M t=0` on PRE (iv none): pre-existing, not the real class.
/// Cell `H09b_holder_default_init` (W/eng1/lens-diff-r1/o7/H09b_holder_default_init). Oracles: iv "does not run ((no iverilog binary))"; s2v "does not run ((no sv2v->iverilog binary))"; vl "W t=0 r=2.500000 | M t=0 r=1.500000 | W t=5 r=0.750000 | M t=5 r=0.250000 | A end".
/// PRE "W t=0 r=2.500000 | M t=0 r=1.500000 | W t=5 r=0.750000 | M t=5 r=0.250000 | A end"; r1 "W t=0 r=2.500000 | M t=0 r=1.500000 | W t=5 r=0.750000 | M t=5 r=0.250000 | A end". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_h09b_default_pattern_holders_no_t0() {
    pin(
        r##"module top;
  real ra [0:1] = '{default: 2.5};
  real m [0:1][0:1] = '{'{1.5, 1.5}, '{1.5, 1.5}};
  always @(ra[1]) $display("W t=%0t r=%f", $time, ra[1]);
  always @(m[1][0]) $display("M t=%0t r=%f", $time, m[1][0]);
  initial begin #5 ra[1] = 0.75; m[1][0] = 0.25; #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"M t=0 r=1.500000"##,
            r##"W t=5 r=0.750000"##,
            r##"M t=5 r=0.250000"##,
            r##"A end"##,
        ],
    );
}

/// F1: no time-0 wake of `always @(ra[1])` (n = 0 at t=1).
/// Cell `H09d_holder_nonzero_deferred_assert` (W/eng1/lens-diff-r1/o8/H09d_holder_nonzero_deferred_assert). Oracles: iv "S t=1 n=0 | S t=7 n=1 | A end"; s2v "does not run ((no sv2v->iverilog binary))"; vl "S t=1 n=1 | S t=7 n=2 | A end".
/// PRE "S t=1 n=1 | S t=7 n=1 | A end"; r1 "S t=1 n=1 | S t=7 n=2 | A end". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_h09d_select_holder_wakes_nothing_at_t0() {
    pin(
        r##"module top;
  real ra [0:1] = '{0.0, 0.5}; int n = 0;
  always @(ra[1]) n = n + 1;
  initial begin #1 $display("S t=%0t n=%0d", $time, n); #5 ra[1] = 0.75; #1 $display("S t=%0t n=%0d", $time, n); $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=1 n=0"##, r##"S t=7 n=1"##, r##"A end"##],
    );
}

/// F1: a user `assign c = ra[1];` of a real word is the same copy net.
/// Cell `H09u_user_ca_real_elem_nonzero` (W/eng1/lens-diff-r1/o8/H09u_user_ca_real_elem_nonzero). Oracles: iv "W t=5 c=0.750000 | A end"; s2v "does not run ((no sv2v->iverilog binary))"; vl "W t=0 c=0.500000 | W t=5 c=0.750000 | A end".
/// PRE "W t=0 c=0.500000 | W t=5 c=0.750000 | A end"; r1 "W t=0 c=0.500000 | W t=5 c=0.750000 | A end". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_h09u_user_real_word_copy_no_t0() {
    pin(
        r##"module top;
  real ra [0:1] = '{0.0, 0.5}; real c;
  assign c = ra[1];
  always @(c) $display("W t=%0t c=%f", $time, c);
  initial begin #5 ra[1] = 0.75; #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"W t=5 c=0.750000"##, r##"A end"##],
    );
}

/// F1: a real input port is a real copy net.
/// Cell `N33_port_real_nonzero_init` (W/eng1/lens-diff-r1/o9/N33_port_real_nonzero_init). Oracles: iv "W t=5 r=0.750000 | A end"; s2v "W t=5 r=0.750000 | A end"; vl "W t=0 r=0.500000 | W t=5 r=0.750000 | A end".
/// PRE "W t=0 r=0.500000 | W t=5 r=0.750000 | A end"; r1 "W t=0 r=0.500000 | W t=5 r=0.750000 | A end". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_n33_real_port_copy_no_t0() {
    pin(
        r##"module sub(input real ri);
  always @(ri) $display("W t=%0t r=%f", $time, ri);
endmodule
module top;
  real r = 0.5;
  sub u(.ri(r));
  initial begin #5 r = 0.75; #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"W t=5 r=0.750000"##, r##"A end"##],
    );
}

/// F1: a chain of real copies = iv's lines (the order of the two t=5 wakes is a §4.7 race).
/// Cell `C01_real_copy_chain` (W/eng/replan-eng1/engine/o/q2/C01_real_copy_chain). Oracles: iv "V t=5 c=0.750000 | W t=5 d=0.750000 | A end"; s2v "V t=5 c=0.750000 | W t=5 d=0.750000 | A end"; vl "W t=0 d=0.500000 | V t=0 c=0.500000 | W t=5 d=0.750000 | V t=5 c=0.750000 | A end".
/// PRE "W t=0 d=0.500000 | V t=0 c=0.500000 | W t=5 d=0.750000 | V t=5 c=0.750000 | A end"; r1 "W t=0 d=0.500000 | V t=0 c=0.500000 | W t=5 d=0.750000 | V t=5 c=0.750000 | A end". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_c01_real_copy_chain_no_t0() {
    pin(
        r##"module top;
  real r = 0.5; real c, d;
  assign c = r;
  assign d = c;
  always @(d) $display("W t=%0t d=%f", $time, d);
  always @(c) $display("V t=%0t c=%f", $time, c);
  initial begin #5 r = 0.75; #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=5 d=0.750000"##,
            r##"V t=5 c=0.750000"##,
            r##"A end"##,
        ],
    );
}

/// F1: two port hops.
/// Cell `C03_real_port_chain2` (W/eng/replan-eng1/engine/o/q2/C03_real_port_chain2). Oracles: iv "W t=5 r=0.750000 | A end"; s2v "W t=5 r=0.750000 | A end"; vl "W t=0 r=0.500000 | W t=5 r=0.750000 | A end".
/// PRE "W t=0 r=0.500000 | W t=5 r=0.750000 | A end"; r1 "W t=0 r=0.500000 | W t=5 r=0.750000 | A end". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_c03_real_port_chain_no_t0() {
    pin(
        r##"module leaf(input real ri);
  always @(ri) $display("W t=%0t r=%f", $time, ri);
endmodule
module mid(input real mi);
  leaf l(.ri(mi));
endmodule
module top;
  real r = 0.5;
  mid u(.mi(r));
  initial begin #5 r = 0.75; #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"W t=5 r=0.750000"##, r##"A end"##],
    );
}

/// F1: an `assign #0` copy that landed in the time-0 settle is a copy net too.
/// Cell `C04_real_copy_delay0` (W/eng/replan-eng1/engine/o/q2/C04_real_copy_delay0). Oracles: iv "W t=5 c=0.750000 | A end"; s2v "W t=5 c=0.750000 | A end"; vl "W t=0 c=0.000000 | W t=0 c=0.500000 | W t=5 c=0.750000 | A end".
/// PRE "W t=0 c=0.500000 | W t=5 c=0.750000 | A end"; r1 "W t=0 c=0.500000 | W t=5 c=0.750000 | A end". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_c04_real_copy_delay0_landed_no_t0() {
    pin(
        r##"module top;
  real r = 0.5; real c;
  assign #0 c = r;
  always @(c) $display("W t=%0t c=%f", $time, c);
  initial begin #5 r = 0.75; #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"W t=5 c=0.750000"##, r##"A end"##],
    );
}

/// F1: a NaN-initialized source.
/// Cell `C06_real_copy_nan_init` (W/eng/replan-eng1/engine/o/q2/C06_real_copy_nan_init). Oracles: iv "W t=5 bits=3fe8000000000000 | A end"; s2v "W t=5 bits=3fe8000000000000 | A end"; vl "does not run (%Error-DIDNOTCONVERGE: Active region did not converge after '--converge-limit' of 10000 tr)".
/// PRE "W t=0 bits=7ff8000000000001 | W t=5 bits=3fe8000000000000 | A end"; r1 "W t=0 bits=7ff8000000000001 | W t=5 bits=3fe8000000000000 | A end". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_c06_real_copy_nan_init_no_t0() {
    pin(
        r##"module top;
  real r = $bitstoreal(64'h7ff8000000000001); real c;
  assign c = r;
  always @(c) $display("W t=%0t bits=%h", $time, $realtobits(c));
  initial begin #5 r = 0.75; #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"W t=5 bits=3fe8000000000000"##, r##"A end"##],
    );
}

/// F1: the suppression is transitive -- an integral copy of the real copy loses its time-0 wake too.
/// Cell `C18_int_copy_of_real_copy` (W/eng/replan-eng1/engine/o/q2/C18_int_copy_of_real_copy). Oracles: iv "V t=5 c=0.750000 | A end"; s2v "V t=5 c=0.750000 | A end"; vl "W t=0 w=0000000000000001 | V t=0 c=0.500000 | V t=5 c=0.750000 | A end".
/// PRE "W t=0 w=0000000000000001 | V t=0 c=0.500000 | V t=5 c=0.750000 | A end"; r1 "W t=0 w=0000000000000001 | V t=0 c=0.500000 | V t=5 c=0.750000 | A end". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_c18_int_copy_of_real_copy_dropped() {
    pin(
        r##"module top;
  real r = 0.5; real c; wire [63:0] w;
  assign c = r;
  assign w = c;
  always @(w) $display("W t=%0t w=%h", $time, w);
  always @(c) $display("V t=%0t c=%f", $time, c);
  initial begin #5 r = 0.75; #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"V t=5 c=0.750000"##, r##"A end"##],
    );
}

/// Control (integral twin, already a copy net).
/// Cell `H09i_int_holder_nonzero_init` (W/eng1/lens-diff-r1/o8/H09i_int_holder_nonzero_init). Oracles: iv "W t=5 v=6 | A end"; s2v "W t=5 v=6 | A end"; vl "W t=0 v=5 | W t=5 v=6 | A end".
/// PRE "W t=5 v=6 | A end"; r1 "W t=5 v=6 | A end". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_h09i_int_holder_control() {
    pin(
        r##"module top;
  int ia [0:1] = '{0, 5};
  always @(ia[1]) $display("W t=%0t v=%0d", $time, ia[1]);
  initial begin #5 ia[1] = 6; #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"W t=5 v=6"##, r##"A end"##],
    );
}

/// Control (-0.0 initializer: r1 already value-only).
/// Cell `H09n_holder_negzero_decl_init` (W/eng1/lens-diff-r1/o8/H09n_holder_negzero_decl_init). Oracles: iv "W t=7 bits=3fe8000000000000 | A end"; s2v "does not run ((no sv2v->iverilog binary))"; vl "W t=0 bits=8000000000000000 | W t=7 bits=3fe8000000000000 | A end".
/// PRE "W t=0 bits=8000000000000000 | W t=7 bits=3fe8000000000000 | A end"; r1 "W t=7 bits=3fe8000000000000 | A end". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_h09n_negzero_holder_control() {
    pin(
        r##"module top;
  real ra [0:1] = '{0.0, -0.0};
  always @(ra[1]) $display("W t=%0t bits=%h", $time, $realtobits(ra[1]));
  initial begin #5 ra[1] = 0.0; #2 ra[1] = 0.75; #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"W t=7 bits=3fe8000000000000"##, r##"A end"##],
    );
}

/// Control (integral port).
/// Cell `N33b_port_int_nonzero_init` (W/eng1/lens-diff-r1/o9/N33b_port_int_nonzero_init). Oracles: iv "W t=5 r=6 | A end"; s2v "W t=5 r=6 | A end"; vl "W t=0 r=5 | W t=5 r=6 | A end".
/// PRE "W t=5 r=6 | A end"; r1 "W t=5 r=6 | A end". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_n33b_int_port_control() {
    pin(
        r##"module sub(input int ri);
  always @(ri) $display("W t=%0t r=%0d", $time, ri);
endmodule
module top;
  int r = 5;
  sub u(.ri(r));
  initial begin #5 r = 6; #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"W t=5 r=6"##, r##"A end"##],
    );
}

/// Control: the source moved at time 0 (an `initial` write), so the copy wakes = iv.
/// Cell `C02_real_copy_src_initial_t0` (W/eng/replan-eng1/engine/o/q2/C02_real_copy_src_initial_t0). Oracles: iv "W t=0 c=0.500000 | W t=5 c=0.750000 | A end"; s2v "W t=0 c=0.500000 | W t=5 c=0.750000 | A end"; vl "W t=0 c=0.500000 | W t=5 c=0.750000 | A end".
/// PRE "W t=0 c=0.500000 | W t=5 c=0.750000 | A end"; r1 "W t=0 c=0.500000 | W t=5 c=0.750000 | A end". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_c02_source_written_by_initial_control() {
    pin(
        r##"module top;
  real r; real c;
  assign c = r;
  always @(c) $display("W t=%0t c=%f", $time, c);
  initial r = 0.5;
  initial begin #5 r = 0.75; #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=0 c=0.500000"##,
            r##"W t=5 c=0.750000"##,
            r##"A end"##,
        ],
    );
}

/// Control (integral word copy).
/// Cell `C11_int_word_copy_twin` (W/eng/replan-eng1/engine/o/q2/C11_int_word_copy_twin). Oracles: iv "W t=5 c=6 | A end"; s2v "W t=5 c=6 | A end"; vl "W t=0 c=5 | W t=5 c=6 | A end".
/// PRE "W t=5 c=6 | A end"; r1 "W t=5 c=6 | A end". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_c11_int_word_copy_control() {
    pin(
        r##"module top;
  int ia [0:1] = '{0, 5}; int c;
  assign c = ia[1];
  always @(c) $display("W t=%0t c=%0d", $time, c);
  initial begin #5 ia[1] = 6; #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"W t=5 c=6"##, r##"A end"##],
    );
}

/// Control: a copy of a net a constant `assign` drove moves at time 0 = iv.
/// Cell `C17_real_copy_of_real_ca_const` (W/eng/replan-eng1/engine/o/q2/C17_real_copy_of_real_ca_const). Oracles: iv "W t=0 c=2.500000 | A end"; s2v "W t=0 c=2.500000 | A end"; vl "W t=0 c=2.500000 | A end".
/// PRE "W t=0 c=2.500000 | A end"; r1 "W t=0 c=2.500000 | A end". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_c17_copy_of_constant_ca_control() {
    pin(
        r##"module top;
  real a; real c;
  assign a = 2.5;
  assign c = a;
  always @(c) $display("W t=%0t c=%f", $time, c);
  initial begin #5 $display("A end"); $finish; end
endmodule
"##,
        &[r##"W t=0 c=2.500000"##, r##"A end"##],
    );
}

/// Control: `wait` on a -0.0 copy at time 0 = iv.
/// Cell `C19_real_copy_negzero_wait_t0` (W/eng/replan-eng1/engine/o/q2b/C19_real_copy_negzero_wait_t0). Oracles: iv "W t=0 | E t=5 bits=3ff0000000000000 | A end"; s2v "W t=0 | E t=5 bits=3ff0000000000000 | A end"; vl "W t=0 | E t=0 bits=8000000000000000 | E t=5 bits=3ff0000000000000 | A end".
/// PRE "W t=0 | E t=0 bits=8000000000000000 | E t=5 bits=3ff0000000000000 | A end"; r1 "W t=0 | E t=5 bits=3ff0000000000000 | A end". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_c19_negzero_copy_wait_control() {
    pin(
        r##"module top;
  real r = $bitstoreal(64'h8000000000000000); real c;
  assign c = r;
  initial begin wait ((1.0 / c) < 0.0); $display("W t=%0t", $time); end
  always @(c) $display("E t=%0t bits=%h", $time, $realtobits(c));
  initial begin #5 r = 1.0; #2 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=0"##,
            r##"E t=5 bits=3ff0000000000000"##,
            r##"A end"##,
        ],
    );
}

/// Control, outside the arm (mutant M19): an int -> real conversion copy keeps its r1 text; its t=0 line and stale read are queued (Q-CONVT0).
/// Cell `C14_haz_real_of_lv64` (W/eng/replan-eng1/engine/o/q2/C14_haz_real_of_lv64). Oracles: iv "R t=1 c=7.000000 | W t=1 c=7.000000 | A end"; s2v "R t=1 c=7.000000 | W t=1 c=7.000000 | A end"; vl "W t=0 c=5.000000 | R t=1 c=5.000000 | W t=1 c=7.000000 | A end".
/// PRE "W t=0 c=5.000000 | R t=1 c=5.000000 | W t=1 c=7.000000 | A end"; r1 "W t=0 c=5.000000 | R t=1 c=5.000000 | W t=1 c=7.000000 | A end". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_c14_int_to_real_conversion_copy_unchanged() {
    pin(
        r##"module top;
  logic [63:0] lv = 5; real c;
  assign c = lv;
  always @(c) $display("W t=%0t c=%f", $time, c);
  initial begin #1 lv = 7; $display("R t=%0t c=%f", $time, c); #4 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"W t=0 c=5.000000"##,
            r##"R t=1 c=5.000000"##,
            r##"W t=1 c=7.000000"##,
            r##"A end"##,
        ],
    );
}

/// Refusal unchanged (a forced copy).
/// Cell `C05_real_copy_forced` (W/eng/replan-eng1/engine/o/q2/C05_real_copy_forced). Oracles: iv "W t=3 c=2.000000 | W t=7 c=0.750000 | S t=9 c=0.750000 | A end"; s2v "W t=3 c=2.000000 | W t=7 c=0.750000 | S t=9 c=0.750000 | A end"; vl "W t=0 c=0.500000 | W t=3 c=2.000000 | W t=7 c=0.750000 | S t=9 c=0.750000 | A end".
/// PRE "t.sv:3:3: error[VITA-E3018] E-ELAB-LVALUE-KIND: variable `top.c` is driven by a continuous `assign` and also by a `force` or `release`; IEEE 1800 §10.6.2 allows that, but v1 keeps a continuous `assign` to a variable that is not `logic` only as its sole writer — declare it `logic` [in top]"; r1 "t.sv:3:3: error[VITA-E3018] E-ELAB-LVALUE-KIND: variable `top.c` is driven by a continuous `assign` and also by a `force` or `release`; IEEE 1800 §10.6.2 allows that, but v1 keeps a continuous `assign` to a variable that is not `logic` only as its sole writer — declare it `logic` [in top]". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_c05_forced_real_copy_refused() {
    refused(
        r##"module top;
  real r = 0.5; real c;
  assign c = r;
  always @(c) $display("W t=%0t c=%f", $time, c);
  initial begin #3 force c = 2.0; #2 r = 0.75; #2 release c; #2 $display("S t=%0t c=%f", $time, c); $display("A end"); $finish; end
endmodule
"##,
        r##"t.sv:3:3: "##,
        r##"error[VITA-E3018]"##,
    );
}

/// Refusal unchanged (unpacked-struct initializer).
/// Cell `C07_real_copy_struct_member` (W/eng/replan-eng1/engine/o/q2/C07_real_copy_struct_member). Oracles: iv "does not run ((no iverilog binary))"; s2v "does not run ((no sv2v->iverilog binary))"; vl "W t=0 c=0.500000 | W t=5 c=0.750000 | A end".
/// PRE "t.sv:3:7: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected an unpacked-struct declaration initializer / '{…} pattern is unsupported in v1, found keyword 'real'"; r1 "t.sv:3:7: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected an unpacked-struct declaration initializer / '{…} pattern is unsupported in v1, found keyword 'real'". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_c07_struct_member_copy_refused() {
    refused(
        r##"module top;
  typedef struct { real r; int k; } s_t;
  s_t s = '{0.5, 1}; real c;
  assign c = s.r;
  always @(c) $display("W t=%0t c=%f", $time, c);
  initial begin #5 s.r = 0.75; #2 $display("A end"); $finish; end
endmodule
"##,
        r##"t.sv:3:7: "##,
        r##"error[VITA-E2002]"##,
    );
}

/// Refusal unchanged (string copy net).
/// Cell `C10_string_copy` (W/eng/replan-eng1/engine/o/q2/C10_string_copy). Oracles: iv "internal error: 12vvp_fun_bufz: recv_string(ab) not implemented | Assertion failed: (0), function recv_string, file vvp_net.cc, line 3353."; s2v "internal error: 12vvp_fun_bufz: recv_string(ab) not implemented | Assertion failed: (0), function recv_string, file vvp_net.cc, line 3353."; vl "W t=0 s2=ab | W t=5 s2=cd | A end".
/// PRE "t.sv:1:8: error[VITA-E3018] E-ELAB-LVALUE-KIND: continuous assign drives variable `top.s2` (declare it wire/logic) [in top] | t.sv:1:8: error[VITA-E3009] E-ELAB-UNSUPPORTED: a dynamic-storage handle cannot appear in an event control [in top]"; r1 "t.sv:1:8: error[VITA-E3018] E-ELAB-LVALUE-KIND: continuous assign drives variable `top.s2` (declare it wire/logic) [in top] | t.sv:1:8: error[VITA-E3009] E-ELAB-UNSUPPORTED: a dynamic-storage handle cannot appear in an event control [in top]". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_c10_string_copy_refused() {
    refused(
        r##"module top;
  string s1 = "ab"; string s2;
  assign s2 = s1;
  always @(s2) $display("W t=%0t s2=%s", $time, s2);
  initial begin #5 s1 = "cd"; #2 $display("A end"); $finish; end
endmodule
"##,
        r##"t.sv:1:8: "##,
        r##"error[VITA-E3018]"##,
    );
}

/// Refusal unchanged (a copy of an `assign` that calls a function).
/// Cell `C20_real_copy_held_fn` (W/eng/replan-eng1/engine/o/q2b/C20_real_copy_held_fn). Oracles: iv "W t=0 c=1.000000 | W t=5 c=1.500000 | A end"; s2v "W t=0 c=1.000000 | W t=5 c=1.500000 | A end"; vl "W t=0 c=1.000000 | W t=5 c=1.500000 | A end".
/// PRE "t.sv:4:3: error[VITA-E3018] E-ELAB-LVALUE-KIND: variable `top.m` is driven by a continuous `assign` that calls a function; v1 keeps a continuous `assign` to a variable that is not `logic` only when its right-hand side calls none — a variable the function body reads that is not an argument re-runs the `assign` here and in Verilator, not in Icarus Verilog (IEEE 1800 §10.3.2 re-evaluates on an operand) [in top]"; r1 "t.sv:4:3: error[VITA-E3018] E-ELAB-LVALUE-KIND: variable `top.m` is driven by a continuous `assign` that calls a function; v1 keeps a continuous `assign` to a variable that is not `logic` only when its right-hand side calls none — a variable the function body reads that is not an argument re-runs the `assign` here and in Verilator, not in Icarus Verilog (IEEE 1800 §10.3.2 re-evaluates on an operand) [in top]". §6.8 (ieee1800-2017.txt:4228-4230): a variable initializer is no event; a copy net takes its source's time-0 status.
#[test]
fn eng1r2_f1_c20_held_function_copy_refused() {
    refused(
        r##"module top;
  function automatic real f(real x); return x * 2.0; endfunction
  real r = 0.5; real m; real c;
  assign m = f(r);
  assign c = m;
  always @(c) $display("W t=%0t c=%f", $time, c);
  initial begin #5 r = 0.75; #2 $display("A end"); $finish; end
endmodule
"##,
        r##"t.sv:4:3: "##,
        r##"error[VITA-E3018]"##,
    );
}

/// B2 (H1 SVA_PREV): `$stable` of a `real` starting at its type default, hand (= vl here).
/// Cell `SV01_stable_real_default` (W/eng1/r2/o/new/SV01_stable_real_default). Oracles: iv "does not run ((no iverilog binary))"; s2v "A end"; vl "F t=15 | A end".
/// PRE "F t=15 | F t=25 | A end"; r1 "F t=15 | F t=25 | A end". §16.9.3 (ieee1800-2017.txt:17820-17826 $stable/$changed, :17866-17871 $past); §16.5.1 (:16792-16797) default sampled value. vl rounds a real sampled value (SC14: past=1.000000 for a source only ever 0.5): disqualified for real SVA values.
#[test]
fn eng1r2_b2_sv01_stable_real_type_default() {
    pin(
        r##"module top;
  logic clk = 0; real r = 0.0;
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) $stable(r)) else $display("F t=%0t", $time);
  initial begin #12 r = 0.75; #20 $display("A end"); $finish; end
endmodule
"##,
        &[r##"F t=15"##, r##"A end"##],
    );
}

/// B2: `!$changed` over 0.0 -> 0.25, hand.
/// Cell `SV02_changed_real_default` (W/eng1/r2/o/new/SV02_changed_real_default). Oracles: iv "does not run ((no iverilog binary))"; s2v "A end"; vl "A end".
/// PRE "F t=15 | F t=25 | A end"; r1 "F t=15 | F t=25 | A end". §16.9.3 (ieee1800-2017.txt:17820-17826 $stable/$changed, :17866-17871 $past); §16.5.1 (:16792-16797) default sampled value. vl rounds a real sampled value (SC14: past=1.000000 for a source only ever 0.5): disqualified for real SVA values.
#[test]
fn eng1r2_b2_sv02_changed_real_type_default() {
    pin(
        r##"module top;
  logic clk = 0; real r = 0.0;
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) !$changed(r)) else $display("F t=%0t", $time);
  initial begin #12 r = 0.25; #20 $display("A end"); $finish; end
endmodule
"##,
        &[r##"F t=15"##, r##"A end"##],
    );
}

/// B2: `$past` of a real keeps the value, hand.
/// Cell `SV03_past_real_default` (W/eng1/r2/o/new/SV03_past_real_default). Oracles: iv "does not run ((no iverilog binary))"; s2v "A end"; vl "F t=5 past=0.000000 r=0.000000 | F t=15 past=0.000000 r=0.750000 | F t=25 past=1.000000 r=0.750000 | A end".
/// PRE "F t=5 past=0.000000 r=0.000000 | F t=15 past=0.000000 r=0.750000 | F t=25 past=1.000000 r=0.750000 | A end"; r1 "F t=5 past=0.000000 r=0.000000 | F t=15 past=0.000000 r=0.750000 | F t=25 past=1.000000 r=0.750000 | A end". §16.9.3 (ieee1800-2017.txt:17820-17826 $stable/$changed, :17866-17871 $past); §16.5.1 (:16792-16797) default sampled value. vl rounds a real sampled value (SC14: past=1.000000 for a source only ever 0.5): disqualified for real SVA values.
#[test]
fn eng1r2_b2_sv03_past_real_type_default() {
    pin(
        r##"module top;
  logic clk = 0; real r = 0.0;
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) 0) else $display("F t=%0t past=%f r=%f", $time, $past(r), r);
  initial begin #12 r = 0.75; #20 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"F t=5 past=0.000000 r=0.000000"##,
            r##"F t=15 past=0.000000 r=0.750000"##,
            r##"F t=25 past=0.750000 r=0.750000"##,
            r##"A end"##,
        ],
    );
}

/// B2: `$stable` of a `realtime`, hand.
/// Cell `SV04_stable_realtime_default` (W/eng1/r2/o/new/SV04_stable_realtime_default). Oracles: iv "does not run ((no iverilog binary))"; s2v "A end"; vl "A end".
/// PRE "F t=15 | F t=25 | A end"; r1 "F t=15 | F t=25 | A end". §16.9.3 (ieee1800-2017.txt:17820-17826 $stable/$changed, :17866-17871 $past); §16.5.1 (:16792-16797) default sampled value. vl rounds a real sampled value (SC14: past=1.000000 for a source only ever 0.5): disqualified for real SVA values.
#[test]
fn eng1r2_b2_sv04_stable_realtime_type_default() {
    pin(
        r##"module top;
  logic clk = 0; realtime r;
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) $stable(r)) else $display("F t=%0t", $time);
  initial begin #12 r = 0.25; #20 $display("A end"); $finish; end
endmodule
"##,
        &[r##"F t=15"##, r##"A end"##],
    );
}

/// B2: `$past` of a `realtime`, hand.
/// Cell `SV05_past_realtime_default` (W/eng1/r2/o/new/SV05_past_realtime_default). Oracles: iv "does not run ((no iverilog binary))"; s2v "A end"; vl "F t=5 past=0.000000 r=0.000000 | F t=15 past=0.000000 r=2.500000 | F t=25 past=3.000000 r=2.500000 | A end".
/// PRE "F t=5 past=0.000000 r=0.000000 | F t=15 past=0.000000 r=2.500000 | F t=25 past=3.000000 r=2.500000 | A end"; r1 "F t=5 past=0.000000 r=0.000000 | F t=15 past=0.000000 r=2.500000 | F t=25 past=3.000000 r=2.500000 | A end". §16.9.3 (ieee1800-2017.txt:17820-17826 $stable/$changed, :17866-17871 $past); §16.5.1 (:16792-16797) default sampled value. vl rounds a real sampled value (SC14: past=1.000000 for a source only ever 0.5): disqualified for real SVA values.
#[test]
fn eng1r2_b2_sv05_past_realtime_type_default() {
    pin(
        r##"module top;
  logic clk = 0; realtime r;
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) 0) else $display("F t=%0t past=%f r=%f", $time, $past(r), r);
  initial begin #12 r = 2.5; #20 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"F t=5 past=0.000000 r=0.000000"##,
            r##"F t=15 past=0.000000 r=2.500000"##,
            r##"F t=25 past=2.500000 r=2.500000"##,
            r##"A end"##,
        ],
    );
}

/// Control: a -0.0 sample over +0.0 is no change (= s2v = vl).
/// Cell `C11_assert_changed_negzero` (W/eng1/lens-sound-r1/o/b2/C11_assert_changed_negzero). Oracles: iv "does not run ((no iverilog binary))"; s2v "A end"; vl "A end".
/// PRE "A end"; r1 "A end". §16.9.3 (ieee1800-2017.txt:17820-17826 $stable/$changed, :17866-17871 $past); §16.5.1 (:16792-16797) default sampled value.
#[test]
fn eng1r2_b2_c11_changed_negzero_control() {
    pin(
        r##"module top;
  logic clk = 0; real r = 0.0;
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) !$changed(r)) else $display("F t=%0t", $time);
  initial begin #12 r = $bitstoreal(64'h8000000000000000); #20 $display("A end"); $finish; end
endmodule
"##,
        &[r##"A end"##],
    );
}

/// Control: as C11 for `$stable`.
/// Cell `C12_assert_stable_negzero` (W/eng1/lens-sound-r1/o/b2/C12_assert_stable_negzero). Oracles: iv "does not run ((no iverilog binary))"; s2v "A end"; vl "A end".
/// PRE "A end"; r1 "A end". §16.9.3 (ieee1800-2017.txt:17820-17826 $stable/$changed, :17866-17871 $past); §16.5.1 (:16792-16797) default sampled value.
#[test]
fn eng1r2_b2_c12_stable_negzero_control() {
    pin(
        r##"module top;
  logic clk = 0; real r = 0.0;
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) $stable(r)) else $display("F t=%0t", $time);
  initial begin #12 r = $bitstoreal(64'h8000000000000000); #20 $display("A end"); $finish; end
endmodule
"##,
        &[r##"A end"##],
    );
}

/// B2 residue (r2 text; hand "A end"): first tick = Q-DSV: default sampled value is the declaration value (vita: the type default).
/// Cell `H20_changed_real_const` (W/eng1/lens-sound-r1/o/b5/H20_changed_real_const). Oracles: iv "does not run ((no iverilog binary))"; s2v "A end"; vl "A end".
/// PRE "F t=5 | F t=15 | F t=25 | A end"; r1 "F t=5 | F t=15 | F t=25 | A end". §16.9.3 (ieee1800-2017.txt:17820-17826 $stable/$changed, :17866-17871 $past); §16.5.1 (:16792-16797) default sampled value.
#[test]
fn eng1r2_b2_h20_changed_real_decl_init_residue() {
    pin(
        r##"module top;
  logic clk = 0; real r = 0.5;
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) !$changed(r)) else $display("F t=%0t", $time);
  initial begin #30 $display("A end"); $finish; end
endmodule
"##,
        &[r##"F t=5"##, r##"A end"##],
    );
}

/// B2 residue (r2 text; hand "A end"): first tick = Q-DSV: default sampled value is the declaration value (vita: the type default).
/// Cell `H23_assert_stable_real_bool_ctx` (W/eng1/lens-sound-r1/o/b5/H23_assert_stable_real_bool_ctx). Oracles: iv "does not run ((no iverilog binary))"; s2v "A end"; vl "A end".
/// PRE "F t=5 | F t=15 | F t=25 | A end"; r1 "F t=5 | F t=15 | F t=25 | A end". §16.9.3 (ieee1800-2017.txt:17820-17826 $stable/$changed, :17866-17871 $past); §16.5.1 (:16792-16797) default sampled value.
#[test]
fn eng1r2_b2_h23_stable_real_decl_init_residue() {
    pin(
        r##"module top;
  logic clk = 0; real r = 0.5; logic ok;
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) $stable(r)) else $display("F t=%0t", $time);
  initial begin #12 r = 0.5; #20 $display("A end"); $finish; end
endmodule
"##,
        &[r##"F t=5"##, r##"A end"##],
    );
}

/// B2 residue (r2 text; hand has past=0.500000 at t=5): first tick = Q-DSV: default sampled value is the declaration value (vita: the type default).
/// Cell `C14_assert_past_real_print` (W/eng1/lens-sound-r1/o/b2/C14_assert_past_real_print). Oracles: iv "does not run ((no iverilog binary))"; s2v "A end"; vl "F t=5 past=1.000000 r=0.500000 | F t=15 past=1.000000 r=0.750000 | F t=25 past=1.000000 r=0.750000 | A end".
/// PRE "F t=5 past=0.000000 r=0.500000 | F t=15 past=1.000000 r=0.750000 | F t=25 past=1.000000 r=0.750000 | A end"; r1 "F t=5 past=0.000000 r=0.500000 | F t=15 past=1.000000 r=0.750000 | F t=25 past=1.000000 r=0.750000 | A end". §16.9.3 (ieee1800-2017.txt:17820-17826 $stable/$changed, :17866-17871 $past); §16.5.1 (:16792-16797) default sampled value. vl rounds a real sampled value (SC14: past=1.000000 for a source only ever 0.5): disqualified for real SVA values.
#[test]
fn eng1r2_b2_c14_past_real_decl_init_residue() {
    pin(
        r##"module top;
  logic clk = 0; real r = 0.5;
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) 0) else $display("F t=%0t past=%f r=%f", $time, $past(r), r);
  initial begin #12 r = 0.75; #20 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"F t=5 past=0.000000 r=0.500000"##,
            r##"F t=15 past=0.500000 r=0.750000"##,
            r##"F t=25 past=0.750000 r=0.750000"##,
            r##"A end"##,
        ],
    );
}

/// B2 residue (r2 text; hand "A end"): first tick = Q-DSV: default sampled value is the declaration value (vita: the type default).
/// Cell `C15_assert_stable_realtime` (W/eng1/lens-sound-r1/o/b2/C15_assert_stable_realtime). Oracles: iv "does not run ((no iverilog binary))"; s2v "A end"; vl "A end".
/// PRE "F t=5 | F t=15 | A end"; r1 "F t=5 | F t=15 | A end". §16.9.3 (ieee1800-2017.txt:17820-17826 $stable/$changed, :17866-17871 $past); §16.5.1 (:16792-16797) default sampled value.
#[test]
fn eng1r2_b2_c15_stable_realtime_decl_init_residue() {
    pin(
        r##"module top;
  logic clk = 0; realtime r = 0.5;
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) $stable(r)) else $display("F t=%0t", $time);
  initial begin #20 $display("A end"); $finish; end
endmodule
"##,
        &[r##"F t=5"##, r##"A end"##],
    );
}

/// B2 residue (r2 text; hand "F t=15 | A end"): first tick = Q-DSV: default sampled value is the declaration value (vita: the type default).
/// Cell `P06_prev_changed_realtime_value` (W/eng/replan-eng1/elab/o/a/P06_prev_changed_realtime_value). Oracles: iv "does not run ((no iverilog binary))"; s2v "A end"; vl "A end".
/// PRE "F t=5 | F t=15 | F t=25 | A end"; r1 "F t=5 | F t=15 | F t=25 | A end". §16.9.3 (ieee1800-2017.txt:17820-17826 $stable/$changed, :17866-17871 $past); §16.5.1 (:16792-16797) default sampled value.
#[test]
fn eng1r2_b2_p06_changed_realtime_value_residue() {
    pin(
        r##"module top;
  logic clk = 0; realtime r = 0.5;
  always #5 clk = ~clk;
  initial #12 r = 0.75;
  a1: assert property (@(posedge clk) !$changed(r)) else $display("F t=%0t", $time);
  initial begin #30 $display("A end"); $finish; end
endmodule
"##,
        &[r##"F t=5"##, r##"F t=15"##, r##"A end"##],
    );
}

/// B2 (H1 CAP_DELAY): `t = #2 s` holds the string value.
/// Cell `H01_capture_string` (W/eng1/lens-sound-r1/o/b1/H01_capture_string). Oracles: iv "S t=2 t=ab | A end"; s2v "S t=2 t=ab | A end"; vl "S t=2 t=ab | A end".
/// PRE "S t=2 t= | A end"; r1 "S t=2 t= | A end". §9.4.5 / §9.2.2 intra-assignment timing: the RHS value is captured, then written unchanged.
#[test]
fn eng1r2_cap_h01_delay_capture_of_a_string() {
    pin(
        r##"module top;
  string s = "ab", t;
  initial begin t = #2 s; $display("S t=%0t t=%s", $time, t); end
  initial #1 s = "cd";
  initial #5 begin $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=2 t=ab"##, r##"A end"##],
    );
}

/// B2 (H1 CAP_EVENT): `t = @(go) s`.
/// Cell `H17_capture_event_string` (W/eng1/lens-sound-r1/o/b2/H17_capture_event_string). Oracles: iv "S t=2 t=ab | A end"; s2v "S t=2 t=ab | A end"; vl "S t=2 t=ab | A end".
/// PRE "S t=2 t= | A end"; r1 "S t=2 t= | A end". §9.4.5 / §9.2.2 intra-assignment timing: the RHS value is captured, then written unchanged.
#[test]
fn eng1r2_cap_h17_event_capture_of_a_string() {
    pin(
        r##"module top;
  string s = "ab", t; logic go = 0;
  initial begin t = @(go) s; $display("S t=%0t t=%s", $time, t); end
  initial begin #1 s = "cd"; #1 go = 1; #2 $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=2 t=ab"##, r##"A end"##],
    );
}

/// B2 (H1 CAP_NBA_EVENT): `t <= @(go) s` (vl alone runs it).
/// Cell `C01_cap_nba_event_string` (W/eng/replan-eng1/elab/o/a/C01_cap_nba_event_string). Oracles: iv "does not run (iv.vvp:30: syntax error)"; s2v "does not run (s2v.vvp:30: syntax error)"; vl "S t=3 t=ab | A end".
/// PRE "S t=3 t= | A end"; r1 "S t=3 t= | A end". §9.4.5 / §9.2.2 intra-assignment timing: the RHS value is captured, then written unchanged.
#[test]
fn eng1r2_cap_c01_nba_event_capture_of_a_string() {
    pin(
        r##"module top;
  string s = "ab", t; logic go = 0;
  initial begin t <= @(go) s; #1 s = "cd"; #1 go = 1; #1 $display("S t=%0t t=%s", $time, t); #1 $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=3 t=ab"##, r##"A end"##],
    );
}

/// B2: `t = repeat (2) @(posedge clk) s`.
/// Cell `C02_cap_repeat_string` (W/eng/replan-eng1/elab/o/a/C02_cap_repeat_string). Oracles: iv "S t=3 t=ab | A end"; s2v "does not run ((no sv2v->iverilog binary))"; vl "does not run ((no verilator binary) | %Error-UNSUPPORTED: t.sv:4:21: Unsupported: repeat event control |)".
/// PRE "S t=3 t= | A end"; r1 "S t=3 t= | A end". §9.4.5 / §9.2.2 intra-assignment timing: the RHS value is captured, then written unchanged.
#[test]
fn eng1r2_cap_c02_repeat_event_capture_of_a_string() {
    pin(
        r##"module top;
  string s = "ab", t; logic clk = 0;
  always #1 clk = ~clk;
  initial begin t = repeat (2) @(posedge clk) s; $display("S t=%0t t=%s", $time, t); end
  initial #2 s = "cd";
  initial #9 begin $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=3 t=ab"##, r##"A end"##],
    );
}

/// B2: `t = #2 "xy"` into a `string t` -- the value assigned is a string (iv / s2v print an empty t, as they do not for `t = "xy"`).
/// Cell `C14_cap_delay_string_lit` (W/eng/replan-eng1/elab/o/b/C14_cap_delay_string_lit). Oracles: iv "S t=2 t= | A end"; s2v "S t=2 t= | A end"; vl "S t=2 t=xy | A end".
/// PRE "S t=2 t= | A end"; r1 "S t=2 t= | A end". §9.4.5 / §9.2.2 intra-assignment timing: the RHS value is captured, then written unchanged.
#[test]
fn eng1r2_cap_c14_delay_capture_of_a_literal_into_a_string() {
    pin(
        r##"module top;
  string t;
  initial begin t = #2 "xy"; $display("S t=%0t t=%s", $time, t); #1 $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=2 t=xy"##, r##"A end"##],
    );
}

/// B2: `t = #2 q[0]` of a string queue.
/// Cell `C16_cap_delay_string_qelem` (W/eng/replan-eng1/elab/o/b/C16_cap_delay_string_qelem). Oracles: iv "S t=2 t=ab | A end"; s2v "does not run ((no sv2v->iverilog binary))"; vl "S t=2 t=ab | A end".
/// PRE "S t=2 t= | A end"; r1 "S t=2 t= | A end". §9.4.5 / §9.2.2 intra-assignment timing: the RHS value is captured, then written unchanged.
#[test]
fn eng1r2_cap_c16_delay_capture_of_a_string_queue_element() {
    pin(
        r##"module top;
  string q [$]; string t;
  initial begin q.push_back("ab"); t = #2 q[0]; $display("S t=%0t t=%s", $time, t); end
  initial #1 q[0] = "cd";
  initial #5 begin $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=2 t=ab"##, r##"A end"##],
    );
}

/// B2: a 10-character string.
/// Cell `C17_cap_delay_string_long` (W/eng/replan-eng1/elab/o/b/C17_cap_delay_string_long). Oracles: iv "S t=2 t=abcdefghij len=10 | A end"; s2v "S t=2 t=abcdefghij len=10 | A end"; vl "S t=2 t=abcdefghij len=10 | A end".
/// PRE "S t=2 t= len=0 | A end"; r1 "S t=2 t= len=0 | A end". §9.4.5 / §9.2.2 intra-assignment timing: the RHS value is captured, then written unchanged.
#[test]
fn eng1r2_cap_c17_delay_capture_of_a_long_string() {
    pin(
        r##"module top;
  string s = "abcdefghij", t;
  initial begin t = #2 s; $display("S t=%0t t=%s len=%0d", $time, t, t.len()); end
  initial #1 s = "cd";
  initial #5 begin $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=2 t=abcdefghij len=10"##, r##"A end"##],
    );
}

/// Control: a string captured for an integral destination converts at the write as `t = s` does (iv / s2v refuse the implicit cast).
/// Cell `C12_cap_delay_string_int_lhs` (W/eng/replan-eng1/elab/o/a/C12_cap_delay_string_int_lhs). Oracles: iv "does not run ((no iverilog binary))"; s2v "does not run ((no sv2v->iverilog binary))"; vl "does not run ((no verilator binary) | %Error: 'make -C obj -f Vtop.mk -j 2' exited with 2 | %Error: Comm)".
/// PRE "S t=2 t=6162 | A end"; r1 "S t=2 t=6162 | A end". §9.4.5 / §9.2.2 intra-assignment timing: the RHS value is captured, then written unchanged.
#[test]
fn eng1r2_cap_c12_string_into_int_lhs_control() {
    pin(
        r##"module top;
  string s = "ab"; logic [15:0] t;
  initial begin t = #2 s; $display("S t=%0t t=%h", $time, t); end
  initial #1 s = "cd";
  initial #5 begin $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=2 t=6162"##, r##"A end"##],
    );
}

/// Control: as C12 with truncation.
/// Cell `C18_cap_delay_string_to_int_lhs_long` (W/eng/replan-eng1/elab/o/b/C18_cap_delay_string_to_int_lhs_long). Oracles: iv "does not run ((no iverilog binary))"; s2v "does not run ((no sv2v->iverilog binary))"; vl "does not run ((no verilator binary) | %Error: Internal Error: t.sv:3:24: ../V3Number.cpp:2487: Number op)".
/// PRE "S t=2 t=6768696a | A end"; r1 "S t=2 t=6768696a | A end". §9.4.5 / §9.2.2 intra-assignment timing: the RHS value is captured, then written unchanged.
#[test]
fn eng1r2_cap_c18_long_string_into_int_lhs_control() {
    pin(
        r##"module top;
  string s = "abcdefghij"; logic [31:0] t;
  initial begin t = #2 s; $display("S t=%0t t=%h", $time, t); end
  initial #5 begin $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=2 t=6768696a"##, r##"A end"##],
    );
}

/// Control: `t <= #2 s` (= vl already).
/// Cell `C13_cap_nba_delay_string` (W/eng/replan-eng1/elab/o/a/C13_cap_nba_delay_string). Oracles: iv "does not run (iv.vvp:25: syntax error)"; s2v "does not run (s2v.vvp:25: syntax error)"; vl "S t=3 t=ab | A end".
/// PRE "S t=3 t=ab | A end"; r1 "S t=3 t=ab | A end". §9.4.5 / §9.2.2 intra-assignment timing: the RHS value is captured, then written unchanged.
#[test]
fn eng1r2_cap_c13_nba_delay_string_control() {
    pin(
        r##"module top;
  string s = "ab", t;
  initial begin t <= #2 s; #1 s = "cd"; #2 $display("S t=%0t t=%s", $time, t); #1 $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=3 t=ab"##, r##"A end"##],
    );
}

/// Refusal unchanged (a named event has no value).
/// Cell `C04_cap_delay_event` (W/eng/replan-eng1/elab/o/a/C04_cap_delay_event). Oracles: iv "does not run ((no iverilog binary))"; s2v "does not run ((no sv2v->iverilog binary))"; vl "does not run ((no verilator binary) | %Error-UNSUPPORTED: t.sv:3:20: Assignment to and from event in sta)".
/// PRE "t.sv:3:17: error[VITA-E3009] E-ELAB-UNSUPPORTED: a named event has no value: it cannot be read in an expression (only `->e` and `@(e)` touch it) [in top] | t.sv:3:17: error[VITA-E3009] E-ELAB-UNSUPPORTED: a named event cannot be assigned (only `->e` triggers it) [in top]"; r1 "t.sv:3:17: error[VITA-E3009] E-ELAB-UNSUPPORTED: a named event has no value: it cannot be read in an expression (only `->e` and `@(e)` touch it) [in top] | t.sv:3:17: error[VITA-E3009] E-ELAB-UNSUPPORTED: a named event cannot be assigned (only `->e` triggers it) [in top]". §9.4.5 / §9.2.2 intra-assignment timing: the RHS value is captured, then written unchanged.
#[test]
fn eng1r2_cap_c04_event_capture_refused() {
    refused(
        r##"module top;
  event e1, e2;
  initial begin e2 = #2 e1; #1 -> e1; end
  always @(e2) $display("W t=%0t", $time);
  initial #6 begin $display("A end"); $finish; end
endmodule
"##,
        r##"t.sv:3:17: "##,
        r##"error[VITA-E3009]"##,
    );
}

/// Refusal unchanged (unpacked struct).
/// Cell `C11_cap_delay_struct_string` (W/eng/replan-eng1/elab/o/a/C11_cap_delay_struct_string). Oracles: iv "does not run ((no iverilog binary))"; s2v "does not run ((no sv2v->iverilog binary))"; vl "S t=2 s=ab b=3 | A end".
/// PRE "t.sv:4:40: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: undeclared net/variable `top.s1` [in top] | t.sv:4:40: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: undeclared net/variable `top.s2` [in top]"; r1 "t.sv:4:40: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: undeclared net/variable `top.s1` [in top] | t.sv:4:40: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: undeclared net/variable `top.s2` [in top]". §9.4.5 / §9.2.2 intra-assignment timing: the RHS value is captured, then written unchanged.
#[test]
fn eng1r2_cap_c11_struct_capture_refused() {
    refused(
        r##"module top;
  typedef struct { string s; int b; } S;
  S s1, s2;
  initial begin s1.s = "ab"; s1.b = 3; s2 = #2 s1; $display("S t=%0t s=%s b=%0d", $time, s2.s, s2.b); end
  initial #1 s1.s = "cd";
  initial #5 begin $display("A end"); $finish; end
endmodule
"##,
        r##"t.sv:4:40: "##,
        r##"error[VITA-E3010]"##,
    );
}

/// Refusal unchanged (intra-assignment delay on a string concatenation).
/// Cell `C15_cap_delay_string_concat` (W/eng/replan-eng1/elab/o/b/C15_cap_delay_string_concat). Oracles: iv "S t=2 t=abx | A end"; s2v "S t=2 t=abx | A end"; vl "S t=2 t=abx | A end".
/// PRE "t.sv:3:17: error[VITA-E3009] E-ELAB-UNSUPPORTED: an intra-assignment delay on a string concatenation is unsupported [in top]"; r1 "t.sv:3:17: error[VITA-E3009] E-ELAB-UNSUPPORTED: an intra-assignment delay on a string concatenation is unsupported [in top]". §9.4.5 / §9.2.2 intra-assignment timing: the RHS value is captured, then written unchanged.
#[test]
fn eng1r2_cap_c15_string_concat_capture_refused() {
    refused(
        r##"module top;
  string s = "ab", t;
  initial begin t = #2 {s, "x"}; $display("S t=%0t t=%s", $time, t); end
  initial #1 s = "cd";
  initial #5 begin $display("A end"); $finish; end
endmodule
"##,
        r##"t.sv:3:17: "##,
        r##"error[VITA-E3009]"##,
    );
}

/// B2 (H1 QPAT): the snapshot of a string queue element is a string (iv "Q cd cd" disqualified by its int twin H24i).
/// Cell `H24_string_queue_pattern_self` (W/eng1/lens-sound-r1/o/b7/H24_string_queue_pattern_self). Oracles: iv "Q cd cd"; s2v "does not run ((no sv2v->iverilog binary))"; vl "Q cd ab".
/// PRE "Q  "; r1 "Q  ". §7.10 / §10.10: the RHS is evaluated before the assignment.
#[test]
fn eng1r2_qpat_h24_string_queue_self_pattern() {
    pin(
        r##"module top;
  string q [$];
  initial begin q.push_back("ab"); q.push_back("cd"); q = '{q[1], q[0]}; $display("Q %s %s", q[0], q[1]); $finish; end
endmodule
"##,
        &[r##"Q cd ab"##],
    );
}

/// B2 (QPAT): an element that concatenates.
/// Cell `Q05_qpat_string_concat` (W/eng/replan-eng1/elab/o/b/Q05_qpat_string_concat). Oracles: iv "Q cd cdx"; s2v "does not run ((no sv2v->iverilog binary))"; vl "Q cd abx".
/// PRE "Q  "; r1 "Q  ". §7.10 / §10.10: the RHS is evaluated before the assignment.
#[test]
fn eng1r2_qpat_q05_string_queue_self_pattern_concat() {
    pin(
        r##"module top;
  string q [$];
  initial begin q.push_back("ab"); q.push_back("cd"); q = '{q[1], {q[0], "x"}}; $display("Q %s %s", q[0], q[1]); $finish; end
endmodule
"##,
        &[r##"Q cd abx"##],
    );
}

/// B2 (QPAT): the pattern is longer than the queue.
/// Cell `Q06_qpat_string_size` (W/eng/replan-eng1/elab/o/b/Q06_qpat_string_size). Oracles: iv "Q n=3 ab zz ab"; s2v "does not run ((no sv2v->iverilog binary))"; vl "Q n=3 ab zz ab".
/// PRE "Q n=3   "; r1 "Q n=3   ". §7.10 / §10.10: the RHS is evaluated before the assignment.
#[test]
fn eng1r2_qpat_q06_string_queue_self_pattern_grows() {
    pin(
        r##"module top;
  string q [$];
  initial begin q.push_back("ab"); q = '{q[0], "zz", q[0]}; $display("Q n=%0d %s %s %s", q.size(), q[0], q[1], q[2]); $finish; end
endmodule
"##,
        &[r##"Q n=3 ab zz ab"##],
    );
}

/// Control (int twin = vl).
/// Cell `H24i_int_queue_pattern_self` (W/eng1/lens-sound-r1/o/b7/H24i_int_queue_pattern_self). Oracles: iv "Q 2 2"; s2v "does not run ((no sv2v->iverilog binary))"; vl "Q 2 1".
/// PRE "Q 2 1"; r1 "Q 2 1". §7.10 / §10.10: the RHS is evaluated before the assignment.
#[test]
fn eng1r2_qpat_h24i_int_queue_control() {
    pin(
        r##"module top;
  int q [$];
  initial begin q.push_back(1); q.push_back(2); q = '{q[1], q[0]}; $display("Q %0d %0d", q[0], q[1]); $finish; end
endmodule
"##,
        &[r##"Q 2 1"##],
    );
}

/// B2 (H1 SVA_PREV): `$stable` of a string starting at "", hand (vl refuses a string).
/// Cell `SV06_stable_string_default` (W/eng1/r2/o/new/SV06_stable_string_default). Oracles: iv "does not run ((no iverilog binary))"; s2v "A end"; vl "does not run ((no verilator binary) | %Error: t.sv:4:39: Expected numeric type, but got a 'string' data )".
/// PRE "F t=5 | F t=15 | F t=25 | A end"; r1 "F t=5 | F t=15 | F t=25 | A end". §16.9.3 (ieee1800-2017.txt:17820-17826 $stable/$changed, :17866-17871 $past); §16.5.1 (:16792-16797) default sampled value.
#[test]
fn eng1r2_sva_sv06_stable_string_type_default() {
    pin(
        r##"module top;
  logic clk = 0; string s = "";
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) $stable(s)) else $display("F t=%0t", $time);
  initial begin #12 s = "cd"; #20 $display("A end"); $finish; end
endmodule
"##,
        &[r##"F t=15"##, r##"A end"##],
    );
}

/// B2: `!$changed` of a string, hand.
/// Cell `SV07_changed_string_default` (W/eng1/r2/o/new/SV07_changed_string_default). Oracles: iv "does not run ((no iverilog binary))"; s2v "A end"; vl "does not run ((no verilator binary) | %Error: t.sv:4:40: Expected numeric type, but got a 'string' data )".
/// PRE "F t=5 | F t=15 | F t=25 | A end"; r1 "F t=5 | F t=15 | F t=25 | A end". §16.9.3 (ieee1800-2017.txt:17820-17826 $stable/$changed, :17866-17871 $past); §16.5.1 (:16792-16797) default sampled value.
#[test]
fn eng1r2_sva_sv07_changed_string_type_default() {
    pin(
        r##"module top;
  logic clk = 0; string s = "";
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) !$changed(s)) else $display("F t=%0t", $time);
  initial begin #12 s = "cd"; #20 $display("A end"); $finish; end
endmodule
"##,
        &[r##"F t=15"##, r##"A end"##],
    );
}

/// B2: `$past` of a string, hand.
/// Cell `SV08_past_string_default` (W/eng1/r2/o/new/SV08_past_string_default). Oracles: iv "does not run ((no iverilog binary))"; s2v "A end"; vl "does not run ((no verilator binary) | %Error: t.sv:4:87: Expected numeric type, but got a 'string' data )".
/// PRE "F t=5 past=  s= | F t=15 past=  s=cd | F t=25 past=  s=cd | A end"; r1 "F t=5 past=  s= | F t=15 past=  s=cd | F t=25 past=  s=cd | A end". §16.9.3 (ieee1800-2017.txt:17820-17826 $stable/$changed, :17866-17871 $past); §16.5.1 (:16792-16797) default sampled value.
#[test]
fn eng1r2_sva_sv08_past_string_type_default() {
    pin(
        r##"module top;
  logic clk = 0; string s = "";
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) 0) else $display("F t=%0t past=%s s=%s", $time, $past(s), s);
  initial begin #12 s = "cd"; #20 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"F t=5 past= s="##,
            r##"F t=15 past= s=cd"##,
            r##"F t=25 past=cd s=cd"##,
            r##"A end"##,
        ],
    );
}

/// B2 residue (r2 text; hand "A end"): first tick = Q-DSV: default sampled value is the declaration value (vita: the type default).
/// Cell `P01_prev_stable_string` (W/eng/replan-eng1/elab/o/a/P01_prev_stable_string). Oracles: iv "does not run ((no iverilog binary))"; s2v "A end"; vl "does not run ((no verilator binary) | %Error: t.sv:4:39: Expected numeric type, but got a 'string' data )".
/// PRE "F t=5 | F t=15 | F t=25 | A end"; r1 "F t=5 | F t=15 | F t=25 | A end". §16.9.3 (ieee1800-2017.txt:17820-17826 $stable/$changed, :17866-17871 $past); §16.5.1 (:16792-16797) default sampled value.
#[test]
fn eng1r2_sva_p01_stable_string_decl_init_residue() {
    pin(
        r##"module top;
  logic clk = 0; string s = "ab";
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) $stable(s)) else $display("F t=%0t", $time);
  initial begin #30 $display("A end"); $finish; end
endmodule
"##,
        &[r##"F t=5"##, r##"A end"##],
    );
}

/// B2 residue (r2 text; hand has past=ab at t=5): first tick = Q-DSV: default sampled value is the declaration value (vita: the type default).
/// Cell `P05_prev_past_string` (W/eng/replan-eng1/elab/o/a/P05_prev_past_string). Oracles: iv "does not run ((no iverilog binary))"; s2v "A end"; vl "does not run ((no verilator binary) | %Error: t.sv:5:87: Expected numeric type, but got a 'string' data )".
/// PRE "F t=5 past=  s=ab | F t=15 past=  s=cd | F t=25 past=  s=cd | A end"; r1 "F t=5 past=  s=ab | F t=15 past=  s=cd | F t=25 past=  s=cd | A end". §16.9.3 (ieee1800-2017.txt:17820-17826 $stable/$changed, :17866-17871 $past); §16.5.1 (:16792-16797) default sampled value.
#[test]
fn eng1r2_sva_p05_past_string_decl_init_residue() {
    pin(
        r##"module top;
  logic clk = 0; string s = "ab";
  always #5 clk = ~clk;
  initial #12 s = "cd";
  a1: assert property (@(posedge clk) 0) else $display("F t=%0t past=%s s=%s", $time, $past(s), s);
  initial begin #30 $display("A end"); $finish; end
endmodule
"##,
        &[
            r##"F t=5 past= s=ab"##,
            r##"F t=15 past=ab s=cd"##,
            r##"F t=25 past=cd s=cd"##,
            r##"A end"##,
        ],
    );
}

/// B2 (H3): a clocking input of a string is refused by name (r1 read it blank; vl "cb.s=ab").
/// Cell `H11_clocking_string` (W/eng1/lens-sound-r1/o/b1/H11_clocking_string). Oracles: iv "does not run ((no iverilog binary))"; s2v "does not run ((no sv2v->iverilog binary))"; vl "S t=2 cb.s=ab | A end".
/// PRE "S t=2 cb.s=  | A end"; r1 "S t=2 cb.s=  | A end". §14.3 / §14.16; refused by name at the first use of `cb.x` (ENG-1b C6; the lift is Q-CLKHEAP).
#[test]
fn eng1r2_clk_h11_string_input_refused() {
    refused(
        r##"module top;
  logic clk = 0; string s = "ab";
  clocking cb @(posedge clk); input s; endclocking
  initial begin #1 clk = 1; #1 $display("S t=%0t cb.s=%s", $time, cb.s); #1 $display("A end"); $finish; end
endmodule
"##,
        r##"t.sv:4:32:"##,
        r##"clocking input `cb.s` samples `s`, a string: vita's clocking sample holds one integral or `real` value, not this type (follow-on)"##,
    );
}

/// B2 (H3): a clocking output of a string (r1 left s=ab; vl s=xy).
/// Cell `H16_clocking_out_string` (W/eng1/lens-sound-r1/o/b2/H16_clocking_out_string). Oracles: iv "does not run ((no iverilog binary))"; s2v "does not run ((no sv2v->iverilog binary))"; vl "S t=3 s=xy | A end".
/// PRE "S t=3 s=ab | A end"; r1 "S t=3 s=ab | A end". §14.3 / §14.16; refused by name at the first use of `cb.x` (ENG-1b C6; the lift is Q-CLKHEAP).
#[test]
fn eng1r2_clk_h16_string_output_refused() {
    refused(
        r##"module top;
  logic clk = 0; string s = "ab";
  clocking cb @(posedge clk); output s; endclocking
  initial begin #1 cb.s <= "xy"; #1 clk = 1; #1 $display("S t=%0t s=%s", $time, s); #1 $display("A end"); $finish; end
endmodule
"##,
        r##"t.sv:4:20:"##,
        r##"clocking output `cb.s` drives `s`, a string: vita's clocking drive holds one integral or `real` value, not this type (follow-on)"##,
    );
}

/// B2 (H3): a queue input (r1 native v=0, vm / interp W4020 v=x; vl v=4).
/// Cell `K02_clk_in_queue` (W/eng/replan-eng1/elab/o/a/K02_clk_in_queue). Oracles: iv "does not run ((no iverilog binary))"; s2v "does not run ((no sv2v->iverilog binary))"; vl "S t=2 v=4 | A end".
/// PRE "S t=2 v=0 | A end"; r1 "S t=2 v=0 | A end". §14.3 / §14.16; refused by name at the first use of `cb.x` (ENG-1b C6; the lift is Q-CLKHEAP).
#[test]
fn eng1r2_clk_k02_queue_input_refused() {
    refused(
        r##"module top;
  logic clk = 0; int q [$];
  clocking cb @(posedge clk); input q; endclocking
  initial begin q.push_back(4); #1 clk = 1; #1 $display("S t=%0t v=%0d", $time, cb.q[0]); #1 $display("A end"); $finish; end
endmodule
"##,
        r##"t.sv:4:81:"##,
        r##"clocking input `cb.q` samples `q`, a queue: vita's clocking sample holds one integral or `real` value, not this type (follow-on)"##,
    );
}

/// B2 (H3): a string queue input.
/// Cell `K02c_clk_in_string_queue` (W/eng/replan-eng1/elab/o/b/K02c_clk_in_string_queue). Oracles: iv "does not run ((no iverilog binary))"; s2v "does not run ((no sv2v->iverilog binary))"; vl "S t=2 v=ab | A end".
/// PRE "S t=2 v=  | A end"; r1 "S t=2 v=  | A end". §14.3 / §14.16; refused by name at the first use of `cb.x` (ENG-1b C6; the lift is Q-CLKHEAP).
#[test]
fn eng1r2_clk_k02c_string_queue_input_refused() {
    refused(
        r##"module top;
  logic clk = 0; string q [$];
  clocking cb @(posedge clk); input q; endclocking
  initial begin q.push_back("ab"); #1 clk = 1; #1 $display("S t=%0t v=%s", $time, cb.q[0]); #1 $display("A end"); $finish; end
endmodule
"##,
        r##"t.sv:4:83:"##,
        r##"clocking input `cb.q` samples `q`, a queue: vita's clocking sample holds one integral or `real` value, not this type (follow-on)"##,
    );
}

/// B2 (H3): a dynamic array input.
/// Cell `K02d_clk_in_dyn` (W/eng/replan-eng1/elab/o/b/K02d_clk_in_dyn). Oracles: iv "does not run ((no iverilog binary))"; s2v "does not run ((no sv2v->iverilog binary))"; vl "S t=2 v=4 | A end".
/// PRE "S t=2 v=0 | A end"; r1 "S t=2 v=0 | A end". §14.3 / §14.16; refused by name at the first use of `cb.x` (ENG-1b C6; the lift is Q-CLKHEAP).
#[test]
fn eng1r2_clk_k02d_dyn_array_input_refused() {
    refused(
        r##"module top;
  logic clk = 0; int d [];
  clocking cb @(posedge clk); input d; endclocking
  initial begin d = new[2]; d[0] = 4; #1 clk = 1; #1 $display("S t=%0t v=%0d", $time, cb.d[0]); #1 $display("A end"); $finish; end
endmodule
"##,
        r##"t.sv:4:87:"##,
        r##"clocking input `cb.d` samples `d`, a dynamic array: vita's clocking sample holds one integral or `real` value, not this type (follow-on)"##,
    );
}

/// B2 (H3): an associative array input.
/// Cell `K02e_clk_in_assoc` (W/eng/replan-eng1/elab/o/b/K02e_clk_in_assoc). Oracles: iv "does not run ((no iverilog binary))"; s2v "does not run ((no sv2v->iverilog binary))"; vl "S t=2 v=4 | A end".
/// PRE "S t=2 v=0 | A end"; r1 "S t=2 v=0 | A end". §14.3 / §14.16; refused by name at the first use of `cb.x` (ENG-1b C6; the lift is Q-CLKHEAP).
#[test]
fn eng1r2_clk_k02e_assoc_input_refused() {
    refused(
        r##"module top;
  logic clk = 0; int a [int];
  clocking cb @(posedge clk); input a; endclocking
  initial begin a[3] = 4; #1 clk = 1; #1 $display("S t=%0t v=%0d", $time, cb.a[3]); #1 $display("A end"); $finish; end
endmodule
"##,
        r##"t.sv:4:75:"##,
        r##"clocking input `cb.a` samples `a`, an associative array: vita's clocking sample holds one integral or `real` value, not this type (follow-on)"##,
    );
}

/// B2 (H3): a queue output, used by `cb.q <= q`: ONE error for the one use (the write is refused before its
/// right-hand side lowers; PRE printed only the whole-handle read of `q`).
/// Cell `K04_clk_out_queue` (W/eng/replan-eng1/elab/o/a/K04_clk_out_queue). Oracles: iv "does not run ((no iverilog binary))"; s2v "does not run ((no sv2v->iverilog binary))"; vl "does not run ((no verilator binary) | %Error-UNSUPPORTED: t.sv:3:31: Unsupported: Cannot detect changes )".
/// PRE "t.sv:4:36: error[VITA-E3009] E-ELAB-UNSUPPORTED: a dynamic-storage handle has no whole-value surface (read elements or call methods) [in top]"; r1 "t.sv:4:36: error[VITA-E3009] E-ELAB-UNSUPPORTED: a dynamic-storage handle has no whole-value surface (read elements or call methods) [in top]". §14.3 / §14.16; refused by name at the first use of `cb.x` (ENG-1b C6; the lift is Q-CLKHEAP).
#[test]
fn eng1r2_clk_k04_queue_output_refused() {
    refused_once(
        r##"module top;
  logic clk = 0; int q [$];
  clocking cb @(posedge clk); output q; endclocking
  initial begin q.push_back(4); #1 cb.q <= q; #1 clk = 1; #1 $display("S t=%0t n=%0d", $time, q.size()); #1 $display("A end"); $finish; end
endmodule
"##,
        r##"t.sv:4:36:"##,
        r##"clocking output `cb.q` drives `q`, a queue: vita's clocking drive holds one integral or `real` value, not this type (follow-on)"##,
    );
}

/// B2 (H3) / ENG-1b C6: a string output never written gets no holder and drives nothing, so the design runs
/// (r2's declaration refusal was right->loud). vl "S t=2 s=ab | A end"; iv / s2v do not run clocking blocks.
/// Cell `K08s_clk_out_string_unwritten`. §14.3: an item no statement uses is never sampled or driven.
#[test]
fn eng1r2_clk_k08s_string_output_unused_runs() {
    pin(
        r##"module top;
  logic clk = 0; string s = "ab";
  clocking cb @(posedge clk); output s; endclocking
  initial begin #1 clk = 1; #1 $display("S t=%0t s=%s", $time, s); #1 $display("A end"); $finish; end
endmodule
"##,
        &[r##"S t=2 s=ab"##, r##"A end"##],
    );
}

// ───────────────────────────── ENG-1b C1-C7 witnesses ─────────────────────────────
// Cells under W/eng/eng1b/probe/a2 (src2 / src3 / src4); the declarations the probe header shares
// with every cell of its family are kept so the text is the measured one.

const BR_HDR: &str = r##"package p; parameter string PS = "pk"; parameter real PR = 2.25; endpackage
module top;
  import p::*;
  parameter string SP = "sp"; parameter real RP = 1.25; localparam real LR = -0.5;
  string s1 = "ab", s2 = "cd"; string srcq [$] = '{"e0", "e1"};
  real x = 2.5, xm = -2.5; real srca [0:1] = '{0.5, 0.75};
  int i = -5; logic signed [7:0] s8 = -3; logic [7:0] b = 200, c = 100; logic [15:0] v = 16'h7879;
  bit sel = 1; event go; integer sd1 = 5, sd2 = 5;
  typedef struct { real r; string s; int k; } st_t;
  function string fs(input int k); fs = (k == 1) ? "fx" : "fy"; endfunction
  function automatic string fa(input int k); return (k == 1) ? "ax" : "ay"; endfunction
  function real fr(input int k); fr = k * 0.5; endfunction
  function automatic real far(input int k); return k * 0.25; endfunction
"##;

fn br(body: &str) -> String {
    format!("{BR_HDR}{body}")
}

/// C1 (mutant M16): a `real` destination's capture holds the converted value. `r = #1 i`, `int i = -5`:
/// vl "C v=-5.000000 | D v=-5.000000"; vita's own direct `d = i` "-5.000000". PRE "C v=18446744073709551616.000000"
/// (an unsigned 32-bit holder, then the conversion). iv / s2v cannot parse the shared header. Cell `BR_whole_int_dly`.
#[test]
fn c1_real_destination_capture_of_a_signed_int() {
    pin_staged(
        &br(r##"  real r; real d;
  initial begin
    r = #1 i;
    d = i;
    #1 $display("C v=%f", r);
    $display("D v=%f", d);
    $display("A end"); $finish;
  end
endmodule
"##),
        &["C v=-5.000000", "D v=-5.000000", "A end"],
    );
}

/// C1: the capture is evaluated in the DESTINATION's context — `r = #1 b + c` with 8-bit `b = 200`,
/// `c = 100` is `44` (§11.8.2: a real result leaves an integral operand self-determined), as vl and the
/// direct `d = b + c`. PRE "C v=300.000000" (a holder at the lvalue width computed in 64 bits). Cell `BR_whole_ctx_dly`.
#[test]
fn c1_real_destination_capture_keeps_the_assignment_context() {
    pin(
        &br(r##"  real r; real d;
  initial begin
    r = #1 b + c;
    d = b + c;
    #1 $display("C v=%f", r);
    $display("D v=%f", d);
    $display("A end"); $finish;
  end
endmodule
"##),
        &["C v=44.000000", "D v=44.000000", "A end"],
    );
}

/// C1 / C2 (mutant N1): a string-queue ELEMENT destination holds the string. vl "C v=ab | D v=ab".
/// PRE "C v=" (an integral holder; the ternary of strings had no expression type). Cell `BS_qelem_tern_dly`.
#[test]
fn c1_string_queue_element_capture_of_a_ternary() {
    pin(
        &br(
            r##"  string q [$] = '{"q0", "q1"}; string dq [$] = '{"q0", "q1"};
  initial begin
    q[1] = #1 sel ? s1 : s2;
    dq[1] = sel ? s1 : s2;
    #1 $display("C v=%s", q[1]);
    $display("D v=%s", dq[1]);
    $display("A end"); $finish;
  end
endmodule
"##,
        ),
        &["C v=ab", "D v=ab", "A end"],
    );
}

/// C1 (mutant N1): a fixed string-array element destination and a literal. vl "C v=lit | D v=lit". PRE "C v=".
/// Cell `BS_fixelem_lit_dly`.
#[test]
fn c1_string_array_element_capture_of_a_literal() {
    pin(
        &br(
            r##"  string fa1 [0:1] = '{"f0", "f1"}; string dfa [0:1] = '{"f0", "f1"};
  initial begin
    fa1[1] = #1 "lit";
    dfa[1] = "lit";
    #1 $display("C v=%s", fa1[1]);
    $display("D v=%s", dfa[1]);
    $display("A end"); $finish;
  end
endmodule
"##,
        ),
        &["C v=lit", "D v=lit", "A end"],
    );
}

/// C1 (mutant N8): the `$random` / `$dist_*` / `$cast` intercept captures into the destination's type too.
/// vl "C v=-10.000000 | D v=-10.000000". PRE "C v=18446744073709551616.000000". iv / s2v reject the intra-assignment
/// form here (`syntax error`). Cell `BX_real_dist`.
#[test]
fn c1_intercept_capture_into_a_real() {
    pin(
        r##"module top;
  integer sd1 = 5, sd2 = 5; real r, d;
  initial begin
    r = #1 $dist_uniform(sd1, -10, -5);
    d = $dist_uniform(sd2, -10, -5);
    #1 $display("C v=%f", r);
    $display("D v=%f", d);
    $display("A end"); $finish;
  end
endmodule
"##,
        &["C v=-10.000000", "D v=-10.000000", "A end"],
    );
}

/// C3 (mutant N6): a hierarchical `real` destination is recorded at the capture and retyped when the write
/// resolves. vl "C v=1.500000 | D v=1.500000". PRE "C v=2.000000" (an integral holder rounded the 1.5).
/// Cell `HX_cap_real_hier_realrhs`.
#[test]
fn c3_hierarchical_real_destination_capture() {
    pin_staged(
        r##"module child; real r; real d; endmodule
module top;
  child u();
  real x = 1.5;
  initial begin
    u.r = #1 x;
    u.d = x;
    #1 $display("C v=%f", u.r);
    $display("D v=%f", u.d);
    $display("A end"); $finish;
  end
endmodule
"##,
        &["C v=1.500000", "D v=1.500000", "A end"],
    );
}

/// C3 (mutant N6): a hierarchical `string` destination. vl "C v=ab | D v=ab". PRE "C v=". Cell `HX_cap_string_hier`.
#[test]
fn c3_hierarchical_string_destination_capture() {
    pin(
        r##"module child; string s; string d; endmodule
module top;
  child u();
  bit sel = 1; string s1 = "ab", s2 = "cd";
  initial begin
    u.s = #1 (sel ? s1 : s2);
    u.d = (sel ? s1 : s2);
    #1 $display("C v=%s", u.s);
    $display("D v=%s", u.d);
    $display("A end"); $finish;
  end
endmodule
"##,
        &["C v=ab", "D v=ab", "A end"],
    );
}

/// C4 (mutant N2): `$past` of a signed `int` is signed. vl "V t=5 past=-5 | V t=15 past=-5 | V t=25 past=-7 | ...",
/// no `N` line. PRE "past=4294967291" and `$past(i) < 0` false (N t=15..45). The first tick ("past=x", "N t=5") is the
/// Q-DSV residue (§16.5.1: the declaration value -5; vita's register starts at the type default). Cell `SA_signed_int_past`.
#[test]
fn c4_past_of_a_signed_int_is_signed() {
    pin(
        r##"module top;
  int i = -5;
  logic clk = 0;
  always #5 clk = ~clk;
  a3: assert property (@(posedge clk) 1) $display("V t=%0t past=%0d", $time, $past(i));
  a4: assert property (@(posedge clk) $past(i) < 0) else $display("N t=%0t", $time);
  initial begin #12 i = -7; #40 $display("A end"); $finish; end
endmodule
"##,
        &[
            "V t=5 past=x",
            "N t=5",
            "V t=15 past=-5",
            "V t=25 past=-7",
            "V t=35 past=-7",
            "V t=45 past=-7",
            "A end",
        ],
    );
}

/// C5 (mutant N3): a constant has no register; its sampled value, and its past, is itself (§16.5.1). Hand:
/// no `F` line, `pp=1.5 lp=-0.5` (vl rounds them to 2.0 / -1.0: disqualified). PRE "F t=5..35", `pp=0`.
/// Cell `SA_param_real_past`.
#[test]
fn c5_past_of_a_real_parameter_is_the_constant() {
    pin(
        r##"module top;
  parameter real P = 1.5; localparam real LP = -0.5;
  logic clk = 0;
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) $stable(P) && $stable(LP)) else $display("F t=%0t", $time);
  a3: assert property (@(posedge clk) 1) $display("V t=%0t pp=%f lp=%f", $time, $past(P), $past(LP));
  initial begin  #40 $display("A end"); $finish; end
endmodule
"##,
        &[
            "V t=5 pp=1.500000 lp=-0.500000",
            "V t=15 pp=1.500000 lp=-0.500000",
            "V t=25 pp=1.500000 lp=-0.500000",
            "V t=35 pp=1.500000 lp=-0.500000",
            "A end",
        ],
    );
}

/// C5 (mutant N3): an enum label. vl "V t=5 pe=2 | ... | A end", no `F`. PRE "F t=5 | V t=5 pe=x | F t=15 | V t=15 pe=0 ...".
/// Cell `SA_enum_label_past`.
#[test]
fn c5_past_of_an_enum_label_is_the_constant() {
    pin(
        r##"module top;
  typedef enum logic [1:0] {E0, E1, E2} e_t;
  logic clk = 0;
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) $stable(E2)) else $display("F t=%0t", $time);
  a3: assert property (@(posedge clk) 1) $display("V t=%0t pe=%0d", $time, $past(E2));
  initial begin  #40 $display("A end"); $finish; end
endmodule
"##,
        &[
            "V t=5 pe=2",
            "V t=15 pe=2",
            "V t=25 pe=2",
            "V t=35 pe=2",
            "A end",
        ],
    );
}

/// C6 (mutant N4): an unpacked-array clocking input gets no holder; its use is refused by name at the use.
/// vl "S t=2 v=22"; PRE "S t=2 v=0" (a scalar holder at the element width, never sampled right). The lift is
/// Q-CLKHEAP. Cell `CK_arr_int_in`.
#[test]
fn c6_unpacked_array_clocking_input_refused_at_use() {
    refused_once(
        r##"module top;
  logic clk = 0; logic [7:0] m [0:1] = '{8'h11, 8'h22};
  clocking cb @(posedge clk); input m; endclocking
  initial begin #1 clk = 1; #1 $display("S t=%0t v=%h", $time, cb.m[1]); #1 $display("A end"); $finish; end
endmodule
"##,
        "t.sv:4:64:",
        "clocking input `cb.m` samples `m`, an unpacked array: vita's clocking sample holds one integral or `real` value, not this type (follow-on)",
    );
}

/// C6: an UNUSED queue item is harmless and all three backends agree. vl "S t=2 a=3 | A end". PRE split
/// the backends here (native `v=0`, vm / interp W4020) where the item was read; unread it ran. Cell `CK_queue_unread_only`.
#[test]
fn c6_unused_queue_clocking_input_runs() {
    pin_quiet(
        r##"module top;
  logic clk = 0; int q [$]; int a = 3;
  clocking cb @(posedge clk); input a; input q; endclocking
  initial begin #1 clk = 1; #1 q.push_back(1); $display("S t=%0t a=%0d", $time, cb.a); #1 $display("A end"); $finish; end
endmodule
"##,
        &["S t=2 a=3", "A end"],
    );
}

/// C6: an unused byte-queue item beside a used integral one (R2-5's right->loud). vl "S t=2 a=3 n=1 | A end".
/// Cell `K10_clk_in_byte_queue_unread`.
#[test]
fn c6_unused_byte_queue_clocking_input_runs() {
    pin_quiet(
        r##"module top;
  logic clk = 0; byte q [$]; int a = 3;
  clocking cb @(posedge clk); input a; input q; endclocking
  initial begin q.push_back(1); #1 clk = 1; #1 $display("S t=%0t a=%0d n=%0d", $time, cb.a, q.size()); #1 $display("A end"); $finish; end
endmodule
"##,
        &["S t=2 a=3 n=1", "A end"],
    );
}

/// C7 (mutant N7): `$rose` / `$fell` of a string is refused by name. PRE ran it on `s[0]`: "F t=5 | R t=15 | R t=25 |
/// F t=25"; vl refuses ("Expected numeric type, but got a 'string' data type"); s2v drops the assertions: no oracle
/// for a value, so loud. Cell `S4b_rose_fell_string`.
#[test]
fn c7_rose_fell_of_a_string_refused_by_name() {
    let src = r##"module top;
  logic clk = 0; string s = "";
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) !$rose(s)) else $display("R t=%0t", $time);
  a2: assert property (@(posedge clk) !$fell(s)) else $display("F t=%0t", $time);
  initial begin #12 s = "a"; #10 s = "b"; #10 s = ""; #10 $display("A end"); $finish; end
endmodule
"##;
    refused(
        src,
        "t.sv:4:46:",
        "`$rose` of `s`, a `string`, is unsupported: `$rose` / `$fell` test the LSB (§16.9.3), and a `string` has none",
    );
    refused(
        src,
        "t.sv:5:46:",
        "`$fell` of `s`, a `string`, is unsupported: `$rose` / `$fell` test the LSB (§16.9.3), and a `string` has none",
    );
}

/// C7: `$rose` of a `real` is refused by name ONCE. PRE printed "bit/part-select not defined on real operand" twice
/// (the register's bit 0 and the signal's). vl "F t=15" (it rounds the real), no other oracle. Cell `S12_rose_real`.
#[test]
fn c7_rose_of_a_real_refused_once() {
    refused_once(
        r##"module top;
  logic clk = 0; real r = 0.0;
  always #5 clk = ~clk;
  a1: assert property (@(posedge clk) !$rose(r)) else $display("F t=%0t", $time);
  initial begin #12 r = 1.0; #20 $display("A end"); $finish; end
endmodule
"##,
        "t.sv:4:46:",
        "`$rose` of `r`, a `real`, is unsupported: `$rose` / `$fell` test the LSB (§16.9.3), and a `real` has none",
    );
}

/// `src` is refused on every backend with exactly these `(at, diag)` lines and no other error.
fn refused_exactly(src: &str, want: &[(&str, &str)]) {
    let (at, diag) = want[0];
    for all in refused(src, at, diag) {
        for (at, diag) in want {
            assert!(
                all.lines().any(|l| l.starts_with(at) && l.contains(diag)),
                "no `{at}…{diag}` line\n{all}"
            );
        }
        let n = format!("errors={} ", want.len());
        assert!(all.lines().any(|l| l.starts_with(&n)), "not {n}\n{all}");
    }
}

// ── r2: the SVA register is typed from what the EXPRESSION RESOLVER binds the argument to (lens B1) ──

/// r2 F1 (mutant M28): a generate-local `localparam N` shadows a module net `N`; `prev <= N` stores the
/// constant, so the register is the constant's, not the outer `logic signed [7:0]` net's (r1 copied its
/// signedness: 240 read back -16). §16.5.1: the sampled value of a constant is the constant (240 > 100).
/// vl "A end", s2v "A end"; PRE "A end"; r1 "F t=15 p=-16 | F t=25 p=-16 | F t=35 p=-16 | A end". Cell `SH_outer_signed`.
#[test]
fn r2_past_of_a_local_constant_shadowing_a_signed_net() {
    pin(
        r##"module top;
  reg clk = 0; always #5 clk = ~clk;
  initial begin #42 $display("A end"); $finish; end
  logic signed [7:0] N = 0;
  if (1) begin : g
    localparam logic [7:0] N = 8'hF0;
    a1: assert property (@(posedge clk) $past(N) > 100 || $time < 10) else $display("F t=%0t p=%0d", $time, $past(N));
  end
endmodule
"##,
        &["A end"],
    );
}

/// r2 F1 (mutant M28): the outer `N` is an unpacked array, the argument binds the local constant — no
/// refusal. vl "A end", s2v "A end"; PRE "A end"; r1 "`$past` of `N`, an unpacked array, is unsupported". Cell `SH_outer_arr`.
#[test]
fn r2_past_of_a_local_constant_shadowing_an_unpacked_array() {
    pin(
        r##"module top;
  reg clk = 0; always #5 clk = ~clk;
  initial begin #42 $display("A end"); $finish; end
  logic [7:0] N [2];
  if (1) begin : g
    localparam int N = 5;
    a1: assert property (@(posedge clk) $past(N) == 5 || $time < 10) else $display("F t=%0t", $time);
  end
endmodule
"##,
        &["A end"],
    );
}

/// r2 F1 (mutant M28): the outer `N` is a queue. vl "A end" (s2v cannot parse `[$]`); PRE "A end"; r1 "`$past`
/// of `N`, a queue, is unsupported". Cell `SH_outer_queue`.
#[test]
fn r2_past_of_a_local_constant_shadowing_a_queue() {
    pin(
        r##"module top;
  reg clk = 0; always #5 clk = ~clk;
  initial begin #42 $display("A end"); $finish; end
  int N [$];
  if (1) begin : g
    localparam int N = 5;
    a1: assert property (@(posedge clk) $past(N) == 5 || $time < 10) else $display("F t=%0t", $time);
  end
endmodule
"##,
        &["A end"],
    );
}

/// r2 F1 (mutant M28): the outer net is NARROWER than the local constant (4 bits vs 20); PRE and r1 truncated
/// the register to 4 bits ("F t=15 | F t=25 | F t=35"). vl "A end", s2v "A end". Cell `SH_outer_narrow`.
#[test]
fn r2_past_of_a_local_constant_shadowing_a_narrow_net() {
    pin(
        r##"module top;
  reg clk = 0; always #5 clk = ~clk;
  initial begin #42 $display("A end"); $finish; end
  logic [3:0] N = 0;
  if (1) begin : g
    localparam int N = 20;
    a1: assert property (@(posedge clk) $past(N) == 20 || $time < 10) else $display("F t=%0t", $time);
  end
endmodule
"##,
        &["A end"],
    );
}

/// r2 F1 (mutant M28): the first tick reads the constant too (PRE and r1 "F t=5 n=x": an `x` register at the
/// outer net's width). vl "A end", s2v "A end". Cell `SC_shadow_gen`.
#[test]
fn r2_past_of_a_local_constant_first_tick() {
    pin(
        r##"module top;
  reg clk = 0; always #5 clk = ~clk;
  initial begin #42 $display("A end"); $finish; end
  logic [3:0] N = 4'd2;
  if (1) begin : g
    localparam int N = 5;
    a1: assert property (@(posedge clk) $past(N) == 5) else $display("F t=%0t n=%0d", $time, $past(N));
  end
endmodule
"##,
        &["A end"],
    );
}

/// r2 F1 (mutant M29): a `let` is neither a variable nor a constant; its register keeps PRE's typing (exclude,
/// never refuse). §16.5.1: the sampled value of any other expression is built from its operands' sampled values.
/// vl "A end"; PRE "A end"; r1 "`$rose` of `L` is unsupported: it is neither a variable nor a constant here" (and
/// `$past`). Cell `LT_let_rose`.
#[test]
fn r2_rose_and_past_of_a_let() {
    pin(
        r##"module top;
  reg clk = 0; always #5 clk = ~clk;
  initial begin #42 $display("A end"); $finish; end
  logic a = 0, b = 1; always @(posedge clk) a <= ~a;
  let L = a & b;
  a1: assert property (@(posedge clk) $rose(L) |-> a) else $display("F t=%0t", $time);
  a2: assert property (@(posedge clk) $past(L) != L || $time < 10) else $display("G t=%0t", $time);
endmodule
"##,
        &["A end"],
    );
}

// ── r2: `$rose` / `$fell` of a real or string CONSTANT is refused by name, once (lens N1, C7) ──

/// r2 F2 (mutant M30): `parameter real`. PRE one generic "bit/part-select not defined on real operand", r1 two of
/// them. vl "A end", s2v "A end" (a value; C7 refuses by name). Cells `SC_rose_realparam`, `SV_param_real_rose`.
#[test]
fn r2_rose_of_a_real_parameter_refused_once() {
    refused_once(
        r##"module top;
  logic clk = 0;
  always #5 clk = ~clk;
  parameter real PR = 1.5;
  a1: assert property (@(posedge clk) !$rose(PR)) else $display("R t=%0t", $time);
  initial begin  #40 $display("A end"); $finish; end
endmodule
"##,
        "t.sv:5:46:",
        "`$rose` of `PR`, a `real`, is unsupported: `$rose` / `$fell` test the LSB (§16.9.3), and a `real` has none",
    );
}

/// r2 F2 (mutant M30): `localparam real` under `$fell`. PRE 1 / r1 2 generic lines; vl "A end", s2v "A end".
/// Cell `SV_param_real_fell`.
#[test]
fn r2_fell_of_a_real_localparam_refused_once() {
    refused_once(
        r##"module top;
  logic clk = 0;
  always #5 clk = ~clk;
  localparam real LR = 2.0;
  a1: assert property (@(posedge clk) !$fell(LR)) else $display("F t=%0t", $time);
  initial begin  #40 $display("A end"); $finish; end
endmodule
"##,
        "t.sv:5:46:",
        "`$fell` of `LR`, a `real`, is unsupported: `$rose` / `$fell` test the LSB (§16.9.3), and a `real` has none",
    );
}

/// r2 F2 (mutant M30): a real constant and a real variable in one property: one by-name line each (PRE 2 generic
/// lines, r1 the variable's by name + 2 generic). Cell `SV_param_real_rose_named`.
#[test]
fn r2_rose_of_a_real_parameter_and_variable_each_named() {
    refused_exactly(
        r##"module top;
  logic clk = 0;
  always #5 clk = ~clk;
  parameter real PR = 1.5; real r = 0.0;
  a1: assert property (@(posedge clk) !$rose(PR) && !$rose(r)) else $display("R t=%0t", $time);
  initial begin  #40 $display("A end"); $finish; end
endmodule
"##,
        &[
            (
                "t.sv:5:46:",
                "`$rose` of `PR`, a `real`, is unsupported: `$rose` / `$fell` test the LSB (§16.9.3), and a `real` has none",
            ),
            (
                "t.sv:5:60:",
                "`$rose` of `r`, a `real`, is unsupported: `$rose` / `$fell` test the LSB (§16.9.3), and a `real` has none",
            ),
        ],
    );
}

/// r2 F2 (mutant M30): `parameter string` under `$rose` and `$fell`. PRE "G t=5" (bit 0 of the bytes), r1 "A end"
/// with no refusal; vl refuses ("Expected numeric type, but got a 'string' data type"), s2v "A end": no oracle for a
/// value, so loud by name like a string variable. Cells `SR_rose_strparam`, `SV_param_str_rose`.
#[test]
fn r2_rose_fell_of_a_string_parameter_refused_by_name() {
    refused_exactly(
        r##"module top;
  reg clk = 0; always #5 clk = ~clk;
  initial begin #42 $display("A end"); $finish; end
  parameter string SP = "ab";
  a1: assert property (@(posedge clk) !$rose(SP)) else $display("F t=%0t", $time);
  a2: assert property (@(posedge clk) !$fell(SP)) else $display("G t=%0t", $time);
endmodule
"##,
        &[
            (
                "t.sv:5:46:",
                "`$rose` of `SP`, a `string`, is unsupported: `$rose` / `$fell` test the LSB (§16.9.3), and a `string` has none",
            ),
            (
                "t.sv:6:46:",
                "`$fell` of `SP`, a `string`, is unsupported: `$rose` / `$fell` test the LSB (§16.9.3), and a `string` has none",
            ),
        ],
    );
}

// ── r2: a use of a refused clocking item inside a function / generate block, or in a continuous
//    assignment, is refused by name at the use (lens N2, C6) ──

/// r2 F3 (mutant M31): `cb.s` read in a function finds the item with the walk `lookup_net_scoped` uses (r1: the
/// generic E3010 "undeclared hierarchical name `cb.s`"). vl "S t=3 v=ab"; PRE "S t=3 v=" (empty). Cell `CL_func_str`.
#[test]
fn r2_clocking_string_item_used_in_a_function_refused_by_name() {
    refused_once(
        r##"module top;
  reg clk = 0; always #1 clk = ~clk;
  string s = "ab";
  clocking cb @(posedge clk); input s; endclocking
  function automatic string f(); return cb.s; endfunction
  initial begin @(cb); @(cb); $display("S t=%0t v=%s", $time, f()); $display("A end"); $finish; end
endmodule
"##,
        "t.sv:5:34:",
        "clocking input `cb.s` samples `s`, a string: vita's clocking sample holds one integral or `real` value, not this type (follow-on)",
    );
}

/// r2 F3 (mutant M31): `cb.s` read in a generate block. vl "S v=ab"; PRE "S v= "; r1 generic E3010. Cell `CL_gen_use`.
#[test]
fn r2_clocking_string_item_used_in_a_generate_block_refused_by_name() {
    refused_once(
        r##"module top;
  reg clk = 0; always #1 clk = ~clk;
  string s = "ab";
  clocking cb @(posedge clk); input s; endclocking
  if (1) begin : g
    initial begin @(cb); @(cb); $display("S v=%s", cb.s); $display("A end"); $finish; end
  end
endmodule
"##,
        "t.sv:6:33:",
        "clocking input `cb.s` samples `s`, a string: vita's clocking sample holds one integral or `real` value, not this type (follow-on)",
    );
}

/// r2 F3 (mutant M32): a select of an unpacked-array item in a continuous assignment is refused at the select (r1:
/// at the module header, t.sv:1:8). vl "S w=22"; PRE "S w=1". Cell `CL_contassign_arr`.
#[test]
fn r2_clocking_array_item_in_a_continuous_assignment_refused_at_the_use() {
    refused_once(
        r##"module top;
  reg clk = 0; always #1 clk = ~clk;
  logic [7:0] m [2] = '{8'd11, 8'd22};
  wire [7:0] w;
  clocking cb @(posedge clk); input m; endclocking
  assign w = cb.m[1];
  initial begin #4 $display("S w=%0d", w); $display("A end"); $finish; end
endmodule
"##,
        "t.sv:6:14:",
        "clocking input `cb.m` samples `m`, an unpacked array: vita's clocking sample holds one integral or `real` value, not this type (follow-on)",
    );
}

/// `src` on `backend` with `--obs-dir`: its stdout lines and run.json's `codegen.total`, the
/// number of processes the design lowered to.
fn run_obs_processes(src: &str, backend: &str) -> (Vec<String>, u64) {
    let d = scratch(backend);
    std::fs::write(d.join("t.sv"), src).unwrap();
    let (out, err, code) = vita(&d, &["--backend", backend, "--obs-dir", "obs", "t.sv"]);
    let json = std::fs::read_to_string(d.join("obs").join("run.json")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(code, 0, "{backend}: exit {code}\n{out}{err}");
    let total = json
        .split("\"codegen\"")
        .nth(1)
        .and_then(|t| t.split("\"total\":").nth(1))
        .and_then(|t| t.trim_start().split(|c: char| !c.is_ascii_digit()).next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("{backend}: no codegen.total in run.json\n{json}"));
    (lines(&out), total)
}

/// C6 (mutant N5): an unused queue / string clocking INPUT is never sampled — no holder, no commit pair, so a
/// clocking block of only such items lowers no commit process. The holder N5 restored is a `$ia_tmp$` net the VCD
/// drops and nothing reads, so stdout and the VCD cannot see it; run.json's `codegen.total` (the process count)
/// can: the heap-only block lowers as many processes as the design without it, while the positive control, an
/// unused `int` item (a holder and a commit pair), lowers one more.
#[test]
fn c6_unused_heap_clocking_input_lowers_no_commit_process() {
    let design = |items: &str| {
        format!(
            r##"module top;
  logic clk = 0; int q [$]; string s = "ab"; int a = 3;
{items}  initial begin #1 clk = 1; #1 q.push_back(1); #1 clk = 0; #1 clk = 1; #1 $display("A end"); $finish; end
endmodule
"##
        )
    };
    let none = design("");
    let heap = design("  clocking cb @(posedge clk); input q; input s; endclocking\n");
    let int = design("  clocking cb @(posedge clk); input a; endclocking\n");
    for be in ["native", "vm", "interp"] {
        let (out_none, p_none) = run_obs_processes(&none, be);
        let (out_heap, p_heap) = run_obs_processes(&heap, be);
        let (out_int, p_int) = run_obs_processes(&int, be);
        for out in [&out_none, &out_heap, &out_int] {
            assert_eq!(out, &["A end"], "backend {be}");
        }
        assert_eq!(
            p_int,
            p_none + 1,
            "{be}: the used-type control lowers its commit process"
        );
        assert_eq!(
            p_heap, p_none,
            "{be}: an unused heap clocking input lowered a commit process"
        );
    }
}
