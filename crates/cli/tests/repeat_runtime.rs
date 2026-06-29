//! `repeat(N)` with a NON-constant (or large) count used to run the body ZERO
//! times (it warned and omitted the body). It now desugars to a signed
//! down-counter while-loop, running the body the runtime count of times (a
//! negative/zero count runs zero times, IEEE §12.7.3). Oracle: iverilog -g2012.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_rep_{}_{n}.sv", std::process::id()));
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
fn variable_count() {
    let out = run("module t;\n\
           integer n, cnt;\n\
           initial begin\n\
             n = 4; cnt = 0; repeat (n) cnt = cnt + 1; $display(\"%0d\", cnt);\n\
             n = 0; cnt = 0; repeat (n) cnt = cnt + 1; $display(\"%0d\", cnt);\n\
             n = -1; cnt = 0; repeat (n) cnt = cnt + 1; $display(\"%0d\", cnt);\n\
           end endmodule\n");
    assert_eq!(out, "4\n0\n0\n");
}

#[test]
fn narrow_variable_count() {
    let out = run("module t;\n\
           logic [3:0] rc; integer cnt;\n\
           initial begin rc = 3; cnt = 0; repeat (rc) cnt = cnt + 1; $display(\"%0d\", cnt); end\n\
         endmodule\n");
    assert_eq!(out, "3\n");
}

#[test]
fn count_evaluated_once() {
    // The count is captured once at entry; mutating the source inside the body
    // does not change the iteration count.
    let out = run("module t;\n\
           integer n, cnt;\n\
           initial begin\n\
             n = 3; cnt = 0;\n\
             repeat (n) begin cnt = cnt + 1; n = n + 10; end\n\
             $display(\"%0d\", cnt);\n\
           end endmodule\n");
    assert_eq!(out, "3\n");
}

#[test]
fn constant_count_still_unrolls() {
    let out = run("module t;\n\
           integer cnt;\n\
           initial begin cnt = 0; repeat (5) cnt = cnt + 1; $display(\"%0d\", cnt); end\n\
         endmodule\n");
    assert_eq!(out, "5\n");
}
