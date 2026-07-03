//! A `string` LITERAL actual passed to a FRAME function's `string` formal was
//! silently truncated. A `string` formal lowers to a 1-bit `Wire` slot (a string
//! is a dynamic handle, not a fixed width); a string VAR actual is a heap-string
//! `Value` that `resize_keep_sign` preserves (is_str short-circuit), so it
//! survived — but a string LITERAL actual is a PACKED const that `Expr::Call` arg
//! binding evaluated at the 1-bit formal width and stored truncated. Result:
//! `f("ab","b")` on `function automatic f(input string a, input string b); f=a<b;`
//! gave 0 (the low byte of each = "b" vs "b") instead of 1 ("ab" < "b"
//! lexicographically). The fix marks each `string` formal in `FuncMeta.str_params`
//! and, for a marked formal, evaluates the actual at its NATURAL width then
//! materialises a heap-string value before binding. Comparison routing (StrCmp)
//! was already correct (elaborate `formal_str`); only the bound VALUE was wrong.
//! Pinned to iverilog 13.0. "ab" vs "b": packed low-byte compare = 0, lexicographic
//! = 1 — unambiguous.
//!
//! Out of scope (follow-ons): a `$display`/foreach in a frame body (loud E3009,
//! outside the frame-call subset); CLASS-METHOD string formals (the frame prepends
//! a `this` slot, offsetting formal indices vs `Expr::Call` arg order — needs its
//! own grounding); TASK string formals (bind via `run_task`, not `Expr::Call`).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_frmstrlit_{}_{n}", std::process::id()));
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
fn frame_automatic_literal_lt() {
    // The core bug: FRAME (automatic) function, BOTH operands string LITERAL
    // actuals. "ab" < "b" = 1 lexicographically (was 0 from a truncated slot).
    let (out, code) = run("module m;\n\
         function automatic f(input string a, input string b); f = a < b; endfunction\n\
         initial begin $display(\"r=%0d\", f(\"ab\", \"b\")); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=1"), "frame literal string <, got:\n{out}");
}

#[test]
fn all_relational_ops_frame_literal() {
    // `<`,`<=`,`>`,`>=` on "ab" vs "b" (frame, LITERAL) = 1,1,0,0 lexicographically.
    for (op, want) in [("<", 1), ("<=", 1), (">", 0), (">=", 0)] {
        let src = format!(
            "module m;\n\
             function automatic f(input string a, input string b); f = a {op} b; endfunction\n\
             initial begin $display(\"r=%0d\", f(\"ab\", \"b\")); #1 $finish; end endmodule\n"
        );
        let (out, code) = run(&src);
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains(&format!("r={want}")), "op {op}, got:\n{out}");
    }
}

#[test]
fn frame_literal_eq_neq() {
    // Equality on unequal-length strings: "ab" == "b" is 0, "ab" != "b" is 1. A
    // truncated slot would make both operands "b" → ==1/!=0 (the pre-fix wrong).
    for (op, want) in [("==", 0), ("!=", 1)] {
        let src = format!(
            "module m;\n\
             function automatic f(input string a, input string b); f = a {op} b; endfunction\n\
             initial begin $display(\"r=%0d\", f(\"ab\", \"b\")); #1 $finish; end endmodule\n"
        );
        let (out, code) = run(&src);
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains(&format!("r={want}")), "op {op}, got:\n{out}");
    }
}

#[test]
fn frame_literal_longer_strings() {
    // Longer literals (would be badly truncated to 1 byte pre-fix): "apple" <
    // "banana" = 1, "banana" < "apple" = 0.
    for (a, b, want) in [("apple", "banana", 1), ("banana", "apple", 0)] {
        let src = format!(
            "module m;\n\
             function automatic f(input string a, input string b); f = a < b; endfunction\n\
             initial begin $display(\"r=%0d\", f(\"{a}\", \"{b}\")); #1 $finish; end endmodule\n"
        );
        let (out, code) = run(&src);
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains(&format!("r={want}")), "{a}<{b}, got:\n{out}");
    }
}

#[test]
fn frame_var_actual_unchanged() {
    // REGRESSION GUARD: a string VAR actual worked before (is_str survives resize);
    // it must still be lexicographic after the fix.
    let (out, code) = run("module m;\n\
         function automatic f(input string a, input string b); f = a < b; endfunction\n\
         string x, y;\n\
         initial begin x=\"ab\"; y=\"b\"; $display(\"r=%0d\", f(x, y)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=1"), "frame var string <, got:\n{out}");
}

