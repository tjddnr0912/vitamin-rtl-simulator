//! Named SVA `sequence`/`property` FORMAL ARGUMENTS (Phase-3 slice A1). A declared
//! `sequence s(x,y); …; endsequence` / `property p(x,y); …; endproperty` may take
//! positional formal arguments; an instance `s(a,b)` / `assert property(p(a,b))`
//! binds the actuals by position and SUBSTITUTES them into the declared body before
//! the ordinary named-SVA desugar — a pure AST rewrite (IR-0, sim-ir frozen,
//! format_version 8; the `Sequence::Instance.args` / `*Decl.formals` fields were
//! reserved in the named-SVA slice, so this adds NO `.vu` AST-hash re-pin).
//!
//! iverilog 13.0 rejects concurrent assertions AND named property/sequence decls,
//! so this is HAND-IEEE pinned (IEEE 1800 §16.8/§16.12): a parameterized instance
//! must be BEHAVIORALLY equivalent to the inline body with its formals replaced by
//! the actual expressions.
//!
//! Subset boundary (LOUD, never silent): arity mismatch is an error; formal default
//! values are unsupported (all actuals must be passed); a parameterized SEQUENCE used
//! directly as a CONSEQUENT is out of scope (a named sequence as a bare consequent is
//! already a documented limitation). Sequence-call instances are supported in the
//! antecedent / nested in a sequence (the `s(a,b)` boolean-leaf `Call` path).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svafa_{}_{n}", std::process::id()));
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
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

// ── property formal arguments ──────────────────────────────────────────────

#[test]
fn parameterized_property_holds_no_error() {
    // property p(x,y); @(posedge clk) x |-> y; endproperty  +  assert property(p(a,b))
    // ≡ inline `@(posedge clk) a |-> b`. a=1,b=1 at every posedge → holds → clean.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         property p_imp(x,y); @(posedge clk) x |-> y; endproperty\n\
         initial assert property(p_imp(a,b));\n\
         initial begin #10 a=1; b=1; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "parameterized property holds → clean exit. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\nstderr={err}\nout={out}"
    );
}

#[test]
fn parameterized_property_violation_fires() {
    // a=1,b=0 at a posedge → a |-> b violated through the bound formals → $error.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         property p_imp(x,y); @(posedge clk) x |-> y; endproperty\n\
         initial assert property(p_imp(a,b));\n\
         initial begin #10 a=1; b=0; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "parameterized property violation must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "expected an assertion violation:\n{err}\n{out}"
    );
}

#[test]
fn formal_actually_substitutes_not_aliases() {
    // STRONG substitution proof: there are real nets `x`,`y` (both 1, so the body
    // would HOLD if the formals aliased the same-named nets) and the actuals are
    // `a`,`b` driven to a violating 1/0. Firing (exit 1) proves x→a, y→b — a true
    // substitution, not a name-aliasing accident.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, x=1, y=1;\n\
         always #5 clk=~clk;\n\
         property p_imp(x,y); @(posedge clk) x |-> y; endproperty\n\
         initial assert property(p_imp(a,b));\n\
         initial begin #10 a=1; b=0; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "formal substitution must read the actuals (a,b), not nets named x,y. \
         stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "expected an assertion violation proving substitution:\n{err}\n{out}"
    );
}

// ── sequence formal arguments (antecedent `s(a,b)` boolean-leaf Call path) ──

#[test]
fn parameterized_sequence_antecedent_violation_fires() {
    // sequence s(p,q); p ##1 q; endsequence  used as the ANTECEDENT:
    //   assert property(@(posedge clk) s(a,b) |-> c);  ≡  `a ##1 b |-> c`.
    // a=1 (seed), b=1 (next) → antecedent completes; c=0 at that posedge → fire.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=0;\n\
         always #5 clk=~clk;\n\
         sequence s_del(p,q); p ##1 q; endsequence\n\
         initial assert property(@(posedge clk) s_del(a,b) |-> c);\n\
         initial begin #30 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "parameterized sequence antecedent must complete and fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "expected an assertion violation:\n{err}\n{out}"
    );
}

#[test]
fn parameterized_sequence_antecedent_holds_no_error() {
    // Same sequence, but c=1 at the completion posedge → antecedent ⇒ c holds → clean.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=1, b=1, c=1;\n\
         always #5 clk=~clk;\n\
         sequence s_del(p,q); p ##1 q; endsequence\n\
         initial assert property(@(posedge clk) s_del(a,b) |-> c);\n\
         initial begin #30 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "antecedent ⇒ c with c=1 holds → clean exit. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "no violation expected:\nstderr={err}\nout={out}"
    );
}

// ── arity / boundary diagnostics ───────────────────────────────────────────

