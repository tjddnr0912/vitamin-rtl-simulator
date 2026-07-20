//! §4.5.175: a `foreach` over a dynamic array / queue / associative array (and a direct
//! `a.first/next/last/prev(k)`) inside a FUNCTION body, or a SUBSET (non-suspendable)
//! task body, is fatal-loud instead of silently returning 0.
//!
//! The lowered walk step `__st = a.first/next(__i)` WRITES the iteration key as a side
//! effect. The process executor does that via `&mut write_lvalue`; the SYNCHRONOUS frame
//! executors (`run_frame_call` for functions, `run_task` for subset tasks) are `&self`
//! and cannot advance the key — a plain eval left it stuck, so the loop never iterated
//! and the body silently returned 0 (a silent-wrong found by adversarial review). Now it
//! latches a runtime fatal (correct-or-loud). SUSPENDABLE tasks (a `$display`/`@`/`#`
//! makes the task run on the `&mut` process executor) and module processes are unaffected,
//! and a `for (int i = 0; i < a.size(); i++)` loop works everywhere (the fix's suggested
//! workaround). A fixed-size array `foreach` and a direct dynamic index/`.size()` read
//! inside a function were never affected.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ffdl_{}_{n}", std::process::id()));
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

/// The run ended in a loud fatal (not a clean quiescent/finish) — the silent-wrong guard.
fn is_loud(o: &str) -> bool {
    o.contains("F4004") || o.contains("ended (Error)")
}

// ── FUNCTION + dynamic-array foreach → loud (was a silent 0) ──
#[test]
fn function_dyn_foreach_is_loud() {
    let o = run("module top;\n\
         int marr[];\n\
         function automatic int g();\n\
           int s=0; foreach(marr[i]) s+=marr[i]; return s;\n\
         endfunction\n\
         initial begin marr=new[3]; marr[0]=4;marr[1]=5;marr[2]=6; $display(\"g=%0d\", g()); end\n\
         endmodule\n");
    assert!(
        is_loud(&o),
        "dyn foreach in a function must be fatal-loud, not a silent 0:\n{o}"
    );
}

// ── FUNCTION + queue foreach → loud ──
#[test]
fn function_queue_foreach_is_loud() {
    let o = run("module top;\n\
         int mq[$];\n\
         function automatic int g(); int s=0; foreach(mq[i]) s+=mq[i]; return s; endfunction\n\
         initial begin mq.push_back(3); mq.push_back(4); $display(\"g=%0d\", g()); end\n\
         endmodule\n");
    assert!(
        is_loud(&o),
        "queue foreach in a function must be loud:\n{o}"
    );
}

// ── FUNCTION + direct associative-array iterator → loud ──
#[test]
fn function_assoc_first_is_loud() {
    let o = run("module top;\n\
         int m[int];\n\
         function automatic int g(); int k; int st; st=m.first(k); return k; endfunction\n\
         initial begin m[5]=1; m[9]=2; $display(\"g=%0d\", g()); end\n\
         endmodule\n");
    assert!(
        is_loud(&o),
        "a direct assoc first() in a function must be loud:\n{o}"
    );
}

// ── SUBSET (non-suspendable) task + dynamic foreach → loud (run_task path) ──
#[test]
fn subset_task_dyn_foreach_is_loud() {
    let o = run("module top;\n\
         int marr[];\n\
         task automatic t(output int s); s=0; foreach(marr[i]) s+=marr[i]; endtask\n\
         int r; initial begin marr=new[3]; marr[0]=4;marr[1]=5;marr[2]=6; t(r); $display(\"r=%0d\", r); end\n\
         endmodule\n");
    assert!(
        is_loud(&o),
        "dyn foreach in a subset task must be loud:\n{o}"
    );
}

// ── a SUSPENDABLE task ($display makes it run on the process executor) still works ──
#[test]
fn suspendable_task_dyn_foreach_works() {
    let o = run("module top;\n\
         int marr[];\n\
         task automatic t(); int s=0; foreach(marr[i]) s+=marr[i]; $display(\"t=%0d\", s); endtask\n\
         initial begin marr=new[3]; marr[0]=4;marr[1]=5;marr[2]=6; t(); end\n\
         endmodule\n");
    assert!(
        !is_loud(&o),
        "a suspendable task must NOT be caught by the guard:\n{o}"
    );
    assert!(
        o.contains("t=15"),
        "suspendable task dyn foreach must still work (t=15):\n{o}"
    );
}

// ── the suggested workaround: a `for` loop over `.size()` works inside a function ──
#[test]
fn function_for_loop_over_size_works() {
    let o = run("module top;\n\
         int marr[];\n\
         function automatic int g();\n\
           int s=0; for (int i=0; i<marr.size(); i++) s+=marr[i]; return s;\n\
         endfunction\n\
         initial begin marr=new[3]; marr[0]=4;marr[1]=5;marr[2]=6; $display(\"g=%0d\", g()); end\n\
         endmodule\n");
    assert!(!is_loud(&o), "a for-loop workaround must not be loud:\n{o}");
    assert!(
        o.contains("g=15"),
        "for-loop over size must work (g=15):\n{o}"
    );
}

// ── regression: a FIXED-size array foreach inside a function was never affected ──
#[test]
fn function_fixed_array_foreach_works() {
    let o = run("module top;\n\
         int marr[0:2];\n\
         function automatic int g(); int s=0; foreach(marr[i]) s+=marr[i]; return s; endfunction\n\
         initial begin marr[0]=4;marr[1]=5;marr[2]=6; $display(\"g=%0d\", g()); end\n\
         endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("g=15"),
        "fixed-array foreach in a function must work:\n{o}"
    );
}

// ── regression: a direct dynamic index / .size() read inside a function still works ──
#[test]
fn function_direct_dyn_read_works() {
    let o = run("module top;\n\
         int marr[];\n\
         function automatic int g(); return marr[1] + marr.size(); endfunction\n\
         initial begin marr=new[3]; marr[0]=4;marr[1]=5;marr[2]=6; $display(\"g=%0d\", g()); end\n\
         endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("g=8"),
        "direct dyn read in a function must work (5+3=8):\n{o}"
    );
}
