//! Round-18 (r18): a hierarchical TASK enable `u1.tk(x);` loud→supported — the callee
//! is a framed task in a child instance, resolved to that instance's per-instance FuncId
//! (`hier_tasks` + `DeferredHierTaskCall` + `resolve_deferred_hier_task_call`), exactly
//! mirroring the hier FUNCTION call (§4.5.196). The task runs on the SUSPENDABLE process
//! path, so it may WRITE the callee instance's own nets — enabled by the r18 infra that
//! makes a blocking assign to an out-of-frame net a suspend signal (`frame_task_module_write.rs`).
//!
//! SCALAR formals of any direction: input/inout copy IN by index (the engine coerces each
//! to the per-instance formal width at frame entry) and output/inout copy OUT to the caller
//! lvalue at the task's exit (§4.5.201). An array/string formal, a non-lvalue output actual,
//! a bad path, wrong arity, or named args stay loud (correct-or-loud). iverilog is the
//! oracle for every supported case here.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_hiertask_{}_{n}", std::process::id()));
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

// ── supported: instance-net write via a hier task enable ─────────────────────

#[test]
fn hier_task_writes_instance_net() {
    // u1.bump() increments sub's own `cnt`; a second hier task reads it. iverilog: cnt=3.
    let o = run("module sub;\n\
         int cnt;\n\
         task automatic bump(); cnt = cnt + 1; endtask\n\
         task automatic show(); $display(\"sub.cnt=%0d\", cnt); endtask\n\
         endmodule\n\
         module t; sub u1();\n\
         initial begin u1.bump(); u1.bump(); u1.bump(); u1.show(); $finish; end endmodule\n");
    assert!(
        o.contains("sub.cnt=3"),
        "hier task instance-net write:\n{o}"
    );
}

#[test]
fn hier_task_input_scalar_formal() {
    let o = run("module sub;\n\
         int acc;\n\
         task automatic add(input int x); acc = acc + x; endtask\n\
         endmodule\n\
         module t; sub u1();\n\
         initial begin u1.add(10); u1.add(32); $display(\"acc=%0d\", u1.acc); $finish; end endmodule\n");
    assert!(o.contains("acc=42"), "hier task input formal:\n{o}");
}

#[test]
fn hier_task_per_instance_isolation() {
    // Two instances with different param overrides — each `u.add` touches its OWN acc/K.
    let o = run("module sub #(parameter int K=0); int acc;\n\
         task automatic add(input int x); acc = acc + x + K; endtask\n\
         task automatic show(); $display(\"K=%0d acc=%0d\", K, acc); endtask\n\
         endmodule\n\
         module t; sub #(.K(100)) u1(); sub #(.K(200)) u2();\n\
         initial begin u1.add(5); u2.add(5); u1.add(1); u1.show(); u2.show(); $finish; end endmodule\n");
    assert!(
        o.contains("K=100 acc=206") && o.contains("K=200 acc=205"),
        "per-instance isolation:\n{o}"
    );
}

#[test]
fn hier_task_deep_path() {
    // top.m.lf.set(77) — a 3-segment instance path resolved by `hier_resolve`.
    let o = run("module leaf; int v;\n\
         task automatic set(input int x); v = x; endtask\n\
         task automatic dump(); $display(\"leaf.v=%0d\", v); endtask\n\
         endmodule\n\
         module mid; leaf lf(); endmodule\n\
         module t; mid m(); initial begin m.lf.set(77); m.lf.dump(); $finish; end endmodule\n");
    assert!(o.contains("leaf.v=77"), "deep hier path:\n{o}");
}

#[test]
fn hier_task_multi_arg_signed_wide() {
    let o = run("module sub; int s;\n\
         task automatic combine(input byte a, input int b, input logic [15:0] c);\n\
           s = a + b + c; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         initial begin u.combine(-8'sd5, 1000, 16'hFFFF); $display(\"s=%0d\", u.s); $finish; end endmodule\n");
    // vita matches iverilog (self-checked): -5 + 1000 + 65535 with SV width/sign rules.
    assert!(o.contains("s=66786"), "multi-arg signed/wide:\n{o}");
}

#[test]
fn hier_task_with_timing_suspends() {
    // The callee has a `#5` — a genuine suspend across the module boundary.
    let o = run("module sub; int t0;\n\
         task automatic wait_set(input int x); #5 t0 = x; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         initial begin u.wait_set(9); $display(\"at %0t t0=%0d\", $time, u.t0); $finish; end endmodule\n");
    assert!(o.contains("at 5 t0=9"), "hier task with timing:\n{o}");
}

