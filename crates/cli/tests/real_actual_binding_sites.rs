//! A real actual binds to a narrower integral formal by the assignment conversion (§6.12.2,
//! then the formal's width) at every binding site — ROADMAP §2 "Inline / frame binds" recorded
//! five open sites (`f(300.0)` into an `input byte` giving 300 where the oracles give 44): a frame
//! function with an output formal, a hierarchical task call, a hierarchical function call, a class
//! method or task, a class constructor. §4.5.551 re-measured them stale: 180 cells (six formal
//! types × the five sites and a class task × five actual shapes) match both oracles wherever both
//! run (140) and the one that runs (30); the other ten are an honest refusal — a hierarchical
//! function call to a callee the inline lane lowered has no frame to call (ROADMAP §3.b
//! `hier-fn-inline-callee`), and a `logic` formal or return makes it so whatever the actual is.
//!
//! These pins keep the sites there. Each asserts the native lines (iverilog 13.0 and verilator
//! 5.052 print them where they run) and that the interpreter and the VM print the same.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_any(src: &str, backend: &str) -> (bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_rbs_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["--backend", backend])
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let s =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    (out.status.success(), s)
}

fn run(src: &str, backend: &str) -> Vec<String> {
    let (ok, s) = run_any(src, backend);
    assert!(ok, "expected exit 0 on {backend}, got:\n{s}");
    s.lines()
        .filter(|l| l.starts_with("T "))
        .map(|l| l.trim().to_string())
        .collect()
}

fn check(src: &str, want: &[&str]) {
    let native = run(src, "native");
    assert_eq!(native, want, "native");
    assert_eq!(run(src, "interp"), native, "interp");
    assert_eq!(run(src, "vm"), native, "vm");
}

/// a frame function with an OUTPUT formal: a `byte`, a `longint` and a `bit [7:0]` formal, actuals `300.0`, `-2.5`, `rv * 1.0`, `1e20`, `rv` (`rv = 70000.4`).
#[test]
fn at_a_frame_function_with_an_output_formal() {
    check(
        r#"
module t;
  real rv = 70000.4;
  function automatic longint f(input byte x, output longint o); o = x; f = x; endfunction
  longint oo;
  initial begin
    $display("T a0 %h", f(300.0, oo));
    $display("T a1 %h", f(-2.5, oo));
    $display("T a2 %h", f(rv * 1.0, oo));
    $display("T a3 %h", f(1e20, oo));
    $display("T a4 %h", f(rv, oo));
    #1 $finish;
  end
endmodule
"#,
        &[
            r#"T a0 000000000000002c"#,
            r#"T a1 fffffffffffffffd"#,
            r#"T a2 0000000000000070"#,
            r#"T a3 0000000000000000"#,
            r#"T a4 0000000000000070"#,
        ],
    );
    check(
        r#"
module t;
  real rv = 70000.4;
  function automatic longint f(input longint x, output longint o); o = x; f = x; endfunction
  longint oo;
  initial begin
    $display("T a0 %h", f(300.0, oo));
    $display("T a1 %h", f(-2.5, oo));
    $display("T a2 %h", f(rv * 1.0, oo));
    $display("T a3 %h", f(1e20, oo));
    $display("T a4 %h", f(rv, oo));
    #1 $finish;
  end
endmodule
"#,
        &[
            r#"T a0 000000000000012c"#,
            r#"T a1 fffffffffffffffd"#,
            r#"T a2 0000000000011170"#,
            r#"T a3 6bc75e2d63100000"#,
            r#"T a4 0000000000011170"#,
        ],
    );
    check(
        r#"
module t;
  real rv = 70000.4;
  function automatic longint f(input bit [7:0] x, output longint o); o = x; f = x; endfunction
  longint oo;
  initial begin
    $display("T a0 %h", f(300.0, oo));
    $display("T a1 %h", f(-2.5, oo));
    $display("T a2 %h", f(rv * 1.0, oo));
    $display("T a3 %h", f(1e20, oo));
    $display("T a4 %h", f(rv, oo));
    #1 $finish;
  end
endmodule
"#,
        &[
            r#"T a0 000000000000002c"#,
            r#"T a1 00000000000000fd"#,
            r#"T a2 0000000000000070"#,
            r#"T a3 0000000000000000"#,
            r#"T a4 0000000000000070"#,
        ],
    );
}

