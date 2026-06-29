//! v5 increment (C)-⑥: dynamic-storage FRONT-END (.sv syntax → engine).
//!
//! End-to-end through the `vita` oneshot binary. Value pins:
//! - dyn array / queue lanes: iverilog -g2012 LIVE oracle (probed 2026-06-11;
//!   exact stdout asserted below).
//! - `q[$-1]` arithmetic: iverilog REJECTS the syntax (its own gap) — hand-IEEE
//!   pin (§7.10: `$` = the last index).
//! - assoc lanes: iverilog rejects assoc declarations outright — hand-IEEE pin
//!   (§7.8/§7.9), same as the engine-layer tests.
//!
//! Loud-reject lanes assert a non-zero exit + an E3009 mention on stderr
//! (the MVP cuts are DELIBERATE: silent-wrong is the one forbidden outcome).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Write a temp `.sv`, run `vita <file>`, return (stdout, stderr, success).
fn run_vita_full(src: &str) -> (String, String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_dynfe_{}_{n}.sv", std::process::id()));
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

fn run_vita(src: &str) -> String {
    let (out, err, ok) = run_vita_full(src);
    assert!(ok, "vita must succeed; stderr:\n{err}");
    // Drop vita's own end-of-run epilogue ("simulation ended (…)") so the
    // pins compare the DESIGN's stdout 1:1 with the iverilog oracle.
    let mut s = String::new();
    for l in out.lines().filter(|l| !l.starts_with("simulation ended")) {
        s.push_str(l);
        s.push('\n');
    }
    s
}

/// The design must be REJECTED loudly: non-zero exit + E3009 on stderr.
fn assert_loud_reject(src: &str, what: &str) {
    let (_, err, ok) = run_vita_full(src);
    assert!(!ok, "{what}: must exit non-zero (loud reject)");
    assert!(
        err.contains("E3009") || err.contains("E30"),
        "{what}: stderr must carry an elaborate E-code; got:\n{err}"
    );
}

// ───────────────────────── dyn array (iverilog-pinned) ─────────────────────────

#[test]
fn dyn_array_new_size_index_delete() {
    // Oracle: iverilog live — 3 / 10 20 30 / 5 20 30 / 0.
    let src = r#"
module t;
  integer d [];
  integer e [];
  initial begin
    d = new[3];
    $display("%0d", d.size());
    d[0] = 10; d[1] = 20; d[2] = 30;
    $display("%0d %0d %0d", d[0], d[1], d[2]);
    e = new[5](d);
    $display("%0d %0d %0d", e.size(), e[1], e[2]);
    d.delete();
    $display("%0d", d.size());
  end
endmodule
"#;
    assert_eq!(run_vita(src), "3\n10 20 30\n5 20 30\n0\n");
}

#[test]
fn dyn_array_two_state_default_is_zero() {
    // IEEE 1800-2017 §7.5.2 / doc-02: a `new[N]` element takes its type's
    // default — 0 for 2-state (int/bit/byte/shortint/longint), X for 4-state.
    // Oracle: iverilog live — int=0 0 0 / bit=0 0 / byte=0 0 / 4-state X / grow tail 0.
    let src = r#"
module t;
  int        di [];
  bit  [7:0] db [];
  byte       by [];
  logic[3:0] lg [];   // 4-state: X is correct, must NOT be coerced to 0
  int        gr [];
  initial begin
    di = new[3];
    db = new[2];
    by = new[2];
    lg = new[2];
    gr = new[2];
    gr[0] = 7; gr[1] = 9;
    gr = new[4](gr);   // grow: prefix copied, new tail = 2-state default 0
    $display("int=%0d %0d %0d", di[0], di[1], di[2]);
    $display("bit=%0d %0d", db[0], db[1]);
    $display("byte=%0d %0d", by[0], by[1]);
    $display("logic=%b %b", lg[0], lg[1]);
    $display("grow=%0d %0d %0d %0d", gr[0], gr[1], gr[2], gr[3]);
  end
endmodule
"#;
    assert_eq!(
        run_vita(src),
        "int=0 0 0\nbit=0 0\nbyte=0 0\nlogic=xxxx xxxx\ngrow=7 9 0 0\n"
    );
}

// ───────────────────────── queue (iverilog-pinned) ─────────────────────────

#[test]
fn queue_push_pop_index_size() {
    // Oracle: iverilog live — 3 / 5 10 20 / 20 / 20 2 / 5 1 / 0.
    let src = r#"
module t;
  integer q [$];
  integer x;
  initial begin
    q.push_back(10);
    q.push_back(20);
    q.push_front(5);
    $display("%0d", q.size());
    $display("%0d %0d %0d", q[0], q[1], q[2]);
    $display("%0d", q[$]);
    x = q.pop_back();
    $display("%0d %0d", x, q.size());
    x = q.pop_front();
    $display("%0d %0d", x, q.size());
    q.delete();
    $display("%0d", q.size());
  end
endmodule
"#;
    assert_eq!(run_vita(src), "3\n5 10 20\n20\n20 2\n5 1\n0\n");
}

#[test]
fn dyn_methods_in_expressions() {
    // Oracle: iverilog live — 7 / three. size() participates in arithmetic
    // and conditions like any int expression.
    let src = r#"
module t;
  integer q [$];
  initial begin
    q.push_back(7); q.push_back(8); q.push_back(9);
    $display("%0d", q.size() * 2 + 1);
    if (q.size() == 3) $display("three");
  end
endmodule
"#;
    assert_eq!(run_vita(src), "7\nthree\n");
}

#[test]
fn queue_dollar_arithmetic() {
    // `q[$-1]` — iverilog 13.0 REJECTS the syntax (its own gap); hand-IEEE pin
    // (§7.10.1: `$` = q.size()-1, so `$-1` = the second-to-last element).
    let src = r#"
module t;
  integer q [$];
  initial begin
    q.push_back(7); q.push_back(8); q.push_back(9);
    $display("%0d", q[$]);
    $display("%0d", q[$-1]);
  end
endmodule
"#;
    assert_eq!(run_vita(src), "9\n8\n");
}

#[test]
fn queue_write_at_size_appends() {
    // `q[q.size()] = v` = push_back equivalent (IEEE §7.10.1, legal-SILENT) —
    // iverilog live confirmed at the engine slice (size 1→2).
    let src = r#"
module t;
  integer q [$];
  initial begin
    q[0] = 11;
    q[1] = 22;
    $display("%0d %0d %0d", q.size(), q[0], q[1]);
  end
endmodule
"#;
    assert_eq!(run_vita(src), "2 11 22\n");
}

// ───────────────────────── assoc (hand-IEEE pinned) ─────────────────────────

#[test]
fn assoc_integer_key_roundtrip() {
    // Hand-IEEE (iverilog has no assoc support): negative keys legal,
    // exists 1/0, num counts, delete(k) then delete().
    let src = r#"
module t;
  integer a [integer];
  initial begin
    a[5] = 10;
    a[-3] = 20;
    $display("%0d %0d %0d", a[5], a[-3], a.num());
    $display("%0d %0d", a.exists(5), a.exists(6));
    a.delete(5);
    $display("%0d %0d", a.num(), a.exists(5));
    a.delete();
    $display("%0d", a.num());
  end
endmodule
"#;
    assert_eq!(run_vita(src), "10 20 2\n1 0\n1 0\n0\n");
}

#[test]
fn assoc_time_key_large() {
    // 64-bit `time` keys exercise the beyond-u32 lane through the front-end.
    let src = r#"
module t;
  integer a [time];
  time k;
  initial begin
    k = 64'd1099511627776; // 2^40
    a[k] = 7;
    $display("%0d %0d", a[k], a.size());
  end
endmodule
"#;
    assert_eq!(run_vita(src), "7 1\n");
}

// ───────────────────────── loud-reject lanes (MVP cuts) ─────────────────────────

#[test]
fn pop_outside_direct_rhs_is_loud() {
    // IEEE allows pops anywhere; the MVP pins them to the DIRECT rhs of a
    // blocking assign (engine StmtEffect intercept) — everything else is loud.
    assert_loud_reject(
        r#"
module t;
  integer q [$];
  integer x;
  initial x = q.pop_back() + 1;
endmodule
"#,
        "pop inside arithmetic",
    );
    assert_loud_reject(
        r#"
module t;
  integer q [$];
  integer x;
  initial x <= q.pop_back();
endmodule
"#,
        "pop as NBA rhs",
    );
}

#[test]
fn new_outside_blocking_assign_is_loud() {
    assert_loud_reject(
        r#"
module t;
  integer d [];
  integer x;
  initial x = new[3];
endmodule
"#,
        "new into a non-handle",
    );
}

#[test]
fn dollar_outside_queue_select_is_loud() {
    assert_loud_reject(
        r#"
module t;
  integer x;
  initial x = $;
endmodule
"#,
        "bare $ expression",
    );
}

#[test]
fn wrong_kind_method_is_loud() {
    assert_loud_reject(
        r#"
module t;
  integer d [];
  initial d.push_back(5);
endmodule
"#,
        "push on a dyn array",
    );
    assert_loud_reject(
        r#"
module t;
  integer d [];
  initial d.delete(0);
endmodule
"#,
        "indexed delete(i) on a dyn array (queue/assoc only)",
    );
}

#[test]
fn wildcard_assoc_is_loud() {
    // Both the canonical no-space `[*]` (lexed as one LBracketStar since slice S4)
    // and the spaced `[ *]` (lexed as `[`+`*`) must reject with the SAME precise
    // wildcard diagnostic — not a generic LBracketStar-token cascade.
    for decl in ["integer a[*];", "integer a [*];"] {
        let (_, err, ok) = run_vita_full(&format!(
            "module t;\n  {decl}\n  initial a[0] = 1;\nendmodule\n"
        ));
        assert!(
            !ok,
            "wildcard assoc must be rejected ({decl}); stderr:\n{err}"
        );
        assert!(
            err.to_lowercase().contains("wildcard"),
            "wildcard assoc must keep its precise diagnostic ({decl}); stderr:\n{err}"
        );
    }
}

#[test]
fn handle_misuse_is_loud() {
    // whole-handle copy (`d2 = d`) — IEEE-legal, MVP-deferred → loud.
    assert_loud_reject(
        r#"
module t;
  integer d [];
  integer d2 [];
  initial begin
    d = new[2];
    d2 = d;
  end
endmodule
"#,
        "whole-handle assignment",
    );
    // a handle as a port — excluded. The ANSI header form never parses
    // (E2002), which is the loud surface; the body-decl form is caught by
    // elaborate (E3009 port-dir guard).
    let (_, err, ok) = run_vita_full(
        r#"
module sub(input integer p []);
endmodule
module t;
  integer d [];
  sub s(.p(d));
endmodule
"#,
    );
    assert!(!ok, "dyn handle ANSI port must be rejected");
    assert!(err.contains("VITA-E"), "loud E-code expected; got:\n{err}");
    assert_loud_reject(
        r#"
module sub(p);
  input p;
  integer p [];
endmodule
module t;
  integer d [];
  sub s(.p(d));
endmodule
"#,
        "dyn handle non-ANSI port",
    );
}

#[test]
fn foreach_over_queue_and_dyn_array() {
    // Oracle: iverilog live — 12 / 0 10 20 30. `foreach (arr[i])` is a
    // parse-time desugar to a counting loop over arr.size() (no new IR).
    let src = r#"
module t;
  integer q [$];
  integer d [];
  integer sum;
  initial begin
    q.push_back(3); q.push_back(4); q.push_back(5);
    sum = 0;
    foreach (q[i]) sum = sum + q[i];
    $display("%0d", sum);
    d = new[4];
    foreach (d[k]) d[k] = k * 10;
    $display("%0d %0d %0d %0d", d[0], d[1], d[2], d[3]);
  end
endmodule
"#;
    assert_eq!(run_vita(src), "12\n0 10 20 30\n");
}

#[test]
fn foreach_index_is_block_local() {
    // The desugared index var must not collide with (or leak into) an outer
    // name — it lives in the synthetic block's decls.
    let src = r#"
module t;
  integer q [$];
  integer i;
  initial begin
    i = 99;
    q.push_back(1); q.push_back(2);
    foreach (q[i]) $display("%0d", q[i]);
    $display("%0d", i);
  end
endmodule
"#;
    assert_eq!(run_vita(src), "1\n2\n99\n");
}

// ───────────────────────── v6 follow-on batch ─────────────────────────

#[test]
fn queue_insert_delete_index() {
    // Oracle: iverilog live (2026-06-11) — insert shifts right, insert(size)
    // appends, OOB insert/delete warn + no-op (sizes unchanged).
    let src = r#"
module t;
  integer q [$];
  initial begin
    q.push_back(10); q.push_back(20); q.push_back(30);
    q.insert(1, 99);
    $display("%0d %0d %0d %0d %0d", q.size(), q[0], q[1], q[2], q[3]);
    q.insert(4, 77);
    $display("%0d %0d", q.size(), q[4]);
    q.insert(9, 55);
    $display("%0d", q.size());
    q.delete(0);
    q.delete(2);
    $display("%0d %0d %0d %0d", q.size(), q[0], q[1], q[2]);
    q.delete(7);
    $display("%0d", q.size());
  end
endmodule
"#;
    assert_eq!(run_vita(src), "4 10 99 20 30\n5 77\n5\n3 99 20 77\n3\n");
}

#[test]
fn assoc_first_next_hand_loop() {
    // hand-IEEE §7.9.4 (iverilog rejects assoc): 64-bit key var walks the
    // full signed-i64 key domain, ascending; last/prev walk back down.
    let src = r#"
module t;
  integer a [integer];
  reg signed [63:0] k;
  integer st;
  initial begin
    a[7] = 2; a[-3] = 1; a[64'd1099511627776] = 3;
    st = a.first(k);
    while (st == 1) begin
      $display("%0d -> %0d", k, a[k]);
      st = a.next(k);
    end
    st = a.last(k);
    $display("last %0d", k);
    st = a.prev(k);
    $display("prev %0d", k);
  end
endmodule
"#;
    assert_eq!(
        run_vita(src),
        "-3 -> 1\n7 -> 2\n1099511627776 -> 3\nlast 1099511627776\nprev 7\n"
    );
}

#[test]
fn foreach_assoc_walks_keys_ascending() {
    // hand-IEEE: foreach over an assoc = key order (signed ascending), the
    // index var holds the KEY (not a position).
    let src = r#"
module t;
  integer a [integer];
  initial begin
    a[5] = 50; a[-2] = 20; a[9] = 90;
    foreach (a[k]) $display("%0d=%0d", k, a[k]);
  end
endmodule
"#;
    assert_eq!(run_vita(src), "-2=20\n5=50\n9=90\n");
}

#[test]
fn foreach_assoc_wide_key_stops_loud() {
    // A key beyond the 32-bit foreach index → status −1 stops the walk and
    // the engine warns (W4020) — LOUD degrade, never a silent wrong walk.
    let src = r#"
module t;
  integer a [integer];
  initial begin
    a[64'd1099511627776] = 1;
    foreach (a[k]) $display("%0d", k);
    $display("done");
  end
endmodule
"#;
    let (out, err, ok) = run_vita_full(src);
    assert!(ok, "must still finish; stderr:\n{err}");
    assert!(
        out.starts_with("done"),
        "the walk must not yield a truncated key; got:\n{out}"
    );
    assert!(err.contains("W4020"), "truncation must warn: {err}");
}

#[test]
fn assoc_string_keys_roundtrip() {
    // hand-IEEE §7.8.2: [string] keys; literal keys at different padded
    // widths are the SAME key; first/next = lexicographic ("b" < "mode").
    let src = r#"
module t;
  integer cfg [string];
  reg [63:0] k;
  integer st;
  initial begin
    cfg["mode"] = 3;
    cfg["b"] = 1;
    $display("%0d %0d", cfg["mode"], cfg.num());
    if (cfg.exists("mode")) $display("has mode");
    if (!cfg.exists("nope")) $display("no nope");
    st = cfg.first(k);
    $display("%0d %0d", st, k);
    st = cfg.next(k);
    $display("%0d %0d", st, k);
    cfg.delete("mode");
    $display("%0d", cfg.num());
    cfg.delete();
    $display("%0d", cfg.num());
  end
endmodule
"#;
    assert_eq!(
        run_vita(src),
        "3 2\nhas mode\nno nope\n1 98\n1 1836016741\n1\n0\n"
    );
}

#[test]
fn foreach_string_assoc_short_keys() {
    // String keys ≤4 bytes fit the 32-bit foreach index — the walk yields
    // the packed key bytes ("aa"=24929, "b"=98 — lexicographic order).
    let src = r#"
module t;
  integer a [string];
  initial begin
    a["b"] = 2;
    a["aa"] = 1;
    foreach (a[k]) $display("%0d=%0d", k, a[k]);
  end
endmodule
"#;
    assert_eq!(run_vita(src), "24929=1\n98=2\n");
}

#[test]
fn iter_misuse_is_loud() {
    // user-surface first() on a queue (IEEE: assoc-only method)
    assert_loud_reject(
        r#"
module t;
  integer q [$];
  integer k, st;
  initial st = q.first(k);
endmodule
"#,
        "first on a queue",
    );
    // expression position (not the direct blocking rhs)
    assert_loud_reject(
        r#"
module t;
  integer a [integer];
  integer k;
  initial $display("%0d", a.first(k));
endmodule
"#,
        "first in expr position",
    );
    // bare statement (status discarded)
    assert_loud_reject(
        r#"
module t;
  integer a [integer];
  integer k;
  initial a.first(k);
endmodule
"#,
        "bare first stmt",
    );
    // non-variable key
    assert_loud_reject(
        r#"
module t;
  integer a [integer];
  wire w;
  integer st;
  initial st = a.first(w);
endmodule
"#,
        "wire iteration key",
    );
    // insert arity
    assert_loud_reject(
        r#"
module t;
  integer q [$];
  initial q.insert(1);
endmodule
"#,
        "insert arity",
    );
}

#[test]
fn string_literal_numeric_surface_is_ieee_order() {
    // Regression for the latent pre-v6 bug the string-keyed assoc work
    // exposed: string literals packed LSB-first, so their NUMERIC surface
    // was byte-reversed ("ab" → 25185). IEEE §5.9 + iverilog live: the first
    // character is the MOST significant byte → "ab" = 24930.
    let src = r#"
module t;
  initial begin
    $display("%0d", "ab");
    $display("%0d", "mode");
  end
endmodule
"#;
    assert_eq!(run_vita(src), "24930\n1836016741\n");
}

// ── v6 ③: bounded queue [$:N] (iverilog-pinned) ──

#[test]
fn bounded_queue_truncates_tail() {
    // Oracle: iverilog live (2026-06-11) — bound 2 = max size 3 (N+1).
    // ONE rule covers all ops: whatever ends beyond the bound falls off the
    // TAIL (push_back-on-full = skip; push_front/insert-on-full drop the
    // back element).
    let src = r#"
module t;
  integer q [$:2];
  integer x;
  initial begin
    q.push_back(1); q.push_back(2); q.push_back(3);
    $display("size=%0d", q.size());
    q.push_back(4);
    $display("after push4 size=%0d back=%0d", q.size(), q[q.size()-1]);
    q.push_front(0);
    $display("after pushf size=%0d front=%0d back=%0d", q.size(), q[0], q[q.size()-1]);
    q.insert(1, 99);
    $display("after ins size=%0d", q.size());
    x = q.pop_back();
    $display("pop=%0d size=%0d", x, q.size());
  end
endmodule
"#;
    let (out, err, ok) = run_vita_full(src);
    assert!(ok, "stderr:\n{err}");
    let mut body = String::new();
    for l in out.lines().filter(|l| !l.starts_with("simulation ended")) {
        body.push_str(l);
        body.push('\n');
    }
    assert_eq!(
        body,
        "size=3\nafter push4 size=3 back=3\nafter pushf size=3 front=0 back=2\nafter ins size=3\npop=1 size=2\n"
    );
    assert!(err.contains("W4020"), "bound drops must warn: {err}");
}

#[test]
fn bounded_queue_staged_roundtrip_preserves_bound() {
    // vcmp → velab → vrun: the bound rides the .velab trailer (sidecar), so
    // the staged flow enforces it exactly like the oneshot.
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_bq_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let sv = dir.join("b.sv");
    std::fs::write(
        &sv,
        r#"
module t;
  integer q [$:1];
  initial begin
    q.push_back(7); q.push_back(8); q.push_back(9);
    $display("%0d %0d %0d", q.size(), q[0], q[1]);
    $finish;
  end
endmodule
"#,
    )
    .unwrap();
    let vu = dir.join("b.vu");
    let velab = dir.join("b.velab");
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_vita"))
            .args(args)
            .output()
            .expect("run vita");
        (
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
            out.status.success(),
        )
    };
    let (_, err, ok) = run(&["vcmp", sv.to_str().unwrap(), "-o", vu.to_str().unwrap()]);
    assert!(ok, "vcmp: {err}");
    let (_, err, ok) = run(&["velab", vu.to_str().unwrap(), "-o", velab.to_str().unwrap()]);
    assert!(ok, "velab: {err}");
    let (out, err, ok) = run(&["vrun", velab.to_str().unwrap()]);
    assert!(ok, "vrun: {err}");
    assert!(
        out.starts_with("2 7 8\n"),
        "staged bound must hold (max size 2); got:\n{out}"
    );
    assert!(err.contains("W4020"), "staged drop warns: {err}");
}
