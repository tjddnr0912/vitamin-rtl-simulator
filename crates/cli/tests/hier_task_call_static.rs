//! §4.5.200 (frame↔inline parity, step 3): a hierarchical enable of a STATIC
//! (non-automatic) task `u1.tk(...)` loud→supported. §4.5.197 supported hier calls to
//! AUTOMATIC tasks (they are already framed, so they have a per-instance FuncId); a
//! STATIC task inlines and had no such FuncId, so the hier defer/resolve found nothing →
//! loud. This was the last piece: earlier it would have regressed the task's LOCAL callers
//! (framing loses inline-only capability), but §4.5.198/199 closed the frame ⊂ inline gaps
//! (module writes incl. array elements, multi-dim locals, $display), so frame ⊇ inline and
//! force-framing is safe.
//!
//! Fix: a ONE-TIME pre-scan (`collect_hier_task_stmt` over every module's procedural
//! blocks) records the NAME of every hierarchically-enabled task; `build_task_frame_set`
//! force-frames those, and the existing §4.5.197 `hier_tasks` registration + defer/resolve
//! binds the call. The pre-scan is name-based (framing precedes instance elaboration), so
//! an unrelated same-named task is also framed — harmless now that frame ⊇ inline.
//!
//! Correct-or-loud: an output/inout/string/array formal stays loud (not hier-callable);
//! a task called BOTH locally and hierarchically keeps its local call working (frame path).
//! iverilog is the oracle for every supported case.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_hts_{}_{n}", std::process::id()));
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
fn static_task_hier_call_writes_instance_net() {
    // The original gap: a plain (static) task hier-enabled. iverilog: cnt=2.
    let o = run(
        "module sub; int cnt; task bump(); cnt=cnt+1; endtask endmodule\n\
         module t; sub u();\n\
         initial begin u.bump(); u.bump(); $display(\"cnt=%0d\", u.cnt); $finish; end endmodule\n",
    );
    assert!(o.contains("cnt=2"), "static task hier call:\n{o}");
}

#[test]
fn static_task_register_file_write() {
    // A static task with an input formal + a module ARRAY-element write (register-file
    // idiom) — needs both §4.5.198 (array write) and §4.5.200 (force-frame). iverilog:
    // mem[3]=aa mem[7]=bb.
    let o = run("module dut; logic [7:0] mem[16];\n\
         task write(input int a, input logic [7:0] d); mem[a]=d; endtask\n\
         task show(input int a); $display(\"mem[%0d]=%0h\", a, mem[a]); endtask\n\
         endmodule\n\
         module tb; dut u();\n\
         initial begin u.write(3,8'hAA); u.write(7,8'hBB); u.show(3); u.show(7); $finish; end endmodule\n");
    assert!(
        o.contains("mem[3]=aa") && o.contains("mem[7]=bb"),
        "static task register-file:\n{o}"
    );
}

#[test]
fn static_task_called_locally_and_hierarchically() {
    // THE regression guard: a static task force-framed for a hier call must keep its LOCAL
    // call working (frame ⊇ inline). Local bump() + two hier u.bump() → cnt=3.
    let o = run("module dut; int cnt;\n\
         task bump(); cnt=cnt+1; endtask\n\
         initial bump();\n\
         endmodule\n\
         module tb; dut u();\n\
         initial begin #1; u.bump(); u.bump(); $display(\"cnt=%0d\", u.cnt); $finish; end endmodule\n");
    assert!(o.contains("cnt=3"), "static task local + hier:\n{o}");
}

#[test]
fn static_task_multidim_local_and_control_flow() {
    // A static hier task exercising the §4.5.199 multi-dim local + control flow. iverilog: acc=20.
    let o = run("module dut; int acc;\n\
         task proc(input int n); int m[2][2];\n\
           for(int i=0;i<2;i++) for(int j=0;j<2;j++) m[i][j]=(i+j)*n;\n\
           acc=m[0][1]+m[1][0]+m[1][1];\n\
         endtask\n\
         endmodule\n\
         module tb; dut u(); initial begin u.proc(5); $display(\"acc=%0d\", u.acc); $finish; end endmodule\n");
    assert!(
        o.contains("acc=20"),
        "static hier + 2D local + control flow:\n{o}"
    );
}

#[test]
fn static_task_with_timing_suspends() {
    let o = run("module dut; int t0; task waitset(input int x); #5 t0=x; endtask endmodule\n\
         module tb; dut u();\n\
         initial begin u.waitset(9); $display(\"at %0t t0=%0d\", $time, u.t0); $finish; end endmodule\n");
    assert!(o.contains("at 5 t0=9"), "static hier + timing:\n{o}");
}

#[test]
fn same_task_name_in_two_modules() {
    // Name-based force-framing: two modules each have a static `go`; the hier calls resolve
    // to each instance's own `go` (over-application is harmless). iverilog: a=11 b=110.
    let o = run("module a; int x; task go(input int v); x=v+1; endtask endmodule\n\
         module b; int x; task go(input int v); x=v+100; endtask endmodule\n\
         module tb; a ua(); b ub();\n\
         initial begin ua.go(10); ub.go(10); $display(\"a=%0d b=%0d\", ua.x, ub.x); $finish; end endmodule\n");
    assert!(
        o.contains("a=11 b=110"),
        "same task name in two modules:\n{o}"
    );
}

#[test]
fn static_task_per_instance_isolation() {
    let o = run("module sub #(parameter int K=0); int acc; task add(input int x); acc=acc+x+K; endtask endmodule\n\
         module tb; sub #(.K(100)) u1(); sub #(.K(200)) u2();\n\
         initial begin u1.add(5); u2.add(5); u1.add(1); $display(\"%0d %0d\", u1.acc, u2.acc); $finish; end endmodule\n");
    assert!(
        o.contains("206 205"),
        "static hier per-instance isolation:\n{o}"
    );
}

// ── correct-or-loud boundaries ───────────────────────────────────────────────

#[test]
fn static_task_output_formal_stays_loud() {
    let o = run("module sub; task get(output int y); y=42; endtask endmodule\n\
         module t; sub u(); initial begin int r; u.get(r); $display(\"r=%0d\", r); $finish; end endmodule\n");
    assert!(
        o.contains("E3009") && !o.contains("r=42"),
        "static output-formal hier task must be loud:\n{o}"
    );
}

#[test]
fn static_task_string_formal_stays_loud() {
    let o = run(
        "module sub; string last; task put(input string s); last=s; endtask endmodule\n\
         module t; sub u(); initial begin u.put(\"hi\"); $finish; end endmodule\n",
    );
    assert!(
        o.contains("E3009"),
        "static string-formal hier task must be loud:\n{o}"
    );
}
