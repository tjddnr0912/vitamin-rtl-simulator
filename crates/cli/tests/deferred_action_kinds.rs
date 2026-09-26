//! What a deferred assertion's action does at maturation depends on the kind of task it is
//! (IEEE 1800-2017 §16.4.2).
//!
//! The engine used to capture EVERY system task of a deferred action as a rendered line and
//! print that line at maturation. For a print that is the report; for anything else it printed
//! the task's arguments and dropped its effect: `else $finish;` printed an empty line and the
//! run went on to its next `$finish`, `else q.push_back(7);` printed `x 7` and left the queue
//! empty, `else $fdisplay(fd, …)` printed the descriptor to stdout and wrote nothing to the
//! file. One entry per assertion instance also kept only the LAST task of an arm, so
//! `else begin $display("A"); $display("B"); end` printed `B` alone.
//!
//! Now:
//! - a print (`$display`/`$write` families, `$strobe`, a severity task) is rendered at reach
//!   and printed at maturation, as before;
//! - a file print (`$fdisplay`, `$fwrite`, `$fstrobe`) reads its descriptor at reach with the
//!   text and writes to that descriptor at maturation;
//! - `$finish` matures as a `$finish` reached in that time step (the run ends at the step's
//!   stable point) and `$stop` as an inline `$stop` (the step's pending reports still mature);
//! - every task the taken arm reaches joins the report, in order;
//! - any other system task, and a user task or void function call, runs when reached, with
//!   W3056 saying so — the precedent the user-task call already had.
//!
//! Oracles: verilator 5.052 runs a deferred action at reach (as an immediate assertion), and
//! iverilog 13.0 refuses deferred assertions, so each cell's CONTENT is pinned to verilator
//! and to iverilog on the same design with `assert #0` spelled as an immediate `assert`. The
//! position of a matured print (after the reaching process's later statements of the same
//! time step) is hand-IEEE §16.4, the Observed region, and differs from verilator's.
//! Every cell asserts the native lines and that `--backend interp` and `--backend vm` print
//! the same.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn tmp(ext: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("vita_dak_{}_{n}.{ext}", std::process::id()))
}

/// Run on one backend; return the `T` lines plus the termination line, and the raw output.
fn run(src: &str, backend: &str) -> (Vec<String>, String) {
    let path = tmp("sv");
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["--backend", backend])
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let s =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "expected exit 0 on {backend}, got:\n{s}"
    );
    let lines = s
        .lines()
        .filter(|l| l.starts_with('T') || l.starts_with("simulation ended"))
        .map(|l| l.trim_end().to_string())
        .collect();
    (lines, s)
}

/// Native must print `want`; the interpreter and the VM must print the same. Returns the raw
/// native output for further checks.
fn check(src: &str, want: &[&str]) -> String {
    let (native, raw) = run(src, "native");
    assert_eq!(native, want, "native:\n{raw}");
    for be in ["interp", "vm"] {
        assert_eq!(run(src, be).0, native, "{be} disagrees with native");
    }
    raw
}

fn w3056(raw: &str) -> bool {
    raw.contains("VITA-W3056")
}

/// `else $finish(0);` — PRE printed `0` at maturation and ran on to 20. verilator ends at 5
/// after `after at 5`; the argument is a diagnostic level, not a value to print.
#[test]
fn a_matured_finish_ends_the_run_in_its_time_step() {
    let raw = check(
        r#"module t;
  initial begin
    #5 assert #0 (0) else $finish(0);
    $display("T after at %0t", $time);
  end
  initial begin #10 $display("T ten"); #10 $display("T twenty"); $finish; end
endmodule
"#,
        &["T after at 5", "simulation ended (Finish) at time 5"],
    );
    assert!(!w3056(&raw), "a $finish action is a report:\n{raw}");
}

