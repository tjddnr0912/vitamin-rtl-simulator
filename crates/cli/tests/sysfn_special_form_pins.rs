//! The whole diagnostic text, line:col and exit code of the special-form cells whose output
//! depends on WHERE a special-form column is read (ROADMAP §5.2 SYS-1b, B7b): the refusal
//! before argument lowering (`expr_main.rs`, C7 / C8), the arity test before `$sampled`'s
//! recursion (`sva_decl.rs`, C9), and the argc clamps of `write_arg_range` (C2) and
//! `syscall_read_args` (C5) that run before each form's arity refusal, so an unclamped
//! range would panic. Every expected text was measured on the PRE binary (main 4a8f086b +
//! SYS-1a, `vita-post` md5 56c997be; W/sys1b/plan/probe/out) with `t.sv` as the argument,
//! so the positions print as `t.sv:<line>:<col>`.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// (stdout, stderr, exit code) of `vita t.sv` in a temp dir of its own.
fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_sysfn_sf_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("t.sv"), src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&dir)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&dir);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

fn check(id: &str, src: &str, want_stderr: &str) {
    let (so, se, rc) = run(src);
    assert_eq!(so, "", "{id}: stdout");
    assert_eq!(se, want_stderr, "{id}: stderr");
    assert_eq!(rc, Some(1), "{id}: exit code");
}

/// C7 ORDER: the nested-placement refusal comes before argument lowering (the inner `$fgetc` prints no E3009 of its own).
#[test]
fn c7_order_arg() {
    check(
        "c7_order_arg",
        r#"module m;
integer x, fd; reg [64:1] s; reg c;
initial begin
  x = c ? $fgets(s, $fgetc(fd)) : 0;
  #1 $finish;
end
endmodule
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:4:11: error[VITA-E3009] E-ELAB-UNSUPPORTED: `$fgets` advances the file position, so vita evaluates it into a temporary once, before the statement runs. This position is refused because the call would then happen a DIFFERENT NUMBER OF TIMES than written: the right operand of `&&` / `||` and an arm of `?:` may be skipped, a loop condition is re-evaluated per iteration, and a `$monitor` / `$strobe` argument is re-rendered on every later change and would show the frozen temporary. Read it into a variable in its own statement first, then use that variable here. Placements evaluated exactly once per execution are supported — a blocking or nonblocking rhs, an `if` condition, a `case` scrutinee, a `repeat` count, a `$display`-style argument, an lvalue index [in m]
errors=1 warnings=1 notes=0
"#,
    );
}

/// C8 ORDER: the `$dist_*` refusal comes before argument lowering (the inner `$dist_t` prints no second E3009).
#[test]
fn c8_order_arg() {
    check(
        "c8_order_arg",
        r#"module m;
integer x;
initial begin
  x = 1 + $dist_normal(x, $dist_t(x, 2), 1);
  #1 $finish;
end
endmodule
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:4:3: error[VITA-E3009] E-ELAB-UNSUPPORTED: $dist_* / $cast (function form) are supported only as the direct rhs of a blocking assignment (v9/v19) [in m]
errors=1 warnings=1 notes=0
"#,
    );
}

/// C9 ORDER: the arity test before `$sampled`'s recursion (an empty argument list is not indexed).
#[test]
fn c9_sampled_argc0() {
    check(
        "c9_sampled_argc0",
        r#"module m;
  logic clk = 0; logic a = 0, b = 0;
  always #1 clk = ~clk;
  initial begin #4 a = 1; #4 a = 0; #4 $display("C9 done"); $finish; end
  assert property (@(posedge clk) $sampled() |-> b) else $display("C9 fail t=%0t", $time);
endmodule
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:8: error[VITA-E3009] E-ELAB-UNSUPPORTED: `$sampled` takes one signal argument in v1 [in m]
t.sv:5:3: error[VITA-E3009] E-ELAB-UNSUPPORTED: unsupported system function in expression [in m]
errors=2 warnings=1 notes=0
"#,
    );
}

