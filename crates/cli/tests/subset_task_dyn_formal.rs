//! §4.5.194 V2A-dyn: a SUBSET (non-suspendable) task with a dynamic-array `input`
//! formal (`task automatic t(input byte b[], output int s); ... s += b[i];`) — a
//! pure-compute task with no observable/timing statement — is now supported. Before,
//! it was loud (E3009 "…subset (non-suspendable) task…") because the synchronous
//! `run_task_call` executor is `&self` and could not populate the heap; now the dyn
//! heap is interior-mutable (`RefCell`), so the caller's array is deep-copied
//! (pass-by-value, IEEE §13.5.1) into the formal's per-activation heap slot right
//! before the synchronous call — at BOTH the process-driven call site (exec.rs) and a
//! NESTED subset call driven inside `run_task` — and freed after.
//!
//! iverilog 13.0 IS a usable oracle here (it runs a dyn input array formal on a task):
//! every value below is the iverilog result.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_stdf_{}_{n}", std::process::id()));
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
fn subset_task_reads_dyn_input_formal() {
    // The core V2A-dyn case: a pure-compute subset task summing a dyn input array.
    let o = run(
        "module top;\n\
         task automatic sum_it(input byte b[], output int s);\n\
           s = 0; for (int i=0;i<b.size();i++) s += b[i];\n\
         endtask\n\
         byte a[]; int r;\n\
         initial begin a=new[3]; a[0]=10; a[1]=20; a[2]=30; sum_it(a, r); $display(\"r=%0d\", r); end\n\
         endmodule\n",
    );
    assert!(o.contains("r=60"), "got:\n{o}");
}

#[test]
fn two_calls_are_pass_by_value_isolated() {
    // Each call snapshots its own actual — distinct arrays give distinct results.
    let o = run(
        "module top;\n\
         task automatic sm(input int b[], output int s); s=0; for(int i=0;i<b.size();i++) s+=b[i]; endtask\n\
         int x[]; int y[]; int r1; int r2;\n\
         initial begin\n\
           x=new[2]; x[0]=1; x[1]=2; y=new[3]; y[0]=10; y[1]=20; y[2]=30;\n\
           sm(x,r1); sm(y,r2); $display(\"r1=%0d r2=%0d\", r1, r2);\n\
         end\nendmodule\n",
    );
    assert!(o.contains("r1=3 r2=60"), "got:\n{o}");
}

#[test]
fn signed_byte_elements_sum() {
    let o = run(
        "module top;\n\
         task automatic sm(input byte b[], output int s); s=0; for(int i=0;i<b.size();i++) s+=b[i]; endtask\n\
         byte a[]; int r;\n\
         initial begin a=new[3]; a[0]=-5; a[1]=100; a[2]=-1; sm(a,r); $display(\"r=%0d\", r); end\n\
         endmodule\n",
    );
    assert!(o.contains("r=94"), "got:\n{o}");
}

#[test]
fn nested_subset_call_forwards_dyn_formal() {
    // A subset task passing its OWN dyn formal to a nested subset task — the nested
    // call is driven inside `run_task` (not exec.rs), so it needs its own snapshot.
    let o = run(
        "module top;\n\
         task automatic inner(input int c[], output int s); s=0; for(int i=0;i<c.size();i++) s+=c[i]; endtask\n\
         task automatic outer(input int b[], output int t); int u; inner(b,u); t=u*2; endtask\n\
         int a[]; int r;\n\
         initial begin a=new[3]; a[0]=1; a[1]=2; a[2]=3; outer(a,r); $display(\"r=%0d\", r); end\n\
         endmodule\n",
    );
    assert!(o.contains("r=12"), "got:\n{o}");
}

#[test]
fn element_and_size_reads() {
    let o = run(
        "module top;\n\
         task automatic pk(input int b[], output int a0, output int aN, output int sz);\n\
           sz=b.size(); a0=b[0]; aN=b[b.size()-1];\n\
         endtask\n\
         int d[]; int x0; int xn; int n;\n\
         initial begin d=new[4]; d[0]=7; d[1]=8; d[2]=9; d[3]=42; pk(d,x0,xn,n); $display(\"%0d %0d %0d\", x0, xn, n); end\n\
         endmodule\n",
    );
    assert!(o.contains("7 42 4"), "got:\n{o}");
}

#[test]
fn writing_a_dyn_input_formal_stays_loud() {
    // Correct-or-loud: writing the pass-by-value copy is still a heap write the `&self`
    // synchronous executor cannot do — a loud F4004, never a silent mis-run.
    let o = run("module top;\n\
         task automatic wr(input int b[], output int s); b[0]=999; s=b[0]; endtask\n\
         int a[]; int r;\n\
         initial begin a=new[2]; a[0]=1; a[1]=2; wr(a,r); $display(\"r=%0d\", r); end\n\
         endmodule\n");
    assert!(o.contains("F4004"), "should be loud:\n{o}");
}

#[test]
fn suspendable_dyn_formal_task_still_works() {
    // Regression: a task WITH an observable statement is lifted to the suspendable
    // path (§4.5.173) — unchanged by this slice.
    let o = run(
        "module top;\n\
         task automatic sh(input byte b[]); $display(\"sz=%0d\", b.size()); for(int i=0;i<b.size();i++) $display(\"v%0d=%0d\",i,b[i]); endtask\n\
         byte a[];\n\
         initial begin a=new[2]; a[0]=7; a[1]=8; sh(a); end\n\
         endmodule\n",
    );
    assert!(
        o.contains("sz=2") && o.contains("v0=7") && o.contains("v1=8"),
        "got:\n{o}"
    );
}
