//! F-record-out (§4.5.215): a function with an OUTPUT/INOUT formal, called in a
//! SHORT-CIRCUIT `&&`/`||` operand of a `while`/`for` LOOP CONDITION
//! (`while (n < 5 && rsp_next(n, r) == 1) …`) is now supported. The loop condition is
//! lowered as an explicit short-circuit branch chain (`lower_shortcircuit_loop_cond`), so
//! the call's copy-out fires ONLY on the path that reaches it — never when the other
//! operand already short-circuits the result. Was E3009 "function has an output/inout
//! formal (illegal)".
//!
//! The round-19 report misdiagnosed this as a "≥2 string members" record issue; the real
//! trigger is purely CALL CONTEXT (a conditionally-evaluated output-formal call). Member
//! count is irrelevant (0-string and 2-string both work), and the copy-out itself is
//! value-correct on the already-working direct-rhs / plain-condition paths.
//!
//! ORACLE: iverilog 13.0 and verilator both REJECT a function with an output port
//! ("port is not an input port"), so this is hand-IEEE (§13.5.2 pass-by-value-result,
//! §11.4.7 short-circuit `&&`/`||`), cross-checked against the passing plain-condition
//! boundary. The short-circuit-no-call test uses a MODULE-net counter carried by an
//! `inout` formal (a genuine module-level side effect) to prove non-evaluation.
//!
//! §4.5.217 (round-19 follow-on) closes a SILENT-WRONG in the §4.5.216 `?:` transform: a
//! DEFINITE (0/1) condition's taken arm was lowered in ISOLATION (`x = T` / `x = E`), which
//! is byte-identical to the unified bare ternary ONLY when the two arms are coercion-safe.
//! `shortcircuit_rhs_special` now GATES the ternary split on `ternary_arms_coercion_safe`
//! (same effective signedness AND lhs ≥ both arm self-widths); a sign mismatch (§11.8.1
//! zero- vs sign-extend) or a narrow-lhs width divergence (§11.6.1 shift width) declines the
//! split → generic lowering → loud (correct-or-loud). The `&&`/`||` transform (1-bit boolean
//! result) and the c=X x_merge path (a real `Ternary` node — the engine unifies) are sound
//! and untouched.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ofsc_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn is_loud(o: &str) -> bool {
    o.contains("E3009")
}

// A record output formal with TWO string members — the report's shape (`rsp_t`).
const RSP_PKG: &str = "package pp;\n\
    typedef struct { int len_bits; int outputlen_bits; string msg_hex; string md_hex; } rsp_t;\n\
    function automatic int rsp_next (input int fd, output rsp_t r);\n\
      r.len_bits = fd*10; r.outputlen_bits = fd+1; r.msg_hex = \"mm\"; r.md_hex = \"dd\";\n\
      rsp_next = (fd < 2);\n\
    endfunction\n\
    endpackage\n";

// A framed function whose INOUT formal is a MODULE net used as a call counter: each call
// increments the module net, so a call that never fires leaves the counter untouched.
const STEP_CNT: &str = "int calls = 0;\n\
    function automatic int step (input int fd, inout int c);\n\
      c = c + 1;\n\
      step = (fd < 3);\n\
    endfunction\n";

// ── F-record-out repro: output-formal call in a `&&` while-condition ──
#[test]
fn output_formal_in_while_and() {
    let o = run(&format!(
        "{RSP_PKG}module t; import pp::*;\n\
        initial begin rsp_t r; int n = 0;\n\
          while (n < 5 && rsp_next(n, r) == 1) begin\n\
            $display(\"iter n=%0d len=%0d\", n, r.len_bits); n++;\n\
          end\n\
          $display(\"PASS n=%0d last_len=%0d\", n, r.len_bits); $finish;\n\
        end\n\
        endmodule\n"
    ));
    // rsp_next returns (fd<2): n=0,1 true → body twice; n=2 false → exit. r populated each
    // call (last call at n=2 still writes r.len_bits=20).
    assert!(
        !is_loud(&o)
            && o.contains("iter n=0 len=0")
            && o.contains("iter n=1 len=10")
            && o.contains("PASS n=2 last_len=20"),
        "F-record-out repro (&&):\n{o}"
    );
}

