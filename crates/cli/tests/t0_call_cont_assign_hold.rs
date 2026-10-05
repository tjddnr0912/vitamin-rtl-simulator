//! A continuous assign that reaches an effectful call runs it at time 0 once, after the
//! first time-0 batch has written its inputs (ROADMAP §2 🆕 AB, §4.5.590;
//! `sim-engine/src/sched/t0_hold.rs`).
//!
//! Before this slice the time-0 settle evaluated every continuous assign before any
//! process ran, so `assign y = f(a, b);` called `f` on `x` first: one extra `$display`
//! line, a `unique case` miss reported at time 0, an `assert` failing with exit 1, a
//! `$fatal` ending the run. Both tools call `f` once at time 0, on the written inputs.
//! Held: an assign whose rhs or lhs index reaches a user function that is not
//! effect-free (a system task reachable, or a body the dependency walk declines), or
//! that reads a class handle — minus an assign certified with an empty read set, which
//! nothing a time-0 process writes can reach — plus every assign reading a net a held
//! assign drives. Released by the first settle after the first batch, in dependency
//! waves.
//!
//! Oracles: iverilog 13.0 (`-g2012`, `vvp -n`) and verilator 5.052 (`--binary --timing
//! --assert`, `+verilator+error+limit+1000`), quoted verbatim above each pin; lines a
//! tool prints after `$finish` are left out. verilator is no oracle on x/z or on time-0
//! order (it is 2-state, and it runs a continuous assign before or after an `initial`
//! depending on its arguments), and it re-evaluates an assign twice per later step, so
//! only its time-0 call count is compared. Every design runs on `--backend native`,
//! `interp` and `vm`, which must agree on stdout, exit code and every W4031 / error /
//! fatal line; one also runs through `vita vcmp` / `velab` / `vrun`.
//!
//! The value a process reads from a held net before the release (`i0` lines) is a
//! time-0 race: iverilog reads the net's `z` default, verilator the assign's value, and
//! each tool answers both ways in other designs (ROADMAP §2 Oracle splits). Those lines
//! are pinned as vita's measured output, marked as race pins.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Run {
    out: String,
    err: String,
    code: i32,
}

impl Run {
    /// The lines the backends must agree on: W4031 and every error / fatal.
    fn verdict(&self) -> Vec<&str> {
        self.err
            .lines()
            .filter(|l| l.contains("VITA-W4031") || l.contains("error[") || l.contains("fatal["))
            .collect()
    }

    /// How many stderr lines contain `needle`.
    fn count(&self, needle: &str) -> usize {
        self.err.lines().filter(|l| l.contains(needle)).count()
    }
}

fn vita(dir: &Path, args: &[&str]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run vita");
    Run {
        out: String::from_utf8_lossy(&out.stdout).into_owned(),
        err: String::from_utf8_lossy(&out.stderr).into_owned(),
        code: out.status.code().unwrap_or(-1),
    }
}

/// A fresh directory holding `src` saved as `name`.
fn scratch(name: &str, src: &str) -> PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_t0_hold_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(name), src).unwrap();
    d
}

/// Runs `src` on the three backends; they must agree on stdout, the exit code and the
/// W4031 / error / fatal lines. Returns the native run.
fn run(name: &str, src: &str) -> Run {
    let d = scratch(name, src);
    let mut runs = ["native", "interp", "vm"].map(|be| {
        let vcd = format!("{be}.vcd");
        (be, vita(&d, &["--backend", be, "-o", &vcd, name]))
    });
    let _ = std::fs::remove_dir_all(&d);
    let (head, rest) = runs.split_at_mut(1);
    let n = &head[0].1;
    for (be, r) in rest.iter() {
        assert_eq!(
            (r.code, r.out.as_str(), r.verdict()),
            (n.code, n.out.as_str(), n.verdict()),
            "{name}: --backend {be} differs from native\n{be} stderr:\n{}\nnative stderr:\n{}",
            r.err,
            n.err
        );
    }
    let [(_, native), _, _] = runs;
    native
}

/// `vita vcmp` → `velab` → `vrun` on `src`.
fn run_staged(name: &str, src: &str) -> Run {
    let d = scratch(name, src);
    let stem = name.trim_end_matches(".sv");
    let (vu, velab) = (format!("{stem}.vu"), format!("{stem}.velab"));
    let c = vita(&d, &["vcmp", "-o", &vu, name]);
    assert_eq!(c.code, 0, "vcmp\n{}", c.err);
    let e = vita(&d, &["velab", "-o", &velab, &vu]);
    assert_eq!(e.code, 0, "velab\n{}", e.err);
    let r = vita(&d, &["vrun", "-o", "s.vcd", &velab]);
    let _ = std::fs::remove_dir_all(&d);
    r
}

fn w4031(loc: &str, scope: &str, t: u32) -> String {
    format!(
        "{loc}: warning[VITA-W4031] W-RUN-UNIQUE-VIOLATION: value is unhandled for \
         priority or unique case statement [in {scope}] [at time {t}]"
    )
}

/// Every `VITA-W4031` line, in order.
fn w4031_lines(r: &Run) -> Vec<&str> {
    r.err.lines().filter(|l| l.contains("VITA-W4031")).collect()
}

fn assert_ok(r: &Run, out: &str) {
    assert_eq!((r.code, r.out.as_str()), (0, out), "stderr:\n{}", r.err);
}

const K1: &str = r#"module dut(input logic a, input logic b, output logic [1:0] y);
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  assign y = f(a, b);
endmodule
module top;
  logic a, b; logic [1:0] y;
  dut u(.a(a), .b(b), .y(y));
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 b = 0;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#;

/// iverilog:
/// ```text
/// f t=0 x=0 z=1
/// t=1 y=1
/// f t=2 x=0 z=0
/// t=3 y=0
/// ```
/// verilator prints `f t=0 x=0 z=1` once at time 0 too. Before the slice: `f t=0 x=x
/// z=x` first. Also through the staged `vcmp` / `velab` / `vrun` path.
#[test]
fn a_display_in_the_called_function_prints_once_at_time_0() {
    let want =
        "f t=0 x=0 z=1\nt=1 y=1\nf t=2 x=0 z=0\nt=3 y=0\nsimulation ended (Finish) at time 4\n";
    assert_ok(&run("k1_ca_display.sv", K1), want);
    let s = run_staged("k1_ca_display.sv", K1);
    assert_ok(&s, want);
}

