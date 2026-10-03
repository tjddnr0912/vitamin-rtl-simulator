//! `unique if … else if …` with no final `else` (IEEE 1800-2017 §12.4.2) — the
//! no-match report covers the whole series (§4.5.585). Before this slice only a lone
//! `unique if` reported: the arm went on the first `if` only when its own `else` was
//! empty, so every chain was silent where Verilator reports.
//!
//! In procedural code outside a subroutine (`initial`, `always*`, `final` and the
//! `fork` branches inside them) the arm goes on the last `if` of the `else if` series
//! and reports at the first `if`, as Verilator does. The series is read from the
//! written tokens: only a bare `if` right after `else` continues it. Anything else after
//! `else` — a `begin … end` block, a labelled statement, `;`, a delay, event or `wait`
//! control, a `case`, a `repeat`, an immediate `assert` / `assume`, or a qualified
//! `unique` / `priority` / `unique0` / `priority0 if` — ends it with no arm, and a
//! then-branch is never entered. A qualified `if` after `else` is the series' final
//! `else` statement (IEEE 1800-2017 Syntax 12-2: `{ else if ( … ) … }` takes no
//! qualifier), so it arms itself by its own qualifier only; Verilator continues the outer
//! series through an `else unique0 if` and reports there — a residue pinned below.
//!
//! Every function and task body keeps the lone-`if` rule (`hdl-parser`'s
//! `first_if_arm_only`), so a chain there stays silent where Verilator reports (ROADMAP
//! §3.b `unique-if-chain`, BLOCKED). Measured reasons: the constant-function interpreter
//! refuses at elaboration (`VITA-E3009`) a function whose call reaches an armed tail
//! (ROADMAP §3.b `unique-const-fn`); a continuous assign runs the function it calls once
//! more at time 0, on `x`, before the `initial` that writes its inputs, and that run
//! reaches what the function calls (a class method, a constructor, an item
//! `function void` with no formals, a task), so an armed miss there would report at time
//! 0 where both tools are silent (ROADMAP §2 🆕 AB); and the package-scoped call closure
//! walk treats the synthesized arm as impure, so a `pk::f(…)` call reaching an armed
//! body would be refused.
//!
//! Oracle: verilator 5.052 (`--binary --timing --assert`, run with
//! `+verilator+error+limit+1000`), quoted verbatim above each pin. iverilog 13 rejects
//! every `unique`/`priority if` (syntax error), so it is quoted only on a `unique case`
//! twin. Verilator is silent on every `priority if`, single or chain, so priority
//! chains are pinned hand-IEEE (§12.4.2 gives `priority if` the same no-match
//! violation). The W4031 text says `case statement` on `if` forms too (ROADMAP §3.b
//! `unique-if-text`); only the location and time are compared with Verilator here.
//! Every design runs on `--backend native`, `interp` and `vm`, which must agree on
//! stdout, exit code and every W4031 and error line; one also runs through `vita vcmp`
//! / `velab` / `vrun`.
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
    /// Every `VITA-W4031` line, in order.
    fn w4031(&self) -> Vec<&str> {
        self.err
            .lines()
            .filter(|l| l.contains("VITA-W4031"))
            .collect()
    }

    /// The lines the backends must agree on: W4031 and every error.
    fn verdict(&self) -> Vec<&str> {
        self.err
            .lines()
            .filter(|l| l.contains("VITA-W4031") || l.contains("error[") || l.contains("fatal["))
            .collect()
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

/// A fresh directory holding `src` saved as `name` (relative, so a diagnostic names
/// `name:line:col`).
fn scratch(name: &str, src: &str) -> PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_uif_chain_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(name), src).unwrap();
    d
}

/// Runs `src` on the three backends; they must agree on stdout, the exit code and the
/// W4031 / error lines. Returns the native run.
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

fn w(loc: &str, scope: &str, t: u32) -> String {
    format!(
        "{loc}: warning[VITA-W4031] W-RUN-UNIQUE-VIOLATION: value is unhandled for \
         priority or unique case statement [in {scope}] [at time {t}]"
    )
}

fn assert_w4031(r: &Run, want: &[String]) {
    assert_eq!(r.code, 0, "rc\nstdout:\n{}\nstderr:\n{}", r.out, r.err);
    assert_eq!(r.w4031(), want, "W4031 lines\nstderr:\n{}", r.err);
}