#[test]
fn too_few_sequence_arguments_is_loud() {
    let (_o, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         sequence s_del(p,q); p ##1 q; endsequence\n\
         initial assert property(@(posedge clk) s_del(a) |-> c);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_eq!(code, Some(1), "arity mismatch must be loud. {err}");
    assert!(
        err.contains("expects") && err.contains("argument"),
        "arity error should name the expected count:\n{err}"
    );
}

#[test]
fn too_many_property_arguments_is_loud() {
    let (_o, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         property p_imp(x); @(posedge clk) x |-> b; endproperty\n\
         initial assert property(p_imp(a,b));\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_eq!(code, Some(1), "too-many-args must be loud. {err}");
    assert!(
        err.contains("expects") && err.contains("argument"),
        "arity error should name the expected count:\n{err}"
    );
}

// ── adversarial-review regressions (2026-06-16) ────────────────────────────

#[test]
fn bare_top_level_parameterized_sequence_is_loud_not_silent() {
    // REVIEW (substitution/arity lens, HIGH): a BARE top-level use `s_del |-> c` of a
    // 2-formal sequence passes ZERO actuals. The formal names `p`,`q` SHADOW real
    // nets `p`,`q` (both 1) — pre-fix, resolve_named_top inlined the body against
    // those nets (silent-wrong). It must be a loud arity error, not a half-checker.
    let (_o, err, code) = run("module top;\n\
         reg clk=0, c=0, p=1, q=1;\n\
         always #5 clk=~clk;\n\
         sequence s_del(p,q); p ##1 q; endsequence\n\
         initial assert property(@(posedge clk) s_del |-> c);\n\
         initial begin #30 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "bare use of a parameterized sequence must be a loud arity error. {err}"
    );
    assert!(
        err.contains("expects") && err.contains("got 0"),
        "must name the missing actuals (not run against shadow nets):\n{err}"
    );
}

#[test]
fn positional_module_named_sequence_before_a_decl_still_instantiates() {
    // REVIEW (parser-masking lens, HIGH): a positional instantiation of a module
    // literally named `sequence` must STILL parse even when a real
    // `sequence … endsequence` decl follows it in the same module — the
    // disambiguation must not be poisoned by the later decl's `endsequence`.
    let (out, err, code) = run("module sequence(output o);\n\
         assign o = 1'b1;\n\
         endmodule\n\
         module top;\n\
         reg clk=0, a=0, b=0; wire o;\n\
         always #5 clk=~clk;\n\
         sequence u(o);\n\
         sequence s_ab; a ##1 b; endsequence\n\
         initial assert property(@(posedge clk) s_ab |-> 1'b1);\n\
         initial begin #1 $display(\"o=%b\", o); #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "positional `sequence u(o);` instantiation must parse despite a later decl. {err}{out}"
    );
    assert!(
        out.contains("o=1"),
        "the instantiated module must drive o=1:\n{out}"
    );
}

#[test]
fn long_bodied_parameterized_sequence_decl_is_recognized() {
    // REVIEW (parser-masking lens, MEDIUM): a valid parameterized sequence whose body
    // is long (>512 tokens, the old budget) must still be recognized as a decl, not
    // flipped to a malformed instantiation. Build a ~300-term `x ##1 x ##1 …` body.
    let body = std::iter::repeat("x")
        .take(300)
        .collect::<Vec<_>>()
        .join(" ##1 ");
    // The long parameterized decl is declared (and unused); recognition is what is
    // under test — a misroute to an instantiation chokes on `##1`.
    let src = format!(
        "module top;\n\
         reg clk=0, x=1, a=1, b=1;\n\
         always #5 clk=~clk;\n\
         sequence s_long(x); {body}; endsequence\n\
         initial assert property(@(posedge clk) a |-> b);\n\
         initial begin #20 $finish; end\n\
         endmodule\n"
    );
    let (_o, err, code) = run(&src);
    // The decl must PARSE (no E2002 "expected identifier, found HashHash" cascade).
    assert!(
        !err.contains("found HashHash"),
        "a long-bodied sequence decl must be recognized, not parsed as an instantiation:\n{err}"
    );
    assert_eq!(
        code,
        Some(0),
        "the unrelated `a |-> b` holds → clean exit; the long decl just parses. {err}"
    );
}

#[test]
fn malformed_formal_list_is_loud() {
    // REVIEW (parser-masking lens, LOW): an empty formal entry (leading/trailing/
    // double comma) must be a loud diagnostic, not silently dropped.
    let (_o, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         sequence s_bad(a,); a ##1 b; endsequence\n\
         initial assert property(@(posedge clk) a |-> b);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_eq!(code, Some(1), "malformed formal list must be loud. {err}");
    assert!(
        err.contains("formal name"),
        "an empty formal entry should be diagnosed:\n{err}"
    );
}