/// iverilog and verilator, the same two lines:
/// ```text
/// t=1 y=1
/// t=3 y=1
/// ```
/// Before the slice: `error[VITA-E4003] … Assertion failed [in top.u.f] [at time 0]`
/// from the call on `x`, and exit 1.
#[test]
fn an_assert_on_inputs_written_at_time_0_does_not_fail() {
    let r = run(
        "k2b_ca_assert_clean.sv",
        r#"module dut(input logic a, input logic b, output logic [1:0] y);
  function logic [1:0] f(input logic x, input logic z);
    assert (x || z);
    return {x, z};
  endfunction
  assign y = f(a, b);
endmodule
module top;
  logic a, b; logic [1:0] y;
  dut u(.a(a), .b(b), .y(y));
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 b = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "t=1 y=1\nt=3 y=1\nsimulation ended (Finish) at time 4\n",
    );
    assert_eq!(r.count("VITA-E4003"), 0, "{}", r.err);
}

/// iverilog, both spellings (`tbF`: testbench first, `tbL`: last):
/// ```text
/// t=1 y=2
/// WARNING: q1caf_case_tbL.sv:4: value is unhandled for priority or unique case statement
///          Time: 2  Scope: top.u.f
/// t=3 y=0
/// ```
/// verilator reports the miss at 2 and nothing at 0. Before the slice: one more W4031
/// `[at time 0]`, from the call on `x`.
#[test]
fn a_unique_case_on_written_inputs_reports_only_its_real_miss() {
    let dut = r#"module dut(input logic a, input logic b, output logic [1:0] y);
  function logic [1:0] f(input logic x, input logic z);
    f = 0;
    unique case ({x, z}) 2'b10: f = 1; 2'b01: f = 2; endcase
  endfunction
  assign y = f(a, b);
endmodule
"#;
    let tb = r#"module top;
  logic a, b; logic [1:0] y;
  dut u(.a(a), .b(b), .y(y));
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 b = 0;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#;
    let out = "t=1 y=2\nt=3 y=0\nsimulation ended (Finish) at time 4\n";
    let r = run("q1caf_case_tbL.sv", &format!("{dut}{tb}"));
    assert_ok(&r, out);
    assert_eq!(
        w4031_lines(&r),
        [w4031("q1caf_case_tbL.sv:4:12", "top.u.f", 2)]
    );
    let r = run("q1caf_case_tbF.sv", &format!("{tb}{dut}"));
    assert_ok(&r, out);
    assert_eq!(
        w4031_lines(&r),
        [w4031("q1caf_case_tbF.sv:16:12", "top.u.f", 2)]
    );
}

/// iverilog and verilator: `f t=0 x=1 z=1` once, then `t=1 y=11`. Before the slice
/// three calls at time 0: on `x`, on the declaration initializers (`x=0 z=1`), and on
/// the `initial`'s write. A release before the first batch would still see `x=0`.
#[test]
fn a_declaration_initializer_the_initial_overwrites_is_not_seen() {
    let r = run(
        "d1_declinit_overwrite.sv",
        r#"module top;
  logic a = 0, b = 1; logic [1:0] y;
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  assign y = f(a, b);
  initial begin
    a = 1;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "f t=0 x=1 z=1\nt=1 y=11\nsimulation ended (Finish) at time 2\n",
    );
}

/// iverilog and verilator print `t=1 y=01` and no report, for the direct call (`d2`) and
/// through a port (`d3`): the declaration initializers miss the `unique case`, the
/// `initial` fixes the inputs in the first batch. Before the slice two W4031 `[at time
/// 0]` each, on `x` and on the initializers.
#[test]
fn declaration_initializers_the_initial_fixes_report_nothing() {
    let f = r#"  function logic [1:0] f(input logic x, input logic z);
    f = 0;
    unique case ({x, z}) 2'b01: f = 1; 2'b10: f = 2; endcase
  endfunction
"#;
    let body = r#"  initial begin
    b = 1;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#;
    let out = "t=1 y=01\nsimulation ended (Finish) at time 2\n";
    let d2 = format!(
        "module top;\n  logic a = 0, b = 0; logic [1:0] y;\n{f}  assign y = f(a, b);\n{body}"
    );
    let r = run("d2_declinit_unique.sv", &d2);
    assert_ok(&r, out);
    assert_eq!(r.count("VITA-W4031"), 0, "{}", r.err);
    let d3 = format!(
        "module dut(input logic a, input logic b, output logic [1:0] y);\n{f}  assign y = \
         f(a, b);\nendmodule\nmodule top;\n  logic a = 0, b = 0; logic [1:0] y;\n  dut \
         u(.a(a), .b(b), .y(y));\n{body}"
    );
    let r = run("d3_declinit_unique_port.sv", &d3);
    assert_ok(&r, out);
    assert_eq!(r.count("VITA-W4031"), 0, "{}", r.err);
}

const STAY: &str = r#"  function logic [1:0] f(input logic x, input logic z);
    f = 0;
    unique case ({x, z}) 2'b01: f = 1; 2'b10: f = 2; endcase
  endfunction
  assign y = f(a, b);
"#;