// ── THE critical short-circuit-correctness test: left operand false ⇒ the output-formal
//    call must NEVER fire and its module-net side effect must NEVER happen ──
#[test]
fn output_formal_and_shortcircuit_no_call() {
    let o = run(&format!(
        "module t;\n{STEP_CNT}\
        logic gate = 0;\n\
        initial begin int n = 0;\n\
          while (gate && step(n, calls) == 1) begin n++; end\n\
          $display(\"calls=%0d n=%0d\", calls, n); $finish;\n\
        end\n\
        endmodule\n"
    ));
    // gate is false ⇒ `&&` short-circuits ⇒ step() never called ⇒ counter stays 0, body
    // never runs. If the call were made unconditional this would be calls>0 (silent-wrong).
    assert!(
        !is_loud(&o) && o.contains("calls=0 n=0"),
        "short-circuit no-call (&&):\n{o}"
    );
}

// ── `||` variant: the left operand being TRUE short-circuits the call ──
#[test]
fn output_formal_or_shortcircuit() {
    let o = run("module t;\n\
        int calls = 0;\n\
        function automatic int step (input int fd, inout int c);\n\
          c = c + 1; step = (fd < 5);\n\
        endfunction\n\
        initial begin int cnt = 0;\n\
          while ((cnt < 3) || step(cnt, calls)) begin cnt++; end\n\
          $display(\"calls=%0d cnt=%0d\", calls, cnt); $finish;\n\
        end\n\
        endmodule\n");
    // cnt 0,1,2: (cnt<3) TRUE ⇒ `||` short-circuits ⇒ step NOT called (3 iters).
    // cnt 3,4,5: (cnt<3) FALSE ⇒ step called (returns fd<5): cnt3→T,cnt4→T,cnt5→F ⇒ exit.
    // step called exactly 3 times (only when the left operand is false).
    assert!(
        !is_loud(&o) && o.contains("calls=3 cnt=5"),
        "|| short-circuit:\n{o}"
    );
}

// ── ≥2 string members: value-correct via the `&&` path (locks in "member count irrelevant") ──
#[test]
fn output_formal_2strings_valcorrect() {
    let o = run(&format!(
        "{RSP_PKG}module t; import pp::*;\n\
        initial begin rsp_t r; int n = 0;\n\
          while (n < 5 && rsp_next(n, r) == 1) begin\n\
            $display(\"n=%0d len=%0d ol=%0d msg=%s md=%s\", n, r.len_bits, r.outputlen_bits, r.msg_hex, r.md_hex);\n\
            n++;\n\
          end\n\
          $finish;\n\
        end\n\
        endmodule\n"
    ));
    // Both string members copied out correctly every call; int members too.
    assert!(
        !is_loud(&o)
            && o.contains("n=0 len=0 ol=1 msg=mm md=dd")
            && o.contains("n=1 len=10 ol=2 msg=mm md=dd"),
        "2-string record value-correct (&&):\n{o}"
    );
}

// ── all-int output record via `&&` → value-correct (0 string members) ──
#[test]
fn output_formal_0strings_valcorrect() {
    let o = run("package qp;\n\
        typedef struct { int a; int b; int c; } allint_t;\n\
        function automatic int nxt (input int fd, output allint_t r);\n\
          r.a = fd; r.b = fd*2; r.c = fd*3; nxt = (fd < 2);\n\
        endfunction\n\
        endpackage\n\
        module t; import qp::*;\n\
          initial begin allint_t r; int n = 0;\n\
            while (n < 5 && nxt(n, r) == 1) begin\n\
              $display(\"n=%0d a=%0d b=%0d c=%0d\", n, r.a, r.b, r.c); n++;\n\
            end\n\
            $display(\"PASS n=%0d\", n); $finish;\n\
          end\n\
        endmodule\n");
    assert!(
        !is_loud(&o)
            && o.contains("n=0 a=0 b=0 c=0")
            && o.contains("n=1 a=1 b=2 c=3")
            && o.contains("PASS n=2"),
        "0-string record value-correct (&&):\n{o}"
    );
}