/// a hierarchical task call: a `byte`, a `longint` and a `bit [7:0]` formal, actuals `300.0`, `-2.5`, `rv * 1.0`, `1e20`, `rv` (`rv = 70000.4`).
#[test]
fn at_b_hierarchical_task_call() {
    check(
        r#"module sub; task tk(input byte x, input int i); $display("T b%0d %h", i, x); endtask endmodule
module t;
  real rv = 70000.4;
  
  sub u();
  initial begin
    u.tk(300.0, 0);
    u.tk(-2.5, 1);
    u.tk(rv * 1.0, 2);
    u.tk(1e20, 3);
    u.tk(rv, 4);
    #1 $finish;
  end
endmodule
"#,
        &[
            r#"T b0 2c"#,
            r#"T b1 fd"#,
            r#"T b2 70"#,
            r#"T b3 00"#,
            r#"T b4 70"#,
        ],
    );
    check(
        r#"module sub; task tk(input longint x, input int i); $display("T b%0d %h", i, x); endtask endmodule
module t;
  real rv = 70000.4;
  
  sub u();
  initial begin
    u.tk(300.0, 0);
    u.tk(-2.5, 1);
    u.tk(rv * 1.0, 2);
    u.tk(1e20, 3);
    u.tk(rv, 4);
    #1 $finish;
  end
endmodule
"#,
        &[
            r#"T b0 000000000000012c"#,
            r#"T b1 fffffffffffffffd"#,
            r#"T b2 0000000000011170"#,
            r#"T b3 6bc75e2d63100000"#,
            r#"T b4 0000000000011170"#,
        ],
    );
    check(
        r#"module sub; task tk(input bit [7:0] x, input int i); $display("T b%0d %h", i, x); endtask endmodule
module t;
  real rv = 70000.4;
  
  sub u();
  initial begin
    u.tk(300.0, 0);
    u.tk(-2.5, 1);
    u.tk(rv * 1.0, 2);
    u.tk(1e20, 3);
    u.tk(rv, 4);
    #1 $finish;
  end
endmodule
"#,
        &[
            r#"T b0 2c"#,
            r#"T b1 fd"#,
            r#"T b2 70"#,
            r#"T b3 00"#,
            r#"T b4 70"#,
        ],
    );
}

/// a hierarchical function call: a `byte`, a `longint` and a `bit [7:0]` formal, actuals `300.0`, `-2.5`, `rv * 1.0`, `1e20`, `rv` (`rv = 70000.4`).
#[test]
fn at_c_hierarchical_function_call() {
    check(
        r#"module sub; function byte fn(input byte x); fn = x; endfunction endmodule
module t;
  real rv = 70000.4;
  
  sub u();
  initial begin
    $display("T c0 %h", u.fn(300.0));
    $display("T c1 %h", u.fn(-2.5));
    $display("T c2 %h", u.fn(rv * 1.0));
    $display("T c3 %h", u.fn(1e20));
    $display("T c4 %h", u.fn(rv));
    #1 $finish;
  end
endmodule
"#,
        &[
            r#"T c0 2c"#,
            r#"T c1 fd"#,
            r#"T c2 70"#,
            r#"T c3 00"#,
            r#"T c4 70"#,
        ],
    );
    check(
        r#"module sub; function longint fn(input longint x); fn = x; endfunction endmodule
module t;
  real rv = 70000.4;
  
  sub u();
  initial begin
    $display("T c0 %h", u.fn(300.0));
    $display("T c1 %h", u.fn(-2.5));
    $display("T c2 %h", u.fn(rv * 1.0));
    $display("T c3 %h", u.fn(1e20));
    $display("T c4 %h", u.fn(rv));
    #1 $finish;
  end
endmodule
"#,
        &[
            r#"T c0 000000000000012c"#,
            r#"T c1 fffffffffffffffd"#,
            r#"T c2 0000000000011170"#,
            r#"T c3 6bc75e2d63100000"#,
            r#"T c4 0000000000011170"#,
        ],
    );
    check(
        r#"module sub; function bit [7:0] fn(input bit [7:0] x); fn = x; endfunction endmodule
module t;
  real rv = 70000.4;
  
  sub u();
  initial begin
    $display("T c0 %h", u.fn(300.0));
    $display("T c1 %h", u.fn(-2.5));
    $display("T c2 %h", u.fn(rv * 1.0));
    $display("T c3 %h", u.fn(1e20));
    $display("T c4 %h", u.fn(rv));
    #1 $finish;
  end
endmodule
"#,
        &[
            r#"T c0 2c"#,
            r#"T c1 fd"#,
            r#"T c2 70"#,
            r#"T c3 00"#,
            r#"T c4 70"#,
        ],
    );
}

