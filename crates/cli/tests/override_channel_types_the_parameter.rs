//! An override binds its OWN type (IEEE 1800 §6.20.2) on every channel and for every
//! operand shape the parent can type — ROADMAP §2 row 25, whose headline literal cells
//! (`#(.P(32'hF0F0F0F0))` onto `parameter P = 5`, `#(.Q(32'hDEADBEEF))` onto `parameter
//! Q = 8'sd1`) already bound both oracles' answer at HEAD. What was still open, all in the
//! same binder (`params.rs::bind_one_param` and the channels that feed it):
//!
//! 1. A `defparam` refused every shape the i64 fold declines — a literal wider than 64
//!    bits, `$signed(…)` / `$unsigned(…)`, a string — while `#()` bound it through the wide
//!    or the string channel. The defparam record now carries all four channels; one
//!    carrying an x/z bit keeps the refusal (the `#()` twin drops the plane, §2 row 15).
//! 2. The operator channel (`override_self_meta`) certified bare names only, so an operator
//!    over a SELECT (`~W8[3:0]`, `W8[3:0] >> 1`) fell back to the DEFAULT literal's type:
//!    32 signed bits `fffffffa` where both oracles bind 4 bits `a`. A select of a certified
//!    declared-width name is now a certified leaf (its width is structural, §11.5.1).
//! 3. Every overridden untyped parameter was marked a GUESSED type, so the size-cast
//!    classifier sent its casts down the pre-slice route even when an override channel had
//!    typed it: `64'(-P)` over `#(.P(32'hF0F0F0F0))` was `000000000f0f0f10` where both
//!    oracles print `ffffffff0f0f0f10`. An override a channel typed is no longer a guess,
//!    except a signed one onto a declaration with no `signed` keyword (see the last tests).
//! 4. `#(.P(A[0][3:0]))` (a select of an array-parameter element) was E3009 on `#()` while
//!    the `defparam` of the same text bound it; the wide channel types it, so the refusal
//!    fires only when that channel declines.
//!
//! Oracles: iverilog 13.0 `-g2012` and verilator 5.052 `--binary --timing`. Where they
//! split on a `+` / `-` / `*` width (iverilog one bit wider — its §4.5.466
//! self-contradiction) or on a fill (`'1`: iverilog 1 bit, verilator the default's width)
//! the pinned value is vita's, which is verilator's for the operators — the answer vita's
//! own `localparam` lane gives for the same text. iverilog has no unpacked-array
//! parameters, so the element cells are verilator's.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_any(src: &str) -> (bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_octp_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), s)
}

/// The sorted `top.…` / `T …` lines of a run that must exit 0.
fn lines(src: &str) -> Vec<String> {
    let (ok, s) = run_any(src);
    assert!(ok, "expected exit 0, got:\n{s}");
    let mut v: Vec<String> = s
        .lines()
        .filter(|l| l.starts_with("top.") || l.starts_with("T "))
        .map(|l| l.trim().to_string())
        .collect();
    v.sort();
    v
}

fn check(src: &str, want: &[&str]) {
    let mut w: Vec<String> = want.iter().map(|s| s.to_string()).collect();
    w.sort();
    assert_eq!(lines(src), w);
}

fn loud(src: &str, needle: &str) {
    let (ok, s) = run_any(src);
    assert!(
        !ok && s.contains(needle),
        "expected a rejection naming {needle:?}, got:\n{s}"
    );
}

