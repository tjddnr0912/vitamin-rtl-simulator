//! A procedural read of a whole-net copy (`assign c = v;`) after the reader's own
//! blocking write of `v`, where the read or the write sits inside a FRAME body — an
//! `automatic` task or function the process calls — or in the call's actual.
//! ROADMAP §2 🆕 I ⓓ/ⓖ (review of §4.5.408).
//!
//! `levelize::proc_read_alias` walked the process body only: a write inside a callee
//! was invisible to the writer predicate (`task automatic wr(); v = x; endtask` then
//! `$display(c)`), a read inside a callee (`y = c;`) and the call's in-bind actual
//! (`tk(c, r2)`, whose expression is in the task-call sidecar, not in a statement)
//! were never marked. The read stayed on the settle's value (`xx`) where iverilog
//! 13.0 and verilator 5.050 both print the fresh `a5`. The walk now covers the
//! callee bodies the process runs (`Terminator::Call` targets and `Expr::Call`
//! functions, transitively) and the sidecar's in-binds.
//!
//! Every value here was measured on iverilog 13.0 AND verilator 5.050 (26 cells);
//! the lines are the oracles' output, copied. Cells whose two oracles disagree
//! (a `for` loop re-calling the task — iverilog `a0 a1`, verilator `00 a1`; an NBA
//! write before the call — `xx` / `00`) are not pinned.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_backend(src: &str, backend: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_crtf_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--backend")
        .arg(backend)
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

fn prints_all(src: &str, want: &[&str]) {
    for b in ["native", "interp", "vm"] {
        let (out, code) = run_backend(src, b);
        assert_eq!(code, Some(0), "[{b}] exit\n{out}");
        let got: Vec<&str> = out.lines().filter(|l| l.starts_with("D=")).collect();
        assert_eq!(got, want, "[{b}]\n{out}");
    }
}

/// `v` with its copy `c`, plus `decls`, then one `initial` running `body`.
fn top(decls: &str, body: &str) -> String {
    format!(
        "`timescale 1ns/1ns\nmodule top;\n  logic [7:0] v, c;\n  assign c = v;\n  {decls}\n  \
         logic [7:0] r2;\n  initial begin {body} #5 $finish; end\nendmodule\n"
    )
}

const TK: &str = "task automatic tk(input logic [7:0] x, output logic [7:0] y); y = x; endtask";

#[test]
fn an_automatic_task_actual_reads_through_the_copy() {
    // ⓖ's exact shape: the actual `c` is the call's in-bind (sidecar), not a statement.
    prints_all(
        &top(TK, "v = 8'hA5; tk(c, r2); $display(\"D=%h\", r2);"),
        &["D=a5"],
    );
    // an expression actual, a part-select actual, two actuals
    prints_all(
        &top(TK, "v = 8'hA5; tk(c + 8'd1, r2); $display(\"D=%h\", r2);"),
        &["D=a6"],
    );
    prints_all(
        &top(
            "task automatic tk(input logic [3:0] x, output logic [3:0] y); y = x; endtask",
            "v = 8'hA5; tk(c[7:4], r2[3:0]); $display(\"D=%h\", r2[3:0]);",
        ),
        &["D=a"],
    );
    prints_all(
        &top(
            "logic [7:0] w, d; assign d = w;\n  task automatic tk(input logic [7:0] x, input logic [7:0] z, output logic [7:0] y); y = x ^ z; endtask",
            "v = 8'hA5; w = 8'h0F; tk(c, d, r2); $display(\"D=%h\", r2);",
        ),
        &["D=aa"],
    );
}

#[test]
fn a_read_inside_the_frame_body_reads_through() {
    // the callee reads `c` itself (no actual), with and without a delay first
    prints_all(
        &top(
            "task automatic tk(output logic [7:0] y); y = c; endtask",
            "v = 8'hA5; tk(r2); $display(\"D=%h\", r2);",
        ),
        &["D=a5"],
    );
    prints_all(
        &top(
            "task automatic tk(input logic [7:0] x, output logic [7:0] y); #1 y = x; endtask",
            "v = 8'hA5; tk(c, r2); $display(\"D=%h\", r2);",
        ),
        &["D=a5"],
    );
    // a recursive task (the read is in the innermost frame) and a nested task pair
    prints_all(
        &top(
            "task automatic tk(input int n, input logic [7:0] x, output logic [7:0] y);\n    if (n == 0) y = x; else tk(n - 1, x, y);\n  endtask",
            "v = 8'hA5; tk(2, c, r2); $display(\"D=%h\", r2);",
        ),
        &["D=a5"],
    );
    prints_all(
        &top(
            "task automatic inner(output logic [7:0] y); y = c; endtask\n  task automatic outer(output logic [7:0] y); inner(y); endtask",
            "v = 8'hA5; outer(r2); $display(\"D=%h\", r2);",
        ),
        &["D=a5"],
    );
    // review B-1: a nested task whose NAME sorts after its caller's (its body lowers
    // later, so the caller's `Call` target was the reservation placeholder) — the
    // walk resolves the callee through the sidecar, in either name order
    prints_all(
        &top(
            "task automatic zz(output logic [7:0] y); y = c; endtask\n  task automatic aa(output logic [7:0] y); zz(y); endtask",
            "v = 8'hA5; aa(r2); $display(\"D=%h\", r2);",
        ),
        &["D=a5"],
    );
    prints_all(
        &top(
            "task automatic aa(output logic [7:0] y); y = c; endtask\n  task automatic zz(output logic [7:0] y); aa(y); endtask",
            "v = 8'hA5; zz(r2); $display(\"D=%h\", r2);",
        ),
        &["D=a5"],
    );
    // a recursive FUNCTION (an `Expr::Call`, not a `Terminator::Call`)
    prints_all(
        &top(
            "function automatic logic [7:0] f(input int n); if (n == 0) return c; else return f(n - 1); endfunction",
            "v = 8'hA5; r2 = f(2); $display(\"D=%h\", r2);",
        ),
        &["D=a5"],
    );
}

