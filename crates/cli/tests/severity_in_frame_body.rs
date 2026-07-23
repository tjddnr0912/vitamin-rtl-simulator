//! r18 (F2): a SEVERITY task (`$info`/`$warning`/`$error`/`$fatal`) inside a synchronous
//! frame FUNCTION / subset-TASK body is now correct-support (was a false-positive E3009
//! "…uses a $systask…"). Severity tasks lower as a `Display` + a `severities` sidecar; the
//! misclassification was that `classify_frame_body` only admitted `Display`s in
//! `frame_print_stmts`, so the `$warning` a `unique`/`priority case` synthesizes for its
//! no-match arm (and any explicit severity call) rejected the whole subroutine — even when
//! that arm never executes.
//!
//! The fix admits severity `Display`s in `classify_frame_body` and renders them in the
//! `&self` frame executors (`frame_emit_severity`): the message goes to the diag stream,
//! `$error` latches `had_error` (now a `Cell`), `$fatal` latches `call_fatal` (the
//! scheduler converts it to an error finish — the channel `fatal_frame_heap_write` uses).
//!
//! ORACLE: iverilog 13.0 runs severity in functions/tasks, so the firing cases are
//! iverilog-verified (message text class + continue/abort + exit).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns (combined stdout+stderr, exit-code).
fn run(src: &str) -> (String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sevfb_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (combined, out.status.code().unwrap_or(-1))
}

fn is_loud(o: &str) -> bool {
    o.contains("E3009")
}

// ── the report's F2: a pure `unique case` function called from a suspending task ──
#[test]
fn unique_case_fn_in_suspending_task() {
    let (o, _) = run("module t;\n\
        logic clk = 0; always #5 clk = ~clk;\n\
        typedef enum logic [1:0] { M0=0, M1=1 } m_t;\n\
        function automatic int dbytes (input m_t m);\n\
          unique case (m) M0: dbytes = 28; M1: dbytes = 32; endcase\n\
        endfunction\n\
        task automatic run (input m_t m);\n\
          int r; @(posedge clk); r = dbytes(m); if (r == 32) $display(\"PASS\");\n\
        endtask\n\
        initial begin run(M1); $finish; end\n\
        endmodule\n");
    assert!(!is_loud(&o) && o.contains("PASS"), "F2 repro:\n{o}");
}

// ── a `unique case` function called from a plain (non-suspending) context ──
#[test]
fn unique_case_fn_nonsuspending() {
    let (o, _) = run("module t;\n\
        typedef enum logic [1:0] { M0=0, M1=1 } m_t;\n\
        function automatic int dbytes (input m_t m);\n\
          unique case (m) M0: dbytes = 28; M1: dbytes = 32; endcase\n\
        endfunction\n\
        initial begin int r; r = dbytes(M1); if (r==32) $display(\"PASS\"); $finish; end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("PASS"),
        "unique-case fn (nonsusp):\n{o}"
    );
}

// ── the synthesized `$warning` FIRES when the value is unhandled (iverilog: W + r=0) ──
#[test]
fn unique_case_violation_warning_fires() {
    let (o, code) = run("module t;\n\
        typedef enum logic [1:0] { M0=0, M1=1, M2=2 } m_t;\n\
        function automatic int dbytes (input m_t m);\n\
          unique case (m) M0: dbytes = 28; M1: dbytes = 32; endcase\n\
        endfunction\n\
        initial begin int r; r = dbytes(M2); $display(\"r=%0d\", r); $finish; end\n\
        endmodule\n");
    assert!(
        !is_loud(&o)
            && o.contains("value is unhandled for priority or unique case")
            && o.contains("r=0")
            && code == 0,
        "unique-case violation warning fires (a $warning, not an error):\n{o} (code={code})"
    );
}

// ── explicit `$error` in a FUNCTION body → message + nonzero exit (had_error) ──
#[test]
fn error_in_function_sets_exit() {
    let (o, code) = run("module t;\n\
        function automatic int chk (input int x);\n\
          if (x > 10) $error(\"too big: %0d\", x);\n\
          chk = x;\n\
        endfunction\n\
        initial begin int r; r = chk(20); $display(\"r=%0d\", r); $finish; end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("too big: 20") && o.contains("r=20") && code == 1,
        "$error in fn: message + r=20 + exit 1:\n{o} (code={code})"
    );
}

// ── explicit `$fatal` in a subset TASK body → aborts (later stmts do NOT run) ──
#[test]
fn fatal_in_task_aborts() {
    let (o, code) = run("module t;\n\
        task automatic tk (input int x);\n\
          if (x > 10) $fatal(1, \"fatal x=%0d\", x);\n\
          $display(\"after x=%0d\", x);\n\
        endtask\n\
        initial begin tk(20); $display(\"SHOULD-NOT-PRINT\"); $finish; end\n\
        endmodule\n");
    assert!(
        !is_loud(&o)
            && o.contains("fatal x=20")
            && !o.contains("after x=20")
            && !o.contains("SHOULD-NOT-PRINT")
            && code == 1,
        "$fatal in task aborts:\n{o} (code={code})"
    );
}

// ── `$warning` in a subset TASK body → message + continues ──
#[test]
fn warning_in_task_continues() {
    let (o, code) = run("module t;\n\
        task automatic tk (input int x);\n\
          if (x > 10) $warning(\"warn x=%0d\", x);\n\
          $display(\"done x=%0d\", x);\n\
        endtask\n\
        initial begin tk(20); $finish; end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("warn x=20") && o.contains("done x=20") && code == 0,
        "$warning in task: message + continues:\n{o} (code={code})"
    );
}

// ── `$info` in a function body → message, no exit change ──
#[test]
fn info_in_function() {
    let (o, code) = run("module t;\n\
        function automatic int f (input int x);\n\
          $info(\"got %0d\", x); f = x + 1;\n\
        endfunction\n\
        initial begin int r; r = f(7); $display(\"r=%0d\", r); $finish; end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("got 7") && o.contains("r=8") && code == 0,
        "$info in fn:\n{o} (code={code})"
    );
}

// ── regression: a plain `case` (no unique/priority) in a frame fn still works ──
#[test]
fn plain_case_fn_unchanged() {
    let (o, _) = run("module t;\n\
        typedef enum logic [1:0] { M0=0, M1=1 } m_t;\n\
        function automatic int dbytes (input m_t m);\n\
          case (m) M0: dbytes = 28; M1: dbytes = 32; endcase\n\
        endfunction\n\
        initial begin int r; r = dbytes(M1); if (r==32) $display(\"PASS\"); $finish; end\n\
        endmodule\n");
    assert!(!is_loud(&o) && o.contains("PASS"), "plain-case fn:\n{o}");
}

// ── `unique case` in a subset TASK body (the no-match arm never taken) ──
#[test]
fn unique_case_in_task() {
    let (o, _) = run("module t;\n\
        typedef enum logic [1:0] { M0=0, M1=1 } m_t;\n\
        task automatic classify (input m_t m, output int n);\n\
          unique case (m) M0: n = 28; M1: n = 32; endcase\n\
        endtask\n\
        initial begin int r; classify(M0, r); if (r==28) $display(\"PASS\"); $finish; end\n\
        endmodule\n");
    assert!(!is_loud(&o) && o.contains("PASS"), "unique-case task:\n{o}");
}
