//! Named SVA `sequence`/`property` declarations + instantiation (Phase-3 named-SVA
//! slice). `sequence s; …; endsequence` / `property p; …; endproperty` are stored
//! at elaborate and INLINED at each use site, reusing the existing synthesized-
//! clocked-checker desugar — pure IR-0 (sim-ir frozen, format_version 8; only the
//! `.vu` AST-hash re-pins). iverilog 13.0 rejects concurrent assertions AND named
//! property/sequence decls, so this is HAND-IEEE pinned: each named form must be
//! BEHAVIORALLY equivalent to the inline form it desugars to.
//!
//! MVP scope: module-scope. Formal arguments are now supported (slice A1, in
//! `sva_formal_args.rs`): `sequence s(x,y); …; endsequence` + `s(a,b)` binds the
//! actuals by position and substitutes them into the body before the desugar.
//! Known subset limitations (all LOUD, never silent-wrong; deferred follow-ons):
//!   - a decl inside a `generate` block is not collected (the prescan does not
//!     recurse into generate) — the reference is then a loud "undeclared net".
//!   - a property body that references another named PROPERTY is unsupported (a
//!     property body referencing a named SEQUENCE works) — loud "undeclared net".
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svan_{}_{n}", std::process::id()));
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

#[test]
fn named_property_instance_holds_no_error() {
    // `property p; @(posedge clk) a |-> b; endproperty` + `assert property(p);`
    // must behave like the inline form: a |-> b holds at every posedge → clean.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         property p_ab; @(posedge clk) a |-> b; endproperty\n\
         initial assert property(p_ab);\n\
         initial begin #10 a=1; b=1; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "named property holds → clean exit. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !err.to_lowercase().contains("assertion") && !out.to_lowercase().contains("assertion"),
        "no violation expected:\nstderr={err}\nout={out}"
    );
}

#[test]
fn named_property_instance_violation_fires() {
    // a=1,b=0 at a posedge → a |-> b violated through the named property → $error.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         property p_ab; @(posedge clk) a |-> b; endproperty\n\
         initial assert property(p_ab);\n\
         initial begin #10 a=1; b=0; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "named property violation must fire. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "expected an assertion violation:\n{err}\n{out}"
    );
}

#[test]
fn named_property_instance_carries_its_own_clock() {
    // The named property is clocked @(posedge clk); the instance must check on
    // posedge clk (not the empty sentinel). A violation at a posedge proves the
    // real clock was spliced. (Same trace as the violation test, distinct module.)
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         property p_ab; @(posedge clk) a |-> b; endproperty\n\
         initial assert property(p_ab);\n\
         initial begin #12 a=1; b=0; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "instance must be clocked by the named property's clock. {err}{out}"
    );
}

// ── named SEQUENCE instance ≡ the inline sequence it desugars to ──
// Mirrors sva_property.rs::sva_seq_delay_violation_fires / _holds_no_error, but the
// `a ##1 b ##1 c` antecedent is a NAMED sequence. Identical traces → identical
// outcomes proves the inline-at-elaborate path is behaviorally equivalent.

#[test]
fn named_sequence_equiv_inline_violation_fires() {
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         sequence s_abc; a ##1 b ##1 c; endsequence\n\
         initial assert property(@(posedge clk) s_abc |-> d);\n\
         initial begin\n\
           #10 a=1; b=0; c=0; d=0;\n\
           #10 a=0; b=1; c=0; d=0;\n\
           #10 a=0; b=0; c=1; d=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "named-seq completion with low consequent must fire. {err}{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}{out}"
    );
}

#[test]
fn named_sequence_equiv_inline_holds() {
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         sequence s_abc; a ##1 b ##1 c; endsequence\n\
         initial assert property(@(posedge clk) s_abc |-> d);\n\
         initial begin\n\
           #10 a=1; b=0; c=0; d=0;\n\
           #10 a=0; b=1; c=0; d=0;\n\
           #10 a=0; b=0; c=1; d=1;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "named-seq completion with high consequent holds. {err}{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "{err}{out}"
    );
}

#[test]
fn nested_sequence_instance_in_delay_chain() {
    // `s_ab ##1 c` where `s_ab` = `a ##1 b` ⇒ the full `a ##1 b ##1 c |-> d` —
    // exercises the Delay-arm recursion through a sequence instance.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         sequence s_ab; a ##1 b; endsequence\n\
         initial assert property(@(posedge clk) s_ab ##1 c |-> d);\n\
         initial begin\n\
           #10 a=1; b=0; c=0; d=0;\n\
           #10 a=0; b=1; c=0; d=0;\n\
           #10 a=0; b=0; c=1; d=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "nested seq-instance must complete and fire. {err}{out}"
    );
}

