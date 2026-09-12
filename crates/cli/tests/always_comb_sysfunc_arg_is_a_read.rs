//! A system FUNCTION's argument in an `always_comb` right-hand side is a READ, not
//! a write — the multidriver census's Rule A no longer refuses it.
//!
//! Rule A (a declaration initializer plus an `always_comb` writer, IEEE §9.2.2.2)
//! keeps the conservative `stmt_never_writes_ident` walk on purpose (an actual bound
//! to an `inout` formal IS a driver in verilator). That walk's expression arm counted
//! EVERY argument of a `SysCall` as a possible write, so the `always_comb`
//! `a = $signed(u8) * q8;` beside `logic [7:0] u8 = 8'hF7` was a false-loud E3001 where
//! both oracles print `120` — and so were `$unsigned`, `$clog2`, `$bits`,
//! `$countones` and a size cast beside them — while the stamp-free `u8 * q8` was
//! accepted. The arm now mirrors the statement-form `SysTaskCall` arm: only a
//! WRITE-dest argument counts as a write (`syscall_read_args` isolates them: the
//! destinations of `$sscanf`, the buffer of `$fgets`, and so on), and every
//! argument is still scanned for a nested copy-back call.
//!
//! ORACLES: iverilog 13.0 (-g2012) and verilator 5.052 (--binary --timing); the
//! values below were measured in BOTH. PRE (a release binary at the parent commit)
//! refused every design in ① with E3001.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_acsf_{}_{n}", std::process::id()));
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

/// ① THE HEADLINE: six system functions over an initialized variable in an
/// `always_comb` rhs, each beside the plain read that was already accepted.
#[test]
fn a_system_function_argument_in_an_always_comb_rhs_is_a_read() {
    let (o, e, code) = run(r#"module t;
  logic [7:0] u8 = 8'hF7, b8 = 8'hFF;
  logic signed [7:0] s8 = -9, q8 = -32;
  logic [31:0] a1, a2, a3, a4, a5, a7;
  always_comb a1 = $signed(u8) * q8;
  always_comb a2 = $unsigned(s8) * q8;
  always_comb a3 = $clog2(u8);
  always_comb a4 = $bits(u8) + u8;
  always_comb a5 = 8'(u8) * b8;
  always_comb a7 = u8 * b8;
  initial begin #1 $display("A=%h %h %h %h %h %h", a1, a2, a3, a4, a5, a7); #1 $finish; end
endmodule
"#);
    assert_eq!(code, Some(0), "{e}");
    assert_eq!(o, "A=00000120 0000d820 00000008 000000ff 0000f609 0000f609");
}

/// ② The WRITE-dest argument of a system function still counts: `$sscanf`'s
/// destination beside a declaration initializer stays E3001 (PRE-identical), and a
/// user-call actual keeps the conservative answer (ROADMAP §3.b `mdrv-actual`).
#[test]
fn a_write_dest_argument_and_a_user_call_actual_still_count_as_writes() {
    let (_, e, code) = run(r#"module t;
  logic [31:0] o1 = 32'd1;
  int n; string str = "ab";
  always_comb begin n = $sscanf(str, "%d", o1); end
  initial begin #1 $finish; end
endmodule
"#);
    assert_ne!(code, Some(0));
    assert!(
        e.contains("variable `o1` has a declaration initializer AND is written by"),
        "{e}"
    );
    let (_, e, code) = run(r#"module t;
  logic [7:0] u8 = 8'hF7, b8 = 8'hFF;
  logic [31:0] a6;
  function [7:0] id8(input [7:0] v); id8 = v; endfunction
  always_comb a6 = id8(u8) * b8;
  initial begin #1 $finish; end
endmodule
"#);
    assert_ne!(code, Some(0));
    assert!(
        e.contains("variable `u8` has a declaration initializer AND is written by"),
        "{e}"
    );
}

/// ③ ROUND-1 SOUNDNESS FINDING (fixed): the SEED of `$random(seed)` and of every
/// `$dist_*(seed, …)` is written back (the engine's `SeededRandom` / `SeededDist`
/// effects), so it stays a write for the never-writes walk — `syscall_writes_arg`
/// is the write view of the table, not the complement of `syscall_read_args`. The
/// first draft narrowed the arm to the read table alone, and an `automatic integer
/// sd = 7; a = $random(sd);` under a `fork` was proven "never reassigned" and
/// flattened: both activations drew from ONE seed (`SAME=0` where verilator prints
/// `SAME=1`; iverilog refuses the lifetime override). PRE and POST refuse the shape
/// with the same E3009. The `always_comb` twin is a genuine two-driver report.
#[test]
fn a_seeded_random_argument_is_still_a_write() {
    let (_, e, code) = run(r#"module t;
  integer p0, p1;
  initial begin
    for (int i = 0; i < 2; i++) begin
      fork
        begin
          automatic integer sd = 7;
          integer a;
          a = $random(sd);
          if (i == 0) p0 = a; else p1 = a;
        end
      join
    end
    $display("SAME=%0d", (p0 === p1));
    #2 $finish;
  end
endmodule
"#);
    assert_ne!(code, Some(0));
    assert!(
        e.contains("an `automatic` block-local `sd` whose per-entry lifetime differs from static"),
        "{e}"
    );
    let (_, e, code) = run(r#"module t;
  integer seed = 7;
  logic [31:0] a;
  always_comb a = $random(seed);
  initial begin #1 $finish; end
endmodule
"#);
    assert_ne!(code, Some(0));
    assert!(
        e.contains("variable `seed` has a declaration initializer AND is written by"),
        "{e}"
    );
}