/// `else $stop;` on `assert #0` and on `assert final`. PRE printed an empty line and ran on to
/// 20. vita ends a matured `$stop` as it ends an inline one: `simulation ended (Stop)`.
#[test]
fn a_matured_stop_ends_the_run_like_an_inline_stop() {
    for kw in ["#0", "final"] {
        check(
            &format!(
                r#"module t;
  initial begin
    #5 assert {kw} (0) else $stop;
    $display("T after at %0t", $time);
  end
  initial begin #10 $display("T ten"); #10 $display("T twenty"); $finish; end
endmodule
"#
            ),
            &["T after at 5", "simulation ended (Stop) at time 5"],
        );
    }
}

/// A clocked checker. Both oracles end at the edge where `n` reaches 2; PRE ran to 100.
#[test]
fn a_clocked_deferred_finish_ends_at_its_edge() {
    check(
        r#"module t;
  reg clk = 0; int n = 0;
  always #5 clk = ~clk;
  always @(posedge clk) begin
    n <= n + 1;
    assert #0 (n != 2) else $finish;
  end
  always @(posedge clk) $display("T edge n=%0d at %0t", n, $time);
  initial #100 begin $display("T hundred"); $finish; end
endmodule
"#,
        &[
            "T edge n=0 at 5",
            "T edge n=1 at 15",
            "T edge n=2 at 25",
            "simulation ended (Finish) at time 25",
        ],
    );
}

/// The assertion sits in an automatic task body (a frame). The process continues past the
/// assertion until the time step ends; verilator prints the same three lines and ends at 10.
#[test]
fn a_deferred_finish_in_a_task_body_ends_its_step() {
    check(
        r#"module t;
  task automatic chk(input int x);
    assert #0 (x != 3) else $finish;
    $display("T chk %0d at %0t", x, $time);
  endtask
  initial begin
    #5 chk(1);
    #5 chk(3);
    $display("T after at %0t", $time);
  end
  initial begin #20 $display("T twenty"); #10 $display("T thirty"); $finish; end
endmodule
"#,
        &[
            "T chk 1 at 5",
            "T chk 3 at 10",
            "T after at 10",
            "simulation ended (Finish) at time 10",
        ],
    );
}

/// Re-reaching the assertion in the same time step flushes the pending `$finish`
/// (hand-IEEE §16.4.1): `c` goes 0 then, at `#0`, back to 1, and the run goes on to 20.
/// verilator runs the action at reach and ends at 5.
#[test]
fn a_flushed_finish_does_not_end_the_run() {
    check(
        r#"module t;
  reg c = 1;
  always @(c) assert #0 (c) $display("T pass at %0t", $time); else $finish;
  initial begin
    #5 c = 0;
    #0 c = 1;
  end
  initial begin #10 $display("T ten"); #10 $display("T twenty"); $finish; end
endmodule
"#,
        &[
            "T pass at 5",
            "T ten",
            "T twenty",
            "simulation ended (Finish) at time 20",
        ],
    );
}

/// Every task of the taken arm matures, in order. PRE kept the last one (`B` alone). A
/// `begin … end` action is an extension (§16.4 asks for one subroutine call); verilator and
/// iverilog print both lines.
#[test]
fn every_task_of_the_arm_matures_in_order() {
    check(
        r#"module t;
  initial begin
    #5 assert #0 (0) else begin $display("T A at %0t", $time); $display("T B at %0t", $time); end
    $display("T after at %0t", $time);
  end
  initial begin #10 $display("T ten"); #10 $display("T twenty"); $finish; end
endmodule
"#,
        &[
            "T after at 5",
            "T A at 5",
            "T B at 5",
            "T ten",
            "T twenty",
            "simulation ended (Finish) at time 20",
        ],
    );
}

/// A matured `$finish` does not cut off the step's other reports: the next `assert #0` and
/// the `assert final` still print (verilator prints both). A matured `$stop` drains them
/// the same way before it ends the run.
#[test]
fn a_matured_control_still_matures_the_steps_other_reports() {
    for (task, end) in [
        ("$finish", "simulation ended (Finish) at time 5"),
        ("$stop", "simulation ended (Stop) at time 5"),
    ] {
        check(
            &format!(
                r#"module t;
  initial begin
    #5 assert #0 (0) else {task};
    assert #0 (0) else $display("T second at %0t", $time);
    assert final (0) else $display("T final at %0t", $time);
    $display("T after at %0t", $time);
  end
  initial begin #10 $display("T ten"); #10 $display("T twenty"); $finish; end
endmodule
"#
            ),
            &["T after at 5", "T second at 5", "T final at 5", end],
        );
    }
}

