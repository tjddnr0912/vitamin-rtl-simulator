//! r19 follow-on — an output/inout-formal call in ARBITRARY expression positions.
//!
//! §4.5.274 opened the positions a *statement-shaped* call can occupy. A call that
//! returns a value can go anywhere an expression can, and every other position was
//! loud. These pins cover the positions the general hoister (`elaborate/hoist/general.rs`)
//! added, plus the evaluation-order and conditional-evaluation rules it has to respect.
//!
//! ORACLE. iverilog rejects output formals on FUNCTIONS outright ("Function arguments
//! must be input ports"), so the expected values here were pinned with a COMPOSED oracle,
//! the same one §4.5.274 used: a function whose observable side effect is writing a module
//! net stands in for the copy-out, which lets iverilog fix the evaluation ORDER and the
//! CONDITIONAL evaluation of every position; output-formal mapping itself is pinned by
//! tasks, where iverilog does support output. Every literal below came from that
//! measurement, not from reading the LRM.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Elaborate+run `src` in a per-test temp dir, returning stdout+stderr merged. A unique
/// dir per test keeps the parallel harness from racing on a shared artifact path.
fn run_src(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ofap_{}_{n}", std::process::id()));
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

/// Run `body` inside a block with the standard fixture and return stdout.
fn run(body: &str) -> String {
    let src = format!(
        "module t;\n\
         reg [31:0] nb;\n\
         int arr[8];\n\
         function automatic int nxt (input int i, output int oo);\n\
           oo = i * 10;\n\
           return i + 1;\n\
         endfunction\n\
         task automatic tsk (input int i, output int oo); oo = i * 10; endtask\n\
         initial begin\n\
           begin\n\
             automatic int o = -1;\n\
             automatic int q = -1;\n\
             automatic int a = 1;\n\
             {body}\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n"
    );
    run_src(&src)
}

fn assert_out(body: &str, want: &str) {
    let o = run(body);
    assert!(
        !o.contains("error[VITA"),
        "must not be loud (want {want}):\n{o}"
    );
    assert!(o.contains(want), "want {want}:\n{o}");
}

// ── unconditionally-evaluated expression positions ───────────────────────────

#[test]
fn call_in_ternary_condition() {
    // A `?:` CONDITION is unconditional, unlike its arms.
    assert_out(
        "q = nxt(5, o) == 6 ? 1 : 2; $display(\"q=%0d o=%0d\", q, o);",
        "q=1 o=50",
    );
}

#[test]
fn call_in_display_argument() {
    assert_out(
        "$display(\"ret=%0d\", nxt(5, o)); $display(\"o=%0d\", o);",
        "o=50",
    );
}

#[test]
fn call_in_case_scrutinee() {
    assert_out(
        "case (nxt(5, o)) 6: $display(\"six o=%0d\", o); default: $display(\"other\"); endcase",
        "six o=50",
    );
}

#[test]
fn call_in_concat_and_replicate() {
    assert_out(
        "q = {nxt(5, o)}; $display(\"q=%0d o=%0d\", q, o);",
        "q=6 o=50",
    );
    assert_out(
        "q = {1{nxt(5, o)}}; $display(\"q=%0d o=%0d\", q, o);",
        "q=6 o=50",
    );
}

#[test]
fn call_nested_in_another_calls_argument() {
    // The inner copy-out must be emitted BEFORE the outer one, so the outer call reads the
    // inner's return: `nxt(2,·)` writes o=20 returns 3, then `nxt(3,·)` writes o=30
    // returns 4.
    assert_out(
        "q = nxt(nxt(2, o), o); $display(\"q=%0d o=%0d\", q, o);",
        "q=4 o=30",
    );
}

#[test]
fn call_in_repeat_count() {
    // A `repeat` count is evaluated ONCE — three ticks, one copy-out (o stays 20).
    let o = run("repeat (nxt(2, o)) $display(\"tick o=%0d\", o);");
    assert_eq!(o.matches("tick o=20").count(), 3, "{o}");
}

#[test]
fn call_in_lvalue_and_rvalue_index() {
    assert_out(
        "arr[nxt(0, o)] = 7; $display(\"o=%0d arr1=%0d\", o, arr[1]);",
        "o=0 arr1=7",
    );
    assert_out(
        "arr[1] = 7; q = arr[nxt(0, o)]; $display(\"v=%0d o=%0d\", q, o);",
        "v=7 o=0",
    );
}

#[test]
fn call_in_nonblocking_rhs_and_cast_and_task_arg() {
    assert_out(
        "nb <= nxt(5, o); #1; $display(\"nb=%0d o=%0d\", nb, o);",
        "nb=6 o=50",
    );
    assert_out(
        "q = int'(nxt(5, o)); $display(\"q=%0d o=%0d\", q, o);",
        "q=6 o=50",
    );
    assert_out(
        "begin automatic int o2 = -1; tsk(nxt(5, o) - 1, o2); \
         $display(\"o2=%0d o=%0d\", o2, o); end",
        "o2=50 o=50",
    );
}

// ── evaluation order (measured; the read's SIDE of the call decides) ─────────

#[test]
fn read_to_the_right_sees_the_post_call_value() {
    // Both were declined before as "eval-order unsafe". They are safe: the source
    // evaluates these reads after the call, so does the hoisted form.
    assert_out(
        "if (nxt(5, o) == 6 && o == 50) $display(\"taken\"); else $display(\"not\");",
        "taken",
    );
    assert_out(
        "o = 7; q = nxt(5, o) + o; $display(\"q=%0d o=%0d\", q, o);",
        "q=56 o=50",
    );
}

#[test]
fn read_to_the_left_sees_the_pre_call_value() {
    // The hazard case: hoisting alone would read the POST-call `o` (giving 56). A
    // pre-call snapshot keeps the left operand at 7, so the result is 7 + 6.
    assert_out(
        "o = 7; q = o + nxt(5, o); $display(\"q=%0d o=%0d\", q, o);",
        "q=13 o=50",
    );
}

#[test]
fn read_to_the_left_of_two_mutating_calls_stays_loud() {
    // One snapshot cannot serve the reads between two calls that write the same root —
    // each needs its own generation. Honest-loud rather than a guessed order.
    let o = run("o = 7; q = o + nxt(5, o) + nxt(6, o); $display(\"q=%0d\", q);");
    assert!(o.contains("error[VITA-E3009]"), "must stay loud:\n{o}");
}

// ── conditional evaluation at arbitrary depth ───────────────────────────────

#[test]
fn buried_shortcircuit_skips_the_call() {
    // The `&&` is inside a ternary condition, not the whole rhs — the depth §4.5.216's
    // specialized path never reached.
    assert_out(
        "a = 0; q = (a == 1 && nxt(5, o) == 6) ? 1 : 0; $display(\"q=%0d o=%0d\", q, o);",
        "q=0 o=-1",
    );
    // …and inside a `$display` argument.
    assert_out(
        "a = 0; $display(\"v=%0d\", a == 1 && nxt(5, o) == 6); $display(\"o=%0d\", o);",
        "o=-1",
    );
}

#[test]
fn buried_ternary_arm_skips_the_call() {
    assert_out(
        "a = 0; q = 1 + ((a == 1) ? nxt(5, o) : 77); $display(\"q=%0d o=%0d\", q, o);",
        "q=78 o=-1",
    );
    assert_out(
        "a = 0; if (((a == 1) ? nxt(5, o) : 77) == 77) $display(\"t\"); else $display(\"f\"); \
         $display(\"o=%0d\", o);",
        "o=-1",
    );
}

#[test]
fn buried_ternary_arm_fires_on_the_taken_path() {
    assert_out(
        "q = 1 + ((a == 1) ? nxt(5, o) : 77); $display(\"q=%0d o=%0d\", q, o);",
        "q=7 o=50",
    );
}

#[test]
fn x_condition_evaluates_both_ternary_arms() {
    // IEEE §11.4.11: an x condition evaluates BOTH arms and bit-merges. Both guards are
    // case-inequalities, so both copy-outs fire — here the two arms write different nets,
    // and both writes must land.
    let o = run("begin automatic int o2 = -1; automatic logic sel = 1'bx;\n\
         q = sel ? nxt(5, o) : nxt(7, o2);\n\
         $display(\"o=%0d o2=%0d\", o, o2); end");
    assert!(
        !o.contains("error[VITA"),
        "x-condition ternary must not be loud:\n{o}"
    );
    assert!(o.contains("o=50 o2=70"), "both arms must fire:\n{o}");
}

// ── the report's own shape, now including a read of the record ───────────────

#[test]
fn rsp_walker_reading_the_record_in_the_condition() {
    // The round-19 report's `.rsp` walker idiom, extended with the read of the record that
    // used to make the whole condition loud (`hoist_is_safe` declined any read of a
    // mutated root, including one to the RIGHT of the call).
    let src = "module t;\n\
        typedef struct { int len; string h; } rec_t;\n\
        function automatic int rsp_next (input int i, output rec_t r);\n\
          if (i >= 3) return 0;\n\
          r.len = i * 4; r.h = \"row\";\n\
          return 1;\n\
        endfunction\n\
        initial begin\n\
          begin\n\
            automatic rec_t r;\n\
            automatic int n = 0;\n\
            while (n < 10 && rsp_next(n, r) == 1 && r.len >= 0) begin\n\
              $display(\"n=%0d len=%0d h=%s\", n, r.len, r.h);\n\
              n = n + 1;\n\
            end\n\
            $display(\"DONE n=%0d\", n);\n\
          end\n\
          $finish;\n\
        end\n\
        endmodule\n";
    let o = run_src(src);
    assert!(!o.contains("error[VITA"), "must not be loud:\n{o}");
    for want in ["n=0 len=0", "n=1 len=4", "n=2 len=8", "DONE n=3"] {
        assert!(o.contains(want), "want {want}:\n{o}");
    }
}

// ── adversarial-review pins (the two lenses found these; each was a measured defect) ────
//
// Every expectation below is the iverilog value, measured with the composed oracle (a
// function whose side effect writes a module net) or a hand-hoisted twin.

#[test]
fn conditional_write_does_not_claim_assignment() {
    // The DA walk may say "the read on the right of `&&` is safe" without saying "this
    // expression always writes". Claiming the latter made the block-local gate treat the
    // local as assigned on the SHORT-CIRCUIT path too, so a same-named sibling block's
    // leftover value was read at exit 0. `g` is 0 at run time, so the call never fires and
    // nothing writes `r` — this must stay loud, and must never print the sibling's 777.
    let o = run_src(
        "module top;\n\
         int g;\n\
         function automatic int nxt(input int i, output int r); r = i*10; return i+1; endfunction\n\
         initial begin\n\
           g = 0;\n\
           begin : b1 int r; r = 777; $display(\"b1 r=%0d\", r); end\n\
           begin : b2\n\
             int r;\n\
             if ((g && nxt(5, r) == 6) && r == 50) $display(\"then r=%0d\", r);\n\
             else                                  $display(\"else r=%0d\", r);\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule\n",
    );
    assert!(
        o.contains("error[VITA-E3009]") && !o.contains("r=777\nelse"),
        "a conditional write must not establish assignment:\n{o}"
    );
}

#[test]
fn hierarchical_read_of_the_hazard_root_stays_loud() {
    // `top.o` names the same net as `o`, but the snapshot substitution rewrites
    // single-segment reads only — so the hazard cannot be repaired and the statement must
    // stay loud rather than silently read the POST-call value (measured 56 vs iverilog 13).
    let o = run_src(
        "module top;\n\
         int o, q;\n\
         function automatic int nxt(input int i, output int oo); oo = i*10; return i+1; endfunction\n\
         initial begin o = 7; q = top.o + nxt(5, o); $display(\"q=%0d\", q); $finish; end\n\
         endmodule\n",
    );
    assert!(
        o.contains("error[VITA-E3009]") && !o.contains("q=56"),
        "a hierarchical read of the hazard root must stay loud:\n{o}"
    );
}

#[test]
fn read_inside_a_callee_body_stays_loud_unless_provably_inert() {
    // `rd()` is literally `return o`, so `rd() + nxt(5,o)` must equal `o + nxt(5,o)` — 13.
    // The read is in the callee's BODY, where no substitution can reach, so this is loud.
    let o = run_src(
        "module top;\n\
         int o, q;\n\
         function automatic int rd(); return o; endfunction\n\
         function automatic int nxt(input int i, output int oo); oo = i*10; return i+1; endfunction\n\
         initial begin o = 7; q = rd() + nxt(5, o); $display(\"q=%0d\", q); $finish; end\n\
         endmodule\n",
    );
    assert!(
        o.contains("error[VITA-E3009]") && !o.contains("q=56"),
        "a callee-body read of the hazard root must stay loud:\n{o}"
    );
    // …but a callee provably INERT with respect to the root keeps working: `h` cannot touch
    // `o`, so hoisting past it is safe (this is the boundary the loud above must not eat).
    let o = run_src(
        "module top;\n\
         int o, q;\n\
         function automatic int h(input int k); return k + 1; endfunction\n\
         function automatic int nxt(input int i, output int oo); oo = i*10; return i+1; endfunction\n\
         initial begin o = 7; q = h(5) + nxt(5, o); $display(\"q=%0d o=%0d\", q, o); $finish; end\n\
         endmodule\n",
    );
    assert!(
        !o.contains("error[VITA") && o.contains("q=12 o=50"),
        "an inert callee must not block the hoist:\n{o}"
    );
}

#[test]
fn bits_must_not_evaluate_its_operand() {
    // `$bits` reports a property of the operand's TYPE and does not evaluate it (IEEE
    // §20.5). Hoisting a copy-out out of it fired a side effect the source never performs
    // (measured: `o` became 50 where iverilog leaves 7).
    let o = run_src(
        "module top;\n\
         int o, q;\n\
         function automatic int nxt(input int i, output int oo); oo = i*10; return i+1; endfunction\n\
         initial begin o = 7; q = $bits(nxt(5, o)); $display(\"q=%0d o=%0d\", q, o); $finish; end\n\
         endmodule\n",
    );
    assert!(
        o.contains("error[VITA-E3009]") && !o.contains("o=50"),
        "$bits must not fire the copy-out:\n{o}"
    );
}

#[test]
fn cross_boundary_hazards_are_one_sequence() {
    // The transform hoists ACROSS sub-expression boundaries — an lvalue index's copy-out
    // lands before the rhs is evaluated, and one argument's before the next argument's.
    // Analysing each piece alone made those hazards invisible; all four values below are
    // the iverilog ones.
    // argument list vs argument list
    assert_out("o = 7; $display(\"R %0d %0d\", o+0, nxt(5, o));", "R 7 6");
    // rhs vs lvalue index
    let o = run_src(
        "module top;\n\
         int o, arr[8];\n\
         function automatic int nxt(input int i, output int oo); oo = i*10; return i+1; endfunction\n\
         initial begin o = 3; arr[nxt(0, o)] = o; $display(\"arr1=%0d o=%0d\", arr[1], o); $finish; end\n\
         endmodule\n",
    );
    assert!(o.contains("arr1=3 o=0"), "rhs is evaluated first:\n{o}");
    // index vs index inside ONE lvalue
    let o = run_src(
        "module top;\n\
         int o; int mem[4][4];\n\
         function automatic int nxt(input int i, output int oo); oo = i*10; return i+1; endfunction\n\
         initial begin o = 2; mem[o][nxt(0, o)] = 7;\n\
           $display(\"m21=%0d m01=%0d o=%0d\", mem[2][1], mem[0][1], o); $finish; end\n\
         endmodule\n",
    );
    assert!(
        o.contains("m21=7 m01=0 o=0"),
        "outer index is pre-call:\n{o}"
    );
    // a user task's argument list
    let o = run_src(
        "module top;\n\
         int o;\n\
         function automatic int nxt(input int i, output int oo); oo = i*10; return i+1; endfunction\n\
         task automatic tk(input int a, input int b); $display(\"a=%0d b=%0d\", a, b); endtask\n\
         initial begin o = 7; tk(o, nxt(5, o)); $finish; end\n\
         endmodule\n",
    );
    assert!(o.contains("a=7 b=6"), "task args are one sequence:\n{o}");
}

#[test]
fn captured_operand_still_sees_the_reads_to_its_left() {
    // A `&&` left operand / `?:` condition is captured at the point the source evaluates
    // it, so its OWN reads are not hazards — but a call INSIDE it must still be checked
    // against the reads to ITS left. Starting that check from an empty set silently gave
    // `q=51` where iverilog gives 8.
    assert_out(
        "o = 7; q = o + (nxt(5, o) && 1); $display(\"q=%0d\", q);",
        "q=8",
    );
    assert_out(
        "o = 7; q = o + ((nxt(5, o) > 0) ? 1 : 7); $display(\"q=%0d\", q);",
        "q=8",
    );
}

#[test]
fn deferred_print_arguments_are_not_hoisted() {
    // §4.5.250: `$monitor`/`$strobe` RE-RENDER their arguments later. A hoist freezes them
    // at the statement and fires the copy-out once, early — the monitor then printed the
    // same stale temp on every change. Stays loud.
    let o = run_src(
        "module top;\n\
         int o, k;\n\
         function automatic int nxt(input int i, output int oo); oo = i*10; return i+1; endfunction\n\
         initial begin o = 7; k = 1;\n\
           $monitor(\"MON k=%0d call=%0d\", k, nxt(k, o));\n\
           #1 k = 2; #1 $finish;\n\
         end\n\
         endmodule\n",
    );
    assert!(
        o.contains("error[VITA-E3009]"),
        "a $monitor argument must not be hoisted:\n{o}"
    );
}

#[test]
fn frame_body_keeps_its_own_accurate_diagnostic() {
    // The general hoister must stand down inside a frame function/task body: emitting a
    // copy-out `Terminator::Call` there made the frame classifier report a cause the source
    // does not contain ("uses a timing/suspend/fork control"). The call's own message must
    // survive instead.
    let o = run_src(
        "module top;\n\
         int o;\n\
         function automatic int nxt(input int i, output int oo); oo = i*10; return i+1; endfunction\n\
         function automatic int wrap(input int i); return 100 + nxt(i, o); endfunction\n\
         initial begin int z; z = wrap(5); $display(\"z=%0d\", z); $finish; end\n\
         endmodule\n",
    );
    assert!(
        o.contains("output/inout formal") && !o.contains("timing/suspend/fork"),
        "the frame body must not report a cause the source lacks:\n{o}"
    );
}

#[test]
fn real_return_keeps_its_fraction() {
    // `fresh_ret_temp` built a `Reg` net for every non-string return, so a `real` return
    // was rounded through the integer domain — `return 1.5` came back as 2.0, silently, on
    // the direct-rhs path too (pre-existing). The temp is now an f64 net.
    let o = run_src(
        "module top;\n\
         int o;\n\
         function automatic real rfn(input int i, output int oo); oo = i*10; return 1.5; endfunction\n\
         initial begin real r; o = 0;\n\
           r = rfn(5, o); $display(\"E %f o=%0d\", r, o);\n\
           o = 0; $display(\"D %f\", rfn(5, o));\n\
           $finish;\n\
         end\n\
         endmodule\n",
    );
    assert!(
        o.contains("E 1.500000 o=50") && o.contains("D 1.500000"),
        "a real return must not be rounded:\n{o}"
    );
}

#[test]
fn truth_capture_evaluates_its_operand_once() {
    // The captured truth of a `&&`/`||` left operand / `?:` condition was built as
    // `x || x`, which names the same expr id twice and so evaluates it twice: a `$random`
    // operand drew two values and the global sequence skewed. `!!x` is the same 4-state
    // reduction with ONE evaluation. Pinned by internal equivalence — the value drawn AFTER
    // the captured expression must match the one drawn after the equivalent call-free form,
    // which is only true if both consumed the same number of draws.
    let after_draw = |src: &str| {
        let o = run_src(src);
        assert!(!o.contains("error[VITA"), "must not be loud:\n{o}");
        o.lines()
            .find(|l| l.starts_with("after="))
            .unwrap_or_default()
            .to_owned()
    };
    let with_call = after_draw(
        "module top;\n\
         int o, q, b;\n\
         function automatic int nxt(input int i, output int oo); oo = i*10; return i+1; endfunction\n\
         initial begin\n\
           q = 1 + (($random != 0) && (nxt(5, o) == 6));\n\
           b = $random;\n\
           $display(\"after=%0d q=%0d o=%0d\", b, q, o); $finish;\n\
         end\n\
         endmodule\n",
    );
    let no_call = after_draw(
        "module top;\n\
         int q, b;\n\
         initial begin\n\
           q = 1 + (($random != 0) && 1);\n\
           b = $random;\n\
           $display(\"after=%0d q=%0d o=%0d\", b, q, 50); $finish;\n\
         end\n\
         endmodule\n",
    );
    assert_eq!(
        with_call, no_call,
        "the truth capture must consume exactly one $random draw"
    );
}

// ── re-review pins (the second adversarial round; each was a measured defect) ──────────

#[test]
fn task_call_output_actual_keeps_its_write() {
    // The pre-call snapshot rewrites READS. An output actual of the statement's OWN call is
    // a write DESTINATION, and rewriting it sent the callee's copy-out into the snapshot net
    // while the user's variable kept its stale value — a LOST WRITE at exit 0. iverilog:
    // `b` is the pre-call `nxt` result 6, so `a = 6*10 = 60`.
    let o = run_src(
        "module top;\n\
         int o;\n\
         task automatic tk(output int a, input int b); a = b * 10; endtask\n\
         function automatic int nxt(input int v, output int r); r = 50; nxt = v + 1; endfunction\n\
         initial begin o = 7; tk(o, nxt(5, o)); $display(\"o=%0d\", o); $finish; end\n\
         endmodule\n",
    );
    assert!(
        !o.contains("error[VITA") && o.contains("o=60"),
        "the output actual must receive the copy-out:\n{o}"
    );
    // An INOUT actual is different: its copy-in READS the actual, at the call — after every
    // hoisted copy-out — and it is also the destination, so no snapshot can redirect it.
    // Correct-or-loud rather than iverilog's 13 by luck.
    let o = run_src(
        "module top;\n\
         int o;\n\
         task automatic tk(inout int a, input int b); a = a + b; endtask\n\
         function automatic int nxt(input int v, output int r); r = 50; nxt = v + 1; endfunction\n\
         initial begin o = 7; tk(o, nxt(5, o)); $display(\"o=%0d\", o); $finish; end\n\
         endmodule\n",
    );
    assert!(
        o.contains("error[VITA-E3009]") && !o.contains("o=56"),
        "an inout actual whose root a sibling arg writes must stay loud:\n{o}"
    );
}

#[test]
fn frame_body_never_emits_a_copy_out() {
    // A frame function/task body's writes must target frame-LOCAL nets. Emitting a copy-out
    // there reached the engine and tripped `debug_assert!(frame_local[net])` — a PANIC with
    // no diagnostic (and, with the assert stripped, a write into the wrong net). Both
    // lowering flags matter: a function body sets one, a task body the other.
    for body in [
        "case (nxt(5,gv)) 6: r=111; default: r=222; endcase",
        "r = nxt(5,gv);",
        "r = nxt(5,gv) + 1;",
        "if (nxt(5,gv) == 6) r = 1; else r = 2;",
    ] {
        let o = run_src(&format!(
            "module t;\n\
             int gv;\n\
             function automatic int nxt (input int a, output int o); o = a + 1; nxt = o; endfunction\n\
             task automatic tk(output int r); {body} endtask\n\
             initial begin int z; gv = 50; tk(z); $display(\"z=%0d gv=%0d\", z, gv); $finish; end\n\
             endmodule\n"
        ));
        assert!(
            !o.contains("panicked") && o.contains("error[VITA-E3009]"),
            "a frame task body must report, not panic ({body}):\n{o}"
        );
    }
}

#[test]
fn a_non_hoist_node_is_transparent_to_detection_but_not_to_hoisting() {
    // `$bits` / `min:typ:max` are not hoist sites. Answering "a call may be in here" for
    // them unconditionally made every statement containing one stand down — including these,
    // where the node is just an ARGUMENT and carries no call at all (PRE ran them fine).
    assert_out(
        "o = 1; q = nxt($bits(o), o); $display(\"q=%0d o=%0d\", q, o);",
        "q=33 o=320",
    );
    assert_out(
        "o = 1; q = nxt((1:2:3), o); $display(\"q=%0d o=%0d\", q, o);",
        "q=3 o=20",
    );
    // But a read INSIDE such a node cannot be repaired by a snapshot (the rewrite does not
    // descend into it), so a hazard there stands the statement down instead of silently
    // reading the post-call value.
    let o = run_src(
        "module top;\n\
         int gv, q;\n\
         function automatic int nxt(input int a, output int o); o = a + 1; nxt = o; endfunction\n\
         initial begin gv = 50; q = (1:gv:3) + nxt(5, gv); $display(\"q=%0d\", q); $finish; end\n\
         endmodule\n",
    );
    assert!(
        o.contains("error[VITA-E3009]") && !o.contains("q=12"),
        "a read inside a non-hoist node must stand the statement down:\n{o}"
    );
    // A read to the RIGHT of the call through the same node is still fine.
    let o = run_src(
        "module top;\n\
         int gv, q;\n\
         function automatic int nxt(input int a, output int o); o = a + 1; nxt = o; endfunction\n\
         initial begin gv = 50; q = nxt(5, gv) + (1:gv:3); $display(\"q=%0d gv=%0d\", q, gv); $finish; end\n\
         endmodule\n",
    );
    assert!(
        !o.contains("error[VITA") && o.contains("q=12 gv=6"),
        "a read to the right of the call must still work:\n{o}"
    );
}

#[test]
fn a_child_scope_read_does_not_poison_the_parents_root() {
    // A hierarchical read is only an alias when it is a SELF-path: v1 flattens block-locals
    // to module nets by bare name, so `t.o` and `o` are one net while `sub.o` is another
    // module's. Poisoning on the segment SPELLING false-louded this correct design.
    let o = run_src(
        "module s_m; int gv; endmodule\n\
         module t;\n\
         int gv; s_m sub();\n\
         function automatic int nxt (input int a, output int o); o = a + 1; nxt = o; endfunction\n\
         initial begin int q; gv = 50; sub.gv = 7;\n\
           q = sub.gv + nxt(5, gv); $display(\"q=%0d gv=%0d\", q, gv); $finish; end\n\
         endmodule\n",
    );
    assert!(
        !o.contains("error[VITA") && o.contains("q=13 gv=6"),
        "an unrelated child-scope read must not poison the root:\n{o}"
    );
}

#[test]
fn a_named_output_actual_is_visible_to_the_order_analysis() {
    // Formals were zipped with actuals by POSITION, so a `.formal(o)` output actual wrote a
    // root the eval-order analysis never knew about — it silently read the post-call value,
    // and the two-calls-one-root guard could not see the call either.
    assert_out(
        "o = 7; q = o + nxt(.i(5), .oo(o)); $display(\"q=%0d o=%0d\", q, o);",
        "q=13 o=50",
    );
}

#[test]
fn a_call_reached_through_a_concat_is_seen_by_the_callee_body_check() {
    // The candidate roots were collected with a walker that only descends
    // Unary/Binary/Paren/Ternary, so a call reached through a CONCAT left the set empty and
    // the callee-body opacity check never ran — `rd()`, which is `return o`, then silently
    // read the post-call value.
    let o = run_src(
        "module top;\n\
         int o; logic [63:0] y;\n\
         function automatic int rd(); rd = o; endfunction\n\
         function automatic int nxt(input int v, output int r); r = 50; nxt = v + 1; endfunction\n\
         initial begin o = 7; y = {rd(), nxt(5, o)};\n\
           $display(\"hi=%0d\", y[63:32]); $finish; end\n\
         endmodule\n",
    );
    assert!(
        o.contains("error[VITA-E3009]") && !o.contains("hi=50"),
        "a callee-body read reached through a concat must stand down:\n{o}"
    );
}

#[test]
fn a_typedefd_real_return_keeps_its_fraction() {
    // The PARSER maps a typedef'd return type onto the return fields but never mapped
    // `real`/`realtime` onto `ParamType::Real`/`Realtime` — the only place a return's
    // realness is recorded. So `typedef real myreal; function myreal f(…)` returned an
    // integer: rounded on the inline path and 0.0 through a frame call's return temp.
    let o = run_src(
        "module top;\n\
         typedef real myreal; typedef realtime myrt;\n\
         int o3;\n\
         function automatic myreal ftd(input int v, output int r); r = v - 1; return 3.75; endfunction\n\
         function automatic myrt   rt (input int v, output int r); r = v;     return 2.25; endfunction\n\
         initial begin real d1, d2; o3 = 0;\n\
           d1 = ftd(5, o3); $display(\"d1=%f o3=%0d\", d1, o3);\n\
           d2 = rt(7, o3);  $display(\"d2=%f o3=%0d\", d2, o3);\n\
           $finish;\n\
         end\n\
         endmodule\n",
    );
    assert!(
        o.contains("d1=3.750000 o3=4") && o.contains("d2=2.250000 o3=7"),
        "a typedef'd real return must not be rounded:\n{o}"
    );
}
