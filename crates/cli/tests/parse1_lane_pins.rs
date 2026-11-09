//! The whole stdout, stderr (diagnostic code, text, line:col) and exit code of the PARSE-1
//! lane cells whose output shows a lane of the parse-time fold or a kernel re-point (ROADMAP
//! §5.2 PARSE-1, PIN6): the parser's op sets (L2 / L3), the context fold's i32 cutoff (L9),
//! `const_param_fits`, and the `wrap_i128` / `decimal_i64` re-points that reach an output.
//! Every expected text was measured on the PRE binary (batch2 bea52457's base, `vita-post`
//! md5 56c997be; W/parse1/build/gen/pre_out) with `t.sv` as the argument, so positions print
//! as `t.sv:<line>:<col>`. Not here: g_p1 (a dev build panics there, E1) and the
//! PRE-EXISTING witnesses (a pin would hold a possibly wrong output; the lane holds them).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// (stdout, stderr, exit code) of `vita t.sv` in a temp dir of its own.
fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_parse1_lane_{}_{n}", std::process::id()));
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

/// L2 Parser4 lacks `%`: try_const_index declines, the member width is refused.
#[test]
fn a_mod() {
    check(
        "a_mod",
        r#"module top;
  localparam X = 7 % 3;
  typedef struct packed { logic [X:0] a; logic b; } t_t;
  initial begin
    $display("L w=%0d", $bits(t_t));
    $finish;
  end
endmodule
"#,
        r#""#,
        r#"t.sv:3:27: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or a constant-literal range in v1 (a `[E:0]` range over a header parameter is laid out per instance), found keyword 'initial'
errors=1 warnings=0 notes=0
"#,
        1,
    );
}

/// L2 Parser4 lacks `<<`: refused.
#[test]
fn a_shl() {
    check(
        "a_shl",
        r#"module top;
  localparam X = 1 << 2;
  typedef struct packed { logic [X:0] a; logic b; } t_t;
  initial begin
    $display("L w=%0d", $bits(t_t));
    $finish;
  end
endmodule
"#,
        r#""#,
        r#"t.sv:3:27: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or a constant-literal range in v1 (a `[E:0]` range over a header parameter is laid out per instance), found keyword 'initial'
errors=1 warnings=0 notes=0
"#,
        1,
    );
}

/// L2 / L3 on an enum label: Lit3 declines `/`, Parser4 folds it.
#[test]
fn a_enum_div() {
    check(
        "a_enum_div",
        r#"module top;
  typedef enum logic [7:0] {A = 8 / 2, B} e_t;
  e_t v;
  initial begin
    v = B;
    $display("L n=%s v=%0d", v.name(), v);
    $finish;
  end
endmodule
"#,
        r#"L n=B v=5
simulation ended (Finish) at time 0
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"#,
        0,
    );
}

/// L3 Lit3 lacks `/`: the enum base width does not fold, so the parser's range check is skipped and elaborate refuses (pair of b_base_mul).
#[test]
fn b_base_div() {
    check(
        "b_base_div",
        r#"module top;
  typedef enum logic [8/2-1:0] {A = 20, B} e_t;
  e_t v;
  initial begin
    v = B;
    $display("L n=%s v=%0d", v.name(), v);
    $finish;
  end
endmodule
"#,
        r#""#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
t.sv:1:8: error[VITA-E3009] E-ELAB-UNSUPPORTED: enum label `A` = 20 does not fit its 4-bit base type (§6.19); no reading of a 4-bit value reaches it (the widest range is [-8, 15]) [in top]
errors=1 warnings=1 notes=0
"#,
        1,
    );
}

/// L3 Lit3 folds `*`: the parser's own range check refuses (pair of b_base_div).
#[test]
fn b_base_mul() {
    check(
        "b_base_mul",
        r#"module top;
  typedef enum logic [2*2-1:0] {A = 20, B} e_t;
  e_t v;
  initial begin
    v = B;
    $display("L n=%s v=%0d", v.name(), v);
    $finish;
  end
endmodule
"#,
        r#""#,
        r#"t.sv:2:37: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected an enum label value that fits the enum base type, found identifier 'e_t'
t.sv:2:41: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected an enum label value that fits the enum base type, found identifier 'e_t'
errors=2 warnings=0 notes=0
"#,
        1,
    );
}

