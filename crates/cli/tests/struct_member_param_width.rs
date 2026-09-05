//! §3 ⑤ ⓓ: a packed struct/union member whose width names a CONSTANT —
//! `logic [W-1:0]`, `logic [W:0]`, `[0:W-1]`, `[W+3:4]`, a typedef of one,
//! `$clog2(N)`, `W/2` — where `W` is a `localparam`, a package `parameter`
//! (IEEE §6.20.1), a body `parameter` of a module with an ANSI parameter header
//! (§6.20.1), or one of those reached through `import p::*` / `import p::W` /
//! `p::W`. The parser folds the member width through the same table the
//! constant generate-index fold uses (`const_locals`), so an OVERRIDABLE
//! `parameter` (a header parameter, a body parameter without a header) stays
//! loud — both oracles change the layout per instance for those — and every
//! declaration of the name (a port, a variable, a generate-local localparam)
//! shadows the constant.
//!
//! Every expected value is the census oracle line (iverilog 13.0 `-g2012` and
//! verilator 5.050 agree unless the comment says otherwise).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_stru_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// Every `DIGEST=` line, in emission order, joined by `|` (the census harness format).
fn digest(name: &str, src: &str, expect: &str) {
    let (out, rc) = run(src);
    assert_eq!(rc, Some(0), "{name}: expected exit 0, got {rc:?}:\n{out}");
    let v: Vec<&str> = out
        .lines()
        .filter_map(|l| l.strip_prefix("DIGEST="))
        .collect();
    assert_eq!(v.join("|"), expect, "{name}:\n{out}");
}

fn loud(name: &str, src: &str, needle: &str) {
    let (out, rc) = run(src);
    assert_ne!(rc, Some(0), "{name}: expected a loud reject:\n{out}");
    assert!(
        out.contains(needle),
        "{name}: expected `{needle}` in:\n{out}"
    );
}

#[test]
fn pw_localparam() {
    // pw_localparam: both oracles
    digest(
        "pw_localparam",
        r#"module tb;
  localparam int W = 6;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_localparam_after: verilator
    loud(
        "pw_localparam_after",
        r#"module tb;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  localparam int W = 6;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "expected struct member width must be a named integer type or a constan",
    );
    // pw_localparam_bit4: both oracles
    digest(
        "pw_localparam_bit4",
        r#"module tb;
  localparam logic [3:0] W = 6;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_localparam_byte: both oracles
    digest(
        "pw_localparam_byte",
        r#"module tb;
  localparam byte W = 6;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_localparam_clog2: both oracles
    digest(
        "pw_localparam_clog2",
        r#"module tb;
  localparam int B = 64;
  localparam int W = $clog2(B);
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_localparam_derived: both oracles
    digest(
        "pw_localparam_derived",
        r#"module tb;
  localparam int B = 3;
  localparam int W = B*2;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_localparam_div: both oracles
    digest(
        "pw_localparam_div",
        r#"module tb;
  localparam int B = 12;
  localparam int W = B/2;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_localparam_integer: both oracles
    digest(
        "pw_localparam_integer",
        r#"module tb;
  localparam integer W = 6;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_localparam_longint: both oracles
    digest(
        "pw_localparam_longint",
        r#"module tb;
  localparam longint W = 6;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_localparam_neg: both oracles
    digest(
        "pw_localparam_neg",
        r#"module tb;
  localparam int B = -2;
  localparam int W = B + 8;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_localparam_paren: both oracles
    digest(
        "pw_localparam_paren",
        r#"module tb;
  localparam int B = 2;
  localparam int W = (B+1)*2;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_localparam_shl: both oracles
    loud(
        "pw_localparam_shl",
        r#"module tb;
  localparam int W = 3 << 1;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "expected struct member width must be a named integer type or a constan",
    );
    // pw_localparam_sized: both oracles (§4.5.418: a lone based literal is the
    // parameter's value in the parse-time table; was a wording pin of the loud)
    digest(
        "pw_localparam_sized",
        r#"module tb;
  localparam int W = 8'd6;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_localparam_uint: both oracles
    digest(
        "pw_localparam_uint",
        r#"module tb;
  localparam int unsigned W = 6;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
    // pw_localparam_untyped: both oracles
    digest(
        "pw_localparam_untyped",
        r#"module tb;
  localparam W = 6;
  typedef struct packed { logic [W-1:0] a; logic [W:0] b; logic c; } s_t;
  s_t s;
  initial begin
    s = '0; s.a = '1; s.c = 1;
    $display("DIGEST=%0d %0d %h %b", $bits(s), $bits(s.a), s, s.c);
  end

  initial #5 $finish;
endmodule
"#,
        "14 6 3f01 1",
    );
}
