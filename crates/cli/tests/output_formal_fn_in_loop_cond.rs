//! r18 (F1): a function with an OUTPUT/INOUT formal called in a `while`/`for` loop
//! CONDITION (`while (getnext(i, v) == 1) …`) is now supported — the call is hoisted to a
//! copy-out temp at the loop head, which is re-entered on every iteration, so the call
//! (and its output copy-out) runs once per iteration exactly like the un-hoisted form.
//! Was E3009 "an inout/output-function call in a `while` condition is unsupported".
//!
//! (Function output/inout formals themselves are supported via the R5-B copy-out path;
//! the gap was only the loop-condition position.)
//!
//! ORACLE: iverilog 13.0 rejects a function with an output port ("port is not an input
//! port"), so this is hand-IEEE, cross-checked by hand-tracing the driver loop.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_offlc_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn is_loud(o: &str) -> bool {
    o.contains("E3009")
}

const GETNEXT: &str = "int src[4] = '{10,20,30,0};\n\
    function automatic int getnext (input int fd, output int val);\n\
      val = src[fd];\n\
      getnext = (src[fd] != 0);\n\
    endfunction\n";

// ── the report's F1: output-formal fn in a `while` condition ──
#[test]
fn output_fn_in_while_condition() {
    let o = run(&format!(
        "module t;\n{GETNEXT}\
        initial begin\n\
          int i = 0, v;\n\
          while (getnext(i, v) == 1) begin $display(\"v=%0d\", v); i++; end\n\
          $display(\"PASS\"); $finish;\n\
        end\n\
        endmodule\n"
    ));
    assert!(
        !is_loud(&o)
            && o.contains("v=10")
            && o.contains("v=20")
            && o.contains("v=30")
            && o.contains("PASS"),
        "F1 repro (while):\n{o}"
    );
}

// ── same call in a `for`-loop condition ──
#[test]
fn output_fn_in_for_condition() {
    let o = run(&format!(
        "module t;\n{GETNEXT}\
        initial begin\n\
          int v;\n\
          for (int i = 0; getnext(i, v) == 1; i++) $display(\"v=%0d\", v);\n\
          $display(\"DONE\"); $finish;\n\
        end\n\
        endmodule\n"
    ));
    assert!(
        !is_loud(&o)
            && o.contains("v=10")
            && o.contains("v=20")
            && o.contains("v=30")
            && o.contains("DONE"),
        "F1 (for):\n{o}"
    );
}

// ── the call is the WHOLE condition (no comparison) ──
#[test]
fn output_fn_is_whole_condition() {
    let o = run(&format!(
        "module t;\n{GETNEXT}\
        initial begin\n\
          int i = 0, v, cnt = 0;\n\
          while (getnext(i, v)) begin cnt++; i++; end\n\
          $display(\"cnt=%0d\", cnt); $finish;\n\
        end\n\
        endmodule\n"
    ));
    // 3 non-zero entries (10,20,30) then 0 stops → cnt=3.
    assert!(!is_loud(&o) && o.contains("cnt=3"), "whole-cond call:\n{o}");
}

// ── regression: an output-formal fn on the DIRECT rhs (R5-B) still works ──
#[test]
fn output_fn_direct_rhs_unchanged() {
    let o = run(&format!(
        "module t;\n{GETNEXT}\
        initial begin\n\
          int v, st;\n\
          st = getnext(1, v);\n\
          $display(\"st=%0d v=%0d\", st, v); $finish;\n\
        end\n\
        endmodule\n"
    ));
    assert!(!is_loud(&o) && o.contains("st=1 v=20"), "direct rhs:\n{o}");
}

// ── an output-formal fn INSIDE the loop body (assigned form) ──
#[test]
fn output_fn_in_loop_body() {
    let o = run(&format!(
        "module t;\n{GETNEXT}\
        initial begin\n\
          int v, sum = 0, st;\n\
          for (int i = 0; i < 3; i++) begin\n\
            st = getnext(i, v); sum += v;\n\
          end\n\
          $display(\"sum=%0d\", sum); $finish;\n\
        end\n\
        endmodule\n"
    ));
    assert!(!is_loud(&o) && o.contains("sum=60"), "loop-body call:\n{o}");
}