#[test]
fn a_write_inside_the_frame_body_counts_as_the_process_write() {
    // ⓓ: the blocking write is in the callee; the read is back in the process
    prints_all(
        &top(
            "task automatic wr(input logic [7:0] x); v = x; endtask",
            "wr(8'hA5); $display(\"D=%h\", c);",
        ),
        &["D=a5"],
    );
    // an array-word copy written inside the callee
    prints_all(
        "`timescale 1ns/1ns\nmodule top;\n  logic [7:0] m [0:3];\n  logic [7:0] c;\n  assign c = m[2];\n  \
         task automatic wr(); m[2] = 8'hA5; endtask\n  initial begin wr(); $display(\"D=%h\", c); #5 $finish; end\nendmodule\n",
        &["D=a5"],
    );
    // a second call after a second write sees the second value
    prints_all(
        &top(
            TK,
            "v = 8'h11; tk(c, r2); v = 8'hA5; tk(c, r2); $display(\"D=%h\", r2);",
        ),
        &["D=a5"],
    );
}

#[test]
fn the_call_from_a_generate_process_and_the_controls_are_unchanged() {
    prints_all(
        "`timescale 1ns/1ns\nmodule top;\n  logic [7:0] v, c;\n  assign c = v;\n  \
         task automatic tk(input logic [7:0] x, output logic [7:0] y); y = x; endtask\n  logic [7:0] r2;\n  \
         generate if (1) begin : gi\n    initial begin v = 8'hA5; tk(c, r2); $display(\"D=%h\", r2); end\n  end endgenerate\n  \
         initial #5 $finish;\nendmodule\n",
        &["D=a5"],
    );
    // controls (PRE == POST == oracles): a static task, an inlined function, a fork
    // arm, an inout, a plain read
    prints_all(
        &top(
            "task tk(input logic [7:0] x, output logic [7:0] y); y = x; endtask",
            "v = 8'hA5; tk(c, r2); $display(\"D=%h\", r2);",
        ),
        &["D=a5"],
    );
    prints_all(
        &top(
            "function automatic logic [7:0] f(input logic [7:0] x); return x; endfunction",
            "v = 8'hA5; r2 = f(c); $display(\"D=%h\", r2);",
        ),
        &["D=a5"],
    );
    prints_all(
        &top("", "fork v = 8'hA5; join $display(\"D=%h\", c);"),
        &["D=a5"],
    );
    prints_all(
        &top(
            TK,
            "v = 8'hA5; fork tk(c, r2); join $display(\"D=%h\", r2);",
        ),
        &["D=a5"],
    );
    prints_all(
        &top(
            "task automatic tk(inout logic [7:0] x); x = x + 1; endtask",
            "v = 8'hA5; r2 = c; tk(r2); $display(\"D=%h\", r2);",
        ),
        &["D=a6"],
    );
    prints_all(&top("", "v = 8'hA5; $display(\"D=%h\", c);"), &["D=a5"]);
}

#[test]
fn a_callee_shared_with_a_non_writing_caller_keeps_the_settle_value() {
    // Recorded residue: the callee body is one set of expressions; it is marked only
    // for the roots EVERY caller writes. A second caller that never writes `v` keeps
    // the first caller's read on the settle's value (both oracles `a5 a5`; PRE ==
    // POST `xx a5`). Pinned as PRE's shape so a change here is a deliberate one.
    let src = "`timescale 1ns/1ns\nmodule top;\n  logic [7:0] v, c;\n  assign c = v;\n  \
         task automatic wr(); v = 8'hA5; endtask\n  task automatic rd(output logic [7:0] y); y = c; endtask\n  \
         logic [7:0] r1, r2;\n  initial begin wr(); rd(r1); $display(\"D=%h\", r1); end\n  \
         initial begin #1 rd(r2); $display(\"D=%h\", r2); #5 $finish; end\nendmodule\n";
    prints_all(src, &["D=xx", "D=a5"]);
}
