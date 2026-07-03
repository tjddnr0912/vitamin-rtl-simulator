//! TASK `string` formals: two silent-wrongs (mirror of the frame-FUNCTION fixes
//! §4.5.81 + §4.5.87, at the task binding path — `run_task`, not `Expr::Call`).
//!
//! (A) A `string` relational compare (`a < b`) in a task BODY did a PACKED compare
//! (non-lexicographic) because `lower_frame_task_body` never pushed the task's
//! `string` formals to `formal_str` (the function body did, §4.5.81). So even a
//! string VAR actual compared wrong. (B) A `string` LITERAL actual was truncated to
//! the formal's 1-bit `Wire` slot at the `run_task` copy-in (`resize_keep_sign` to
//! width 1), because the task path never marked `string` formals in
//! `FuncMeta.str_params` (the function path did, §4.5.87).
//!
//! Fix: (A) push task `string` formals to `formal_str` in `lower_frame_task_body`;
//! (B) populate `str_params` in `reserve_frame_task` and materialise a heap-string
//! value at the `run_task` copy-in for a marked input formal. "ab" vs "b": packed
//! low-byte compare = 0, lexicographic = 1 — unambiguous. Pinned to iverilog 13.0.
//!
//! Only `automatic`/recursive tasks take the (fixed) frame path; a plain STATIC
//! task takes `inline_task`, whose 1-bit-slot copy-in would silently truncate a
//! string formal — so that path LOUD-rejects a `string` formal (declare it
//! `automatic`) rather than diverge from the correct frame path.
//!
//! Out of scope (loud, not silent — separate paths): a `string` OUTPUT/INOUT formal
//! (assign to a String net in a frame body = E3018); `$display`/methods in a task
//! body (E3009); a `string` formal beyond param index 63 (loud-rejected here too).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_taskstrlit_{}_{n}", std::process::id()));
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

/// `task automatic t(input string a, input string b, output logic r); r=(a OP b);`
/// called with the two given actuals (each already a full SV expression).
fn task_cmp(op: &str, a_actual: &str, b_actual: &str, decls: &str) -> String {
    format!(
        "module m;\n\
         task automatic t(input string a, input string b, output logic r); r=(a {op} b); endtask\n\
         logic x;{decls}\n\
         initial begin t({a_actual}, {b_actual}, x); $display(\"r=%0d\", x); #1 $finish; end\n\
         endmodule\n"
    )
}

#[test]
fn task_literal_all_relational_ops() {
    // (B)+(A): LITERAL actuals, "ab" vs "b" = 1,1,0,0 lexicographically (was all
    // wrong: truncated slot AND packed compare).
    for (op, want) in [("<", 1), ("<=", 1), (">", 0), (">=", 0)] {
        let (out, code) = run(&task_cmp(op, "\"ab\"", "\"b\"", ""));
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains(&format!("r={want}")), "op {op}, got:\n{out}");
    }
}

#[test]
fn task_literal_eq_neq() {
    // "ab" == "b" is 0, "ab" != "b" is 1. A truncated slot made both operands "b".
    for (op, want) in [("==", 0), ("!=", 1)] {
        let (out, code) = run(&task_cmp(op, "\"ab\"", "\"b\"", ""));
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains(&format!("r={want}")), "op {op}, got:\n{out}");
    }
}

#[test]
fn task_var_relational_routes_lexicographic() {
    // (A) alone: string VAR actuals, no truncation involved — but the compare was
    // still PACKED before the formal_str push. "ab" < "b" = 1 lexicographically.
    let src = "module m;\n\
         task automatic t(input string a, input string b, output logic r); r=(a<b); endtask\n\
         logic x; string p, q;\n\
         initial begin p=\"ab\"; q=\"b\"; t(p, q, x); $display(\"r=%0d\", x); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=1"), "task var relational, got:\n{out}");
}

#[test]
fn task_literal_longer_strings() {
    for (a, b, want) in [("apple", "banana", 1), ("banana", "apple", 0)] {
        let (out, code) = run(&task_cmp("<", &format!("\"{a}\""), &format!("\"{b}\""), ""));
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains(&format!("r={want}")), "{a}<{b}, got:\n{out}");
    }
}

