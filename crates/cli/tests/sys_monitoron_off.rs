//! v9 Medium-bundle rank 6: `$monitoroff` / `$monitoron` (IEEE 1364-2005 §17.1).
//! `$monitoroff` suppresses the continuous monitor; `$monitoron` re-enables it
//! AND prints the current values once immediately (it clears the change-tracking
//! baseline). Pinned to LIVE iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_mon_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

#[test]
fn off_suppresses_then_on_reprints() {
    // x changes every tick. Monitor off at t=2 (so the t=3 change is NOT printed),
    // on at t=4 (which reprints the current value x=2 immediately, even though x
    // did not change AT t=4). iverilog live: t=0 x=0, t=1 x=1, t=4 x=2, t=5 x=3.
    let (out, _c) = run("module t;\n\
         reg [7:0] x;\n\
         initial begin\n\
           x=0; #1 x=1; #1 $monitoroff; #1 x=2; #1 $monitoron; #1 x=3; #1 $finish;\n\
         end\n\
         initial $monitor(\"t=%0t x=%0d\", $time, x);\n\
         endmodule\n");
    let lines: Vec<&str> = out.lines().filter(|l| l.starts_with("t=")).collect();
    assert_eq!(
        lines,
        vec!["t=0 x=0", "t=1 x=1", "t=4 x=2", "t=5 x=3"],
        "monitor on/off sequence:\n{out}"
    );
    // the t=3 transition (x 1→2) while OFF must NOT appear as its own line.
    assert!(
        !out.contains("t=3 x=2"),
        "change while monitor off must be suppressed:\n{out}"
    );
}

#[test]
fn off_persists_across_reestablishment() {
    // review H2: the monitor-enable is GLOBAL — a $monitoroff issued BEFORE
    // $monitor keeps suppressing change-reprints (and survives a re-$monitor),
    // while the establishment line still prints once. iverilog live: only `A=1`.
    let (out, _c) = run("module t;\n\
         reg [7:0] a;\n\
         initial begin\n\
           a = 1; $monitoroff; $monitor(\"A=%0d\", a); #10 a = 2; #10 $finish;\n\
         end\n\
         endmodule\n");
    let lines: Vec<&str> = out.lines().filter(|l| l.starts_with("A=")).collect();
    assert_eq!(
        lines,
        vec!["A=1"],
        "establishment prints once, change suppressed:\n{out}"
    );
}

#[test]
fn same_tick_off_keeps_establishment() {
    // review H3: $monitor immediately followed (SAME time-step) by $monitoroff
    // still prints the establishment line — it is bound to the $monitor
    // time-step, not re-gated at flush. iverilog live: `M=5`.
    let (out, _c) = run("module t;\n\
         reg [7:0] a;\n\
         initial begin a = 5; $monitor(\"M=%0d\", a); $monitoroff; #10 $finish; end\n\
         endmodule\n");
    assert!(
        out.contains("M=5"),
        "same-tick $monitoroff keeps the establishment print:\n{out}"
    );
}

#[test]
fn off_with_no_monitor_is_harmless() {
    // $monitoroff / $monitoron with no established monitor must not crash.
    let (out, code) = run("module t;\n\
         initial begin $monitoroff; $monitoron; $display(\"OK\"); end\n\
         endmodule\n");
    assert!(out.contains("OK"), "no-monitor on/off harmless:\n{out}");
    assert_ne!(code, Some(101), "must not panic");
}

#[test]
fn stime_arg_does_not_retrigger_monitor() {
    // IEEE §21.2.3: a DIRECT `$time`/`$stime`/`$realtime` arg is RENDERED but does
    // NOT participate in $monitor change detection — time advancing alone must not
    // reprint. `$stime` (the low 32 bits of `$time`) was omitted from the
    // exclusion, so it re-fired on EVERY 1ns step (16 spurious lines, silent-wrong).
    // iverilog-13.0 live: only the establishment (s=0 x=0) and the real x change
    // (s=5 x=1) — exactly 2 lines. (iverilog rejects a non-time expression like
    // $random in $monitor; the three time functions are its only allowed non-simple
    // args — so this is the exhaustive oracle-backed exclusion set.)
    let (out, _c) = run("`timescale 1ns/1ps\n\
         module t;\n\
           reg clk=0, x=0;\n\
           initial forever #1 clk=~clk;\n\
           initial $monitor(\"mon s=%0d x=%b\", $stime, x);\n\
           initial begin #5 x=1; #10 $finish; end\n\
         endmodule\n");
    let lines: Vec<&str> = out.lines().filter(|l| l.starts_with("mon ")).collect();
    assert_eq!(
        lines,
        vec!["mon s=0 x=0", "mon s=5 x=1"],
        "$stime must not retrigger the monitor each tick:\n{out}"
    );
}
