//! A fill override (`#(.P('1))`, `defparam u.P = '1`) onto an untyped parameter with no sign
//! keyword TYPES that parameter — one unsigned bit, the fill arm of `bind_one_param`'s meta
//! chain — so it is not a guessed type. The size-cast classifier used to take the pre-slice
//! route for it (`param_type_guessed`), which read `64'(-P)` over `#(.P('1))` as
//! `0000000000000001` and `64'(~P)` as `…0000` where both oracles give all ones and `…fffe`.
//!
//! Onto a `signed` keyword the fill is still recorded unsigned (both oracles bind −1): that
//! record is not a fact, so it stays a guess (ROADMAP §2 "Index sealing", the sign-keyword
//! bullet, whose fix waits on a generate-scope alias's recorded width).
//!
//! Values: iverilog 13.0 and verilator 5.052 agree on every line. `PRE` is vita before §4.5.559.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn lines(src: &str) -> Vec<String> {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_fotp_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let s =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "expected exit 0, got:\n{s}");
    let mut v: Vec<String> = s
        .lines()
        .filter(|l| l.starts_with("T "))
        .map(|l| l.trim_end().to_string())
        .collect();
    v.sort();
    v
}

/// The size casts over a fill-overridden parameter: `'1` named and by `defparam`, and `'0`.
/// PRE: `u1` / `u2` `0000000000000001 0000000000000001 0000000000000000`, `u0`
/// `… 0000000000000001`.
#[test]
fn a_size_cast_reads_the_fills_type() {
    let src = "module su #(parameter P = 5) ();\n  logic [63:0] a, b, c; logic [7:0] d;\n  initial \
               begin a = 64'(P); b = 64'(-P); c = 64'(~P); d = 8'(P + 1); #1 $display(\"T %m %h %h \
               %h %h %0d\", a, b, c, d, $bits(P)); end\nendmodule\nmodule t;\n  su #(.P('1)) u1();\n  \
               su #(.P('0)) u0();\n  su u2();\n  defparam u2.P = '1;\n  initial #3 $finish;\n\
               endmodule\n";
    let mut want = vec![
        "T t.u0 0000000000000000 0000000000000000 ffffffffffffffff 01 1".to_string(),
        "T t.u1 0000000000000001 ffffffffffffffff fffffffffffffffe 02 1".to_string(),
        "T t.u2 0000000000000001 ffffffffffffffff fffffffffffffffe 02 1".to_string(),
    ];
    want.sort();
    assert_eq!(lines(src), want, "{src}");
}