#[test]
fn frame_mixed_var_and_literal() {
    // One VAR + one LITERAL actual: x="ab" (heap), "b" (literal) → "ab" < "b" = 1.
    let (out, code) = run("module m;\n\
         function automatic f(input string a, input string b); f = a < b; endfunction\n\
         string x;\n\
         initial begin x=\"ab\"; $display(\"r=%0d\", f(x, \"b\")); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=1"),
        "frame mixed var/literal <, got:\n{out}"
    );
}

#[test]
fn frame_string_at_nonzero_arg_index() {
    // BITMASK-INDEX GUARD: the string formal is at arg index 1 (an `int` precedes
    // it), so `str_params` must set bit 1, not bit 0. f(1,"ab","b") = 1 && ("ab"<"b").
    let (out, code) = run("module m;\n\
         function automatic f(input int p, input string a, input string b);\n\
             f = (p != 0) && (a < b);\n\
         endfunction\n\
         initial begin $display(\"r=%0d\", f(1, \"ab\", \"b\")); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=1"), "string at arg index 1, got:\n{out}");
}

#[test]
fn non_string_frame_formal_unaffected() {
    // OVER-TRIGGER GUARD: a PACKED formal is not marked in `str_params`, so its
    // binding is byte-identical — `f(200,200)` = 200*200 = 40000 (no materialize).
    let (out, code) = run("module m;\n\
         function automatic [15:0] f(input [7:0] a, input [7:0] b); f = a * b; endfunction\n\
         initial begin $display(\"%0d\", f(8'd200, 8'd200)); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("40000"),
        "packed formal unaffected, got:\n{out}"
    );
}

#[test]
fn frame_packed_literal_actual_to_string_formal() {
    // A PACKED literal (8'h62 = 'b') passed to a string formal is denoted-string
    // "b" (§6.16): f(8'h62,"b") with `a==b` = 1. Guards the natural-width eval +
    // to_str_bytes path for a non-string-literal actual.
    let (out, code) = run("module m;\n\
         function automatic f(input string a, input string b); f = a == b; endfunction\n\
         initial begin $display(\"r=%0d\", f(8'h62, \"b\")); #1 $finish; end endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=1"),
        "packed actual to string formal, got:\n{out}"
    );
}

// The `str_params` mask is 64 wide. A `string` formal AT index 63 is the last one
// that fits (must work); a `string` formal beyond index 63 cannot be marked, so
// rather than silently truncate its literal actual, elaborate loud-rejects the
// (pathological 65+-param) function. Verifies correct-or-loud at the mask boundary.
fn nparam_string_at(idx: usize, total: usize) -> String {
    let mut ports = String::new();
    for i in 0..total {
        if i > 0 {
            ports.push_str(", ");
        }
        if i == idx {
            ports.push_str("input string s");
        } else {
            ports.push_str(&format!("input int a{i}"));
        }
    }
    let mut actuals = String::new();
    for i in 0..total {
        if i > 0 {
            actuals.push_str(", ");
        }
        if i == idx {
            actuals.push_str("\"b\"");
        } else {
            actuals.push('0');
        }
    }
    format!(
        "module m;\n\
         function automatic f({ports}); f = (s == \"b\"); endfunction\n\
         initial begin $display(\"r=%0d\", f({actuals})); #1 $finish; end endmodule\n"
    )
}

#[test]
fn string_formal_at_index_63_works() {
    // Index 63 = the last markable bit (64-param function). Must work: r=1.
    let (out, code) = run(&nparam_string_at(63, 64));
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=1"),
        "string formal at index 63, got:\n{out}"
    );
}

#[test]
fn string_formal_beyond_index_63_loud_rejects() {
    // Index 64 (65-param function) exceeds the mask → must be a LOUD reject (nonzero
    // exit), NOT a silent truncated `r=0`. Correct-or-loud at the boundary.
    let (out, code) = run(&nparam_string_at(64, 65));
    assert_eq!(
        code,
        Some(1),
        "expected loud reject, got code={code:?}\n{out}"
    );
    assert!(
        !out.contains("r=0"),
        "must not silently compute r=0, got:\n{out}"
    );
}
