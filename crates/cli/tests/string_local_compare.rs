//! Two `string` LOCAL variables of an INLINE (straight-line, non-`automatic`, 4-state
//! return) function compared relationally (`s < t`) were silently wrong. Root: the
//! inline path binds a local by SUBSTITUTION and, in `fold_straight_line`, resizes the
//! assigned value to the local's declared width — which for a `string` is
//! `range_to_dims(String)` = 1, truncating the heap string to a single bit. So `s < t`
//! compared two 1-bit packed values (`s="aa"; t="ab"` → 0, not the lexicographic 1). The
//! fix (1) omits a `string`/handle local from `local_dims` so `fold_straight_line` takes
//! its `ctx_w == 0` path (no resize, heap value preserved), and (2) records each local's
//! declared `string`-ness in `formal_str` (like §4.5.81 formals) so a `string` compare on
//! two string locals routes to lexicographic `StrCmp`. Pinned to iverilog 13.0. Unequal-
//! length "ab" vs "b": packed 0x6162 < 0x0062 = 0, string = 1 (unambiguous).
//!
//! Out of scope (separate, pre-existing): a `string` local in a FRAME function (a `bit`/
//! 2-state return, or a body with a statement task like `$display`) is a loud E3018
//! ("procedural assignment to net … declare it reg/logic"), not silent-wrong; and a
//! multi-dim PACKED inline local's element-select (subst carries no dims — a deeper gap).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_slcmp_{}_{n}", std::process::id()));
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

/// Wrap a straight-line inline-function body that computes `bit f` and print `r=<f>`.
fn f_body(body: &str) -> String {
    format!(
        "module m;\n\
         function f(input int d); string s, t; begin {body} end endfunction\n\
         initial begin $display(\"r=%0d\", f(0)); #1 $finish; end endmodule\n"
    )
}

#[test]
fn string_local_lt_equal_and_unequal_length() {
    // "aa" < "ab" = 1 (equal length); "ab" < "b" = 1 (unequal — packed would give 0).
    for (a, b) in [("aa", "ab"), ("ab", "b")] {
        let (out, code) = run(&f_body(&format!("s = \"{a}\"; t = \"{b}\"; f = s < t;")));
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains("r=1"), "\"{a}\" < \"{b}\", got:\n{out}");
    }
}

#[test]
fn string_local_all_relational_ops() {
    // "ab" vs "b": < =1, <= =1, > =0, >= =0 lexicographically.
    for (op, want) in [("<", 1), ("<=", 1), (">", 0), (">=", 0)] {
        let (out, code) = run(&f_body(&format!("s = \"ab\"; t = \"b\"; f = s {op} t;")));
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains(&format!("r={want}")), "op {op}, got:\n{out}");
    }
}

#[test]
fn string_local_from_var() {
    // Locals assigned from string VAR actuals (not literals) also compare lexicographically.
    let (out, code) = run("module m;\n\
         function f(input string a, input string b); string s, t; begin s = a; t = b; f = s < t; end endfunction\n\
         string x, y;\n\
         initial begin x = \"ab\"; y = \"b\"; $display(\"r=%0d\", f(x, y)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=1"), "local from var, got:\n{out}");
}

#[test]
fn string_local_reassigned() {
    // A reassigned local keeps the latest (innermost-wins subst) heap value.
    let (out, code) = run(&f_body("s = \"zz\"; s = \"ab\"; t = \"b\"; f = s < t;"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=1"), "reassigned local, got:\n{out}");
}

#[test]
fn string_local_eq_and_ne_still_correct() {
    // `==` / `!=` were already right (packed coincided for these); must stay right.
    let (out, code) = run(&f_body("s = \"ab\"; t = \"ab\"; f = s == t;"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=1"), "== equal, got:\n{out}");
    let (out, code) = run(&f_body("s = \"ab\"; t = \"b\"; f = s != t;"));
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=1"), "!= unequal, got:\n{out}");
}

#[test]
fn nonstring_locals_unchanged() {
    // A non-string local still bit-resizes: widen zero-extends, narrow truncates.
    let (out, code) = run("module m;\n\
         function [15:0] f(input [7:0] a); logic [15:0] x; begin x = a; f = x; end endfunction\n\
         initial begin $display(\"%h\", f(8'hAB)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("00ab"), "widen local, got:\n{out}");
    let (out, code) = run("module m;\n\
         function [3:0] f(input [7:0] a); logic [3:0] x; begin x = a; f = x; end endfunction\n\
         initial begin $display(\"%h\", f(8'hAB)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains('b'), "narrow local, got:\n{out}");
}

#[test]
fn nonstring_local_shadows_string_formal() {
    // A NON-string local shadowing a `string` formal of the same-kind-space resolves to
    // the local's declared type (innermost-wins in `formal_str`), NOT the string formal.
    let (out, code) = run("module m;\n\
         function [7:0] f(input string a); logic [7:0] a2; begin a2 = 8'hCD; f = a2; end endfunction\n\
         string y;\n\
         initial begin y = \"x\"; $display(\"%h\", f(y)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("cd"), "shadowing local, got:\n{out}");
}
