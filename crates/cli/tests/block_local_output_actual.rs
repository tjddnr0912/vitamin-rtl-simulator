//! round-19 BL4: signature-aware definite-assignment. An `automatic` block-local is
//! flattened by v1 to ONE static module net; the flatten is byte-identical to a real
//! per-activation `automatic` only when the local is DEFINITELY ASSIGNED before every
//! read on every path (`automatic_local_definitely_assigned` / `da_stmt`). That gate
//! treated EVERY reference to the name as a READ — so a call `f(…, name, …)` where
//! `name` is at an OUTPUT actual position (the callee WRITES it, IEEE §13.5.2 copy-out)
//! was misread as a read-before-write and rejected E3009. It is now recognized as a
//! definite ASSIGNMENT when the call is UNCONDITIONALLY evaluated on the path (a bare
//! call STATEMENT, or the whole cond / scrutinee of an `if`/`while`/`case`, incl. a
//! `!`/paren wrapper) and `name` appears at NO input-read position of that same call.
//!
//! No external oracle — iverilog 13.0 / verilator reject `automatic` lifetime override
//! (`sorry: Overriding the default variable lifetime`). Reference behavior is the
//! ALREADY-WORKING boundary: a direct `name = …;` write is accepted today (P7); the
//! ONLY gap was that an output-actual call was not recognized as that write.
//!
//! SOUNDNESS (this is an ACCEPT gate — a wrong "assigned" would read a leftover/X value
//! instead of erroring = silent-wrong):
//!   * Only a PURE OUTPUT actual (copy-out only, no copy-in) counts as a definite
//!     assignment. An INOUT actual has a copy-IN that READS `name` (verified: vita's
//!     inout copy-in reads the actual's current value), so an inout FIRST reference is a
//!     genuine read-before-write on the flatten (leftover ≠ a fresh automatic's default)
//!     and stays LOUD. An inout after `name` is already assigned is fine (the copy-in
//!     reads the assigned value).
//!   * A conditionally-evaluated call (`cond && setval(x)`, a `?:` arm) may not run, so
//!     it never establishes assignment → stays LOUD.
//!   * A call that reads `name` at an INPUT position AND writes it at an OUTPUT position
//!     in the SAME call (`g(x, x)`) reads it first → stays LOUD.
//!   * A partial-write output actual (`f(name[i])`) is not a definite WHOLE assignment →
//!     stays LOUD.
//!   * A callee whose directions can't be resolved (hierarchical `u.f`, named args) →
//!     conservative → stays LOUD.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Returns (combined stdout+stderr, process_success).
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_bloa_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (text, out.status.success())
}

fn loud(src: &str) -> bool {
    let (o, ok) = run(src);
    !ok && o.contains("E3009")
}

// ── supported (loud → correct-support) ──────────────────────────────────────

#[test]
fn output_actual_da() {
    // BL4 repro: the block-local `cavp_mode` is written by `setmode`'s OUTPUT formal in
    // the (unconditionally-evaluated) `if` condition, before any read. Now accepted; the
    // later `if (cavp_mode == 3)` observes the written value 3.
    let (o, ok) = run("module t;\n\
         function automatic bit setmode (input int x, output logic [4:0] m);\n\
           m = x[4:0]; setmode = (x!=0);\n\
         endfunction\n\
         initial begin\n\
           begin\n\
             automatic logic [4:0] cavp_mode;\n\
             if (!setmode(3, cavp_mode)) $display(\"fail\");\n\
             if (cavp_mode == 3) $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule");
    assert!(ok && o.contains("PASS"), "output_actual_da: {o}");
}

#[test]
fn output_actual_stmt_position() {
    // A bare call STATEMENT `getval(r);` (a task's OUTPUT formal) writes `r`
    // unconditionally, so the following `if (r == 42)` read is safe.
    let (o, ok) = run("module t;\n\
         task automatic getval (output int r); r = 42; endtask\n\
         initial begin\n\
           begin\n\
             automatic int r;\n\
             getval(r);\n\
             if (r == 42) $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule");
    assert!(ok && o.contains("PASS"), "output_actual_stmt_position: {o}");
}

#[test]
fn inout_actual_da() {
    // An OUTPUT call establishes `x`; a following INOUT call reads (copy-in) the
    // assigned value and writes it back — sound, because `x` is already assigned when
    // the inout copy-in runs. `x` = 10 (seed) then 15 (adjust); the read sees 15.
    let (o, ok) = run("module t;\n\
         task automatic seed   (output int v); v = 10; endtask\n\
         task automatic adjust (inout  int v); v = v + 5; endtask\n\
         initial begin\n\
           begin\n\
             automatic int x;\n\
             seed(x);\n\
             adjust(x);\n\
             if (x == 15) $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule");
    assert!(ok && o.contains("PASS"), "inout_actual_da: {o}");
}

#[test]
fn output_actual_paren_cond() {
    // A paren wrapper around the whole-cond call is still unconditionally evaluated.
    let (o, ok) = run("module t;\n\
         function automatic bit setmode (input int x, output logic [4:0] m);\n\
           m = x[4:0]; setmode = 1'b1;\n\
         endfunction\n\
         initial begin\n\
           begin\n\
             automatic logic [4:0] m;\n\
             if ((setmode(7, m))) ;\n\
             if (m == 7) $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule");
    assert!(ok && o.contains("PASS"), "output_actual_paren_cond: {o}");
}

