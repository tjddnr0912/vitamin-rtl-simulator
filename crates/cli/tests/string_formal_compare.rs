//! A `string` RELATIONAL compare (`a < b`, `<=`, `>`, `>=`) routes through `StrCmp`
//! (lexicographic, IEEE §11.4.4) only if an operand is detected string-domain. A
//! `string`-declared FORMAL was not detected — an INLINE literal actual is a packed
//! const (not a string net), and a FRAME (automatic) formal is a scoped net not in the
//! `subst` map — so `a < b` on two string formals silently did a PACKED compare
//! (non-lexicographic for unequal lengths). The fix records each formal's declared
//! `string`-ness (`formal_str`) and consults it first in `expr_is_string_ast`, keyed on
//! the FORMAL's declared type (not the actual's, so a string literal passed to a PACKED
//! formal still compares packed). All inputs use "ab" vs "b": packed 0x6162 < 0x0062 =
//! false (0), string "ab" < "b" = true (1) — unambiguous. Pinned to iverilog 13.0.
//!
//! Out of scope (separate follow-ons): a FRAME formal fed a string LITERAL still holds a
//! mangled slot value (a distinct engine string-literal → String-slot materialization
//! bug, pre-existing and unchanged by this fix); two string LOCALS compared in a function
//! body; a `string` function RETURN type / string method on a formal (both loud); and
//! TASK string formals (an inline task copies formals to `__taskarg_` locals — the same
//! local-string gap — and a frame task would be a clean mirror; both left as follow-ons).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_strfml_{}_{n}", std::process::id()));
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

#[test]
fn inline_literal_both_formals_lt() {
    // INLINE (implicit reg return, straight-line) function, BOTH operands string LITERAL
    // actuals: `a < b` = "ab" < "b" = 1 (lexicographic), NOT the packed 0.
    let (out, code) = run("module m;\n\
         function f(input string a, input string b); f = a < b; endfunction\n\
         initial begin $display(\"r=%0d\", f(\"ab\", \"b\")); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=1"), "inline literal string <, got:\n{out}");
}

#[test]
fn inline_var_both_formals_lt_unchanged() {
    // INLINE + string VAR actuals — worked before (the actual's net is string); must
    // still be lexicographic after the fix (formal_str catches it first, same result).
    let (out, code) = run("module m;\n\
         function f(input string a, input string b); f = a < b; endfunction\n\
         string x, y;\n\
         initial begin x=\"ab\"; y=\"b\"; $display(\"r=%0d\", f(x, y)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=1"), "inline var string <, got:\n{out}");
}

#[test]
fn frame_automatic_var_lt() {
    // FRAME (automatic) function + string VAR actuals: the frame formal is a scoped net
    // not in subst, so it was packed-compared. Now lexicographic: "ab" < "b" = 1.
    let (out, code) = run("module m;\n\
         function automatic f(input string a, input string b); f = a < b; endfunction\n\
         string x, y;\n\
         initial begin x=\"ab\"; y=\"b\"; $display(\"r=%0d\", f(x, y)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=1"), "frame automatic string <, got:\n{out}");
}

#[test]
fn frame_bit_return_var_lt() {
    // A `bit` (2-state) return also routes to the frame path (ret_two_state). String
    // VAR formals must still compare lexicographically there.
    let (out, code) = run("module m;\n\
         function bit f(input string a, input string b); f = a < b; endfunction\n\
         string x, y;\n\
         initial begin x=\"ab\"; y=\"b\"; $display(\"r=%0d\", f(x, y)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=1"),
        "frame bit-return string <, got:\n{out}"
    );
}

#[test]
fn all_relational_ops_frame_var() {
    // `<`,`<=`,`>`,`>=` on "ab" vs "b" (frame, var) = 1,1,0,0 lexicographically.
    for (op, want) in [("<", 1), ("<=", 1), (">", 0), (">=", 0)] {
        let src = format!(
            "module m;\n\
             function automatic f(input string a, input string b); f = a {op} b; endfunction\n\
             string x, y;\n\
             initial begin x=\"ab\"; y=\"b\"; $display(\"r=%0d\", f(x, y)); #1 $finish; end endmodule\n"
        );
        let (out, code) = run(&src);
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains(&format!("r={want}")), "op {op}, got:\n{out}");
    }
}

#[test]
fn packed_formal_string_literal_stays_packed() {
    // OVER-TRIGGER GUARD: a string literal passed to a PACKED formal must NOT route to
    // StrCmp — the FORMAL's declared type governs. `c < d` with c="ab"(0x6162),
    // d="b"(0x0062) packed = 0x6162 < 0x0062 = 0. (iverilog agrees.)
    let (out, code) = run("module m;\n\
         function f(input [15:0] c, input [15:0] d); f = c < d; endfunction\n\
         initial begin $display(\"r=%0d\", f(\"ab\", \"b\")); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=0"),
        "packed formal stays packed, got:\n{out}"
    );
}
