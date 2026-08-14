//! `$fmonitor` / `$fstrobe` — the FILE-directed twins of `$monitor` / `$strobe`.
//!
//! Both were a W3056 "unsupported system task skipped": the file output was silently
//! never produced. They reuse the FROZEN `SysTaskId::Monitor` / `Strobe` ids rather than
//! adding variants (which would flip the SimIr schema hash and re-pin every golden); what
//! makes `args[0]` a descriptor is the `file_directed_stmts` sidecar, written by elaborate
//! from the dollar-name and read by the engine at registration.
//!
//! IEEE §21.3.4 makes `$fmonitor` the file analogue of `$monitor`, so monitors are kept
//! PER DESTINATION — a `$fmonitor` must not displace a standing `$monitor`.
//!
//! ORACLE: iverilog 13.0, with one pinned divergence (see the last test).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Runs `src` in a fresh dir; returns `(stdout+stderr, dir)` so the caller can read the
/// files the design wrote.
fn run_in_dir(src: &str) -> (String, std::path::PathBuf) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fms_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let txt = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !txt.contains("unsupported system task"),
        "still skipped:\n{txt}"
    );
    (txt, d)
}

fn file_of(d: &std::path::Path, name: &str) -> String {
    std::fs::read_to_string(d.join(name)).unwrap_or_default()
}

