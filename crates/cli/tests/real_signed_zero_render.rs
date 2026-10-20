//! Q-FMTNEGZ (§4.5.619): a real holding IEEE 754 negative zero renders with its sign, pinned on
//! native, vm and interp.
//!
//! Every renderer prints the stored bits the way C's printf does: `%f %e %g %E %G %F`, a width,
//! a precision, `-`, `0` and `+`, `%t` with and without `$timeformat`, `$sformatf`, `$write`,
//! `$monitor`, an argument with no format (`%g`), `%p`, and the VCD value change (`r-0`, the
//! shared `vcd_writer::fmt_g`). PRE normalised -0.0 to +0.0 in `fmt_real`, the `%t` real path
//! and `fmt_g`, printing `0.000000` / `0` / `r0` and, under `+`, `+0.000000`.
//!
//! Oracles: Icarus Verilog 13.0 `-g2012` (iv), sv2v 0.0.13 -> iverilog (s2v), Verilator 5.050
//! `--binary` (vl). All three render the bits: where their `$realtobits` is 8000000000000000
//! they print the sign. They split on the VALUE of a constant -0.0: vl stores -0.0 for a
//! literal `-0.0`, `-(0.0)`, `-5.0*0.0`, `0.0 * -1`, a `parameter real P = -0.0` and
//! `$ceil(-0.3)` (IEEE 754: negation flips the sign, -5*0 is -0, ceil(-0.3) is -0), and so
//! does vita; iv and s2v store +0.0 there, but iv contradicts itself on the same literal
//! (`$realtobits(-0.0)` is 0 while `1.0/(-0.0)` folds to -inf and `$atan2(-0.0, -1.0)` to
//! −π), so it is no oracle on that axis (ER §7.3). iv also negates a runtime +0.0 to +0.0
//! (`r = -z`, its negation is 0 - x), the ROADMAP §0 iverilog defect ⑩.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn scratch(tag: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_rsz_{tag}_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// `src` on one backend: its stdout lines (no `simulation ended` line) and the text of
/// `out.vcd` when the design dumps one, after a clean exit.
fn run_on(src: &str, backend: &str) -> (Vec<String>, Option<String>) {
    let d = scratch(backend);
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["--backend", backend, "t.sv"])
        .current_dir(&d)
        .output()
        .expect("run vita");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let code = out.status.code().unwrap_or(-1);
    let vcd = std::fs::read_to_string(d.join("out.vcd")).ok();
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(code, 0, "{backend}: exit {code}\n{stdout}{stderr}");
    let lines = stdout
        .lines()
        .filter(|l| !l.starts_with("simulation ended"))
        .map(str::to_string)
        .collect();
    (lines, vcd)
}

/// Every backend prints exactly `want`.
fn pin(src: &str, want: &[&str]) {
    for be in ["native", "vm", "interp"] {
        assert_eq!(run_on(src, be).0, want, "backend {be}");
    }
}

/// Every spec, width, precision and flag on a runtime -0.0 (`$bitstoreal` of the sign bit):
/// iv = s2v = vl, line for line, except `%E` (iv = s2v `-0.000000E+00`; vl lowercases the
/// exponent letter for every value, a vl defect vita does not follow).
/// Cell `Z01_every_spec`.
#[test]
fn z01_every_spec_keeps_the_sign() {
    pin(
        "module top;\n\
  real r;\n\
  initial begin\n\
    r = $bitstoreal(64'h8000000000000000);\n\
    $display(\"f=%f e=%e g=%g f1=%0.1f w=[%10.3f] l=[%-8.2f] z=%010.2f E=%E G=%G F=%F\", r, r, r, r, r, r, r, r, r, r);\n\
    $display(\"%.0f|%.0e|%.3g|%5.1e|%-6.0f|\", r, r, r, r, r);\n\
    $display(\"s=%s\", $sformatf(\"%e\", r));\n\
    $write(\"w=%f\\n\", r);\n\
  end\n\
endmodule\n",
        &[
            "f=-0.000000 e=-0.000000e+00 g=-0 f1=-0.0 w=[    -0.000] l=[-0.00   ] z=-000000.00 E=-0.000000E+00 G=-0 F=-0.000000",
            "-0|-0e+00|-0|-0.0e+00|-0    |",
            "s=-0.000000e+00",
            "w=-0.000000",
        ],
    );
}