#[test]
fn forward_reference_resolves() {
    // `assert property(p)` textually BEFORE `property p; … endproperty` — the
    // whole-body prescan makes the forward reference resolve.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(p_fwd);\n\
         property p_fwd; @(posedge clk) a |-> b; endproperty\n\
         initial begin #10 a=1; b=1; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "forward reference must resolve. {err}{out}");
}

#[test]
fn net_and_sequence_resolve_independently() {
    // `b` is a real net (a boolean leaf); `s_a` is a sequence. In one property
    // both appear — the sequence inlines, the net stays a leaf. No collision.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, d=0;\n\
         always #5 clk=~clk;\n\
         sequence s_a; a; endsequence\n\
         initial assert property(@(posedge clk) (s_a ##1 b) |-> d);\n\
         initial begin\n\
           #10 a=1; b=0; d=0;\n\
           #10 a=0; b=1; d=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "net leaf + sequence instance must both resolve. {err}{out}"
    );
}

#[test]
fn recursive_sequence_is_loud_and_terminates() {
    // `sequence s; a ##1 s; endsequence` — IEEE §16.8 illegal. Must be a LOUD
    // error and must NOT hang (the inline-stack cycle guard).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         sequence s_rec; a ##1 s_rec; endsequence\n\
         initial assert property(@(posedge clk) s_rec |-> b);\n\
         initial begin #10 a=1; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(1), "recursive sequence must be loud. {err}{out}");
    assert!(
        err.contains("VITA-E3009") && err.to_lowercase().contains("recursive"),
        "{err}"
    );
}

#[test]
fn mutually_recursive_sequences_are_loud() {
    let (_o, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, c=0;\n\
         always #5 clk=~clk;\n\
         sequence s1; a ##1 s2; endsequence\n\
         sequence s2; b ##1 s1; endsequence\n\
         initial assert property(@(posedge clk) s1 |-> c);\n\
         initial begin #10 a=1; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(1), "mutual recursion must be loud. {err}");
    assert!(
        err.contains("VITA-E3009") && err.to_lowercase().contains("recursive"),
        "{err}"
    );
}

#[test]
fn unknown_property_name_is_loud() {
    let (_o, err, code) = run("module top;\n\
         reg clk=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(nonexistent);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "unknown property must be loud, not a silent instantiation. {err}"
    );
    assert!(
        err.contains("VITA-E3009") && err.to_lowercase().contains("unknown property"),
        "{err}"
    );
}

#[test]
fn unused_parameterized_sequence_declaration_is_clean() {
    // Slice A1: a parameterized sequence is no longer parser-rejected. Declaring it
    // (even unused) must NOT error — the assert here uses an unrelated inline
    // property that holds. (Formal-args BEHAVIOR is tested in sva_formal_args.rs.)
    let (_o, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         sequence s_p(x); x ##1 b; endsequence\n\
         initial assert property(@(posedge clk) a |-> b);\n\
         initial begin #10 a=1; b=1; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "an unused parameterized sequence decl is clean (A1). {err}"
    );
}

#[test]
fn multiclock_named_property_is_loud() {
    // an OR-of-clocks property clock is multi-clock — the single-`always` checker
    // does not implement it; the single-clock gate must reject it after inlining.
    let (_o, err, code) = run("module top;\n\
         reg c1=0, c2=0, a=0, b=0;\n\
         always #5 c1=~c1;\n\
         always #7 c2=~c2;\n\
         property p_mc; @(posedge c1 or posedge c2) a |-> b; endproperty\n\
         initial assert property(p_mc);\n\
         initial #30 $finish;\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "multi-clock named property must be loud. {err}"
    );
    assert!(
        err.to_lowercase().contains("single clocking") || err.contains("VITA-E3009"),
        "{err}"
    );
}

#[test]
fn sequence_as_identifier_still_parses() {
    // masking regression: `sequence` is a CONTEXTUAL keyword. A reg named `sequence`
    // (a plausible adjacent surface) must still declare + drive normally — the
    // net-decl arm matches before the contextual-keyword arm.
    let (out, err, code) = run("module top;\n\
         reg sequence;\n\
         initial begin sequence = 1; if (sequence) $display(\"ok\"); $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "`sequence` as an identifier must still work. {err}{out}"
    );
    assert!(out.contains("ok"), "{out}");
}

#[test]
fn named_property_body_references_named_sequence() {
    // a property whose body uses a NAMED sequence — both tables consulted in one
    // checker (prop spliced at collect, the sequence inlined at materialize).
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, d=0;\n\
         always #5 clk=~clk;\n\
         sequence s_ab; a ##1 b; endsequence\n\
         property p_uses_s; @(posedge clk) s_ab |-> d; endproperty\n\
         initial assert property(p_uses_s);\n\
         initial begin\n\
           #10 a=1; b=0; d=0;\n\
           #10 a=0; b=1; d=0;\n\
           #10 $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "property using a named sequence must complete + fire. {err}{out}"
    );
}

#[test]
fn named_property_with_disable_iff() {
    // `disable iff(rst)` inside a named property body must splice through and abort
    // the attempt while rst is high (reusing slice S12). rst high the whole time →
    // a would-be violation is suppressed → clean exit.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0, rst=1;\n\
         always #5 clk=~clk;\n\
         property p_d; @(posedge clk) disable iff(rst) a |-> b; endproperty\n\
         initial assert property(p_d);\n\
         initial begin #10 a=1; b=0; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "disable iff(rst) high must suppress the violation. {err}{out}"
    );
}

