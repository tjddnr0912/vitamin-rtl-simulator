//! The whole stdout, stderr (diagnostic code, text, line:col) and exit code of the VAL-3 lane cells
//! that show the range router's WRAP lane, its path C and the declared-range check chain at each
//! site (ROADMAP §5.2 VAL-3: `value/chains.rs`'s `range_bounds_checked` in `packed_extents` (S1),
//! `check_param_decl_range` (S2), `array_dim_extents` (S3) and `range_to_dims_opt` (S4) ; the
//! router's paths in `value/mod.rs`'s head note). Every expected text was measured on the PRE binary
//! (257c4990, `vita-post` md5 d5ac3b05 ; W/val3/build/gen/pre_out) with `t.sv` as the argument.
//! `w40` is not here: `range_rules_pins.rs` holds the same source.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// (stdout, stderr, exit code) of `vita t.sv` in a temp dir of its own.
fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_range_bound_{}_{n}", std::process::id()));
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

fn check(id: &str, src: &str, want_stdout: &str, want_stderr: &str, want_rc: i32) {
    let (so, se, rc) = run(src);
    assert_eq!(so, want_stdout, "{id}: stdout");
    assert_eq!(se, want_stderr, "{id}: stderr");
    assert_eq!(rc, Some(want_rc), "{id}: exit code");
}