/// `$fdisplay` / `$fwrite` write to their descriptor. PRE printed the descriptor as a value on
/// stdout and the file stayed empty (`T LINE none`). Both oracles read the line back.
#[test]
fn a_file_print_writes_to_its_descriptor() {
    for (task, text) in [
        ("$fdisplay(fd, \"F at %0t\", $time)", "F at 5"),
        ("$fwrite(fd, \"W%0d\\n\", 5)", "W5"),
    ] {
        let file = tmp("txt");
        let src = format!(
            r#"module t;
  integer fd; string s;
  initial fd = $fopen("{f}", "w");
  initial begin
    #5 assert #0 (0) else {task};
    $display("T after at %0t", $time);
  end
  initial begin
    #10 $fclose(fd);
    fd = $fopen("{f}", "r");
    if ($fgets(s, fd)) $display("T LINE %s", s); else $display("T LINE none");
    $fclose(fd);
    #10 $finish;
  end
endmodule
"#,
            f = file.display()
        );
        let want_line = format!("T LINE {text}");
        check(
            &src,
            &[
                "T after at 5",
                &want_line,
                "simulation ended (Finish) at time 20",
            ],
        );
        let _ = std::fs::remove_file(&file);
    }
}

/// The STDOUT descriptor. PRE printed `F to stdout at 2147483649 5`.
#[test]
fn a_file_print_to_stdout_prints_the_text() {
    check(
        r#"module t;
  initial begin
    #5 assert #0 (0) else $fdisplay(32'h8000_0001, "T F to stdout at %0t", $time);
    $display("T after at %0t", $time);
  end
  initial begin #10 $finish; end
endmodule
"#,
        &[
            "T after at 5",
            "T F to stdout at 5",
            "simulation ended (Finish) at time 10",
        ],
    );
}

/// `$fstrobe` writes to its file with the values sampled at reach, as a deferred `$strobe`
/// prints them (§16.4.2: a pending report holds the current values of its arguments):
/// `v=1`. verilator and the immediate-assert iverilog run it at reach and sample `v=2` at the
/// end of the step. PRE printed the descriptor to stdout and wrote nothing.
#[test]
fn a_file_strobe_writes_the_reach_values_to_its_descriptor() {
    let file = tmp("txt");
    let src = format!(
        r#"module t;
  integer fd; string s; int v = 0;
  initial fd = $fopen("{f}", "w");
  initial begin
    #5 v = 1;
    assert #0 (0) else $fstrobe(fd, "S v=%0d", v);
    v = 2;
  end
  initial begin
    #10 $fclose(fd);
    fd = $fopen("{f}", "r");
    if ($fgets(s, fd)) $display("T LINE %s", s); else $display("T LINE none");
    $fclose(fd);
    #10 $finish;
  end
endmodule
"#,
        f = file.display()
    );
    check(
        &src,
        &["T LINE S v=1", "simulation ended (Finish) at time 20"],
    );
    let _ = std::fs::remove_file(&file);
}