#[test]
fn fmonitor_writes_every_change_to_its_file() {
    let (_o, d) = run_in_dir(
        "module t; integer fd; reg [7:0] c = 0;\n\
         initial begin fd = $fopen(\"o.txt\", \"w\");\n\
           $fmonitor(fd, \"M c=%0d\", c);\n\
           #1 c = 1; #1 c = 2; #1 $fclose(fd); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(file_of(&d, "o.txt"), "M c=0\nM c=1\nM c=2\n"); // iverilog
}

#[test]
fn fstrobe_writes_the_settled_end_of_step_value() {
    let (_o, d) = run_in_dir(
        "module t; integer fd; reg [7:0] c = 0;\n\
         initial begin fd = $fopen(\"o.txt\", \"w\");\n\
           c = 5; $fstrobe(fd, \"S c=%0d\", c); c = 6;\n\
           #1 $fclose(fd); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(file_of(&d, "o.txt"), "S c=6\n"); // iverilog: the SETTLED value
}

// ── several `$fstrobe` in one step keep call order, and a plain `$strobe` interleaved
// still goes to stdout ──
#[test]
fn multiple_fstrobes_and_an_interleaved_strobe() {
    let (o, d) = run_in_dir(
        "module t; integer fd; reg [7:0] c = 0;\n\
         initial begin fd = $fopen(\"o.txt\", \"w\");\n\
           c = 1; $fstrobe(fd, \"A=%0d\", c); $strobe(\"SO=%0d\", c);\n\
           $fstrobe(fd, \"B=%0d\", c); c = 9;\n\
           #1 $fclose(fd); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(file_of(&d, "o.txt"), "A=9\nB=9\n");
    assert!(
        o.contains("SO=9"),
        "plain $strobe must stay on stdout:\n{o}"
    );
}

// ── the b/o/h variants set the default radix, and a pre-opened STDOUT descriptor works ──
#[test]
fn fmonitorh_to_the_stdout_descriptor() {
    let (o, _d) = run_in_dir(
        "module t; reg [7:0] c = 0;\n\
         initial begin $fmonitorh(32'h8000_0001, \"H c=%h\", c);\n\
           #1 c = 10; #1 c = 255; #1 $finish; end\n\
         endmodule\n",
    );
    for want in ["H c=00", "H c=0a", "H c=ff"] {
        assert!(o.contains(want), "missing `{want}`:\n{o}");
    }
}

// ── PER-DESTINATION: a `$fmonitor` must not displace a standing `$monitor`. A single
// shared slot did, and the stdout monitor silently stopped reporting. ──
#[test]
fn an_fmonitor_does_not_displace_a_standing_monitor() {
    let (o, d) = run_in_dir(
        "module t; integer fd; reg [7:0] c = 0;\n\
         initial begin fd = $fopen(\"o.txt\", \"w\");\n\
           $monitor(\"STDOUT c=%0d\", c);\n\
           #1 c = 1;\n\
           #1 $fmonitor(fd, \"FILE c=%0d\", c);\n\
           #1 c = 2;\n\
           #1 $fclose(fd); $finish; end\n\
         endmodule\n",
    );
    // iverilog: stdout 0,1,2 — the monitor keeps firing after the $fmonitor.
    for want in ["STDOUT c=0", "STDOUT c=1", "STDOUT c=2"] {
        assert!(o.contains(want), "missing `{want}`:\n{o}");
    }
    assert_eq!(file_of(&d, "o.txt"), "FILE c=1\nFILE c=2\n");
}

#[test]
fn two_fmonitors_on_different_descriptors_are_independent() {
    let (_o, d) = run_in_dir(
        "module t; integer f1, f2; reg [7:0] c = 0;\n\
         initial begin f1 = $fopen(\"a.txt\", \"w\"); f2 = $fopen(\"b.txt\", \"w\");\n\
           $fmonitor(f1, \"A c=%0d\", c);\n\
           $fmonitor(f2, \"B c=%0d\", c);\n\
           #1 c = 1; #1 $fclose(f1); $fclose(f2); $finish; end\n\
         endmodule\n",
    );
    assert_eq!(file_of(&d, "a.txt"), "A c=0\nA c=1\n"); // iverilog
    assert_eq!(file_of(&d, "b.txt"), "B c=0\nB c=1\n"); // iverilog
}

#[test]
fn an_invalid_descriptor_is_loud_and_writes_nothing() {
    let (o, _d) = run_in_dir(
        "module t; reg [7:0] c = 0;\n\
         initial begin $fmonitor(32'hdead_beef, \"X c=%0d\", c);\n\
           #1 c = 1; #1 $finish; end\n\
         endmodule\n",
    );
    assert!(o.contains("W4022"), "bad fd must warn:\n{o}");
    assert!(!o.contains("X c="), "must not fall back to stdout:\n{o}");

    // …and the X/Z descriptor, which is a DIFFERENT arm: `split_file_directed`
    // cannot turn it into a number at all, so what stops it falling back to
    // stdout is the `unwrap_or(u32::MAX)` — "an unusable descriptor still
    // consumes `args[0]`". ⚠️ Without this half the mutation that returns a
    // bare `None` there SURVIVES: the value above is a perfectly readable
    // 32'hdeadbeef and never reaches the `unwrap_or`.
    let (o2, _d2) = run_in_dir(
        "module t; reg [7:0] c = 0; reg [31:0] bad;\n\
         initial begin $fmonitor(bad, \"Z c=%0d\", c);\n\
           #1 c = 1; #1 $finish; end\n\
         endmodule\n",
    );
    assert!(
        !o2.contains("Z c="),
        "an X/Z descriptor must not fall back to stdout:\n{o2}"
    );
}

// ── PINNED DIVERGENCE. iverilog ACCUMULATES `$fmonitor`s on ONE descriptor: two calls
// with the same fd print two lines per change (`A c=0 / B c=0 / B c=1 / A c=1`), which
// contradicts its own singleton `$monitor` — a second `$monitor` replaces the first.
// vita keeps ONE monitor per destination, so the second call replaces the first, exactly
// as `$monitor` does. Hand-IEEE (§21.3.4 defines `$fmonitor` as the file `$monitor`).
#[test]
fn a_second_fmonitor_on_the_same_descriptor_replaces_the_first() {
    let (_o, d) = run_in_dir(
        "module t; integer fd; reg [7:0] c = 0;\n\
         initial begin fd = $fopen(\"o.txt\", \"w\");\n\
           $fmonitor(fd, \"A c=%0d\", c);\n\
           #1 $fmonitor(fd, \"B c=%0d\", c);\n\
           #1 c = 1; #1 $fclose(fd); $finish; end\n\
         endmodule\n",
    );
    // A's establishment line, then B replaces it: no second `A` line.
    assert_eq!(file_of(&d, "o.txt"), "A c=0\nB c=0\nB c=1\n");
}

/// Slice #4 ABSOLUTE ANCHOR — the same file content on the TIER-3 backend,
/// iverilog-pinned.
///
/// `$fmonitor`/`$fstrobe` reuse the frozen `Monitor`/`Strobe` ids, so the only
/// thing that makes `args[0]` a descriptor is `file_directed_stmts` — and
/// `split_file_directed` evaluated that descriptor with a bare `sched.eval`,
/// the ENGINE's nets. On a native run that store is never written, so `fd`
/// read as whatever the engine's slot held instead of the value `$fopen`
/// returned: the monitor would address a descriptor the design never opened.
///
/// The `fd` here is a MODULE NET rather than a literal, which is the whole
/// discriminator — a literal descriptor is a constant and would agree either
/// way. Values are iverilog's.
#[test]
fn fmonitor_and_fstrobe_on_tier_3_write_the_same_file() {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fms_nat_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(
        &f,
        "module top;\n\
           integer fd; reg [7:0] a = 8'd0; reg clk = 1'b0;\n\
           always #1 clk = ~clk;\n\
           always @(posedge clk) a = a + 8'd1;\n\
           initial begin\n\
             fd = $fopen(\"fm.txt\", \"w\");\n\
             $fmonitor(fd, \"M t=%0t a=%0d\", $time, a);\n\
             #3 $fstrobe(fd, \"S t=%0t a=%0d\", $time, a);\n\
             #4 $fclose(fd);\n\
             $display(\"done\");\n\
             $finish;\n\
           end\n\
         endmodule\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--backend")
        .arg("native")
        .arg("--obs-dir")
        .arg("obs")
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let txt = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // ANTI-VACUITY: a refused design falls back to the VM and this test would
    // then compare the VM against iverilog, which the tests above already do.
    let run = std::fs::read_to_string(d.join("obs").join("run.json")).unwrap_or_default();
    assert!(
        run.contains("\"backend\": \"native\""),
        "the design did not run natively:\n{run}\n{txt}"
    );
    assert_eq!(
        file_of(&d, "fm.txt"),
        "M t=0 a=0\nM t=1 a=1\nS t=3 a=2\nM t=3 a=2\nM t=5 a=3\n",
        "tier-3 $fmonitor/$fstrobe (iverilog-pinned); stdout was:\n{txt}"
    );
}
