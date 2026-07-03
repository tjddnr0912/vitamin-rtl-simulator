//! CLASS-METHOD `string` formals: two silent-wrongs, mirror of the frame-FUNCTION
//! fixes §4.5.81 + §4.5.87 (and the task fix §4.5.88), at the method binding path.
//!
//! A method call lowers to `Expr::Call{func, args:[this, a0, …]}` — the SAME runtime
//! path as a frame function, but the frame prepends a `this` slot (slot 0), so formal
//! `i` lives at slot `1+i`. Before this fix `reserve_class_method` left `str_params`
//! empty and `lower_class_method_body` never pushed the method's `string` formals to
//! `formal_str`, so: (A) a relational compare (`a < b`) in the body did a PACKED
//! (non-lexicographic) compare — wrong even for a string VAR actual; (B) a string
//! LITERAL actual was truncated to the formal's 1-bit `Wire` slot.
//!
//! Fix: mark each `string` formal in `str_params` at its SLOT index `1+i` (so it
//! aligns with the `this`-prefixed arg order — the existing eval.rs materialization
//! then binds a heap string), and push the formals to `formal_str` for StrCmp
//! routing. "ab" vs "b": packed low-byte compare = 0, lexicographic = 1. Pinned to
//! iverilog 13.0 (except virtual dispatch through a base handle, where iverilog's
//! vtable is buggy even for non-string returns — vita is IEEE-correct there).
//!
//! Out of scope (loud): output/inout method ports (E3009, N7 MVP); a `string` formal
//! beyond slot 63 (loud-rejected here too).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_methstrlit_{}_{n}", std::process::id()));
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

/// `class C; function bit mm(input string a, input string b); return (a OP b); …`
/// called `c.mm("ab","b")`.
fn method_cmp(op: &str) -> String {
    format!(
        "module m;\n\
         class C; function bit mm(input string a, input string b); return (a {op} b); endfunction endclass\n\
         C c;\n\
         initial begin c=new; $display(\"r=%0d\", c.mm(\"ab\", \"b\")); #1 $finish; end endmodule\n"
    )
}

#[test]
fn method_literal_all_relational_ops() {
    for (op, want) in [("<", 1), ("<=", 1), (">", 0), (">=", 0)] {
        let (out, code) = run(&method_cmp(op));
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains(&format!("r={want}")), "op {op}, got:\n{out}");
    }
}

#[test]
fn method_literal_eq_neq() {
    for (op, want) in [("==", 0), ("!=", 1)] {
        let (out, code) = run(&method_cmp(op));
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains(&format!("r={want}")), "op {op}, got:\n{out}");
    }
}

#[test]
fn method_var_relational_routes_lexicographic() {
    // (routing) string VAR actuals — no truncation, but the compare was PACKED
    // before the formal_str push. "ab" < "b" = 1 lexicographically.
    let src = "module m;\n\
         class C; function bit mm(input string a, input string b); return (a<b); endfunction endclass\n\
         C c; string p, q;\n\
         initial begin c=new; p=\"ab\"; q=\"b\"; $display(\"r=%0d\", c.mm(p, q)); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=1"), "method var relational, got:\n{out}");
}

#[test]
fn method_literal_longer_strings() {
    for (a, b, want) in [("apple", "banana", 1), ("banana", "apple", 0)] {
        let src = format!(
            "module m;\n\
             class C; function bit mm(input string a, input string b); return (a<b); endfunction endclass\n\
             C c;\n\
             initial begin c=new; $display(\"r=%0d\", c.mm(\"{a}\", \"{b}\")); #1 $finish; end endmodule\n"
        );
        let (out, code) = run(&src);
        assert_eq!(code, Some(0), "{out}");
        assert!(out.contains(&format!("r={want}")), "{a}<{b}, got:\n{out}");
    }
}

#[test]
fn method_string_at_nonzero_arg_index() {
    // THIS-OFFSET GUARD: `this` is slot 0, `int k` is formal 0 (slot 1), `string s`
    // is formal 1 (slot 2). str_params must set bit 2 (not bit 1). f(1,"hi") = 1.
    let src = "module m;\n\
         class C; function bit mm(input int k, input string s); return (k!=0)&&(s==\"hi\"); endfunction endclass\n\
         C c;\n\
         initial begin c=new; $display(\"r=%0d\", c.mm(1, \"hi\")); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=1"),
        "string at method arg index 1, got:\n{out}"
    );
}

#[test]
fn method_nested_and_self_and_super_calls() {
    // A method calling another method (bare self-call) and a super. call must all
    // forward the string formal correctly (each is its own Expr::Call).
    let nested = "module m;\n\
         class C; function bit inner(input string a, input string b); return (a<b); endfunction\n\
          function bit outer(input string a, input string b); return inner(a, b); endfunction endclass\n\
         C c;\n\
         initial begin c=new; $display(\"r=%0d\", c.outer(\"ab\", \"b\")); #1 $finish; end endmodule\n";
    let (out, code) = run(nested);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=1"), "nested method, got:\n{out}");

    let sup = "module m;\n\
         class B; function bit f(input string s); return (s==\"hi\"); endfunction endclass\n\
         class D extends B; function bit f(input string s); return super.f(s); endfunction endclass\n\
         D d;\n\
         initial begin d=new; $display(\"r=%0d\", d.f(\"hi\")); #1 $finish; end endmodule\n";
    let (out, code) = run(sup);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("r=1"), "super call, got:\n{out}");
}

#[test]
fn method_virtual_dispatch_string_formal() {
    // Virtual dispatch through a base handle to a derived object must call D::f with
    // a correctly-bound string formal: h.f("deri") = ("deri"=="deri") = 1. (iverilog
    // mis-dispatches to B::f here even for non-string returns; vita is IEEE-correct.)
    let src = "module m;\n\
         class B; virtual function bit f(input string s); return (s==\"base\"); endfunction endclass\n\
         class D extends B; virtual function bit f(input string s); return (s==\"deri\"); endfunction endclass\n\
         B h; D d;\n\
         initial begin d=new; h=d; $display(\"r=%0d\", h.f(\"deri\")); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("r=1"),
        "virtual dispatch string formal, got:\n{out}"
    );
}

#[test]
fn non_string_method_formal_unaffected() {
    // OVER-TRIGGER GUARD: a non-string method is byte-identical. mm(12,11) = 132.
    let src = "module m;\n\
         class C; function int mm(input int a, input int b); return a*b; endfunction endclass\n\
         C c;\n\
         initial begin c=new; $display(\"%0d\", c.mm(12, 11)); #1 $finish; end endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("132"), "non-string method formal, got:\n{out}");
}

#[test]
fn method_string_formal_beyond_slot_63_loud_rejects() {
    // `this` takes slot 0, so a `string` formal at declared index 63 sits at slot 64,
    // past the 64-wide mask → loud-reject (not silent truncation). Build a 64-formal
    // method with 63 ints (indices 0..62) then a string at index 63.
    let mut ports = String::new();
    let mut actuals = String::new();
    for i in 0..63 {
        ports.push_str(&format!("input int a{i}, "));
        actuals.push_str("0, ");
    }
    ports.push_str("input string s");
    actuals.push_str("\"b\"");
    let src = format!(
        "module m;\n\
         class C; function bit mm({ports}); return (s==\"b\"); endfunction endclass\n\
         C c;\n\
         initial begin c=new; $display(\"r=%0d\", c.mm({actuals})); #1 $finish; end endmodule\n"
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
