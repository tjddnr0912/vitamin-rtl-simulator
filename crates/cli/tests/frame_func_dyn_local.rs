//! §4.5.194 V5: a FUNCTION body with a dynamic-array LOCAL and `new[n]` — `int loc[];
//! loc = new[n]; loc[i] = …; return …` — loud→supported. `new[]` lowers to a
//! `SysTaskId::DynNew` statement that the frame subset classifier rejected and the
//! synchronous `run_frame_call` executor skipped; now the interior-mutable `dyn_heap`
//! lets the `&self` executor allocate + element-write the heap. The function's own dyn
//! LOCAL is per-net heap, so a per-activation reentry guard fatal-louds on
//! recursive/concurrent use and a free-at-exit gives each call a fresh (empty) array.
//! iverilog 13.0 is a usable oracle (it runs function dyn locals) — values match it.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, bool) {
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
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (text, out.status.success())
}

#[test]
fn function_dyn_local_new_and_sum() {
    let o = run("module top;\n\
         function int sumf();\n\
           int loc[]; loc = new[3]; loc[0]=10; loc[1]=20; loc[2]=30;\n\
           sumf = loc[0]+loc[1]+loc[2];\n\
         endfunction\n\
         initial $display(\"sum=%0d\", sumf());\n\
         endmodule\n");
    assert!(o.0.contains("sum=60"), "got:\n{}", o.0);
}

#[test]
fn dynamic_index_write_read() {
    let o = run("module top;\n\
         function int f(input int n);\n\
           int loc[]; int s;\n\
           loc = new[n]; for(int i=0;i<n;i++) loc[i]=i*i;\n\
           s=0; for(int i=0;i<n;i++) s+=loc[i];\n\
           return s;\n\
         endfunction\n\
         initial $display(\"s=%0d\", f(4));\n\
         endmodule\n");
    assert!(o.0.contains("s=14"), "0+1+4+9=14:\n{}", o.0);
}

#[test]
fn two_calls_get_fresh_locals() {
    // free-at-exit: each call's dyn local starts empty and is sized fresh.
    let o = run("module top;\n\
         function int f(input int n);\n\
           int loc[]; loc=new[n]; loc[0]=100; return loc.size();\n\
         endfunction\n\
         initial $display(\"a=%0d b=%0d\", f(3), f(5));\n\
         endmodule\n");
    assert!(o.0.contains("a=3 b=5"), "fresh local per call:\n{}", o.0);
}

#[test]
fn new_with_copy_source() {
    let o = run("module top;\n\
         function int f();\n\
           int a[]; int b[];\n\
           a=new[3]; a[0]=7;a[1]=8;a[2]=9;\n\
           b=new[5](a);\n\
           return b[0]+b[1]+b[2]+b.size();\n\
         endfunction\n\
         initial $display(\"r=%0d\", f());\n\
         endmodule\n");
    assert!(o.0.contains("r=29"), "7+8+9+5=29:\n{}", o.0);
}

#[test]
fn delete_shrinks_to_empty() {
    let o = run("module top;\n\
         function int f();\n\
           int loc[]; loc=new[4]; loc.delete(); return loc.size();\n\
         endfunction\n\
         initial $display(\"r=%0d\", f());\n\
         endmodule\n");
    assert!(o.0.contains("r=0"), "delete → size 0:\n{}", o.0);
}

#[test]
fn nested_functions_each_with_dyn_local() {
    let o = run("module top;\n\
         function int inner(input int n); int loc[]; loc=new[n]; for(int i=0;i<n;i++) loc[i]=i+1; begin int s; s=0; for(int i=0;i<n;i++) s+=loc[i]; return s; end endfunction\n\
         function int outer(input int n); int tmp[]; tmp=new[2]; tmp[0]=inner(n); return tmp[0]*10; endfunction\n\
         initial $display(\"r=%0d\", outer(3));\n\
         endmodule\n");
    assert!(o.0.contains("r=60"), "inner sum 6, ×10 = 60:\n{}", o.0);
}

#[test]
fn subset_task_dyn_local() {
    let o = run("module top;\n\
         task automatic t(output int s);\n\
           int loc[]; loc=new[3]; loc[0]=1;loc[1]=2;loc[2]=3;\n\
           s = loc[0]+loc[1]+loc[2];\n\
         endtask\n\
         int r; initial begin t(r); $display(\"r=%0d\", r); end\n\
         endmodule\n");
    assert!(o.0.contains("r=6"), "task dyn local:\n{}", o.0);
}

#[test]
fn recursion_with_dyn_local_stays_loud() {
    // A per-net heap can't hold two live activations of the same dyn local → fatal-loud
    // (correct-or-loud; a per-activation heap stash is a follow-on, as §4.5.171).
    let o = run("module top;\n\
         function automatic int f(input int n);\n\
           int loc[]; loc=new[2]; loc[0]=n;\n\
           if (n<=0) return 0;\n\
           return loc[0] + f(n-1);\n\
         endfunction\n\
         initial $display(\"r=%0d\", f(3));\n\
         endmodule\n");
    assert!(
        !o.1 && o.0.contains("F4004"),
        "recursive dyn-local must be fatal-loud, not silent:\n{}",
        o.0
    );
}
