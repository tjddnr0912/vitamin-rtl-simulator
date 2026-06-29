//! N7-REST ⓑ-breadth: parameterized classes (IEEE 1800 §8.25).
//!
//! iverilog 13 does NOT support parameterized classes, so the oracle is hand-IEEE.
//! vita MONOMORPHIZES at parse time: `C #(16) h` substitutes the param value into a
//! concrete specialization; `C h` uses the parameter defaults. Pure parser (IR-0,
//! `.vu` AST repin only — no sim-ir/format change).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_full(src: &str) -> (String, String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_pclass_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("failed to run vita");
    let _ = std::fs::remove_file(&path);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

fn run(src: &str) -> String {
    let (out, err, ok) = run_full(src);
    assert!(ok, "vita must succeed; stderr:\n{err}");
    let mut s = String::new();
    for l in out.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    s
}

#[test]
fn default_param_value() {
    let out = run("class C #(int W = 8);\n\
           int v;\n\
           function new(); v = W; endfunction\n\
         endclass\n\
         module t; C c; initial begin c = new; $display(\"%0d\", c.v); end endmodule\n");
    assert_eq!(out, "8\n");
}

#[test]
fn override_param_value() {
    let out = run("class C #(int W = 8);\n\
           int v;\n\
           function new(); v = W; endfunction\n\
         endclass\n\
         module t; C #(16) c; initial begin c = new; $display(\"%0d\", c.v); end endmodule\n");
    assert_eq!(out, "16\n");
}

#[test]
fn param_drives_field_width() {
    // W governs the field width; W=4 → mask 4'hF wraps to 15.
    let out = run(
        "class Reg #(int W = 8);\n\
           logic [W-1:0] data;\n\
           function void setmax(); data = '1; endfunction\n\
         endclass\n\
         module t; Reg #(4) r; initial begin r = new; r.setmax(); $display(\"%0d\", r.data); end endmodule\n",
    );
    assert_eq!(out, "15\n");
}

#[test]
fn two_specializations_coexist() {
    let out = run("class C #(int W = 8);\n\
           int v;\n\
           function new(); v = W; endfunction\n\
         endclass\n\
         module t; C #(4) a; C #(20) b;\n\
         initial begin a = new; b = new; $display(\"%0d %0d\", a.v, b.v); end endmodule\n");
    assert_eq!(out, "4 20\n");
}

#[test]
fn param_in_method_expression() {
    let out = run("class Mul #(int K = 3);\n\
           function int f(int x); return x * K; endfunction\n\
         endclass\n\
         module t; Mul #(5) m; initial begin m = new; $display(\"%0d\", m.f(4)); end endmodule\n");
    assert_eq!(out, "20\n");
}

#[test]
fn multi_param() {
    let out = run(
        "class Pair #(int A = 1, int B = 2);\n\
           function int sum(); return A + B; endfunction\n\
         endclass\n\
         module t; Pair #(10, 20) p; initial begin p = new; $display(\"%0d\", p.sum()); end endmodule\n",
    );
    assert_eq!(out, "30\n");
}

// ── Adversarial-hunt regressions (silent-wrongs found 2026-06-24) ────────────

#[test]
fn param_in_method_local_width() {
    // HUNT #1: a method-local whose width uses the param must take the substituted
    // width (was collapsing to 1 bit → 255&1=1).
    let out = run("class C #(int W = 8);\n\
           function int f(); logic [W-1:0] t; t = 8'hFF; return t; endfunction\n\
         endclass\n\
         module t; C c; initial begin c = new; $display(\"%0d\", c.f()); end endmodule\n");
    assert_eq!(out, "255\n");
}

#[test]
fn method_arg_shadows_param() {
    // HUNT #2: a method formal named like the param must SHADOW it (was substituted).
    let out = run("class C #(int W = 8);\n\
           function int f(int W); return W; endfunction\n\
         endclass\n\
         module t; C c; initial begin c = new; $display(\"%0d\", c.f(3)); end endmodule\n");
    assert_eq!(out, "3\n");
}

#[test]
fn method_local_shadows_param() {
    // HUNT #2: a method-local named like the param shadows it.
    let out = run("class C #(int W = 8);\n\
           function int f(); int W; W = 5; return W; endfunction\n\
         endclass\n\
         module t; C c; initial begin c = new; $display(\"%0d\", c.f()); end endmodule\n");
    assert_eq!(out, "5\n");
}

#[test]
fn class_field_shadows_param() {
    // HUNT #2 (field collision): a member named like the param wins (not the value).
    let out = run("class C #(int W = 8);\n\
           int W;\n\
           function void s(); W = 5; endfunction\n\
           function int g(); return W; endfunction\n\
         endclass\n\
         module t; C c; initial begin c = new; c.s(); $display(\"%0d\", c.g()); end endmodule\n");
    assert_eq!(out, "5\n");
}

#[test]
fn non_literal_spec_arg_is_loud() {
    // v1: a non-integer-literal spec arg is a loud reject, never silent.
    let (_, err, ok) = run_full(
        "class C #(int W = 8); int v; function new(); v = W; endfunction endclass\n\
         module t; localparam P = 4; C #(P) c; initial begin c = new; $display(\"%0d\", c.v); end endmodule\n",
    );
    assert!(!ok, "non-literal spec arg must exit non-zero");
    assert!(
        err.contains("E2") || err.contains("E3"),
        "expected a loud E-code; got:\n{err}"
    );
}
