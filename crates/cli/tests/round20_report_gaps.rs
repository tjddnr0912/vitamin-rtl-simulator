//! Round-20 external report — three items, plus the boundary probes that pin each fix's
//! soundness. correct-or-loud throughout.
//!
//!   §3.1 DYNFORMAL-CROSSTALK  a REGRESSION introduced by §4.5.275: declaring any
//!        value-returning output-formal function, even one that is never called, stopped
//!        every dyn-array-formal hoist inside a frame body. The stand-down was a
//!        function-wide early return gated on the MODULE-GLOBAL `inout_func_names`, so it
//!        disabled the unrelated dyn-formal arms too. Now per-arm.
//!   §3.2 INOUT-COPYIN         an `inout` formal's copy-in is a read of the actual, so a
//!        block-local the callee is about to FILL had to be written before the call. It is
//!        a read whose value is DEAD when the callee overwrites the whole formal before
//!        looking at it — then the flatten leftover is unobservable and the call is a
//!        definite write.
//!   §3.3 LOOP-WRITE           a write inside a loop body was never counted, because a
//!        loop may run zero times. When the trip count proves at least one iteration and
//!        nothing can jump past the write, it counts.
//!
//! Oracles: iverilog 13.0 says `sorry:` for both `automatic` block-local lifetime and
//! unpacked structs, so these are hand-IEEE, pinned against vita's own `static` twin
//! (the same design with the lifetime dropped — measured byte-identical). §3.3's static
//! twin additionally matches iverilog. Every "must stay loud" case below was measured to
//! be genuinely unsound, not merely unproven.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_src(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r20_{}_{n}", std::process::id()));
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

/// The reporter's design, parameterised on the TRIGGER declaration and on WHERE the
/// dyn-array-formal call sits. `b2h` is called from a task body (a frame body), which is
/// what made the crosstalk reachable.
fn crosstalk_src(trigger: &str, call_site: &str) -> String {
    format!(
        "module t;\n\
         {trigger}\n\
         function automatic int fl (input string s); return s.len(); endfunction\n\
         function automatic string b2h (input byte b []);\n\
           string s = \"\";\n\
           foreach (b[i]) s = $sformatf(\"%s%02x\", s, b[i]);\n\
           return s;\n\
         endfunction\n\
         string h; bit en;\n\
         task automatic chk ();\n\
           byte d [];\n\
           d = new[1]; d[0] = 8'h61; en = 1;\n\
           {call_site}\n\
           $display(\"PASS\");\n\
         endtask\n\
         initial begin chk(); #1 $finish; end\n\
         endmodule\n"
    )
}

const VALUE_FN: &str =
    "function automatic int f (input int a, output int m); m = a; return 1; endfunction";

// ─────────────────── §3.1 DYNFORMAL-CROSSTALK (the regression) ───────────────────

#[test]
fn an_unrelated_output_formal_declaration_does_not_disable_dyn_formal_calls() {
    // The reporter's exact case: `f` is never called and shares nothing with `b2h`, yet
    // deleting its DECLARATION was the difference between error and PASS. A module-global
    // predicate has no business deciding a per-statement hoist.
    let o = run_src(&crosstalk_src(
        VALUE_FN,
        "if (b2h(d) != \"61\") $display(\"BAD\");",
    ));
    assert!(
        !o.contains("error[VITA") && o.contains("PASS") && !o.contains("BAD"),
        "an uncalled output-formal function must not affect a dyn-formal call:\n{o}"
    );
}

#[test]
fn the_trigger_declaration_changes_no_dyn_formal_position() {
    // The regression was position-dependent (only the bare `h = b2h(d);` survived), so the
    // pin is that the trigger axis is IRRELEVANT: every position must reach the same verdict
    // with and without it. Both the supported and the still-loud positions are included —
    // if a future change makes the trigger matter again, either column catches it.
    for site in [
        "h = b2h(d);",                                             // blocking-assign rhs
        "if (b2h(d) != \"61\") $display(\"BAD\");",                // comparison
        "$display(\"H=%s\", b2h(d));",                             // system-task argument
        "h = {b2h(d), \"z\"};",                                    // concat
        "h = en ? b2h(d) : \"\";",                                 // `?:` arm
        "if (en && (b2h(d) == \"61\")) ; else $display(\"BAD\");", // `&&` rhs — loud
        "if (fl(b2h(d)) != 2) $display(\"BAD\");",                 // call argument — loud
    ] {
        let with = run_src(&crosstalk_src(VALUE_FN, site));
        let without = run_src(&crosstalk_src("", site));
        assert_eq!(
            with.contains("error[VITA"),
            without.contains("error[VITA"),
            "the trigger declaration changed the verdict for `{site}`:\nWITH:\n{with}\nWITHOUT:\n{without}"
        );
        assert!(!with.contains("BAD"), "wrong value for `{site}`:\n{with}");
    }
}

#[test]
fn every_trigger_spelling_is_equally_irrelevant() {
    // The reporter measured nine boundary variants and found the split ran along "is it a
    // value-returning function with a non-input formal" — i.e. exactly `inout_func_names`.
    // All of them must now behave identically.
    let base = run_src(&crosstalk_src(
        "",
        "if (b2h(d) != \"61\") $display(\"BAD\");",
    ));
    assert!(!base.contains("error[VITA"), "baseline must pass:\n{base}");
    for trigger in [
        VALUE_FN,
        "function automatic void f (input int a, output int m); m = a; endfunction",
        "function automatic int f (input int a); return a; endfunction",
        "task automatic f (input int a, output int m); m = a; endtask",
        "function automatic int f (input int a, inout int m); m = a; return 1; endfunction",
        "function int f (input int a, output int m); m = a; return 1; endfunction",
    ] {
        let o = run_src(&crosstalk_src(
            trigger,
            "if (b2h(d) != \"61\") $display(\"BAD\");",
        ));
        assert!(
            !o.contains("error[VITA") && o.contains("PASS") && !o.contains("BAD"),
            "trigger `{trigger}` must be irrelevant:\n{o}"
        );
    }
}