// ── the call is a BARE operand (no `== 1` comparison) ──
#[test]
fn output_formal_and_bare_call() {
    let o = run(&format!(
        "module t;\n{STEP_CNT}\
        initial begin int n = 0;\n\
          while (n < 10 && step(n, calls)) begin n++; end\n\
          $display(\"calls=%0d n=%0d\", calls, n); $finish;\n\
        end\n\
        endmodule\n"
    ));
    // step returns (fd<3): n=0,1,2 true (body), n=3 false → exit. Called 4×, body 3×.
    assert!(
        !is_loud(&o) && o.contains("calls=4 n=3"),
        "bare-call operand (&&):\n{o}"
    );
}

// ── `for`-loop condition variant (lower_for splits identically) ──
#[test]
fn output_formal_in_for_and() {
    let o = run(&format!(
        "module t;\n{STEP_CNT}\
        initial begin\n\
          for (int n = 0; n < 10 && step(n, calls) == 1; n++) $display(\"n=%0d\", n);\n\
          $display(\"calls=%0d\", calls); $finish;\n\
        end\n\
        endmodule\n"
    ));
    // n=0,1,2 → step true → body; n=3 → step false → exit. Called 4×.
    assert!(
        !is_loud(&o)
            && o.contains("n=0")
            && o.contains("n=1")
            && o.contains("n=2")
            && !o.contains("n=3")
            && o.contains("calls=4"),
        "for-loop && condition:\n{o}"
    );
}

// ── a call-free operand may itself be a `&&`/`||` (left-assoc top-level `&&` with a
//    call-free LHS) — correct-by-construction (the LHS lowers with normal short-circuit,
//    the call stays guarded in `eval_b`). Locks in the behavior. ──
#[test]
fn output_formal_deeper_top_level_and_supported() {
    let o = run(&format!(
        "module t;\n{STEP_CNT}\
        logic t1 = 1, t2 = 1;\n\
        initial begin int n = 0;\n\
          while (t1 && t2 && step(n, calls)) begin n++; end\n\
          $display(\"calls=%0d n=%0d\", calls, n); $finish;\n\
        end\n\
        endmodule\n"
    ));
    // Equivalent to the plain-condition reference (t1&&t2 always true): called 4×, body 3×.
    assert!(
        !is_loud(&o) && o.contains("calls=4 n=3"),
        "deeper top-level && (call-free LHS):\n{o}"
    );
}

// ── adversarial: the LHS READS the same net the RHS call MUTATES. Each iteration's LHS
//    (evaluated at the loop head) must see the PREVIOUS iteration's copy-out value. ──
#[test]
fn output_formal_lhs_reads_mutated_net() {
    let o = run("module t;\n\
        int calls = 0;\n\
        function automatic int step (input int fd, inout int c);\n\
          c = c + 1; step = (fd < 10);\n\
        endfunction\n\
        initial begin int n = 0;\n\
          while ((calls < 3) && step(n, calls)) begin n++; end\n\
          $display(\"calls=%0d n=%0d\", calls, n); $finish;\n\
        end\n\
        endmodule\n");
    // n0 calls0: 0<3 T→step calls1 T→body; n1 c1:1<3 T→step c2 T→body; n2 c2:2<3 T→step c3 T→body;
    // n3 c3: 3<3 F→exit (step NOT called). ⇒ calls=3 n=3.
    assert!(
        !is_loud(&o) && o.contains("calls=3 n=3"),
        "adversarial LHS-reads-mutated:\n{o}"
    );
}

// ══════════════════ regressions: already-working paths unchanged ══════════════════

#[test]
fn output_formal_direct_rhs_regression() {
    let o = run(&format!(
        "{RSP_PKG}module t; import pp::*;\n\
        initial begin rsp_t r; int st;\n\
          st = rsp_next(1, r);\n\
          $display(\"st=%0d len=%0d\", st, r.len_bits); $finish;\n\
        end\n\
        endmodule\n"
    ));
    assert!(
        !is_loud(&o) && o.contains("st=1 len=10"),
        "direct-rhs regression:\n{o}"
    );
}

#[test]
fn output_formal_plain_condition_regression() {
    let o = run(&format!(
        "{RSP_PKG}module t; import pp::*;\n\
        initial begin rsp_t r; int n = 0;\n\
          while (rsp_next(n, r) == 1) begin n++; end\n\
          $display(\"n=%0d\", n); $finish;\n\
        end\n\
        endmodule\n"
    ));
    assert!(
        !is_loud(&o) && o.contains("n=2"),
        "plain-cond regression:\n{o}"
    );
}

