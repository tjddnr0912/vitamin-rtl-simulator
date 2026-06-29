//! N7-REST ⓑ-breadth: SystemVerilog array manipulation methods (IEEE 1800 §7.12).
//!
//! iverilog 13 does NOT support array reduction/ordering/locator methods, so the
//! oracle is hand-IEEE §7.12 + determinism. Reduction results take the ELEMENT
//! type (width/sign); arithmetic wraps at the element width.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_full(src: &str) -> (String, String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_arrm_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("failed to run vita");
    let _ = std::fs::remove_file(&path);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

fn run(src: &str) -> String {
    let (out, err, ok) = run_full(src);
    assert!(ok, "vita must succeed; stderr:\n{err}");
    let mut s = String::new();
    for l in out.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    s
}

fn assert_loud_reject(src: &str, what: &str) {
    let (_, err, ok) = run_full(src);
    assert!(!ok, "{what}: must exit non-zero (loud reject)");
    assert!(
        err.contains("E3009") || err.contains("E30"),
        "{what}: stderr must carry an elaborate E-code; got:\n{err}"
    );
}

// ── Reduction methods on a queue ─────────────────────────────────────────────

#[test]
fn queue_sum_product() {
    // int q[$] = {1,2,3,4}: sum=10, product=24.
    let out = run("module top; int q[$]; int s; int p;\n\
         initial begin\n\
           q.push_back(1); q.push_back(2); q.push_back(3); q.push_back(4);\n\
           s = q.sum();\n\
           p = q.product();\n\
           $display(\"s=%0d p=%0d\", s, p);\n\
         end endmodule\n");
    assert_eq!(out, "s=10 p=24\n");
}

#[test]
fn queue_bitwise_reductions() {
    // {1,2,3,4}: and=0, or=7, xor=4.
    let out = run("module top; int q[$]; int a; int o; int x;\n\
         initial begin\n\
           q.push_back(1); q.push_back(2); q.push_back(3); q.push_back(4);\n\
           a = q.and(); o = q.or(); x = q.xor();\n\
           $display(\"a=%0d o=%0d x=%0d\", a, o, x);\n\
         end endmodule\n");
    assert_eq!(out, "a=0 o=7 x=4\n");
}

#[test]
fn dyn_array_sum() {
    // dynamic array new[3] then element writes; sum across.
    let out = run("module top; int d[]; int s;\n\
         initial begin\n\
           d = new[3];\n\
           d[0]=10; d[1]=20; d[2]=30;\n\
           s = d.sum();\n\
           $display(\"s=%0d\", s);\n\
         end endmodule\n");
    assert_eq!(out, "s=60\n");
}

#[test]
fn empty_queue_sum_is_zero() {
    // IEEE leaves the empty case to the accumulator default (0) — documented pin.
    let out = run("module top; int q[$]; int s;\n\
         initial begin s = q.sum(); $display(\"s=%0d\", s); end endmodule\n");
    assert_eq!(out, "s=0\n");
}

#[test]
fn sum_wraps_at_element_width() {
    // byte q[$] = {100, 100}: sum 200 wraps to a signed byte → -56.
    let out = run("module top; byte q[$]; int s;\n\
         initial begin\n\
           q.push_back(100); q.push_back(100);\n\
           s = q.sum();\n\
           $display(\"s=%0d\", s);\n\
         end endmodule\n");
    assert_eq!(out, "s=-56\n");
}

#[test]
fn reduction_on_string_handle_is_loud() {
    // `.sum()` on a string is a kind mismatch — must loud-reject, never silent.
    assert_loud_reject(
        "module top; string s; int r;\n\
         initial begin s = \"hi\"; r = s.sum(); end endmodule\n",
        "sum on string",
    );
}

// ── Ordering methods (in-place mutators) ─────────────────────────────────────

#[test]
fn queue_sort_ascending() {
    let out = run("module top; int q[$];\n\
         initial begin\n\
           q.push_back(3); q.push_back(1); q.push_back(2);\n\
           q.sort();\n\
           $display(\"%0d %0d %0d\", q[0], q[1], q[2]);\n\
         end endmodule\n");
    assert_eq!(out, "1 2 3\n");
}

#[test]
fn queue_rsort_descending() {
    let out = run("module top; int q[$];\n\
         initial begin\n\
           q.push_back(3); q.push_back(1); q.push_back(2);\n\
           q.rsort();\n\
           $display(\"%0d %0d %0d\", q[0], q[1], q[2]);\n\
         end endmodule\n");
    assert_eq!(out, "3 2 1\n");
}