#[test]
fn a_frame_body_still_refuses_to_emit_a_copy_out() {
    // The stand-down the regression came from is still needed for the arms it was written
    // for: a copy-out inside a frame body writes a module net from a frame-local context
    // (it panicked in the engine before §4.5.275). Narrowing its scope must not re-open it.
    for body in [
        "r = nxt(5,gv);",
        "r = nxt(5,gv) + 1;",
        "if (nxt(5,gv) == 6) r = 1; else r = 2;",
        "case (nxt(5,gv)) 6: r=111; default: r=222; endcase",
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

// ───────────────────────── §3.2 INOUT-COPYIN (dead copy-in) ─────────────────────────

/// The reporter's table-driven walker: `nxt` fills `r` and returns whether to continue.
/// `body` is `nxt`'s body, which is the whole of what decides soundness.
fn walker_src(body: &str) -> String {
    format!(
        "module t;\n\
         task automatic helper (inout int q, input int v); q = v; endtask\n\
         function automatic int nxt (input int fd, inout int r);\n\
           {body}\n\
         endfunction\n\
         initial begin\n\
           begin\n\
             automatic int r;\n\
             automatic int n = 0;\n\
             while (n < 5 && nxt(n, r) == 1) begin\n\
               if (r != n) $display(\"BAD r=%0d n=%0d\", r, n);\n\
               n++;\n\
             end\n\
             $display(\"PASS n=%0d\", n);\n\
           end\n\
         end\n\
         endmodule\n"
    )
}

#[test]
fn an_inout_formal_the_callee_overwrites_is_a_definite_write() {
    let o = run_src(&walker_src("r = fd; return (fd<2);"));
    assert!(
        !o.contains("error[VITA") && o.contains("PASS n=2") && !o.contains("BAD"),
        "a dead inout copy-in must not block the flatten:\n{o}"
    );
}

#[test]
fn an_unpacked_struct_inout_formal_works_member_by_member() {
    // The report's own shape. The SoA fan-out makes each member a separate formal, so the
    // proof is per-member — and the reporter's boundary (pre-writing ONE member left
    // exactly the other loud) is what says the granularity was already right.
    let o = run_src(
        "module t;\n\
         typedef struct { int count; string h; } rec_t;\n\
         function automatic int nxt (input int fd, inout rec_t r);\n\
           r.count = fd; r.h = \"x\";\n\
           return (fd < 2);\n\
         endfunction\n\
         initial begin\n\
           begin\n\
             automatic rec_t r;\n\
             automatic int   n = 0;\n\
             while (n < 5 && nxt(n, r) == 1) begin\n\
               if (r.count != n) $display(\"BAD\");\n\
               if (r.h != \"x\") $display(\"BAD\");\n\
               n++;\n\
             end\n\
             $display(\"PASS n=%0d\", n);\n\
           end\n\
         end\n\
         endmodule\n",
    );
    assert!(
        !o.contains("error[VITA") && o.contains("PASS n=2") && !o.contains("BAD"),
        "an unpacked-struct inout formal must work per member:\n{o}"
    );
}

#[test]
fn a_copy_in_the_callee_can_observe_stays_loud() {
    // Each of these leaves the copy-in value LIVE, so the flatten's leftover would be
    // visible where a fresh `automatic` would give the type default. Measured unsound, not
    // merely unproven — every one either reads the formal or leaves a path that never
    // writes it, and the copy-out then hands the leftover back to the caller.
    for (body, why) in [
        (
            "if (fd==0) r = fd; return (fd<2);",
            "written only on one path",
        ),
        ("r = r + fd; return (fd<2);", "reads the formal first"),
        (
            "if (fd==9) return 0; r = fd; return (fd<2);",
            "a return before the write",
        ),
        (
            "if (r==0) fd=fd; r = fd; return (fd<2);",
            "reads the formal before writing",
        ),
        ("helper(r, fd); return (fd<2);", "delegates the fill"),
        ("return (fd<2);", "never writes the formal"),
    ] {
        let o = run_src(&walker_src(body));
        assert!(
            o.contains("error[VITA-E3009]"),
            "must stay loud ({why}): `{body}`:\n{o}"
        );
    }
}

#[test]
fn a_write_after_the_first_write_does_not_un_prove_it() {
    // Once the whole formal is written, a later read of it is a read of the callee's own
    // value — the copy-in is already dead and nothing can revive it.
    let o = run_src(&walker_src("r = fd; if (r==0) fd=fd; return (fd<2);"));
    assert!(
        !o.contains("error[VITA") && o.contains("PASS n=2") && !o.contains("BAD"),
        "a read AFTER the write must not block the proof:\n{o}"
    );
}

// ───────────────────────────── §3.3 LOOP-WRITE ─────────────────────────────

fn loop_src(loop_stmt: &str) -> String {
    format!(
        "module t;\n\
         localparam int NN = 3;\n\
         int vn;\n\
         task automatic fill (output byte o []);\n\
           o = new[2]; o[0] = 8'h61; o[1] = 8'h62;\n\
         endtask\n\
         initial begin\n\
           vn = 3;\n\
           begin\n\
             automatic byte cur [];\n\
             {loop_stmt}\n\
             $display(\"PASS size=%0d\", cur.size());\n\
           end\n\
         end\n\
         endmodule\n"
    )
}

#[test]
fn a_loop_that_provably_iterates_carries_its_bodys_write_out() {
    for loop_stmt in [
        "for (int j = 0; j < 3; j++) fill(cur);",
        "repeat (2) fill(cur);",
    ] {
        let o = run_src(&loop_src(loop_stmt));
        assert!(
            !o.contains("error[VITA") && o.contains("PASS size=2"),
            "`{loop_stmt}` must count as a write:\n{o}"
        );
    }
}

#[test]
fn the_trip_count_proof_admits_only_literals() {
    // Both review lenses broke the first version of this proof, in three independent ways,
    // all of them a 64-bit signed AST fold disagreeing with what the engine executes. The
    // proof now admits ONLY unsized decimal literals; each case below is a measured 0-trip
    // loop (vita and iverilog agree on the count) that the fold previously called ">= 1".
    for (loop_stmt, why) in [
        // A signed `byte` holds 200 as -56, so `-56 > 100` is false.
        (
            "for (byte j = 200; j > 100; j--) fill(cur);",
            "the init truncates in the loop variable's type",
        ),
        // IEEE 1800 §11.8.1: one unsigned operand makes the whole comparison unsigned.
        (
            "for (int j = -1; j < 4'd3; j++) fill(cur);",
            "an unsigned operand makes the comparison unsigned",
        ),
        ("while ((-1) < 4'd3) fill(cur);", "same, in a `while`"),
        ("repeat ((-1) < 4'd3) fill(cur);", "same, in a `repeat`"),
        // Self-determined 8-bit truncation: iverilog runs this 0 times.
        (
            "repeat (8'd128 + 8'd128) fill(cur);",
            "a sized literal carries its own width",
        ),
    ] {
        let o = run_src(&loop_src(loop_stmt));
        assert!(
            o.contains("error[VITA-E3009]"),
            "must stay loud ({why}): `{loop_stmt}`:\n{o}"
        );
    }
    // A `localparam` bound is a KNOWN precision loss, not an oversight: folding an
    // identifier needs `lookup_scoped`, which is params-only and not net-aware, so a net
    // shadowing the parameter would diverge from the lowering — and the fold cannot be made
    // shadow-aware, because `walk_scopes_key_shadowed`'s contract forbids opting in anything
    // reachable from `const_eval_in_scope` (an order-dependent answer there once deleted a
    // whole generate body at exit 0). Pinned as loud so the day it becomes supported, this
    // test says so out loud.
    let o = run_src(&loop_src("for (int j = 0; j < NN; j++) fill(cur);"));
    assert!(
        o.contains("error[VITA-E3009]"),
        "a localparam bound is honest-loud for now:\n{o}"
    );
}

#[test]
fn a_net_shadowing_a_parameter_does_not_fool_the_trip_count() {
    // The differential lens's case: a generate-scope net `K` shadows a module `localparam
    // K = 3`. The lowering reads the net (0), so the loop runs zero times; the fold read the
    // parameter and claimed the body ran, and activation 2 then read activation 1's leftover.
    let o = run_src(
        "module t;\n\
         localparam int K = 3;\n\
         task automatic fill (output byte o []); o = new[2]; o[0]=8'h61; o[1]=8'h62; endtask\n\
         generate if (1) begin : g\n\
           int K;\n\
           initial begin\n\
             K = 0;\n\
             for (int a = 0; a < 2; a++) begin\n\
               automatic byte cur [];\n\
               for (int j = 0; j < K; j++) fill(cur);\n\
               $display(\"size=%0d\", cur.size());\n\
               cur = new[7];\n\
             end\n\
             #1 $finish;\n\
           end\n\
         end endgenerate\n\
         endmodule\n",
    );
    assert!(
        o.contains("error[VITA-E3009]") && !o.contains("size=7"),
        "a net shadowing the parameter must not prove the trip count:\n{o}"
    );
}

#[test]
fn the_callee_body_is_not_folded_in_the_callers_parameter_scope() {
    // The inout proof walks the CALLEE's body, so it must not carry the caller's trip-count
    // folder in: a formal or local named like a module `localparam` — `LIM`, `N`, `WIDTH`,
    // ordinary code with no shadowing trick — took the parameter's value, and a 0-trip loop
    // was "proven" to write the formal. The caller then read its own planted leftover 999.
    for callee in [
        "function automatic int nxt (input int LIM, inout int p);\n\
           for (int i = 0; i < LIM; i++) p = 7;\n\
           return 1;\n\
         endfunction\n",
        "function automatic int nxt (input int fd, inout int p);\n\
           int LIM = 0;\n\
           for (int i = 0; i < LIM; i++) p = 7;\n\
           return 1;\n\
         endfunction\n",
    ] {
        let o = run_src(&format!(
            "module t;\n\
             localparam int LIM = 3;\n\
             {callee}\
             initial begin\n\
               for (int a = 0; a < 2; a++) begin\n\
                 automatic int q;\n\
                 void'(nxt(0, q));\n\
                 $display(\"a=%0d q=%0d\", a, q);\n\
                 q = 999;\n\
               end\n\
               #1 $finish;\n\
             end\n\
             endmodule\n"
        ));
        assert!(
            o.contains("error[VITA-E3009]") && !o.contains("q=999"),
            "the callee's loops must not fold in the caller's scope:\n{o}"
        );
    }
}

#[test]
fn a_declaration_initializer_observes_the_copy_in() {
    // Found independently by BOTH lenses. `inout_copy_in_is_dead` unwrapped the callee body's
    // `Block` and iterated its `stmts`, dropping the `decls` — so a declarator initializer
    // reading the formal was never checked, while `da_stmt`'s own `Block` arm checks exactly
    // that for NESTED blocks. Two spellings of one hazard disagreed: `int save = r;` was
    // accepted and `save = r;` refused.
    for body in [
        "int save = r; r = fd; return (fd<3);",
        "int a = 1, save = r; r = fd; return (fd<3);",
        "int save = r + 1; r = fd; return (fd<3);",
    ] {
        let o = run_src(&walker_src(&format!("begin {body} end")));
        assert!(
            o.contains("error[VITA-E3009]"),
            "a decl initializer reading the formal must stay loud: `{body}`:\n{o}"
        );
    }
    // A nested block that REDECLARES the formal: its write is a write of something else.
    let o = run_src(&walker_src(
        "begin begin int r; r = fd; end return (fd<3); end",
    ));
    assert!(
        o.contains("error[VITA-E3009]"),
        "a shadowing redeclaration must not count as writing the formal:\n{o}"
    );
}

#[test]
fn a_tf_body_top_level_declaration_initializer_is_checked() {
    // Round-2 review. A tf-body's top-level declarations do NOT live in the body `Block` —
    // `tf_body` collects them into `FunctionDef/TaskDef::body_decls` and builds the wrapper
    // with `decls: Vec::new()`. So a check that read only the Block's `decls` inspected an
    // ALWAYS-EMPTY list: `int save = r;` at tf-body top level was accepted and handed back the
    // caller's planted leftover, while the same read one block deeper and the same read as a
    // statement were both refused. Three spellings, two right and one wrong.
    for body in [
        // top level of the tf body (the canonical spelling — this is the one that was wrong)
        "int save = r;\n r = fd + save;\n return 1;",
        // one block deeper (was already refused, by `da_stmt`'s own `Block` arm)
        "begin int save = r; r = fd + save; end\n return 1;",
        // as a statement (was already refused)
        "int save;\n save = r;\n r = fd + save;\n return 1;",
    ] {
        let o = run_src(&format!(
            "module t;\n\
             function automatic int nxt (input int fd, inout int r);\n\
               {body}\n\
             endfunction\n\
             initial begin\n\
               for (int a = 0; a < 2; a++) begin\n\
                 automatic int q;\n\
                 void'(nxt(10, q));\n\
                 $display(\"a=%0d q=%0d\", a, q);\n\
                 q = 999;\n\
               end\n\
               #1 $finish;\n\
             end\n\
             endmodule\n"
        ));
        assert!(
            o.contains("error[VITA-E3009]") && !o.contains("q=1009"),
            "a declaration initializer reading the formal must stay loud:\n{body}\n{o}"
        );
    }
}

#[test]
fn an_omitted_formals_default_can_read_the_mutated_root() {
    // Round-2 review. The `order_walk` opacity check asks whether a call it does NOT lift can
    // read a root a hoisted call writes. An OMITTED formal binds its DEFAULT, which is lowered
    // in the CALLER's scope and so can name that root while no written-out argument does —
    // and the default lives on the callee's port, outside this expression, so the pre-call
    // snapshot cannot reach it. Loud is the honest answer.
    let src = |call: &str| {
        format!(
            "module t;\n\
             int o, q;\n\
             function automatic int nxt (input int v, output int oo); oo = 50; return v + 1; endfunction\n\
             function automatic int rd (input int x = o); return x; endfunction\n\
             initial begin o = 7; q = {call} + nxt(5, o); $display(\"q=%0d\", q); #1 $finish; end\n\
             endmodule\n"
        )
    };
    let omitted = run_src(&src("rd()"));
    assert!(
        omitted.contains("error[VITA-E3009]"),
        "an omitted default reading the mutated root must stay loud:\n{omitted}"
    );
    // Written out, the read IS in this expression, so the snapshot repairs it. `13` = the
    // PRE-call `o` (7) plus the return (6) — confirmed against an iverilog task twin, which
    // also gives 13. (The review reported 56 here; that is the POST-call value and iverilog
    // contradicts it — the mechanism was real, the stated oracle was not.)
    let explicit = run_src(&src("rd(o)"));
    assert!(
        explicit.contains("q=13"),
        "a written-out left read must see the pre-call value:\n{explicit}"
    );
}

#[test]
fn the_trip_count_fold_bounds_the_result_not_just_the_leaves() {
    // Round-2 review. Clamping each literal to `0..=i32::MAX` and then folding in i64 admits
    // expressions that LEAVE the 32-bit domain the engine executes in. Measured engine truth:
    // `2147483647+1 = -2147483648`, `65536*65536 = 0`, `2**32 = 0`, `1<<32 = 0` — so every
    // loop below runs ZERO times, and the block then read the previous activation's leftover.
    for loop_stmt in [
        "repeat (65536 * 65536) fill(cur);",
        "repeat (2 ** 32) fill(cur);",
        "repeat (1 << 32) fill(cur);",
        "repeat (2147483647 + 1) fill(cur);",
        "for (int j = 0; j < 65536 * 65536; j++) fill(cur);",
        "while (65536 * 65536 > 0) fill(cur);",
    ] {
        let o = run_src(&format!(
            "module t;\n\
             task automatic fill (output byte o []); o = new[2]; o[0]=8'h61; o[1]=8'h62; endtask\n\
             initial begin\n\
               for (int a = 0; a < 2; a++) begin\n\
                 automatic byte cur [];\n\
                 {loop_stmt}\n\
                 $display(\"a=%0d size=%0d\", a, cur.size());\n\
                 cur = new[7];\n\
               end\n\
               #1 $finish;\n\
             end\n\
             endmodule\n"
        ));
        assert!(
            o.contains("error[VITA-E3009]") && !o.contains("size=7"),
            "an out-of-domain trip count must stay loud: `{loop_stmt}`:\n{o}"
        );
    }
}

#[test]
fn a_dyn_formal_hoist_stands_down_in_a_frame_function_body() {
    // The hoist emits `__t = f(arr)` into a MODULE net, which a frame FUNCTION body may not
    // write — so the frame-body validator reported "assignment to a net outside the function"
    // about a temp the user never wrote. Every such position is loud either way; the point is
    // WHICH message. A frame TASK body is deliberately not gated (that is §3.1's case).
    for body in ["s = {b2h(d), \"!\"};", "if (b2h(d) == \"61\") s = \"y\";"] {
        let o = run_src(&format!(
            "module t;\n\
             function automatic string b2h (input byte b []);\n\
               string s = \"\";\n\
               foreach (b[i]) s = $sformatf(\"%s%02x\", s, b[i]);\n\
               return s;\n\
             endfunction\n\
             function automatic string wrap (input byte d []);\n\
               string s;\n\
               {body}\n\
               return s;\n\
             endfunction\n\
             string h;\n\
             initial begin byte d []; d = new[1]; d[0] = 8'h61;\n\
               h = wrap(d); $display(\"H=%s\", h); #1 $finish; end\n\
             endmodule\n"
        ));
        assert!(
            o.contains("dynamic-array formal") && !o.contains("net outside the function"),
            "the dyn-formal message must survive in a frame function body ({body}):\n{o}"
        );
    }
}

#[test]
fn a_pure_container_method_left_of_the_call_still_works() {
    // Round-2 review caught a LOUD REGRESSION here: the opacity check's replacement answered
    // `false` for every 2-segment `Call` (a method on its head), because there is no
    // single-segment body to walk — so `q = qq.size() + nxt(5,o)` went from working to loud.
    // The predicate it replaced accepted ANY 2-segment call, which was unsound for a CLASS
    // method (its body can reach a module net through a hierarchical path). Purity splits them.
    let src = |stmt: &str, extra: &str| {
        format!(
            "module t;\n\
             int o, q; int qq[$]; string ss;\n\
             {extra}\n\
             function automatic int nxt (input int a, output int oo); oo = a + 1; nxt = oo; endfunction\n\
             initial begin\n\
               o = 5; qq.push_back(1); qq.push_back(2); ss = \"abcde\";\n\
               {stmt}\n\
               $display(\"q=%0d\", q); #1 $finish;\n\
             end\n\
             endmodule\n"
        )
    };
    // Built-in container/string queries have no user body at all: the only storage they can
    // read is the receiver (checked) and their arguments (walked as expression children).
    for (stmt, want) in [
        ("q = qq.size() + nxt(5,o);", "q=8"),
        ("q = ss.len() + nxt(5,o);", "q=11"),
        ("q = nxt(5,o) + qq.size();", "q=8"),
    ] {
        let o = run_src(&src(stmt, ""));
        assert!(
            !o.contains("error[VITA") && o.contains(want),
            "`{stmt}` must still work ({want}):\n{o}"
        );
    }
    // A USER subroutine reaching a module net through `t.o` was a SILENT-WRONG at 8cf4165
    // (`q=12` where 11 is correct). It must be loud, not recovered along with the built-ins —
    // and crucially that must hold whatever the subroutine is NAMED. Round 3 measured that
    // `container_method_is_pure` alone selects for a method NAME, not for a built-in, so 30 of
    // its 34 whitelisted names admitted a user body in three receiver forms. The receiver is
    // now identified positively: a class handle is an integral net, and a module instance or
    // the enclosing module's own name is not a net at all.
    for name in ["get", "size", "len"] {
        let cls = run_src(&src(
            &format!("c = new(); q = c.{name}() + nxt(5,o);"),
            &format!("class C; function int {name}(); return t.o; endfunction endclass\n C c;"),
        ));
        assert!(
            cls.contains("error[VITA-E3009]") && !cls.contains("q=12"),
            "a class method named `{name}` must stay loud:\n{cls}"
        );
        let selfpath = run_src(&src(
            &format!("q = t.{name}() + nxt(5,o);"),
            &format!("function int {name}(); return t.o; endfunction"),
        ));
        assert!(
            selfpath.contains("error[VITA-E3009]") && !selfpath.contains("q=12"),
            "a module-scope function named `{name}` must stay loud:\n{selfpath}"
        );
    }
}

#[test]
fn a_read_smuggled_through_a_callee_body_observes_the_copy_in() {
    // Round 3, and a loud→silent regression from THIS gate. The proof's obligation "nothing
    // before the write can read the formal" was enforced by syntactic walkers only:
    // `expr_no_ref_deep`'s `Call` arm never enters the callee, and `da_stmt` steps over any
    // statement `stmt_no_ref` says does not MENTION the formal. So one level of indirection hid
    // the read — while the DIRECT spelling was refused all along.
    //
    // The oracle is a per-ACTIVATION one: two `always @(posedge clk)` activations, not a loop
    // (§4.5.266 measured that automatic storage is per activation, so a loop re-entry would not
    // distinguish them). A fresh `automatic int r` gives 0 on activation 2; the flatten gave the
    // planted 999.
    let src = |body: &str| {
        format!(
            "module t;\n\
             reg clk;\n\
             function automatic int rd(); return r; endfunction\n\
             function automatic int rd2(); return rd(); endfunction\n\
             function automatic int obs (input int fd, inout int r);\n\
               {body}\n\
             endfunction\n\
             always @(posedge clk) begin\n\
               automatic int r; automatic int seen;\n\
               seen = obs(1, r);\n\
               $display(\"seen=%0d\", seen);\n\
               r = 999;\n\
             end\n\
             initial begin clk=0; #1 clk=1; #1 clk=0; #1 clk=1; #1 clk=0; #1 $finish; end\n\
             endmodule\n"
        )
    };
    for (body, why) in [
        (
            "int save = rd(); r = fd; return save;",
            "decl-init, one level down",
        ),
        (
            "int save; save = rd(); r = fd; return save;",
            "statement, one level down",
        ),
        (
            "int save = rd2(); r = fd; return save;",
            "decl-init, two levels down",
        ),
        (
            "int save = r; r = fd; return save;",
            "direct (was already loud)",
        ),
    ] {
        let o = run_src(&src(body));
        assert!(
            o.contains("error[VITA-E3009]") && !o.contains("seen=999"),
            "must stay loud ({why}):\n{body}\n{o}"
        );
    }
    // A callee that never reads the formal is still proven, so the fix is not a blanket refusal.
    let clean = run_src(&src("r = fd; return 7;"));
    assert!(
        !clean.contains("error[VITA") && clean.contains("seen=7"),
        "a callee that does not read the formal must still be proven:\n{clean}"
    );
}

#[test]
fn the_frame_body_stand_down_does_not_touch_the_conditional_rhs_transform() {
    // Round 3 caught a LOUD REGRESSION: the frame-body stand-down was put in
    // `lower_loop_cond_operand`, which despite its name also lowers the operands of §4.5.216's
    // `?:` / short-circuit-rhs transform — and those work correctly in a frame TASK body,
    // including NOT firing the copy-out on the untaken path. Guarding the shared helper made all
    // four loud. The stand-down belongs at the two genuine loop-condition sites.
    let src = |cval: &str, body: &str| {
        format!(
            "module t;\n\
             int gv, qq;\n\
             function automatic int nxt (input int a, output int oo); oo = a + 1; nxt = oo; endfunction\n\
             task automatic tk(output int r);\n\
               bit c; c = {cval};\n\
               {body}\n\
               r = 1;\n\
             endtask\n\
             initial begin int z; gv = 50; tk(z); $display(\"qq=%0d gv=%0d\", qq, gv); #1 $finish; end\n\
             endmodule\n"
        )
    };
    for (cval, body, want) in [
        // condition true: the arm runs, so the copy-out fires and `gv` becomes 6
        ("1", "qq = c ? nxt(5,gv) : 77;", "qq=6 gv=6"),
        ("1", "qq = c && (nxt(5,gv)==6);", "qq=1 gv=6"),
        // condition false: the arm is skipped, so the copy-out must NOT fire — `gv` stays 50
        ("0", "qq = c ? nxt(5,gv) : 77;", "qq=77 gv=50"),
        ("0", "qq = c && (nxt(5,gv)==6);", "qq=0 gv=50"),
    ] {
        let o = run_src(&src(cval, body));
        assert!(
            !o.contains("error[VITA") && o.contains(want),
            "`{body}` with c={cval} must give `{want}`:\n{o}"
        );
    }
}

#[test]
fn an_output_formal_call_in_a_frame_task_body_keeps_working() {
    // A frame-body stand-down was tried here TWICE and reverted both times. It looked justified
    // — a copy-out in a frame body CAN trip the engine's frame-local assertion (rc=101) — but
    // `in_frame_body()` is far wider than the panic: measured, only a call in the FIRST statement
    // of a frame body panics, and a single preceding statement flips PRE from panic to correct.
    // Guarding on it made 10 of 12 measured shapes loud that PRE answered CORRECTLY, including
    // the round-20 report's own `.rsp` walker moved into a `task automatic`. The pre-existing
    // panic is recorded in ROADMAP §2 with its measured shape; these are the shapes that must
    // NOT be traded away to chase it.
    let src = |body: &str| {
        format!(
            "module t;\n\
             int gv;\n\
             function automatic int nxt (input int a, output int oo); oo = a + 1; nxt = oo; endfunction\n\
             task automatic walk();\n\
               begin\n\
                 automatic int r;\n\
                 {body}\n\
                 $display(\"PASS r=%0d gv=%0d\", r, gv);\n\
               end\n\
             endtask\n\
             initial begin walk(); #1 $finish; end\n\
             endmodule\n"
        )
    };
    for (body, want) in [
        ("nxt(5, gv); r = 0;", "PASS r=0 gv=6"),
        ("void'(nxt(5, gv)); r = 0;", "PASS r=0 gv=6"),
        ("nxt(5, r);", "PASS r=6 gv=0"),
    ] {
        let o = run_src(&src(body));
        assert!(
            !o.contains("error[VITA") && o.contains(want),
            "`{body}` in a frame task body must give `{want}`:\n{o}"
        );
    }
    // And the report's own walker, moved verbatim into a task.
    let walker = run_src(
        "module t;\n\
         function automatic int nxt (input int fd, inout int r); r = fd; return (fd < 2); endfunction\n\
         task automatic walk();\n\
           begin\n\
             automatic int r; automatic int n; n = 0;\n\
             while (n < 5 && nxt(n, r) == 1) begin\n\
               if (r != n) $display(\"BAD\");\n\
               n++;\n\
             end\n\
             $display(\"PASS n=%0d\", n);\n\
           end\n\
         endtask\n\
         initial begin walk(); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(
        !walker.contains("error[VITA") && walker.contains("PASS n=2") && !walker.contains("BAD"),
        "the §3.2 walker must work inside a `task automatic` too:\n{walker}"
    );
}

#[test]
fn every_container_receiver_class_resolves_the_way_the_lowering_does() {
    // Round 5. The receiver test must use the SAME resolver the lowering uses. A first version
    // asked `lookup_net_scoped(&recv.name)` and lost the routed fixed `string` array: it is
    // registered under a MANGLED net name (`<name>$sad`, deliberately, so the bare name stays
    // free in the module namespace), so the declared name resolved to nothing and
    // `string rv[3]; rv.size()` went from working to loud — 48 measured shapes. `dyn_handle` is
    // the lowering's own resolver; a scalar `string` is not a dyn handle, so it is resolved
    // plainly alongside.
    let src = |decl: &str, init: &str, call: &str| {
        format!(
            "module top;\n\
             function automatic int nxt (input int n, output int o); o = 99; return 100; endfunction\n\
             {decl}\n\
             int o, q;\n\
             initial begin\n\
               {init}\n\
               o = 1;\n\
               q = {call} + nxt(5, o);\n\
               $display(\"q=%0d o=%0d\", q, o);\n\
               #1 $finish;\n\
             end\n\
             endmodule\n"
        )
    };
    for (decl, init, call, want) in [
        // the routed fixed string array — the class the first version lost
        (
            "string rv [3];",
            "rv[0]=\"a\"; rv[1]=\"bb\"; rv[2]=\"ccc\";",
            "rv.size()",
            "q=103",
        ),
        (
            "string rq [$];",
            "rq.push_back(\"a\"); rq.push_back(\"b\");",
            "rq.size()",
            "q=102",
        ),
        ("string rd [];", "rd = new[2];", "rd.size()", "q=102"),
        (
            "int iq [$];",
            "iq.push_back(1); iq.push_back(2);",
            "iq.size()",
            "q=102",
        ),
        ("string s1;", "s1 = \"abcde\";", "s1.len()", "q=105"),
    ] {
        let o = run_src(&src(decl, init, call));
        assert!(
            !o.contains("error[VITA") && o.contains(want),
            "`{call}` on `{decl}` must work ({want}):\n{o}"
        );
    }
}

#[test]
fn a_package_scoped_call_left_of_the_output_formal_call_still_works() {
    // Round 6. Tightening the 2-segment arm to "container receivers only" also caught every
    // `pk::h()`. That was needless: `inline_pkg_function` admits ONLY a self-contained,
    // straight-line package function — a body that reads a module net, or has control flow, is
    // already loud there (measured, PRE and POST) — which IS this predicate's obligation,
    // discharged upstream.
    let o = run_src(
        "package pk; function automatic int h (input int a); return a*2; endfunction endpackage\n\
         module t;\n\
           int gv, q;\n\
           function automatic int nxt (input int a, output int oo); oo = a + 1; nxt = oo; endfunction\n\
           initial begin gv = 50; q = pk::h(3) + nxt(5, gv); $display(\"q=%0d gv=%0d\", q, gv); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(
        !o.contains("error[VITA") && o.contains("q=12 gv=6"),
        "a self-contained package call must still work:\n{o}"
    );
    // The upstream guard is what makes that sound — a package body that reads a module net is
    // loud before this predicate ever sees it.
    let reads = run_src(
        "package pk; function automatic int h (input int a); return a*2 + t.gv; endfunction endpackage\n\
         module t;\n\
           int gv, q;\n\
           function automatic int nxt (input int a, output int oo); oo = a + 1; nxt = oo; endfunction\n\
           initial begin gv = 50; q = pk::h(3) + nxt(5, gv); $display(\"q=%0d gv=%0d\", q, gv); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(
        reads.contains("error[VITA-E3009]") && reads.contains("self-contained"),
        "a package body reading a module net must stay loud:\n{reads}"
    );
}

#[test]
fn the_callee_body_walk_uses_the_all_segments_rule() {
    // Round 6. `da_stmt` resolves names with `expr_no_ref`, whose head-segment-only rule is the
    // CALLER-side one; the sibling checks in this proof use the all-segments rule. So the same
    // hazard split by spelling once more — `save = t.r;` was accepted while `int save = t.r;`
    // and `save = r;` were refused, and the accepted one read the copy-in the proof had called
    // dead. (`t.r` naming a block-local is itself a vita laxity iverilog rejects, so this was a
    // wrongly-ACCEPTED design rather than a demonstrated silent-wrong on legal SV — but the
    // gate's stated obligation has to hold whatever reaches it.)
    let src = |body: &str| {
        format!(
            "module t;\n\
             int n;\n\
             task automatic nxt(input int nn, inout int r);\n\
               {body}\n\
             endtask\n\
             initial begin int r; r = 999; end\n\
             initial begin\n\
               int r;\n\
               n = 0;\n\
               nxt(n, r);\n\
               $display(\"B=%0d\", r);\n\
             end\n\
             initial #2 $finish;\n\
             endmodule\n"
        )
    };
    for body in [
        "int save; save = t.r; r = nn + 100 + (save == 999);",
        "int save = t.r; r = nn + 100 + (save == 999);",
        "int save; save = r; r = nn + 100 + (save == 999);",
    ] {
        let o = run_src(&src(body));
        assert!(
            o.contains("error[VITA-E3009]") && !o.contains("B=101"),
            "every spelling of the read must be refused:\n{body}\n{o}"
        );
    }
    // A callee that does not read the formal is still proven.
    let clean = run_src(&src("r = nn + 100;"));
    assert!(
        !clean.contains("error[VITA") && clean.contains("B=100"),
        "a clean callee must still be proven:\n{clean}"
    );
}

#[test]
fn the_dyn_formal_message_matches_the_measured_boundary() {
    // Round 3 found two claims in the rewritten message that measurement contradicts. Both are
    // pinned here so the wording cannot drift back.
    let src = |stmt: &str| {
        format!(
            "module t;\n\
             function automatic logic [63:0] f (input byte b []);\n\
               f = 0; foreach (b[i]) f += b[i];\n\
             endfunction\n\
             byte g[]; logic [63:0] r;\n\
             initial begin g = new[2]; g[0]=1; g[1]=2; {stmt} $display(\"r=%0d\", r); #1 $finish; end\n\
             endmodule\n"
        )
    };
    // A plain concat PART works; the call as a select's BASE does not. The message used to say
    // "a select or lvalue INDEX", which names the wrong half of the shape.
    // `{f(g), 32'd0}` is `3 << 32`, not 3 — the sum lands in the concat's high half. Both the
    // verdict and the value are asserted so a future change cannot quietly alter either.
    for (stmt, want) in [
        ("r = {f(g)};", "r=3"),
        ("r = {32'd0, f(g)};", "r=3"),
        ("r = {f(g), 32'd0};", "r=12884901888"),
    ] {
        let o = run_src(&src(stmt));
        assert!(
            !o.contains("error[VITA") && o.contains(want),
            "a concat part must work ({want}): `{stmt}`:\n{o}"
        );
    }
    let base = run_src(&src("r = {32'd0, f(g)[31:0]};"));
    assert!(
        base.contains("error[VITA-E3009]") && base.contains("the BASE of a select"),
        "a select BASE must be loud and the message must name it:\n{base}"
    );
    // A RECURSIVE call is loud only when it is not the whole right-hand side — the direct-rhs
    // spelling elaborates. The message used to call recursion unconditionally unsupported.
    let rec = |stmt: &str| {
        format!(
            "module t;\n\
             function automatic int fr (input byte c []);\n\
               int s;\n\
               if (c.size() <= 1) return c.size();\n\
               {stmt}\n\
               return s;\n\
             endfunction\n\
             byte g[];\n\
             initial begin int r; g = new[3]; r = fr(g); $display(\"rr=%0d\", r); #1 $finish; end\n\
             endmodule\n"
        )
    };
    assert!(
        !run_src(&rec("s = fr(c);")).contains("error[VITA"),
        "a direct-rhs recursive call elaborates"
    );
    assert!(
        run_src(&rec("s = s + fr(c);")).contains("error[VITA-E3009]"),
        "an arithmetic-nested recursive call is loud"
    );
}

#[test]
fn a_ternary_arm_needs_a_side_effect_free_dyn_formal_callee() {
    // Round-2 review: the message listed `?:` as supported unconditionally, but the real
    // discriminator is the CALLEE, not the position — a conditionally evaluated call cannot be
    // hoisted without performing its effect on the arm that was not taken. Same position, same
    // caller, two callees, two verdicts.
    let src = |callee: &str| {
        format!(
            "module t;\n\
             logic [63:0] r; bit en;\n\
             function automatic logic [63:0] f (input byte b[]);\n\
               f = 0; foreach (b[i]) f += b[i];{callee}\n\
             endfunction\n\
             task automatic run (input byte b[]); en = 1; r = en ? f(b) : 64'd0; endtask\n\
             initial begin byte v[]; v = new[2]; v[0]=1; v[1]=2; run(v);\n\
               $display(\"r=%0d\", r); #1 $finish; end\n\
             endmodule\n"
        )
    };
    let pure = run_src(&src(""));
    assert!(
        !pure.contains("error[VITA") && pure.contains("r=3"),
        "a pure callee must work in a `?:` arm:\n{pure}"
    );
    let impure = run_src(&src(" $display(\"called\");"));
    assert!(
        impure.contains("error[VITA-E3009]") && impure.contains("side-effect free"),
        "an impure callee must be loud AND the message must say why:\n{impure}"
    );
}

#[test]
fn two_calls_to_one_dyn_formal_function_in_one_expression_work() {
    // The message used to list this as a cause. Measured supported and CORRECT — each call is
    // hoisted to its own temp before the expression, so the single slot is reused in
    // sequence, not shared. iverilog agrees on both values.
    let o = run_src(
        "module t;\n\
         function automatic string b2h (input byte b []);\n\
           string s = \"\";\n\
           foreach (b[i]) s = $sformatf(\"%s%02x\", s, b[i]);\n\
           return s;\n\
         endfunction\n\
         string h1, h2; byte d[], e[];\n\
         task automatic body ();\n\
           d = new[1]; d[0] = 8'h61; e = new[1]; e[0] = 8'h62;\n\
           h1 = {b2h(d), b2h(e)};\n\
           h2 = {b2h(d), b2h(d)};\n\
           $display(\"h1=%s h2=%s\", h1, h2);\n\
         endtask\n\
         initial begin body(); #1 $finish; end\n\
         endmodule\n",
    );
    assert!(
        !o.contains("error[VITA") && o.contains("h1=6162 h2=6161"),
        "two calls in one expression must work:\n{o}"
    );
}

#[test]
fn a_shared_flattened_net_still_refuses_a_suspending_loop_body() {
    // Fix C claims a loop body's write; the shared-net rule must still win over it. Two
    // same-named `automatic` locals get separate `$blk$` nets (§4.5.249) and are independent
    // — that case is a false-loud C legitimately removed. Mixing `automatic` with a plain
    // static local of the same name really does share one net, and must stay loud.
    let src = |b_decl: &str| {
        format!(
            "module t;\n\
             reg clk = 0;\n\
             always #1 clk = ~clk;\n\
             task automatic fill (output int o); o = 7; endtask\n\
             initial begin : A\n\
               automatic int v;\n\
               for (int j = 0; j < 3; j++) begin fill(v); @(posedge clk); end\n\
               $display(\"A v=%0d\", v);\n\
             end\n\
             initial begin : B\n\
               {b_decl}\n\
               #1 v = 99; $display(\"B v=%0d\", v);\n\
             end\n\
             initial #20 $finish;\n\
             endmodule\n"
        )
    };
    let sep = run_src(&src("automatic int v;"));
    assert!(
        !sep.contains("error[VITA") && sep.contains("A v=7") && sep.contains("B v=99"),
        "two `automatic` locals are independent:\n{sep}"
    );
    let shared = run_src(&src("int v;"));
    assert!(
        shared.contains("error[VITA-E3009]") && shared.contains("shares one flattened net"),
        "a genuinely shared net must stay loud:\n{shared}"
    );
}

#[test]
fn a_loop_whose_body_may_be_skipped_stays_loud() {
    // The trip count and the escape check are separate obligations and BOTH are load-bearing:
    // the first three cases can run zero times, the last three run but can jump past the
    // write. A `break` is the subtle one — it lands after the loop, on a path the walk's own
    // join has already dropped, so only a syntactic check can see it.
    for (loop_stmt, why) in [
        ("for (int j = 0; j < 0; j++) fill(cur);", "zero trips"),
        ("repeat (0) fill(cur);", "zero trips"),
        (
            "for (int j = 0; j < vn; j++) fill(cur);",
            "a variable bound",
        ),
        (
            "for (int j = 0; j < 3; j++) begin if (j==0) break; fill(cur); end",
            "a break before the write",
        ),
        (
            "for (int j = 0; j < 3; j++) begin if (j<9) continue; fill(cur); end",
            "a continue before the write",
        ),
        (
            "for (int j = 0; j < 3; j++) if (j > 9) fill(cur);",
            "a conditional write",
        ),
    ] {
        let o = run_src(&loop_src(loop_stmt));
        assert!(
            o.contains("error[VITA-E3009]"),
            "must stay loud ({why}): `{loop_stmt}`:\n{o}"
        );
    }
}

#[test]
fn the_automatic_and_static_spellings_agree() {
    // The strongest available pin: iverilog cannot oracle an `automatic` block-local at all
    // (`sorry:`), so the differential is against vita's own STATIC twin, where no gate is
    // involved. If the flatten were not equivalent, these would differ.
    let auto = run_src(&loop_src("for (int j = 0; j < 3; j++) fill(cur);"));
    let stat = run_src(
        &loop_src("for (int j = 0; j < 3; j++) fill(cur);")
            .replace("automatic byte cur [];", "byte cur [];"),
    );
    let line = |o: &str| {
        o.lines()
            .find(|l| l.starts_with("PASS size="))
            .unwrap_or_default()
            .to_owned()
    };
    assert_eq!(
        line(&auto),
        line(&stat),
        "automatic and static must agree:\nAUTO:\n{auto}\nSTATIC:\n{stat}"
    );
    assert_eq!(line(&auto), "PASS size=2");
}

// ───────────────────────────── §4 diagnostic quality ─────────────────────────────

#[test]
fn the_dyn_formal_message_names_only_real_causes() {
    // The reporter read three listed causes and found none of them present. One was simply
    // WRONG: a `?:` arm has been supported since r18, and the message still called it out.
    // A message that names a supported position as the cause sends the reader to rewrite
    // working code.
    let ternary = run_src(&crosstalk_src(VALUE_FN, "h = en ? b2h(d) : \"\";"));
    assert!(
        !ternary.contains("error[VITA"),
        "a `?:` arm must be supported:\n{ternary}"
    );
    // A genuinely unsupported position must name ITSELF in the message.
    let loud = run_src(&crosstalk_src(
        "",
        "if (en && (b2h(d) == \"61\")) ; else $display(\"BAD\");",
    ));
    assert!(
        loud.contains("error[VITA-E3009]") && loud.contains("`&&`/`||`"),
        "the message must name the position that is actually unsupported:\n{loud}"
    );
    assert!(
        !loud.contains("a conditionally evaluated operand (a `?:` arm"),
        "the message must not claim a `?:` arm is unsupported:\n{loud}"
    );
}