// ══════════════════ correct-or-loud: documented follow-ons stay LOUD ══════════════════

// §4.5.215 FOLLOW-ON: an output/inout-formal call in an `if`-condition `&&`/`||` is now
// lowered via the SAME short-circuit branch chain as a loop cond (`lower_shortcircuit_cond`
// routing to the if's then_bb/else_bb) — supported.
#[test]
fn output_formal_if_cond_and_now_supported() {
    let o = run(&format!(
        "module t;\n{STEP_CNT}\
        initial begin int n = 0;\n\
          if (n < 5 && step(n, calls) == 1) $display(\"hi\");\n\
          else $display(\"no\");\n\
          $display(\"calls=%0d\", calls); $finish;\n\
        end\n\
        endmodule\n"
    ));
    // n<5 true → step(0) called (calls=1), returns 0<3=1 → ==1 true → "hi".
    assert!(
        !is_loud(&o) && o.contains("hi") && o.contains("calls=1"),
        "if-cond && output-formal call should be supported:\n{o}"
    );
}

// THE critical short-circuit-no-call for if-cond: left operand FALSE ⇒ the call NEVER fires
// and its inout copy-out never touches the counter (calls stays 0).
#[test]
fn output_formal_if_cond_shortcircuit_no_call() {
    let o = run(&format!(
        "module t;\n{STEP_CNT}\
        initial begin int gate = 0;\n\
          if (gate && step(0, calls) == 1) $display(\"taken\");\n\
          else $display(\"not-taken\");\n\
          $display(\"calls=%0d\", calls); $finish;\n\
        end\n\
        endmodule\n"
    ));
    assert!(
        !is_loud(&o) && o.contains("not-taken") && o.contains("calls=0"),
        "gate false ⇒ step must NEVER be called (calls=0):\n{o}"
    );
}

// if-cond `||`: left operand TRUE ⇒ the RHS call is short-circuited (never fires).
#[test]
fn output_formal_if_cond_or_shortcircuit() {
    let o = run(&format!(
        "module t;\n{STEP_CNT}\
        initial begin int done = 1;\n\
          if (done || step(0, calls) == 1) $display(\"taken\");\n\
          else $display(\"not-taken\");\n\
          $display(\"calls=%0d\", calls); $finish;\n\
        end\n\
        endmodule\n"
    ));
    assert!(
        !is_loud(&o) && o.contains("taken") && o.contains("calls=0"),
        "done true ⇒ || short-circuits, step never called (calls=0):\n{o}"
    );
}

// ══════════════════ §4.5.216: `?:` arm / general `&&`/`||` RHS supported ══════════════════
// An output/inout-formal call in a CONDITIONALLY-evaluated whole rhs (a `?:` arm or a
// top-level short-circuit `&&`/`||` RHS) is lowered as explicit control flow that assigns
// `lhs` on every path (`shortcircuit_rhs_special` → `lower_ternary_rhs` /
// `lower_shortcircuit_rhs`), so the copy-out fires ONLY on the path that reaches it. The
// result value is byte-identical to a bare `c?T:E` / `A&&B` / `A||B` (assembled with the
// engine's own tri-valued `merge_x`/`log_and`/`log_or`, incl. the 4-state corners). Same
// NO ORACLE as §4.5.215 (iverilog/verilator reject an output-formal function) → hand-IEEE
// (§11.4.7/§11.4.11) cross-checked by a within-vita differential vs the call-free form.

// §4.5.216 FLIP (was `_stays_loud`): a `?:` arm output-formal call is now supported.
#[test]
fn output_formal_ternary_arm_now_supported() {
    let o = run(&format!(
        "module t;\n{STEP_CNT}\
        initial begin int n = 0, x;\n\
          x = (n < 5) ? step(n, calls) : 0;\n\
          $display(\"x=%0d calls=%0d\", x, calls); $finish;\n\
        end\n\
        endmodule\n"
    ));
    // n<5 true ⇒ THEN arm taken: step(0) called (calls=1), returns 0<3=1 ⇒ x=1.
    assert!(
        !is_loud(&o) && o.contains("x=1 calls=1"),
        "?: arm output-formal call should be supported:\n{o}"
    );
}

