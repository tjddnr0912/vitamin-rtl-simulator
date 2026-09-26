//! A real bound to an inline formal wider than 128 bits converts at the formal's width —
//! ROADMAP §2 "Inline / frame binds", closed by §4.5.550.
//!
//! The inline function lane stores a real into an integral formal / return as
//! `RealToInt(e) + <w-bit signed 0>`: `RealToInt` is a 128-bit node, and the add extended its
//! 128-bit image, so a real with |x| ≥ 2^127 kept only its low 128 bits (`g(1e40)` into a
//! `[191:0]` formal was `00000000000000006329f1c35ca5…` where both oracles hold
//! `000000000000001d6329f1c35ca5…`; 2^127 exactly into a 129-bit formal was `1800…`, both
//! `0800…`). `RealToInt` evaluated in a context wider than 128 bits now converts at the
//! context's width (`real_to_int_round` is exact at any width); at 128 bits or less the
//! node's value truncated to the context is the same number. The automatic (frame) lane, the
//! module store and `return x` were already exact and are the controls in each cell.
//!
//! Oracles: iverilog 13.0 `-g2012` and verilator 5.052 print every line below. Each cell
//! asserts the native lines and that the interpreter and the VM print the same.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str, backend: &str) -> Vec<String> {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_wrif_{}_{n}.sv", std::process::id()));
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
        .filter(|l| l.starts_with("T "))
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

fn check(src: &str, want: &[&str]) {
    let native = run(src, "native");
    assert_eq!(native, want, "native");
    assert_eq!(run(src, "interp"), native, "interp");
    assert_eq!(run(src, "vm"), native, "vm");
}

/// `rv = 1e40`: a static function's wide formal (`inl`), a real formal stored into a wide return (`rform`), a signed wide formal (`sgn`), a 129-bit formal (`h129`); `auto`, `rret` and `mod` are the lanes that were already exact.
#[test]
fn value_p40() {
    check(
        r#"module t;
  real rv = 1e40;
  function reg [191:0] g(input reg [191:0] x); g = x; endfunction
  function automatic reg [191:0] ga(input reg [191:0] x); ga = x; endfunction
  function reg [191:0] g2(input real x); g2 = x; endfunction
  function reg [191:0] g3(input real x); return x; endfunction
  function reg signed [191:0] gs(input reg signed [191:0] x); gs = x; endfunction
  function reg [128:0] h(input reg [128:0] x); h = x; endfunction
  reg [191:0] m;
  initial begin
    m = rv;
    #1;
    $display("T inl   %h", g(rv * 1.0));
    $display("T auto  %h", ga(rv * 1.0));
    $display("T rform %h", g2(rv));
    $display("T rret  %h", g3(rv));
    $display("T sgn   %h", gs(rv * 1.0));
    $display("T h129  %h", h(rv * 1.0));
    $display("T mod   %h", m);
    $finish;
  end
endmodule
"#,
        &[
            r#"T inl 000000000000001d6329f1c35ca500000000000000000000"#,
            r#"T auto 000000000000001d6329f1c35ca500000000000000000000"#,
            r#"T rform 000000000000001d6329f1c35ca500000000000000000000"#,
            r#"T rret 000000000000001d6329f1c35ca500000000000000000000"#,
            r#"T sgn 000000000000001d6329f1c35ca500000000000000000000"#,
            r#"T h129 16329f1c35ca500000000000000000000"#,
            r#"T mod 000000000000001d6329f1c35ca500000000000000000000"#,
        ],
    );
}