/// C9 arity != 1.
#[test]
fn c9_arity() {
    check(
        "c9_arity",
        r#"module m;
  logic clk = 0; logic a = 0, b = 0;
  always #1 clk = ~clk;
  initial begin #4 a = 1; #4 a = 0; #4 $display("C9 done"); $finish; end
  assert property (@(posedge clk) $past(a, 2) |-> a) else $display("C9 fail t=%0t", $time);
endmodule
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:8: error[VITA-E3009] E-ELAB-UNSUPPORTED: `$past` takes one signal argument in v1 [in m]
t.sv:5:3: error[VITA-E3009] E-ELAB-UNSUPPORTED: unsupported system function in expression [in m]
errors=2 warnings=1 notes=0
"#,
    );
}

/// C5 / C4 L2: `$value$plusargs`'s third argument neither read nor written (no read-gate E3009, the arity E3009 only).
#[test]
fn c5_vplus_argc3() {
    check(
        "c5_vplus_argc3",
        r#"module m;
  integer r1, n, fd, ok, v;
  initial begin
  fd = $fopen("d.txt", "w"); $fdisplay(fd, "42 7"); $fclose(fd);
  fd = $fopen("d.txt", "r");
    begin : b1 logic [7:0] s; s = 8'h41; r1 = s[0]; end
    begin : b2 string s; string d; ok = $value$plusargs("foo=%s", d, s); end
    $display("C5 r=%0d", r1); #1 $finish;
  end
endmodule
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:7:36: error[VITA-E3009] E-ELAB-UNSUPPORTED: $value$plusargs takes (format, variable) [in m]
errors=1 warnings=1 notes=0
"#,
    );
}

/// C2 clamp `0..1.min(argc)` at argc 0 (the hoist runs it before the arity refusal).
#[test]
fn c2_fgets_argc0() {
    check(
        "c2_fgets_argc0",
        r#"module m;
integer x;
initial begin
  x = 1 + $fgets(); $display("C2 x=%0d", x);
  #1 $finish;
end
endmodule
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:4:11: error[VITA-E3009] E-ELAB-UNSUPPORTED: $fgets takes (str, fd) [in m]
errors=1 warnings=1 notes=0
"#,
    );
}

/// C2 clamp `1.min(argc)..2.min(argc)` at argc 0.
#[test]
fn c2_vplus_argc0() {
    check(
        "c2_vplus_argc0",
        r#"module m;
integer x;
initial begin
  x = 1 + $value$plusargs(); $display("C2 x=%0d", x);
  #1 $finish;
end
endmodule
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:4:11: error[VITA-E3009] E-ELAB-UNSUPPORTED: $value$plusargs takes (format, variable) [in m]
errors=1 warnings=1 notes=0
"#,
    );
}

/// C5 clamp `..2.min(len)` at argc 0 (the read gate runs it before the arity refusal).
#[test]
fn c5_sscanf_argc0() {
    check(
        "c5_sscanf_argc0",
        r#"module m;
  integer r1, n, fd, ok, v;
  initial begin
  fd = $fopen("d.txt", "w"); $fdisplay(fd, "42 7"); $fclose(fd);
  fd = $fopen("d.txt", "r");
    begin : b1 logic [7:0] s; s = 8'h41; r1 = s[0]; end
    begin : b2 string s; n = $sscanf(); end
    $display("C5 r=%0d", r1); #1 $finish;
  end
endmodule
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:7:26: error[VITA-E3009] E-ELAB-UNSUPPORTED: $fscanf/$sscanf take (source, format, args...) [in m]
errors=1 warnings=1 notes=0
"#,
    );
}

/// C5 clamp `..1.min(len)` at argc 0 (the only cell for that clamp).
#[test]
fn c5_vplus_argc0() {
    check(
        "c5_vplus_argc0",
        r#"module m;
  integer r1, n, fd, ok, v;
  initial begin
  fd = $fopen("d.txt", "w"); $fdisplay(fd, "42 7"); $fclose(fd);
  fd = $fopen("d.txt", "r");
    begin : b1 logic [7:0] s; s = 8'h41; r1 = s[0]; end
    begin : b2 string s; ok = $value$plusargs(); end
    $display("C5 r=%0d", r1); #1 $finish;
  end
endmodule
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:7:26: error[VITA-E3009] E-ELAB-UNSUPPORTED: $value$plusargs takes (format, variable) [in m]
errors=1 warnings=1 notes=0
"#,
    );
}