// THE critical short-circuit-no-call for `?:`: the NOT-taken arm's call must NEVER fire
// (its inout copy-out never touches the module counter).
#[test]
fn ternary_arm_shortcircuit_no_call() {
    let o = run(&format!(
        "module t;\n{STEP_CNT}\
        initial begin int n = 9, x;\n\
          x = (n < 5) ? step(n, calls) : 42;\n\
          $display(\"x=%0d calls=%0d\", x, calls); $finish;\n\
        end\n\
        endmodule\n"
    ));
    // n<5 FALSE ⇒ ELSE arm (42); the THEN arm's step() must NEVER be called ⇒ calls=0.
    assert!(
        !is_loud(&o) && o.contains("x=42 calls=0"),
        "not-taken ?: arm must NEVER fire its call (calls=0):\n{o}"
    );
}

// Both arms carry a call: each fires only on its own taken path (independent counters).
#[test]
fn ternary_both_arms_have_calls() {
    let o = run("module t;\n\
        int ca = 0, cb = 0;\n\
        function automatic int fa (input int v, inout int c); c = c + 1; fa = v*10; endfunction\n\
        function automatic int fb (input int v, inout int c); c = c + 1; fb = v*100; endfunction\n\
        initial begin int x;\n\
          for (int n = 0; n < 3; n++) begin\n\
            x = (n < 1) ? fa(n, ca) : fb(n, cb);\n\
            $display(\"n=%0d x=%0d ca=%0d cb=%0d\", n, x, ca, cb);\n\
          end\n\
          $finish;\n\
        end\n\
        endmodule\n");
    // n=0 → fa (ca=1, x=0); n=1,2 → fb (cb=1,2, x=100,200). ca frozen at 1 (fa only at n=0).
    assert!(
        !is_loud(&o)
            && o.contains("n=0 x=0 ca=1 cb=0")
            && o.contains("n=1 x=100 ca=1 cb=1")
            && o.contains("n=2 x=200 ca=1 cb=2"),
        "both ?: arms with calls — each fires only on its taken path:\n{o}"
    );
}

// Within-vita differential: a `?:` whose condition is X must produce the SAME 4-state
// result (bit-exact via `===`) as vita's own `c ? T : E` — the x_merge path reproduces
// the engine's `merge_x`. `mine` fires the intercept (THEN arm has a call); `refv` (arms
// call-free) lowers via the normal ternary path. The arms are COERCION-SAFE (both 4-bit
// unsigned, lhs 4-bit — §4.5.217), so the transform still fires; c is X ⇒ ONLY the x_merge
// path runs (both copy-outs fire, exactly as a bare ternary evaluates both when c is x) ⇒
// both = merge(3,5) = 4'b0xx1.
#[test]
fn ternary_value_matches_bare_xcorner() {
    let o = run("module t;\n\
        int calls = 0;\n\
        function automatic logic [3:0] f (input int v, inout int c); c = c + 1; f = v; endfunction\n\
        initial begin logic c; logic [3:0] mine, refv;\n\
          mine = c ? f(3, calls) : 4'd5;\n\
          refv = c ? 4'd3 : 4'd5;\n\
          $display(\"eq=%b mine=%b\", mine === refv, mine); $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("eq=1") && o.contains("mine=0xx1"),
        "?: c=x merge value must be bit-exact to bare `c ? 4'd3 : 4'd5`:\n{o}"
    );
}

// §4.5.217 (was `ternary_width_mismatch_matches_bare`, a supported "value matches"): the
// arms here are NOT coercion-safe — a signed `byte` arm and an unsigned 16-bit arm, lhs only
// 4-bit. The taken value is COINCIDENTALLY correct (the low nibble is 0x7 either way — the
// sign/width divergence lands above bit 3 and is truncated), but the gate cannot prove that
// statically, so it conservatively declines the split → loud (correct-or-loud). Before the
// §4.5.217 gate this was silently transformed. (Contrast `ternary_arm_width_mismatch_stays_loud`,
// where a shift pulls the divergence down INTO the surviving bits — a genuine silent-wrong.)
#[test]
fn ternary_width_mismatch_now_loud() {
    let o = run("module t;\n\
        int calls = 0;\n\
        function automatic byte f (input byte v, inout int c); c = c + 1; f = v; endfunction\n\
        initial begin bit sel = 1; logic [3:0] mine; byte a = 8'hA7;\n\
          mine = sel ? f(8'hA7, calls) : 16'h0123;\n\
          $display(\"mine=%h\", mine); $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        is_loud(&o),
        "sign/width-mismatched ?: arms must stay loud (coercion-unsafe):\n{o}"
    );
}

