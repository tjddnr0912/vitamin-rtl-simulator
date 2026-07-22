//! Round-18 (r18) infra: an `automatic` (framed) task that WRITES a module/instance net
//! via a blocking assign was loud ("an assignment to a net outside the function") because
//! a blocking-only body classified as a NON-suspendable subset, and the `&self` subset
//! executor cannot write a module net. r18 makes `compute_suspendable_tasks` frame-aware:
//! a blocking assign whose lhs writes a net OUTSIDE the task's own frame window is a
//! SUSPEND SIGNAL, so the task lifts to the suspendable process path where the `&mut`
//! write reaches the module net (mirroring how a $display/NBA/# already forced suspension).
//! This is the prerequisite that lets a hierarchical task enable (`hier_task_call.rs`)
//! mutate a child instance's state. iverilog is the oracle for the supported cases.
//!
//! Correct-or-loud: a `word`-indexed out-of-frame write (a module ARRAY element `mem[i]=v`)
//! is NOT marked a signal (it stays a subset part-select reject) — supporting it needs the
//! `&mut` array-element path plumbed through the lift, a separate follow-on.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fmw_{}_{n}", std::process::id()));
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

#[test]
fn automatic_task_whole_module_write() {
    // Was E3009. iverilog: cnt=2.
    let o = run("module t;\n\
         int cnt;\n\
         task automatic bump(); cnt = cnt + 1; endtask\n\
         initial begin bump(); bump(); $display(\"cnt=%0d\", cnt); $finish; end endmodule\n");
    assert!(o.contains("cnt=2"), "whole module-net write:\n{o}");
}

#[test]
fn automatic_task_module_write_with_input() {
    let o = run("module t;\n\
         int acc;\n\
         task automatic addv(input int x); acc = acc + x; endtask\n\
         initial begin addv(10); addv(32); $display(\"acc=%0d\", acc); $finish; end endmodule\n");
    assert!(o.contains("acc=42"), "module write + input formal:\n{o}");
}

#[test]
fn automatic_task_module_part_select_write() {
    // A part-select (word=None) write to a module net is also supported (suspendable).
    // iverilog: reg8=fa.
    let o = run("module t; logic [7:0] reg8;\n\
         task automatic setlow(input logic [3:0] v); reg8[3:0] = v; endtask\n\
         initial begin reg8 = 8'hF0; setlow(4'hA); $display(\"reg8=%h\", reg8); $finish; end endmodule\n");
    assert!(o.contains("reg8=fa"), "part-select module write:\n{o}");
}

#[test]
fn automatic_task_read_modify_write() {
    // Repeated read-modify-write of a module net across calls (static acc, per-call frame).
    let o = run("module t; int sum;\n\
         task automatic acc(input int x); sum = sum + x; endtask\n\
         initial begin for (int i = 1; i <= 5; i++) acc(i); $display(\"sum=%0d\", sum); $finish; end endmodule\n");
    assert!(o.contains("sum=15"), "read-modify-write:\n{o}");
}

#[test]
fn subset_task_only_frame_locals_still_works() {
    // A task that writes ONLY frame-locals + an output formal stays a subset (unchanged) —
    // r18 only marks OUT-OF-FRAME writes, so this is not over-lifted. iverilog: r=31.
    let o = run("module t;\n\
         task automatic compute(input int x, output int y); int tmp; tmp = x * 3; y = tmp + 1; endtask\n\
         initial begin int r; compute(10, r); $display(\"r=%0d\", r); $finish; end endmodule\n");
    assert!(
        o.contains("r=31"),
        "frame-local-only subset unchanged:\n{o}"
    );
}

#[test]
fn module_array_element_write_stays_loud() {
    // BOUNDARY: a module ARRAY-element write (`mem[i]=v`, word-indexed, out of frame) is
    // NOT lifted — it stays a subset part-select/array-element reject (correct-or-loud).
    let o = run("module t; int mem[4];\n\
         task automatic put(input int i, input int v); mem[i] = v; endtask\n\
         initial begin put(2, 99); $display(\"mem2=%0d\", mem[2]); $finish; end endmodule\n");
    assert!(
        o.contains("E3009"),
        "module array-element write must stay loud:\n{o}"
    );
}