#[test]
fn queue_reverse() {
    let out = run("module top; int q[$];\n\
         initial begin\n\
           q.push_back(10); q.push_back(20); q.push_back(30);\n\
           q.reverse();\n\
           $display(\"%0d %0d %0d\", q[0], q[1], q[2]);\n\
         end endmodule\n");
    assert_eq!(out, "30 20 10\n");
}

#[test]
fn sort_signed_orders_negatives_first() {
    // signed byte elements: -5 < -1 < 3 (signed ordering, not raw bits).
    let out = run("module top; byte q[$];\n\
         initial begin\n\
           q.push_back(3); q.push_back(-5); q.push_back(-1);\n\
           q.sort();\n\
           $display(\"%0d %0d %0d\", q[0], q[1], q[2]);\n\
         end endmodule\n");
    assert_eq!(out, "-5 -1 3\n");
}

#[test]
fn dyn_array_sort_in_place() {
    let out = run("module top; int d[];\n\
         initial begin\n\
           d = new[4];\n\
           d[0]=40; d[1]=10; d[2]=30; d[3]=20;\n\
           d.sort();\n\
           $display(\"%0d %0d %0d %0d\", d[0], d[1], d[2], d[3]);\n\
         end endmodule\n");
    assert_eq!(out, "10 20 30 40\n");
}

// ── `with` clause on reductions (item iterator) ──────────────────────────────

#[test]
fn sum_with_expression() {
    // sum of item*2 over {1,2,3} = 12.
    let out = run("module top; int q[$]; int s;\n\
         initial begin\n\
           q.push_back(1); q.push_back(2); q.push_back(3);\n\
           s = q.sum() with (item * 2);\n\
           $display(\"s=%0d\", s);\n\
         end endmodule\n");
    assert_eq!(out, "s=12\n");
}

#[test]
fn sum_with_index() {
    // sum of item.index over {7,7,7} = 0+1+2 = 3.
    let out = run("module top; int q[$]; int s;\n\
         initial begin\n\
           q.push_back(7); q.push_back(7); q.push_back(7);\n\
           s = q.sum() with (item.index);\n\
           $display(\"s=%0d\", s);\n\
         end endmodule\n");
    assert_eq!(out, "s=3\n");
}

// ── Locator methods returning a queue ────────────────────────────────────────

#[test]
fn min_max_return_queue() {
    let out = run("module top; int q[$]; int mn[$]; int mx[$];\n\
         initial begin\n\
           q.push_back(5); q.push_back(1); q.push_back(9); q.push_back(3);\n\
           mn = q.min();\n\
           mx = q.max();\n\
           $display(\"min=%0d max=%0d\", mn[0], mx[0]);\n\
         end endmodule\n");
    assert_eq!(out, "min=1 max=9\n");
}

#[test]
fn unique_returns_distinct() {
    // unique() returns the distinct values in first-seen order: {3,1,2}.
    let out = run("module top; int q[$]; int u[$];\n\
         initial begin\n\
           q.push_back(3); q.push_back(1); q.push_back(3); q.push_back(2); q.push_back(1);\n\
           u = q.unique();\n\
           $display(\"n=%0d %0d %0d %0d\", u.size(), u[0], u[1], u[2]);\n\
         end endmodule\n");
    assert_eq!(out, "n=3 3 1 2\n");
}

#[test]
fn find_with_predicate() {
    // find() with (item > 2): {5,9,3} of {5,1,9,3}.
    let out = run("module top; int q[$]; int f[$];\n\
         initial begin\n\
           q.push_back(5); q.push_back(1); q.push_back(9); q.push_back(3);\n\
           f = q.find() with (item > 2);\n\
           $display(\"n=%0d %0d %0d %0d\", f.size(), f[0], f[1], f[2]);\n\
         end endmodule\n");
    assert_eq!(out, "n=3 5 9 3\n");
}

#[test]
fn find_index_with_predicate() {
    // find_index() with (item > 2): indices {0,2,3} of {5,1,9,3}.
    let out = run("module top; int q[$]; int idx[$];\n\
         initial begin\n\
           q.push_back(5); q.push_back(1); q.push_back(9); q.push_back(3);\n\
           idx = q.find_index() with (item > 2);\n\
           $display(\"n=%0d %0d %0d %0d\", idx.size(), idx[0], idx[1], idx[2]);\n\
         end endmodule\n");
    assert_eq!(out, "n=3 0 2 3\n");
}

