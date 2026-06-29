//! P0-8/9: `$display` argument semantics + `$monitor` trigger discipline
//! (REMAINING_WORK 2026-06-10).
//!
//! IEEE 1364-2005 §17.1: the display tasks process their argument list
//! SEQUENTIALLY — a string-literal argument is a format segment whose `%`
//! specs consume the following arguments; any argument not consumed by a
//! format string prints in the default radix (`%d` field for the $display
//! family). vitamin previously dropped every argument after the leading
//! format string, printed bare string args as decimals, and left `%v/%u/%z/%p`
//! unconsumed (shifting later specs onto wrong args). `$monitor` §17.1.3:
//! a DIRECT `$time`/`$realtime` argument must not retrigger the monitor.

use diag::{LogEvent, LogSink};
use sim_engine::{simulate_capture, SimOpts};

#[derive(Default)]
struct DiagSink(std::cell::RefCell<Vec<String>>);
impl LogSink for DiagSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::Diagnostic(d) = e {
            self.0
                .borrow_mut()
                .push(format!("{:?}: {}", d.severity, d.message));
        }
    }
}

fn run(src: &str) -> String {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = DiagSink::default();
    let ir = elaborate::elaborate(&su.expect("source unit"), &sink);
    let hard: Vec<String> = sink
        .0
        .borrow()
        .iter()
        .filter(|d| d.starts_with("Error") || d.starts_with("Fatal"))
        .cloned()
        .collect();
    assert!(hard.is_empty(), "elaborate errors: {hard:?}");
    let (_res, out) = simulate_capture(&ir.expect("ir"), SimOpts::default());
    out
}