/// L9 cw_shape's i32 cutoff: the context fold declines a decimal past i32 and the i64 fold's answer is refused.
#[test]
fn c_big_i32() {
    check(
        "c_big_i32",
        r#"module top;
  typedef struct packed { logic [4294967296 + 7:0] a; logic b; } t_t;
  initial begin
    $display("L w=%0d", $bits(t_t));
    $finish;
  end
endmodule
"#,
        r#""#,
        r#"t.sv:2:27: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or a constant-literal range in v1 (a `[E:0]` range over a header parameter is laid out per instance), found keyword 'initial'
errors=1 warnings=0 notes=0
"#,
        1,
    );
}

/// const_param_fits: an unfoldable range fits (w = 0), so the constant is recorded.
#[test]
fn h_fits_unfold() {
    check(
        "h_fits_unfold",
        r#"module top;
  localparam int K = $countones(3);
  localparam logic [K:0] X = 3;
  typedef struct packed { logic [X:0] a; logic b; } t_t;
  initial begin
    $display("L w=%0d", $bits(t_t));
    $finish;
  end
endmodule
"#,
        r#"L w=5
simulation ended (Finish) at time 0
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"#,
        0,
    );
}

/// B05 -> wrap_i128: a signed based parameter sign-fills (`4'sb1110` = -2).
#[test]
fn k7_bl_neg() {
    check(
        "k7_bl_neg",
        r#"module top;
  localparam P = 4'sb1110;
  typedef struct packed { logic [P+5:0] a; logic b; } t_t;
  initial begin
    $display("L w=%0d", $bits(t_t));
    $finish;
  end
endmodule
"#,
        r#"L w=5
simulation ended (Finish) at time 0
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"#,
        0,
    );
}

/// B05 -> wrap_i128: an unsigned 64-bit pattern past i64::MAX declines.
#[test]
fn k7_bl_64u() {
    check(
        "k7_bl_64u",
        r#"module top;
  localparam P = 64'hFFFF_FFFF_FFFF_FFFB;
  typedef struct packed { logic [P+9:0] a; logic b; } t_t;
  initial begin
    $display("L w=%0d", $bits(t_t));
    $finish;
  end
endmodule
"#,
        r#""#,
        r#"t.sv:3:27: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected struct member width must be a named integer type or a constant-literal range in v1 (a `[E:0]` range over a header parameter is laid out per instance), found keyword 'initial'
errors=1 warnings=0 notes=0
"#,
        1,
    );
}

/// B06 -> wrap_i128: `byte B = 200` (a byte pin of a correct output; `L b=` is make_const_i64's 8-bit sign fill, not B06's killer).
#[test]
fn coerce_byte() {
    check(
        "coerce_byte",
        r#"module top;
  localparam byte B = 200;
  initial begin
    $display("L b=%0d", B);
    $finish;
  end
endmodule
"#,
        r#"L b=-56
simulation ended (Finish) at time 0
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"#,
        0,
    );
}

/// B07 -> wrap_i128: a 4-bit signed function return of 13 is -3.
#[test]
fn coerce_ret() {
    check(
        "coerce_ret",
        r#"module top;
  function automatic logic signed [3:0] f(input int a);
    return a;
  endfunction
  localparam int R = f(13);
  initial begin
    $display("L r=%0d", R);
    $finish;
  end
endmodule
"#,
        r#"L r=-3
simulation ended (Finish) at time 0
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"#,
        0,
    );
}

/// A05 -> decimal_i64: a based literal's size `1_6` reads 16.
#[test]
fn k8_size_us() {
    check(
        "k8_size_us",
        r#"module top;
  typedef struct packed { logic [1_6'd5:0] a; logic b; } t_t;
  initial begin
    $display("L w=%0d", $bits(t_t));
    $finish;
  end
endmodule
"#,
        r#"L w=7
simulation ended (Finish) at time 0
"#,
        r#"warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT: no `timescale in the design; assuming the 1ns/1ns base
errors=0 warnings=1 notes=0
"#,
        0,
    );
}
