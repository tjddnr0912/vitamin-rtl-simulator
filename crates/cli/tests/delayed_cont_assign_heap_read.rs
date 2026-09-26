//! A DELAYED continuous assign that reads heap content is evaluated through the heap router on
//! the native backend (ROADMAP §2 "Delays / events").
//!
//! `Scheduler::schedule_delayed_cas` is shared by both kernels; on tier-3 it evaluates the
//! delayed rhs, the runtime delay and the dynamic left-side index against the reader it is
//! handed — the bare arena, which does not own queue / dynamic-array / associative-array /
//! string contents. So `assign #0 n = q.size();` read x at every hop on native where the
//! interpreter and the VM read 0, then 1, and `assign #(q.size()) n = a;` fired with no delay.
//! All three reads now go through `HeapRouted`, the wrapper the settle's own evaluation uses.
//!
//! Every cell asserts the native value AND that `--backend interp` prints the same lines — the
//! reference backend is the arbiter here: iverilog 13.0 refuses most of these designs, and
//! verilator 5.052 agrees with the interpreter on every value except the same-time ordering of
//! the undelayed twin in `h1` (verilator `m = 1` at 1) and the two `#(1, 2)` hold windows in
//! `h7` (verilator 0 where vita holds the driver's initial x).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str, backend: &str) -> Vec<String> {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_dcah_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(["--backend", backend])
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let s =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "expected exit 0 on {backend}, got:\n{s}"
    );
    s.lines()
        .filter(|l| l.starts_with('T'))
        .map(|l| l.trim().to_string())
        .collect()
}

fn check(src: &str, want: &[&str]) {
    let native = run(src, "native");
    assert_eq!(native, want, "native");
    assert_eq!(
        run(src, "interp"),
        native,
        "the reference backend disagrees"
    );
}

/// `q.size()` through `#0` beside its undelayed twin. PRE native: `x` at every hop.
#[test]
fn delayed_heap_read_h1() {
    check(
        r#"module t;
  int q[$];
  wire [31:0] n, m;
  assign #0 n = q.size();
  assign m = q.size();
  initial begin
    #1 q.push_back(5); #2 q.push_back(6); #2 q.delete();
  end
  initial begin
    #1 $display("T1 %0d %0d", n, m);
    #1 $display("T2 %0d %0d", n, m);
    #1 $display("T3 %0d %0d", n, m);
    #1 $display("T4 %0d %0d", n, m);
    #2 $display("T6 %0d %0d", n, m);
    $finish;
  end
endmodule
"#,
        &[
            r#"T1 0 0"#,
            r#"T2 1 1"#,
            r#"T3 1 1"#,
            r#"T4 2 2"#,
            r#"T6 0 0"#,
        ],
    );
}