const M_IF_UNIQUE: &str = r#"module top;
  logic a = 0, b = 0, c = 0;
  logic [1:0] r = 0;
  initial begin
    #1 unique if (a) r = 1;
    $display("t=%0t if1 done", $time);
    #1 unique if (a) r = 1; else if (b) r = 2;
    $display("t=%0t chain2 done", $time);
    #1 unique if (a) r = 1; else if (b) r = 2; else if (c) r = 3;
    $display("t=%0t chain3 done", $time);
    #1 unique if (a) r = 1; else if (b) r = 2; else r = 3;
    $display("t=%0t chain-else done", $time);
    #1 unique if (a) r = 1; else begin if (b) r = 2; end
    $display("t=%0t else-begin-if done", $time);
    #1 unique if (a) begin if (c) r = 1; end else if (b) r = 2;
    $display("t=%0t nested-in-then done", $time);
    #1 unique if (a) if (c) r = 1; else r = 2;
    $display("t=%0t dangling-else done", $time);
    #1 a = 1; unique if (a) r = 1; else if (b) r = 2;
    $display("t=%0t chain-match done", $time);
    #1 a = 0; b = 1; unique if (a) r = 1; else if (b) r = 2;
    $display("t=%0t chain-match2 done", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#;

/// verilator:
/// ```text
/// [1] %Error: m_if_unique.sv:5: Assertion failed in top: 'unique if' statement violated
/// t=1 if1 done
/// [2] %Error: m_if_unique.sv:7: Assertion failed in top: 'unique if' statement violated
/// t=2 chain2 done
/// [3] %Error: m_if_unique.sv:9: Assertion failed in top: 'unique if' statement violated
/// t=3 chain3 done
/// t=4 chain-else done
/// t=5 else-begin-if done
/// [6] %Error: m_if_unique.sv:15: Assertion failed in top: 'unique if' statement violated
/// t=6 nested-in-then done
/// [7] %Error: m_if_unique.sv:17: Assertion failed in top: 'unique if' statement violated
/// t=7 dangling-else done
/// t=8 chain-match done
/// t=9 chain-match2 done
/// - m_if_unique.sv:23: Verilog $finish
/// ```
/// t2/t3/t6 are the chains (silent before this slice); t4 (final `else`), t5
/// (`else begin if … end`), t8/t9 (a branch matches) stay silent; t1 and the
/// dangling-else t7 (the `else` binds to the inner `if`) report as before. The staged
/// pipeline reports the same lines.
#[test]
fn unique_if_chains_report_at_the_first_if() {
    let want = [
        w("m_if_unique.sv:5:15", "top", 1),
        w("m_if_unique.sv:7:15", "top", 2),
        w("m_if_unique.sv:9:15", "top", 3),
        w("m_if_unique.sv:15:15", "top", 6),
        w("m_if_unique.sv:17:15", "top", 7),
    ];
    let r = run("m_if_unique.sv", M_IF_UNIQUE);
    assert_w4031(&r, &want);
    assert!(r.out.contains("t=9 chain-match2 done"), "{}", r.out);
    let s = run_staged("m_if_unique.sv", M_IF_UNIQUE);
    assert_w4031(&s, &want);
    assert_eq!(s.out, r.out);
}

const VL_LINES: &str = r#"module top;
  logic a = 0, b = 0, c = 0;
  logic [1:0] r = 0;
  int n = 0;
  function automatic logic inc(input logic v); n = n + 1; return v; endfunction
  initial begin
    #1 unique if (a) r = 1;
       else if (b) r = 2;
       else if (c) r = 3;
    $display("t=%0t multiline done", $time);
    #1 unique if (a) r = 1;
       else unique if (b) r = 2;
    $display("t=%0t split else-unique-if done", $time);
    #1 unique if (a) r = 1;
       else unique0 if (b) r = 2;
    $display("t=%0t split else-unique0-if done", $time);
    #1 unique if (a) r = 1;
       else unique if (b) r = 2;
       else if (c) r = 3;
    $display("t=%0t split middle-unique chain3 done", $time);
    #1 unique if (a) r = 1;
       else unique0 if (b) r = 2;
       else if (c) r = 3;
    $display("t=%0t split middle-unique0 chain3 done", $time);
    #1 unique if (a) r = 1; else if (b) r = 2; else ;
    $display("t=%0t explicit null else done", $time);
    #1 n = 0; unique if (inc(a)) r = 1; else if (inc(b)) r = 2;
    $display("t=%0t side-effect n=%0d", $time, n);
    #1 n = 0; b = 1; unique if (inc(a)) r = 1; else if (inc(b)) r = 2; else if (inc(c)) r = 3;
    $display("t=%0t side-effect match n=%0d r=%0d", $time, n, r);
    #1 a = 1; b = 0; unique if (a) begin if (c) r = 1; end else if (b) r = 2;
    $display("t=%0t matched-then inner-miss done", $time);
    #1 $finish;
  end
endmodule
"#;

/// verilator:
/// ```text
/// %Warning-SIDEEFFECT: vl_lines.sv:27:50: Expression side effect may be mishandled
/// %Warning-SIDEEFFECT: vl_lines.sv:29:33: Expression side effect may be mishandled
/// %Warning-SIDEEFFECT: vl_lines.sv:29:57: Expression side effect may be mishandled
/// %Warning-SIDEEFFECT: vl_lines.sv:29:81: Expression side effect may be mishandled run rc=0
/// [1] %Error: vl_lines.sv:7: Assertion failed in top: 'unique if' statement violated
/// t=1 multiline done
/// [2] %Error: vl_lines.sv:11: Assertion failed in top: 'unique if' statement violated
/// t=2 split else-unique-if done
/// [3] %Error: vl_lines.sv:14: Assertion failed in top: 'unique if' statement violated
/// t=3 split else-unique0-if done
/// [4] %Error: vl_lines.sv:17: Assertion failed in top: 'unique if' statement violated
/// t=4 split middle-unique chain3 done
/// [5] %Error: vl_lines.sv:21: Assertion failed in top: 'unique if' statement violated
/// t=5 split middle-unique0 chain3 done
/// t=6 explicit null else done
/// [7] %Error: vl_lines.sv:27: Assertion failed in top: 'unique if' statement violated
/// t=7 side-effect n=2
/// t=8 side-effect match n=5 r=2
/// t=9 matched-then inner-miss done
/// - vl_lines.sv:33: Verilog $finish
/// ```
/// A multi-line chain reports at the first `if`'s line (t1). An inner `unique if`
/// after `else` arms itself and the outer series ends there (t2, t4): one report, as
/// Verilator, but at the inner line where Verilator names the outer one. Residue: an
/// inner `unique0 if` after `else` is the outer series' final `else` statement (IEEE
/// 1800-2017 Syntax 12-2) and `unique0` suppresses its own miss, so t3 and t5 are
/// silent, as before this slice, where Verilator continues the outer series through it
/// and reports. `else ;` ends the series (t6). A matched then-branch whose nested `if`
/// misses is not a violation (t9). The conditions are evaluated as often as before the
/// slice (`n=2` at t7 and t8 on PRE too); Verilator's `n=5` at t8 is its own one-hot
/// check re-evaluating the conditions, not a count oracle.
#[test]
fn chain_series_ends_and_nested_qualifiers() {
    let r = run("vl_lines.sv", VL_LINES);
    assert_w4031(
        &r,
        &[
            w("vl_lines.sv:7:15", "top", 1),
            w("vl_lines.sv:12:20", "top", 2),
            w("vl_lines.sv:18:20", "top", 4),
            w("vl_lines.sv:27:22", "top", 7),
        ],
    );
    assert!(r.out.contains("t=7 side-effect n=2\n"), "{}", r.out);
    assert!(
        r.out.contains("t=8 side-effect match n=2 r=2\n"),
        "{}",
        r.out
    );
    assert!(
        r.out.contains("t=9 matched-then inner-miss done"),
        "{}",
        r.out
    );
}

const B01: &str = r#"module top;
  logic a = 0, b = 0;
  logic [1:0] r = 0;
  initial begin
    #1 unique if (a) r = 1; else unique0 if (b) r = 2;
    $display("t=%0t else-unique0-if done", $time);
    #1 unique if (a) r = 1; else unique if (b) r = 2;
    $display("t=%0t else-unique-if done", $time);
    #1 unique if (a) r = 1; else priority if (b) r = 2;
    $display("t=%0t else-priority-if done", $time);
    #1 unique0 if (a) r = 1; else unique if (b) r = 2;
    $display("t=%0t u0-else-unique-if done", $time);
    #1 unique if (a) r = 1; else L1: if (b) r = 2;
    $display("t=%0t else-labeled-if done", $time);
    #1 unique if (a) r = 1; else if (b) r = 2; else if (a) r = 3;
    $display("t=%0t chain3-dup done", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#;

/// verilator:
/// ```text
/// [1] %Error: b01_nested_qual.sv:5: Assertion failed in top: 'unique if' statement violated
/// t=1 else-unique0-if done
/// [2] %Error: b01_nested_qual.sv:7: Assertion failed in top: 'unique if' statement violated
/// t=2 else-unique-if done
/// [3] %Error: b01_nested_qual.sv:9: Assertion failed in top: 'unique if' statement violated
/// t=3 else-priority-if done
/// [4] %Error: b01_nested_qual.sv:11: Assertion failed in top: 'unique if' statement violated
/// t=4 u0-else-unique-if done
/// t=5 else-labeled-if done
/// [6] %Error: b01_nested_qual.sv:15: Assertion failed in top: 'unique if' statement violated
/// t=6 chain3-dup done
/// - b01_nested_qual.sv:17: Verilog $finish
/// ```
/// t6 (a three-`if` chain) reports at the outer `if` now. t2/t3/t4 report once, at the
/// inner qualified `if`, exactly as before the slice (Verilator names the outer line).
/// A labelled statement after `else` (t5) parses as a block and ends the series: silent
/// in both tools. Residue: t1 (`else unique0 if`) is silent, as before the slice, where
/// Verilator reports — the qualified `if` is the outer series' final `else` statement
/// (IEEE 1800-2017 Syntax 12-2).
#[test]
fn nested_qualifiers_and_a_labelled_else() {
    let r = run("b01_nested_qual.sv", B01);
    assert_w4031(
        &r,
        &[
            w("b01_nested_qual.sv:7:41", "top", 2),
            w("b01_nested_qual.sv:9:43", "top", 3),
            w("b01_nested_qual.sv:11:42", "top", 4),
            w("b01_nested_qual.sv:15:15", "top", 6),
        ],
    );
}

/// verilator:
/// ```text
/// [3] %Error: q23_prio_u0.sv:9: Assertion failed in top: 'unique if' statement violated
/// t=6 end
/// - q23_prio_u0.sv:13: Verilog $finish
/// ```
/// A qualified `if` after `else` ends the series with no outer arm. t1, t2: a
/// `priority if` whose `else` is a `unique0 if` series is silent — IEEE 1800-2017
/// Syntax 12-2 makes the `unique0 if` the final `else` statement, §12.4.2 reports no
/// miss when there is an explicit `else`, and `unique0` suppresses its own; Verilator
/// checks no `priority if`. t3 is the residue Verilator reports (`unique if … else
/// unique0 if`), silent as before the slice. t4, t5 are silent in both tools.
#[test]
fn a_qualified_if_after_else_ends_the_series() {
    let r = run(
        "q23_prio_u0.sv",
        r#"module top;
  logic a = 0, b = 0, c = 0;
  logic [1:0] r = 0;
  initial begin
    #1 priority if (a) r = 1; else unique0 if (b) r = 2;
    #1 priority if (a) r = 1;
       else unique0 if (b) r = 2;
       else if (c) r = 3;
    #1 unique if (a) r = 1; else unique0 if (b) r = 2;
    #1 unique0 if (a) r = 1; else unique0 if (b) r = 2;
    #1 priority if (a) r = 1; else unique0 if (b) r = 2; else r = 0;
    #1 $display("t=%0t end", $time);
    #1 $finish;
  end
endmodule
"#,
    );
    assert_w4031(&r, &[]);
    assert!(r.out.contains("t=6 end\n"), "{}", r.out);
}

/// verilator (compile: `%Warning-WAITCONST: f1b_enders.sv:16:40: Wait statement
/// condition is constant`):
/// ```text
/// t=1 B1 n=0
/// t=2 B2 n=0
/// t=3 B3 n=0
/// t=5 B4 n=0
/// t=6 B5 n=0
/// t=7 B6 n=0
/// t=8 B7 n=0
/// t=12 B8 n=0
/// t=13 B9 n=0
/// - f1b_enders.sv:24: Verilog $finish
/// ```
/// No assertion line: a `begin … end` block, a labelled statement, `;`, a delay, a
/// `case`, a `wait`, an `else begin end` tail, an event control and a `repeat` after
/// `else` each end the series with no arm.
#[test]
fn a_block_or_control_after_else_ends_the_series() {
    let r = run(
        "f1b_enders.sv",
        r#"module top;
  logic a, b, c; int n; event e;
  initial #12 -> e;
  initial begin
    a = 0; b = 0; c = 0; n = 0;
    #1 unique if (a) n = 1; else begin if (b) n = 2; end
    $display("t=%0t B1 n=%0d", $time, n);
    #1 unique if (a) n = 1; else L2: if (b) n = 2;
    $display("t=%0t B2 n=%0d", $time, n);
    #1 unique if (a) n = 1; else ;
    $display("t=%0t B3 n=%0d", $time, n);
    #1 unique if (a) n = 1; else #1 if (b) n = 2;
    $display("t=%0t B4 n=%0d", $time, n);
    #1 unique if (a) n = 1; else case (b) 1'b1: n = 2; endcase
    $display("t=%0t B5 n=%0d", $time, n);
    #1 unique if (a) n = 1; else wait (1) if (b) n = 2;
    $display("t=%0t B6 n=%0d", $time, n);
    #1 unique if (a) n = 1; else if (b) n = 2; else begin end
    $display("t=%0t B7 n=%0d", $time, n);
    #1 unique if (a) n = 1; else @(e) if (b) n = 2;
    $display("t=%0t B8 n=%0d", $time, n);
    #1 unique if (a) n = 1; else repeat (1) if (b) n = 2;
    $display("t=%0t B9 n=%0d", $time, n);
    #1 $finish;
  end
endmodule
"#,
    );
    assert_w4031(&r, &[]);
    assert!(r.out.contains("t=13 B9 n=0"), "{}", r.out);
}

/// hand-IEEE §12.4.2: a `priority if` series with no final `else` reports when no
/// condition is true, like `unique if`. verilator 5.052 is silent on every
/// `priority if` (single and chain), so it is no oracle for these:
/// ```text
/// t=1 if1 done
/// t=2 chain2 done
/// t=3 chain3 done
/// t=4 chain-else done
/// t=5 else-begin-if done
/// t=6 nested-in-then done
/// t=7 dangling-else done
/// t=8 chain-match done
/// t=9 chain-match2 done
/// - m_if_priority.sv:23: Verilog $finish
/// ```
#[test]
fn priority_if_chains_report_hand_ieee() {
    let src = M_IF_UNIQUE.replace("unique if", "priority if");
    let r = run("m_if_priority.sv", &src);
    assert_w4031(
        &r,
        &[
            w("m_if_priority.sv:5:17", "top", 1),
            w("m_if_priority.sv:7:17", "top", 2),
            w("m_if_priority.sv:9:17", "top", 3),
            w("m_if_priority.sv:15:17", "top", 6),
            w("m_if_priority.sv:17:17", "top", 7),
        ],
    );
}

/// verilator on the `unique0` design: no assertion line at all.
/// ```text
/// t=1 if1 done
/// t=2 chain2 done
/// t=3 chain3 done
/// t=4 chain-else done
/// t=5 else-begin-if done
/// t=6 nested-in-then done
/// t=7 dangling-else done
/// t=8 chain-match done
/// t=9 chain-match2 done
/// - m_if_unique0.sv:23: Verilog $finish
/// ```
/// `priority0` is rejected by verilator 5.052 (`syntax error, unexpected if`); vita
/// reads it like `unique0` (`unique0_priority0.rs`). Both suppress the no-match
/// report, so their chains stay silent.
#[test]
fn unique0_and_priority0_chains_stay_silent() {
    let r = run(
        "m_if_unique0.sv",
        &M_IF_UNIQUE.replace("unique if", "unique0 if"),
    );
    assert_w4031(&r, &[]);
    assert!(r.out.contains("t=9 chain-match2 done"), "{}", r.out);
    let r = run(
        "m_if_priority0.sv",
        &M_IF_UNIQUE.replace("unique if", "priority0 if"),
    );
    assert_w4031(&r, &[]);
}

/// verilator:
/// ```text
/// [1] %Error: lt_P.sv:9: Assertion failed in top.st: 'unique if' statement violated
/// t=1 static task done
/// [2] %Error: lt_P.sv:2: Assertion failed in pk.pt: 'unique if' statement violated
/// t=2 pkg task done
/// [3] %Error: lt_P.sv:5: Assertion failed in $unit.K.m: 'unique if' statement violated
/// t=3 class task done
/// [4] %Error: lt_P.sv:16: Assertion failed in top: 'unique if' statement violated
/// t=4 fork child done
/// - lt_P.sv:17: Verilog $finish
/// [5] %Error: lt_P.sv:19: Assertion failed in top: 'unique if' statement violated
/// ```
/// The `fork` branch (t4) and the `final` block (t5) report as Verilator does. Residue:
/// the chains in the static task (t1), the package task (t2) and the class task (t3)
/// keep the lone-`if` rule of every subroutine body and stay silent, as before the
/// slice, where Verilator reports (ROADMAP §3.b `unique-if-chain`).
#[test]
fn chains_in_fork_and_final_report_task_chains_do_not() {
    let r = run(
        "lt_P.sv",
        r#"package pk;
  task pt(input logic x, input logic z, output logic [1:0] o); o = 0; unique if (x) o = 1; else if (z) o = 2; endtask
endpackage
class K;
  task m(input logic x, input logic z); unique if (x) $display("cx"); else if (z) $display("cz"); endtask
endclass
module top; import pk::*;
  logic a = 0, b = 0; logic [1:0] y;
  task st(input logic x, input logic z, output logic [1:0] o); o = 0; unique if (x) o = 1; else if (z) o = 2; endtask
  K k;
  initial begin
    k = new;
    #1 st(a, b, y);     $display("t=%0t static task done", $time);
    #1 pt(a, b, y);     $display("t=%0t pkg task done", $time);
    #1 k.m(a, b);       $display("t=%0t class task done", $time);
    #1 fork begin unique if (a) $display("cx"); else if (b) $display("cz"); end join $display("t=%0t fork child done", $time);
    #1 $finish;
  end
  final begin unique if (a) $display("cx"); else if (b) $display("cz"); end
endmodule
"#,
    );
    assert_w4031(
        &r,
        &[w("lt_P.sv:16:26", "top", 4), w("lt_P.sv:19:22", "top", 5)],
    );
    assert!(r.out.contains("t=3 class task done\n"), "{}", r.out);
}

/// verilator:
/// ```text
/// [1] %Error: b02_frame_lanes.sv:7: Assertion failed in top.fchain: 'unique if' statement violated
/// t=1 fchain done
/// [2] %Error: b02_frame_lanes.sv:13: Assertion failed in top.fsingle: 'unique if' statement violated
/// t=2 fsingle done
/// [3] %Error: b02_frame_lanes.sv:18: Assertion failed in top.tchain: 'unique if' statement violated
/// t=3 tchain done
/// [4] %Error: b02_frame_lanes.sv:22: Assertion failed in top.tsingle: 'unique if' statement violated
/// t=4 tsingle done
/// - b02_frame_lanes.sv:29: Verilog $finish
/// ```
/// A lone `unique if` in a function or task body reports, as before the slice (t2, t4).
/// Residue: the chains in the function `fchain` (t1) and the task `tchain` (t3) stay
/// silent where Verilator reports: arming a function body would refuse a constant
/// function that reaches the no-match (`function_body_chain_still_folds`, ROADMAP §3.b
/// `unique-const-fn`) and let a continuous assign report at time 0 (ROADMAP §2 🆕 AB).
#[test]
fn subroutine_chains_stay_silent_lone_ifs_report() {
    let r = run(
        "b02_frame_lanes.sv",
        r#"module top;
  logic a = 0, b = 0;
  logic [1:0] r = 0, y;
  function automatic logic [1:0] fchain(input logic x, input logic z);
    logic [1:0] v;
    v = 0;
    unique if (x) v = 1; else if (z) v = 2;
    return v;
  endfunction
  function automatic logic [1:0] fsingle(input logic x);
    logic [1:0] v;
    v = 0;
    unique if (x) v = 1;
    return v;
  endfunction
  task automatic tchain(input logic x, input logic z, output logic [1:0] o);
    o = 0;
    unique if (x) o = 1; else if (z) o = 2;
  endtask
  task automatic tsingle(input logic x, output logic [1:0] o);
    o = 0;
    unique if (x) o = 1;
  endtask
  initial begin
    #1 y = fchain(a, b);  $display("t=%0t fchain done", $time);
    #1 y = fsingle(a);    $display("t=%0t fsingle done", $time);
    #1 tchain(a, b, y);   $display("t=%0t tchain done", $time);
    #1 tsingle(a, y);     $display("t=%0t tsingle done", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_w4031(
        &r,
        &[
            w("b02_frame_lanes.sv:13:12", "top.fsingle", 2),
            w("b02_frame_lanes.sv:22:12", "top.tsingle", 4),
        ],
    );
    assert!(r.out.contains("t=3 tchain done"), "{}", r.out);
}

/// verilator:
/// ```text
/// eval t=0 ab=10
/// eval t=2 ab=00
/// [2] %Error: b03.sv:6: Assertion failed in top: 'unique if' statement violated
/// eval t=3 ab=00
/// [3] %Error: b03.sv:6: Assertion failed in top: 'unique if' statement violated
/// - b03.sv:13: Verilog $finish
/// eval t=4 ab=00
/// [4] %Error: b03.sv:6: Assertion failed in top: 'unique if' statement violated
/// ```
/// The zero-delay glitch at t2 reports at once, as in Verilator; t3 is a real
/// no-match (Verilator's t4 line runs after `$finish`). The `always_comb` written
/// before the `initial` that drives it waits for its inputs at time 0 (§2 🆕 Z,
/// closed), so it first runs on `ab=10` and nothing reports at time 0.
#[test]
fn always_comb_chain_reports_the_glitch() {
    let r = run(
        "b03.sv",
        r#"module top;
  logic a, b;
  logic [1:0] y;
  always_comb begin
    $display("eval t=%0t ab=%b%b", $time, a, b);
    unique if (a) y = 1; else if (b) y = 2;
  end
  initial begin
    a = 1; b = 0;
    #2 a = 0;
    #0 b = 1;
    #1 a = 0; b = 0;
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_w4031(
        &r,
        &[w("b03.sv:6:12", "top", 2), w("b03.sv:6:12", "top", 3)],
    );
    assert!(r.out.starts_with("eval t=0 ab=10\n"), "{}", r.out);
}

/// verilator: `P0=0 P1=1` (no assertion line; both oracles fold the constant
/// function silently).
/// ```text
/// P0=0 P1=1
/// - l08_P.sv:8: Verilog $finish
/// ```
/// An armed chain tail here would be a `$`-task that the elaborate-time
/// constant-function interpreter refuses (`VITA-E3009`) on the `P0 = cf(0, 0)`
/// no-match path; the function-body rule keeps it folding.
#[test]
fn function_body_chain_still_folds() {
    let r = run(
        "l08_P.sv",
        r#"module top;
  function automatic int cf(input int x, input int z);
    cf = 0;
    unique if (x == 1) cf = 1; else if (z == 1) cf = 2;
  endfunction
  localparam int P0 = cf(0, 0);
  localparam int P1 = cf(1, 1);
  initial begin $display("P0=%0d P1=%0d", P0, P1); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(r.code, 0, "stderr:\n{}", r.err);
    assert!(r.out.contains("P0=0 P1=1"), "{}", r.out);
    assert!(!r.err.contains("E3009"), "{}", r.err);
    assert_w4031(&r, &[]);
}

/// verilator on `dif_c01_assert_else_if.sv` — no assertion line:
/// ```text
/// t=1 u-assert-fail-elseif y=0
/// t=2 u-assert-pass y=0
/// t=3 u-assume-fail-elseif y=0
/// t=4 u-assert-passact-fail-elseif y=0
/// t=5 p-assert-fail-elseif y=0
/// t=6 u-assert-fail-elseif-c1 y=2
/// - dif_c01_assert_else_if.sv:17: Verilog $finish
/// ```
/// `dif_c15_assert_semi_else.sv`:
/// ```text
/// [1] %Error: dif_c15_assert_semi_else.sv:5: Assertion failed in top: 'unique if' statement violated
/// t=1 u-semi-else y=0
/// t=2 plain-semi-else y=0
/// t=3 u-deferred-else y=2
/// [4] %Error: dif_c15_assert_semi_else.sv:11: Assertion failed in top: 'unique if' statement violated
/// t=4 u-assert-else-unique y=0
/// - dif_c15_assert_semi_else.sv:13: Verilog $finish
/// ```
/// `dif_c02_attr_else_if.sv`:
/// ```text
/// [1] %Error: dif_c02_attr_else_if.sv:5: Assertion failed in top: 'unique if' statement violated
/// t=1 attr-elseif done
/// [2] %Error: dif_c02_attr_else_if.sv:7: Assertion failed in top: 'unique if' statement violated
/// t=2 attr-after-unique done
/// - dif_c02_attr_else_if.sv:9: Verilog $finish
/// ```
/// `f1c2_qual.sv`:
/// ```text
/// [1] %Error: f1c2_qual.sv:5: Assertion failed in top: 'unique if' statement violated
/// t=1 C1 n=0
/// [2] %Error: f1c2_qual.sv:7: Assertion failed in top: 'unique if' statement violated
/// t=2 C2 n=0
/// [3] %Error: f1c2_qual.sv:9: Assertion failed in top: 'unique if' statement violated
/// t=3 C3 n=0
/// [4] %Error: f1c2_qual.sv:11: Assertion failed in top: 'unique if' statement violated
/// t=4 C4 n=0
/// t=5 C5 n=0
/// [6] %Error: f1c2_qual.sv:17: Assertion failed in top: 'unique if' statement violated
/// t=6 C6 n=0
/// [7] %Error: f1c2_qual.sv:19: Assertion failed in top: 'unique if' statement violated
/// t=7 C7 n=0
/// [8] %Error: f1c2_qual.sv:21: Assertion failed in top: 'unique if' statement violated
/// t=8 C8 n=0
/// [9] %Error: f1c2_qual.sv:23: Assertion failed in top: 'unique if' statement violated
/// t=9 C9 n=0
/// t=10 C10 n=0
/// t=11 C11 n=0
/// t=12 C12 n=9
/// [13] %Error: f1c2_qual.sv:31: Assertion failed in top: 'unique if' statement violated
/// t=13 C13 n=9
/// - f1c2_qual.sv:35: Verilog $finish
/// ```
/// An immediate `assert` / `assume` after `else` is a `Stmt::If` in the tree, but the
/// written token after `else` is not `if`, so it ends the series: no arm, and the
/// `else if` inside the assertion's fail action still runs (t6 `y=2`). An attribute
/// after `else` or after the qualifier is followed (`dif_c02`, `f1c2_qual` C1). A
/// `priority if` series is hand-IEEE (`f1c2_qual` C11, see
/// `priority_if_chains_report_hand_ieee`); an inner qualified `if` reports at its own
/// column (`f1c2_qual` C3, C8; `dif_c15` t4). Residue: `f1c2_qual` C2 and C7
/// (`else unique0 if`) are silent, as before the slice, where Verilator reports (IEEE
/// 1800-2017 Syntax 12-2, see `a_qualified_if_after_else_ends_the_series`).
#[test]
fn an_assert_or_attribute_after_else() {
    let r = run(
        "dif_c01_assert_else_if.sv",
        r#"module top;
  logic a, b, c; logic [1:0] y;
  initial begin
    a = 0; b = 0; c = 0; y = 0;
    #1 unique if (a) y = 1; else assert (b) else if (c) y = 2;
    $display("t=%0t u-assert-fail-elseif y=%0d", $time, y);
    #1 b = 1; unique if (a) y = 1; else assert (b) else if (c) y = 2;
    $display("t=%0t u-assert-pass y=%0d", $time, y);
    #1 b = 0; unique if (a) y = 1; else assume (b) else if (c) y = 2;
    $display("t=%0t u-assume-fail-elseif y=%0d", $time, y);
    #1 unique if (a) y = 1; else assert (b) $display("  pass-action"); else if (c) y = 2;
    $display("t=%0t u-assert-passact-fail-elseif y=%0d", $time, y);
    #1 priority if (a) y = 1; else assert (b) else if (c) y = 2;
    $display("t=%0t p-assert-fail-elseif y=%0d", $time, y);
    #1 c = 1; unique if (a) y = 1; else assert (b) else if (c) y = 2;
    $display("t=%0t u-assert-fail-elseif-c1 y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_w4031(&r, &[]);
    assert!(
        r.out.contains("t=6 u-assert-fail-elseif-c1 y=2\n"),
        "{}",
        r.out
    );
    let r = run(
        "dif_c15_assert_semi_else.sv",
        r#"module top;
  logic a, b, c; logic [1:0] y;
  initial begin
    a = 0; b = 0; c = 1; y = 0;
    #1 unique if (a) y = 1; else if (b) assert (c); else y = 3;
    $display("t=%0t u-semi-else y=%0d", $time, y);
    #1 y = 0; if (b) assert (c); else y = 3;
    $display("t=%0t plain-semi-else y=%0d", $time, y);
    #1 y = 0; unique if (a) y = 1; else assert #0 (b) else if (c) y = 2;
    $display("t=%0t u-deferred-else y=%0d", $time, y);
    #1 y = 0; unique if (a) y = 1; else assert (b) else unique if (a) y = 2;
    $display("t=%0t u-assert-else-unique y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_w4031(
        &r,
        &[
            w("dif_c15_assert_semi_else.sv:5:15", "top", 1),
            w("dif_c15_assert_semi_else.sv:11:64", "top", 4),
        ],
    );
    let r = run(
        "dif_c02_attr_else_if.sv",
        r#"module top;
  logic a, b; logic [1:0] y;
  initial begin
    a = 0; b = 0; y = 0;
    #1 unique if (a) y = 1; else (* mark *) if (b) y = 2;
    $display("t=%0t attr-elseif done", $time);
    #1 unique (* mark *) if (a) y = 1; else if (b) y = 2;
    $display("t=%0t attr-after-unique done", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_w4031(
        &r,
        &[
            w("dif_c02_attr_else_if.sv:5:15", "top", 1),
            w("dif_c02_attr_else_if.sv:7:26", "top", 2),
        ],
    );
    let r = run(
        "f1c2_qual.sv",
        r#"module top;
  logic a, b, c, d; int n;
  initial begin
    a = 0; b = 0; c = 0; d = 0; n = 0;
    #1 unique if (a) n = 1; else (* mark *) if (b) n = 2;
    $display("t=%0t C1 n=%0d", $time, n);
    #1 unique if (a) n = 1; else unique0 if (b) n = 2;
    $display("t=%0t C2 n=%0d", $time, n);
    #1 unique if (a) n = 1; else priority if (b) n = 2;
    $display("t=%0t C3 n=%0d", $time, n);
    #1 unique if (a) begin if (c) n = 3; else if (d) n = 4; end else if (b) n = 2;
    $display("t=%0t C4 n=%0d", $time, n);
    a = 1;
    #1 unique if (a) if (c) n = 3; else if (b) n = 2;
    $display("t=%0t C5 n=%0d", $time, n);
    a = 0;
    #1 unique if (a) if (c) n = 3; else if (b) n = 2;
    $display("t=%0t C6 n=%0d", $time, n);
    #1 unique if (a) n = 1; else unique0 if (b) n = 2; else if (c) n = 3;
    $display("t=%0t C7 n=%0d", $time, n);
    #1 unique if (a) n = 1; else unique if (b) n = 2; else if (c) n = 3;
    $display("t=%0t C8 n=%0d", $time, n);
    #1 unique if (a) n = 1; else if (b) n = 2; else if (c) n = 3; else if (d) n = 4;
    $display("t=%0t C9 n=%0d", $time, n);
    #1 unique0 if (a) n = 1; else if (b) n = 2;
    $display("t=%0t C10 n=%0d", $time, n);
    #1 priority if (a) n = 1; else if (b) n = 2;
    $display("t=%0t C11 n=%0d", $time, n);
    #1 unique if (a) n = 1; else if (b) n = 2; else n = 9;
    $display("t=%0t C12 n=%0d", $time, n);
    #1 unique if (a) n = 1;
       else
         if (b) n = 2;
    $display("t=%0t C13 n=%0d", $time, n);
    #1 $finish;
  end
endmodule
"#,
    );
    assert_w4031(
        &r,
        &[
            w("f1c2_qual.sv:5:15", "top", 1),
            w("f1c2_qual.sv:9:43", "top", 3),
            w("f1c2_qual.sv:11:15", "top", 4),
            w("f1c2_qual.sv:17:15", "top", 6),
            w("f1c2_qual.sv:21:41", "top", 8),
            w("f1c2_qual.sv:23:15", "top", 9),
            w("f1c2_qual.sv:27:17", "top", 11),
            w("f1c2_qual.sv:31:15", "top", 13),
        ],
    );
    assert!(r.out.contains("t=13 C13 n=9\n"), "{}", r.out);
}

/// Residue — verilator, `m_void_rt.sv` (module `function void`), `ifc_vfn_rt.sv`
/// (interface), `pkg_vfn3_imp_rt.sv` (package `function void` and task, imported),
/// `cls_task_rt.sv` (class task):
/// ```text
/// [1] %Error: m_void_rt.sv:4: Assertion failed in top.fv: 'unique if' statement violated
/// t=1 q=7
/// - m_void_rt.sv:7: Verilog $finish
/// [1] %Error: ifc_vfn_rt.sv:4: Assertion failed in top.i.fv: 'unique if' statement violated
/// t=1 q=7
/// - ifc_vfn_rt.sv:9: Verilog $finish
/// [1] %Error: pkg_vfn3_imp_rt.sv:4: Assertion failed in pk.fv: 'unique if' statement violated
/// t=1
/// [3] %Error: pkg_vfn3_imp_rt.sv:8: Assertion failed in pk.t: 'unique if' statement violated
/// t=3
/// - pkg_vfn3_imp_rt.sv:13: Verilog $finish
/// [1] %Error: cls_task_rt.sv:5: Assertion failed in $unit.C.t: 'unique if' statement violated
/// t=1 r=7
/// - cls_task_rt.sv:10: Verilog $finish
/// ```
/// `n_comb_vfn_tbL.sv` (a `function void` called from an `always_comb`):
/// ```text
/// t=1 y=2
/// [2] %Error: n_comb_vfn_tbL.sv:4: Assertion failed in top.u.fv: 'unique if' statement violated
/// [2] %Error: n_comb_vfn_tbL.sv:4: Assertion failed in top.u.fv: 'unique if' statement violated
/// t=3 y=0
/// [3] %Error: n_comb_vfn_tbL.sv:4: Assertion failed in top.u.fv: 'unique if' statement violated
/// [3] %Error: n_comb_vfn_tbL.sv:4: Assertion failed in top.u.fv: 'unique if' statement violated
/// - n_comb_vfn_tbL.sv:16: Verilog $finish
/// [4] %Error: n_comb_vfn_tbL.sv:4: Assertion failed in top.u.fv: 'unique if' statement violated
/// [4] %Error: n_comb_vfn_tbL.sv:4: Assertion failed in top.u.fv: 'unique if' statement violated
/// ```
/// Every function and task body keeps the lone-`if` rule, so each chain stays silent,
/// as before the slice, where Verilator reports (ROADMAP §3.b `unique-if-chain`).
#[test]
fn void_function_and_task_chains_stay_silent() {
    let r = run(
        "m_void_rt.sv",
        r#"module top;
  function void fv(input int x, output int r);
    r = 7;
    unique if (x == 1) r = 1; else if (x == 2) r = 2;
  endfunction
  int q;
  initial begin #1 fv(0, q); $display("t=%0t q=%0d", $time, q); #1 $finish; end
endmodule
"#,
    );
    assert_w4031(&r, &[]);
    assert!(r.out.contains("t=1 q=7\n"), "{}", r.out);
    let r = run(
        "ifc_vfn_rt.sv",
        r#"interface I;
  function void fv(input int x, output int r);
    r = 7;
    unique if (x == 1) r = 1; else if (x == 2) r = 2;
  endfunction
endinterface
module top;
  I i(); int q;
  initial begin #1 i.fv(0, q); $display("t=%0t q=%0d", $time, q); #1 $finish; end
endmodule
"#,
    );
    assert_w4031(&r, &[]);
    let r = run(
        "pkg_vfn3_imp_rt.sv",
        r#"package pk;
  function void fv(input int x);
    int r; r = 7;
    unique if (x == 1) r = 1; else if (x == 2) r = 2;
  endfunction
  task t(input int x);
    int r; r = 7;
    unique if (x == 1) r = 1; else if (x == 2) r = 2;
  endtask
endpackage
module top;
  import pk::*;
  initial begin #1 fv(0); $display("t=%0t", $time); #2 t(0); $display("t=%0t", $time); #1 $finish; end
endmodule
"#,
    );
    assert_w4031(&r, &[]);
    assert!(r.out.contains("t=3\n"), "{}", r.out);
    let r = run(
        "cls_task_rt.sv",
        r#"class C;
  int r;
  task t(input int x);
    r = 7;
    unique if (x == 1) r = 1; else if (x == 2) r = 2;
  endtask
endclass
module top;
  C c;
  initial begin c = new; #1 c.t(0); $display("t=%0t r=%0d", $time, c.r); #1 $finish; end
endmodule
"#,
    );
    assert_w4031(&r, &[]);
    assert!(r.out.contains("t=1 r=7\n"), "{}", r.out);
    let r = run(
        "n_comb_vfn_tbL.sv",
        r#"module dut(input logic a, input logic b, output logic [1:0] y);
  function void fv(input logic x, input logic z, output logic [1:0] r);
    r = 0;
    unique if (x) r = 1; else if (z) r = 2;
  endfunction
  always_comb fv(a, b, y);
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
    assert_w4031(&r, &[]);
    assert_eq!(
        r.out, "t=1 y=2\nt=3 y=0\nsimulation ended (Finish) at time 4\n",
        "stderr:\n{}",
        r.err
    );
}

/// Residue — verilator, `cls_fn_rt.sv`, `cls_vfn_rt.sv`, `cls_ctor_rt.sv`:
/// ```text
/// [1] %Error: cls_fn_rt.sv:4: Assertion failed in $unit.C.f: 'unique if' statement violated
/// t=1 f0=7
/// - cls_fn_rt.sv:10: Verilog $finish
/// [1] %Error: cls_vfn_rt.sv:5: Assertion failed in $unit.C.m: 'unique if' statement violated
/// t=1 r=7
/// - cls_vfn_rt.sv:10: Verilog $finish
/// [1] %Error: cls_ctor_rt.sv:5: Assertion failed in $unit.C.new: 'unique if' statement violated
/// t=1 r=7
/// - cls_ctor_rt.sv:10: Verilog $finish
/// ```
/// Every class function, the constructor included, keeps the lone-`if` rule, so these
/// chains report nothing where Verilator reports (ROADMAP §3.b `unique-if-chain`). The
/// tests below show why a function body cannot be armed yet: a continuous assign
/// reaches it at time 0.
#[test]
fn class_function_chains_keep_the_lone_if_rule() {
    let r = run(
        "cls_fn_rt.sv",
        r#"class C;
  function int f(input int x);
    int r; r = 7;
    unique if (x == 1) r = 1; else if (x == 2) r = 2;
    return r;
  endfunction
endclass
module top;
  C c;
  initial begin c = new; #1 $display("t=%0t f0=%0d", $time, c.f(0)); #1 $finish; end
endmodule
"#,
    );
    assert_w4031(&r, &[]);
    assert!(r.out.contains("t=1 f0=7\n"), "{}", r.out);
    let r = run(
        "cls_vfn_rt.sv",
        r#"class C;
  int r;
  function void m(input int x);
    r = 7;
    unique if (x == 1) r = 1; else if (x == 2) r = 2;
  endfunction
endclass
module top;
  C c;
  initial begin c = new; #1 c.m(0); $display("t=%0t r=%0d", $time, c.r); #1 $finish; end
endmodule
"#,
    );
    assert_w4031(&r, &[]);
    assert!(r.out.contains("t=1 r=7\n"), "{}", r.out);
    let r = run(
        "cls_ctor_rt.sv",
        r#"class C;
  int r;
  function new(input int x);
    r = 7;
    unique if (x == 1) r = 1; else if (x == 2) r = 2;
  endfunction
endclass
module top;
  C c;
  initial begin #1 c = new(0); $display("t=%0t r=%0d", $time, c.r); #1 $finish; end
endmodule
"#,
    );
    assert_w4031(&r, &[]);
    assert!(r.out.contains("t=1 r=7\n"), "{}", r.out);
}

/// verilator, `n_ca_objf2_tbL.sv` (a continuous assign calls a class function through
/// a handle):
/// ```text
/// t=1 y=2
/// [2] %Error: n_ca_objf2_tbL.sv:4: Assertion failed in $unit.C.f: 'unique if' statement violated
/// [2] %Error: n_ca_objf2_tbL.sv:4: Assertion failed in $unit.C.f: 'unique if' statement violated
/// t=3 y=0
/// [3] %Error: n_ca_objf2_tbL.sv:4: Assertion failed in $unit.C.f: 'unique if' statement violated
/// [3] %Error: n_ca_objf2_tbL.sv:4: Assertion failed in $unit.C.f: 'unique if' statement violated
/// - n_ca_objf2_tbL.sv:20: Verilog $finish
/// [4] %Error: n_ca_objf2_tbL.sv:4: Assertion failed in $unit.C.f: 'unique if' statement violated
/// [4] %Error: n_ca_objf2_tbL.sv:4: Assertion failed in $unit.C.f: 'unique if' statement violated
/// ```
/// Silent at every time (ROADMAP §3.b `unique-if-chain`). Armed — the `_H` spelling,
/// the chain written as a lone `unique if`, on the pre-slice binary — it prints
/// `n_ca_objf2_tbL_H.sv:4:31 … [in top.C.f] [at time 0]` ×4 through the continuous
/// assign, where Verilator prints nothing at time 0 (ROADMAP §2 🆕 AB).
#[test]
fn a_class_function_a_continuous_assign_calls_stays_silent() {
    let r = run(
        "n_ca_objf2_tbL.sv",
        r#"class C;
  function logic [1:0] f(input logic x, input logic z);
    logic [1:0] r; r = 0;
    unique if (x) r = 1; else if (z) r = 2;
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
    assert_w4031(&r, &[]);
    assert_eq!(
        r.out, "t=1 y=2\nt=3 y=0\nsimulation ended (Finish) at time 4\n",
        "stderr:\n{}",
        r.err
    );
}

/// verilator, `t2t_c_fg_this.sv` (the class function a continuous assign calls calls a
/// class void method through `this.`):
/// ```text
/// t=1 y=1
/// [2] %Error: t2t_c_fg_this.sv:4: Assertion failed in $unit.C.g: 'unique if' statement violated
/// [2] %Error: t2t_c_fg_this.sv:4: Assertion failed in $unit.C.g: 'unique if' statement violated
/// t=3 y=0
/// [3] %Error: t2t_c_fg_this.sv:4: Assertion failed in $unit.C.g: 'unique if' statement violated
/// [3] %Error: t2t_c_fg_this.sv:4: Assertion failed in $unit.C.g: 'unique if' statement violated
/// - t2t_c_fg_this.sv:23: Verilog $finish
/// [4] %Error: t2t_c_fg_this.sv:4: Assertion failed in $unit.C.g: 'unique if' statement violated
/// [4] %Error: t2t_c_fg_this.sv:4: Assertion failed in $unit.C.g: 'unique if' statement violated
/// ```
/// Silent at every time (ROADMAP §3.b `unique-if-chain`). Armed — the `_H` spelling on
/// the pre-slice binary, and a candidate build of this slice that armed every class
/// method — it prints `t2t_c_fg_this_H.sv:4:31 … [in top.C.g] [at time 0]` ×2, where
/// Verilator prints nothing at time 0 (ROADMAP §2 🆕 AB).
#[test]
fn a_void_method_called_through_this_stays_silent_at_time_0() {
    let r = run(
        "t2t_c_fg_this.sv",
        r#"class C;
  function void g(input logic x, input logic z);
    logic [1:0] r; r = 0;
    unique if (x) r = 1; else if (z) r = 2;
  endfunction
  function logic [1:0] f(input logic x, input logic z);
    this.g(x, z);
    return {x, z};
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
    assert_w4031(&r, &[]);
    assert_eq!(
        r.out, "t=1 y=1\nt=3 y=0\nsimulation ended (Finish) at time 4\n",
        "stderr:\n{}",
        r.err
    );
}

/// verilator, `u20_c_fnew_member.sv` (the class function a continuous assign calls
/// constructs a member handle with `new`):
/// ```text
/// t=1 y=1
/// [2] %Error: u20_c_fnew_member.sv:5: Assertion failed in $unit.D.new: 'unique if' statement violated
/// [2] %Error: u20_c_fnew_member.sv:5: Assertion failed in $unit.D.new: 'unique if' statement violated
/// t=3 y=0
/// [3] %Error: u20_c_fnew_member.sv:5: Assertion failed in $unit.D.new: 'unique if' statement violated
/// [3] %Error: u20_c_fnew_member.sv:5: Assertion failed in $unit.D.new: 'unique if' statement violated
/// - u20_c_fnew_member.sv:27: Verilog $finish
/// [4] %Error: u20_c_fnew_member.sv:5: Assertion failed in $unit.D.new: 'unique if' statement violated
/// [4] %Error: u20_c_fnew_member.sv:5: Assertion failed in $unit.D.new: 'unique if' statement violated
/// ```
/// Silent at every time (ROADMAP §3.b `unique-if-chain`). Armed — the `_H` spelling on
/// the pre-slice binary — it prints `u20_c_fnew_member_H.sv:5:31 … [in top.D.new]
/// [at time 0]` ×2, where Verilator prints nothing at time 0 (ROADMAP §2 🆕 AB). The
/// two `VITA-W4020` lines are the time-0 run of `d = new(…)` through a handle not yet
/// constructed, before this slice too.
#[test]
fn a_constructor_called_through_new_stays_silent_at_time_0() {
    let r = run(
        "u20_c_fnew_member.sv",
        r#"class D;
  logic [1:0] r;
  function new(input logic x, input logic z);
    r = 0;
    unique if (x) r = 1; else if (z) r = 2;
  endfunction
endclass
class C;
  D d;
  function logic [1:0] f(input logic x, input logic z);
    d = new(x, z);
    return {x, z};
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
    assert_w4031(&r, &[]);
    assert_eq!(
        r.out, "t=1 y=1\nt=3 y=0\nsimulation ended (Finish) at time 4\n",
        "stderr:\n{}",
        r.err
    );
    assert_eq!(r.err.matches("VITA-W4020").count(), 2, "{}", r.err);
}

/// verilator, `q6a_fn_vfn_noformal.sv` (a continuous assign calls `f`, which calls a
/// formal-less module `function void g()`):
/// ```text
/// t=1 y=1
/// [2] %Error: q6a_fn_vfn_noformal.sv:3: Assertion failed in top.u.g: 'unique if' statement violated
/// [2] %Error: q6a_fn_vfn_noformal.sv:3: Assertion failed in top.u.g: 'unique if' statement violated
/// t=3 y=0
/// [3] %Error: q6a_fn_vfn_noformal.sv:3: Assertion failed in top.u.g: 'unique if' statement violated
/// [3] %Error: q6a_fn_vfn_noformal.sv:3: Assertion failed in top.u.g: 'unique if' statement violated
/// - q6a_fn_vfn_noformal.sv:19: Verilog $finish
/// [4] %Error: q6a_fn_vfn_noformal.sv:3: Assertion failed in top.u.g: 'unique if' statement violated
/// [4] %Error: q6a_fn_vfn_noformal.sv:3: Assertion failed in top.u.g: 'unique if' statement violated
/// ```
/// iverilog on the `unique case` twin `q6a_fn_vfn_noformal_case.sv`:
/// ```text
/// t=1 y=1
/// WARNING: q6a_fn_vfn_noformal_case.sv:3: value is unhandled for priority or unique case statement
///          Time: 2  Scope: top.u.g
/// t=3 y=0
/// q6a_fn_vfn_noformal_case.sv:19: $finish called at 4 (1s)
/// ```
/// Silent at every time (ROADMAP §3.b `unique-if-chain`). Armed — the `_H` spelling on
/// the pre-slice binary, and the `unique case` twin on this one — it reports
/// `… [in top.u.g] [at time 0]` and again at time 2: `f` runs `g()` in the continuous
/// assign's time-0 pass, where both tools are silent at time 0 (ROADMAP §2 🆕 AB).
#[test]
fn a_void_function_a_continuous_assign_reaches_stays_silent_at_time_0() {
    let r = run(
        "q6a_fn_vfn_noformal.sv",
        r#"module dut(input logic a, input logic b, output logic [1:0] y);
  function void g();
    unique if (a) ; else if (b) ;
  endfunction
  function logic [1:0] f(input logic x, input logic z);
    g();
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
"#,
    );
    assert_w4031(&r, &[]);
    assert_eq!(
        r.out, "t=1 y=1\nt=3 y=0\nsimulation ended (Finish) at time 4\n",
        "stderr:\n{}",
        r.err
    );
}

/// verilator, `q6t_pkg_scoped_const_chain.sv` (a continuous assign calls `pk::f`, which
/// calls a package `function void g()` holding the chain):
/// ```text
/// [0] %Error: q6t_pkg_scoped_const_chain.sv:4: Assertion failed in pk.g: 'unique if' statement violated
/// t=1 y=1
/// t=3 y=0
/// - q6t_pkg_scoped_const_chain.sv:22: Verilog $finish
/// ```
/// The design runs, as before the slice; the chain is the residue Verilator reports at
/// time 0. Armed, the package-scoped call closure walk would refuse it (`VITA-E3009`
/// naming `pk::g`), as it refuses the `_H` spelling and the `unique case` twin.
#[test]
fn a_package_scoped_call_reaching_a_chain_still_runs() {
    let r = run(
        "q6t_pkg_scoped_const_chain.sv",
        r#"package pk;
  localparam int MODE = 2;
  function void g();
    unique if (MODE == 0) begin end else if (MODE == 1) begin end
  endfunction
  function logic [1:0] f(input logic x, input logic z);
    g();
    return {x, z};
  endfunction
endpackage
module dut(input logic a, input logic b, output logic [1:0] y);
  assign y = pk::f(a, b);
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
    assert_w4031(&r, &[]);
    assert!(!r.err.contains("error["), "{}", r.err);
    assert_eq!(
        r.out, "t=1 y=1\nt=3 y=0\nsimulation ended (Finish) at time 4\n",
        "stderr:\n{}",
        r.err
    );
}

/// Recorded splits, not expected values: the slice arms procedural chains, so two
/// time-0 order splits that a lone `unique if` and `unique case` already show reach
/// chains too. Pinned so an order change moves them.
///
/// `aa1_chain.sv` (ROADMAP §2 🆕 AA, an `always_comb` read before the `always_comb`
/// that drives its input) — verilator, no assertion line:
/// ```text
/// t=6 y=3 done
/// - aa1_chain.sv:11: Verilog $finish
/// ```
/// iverilog on the `unique case` twin `a08_comb_chain.sv`, which vita also reports at
/// time 0 and t5:
/// ```text
/// a08_comb_chain.sv:6: vvp.tgt sorry: Case unique/unique0 qualities are ignored. run rc=0
/// evalcase t=0 ab=00
/// evalcase t=5 ab=10
/// WARNING: a08_comb_chain.sv:6: value is unhandled for priority or unique case statement
///          Time: 5  Scope: top
/// evalcase t=5 ab=11
/// t=6 y=3 done
/// a08_comb_chain.sv:15: $finish called at 7 (1s)
/// ```
/// `s1s_selftimed_first.sv` (a self-timed `always` written before the `initial` that
/// drives it) — verilator:
/// ```text
/// [1] %Error: s1s_selftimed_first.sv:5: Assertion failed in top.chk: 'unique if' statement violated
/// - s1s_selftimed_first.sv:8: Verilog $finish
/// ```
/// iverilog on the `unique case` twin `s1s_case.sv` (vita = iverilog there):
/// ```text
/// WARNING: s1s_case.sv:5: value is unhandled for priority or unique case statement
///          Time: 0  Scope: top.chk
/// WARNING: s1s_case.sv:5: value is unhandled for priority or unique case statement
///          Time: 1  Scope: top.chk
/// s1s_case.sv:8: $finish called at 2 (1s)
/// ```
#[test]
fn time0_splits_reach_chains() {
    let r = run(
        "aa1_chain.sv",
        r#"module top;
  logic a = 0, b;
  logic [1:0] y;
  always_comb begin
    unique if (a && b) y = 3; else if (!a && !b) y = 0;
  end
  always_comb b = a;
  initial begin
    #5 a = 1;
    #1 $display("t=%0t y=%0d done", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
"#,
    );
    assert_w4031(
        &r,
        &[
            w("aa1_chain.sv:5:12", "top", 0),
            w("aa1_chain.sv:5:12", "top", 5),
        ],
    );
    assert!(r.out.contains("t=6 y=3 done\n"), "{}", r.out);
    let r = run(
        "s1s_selftimed_first.sv",
        r#"module top;
  logic a, b;
  always begin : chk
    logic x, z; x = a; z = b;
    unique if (x) begin end else if (z) begin end
    @(a or b);
  end
  initial begin a = 0; b = 1; #1 b = 0; #1 $finish; end
  initial #100 $finish;
endmodule
"#,
    );
    assert_w4031(
        &r,
        &[
            w("s1s_selftimed_first.sv:5:12", "top.chk", 0),
            w("s1s_selftimed_first.sv:5:12", "top.chk", 1),
        ],
    );
}
