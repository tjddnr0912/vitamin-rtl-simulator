//! CONCURRENT activations of one task, each with its own frame-local dynamic array.
//!
//! This was a graceful F4004: a frame-local dyn array lives in a NET-keyed heap slot, and
//! the per-activation stash (take the outer contents at entry, put them back at `Return`)
//! is sound only while `[enter, exit]` intervals NEST. A `fork` makes them OVERLAP — B's
//! entry stashed A's live array and A resumed onto B's — so the entry guard fatal'd.
//!
//! Lifted by giving the dyn slots the AUTOMATIC window's lifetime: parked off the shared
//! heap while an activity is suspended, unparked when it resumes, at the same two points
//! (`stash_frame_windows` / `restore_frame_windows`). Only the TOP frame parks, and a fork
//! ARM parks nothing — an outer activation's values already live in the `dyn_stash` above
//! it, and an arm rides its parent's frame rather than owning it.
//!
//! ORACLE: iverilog 13.0, except where noted.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fdc_{}_{n}", std::process::id()));
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

fn has_all(o: &str, wants: &[&str]) {
    for w in wants {
        assert!(o.contains(w), "missing `{w}`:\n{o}");
    }
    assert!(!o.contains("F4004"), "unexpected fatal:\n{o}");
}

#[test]
fn two_concurrent_activations_keep_their_own_array() {
    let o = run("module t;\n\
        reg clk=0; always #5 clk=~clk;\n\
        task automatic tk(input int id);\n\
          int v[]; v = new[2]; v[0]=id*10; v[1]=id*10+1;\n\
          @(posedge clk);\n\
          $display(\"id=%0d v=%0d,%0d sz=%0d\", id, v[0], v[1], v.size());\n\
        endtask\n\
        initial begin fork tk(1); tk(2); join $finish; end\n\
        endmodule\n");
    has_all(&o, &["id=1 v=10,11 sz=2", "id=2 v=20,21 sz=2"]);
}

// ── the arrays must survive a WRITE after the resume, across two suspends ──
#[test]
fn concurrent_activations_write_after_resume() {
    let o = run("module t;\n\
        reg clk=0; always #5 clk=~clk;\n\
        task automatic tk(input int id);\n\
          int v[]; v = new[2]; v[0]=id;\n\
          @(posedge clk); v[1] = v[0] * 7; @(posedge clk);\n\
          $display(\"id=%0d v=%0d,%0d\", id, v[0], v[1]);\n\
        endtask\n\
        initial begin fork tk(1); tk(2); join $finish; end\n\
        endmodule\n");
    has_all(&o, &["id=1 v=1,7", "id=2 v=2,14"]);
}

// ── resume order REVERSED from entry order: the park is per-activation, not a stack ──
#[test]
fn concurrent_activations_resuming_out_of_order() {
    let o = run("module t;\n\
        task automatic tk(input int id, input int d);\n\
          int v[]; v = new[1]; v[0]=id*11;\n\
          #d; $display(\"id=%0d v=%0d t=%0t\", id, v[0], $time);\n\
        endtask\n\
        initial begin fork tk(1,30); tk(2,20); tk(3,10); join $finish; end\n\
        endmodule\n");
    has_all(&o, &["id=3 v=33 t=10", "id=2 v=22 t=20", "id=1 v=11 t=30"]);
}

// ── RECURSION inside concurrent activations: both mechanisms at once ──
#[test]
fn recursion_inside_concurrent_activations() {
    let o = run("module t;\n\
        reg clk=0; always #5 clk=~clk;\n\
        task automatic rec(input int id, input int n);\n\
          int v[]; v = new[1]; v[0]=id*100+n;\n\
          @(posedge clk);\n\
          if (n > 1) rec(id, n-1);\n\
          $display(\"id=%0d n=%0d v=%0d\", id, n, v[0]);\n\
        endtask\n\
        initial begin fork rec(1,2); rec(2,2); join $finish; end\n\
        endmodule\n");
    has_all(
        &o,
        &[
            "id=1 n=1 v=101",
            "id=1 n=2 v=102",
            "id=2 n=1 v=201",
            "id=2 n=2 v=202",
        ],
    );
}

// ── survivors of a `join_none` outlive the parent's fork statement ──
#[test]
fn concurrent_activations_under_join_none() {
    let o = run("module t;\n\
        reg clk=0; always #5 clk=~clk;\n\
        task automatic tk(input int id);\n\
          int v[]; v = new[1]; v[0]=id*3;\n\
          @(posedge clk); @(posedge clk);\n\
          $display(\"id=%0d v=%0d\", id, v[0]);\n\
        endtask\n\
        initial begin fork tk(1); tk(2); join_none #40 $finish; end\n\
        endmodule\n");
    has_all(&o, &["id=1 v=3", "id=2 v=6"]);
}