/// Tasks that are not a report run when reached, and W3056 says so. PRE printed each one's
/// arguments at maturation and dropped its effect (`q=0 … s= … e=0 …`). Every value below
/// is what verilator prints; iverilog agrees on the queue, `$sformat`, `new[]` and the
/// insert (it refuses `putc`, `sort`, `$cast` and this associative array).
#[test]
fn a_non_report_task_runs_when_reached() {
    let raw = check(
        r#"module t;
  typedef enum {E0, E1, E2} e_t;
  int q[$]; int q2[$]; int d[]; int s3[]; int aa[int]; string s, p, n; e_t e;
  initial begin
    p = "abc"; q2 = {1, 2}; s3 = new[3]; s3[0] = 3; s3[1] = 1; s3[2] = 2; aa[3] = 1; aa[4] = 2;
    #5;
    assert #0 (0) else q.push_back(7);
    assert #0 (0) else $sformat(s, "X%0d", 5);
    assert #0 (0) else d = new[4];
    assert #0 (0) else q2.insert(0, 9);
    assert #0 (0) else $cast(e, 2);
    assert #0 (0) else p.putc(0, "Z");
    assert #0 (0) else s3.sort();
    assert #0 (0) else aa.delete(3);
    assert #0 (0) else n.itoa(42);
  end
  initial begin
    #10 $display("T q=%0d s=%s d=%0d q2=%0d/%0d e=%0d p=%s s3=%0d%0d%0d aa=%0d n=%s",
                 q.size(), s, d.size(), q2[0], q2.size(), e, p, s3[0], s3[1], s3[2],
                 aa.num(), n);
    $finish;
  end
endmodule
"#,
        &[
            "T q=1 s=X5 d=4 q2=9/3 e=2 p=Zbc s3=123 aa=1 n=42",
            "simulation ended (Finish) at time 10",
        ],
    );
    assert!(w3056(&raw), "the inline actions are announced:\n{raw}");
}

/// `$readmemh` runs when reached. PRE printed the file name and left the memory x.
#[test]
fn a_readmem_action_loads_the_memory() {
    let hex = tmp("hex");
    std::fs::write(&hex, "5a\na5\n").unwrap();
    check(
        &format!(
            r#"module t;
  reg [7:0] mem [0:1];
  initial begin
    #5 assert #0 (0) else $readmemh("{f}", mem);
  end
  initial begin #10 $display("T MEM %h %h", mem[0], mem[1]); $finish; end
endmodule
"#,
            f = hex.display()
        ),
        &["T MEM 5a a5", "simulation ended (Finish) at time 10"],
    );
    let _ = std::fs::remove_file(&hex);
}

/// `$monitor` establishes a monitor when reached (`M v=2` at the end of the step, as both
/// oracles print). PRE printed its arguments once, sampled at reach, and set no monitor.
#[test]
fn a_monitor_action_establishes_the_monitor() {
    check(
        r#"module t;
  int v = 0;
  initial begin
    #5 v = 1;
    assert #0 (0) else $monitor("T M v=%0d", v);
    v = 2;
    #3 v = 3;
  end
  initial begin #10 $finish; end
endmodule
"#,
        &["T M v=2", "T M v=3", "simulation ended (Finish) at time 10"],
    );
}

/// A static task inlined into the action runs as a whole when reached. PRE captured its body's
/// tasks one by one: the `$display` was overwritten by the `push_back`, which printed `x 7`
/// and pushed nothing (`q=0`). Both oracles: `TK at 5 k=7 v=1`, `q=1`.
#[test]
fn an_inlined_task_action_runs_as_a_whole() {
    let raw = check(
        r#"module t;
  int v = 0; int k; int q[$];
  task tk(input int a); k = a; $display("TK at %0t k=%0d v=%0d", $time, k, v); q.push_back(a); endtask
  initial begin
    #5 v = 1;
    assert #0 (0) else tk(7);
    v = 2;
  end
  initial begin #10 $display("T q=%0d", q.size()); $finish; end
endmodule
"#,
        &[
            "TK at 5 k=7 v=1",
            "T q=1",
            "simulation ended (Finish) at time 10",
        ],
    );
    assert!(w3056(&raw), "the inline call is announced:\n{raw}");
}

/// A task call beside a print in one arm (a `begin … end` action, an extension): the print is
/// the report and the call runs when reached — W3056 names it even though the arm has a report.
#[test]
fn a_task_call_beside_a_print_is_announced() {
    let raw = check(
        r#"module t;
  int k;
  task automatic tk(); k = 5; endtask
  initial begin
    #5 assert #0 (0) else begin tk(); $display("T fail k=%0d at %0t", k, $time); end
    $display("T after at %0t", $time);
  end
  initial begin #10 $finish; end
endmodule
"#,
        &[
            "T after at 5",
            "T fail k=5 at 5",
            "simulation ended (Finish) at time 10",
        ],
    );
    assert!(w3056(&raw), "the inline call is announced:\n{raw}");
}