/// A miss the time-0 processes leave in place still reports at time 0 — the call is
/// moved, not muted. Declaration initializers, no `initial` write (`d4`); iverilog:
/// ```text
/// WARNING: d4_declinit_unique_stay.sv:5: value is unhandled for priority or unique case statement
///          Time: 0  Scope: top.f
/// t=1 y=00
/// ```
/// verilator: `[0] %Error: d4_declinit_unique_stay.sv:5: Assertion failed in top.f:
/// unique case, but none matched for '2'h0'`. Before the slice: two W4031 at 0.
/// Inputs never written (`j1`), written by an NBA (`a11`), by an `always` at 1 (`a10`)
/// or after `#0` (`a12`): iverilog `WARNING: … Time: 0` once in each, and vita one
/// W4031 `[at time 0]` each, before and after the slice. A hold that muted the call
/// instead of moving it would lose all of them.
#[test]
fn a_miss_that_stays_reports_once_at_time_0() {
    let d4 = format!(
        "module top;\n  logic a = 0, b = 0; logic [1:0] y;\n{STAY}  initial begin\n    #1 \
         $display(\"t=%0t y=%b\", $time, y);\n    #1 $finish;\n  end\n  initial #100 \
         $finish;\nendmodule\n"
    );
    let r = run("d4_declinit_unique_stay.sv", &d4);
    assert_ok(&r, "t=1 y=00\nsimulation ended (Finish) at time 2\n");
    assert_eq!(
        w4031_lines(&r),
        [w4031("d4_declinit_unique_stay.sv:5:12", "top.f", 0)]
    );
    let cases = [
        (
            "j1_unique_stayx.sv",
            "",
            "",
            "t=1 y=00\nsimulation ended (Finish) at time 2\n",
        ),
        (
            "a11_nba_t0.sv",
            "",
            "    a <= 0; b <= 1;\n",
            "t=1 y=01\nsimulation ended (Finish) at time 2\n",
        ),
        (
            "a12_delay0_writer.sv",
            "",
            "    #0 a = 0; b = 1;\n",
            "t=1 y=01\nsimulation ended (Finish) at time 2\n",
        ),
    ];
    for (name, decl, first, out) in cases {
        let src = format!(
            "module top;\n  logic a, b; logic [1:0] y;{decl}\n{STAY}  initial begin\n{first}    \
             #1 $display(\"t=%0t y=%b\", $time, y);\n    #1 $finish;\n  end\n  initial #100 \
             $finish;\nendmodule\n"
        );
        let r = run(name, &src);
        assert_ok(&r, out);
        assert_eq!(
            w4031_lines(&r),
            [w4031(&format!("{name}:5:12"), "top.f", 0)]
        );
    }
    let a10 = format!(
        "module top;\n  logic clk = 0, a, b; logic [1:0] y;\n{STAY}  always @(posedge clk) \
         begin a <= 0; b <= 1; end\n  initial begin\n    #1 clk = 1;\n    #1 \
         $display(\"t=%0t y=%b\", $time, y);\n    #1 $finish;\n  end\n  initial #100 \
         $finish;\nendmodule\n"
    );
    let r = run("a10_alwaysblk_writer.sv", &a10);
    assert_ok(&r, "t=2 y=01\nsimulation ended (Finish) at time 3\n");
    assert_eq!(
        w4031_lines(&r),
        [w4031("a10_alwaysblk_writer.sv:5:12", "top.f", 0)]
    );
}

