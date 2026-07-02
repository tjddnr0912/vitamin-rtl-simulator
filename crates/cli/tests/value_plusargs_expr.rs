//! `$value$plusargs` in an `if`-condition — `if ($value$plusargs("LIMIT=%d", n)) …`
//! (report §6 B1; IEEE 1800-2017 §21.6). `$value$plusargs` is a system FUNCTION
//! that writes its ref `var` (a side effect) and returns 1/0; vita supported it
//! only as the direct RHS of a blocking assignment because of that write, so the
//! universal `if (...)` idiom errored (E3009). iverilog 13.0 accepts it anywhere.
//!
//! Fixed by desugaring the if-CONDITION form to the supported statement form: a
//! synthetic `__tmp = $value$plusargs(...)` (the write gets a controlled
//! placement before the branch) then branch on `__tmp`. The validation +
//! SysFunc build is shared with the statement form (`value_plusargs_rhs`). A
//! `$value$plusargs` nested deeper in an expression (`x = 1 + $value$plusargs`),
//! or in a `while`/`case`/ternary condition, stays honest-loud (v1). No IR /
//! frozen-type change. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `src` with optional `plusargs`, return (first `R:` line w/o prefix, success).
fn run(src: &str, plusargs: &[&str]) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_vpe_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_vita"));
    cmd.arg(f.to_str().unwrap());
    for p in plusargs {
        cmd.arg(p);
    }
    let out = cmd.current_dir(&d).output().expect("run vita");
    let r = String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|l| l.trim().strip_prefix("R:").map(str::to_owned))
        .unwrap_or_default();
    (r, out.status.success())
}

#[test]
fn if_condition_plusarg_present() {
    // The call writes n=42 and returns 1; the then-branch reads n.
    let (o, ok) = run(
        "module tb; int n; initial begin\n\
         if ($value$plusargs(\"LIMIT=%d\", n)) $display(\"R:got %0d\", n);\n\
         else $display(\"R:none\"); $finish; end endmodule",
        &["+LIMIT=42"],
    );
    assert!(ok && o == "got 42", "got:\n{o}");
}

#[test]
fn if_condition_plusarg_absent_leaves_var() {
    // No matching plusarg → returns 0, n is left untouched (else branch).
    let (o, ok) = run(
        "module tb; int n; initial begin n=99;\n\
         if ($value$plusargs(\"LIMIT=%d\", n)) $display(\"R:got %0d\", n);\n\
         else $display(\"R:none n=%0d\", n); $finish; end endmodule",
        &[],
    );
    assert!(ok && o == "none n=99", "got:\n{o}");
}

#[test]
fn else_if_chain() {
    // The condition desugar also fires in an `else if` (a nested If).
    let (o, ok) = run(
        "module tb; int n; initial begin\n\
         if ($value$plusargs(\"X=%d\", n)) $display(\"R:x\");\n\
         else if ($value$plusargs(\"LIMIT=%d\", n)) $display(\"R:lim %0d\", n);\n\
         else $display(\"R:none\"); $finish; end endmodule",
        &["+LIMIT=7"],
    );
    assert!(ok && o == "lim 7", "got:\n{o}");
}

#[test]
fn string_spec_in_if() {
    let (o, ok) = run(
        "module tb; string s; initial begin\n\
         if ($value$plusargs(\"NAME=%s\", s)) $display(\"R:name %s\", s);\n\
         else $display(\"R:none\"); $finish; end endmodule",
        &["+NAME=hello"],
    );
    assert!(ok && o == "name hello", "got:\n{o}");
}

#[test]
fn two_independent_if_calls() {
    let (o, ok) = run(
        "module tb; int a,b; initial begin\n\
         if ($value$plusargs(\"A=%d\", a)) $display(\"R:a%0d\", a);\n\
         if ($value$plusargs(\"B=%d\", b)) $display(\"R:b%0d\", b); $finish; end endmodule",
        &["+A=3"],
    );
    assert!(ok && o == "a3", "got:\n{o}");
}

#[test]
fn statement_form_unchanged() {
    // The supported `r = $value$plusargs(...)` form is regression-checked.
    let (o, ok) = run(
        "module tb; int n,r; initial begin\n\
         r = $value$plusargs(\"LIMIT=%d\", n); $display(\"R:r=%0d n=%0d\", r, n);\n\
         $finish; end endmodule",
        &["+LIMIT=42"],
    );
    assert!(ok && o == "r=1 n=42", "got:\n{o}");
}

#[test]
fn nested_in_expression_is_loud() {
    // Deeper in an expression the side-effect placement is uncontrolled — loud.
    let (_o, ok) = run(
        "module tb; int n,x; initial begin\n\
         x = 1 + $value$plusargs(\"L=%d\", n); $display(\"R:%0d\", x); $finish; end endmodule",
        &["+L=5"],
    );
    assert!(!ok, "$value$plusargs nested in an expression must be loud");
}

#[test]
fn parenthesized_if_condition() {
    // `if (($value$plusargs(...)))` — redundant parens are peeled (same idiom).
    let (o, ok) = run(
        "module tb; int n; initial begin\n\
         if (($value$plusargs(\"L=%d\", n))) $display(\"R:%0d\", n);\n\
         else $display(\"R:none\"); $finish; end endmodule",
        &["+L=42"],
    );
    assert!(ok && o == "42", "got:\n{o}");
}

#[test]
fn compound_condition_is_loud() {
    // A COMPOUND condition (`(call && x)`) peels to a Binary, not the bare call,
    // so it stays loud (v1) — not silently mis-evaluated.
    let (_o, ok) = run(
        "module tb; int n; logic x=0; initial begin\n\
         if (($value$plusargs(\"L=%d\", n)) && x) $display(\"R:y\");\n\
         else $display(\"R:none\"); $finish; end endmodule",
        &["+L=42"],
    );
    assert!(
        !ok,
        "a compound $value$plusargs condition must be loud (v1)"
    );
}

#[test]
fn while_condition_is_loud() {
    // Only the if-condition form is supported in v1; a while condition is loud.
    let (_o, ok) = run(
        "module tb; int n; initial begin\n\
         while ($value$plusargs(\"L=%d\", n)) begin $display(\"R:%0d\", n); n=0; end\n\
         $finish; end endmodule",
        &["+L=5"],
    );
    assert!(
        !ok,
        "$value$plusargs in a while condition must be loud (v1)"
    );
}

#[test]
fn bad_arity_in_if_is_loud() {
    // The shared validation rejects a malformed call even in the if form.
    let (_o, ok) = run(
        "module tb; initial begin if ($value$plusargs(\"L=%d\")) $display(\"R:x\");\n\
         $finish; end endmodule",
        &[],
    );
    assert!(!ok, "a malformed $value$plusargs in an if must be loud");
}