/// `rv = -1e40`: a static function's wide formal (`inl`), a real formal stored into a wide return (`rform`), a signed wide formal (`sgn`), a 129-bit formal (`h129`); `auto`, `rret` and `mod` are the lanes that were already exact.
#[test]
fn value_n40() {
    check(
        r#"module t;
  real rv = -1e40;
  function reg [191:0] g(input reg [191:0] x); g = x; endfunction
  function automatic reg [191:0] ga(input reg [191:0] x); ga = x; endfunction
  function reg [191:0] g2(input real x); g2 = x; endfunction
  function reg [191:0] g3(input real x); return x; endfunction
  function reg signed [191:0] gs(input reg signed [191:0] x); gs = x; endfunction
  function reg [128:0] h(input reg [128:0] x); h = x; endfunction
  reg [191:0] m;
  initial begin
    m = rv;
    #1;
    $display("T inl   %h", g(rv * 1.0));
    $display("T auto  %h", ga(rv * 1.0));
    $display("T rform %h", g2(rv));
    $display("T rret  %h", g3(rv));
    $display("T sgn   %h", gs(rv * 1.0));
    $display("T h129  %h", h(rv * 1.0));
    $display("T mod   %h", m);
    $finish;
  end
endmodule
"#,
        &[
            r#"T inl ffffffffffffffe29cd60e3ca35b00000000000000000000"#,
            r#"T auto ffffffffffffffe29cd60e3ca35b00000000000000000000"#,
            r#"T rform ffffffffffffffe29cd60e3ca35b00000000000000000000"#,
            r#"T rret ffffffffffffffe29cd60e3ca35b00000000000000000000"#,
            r#"T sgn ffffffffffffffe29cd60e3ca35b00000000000000000000"#,
            r#"T h129 09cd60e3ca35b00000000000000000000"#,
            r#"T mod ffffffffffffffe29cd60e3ca35b00000000000000000000"#,
        ],
    );
}

/// `rv = 2^127 exactly`: a static function's wide formal (`inl`), a real formal stored into a wide return (`rform`), a signed wide formal (`sgn`), a 129-bit formal (`h129`); `auto`, `rret` and `mod` are the lanes that were already exact.
#[test]
fn value_p127() {
    check(
        r#"module t;
  real rv = 170141183460469231731687303715884105728.0;
  function reg [191:0] g(input reg [191:0] x); g = x; endfunction
  function automatic reg [191:0] ga(input reg [191:0] x); ga = x; endfunction
  function reg [191:0] g2(input real x); g2 = x; endfunction
  function reg [191:0] g3(input real x); return x; endfunction
  function reg signed [191:0] gs(input reg signed [191:0] x); gs = x; endfunction
  function reg [128:0] h(input reg [128:0] x); h = x; endfunction
  reg [191:0] m;
  initial begin
    m = rv;
    #1;
    $display("T inl   %h", g(rv * 1.0));
    $display("T auto  %h", ga(rv * 1.0));
    $display("T rform %h", g2(rv));
    $display("T rret  %h", g3(rv));
    $display("T sgn   %h", gs(rv * 1.0));
    $display("T h129  %h", h(rv * 1.0));
    $display("T mod   %h", m);
    $finish;
  end
endmodule
"#,
        &[
            r#"T inl 000000000000000080000000000000000000000000000000"#,
            r#"T auto 000000000000000080000000000000000000000000000000"#,
            r#"T rform 000000000000000080000000000000000000000000000000"#,
            r#"T rret 000000000000000080000000000000000000000000000000"#,
            r#"T sgn 000000000000000080000000000000000000000000000000"#,
            r#"T h129 080000000000000000000000000000000"#,
            r#"T mod 000000000000000080000000000000000000000000000000"#,
        ],
    );
}

/// `rv = -2^127 (the i128 edge, already exact)`: a static function's wide formal (`inl`), a real formal stored into a wide return (`rform`), a signed wide formal (`sgn`), a 129-bit formal (`h129`); `auto`, `rret` and `mod` are the lanes that were already exact.
#[test]
fn value_n127() {
    check(
        r#"module t;
  real rv = -170141183460469231731687303715884105728.0;
  function reg [191:0] g(input reg [191:0] x); g = x; endfunction
  function automatic reg [191:0] ga(input reg [191:0] x); ga = x; endfunction
  function reg [191:0] g2(input real x); g2 = x; endfunction
  function reg [191:0] g3(input real x); return x; endfunction
  function reg signed [191:0] gs(input reg signed [191:0] x); gs = x; endfunction
  function reg [128:0] h(input reg [128:0] x); h = x; endfunction
  reg [191:0] m;
  initial begin
    m = rv;
    #1;
    $display("T inl   %h", g(rv * 1.0));
    $display("T auto  %h", ga(rv * 1.0));
    $display("T rform %h", g2(rv));
    $display("T rret  %h", g3(rv));
    $display("T sgn   %h", gs(rv * 1.0));
    $display("T h129  %h", h(rv * 1.0));
    $display("T mod   %h", m);
    $finish;
  end
endmodule
"#,
        &[
            r#"T inl ffffffffffffffff80000000000000000000000000000000"#,
            r#"T auto ffffffffffffffff80000000000000000000000000000000"#,
            r#"T rform ffffffffffffffff80000000000000000000000000000000"#,
            r#"T rret ffffffffffffffff80000000000000000000000000000000"#,
            r#"T sgn ffffffffffffffff80000000000000000000000000000000"#,
            r#"T h129 180000000000000000000000000000000"#,
            r#"T mod ffffffffffffffff80000000000000000000000000000000"#,
        ],
    );
}