/// a class method: a `byte`, a `longint` and a `bit [7:0]` formal, actuals `300.0`, `-2.5`, `rv * 1.0`, `1e20`, `rv` (`rv = 70000.4`).
#[test]
fn at_d_class_method() {
    check(
        r#"
module t;
  real rv = 70000.4;
  class C; function byte m(input byte x); return x; endfunction endclass
  C c;
  initial begin
    c = new;
    $display("T d0 %h", c.m(300.0));
    $display("T d1 %h", c.m(-2.5));
    $display("T d2 %h", c.m(rv * 1.0));
    $display("T d3 %h", c.m(1e20));
    $display("T d4 %h", c.m(rv));
    #1 $finish;
  end
endmodule
"#,
        &[
            r#"T d0 2c"#,
            r#"T d1 fd"#,
            r#"T d2 70"#,
            r#"T d3 00"#,
            r#"T d4 70"#,
        ],
    );
    check(
        r#"
module t;
  real rv = 70000.4;
  class C; function longint m(input longint x); return x; endfunction endclass
  C c;
  initial begin
    c = new;
    $display("T d0 %h", c.m(300.0));
    $display("T d1 %h", c.m(-2.5));
    $display("T d2 %h", c.m(rv * 1.0));
    $display("T d3 %h", c.m(1e20));
    $display("T d4 %h", c.m(rv));
    #1 $finish;
  end
endmodule
"#,
        &[
            r#"T d0 000000000000012c"#,
            r#"T d1 fffffffffffffffd"#,
            r#"T d2 0000000000011170"#,
            r#"T d3 6bc75e2d63100000"#,
            r#"T d4 0000000000011170"#,
        ],
    );
    check(
        r#"
module t;
  real rv = 70000.4;
  class C; function bit [7:0] m(input bit [7:0] x); return x; endfunction endclass
  C c;
  initial begin
    c = new;
    $display("T d0 %h", c.m(300.0));
    $display("T d1 %h", c.m(-2.5));
    $display("T d2 %h", c.m(rv * 1.0));
    $display("T d3 %h", c.m(1e20));
    $display("T d4 %h", c.m(rv));
    #1 $finish;
  end
endmodule
"#,
        &[
            r#"T d0 2c"#,
            r#"T d1 fd"#,
            r#"T d2 70"#,
            r#"T d3 00"#,
            r#"T d4 70"#,
        ],
    );
}

/// a class task: a `byte`, a `longint` and a `bit [7:0]` formal, actuals `300.0`, `-2.5`, `rv * 1.0`, `1e20`, `rv` (`rv = 70000.4`).
#[test]
fn at_t_class_task() {
    check(
        r#"
module t;
  real rv = 70000.4;
  class C; task t(input byte x, input int i); $display("T t%0d %h", i, x); endtask endclass
  C c;
  initial begin
    c = new;
    c.t(300.0, 0);
    c.t(-2.5, 1);
    c.t(rv * 1.0, 2);
    c.t(1e20, 3);
    c.t(rv, 4);
    #1 $finish;
  end
endmodule
"#,
        &[
            r#"T t0 2c"#,
            r#"T t1 fd"#,
            r#"T t2 70"#,
            r#"T t3 00"#,
            r#"T t4 70"#,
        ],
    );
    check(
        r#"
module t;
  real rv = 70000.4;
  class C; task t(input longint x, input int i); $display("T t%0d %h", i, x); endtask endclass
  C c;
  initial begin
    c = new;
    c.t(300.0, 0);
    c.t(-2.5, 1);
    c.t(rv * 1.0, 2);
    c.t(1e20, 3);
    c.t(rv, 4);
    #1 $finish;
  end
endmodule
"#,
        &[
            r#"T t0 000000000000012c"#,
            r#"T t1 fffffffffffffffd"#,
            r#"T t2 0000000000011170"#,
            r#"T t3 6bc75e2d63100000"#,
            r#"T t4 0000000000011170"#,
        ],
    );
    check(
        r#"
module t;
  real rv = 70000.4;
  class C; task t(input bit [7:0] x, input int i); $display("T t%0d %h", i, x); endtask endclass
  C c;
  initial begin
    c = new;
    c.t(300.0, 0);
    c.t(-2.5, 1);
    c.t(rv * 1.0, 2);
    c.t(1e20, 3);
    c.t(rv, 4);
    #1 $finish;
  end
endmodule
"#,
        &[
            r#"T t0 2c"#,
            r#"T t1 fd"#,
            r#"T t2 70"#,
            r#"T t3 00"#,
            r#"T t4 70"#,
        ],
    );
}

