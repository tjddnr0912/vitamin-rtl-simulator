//! §4.5.176: a `foreach` over a dynamic array / queue / associative array (and a direct
//! `a.first/next/last/prev(k)` iteration loop) inside a FUNCTION body, or a SUBSET
//! (non-suspendable) task body, is SUPPORTED.
//!
//! The lowered walk step `__st = a.first/next(__i)` WRITES the iteration key as a side
//! effect. The `&mut` process executor does that via `write_lvalue`; the SYNCHRONOUS
//! frame executors (`run_frame_call` for functions, `run_task` for subset tasks) are
//! `&self` but the key is a FRAME-LOCAL — so they now advance it through the interior-
//! mutable frame window (`frame_assoc_iter` → `frame_write_lvalue`), driven by the same
//! `assoc_iter_compute` the process path uses. §4.5.175 made this fatal-loud (it used to
//! silently return 0); §4.5.176 makes it correct. A direct iterator whose key is NOT a
//! local (`st = aa.first(module_net)` in a function) still needs a `&mut` module-net
//! write and stays loud (correct-or-loud). iverilog cannot compile an associative-array
//! `foreach` inside a function, so those cases are hand-IEEE (values checked by hand).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ffd_{}_{n}", std::process::id()));
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

/// A clean (non-fatal) run whose stdout contains `needle`.
fn ok_with(o: &str, needle: &str) -> bool {
    !o.contains("F4004")
        && !o.contains("ended (Error)")
        && !o.contains("E3009")
        && o.contains(needle)
}

// ── FUNCTION + dynamic-array foreach → supported (iverilog: 15) ──
#[test]
fn function_dyn_foreach_supported() {
    let o = run("module top;\n\
         int marr[];\n\
         function automatic int g();\n\
           int s=0; foreach(marr[i]) s+=marr[i]; return s;\n\
         endfunction\n\
         initial begin marr=new[3]; marr[0]=4;marr[1]=5;marr[2]=6; $display(\"g=%0d\", g()); end\n\
         endmodule\n");
    assert!(
        ok_with(&o, "g=15"),
        "dyn foreach in a function must sum to 15:\n{o}"
    );
}

// ── FUNCTION + queue foreach → supported (iverilog: 7) ──
#[test]
fn function_queue_foreach_supported() {
    let o = run("module top;\n\
         int mq[$];\n\
         function automatic int g(); int s=0; foreach(mq[i]) s+=mq[i]; return s; endfunction\n\
         initial begin mq.push_back(3); mq.push_back(4); $display(\"g=%0d\", g()); end\n\
         endmodule\n");
    assert!(
        ok_with(&o, "g=7"),
        "queue foreach in a function must sum to 7:\n{o}"
    );
}

// ── FUNCTION + signed byte dynamic foreach (-5+10-2 = 3) ──
#[test]
fn function_signed_byte_dyn_foreach() {
    let o = run("module top;\n\
         byte marr[];\n\
         function automatic int g(); int s=0; foreach(marr[i]) s+=marr[i]; return s; endfunction\n\
         initial begin marr=new[3]; marr[0]=-5;marr[1]=10;marr[2]=-2; $display(\"g=%0d\", g()); end\n\
         endmodule\n");
    assert!(
        ok_with(&o, "g=3"),
        "signed byte foreach in a function = 3:\n{o}"
    );
}

// ── FUNCTION + associative-array foreach (int key, hand-IEEE: 2*10+5*20 = 120) ──
#[test]
fn function_assoc_int_foreach() {
    let o = run("module top;\n\
         int m[int];\n\
         function automatic int g(); int s=0; foreach(m[k]) s+=k*m[k]; return s; endfunction\n\
         initial begin m[2]=10; m[5]=20; $display(\"g=%0d\", g()); end\n\
         endmodule\n");
    assert!(
        ok_with(&o, "g=120"),
        "assoc foreach in a function = 120:\n{o}"
    );
}

