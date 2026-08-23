//! §3 ③ — the direct-rhs-only system functions in an arbitrary expression position.
//!
//! `$fgetc`, `$value$plusargs`, `$fscanf`, `$fopen` and their siblings each mutate state
//! from inside the call, so vita lowered them as statement-level special forms and
//! `E3009`'d every other placement. Both oracles accept them anywhere an expression goes:
//! a 42-cell census (4 forms × 11 positions) found vita loud in 41, the sole exception
//! being `if ($value$plusargs(…))`, which had its own one-family one-position desugar.
//!
//! `hoist/special.rs` generalises that desugar: evaluate the call into a temp before the
//! statement, read the temp where it stood. Every VALUE below is iverilog 13.0's, taken
//! live; the loud cells are the positions the pass declines on purpose.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `src` in a fresh directory holding a data file `d.txt` = "ABCDEFGH", so `$fgetc`
/// reads 65, 66, … and hits EOF (-1) after eight bytes.
fn run(src: &str) -> (String, Option<i32>, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_rhsonly_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("d.txt"), "ABCDEFGH").unwrap();
    let path = dir.join("t.sv");
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&dir)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&dir);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A design whose `initial` block opens `d.txt` as `fd` and then runs `body`.
fn design(decls: &str, body: &str) -> String {
    format!(
        "module t;\n  integer fd; integer r; integer k; integer n; integer m[0:255];\n\
         {decls}\n  initial begin\n    k = 0; r = 0; n = 0;\n\
         fd = $fopen(\"d.txt\", \"r\");\n{body}\n    #10 $finish;\n  end\nendmodule\n"
    )
}

/// The value the oracle prints must be the value vita prints, at exit 0.
fn runs(decls: &str, body: &str, want: &str) {
    let (out, code, err) = run(&design(decls, body));
    assert_eq!(code, Some(0), "expected a run, stderr:\n{err}");
    assert!(
        out.contains(&format!("VAL={want}")),
        "want VAL={want}; got:\n{out}"
    );
}

/// A position the pass declines: exit 1, and the message must still name the real reason.
fn loud(decls: &str, body: &str) {
    let (out, code, err) = run(&design(decls, body));
    assert_eq!(
        code,
        Some(1),
        "expected a loud reject, not {code:?}:\n{out}"
    );
    assert!(
        err.contains("direct rhs of a blocking assignment"),
        "expected the direct-rhs diagnostic, got:\n{err}"
    );
}

// ── the positions the pass opens (iverilog values, measured) ──────────────────────

#[test]
fn nonblocking_rhs_is_the_darkriscv_blocker() {
    // `UART_RFIFO <= $fgetc(fd);` — darkuart.v:303, the single error that kept the whole
    // 3115-line darkriscv SoC from elaborating. The rhs is SAMPLED in Active exactly like
    // a blocking one, so hoisting it changes nothing about when the read happens.
    runs(
        "",
        "    r <= $fgetc(fd);\n    #1 $display(\"VAL=%0d\", r);",
        "65",
    );
}

#[test]
fn nonblocking_rhs_with_an_indexed_lvalue() {
    // darkuart.v:301, the `__UARTQUEUE__` twin: both the lvalue index and the rhs carry
    // work. iverilog evaluates the rhs first.
    runs(
        "",
        "    k = 3;\n    m[k] <= $fgetc(fd);\n    #1 $display(\"VAL=%0d\", m[3]);",
        "65",
    );
}

#[test]
fn if_condition_under_a_unary_not() {
    // serv's testbench idiom. `if ($value$plusargs(…))` already worked — the bare call was
    // the one shape `lower_branch_cond` desugared — but one `!` put it back to E3009.
    runs(
        "",
        "    if (!$value$plusargs(\"N=%d\", n)) $display(\"VAL=t\"); else $display(\"VAL=f\");",
        "t",
    );
}

#[test]
fn nested_in_arithmetic_in_a_blocking_rhs() {
    // Not the DIRECT rhs, so this was loud even in the position the feature was named for.
    runs(
        "",
        "    r = $fgetc(fd) + 1;\n    $display(\"VAL=%0d\", r);",
        "66",
    );
}

#[test]
fn a_system_task_argument() {
    runs("", "    $display(\"VAL=%0d\", $fgetc(fd));", "65");
}

#[test]
fn a_case_scrutinee() {
    runs(
        "",
        "    case ($fgetc(fd)) 65: $display(\"VAL=a\"); default: $display(\"VAL=d\"); endcase",
        "a",
    );
}

#[test]
fn a_repeat_count_is_evaluated_once() {
    // `repeat (n)` reads its count ONCE — unlike a loop condition, which is why one is
    // hoisted and the other is not. 65 iterations, one `$fgetc`.
    runs(
        "",
        "    repeat ($fgetc(fd)) k = k + 1;\n    $display(\"VAL=%0d\", k);",
        "65",
    );
}

#[test]
fn an_lvalue_index() {
    runs(
        "",
        "    m[$fgetc(fd) & 3] = 9;\n    $display(\"VAL=%0d\", m[1]);",
        "9",
    );
}

