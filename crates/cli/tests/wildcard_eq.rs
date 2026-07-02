//! Wildcard equality `==?` / `!=?` (IEEE 1800 §11.4.6) — iverilog-13.0-pinned.
//!
//! The RHS PATTERN's x/z (`?` ≡ z in a literal) bits are don't-care; every
//! other bit compares like plain `==` — an LHS x/z in a COMPARED position
//! propagates x. This is NOT `CasexEq` (which wildcards either side): mapping
//! there would silently match an exposed LHS x. Lowered as a const-pattern
//! `(lhs & mask) ==/!= cleaned`; a runtime pattern (whose x/z mask would need
//! an unk-plane IR primitive) is honest-loud.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, i32) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_weq_{}_{n}", std::process::id()));
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
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn wildcard_eq_basic_matrix() {
    // 1010 ==? 1?1? → 1 (wildcards cover bits 2,0); !=? negates; a mismatch in
    // a compared bit → 0; x AND z pattern bits are BOTH wildcards.
    let (out, _, code) = run("module t; logic [3:0] a; initial begin\n\
           a = 4'b1010;\n\
           $display(\"[%b][%b]\", a ==? 4'b1?1?, a !=? 4'b1?1?);\n\
           $display(\"[%b]\", a ==? 4'b1?0?);\n\
           $display(\"[%b]\", a ==? 4'b1x1z);\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(out.contains("[1][0]"), "match + negation:\n{out}");
    assert!(out.contains("[0]"), "compared-bit mismatch:\n{out}");
    assert!(out.contains("[1]"), "x and z both wildcard:\n{out}");
}

#[test]
fn lhs_unknown_masked_vs_exposed() {
    // An LHS x/z under a WILDCARD position is covered (→1); under a COMPARED
    // position it propagates x (iverilog-pinned — CasexEq would return 1 here,
    // the silent-wrong this lowering avoids).
    let (out, _, _) = run("module t; logic [3:0] a; initial begin\n\
           a = 4'b10x0;\n\
           $display(\"m[%b]\", a ==? 4'b1?x?);\n\
           $display(\"e[%b]\", a ==? 4'b1?10);\n\
           $display(\"n[%b]\", a !=? 4'b1?10);\n\
           a = 4'b10z0;\n\
           $display(\"z[%b]\", a ==? 4'b1?10);\n\
           $display(\"w[%b]\", a ==? 4'b????);\n\
           $finish; end endmodule\n");
    assert!(out.contains("m[1]"), "masked lhs x:\n{out}");
    assert!(out.contains("e[x]"), "exposed lhs x propagates:\n{out}");
    assert!(out.contains("n[x]"), "!=? of x is x:\n{out}");
    assert!(out.contains("z[x]"), "exposed lhs z propagates:\n{out}");
    assert!(
        out.contains("w[1]"),
        "all-wildcard pattern always matches:\n{out}"
    );
}

#[test]
fn width_extension_bits_are_compared() {
    // The pattern zero-extends to the comparison width: extension bits are
    // KNOWN-0, not wildcards (8'b1000_1010 ==? 4'b1?1? → 0, iverilog-pinned).
    let (out, _, _) = run("module t; initial begin\n\
           $display(\"[%b]\", 8'b0000_1010 ==? 4'b1?1?);\n\
           $display(\"[%b]\", 8'b1000_1010 ==? 4'b1?1?);\n\
           $finish; end endmodule\n");
    assert!(out.contains("[1]"), "zero high bits match:\n{out}");
    assert!(out.contains("[0]"), "extension bits are compared:\n{out}");
}

#[test]
fn wide_pattern_multiword_mask() {
    // >64-bit pattern exercises the multi-word mask/clean construction.
    let (out, _, _) = run("module t; logic [69:0] wa; initial begin\n\
           wa = 70'h2AAAAAAAAAAAAAAAAA;\n\
           $display(\"[%b]\", wa ==? 70'h2AAAAAAAAAAAAAAAAx);\n\
           $display(\"[%b]\", wa ==? 70'h0AAAAAAAAAAAAAAAAx);\n\
           $finish; end endmodule\n");
    assert!(
        out.contains("[1]"),
        "wide match with low-nibble wildcard:\n{out}"
    );
    assert!(out.contains("[0]"), "wide high-word mismatch:\n{out}");
}

#[test]
fn exact_64bit_and_signed_lhs() {
    // R1 soundness gaps: the w=64 top-trim-skipped path and a SIGNED lhs
    // (raw-bit compare — sign never extends into the masked compare).
    let (out, _, _) = run(
        "module t; logic [63:0] q; logic signed [7:0] s; initial begin\n\
           q = 64'hFFFF_FFFF_FFFF_FFFA;\n\
           $display(\"q[%b]\", q ==? 64'hFFFF_FFFF_FFFF_FFF?);\n\
           $display(\"r[%b]\", q ==? 64'h7FFF_FFFF_FFFF_FFF?);\n\
           s = 8'sb1000_1010;\n\
           $display(\"s[%b]\", s ==? 8'b1?0?1?1?);\n\
           $display(\"n[%b]\", s ==? 4'b101?);\n\
           $finish; end endmodule\n",
    );
    assert!(out.contains("q[1]"), "64-bit exact width match:\n{out}");
    assert!(out.contains("r[0]"), "64-bit top-bit mismatch:\n{out}");
    assert!(out.contains("s[1]"), "signed lhs raw bits:\n{out}");
    assert!(out.contains("n[0]"), "signed lhs vs narrow pattern:\n{out}");
}

#[test]
fn definite_mismatch_beats_x_poison() {
    // R1 differential harvest — a PRE-EXISTING core `==`/`!=` bug the ==?
    // lowering inherited: a both-known DIFFERING bit decides the compare even
    // when another bit is x/z (IEEE §11.4.5 "ambiguous" only → x;
    // iverilog-pinned: 4'b1x00 == 4'b0000 is 0, not x).
    let (out, _, _) = run("module t; logic [3:0] a; initial begin\n\
           a = 4'b1x00;\n\
           $display(\"e[%b] n[%b]\", a == 4'b0000, a != 4'b0000);\n\
           $display(\"a[%b]\", a == 4'b1000);\n\
           a = 4'b10x0;\n\
           $display(\"w[%b] wn[%b]\", a ==? 4'b0?10, a !=? 4'b0?10);\n\
           a = 4'bx0x0;\n\
           $display(\"f[%b]\", a ==? '1);\n\
           $finish; end endmodule\n");
    assert!(
        out.contains("e[0] n[1]"),
        "definite mismatch decides ==/!=:\n{out}"
    );
    assert!(out.contains("a[x]"), "ambiguous stays x:\n{out}");
    assert!(
        out.contains("w[0] wn[1]"),
        "==?/!=? inherit the rule:\n{out}"
    );
    assert!(
        out.contains("f[0]"),
        "fill pattern + known mismatch:\n{out}"
    );
}

#[test]
fn string_literal_pattern() {
    // A string pattern has known bytes and no wildcards — plain equality
    // (iverilog-pinned: "ab" ==? "ab" → 1).
    let (out, _, code) = run("module t; initial begin\n\
           $display(\"[%b][%b]\", \"ab\" ==? \"ab\", \"ab\" ==? \"ac\");\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(out.contains("[1][0]"), "string pattern:\n{out}");
}

#[test]
fn pattern_from_parameter_and_fill() {
    // A parameter RHS const-folds → supported; a fill pattern sizes to the
    // LHS width like a case label (§11.6).
    let (out, _, _) = run(
        "module t; parameter [3:0] P = 4'b1010; logic [3:0] a; initial begin\n\
           a = 4'b1010;\n\
           $display(\"p[%b]\", a ==? P);\n\
           $display(\"f[%b][%b]\", 4'b1111 ==? '1, a ==? '1);\n\
           $finish; end endmodule\n",
    );
    assert!(out.contains("p[1]"), "parameter pattern folds:\n{out}");
    assert!(out.contains("f[1][0]"), "fill pattern sizes to lhs:\n{out}");
}

#[test]
fn usable_in_expressions_and_control() {
    // The 1-bit result composes: arithmetic operand, if-condition,
    // continuous assign.
    let (out, _, code) = run("module t; logic [3:0] a = 4'b1010; wire m;\n\
           assign m = a ==? 4'b1?1?;\n\
           initial begin\n\
           #1 $display(\"ca[%b]\", m);\n\
           $display(\"ar[%0d]\", (a ==? 4'b1?1?) + 1);\n\
           if (a ==? 4'b1?1?) $display(\"if-taken\");\n\
           $finish; end endmodule\n");
    assert_eq!(code, 0);
    assert!(out.contains("ca[1]"), "continuous assign:\n{out}");
    assert!(out.contains("ar[2]"), "arith operand:\n{out}");
    assert!(out.contains("if-taken"), "if condition:\n{out}");
}

#[test]
fn const_fold_in_param_context() {
    // `==?` between folded constants collapses to plain eq (no x/z in a
    // folded i64) — usable in a parameter initializer.
    let (out, _, code) = run(
        "module t; localparam W = (5 ==? 5) ? 8 : 4; logic [W-1:0] v;\n\
           initial begin $display(\"w=%0d\", $bits(v)); $finish; end endmodule\n",
    );
    assert_eq!(code, 0);
    assert!(out.contains("w=8"), "const-folds in param ctx:\n{out}");
}

#[test]
fn runtime_pattern_is_loud() {
    // A non-constant RHS pattern needs a runtime x/z mask (no frozen-IR
    // primitive exposes the unk plane) — honest-loud, never plain-eq fallback
    // (that would silently ignore runtime wildcards; iverilog supports this,
    // recorded as a follow-on candidate).
    let (_, err, code) = run(
        "module t; logic [3:0] a = 4'b1010; logic [3:0] b = 4'b1x1z;\n\
           logic r; initial begin r = a ==? b; $display(\"r=%b\", r); $finish; end endmodule\n",
    );
    assert_ne!(code, 0, "runtime pattern must fail loud");
    assert!(
        err.contains("constant right-hand pattern"),
        "diagnostic:\n{err}"
    );
}

#[test]
fn ternary_after_eq_is_unchanged() {
    // Lexer regression: `==` followed by a ternary still parses — `==?` only
    // lexes CONTIGUOUSLY, so `a==b?c:d` (ident comparand) is untouched. (A
    // LITERAL comparand like `4'b1010?7:3` is different pre-existing territory:
    // `?` is a z-DIGIT inside a based literal and gets munched by it.)
    let (out, _, code) = run(
        "module t; logic [3:0] a = 4'b1010; logic [3:0] b = 4'b1010;\n\
           int r; initial begin\n\
           r = a==b?7:3;\n\
           $display(\"t[%0d]\", r);\n\
           r = (a === 4'b1010) ? 9 : 1;\n\
           $display(\"c[%0d]\", r);\n\
           r = a == 4'b1010 ? 5 : 2;\n\
           $display(\"s[%0d]\", r);\n\
           $finish; end endmodule\n",
    );
    assert_eq!(code, 0);
    assert!(
        out.contains("t[7]"),
        "compact ident ternary unchanged:\n{out}"
    );
    assert!(out.contains("c[9]"), "case-eq + ternary unchanged:\n{out}");
    assert!(
        out.contains("s[5]"),
        "spaced literal ternary unchanged:\n{out}"
    );
}