// §4.5.217 SILENT-WRONG FIX (repro 1 — SIGN contamination): a SIGNED narrow arm (`a`,
// 4-bit) and an UNSIGNED wide call arm (`f` returns `logic [7:0]`). A bare `sel ? a : f(..)`
// is §11.8.1 UNSIGNED (one unsigned operand ⇒ the whole is unsigned ⇒ ZERO-extend `a`) ⇒
// 0x0A when `sel`=1. Lowering the taken THEN arm in isolation (`x = a`) would SIGN-extend
// `a` (0xFA) — silently wrong. The gate sees the sign mismatch and declines → loud; it must
// NEVER silently produce 0xFA.
#[test]
fn ternary_arm_sign_mismatch_stays_loud() {
    let o = run("module t;\n\
        int calls = 0;\n\
        logic signed [3:0] a = 4'sb1010;\n\
        function automatic logic [7:0] f (input int d, inout int c); c = c + 1; f = 8'h0A; endfunction\n\
        initial begin logic sel = 1; logic [7:0] x;\n\
          x = sel ? a : f(0, calls);\n\
          $display(\"x=%h\", x); $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        is_loud(&o) && !o.contains("x=fa"),
        "sign-mismatched ?: arms must stay loud, never silently 0xFA:\n{o}"
    );
}

// §4.5.217 SILENT-WRONG FIX (repro 2 — WIDTH divergence): both arms SIGNED, but the sibling
// call arm (`g` returns `logic signed [15:0]`) is WIDER than the lhs (8-bit). A bare
// `sel ? (a>>1) : g(..)` evaluates the taken `a>>1` at the unified width 16 (`a` sign-extended
// to 16 FIRST, then logical `>>1`) ⇒ 0xfd; lowering it in isolation widens `a` only to lhs=8
// ⇒ 0x7d — silently wrong, because the shift pulls the extended top bit DOWN into the low
// (surviving) bits. lhs < max(arm self-widths) ⇒ gate declines → loud.
#[test]
fn ternary_arm_width_mismatch_stays_loud() {
    let o = run("module t;\n\
        int calls = 0;\n\
        logic signed [3:0] a = 4'sb1010;\n\
        function automatic logic signed [15:0] g (input int d, inout int c); c = c + 1; g = 16'sh0005; endfunction\n\
        initial begin logic sel = 1; logic [7:0] x;\n\
          x = sel ? (a >> 1) : g(0, calls);\n\
          $display(\"x=%h\", x); $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        is_loud(&o) && !o.contains("x=7d"),
        "width-mismatched ?: arms must stay loud, never silently 0x7d:\n{o}"
    );
}

// §4.5.217: a COERCION-SAFE `?:` (same effective signedness AND lhs ≥ both arm self-widths)
// carrying an output/inout-formal call arm still fires the transform and is byte-identical
// (within-vita `===`) to the bare CALL-FREE ternary. Both arms unsigned 16-bit, lhs 16-bit.
#[test]
fn ternary_arm_coercion_safe_supported() {
    let o = run("module t;\n\
        int calls = 0;\n\
        function automatic logic [15:0] f (input logic [15:0] v, inout int c); c = c + 1; f = v; endfunction\n\
        initial begin bit sel = 1; logic [15:0] mine, refv; logic [15:0] a = 16'hBEEF;\n\
          mine = sel ? f(a, calls) : 16'h1234;\n\
          refv = sel ? a : 16'h1234;\n\
          $display(\"eq=%b mine=%h calls=%0d\", mine === refv, mine, calls); $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("eq=1") && o.contains("mine=beef") && o.contains("calls=1"),
        "coercion-safe ?: with a call arm must be supported and match the bare ternary:\n{o}"
    );
}

// §4.5.216 FLIP (was `_stays_loud`): a general `x = A && f(out r)` (top-level `&&`, whole
// rhs) is now supported.
#[test]
fn output_formal_general_expr_now_supported() {
    let o = run(&format!(
        "module t;\n{STEP_CNT}\
        initial begin int n = 0; logic x;\n\
          x = (n < 5) && (step(n, calls) == 1);\n\
          $display(\"x=%0d calls=%0d\", x, calls); $finish;\n\
        end\n\
        endmodule\n"
    ));
    // n<5 true ⇒ step(0) called (calls=1), returns 1 ⇒ ==1 true ⇒ x=1.
    assert!(
        !is_loud(&o) && o.contains("x=1 calls=1"),
        "general-expr && output-formal call should be supported:\n{o}"
    );
}

