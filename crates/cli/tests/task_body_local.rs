//! A STATIC (non-automatic) task with body-local declarations used to fail with
//! E3010 "undeclared net/variable" — the inline-task path never hoisted the
//! task's body-local decls, so an assigned local (`int base; base = 10;`) never
//! resolved. Automatic tasks (frame path) were fine. Fix: a static task that
//! declares body-locals is routed through the frame path too (which hoists +
//! initializes locals correctly). Oracle: iverilog -g2012.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_tbl_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut s = String::new();
    for l in so.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    s
}

#[test]
fn static_task_assigned_body_local() {
    let out = run("module t;\n\
           task doit(output int r);\n\
             int base;\n\
             base = 10;\n\
             base = base + 5;\n\
             r = base;\n\
           endtask\n\
           int o;\n\
           initial begin doit(o); $display(\"%0d\", o); end\n\
         endmodule\n");
    assert_eq!(out, "15\n");
}

#[test]
fn static_task_body_local_with_initializer() {
    let out = run("module t;\n\
           task doit(output int r);\n\
             int base = 10;\n\
             base = base + 5;\n\
             r = base;\n\
           endtask\n\
           int o;\n\
           initial begin doit(o); $display(\"%0d\", o); end\n\
         endmodule\n");
    assert_eq!(out, "15\n");
}

#[test]
fn static_task_two_call_sites_same_scope() {
    let out = run("module t;\n\
           task doit(input int a, output int r);\n\
             int base;\n\
             base = a * 2;\n\
             r = base;\n\
           endtask\n\
           int x, y;\n\
           initial begin doit(3, x); doit(7, y); $display(\"%0d %0d\", x, y); end\n\
         endmodule\n");
    assert_eq!(out, "6 14\n");
}

#[test]
fn static_task_local_used_in_loop() {
    let out = run("module t;\n\
           task sumto(input int n, output int r);\n\
             int acc;\n\
             int i;\n\
             acc = 0;\n\
             for (i = 1; i <= n; i = i + 1) acc = acc + i;\n\
             r = acc;\n\
           endtask\n\
           int o;\n\
           initial begin sumto(5, o); $display(\"%0d\", o); end\n\
         endmodule\n");
    assert_eq!(out, "15\n");
}

#[test]
fn static_task_block_local() {
    // A local declared inside a `begin … end` (block-local), plus an inout — both
    // previously failed (block-locals were never reserved in the frame path).
    let out = run("module t;\n\
           task incr(inout int v);\n\
             int step;\n\
             step = 2;\n\
             v = v + step;\n\
           endtask\n\
           task blockl(input int a, output int r);\n\
             begin\n\
               int tmp;\n\
               tmp = a + 100;\n\
               r = tmp;\n\
             end\n\
           endtask\n\
           int x, y;\n\
           initial begin x = 5; incr(x); blockl(7, y); $display(\"%0d %0d\", x, y); end\n\
         endmodule\n");
    assert_eq!(out, "7 107\n");
}

#[test]
fn frame_function_block_local() {
    // A block-local in a frame FUNCTION body (the same pre-existing gap as tasks).
    let out = run("module t;\n\
           function automatic int f(input int a);\n\
             begin\n\
               int tmp;\n\
               tmp = a + 100;\n\
               return tmp;\n\
             end\n\
           endfunction\n\
           initial $display(\"%0d\", f(7));\n\
         endmodule\n");
    assert_eq!(out, "107\n");
}

#[test]
fn unpacked_array_task_local_works() {
    // An unpacked-array body-local in a (non-automatic) task now gets real element
    // storage (was loud "scalar/packed locals only"). arr[0]+arr[1] = 5+9 = 14.
    // Oracle: iverilog. (Broader coverage in tests/task_unpacked_local.rs.)
    let out = run("module t;\n\
           task tk; int arr [0:1]; begin arr[0]=5; arr[1]=9; $display(\"%0d\", arr[0]+arr[1]); end endtask\n\
           initial tk();\n\
         endmodule\n");
    assert_eq!(out, "14\n");
}

#[test]
fn static_task_persistent_local() {
    // IEEE §6.21 / §13.4.1: a non-automatic (static) task's local has STATIC
    // storage; its `= 0` initializer runs ONCE before time 0 and the value is
    // retained across calls. Three calls must print 1,2,3 — not 1,1,1 (the
    // pre-fix per-call-fresh-storage divergence). Oracle: iverilog.
    let out = run("module t;\n\
           task tk; int cnt = 0; cnt = cnt + 1; $display(\"%0d\", cnt); endtask\n\
           initial begin tk(); tk(); tk(); end\n\
         endmodule\n");
    assert_eq!(out, "1\n2\n3\n");
}

#[test]
fn static_task_persistent_no_initializer() {
    // A static local without an initializer still has persistent storage: the
    // accumulator survives across calls.
    let out = run("module t;\n\
           int o;\n\
           task tk(output int r); int acc; acc = acc + 5; r = acc; endtask\n\
           initial begin tk(o); $display(\"%0d\", o); tk(o); $display(\"%0d\", o); end\n\
         endmodule\n");
    // First call: acc starts 0 (2-state default) -> 5. Second: 5 -> 10.
    assert_eq!(out, "5\n10\n");
}

#[test]
fn static_task_no_locals_still_inlines() {
    // CONTROL: a static task with no body-locals keeps inlining (byte-identical
    // path); just confirm it still computes correctly.
    let out = run("module t;\n\
           task addone(input int a, output int r);\n\
             r = a + 1;\n\
           endtask\n\
           int o;\n\
           initial begin addone(41, o); $display(\"%0d\", o); end\n\
         endmodule\n");
    assert_eq!(out, "42\n");
}