/// Each runtime producer of -0.0 and three of +0.0 (IEEE 754: -0 + +0 is +0, -(-0) is +0):
/// iv = vl on every line, bits included — the bits are the input, the text follows them
/// (sv2v cannot parse `real d[]`).
/// Cell `Z02_runtime_producers`.
#[test]
fn z02_runtime_producers_and_positive_controls() {
    pin(
        "module top;\n\
  real r, z, m, n, u, inf;\n\
  real d[];\n\
  task show(input [63:0] tag, input real v);\n\
    $display(\"%0s bits=%h f=%f g=%g\", tag, $realtobits(v), v, v);\n\
  endtask\n\
  initial begin\n\
    z = 0.0; m = -1.0; n = 1.0e-10; inf = 1.0e308 * 10.0;\n\
    r = m * z;                 show(\"mul\", r);\n\
    r = z / m;                 show(\"div\", r);\n\
    r = m / inf;               show(\"divinf\", r);\n\
    u = -1.0e-300; r = u * n * 1.0e-100; show(\"under\", r);\n\
    r = m * z; r = r - z;      show(\"sub\", r);\n\
    d = new[2]; d[1] = m * z;  show(\"heap\", d[1]);\n\
    show(\"sin\", $sin(m * z));\n\
    show(\"ceil\", $ceil(m * 0.3));\n\
    r = m * z; r = r + z;      show(\"add_p\", r);\n\
    r = m * z; r = -r;         show(\"neg_p\", r);\n\
    r = 0.0;                   show(\"zero_p\", r);\n\
  end\n\
endmodule\n",
        &[
            "mul bits=8000000000000000 f=-0.000000 g=-0",
            "div bits=8000000000000000 f=-0.000000 g=-0",
            "divinf bits=8000000000000000 f=-0.000000 g=-0",
            "under bits=8000000000000000 f=-0.000000 g=-0",
            "sub bits=8000000000000000 f=-0.000000 g=-0",
            "heap bits=8000000000000000 f=-0.000000 g=-0",
            "sin bits=8000000000000000 f=-0.000000 g=-0",
            "ceil bits=8000000000000000 f=-0.000000 g=-0",
            "add_p bits=0000000000000000 f=0.000000 g=0",
            "neg_p bits=0000000000000000 f=0.000000 g=0",
            "zero_p bits=0000000000000000 f=0.000000 g=0",
        ],
    );
}

/// A constant -0.0 and a runtime negation of +0.0: vl, line for line (bits -0, sign printed),
/// with IEEE 754 negation and `ceil`; iv and s2v print `bits=0000000000000000 f=0.000000 g=0`
/// on each, a value they do not hold consistently (module doc). vita's bits equal vl's at
/// PRE; only the text moves.
/// Cell `Z03_constant_producers`.
#[test]
fn z03_constant_negative_zero_follows_its_bits() {
    pin(
        "module top;\n\
  parameter real P = -0.0;\n\
  real r, z;\n\
  task show(input [63:0] tag, input real v);\n\
    $display(\"%0s bits=%h f=%f g=%g\", tag, $realtobits(v), v, v);\n\
  endtask\n\
  initial begin\n\
    z = 0.0;\n\
    r = -0.0;        show(\"lit\", r);\n\
    r = -z;          show(\"neg\", r);\n\
    r = -5.0 * 0.0;  show(\"cmul\", r);\n\
    r = 0.0 * -1;    show(\"cmuli\", r);\n\
    r = P;           show(\"par\", r);\n\
    show(\"ceil\", $ceil(-0.3));\n\
    $display(\"direct f=%f e=%e g=%g\", -0.0, -0.0, -0.0);\n\
  end\n\
endmodule\n",
        &[
            "lit bits=8000000000000000 f=-0.000000 g=-0",
            "neg bits=8000000000000000 f=-0.000000 g=-0",
            "cmul bits=8000000000000000 f=-0.000000 g=-0",
            "cmuli bits=8000000000000000 f=-0.000000 g=-0",
            "par bits=8000000000000000 f=-0.000000 g=-0",
            "ceil bits=8000000000000000 f=-0.000000 g=-0",
            "direct f=-0.000000 e=-0.000000e+00 g=-0",
        ],
    );
}

/// `%t` of a real -0.0, at the default format and under `$timeformat`: iv = s2v = vl.
/// Cell `Z04_time_spec`.
#[test]
fn z04_time_spec_keeps_the_sign() {
    pin(
        "module top;\n\
  real r;\n\
  initial begin\n\
    r = $bitstoreal(64'h8000000000000000);\n\
    $display(\"t=[%t]\", r);\n\
    $timeformat(-9, 2, \" ns\", 12);\n\
    $display(\"t=[%t]\", r);\n\
  end\n\
endmodule\n",
        &["t=[                  -0]", "t=[    -0.00 ns]"],
    );
}