/// iverilog:
/// ```text
/// f t=0 x=1
/// V t=0 v=0
/// NV t=0 v=0
/// t=1 w=1 v=0
/// ```
/// verilator `f t=0 x=1`, `V t=0 v=0` (no z). `v` reads the held `w`, so it is held too:
/// settled on `w`'s `z` default it went to 1 and the first batch saw a posedge (`PV t=0
/// v=0`) iverilog never fires. Before the slice: `f t=0 x=x` first, events as iverilog.
#[test]
fn an_assign_below_a_held_one_waits_with_it() {
    let r = run(
        "x2b_hold_window_z_edges.sv",
        r#"module top;
  logic a; wire w; wire v;
  function logic f(input logic x);
    $display("f t=%0t x=%b", $time, x);
    return x;
  endfunction
  assign w = f(a);
  assign v = (w === 1'bz);
  always @(v) $display("V t=%0t v=%b", $time, v);
  always @(posedge v) $display("PV t=%0t v=%b", $time, v);
  always @(negedge v) $display("NV t=%0t v=%b", $time, v);
  initial begin
    a = 1;
    #1 $display("t=%0t w=%b v=%b", $time, w, v);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "f t=0 x=1\nV t=0 v=0\nNV t=0 v=0\nt=1 w=1 v=0\nsimulation ended (Finish) at time 2\n",
    );
}

/// iverilog:
/// ```text
/// i0 y=zz
/// f t=0 x=0 z=1
/// B t=0 y=01
/// Y t=0 y=01
/// P t=0 y=01
/// t=1 y=01
/// ```
/// verilator calls `f` once at time 0 as well. Before the slice: `f t=0 x=x z=x` and a
/// W4031 `[at time 0]` first. The testbench's `wire` on the port is a copy of the held
/// output and reads `z` before the release, as iverilog's does; the `P` / `Y` / `B`
/// order is the time-0 race (race pin).
#[test]
fn a_testbench_reading_a_held_port_output_first() {
    let r = run(
        "e4_port_tbfirst.sv",
        r#"module top;
  logic a, b; wire [1:0] y;
  initial begin
    $display("i0 y=%b", y);
    a = 0; b = 1;
    @(y) $display("B t=%0t y=%b", $time, y);
  end
  initial begin
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  always @(posedge y[0]) $display("P t=%0t y=%b", $time, y);
  always @(y) $display("Y t=%0t y=%b", $time, y);
  dut u(.a(a), .b(b), .y(y));
  initial #100 $finish;
endmodule
module dut(input logic a, input logic b, output logic [1:0] y);
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    f = 0;
    unique case ({x, z}) 2'b01: f = 1; 2'b10: f = 2; endcase
  endfunction
  assign y = f(a, b);
  always_comb if (y == 2'b11) $display("C bad");
endmodule
"#,
    );
    assert_ok(
        &r,
        "i0 y=zz\nf t=0 x=0 z=1\nB t=0 y=01\nP t=0 y=01\nY t=0 y=01\nt=1 y=01\n\
         simulation ended (Finish) at time 2\n",
    );
    assert_eq!(r.count("VITA-W4031"), 0, "{}", r.err);
}

/// iverilog and verilator: `t=1 y=01 c=01` and no report. The `always_comb` reading
/// the assign's output is declared ahead of the `initial`; before the slice the call on
/// `x` reported W4031 `[at time 0]`.
#[test]
fn an_always_comb_ahead_of_the_initial_sees_no_x_run() {
    let r = run(
        "e7_comb_before_initial.sv",
        r#"module top;
  logic a, b; logic [1:0] y; logic [1:0] c;
  function logic [1:0] f(input logic x, input logic z);
    f = 0;
    unique case ({x, z}) 2'b01: f = 1; 2'b10: f = 2; endcase
  endfunction
  always_comb begin
    c = y;
    unique case (y) 2'd1: ; 2'd2: ; endcase
  end
  assign y = f(a, b);
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%b c=%b", $time, y, c);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_ok(&r, "t=1 y=01 c=01\nsimulation ended (Finish) at time 2\n");
    assert_eq!(r.count("VITA-W4031"), 0, "{}", r.err);
}

/// iverilog and verilator: `f t=0 x=0 z=1`, once, though the `initial` reaches
/// `$finish` in the same batch that writes the inputs — the release runs in the
/// rest of time step 0. Before the slice `f t=0 x=x z=x` first.
#[test]
fn finish_at_time_0_still_runs_the_function_once() {
    let r = run(
        "k7_finish_t0.sv",
        r#"module top;
  logic a, b; logic [1:0] y;
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  assign y = f(a, b);
  initial begin
    a = 0; b = 1;
    $finish;
  end
endmodule
"#,
    );
    assert_ok(&r, "f t=0 x=0 z=1\nsimulation ended (Finish) at time 0\n");
}

/// iverilog and verilator, identical:
/// ```text
/// PG t=5
/// PG t=15
/// PG t=25
/// ```
/// A clock gate whose function asserts its enable is known. Before the slice the call
/// on `x` failed it: `error[VITA-E4003] … gate en x at 0 [in top.gate] [at time 0]`,
/// exit 1.
#[test]
fn a_gated_clock_assert_does_not_fail_at_time_0() {
    let r = run(
        "e6_clkgen_gate_assert.sv",
        r#"module top;
  logic clk, en; wire gclk;
  initial begin clk = 0; forever #5 clk = ~clk; end
  function logic gate(input logic c, input logic e);
    assert (e !== 1'bx) else $error("gate en x at %0t", $time);
    return c & e;
  endfunction
  assign gclk = gate(clk, en);
  always @(posedge gclk) $display("PG t=%0t", $time);
  initial begin
    en = 1;
    #32 $finish;
  end
  initial #200 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "PG t=5\nPG t=15\nPG t=25\nsimulation ended (Finish) at time 32\n",
    );
    assert_eq!(r.count("VITA-E4003"), 0, "{}", r.err);
}

/// iverilog:
/// ```text
/// i0 w=xx
/// f t=0 x=0 z=1
/// i1 w=xx
/// t=2 w=01
/// ```
/// A held DELAYED assign still drives its initial `x` (the rhs is not evaluated for
/// it), so `w` reads `xx` and not its `z` default until the first delayed write lands.
/// Residue: `f` still runs four times at time 0 after the release and twice more after
/// `i1` (iverilog once) — a delayed assign is re-evaluated on every settle (ROADMAP §2
/// 🆕 AB residue); before the slice five more calls on `x` came first. A call in a
/// delayed assign is outside the native backend's reach (it runs on `vm`), so the second
/// design holds a delayed assign through the closure instead — `v` copies the held `w`
/// one unit later — which native runs itself. iverilog, the same as vita:
/// ```text
/// i0 v=x
/// f t=0 x=1
/// i1 v=x
/// V t=1 v=1
/// t=2 v=1
/// ```
#[test]
fn a_held_delayed_assign_reads_x_until_its_first_write() {
    let r = run(
        "q7_delay_held_i0.sv",
        r#"module top;
  logic a, b; wire [1:0] w;
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  assign #1 w = f(a, b);
  initial begin
    $display("i0 w=%b", w);
    a = 0; b = 1;
    #0 $display("i1 w=%b", w);
    #2 $display("t=%0t w=%b", $time, w);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    let f0 = "f t=0 x=0 z=1\n";
    assert_ok(
        &r,
        &format!(
            "i0 w=xx\n{}i1 w=xx\n{}f t=1 x=0 z=1\nf t=2 x=0 z=1\nt=2 w=01\nf t=2 x=0 z=1\n\
             f t=3 x=0 z=1\nf t=3 x=0 z=1\nsimulation ended (Finish) at time 3\n",
            f0.repeat(4),
            f0.repeat(2)
        ),
    );
    let r = run(
        "q9_delay_closure_i0.sv",
        r#"module top;
  logic a; wire w; wire v;
  function logic f(input logic x);
    $display("f t=%0t x=%b", $time, x);
    return x;
  endfunction
  assign w = f(a);
  assign #1 v = w;
  always @(v) $display("V t=%0t v=%b", $time, v);
  initial begin
    $display("i0 v=%b", v);
    a = 1;
    #0 $display("i1 v=%b", v);
    #2 $display("t=%0t v=%b", $time, v);
    $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "i0 v=x\nf t=0 x=1\ni1 v=x\nV t=1 v=1\nt=2 v=1\nsimulation ended (Finish) at time 2\n",
    );
    assert!(
        !r.err.contains("VITA-W4030"),
        "native must run it: {}",
        r.err
    );
}

/// iverilog:
/// ```text
/// i0 y=zz w=zz w2=zz v=zz
/// f t=0 x=x z=x
/// f t=0 x=x z=x
/// t=1 y=xx w=xx w2=xx v=xx
/// ```
/// No event line: the release lands `x` on every net and a `z → x` hop is no event, so
/// none of the five `always` blocks runs. `v = ~w` reads the held `w` and is held with
/// it (`zz` before the release, iverilog's value). Race pin: `y` is a `logic`, `xx` by
/// its declared default (iverilog `zz`). Before the slice the two calls came first and
/// `i0` read `xx` everywhere.
#[test]
fn a_release_landing_x_wakes_nothing() {
    let r = run(
        "n4e_xret_effectful.sv",
        r#"module top;
  logic a, b; logic [1:0] y; wire [1:0] w, w2, v;
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  assign y = f(a, b);
  assign w = f(a, b);
  assign w2 = w;
  assign v = ~w;
  always @(y) $display("Y t=%0t y=%b", $time, y);
  always @(w) $display("W t=%0t w=%b", $time, w);
  always @(w2) $display("W2 t=%0t w2=%b", $time, w2);
  always @(v) $display("V t=%0t v=%b", $time, v);
  always @(posedge w[0]) $display("P t=%0t w=%b", $time, w);
  initial begin
    $display("i0 y=%b w=%b w2=%b v=%b", y, w, w2, v);
    #1 $display("t=%0t y=%b w=%b w2=%b v=%b", $time, y, w, w2, v);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "i0 y=xx w=zz w2=zz v=zz\nf t=0 x=x z=x\nf t=0 x=x z=x\nt=1 y=xx w=xx w2=xx v=xx\n\
         simulation ended (Finish) at time 2\n",
    );
}

/// iverilog:
/// ```text
/// t=1 y=01
/// FATAL: k3_fatal_late.sv:4: both t=1
///        Time: 1  Scope: top.f
/// ```
/// verilator `[1] %Fatal: k3_fatal_late.sv:4: Assertion failed in top.f: both t=1`. A
/// `$fatal` a continuous assign's call latches inside the settle ends the run at that
/// time on every backend: the native loop now checks the latch after its loop-top
/// settle as the engine does (before the slice native ended `(Error) at time 2`, the
/// interpreter and the VM at 1). The harness compares the three backends' stdout.
#[test]
fn a_fatal_in_a_continuous_assign_ends_every_backend_at_its_time() {
    let r = run(
        "k3_fatal_late.sv",
        r#"module top;
  logic a, b; logic [1:0] y;
  function logic [1:0] f(input logic x, input logic z);
    if (x & z) $fatal(1, "both t=%0t", $time);
    return {x, z};
  endfunction
  assign y = f(a, b);
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%b", $time, y);
    a = 1;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_eq!(
        (r.code, r.out.as_str()),
        (1, "t=1 y=01\nsimulation ended (Error) at time 1\n"),
        "{}",
        r.err
    );
    assert_eq!(
        r.verdict(),
        ["k3_fatal_late.sv:4:16: fatal[VITA-F4004] F-RUN-FATAL: both t=1 [in top.f] [at time 1]"]
    );
}

/// verilator:
/// ```text
/// f1 t=0 x=0
/// f2 t=0 x=0
/// f3 t=0 x=0
/// t=1 y1=1 y2=1 y3=1
/// ```
/// iverilog, on the same bits spelled three ways:
/// ```text
/// f1 t=0 x=0
/// f2 t=0 x=x
/// WARNING: t1_iv_xcall_spelling.sv:10: value is unhandled for priority or unique case statement
///          Time: 0  Scope: top.f2
/// f3 t=0 x=x
/// WARNING: t1_iv_xcall_spelling.sv:14: value is unhandled for priority or unique case statement
///          Time: 0  Scope: top.f3
/// f2 t=0 x=0
/// f3 t=0 x=0
/// t=1 y1=1 y2=1 y3=1
/// ```
/// iverilog calls an assign's function on `x` first only when an argument is an
/// expression (`a | 1'b0`) or a select (`av[0]`), not when it is the net `a`: no oracle
/// on that axis (ROADMAP §0 iverilog defects). vita gives all three verilator's single
/// call. Before the slice all three ran on `x` first with a W4031 `[at time 0]` each.
#[test]
fn an_expression_argument_is_not_called_on_x() {
    let r = run(
        "t1_iv_xcall_spelling.sv",
        r#"module top;
  logic a; logic [1:0] av;
  logic y1, y2, y3;
  function logic f1(input logic x);
    $display("f1 t=%0t x=%b", $time, x);
    unique case (x) 1'b0: f1 = 1; 1'b1: f1 = 0; endcase
  endfunction
  function logic f2(input logic x);
    $display("f2 t=%0t x=%b", $time, x);
    unique case (x) 1'b0: f2 = 1; 1'b1: f2 = 0; endcase
  endfunction
  function logic f3(input logic x);
    $display("f3 t=%0t x=%b", $time, x);
    unique case (x) 1'b0: f3 = 1; 1'b1: f3 = 0; endcase
  endfunction
  assign y1 = f1(a);
  assign y2 = f2(a | 1'b0);
  assign y3 = f3(av[0]);
  initial begin
    a = 0; av = 2'b10;
    #1 $display("t=%0t y1=%b y2=%b y3=%b", $time, y1, y2, y3);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "f1 t=0 x=0\nf2 t=0 x=0\nf3 t=0 x=0\nt=1 y1=1 y2=1 y3=1\nsimulation ended (Finish) at time 2\n",
    );
    assert_eq!(r.count("VITA-W4031"), 0, "{}", r.err);
}

/// RACE PINS — an assign nothing a time-0 process writes can reach is not held.
///
/// `a4`: constant arguments. verilator `f t=0 x=0 z=1` / `i0 y=01` / `i1 y=01` / `t=1
/// y=01`; iverilog `i0 y=zz` / `f t=0 x=0 z=1` / `i1 y=01` / `t=1 y=01`. Which comes
/// first is a race both tools answer differently; vita keeps its pre-slice order
/// (verilator's) because the assign's read set is empty and it runs once either way.
/// `T3`: the same constant call beside a held one (`f2(a, 1'b1)`): `f1` keeps its place
/// ahead of `i0`, `f2` loses its call on `x` (verilator: `f1`, `i0 y1=01 y2=00`, `f2`;
/// iverilog `i0 y1=zz y2=zz`, `f2`, `f1`); `y2` is a `logic` read before its release,
/// `xx` by its declared default.
#[test]
fn a_constant_argument_call_keeps_its_time_0_place() {
    let r = run(
        "a4_constargs.sv",
        r#"module top;
  wire [1:0] y = f(1'b0, 1'b1);
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  initial begin
    $display("i0 y=%b", y);
    #0 $display("i1 y=%b", y);
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "f t=0 x=0 z=1\ni0 y=01\ni1 y=01\nt=1 y=01\nsimulation ended (Finish) at time 2\n",
    );
    let r = run(
        "t3_const_vs_written.sv",
        r#"module top;
  logic a; logic [1:0] y1, y2;
  function logic [1:0] f1(input logic x, input logic z);
    $display("f1 t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  function logic [1:0] f2(input logic x, input logic z);
    $display("f2 t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  assign y1 = f1(1'b0, 1'b1);
  assign y2 = f2(a, 1'b1);
  initial begin
    $display("i0 y1=%b y2=%b", y1, y2);
    a = 0;
    #1 $display("t=%0t y1=%b y2=%b", $time, y1, y2);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "f1 t=0 x=0 z=1\ni0 y1=01 y2=xx\nf2 t=0 x=0 z=1\nt=1 y1=01 y2=01\n\
         simulation ended (Finish) at time 2\n",
    );
}

/// RACE PIN — a call to an effect-free function is not held: verilator `i0 y1=01 y2=01
/// y3=01`, iverilog `i0 y1=zz y2=zz y3=01` (the calls after the `initial`, the plain
/// `{a, b}` before it, on the same bits — no oracle). vita keeps its pre-slice answer.
#[test]
fn a_pure_call_is_not_held() {
    let r = run(
        "t2b_order_port_pure.sv",
        r#"module dut(input logic a, input logic b, output logic [1:0] y);
  function logic [1:0] g(input logic x, input logic z);
    return {x, z};
  endfunction
  assign y = g(a, b);
endmodule
module top;
  logic a = 0, b = 1; logic [1:0] y1, y2; wire [1:0] y3 = {a, b};
  function logic [1:0] f(input logic x, input logic z);
    return {x, z};
  endfunction
  assign y1 = f(a, b);
  dut u(.a(a), .b(b), .y(y2));
  initial begin
    $display("i0 y1=%b y2=%b y3=%b", y1, y2, y3);
    #1 $display("t=%0t y1=%b y2=%b y3=%b", $time, y1, y2, y3);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "i0 y1=01 y2=01 y3=01\nt=1 y1=01 y2=01 y3=01\nsimulation ended (Finish) at time 2\n",
    );
}

/// iverilog:
/// ```text
/// i0 w=zz c=zz
/// f t=0 x=01
/// C t=0 c=01
/// i1 w=01 c=01
/// ```
/// verilator calls `f` once at time 0 too. RACE PIN on `i0`: the copy `c = w` reads the
/// held `w` and is held with it, so the time-0 copy repair leaves it alone and the
/// `logic` reads `xx`, its declared default, until the release (verilator `00`).
#[test]
fn a_copy_of_a_held_net_waits_for_the_release() {
    let r = run(
        "q3_copy_held_logic.sv",
        r#"module top;
  logic [1:0] a;
  wire [1:0] w;
  logic [1:0] c;
  function logic [1:0] f(input logic [1:0] x);
    $display("f t=%0t x=%b", $time, x);
    return x;
  endfunction
  assign w = f(a);
  assign c = w;
  always @(c) $display("C t=%0t c=%b", $time, c);
  initial begin
    a = 2'b01;
    $display("i0 w=%b c=%b", w, c);
    #1 $display("i1 w=%b c=%b", w, c);
    $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "i0 w=zz c=xx\nf t=0 x=01\nC t=0 c=01\ni1 w=01 c=01\nsimulation ended (Finish) at time 1\n",
    );
}

/// iverilog and verilator:
/// ```text
/// f t=0 x=0 z=1
/// g t=0 v=01
/// t=1 y=01 z=10
/// ```
/// `z = g(y)` is declared ahead of `y = f(a, b)`; the release waits for `y` before it
/// releases `z`, so `g` runs once, on `y`'s settled value. Before the slice `g t=0
/// v=xx` and `f t=0 x=x z=x` came first. Also a hold two module levels down (`s5`):
/// both tools print `t=1 y=01` and no report; before the slice a W4031 `[at time 0]`.
#[test]
fn a_chain_of_held_assigns_releases_in_dependency_order() {
    let r = run(
        "b1_cachain.sv",
        r#"module top;
  logic a, b; logic [1:0] y, z;
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  function logic [1:0] g(input logic [1:0] v);
    $display("g t=%0t v=%b", $time, v);
    return ~v;
  endfunction
  assign z = g(y);
  assign y = f(a, b);
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%b z=%b", $time, y, z);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "f t=0 x=0 z=1\ng t=0 v=01\nt=1 y=01 z=10\nsimulation ended (Finish) at time 2\n",
    );
    let r = run(
        "s5_two_level_hier.sv",
        r#"module leaf(input logic a, input logic b, output logic [1:0] y);
  function logic [1:0] f(input logic x, input logic z);
    f = 0;
    unique case ({x, z}) 2'b01: f = 1; 2'b10: f = 2; endcase
  endfunction
  assign y = f(a, b);
endmodule
module mid(input logic a, input logic b, output logic [1:0] y);
  leaf l(.a(a), .b(b), .y(y));
endmodule
module top;
  logic a, b; logic [1:0] y;
  mid m(.a(a), .b(b), .y(y));
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_ok(&r, "t=1 y=01\nsimulation ended (Finish) at time 2\n");
    assert_eq!(r.count("VITA-W4031"), 0, "{}", r.err);
}

/// iverilog and verilator: `t=1 y=10` and no report (`unique case` on `mem[0]`, which
/// the `initial` writes). The call has no argument, but its function reads the module
/// array, so its read set is `{mem}`, not empty, and it is held: the empty-read-set
/// exemption cannot reach a call whose body reads a net. Before the slice a W4031
/// `[at time 0]`.
#[test]
fn a_call_with_no_argument_reading_a_module_net_is_held() {
    let r = run(
        "q5_exempt_mem_unique.sv",
        r#"module top;
  logic [1:0] mem [0:3];
  wire [1:0] y;
  function logic [1:0] f();
    unique case (mem[0])
      2'b01: f = 2'b10;
      2'b10: f = 2'b01;
    endcase
  endfunction
  assign y = f();
  initial begin
    mem[0] = 2'b01;
    #1 $display("i1 y=%b", y);
    $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_ok(&r, "i1 y=10\nsimulation ended (Finish) at time 1\n");
    assert_eq!(r.count("VITA-W4031"), 0, "{}", r.err);
}

/// RESIDUE PINS — what the hold does not change (ROADMAP §2 🆕 AB residue): an assign
/// `levelize::ca_deps` cannot certify is still re-evaluated on every settle pass after
/// the release.
///
/// `s4`, a multi-driver member: iverilog and verilator print `f t=0 x=0` once and
/// nothing at 1; vita `f t=0 x=0` twice and `f t=1 x=0` (before the slice three calls
/// on `x` first).
/// `c1_delay`, a delayed assign: iverilog `f t=0 x=0 z=1`, `t=2 y=01`, `f t=2 x=0
/// z=0`, `t=4 y=00`; vita two calls at 0 and one at every later step (before the slice
/// four calls on `x` first).
/// `n_ca_objf2case_tbL`, a class method through a handle: verilator `[2] %Error: …
/// unique case, but none matched for '2'h0'` twice at 2, 3 and 4, nothing at 0; vita
/// W4031 ×3 at 2, ×2 at 3, ×2 at 4 (before the slice also ×4 `[at time 0]`; iverilog
/// aborts on the class handle).
#[test]
fn an_uncertified_assign_still_runs_per_settle_pass() {
    let r = run(
        "s4_md_call.sv",
        r#"module top;
  logic a, b, en; wire y;
  function logic f(input logic x);
    $display("f t=%0t x=%b", $time, x);
    return x;
  endfunction
  assign y = en ? f(a) : 1'bz;
  assign y = en ? 1'bz : b;
  initial begin
    en = 1; a = 0; b = 1;
    #1 $display("t=%0t y=%b", $time, y);
    en = 0;
    #1 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "f t=0 x=0\nf t=0 x=0\nf t=1 x=0\nt=1 y=0\nt=2 y=1\nsimulation ended (Finish) at time 3\n",
    );
    let r = run(
        "c1_delay.sv",
        r#"module top;
  logic a, b; logic [1:0] y;
  function logic [1:0] f(input logic x, input logic z);
    $display("f t=%0t x=%b z=%b", $time, x, z);
    return {x, z};
  endfunction
  assign #1 y = f(a, b);
  initial begin
    a = 0; b = 1;
    #2 $display("t=%0t y=%b", $time, y);
    b = 0;
    #2 $display("t=%0t y=%b", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "f t=0 x=0 z=1\nf t=0 x=0 z=1\nf t=1 x=0 z=1\nf t=2 x=0 z=1\nt=2 y=01\nf t=2 x=0 z=0\n\
         f t=3 x=0 z=0\nf t=4 x=0 z=0\nt=4 y=00\nf t=4 x=0 z=0\nf t=5 x=0 z=0\nf t=5 x=0 z=0\n\
         simulation ended (Finish) at time 5\n",
    );
    let r = run(
        "n_ca_objf2case_tbL.sv",
        r#"class C;
  function logic [1:0] f(input logic x, input logic z);
    logic [1:0] r; r = 0;
    unique case ({x, z}) 2'b10: r = 1; 2'b01: r = 2; endcase
    return r;
  endfunction
endclass
module dut(input logic a, input logic b, output logic [1:0] y);
  C obj; initial obj = new;
  assign y = obj.f(a, b);
endmodule
module top;
  logic a, b; logic [1:0] y;
  dut u(.a(a), .b(b), .y(y));
  initial begin
    a = 0; b = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 b = 0;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "t=1 y=2\nt=3 y=0\nsimulation ended (Finish) at time 4\n",
    );
    let at = |t| w4031("n_ca_objf2case_tbL.sv:4:12", "top.C.f", t);
    assert_eq!(
        w4031_lines(&r),
        [at(2), at(2), at(2), at(3), at(3), at(4), at(4)]
    );
}

/// iverilog:
/// ```text
/// f2 t=0 x=0
/// sd01a_finish_seed_nobatch.sv:4: $finish called at 0 (1s)
/// ```
/// verilator `f2 t=0 x=0`. vita refuses a `$finish` inside a function (`VITA-F4004`,
/// loud, exit 1 — before and after the slice). `y1 = f1(1'b0)` is not held (constant
/// argument, empty read set), so `f1` latches that fatal in the time-0 settle; the
/// design has no process, so the release is due at once. Every backend checks the latch
/// at the loop top before it settles, so none of them runs the release and calls `f2`:
/// the native loop used to check only after its settle and printed `f2 t=0 x=0` where
/// the interpreter and the VM printed nothing.
#[test]
fn a_fatal_latched_at_time_0_stops_every_backend_before_the_release() {
    let r = run(
        "sd01a_finish_seed_nobatch.sv",
        r#"module top;
  logic a = 1'b0;
  wire y1, y2;
  function automatic logic f1(input logic x); if (x == 1'b0) $finish; return x; endfunction
  function automatic logic f2(input logic x); $display("f2 t=%0t x=%b", $time, x); return x; endfunction
  assign y1 = f1(1'b0);
  assign y2 = f2(a);
endmodule
"#,
    );
    assert_eq!(
        (r.code, r.out.as_str()),
        (1, "simulation ended (Error) at time 0\n"),
        "{}",
        r.err
    );
    let v = r.verdict();
    assert_eq!(v.len(), 1, "{}", r.err);
    assert!(
        v[0].starts_with(
            "sd01a_finish_seed_nobatch.sv:4:62: fatal[VITA-F4004] F-RUN-FATAL: `$finish` was \
             reached inside a subroutine body."
        ),
        "{}",
        r.err
    );
}

/// iverilog and verilator, identical:
/// ```text
/// f t=0 x=0
/// t1 y=0
/// ```
/// `g` has no system task of its own but calls `f`, which prints: a callee that is not
/// effect-free makes every caller not effect-free, so `y = g(a)` is held. Before the
/// slice `f t=0 x=x` came first.
#[test]
fn an_effectful_callee_behind_a_pure_wrapper_is_held() {
    let r = run(
        "sd06_wrapper.sv",
        r#"module top;
  logic a;
  wire y;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  function automatic logic g(input logic x); return f(x); endfunction
  assign y = g(a);
  initial a = 1'b0;
  initial #1 $display("t1 y=%b", y);
  initial #10 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "f t=0 x=0\nt1 y=0\nsimulation ended (Finish) at time 10\n",
    );
}

/// iverilog, both declaration orders:
/// ```text
/// f t=0 x=0
/// NV t=0 v=0
/// V t=0 v=0
/// ```
/// (verilator `PV t=0 v=1`, `V t=0 v=1`: it holds no `z`, so `u[1] === 1'bz` is 0 for it
/// only after its own settle — no oracle on that axis.) `v` reads `u`, which reads the
/// held `w`: the closure holds every reader downstream, two levels here, whatever order
/// the assigns are declared in (the second design declares them reader first). Settled on
/// `w`'s `z` default, `v` would go to 1 and the first batch would print `PV t=0 v=1`
/// before the release took it back. Before the slice `f t=0 x=x` came first.
#[test]
fn a_reader_two_assigns_below_a_held_one_is_held_in_either_order() {
    let tail = r#"  always @(posedge v) $display("PV t=%0t v=%b", $time, v);
  always @(negedge v) $display("NV t=%0t v=%b", $time, v);
  always @(v) $display("V t=%0t v=%b", $time, v);
  initial a = 1'b0;
  initial #10 $finish;
endmodule
"#;
    let f = "  function automatic logic f(input logic x); $display(\"f t=%0t x=%b\", $time, x); \
             return x; endfunction\n";
    let out = "f t=0 x=0\nNV t=0 v=0\nV t=0 v=0\nsimulation ended (Finish) at time 10\n";
    let fwd = format!(
        "module top;\n  logic a;\n  wire w;\n  wire [1:0] u;\n  wire v;\n{f}  assign w = f(a);\n  \
         assign u = {{w, 1'b0}};\n  assign v = (u[1] === 1'bz);\n{tail}"
    );
    assert_ok(&run("sd07_two_level.sv", &fwd), out);
    let rev = format!(
        "module top;\n  logic a;\n  wire v;\n  wire [1:0] u;\n  wire w;\n{f}  assign v = (u[1] \
         === 1'bz);\n  assign u = {{w, 1'b0}};\n  assign w = f(a);\n{tail}"
    );
    assert_ok(&run("sd07r_two_level_rev.sv", &rev), out);
}

/// verilator:
/// ```text
/// idx t=0 x=0
/// t1 arr=0001
/// ```
/// (iverilog refuses the non-constant lvalue index.) A call in a continuous assign's
/// LVALUE index is held like one in its right-hand side: `idx_f` runs once, on the
/// written `a`. The undriven bits read `z` in vita, `0` in 2-state verilator. Before
/// the slice `idx t=0 x=x` came first.
#[test]
fn a_call_in_an_lvalue_index_is_held() {
    let r = run(
        "sd08_lhs_index_call.sv",
        r#"module top;
  logic a, b;
  wire [3:0] arr;
  function automatic integer idx_f(input logic x); $display("idx t=%0t x=%b", $time, x); return x ? 1 : 0; endfunction
  assign arr[idx_f(a)] = b;
  initial begin a = 1'b0; b = 1'b1; end
  initial #1 $display("t1 arr=%b", arr);
  initial #10 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "idx t=0 x=0\nt1 arr=zzz1\nsimulation ended (Finish) at time 10\n",
    );
}

/// iverilog:
/// ```text
/// f t=0 x=0
/// g t=0 x=z
/// g t=0 x=1
/// t1 n=10 m=0
/// ```
/// verilator (`%Warning-UNOPTFLAT … Circular combinational logic: 'top.n'`): `f t=0 x=0`,
/// `g t=0 x=1` three times, `t1 n=10 m=0`. No oracle on the call count: iverilog's first
/// `g` call on `z` is its select-argument spelling (`n[1]`; see
/// `an_expression_argument_is_not_called_on_x`), verilator iterates its circular net.
/// ORDER PIN for the release waves: `n[1] = ~n[0]` reads the net it drives, which the
/// held `n[0] = f(a)` also drives, so its wave waits for that other driver only (not for
/// itself), and `m = g(n[1])` — declared first — waits for both. `g` therefore runs once,
/// on the settled `n[1]`. Before the slice: six calls, `g` on `z` and `x` first.
#[test]
fn a_held_assign_reading_its_own_net_waits_only_for_the_other_driver() {
    let r = run(
        "r2k_shared_net_order.sv",
        r#"module top;
  logic a;
  wire [1:0] n;
  wire m;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  function automatic logic g(input logic x); $display("g t=%0t x=%b", $time, x); return ~x; endfunction
  assign m = g(n[1]);
  assign n[0] = f(a);
  assign n[1] = ~n[0];
  initial a = 1'b0;
  initial #1 $display("t1 n=%b m=%b", n, m);
  initial #10 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "f t=0 x=0\ng t=0 x=1\nt1 n=10 m=0\nsimulation ended (Finish) at time 10\n",
    );
}

/// iverilog and verilator, identical:
/// ```text
/// h t=0 x=0
/// g t=0 x=0
/// g t=0 x=1
/// g3 t=0 x=1
/// t1 p=0 q=1 y=0 z=1 w=1
/// ```
/// `{p, q} = h(a)` drives TWO nets; `z = g(q)` waits for it on `q`, `y = g(p)` on `p`,
/// and `w = g3(z)` — declared first — waits for `z`. Every call runs once, on a settled
/// value. The two `g` lines are in declaration order (`z` before `y`) where both tools
/// print `y`'s first: which of two readers released in one wave runs first is the
/// release's order, not a value (race pin). Before the slice eleven calls, on `z` and
/// `x` first.
#[test]
fn an_assign_driving_two_nets_releases_the_readers_of_both() {
    let r = run(
        "r2c_concat2_rev.sv",
        r#"module top;
  logic a;
  wire p, q, y, z, w;
  function automatic logic [1:0] h(input logic x); $display("h t=%0t x=%b", $time, x); return {x, ~x}; endfunction
  function automatic logic g(input logic x); $display("g t=%0t x=%b", $time, x); return x; endfunction
  function automatic logic g3(input logic x); $display("g3 t=%0t x=%b", $time, x); return x; endfunction
  assign w = g3(z);
  assign z = g(q);
  assign y = g(p);
  assign {p, q} = h(a);
  initial a = 1'b0;
  initial #1 $display("t1 p=%b q=%b y=%b z=%b w=%b", p, q, y, z, w);
  initial #10 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "h t=0 x=0\ng t=0 x=1\ng t=0 x=0\ng3 t=0 x=1\nt1 p=0 q=1 y=0 z=1 w=1\n\
         simulation ended (Finish) at time 10\n",
    );
}

/// iverilog:
/// ```text
/// gv t=0 v=zx
/// f t=0 x=x
/// gv t=0 v=x0
/// f t=0 x=0
/// gv t=0 v=00
/// t1 v=00 yy=00
/// ```
/// verilator: `f t=0 x=0`, `gv t=0 v=00` twice each, `t1 v=00 yy=00`. No oracle on the
/// counts (iverilog runs both functions on unsettled values, verilator iterates). The
/// held `v[1] = f(v[0])` is the only held driver of `v` and reads `v` itself, so it waits
/// for nobody; `yy = gv(v)` — declared first — waits for it, and runs once on `00`. `f`
/// runs twice: its own write to `v` re-dirties it within its wave (a self-reading assign
/// re-runs, as on PRE). Before the slice ten calls, `gv` on `zz`, `zx`, `xx`, `x0` first.
#[test]
fn a_held_self_reading_sole_driver_is_released_at_once() {
    let r = run(
        "r2e_selfread_rev.sv",
        r#"module top;
  logic a;
  wire [1:0] v;
  wire [1:0] yy;
  function automatic logic f(input logic x); $display("f t=%0t x=%b", $time, x); return x; endfunction
  function automatic logic [1:0] gv(input logic [1:0] x); $display("gv t=%0t v=%b", $time, x); return x; endfunction
  assign yy = gv(v);
  assign v[1] = f(v[0]);
  assign v[0] = a;
  initial a = 1'b0;
  initial #1 $display("t1 v=%b yy=%b", v, yy);
  initial #10 $finish;
endmodule
"#,
    );
    assert_ok(
        &r,
        "f t=0 x=0\nf t=0 x=0\ngv t=0 v=00\nt1 v=00 yy=00\nsimulation ended (Finish) at time 10\n",
    );
}
