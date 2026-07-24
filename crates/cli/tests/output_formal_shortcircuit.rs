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

// An output-formal call in an `if`-condition `&&` (not a loop condition) is NOT split —
// stays loud (follow-on: the split is scoped to while/for conditions).
#[test]
fn output_formal_if_cond_and_stays_loud() {
    let o = run(&format!(
        "module t;\n{STEP_CNT}\
        initial begin int n = 0;\n\
          if (n < 5 && step(n, calls) == 1) $display(\"hi\");\n\
          $finish;\n\
        end\n\
        endmodule\n"
    ));
    assert!(is_loud(&o), "if-cond && should stay loud:\n{o}");
}

// A `?:` arm output-formal call stays loud (documented follow-on).
#[test]
fn output_formal_ternary_arm_stays_loud() {
    let o = run(&format!(
        "module t;\n{STEP_CNT}\
        initial begin int n = 0, x;\n\
          x = (n < 5) ? step(n, calls) : 0;\n\
          $display(\"x=%0d\", x); $finish;\n\
        end\n\
        endmodule\n"
    ));
    assert!(is_loud(&o), "?: arm should stay loud:\n{o}");
}

// A general expression `x = A && f(out r)` (not a loop condition) stays loud.
#[test]
fn output_formal_general_expr_stays_loud() {
    let o = run(&format!(
        "module t;\n{STEP_CNT}\
        initial begin int n = 0; logic x;\n\
          x = (n < 5) && (step(n, calls) == 1);\n\
          $display(\"x=%0d\", x); $finish;\n\
        end\n\
        endmodule\n"
    ));
    assert!(is_loud(&o), "general-expr && should stay loud:\n{o}");
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