/// `rv = 2^130`: a static function's wide formal (`inl`), a real formal stored into a wide return (`rform`), a signed wide formal (`sgn`), a 129-bit formal (`h129`); `auto`, `rret` and `mod` are the lanes that were already exact.
#[test]
fn value_p130() {
    check(
        r#"module t;
  real rv = 1361129467683753853853498429727072845824.0;
  function reg [191:0] g(input reg [191:0] x); g = x; endfunction
  function automatic reg [191:0] ga(input reg [191:0] x); ga = x; endfunction
  function reg [191:0] g2(input real x); g2 = x; endfunction
  function reg [191:0] g3(input real x); return x; endfunction
  function reg signed [191:0] gs(input reg signed [191:0] x); gs = x; endfunction
  function reg [128:0] h(input reg [128:0] x); h = x; endfunction
  reg [191:0] m;
  initial begin
    m = rv;
    #1;
    $display("T inl   %h", g(rv * 1.0));
    $display("T auto  %h", ga(rv * 1.0));
    $display("T rform %h", g2(rv));
    $display("T rret  %h", g3(rv));
    $display("T sgn   %h", gs(rv * 1.0));
    $display("T h129  %h", h(rv * 1.0));
    $display("T mod   %h", m);
    $finish;
  end
endmodule
"#,
        &[
            r#"T inl 000000000000000400000000000000000000000000000000"#,
            r#"T auto 000000000000000400000000000000000000000000000000"#,
            r#"T rform 000000000000000400000000000000000000000000000000"#,
            r#"T rret 000000000000000400000000000000000000000000000000"#,
            r#"T sgn 000000000000000400000000000000000000000000000000"#,
            r#"T h129 000000000000000000000000000000000"#,
            r#"T mod 000000000000000400000000000000000000000000000000"#,
        ],
    );
}

/// `rv = -2.5 (the small control)`: a static function's wide formal (`inl`), a real formal stored into a wide return (`rform`), a signed wide formal (`sgn`), a 129-bit formal (`h129`); `auto`, `rret` and `mod` are the lanes that were already exact.
#[test]
fn value_sm() {
    check(
        r#"module t;
  real rv = -2.5;
  function reg [191:0] g(input reg [191:0] x); g = x; endfunction
  function automatic reg [191:0] ga(input reg [191:0] x); ga = x; endfunction
  function reg [191:0] g2(input real x); g2 = x; endfunction
  function reg [191:0] g3(input real x); return x; endfunction
  function reg signed [191:0] gs(input reg signed [191:0] x); gs = x; endfunction
  function reg [128:0] h(input reg [128:0] x); h = x; endfunction
  reg [191:0] m;
  initial begin
    m = rv;
    #1;
    $display("T inl   %h", g(rv * 1.0));
    $display("T auto  %h", ga(rv * 1.0));
    $display("T rform %h", g2(rv));
    $display("T rret  %h", g3(rv));
    $display("T sgn   %h", gs(rv * 1.0));
    $display("T h129  %h", h(rv * 1.0));
    $display("T mod   %h", m);
    $finish;
  end
endmodule
"#,
        &[
            r#"T inl fffffffffffffffffffffffffffffffffffffffffffffffd"#,
            r#"T auto fffffffffffffffffffffffffffffffffffffffffffffffd"#,
            r#"T rform fffffffffffffffffffffffffffffffffffffffffffffffd"#,
            r#"T rret fffffffffffffffffffffffffffffffffffffffffffffffd"#,
            r#"T sgn fffffffffffffffffffffffffffffffffffffffffffffffd"#,
            r#"T h129 1fffffffffffffffffffffffffffffffd"#,
            r#"T mod fffffffffffffffffffffffffffffffffffffffffffffffd"#,
        ],
    );
}