/// SW3 (2026-06-22 audit): `%0N` zero-padding. Spec 01-display-io.md:186 prints
/// `$display("%06d",42)=="000042"`, but the renderer treated ANY leading `0` as
/// "minimal width" and dropped both the width AND the zero-pad (printed "42").
/// iverilog-pinned: `%0d`/`%0h` bare = minimal; `%0Nd` = zero-pad to N (sign-aware:
/// `-42` → `-00042`); `%Nd` = space-pad; `%h` = full vector width.
#[test]
fn zero_pad_format_specifiers() {
    let out = run(r#"
module t;
  initial begin
    $display("[%06d]", 42);
    $display("[%06h]", 8'hA);
    $display("[%6d]", 42);
    $display("[%0d]", 42);
    $display("[%0h]", 8'hA);
    $display("[%h]", 8'hA);
    $display("[%04b]", 2'b10);
    $display("[%06d]", -42);
  end
endmodule
"#);
    assert_eq!(
        out,
        "[000042]\n[00000a]\n[    42]\n[42]\n[a]\n[0a]\n[0010]\n[-00042]\n"
    );
}

/// IEEE 1800 §21.2.1.2 unknown-value letter case for `%d`/`%h`/`%o`: the letter is
/// LOWERCASE only when the field (whole value for `%d`, a digit for `%h`/`%o`) is
/// UNIFORM — no known bit AND a single unknown kind (`x` if entirely x, `z` if
/// entirely z). ANY mixing — known+unknown, or x and z together — is UPPERCASE
/// (`X` if any x, else `Z`). Previously vita collapsed every unknown to lowercase
/// `x` (so all-z showed `x`, partial-unknown and x+z-mixed lost uppercasing).
/// iverilog-pinned (e.g. `8'bxxxxzzzz`→`X`, `8'bzzzzzzzz`→`z`, `8'bx0000001`→`X`).
#[test]
fn unknown_value_letter_case() {
    let out = run(r#"
module t;
  initial begin
    // %d whole-field: all-x, all-z, partial-x, partial-z, partial-mixed, all-unknown-mixed
    $display("%0d %0d %0d %0d %0d %0d",
             8'bxxxxxxxx, 8'bzzzzzzzz, 8'bx0000001, 8'bz0000001, 8'bx000z001, 8'bxxxxzzzz);
    // %h per-nibble: all-x→x, partial-x→X, partial-z→Z, all-z→z
    $display("%h %h %h %h", 8'bxxxxxxxx, 8'b0000x001, 8'bz0000001, 8'b0000zzzz);
    // %o per-digit: all-x→x, partial-x→X, partial-z→Z
    $display("%o %o %o", 9'bxxxxxxxxx, 9'bx00000001, 9'b00z000000);
  end
endmodule
"#);
    assert_eq!(out, "x z X Z X X\nxx 0X Z1 0z\nxxx X01 Z00\n");
}

/// P0-8①: arguments after the leading format string print in the default
/// radix instead of being silently dropped.
#[test]
fn trailing_arg_after_format_string() {
    let out = run(r#"
module t;
  reg [7:0] v;
  initial begin v = 8'd255; $display("val=", v); end
endmodule
"#);
    assert_eq!(out.trim_end(), "val=255");
}

/// P0-8①: the default radix is a PADDED %d field (8-bit → width 3), exactly
/// like a bare `%d` spec — iverilog parity.
#[test]
fn trailing_small_arg_pads_default_width() {
    let out = run(r#"
module t;
  initial $display("v=", 8'd5);
endmodule
"#);
    assert_eq!(out.trim_end(), "v=  5");
}

/// P0-8②: a string-literal argument prints as TEXT (and its specs would
/// consume following args), not as a packed-ASCII decimal.
#[test]
fn bare_string_arg_prints_text() {
    let out = run(r#"
module t;
  initial $display(8'd7, " is a");
endmodule
"#);
    assert_eq!(out.trim_end(), "  7 is a");
}

/// P0-8①+②: interleaved strings and values consume left-to-right.
#[test]
fn interleaved_strings_consume_in_order() {
    let out = run(r#"
module t;
  initial $display("a=%0d", 4'd1, " b=", 8'd5);
endmodule
"#);
    assert_eq!(out.trim_end(), "a=1 b=  5");
}

/// P0-8②: with NO leading format string, bare value args concatenate as
/// padded %d fields with NO separator (was: space-joined).
#[test]
fn bare_args_concat_padded_fields() {
    let out = run(r#"
module t;
  initial $display(8'd255, 8'd255);
endmodule
"#);
    assert_eq!(out.trim_end(), "255255");
}

/// P0-8③: `%v` consumes its argument and renders the iverilog-style strength
/// form (St0/St1/StX/HiZ) — it previously printed literally and consumed
/// nothing, shifting every later spec onto the wrong argument.
#[test]
fn percent_v_consumes_and_renders_strength() {
    let out = run(r#"
module t;
  reg w, z;
  initial begin
    w = 1'b1;
    $display("%v %0d", w, 8'd5);
    z = 1'bz;
    $display("%v %0d", z, 8'd6);
  end
endmodule
"#);
    assert_eq!(out.trim_end(), "St1 5\nHiZ 6");
}

/// P0-8③: `%u`/`%z` (binary dump specs) consume their argument; vitamin emits
/// no text for them (documented divergence from the raw-byte IEEE form).
#[test]
fn percent_u_consumes_silently() {
    let out = run(r#"
module t;
  initial $display("[%u]%0d", 8'd65, 8'd5);
endmodule
"#);
    assert_eq!(out.trim_end(), "[]5");
}

/// P0-8③: `%p` consumes and renders the value (minimal-width decimal).
#[test]
fn percent_p_renders_value() {
    let out = run(r#"
module t;
  initial $display("%p", 8'd9);
endmodule
"#);
    assert_eq!(out.trim_end(), "9");
}

/// P0-9①: a DIRECT `$time` argument does not retrigger `$monitor` — time
/// advancing with activity (clk toggles) but no monitored VALUE change must
/// print exactly once (IEEE §17.1.3; iverilog parity).
#[test]
fn monitor_ignores_direct_time_arg() {
    let out = run(r#"
module t;
  reg clk;
  reg [7:0] v;
  initial begin
    v = 8'd5;
    clk = 0;
    $monitor("t=%0t v=%0d", $time, v);
  end
  always #1 clk = ~clk;
  initial #4 $finish;
endmodule
"#);
    assert_eq!(out.trim_end(), "t=0 v=5");
}

/// P0-9 keep-green: the monitor still fires when a monitored value REALLY
/// changes (the time-arg filter must not swallow genuine changes).
#[test]
fn monitor_still_fires_on_value_change() {
    let out = run(r#"
module t;
  reg clk;
  reg [7:0] v;
  initial begin
    v = 8'd5;
    clk = 0;
    $monitor("t=%0t v=%0d", $time, v);
    #2 v = 8'd9;
    #2 $finish;
  end
  always #1 clk = ~clk;
endmodule
"#);
    assert_eq!(out.trim_end(), "t=0 v=5\nt=2 v=9");
}
