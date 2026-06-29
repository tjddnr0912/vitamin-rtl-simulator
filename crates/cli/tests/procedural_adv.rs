//! P2-E procedural advanced: fork join_any/join_none (already live — pinned
//! here as iverilog differentials), `disable fork` (kills this process's
//! outstanding children), `do-while` (parser desugar), `unique`/`priority`
//! qualifiers (no-match = runtime warning, iverilog-pinned text class), and
//! `final` blocks ($finish-time one-shot). iverilog 13.0 live pins
//! (probes g1/g2/g6/g7, 2026-06-12); `priority if` is a hand-IEEE pin —
//! iverilog rejects the syntax outright.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_padv_{}_{n}", std::process::id()));
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
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

#[test]
fn join_any_and_join_none_differential_pin() {
    // iverilog-pinned (g1): join_any resumes at the FIRST child (t=1),
    // surplus children run on; join_none never blocks.
    let (out, err, code) = run("module top;\n\
         initial begin\n\
           fork\n\
             #3 $display(\"slow t=%0d\", $time);\n\
             #1 $display(\"fast t=%0d\", $time);\n\
           join_any\n\
           $display(\"after_any t=%0d\", $time);\n\
           fork\n\
             #2 $display(\"bg t=%0d\", $time);\n\
           join_none\n\
           $display(\"after_none t=%0d\", $time);\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    let want = "fast t=1\nafter_any t=1\nafter_none t=1\nslow t=3\nbg t=3";
    assert!(out.contains(want), "got:\n{out}");
}

#[test]
fn disable_fork_kills_outstanding_children() {
    // iverilog-pinned (g2): quick (t=1) prints, victim (t=5) dies at the
    // t=2 disable fork, end at t=12.
    let (out, err, code) = run("module top;\n\
         initial begin\n\
           fork\n\
             #5 $display(\"victim\");\n\
             #1 $display(\"quick\");\n\
           join_none\n\
           #2 disable fork;\n\
           #10 $display(\"end t=%0d\", $time);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("quick"), "got:\n{out}");
    assert!(!out.contains("victim"), "got:\n{out}");
    assert!(out.contains("end t=12"), "got:\n{out}");
}

#[test]
fn do_while_runs_body_first() {
    // iverilog-pinned (g6): two iterations, condition tested AFTER the body.
    let (out, err, code) = run("module top;\n\
         integer i;\n\
         initial begin\n\
           i = 0;\n\
           do begin\n\
             $display(\"dw i=%0d\", i);\n\
             i = i + 1;\n\
           end while (i < 2);\n\
           i = 9;\n\
           do $display(\"once i=%0d\", i); while (i < 5);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("dw i=0\ndw i=1"), "got:\n{out}");
    assert!(out.contains("once i=9"), "got:\n{out}");
}

#[test]
fn unique_priority_case_nomatch_warns() {
    // iverilog-pinned (g6/g7): a matching arm is silent; a NO-MATCH on a
    // unique/priority case is a RUNTIME warning and the run continues.
    let (out, err, code) = run("module top;\n\
         reg [1:0] s;\n\
         initial begin\n\
           s = 2;\n\
           unique case (s)\n\
             0: $display(\"zero\");\n\
             2: $display(\"two\");\n\
           endcase\n\
           s = 3;\n\
           unique case (s)\n\
             0: $display(\"u-zero\");\n\
             2: $display(\"u-two\");\n\
           endcase\n\
           priority case (s)\n\
             0: $display(\"p-zero\");\n\
           endcase\n\
           $display(\"alive\");\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("two"), "got:\n{out}");
    assert!(out.contains("alive"), "got:\n{out}");
    let warns = err.matches("unhandled").count();
    assert_eq!(warns, 2, "two no-match warnings, stderr:\n{err}");
}

#[test]
fn priority_if_nomatch_warns_hand_ieee() {
    // hand-IEEE §12.4.2 pin: `priority if` with no taken branch and no else
    // warns (iverilog REJECTS the syntax, so there is no oracle lane).
    let (out, err, code) = run("module top;\n\
         reg [1:0] s;\n\
         initial begin\n\
           s = 2;\n\
           priority if (s == 0) $display(\"pif\");\n\
           unique if (s == 2) $display(\"uif\");\n\
           $display(\"alive\");\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("uif"), "got:\n{out}");
    assert!(out.contains("alive"), "got:\n{out}");
    assert_eq!(
        err.matches("unhandled").count(),
        1,
        "one warning (the priority-if miss), stderr:\n{err}"
    );
}

#[test]
fn final_block_runs_at_finish() {
    // iverilog-pinned (g6): the final block runs ONCE after $finish, with
    // $time holding the finish time.
    let (out, err, code) = run("module top;\n\
         final $display(\"final ran at t=%0d\", $time);\n\
         initial begin\n\
           $display(\"body\");\n\
           #5 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "stderr:\n{err}");
    assert!(out.contains("body"), "got:\n{out}");
    assert!(out.contains("final ran at t=5"), "got:\n{out}");
}

#[test]
fn final_block_with_timing_is_loud() {
    // IEEE §9.2.3: a final block executes in zero time — timing controls
    // are illegal. Loud at elaborate.
    let (_o, err, code) = run("module top;\n\
         final begin\n\
           #1 $display(\"nope\");\n\
         end\n\
         initial $finish;\n\
         endmodule\n");
    assert_ne!(code, Some(0));
    assert!(err.contains("E3009"), "stderr:\n{err}");
}
