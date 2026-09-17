//! Multidriver Rule A (a declaration initializer plus an `always_comb` writer): a
//! callee BODY that writes the module net by name through an `input` formal is not
//! a second driver, and a frame task call's ACTUALS are reads of the inferred
//! sensitivity.
//!
//! `call_effect` answers `Unknown` for a resolvable callee whose body writes the
//! net (`task automatic tw(input int v); acc = v + 1; endtask`), and Rule A took
//! that as a driver: `always_comb tw(src);` beside `int acc = 0` was E3001 where
//! both oracles run (`ACC=8`) and verilator's MULTIDRIVEN names the `inout` actual
//! shape only. Rule A now reads an `Unknown` whose actuals are all `input` reads
//! (`call_actuals_only_read`) as a read. Accepting the shape exposed a
//! pre-existing stale value: the copy-in expressions of a frame task call live in
//! `TaskCallInfo` beside the `Terminator::Call`, not in the body's statements, so
//! `comb_read_set` never saw `src` and the block ran once (`8 / 8` for the
//! oracles' `8 / 10`, initializer-free twin identical in PRE); the read set now
//! walks the call's in-binds and out-bind indexes.
//!
//! The review measured two more holes the removed loud had hidden, closed here:
//! a frame FUNCTION's body reads are part of an `always_comb`'s sensitivity
//! (IEEE §9.2.2.2.1: "the contents of a function") — `always_comb n = rds();`
//! with `rds = src * 10` ran once (`10 / 10` for both oracles' `10 / 40`) — so
//! `comb_read_set_in` walks the callee bodies (functions only, transitively,
//! minus frame slots; NOT for `always @*`, §9.4.2.2, and not a TASK body:
//! iverilog and the LRM leave `always_comb tz();` blind to `src`, verilator does
//! not); and two `always_comb` blocks that each reach one net through a task body
//! are two drivers (Rule B counted lvalues only): `always_comb tw(src); always_comb
//! tw2(src2);` resolved by source order (`22` / `8`; verilator MULTIDRIVEN, the
//! oracles split), and is E3001 now — for an UNCONDITIONAL enable, the shape
//! verilator flags (a call under `if` beside another writer is not flagged, and
//! both oracles run it).
//!
//! ORACLES: iverilog 13.0 (-g2012) and verilator 5.052 (--binary --timing); every
//! accepted value below was measured in both. The loud cells are verilator's
//! MULTIDRIVEN shapes (an `inout` actual; two `always_comb` writers, where the
//! oracles also split: 8 / 14).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_accbw_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let so = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n");
    (
        so,
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

/// ① A body write through an `input` formal, a nested callee, an `always_ff`
/// twin and a conditional call are accepted; PRE refused `acc` and `acc2`.
#[test]
fn a_callee_body_write_through_an_input_formal_is_not_a_driver() {
    let (o, e, code) = run(r#"module t;
  int acc = 0, acc2 = 0, acc3 = 0; int src = 7;
  task automatic tw(input int v); acc = v + 1; endtask
  task automatic tn(input int v); tw(v); acc2 = v + 2; endtask
  task automatic th(input int v); acc3 = v + 3; endtask
  always_comb tn(src);
  always_ff @(posedge src[0]) th(src);
  always_comb begin if (src > 3) tw(src); end
  initial begin #1 $display("ACC=%0d %0d %0d", acc, acc2, acc3); #1 $finish; end
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "ACC=8 9 0");
}

/// ② An `inout` actual and two `always_comb` writers (one through a body) stay
/// E3001 — verilator MULTIDRIVEN on both; the oracles split on the second (8 / 14).
#[test]
fn an_inout_actual_and_a_second_comb_writer_stay_loud() {
    for src in [
        "task automatic tw(inout int v); v = v + 1; endtask\n  always_comb tw(acc);",
        "task automatic tw(input int v); acc = v + 1; endtask\n  always_comb tw(src);\n  always_comb acc = src * 2;",
    ] {
        let (_, e, code) = run(&format!(
            "module t;\n  int acc = 0; int src = 7;\n  {src}\n  initial begin #1 $finish; end\nendmodule\n"
        ));
        assert_ne!(code, Some(0), "{src}");
        assert!(
            e.contains("variable `acc` has a declaration initializer AND is written by"),
            "{src}\n{e}"
        );
    }
}

/// ③ THE EXPOSED STALE VALUE (closed): a frame task call's actuals are in the
/// inferred sensitivity — a bare net, an `output` actual's task, a named actual, a
/// default-filled call, a select actual (its index too) and an expression actual
/// all re-run the block. PRE printed the `A` line twice.
#[test]
fn a_frame_task_calls_actuals_are_reads_of_the_inferred_sensitivity() {
    let (o, e, code) = run(r#"module t;
  int acc; int src = 7; int acc2; int acc3;
  task automatic tw(input int v); acc = v + 1; endtask
  function automatic int fr(input int v); return v + 1; endfunction
  task automatic tw3(input int v, output int o); o = v + 3; endtask
  always_comb tw(src);
  always_comb acc2 = fr(src);
  always_comb tw3(src, acc3);
  initial begin #1 $display("A=%0d %0d %0d", acc, acc2, acc3); src = 9; #1 $display("B=%0d %0d %0d", acc, acc2, acc3); #1 $finish; end
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "A=8 8 10\nB=10 10 12");
    let (o, e, code) = run(r#"module t;
  int acc2 = 0, acc3 = 0, acc4 = 0; int src = 7; int arr[4] = '{1,2,3,4}; int i = 2;
  task automatic td(input int v, input int k = 5); acc2 = v + k; endtask
  task automatic ts(input int v); acc3 = v * 10; endtask
  task automatic te(input int v); acc4 = v; endtask
  always_comb td(.v(src));
  always_comb ts(arr[i]);
  always_comb te(src + 1);
  initial begin #1 $display("A=%0d %0d %0d", acc2, acc3, acc4); src = 9; i = 3; #1 $display("B=%0d %0d %0d", acc2, acc3, acc4); #1 $finish; end
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "A=12 30 8\nB=14 40 10");
}

/// ④ A frame FUNCTION's body reads are in the `always_comb` sensitivity — a
/// module net and a dynamic array (`w.size()`) read inside the callee — while
/// an `always @*` twin and a zero-actual TASK stay blind to them (iverilog + the
/// LRM; verilator re-runs the task cell, `104`). PRE printed `A=10 0 101` twice.
#[test]
fn a_functions_body_reads_are_in_the_always_comb_sensitivity() {
    let (o, e, code) = run(r#"module t; int src = 1; int n1, n2, n3, n4; int w[];
  function automatic int rds(); rds = src * 10; endfunction
  function automatic int rdw(); rdw = w.size(); endfunction
  task automatic tz(); n3 = src + 100; endtask
  always_comb n1 = rds();
  always @* n2 = rds();
  always_comb tz();
  always_comb n4 = rdw();
  initial begin #1 $display("A=%0d %0d %0d %0d", n1, n2, n3, n4); src = 4; w = new[3]; #1 $display("B=%0d %0d %0d %0d", n1, n2, n3, n4); $finish; end
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "A=10 0 101 0\nB=40 0 101 3");
}

/// ⑤ Two `always_comb` blocks each reaching `acc` through a task body — the
/// same task twice, a nested callee, and without any initializer — are E3001
/// (verilator MULTIDRIVEN ×3; PRE ran the initializer-free one, `ACC=22`, where
/// iverilog prints 8). A call under an `if` beside another writer is accepted
/// (verilator: no lint; both oracles `ACC=8`).
#[test]
fn two_body_writers_are_two_drivers_and_a_conditional_call_is_not() {
    for (decl, blocks) in [
        (
            "int acc = 0;",
            "always_comb tw(src);\n  always_comb tw(src);",
        ),
        (
            "int acc = 0; task automatic tn(input int v); tw(v); endtask",
            "always_comb tn(src);\n  always_comb tw(src);",
        ),
        (
            "int acc;",
            "always_comb tw(src);\n  always_comb tw(src + 1);",
        ),
    ] {
        let (_, e, code) = run(&format!(
            "module t;\n  {decl} int src = 7;\n  task automatic tw(input int v); acc = v + 1; endtask\n  {blocks}\n  initial begin #1 $finish; end\nendmodule\n"
        ));
        assert_ne!(code, Some(0), "{blocks}");
        assert!(
            e.contains("variable `acc` is written by `always_comb` AND by `always_comb`"),
            "{blocks}\n{e}"
        );
    }
    let (o, e, code) = run(r#"module t; int acc; int src = 7;
  task automatic tw(input int v); acc = v + 1; endtask
  always_comb tw(src);
  always_comb begin if (src > 3) tw(src); end
  initial begin #1 $display("ACC=%0d", acc); #1 $finish; end
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "ACC=8");
}
