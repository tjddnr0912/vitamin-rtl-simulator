//! V2A-frame (§4.5.173): an `input` DYNAMIC-array formal in an AUTOMATIC (framed,
//! suspendable) task. The formal is reserved as a per-activation `DynArray` heap
//! handle and the caller's array is DEEP-COPIED into it at frame entry (pass-by-VALUE,
//! IEEE §13.5.1) — so a concurrent mutation of the caller's array while the task is
//! suspended never shows through the formal. V5's net-range lifecycle (§4.5.171)
//! frees the slot at exit and fatal-louds on recursive/concurrent reentry.
//!
//! Supported: the SUSPENDABLE path (a task with `$display`/`@`/`#`/`wait`) AND, since
//! §4.5.194, the SUBSET (pure-compute) path — the interior-mutable dyn heap lets the
//! synchronous executor snapshot the caller's array too (see `subset_task_dyn_formal.rs`).
//! Recursion / concurrent activation stay loud (per-activation heap stash is a follow-on).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_v2af_{}_{n}", std::process::id()));
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

// ── basic: automatic task reads size + elements of an input dyn-array (iverilog: 60/3) ──
#[test]
fn auto_task_dyn_input_basic() {
    let o = run("module top;\n\
         task automatic consume(input byte b[]);\n\
           int s; s = 0;\n\
           for (int i = 0; i < b.size(); i++) s += b[i];\n\
           $display(\"sum=%0d size=%0d\", s, b.size());\n\
         endtask\n\
         byte arr[];\n\
         initial begin\n\
           arr = new[3]; arr[0]=10; arr[1]=20; arr[2]=30;\n\
           consume(arr);\n\
         end\n\
         endmodule\n");
    assert!(!o.contains("E3009"), "must not reject:\n{o}");
    assert!(o.contains("sum=60 size=3"), "iverilog: sum=60 size=3:\n{o}");
}

// ── signed byte elements: signed sum (-5+10-2 = 3) ──
#[test]
fn signed_byte_elements() {
    let o = run("module top;\n\
         task automatic sm(input byte b[]);\n\
           int s=0; foreach(b[i]) s+=b[i]; $display(\"s=%0d\",s);\n\
         endtask\n\
         byte a[]; initial begin a=new[3]; a[0]=-5; a[1]=10; a[2]=-2; sm(a); end\n\
         endmodule\n");
    assert!(o.contains("s=3"), "signed sum must be 3:\n{o}");
}

// ── 4-state element keeps X (an unwritten logic element reads X) ──
#[test]
fn fourstate_element_x() {
    let o = run("module top;\n\
         task automatic pr(input logic [7:0] b[]);\n\
           $display(\"b1=%b sz=%0d\", b[1], b.size());\n\
         endtask\n\
         logic [7:0] a[]; initial begin a=new[2]; a[0]=8'hAA; pr(a); end\n\
         endmodule\n");
    assert!(
        o.contains("b1=xxxxxxxx sz=2"),
        "unwritten 4-state elem = X:\n{o}"
    );
}

// ── two calls with different arrays are isolated (each snapshots its own actual) ──
#[test]
fn twice_call_isolated() {
    let o = run("module top;\n\
         task automatic pr(input int b[]);\n\
           $display(\"first=%0d sz=%0d\", b[0], b.size());\n\
         endtask\n\
         int a[]; initial begin\n\
           a=new[2]; a[0]=11; pr(a);\n\
           a=new[4]; a[0]=99; pr(a);\n\
         end\n\
         endmodule\n");
    assert!(
        o.contains("first=11 sz=2") && o.contains("first=99 sz=4"),
        "each call sees its own snapshot:\n{o}"
    );
}

// ── ★ soundness: a concurrent resize+mutate of the caller array while the task is
//    SUSPENDED must NOT show through the formal (pass-by-value snapshot, IEEE §13.5.1) ──
#[test]
fn snapshot_immune_across_suspend() {
    let o = run("module top;\n\
         int a[];\n\
         task automatic slow(input int b[]);\n\
           #10; $display(\"slow b0=%0d sz=%0d\", b[0], b.size());\n\
         endtask\n\
         initial begin a=new[2]; a[0]=100; slow(a); end\n\
         initial begin #5; a=new[3]; a[0]=777; a[1]=1; a[2]=2; end\n\
         endmodule\n");
    // If the formal aliased the caller net, it would read 777 / size 3.
    assert!(
        o.contains("slow b0=100 sz=2"),
        "snapshot must be immune to the concurrent resize (b0=100 sz=2):\n{o}"
    );
}

// ── re-forward: a task passes its dyn-array formal on to a nested task ──
#[test]
fn reforward_to_nested_task() {
    let o = run("module top;\n\
         task automatic inner(input int b[]);\n\
           $display(\"inner sum=%0d\", b[0]+b[1]);\n\
         endtask\n\
         task automatic outer(input int b[]);\n\
           inner(b); $display(\"outer sz=%0d\", b.size());\n\
         endtask\n\
         int a[]; initial begin a=new[2]; a[0]=3; a[1]=4; outer(a); end\n\
         endmodule\n");
    assert!(
        o.contains("inner sum=7") && o.contains("outer sz=2"),
        "re-forward to a nested task must work:\n{o}"
    );
}

