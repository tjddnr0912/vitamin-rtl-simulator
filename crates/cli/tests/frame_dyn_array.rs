//! V5 (§4.5.171): a frame-local (task-body-local) single-dim DYNAMIC array
//! (`int loc[]; loc = new[n]; loc[i]; loc.size()`) of a simple bit-vector element in
//! an automatic/suspendable TASK gets a real DynArray heap handle. Its heap object
//! lives at `dyn_heap[net]` (the current activation's array); the engine frees it at
//! frame exit and FATAL-louds on recursive / concurrent reentry (correct-or-loud — a
//! per-activation heap stash is a follow-on). Goldens are iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// (combined stdout+stderr trimmed, success) for a one-shot vita run of `src`.
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_v5_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&out.stderr));
    (combined.trim().to_owned(), out.status.success())
}

// ───────────────────────── supported (task, single activation) ─────────────────────────
#[test]
fn task_dyn_local_basic() {
    // new[3] + element writes + reads + .size(). 3*100 + 5 + 6 = 311.
    let src = "module t;\n\
        task automatic mk(input int n, output int r);\n\
          int loc[]; loc = new[n]; loc[0]=5; loc[1]=6;\n\
          r = loc.size()*100 + loc[0] + loc[1]; endtask\n\
        int x; initial begin mk(3,x); $display(\"r=%0d\", x); end endmodule";
    let (out, ok) = run(src);
    assert!(ok, "must run, got:\n{out}");
    assert!(out.contains("r=311"), "got:\n{out}");
}

#[test]
fn task_dyn_dynamic_index() {
    // variable-index read AND write in loops: sum of i*i for i in 0..3 = 0+1+4+9 = 14.
    let src = "module t;\n\
        task automatic mk(input int n, output int r);\n\
          int loc[]; int i; loc = new[n];\n\
          for(i=0;i<n;i=i+1) loc[i]=i*i;\n\
          r=0; for(i=0;i<n;i=i+1) r=r+loc[i]; endtask\n\
        int x; initial begin mk(4,x); $display(\"r=%0d\", x); end endmodule";
    let (out, ok) = run(src);
    assert!(ok && out.contains("r=14"), "got:\n{out}");
}

#[test]
fn task_dyn_signed_byte_element() {
    // a signed `byte` element reads negative (no unsigned collapse): -5 + 100 = 95.
    let src = "module t;\n\
        task automatic mk(output int r);\n\
          byte loc[]; loc = new[2]; loc[0]=-5; loc[1]=100; r = loc[0]+loc[1]; endtask\n\
        int x; initial begin mk(x); $display(\"r=%0d\", x); end endmodule";
    let (out, ok) = run(src);
    assert!(ok && out.contains("r=95"), "got:\n{out}");
}

#[test]
fn task_dyn_resize() {
    // `loc = new[m]` after `new[n]` REPLACES the array (reuses the net's heap slot).
    // size 4 * 10 + loc[3](9) = 49.
    let src = "module t;\n\
        task automatic mk(output int r);\n\
          int loc[]; loc=new[2]; loc[0]=1; loc[1]=2; loc=new[4]; loc[3]=9;\n\
          r = loc.size()*10 + loc[3]; endtask\n\
        int x; initial begin mk(x); $display(\"r=%0d\", x); end endmodule";
    let (out, ok) = run(src);
    assert!(ok && out.contains("r=49"), "got:\n{out}");
}

#[test]
fn task_dyn_delete() {
    // `loc.delete()` empties the array: size 0.
    let src = "module t;\n\
        task automatic mk(output int r);\n\
          int loc[]; loc=new[3]; loc.delete(); r = loc.size(); endtask\n\
        int x; initial begin mk(x); $display(\"r=%0d\", x); end endmodule";
    let (out, ok) = run(src);
    assert!(ok && out.contains("r=0"), "got:\n{out}");
}

#[test]
fn task_dyn_sequential_fresh() {
    // Each call starts fresh (free-at-exit): mk(3)->330, mk(2)->220. The two calls do
    // NOT share the previous array.
    let src = "module t;\n\
        task automatic mk(input int n, output int r);\n\
          int loc[]; loc=new[n]; loc[0]=n*10; r = loc.size()*100 + loc[0]; endtask\n\
        int x,y; initial begin mk(3,x); mk(2,y); $display(\"x=%0d y=%0d\", x, y); end endmodule";
    let (out, ok) = run(src);
    assert!(ok && out.contains("x=330 y=220"), "got:\n{out}");
}

#[test]
fn task_dyn_size_before_new_is_zero() {
    // Reading `.size()` BEFORE `new[]` must be 0 (a fresh activation, not a prior call's
    // array). Guards the free-at-exit reset.
    let src = "module t;\n\
        task automatic mk(input int n, output int r);\n\
          int loc[]; r = loc.size(); loc = new[n]; endtask\n\
        int a,b; initial begin int z; mk(9,z); mk(2,a); mk(5,b);\n\
          $display(\"a=%0d b=%0d\", a, b); end endmodule";
    let (out, ok) = run(src);
    assert!(ok && out.contains("a=0 b=0"), "got:\n{out}");
}

#[test]
fn task_dyn_across_suspend() {
    // A SINGLE suspendable task: the array survives a `#delay` (same activation).
    // 42 + 43 + 3 = 88.
    let src = "module t;\n\
        task automatic mk(input int n, output int r);\n\
          int loc[]; loc=new[n]; loc[0]=42; #5; loc[1]=43;\n\
          r = loc[0]+loc[1]+loc.size(); endtask\n\
        int x; initial begin mk(3,x); $display(\"r=%0d\", x); end endmodule";
    let (out, ok) = run(src);
    assert!(ok && out.contains("r=88"), "got:\n{out}");
}