/// WRAP lane edge at 2^63: `[64'hFFFF_FFFF_FFFF_FFFF : 0]` reads -1 at the router (path B, c1) and the width comes from [-1:0].
#[test]
fn r11_r1w_wide_count() {
    check(
        "r11_r1w_wide_count",
        r##"module top;
  logic [64'hFFFF_FFFF_FFFF_FFFF : 0] w;
  initial begin #1 $display("A bw=%0d l=%0d", $bits(w), $left(w)); $finish; end
endmodule
"##,
        r##"A bw=2 l=-1
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// WRAP lane: a >= 2^63 pair, `$left` / `$right` print the low 32 bits.
#[test]
fn r12_u64_bound_text() {
    check(
        "r12_u64_bound_text",
        r##"module top;
  logic [64'hFFFF_FFFF_0000_0002 : 64'hFFFF_FFFF_0000_0000] w;
  initial begin w = 3'b101; #1 $display("A bw=%0d l=%0d r=%0d", $bits(w), $left(w), $right(w)); $finish; end
endmodule
"##,
        r##"A bw=3 l=2 r=0
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// WRAP lane through a `longint unsigned` name at S2.
#[test]
fn r13_u64_lp_text() {
    check(
        "r13_u64_lp_text",
        r##"module top;
  localparam longint unsigned B = 64'h8000_0000_0000_0001;
  localparam logic [B : B - 1] P = 0;
  initial begin #1 $display("A bp=%0d", $bits(P)); $finish; end
endmodule
"##,
        r##"A bp=2
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// WRAP lane: the net-width cap error names 4294967297.
#[test]
fn r14_u64_count_text() {
    check(
        "r14_u64_count_text",
        r##"module top;
  logic [64'hFFFF_FFFF_0000_0000 : 0] w;
  initial begin #1 $display("A bw=%0d", $bits(w)); $finish; end
endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:2:3: error[VITA-E3009] E-ELAB-UNSUPPORTED: declared net width 4294967297 exceeds the v1 cap (1048576) [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// WRAP lane at S2: the parameter keeps its 1 bit.
#[test]
fn r15_u64_lp_count_text() {
    check(
        "r15_u64_lp_count_text",
        r##"module top;
  localparam logic [64'hFFFF_FFFF_0000_0000 : 0] P = 0;
  initial begin #1 $display("A bp=%0d", $bits(P)); $finish; end
endmodule
"##,
        r##"A bp=1
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// path C: the first fold reads 4294967295, IN reads -1 and IN's value goes on (c6).
#[test]
fn g_negu8() {
    check(
        "g_negu8",
        r##"module top; localparam logic [7:0] U = 0; logic [U-1:0] w;
  initial begin #1 $display("A bw=%0d l=%0d r=%0d", $bits(w), $left(w), $right(w)); $finish; end endmodule
"##,
        r##"A bw=2 l=-1 r=0
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// path C: `int unsigned U = 32'hFFFF_FFFE ; [U:U-1]`, IN's stored i64 replaces the self-determined value (F-c).
#[test]
fn g_w32u() {
    check(
        "g_w32u",
        r##"module top; localparam int unsigned U = 32'hFFFF_FFFE; logic [U:U-1] w;
  initial begin #1 $display("A bw=%0d l=%0d r=%0d", $bits(w), $left(w), $right(w)); $finish; end endmodule
"##,
        r##"A bw=2 l=-2 r=-3
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// path C over a constant function call above i32::MAX (the call is folded twice, F-a).
#[test]
fn g_cfn_big() {
    check(
        "g_cfn_big",
        r##"module top;
  function automatic longint f(); return 64'h1_0000_0003; endfunction
  logic [f() : f() - 3] w;
  initial begin #1 $display("A bw=%0d l=%0d r=%0d", $bits(w), $left(w), $right(w)); $finish; end endmodule
"##,
        r##"A bw=1 l=3 r=0
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// path C after a Four: the x evidence is dropped and IN answers 3.
#[test]
fn x04() {
    check(
        "x04",
        r##"module top;
  localparam int Z = 0;
  logic [((8/Z) & 0) + 3 : 0] v;
  initial $display("A b=%0d l=%0d r=%0d", $bits(v), $left(v), $right(v));
  initial #20 $finish;
endmodule
"##,
        r##"A b=1 l=x r=x
simulation ended (Finish) at time 20
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// S1 on a module net (S4 span-deduped): both bounds fire, msb then lsb (mutants M1 M2 M3).
#[test]
fn o_net() {
    check(
        "o_net",
        r##"module top; logic n, k; logic [n:k] v;
  initial begin #1 $display("A b=%0d", $bits(v)); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:32: error[VITA-E3009] E-ELAB-UNSUPPORTED: a reference to net/variable `n` is not allowed in a constant range bound [in top]
t.sv:1:34: error[VITA-E3009] E-ELAB-UNSUPPORTED: a reference to net/variable `k` is not allowed in a constant range bound [in top]
errors=2 warnings=1 notes=0
"##,
        1,
    );
}

/// S1's loop over a packed dim: both fire, msb then lsb.
#[test]
fn o_pmd() {
    check(
        "o_pmd",
        r##"module top; logic n, k; logic [1:0][n:k] v;
  initial begin #1 $display("A b=%0d", $bits(v)); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:37: error[VITA-E3009] E-ELAB-UNSUPPORTED: a reference to net/variable `n` is not allowed in a constant range bound [in top]
t.sv:1:39: error[VITA-E3009] E-ELAB-UNSUPPORTED: a reference to net/variable `k` is not allowed in a constant range bound [in top]
errors=2 warnings=1 notes=0
"##,
        1,
    );
}

/// S3 an unpacked `[m:l]`: both fire, msb then lsb.
#[test]
fn o_unp() {
    check(
        "o_unp",
        r##"module top; logic n, k; logic v [n:k];
  initial begin #1 $display("A b=%0d", $size(v)); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:34: error[VITA-E3009] E-ELAB-UNSUPPORTED: a reference to net/variable `n` is not allowed in a constant range bound [in top]
t.sv:1:36: error[VITA-E3009] E-ELAB-UNSUPPORTED: a reference to net/variable `k` is not allowed in a constant range bound [in top]
errors=2 warnings=1 notes=0
"##,
        1,
    );
}

/// S2 a parameter range: both fire, msb then lsb (F-b: the text says `undefined name`).
#[test]
fn o_param() {
    check(
        "o_param",
        r##"module top; logic n, k; localparam logic [n:k] P = 0;
  initial begin #1 $display("A b=%0d", $bits(P)); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:43: error[VITA-E3009] E-ELAB-UNSUPPORTED: undefined name `n` is not allowed in a constant range bound [in top]
t.sv:1:45: error[VITA-E3009] E-ELAB-UNSUPPORTED: undefined name `k` is not allowed in a constant range bound [in top]
errors=2 warnings=1 notes=0
"##,
        1,
    );
}

/// route E at S1's loop: the msb is `x` at 16 bits.
#[test]
fn e_pmd16() {
    check(
        "e_pmd16",
        r##"module top; logic n, k; logic [1:0][(16'd1 / (16'hFFFF + 16'd1)) + 16'd2 : 0] v;
  initial begin #1 $display("A b=%0d", $bits(v)); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:37: error[VITA-E3009] E-ELAB-UNSUPPORTED: this range bound is `x`: the divisor of the `/` (the `+` operation) is 0 in its 16-bit context, so the `/` is `x` (IEEE 1800-2017 §11.4.3), and a range bound shall not contain x or z (IEEE 1800-2017 §6.9.1, §7.4.2) [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// route E at S2.
#[test]
fn e_param16() {
    check(
        "e_param16",
        r##"module top; logic n, k; localparam logic [(16'd1 / (16'hFFFF + 16'd1)) + 16'd2 : 0] P = 0;
  initial begin #1 $display("A b=%0d", $bits(P)); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:43: error[VITA-E3009] E-ELAB-UNSUPPORTED: this range bound is `x`: the divisor of the `/` (the `+` operation) is 0 in its 16-bit context, so the `/` is `x` (IEEE 1800-2017 §11.4.3), and a range bound shall not contain x or z (IEEE 1800-2017 §6.9.1, §7.4.2) [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// route E at S3.
#[test]
fn e_unp16() {
    check(
        "e_unp16",
        r##"module top; logic n, k; logic v [(16'd1 / (16'hFFFF + 16'd1)) + 16'd2 : 0];
  initial begin #1 $display("A b=%0d", $size(v)); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:34: error[VITA-E3009] E-ELAB-UNSUPPORTED: this range bound is `x`: the divisor of the `/` (the `+` operation) is 0 in its 16-bit context, so the `/` is `x` (IEEE 1800-2017 §11.4.3), and a range bound shall not contain x or z (IEEE 1800-2017 §6.9.1, §7.4.2) [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// route E on both bounds of a module net, msb then lsb.
#[test]
fn e2_net16() {
    check(
        "e2_net16",
        r##"module top; logic n, k; logic [(16'd1 / (16'hFFFF + 16'd1)) + 16'd2 : (16'd1 % (16'hFFFF + 16'd1)) + 16'd1] v;
  initial begin #1 $display("A b=%0d", $bits(v)); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:32: error[VITA-E3009] E-ELAB-UNSUPPORTED: this range bound is `x`: the divisor of the `/` (the `+` operation) is 0 in its 16-bit context, so the `/` is `x` (IEEE 1800-2017 §11.4.3), and a range bound shall not contain x or z (IEEE 1800-2017 §6.9.1, §7.4.2) [in top]
t.sv:1:71: error[VITA-E3009] E-ELAB-UNSUPPORTED: this range bound is `x`: the divisor of the `%` (the `+` operation) is 0 in its 16-bit context, so the `%` is `x` (IEEE 1800-2017 §11.4.3), and a range bound shall not contain x or z (IEEE 1800-2017 §6.9.1, §7.4.2) [in top]
errors=2 warnings=1 notes=0
"##,
        1,
    );
}

/// msb fires, lsb is route E: the lsb is folded and checked after a failed msb (mutant M8).
#[test]
fn o_mix_net16() {
    check(
        "o_mix_net16",
        r##"module top; logic n, k; logic [n:(16'd1 / (16'hFFFF + 16'd1)) + 16'd2] v;
  initial begin #1 $display("A b=%0d", $bits(v)); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:32: error[VITA-E3009] E-ELAB-UNSUPPORTED: a reference to net/variable `n` is not allowed in a constant range bound [in top]
t.sv:1:34: error[VITA-E3009] E-ELAB-UNSUPPORTED: this range bound is `x`: the divisor of the `/` (the `+` operation) is 0 in its 16-bit context, so the `/` is `x` (IEEE 1800-2017 §11.4.3), and a range bound shall not contain x or z (IEEE 1800-2017 §6.9.1, §7.4.2) [in top]
errors=2 warnings=1 notes=0
"##,
        1,
    );
}

/// the router, not IN: `[C+D:0]` over 4-bit C, D is one bit (mutant M6).
#[test]
fn m_selfdet() {
    check(
        "m_selfdet",
        r##"module top; localparam logic [3:0] C = 15, D = 1; logic [C+D:0] v;
  initial begin #1 $display("A b=%0d l=%0d r=%0d", $bits(v), $left(v), $right(v)); $finish; end endmodule
"##,
        r##"A b=1 l=0 r=0
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// a direction read: `$left` 3, `$right` 0 and bit 0 of a descending vector (mutant M7: the pair returned swapped).
#[test]
fn m7_dir() {
    check(
        "m7_dir",
        r##"module top; logic [3:0] v = 4'b0001; logic w [3:0];
  initial begin #1 $display("A b=%0d v0=%0d l=%0d r=%0d", $bits(v), v[0], $left(v), $right(v)); $finish; end endmodule
"##,
        r##"A b=4 v0=1 l=3 r=0
simulation ended (Finish) at time 1
"##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"##,
        0,
    );
}

/// msb-only fire: the lsb folds, the msb does not (mutant M4: the checks' values crossed).
#[test]
fn half_net() {
    check(
        "half_net",
        r##"module top; logic n; logic [n:0] v;
  initial begin #1 $display("A b=%0d", $bits(v)); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:29: error[VITA-E3009] E-ELAB-UNSUPPORTED: a reference to net/variable `n` is not allowed in a constant range bound [in top]
errors=1 warnings=1 notes=0
"##,
        1,
    );
}

/// S4 first (ports.rs: range_to_dims_opt before packed_extents): both fire on a port, msb then lsb.
#[test]
fn s4_port_nk() {
    check(
        "s4_port_nk",
        r##"module top(input logic [n:k] p);
  initial begin #1 $display("A b=%0d", $bits(p)); $finish; end endmodule
"##,
        r##""##,
        r##"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:25: error[VITA-E3009] E-ELAB-UNSUPPORTED: undefined name `n` is not allowed in a constant range bound [in top]
t.sv:1:27: error[VITA-E3009] E-ELAB-UNSUPPORTED: undefined name `k` is not allowed in a constant range bound [in top]
errors=2 warnings=1 notes=0
"##,
        1,
    );
}