/// `+` on -0.0 prints the minus, not a plus: iv = s2v `-0.000000|-0.000000e+00|-0` (vl
/// refuses `%+`; C's printf agrees). PRE printed `+0.000000|+0.000000e+00|+0`: the flag saw
/// a body without `-`. The +0.0 control keeps its `+` (iv = s2v `+0.000000`).
/// Cell `Z05_plus_flag`.
#[test]
fn z05_plus_flag_on_negative_zero() {
    pin(
        "module top;\n\
  real r;\n\
  initial begin\n\
    r = $bitstoreal(64'h8000000000000000);\n\
    $display(\"%+f|%+e|%+g\", r, r, r);\n\
    r = 0.0;\n\
    $display(\"%+f\", r);\n\
  end\n\
endmodule\n",
        &["-0.000000|-0.000000e+00|-0", "+0.000000"],
    );
}

/// An argument with no format and `%p`: vl `-0|-0` and `P -0`. iv and s2v refuse this cell
/// (`$display does not support argument type (vpiMemory)`); without the array they spell a
/// format-free real `-0.00000|-0.00000` (iverilog's own default, which vita does not follow
/// for any value) and print `%p` of a real as `<%p>` (`unknown format`). The element of a
/// real array under `%p` is the scalar's spelling (vl prints `-0` for the whole array), so
/// that line is vita's own consistency, not an oracle's.
/// Cell `Z06_default_and_pattern`.
#[test]
fn z06_format_free_argument_and_pattern() {
    pin(
        "module top;\n\
  real r;\n\
  real a [2];\n\
  initial begin\n\
    r = $bitstoreal(64'h8000000000000000);\n\
    a[0] = r; a[1] = 1.5;\n\
    $display(\"D\", r, \"|\", r);\n\
    $display(\"P %p\", r);\n\
    $display(\"PA %p\", a);\n\
  end\n\
endmodule\n",
        &["D-0|-0", "P -0", "PA '{-0, 1.5}"],
    );
}

/// The VCD value change of a real -0.0 is `r-0`, of +0.0 `r0`: iv = s2v write `#1 r-0 !`,
/// `#2 r0 !`, `#3 r-0 !`; vl writes `#1 r-0` and nothing after (its change test is IEEE 754
/// `==`, under which -0.0 and +0.0 are equal; vita records each bits change, §4.5.618).
/// PRE wrote `r0` three times, three identical values for three different bit patterns.
/// Cell `Z07_vcd_value_change`.
#[test]
fn z07_vcd_writes_the_sign() {
    let src = "module top;\n\
  real r;\n\
  initial begin\n\
    $dumpfile(\"out.vcd\"); $dumpvars(0, top);\n\
    r = 1.5;\n\
    #1 r = $bitstoreal(64'h8000000000000000);\n\
    #1 r = 0.0;\n\
    #1 r = $bitstoreal(64'h8000000000000000);\n\
    #1 $finish;\n\
  end\n\
endmodule\n";
    for be in ["native", "vm", "interp"] {
        let vcd = run_on(src, be).1.expect("out.vcd written");
        let body = vcd
            .split_once("$enddefinitions $end")
            .expect("VCD header")
            .1;
        let changes: Vec<&str> = body
            .lines()
            .filter(|l| l.starts_with('#') || l.starts_with('r'))
            .skip_while(|l| *l != "#1")
            .collect();
        assert_eq!(
            changes,
            ["#1", "r-0 !", "#2", "r0 !", "#3", "r-0 !"],
            "backend {be}\n{vcd}"
        );
    }
}

/// Unchanged neighbours. `%d` / `%0d` of -0.0 rounds to the integer 0 (vl `0`; iv = s2v `-0`,
/// a recorded split §4.5.619 did not touch). A NaN with its sign bit set prints `nan` where
/// iv = s2v = vl print `-nan` — a residue pinned at its observed value (ROADMAP §5.2
/// Q-FMTNEGNAN), not this slice's.
/// Cell `Z08_unchanged_neighbours`.
#[test]
fn z08_integer_spec_and_nan_sign_are_unchanged() {
    pin(
        "module top;\n\
  real r;\n\
  initial begin\n\
    r = $bitstoreal(64'h8000000000000000);\n\
    $display(\"d=[%d] 0d=[%0d]\", r, r);\n\
    $display(\"nan=%f\", $bitstoreal(64'hfff8000000000000));\n\
  end\n\
endmodule\n",
        &["d=[0] 0d=[0]", "nan=nan"],
    );
}
