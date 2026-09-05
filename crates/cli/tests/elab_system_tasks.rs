//! §4.5.428: elaboration system tasks (IEEE 1800-2017 §20.11) — `$info` / `$warning` /
//! `$error` / `$fatal` as a MODULE ITEM (also inside a generate branch), run at
//! elaboration with constant arguments: `$info`/`$warning` report and continue,
//! `$error`/`$fatal` fail elaboration before any simulation. Both oracles evaluate
//! these at elaboration (verilator formats the arguments; iverilog accepts one
//! string); the message TEXT differs per tool, so these pin vita's exit code, the
//! diagnostic code and whether `D=run` (a t0 `$display`) is ever reached.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_elabtask_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

fn lines(src: &str, prefix: &str) -> Vec<String> {
    let (out, rc) = run(src);
    assert_eq!(
        rc,
        Some(0),
        "expected exit 0, got {rc:?}:
{out}"
    );
    out.lines()
        .filter(|l| l.starts_with(prefix))
        .map(|l| l.to_string())
        .collect()
}

#[test]
fn info_and_warning_report_and_the_simulation_runs() {
    let src = "module top;\n  parameter W = 4;\n  $info(\"info at elab W=%0d\", W);\n  $warning(\"warn at elab\");\n  if (W > 8) $fatal(1, \"W too big\");\n  generate if (W == 4) begin : g $info(\"in generate %m\"); end endgenerate\n  initial begin $display(\"D=run\"); #1 $finish; end\nendmodule\n";
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{out}");
    assert!(
        out.contains("I-ELAB-USER-INFO") && out.contains("info at elab W=4"),
        "{out}"
    );
    assert!(
        out.contains("W-ELAB-USER-WARNING") && out.contains("warn at elab"),
        "{out}"
    );
    assert!(out.contains("in generate top.g"), "{out}");
    assert!(out.contains("D=run"), "{out}");
}

#[test]
fn fatal_and_error_fail_elaboration_before_simulation() {
    // The ibex_top_tracing shape: a `$fatal` under an `ifndef` guard.
    let src = "module top;\n`ifndef RVFI\n  $fatal(\"Fatal error: RVFI needs to be defined globally.\");\n`endif\n  initial begin $display(\"D=run\"); #1 $finish; end\nendmodule\n";
    let (out, rc) = run(src);
    assert_ne!(rc, Some(0), "{out}");
    assert!(
        out.contains("F-ELAB-USER-FATAL") && out.contains("RVFI needs to be defined"),
        "{out}"
    );
    assert!(
        !out.contains("D=run"),
        "no simulation after an elaboration $fatal:\n{out}"
    );
    let src = "module top;\n  parameter W = 4;\n  if (W > 2) $error(\"W=%0d too big\", W);\n  initial begin $display(\"D=run\"); #1 $finish; end\nendmodule\n";
    let (out, rc) = run(src);
    assert_ne!(rc, Some(0), "{out}");
    assert!(
        out.contains("E-ELAB-USER-ERROR") && out.contains("W=4 too big"),
        "{out}"
    );
    assert!(!out.contains("D=run"), "{out}");
}

#[test]
fn a_false_generate_branch_does_not_fire_and_a_non_constant_argument_is_loud() {
    let src = "module top;\n  parameter W = 4;\n  if (W > 8) $fatal(1, \"never\");\n  initial begin $display(\"D=run\"); #1 $finish; end\nendmodule\n";
    assert_eq!(lines(src, "D="), vec!["D=run"]);
    let (out, rc) = run("module top;\n  logic v = 1;\n  $info(\"v=%0d\", v);\n  initial begin $display(\"D=run\"); #1 $finish; end\nendmodule\n");
    assert_ne!(rc, Some(0), "{out}");
    assert!(
        out.contains("E3009") && out.contains("not a constant"),
        "{out}"
    );
    let (out, rc) = run("module top;\n  $display(\"not an elaboration task\");\n  initial begin $display(\"D=run\"); #1 $finish; end\nendmodule\n");
    assert_ne!(rc, Some(0), "{out}");
    assert!(out.contains("E2002"), "{out}");
}

/// Review A BLOCKING (§4.5.428): the elaboration renderer must produce the SAME text as
/// the runtime renderer for the same format and arguments — the field rules live once,
/// in `diag::fmt`. The `P` line is verilator's own (`[   65] [65   ] [0041] [      ab]
/// [ab      ] [AB]`); the others pin elaboration == runtime (`$display` at t0).
#[test]
fn elaboration_field_widths_match_the_runtime_renderer() {
    let src = r#"
module top;
  parameter int N = -3;
  parameter int P = 65;
  parameter logic [15:0] Q = 16'h4142;
  parameter logic [7:0] R = 8'h41;
  parameter logic [7:0] C = 8'd4;
  $info("N d=%d h=%h b=%0b 0d=%0d", N, N, N, N);
  $info("P [%5d] [%-5d] [%04h] [%8s] [%-8s] [%s]", P, P, R, "ab", "ab", Q);
  $info("Q [%s] [%0s] [%c] [%3d]", Q, Q, R, P);
  $info("C d=%d h=%h b=%b", C, C, C);
  initial begin
    $display("N d=%d h=%h b=%0b 0d=%0d", N, N, N, N);
    $display("P [%5d] [%-5d] [%04h] [%8s] [%-8s] [%s]", P, P, R, "ab", "ab", Q);
    $display("Q [%s] [%0s] [%c] [%3d]", Q, Q, R, P);
    $display("C d=%d h=%h b=%b", C, C, C);
    #1 $finish;
  end
endmodule
"#;
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{out}");
    let elab: Vec<&str> = out
        .lines()
        .filter_map(|l| l.split("I-ELAB-USER-INFO: ").nth(1))
        .map(|l| l.trim_end_matches(" [in top]"))
        .collect();
    let runtime: Vec<&str> = out
        .lines()
        .filter(|l| {
            !l.contains("VITA-")
                && (l.starts_with("N ")
                    || l.starts_with("P ")
                    || l.starts_with("Q ")
                    || l.starts_with("C "))
        })
        .collect();
    assert_eq!(elab.len(), 4, "{out}");
    assert_eq!(elab, runtime, "{out}");
    assert_eq!(
        elab[0],
        "N d=         -3 h=fffffffd b=11111111111111111111111111111101 0d=-3"
    );
    assert_eq!(
        elab[1],
        "P [   65] [65   ] [0041] [      ab] [ab      ] [AB]"
    );
    assert_eq!(elab[2], "Q [AB] [AB] [A] [ 65]");
    assert_eq!(elab[3], "C d=  4 h=04 b=00000100");
}
