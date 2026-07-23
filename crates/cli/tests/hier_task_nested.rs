//! §4.5.208: a hierarchical TASK enable (`u.tk(...)`) NESTED inside another frame-task body
//! loud→supported. §4.5.197/201 deferred hier enables only from a TOP-LEVEL process; a
//! nested one was loud because its placeholder `Call.target` is patched only at the
//! finish-phase resolve — AFTER the per-instance `compute_suspendable_tasks` runs — so the
//! elaborate (pre-resolve) and engine (post-resolve) suspend classifications would diverge,
//! breaking the §4.5.197 pure-function contract.
//!
//! Fix: the deferred hier call is now allowed during frame-body lowering (keyed into
//! `task_calls_func` + patched in `func_blocks` at resolve, rebased like `pending_task_calls`),
//! and the caller task's `FuncMeta.has_hier_call` is set so `compute_suspendable_tasks` FORCES
//! it suspendable CONSISTENTLY in both computes (a sound over-approximation — the callee may
//! suspend). This is a `format_version` bump (22→23: the `FuncMeta` staged-trailer sidecar
//! gains `has_hier_call`).
//!
//! ORACLE: unlike an array formal, iverilog DOES support these scalar hier enables, so every
//! supported case here is iverilog-differential (including the critical suspendable-callee).
//!
//! Correct-or-loud: named args, and a frame-FORMAL array forwarded through a nested enable
//! (the actual is an md-packed frame net, not a static array), stay loud.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_htn_{}_{n}", std::process::id()));
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

// ── supported (iverilog-differential) ────────────────────────────────────────

#[test]
fn nested_hier_enable_basic() {
    // driver() (a frame task) calls u.bump() (a hier enable). iverilog: cnt=1.
    let o = run(
        "module sub; int cnt; task automatic bump(); cnt=cnt+1; endtask endmodule\n\
         module t; sub u();\n\
         task automatic driver(); u.bump(); endtask\n\
         initial begin driver(); $display(\"cnt=%0d\",u.cnt); $finish; end endmodule\n",
    );
    assert!(o.contains("cnt=1"), "nested hier enable:\n{o}");
}

#[test]
fn nested_hier_enable_repeated() {
    // driver called 3× — the callee accumulates in the child instance. iverilog: cnt=3.
    let o = run("module sub; int cnt; task automatic bump(); cnt=cnt+1; endtask endmodule\n\
         module t; sub u();\n\
         task automatic driver(); u.bump(); endtask\n\
         initial begin driver(); driver(); driver(); $display(\"cnt=%0d\",u.cnt); $finish; end endmodule\n");
    assert!(o.contains("cnt=3"), "repeated nested hier:\n{o}");
}

#[test]
fn nested_hier_suspendable_callee() {
    // CRITICAL: the hier callee has a `#5` (SUSPENDABLE). `has_hier_call` must force the
    // caller `driver` suspendable, or the `&self` subset executor would drop the suspend.
    // iverilog: at 5 t0=9.
    let o = run(
        "module sub; int t0; task automatic waitset(input int x); #5 t0=x; endtask endmodule\n\
         module t; sub u();\n\
         task automatic driver(input int v); u.waitset(v); endtask\n\
         initial begin driver(9); $display(\"at %0t t0=%0d\",$time,u.t0); $finish; end endmodule\n",
    );
    assert!(
        o.contains("at 5 t0=9"),
        "suspendable nested hier callee:\n{o}"
    );
}

#[test]
fn nested_hier_input_formal_passthrough() {
    // The driver forwards scalar args to the hier callee's input formals. iverilog: acc=42.
    let o = run(
        "module sub; int acc; task automatic add(input int x); acc=acc+x; endtask endmodule\n\
         module t; sub u();\n\
         task automatic driver(input int a, input int b); u.add(a); u.add(b); endtask\n\
         initial begin driver(10,32); $display(\"acc=%0d\",u.acc); $finish; end endmodule\n",
    );
    assert!(o.contains("acc=42"), "input formal passthrough:\n{o}");
}

#[test]
fn nested_hier_output_formal() {
    // The hier callee has an OUTPUT formal; the copy-out reaches the driver's actual.
    // iverilog: x=42.
    let o = run(
        "module sub; task automatic get(output int y); y=42; endtask endmodule\n\
         module t; sub u();\n\
         task automatic driver(output int r); u.get(r); endtask\n\
         initial begin int x; driver(x); $display(\"x=%0d\",x); $finish; end endmodule\n",
    );
    assert!(o.contains("x=42"), "nested hier output formal:\n{o}");
}

#[test]
fn nested_hier_with_local_work() {
    // The driver does its own module-net work AND a hier call (the caller frame task both
    // writes a module net and enables a hier task). iverilog: lc=20 cnt=2.
    let o = run("module sub; int cnt; task automatic bump(); cnt=cnt+1; endtask endmodule\n\
         module t; sub u(); int local_cnt;\n\
         task automatic driver(); local_cnt=local_cnt+10; u.bump(); endtask\n\
         initial begin driver(); driver(); $display(\"lc=%0d cnt=%0d\",local_cnt,u.cnt); $finish; end endmodule\n");
    assert!(o.contains("lc=20 cnt=2"), "nested hier + local work:\n{o}");
}

#[test]
fn nested_hier_per_instance_isolation() {
    // The driver enables tasks on two different instances. iverilog: 105 205.
    let o = run("module sub #(parameter int K=0); int acc; task automatic add(input int x); acc=acc+x+K; endtask endmodule\n\
         module t; sub #(.K(100)) u1(); sub #(.K(200)) u2();\n\
         task automatic drive(input int v); u1.add(v); u2.add(v); endtask\n\
         initial begin drive(5); $display(\"%0d %0d\",u1.acc,u2.acc); $finish; end endmodule\n");
    assert!(o.contains("105 205"), "per-instance via nested hier:\n{o}");
}

// ── correct-or-loud boundaries ───────────────────────────────────────────────

#[test]
fn nested_hier_named_args_stays_loud() {
    let o = run("module sub; int s; task automatic add(input int a, input int b); s=a+b; endtask endmodule\n\
         module t; sub u();\n\
         task automatic driver(); u.add(.a(1),.b(2)); endtask\n\
         initial begin driver(); $finish; end endmodule\n");
    assert!(
        o.contains("E3009"),
        "named args in nested hier must be loud:\n{o}"
    );
}

#[test]
fn nested_hier_forwarded_array_formal_stays_loud() {
    // Forwarding the driver's OWN array formal through a nested hier enable — the actual is
    // an md-packed frame net (not a static array), so it stays loud (a follow-on).
    let o = run("module sub; int acc; task automatic p(input int d[3]); acc=d[0]+d[1]+d[2]; endtask endmodule\n\
         module t; sub u();\n\
         task automatic driver(input int d[3]); u.p(d); endtask\n\
         initial begin int a[3]; a[0]=1;a[1]=2;a[2]=4; driver(a); $display(\"acc=%0d\",u.acc); $finish; end endmodule\n");
    assert!(
        o.contains("E3009"),
        "forwarded frame-formal array through nested hier must be loud:\n{o}"
    );
}
