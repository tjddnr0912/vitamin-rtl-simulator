//! Enum value methods: `.first` `.last` `.num` `.next` `.prev` (SV §6.19.5).
//!
//! vita parse-routed `x.first` etc. as undeclared hierarchical names (E3010);
//! iverilog supports them. They desugar in the PARSER over the enum's folded
//! label values: `.first`/`.last`/`.num` → integer literals; `.next`/`.prev` →
//! a ternary chain (next wraps last→first, prev wraps first→last). No AST/IR
//! change (existing IntLit / Ternary / `==` nodes); a loop with no enum methods
//! is byte-identical. `.name()` is intentionally NOT desugared (a packed
//! string-literal ternary pads shorter labels, unlike iverilog's dynamic
//! string) → it stays a loud error. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_enumm_{}_{n}", std::process::id()));
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
fn first_last_num() {
    let (out, code) = run("module top; typedef enum {A=2,B=5,C=9} e; e x;\n\
         initial begin x=A; $display(\"R first=%0d last=%0d num=%0d\",x.first,x.last,x.num); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("R first=2 last=9 num=3"),
        "first/last/num; got:\n{out}"
    );
}

#[test]
fn next_prev_with_wrap() {
    // default-numbered {A,B,C}=0,1,2. next(B)=C=2; prev(B)=A=0; next(C) wraps→A=0;
    // prev(A) wraps→C=2.
    let (out, code) = run("module top; typedef enum {A,B,C} e; e x;\n\
         initial begin\n\
           x=B; $display(\"R nx=%0d pv=%0d\",x.next,x.prev);\n\
           x=C; $display(\"R wrapn=%0d\",x.next);\n\
           x=A; $display(\"R wrapp=%0d\",x.prev); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(out.contains("R nx=2 pv=0"), "next/prev; got:\n{out}");
    assert!(
        out.contains("R wrapn=0"),
        "next wrap last→first; got:\n{out}"
    );
    assert!(
        out.contains("R wrapp=2"),
        "prev wrap first→last; got:\n{out}"
    );
}

#[test]
fn next_steps_an_fsm() {
    // .next drives an FSM through its states (the common use). IDLE,RUN,DONE → 0,1,2.
    let (out, code) = run("module top; typedef enum {IDLE,RUN,DONE} st; st s; integer i;\n\
         initial begin s=IDLE; for(i=0;i<4;i=i+1) begin $display(\"R s=%0d\",s); s=s.next; end $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("R s=0\nR s=1\nR s=2\nR s=0"),
        "fsm next; got:\n{out}"
    );
}

#[test]
fn valued_next_and_negative() {
    // next(B=5)=C=9; first of a negative-valued enum; next(A=-2)=B=0.
    let (out, code) = run("module top; typedef enum integer {A=-2,B=0,C=3} e; e x;\n\
         initial begin x=A; $display(\"R f=%0d nx=%0d\",x.first,x.next); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("R f=-2 nx=0"),
        "negative-valued enum; got:\n{out}"
    );
}

#[test]
fn single_label_enum() {
    // A one-label enum: num=1, next/prev wrap to itself.
    let (out, code) = run("module top; typedef enum {ONLY} e; e x;\n\
         initial begin x=ONLY; $display(\"R n=%0d nx=%0d pv=%0d\",x.num,x.next,x.prev); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("R n=1 nx=0 pv=0"),
        "single-label enum; got:\n{out}"
    );
}

#[test]
fn name_method_is_loud() {
    // `.name`/`.name()` is honest-loud (the packed string-ternary desugar would
    // pad variable-length labels, unlike iverilog's dynamic string).
    let (_o, code) = run("module top; typedef enum {A,B} e; e x;\n\
         initial begin x=A; $display(\"%s\",x.name); $finish; end endmodule\n");
    assert_ne!(code, Some(0), "`.name` must stay loud, not silently pad");
}

#[test]
fn method_on_non_enum_is_loud() {
    // A method-name access on a NON-enum variable stays a loud error.
    let (_o, code) = run("module top; integer x;\n\
         initial begin $display(\"%0d\",x.first); $finish; end endmodule\n");
    assert_ne!(code, Some(0), "x.first on a non-enum must be loud");
}

#[test]
fn overflowing_label_arithmetic_is_loud_not_panic() {
    // An enum label whose value overflows i64 during const-folding must be a LOUD
    // error, never a parser panic (const_lit uses checked arithmetic). Adversarial
    // review surfaced this: the new enum-fold path calls const_lit.
    let (_o, code) = run(
        "module top; typedef enum integer {A=9223372036854775807+1,B} e; e x;\n\
         initial begin x=A; $display(\"%0d\",x.first); $finish; end endmodule\n",
    );
    assert_ne!(
        code,
        Some(0),
        "overflowing enum label must be loud, not a panic"
    );
}

#[test]
fn non_foldable_enum_methods_are_loud() {
    // An enum whose label value is not literal-foldable (references a parameter)
    // is omitted from enum_defs, so its methods stay loud (correct-or-loud).
    let (_o, code) = run(
        "module top; parameter P=4; typedef enum integer {A=P,B=P+1} e; e x;\n\
         initial begin x=A; $display(\"%0d\",x.first); $finish; end endmodule\n",
    );
    assert_ne!(code, Some(0), "non-foldable enum methods must be loud");
}
