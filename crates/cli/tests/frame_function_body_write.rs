//! A frame FUNCTION whose body assigns a MODULE net runs on the statement executor.
//!
//! `classify_frame_body` refused every blocking write whose net fell outside the
//! function's own frame window, so `function automatic int fw(input int v); acc2 = v + 2;
//! return v; endfunction` was E3009 *"body uses an assignment to a net outside the
//! function"* — and a part-select of one (`accp[3:0] = v[3:0]`) was E3009 *"a part-select /
//! array-element assignment"* — where both oracles run the design. The TASK twin was
//! already supported: `ir::compute_suspendable_tasks` reads an out-of-window lhs chunk as a
//! suspend signal and routes the body to the `&mut` `run_process` executor, which writes a
//! module net like any process statement. That classifier already covers FUNCTIONS (R22);
//! what a function lacked was the CALL SHAPE — only a `Terminator::Call` is visible to the
//! router, and a function gets one only through `inout_func_names`.
//!
//! Such a function now joins that set (`func_body_writes_outside_name`, asked on the AST in
//! `lower_frame_funcs` so the answer exists before any body is lowered), its calls are
//! hoisted to a copy-out call, and `classify_frame_body` is told to accept the write. Where
//! the hoist declines there is no statement to carry the write, so `emit_frame_call`
//! refuses by name instead of emitting an `Expr::Call` the synchronous executor would run
//! with the write dropped.
//!
//! ORACLES: iverilog 13.0 (`-g2012`) and verilator 5.052 (`--binary --timing`); every value
//! below was measured in both unless the cell says otherwise. PRE values are from a release
//! binary built at the parent commit.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn vita(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fnbw_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("t.sv"), src).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (all, out.status.success())
}