// ── FUNCTION + direct first/next while-loop (assoc int, hand-IEEE: 3+4+5 = 12) ──
#[test]
fn function_direct_first_next_loop() {
    let o = run("module top;\n\
         int m[int];\n\
         function automatic int g();\n\
           int k; int st; int s=0;\n\
           st = m.first(k);\n\
           while (st == 1) begin s += m[k]; st = m.next(k); end\n\
           return s;\n\
         endfunction\n\
         initial begin m[1]=3; m[2]=4; m[9]=5; $display(\"g=%0d\", g()); end\n\
         endmodule\n");
    assert!(
        ok_with(&o, "g=12"),
        "direct first/next loop in a function = 12:\n{o}"
    );
}

// ── FUNCTION + reverse last/prev iteration (assoc, hand-IEEE: 3*1+2*10+1*100 = 123) ──
#[test]
fn function_last_prev_loop() {
    let o = run("module top;\n\
         int m[int];\n\
         function automatic int g();\n\
           int k; int st; int s=0; int mul=1;\n\
           st = m.last(k);\n\
           while (st == 1) begin s += m[k]*mul; mul=mul*10; st = m.prev(k); end\n\
           return s;\n\
         endfunction\n\
         initial begin m[1]=1; m[2]=2; m[3]=3; $display(\"g=%0d\", g()); end\n\
         endmodule\n");
    assert!(
        ok_with(&o, "g=123"),
        "last/prev loop in a function = 123:\n{o}"
    );
}

// ── FUNCTION + foreach calling a nested function per element (2*(1+2+3) = 12) ──
#[test]
fn function_foreach_nested_call() {
    let o = run("module top;\n\
         int marr[];\n\
         function automatic int dbl(input int x); return x*2; endfunction\n\
         function automatic int g(); int s=0; foreach(marr[i]) s+=dbl(marr[i]); return s; endfunction\n\
         initial begin marr=new[3]; marr[0]=1;marr[1]=2;marr[2]=3; $display(\"g=%0d\", g()); end\n\
         endmodule\n");
    assert!(
        ok_with(&o, "g=12"),
        "foreach + nested call in a function = 12:\n{o}"
    );
}

// ── SUBSET (non-suspendable) task + dynamic foreach → supported (run_task path, r=15) ──
#[test]
fn subset_task_dyn_foreach_supported() {
    let o = run("module top;\n\
         int marr[];\n\
         task automatic t(output int s); s=0; foreach(marr[i]) s+=marr[i]; endtask\n\
         int r; initial begin marr=new[3]; marr[0]=4;marr[1]=5;marr[2]=6; t(r); $display(\"r=%0d\", r); end\n\
         endmodule\n");
    assert!(
        ok_with(&o, "r=15"),
        "dyn foreach in a subset task must sum to 15:\n{o}"
    );
}

// ── regression: a suspendable task ($display → process executor) still works ──
#[test]
fn suspendable_task_dyn_foreach_still_works() {
    let o = run("module top;\n\
         int marr[];\n\
         task automatic t(); int s=0; foreach(marr[i]) s+=marr[i]; $display(\"t=%0d\", s); endtask\n\
         initial begin marr=new[3]; marr[0]=4;marr[1]=5;marr[2]=6; t(); end\n\
         endmodule\n");
    assert!(
        ok_with(&o, "t=15"),
        "suspendable task foreach must still work (t=15):\n{o}"
    );
}

// ── regression: a module-process foreach is byte-identical (dyn 60) ──
#[test]
fn module_process_dyn_foreach_unchanged() {
    let o = run("module top;\n\
         byte marr[];\n\
         initial begin marr=new[3]; marr[0]=10;marr[1]=20;marr[2]=30;\n\
           begin int s=0; foreach(marr[i]) s+=marr[i]; $display(\"s=%0d\",s); end\n\
         end\n\
         endmodule\n");
    assert!(
        ok_with(&o, "s=60"),
        "module process foreach unchanged:\n{o}"
    );
}
