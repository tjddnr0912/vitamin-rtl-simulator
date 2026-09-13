//! Multidriver Rule A (a declaration initializer plus an `always_comb` writer)
//! resolves a user-call actual's DIRECTION: an actual bound to an `input` formal is
//! a read, one bound to an `output` / `inout` formal — or an unresolvable callee —
//! stays a write.
//!
//! Rule A's conservative walk (`stmt_never_writes_ident`) was handed no call
//! resolver, so every actual of a user call counted as a possible write:
//! `always_comb a6 = id8(u8) * b8;` beside `logic [7:0] u8 = 8'hF7` was a false-loud
//! E3001 where both oracles print `f609`, and `always_comb rd(acc, o)` with `input
//! int v` the same. The census now threads the module's `call_effect` resolver
//! (the closure the block-local gate uses), computed under a shared borrow before
//! the diagnostics are emitted. verilator reports MULTIDRIVEN for an `inout` actual
//! (`always_comb bump(acc)`), and that cell keeps its loud.
//!
//! ORACLES: iverilog 13.0 (-g2012) and verilator 5.052 (--binary --timing); the
//! accepted values below were measured in both. PRE refused every accepted design
//! with E3001.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_accad_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let so = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n");
    (
        so,
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

/// ① An `input` actual of an inline function, of an automatic function, of a
/// nested call and of a task is a read.
#[test]
fn an_input_actual_in_an_always_comb_is_a_read() {
    let (o, e, code) = run(r#"module t;
  logic [7:0] u8 = 8'hF7, b8 = 8'hFF;
  int acc = 0, o1;
  logic [31:0] a1, a2, a3;
  function [7:0] id8(input [7:0] v); id8 = v; endfunction
  function automatic [7:0] fa8(input [7:0] v); fa8 = v; endfunction
  task automatic rd(input int v, output int r); r = v + 1; endtask
  always_comb a1 = id8(u8) * b8;
  always_comb a2 = fa8(u8) * b8;
  always_comb a3 = id8(id8(u8)) * b8;
  always_comb rd(acc, o1);
  initial begin #1 $display("A=%h %h %h O=%0d", a1, a2, a3, o1); #1 $finish; end
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "A=0000f609 0000f609 0000f609 O=1");
}

/// ② An `inout` actual is still a driver (verilator: MULTIDRIVEN), and so is an
/// `output` actual beside an initializer; both stay E3001, PRE-identical.
#[test]
fn an_inout_or_output_actual_stays_a_write() {
    let (_, e, code) = run(r#"module t;
  int acc = 0;
  task automatic bump(inout int v); v = v + 1; endtask
  always_comb bump(acc);
  initial begin #1 $finish; end
endmodule
"#);
    assert_ne!(code, Some(0));
    assert!(
        e.contains("variable `acc` has a declaration initializer AND is written by"),
        "{e}"
    );
    let (_, e, code) = run(r#"module t;
  int acc = 0, o = 5;
  task automatic rd(input int v, output int r); r = v + 1; endtask
  always_comb rd(acc, o);
  initial begin #1 $finish; end
endmodule
"#);
    assert_ne!(code, Some(0));
    assert!(
        e.contains("variable `o` has a declaration initializer AND is written by"),
        "{e}"
    );
}
