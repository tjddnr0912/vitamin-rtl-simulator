//! FIXED string-array declaration initializer — `string s[3] = '{"a","b","c"}`.
//!
//! Previously a blanket loud ("assign elements in an initial block") in every scope,
//! while the DYNAMIC form (`string s[] = '{…}`) already worked — the asymmetry was
//! itself the gap. Now the pattern expands to one `s[k] = <elem>` per declared index
//! in the t0 var-init pre-sweep, driven through the ordinary const-index element path.
//! Oracle: iverilog -g2012 (a fixed string array is an iverilog-supported construct).
//!
//! The subtle part is the FILL ORDER. Pattern element k targets the declared index
//! walking from the LEFT bound toward the right (IEEE §10.9.1), so a descending
//! declaration is not `min+k`:
//!
//! ```text
//!   string s[1:3] = '{"a1","b2","c3"}   →  s[1]=a1 s[2]=b2 s[3]=c3
//!   string s[3:1] = '{"a1","b2","c3"}   →  s[3]=a1 s[2]=b2 s[1]=c3   (reversed)
//! ```
//!
//! Both are pinned below against iverilog, with distinct per-element values so a
//! reversal cannot hide.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_raw(src: &str) -> (String, bool, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_fsai_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut s = String::new();
    for l in so.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    (
        s,
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn run(src: &str) -> String {
    let (out, ok, err) = run_raw(src);
    assert!(ok, "expected success, stderr:\n{err}");
    out
}

fn loud(src: &str, needle: &str) {
    let (_, ok, err) = run_raw(src);
    assert!(!ok, "expected a loud reject");
    assert!(err.contains(needle), "unexpected diagnostic:\n{err}");
}

#[test]
fn size_form_fills_ascending_from_zero() {
    let out = run("module t;\n\
           string s[3] = '{\"a1\",\"b2\",\"c3\"};\n\
           initial $display(\"0=%s 1=%s 2=%s\", s[0], s[1], s[2]);\n\
         endmodule\n");
    assert_eq!(out, "0=a1 1=b2 2=c3\n");
}

#[test]
fn ascending_range_fills_left_to_right() {
    let out = run("module t;\n\
           string s[1:3] = '{\"a1\",\"b2\",\"c3\"};\n\
           initial $display(\"1=%s 2=%s 3=%s\", s[1], s[2], s[3]);\n\
         endmodule\n");
    assert_eq!(out, "1=a1 2=b2 3=c3\n");
}

#[test]
fn descending_range_fills_from_the_left_bound_downward() {
    // The clincher: distinct values, so filling `min+k` instead of from the declared
    // LEFT bound would show up immediately. iverilog gives s[3]=a1 … s[1]=c3.
    let out = run("module t;\n\
           string s[3:1] = '{\"a1\",\"b2\",\"c3\"};\n\
           initial $display(\"1=%s 2=%s 3=%s\", s[1], s[2], s[3]);\n\
         endmodule\n");
    assert_eq!(out, "1=c3 2=b2 3=a1\n");
}

#[test]
fn non_zero_base_ascending() {
    let out = run("module t;\n\
           string s[1:2] = '{\"aa\",\"bb\"};\n\
           initial $display(\"%s|%s\", s[1], s[2]);\n\
         endmodule\n");
    assert_eq!(out, "aa|bb\n");
}

#[test]
fn element_may_be_a_non_literal_expression() {
    // Reads a module-scope string declared earlier — the expansion is pushed in
    // declaration order alongside the scalar inits, so `p` is already assigned.
    let out = run("module t;\n\
           string p = \"PP\";\n\
           string s[2] = '{p, \"bb\"};\n\
           initial $display(\"%s|%s\", s[0], s[1]);\n\
         endmodule\n");
    assert_eq!(out, "PP|bb\n");
}

#[test]
fn block_local_fixed_string_array_init() {
    let out = run("module t;\n\
           initial begin : blk\n\
             string s[2] = '{\"aa\",\"bb\"};\n\
             $display(\"%s|%s\", s[0], s[1]);\n\
           end\n\
         endmodule\n");
    assert_eq!(out, "aa|bb\n");
}

#[test]
fn init_then_overwrite_an_element() {
    let out = run("module t;\n\
           string s[2] = '{\"aa\",\"bb\"};\n\
           initial begin s[0] = \"zz\"; $display(\"%s|%s\", s[0], s[1]); end\n\
         endmodule\n");
    assert_eq!(out, "zz|bb\n");
}

#[test]
fn initialized_elements_support_the_string_methods() {
    let out = run("module t;\n\
           string s[2] = '{\"hello\",\"xy\"};\n\
           initial $display(\"%0d %s\", s[0].len(), s[0].substr(0,2));\n\
         endmodule\n");
    assert_eq!(out, "5 hel\n");
}

#[test]
fn init_form_equals_explicit_element_writes() {
    // The vita-internal equivalence: an initialized array must be byte-identical to
    // the same array filled by hand.
    let out = run("module t;\n\
           string a[3] = '{\"p\",\"qq\",\"rrr\"};\n\
           string b[3];\n\
           initial begin\n\
             b[0]=\"p\"; b[1]=\"qq\"; b[2]=\"rrr\";\n\
             $display(\"%0d%0d%0d\", a[0]==b[0], a[1]==b[1], a[2]==b[2]);\n\
           end\n\
         endmodule\n");
    assert_eq!(out, "111\n");
}

#[test]
fn zero_based_descending_fills_from_the_left_bound() {
    // `[2:0]` is the COMMON descending form (the `[3:1]` test above is the offset one).
    let out = run("module t;\n\
           string s[2:0] = '{\"d0\",\"d1\",\"d2\"};\n\
           initial $display(\"0=%s 1=%s 2=%s\", s[0], s[1], s[2]);\n\
         endmodule\n");
    assert_eq!(out, "0=d2 1=d1 2=d0\n");
}

#[test]
fn negative_declared_indices() {
    // The index expression for a negative declared index has to be `-<lit>`; a decimal
    // literal whose raw text starts with '-' never folds, which made this loud with a
    // misleading "requires a constant index".
    let out = run("module t;\n\
           string s[-3:-1] = '{\"a1\",\"b2\",\"c3\"};\n\
           initial $display(\"[%s][%s][%s]\", s[-3], s[-2], s[-1]);\n\
         endmodule\n");
    assert_eq!(out, "[a1][b2][c3]\n");
}

#[test]
fn spanning_negative_and_positive_indices() {
    let out = run("module t;\n\
           string s[-1:1] = '{\"n1\",\"z0\",\"p1\"};\n\
           initial $display(\"[%s][%s][%s]\", s[-1], s[0], s[1]);\n\
         endmodule\n");
    assert_eq!(out, "[n1][z0][p1]\n");
}

#[test]
fn descending_writes_execute_in_ascending_index_order() {
    // The element WRITES must run in ascending declared-index order, like iverilog —
    // it matters only when an element initializer READS a sibling element mid-fill.
    // Emitting them in pattern order (descending, for a descending bound) made s[2]
    // read the already-written s[3] and produce "AA" instead of the unwritten value.
    let out = run("module t;\n\
           string s[3:1] = '{\"AA\", peek(), \"CC\"};\n\
           function automatic string peek(); return s[3]; endfunction\n\
           initial $display(\"[%s][%s][%s]\", s[1], s[2], s[3]);\n\
         endmodule\n");
    // NOTE: iverilog renders the unwritten element as "[ ]" and vita as "[]" — that is
    // a pre-existing difference in how an UNINITIALIZED string-array element prints
    // (it reproduces on an array with no initializer at all), NOT a fill-order issue.
    // Do not "fix" this assertion to match iverilog's spacing.
    assert_eq!(out, "[CC][][AA]\n");
}

#[test]
fn ascending_element_may_read_an_earlier_element() {
    // The ascending mirror: reading an EARLIER element sees the value just written.
    let out = run("module t;\n\
           string s[2] = '{\"aa\", peek()};\n\
           function automatic string peek(); return s[0]; endfunction\n\
           initial $display(\"[%s][%s]\", s[0], s[1]);\n\
         endmodule\n");
    assert_eq!(out, "[aa][aa]\n");
}

#[test]
fn single_element_array() {
    // Count of 1: `step` is unobservable, so both directions must agree.
    let out = run("module t;\n\
           string a[1] = '{\"one\"};\n\
           string b[5:5] = '{\"five\"};\n\
           initial $display(\"[%s][%s]\", a[0], b[5]);\n\
         endmodule\n");
    assert_eq!(out, "[one][five]\n");
}

#[test]
fn per_instance_isolation() {
    // Each instance gets its own element nets and its own copy of the init, and a
    // mutation in one must not be visible in the other. (Reads stay inside each
    // instance — a hierarchical `a.s[0]` is a separate, still-loud gap.)
    // Each instance appends to its own element 0. If the two instances shared the
    // element nets (one init applied twice, or one storage), one of them would read
    // back "p!!" instead of "p!".
    let out = run("module m;\n\
           string s[2] = '{\"p\",\"q\"};\n\
           initial begin s[0] = {s[0], \"!\"}; #1 $display(\"[%s][%s]\", s[0], s[1]); end\n\
         endmodule\n\
         module t;\n\
           m a();\n\
           m b();\n\
           initial #2 $finish;\n\
         endmodule\n");
    assert_eq!(out, "[p!][q]\n[p!][q]\n");
}

#[test]
fn method_calls_are_valid_element_initializers() {
    // The parser encodes `q.size()` as `Call{name: HierPath[q, size]}` — the same shape
    // as a genuine cross-instance call `u.f()`. Treating any 2-segment head as
    // hierarchical false-rejected this very common idiom; the head is resolved the way
    // the method lowering resolves it instead.
    let out = run("module t;\n\
           int q[$] = '{1,2,3};\n\
           string p = \"hello\";\n\
           string s[3] = '{$sformatf(\"v%0d\", q.size()), p.substr(0,1), \"b\"};\n\
           initial $display(\"[%s][%s][%s]\", s[0], s[1], s[2]);\n\
         endmodule\n");
    assert_eq!(out, "[v3][he][b]\n");
}

// ── shapes that must stay loud ───────────────────────────────────────────────

#[test]
fn hierarchical_call_element_initializer_is_loud() {
    // The cross-instance twin of the method-call case above: `u.f()` reads a CHILD
    // instance at t0, so it must stay loud even though it has the same 2-segment shape.
    loud(
        "module sub(input logic d);\n\
           function automatic string f(); return \"SUBF\"; endfunction\n\
         endmodule\n\
         module t;\n\
           logic d = 0;\n\
           sub u(d);\n\
           string s[2] = '{u.f(), \"b2\"};\n\
           initial $display(\"[%s][%s]\", s[0], s[1]);\n\
         endmodule\n",
        "string-array initializer",
    );
}

#[test]
fn hierarchical_element_initializer_is_loud() {
    // `'{u.p, …}` reads a CHILD instance's string, which is still empty during the
    // parent's t0 pre-sweep — it rendered "" with no error. That ordering bug is
    // pre-existing for scalar strings too, but this expansion must not widen it from
    // a loud reject into a silent wrong value.
    loud(
        "module sub(input logic d); string p = \"SUBP\"; endmodule\n\
         module t;\n\
           logic d = 0;\n\
           sub u(d);\n\
           string s[2] = '{u.p, \"b2\"};\n\
           initial $display(\"[%s][%s]\", s[0], s[1]);\n\
         endmodule\n",
        "string-array initializer",
    );
}

#[test]
fn pathological_bounds_are_loud_not_a_panic() {
    // `left - right` overflows i64 here; a bare subtraction panicked where the decl
    // used to emit a clean diagnostic.
    loud(
        "module t;\n\
           string s[4611686018427387904:-4611686018427387905] = '{\"a\",\"b\"};\n\
           initial $display(\"hi\");\n\
         endmodule\n",
        "string-array initializer",
    );
}

#[test]
fn automatic_block_local_init_stays_loud() {
    // The `automatic` block-local path creates the nets under a `$blk$` scope and does
    // NOT run the collector, so it must keep rejecting rather than silently emptying.
    loud(
        "module t;\n\
           initial begin : blk\n\
             automatic string s[2] = '{\"aa\",\"bb\"};\n\
             $display(\"%s\", s[0]);\n\
           end\n\
         endmodule\n",
        "unsupported",
    );
}

#[test]
fn element_count_mismatch_is_loud() {
    // iverilog rejects this too ("Unpacked array assignment pattern expects 3
    // element(s) … Found 2"), so silently filling a prefix would be wrong.
    loud(
        "module t;\n\
           string s[3] = '{\"aa\",\"bb\"};\n\
           initial $display(\"%s\", s[0]);\n\
         endmodule\n",
        "string-array initializer",
    );
}

#[test]
fn non_pattern_initializer_is_loud() {
    loud(
        "module t;\n\
           string s[2] = \"oops\";\n\
           initial $display(\"%s\", s[0]);\n\
         endmodule\n",
        "string-array initializer",
    );
}

#[test]
fn brace_concat_initializer_is_loud() {
    // `{…}` (not `'{…}`) — iverilog rejects it here as well.
    loud(
        "module t;\n\
           string s[2] = {\"aa\",\"bb\"};\n\
           initial $display(\"%s\", s[0]);\n\
         endmodule\n",
        "string-array initializer",
    );
}

#[test]
fn generate_scope_init_stays_loud() {
    // Generate bodies do not run the string var-init flush (`allow_string_init` is
    // false there), so this stays a loud reject rather than a silently empty array.
    // Documented follow-on, same as the scalar-string generate-scope init.
    loud(
        "module t;\n\
           genvar i;\n\
           generate for (i=0;i<1;i=i+1) begin : g\n\
             string s[2] = '{\"aa\",\"bb\"};\n\
             initial $display(\"%s\", s[0]);\n\
           end endgenerate\n\
         endmodule\n",
        "string-array initializer",
    );
}