#[test]
fn named_instance_with_action_block() {
    // a call-site action block on a named-property instance: the `else` (fail)
    // statement replaces the default $error and must run on a violation.
    let (out, err, _code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         property p_ab; @(posedge clk) a |-> b; endproperty\n\
         initial assert property(p_ab) else $display(\"FAILMARK\");\n\
         initial begin #10 a=1; b=0; #20 $finish; end\n\
         endmodule\n");
    // a custom fail action ($display) replaces $error → no severity exit, but the
    // marker prints, proving the call-site action block reached the named instance.
    assert!(
        out.contains("FAILMARK"),
        "call-site action block must run on the named instance. {err}{out}"
    );
}

// ── adversarial-review fixes ──

#[test]
fn named_within_equiv_inline() {
    // REVIEW (equivalence lens): a named sequence whose body is a top-level `within`
    // must be accepted exactly like the inline antecedent (it was wrongly rejected
    // E3009 because top-level `within` routing only saw a LITERAL Sequence::Within).
    let inline = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a within b ##2 c |-> d);\n\
         initial begin #10 a=1; b=1; c=1; d=1; #50 $finish; end\n\
         endmodule\n");
    let named = run("module top;\n\
         reg clk=0, a=0, b=0, c=0, d=0;\n\
         always #5 clk=~clk;\n\
         sequence sq_w; a within b ##2 c; endsequence\n\
         initial assert property(@(posedge clk) sq_w |-> d);\n\
         initial begin #10 a=1; b=1; c=1; d=1; #50 $finish; end\n\
         endmodule\n");
    assert_eq!(
        named.2, inline.2,
        "named `within` must match inline exit code.\nINLINE {inline:?}\nNAMED {named:?}"
    );
    assert!(
        !named.1.contains("only supported as a top-level"),
        "named within must not be wrongly rejected:\n{}",
        named.1
    );
}

#[test]
fn module_named_sequence_still_instantiates() {
    // REVIEW (masking lens): `sequence` is NOT a Verilog-2005 reserved word, so a
    // module TYPE named `sequence` and its instantiation must still parse (the decl
    // arm must use a 2-token guard, not fire on any leading `sequence`).
    let (out, err, code) = run("module sequence(output o);\n\
         assign o = 1'b1;\n\
         endmodule\n\
         module top;\n\
         wire o;\n\
         sequence u(.o(o));\n\
         initial begin #1 $display(\"o=%b\", o); $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "module named `sequence` must instantiate. {err}{out}"
    );
    assert!(out.contains("o=1"), "{out}");
}

#[test]
fn redeclared_property_first_declaration_wins() {
    // REVIEW (resolution lens): the W3056 warning says "first declaration used" —
    // make it TRUE (first-wins). First prop holds (a|->b with a=0), second would
    // violate (b|->a with b=1,a=0); first-wins → no fire → clean exit.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         property p; @(posedge clk) a |-> b; endproperty\n\
         property p; @(posedge clk) b |-> a; endproperty\n\
         initial assert property(p);\n\
         initial begin a=0; b=1; #20 $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "first declaration must win (no fire). {err}{out}"
    );
    assert!(
        err.to_lowercase().contains("redeclared"),
        "a redeclaration warning is expected:\n{err}"
    );
}

#[test]
fn assert_property_of_sequence_has_clear_message() {
    // REVIEW (resolution lens): `assert property(SEQ)` (a sequence used directly as
    // a property) must not say "unknown" — the name IS declared, just not a property.
    let (_o, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         sequence s_ab; a ##1 b; endsequence\n\
         initial assert property(s_ab);\n\
         initial #20 $finish;\n\
         endmodule\n");
    assert_eq!(code, Some(1), "a sequence-as-property must be loud. {err}");
    assert!(
        err.to_lowercase().contains("sequence")
            && !err.to_lowercase().contains("unknown property `s_ab`"),
        "message should identify `s_ab` as a sequence, not 'unknown property':\n{err}"
    );
}
