//! Frame-local declaration-initializer fix (IEEE §13.4.4). A `int x = 10;` local
//! inside a function / task / class method must run its initializer at frame
//! entry — previously the initializer was silently dropped (local stayed X/0).
//! Surfaced by the parameterized-class adversarial hunt (a broad pre-existing
//! silent-wrong, reproducible with no class). Oracle: iverilog -g2012.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_fli_{}_{n}.sv", std::process::id()));
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
fn free_function_local_init() {
    let out = run("module t;\n\
           function int f(int x); int base = 10; return base + x; endfunction\n\
           initial $display(\"%0d\", f(3));\n\
         endmodule\n");
    assert_eq!(out, "13\n");
}

#[test]
fn class_method_local_init() {
    let out = run(
        "class C; function int f(); int base = 10; return base; endfunction endclass\n\
         module t; C c; initial begin c = new; $display(\"%0d\", c.f()); end endmodule\n",
    );
    assert_eq!(out, "10\n");
}

#[test]
fn local_init_runs_each_call() {
    // the initializer runs at every frame entry (vita locals reset per call).
    let out = run("module t;\n\
           function int f(); int acc = 100; acc = acc + 1; return acc; endfunction\n\
           initial begin $display(\"%0d\", f()); $display(\"%0d\", f()); end\n\
         endmodule\n");
    assert_eq!(out, "101\n101\n");
}
