//! #10 (§3.10 runtime half) — RUNTIME severity diagnostics carry
//! `file:line:col`, the instance path, and the time.
//!
//! The engine runs on span-free IR, so it can never resolve a span itself.
//! Elaborate — which holds both the spans and the `SpanResolver` (§4.5.249;
//! staged since v28) — resolves each severity statement's span ONCE into the
//! `severity_locs` sidecar, and every runtime emitter (statement path, frame
//! path, deferred maturation) attaches that record. iverilog reports the same
//! facts for these designs (`ERROR: sev.sv:5: … Time: 5 Scope: top.u1`), which
//! is the content oracle; the exact rendering is vita's own and pinned
//! absolutely here.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

const TWO_INSTANCES: &str = "`timescale 1ns/1ns
module child;
  initial begin
    #5;
    $error(\"child says %0d\", 42);
  end
endmodule
module top;
  child u1();
  child u2();
  initial begin
    #10;
    $warning(\"top warns\");
    #1 $finish;
  end
endmodule
";

const FOUR_AXES: &str = "`timescale 1ns/1ns
module top;
  logic clk = 0;
  int q = 1;
  function automatic int fcheck(input int v);
    if (v > 3) $warning(\"frame-path warn v=%0d\", v);
    return v + 1;
  endfunction
  always #2 clk = ~clk;
  always @(posedge clk) begin
    assert #0 (q == 0) else $error(\"deferred q=%0d\", q);
  end
  initial begin
    int r;
    r = fcheck(7);
    unique case (q)
      2: ;
      3: ;
    endcase
    #3 $fatal(1, \"fatal here\");
  end
endmodule
";

fn fresh_dir() -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_runloc_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn run_in(d: &std::path::Path, args: &[&str]) -> (String, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .current_dir(d)
        .output()
        .expect("spawn vita");
    (
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        out.status.code().unwrap_or(-1),
    )
}

/// One-shot transcript of `src` (written as `d.sv` in a fresh dir).
fn one_shot(src: &str) -> (String, i32) {
    let d = fresh_dir();
    std::fs::write(d.join("d.sv"), src).unwrap();
    let r = run_in(&d, &["d.sv"]);
    let _ = std::fs::remove_dir_all(&d);
    r
}

/// The severity DIAGNOSTIC lines (located or not) of a transcript, in order.
fn diag_lines(t: &str) -> Vec<String> {
    t.lines()
        .filter(|l| l.contains("[VITA-E4") || l.contains("[VITA-W4") || l.contains("[VITA-F4"))
        .map(str::to_string)
        .collect()
}

// The full record on one line: file:line:col + code + message + instance + time.
// Absolute pin — parity alone would pass two location-less paths.
#[test]
fn a_runtime_severity_reports_file_line_col_instance_and_time() {
    let (out, _) = one_shot(TWO_INSTANCES);
    assert!(
        out.contains(
            "d.sv:5:5: error[VITA-E4003] E-RUN-USER-ERROR: child says 42 [in top.u1] [at time 5]"
        ),
        "full located severity line missing:\n{out}"
    );
    assert!(
        out.contains(
            "d.sv:13:5: warning[VITA-W4007] W-RUN-USER-WARNING: top warns [in top] [at time 10]"
        ),
        "warning line missing:\n{out}"
    );
}

// A module instantiated twice fires the SAME source line twice — only the
// instance path tells the reports apart (the §4.5.249 argument, at runtime).
#[test]
fn two_instances_of_one_line_are_distinguished_by_instance_path() {
    let (out, _) = one_shot(TWO_INSTANCES);
    assert!(
        out.contains("d.sv:5:5:") && out.contains("[in top.u1]") && out.contains("[in top.u2]")
    );
    assert_eq!(
        out.matches("d.sv:5:5:").count(),
        2,
        "both instances report the one source line:\n{out}"
    );
}

// vcmp → velab → vrun reproduces the one-shot diagnostic lines BYTE-FOR-BYTE
// (the staged-drop hazard: the sidecar must survive the `.velab` trailer).
#[test]
fn staged_vrun_matches_one_shot_byte_for_byte() {
    for src in [TWO_INSTANCES, FOUR_AXES] {
        let d = fresh_dir();
        std::fs::write(d.join("d.sv"), src).unwrap();
        let (one, _) = run_in(&d, &["d.sv"]);
        let (c, code) = run_in(&d, &["vcmp", "d.sv", "-o", "d.vu"]);
        assert_eq!(code, 0, "vcmp:\n{c}");
        let (e, code) = run_in(&d, &["velab", "d.vu", "-o", "d.velab"]);
        assert_eq!(code, 0, "velab:\n{e}");
        let (staged, _) = run_in(&d, &["vrun", "d.velab"]);
        let a = diag_lines(&one);
        assert!(
            !a.is_empty(),
            "probe design must produce diagnostics:\n{one}"
        );
        assert!(
            a.iter().all(|l| l.contains(".sv:")),
            "every one-shot severity line must be located:\n{one}"
        );
        assert_eq!(a, diag_lines(&staged), "one-shot vs staged diverged");
        let _ = std::fs::remove_dir_all(&d);
    }
}

// A severity inside a subset FUNCTION body flows through the frame-path twin
// (`frame_emit_severity`), which must report the same kind of record.
#[test]
fn a_frame_path_severity_locates_like_the_statement_path() {
    let (out, _) = one_shot(FOUR_AXES);
    assert!(
        out.contains("d.sv:6:16: warning[VITA-W4007]")
            && out.contains("frame-path warn v=7 [in top.$func$fcheck] [at time 0]"),
        "frame-path severity must locate:\n{out}"
    );
}

// A deferred assert (§16.4) renders at REACH and emits at MATURATION — the
// location is the ACTION's (`$error` at 11:29), not the maturation site's, and
// the value is the reach-time one.
#[test]
fn a_deferred_assert_matures_with_the_action_sites_location() {
    let (out, _) = one_shot(FOUR_AXES);
    assert!(
        out.contains("d.sv:11:29: error[VITA-E4003] E-RUN-USER-ERROR: deferred q=1")
            && out.contains("deferred q=1 [in top] [at time 2]"),
        "deferred action location missing:\n{out}"
    );
}

// A `unique case` violation is a desugared severity statement — its location
// is the CASE statement the user wrote.
#[test]
fn a_unique_violation_points_at_its_case_statement() {
    let (out, _) = one_shot(FOUR_AXES);
    assert!(
        out.contains("d.sv:16:12: warning[VITA-W4031] W-RUN-UNIQUE-VIOLATION:"),
        "unique violation must point at the case:\n{out}"
    );
}

// `$fatal` still aborts (exit != 0) AND reports where from.
#[test]
fn fatal_reports_location_and_still_aborts() {
    let (out, code) = one_shot(FOUR_AXES);
    assert!(
        out.contains("d.sv:20:8: fatal[VITA-F4004] F-RUN-FATAL: fatal here [in top] [at time 3]"),
        "fatal location missing:\n{out}"
    );
    assert_ne!(code, 0, "fatal must not exit 0");
}