// ── a dyn INPUT formal is per-activation too, and the copies must not cross ──
#[test]
fn concurrent_activations_with_distinct_dyn_formals() {
    let o = run("module t;\n\
        reg clk=0; always #5 clk=~clk;\n\
        task automatic tk(input int id, input int b[]);\n\
          @(posedge clk); $display(\"id=%0d b0=%0d sz=%0d\", id, b[0], b.size());\n\
        endtask\n\
        int p[]; int q[];\n\
        initial begin p = new[1]; p[0]=11; q = new[2]; q[0]=22;\n\
          fork tk(1,p); tk(2,q); join $finish; end\n\
        endmodule\n");
    has_all(&o, &["id=1 b0=11 sz=1", "id=2 b0=22 sz=2"]);
}

// ── NO-REGRESSION: a fork INSIDE a framed task whose body holds a dyn local. The arms
// are Case A (they touch no parent frame-local), and the parent's array must survive the
// whole fork. §4.5.214.
#[test]
fn fork_in_frame_with_a_dyn_local_in_the_enclosing_task() {
    let o = run("module t;\n\
        reg clk=0; always #5 clk=~clk;\n\
        task automatic inner(input int id); @(posedge clk); $display(\"arm %0d\", id); endtask\n\
        task automatic outer(input int id);\n\
          int v[]; v = new[1]; v[0]=id*5;\n\
          fork inner(1); inner(2); join\n\
          $display(\"outer id=%0d v=%0d\", id, v[0]);\n\
        endtask\n\
        initial begin outer(9); $finish; end\n\
        endmodule\n");
    has_all(&o, &["arm 1", "arm 2", "outer id=9 v=45"]);
}

// ── REGRESSION (found by the PRE-vs-POST sweep, not by any probe written for this
// slice): an arm READING the parent's frame-local dyn array. The parent parks its arrays
// when it suspends — including on the fork barrier — and a parked array is absent from the
// heap rather than shared, so the arm read `a0=x`. `FrameRec::forked` exempts a frame that
// has spawned arms. iverilog: `ARM n=0 a0=0` … `END n=1 a0=1`.
#[test]
fn a_fork_arm_reads_the_enclosing_frames_dyn_local() {
    let o = run("module t;\n\
        reg clk=0; always #1 clk=~clk;\n\
        task automatic tk(input int n);\n\
          int a[]; a=new[2]; a[0]=n;\n\
          if (n>0) tk(n-1);\n\
          fork begin @(posedge clk); $display(\"ARM n=%0d a0=%0d\", n, a[0]); end join\n\
          $display(\"END n=%0d a0=%0d\", n, a[0]);\n\
        endtask\n\
        initial begin tk(1); $finish; end\n\
        initial #20 $finish;\n\
        endmodule\n");
    has_all(
        &o,
        &[
            "ARM n=0 a0=0",
            "ARM n=1 a0=1",
            "END n=0 a0=0",
            "END n=1 a0=1",
        ],
    );
    assert!(!o.contains("W4020"), "dyn read went out of range:\n{o}");
}

// ── the ARM-parks-nothing rule. The arm suspends DIRECTLY (not inside a called task), so
// its top frame IS the arm frame, whose callee is the enclosing task that owns the live
// array. Under `join_any` the parent resumes while the long arm is still suspended — if
// the arm had parked the parent's array, the parent would read an empty one here.
//
// ORACLE NOTE: hand-IEEE. iverilog 13.0 aborts on this shape (`of_JOIN_DETACH` assertion
// in vthread.cc), so it cannot answer.
#[test]
fn an_arm_that_suspends_does_not_carry_off_the_parents_array() {
    let o = run("module t;\n\
        reg clk=0; always #5 clk=~clk;\n\
        task automatic outer(input int id);\n\
          int v[]; v = new[2]; v[0]=id*5; v[1]=id;\n\
          fork\n\
            begin @(posedge clk); @(posedge clk); @(posedge clk); $display(\"armLONG\"); end\n\
            begin @(posedge clk); $display(\"armSHORT\"); end\n\
          join_any\n\
          $display(\"outer v=%0d,%0d sz=%0d\", v[0], v[1], v.size());\n\
          #40;\n\
        endtask\n\
        initial begin outer(9); $finish; end\n\
        endmodule\n");
    has_all(&o, &["armSHORT", "outer v=45,9 sz=2", "armLONG"]);
}