/// `#1`. PRE native: `x` throughout.
#[test]
fn delayed_heap_read_h2() {
    check(
        r#"module t;
  int q[$];
  wire [31:0] n;
  assign #1 n = q.size();
  initial begin
    #1 q.push_back(5); #2 q.push_back(6);
  end
  initial begin
    #1 $display("T1 %0d", n);
    #1 $display("T2 %0d", n);
    #1 $display("T3 %0d", n);
    #1 $display("T4 %0d", n);
    #2 $display("T6 %0d", n);
    $finish;
  end
endmodule
"#,
        &[r#"T1 0"#, r#"T2 1"#, r#"T3 1"#, r#"T4 2"#, r#"T6 2"#],
    );
}

/// A ternary over `q.size()` and `q[0]`. PRE native: `X`.
#[test]
fn delayed_heap_read_h3() {
    check(
        r#"module t;
  int q[$];
  wire [31:0] n;
  assign #0 n = (q.size() > 0) ? q[0] : 32'd99;
  initial begin
    #1 q.push_back(5); #2 q.push_front(7);
  end
  initial begin
    #1 $display("T1 %0d", n);
    #1 $display("T2 %0d", n);
    #1 $display("T3 %0d", n);
    #1 $display("T4 %0d", n);
    #2 $display("T6 %0d", n);
    $finish;
  end
endmodule
"#,
        &[r#"T1 99"#, r#"T2 5"#, r#"T3 5"#, r#"T4 7"#, r#"T6 7"#],
    );
}

/// A dynamic array's `size()`.
#[test]
fn delayed_heap_read_h4() {
    check(
        r#"module t;
  int da[];
  wire [31:0] n;
  assign #0 n = da.size();
  initial begin
    #1 da = new[3]; #2 da = new[5];
  end
  initial begin
    #1 $display("T1 %0d", n);
    #1 $display("T2 %0d", n);
    #1 $display("T3 %0d", n);
    #1 $display("T4 %0d", n);
    #2 $display("T6 %0d", n);
    $finish;
  end
endmodule
"#,
        &[r#"T1 0"#, r#"T2 3"#, r#"T3 3"#, r#"T4 5"#, r#"T6 5"#],
    );
}

/// A string's `len()`. PRE native: 0 throughout.
#[test]
fn delayed_heap_read_h5() {
    check(
        r#"module t;
  string s;
  wire [31:0] n;
  assign #0 n = s.len();
  initial begin
    #1 s = "ab"; #2 s = "abcd";
  end
  initial begin
    #1 $display("T1 %0d", n);
    #1 $display("T2 %0d", n);
    #1 $display("T3 %0d", n);
    #1 $display("T4 %0d", n);
    #2 $display("T6 %0d", n);
    $finish;
  end
endmodule
"#,
        &[r#"T1 0"#, r#"T2 2"#, r#"T3 2"#, r#"T4 4"#, r#"T6 4"#],
    );
}

/// An associative array's `num()`.
#[test]
fn delayed_heap_read_h6() {
    check(
        r#"module t;
  int aa[int];
  wire [31:0] n;
  assign #0 n = aa.num();
  initial begin
    #1 aa[3] = 1; #2 aa[5] = 2;
  end
  initial begin
    #1 $display("T1 %0d", n);
    #1 $display("T2 %0d", n);
    #1 $display("T3 %0d", n);
    #1 $display("T4 %0d", n);
    #2 $display("T6 %0d", n);
    $finish;
  end
endmodule
"#,
        &[r#"T1 0"#, r#"T2 1"#, r#"T3 1"#, r#"T4 2"#, r#"T6 2"#],
    );
}

/// Rise / fall delays `#(1, 2)`: the driver holds x until its first write lands, then follows.
#[test]
fn delayed_heap_read_h7() {
    check(
        r#"module t;
  int q[$];
  wire [31:0] n;
  assign #(1,2) n = q.size();
  initial begin
    #1 q.push_back(5); #3 q.delete();
  end
  initial begin
    #1 $display("T1 %0d", n);
    #1 $display("T2 %0d", n);
    #1 $display("T3 %0d", n);
    #1 $display("T4 %0d", n);
    #2 $display("T6 %0d", n);
    $finish;
  end
endmodule
"#,
        &[r#"T1 x"#, r#"T2 x"#, r#"T3 1"#, r#"T4 1"#, r#"T6 0"#],
    );
}

/// A RUNTIME delay that reads the queue (`#(q.size())`, 3). PRE native: delay 0 — `11` at 2, `22` at 12.
#[test]
fn delayed_heap_read_d1() {
    check(
        r#"module t;
  int q[$];
  reg [7:0] a = 0;
  wire [7:0] n;
  assign #(q.size()) n = a;
  initial begin
    q.push_back(1); q.push_back(2); q.push_back(3);
    #1 a = 8'h11;
    #10 a = 8'h22;
  end
  initial begin
    #2 $display("T2 %h", n);
    #2 $display("T4 %h", n);
    #8 $display("T12 %h", n);
    #2 $display("T14 %h", n);
    $finish;
  end
endmodule
"#,
        &[r#"T2 00"#, r#"T4 11"#, r#"T12 11"#, r#"T14 22"#],
    );
}

/// NOT this slice, pinned as measured on both backends (ROADMAP §2 "Delays / events"): a delayed assign whose DYNAMIC left-side index moves while its rhs does not keeps the target it was first scheduled at — `assign #1 y[q.size()] = v;` writes `y[0]` (the size at the time-0 settle) where verilator writes `y[1]`; iverilog refuses the non-constant index. The "rhs unchanged, no new write" shortcut in `schedule_delayed_cas` compares the value only.
#[test]
fn a_moving_dynamic_index_is_the_recorded_residue() {
    check(
        r#"module t;
  int q[$];
  reg [7:0] v = 8'hA5;
  wire [7:0] y [0:3];
  assign #1 y[q.size()] = v;
  initial begin
    q.push_back(1);
    #3 q.push_back(2);
  end
  initial begin
    #2 $display("T2 %h %h %h", y[0], y[1], y[2]);
    #3 $display("T5 %h %h %h", y[0], y[1], y[2]);
    $finish;
  end
endmodule
"#,
        &[r#"T2 a5 xx zz"#, r#"T5 a5 xx zz"#],
    );
}