// ── mixed formals: scalar input + dyn input + output (100 + 1+2+3 = 106) ──
#[test]
fn mixed_scalar_dyn_output_formals() {
    let o = run("module top;\n\
         task automatic mx(input int k, input int b[], output int s);\n\
           s = k; foreach(b[i]) s += b[i]; $display(\"s=%0d\",s);\n\
         endtask\n\
         int a[]; int r; initial begin\n\
           a=new[3]; a[0]=1; a[1]=2; a[2]=3; mx(100, a, r); $display(\"r=%0d\", r);\n\
         end\n\
         endmodule\n");
    assert!(
        o.contains("s=106") && o.contains("r=106"),
        "mixed formals: s=106 r=106:\n{o}"
    );
}

// ── empty array (new[0]) reads size 0 ──
#[test]
fn empty_array_size_zero() {
    let o = run("module top;\n\
         task automatic pr(input int b[]); $display(\"sz=%0d\", b.size()); endtask\n\
         int a[]; initial begin a=new[0]; pr(a); end\n\
         endmodule\n");
    assert!(o.contains("sz=0"), "empty dyn array size 0:\n{o}");
}

// ── T1-9: recursion with a dyn formal — per-ACTIVATION, no longer a fatal ──
#[test]
fn recursion_keeps_each_activation_s_formal() {
    // The formal's heap slot is keyed by NET, so every activation addressed the same one.
    // The entry now stashes the outer activation's contents and the exit restores them.
    // Here the recursive call passes the formal AS its own actual, which is why the
    // actual is CAPTURED before the stash takes the slot. iverilog: 42 at every level.
    let o = run("module top;\n\
         task automatic rec(input int b[], input int n);\n\
           $display(\"n=%0d b0=%0d sz=%0d\", n, b[0], b.size());\n\
           if (n>0) rec(b, n-1);\n\
         endtask\n\
         int a[]; initial begin a=new[2]; a[0]=42; rec(a,2); end\n\
         endmodule\n");
    for n in 0..=2 {
        assert!(
            o.contains(&format!("n={n} b0=42 sz=2")),
            "level {n} must see the full actual:\n{o}"
        );
    }
}

// ── LOUD: concurrent fork of the same dyn-formal task (two live activations) ──
#[test]
fn concurrent_fork_stays_loud() {
    let o = run("module top;\n\
         int a[];\n\
         task automatic slow(input int b[]); #10; $display(\"b0=%0d\", b[0]); endtask\n\
         initial begin a=new[2]; a[0]=5; fork slow(a); slow(a); join end\n\
         endmodule\n");
    assert!(
        o.contains("F4004") || o.contains("recursive or concurrent"),
        "concurrent dyn-formal activation must be fatal-loud:\n{o}"
    );
}

// ── SUPPORTED (§4.5.194): a pure-compute (non-suspendable, subset) dyn-formal task.
//    The dyn heap is interior-mutable now, so the engine snapshots the caller's array
//    into the formal's heap slot right before the synchronous `run_task_call` (was loud
//    "…subset…" because the `&self` executor could not populate the heap). iverilog: r=6.
#[test]
fn non_suspendable_subset_dyn_formal_now_supported() {
    let o = run("module top;\n\
         task automatic pc(input int b[], output int s);\n\
           s=0; foreach(b[i]) s+=b[i];\n\
         endtask\n\
         int a[]; int r; initial begin a=new[3]; a[0]=1;a[1]=2;a[2]=3; pc(a,r); $display(\"r=%0d\",r); end\n\
         endmodule\n");
    assert!(
        o.contains("r=6"),
        "subset dyn-formal task now supported:\n{o}"
    );
}

// ── LOUD: element-type sign mismatch (byte b[] ← byte unsigned a[]) ──
#[test]
fn sign_mismatch_stays_loud() {
    let o = run("module top;\n\
         task automatic sm(input byte b[]); $display(\"%0d\", b[0]); endtask\n\
         byte unsigned a[]; initial begin a=new[1]; a[0]=200; sm(a); end\n\
         endmodule\n");
    assert!(
        o.contains("E3009") && o.contains("matching dynamic-array actual"),
        "sign-mismatched actual must stay loud:\n{o}"
    );
}

// ── LOUD: a queue actual passed to a dyn-array formal (kind mismatch) ──
#[test]
fn queue_actual_stays_loud() {
    let o = run("module top;\n\
         task automatic pr(input int b[]); $display(\"%0d\", b[0]); endtask\n\
         int q[$]; initial begin q.push_back(9); pr(q); end\n\
         endmodule\n");
    assert!(
        o.contains("E3009") && o.contains("matching dynamic-array actual"),
        "a queue actual must stay loud (not a dynamic array):\n{o}"
    );
}
