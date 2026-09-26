//! `$monitor` before `$strobe` in one time step (ROADMAP §2 "Delays / events", §4.5.541 cell
//! d09): the Postponed region prints the monitor line of a time step before every `$strobe`
//! registered in that step, whichever statement ran first, and the same on the `$fmonitor` /
//! `$fstrobe` file lane. Both oracles (iverilog 13, verilator 5.052) print every line pinned
//! here; the engine used to drain the strobe FIFO first (`10 S v=3 | 10 M v=3`). Recorded, not
//! changed: a strobe registered BEFORE the step's first monitored change (a `#3` resume that
//! strobes, then the posedge block that changes the value) is printed first by iverilog and
//! after the monitor line by verilator — an oracle split; vita prints the monitor line first.
//! Every cell runs on all three backends.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita_on(src: &str, backend: Option<&str>) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_monstr_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vita"));
    cmd.arg(&path);
    if let Some(b) = backend {
        cmd.arg("--backend").arg(b);
    }
    let out = cmd.output().expect("run vita");
    let _ = std::fs::remove_file(&path);
    let mut all = String::from_utf8_lossy(&out.stdout).into_owned();
    all.push_str(&String::from_utf8_lossy(&out.stderr));
    let mut s = String::new();
    for l in all.lines().filter(|l| {
        !l.starts_with("simulation ended")
            && !l.starts_with("errors=")
            && !l.contains("W-PP-TIMESCALE-DEFAULT")
            && !l.contains("W-RUN-BACKEND-FALLBACK")
    }) {
        s.push_str(l);
        s.push('\n');
    }
    (s, out.status.success())
}

/// Every backend must print the same thing; the default's text is returned.
fn run(src: &str) -> String {
    let (s, ok) = vita_on(src, None);
    assert!(ok, "expected exit 0, got:\n{s}");
    for b in ["interp", "vm", "native"] {
        let (t, ok) = vita_on(src, Some(b));
        assert!(ok, "backend {b}: expected exit 0, got:\n{t}");
        assert_eq!(t, s, "backend {b} diverges from the default on:\n{src}");
    }
    s
}

#[test]
fn the_monitor_line_precedes_every_strobe_of_the_step() {
    // The monitor declared first.
    let s = run(r#"module t; reg [3:0] v=0;
initial $monitor("%0t M v=%0d", $time, v);
initial begin #10 v = 3; $strobe("%0t S v=%0d", $time, v); end
initial #20 $finish; endmodule
"#);
    assert_eq!(s, "0 M v=0\n10 M v=3\n10 S v=3\n");
    // The strobe's process declared first.
    let s = run(r#"module t; reg [3:0] v=0;
initial begin #10 v = 3; $strobe("%0t S v=%0d", $time, v); end
initial $monitor("%0t M v=%0d", $time, v);
initial #20 $finish; endmodule
"#);
    assert_eq!(s, "0 M v=0\n10 M v=3\n10 S v=3\n");
    // Three strobes from two processes keep their call order behind the monitor.
    let s = run(r#"module t; reg [3:0] v=0;
initial $monitor("%0t M v=%0d", $time, v);
initial begin #10 v = 3; $strobe("%0t S1 v=%0d", $time, v); v = 4; $strobe("%0t S2 v=%0d", $time, v); end
initial begin #10 $strobe("%0t S0 v=%0d", $time, v); end
initial #20 $finish; endmodule
"#);
    assert_eq!(s, "0 M v=0\n10 M v=4\n10 S1 v=4\n10 S2 v=4\n10 S0 v=4\n");
}

#[test]
fn the_file_monitor_precedes_the_file_strobe() {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let out = std::env::temp_dir().join(format!("vita_monstr_out_{}_{n}.txt", std::process::id()));
    let src = format!(
        r#"module t; reg [3:0] v=0; integer fd;
initial begin fd = $fopen("{}", "w"); $fmonitor(fd, "%0t FM v=%0d", $time, v); end
initial begin #10 v = 3; $fstrobe(fd, "%0t FS v=%0d", $time, v); #1 $fclose(fd); end
initial #20 $finish; endmodule
"#,
        out.display()
    );
    for b in [None, Some("interp"), Some("vm"), Some("native")] {
        let (s, ok) = vita_on(&src, b);
        assert!(ok, "backend {b:?}: expected exit 0, got:\n{s}");
        let got = std::fs::read_to_string(&out).unwrap();
        let _ = std::fs::remove_file(&out);
        assert_eq!(got, "0 FM v=0\n10 FM v=3\n10 FS v=3\n", "backend {b:?}");
    }
}