/// a class constructor: a `byte`, a `longint` and a `bit [7:0]` formal, actuals `300.0`, `-2.5`, `rv * 1.0`, `1e20`, `rv` (`rv = 70000.4`).
#[test]
fn at_e_class_constructor() {
    check(
        r#"
module t;
  real rv = 70000.4;
  class C; byte v; function new(input byte x); v = x; endfunction endclass
  C c;
  initial begin
    c = new(300.0);
    $display("T e0 %h", c.v);
    c = new(-2.5);
    $display("T e1 %h", c.v);
    c = new(rv * 1.0);
    $display("T e2 %h", c.v);
    c = new(1e20);
    $display("T e3 %h", c.v);
    c = new(rv);
    $display("T e4 %h", c.v);
    #1 $finish;
  end
endmodule
"#,
        &[
            r#"T e0 2c"#,
            r#"T e1 fd"#,
            r#"T e2 70"#,
            r#"T e3 00"#,
            r#"T e4 70"#,
        ],
    );
    check(
        r#"
module t;
  real rv = 70000.4;
  class C; longint v; function new(input longint x); v = x; endfunction endclass
  C c;
  initial begin
    c = new(300.0);
    $display("T e0 %h", c.v);
    c = new(-2.5);
    $display("T e1 %h", c.v);
    c = new(rv * 1.0);
    $display("T e2 %h", c.v);
    c = new(1e20);
    $display("T e3 %h", c.v);
    c = new(rv);
    $display("T e4 %h", c.v);
    #1 $finish;
  end
endmodule
"#,
        &[
            r#"T e0 000000000000012c"#,
            r#"T e1 fffffffffffffffd"#,
            r#"T e2 0000000000011170"#,
            r#"T e3 6bc75e2d63100000"#,
            r#"T e4 0000000000011170"#,
        ],
    );
    check(
        r#"
module t;
  real rv = 70000.4;
  class C; bit [7:0] v; function new(input bit [7:0] x); v = x; endfunction endclass
  C c;
  initial begin
    c = new(300.0);
    $display("T e0 %h", c.v);
    c = new(-2.5);
    $display("T e1 %h", c.v);
    c = new(rv * 1.0);
    $display("T e2 %h", c.v);
    c = new(1e20);
    $display("T e3 %h", c.v);
    c = new(rv);
    $display("T e4 %h", c.v);
    #1 $finish;
  end
endmodule
"#,
        &[
            r#"T e0 2c"#,
            r#"T e1 fd"#,
            r#"T e2 70"#,
            r#"T e3 00"#,
            r#"T e4 70"#,
        ],
    );
}

/// The ten refused cells: a hierarchical function call to a `logic`-typed function, which the
/// inline lane lowered and so has no frame — refused whatever the actual (`u.fn(3)` too; both
/// oracles run it). ROADMAP §3.b `hier-fn-inline-callee`.
#[test]
fn a_hierarchical_call_to_an_inline_lowered_callee_is_refused() {
    let (ok, s) = run_any(
        r#"module sub; function logic signed [11:0] fn(input logic signed [11:0] x); fn = x; endfunction endmodule
module t;
  real rv = 70000.4;
  
  sub u();
  initial begin
    $display("T c0 %h", u.fn(300.0));
    $display("T c1 %h", u.fn(-2.5));
    $display("T c2 %h", u.fn(rv * 1.0));
    $display("T c3 %h", u.fn(1e20));
    $display("T c4 %h", u.fn(rv));
    #1 $finish;
  end
endmodule
"#,
        "native",
    );
    assert!(
        !ok && s.contains("unsupported hierarchical function call `u.fn`"),
        "{s}"
    );
    let (ok, s) = run_any(
        r#"module sub; function logic [128:0] fn(input logic [128:0] x); fn = x; endfunction endmodule
module t;
  real rv = 70000.4;
  
  sub u();
  initial begin
    $display("T c0 %h", u.fn(300.0));
    $display("T c1 %h", u.fn(-2.5));
    $display("T c2 %h", u.fn(rv * 1.0));
    $display("T c3 %h", u.fn(1e20));
    $display("T c4 %h", u.fn(rv));
    #1 $finish;
  end
endmodule
"#,
        "native",
    );
    assert!(
        !ok && s.contains("unsupported hierarchical function call `u.fn`"),
        "{s}"
    );
}
