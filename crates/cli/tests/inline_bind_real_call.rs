//! ROADMAP §2 F8: a REAL actual that may not be repeated — a real-returning call,
//! or any expression containing one (`-rf(0)`, `rf(0) * 2.0`) — bound to an
//! INTEGRAL formal of an inline (static, straight-line) function.
//!
//! §13.5.3 makes the bind an ASSIGNMENT to a variable of the formal's type, so the
//! real rounds half away from zero and narrows to the formal (§6.12.2). The inline
//! lane substitutes the actual's ExprId for the formal's name, so it has no net
//! store to do that; the repeatable shapes (a real variable, a literal, a
//! negation of one) were already converted with the IR-0 cast
//! (`lower_real_to_int_cast`), which names its operand 2–5 times and therefore
//! declined a call. Such an actual was left verbatim: a silent `012d` for the
//! oracles' `002d` into a 16-bit net, and a loud E3009 wherever `%h` or a select
//! met the unconverted real. It now takes `SysFuncId::RealToInt` (format_version
//! 34), which names the operand ONCE, then the formal's width and sign.
//!
//! ORACLES: iverilog 13.0 (`iverilog -g2012`) and verilator 5.052
//! (`--binary --timing`). Every pinned value is the raw output of both unless the
//! test says otherwise. PRE = the binary built at the parent commit.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn dir_for(tag: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ibrc_{tag}_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// (stdout without the end line, stderr, exit code)
fn run(src: &str) -> (String, String, Option<i32>) {
    let d = dir_for("run");
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let so = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.contains("simulation ended"))
        .collect::<Vec<_>>()
        .join("\n");
    let se = String::from_utf8_lossy(&out.stderr).into_owned();
    let _ = std::fs::remove_dir_all(&d);
    (so, se, out.status.code())
}

fn run_ok(src: &str) -> String {
    let (so, se, code) = run(src);
    assert_eq!(code, Some(0), "stdout:\n{so}\nstderr:\n{se}");
    assert!(!se.contains("error["), "no diagnostic expected:\n{se}");
    so
}

