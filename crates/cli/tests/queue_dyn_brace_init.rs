//! §4.5.182 (loud -> supported): a queue / dynamic-array declaration initialized
//! with a `{e0,e1,...}` unpacked-array concatenation (IEEE 1800 §10.10), e.g.
//! `int q[$] = {1,2,3,4};` or `int d[] = {a,b,c};`. Previously vita required the
//! apostrophe assignment-pattern form `'{...}` and rejected the plain-brace form
//! with E3009. iverilog (the oracle) accepts both, and for a scalar-element target
//! the two forms spell the same element list, so the brace form now rides the exact
//! same var-init flush expansion (a push_back sequence for a queue, `new[N]` plus
//! element writes for a dynamic array).
//!
//! The routing is correct-or-loud by construction: it reuses the `'{...}` path, so
//! any element with no scalar surface (an array-typed element under concat
//! flattening) stays loud exactly as `'{...}` already does. Replication `{n{x}}` is
//! a distinct Replicate node, never a Concat, so it never slips in and stays loud.
//! A STRING-element array keeps the brace form loud too, because the `'{...}` string
//! decl-init path is a separate follow-on (tracked as its own silent-empty bug).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns (first `K=` line, process_success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_qdbi_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let key = text
        .lines()
        .find(|l| l.starts_with("K="))
        .unwrap_or_default()
        .trim()
        .to_owned();
    (key, out.status.success())
}

fn loud(src: &str) -> bool {
    !run(src).1
}

// ── supported: brace-concat initializer ─────────────────────────────────────

#[test]
fn queue_brace_literals() {
    let (k, ok) = run("module top; initial begin\n\
         int q[$] = {1,2,3,4};\n\
         $display(\"K=%0d %0d %0d %0d\", q[0],q[1],q[2],q[3]); $finish; end endmodule");
    assert!(ok && k == "K=1 2 3 4", "got ({k}, {ok})");
}

#[test]
fn dynarray_brace_literals() {
    let (k, ok) = run("module top; initial begin\n\
         int d[] = {5,6,7};\n\
         $display(\"K=%0d %0d %0d\", d[0],d[1],d[2]); $finish; end endmodule");
    assert!(ok && k == "K=5 6 7", "got ({k}, {ok})");
}

#[test]
fn queue_brace_scalar_vars() {
    let (k, ok) = run("module top; initial begin\n\
         int a=10,b=20,c=30;\n\
         int q[$] = {a,b,c};\n\
         $display(\"K=%0d %0d %0d\", q[0],q[1],q[2]); $finish; end endmodule");
    assert!(ok && k == "K=10 20 30", "got ({k}, {ok})");
}

#[test]
fn queue_brace_single_element() {
    let (k, ok) = run("module top; initial begin\n\
         int q[$] = {42};\n\
         $display(\"K=%0d %0d\", q.size(), q[0]); $finish; end endmodule");
    assert!(ok && k == "K=1 42", "got ({k}, {ok})");
}

#[test]
fn queue_brace_expression_elements() {
    // Each element is a scalar expression; brace form is element-wise, NOT a packed
    // concatenation (would be 1,5,8 not a fused value).
    let (k, ok) = run("module top; initial begin\n\
         int q[$] = {1, 2+3, 4*2};\n\
         $display(\"K=%0d %0d %0d\", q[0],q[1],q[2]); $finish; end endmodule");
    assert!(ok && k == "K=1 5 8", "got ({k}, {ok})");
}

#[test]
fn queue_brace_signed_byte_elements() {
    // Signed narrow element type: negative literals round-trip with sign.
    let (k, ok) = run("module top; initial begin\n\
         byte q[$] = {-1, 2, -3};\n\
         $display(\"K=%0d %0d %0d\", q[0],q[1],q[2]); $finish; end endmodule");
    assert!(ok && k == "K=-1 2 -3", "got ({k}, {ok})");
}

#[test]
fn queue_brace_size_reported() {
    let (k, ok) = run("module top; initial begin\n\
         int q[$] = {5,6,7,8};\n\
         $display(\"K=%0d %0d\", q.size(), q[3]); $finish; end endmodule");
    assert!(ok && k == "K=4 8", "got ({k}, {ok})");
}

#[test]
fn queue_brace_then_mutate() {
    // A brace-initialized queue is a normal handle: push_back / delete work on it.
    let (k, ok) = run("module top;\n\
         int q[$] = {1,2,3};\n\
         initial begin q.push_back(4); q.delete(0);\n\
         $display(\"K=%0d %0d %0d\", q[0],q[1],q[2]); $finish; end endmodule");
    assert!(ok && k == "K=2 3 4", "got ({k}, {ok})");
}

#[test]
fn module_scope_queue_and_dyn_brace() {
    let (k, ok) = run("module top;\n\
         int q[$] = {1,5,8};\n\
         int d[] = {9,8,7};\n\
         initial begin\n\
         $display(\"K=%0d %0d %0d %0d %0d %0d\", q[0],q[1],q[2], d[0],d[1],d[2]);\n\
         $finish; end endmodule");
    assert!(ok && k == "K=1 5 8 9 8 7", "got ({k}, {ok})");
}

#[test]
fn block_local_queue_brace() {
    let (k, ok) = run("module top; initial begin : blk\n\
         int q[$] = {11,22};\n\
         $display(\"K=%0d %0d\", q[0],q[1]); $finish; end endmodule");
    assert!(ok && k == "K=11 22", "got ({k}, {ok})");
}

// ── unaffected baseline: apostrophe form still works ─────────────────────────

#[test]
fn apostrophe_form_still_works() {
    let (k, ok) = run("module top; initial begin\n\
         int q[$] = '{1,2,3};\n\
         $display(\"K=%0d %0d %0d\", q[0],q[1],q[2]); $finish; end endmodule");
    assert!(
        ok && k == "K=1 2 3",
        "apostrophe form must be unaffected; got ({k}, {ok})"
    );
}

// ── correct-or-loud boundaries: must STAY loud (no silent-wrong) ─────────────

#[test]
fn array_element_concat_stays_loud() {
    // `{a, 3}` where `a` is a queue is unpacked-concat flattening, which has no
    // scalar surface for a queue-of-int; iverilog rejects it at compile, vita is
    // loud (E3009) — never silently pushing the handle id as an element.
    assert!(loud(
        "module top; initial begin\n\
         int a[$] = {1,2};\n\
         int q[$] = {a, 3};\n\
         $display(\"K=%0d\", q.size()); $finish; end endmodule"
    ));
}

#[test]
fn replication_init_stays_loud() {
    // `{3{5}}` is a Replicate node, not a Concat; its queue-init meaning is
    // ambiguous (iverilog itself yields a dubious result), so it stays loud.
    assert!(loud(
        "module top; initial begin\n\
         int q[$] = {3{5}};\n\
         $display(\"K=%0d\", q.size()); $finish; end endmodule"
    ));
}

#[test]
fn string_dyn_brace_stays_loud() {
    // A STRING-element array keeps the brace form loud: the `'{...}` string
    // decl-init path is a separate follow-on, so the brace form must not silently
    // produce an empty array.
    assert!(loud(
        "module top; initial begin\n\
         string s[] = {\"a\",\"b\"};\n\
         $display(\"K=%s\", s[0]); $finish; end endmodule"
    ));
}
