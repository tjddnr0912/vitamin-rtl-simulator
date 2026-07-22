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
//! r19 (accuracy-first frame/inline parity): the signal rule was broadened to ANY
//! out-of-frame write chunk — a module ARRAY-element write (`mem[i]=v`) and a concat-target
//! chunk (`{a,b}=x`) now lift too, so the suspendable `&mut` executor performs them. This
//! closes a real gap that existed independent of hier calls: `task automatic; mem[i]=v;`
//! was loud in vita but accepted by every reference sim. An IN-FRAME `word`-indexed write
//! (a frame-local array element, or a class-field heap write through an in-frame handle)
//! is still NOT marked — it stays a subset the `&self` executor runs.
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
fn module_array_element_write_supported() {
    // r19: a module ARRAY-element write (`mem[i]=v`, word-indexed, out of frame) now lifts
    // to the suspendable path and runs — was loud in r18. iverilog: mem2=99.
    let o = run("module t; int mem[4];\n\
         task automatic put(input int i, input int v); mem[i] = v; endtask\n\
         initial begin put(2, 99); $display(\"mem2=%0d\", mem[2]); $finish; end endmodule\n");
    assert!(
        o.contains("mem2=99"),
        "module array-element write must run:\n{o}"
    );
}

#[test]
fn automatic_task_runtime_index_array_fill() {
    // The register-file idiom: a runtime-index loop writing a module array. iverilog: 100 103 107.
    let o = run("module t; int mem[8];\n\
         task automatic fill(input int base); for (int i = 0; i < 8; i++) mem[i] = base + i; endtask\n\
         initial begin fill(100); $display(\"%0d %0d %0d\", mem[0], mem[3], mem[7]); $finish; end endmodule\n");
    assert!(o.contains("100 103 107"), "runtime-index array fill:\n{o}");
}
