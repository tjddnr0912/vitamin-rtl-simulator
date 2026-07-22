//! Frame-body validator (`classify_frame_body`) reachability regression.
//!
//! The frame-TASK subset reject decision is DEFERRED to a post-pass
//! (`resolve_frame_task_rejects`) that runs AFTER every frame body is lowered.
//! The classifier used to scan `block_base..func_blocks.len()` — a linear range
//! whose upper bound, in the post-pass, is the GLOBAL end of all blocks. So a
//! subset task NOT defined last had its scan run off the end of its own body and
//! into LATER-lowered tasks' blocks, mis-reading their (legitimately out-of-frame)
//! output-formal writes as its own and false-rejecting it with E3009.
//!
//! iverilog accepts these; the fix walks only the blocks reachable from the body's
//! own entry via its CFG edges (a `Call` follows `ret_bb`, never the callee entry),
//! so it can never leave the function. These tests pin the fixed behaviour. (r18: a
//! genuine out-of-frame write is no longer loud — a blocking assign to a module net is
//! now a suspend signal, so such a task lifts to the suspendable path and runs; see
//! `out_of_frame_write_now_supported`.)
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_overscan_{}_{n}", std::process::id()));
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

// ── the exact repro: two automatic subset tasks; the FIRST (not defined last) must
//    not be false-rejected by over-scanning into the second's body ──
#[test]
fn two_subset_tasks_first_not_over_rejected() {
    let o = run("module top;\n\
         task automatic p(output int r);\n\
           int x; x = 6; r = x;\n\
         endtask\n\
         task automatic q(output int r);\n\
           int y; y = 7; r = y;\n\
         endtask\n\
         int a, b;\n\
         initial begin\n\
           p(a); q(b);\n\
           $display(\"p=%0d q=%0d\", a, b);\n\
         end\n\
         endmodule\n");
    assert!(
        !o.contains("E3009"),
        "a subset task defined before another must NOT be rejected:\n{o}"
    );
    assert!(
        o.contains("p=6 q=7"),
        "both tasks must run (iverilog: p=6 q=7):\n{o}"
    );
}

// ── three subset tasks: the MIDDLE one (surrounded on both sides) also runs ──
#[test]
fn three_subset_tasks_middle_runs() {
    let o = run("module top;\n\
         task automatic a1(output int r); r = 1; endtask\n\
         task automatic a2(output int r); int t; t = 2; r = t; endtask\n\
         task automatic a3(output int r); r = 3; endtask\n\
         int x, y, z;\n\
         initial begin\n\
           a1(x); a2(y); a3(z);\n\
           $display(\"%0d %0d %0d\", x, y, z);\n\
         end\n\
         endmodule\n");
    assert!(!o.contains("E3009"), "no subset task may be rejected:\n{o}");
    assert!(o.contains("1 2 3"), "all three tasks must run:\n{o}");
}

// ── automatic FUNCTIONS validate inline (not via the post-pass), so they never had
//    the over-scan bug — but the fix reroutes their validation start to the true
//    entry block; guard that a not-last automatic function still lowers + runs ──
#[test]
fn two_subset_functions_first_runs() {
    let o = run("module top;\n\
         function automatic int f(input int v); int t; t = v + 1; return t; endfunction\n\
         function automatic int g(input int v); int t; t = v + 2; return t; endfunction\n\
         initial $display(\"%0d %0d\", f(10), g(10));\n\
         endmodule\n");
    assert!(
        !o.contains("E3009"),
        "neither function may be rejected:\n{o}"
    );
    assert!(o.contains("11 12"), "both functions must run (11 12):\n{o}");
}

// ── r18: a write to a net OUTSIDE the frame (a module net) from an `automatic`
//    (framed) task is now SUPPORTED — the blocking assign to `g` is a suspend signal
//    (`compute_suspendable_tasks` is frame-aware), so the task lifts to the suspendable
//    path where the process executor's `&mut` write reaches the module net. iverilog: g=5.
#[test]
fn out_of_frame_write_now_supported() {
    let o = run("module top;\n\
         int g;\n\
         task automatic writes_module(input int v);\n\
           int x; x = v; g = x;\n\
         endtask\n\
         initial begin writes_module(5); $display(\"g=%0d\", g); end\n\
         endmodule\n");
    assert!(
        !o.contains("E3009"),
        "an automatic task writing a module net is now supported (r18):\n{o}"
    );
    assert!(
        o.contains("g=5"),
        "module-net write must run (iverilog: g=5):\n{o}"
    );
}
