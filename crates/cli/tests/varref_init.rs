//! A VARIABLE declaration initializer that references another variable/net
//! (a NON-constant initializer) is a one-time value at time 0 (IEEE 1800 §6.8,
//! equivalent to `initial v = expr;`). Pre-fix it was silently dropped — the
//! target kept its uninitialized default (X for 4-state, 0 for 2-state).
//! `logic [7:0] b = a;` (a=5) printed `x` instead of `5`. Oracle: iverilog.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_vri_{}_{n}.sv", std::process::id()));
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
fn logic_init_from_var() {
    let out = run("module t;\n\
           logic [7:0] a = 8'd5;\n\
           logic [7:0] b = a;\n\
           initial $display(\"%0d\", b);\n\
         endmodule\n");
    assert_eq!(out, "5\n");
}

#[test]
fn int_init_from_expr() {
    let out = run("module t;\n\
           int x = 3;\n\
           int y = x + 1;\n\
           initial $display(\"%0d\", y);\n\
         endmodule\n");
    assert_eq!(out, "4\n");
}

#[test]
fn chained_var_init_declaration_order() {
    let out = run("module t;\n\
           logic [7:0] a = 8'd5;\n\
           logic [7:0] b = a + 8'd10;\n\
           logic [7:0] c = b * 2;\n\
           initial begin $display(\"%0d %0d\", b, c); end\n\
         endmodule\n");
    // b = 5+10 = 15; c = 15*2 = 30 (each reads the prior in declaration order).
    assert_eq!(out, "15 30\n");
}

#[test]
fn block_local_nonconst_init() {
    // A NON-constant initializer on a PROCESS block-local (`initial begin
    // logic x = g+1; …`) is applied at block entry, not silently dropped to X.
    let out = run("module t;\n\
           logic [7:0] g = 8'd42;\n\
           initial begin\n\
             logic [7:0] x = g + 1;\n\
             $display(\"x=%0d\", x);\n\
           end\n\
         endmodule\n");
    assert_eq!(out, "x=43\n");
}

#[test]
fn block_local_nonconst_init_always_reentry() {
    // The block-local init re-applies on each `always` re-entry (IEEE static
    // initialization at entry), so all three samples read the fresh value.
    let out = run("module t;\n\
           logic [7:0] g = 8'd10;\n\
           logic clk = 0;\n\
           always #5 clk = ~clk;\n\
           always @(posedge clk) begin\n\
             logic [7:0] x = g + 1;\n\
             $display(\"x=%0d\", x);\n\
           end\n\
           initial #22 $finish;\n\
         endmodule\n");
    // posedges at t=5 and t=15 (t=25 is past the #22 $finish) → two samples.
    assert_eq!(out, "x=11\nx=11\n");
}

#[test]
fn const_var_init_unaffected() {
    // CONTROL: a constant initializer still works via net.init (and a later
    // procedural write is not swallowed by a continuous driver).
    let out = run("module t;\n\
           int x = 7;\n\
           initial begin $display(\"%0d\", x); x = 9; $display(\"%0d\", x); end\n\
         endmodule\n");
    assert_eq!(out, "7\n9\n");
}