#[test]
fn task_mixed_var_and_literal() {
    // One VAR + one LITERAL: p="ab" (heap), "b" (literal) → "ab" < "b" = 1.
    let src = "module m;\n\
         task automatic t(input string a, input string b, output logic r); r=(a<b); endtask\n\
         logic x; string p;\n\
         initial begin p=\"ab\"; t(p, \"b\", x); $display(\"r=%0d\", x); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=1"), "task mixed var/literal, got:\n{out}");
}

#[test]
fn task_string_at_nonzero_arg_index() {
    // BITMASK-INDEX GUARD: the string formal is at port index 1 (an `int` precedes
    // it), so str_params must set bit 1. f(1,"hi") = 1 && ("hi"=="hi").
    let src = "module m;\n\
         task automatic t(input int k, input string s, output logic r); r=(k!=0)&&(s==\"hi\"); endtask\n\
         logic x;\n\
         initial begin t(1, \"hi\", x); $display(\"r=%0d\", x); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=1"),
        "string at task arg index 1, got:\n{out}"
    );
}

#[test]
fn task_nested_call_literal() {
    // The nested task-call path (run_task's own Terminator::Call) must also
    // materialise the literal: outer forwards to inner, both string formals.
    let src = "module m;\n\
         task automatic inner(input string a, input string b, output logic r); r=(a<b); endtask\n\
         task automatic outer(input string a, input string b, output logic r); inner(a, b, r); endtask\n\
         logic x;\n\
         initial begin outer(\"ab\", \"b\", x); $display(\"r=%0d\", x); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=1"),
        "nested task string literal, got:\n{out}"
    );
}

#[test]
fn task_packed_literal_actual_to_string_formal() {
    // A PACKED literal (8'h62 = 'b') to a string formal is denoted-string "b": == 1.
    let (out, code) = run(&task_cmp("==", "8'h62", "\"b\"", ""));
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=1"),
        "packed actual to task string formal, got:\n{out}"
    );
}

#[test]
fn non_string_task_formal_unaffected() {
    // OVER-TRIGGER GUARD: a PACKED formal is not marked → byte-identical binding.
    let src = "module m;\n\
         task automatic t(input int a, input int b, output int s); s=a*b; endtask\n\
         int r;\n\
         initial begin t(200, 200, r); $display(\"%0d\", r); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("40000"), "non-string task formal, got:\n{out}");
}

#[test]
fn task_string_formal_beyond_index_63_loud_rejects() {
    // The str_params mask is 64 wide; a `string` formal past index 63 is loud-
    // rejected (not silently truncated). Build a 65-formal task, string at index 64.
    let mut ports = String::new();
    let mut actuals = String::new();
    for i in 0..64 {
        ports.push_str(&format!("input int a{i}, "));
        actuals.push_str("0, ");
    }
    ports.push_str("input string s, output logic r");
    actuals.push_str("\"b\", x");
    let src = format!(
        "module m;\n\
         task automatic t({ports}); r=(s==\"b\"); endtask\n\
         logic x;\n\
         initial begin t({actuals}); $display(\"r=%0d\", x); #1 $finish; end endmodule\n"
    );
    let (out, code) = run(&src);
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

#[test]
fn static_task_string_formal_loud_rejects() {
    // Only `automatic`/recursive tasks take the (fixed) frame path. A plain STATIC
    // task takes `inline_task`, whose 1-bit-slot copy-in would truncate a string
    // formal AND skip StrCmp routing — both silent. Rather than diverge from the
    // now-correct automatic path, the static path loud-rejects a `string` formal
    // (declare it `automatic`). Both a LITERAL and a VAR actual must be loud, never
    // a silent r=0.
    for actual in ["\"zoo\"", "v"] {
        let src = format!(
            "module m;\n\
             task t(input string s, output int e); e = (s == \"zoo\"); endtask\n\
             int e; string v;\n\
             initial begin v=\"zoo\"; t({actual}, e); $display(\"r=%0d\", e); #1 $finish; end endmodule\n"
        );
        let (out, code) = run(&src);
        assert_eq!(code, Some(1), "expected loud reject for {actual}\n{out}");
        assert!(!out.contains("r="), "must not print a silent result\n{out}");
    }
}

#[test]
fn non_string_static_task_unaffected() {
    // GUARD: the static-task string loud-reject must not touch non-string static
    // tasks (they keep the inline path). `t(12,11)` = 132.
    let src = "module m;\n\
         task t(input int a, input int b, output int s); s = a * b; endtask\n\
         int r;\n\
         initial begin t(12, 11, r); $display(\"%0d\", r); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("132"), "non-string static task, got:\n{out}");
}