#[test]
fn hier_task_local_plus_instance_state() {
    // A body-local `t` plus repeated instance-net read-modify-write (fibonacci).
    let o = run("module sub; int fib_a=0, fib_b=1;\n\
         task automatic step(); int t; t = fib_a + fib_b; fib_a = fib_b; fib_b = t; endtask\n\
         endmodule\n\
         module t; sub u();\n\
         initial begin repeat(6) u.step(); $display(\"a=%0d b=%0d\", u.fib_a, u.fib_b); $finish; end endmodule\n");
    assert!(o.contains("a=8 b=13"), "local + instance state:\n{o}");
}

// ── correct-or-loud boundaries ───────────────────────────────────────────────

#[test]
fn hier_task_output_formal_supported() {
    // §4.5.201: an output formal is a cross-boundary copy-out — the caller lvalue is captured
    // at the call site and written at the task's exit. iverilog: r=42.
    let o = run("module sub; task automatic get(output int y); y = 42; endtask endmodule\n\
         module t; sub u(); initial begin int r; u.get(r); $display(\"r=%0d\", r); $finish; end endmodule\n");
    assert!(o.contains("r=42"), "output-formal hier task:\n{o}");
}

#[test]
fn hier_task_inout_formal_supported() {
    // §4.5.201: inout = copy-in + copy-out. iverilog: r=6.
    let o = run("module sub; task automatic bump(inout int y); y = y + 1; endtask endmodule\n\
         module t; sub u(); initial begin int r = 5; u.bump(r); $display(\"r=%0d\", r); $finish; end endmodule\n");
    assert!(o.contains("r=6"), "inout-formal hier task:\n{o}");
}

#[test]
fn hier_task_output_arg_must_be_lvalue() {
    // correct-or-loud: an output/inout arg must be a writable net/select (not a literal).
    let o = run(
        "module sub; task automatic f(output int y); y = 5; endtask endmodule\n\
         module t; sub u(); initial begin u.f(3); $finish; end endmodule\n",
    );
    assert!(
        o.contains("E3009"),
        "non-lvalue output arg must be loud:\n{o}"
    );
}

#[test]
fn hier_task_string_formal_stays_loud() {
    let o = run("module sub; string last;\n\
         task automatic put(input string s); last = s; endtask endmodule\n\
         module t; sub u(); initial begin u.put(\"hi\"); $finish; end endmodule\n");
    assert!(
        o.contains("E3009"),
        "string-formal hier task must be loud:\n{o}"
    );
}

#[test]
fn hier_task_unknown_stays_loud() {
    let o = run(
        "module sub; task automatic f(input int x); endtask endmodule\n\
         module t; sub u(); initial begin u.nope(5); $display(\"ran\"); $finish; end endmodule\n",
    );
    assert!(
        o.contains("E3009") && !o.contains("ran"),
        "unknown hier task must be loud:\n{o}"
    );
}

#[test]
fn hier_task_wrong_arity_stays_loud() {
    let o = run(
        "module sub; task automatic add(input int a, input int b); endtask endmodule\n\
         module t; sub u(); initial begin u.add(5); $display(\"ran\"); $finish; end endmodule\n",
    );
    assert!(
        o.contains("E3009") && !o.contains("ran"),
        "wrong-arity hier task must be loud:\n{o}"
    );
}

#[test]
fn hier_task_named_args_stays_loud() {
    // Named args can't be positionally reordered without the callee formals at defer time.
    let o = run("module sub; int s; task automatic add(input int a, input int b); s = a + b; endtask endmodule\n\
         module t; sub u(); initial begin u.add(.a(1), .b(2)); $finish; end endmodule\n");
    assert!(
        o.contains("E3009"),
        "named-arg hier task must be loud:\n{o}"
    );
}

#[test]
fn hier_task_nested_in_frame_body_supported() {
    // §4.5.208: a hier enable NESTED inside another frame TASK body is now supported —
    // deferred into task_calls_func, and the caller is forced suspendable via
    // FuncMeta.has_hier_call (format 22→23). Full coverage in `hier_task_nested.rs`. cnt=1.
    let o = run(
        "module sub; int cnt; task automatic bump(); cnt = cnt + 1; endtask endmodule\n\
         module t; sub u();\n\
         task automatic driver(); u.bump(); endtask\n\
         initial begin driver(); $display(\"cnt=%0d\", u.cnt); $finish; end endmodule\n",
    );
    assert!(o.contains("cnt=1"), "nested-in-frame hier task:\n{o}");
}