/// Sixteen override shapes by `defparam` onto an untyped, a sign-keyword and a ranged parameter. PRE: the whole design was E3009 (`defparam: a non-constant override value is unsupported`) on the 65-bit and 128-bit literals and on `$signed(4'hA)`; every other line was already this. Splits pinned on vita's side: `'1` (iverilog 1 bit), `3000000000` (verilator 32 bits), `4'hA + 4'h1` (iverilog 5 bits), `~4'h5` onto `[7:0]` (verilator `0a`, row 16).
#[test]
fn a_defparam_binds_every_shape_the_named_override_binds() {
    check(
        r#"module su #(parameter P = 5) ();
  initial begin #1; $display("%m bits=%0d hex=%h dec=%0d neg=%0d", $bits(P), P, P, P < 0); end
endmodule
module top;
  su u00();
  defparam u00.P = 32'hF0F0F0F0;
  su u01();
  defparam u01.P = 4'hA;
  su u02();
  defparam u02.P = 16'shFFFF;
  su u03();
  defparam u03.P = 33'h1_0000_0003;
  su u04();
  defparam u04.P = 65'h1_0000_0000_0000_0003;
  su u05();
  defparam u05.P = -1;
  su u06();
  defparam u06.P = '1;
  su u07();
  defparam u07.P = 2'b10;
  su u08();
  defparam u08.P = 3000000000;
  su u09();
  defparam u09.P = 'hFF;
  su u10();
  defparam u10.P = 'shFF;
  su u11();
  defparam u11.P = -8'sd3;
  su u12();
  defparam u12.P = 128'hF0F0_0000_0000_0000_0000_0000_0000_0001;
  su u13();
  defparam u13.P = ~4'h5;
  su u14();
  defparam u14.P = 4'hA + 4'h1;
  su u15();
  defparam u15.P = $signed(4'hA);
  initial begin #2 $finish; end
endmodule
"#,
        &[
            r#"top.u00 bits=32 hex=f0f0f0f0 dec=4042322160 neg=0"#,
            r#"top.u01 bits=4 hex=a dec=10 neg=0"#,
            r#"top.u02 bits=16 hex=ffff dec=-1 neg=1"#,
            r#"top.u03 bits=33 hex=100000003 dec=4294967299 neg=0"#,
            r#"top.u04 bits=65 hex=10000000000000003 dec=18446744073709551619 neg=0"#,
            r#"top.u05 bits=32 hex=ffffffff dec=-1 neg=1"#,
            r#"top.u06 bits=1 hex=1 dec=1 neg=0"#,
            r#"top.u07 bits=2 hex=2 dec=2 neg=0"#,
            r#"top.u08 bits=33 hex=0b2d05e00 dec=3000000000 neg=0"#,
            r#"top.u09 bits=32 hex=000000ff dec=255 neg=0"#,
            r#"top.u10 bits=32 hex=000000ff dec=255 neg=0"#,
            r#"top.u11 bits=8 hex=fd dec=-3 neg=1"#,
            r#"top.u12 bits=128 hex=f0f00000000000000000000000000001 dec=320260870234428168127761013586295521281 neg=0"#,
            r#"top.u13 bits=4 hex=a dec=10 neg=0"#,
            r#"top.u14 bits=4 hex=b dec=11 neg=0"#,
            r#"top.u15 bits=4 hex=a dec=-6 neg=1"#,
        ],
    );
    check(
        r#"module su #(parameter signed P = 1) ();
  initial begin #1; $display("%m bits=%0d hex=%h dec=%0d neg=%0d", $bits(P), P, P, P < 0); end
endmodule
module top;
  su u00();
  defparam u00.P = 32'hF0F0F0F0;
  su u01();
  defparam u01.P = 4'hA;
  su u02();
  defparam u02.P = 16'shFFFF;
  su u03();
  defparam u03.P = 33'h1_0000_0003;
  su u04();
  defparam u04.P = 65'h1_0000_0000_0000_0003;
  su u05();
  defparam u05.P = -1;
  su u06();
  defparam u06.P = '1;
  su u07();
  defparam u07.P = 2'b10;
  su u08();
  defparam u08.P = 3000000000;
  su u09();
  defparam u09.P = 'hFF;
  su u10();
  defparam u10.P = 'shFF;
  su u11();
  defparam u11.P = -8'sd3;
  su u12();
  defparam u12.P = 128'hF0F0_0000_0000_0000_0000_0000_0000_0001;
  su u13();
  defparam u13.P = ~4'h5;
  su u14();
  defparam u14.P = 4'hA + 4'h1;
  su u15();
  defparam u15.P = $signed(4'hA);
  initial begin #2 $finish; end
endmodule
"#,
        &[
            r#"top.u00 bits=32 hex=f0f0f0f0 dec=-252645136 neg=1"#,
            r#"top.u01 bits=4 hex=a dec=-6 neg=1"#,
            r#"top.u02 bits=16 hex=ffff dec=-1 neg=1"#,
            r#"top.u03 bits=33 hex=100000003 dec=-4294967293 neg=1"#,
            r#"top.u04 bits=65 hex=10000000000000003 dec=-18446744073709551613 neg=1"#,
            r#"top.u05 bits=32 hex=ffffffff dec=-1 neg=1"#,
            r#"top.u06 bits=1 hex=1 dec=1 neg=0"#,
            r#"top.u07 bits=2 hex=2 dec=-2 neg=1"#,
            r#"top.u08 bits=33 hex=0b2d05e00 dec=3000000000 neg=0"#,
            r#"top.u09 bits=32 hex=000000ff dec=255 neg=0"#,
            r#"top.u10 bits=32 hex=000000ff dec=255 neg=0"#,
            r#"top.u11 bits=8 hex=fd dec=-3 neg=1"#,
            r#"top.u12 bits=128 hex=f0f00000000000000000000000000001 dec=-20021496686510295335613593845472690175 neg=1"#,
            r#"top.u13 bits=4 hex=a dec=-6 neg=1"#,
            r#"top.u14 bits=4 hex=b dec=-5 neg=1"#,
            r#"top.u15 bits=4 hex=a dec=-6 neg=1"#,
        ],
    );
    check(
        r#"module su #(parameter [7:0] P = 1) ();
  initial begin #1; $display("%m bits=%0d hex=%h dec=%0d neg=%0d", $bits(P), P, P, P < 0); end
endmodule
module top;
  su u00();
  defparam u00.P = 32'hF0F0F0F0;
  su u01();
  defparam u01.P = 4'hA;
  su u02();
  defparam u02.P = 16'shFFFF;
  su u03();
  defparam u03.P = 33'h1_0000_0003;
  su u04();
  defparam u04.P = 65'h1_0000_0000_0000_0003;
  su u05();
  defparam u05.P = -1;
  su u06();
  defparam u06.P = '1;
  su u07();
  defparam u07.P = 2'b10;
  su u08();
  defparam u08.P = 3000000000;
  su u09();
  defparam u09.P = 'hFF;
  su u10();
  defparam u10.P = 'shFF;
  su u11();
  defparam u11.P = -8'sd3;
  su u12();
  defparam u12.P = 128'hF0F0_0000_0000_0000_0000_0000_0000_0001;
  su u13();
  defparam u13.P = ~4'h5;
  su u14();
  defparam u14.P = 4'hA + 4'h1;
  su u15();
  defparam u15.P = $signed(4'hA);
  initial begin #2 $finish; end
endmodule
"#,
        &[
            r#"top.u00 bits=8 hex=f0 dec=240 neg=0"#,
            r#"top.u01 bits=8 hex=0a dec=10 neg=0"#,
            r#"top.u02 bits=8 hex=ff dec=255 neg=0"#,
            r#"top.u03 bits=8 hex=03 dec=3 neg=0"#,
            r#"top.u04 bits=8 hex=03 dec=3 neg=0"#,
            r#"top.u05 bits=8 hex=ff dec=255 neg=0"#,
            r#"top.u06 bits=8 hex=ff dec=255 neg=0"#,
            r#"top.u07 bits=8 hex=02 dec=2 neg=0"#,
            r#"top.u08 bits=8 hex=00 dec=0 neg=0"#,
            r#"top.u09 bits=8 hex=ff dec=255 neg=0"#,
            r#"top.u10 bits=8 hex=ff dec=255 neg=0"#,
            r#"top.u11 bits=8 hex=fd dec=253 neg=0"#,
            r#"top.u12 bits=8 hex=01 dec=1 neg=0"#,
            r#"top.u13 bits=8 hex=fa dec=250 neg=0"#,
            r#"top.u14 bits=8 hex=0b dec=11 neg=0"#,
            r#"top.u15 bits=8 hex=fa dec=250 neg=0"#,
        ],
    );
}

/// The string channel: a `defparam` of `"str"`, `{"a","b"}`, `""` and a ten-character string onto an untyped parameter binds what `#()` binds (both oracles, except `""`, which both oracles bind at 8 bits — ROADMAP §2 "Constant domain", the string-literal width bullet). PRE: E3009. A string onto a RANGED parameter stays E3009 on both channels (both oracles bind the truncated bits; pre-existing, ROADMAP §3.b `str-override-ranged`).
#[test]
fn a_string_defparam_binds_like_the_named_override() {
    check(
        r#"module su #(parameter P = 5) ();
  initial begin #1; $display("%m bits=%0d hex=%h s=%s", $bits(P), P, P); end
endmodule
module top;
  su u00();
  defparam u00.P = "str";
  su u01();
  defparam u01.P = {"a","b"};
  su u02();
  defparam u02.P = "";
  su u03();
  defparam u03.P = "abcdefghij";
  initial begin #2 $finish; end
endmodule
"#,
        &[
            r#"top.u00 bits=24 hex=737472 s=str"#,
            r#"top.u01 bits=16 hex=6162 s=ab"#,
            r#"top.u02 bits=1 hex=0 s="#,
            r#"top.u03 bits=80 hex=6162636465666768696a s=abcdefghij"#,
        ],
    );
    check(
        r#"module su #(parameter P = "ab") ();
  initial begin #1; $display("%m bits=%0d hex=%h s=%s", $bits(P), P, P); end
endmodule
module top;
  su u00();
  defparam u00.P = "str";
  su u01();
  defparam u01.P = {"a","b"};
  su u02();
  defparam u02.P = "";
  su u03();
  defparam u03.P = "abcdefghij";
  initial begin #2 $finish; end
endmodule
"#,
        &[
            r#"top.u00 bits=24 hex=737472 s=str"#,
            r#"top.u01 bits=16 hex=6162 s=ab"#,
            r#"top.u02 bits=1 hex=0 s="#,
            r#"top.u03 bits=80 hex=6162636465666768696a s=abcdefghij"#,
        ],
    );
    loud(
        r#"module su #(parameter [23:0] P = 0) ();
  initial begin #1; $display("%m bits=%0d hex=%h s=%s", $bits(P), P, P); end
endmodule
module top;
  su u00();
  defparam u00.P = "str";
  su u01();
  defparam u01.P = {"a","b"};
  su u02();
  defparam u02.P = "";
  su u03();
  defparam u03.P = "abcdefghij";
  initial begin #2 $finish; end
endmodule
"#,
        "VITA-E3009",
    );
}

/// An x/z literal (`8'b1010_010x`, `128'hx`, `128'h1x`, `8'bzzzzz1z0`) keeps the refusal: the `#()` twin binds it with the unknown plane dropped (`a4` where both oracles keep `aX` — §2 row 15), and widening the defparam channel must not carry it there. A signal is not a constant.
#[test]
fn a_defparam_carrying_an_unknown_bit_or_a_signal_is_still_refused() {
    loud(
        r#"module su #(parameter logic [7:0] P = 0) ();
  initial begin #1; $display("T bits=%0d hex=%h", $bits(P), P); end
endmodule
module top;
  su u();
  defparam u.P = 8'b1010_010x;
  initial begin #2 $finish; end
endmodule
"#,
        "defparam: a non-constant override value is unsupported",
    );
    loud(
        r#"module su #(parameter P = 5) ();
  initial begin #1; $display("T bits=%0d hex=%h", $bits(P), P); end
endmodule
module top;
  su u();
  defparam u.P = 128'hx;
  initial begin #2 $finish; end
endmodule
"#,
        "defparam: a non-constant override value is unsupported",
    );
    loud(
        r#"module su #(parameter logic [127:0] P = 0) ();
  initial begin #1; $display("T bits=%0d hex=%h", $bits(P), P); end
endmodule
module top;
  su u();
  defparam u.P = 128'h1x;
  initial begin #2 $finish; end
endmodule
"#,
        "defparam: a non-constant override value is unsupported",
    );
    loud(
        r#"module su #(parameter logic [7:0] P = 0) ();
  initial begin #1; $display("T bits=%0d hex=%h", $bits(P), P); end
endmodule
module top;
  su u();
  defparam u.P = 8'bzzzzz1z0;
  initial begin #2 $finish; end
endmodule
"#,
        "defparam: a non-constant override value is unsupported",
    );
    loud(
        r#"module su #(parameter P = 5) ();
  initial begin #1; $display("T b01 bits=%0d hex=%h dec=%0d", $bits(P), P, P); end
endmodule
module top;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  logic [3:0] sig = 4'd9;
  su u();
  defparam u.P = sig;
  initial begin #2 $finish; end
endmodule
"#,
        "defparam: a non-constant override value is unsupported",
    );
}

/// `override_self_meta` over a SELECT operand. Two oracles: `~W8[3:0]` 4 bits `a` (PRE 32 bits `fffffffa`), `-W8[3:0]` 4 bits `b` (PRE `fffffffb`), `W8[3:0] >> 1` 4 bits `2` (PRE 32 bits). The rest land on verilator's width, which vita's `localparam` lane already gave for the same text (iverilog one bit wider on `+`, `-`, `*`); PRE was 32 bits on every one.
#[test]
fn an_operator_over_a_select_of_a_declared_name_binds_its_own_type() {
    check(
        r#"module su #(parameter P = 5) ();
  initial begin #1; $display("%m bits=%0d hex=%h dec=%0d neg=%0d", $bits(P), P, P, P < 0); end
endmodule
module top;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  su #(.P(~W8[3:0])) u();
  initial begin #2 $finish; end
endmodule
"#,
        &[r#"top.u bits=4 hex=a dec=10 neg=0"#],
    );
    check(
        r#"module su #(parameter P = 5) ();
  initial begin #1; $display("%m bits=%0d hex=%h dec=%0d neg=%0d", $bits(P), P, P, P < 0); end
endmodule
module top;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  su #(.P(-W8[3:0])) u();
  initial begin #2 $finish; end
endmodule
"#,
        &[r#"top.u bits=4 hex=b dec=11 neg=0"#],
    );
    check(
        r#"module su #(parameter P = 5) ();
  initial begin #1; $display("%m bits=%0d hex=%h dec=%0d neg=%0d", $bits(P), P, P, P < 0); end
endmodule
module top;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  su #(.P(W8[3:0] * 4'd3)) u();
  initial begin #2 $finish; end
endmodule
"#,
        &[r#"top.u bits=4 hex=f dec=15 neg=0"#],
    );
    check(
        r#"module su #(parameter P = 5) ();
  initial begin #1; $display("%m bits=%0d hex=%h dec=%0d neg=%0d", $bits(P), P, P, P < 0); end
endmodule
module top;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  su #(.P(W8[3:0] - 4'd5)) u();
  initial begin #2 $finish; end
endmodule
"#,
        &[r#"top.u bits=4 hex=0 dec=0 neg=0"#],
    );
    check(
        r#"module su #(parameter P = 5) ();
  initial begin #1; $display("%m bits=%0d hex=%h dec=%0d neg=%0d", $bits(P), P, P, P < 0); end
endmodule
module top;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  su #(.P(W8[7:4] + 4'd1)) u();
  initial begin #2 $finish; end
endmodule
"#,
        &[r#"top.u bits=4 hex=b dec=11 neg=0"#],
    );
    check(
        r#"module su #(parameter P = 5) ();
  initial begin #1; $display("%m bits=%0d hex=%h dec=%0d neg=%0d", $bits(P), P, P, P < 0); end
endmodule
module top;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  su #(.P(W8[0] + 1'b1)) u();
  initial begin #2 $finish; end
endmodule
"#,
        &[r#"top.u bits=1 hex=0 dec=0 neg=0"#],
    );
    check(
        r#"module su #(parameter P = 5) ();
  initial begin #1; $display("%m bits=%0d hex=%h dec=%0d neg=%0d", $bits(P), P, P, P < 0); end
endmodule
module top;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  su #(.P(S8[3:0] - 4'd5)) u();
  initial begin #2 $finish; end
endmodule
"#,
        &[r#"top.u bits=4 hex=8 dec=8 neg=0"#],
    );
    check(
        r#"module su #(parameter P = 5) ();
  initial begin #1; $display("%m bits=%0d hex=%h dec=%0d neg=%0d", $bits(P), P, P, P < 0); end
endmodule
module top;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  su #(.P({W8[3:0], 4'h0} + 8'd1)) u();
  initial begin #2 $finish; end
endmodule
"#,
        &[r#"top.u bits=8 hex=51 dec=81 neg=0"#],
    );
    check(
        r#"module su #(parameter P = 5) ();
  initial begin #1; $display("%m bits=%0d hex=%h dec=%0d neg=%0d", $bits(P), P, P, P < 0); end
endmodule
module top;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  su #(.P(W8[3:0] >> 1)) u();
  initial begin #2 $finish; end
endmodule
"#,
        &[r#"top.u bits=4 hex=2 dec=2 neg=0"#],
    );
    check(
        r#"module su #(parameter P = 5) ();
  initial begin #1; $display("%m bits=%0d hex=%h dec=%0d neg=%0d", $bits(P), P, P, P < 0); end
endmodule
module top;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  su #(.P(W8[2 +: 3] - 3'd1)) u();
  initial begin #2 $finish; end
endmodule
"#,
        &[r#"top.u bits=3 hex=0 dec=0 neg=0"#],
    );
    check(
        r#"module su #(parameter P = 5) ();
  initial begin #1; $display("%m bits=%0d hex=%h dec=%0d neg=%0d", $bits(P), P, P, P < 0); end
endmodule
module top;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  su #(.P(W8[3:0] - 4'd5 + 8'd0)) u();
  initial begin #2 $finish; end
endmodule
"#,
        &[r#"top.u bits=8 hex=00 dec=0 neg=0"#],
    );
}

/// The certified-width lane the wide domain resizes against (`param_decl_width_declared`) now types `localparam L = ~W8[3:0]`, so a 128-bit declaration over `L` folds. Both oracles on `X`: `{L, 124'd0}` and `L * 128'h1_0000_0000_0000_0000` were E3009, `~L` was `0000000000000000fffffffffffffff4` (both `ffff…f4`), `L << 124` was E3009 (`L`'s own width is a split: iverilog 5, verilator 4).
#[test]
fn a_localparam_over_a_select_typed_constant_resizes_at_its_declared_width() {
    check(
        r#"module top;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  parameter logic [127:0] WW = 128'hF0F0_0000_0000_0000_0000_0000_0000_0005;
  localparam L = ~W8[3:0];
  localparam logic [127:0] X = {L, 124'd0};
  initial begin #1; $display("T bits=%0d hex=%h Lbits=%0d L=%h", $bits(X), X, $bits(L), L); $finish; end
endmodule
"#,
        &[r#"T bits=128 hex=a0000000000000000000000000000000 Lbits=4 L=a"#],
    );
    check(
        r#"module top;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  parameter logic [127:0] WW = 128'hF0F0_0000_0000_0000_0000_0000_0000_0005;
  localparam L = ~W8[3:0];
  localparam logic [127:0] X = L * 128'h1_0000_0000_0000_0000;
  initial begin #1; $display("T bits=%0d hex=%h Lbits=%0d L=%h", $bits(X), X, $bits(L), L); $finish; end
endmodule
"#,
        &[r#"T bits=128 hex=000000000000000a0000000000000000 Lbits=4 L=a"#],
    );
    check(
        r#"module top;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  parameter logic [127:0] WW = 128'hF0F0_0000_0000_0000_0000_0000_0000_0005;
  localparam L = W8[7:4] + 4'd1;
  localparam logic [127:0] X = ~L;
  initial begin #1; $display("T bits=%0d hex=%h Lbits=%0d L=%h", $bits(X), X, $bits(L), L); $finish; end
endmodule
"#,
        &[r#"T bits=128 hex=fffffffffffffffffffffffffffffff4 Lbits=4 L=b"#],
    );
    check(
        r#"module top;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  parameter logic [127:0] WW = 128'hF0F0_0000_0000_0000_0000_0000_0000_0005;
  localparam L = W8[3:0] - 4'd6;
  localparam logic [127:0] X = L << 124;
  initial begin #1; $display("T bits=%0d hex=%h Lbits=%0d L=%h", $bits(X), X, $bits(L), L); $finish; end
endmodule
"#,
        &[r#"T bits=128 hex=f0000000000000000000000000000000 Lbits=4 L=f"#],
    );
}

/// Nine casts of an untyped `parameter P = 5` (with a header sibling `Q = P + 1` and a body
/// `R = P - 2`) overridden by a value an override channel TYPES: `32'hF0F0F0F0`, `~8'h5A`,
/// `W8`, `W8[3:0] - 4'd5` (both channels for the first), a `signed`-keyword declaration
/// overridden by `8'hA5` / `~8'h5A`, and an untyped parent name `UP` (bare and in `UP -
/// 8'd200`). PRE marked every one a guessed type and sent its casts down the pre-slice route:
/// `64'(-P)` over `32'hF0F0F0F0` was `000000000f0f0f10` (both oracles `ffffffff0f0f0f10`),
/// `64'(P >> 1)` on the `signed` keyword was `0000000000000052` (both `7fffffffffffffd2`).
/// The bare `64'(P)` column is a split (iverilog prints the parameter's own width) and the
/// `8'hFF + 8'd1`-style cells land on verilator's width.
#[test]
fn casts_of_an_override_typed_parameter_take_the_classified_route() {
    check(
        r#"module c #(parameter P = 5, parameter Q = P + 1) ();
  localparam R = P - 2;
  initial $display("T %h %h %h %h %h %h %h %h %h", 64'(~(P + 32'd1)), 64'(-P), 8'(P ** 2), 64'(P >> 1), 64'(P), 64'($signed(P)), 64'(P + 1'b1), 64'(Q >> 1), 64'(R - 1));
endmodule
module t;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  c #(.P(32'hF0F0F0F0)) u();
  initial #1 $finish;
endmodule
"#,
        &[
            r#"T ffffffff0f0f0f0e ffffffff0f0f0f10 00 0000000078787878 00000000f0f0f0f0 fffffffff0f0f0f0 00000000f0f0f0f1 0000000078787878 00000000f0f0f0ed"#,
        ],
    );
    check(
        r#"module c #(parameter P = 5, parameter Q = P + 1) ();
  localparam R = P - 2;
  initial $display("T %h %h %h %h %h %h %h %h %h", 64'(~(P + 32'd1)), 64'(-P), 8'(P ** 2), 64'(P >> 1), 64'(P), 64'($signed(P)), 64'(P + 1'b1), 64'(Q >> 1), 64'(R - 1));
endmodule
module t;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  c #(.P(~8'h5A)) u();
  initial #1 $finish;
endmodule
"#,
        &[
            r#"T ffffffffffffff59 ffffffffffffff5b 59 0000000000000052 00000000000000a5 ffffffffffffffa5 00000000000000a6 0000000000000053 00000000000000a2"#,
        ],
    );
    check(
        r#"module c #(parameter P = 5, parameter Q = P + 1) ();
  localparam R = P - 2;
  initial $display("T %h %h %h %h %h %h %h %h %h", 64'(~(P + 32'd1)), 64'(-P), 8'(P ** 2), 64'(P >> 1), 64'(P), 64'($signed(P)), 64'(P + 1'b1), 64'(Q >> 1), 64'(R - 1));
endmodule
module t;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  c #(.P(W8)) u();
  initial #1 $finish;
endmodule
"#,
        &[
            r#"T ffffffffffffff59 ffffffffffffff5b 59 0000000000000052 00000000000000a5 ffffffffffffffa5 00000000000000a6 0000000000000053 00000000000000a2"#,
        ],
    );
    check(
        r#"module c #(parameter P = 5, parameter Q = P + 1) ();
  localparam R = P - 2;
  initial $display("T %h %h %h %h %h %h %h %h %h", 64'(~(P + 32'd1)), 64'(-P), 8'(P ** 2), 64'(P >> 1), 64'(P), 64'($signed(P)), 64'(P + 1'b1), 64'(Q >> 1), 64'(R - 1));
endmodule
module t;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  c #(.P(W8[3:0] - 4'd5)) u();
  initial #1 $finish;
endmodule
"#,
        &[
            r#"T fffffffffffffffe 0000000000000000 00 0000000000000000 0000000000000000 0000000000000000 0000000000000001 0000000000000000 00000000fffffffd"#,
        ],
    );
    check(
        r#"module c #(parameter P = 5, parameter Q = P + 1) ();
  localparam R = P - 2;
  initial $display("T %h %h %h %h %h %h %h %h %h", 64'(~(P + 32'd1)), 64'(-P), 8'(P ** 2), 64'(P >> 1), 64'(P), 64'($signed(P)), 64'(P + 1'b1), 64'(Q >> 1), 64'(R - 1));
endmodule
module t;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  c u();
  defparam u.P = 32'hF0F0F0F0;
  initial #1 $finish;
endmodule
"#,
        &[
            r#"T ffffffff0f0f0f0e ffffffff0f0f0f10 00 0000000078787878 00000000f0f0f0f0 fffffffff0f0f0f0 00000000f0f0f0f1 0000000078787878 00000000f0f0f0ed"#,
        ],
    );
    check(
        r#"module c #(parameter signed P = 1) ();
  initial $display("T %0d %h %h %h %h %h", $bits(P), 64'(P), 64'(P >> 1), 64'(-P), 64'(~(P + 32'd1)), 64'($unsigned(P)));
endmodule
module t;
  parameter UP = 8'hA5;
  c #(.P(8'hA5)) u();
  initial #1 $finish;
endmodule
"#,
        &[
            r#"T 8 ffffffffffffffa5 7fffffffffffffd2 000000000000005b ffffffffffffff59 00000000000000a5"#,
        ],
    );
    check(
        r#"module c #(parameter signed P = 1) ();
  initial $display("T %0d %h %h %h %h %h", $bits(P), 64'(P), 64'(P >> 1), 64'(-P), 64'(~(P + 32'd1)), 64'($unsigned(P)));
endmodule
module t;
  parameter UP = 8'hA5;
  c #(.P(~8'h5A)) u();
  initial #1 $finish;
endmodule
"#,
        &[
            r#"T 8 ffffffffffffffa5 7fffffffffffffd2 000000000000005b ffffffffffffff59 00000000000000a5"#,
        ],
    );
    check(
        r#"module c #(parameter P = 5) ();
  initial $display("T %0d %h %h %h %h %h", $bits(P), 64'(P), 64'(P >> 1), 64'(-P), 64'(~(P + 32'd1)), 64'($unsigned(P)));
endmodule
module t;
  parameter UP = 8'hA5;
  c #(.P(UP - 8'd200)) u();
  initial #1 $finish;
endmodule
"#,
        &[
            r#"T 8 00000000000000dd 000000000000006e ffffffffffffff23 ffffffffffffff21 00000000000000dd"#,
        ],
    );
    check(
        r#"module c #(parameter P = 5) ();
  initial $display("T %0d %h %h %h %h %h", $bits(P), 64'(P), 64'(P >> 1), 64'(-P), 64'(~(P + 32'd1)), 64'($unsigned(P)));
endmodule
module t;
  parameter UP = 8'hA5;
  c #(.P(UP)) u();
  initial #1 $finish;
endmodule
"#,
        &[
            r#"T 8 00000000000000a5 0000000000000052 ffffffffffffff5b ffffffffffffff59 00000000000000a5"#,
        ],
    );
}

/// NOT this slice, pinned as measured: a SIGNED override onto a declaration with no `signed`
/// keyword stays a guessed type, because `ast::ParamDecl.signed` is false both for no keyword
/// and for the `unsigned` keyword (ROADMAP §2 "Index sealing", the `parameter unsigned`
/// bullet). So `#(.P(-3))`, `#(.P(-8'sd3))` and `#(.P(5))` keep the pre-slice casts:
/// `64'(~(P + 32'd1))` over `-3` is `0000000000000001` here where both oracles print
/// `ffffffff00000001`. Un-guessing them fixed those and, on `parameter unsigned P = 1` +
/// `#(.P(-8'sd91))` (the last `check`, which must not move), turned the right `64'(P >> 1)`
/// = `0000000000000052` into `7fffffffffffffd2`.
#[test]
fn a_signed_override_onto_a_keywordless_declaration_stays_a_guess() {
    check(
        r#"module c #(parameter P = 5, parameter Q = P + 1) ();
  localparam R = P - 2;
  initial $display("T %h %h %h %h %h %h %h %h %h", 64'(~(P + 32'd1)), 64'(-P), 8'(P ** 2), 64'(P >> 1), 64'(P), 64'($signed(P)), 64'(P + 1'b1), 64'(Q >> 1), 64'(R - 1));
endmodule
module t;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  c #(.P(-3)) u();
  initial #1 $finish;
endmodule
"#,
        &[
            r#"T 0000000000000001 0000000000000003 09 000000007ffffffe fffffffffffffffd fffffffffffffffd 00000000fffffffe 000000007fffffff fffffffffffffffa"#,
        ],
    );
    check(
        r#"module c #(parameter P = 5, parameter Q = P + 1) ();
  localparam R = P - 2;
  initial $display("T %h %h %h %h %h %h %h %h %h", 64'(~(P + 32'd1)), 64'(-P), 8'(P ** 2), 64'(P >> 1), 64'(P), 64'($signed(P)), 64'(P + 1'b1), 64'(Q >> 1), 64'(R - 1));
endmodule
module t;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  c #(.P(-8'sd3)) u();
  initial #1 $finish;
endmodule
"#,
        &[
            r#"T 00000000ffffff01 0000000000000003 09 000000000000007e fffffffffffffffd fffffffffffffffd 00000000000000fe 000000007fffffff fffffffffffffffa"#,
        ],
    );
    check(
        r#"module c #(parameter P = 5, parameter Q = P + 1) ();
  localparam R = P - 2;
  initial $display("T %h %h %h %h %h %h %h %h %h", 64'(~(P + 32'd1)), 64'(-P), 8'(P ** 2), 64'(P >> 1), 64'(P), 64'($signed(P)), 64'(P + 1'b1), 64'(Q >> 1), 64'(R - 1));
endmodule
module t;
  parameter logic [7:0] W8 = 8'hA5;
  parameter logic signed [7:0] S8 = -8'sd3;
  c #(.P(5)) u();
  initial #1 $finish;
endmodule
"#,
        &[
            r#"T 00000000fffffff9 fffffffffffffffb 19 0000000000000002 0000000000000005 0000000000000005 0000000000000006 0000000000000003 0000000000000002"#,
        ],
    );
    check(
        r#"module c #(parameter unsigned P = 1) ();
  initial $display("T %0d %h %h %h %h %h", $bits(P), 64'(P), 64'(P >> 1), 64'(-P), 64'(~(P + 32'd1)), 64'($unsigned(P)));
endmodule
module t;
  parameter UP = 8'hA5;
  c #(.P(-8'sd91)) u();
  initial #1 $finish;
endmodule
"#,
        &[
            r#"T 8 ffffffffffffffa5 0000000000000052 000000000000005b 00000000ffffff59 00000000000000a5"#,
        ],
    );
}

/// `#(.P(A[0][3:0]))` and its siblings onto an untyped and a sign-keyword parameter (verilator's values; iverilog has no unpacked-array parameters). PRE: E3009 on `#()` (`is a select of an array-parameter element`) while the `defparam` of the same text already bound this.
#[test]
fn a_select_of_an_array_parameter_element_binds_its_width_on_every_channel() {
    check(
        r#"module c #(parameter P = 0) (); initial $display("T bits=%0d hex=%h dec=%0d cat=%h", $bits(P), P, P, {P,P}); endmodule
module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam int I = 1;
c #(.P(A[0][3:0])) u1();
initial begin #1 $finish; end
endmodule
"#,
        &[r#"T bits=4 hex=5 dec=5 cat=55"#],
    );
    check(
        r#"module c #(parameter P = 0) (); initial $display("T bits=%0d hex=%h dec=%0d cat=%h", $bits(P), P, P, {P,P}); endmodule
module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam int I = 1;
c #(.P(A[1][7:4])) u1();
initial begin #1 $finish; end
endmodule
"#,
        &[r#"T bits=4 hex=3 dec=3 cat=33"#],
    );
    check(
        r#"module c #(parameter P = 0) (); initial $display("T bits=%0d hex=%h dec=%0d cat=%h", $bits(P), P, P, {P,P}); endmodule
module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam int I = 1;
c #(.P(A[0][2])) u1();
initial begin #1 $finish; end
endmodule
"#,
        &[r#"T bits=1 hex=1 dec=1 cat=3"#],
    );
    check(
        r#"module c #(parameter P = 0) (); initial $display("T bits=%0d hex=%h dec=%0d cat=%h", $bits(P), P, P, {P,P}); endmodule
module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam int I = 1;
c #(.P(A[1][0 +: 4])) u1();
initial begin #1 $finish; end
endmodule
"#,
        &[r#"T bits=4 hex=c dec=12 cat=cc"#],
    );
    check(
        r#"module c #(parameter P = 0) (); initial $display("T bits=%0d hex=%h dec=%0d cat=%h", $bits(P), P, P, {P,P}); endmodule
module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam int I = 1;
c #(.P(A[0][7 -: 3])) u1();
initial begin #1 $finish; end
endmodule
"#,
        &[r#"T bits=3 hex=5 dec=5 cat=2d"#],
    );
    check(
        r#"module c #(parameter P = 0) (); initial $display("T bits=%0d hex=%h dec=%0d cat=%h", $bits(P), P, P, {P,P}); endmodule
module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam int I = 1;
c #(.P(A[I][3:0])) u1();
initial begin #1 $finish; end
endmodule
"#,
        &[r#"T bits=4 hex=c dec=12 cat=cc"#],
    );
    check(
        r#"module c #(parameter signed P = 0) (); initial $display("T bits=%0d hex=%h dec=%0d cat=%h", $bits(P), P, P, {P,P}); endmodule
module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam int I = 1;
c #(.P(A[0][3:0])) u1();
initial begin #1 $finish; end
endmodule
"#,
        &[r#"T bits=4 hex=5 dec=5 cat=55"#],
    );
    check(
        r#"module c #(parameter signed P = 0) (); initial $display("T bits=%0d hex=%h dec=%0d cat=%h", $bits(P), P, P, {P,P}); endmodule
module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam int I = 1;
c #(.P(A[0][2])) u1();
initial begin #1 $finish; end
endmodule
"#,
        &[r#"T bits=1 hex=1 dec=-1 cat=3"#],
    );
    check(
        r#"module c #(parameter signed P = 0) (); initial $display("T bits=%0d hex=%h dec=%0d cat=%h", $bits(P), P, P, {P,P}); endmodule
module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam int I = 1;
c #(.P(A[1][0 +: 4])) u1();
initial begin #1 $finish; end
endmodule
"#,
        &[r#"T bits=4 hex=c dec=-4 cat=cc"#],
    );
}

/// NOT this slice, pinned as measured (ROADMAP §2 row 25): an OPERATOR over an element select binds the default literal's type — `A[1][3:0] + 4'd1` is 32 bits `0000000d` and `~A[0][3:0]` 32 signed bits `fffffffa`, where verilator binds 4 bits `d` / `a`. `declared_override_widths` certifies declared-width vector names and selects of them, not an element of an unpacked array parameter.
#[test]
fn an_operator_over_an_array_element_is_the_recorded_residue() {
    check(
        r#"module c #(parameter P = 0) (); initial $display("T bits=%0d hex=%h dec=%0d cat=%h", $bits(P), P, P, {P,P}); endmodule
module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam int I = 1;
c #(.P(A[1][3:0] + 4'd1)) u1();
initial begin #1 $finish; end
endmodule
"#,
        &[r#"T bits=32 hex=0000000d dec=13 cat=0000000d0000000d"#],
    );
    check(
        r#"module c #(parameter P = 0) (); initial $display("T bits=%0d hex=%h dec=%0d cat=%h", $bits(P), P, P, {P,P}); endmodule
module tb;
localparam logic [7:0] A[2] = '{8'hA5, 8'h3C};
localparam int I = 1;
c #(.P(~A[0][3:0])) u1();
initial begin #1 $finish; end
endmodule
"#,
        &[r#"T bits=32 hex=fffffffa dec=-6 cat=fffffffafffffffa"#],
    );
}