#[test]
fn task_dyn_module_coexist_no_alias() {
    // A MODULE-scope dyn-array `g[]` and a frame-local `loc[]` do not alias.
    // loc: 7+8=15, + g[0]=100 => 115; g unchanged.
    let src = "module t;\n\
        int g[];\n\
        task automatic mk(output int r);\n\
          int loc[]; loc=new[2]; loc[0]=7; loc[1]=8; r = loc[0]+loc[1]+g[0]; endtask\n\
        int x; initial begin g=new[1]; g[0]=100; mk(x); $display(\"r=%0d g0=%0d\", x, g[0]); end endmodule";
    let (out, ok) = run(src);
    assert!(ok && out.contains("r=115 g0=100"), "got:\n{out}");
}

#[test]
fn task_dyn_nested_different_tasks() {
    // A non-recursive `outer` (dyn-array `b`) calls `inner` (its OWN dyn-array `a`) —
    // different nets, no reentry-guard false-trigger. 10 + 3 + (5+6) = 24.
    let src = "module t;\n\
        task automatic inner(output int r);\n\
          int a[]; a=new[2]; a[0]=5; a[1]=6; r=a[0]+a[1]; endtask\n\
        task automatic outer(output int r);\n\
          int b[]; int ir; b=new[3]; b[0]=10; inner(ir); r = b[0]+b.size()+ir; endtask\n\
        int x; initial begin outer(x); $display(\"r=%0d\", x); end endmodule";
    let (out, ok) = run(src);
    assert!(ok && out.contains("r=24"), "got:\n{out}");
}

// ───────────────────────── correct-or-loud boundaries ─────────────────────────
#[test]
fn task_dyn_recursion_is_per_activation() {
    // T1-9. The heap object is keyed by NET, so every activation of `mk` addresses the
    // same slot — which is why this used to be a fatal. It is now per-ACTIVATION: the
    // entry TAKES the outer contents into a stash that travels with the activation and
    // the exit puts them back, so the inner call starts from an unallocated array and the
    // outer one gets its own back. iverilog: r=6 (3 + 2 + 1). The old fatal's silent-wrong
    // answer was r=3, so a collapsed recursion is still caught by the value.
    let src = "module t;\n\
        task automatic mk(input int n, output int r);\n\
          int loc[]; loc=new[n]; loc[0]=n;\n\
          if(n<=1) r=loc.size(); else begin int s; mk(n-1,s); r=loc.size()+s; end endtask\n\
        int x; initial begin mk(3,x); $display(\"r=%0d\", x); end endmodule";
    let (out, ok) = run(src);
    assert!(ok, "recursive frame dyn-array must run, got:\n{out}");
    assert!(out.contains("r=6"), "got:\n{out}");
}

#[test]
fn suspending_task_dyn_recursion_is_per_activation() {
    // T1-9, the SUSPENDABLE path: the stash rides the activity's own `FrameRec`, so it
    // survives the `@` and is restored at that frame's own Return. iverilog prints the
    // three levels innermost-first with each level's own array intact.
    let src = "module t; reg clk=0; always #1 clk=~clk;\n\
        task automatic tk(input int n); int a[]; a=new[2]; a[0]=n; a[1]=n*10;\n\
          @(posedge clk); if(n>0) tk(n-1);\n\
          $display(\"n=%0d a=[%0d,%0d]\", n, a[0], a[1]); endtask\n\
        initial begin tk(2); $finish; end endmodule";
    let (out, ok) = run(src);
    assert!(ok, "got:\n{out}");
    assert!(
        out.contains("n=0 a=[0,0]") && out.contains("n=1 a=[1,10]") && out.contains("n=2 a=[2,20]"),
        "each activation must keep its own array, got:\n{out}"
    );
}

#[test]
fn task_dyn_concurrent_activations_each_keep_their_own() {
    // This was the T1-9 boundary and a graceful F4004: two fork activations both
    // allocate and then SUSPEND, so their `[enter, exit]` intervals OVERLAP rather than
    // nest, and the entry stash alone would hand A back B's array.
    //
    // Lifted by giving the dyn slots the AUTOMATIC window's lifetime — parked off the
    // net-keyed heap while the activity is suspended, unparked when it resumes, at the
    // same two points (`stash_frame_windows`/`restore_frame_windows`). Only the TOP frame
    // parks: outer activations already live in the `dyn_stash` above them, which is what
    // keeps recursion (the test above) working. iverilog: `x=3 y=2`.
    let src = "module t;\n\
        task automatic mk(input int n, output int r);\n\
          int loc[]; loc=new[n]; #5; loc[0]=n; r=loc.size(); endtask\n\
        int x,y; initial begin fork mk(3,x); mk(2,y); join $display(\"x=%0d y=%0d\",x,y); end endmodule";
    let (out, ok) = run(src);
    assert!(ok, "got:\n{out}");
    assert!(out.contains("x=3 y=2"), "got:\n{out}");
}

#[test]
fn function_dyn_local_now_supported() {
    // §4.5.194 V5: a FUNCTION with a dyn-array local + `new[]` is supported — the
    // interior-mutable dyn heap lets the `&self` executor allocate + element-write.
    // loc.size()=3 + loc[0]=5 = 8. iverilog: 8.
    let src = "module t;\n\
        function automatic int mk(input int n);\n\
          int loc[]; loc = new[n]; loc[0]=5; return loc.size()+loc[0]; endfunction\n\
        initial $display(\"r=%0d\", mk(3)); endmodule";
    let (out, ok) = run(src);
    assert!(
        ok && out.contains("r=8"),
        "function dyn-local now supported (r=8):\n{out}"
    );
}