/// f8h — the silent cell. `o2 = pb(rf(0))` is the inline lane; `o1` is the frame
/// lane beside it, which the engine's slot store already converted. PRE printed
/// `inl=012d` (300.7 rounded but never narrowed to the `byte`).
#[test]
fn a_real_call_actual_narrows_to_the_formal() {
    let o = run_ok(
        r#"module top;
  function automatic real rf(input int k); rf = 300.7 + k; endfunction
  function automatic [7:0] pf(input byte x); return x; endfunction
  function [7:0] pb(input byte x); pb = x; endfunction
  function int pi(input int x); pi = x * 2; endfunction
  logic [15:0] o1, o2; int o3;
  initial begin o1 = pf(rf(0)); o2 = pb(rf(0)); o3 = pi(rf(0)); $display("F8H frame=%h inl=%h int2=%0d negf=%h", o1, o2, o3, pf(-rf(0))); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F8H frame=002d inl=002d int2=602 negf=d3");
}

/// f8a — a `longint` formal read through a select. PRE: E3009 (select on real).
#[test]
fn a_selected_real_call_formal_is_an_integer() {
    let o = run_ok(
        r#"module top;
  function automatic real arf(input int k); arf = 4.4 + k; endfunction
  function [31:0] two(input int xn, input longint b); two = b[31:0]; endfunction
  initial begin $display("F8A %h", two(1, arf(0))); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F8A 00000004");
}

/// f8b — `%h` of the call. PRE: E3009 (hex format on a real argument), twice.
#[test]
fn a_real_call_actual_prints_in_hex() {
    let o = run_ok(
        r#"module top;
  function automatic real rf(input int k); rf = 300.0 + k; endfunction
  function [7:0] pb(input byte x); pb = x; endfunction
  function [7:0] pa(input byte x); pa = x + 1; endfunction
  initial begin $display("F8B pb=%h pa=%h", pb(rf(0)), pa(rf(0))); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F8B pb=2c pa=2d");
}

/// f8e — a call INSIDE the actual (a negation, a product). PRE: E3009 ×2.
#[test]
fn an_expression_containing_a_real_call_narrows() {
    let o = run_ok(
        r#"module top;
  function automatic real rf(input int k); rf = 300.0 + k; endfunction
  function [7:0] pb(input byte x); pb = x; endfunction
  initial begin $display("F8E neg=%h mul=%h", pb(-rf(0)), pb(rf(0) * 2.0)); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F8E neg=d4 mul=58");
}

/// f8f — `int`, `shortint` and `longint` formals under wider returns: the rounded
/// 301 is extended by the formal's sign. PRE: E3009 ×3.
#[test]
fn every_integral_formal_width_takes_the_rounded_value() {
    let o = run_ok(
        r#"module top;
  function automatic real rf(input int k); rf = 300.7 + k; endfunction
  function [15:0] p16(input int x); p16 = x; endfunction
  function [31:0] p32s(input shortint x); p32s = x; endfunction
  function [63:0] p64(input longint x); p64 = x; endfunction
  initial begin $display("F8F i=%h s=%h l=%h", p16(rf(0)), p32s(rf(0)), p64(rf(0))); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F8F i=012d s=0000012d l=000000000000012d");
}

/// f8g — two converted calls in one expression, beside the frame twin.
/// PRE: E3009.
#[test]
fn two_converted_calls_in_one_expression() {
    let o = run_ok(
        r#"module top;
  function automatic real rf(input int k); rf = 300.0 + k; endfunction
  function [7:0] pb(input byte x); pb = x; endfunction
  function automatic [7:0] pf(input byte x); pf = x; endfunction
  initial begin $display("F8G frame=%h twice=%h", pf(rf(0)), pb(rf(1)) + pb(rf(2))); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F8G frame=2c twice=5b");
}

/// A `$random`-bearing real actual is evaluated ONCE. `b1`/`b2` are the 2nd and
/// 3rd draws of the default stream exactly when the bind drew once (iverilog 13's
/// stream, which vita's `$random` reproduces; verilator's stream differs, so it
/// is not an oracle here). The value is the first draw (303379748) / 8.0 =
/// 37922468.5, rounded half away from zero to 37922469 = `0x242a6a5`, narrowed
/// to the `byte` formal: `a5`. PRE printed `a=a6a5` — the verbatim real stored
/// into the 16-bit net.
#[test]
fn a_random_bearing_real_actual_is_drawn_once() {
    let o = run_ok(
        r#"module top;
  function [7:0] pb(input byte x); pb = x; endfunction
  logic [15:0] a16; integer b1, b2;
  initial begin
    a16 = pb($itor($random) / 8.0);
    b1 = $random;
    b2 = $random;
    $display("T3 a=%h b1=%0d b2=%0d", a16, b1, b2);
    #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "T3 a=00a5 b1=-1064739199 b2=-2071669239");
}

/// The repeatable shapes keep the IR-0 conversion (f8c/f8i). Values unchanged
/// from PRE; both oracles.
#[test]
fn repeatable_real_actuals_are_unchanged() {
    let o = run_ok(
        r#"module top;
  real r = 300.0;
  function [7:0] pb(input byte x); pb = x; endfunction
  initial begin $display("F8C var=%h lit=%h neg=%h", pb(r), pb(300.0), pb(-r)); #1 $finish; end
endmodule
"#,
    );
    assert_eq!(o, "F8C var=2c lit=2c neg=d4");
}

const STAGED: &str = r#"module t;
  function automatic real rf(input int k); rf = 300.7 + k; endfunction
  function [7:0] pb(input byte x); pb = x; endfunction
  real r = 1.5;
  function [7:0] f(input [7:0] x); f = r + x; endfunction
  function [7:0] g(input [7:0] x); bit [7:0] b; b = x; g = b; endfunction
  logic [15:0] o1, o2, o3;
  initial begin
    o1 = pb(rf(0)); o2 = f(8'd254); o3 = g(8'bx0000111);
    if (o1 !== 16'h002d) $fatal(1, "RealToInt bind lost on the staged path");
    if (o2 !== 16'h0000) $fatal(1, "RealToInt body store lost on the staged path");
    if (o3 !== 16'h0007) $fatal(1, "TwoState local store lost on the staged path");
    $display("OK %h %h %h", o1, o2, o3);
    #1 $finish;
  end
endmodule
"#;

/// format_version 34: `RealToInt` and `TwoState` are frozen-IR nodes, so they must
/// survive `vcmp → velab → vrun`. iverilog 13 passes every guard and prints
/// `OK 002d 0000 0007`; PRE fails the first guard (F-RUN-FATAL).
#[test]
fn staged_vcmp_velab_vrun_carries_both_conversions() {
    assert_eq!(vita_artifact::CURRENT_FORMAT_VERSION, 34);
    assert_eq!(run_ok(STAGED), "OK 002d 0000 0007");
    let dir = dir_for("staged");
    let s = |p: &std::path::Path| p.to_str().unwrap().to_string();
    let sv = dir.join("t.sv");
    std::fs::write(&sv, STAGED).unwrap();
    let vu = dir.join("t.vu");
    let velab = dir.join("t.velab");
    let o = cli::VitaOpts::default();
    assert_eq!(
        cli::run_vcmp(&[s(&sv)], Some(&s(&vu)), &o),
        0,
        "vcmp failed"
    );
    assert_eq!(cli::run_velab(&s(&vu), &s(&velab), &o), 0, "velab failed");
    assert_eq!(
        cli::run_vrun(&s(&velab), &o),
        0,
        "staged run lost a conversion"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