#[test]
fn output_actual_while_cond() {
    // The whole cond of a `while` is evaluated (at least once) unconditionally, so a
    // bare-call output actual there establishes assignment (verified loud at HEAD → now
    // supported: `da_stmt` unwraps the while cond, and the existing inout-call hoist
    // lowers the function-output copy-out). `pw` writes 0 → the body never runs, and the
    // later read of `w` sees the written 0.
    let (o, ok) = run("module t;\n\
         function automatic bit pw (output int w); w = 0; pw = 1'b0; endfunction\n\
         initial begin\n\
           begin\n\
             automatic int w;\n\
             while (pw(w)) ;\n\
             if (w == 0) $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule");
    assert!(ok && o.contains("PASS"), "output_actual_while_cond: {o}");
}

// ── correct-or-loud (must stay E3009) ───────────────────────────────────────

#[test]
fn output_call_in_case_scrutinee_stays_loud() {
    // The DA walk recognizes a whole-scrutinee output-actual call in a `case` as a
    // definite write, but vita does not yet lower a function-with-output-formal call in
    // a case-scrutinee (a separate hoist gap — the F-record-out family), so this stays
    // LOUD overall (a different E3009, "function … has an output/inout formal
    // (illegal)"). Proves the DA acceptance never yields a silent-wrong here.
    assert!(loud(
        "module t;\n\
         function automatic int pick (output int s); s = 2; pick = s; endfunction\n\
         initial begin\n\
           begin\n\
             automatic int s;\n\
             case (pick(s))\n\
               default: ;\n\
             endcase\n\
             if (s == 2) $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule"
    ));
}

#[test]
fn input_actual_stays_loud() {
    // `f(x)`'s formal is INPUT — passing the still-unwritten `x` is a genuine
    // read-before-write, so the flatten (leftover) diverges from a fresh automatic.
    assert!(loud(
        "module t;\n\
         function automatic int f (input int a); f = a + 1; endfunction\n\
         initial begin\n\
           begin\n\
             automatic int x;\n\
             if (f(x) == 5) $display(\"hm\");\n\
             if (x == 3) $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule"
    ));
}

#[test]
fn conditional_output_actual_stays_loud() {
    // The output call `setval(x)` is behind `&&` — it may NOT be evaluated (short
    // circuit), so it cannot establish assignment. Stays loud.
    assert!(loud(
        "module t;\n\
         function automatic bit setval (output int v); v = 1; setval = 1'b1; endfunction\n\
         initial begin\n\
           begin\n\
             automatic int x;\n\
             logic c; c = 1'b0;\n\
             if (c && setval(x)) $display(\"hm\");\n\
             if (x == 1) $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule"
    ));
}

#[test]
fn mixed_read_and_output_stays_loud() {
    // `g(x, x)` — `x` is at an INPUT position (arg0, read) AND an OUTPUT position
    // (arg1). The input read observes the unwritten value → stays loud.
    assert!(loud(
        "module t;\n\
         task automatic g (input int a, output int b); b = a + 1; endtask\n\
         initial begin\n\
           begin\n\
             automatic int x;\n\
             g(x, x);\n\
             if (x == 1) $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule"
    ));
}

#[test]
fn inout_first_ref_stays_loud() {
    // An INOUT actual has a copy-IN that READS `x` (verified). As the FIRST reference to
    // a fresh automatic, that copy-in reads the flatten's leftover — which diverges from
    // a real automatic's fresh default — so it is a genuine read-before-write. Loud.
    assert!(loud(
        "module t;\n\
         task automatic tw (inout int v); v = v + 1; endtask\n\
         initial begin\n\
           begin\n\
             automatic int x;\n\
             tw(x);\n\
             if (x == 1) $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule"
    ));
}

#[test]
fn partial_output_actual_stays_loud() {
    // `set0(m[0])` writes only element 0 of `m` (a partial/RMW write) — it is NOT a
    // definite WHOLE assignment, so a later whole read of `m` is still read-before-write.
    assert!(loud(
        "module t;\n\
         task automatic set0 (output int e); e = 9; endtask\n\
         initial begin\n\
           begin\n\
             automatic int m [0:3];\n\
             set0(m[0]);\n\
             if (m[1] == 9) $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule"
    ));
}

#[test]
fn hier_call_output_actual_stays_loud() {
    // A hierarchical callee `u.f(x)` — da.rs cannot resolve the child module's port
    // directions here, so it conservatively treats the reference as a read. Loud.
    assert!(loud(
        "module sub;\n\
         task automatic w (output int v); v = 7; endtask\n\
         endmodule\n\
         module t;\n\
         sub u();\n\
         initial begin\n\
           begin\n\
             automatic int x;\n\
             u.w(x);\n\
             if (x == 7) $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule"
    ));
}

// ── regression: a normal direct-assign automatic still works (byte-identical) ─

#[test]
fn direct_assign_still_supported() {
    // The pre-existing accepted boundary (P7): a direct whole-var write. Unchanged.
    let (o, ok) = run("module t;\n\
         initial begin\n\
           begin\n\
             automatic int x;\n\
             x = 8;\n\
             if (x == 8) $display(\"PASS\");\n\
           end\n\
           $finish;\n\
         end\n\
         endmodule");
    assert!(
        ok && o.contains("PASS"),
        "direct_assign_still_supported: {o}"
    );
}