#[test]
fn eof_is_minus_one_so_the_temp_must_be_signed() {
    // ⚠️ The temp is a 32-bit SIGNED `Integer` on purpose. Built as an unsigned `Reg` —
    // which is what the existing `fresh_ia_tmp` makes — `$fgetc(fd) != -1` compares
    // 4294967295 against -1 and is always true. The direct-rhs form never showed this
    // because it assigns straight to the user's own `integer`.
    runs(
        "",
        "    repeat (9) r = $fgetc(fd);\n    $display(\"VAL=%0d\", ($fgetc(fd) < 0) ? 1 : 0);",
        "1",
    );
}

#[test]
fn two_calls_in_one_expression_keep_their_order() {
    // Left to right: 65 then 66.
    runs(
        "",
        "    r = $fgetc(fd) * 1000 + $fgetc(fd);\n    $display(\"VAL=%0d\", r);",
        "65066",
    );
}

#[test]
fn the_rhs_is_evaluated_before_the_lvalue_index() {
    // iverilog: `m[$fgetc(f)] = $fgetc(f)` stores 65 at index 66. Hoisting the index but
    // not the rhs reversed this — a silent swap at exit 0, caught by measurement.
    runs(
        "",
        "    m[$fgetc(fd)] = $fgetc(fd);\n    $display(\"VAL=%0d\", m[66]);",
        "65",
    );
}

#[test]
fn a_ref_write_may_be_read_by_the_calls_own_argument() {
    // `n` is the call's OWN ref argument, so the hoist does not move a read past a write.
    runs(
        "",
        "    n = 99;\n    r = $value$plusargs(\"N=%d\", n);\n    $display(\"VAL=%0d %0d\", r, n);",
        "0 99",
    );
}

// ── the positions the pass declines (correct-or-loud) ─────────────────────────────

#[test]
fn a_short_circuit_right_operand_stays_loud() {
    // §11.4.7 may skip the right operand entirely; hoisting would run the read anyway.
    loud(
        "",
        "    r = (k != 0) && ($fgetc(fd) != 0);\n    $display(\"VAL=%0d\", r);",
    );
}

#[test]
fn a_ternary_arm_stays_loud() {
    // §11.4.11: an arm runs only when the condition selects it (and an x runs both).
    loud(
        "",
        "    r = (k != 0) ? $fgetc(fd) : 7;\n    $display(\"VAL=%0d\", r);",
    );
}

#[test]
fn a_loop_condition_stays_loud() {
    // Re-evaluated per iteration; a one-shot hoist would read once and spin.
    loud(
        "",
        "    while ($fgetc(fd) != -1) k = k + 1;\n    $display(\"VAL=%0d\", k);",
    );
}

#[test]
fn a_ref_write_read_elsewhere_in_the_statement_stays_loud() {
    // The hoist moves the write BEFORE the surrounding expression, so the `n` left behind
    // would read the post-call value where iverilog reads the pre-call one (99).
    loud(
        "",
        "    n = 99;\n    r = n + $value$plusargs(\"N=%d\", n);\n    $display(\"VAL=%0d %0d\", r, n);",
    );
}

#[test]
fn an_feof_left_behind_stays_loud() {
    // ⚠️⚠️ `$feof` is PURE in its argument but READS the file position, and the hoist moves
    // the mutation in FRONT of it. The first version of this module argued the opposite in
    // a comment — "they mutate only fd state, which no expression in the statement can
    // read" — and shipped an early exit on it. Measured with the file exhausted:
    // `$feof(fd)*10 + $fgetc(fd)` gave 9 where iverilog gives -1, at exit 0.
    //
    // The cell has to run the reads up to EOF: mid-file, the pre-read and post-read answers
    // are both 0 and a probe there is green on the wrong answer.
    loud(
        "",
        "    repeat (8) r = $fgetc(fd);\n    r = $feof(fd) * 10 + $fgetc(fd);\n         $display(\"VAL=%0d\", r);",
    );
}

#[test]
fn an_feof_is_no_hazard_with_nothing_to_hoist() {
    // The stand-down is about a hoist moving past it — `$feof` on its own is unaffected.
    runs(
        "",
        "    repeat (8) r = $fgetc(fd);\n    $display(\"VAL=%0d\", $feof(fd));",
        "0",
    );
}

#[test]
fn a_monitor_argument_stays_loud() {
    // `$monitor` RE-RENDERS its arguments on every change, so a hoisted call would fire
    // once, early, and every later render would show the frozen temp.
    let (_, code, err) = run(&design(
        "",
        "    $monitor(\"VAL=%0d\", $fgetc(fd));\n    #1 k = 1;",
    ));
    assert_eq!(code, Some(1), "expected a loud reject");
    assert!(err.contains("direct rhs of a blocking assignment"), "{err}");
}

// ── what already worked must keep working ────────────────────────────────────────

#[test]
fn the_direct_blocking_rhs_is_unchanged() {
    runs(
        "",
        "    r = $fgetc(fd);\n    $display(\"VAL=%0d\", r);",
        "65",
    );
}

#[test]
fn a_bare_call_statement_is_unchanged() {
    // `$fgetc(fd);` with its result discarded still advances the fd.
    runs(
        "",
        "    r = $fgetc(fd);\n    r = $fgetc(fd);\n    $display(\"VAL=%0d\", r);",
        "66",
    );
}

#[test]
fn the_bare_plusargs_condition_is_unchanged() {
    runs(
        "",
        "    if ($value$plusargs(\"N=%d\", n)) $display(\"VAL=t\"); else $display(\"VAL=f\");",
        "f",
    );
}