#[test]
fn find_first_index_with_predicate() {
    // find_first_index() with (item > 2): first matching index = 0.
    let out = run("module top; int q[$]; int fi[$];\n\
         initial begin\n\
           q.push_back(1); q.push_back(5); q.push_back(9);\n\
           fi = q.find_first_index() with (item > 2);\n\
           $display(\"n=%0d %0d\", fi.size(), fi[0]);\n\
         end endmodule\n");
    assert_eq!(out, "n=1 1\n");
}

#[test]
fn empty_min_returns_empty_queue() {
    let out = run("module top; int q[$]; int mn[$];\n\
         initial begin\n\
           mn = q.min();\n\
           $display(\"n=%0d\", mn.size());\n\
         end endmodule\n");
    assert_eq!(out, "n=0\n");
}

// ── Adversarial-hunt regressions (silent-wrongs found 2026-06-24) ────────────

#[test]
fn sort_signed_by_declared_type_not_literal_provenance() {
    // HUNT bug 1: an `int q[$]` element written as a hex literal (32'h80000000,
    // stored with signed=false) must STILL sort as a negative — the order keys
    // off the DECLARED element type (§6.11.1 int is signed), not how the literal
    // was spelled. Before the fix vita sorted unsigned: "0 5 100 -2147483648 -1".
    let out = run("module top; int q[$];\n\
         initial begin\n\
           q.push_back(-1); q.push_back(5); q.push_back(32'h80000000);\n\
           q.push_back(100); q.push_back(0);\n\
           q.sort();\n\
           $display(\"%0d %0d %0d %0d %0d\", q[0],q[1],q[2],q[3],q[4]);\n\
         end endmodule\n");
    assert_eq!(out, "-2147483648 -1 0 5 100\n");
}

#[test]
fn min_max_signed_by_declared_type() {
    // HUNT bug 1 (same root): min/max of a signed queue with a hex-written
    // negative must return the signed extremum.
    let out = run("module top; int q[$]; int mn[$]; int mx[$];\n\
         initial begin\n\
           q.push_back(5); q.push_back(32'hFFFFFFFF); q.push_back(100);\n\
           mn = q.min(); mx = q.max();\n\
           $display(\"min=%0d max=%0d\", mn[0], mx[0]);\n\
         end endmodule\n");
    // 32'hFFFFFFFF as a signed int is -1 → min=-1, max=100.
    assert_eq!(out, "min=-1 max=100\n");
}

#[test]
fn same_named_queue_local_across_blocks_is_loud() {
    // HUNT bug 2: two named blocks each declaring `int q[$]` would share one
    // flattened net + heap (blk2 sees blk1's elements). v1 has no per-block heap,
    // so this is now a loud reject rather than a silent contaminated reduction.
    assert_loud_reject(
        "module t;\n\
           initial begin : blk1 int q[$]; q.push_back(10); end\n\
           initial begin : blk2 int q[$]; q.push_back(2); $display(\"%0d\", q.sum()); end\n\
         endmodule\n",
        "same-named queue local across blocks",
    );
}

#[test]
fn with_clause_accumulator_follows_with_expr_type() {
    // DESIGN PIN (hunt bug 3 — resolved toward commercial-tool parity, NOT the
    // strict §7.12.3 element-type reading): the reduction accumulator/return type
    // follows the WITH-EXPRESSION's self-determined type, so `with (item)` over a
    // byte queue wraps at byte width (-112) while `with (item+0)` widens to int
    // (400). This is the VCS/Questa behavior that makes the standard
    // `with (T'(item))` overflow-avoidance idiom work; a plain `sum()` (no with)
    // still uses the element type. Documented in ROADMAP §4.
    let out = run("module t; byte q[$]; int s;\n\
         initial begin\n\
           q.push_back(100); q.push_back(100); q.push_back(100); q.push_back(100);\n\
           s = q.sum();            $display(\"plain=%0d\", s);\n\
           s = q.sum() with (item);   $display(\"item=%0d\", s);\n\
           s = q.sum() with (item+0); $display(\"widen=%0d\", s);\n\
         end endmodule\n");
    assert_eq!(out, "plain=-112\nitem=-112\nwiden=400\n");
}