/// The design's own printed lines: no diagnostic, no run banner. Fails loudly (with the
/// whole text) when the design did not run, so a refusal can never read as empty output.
fn run(src: &str) -> String {
    let (all, ok) = vita(src);
    assert!(ok, "must run:\n{all}");
    all.lines()
        .filter(|l| {
            // A routed design carries `W4030 W-RUN-BACKEND-FALLBACK` (the native backend
            // declines a subroutine that writes outside its own frame), which wraps onto
            // an indented continuation line — hence the leading-space and empty filters.
            !l.trim().is_empty()
                && !l.contains("simulation ended")
                && !l.starts_with("warning[")
                && !l.starts_with("note[")
                && !l.starts_with("errors=")
                && !l.starts_with(' ')
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The whole diagnostic text of a design that must NOT run.
fn loud(src: &str) -> String {
    let (all, ok) = vita(src);
    assert!(!ok, "must be refused:\n{all}");
    all
}

// ── the headline ──────────────────────────────────────────────────────────────────

/// ① `acc2 = v + 2;` in a frame function body, called from `always_comb`, in BOTH
/// lifetimes. PRE: E3009 "body uses an assignment to a net outside the function" for each.
#[test]
fn a_frame_function_body_writes_a_module_net() {
    const SRC: &str = r#"module t;
  int acc2 = 0; int src = 7; int r;
  function automatic int fw(input int v); acc2 = v + 2; return v; endfunction
  always_comb r = fw(src);
  initial begin #1 $display("ACC2=%0d R=%0d", acc2, r); src = 9; #1 $display("ACC2=%0d R=%0d", acc2, r); $finish; end
endmodule
"#;
    // iverilog and verilator both: `ACC2=9 R=7` then `ACC2=11 R=9`.
    assert_eq!(run(SRC), "ACC2=9 R=7\nACC2=11 R=9");
    // The STATIC twin. `int` returns make it framed too (`ret_two_state`), so one body has
    // one answer in both lifetimes — the asymmetry `inline_fn_writes_outside.rs` opened.
    assert_eq!(
        run(&SRC.replace("function automatic int fw", "function int fw")),
        "ACC2=9 R=7\nACC2=11 R=9"
    );
}

/// ② The call sites that CAN carry the write, in one design: two `always_comb` readers
/// (one of them reading the OTHER function's written net, so the write has to post a
/// change), left-to-right evaluation inside one expression, both operands of a `&&` that
/// does not short-circuit, a `&&` that DOES, and a PART-SELECT write to a module net.
///
/// PRE: three E3009s — `fps` "a part-select / array-element assignment", `fs` and `fw`
/// "an assignment to a net outside the function".
#[test]
fn every_once_evaluated_call_site_carries_the_write() {
    let o = run(r#"module t;
  int acc2 = 0; int src = 7; int r1, r2, r5, r6; int accn; int accp;
  function automatic int fw(input int v); acc2 = v + 2; return v; endfunction
  function int fs(input int v); accn = v + 3; return v + 1; endfunction
  function automatic int fps(input int v); accp[3:0] = v[3:0]; return v; endfunction
  always_comb r1 = fw(src);
  always_comb r2 = fs(src) + 1;
  initial begin
    #1 $display("A r=%0d %0d acc2=%0d accn=%0d", r1, r2, acc2, accn);
    src = 9;
    #1 $display("B r=%0d %0d acc2=%0d accn=%0d", r1, r2, acc2, accn);
    r5 = fw(1) + fw(2); $display("C r5=%0d acc2=%0d", r5, acc2);
    if (fw(3) > 0 && fw(4) > 0) $display("D acc2=%0d", acc2);
    if (fw(0) > 0 && fw(5) > 0) $display("D2"); else $display("D2 acc2=%0d", acc2);
    r6 = fps(5); $display("E accp=%h", accp);
    $finish;
  end
endmodule
"#);
    // Both oracles, line for line. `C`: left-to-right, so the LAST write wins (`fw(2)` ⇒
    // 4) while the value is `1 + 2`. `D`: `fw(3)` is non-zero so `fw(4)` is evaluated too
    // (2 + 4 ⇒ 6). `D2`: `fw(0) > 0` is FALSE, so `fw(5)` is never called and `acc2` keeps
    // the 2 that `fw(0)` left — the short-circuit is preserved because the hoist declines a
    // `&&` right operand and `lower_shortcircuit_cond` gives it a block of its own.
    assert_eq!(
        o,
        "A r=7 9 acc2=9 accn=10\n\
         B r=9 11 acc2=11 accn=12\n\
         C r5=3 acc2=4\n\
         D acc2=6\n\
         D2 acc2=2\n\
         E accp=00000005"
    );
}

/// ③ A block that READS the written net re-runs when the write lands: `always_comb r3 =
/// accn + 100;` beside `always_comb r2 = fs(src) + 1;`. A routed write that did not post
/// the net's change would leave `r3` at its t0 value.
///
/// PRE: E3009 "an assignment to a net outside the function".
#[test]
fn a_reading_block_wakes_on_the_routed_write() {
    let o = run(r#"module t;
  int accn = 0; int src = 7; int r2, r3;
  function int fs(input int v); accn = v + 3; return v + 1; endfunction
  always_comb r2 = fs(src) + 1;
  always_comb r3 = accn + 100;
  initial begin #1 $display("WAKE r2=%0d r3=%0d accn=%0d", r2, r3, accn); src = 9; #1 $display("WAKE r2=%0d r3=%0d accn=%0d", r2, r3, accn); $finish; end
endmodule
"#);
    // Both oracles.
    assert_eq!(o, "WAKE r2=9 r3=110 accn=10\nWAKE r2=11 r3=112 accn=12");
}

/// ④ The buried expression positions the general hoist already owned: a `$display`
/// argument, a `?:` arm, a `case` scrutinee, a `while` condition. Each was E3009 at PRE.
#[test]
fn the_buried_expression_positions_carry_it_too() {
    const HEAD: &str = "module t;\n  int acc2 = 0; int src = 7; int r; int c = 1; int i = 0; int n = 0;\n  \
         function automatic int fw(input int v); acc2 = v + 2; return v; endfunction\n  initial begin ";
    // Both oracles: `DISP 7` / `DISP acc2=9`.
    assert_eq!(
        run(&format!(
            "{HEAD}$display(\"DISP %0d\", fw(src)); $display(\"DISP acc2=%0d\", acc2); $finish; end\nendmodule\n"
        )),
        "DISP 7\nDISP acc2=9"
    );
    // Both oracles: `TERN r=1 acc2=3` (the taken arm's call fires, `fw(1)` ⇒ 3).
    assert_eq!(
        run(&format!(
            "{HEAD}r = c ? fw(1) : 2; $display(\"TERN r=%0d acc2=%0d\", r, acc2); $finish; end\nendmodule\n"
        )),
        "TERN r=1 acc2=3"
    );
    // Both oracles: `CASE r=1 acc2=9`.
    assert_eq!(
        run(&format!(
            "{HEAD}case (fw(src)) 7: r = 1; default: r = 0; endcase $display(\"CASE r=%0d acc2=%0d\", r, acc2); $finish; end\nendmodule\n"
        )),
        "CASE r=1 acc2=9"
    );
    // Both oracles: `WHILE n=3 acc2=5`. The condition is re-evaluated per iteration, so the
    // write fires four times and `acc2` ends at `fw(3)` ⇒ 5.
    assert_eq!(
        run(&format!(
            "{HEAD}while (fw(i) < 3) begin i = i + 1; n = n + 1; end $display(\"WHILE n=%0d acc2=%0d\", n, acc2); $finish; end\nendmodule\n"
        )),
        "WHILE n=3 acc2=5"
    );
}

/// ⑤ The SEQUENTIAL call sites. `always_ff @(posedge clk) r8 <= fw(src) + 1;` agrees with
/// both oracles. `always @*` (not `always_comb`) does not fire at t0 in iverilog and does
/// in verilator — a §9.2.2.2.2 split that predates this slice; vita lands on iverilog, and
/// the two agree from the first real change onward.
///
/// PRE: E3009 for both.
#[test]
fn the_sequential_and_bare_star_call_sites() {
    // iverilog and verilator both: `FF r8=10 acc2=11`.
    assert_eq!(
        run(r#"module t;
  int acc2 = 0; int src = 7; int r8; logic clk = 0;
  function automatic int fw(input int v); acc2 = v + 2; return v; endfunction
  always_ff @(posedge clk) r8 <= fw(src) + 1;
  initial begin #1 src = 9; #1 clk = 1; #1 $display("FF r8=%0d acc2=%0d", r8, acc2); $finish; end
endmodule
"#),
        "FF r8=10 acc2=11"
    );
    // iverilog: `STAR r7=0 acc2=0` then `STAR r7=109 acc2=11`.
    // verilator: `STAR r7=107 acc2=9` then `STAR r7=109 acc2=11` (it fires `always @*` at
    // t0). vita reproduces iverilog; the second line is the two-oracle anchor for the axis
    // this slice is about.
    assert_eq!(
        run(r#"module t;
  int acc2 = 0; int src = 7; int r7;
  function automatic int fw(input int v); acc2 = v + 2; return v; endfunction
  always @* r7 = fw(src) + 100;
  initial begin #1 $display("STAR r7=%0d acc2=%0d", r7, acc2); src = 9; #1 $display("STAR r7=%0d acc2=%0d", r7, acc2); $finish; end
endmodule
"#),
        "STAR r7=0 acc2=0\nSTAR r7=109 acc2=11"
    );
}

/// ⑥ A function with BOTH an `output` formal and a body write. ONE ORACLE: iverilog
/// refuses an output formal on a function at all (*"Function t.fo port oo is not an input
/// port."*), so the value is verilator's. PRE: E3009 on the body write.
#[test]
fn an_output_formal_and_a_body_write_together() {
    // verilator 5.052: `OUT r=7 o=14 acc2=9`.
    assert_eq!(
        run(r#"module t;
  int acc2 = 0; int src = 7; int r, o;
  function automatic int fo(input int v, output int oo); acc2 = v + 2; oo = v * 2; return v; endfunction
  initial begin r = fo(src, o); $display("OUT r=%0d o=%0d acc2=%0d", r, o, acc2); $finish; end
endmodule
"#),
        "OUT r=7 o=14 acc2=9"
    );
}

// ── what stays loud, and why ──────────────────────────────────────────────────────

/// ⑦ The positions with NO statement to carry the write. Each RUNS in both oracles, so
/// each is honest-loud, not correct — and each must name the construct rather than the
/// frame-subset sentence, which lists `acc2 = v + 2;` among the supported forms.
///
/// PRE: all three were E3009 "body uses an assignment to a net outside the function"
/// (attributed to the function, not to the call site).
#[test]
fn a_position_that_cannot_hold_a_statement_is_loud_by_name() {
    // A continuous assign. Both oracles: `CA w=7 acc2=9` / `CA w=9 acc2=11`.
    let e = loud(
        r#"module t;
  int acc2 = 0; int src = 7; wire [31:0] w;
  function automatic int fw(input int v); acc2 = v + 2; return v; endfunction
  assign w = fw(src);
  initial begin #1 $display("CA w=%0d acc2=%0d", w, acc2); $finish; end
endmodule
"#,
    );
    assert!(e.contains("assigns a module net from its body"), "{e}");
    assert!(e.contains("CONTINUOUSLY re-evaluated"), "{e}");

    // A `force` right-hand side. Both oracles: `FORCE wf=7 acc2=9`.
    let e = loud(
        r#"module t;
  int acc2 = 0; int src = 7; wire [31:0] wf;
  function automatic int fw(input int v); acc2 = v + 2; return v; endfunction
  assign wf = 0;
  initial begin force wf = fw(src); #1 $display("FORCE wf=%0d acc2=%0d", wf, acc2); $finish; end
endmodule
"#,
    );
    assert!(e.contains("assigns a module net from its body"), "{e}");

    // Inside ANOTHER frame function's body: a function is entered from the expression that
    // calls it, so it has no call statement of its own. Both oracles: `F2 r=8 acc2=9`.
    let e = loud(
        r#"module t;
  int acc2 = 0; int src = 7; int r;
  function automatic int fw(input int v); acc2 = v + 2; return v; endfunction
  function automatic int f2(input int v); return fw(v) + 1; endfunction
  always_comb r = f2(src);
  initial begin #1 $display("F2 r=%0d acc2=%0d", r, acc2); $finish; end
endmodule
"#,
    );
    assert!(e.contains("assigns a module net from its body"), "{e}");
    assert!(e.contains("inside another FUNCTION body"), "{e}");
}

/// ⑧ The refusal has to be reachable however the lowering is ORDERED. `f2` sorts before
/// `fw`, so its body is lowered FIRST: a route decided from `fw`'s own lowered blocks does
/// not exist yet, the call goes out as a plain `Expr::Call`, and the engine reaches `frame
/// write targets a frame-local net` — a panic at rc 101 with no diagnostic. Measured, on
/// this exact design, with the AST predicate replaced by an IR one.
#[test]
fn the_callee_is_known_before_a_caller_that_sorts_first_is_lowered() {
    // Both oracles: `F2B r8=10 acc2=11`. vita is loud, but it is LOUD — not a panic.
    let e = loud(
        r#"module t;
  int acc2 = 0; int src = 7; int r8; logic clk = 0;
  function automatic int fw(input int v); acc2 = v + 2; return v; endfunction
  function automatic int f2(input int v); return fw(v) + 1; endfunction
  always_ff @(posedge clk) r8 <= f2(src);
  initial begin #1 src = 9; #1 clk = 1; #1 $display("F2B r8=%0d acc2=%0d", r8, acc2); $finish; end
endmodule
"#,
    );
    assert!(e.contains("assigns a module net from its body"), "{e}");
    assert!(!e.contains("panicked"), "not a panic:\n{e}");
}

/// ⑨ What BOTH oracles reject stays rejected. A task enable in a function body inlines to
/// an out-of-window write, so the route has to decline it explicitly — otherwise the one
/// construct neither oracle will compile would start running.
#[test]
fn what_both_oracles_reject_is_not_routed() {
    // iverilog: "Functions cannot enable/call tasks." · verilator: `%Error-FUNCTIMECTL`.
    let e = loud(
        r#"module t;
  int acc2 = 0; int src = 7; int r;
  task tw(input int v); acc2 = v + 2; endtask
  function automatic int fct(input int v); tw(v); return v; endfunction
  initial begin r = fct(src); $display("FCT r=%0d acc2=%0d", r, acc2); $finish; end
endmodule
"#,
    );
    assert!(e.contains("outside the frame-call subset"), "{e}");
    // The task's body need not write anything: the enable alone is what both oracles
    // refuse, and the decline is keyed on the enable.
    let e = loud(
        r#"module t;
  int src = 7; int r;
  task tw(input int v); $display("TW %0d", v); endtask
  function automatic int fct2(input int v); tw(v); return v; endfunction
  initial begin r = fct2(src); $display("FCT2 r=%0d", r); $finish; end
endmodule
"#,
    );
    assert!(!e.is_empty() && e.contains("E3009"), "{e}");

    // A nonblocking assign in a function body: iverilog refuses ("functions cannot have
    // non blocking assignment statements"), verilator runs it. The NBA arm still owns the
    // refusal — the routed write no longer sets the reason, so the message names the `<=`
    // the source actually contains.
    let e = loud(
        r#"module t;
  int acc2 = 0; int src = 7; int r;
  function automatic int fnba(input int v); acc2 <= v + 2; return v; endfunction
  initial begin r = fnba(src); #1 $display("FNBA r=%0d acc2=%0d", r, acc2); $finish; end
endmodule
"#,
    );
    assert!(e.contains("a nonblocking assignment (`<=`)"), "{e}");
}

// ── multidriver: the same table as the TASK twin ──────────────────────────────────

/// ⑩ Rule A stands down for ONE `always_comb` that reaches a variable through a function
/// body — the body write is not a second DRIVER. Measured: both oracles print `acc=8` and
/// verilator reports NO MULTIDRIVEN for this shape.
#[test]
fn one_always_comb_through_a_body_write_is_not_a_multidriver() {
    // Both oracles: `MDB1 acc=8 r1=7`.
    assert_eq!(
        run(r#"module t;
  int acc = 0; int src = 7; int r1;
  function automatic int fw(input int v); acc = v + 1; return v; endfunction
  always_comb r1 = fw(src);
  initial begin #1 $display("MDB1 acc=%0d r1=%0d", acc, r1); $finish; end
endmodule
"#),
        "MDB1 acc=8 r1=7"
    );
    // An `initial` writer beside it is not the flagged pair either: both oracles agree
    // (`MDBI acc=8` / `MDBI2 acc=99`) and only the `always_comb` × `always_comb` pair is
    // MULTIDRIVEN. Verilator does warn here, at the `initial`; the oracles agreeing is what
    // decides the severity, and this is the task twin's answer too.
    assert_eq!(
        run(r#"module t;
  int acc = 0; int src = 7; int r1;
  function automatic int fw(input int v); acc = v + 1; return v; endfunction
  always_comb r1 = fw(src);
  initial begin #1 $display("MDBI acc=%0d r1=%0d", acc, r1); acc = 99; $display("MDBI2 acc=%0d", acc); $finish; end
endmodule
"#),
        "MDBI acc=8 r1=7\nMDBI2 acc=99"
    );
}

/// ⑪ TWO `always_comb` blocks each reaching one variable through a function body IS the
/// flagged pair, and the oracles split on the value: iverilog `acc=8`, verilator `acc=4`
/// with `%Warning-MULTIDRIVEN`. §4.5.505 settled that split for the TASK spelling as
/// E3001; the FUNCTION spelling measures identically and gets the same answer.
///
/// PRE: E3009 on the body write (the multidriver pass never got to speak).
#[test]
fn two_always_comb_through_one_function_body_is_e3001() {
    const SRC: &str = r#"module t;
  int acc = 0; int src = 7; int src2 = 3; int r1, r2;
  function automatic int fw(input int v); acc = v + 1; return v; endfunction
  always_comb r1 = fw(src);
  always_comb r2 = fw(src2);
  initial begin #1 $display("MDBF acc=%0d r1=%0d r2=%0d", acc, r1, r2); $finish; end
endmodule
"#;
    let e = loud(SRC);
    assert!(e.contains("E3001"), "{e}");
    assert!(
        e.contains("written by `always_comb` AND by `always_comb`"),
        "{e}"
    );
    // The TASK spelling of the same shape, which already answered this way, as the control.
    let e = loud(
        r#"module t;
  int acc = 0; int src = 7; int src2 = 3;
  task tw(input int v); acc = v + 1; endtask
  always_comb tw(src);
  always_comb tw(src2);
  initial begin #1 $display("MDBT acc=%0d", acc); $finish; end
endmodule
"#,
    );
    assert!(
        e.contains("written by `always_comb` AND by `always_comb`"),
        "{e}"
    );
    // A CONDITIONAL reach is NOT the flagged pair: verilator reports LATCH, not
    // MULTIDRIVEN, so the error would be over-rejection. The oracles do split on the value
    // (iverilog `MDBC acc=8`, verilator `MDBC acc=4`); vita reproduces verilator. Recorded
    // as measured, not as settled — ROADMAP §2.
    assert_eq!(
        run(r#"module t;
  int acc = 0; int src = 7; int src2 = 3; int r1, r2;
  function automatic int fw(input int v); acc = v + 1; return v; endfunction
  always_comb begin if (src > 3) r1 = fw(src); else r1 = 0; end
  always_comb r2 = fw(src2);
  initial begin #1 $display("MDBC acc=%0d r1=%0d r2=%0d", acc, r1, r2); $finish; end
endmodule
"#),
        "MDBC acc=4 r1=7 r2=3"
    );
}

// ── residues: measured, still loud ────────────────────────────────────────────────

/// ⑫ The INLINE spelling (no `return`, no `automatic`, no 2-state return type) is not
/// framed, so this slice does not reach it: `fold_straight_line` still has no statement to
/// emit the write from. Both oracles run it (`OLD1 acc=8 r1=7`). The message's advice was
/// *"declare `fw` `automatic` for the same diagnostic from the frame path"* — no longer
/// true, and now says which spelling works.
#[test]
fn the_inline_spelling_is_still_loud_and_points_at_the_one_that_works() {
    let e = loud(
        r#"module t;
  logic [7:0] acc = 0; logic [7:0] src = 7; logic [7:0] r1;
  function logic [7:0] fw(input logic [7:0] v); begin acc = v + 1; fw = v; end endfunction
  always_comb r1 = fw(src);
  initial begin #1 $display("OLD1 acc=%0d r1=%0d", acc, r1); $finish; end
endmodule
"#,
    );
    assert!(e.contains("not one of its own formals or locals"), "{e}");
    assert!(e.contains("the frame path performs the write"), "{e}");
    assert!(
        !e.contains("for the same diagnostic from the frame path"),
        "the stale advice is gone:\n{e}"
    );
    // …and the spelling it names does work: the same body with a `return`.
    assert_eq!(
        run(r#"module t;
  logic [7:0] acc = 0; logic [7:0] src = 7; logic [7:0] r1;
  function logic [7:0] fw(input logic [7:0] v); acc = v + 1; return v; endfunction
  always_comb r1 = fw(src);
  initial begin #1 $display("OLD1 acc=%0d r1=%0d", acc, r1); $finish; end
endmodule
"#),
        "OLD1 acc=8 r1=7"
    );
}

/// ⑬ A CLASS METHOD body writing a module net is untouched: the name does not even resolve
/// from a method scope (`E3010 undeclared net/variable $class$C$m.acc2`), which is a
/// separate gap and fires before the frame gate. Both oracles run it (`CLS r=7 acc2=9`).
/// The pin is here so the next change on this axis meets the measured value.
#[test]
fn a_class_method_body_write_is_untouched() {
    let e = loud(
        r#"module t;
  int acc2 = 0;
  class C; function automatic int m(input int v); acc2 = v + 2; return v; endfunction endclass
  C c; int r;
  initial begin c = new(); r = c.m(7); $display("CLS r=%0d acc2=%0d", r, acc2); $finish; end
endmodule
"#,
    );
    assert!(
        e.contains("undeclared net/variable `$class$C$m.acc2`"),
        "{e}"
    );
}
