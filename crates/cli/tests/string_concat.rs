//! String concatenation `c = {a, b, …}` (IEEE §6.16, §11.4.12) where the parts
//! are strings. Previously loud-rejected ("use $sformatf"); now desugared to the
//! existing $sformatf("%s%s…", a, b, …) string-render machinery when it is the
//! direct rhs of a string assignment. Oracle: iverilog -g2012.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_strcat_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "stderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut s = String::new();
    for l in so.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    s
}

#[test]
fn two_string_vars() {
    let out = run("module t;\n\
           string a, b, c;\n\
           initial begin a = \"Hello\"; b = \"World\"; c = {a, b}; $display(\"%s\", c); end\n\
         endmodule\n");
    assert_eq!(out, "HelloWorld\n");
}

#[test]
fn mixed_vars_and_literals() {
    let out = run("module t;\n\
           string a, b, c;\n\
           initial begin a = \"Hello\"; b = \"World\";\n\
             c = {a, \", \", b, \"!\"}; $display(\"%s\", c); end\n\
         endmodule\n");
    assert_eq!(out, "Hello, World!\n");
}

#[test]
fn leading_literal() {
    let out = run("module t;\n\
           string a, c;\n\
           initial begin a = \"Hello\"; c = {\"x\", a}; $display(\"%s\", c); end\n\
         endmodule\n");
    assert_eq!(out, "xHello\n");
}

#[test]
fn all_literals() {
    let out = run("module t;\n\
           string c;\n\
           initial begin c = {\"ab\", \"cd\", \"ef\"}; $display(\"%s\", c); end\n\
         endmodule\n");
    assert_eq!(out, "abcdef\n");
}

#[test]
fn integral_byte_element() {
    // IEEE §6.16: an integral concat element is its character bytes (byte 67 = 'C').
    let out = run("module t;\n\
           string a, c; byte ch;\n\
           initial begin a = \"AB\"; ch = 67; c = {a, ch}; $display(\"%s\", c); end\n\
         endmodule\n");
    assert_eq!(out, "ABC\n");
}

#[test]
fn string_replication() {
    let out = run("module t;\n\
           string a, c;\n\
           initial begin a = \"AB\"; c = {3{a}}; $display(\"%s\", c); end\n\
         endmodule\n");
    assert_eq!(out, "ABABAB\n");
}

#[test]
fn concat_then_reuse() {
    // The destination may also appear on the rhs (a = {a, b} grows a).
    let out = run("module t;\n\
           string a, b;\n\
           initial begin a = \"x\"; b = \"y\"; a = {a, b}; a = {a, b}; $display(\"%s\", a); end\n\
         endmodule\n");
    assert_eq!(out, "xyy\n");
}

// ── G1: string concat in NON-assignment (expression) contexts (IEEE §6.16) ──
// Previously loud-rejected ("string concatenation is outside the v7 scope") in
// any context other than the direct rhs of a string assignment. Now the SAME
// $sformatf("%s%s…") desugar serves display args, comparisons, and nested
// concats. Oracle: iverilog -g2012 (assign-form pinned for the mixed case,
// which iverilog crashes on in expression form).

fn run_expect_loud(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_strcat_loud_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    assert!(
        !out.status.success(),
        "expected a loud reject (nonzero exit)"
    );
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn expr_display_arg() {
    // $display("%s", {a, b}) — the concat is a $display ARGUMENT, not an assign.
    // iverilog: "foobar".
    let out = run("module t;\n\
           string a, b;\n\
           initial begin a = \"foo\"; b = \"bar\"; $display(\"%s\", {a, b}); end\n\
         endmodule\n");
    assert_eq!(out, "foobar\n");
}

#[test]
fn expr_compare() {
    // {a, b} == "foobar" used in an if AND printed as a boolean. iverilog: MATCH / 1.
    let out = run("module t;\n\
           string a, b;\n\
           initial begin a = \"foo\"; b = \"bar\";\n\
             if ({a, b} == \"foobar\") $display(\"MATCH\"); else $display(\"NOMATCH\");\n\
             $display(\"%b\", ({a, b} == \"foobar\")); end\n\
         endmodule\n");
    assert_eq!(out, "MATCH\n1\n");
}

#[test]
fn expr_nested() {
    // Nested concat {a, {b, "baz"}} as a $display arg. iverilog: "foobarbaz".
    let out = run("module t;\n\
           string a, b;\n\
           initial begin a = \"foo\"; b = \"bar\"; $display(\"%s\", {a, {b, \"baz\"}}); end\n\
         endmodule\n");
    assert_eq!(out, "foobarbaz\n");
}

#[test]
fn expr_replicate() {
    // {3{a}} replicate of a string as a $display arg. iverilog: "ababab".
    let out = run("module t;\n\
           string a;\n\
           initial begin a = \"ab\"; $display(\"%s\", {3{a}}); end\n\
         endmodule\n");
    assert_eq!(out, "ababab\n");
}

#[test]
fn expr_mixed_byte_element() {
    // {string, byte} in EXPRESSION context (byte 67 = 'C', IEEE §6.16). iverilog
    // 13 crashes on the expression form, so the oracle is the ASSIGN form
    // `r = {a, c}` (= "fooC", pinned by `integral_byte_element`): the expression
    // path must render the SAME bytes.
    let out = run("module t;\n\
           string a; byte c;\n\
           initial begin a = \"foo\"; c = 67; $display(\"%s\", {a, c}); end\n\
         endmodule\n");
    assert_eq!(out, "fooC\n");
}

#[test]
fn expr_real_part_loud() {
    // A real inside a string concat stays LOUD (correct-or-loud) — iverilog also
    // rejects ("Concatenation operand can not be real").
    let err = run_expect_loud(
        "module t;\n\
           string a; real r;\n\
           initial begin a = \"foo\"; r = 1.5; $display(\"%s\", {a, r}); end\n\
         endmodule\n",
    );
    assert!(
        err.contains("real value may not be a string-concatenation element"),
        "expected the real-part loud diagnostic, got:\n{err}"
    );
}
