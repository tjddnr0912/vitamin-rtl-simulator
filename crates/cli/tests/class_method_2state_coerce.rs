//! A 2-state (`bit`/`byte`/`int`/`shortint`/`longint`) FORMAL, RETURN, or body-decl LOCAL
//! of a class METHOD did not coerce a written X/Z to 0 (IEEE §6.11.3): assigning an
//! X-bearing 4-state value to a `bit [7:0]` and reading it back gave `xA`, not `0A`.
//! `reserve_class_method` registered `intro_kind` for NO net (unlike every frame reserve
//! site), so the engine never coerced a 2-state class-method slot. The fix mirrors the
//! frame reserve: register `intro_kind` for a 2-state formal, the return (width-derived
//! 2-state kind when `ret_two_state`), and each 2-state body-decl local. A 4-state
//! `logic`/`reg`/`integer` net must NOT be coerced (X survives). Pinned to iverilog 13.0
//! with input `8'hxA` (high nibble X): 2-state → `0a`, 4-state → `xa`.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_c2s_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

/// A class `C` with method `f` (the given signature+body) called with `8'hxA`.
fn m(sig_body: &str) -> String {
    format!(
        "module m;\n\
         class C; function {sig_body} endfunction endclass\n\
         C c; initial begin c = new; $display(\"%h\", c.f(8'hxA)); #1 $finish; end endmodule\n"
    )
}

#[test]
fn two_state_local_coerces_x_to_zero() {
    // `bit`/`byte`/`int` body-decl locals coerce the X-write high nibble to 0 → 0a.
    for decl in ["bit [7:0] x", "byte x", "int x"] {
        let (out, code) = run(&m(&format!(
            "[7:0] f(input [7:0] u); {decl}; begin x = u; f = x; end"
        )));
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains("0a"), "{decl} coerce, got:\n{out}");
    }
}

#[test]
fn two_state_formal_coerces() {
    let (out, code) = run(&m("[7:0] f(input bit [7:0] x); begin f = x; end"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("0a"), "bit formal, got:\n{out}");
}

#[test]
fn two_state_return_coerces() {
    let (out, code) = run(&m("bit [7:0] f(input [7:0] u); begin f = u; end"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("0a"), "bit return, got:\n{out}");
}

#[test]
fn four_state_local_keeps_x() {
    // A 4-state `logic` local (and the default `reg` return) must NOT coerce — X survives.
    let (out, code) = run(&m(
        "[7:0] f(input [7:0] u); logic [7:0] x; begin x = u; f = x; end",
    ));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("xa"), "logic local keeps X, got:\n{out}");
    let (out, code) = run(&m("[7:0] f(input [7:0] u); begin f = u; end"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("xa"), "reg return keeps X, got:\n{out}");
}

#[test]
fn normal_value_unchanged() {
    // A non-X value is unaffected by the coercion registration.
    let (out, code) = run("module m;\n\
         class C; function [7:0] f(input [7:0] u); bit [7:0] x; begin x = u; f = x; end endfunction endclass\n\
         C c; initial begin c = new; $display(\"%h\", c.f(8'hAB)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("ab"), "normal value, got:\n{out}");
}

#[test]
fn packed_local_not_regressed() {
    // §4.5.83 packed-local element read must still work (same reserve loop).
    let (out, code) = run("module m;\n\
         class C; function [7:0] f; logic [1:0][7:0] p; begin p = 16'hABCD; f = p[0]; end endfunction endclass\n\
         C c; initial begin c = new; $display(\"%h\", c.f()); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("cd"), "packed local, got:\n{out}");
}