// THE critical short-circuit-no-call for `&&`: left operand FALSE ⇒ the RHS call never
// fires and its inout counter stays 0.
#[test]
fn general_expr_and_shortcircuit_no_call() {
    let o = run(&format!(
        "module t;\n{STEP_CNT}\
        initial begin logic gate = 0; logic x;\n\
          x = gate && (step(0, calls) == 1);\n\
          $display(\"x=%0d calls=%0d\", x, calls); $finish;\n\
        end\n\
        endmodule\n"
    ));
    assert!(
        !is_loud(&o) && o.contains("x=0 calls=0"),
        "gate false ⇒ && RHS call must NEVER fire (calls=0):\n{o}"
    );
}

// THE critical short-circuit-no-call for `||`: left operand TRUE ⇒ the RHS call never fires.
#[test]
fn general_expr_or_shortcircuit_no_call() {
    let o = run(&format!(
        "module t;\n{STEP_CNT}\
        initial begin logic done = 1; logic x;\n\
          x = done || (step(0, calls) == 1);\n\
          $display(\"x=%0d calls=%0d\", x, calls); $finish;\n\
        end\n\
        endmodule\n"
    ));
    assert!(
        !is_loud(&o) && o.contains("x=1 calls=0"),
        "done true ⇒ || RHS call must NEVER fire (calls=0):\n{o}"
    );
}

// Within-vita differential: `&&` with an X-valued LEFT operand must be bit-exact to vita's
// own `&&`. `mine` fires the intercept (RHS has a call); `refv` (call-free RHS) is the
// normal path. A is X, both RHS true ⇒ `x && 1` = X on both ⇒ `===` holds. The call still
// FIRES for A=X (IEEE: `&&` needs its RHS when the LHS is X) — calls=1.
#[test]
fn general_expr_and_value_matches_bare_xcorner() {
    let o = run("module t;\n\
        int calls = 0;\n\
        function automatic int step (input int fd, inout int c); c = c + 1; step = (fd < 3); endfunction\n\
        initial begin logic a; logic mine, refv;\n\
          mine = a && (step(0, calls) == 1);\n\
          refv = a && (1 == 1);\n\
          $display(\"eq=%b mine=%b calls=%0d\", mine === refv, mine, calls); $finish;\n\
        end\n\
        endmodule\n");
    assert!(
        !is_loud(&o) && o.contains("eq=1") && o.contains("mine=x") && o.contains("calls=1"),
        "&& x-corner must be bit-exact to bare `a && (1==1)` and still fire the call:\n{o}"
    );
}

// A BURIED call (`y = (A && f()) + 1` — the call is NOT the whole rhs) stays loud: the
// intercept matches only a WHOLE-rhs ternary / short-circuit form (correct-or-loud).
#[test]
fn output_formal_buried_and_stays_loud() {
    let o = run(&format!(
        "module t;\n{STEP_CNT}\
        initial begin int n = 0; int y;\n\
          y = ((n < 5) && (step(n, calls) == 1)) + 1;\n\
          $display(\"y=%0d\", y); $finish;\n\
        end\n\
        endmodule\n"
    ));
    assert!(
        is_loud(&o),
        "buried && call (not the whole rhs) should stay loud:\n{o}"
    );
}

// The call NESTED inside a loop-condition operand's own `&&`/`||` (not a top-level operand)
// stays loud — the split only isolates the two TOP-LEVEL operands.
#[test]
fn output_formal_nested_in_operand_stays_loud() {
    let o = run(&format!(
        "module t;\n{STEP_CNT}\
        logic b = 0;\n\
        initial begin int n = 0;\n\
          while (n < 5 && (b || step(n, calls) == 1)) n++;\n\
          $display(\"n=%0d\", n); $finish;\n\
        end\n\
        endmodule\n"
    ));
    assert!(
        is_loud(&o),
        "call nested in a `||` operand should stay loud:\n{o}"
    );
}
