//! Round-23 §3.1/§4: a call's COPY-OUT destination outside the calling frame's window.
//!
//! The report's root, narrowed to one line: `compute_suspendable_tasks` reads `Stmt`
//! lvalues, and a `Terminator::Call`'s copy-out destinations are not `Stmt` lvalues —
//! they ride the call-site side table (`task_calls_func`), keyed by the call block's
//! global id. So `inner(a, gv)` inside a `task automatic` — with `gv` a module net —
//! left the CALLER classified "subset", the synchronous `&self` `run_task` performed the
//! copy-out, and `frame_write_lvalue` hit an unrouted net: `thread 'vita-main' panicked
//! … frame lvalue net is routed`, rc=101, no `errors=` line, no user source location.
//!
//! It is the same question the `BlockingAssign` arm has asked since r18 ("does this
//! statement write outside `[lo, hi)`?"), asked of the other statement form that writes a
//! caller lvalue. Every test below therefore pins BOTH halves: the escaping shape now
//! produces iverilog's value, and the shapes that already worked still do — because the
//! tell for this defect class is that an UNRELATED neighbouring statement decides the
//! answer (`#5 inner(a, gv);` and `if (c) inner(a,gv); else gv = 0;` both worked before
//! the fix, for reasons that have nothing to do with the call).
//!
//! Oracle: iverilog 13 accepts every task-form probe here and each expected value was
//! measured against it. The `function`-with-output-formal probes have no iverilog lane
//! (it rejects the declaration), so those are hand-IEEE §13.4.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run a source string, returning combined stdout+stderr and whether exit was 0.
/// Per-test temp DIR so a parallel suite run cannot collide on the source file name.
/// The timeout is a CI killswitch, not a pin: one probe here runs to t=5ns = 5000 ticks.
fn run(src: &str) -> (String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("vita_r23_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("t.sv");
    std::fs::write(&p, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .arg("--timeout")
        .arg("1000000")
        .current_dir(&dir)
        .output()
        .expect("run vita");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    let _ = std::fs::remove_dir_all(&dir);
    (s, out.status.success())
}

/// Assert a source runs clean and prints `want`. Checks for the panic explicitly: the
/// defect this file exists for did not produce a diagnostic, so "no error[VITA-" alone
/// would have passed on the broken build.
#[track_caller]
fn pins(label: &str, src: &str, want: &str) {
    let (o, ok) = run(src);
    assert!(
        !o.contains("panicked"),
        "{label}: aborted instead of running:\n{o}"
    );
    assert!(
        ok && !o.contains("error[VITA") && !o.contains("fatal[VITA") && o.contains(want),
        "{label}: expected `{want}`:\n{o}"
    );
}

// ── §3.1 the reported shape, and every out-actual form it comes in ────────────────────

#[test]
fn a_bare_call_whose_output_actual_is_a_module_net() {
    // The report's §3.1 verbatim. iverilog: `d=1 gv=5`.
    pins(
        "output actual = module net",
        "`timescale 1ns/1ps
module t;
  int gv;
  task automatic inner (input int i, output int o); o = i; endtask
  task automatic outer (input int a, output int done); inner(a, gv); done = 1; endtask
  initial begin int d; outer(5, d); $display(\"d=%0d gv=%0d\", d, gv); $finish; end
endmodule
",
        "d=1 gv=5",
    );
    // The `inout` variant the report also filed. iverilog: `d=1 gv=8` (gv seeded 3, +5).
    pins(
        "inout actual = module net",
        "`timescale 1ns/1ps
module t;
  int gv;
  task automatic inner (input int i, inout int o); o = i + o; endtask
  task automatic outer (input int a, output int done); inner(a, gv); done = 1; endtask
  initial begin int d; gv = 3; outer(5, d); $display(\"d=%0d gv=%0d\", d, gv); $finish; end
endmodule
",
        "d=1 gv=8",
    );
}

#[test]
fn every_escaping_destination_shape() {
    // A whole net was the reported one; the copy-out lvalue can also be a bit-select, a
    // part-select, an array element, or a string. All iverilog-measured.
    pins(
        "bit-select dest",
        "`timescale 1ns/1ps
module t;
  logic [7:0] gv;
  task automatic inner (input int i, output logic o); o = i[0]; endtask
  task automatic outer (input int a, output int done); inner(a, gv[3]); done = 1; endtask
  initial begin int d; gv = 8'h00; outer(5, d); $display(\"d=%0d gv=%02h\", d, gv); $finish; end
endmodule
",
        "d=1 gv=08",
    );
    pins(
        "part-select dest",
        "`timescale 1ns/1ps
module t;
  logic [15:0] gv;
  task automatic inner (input int i, output logic [3:0] o); o = i[3:0]; endtask
  task automatic outer (input int a, output int done); inner(a, gv[7:4]); done = 1; endtask
  initial begin int d; gv = 16'h0000; outer(5, d); $display(\"d=%0d gv=%04h\", d, gv); $finish; end
endmodule
",
        "d=1 gv=0050",
    );
    pins(
        "array-element dest",
        "`timescale 1ns/1ps
module t;
  int gv [0:3];
  task automatic inner (input int i, output int o); o = i; endtask
  task automatic outer (input int a, output int done); inner(a, gv[2]); done = 1; endtask
  initial begin int d; outer(5, d); $display(\"d=%0d gv2=%0d\", d, gv[2]); $finish; end
endmodule
",
        "d=1 gv2=5",
    );
    pins(
        "string dest",
        "`timescale 1ns/1ps
module t;
  string gs;
  task automatic inner (input int i, output string o); o = \"hi\"; endtask
  task automatic outer (input int a, output int done); inner(a, gs); done = 1; endtask
  initial begin int d; outer(5, d); $display(\"d=%0d gs=%s\", d, gs); $finish; end
endmodule
",
        "d=1 gs=hi",
    );
}

#[test]
fn the_gate_is_per_destination_not_per_call() {
    // TWO escaping destinations on one call, and a call MIXING an escaping destination
    // with a frame-local one. A gate that answered per-call ("this call has an escaping
    // dest") rather than per-destination would still have to write the local one through
    // the frame slot — so both must land. iverilog: `d=1 ga=5 gb=6` / `d=6 ga=5`.
    pins(
        "two escaping dests",
        "`timescale 1ns/1ps
module t;
  int ga, gb;
  task automatic inner (input int i, output int x, output int y); x = i; y = i + 1; endtask
  task automatic outer (input int a, output int done); inner(a, ga, gb); done = 1; endtask
  initial begin int d; outer(5, d); $display(\"d=%0d ga=%0d gb=%0d\", d, ga, gb); $finish; end
endmodule
",
        "d=1 ga=5 gb=6",
    );
    pins(
        "escaping + frame-local dest on one call",
        "`timescale 1ns/1ps
module t;
  int ga;
  task automatic inner (input int i, output int x, output int y); x = i; y = i + 1; endtask
  task automatic outer (input int a, output int done);
    automatic int loc; inner(a, ga, loc); done = loc; endtask
  initial begin int d; outer(5, d); $display(\"d=%0d ga=%0d\", d, ga); $finish; end
endmodule
",
        "d=6 ga=5",
    );
}

#[test]
fn the_escaping_call_in_control_flow() {
    // In a loop body (the copy-out fires once per iteration — last write wins, gv=14),
    // and three frames deep with the escape at the INNERMOST level (the caller two
    // levels up must still be routed, which is what the transitive closure buys).
    pins(
        "escaping call in a loop body",
        "`timescale 1ns/1ps
module t;
  int gv;
  task automatic inner (input int i, output int o); o = i * 2; endtask
  task automatic outer (input int a, output int done);
    for (int k = 0; k < 3; k++) inner(a + k, gv);
    done = 1; endtask
  initial begin int d; outer(5, d); $display(\"d=%0d gv=%0d\", d, gv); $finish; end
endmodule
",
        "d=1 gv=14",
    );
    pins(
        "escape three frames deep",
        "`timescale 1ns/1ps
module t;
  int gv;
  task automatic leaf (input int i, output int o); o = i; endtask
  task automatic mid  (input int i, output int o); leaf(i, gv); o = i + 1; endtask
  task automatic outer (input int a, output int done);
    automatic int m; mid(a, m); done = m; endtask
  initial begin int d; outer(5, d); $display(\"d=%0d gv=%0d\", d, gv); $finish; end
endmodule
",
        "d=6 gv=5",
    );
    // The copy-out must be VISIBLE to the rest of the body, not just eventually land:
    // `done = gv + 1` reads it in the next statement. iverilog: `d=16 gv=15`.
    pins(
        "read the escaped destination back in the same body",
        "`timescale 1ns/1ps
module t;
  int gv;
  task automatic inner (input int i, output int o); o = i * 3; endtask
  task automatic outer (input int a, output int done); inner(a, gv); done = gv + 1; endtask
  initial begin int d; outer(5, d); $display(\"d=%0d gv=%0d\", d, gv); $finish; end
endmodule
",
        "d=16 gv=15",
    );
}

// ── the equivalence the defect broke: a neighbouring statement must not decide ─────────

#[test]
fn an_unrelated_neighbouring_statement_does_not_decide() {
    // These four PASSED before the fix, each for a reason unrelated to the call: the
    // `#5` supplies a `Delay` suspend signal, the `else` arm supplies its own
    // out-of-window write, the local/own-formal destinations never escape. They are the
    // boundary that made the defect look shapeless — and they must keep working, at the
    // same values, now that the escaping form works too.
    pins(
        "delay in the same body",
        "`timescale 1ns/1ps
module t;
  int gv;
  task automatic inner (input int i, output int o); o = i; endtask
  task automatic outer (input int a, output int done); #5 inner(a, gv); done = 1; endtask
  initial begin int d; outer(5, d); $display(\"t=%0t d=%0d gv=%0d\", $time, d, gv); $finish; end
endmodule
",
        "t=5000 d=1 gv=5",
    );
    pins(
        "else arm writes the same net",
        "`timescale 1ns/1ps
module t;
  int gv;
  task automatic inner (input int i, output int o); o = i; endtask
  task automatic outer (input int a, output int done);
    if (a > 3) inner(a, gv); else gv = 0;
    done = 1; endtask
  initial begin int d; outer(5, d); $display(\"d=%0d gv=%0d\", d, gv); $finish; end
endmodule
",
        "d=1 gv=5",
    );
    pins(
        "destination is a caller local",
        "`timescale 1ns/1ps
module t;
  task automatic inner (input int i, output int o); o = i; endtask
  task automatic outer (input int a, output int done);
    automatic int loc; inner(a, loc); done = loc; endtask
  initial begin int d; outer(5, d); $display(\"d=%0d\", d); $finish; end
endmodule
",
        "d=5",
    );
    pins(
        "destination is the caller's own output formal",
        "`timescale 1ns/1ps
module t;
  task automatic inner (input int i, output int o); o = i; endtask
  task automatic outer (input int a, output int res); inner(a, res); endtask
  initial begin int r; outer(5, r); $display(\"r=%0d\", r); $finish; end
endmodule
",
        "r=5",
    );
    // …and the two contexts that were never frames at all.
    pins(
        "caller has no formals (inlined, not a frame)",
        "`timescale 1ns/1ps
module t;
  int gv, dn;
  task automatic inner (input int i, output int o); o = i; endtask
  task automatic outer (); inner(5, gv); dn = 1; endtask
  initial begin outer(); $display(\"dn=%0d gv=%0d\", dn, gv); $finish; end
endmodule
",
        "dn=1 gv=5",
    );
    pins(
        "the same call in a module process",
        "`timescale 1ns/1ps
module t;
  int gv;
  task automatic inner (input int i, output int o); o = i; endtask
  initial begin inner(5, gv); $display(\"gv=%0d\", gv); $finish; end
endmodule
",
        "gv=5",
    );
}

// ── a value-returning FUNCTION with an output formal, called from a frame TASK body ───

#[test]
fn a_value_returning_call_with_an_escaping_output_actual() {
    // Hand-IEEE §13.4 (iverilog rejects a function output formal): `nxt(5, gv)` writes
    // gv = 5 and returns 6. Four positions — direct rhs to a frame-local, direct rhs to a
    // MODULE net (both destinations escape), `void'()`, and buried in an expression. The
    // last three were E3009 before this slice; the first was the panic.
    pins(
        "rhs to a frame-local, output actual escapes",
        "`timescale 1ns/1ps
module t;
  int gv;
  function automatic int nxt (input int i, output int o); o = i; return i + 1; endfunction
  task automatic outer (input int a, output int done);
    automatic int r; r = nxt(a, gv); done = r; endtask
  initial begin int d; outer(5, d); $display(\"d=%0d gv=%0d\", d, gv); $finish; end
endmodule
",
        "d=6 gv=5",
    );
    pins(
        "rhs to a module net, output actual escapes too",
        "`timescale 1ns/1ps
module t;
  int gv, res;
  function automatic int nxt (input int i, output int o); o = i; return i + 1; endfunction
  task automatic outer (input int a, output int done); res = nxt(a, gv); done = 1; endtask
  initial begin int d; outer(5, d); $display(\"d=%0d gv=%0d res=%0d\", d, gv, res); $finish; end
endmodule
",
        "d=1 gv=5 res=6",
    );
    pins(
        "void'() form",
        "`timescale 1ns/1ps
module t;
  int gv;
  function automatic int nxt (input int i, output int o); o = i; return i + 1; endfunction
  task automatic outer (input int a, output int done); void'(nxt(a, gv)); done = 1; endtask
  initial begin int d; outer(5, d); $display(\"d=%0d gv=%0d\", d, gv); $finish; end
endmodule
",
        "d=1 gv=5",
    );
    pins(
        "buried in an expression (the general hoist)",
        "`timescale 1ns/1ps
module t;
  int gv;
  function automatic int nxt (input int i, output int o); o = i; return i + 1; endfunction
  task automatic outer (input int a, output int done);
    automatic int r; r = nxt(a, gv) + 10; done = r; endtask
  initial begin int d; outer(5, d); $display(\"d=%0d gv=%0d\", d, gv); $finish; end
endmodule
",
        "d=16 gv=5",
    );
}

// ── §4 diagnostic quality ─────────────────────────────────────────────────────────────

#[test]
fn the_e3009_text_no_longer_claims_the_bare_call_form_works() {
    // The report's §4: the message said "a BARE call statement there does work", and that
    // was the exact construct that aborted the engine. Trusting it turned a loud error
    // into a crash. The claim must be gone, and what replaces it must be true — a frame
    // FUNCTION body is what is left, and the message has to say `task` works.
    let (o, ok) = run("`timescale 1ns/1ps
module t;
  int gv;
  function automatic int nxt (input int i, output int o); o = i; return i + 1; endfunction
  function automatic int outer (input int a);
    automatic int r; r = nxt(a, gv); return r; endfunction
  initial begin int d; d = outer(5); $display(\"d=%0d gv=%0d\", d, gv); $finish; end
endmodule
");
    assert!(
        !ok && !o.contains("panicked"),
        "must be loud, not a crash:\n{o}"
    );
    assert!(o.contains("error[VITA-E3009]"), "expected E3009:\n{o}");
    assert!(
        !o.contains("BARE call statement"),
        "the false claim is still in the message:\n{o}"
    );
    assert!(
        o.contains("FUNCTION body") && o.contains("TASK body"),
        "the message must name what is left and what works:\n{o}"
    );
}

#[test]
fn a_rejected_call_terminator_is_not_reported_as_a_timing_control() {
    // A nested output-formal call in a frame FUNCTION body is rejected by
    // `classify_frame_body(allow_call = false)`. That arm answered "a timing/suspend/fork
    // control (#delay, @, wait, fork)" for EVERY terminator it refused — naming a cause
    // this source does not contain, which is the same misdiagnosis class as §4.
    let (o, ok) = run("`timescale 1ns/1ps
module t;
  function automatic int nxt (input int i, output int o); o = i; return i + 1; endfunction
  function automatic int outer (input int a);
    automatic int r, loc; r = nxt(a, loc); return r + loc; endfunction
  initial begin int d; d = outer(5); $display(\"d=%0d\", d); $finish; end
endmodule
");
    assert!(
        !ok && !o.contains("panicked"),
        "must be loud, not a crash:\n{o}"
    );
    assert!(
        !o.contains("timing/suspend/fork control"),
        "there is no timing control in this body:\n{o}"
    );
    assert!(
        o.contains("output/inout formal"),
        "the diagnostic must name the real terminator:\n{o}"
    );
}

#[test]
fn the_threads_flag_does_not_advertise_a_speedup_it_cannot_give() {
    // §3.2 ④: the report measured `-j 1/4/16` at 4.86/4.85/4.88 s with the process pinned
    // at one core, and reasonably read the help text ("worker threads") as a simulation
    // parallelism knob. It is a waveform-writer budget; simulation is single-threaded.
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("--help")
        .output()
        .expect("run vita --help");
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        s.contains("--threads, -j")
            && s.contains("single-threaded")
            && s.contains("waveform-writer"),
        "the --threads help must say what it actually does:\n{s}"
    );
}

// ── adversarial: the hazards THIS change creates ──────────────────────────────────────

#[test]
fn the_hoist_temp_is_not_shared_across_activations() {
    // Lifting the hoist stand-down means a frame TASK body now mints a MODULE-net temp
    // per call site. Under recursion two activations reach the same temp, so if the temp
    // were live across a suspension point the inner call would clobber the outer's value.
    // It is not: a framed callee carries no timing control and `run_process`'s `Call` arm
    // does not yield, so the temp is written and consumed with no scheduling point in
    // between. Hand-IEEE §13.4.2: down(4) computes 5+4+3+2 = 14 and leaves gv at the
    // INNERMOST write, 1.
    pins(
        "recursion, hoist temp in a direct rhs",
        "`timescale 1ns/1ps
module t;
  int gv;
  function automatic int nx (input int a, output int o); o = a; return a + 1; endfunction
  task automatic down (input int n, output int r);
    automatic int h;
    if (n == 0) begin r = 0; end
    else begin automatic int s; h = nx(n, gv); down(n - 1, s); r = h + s; end
  endtask
  initial begin int d; down(4, d); $display(\"d=%0d gv=%0d\", d, gv); $finish; end
endmodule
",
        "d=14 gv=1",
    );
    // The same with the call BURIED in an expression, and the recursion happening BEFORE
    // it — so the temp is written after every inner activation has finished with it.
    // down(3): r = nx(3)*10 + (nx(2)*10 + (nx(1)*10 + 0)) = 40 + 30 + 20 = 90, gv = 3.
    pins(
        "recursion, hoist temp inside an expression",
        "`timescale 1ns/1ps
module t;
  int gv;
  function automatic int nx (input int a, output int o); o = a; return a + 1; endfunction
  task automatic down (input int n, output int r);
    if (n == 0) r = 0;
    else begin automatic int s; down(n - 1, s); r = nx(n, gv) * 10 + s; end
  endtask
  initial begin int d; down(3, d); $display(\"d=%0d gv=%0d\", d, gv); $finish; end
endmodule
",
        "d=90 gv=3",
    );
}

#[test]
fn every_lvalue_special_above_the_relaxed_gate_still_wins() {
    // The `stmt_main` gate now bypasses both the destination check and the lvalue check
    // inside a frame task body, so each earlier-returning lvalue special has to be shown
    // still to own its shape — a string element must be a BYTE write, an array element a
    // WORD write, a part-select a bit-range write. Hand-IEEE (iverilog rejects the
    // function's output formal in every one of these).
    pins(
        "module array element",
        "`timescale 1ns/1ps
module t;
  int gv; int m [0:3];
  function automatic int nx (input int a, output int o); o = a; return a + 1; endfunction
  task automatic tk (input int a, output int r); m[2] = nx(a, gv); r = 1; endtask
  initial begin int d; tk(5, d); $display(\"d=%0d m2=%0d gv=%0d\", d, m[2], gv); $finish; end
endmodule
",
        "d=1 m2=6 gv=5",
    );
    pins(
        "frame-local array element",
        "`timescale 1ns/1ps
module t;
  int gv;
  function automatic int nx (input int a, output int o); o = a; return a + 1; endfunction
  task automatic tk (input int a, output int r);
    int q [0:3]; q[2] = nx(a, gv); r = q[2]; endtask
  initial begin int d; tk(5, d); $display(\"d=%0d gv=%0d\", d, gv); $finish; end
endmodule
",
        "d=6 gv=5",
    );
    pins(
        "dynamic-array element",
        "`timescale 1ns/1ps
module t;
  int gv; int dq [];
  function automatic int nx (input int a, output int o); o = a; return a + 1; endfunction
  task automatic tk (input int a, output int r); dq[1] = nx(a, gv); r = dq[1]; endtask
  initial begin int d; dq = new[3]; tk(5, d); $display(\"d=%0d gv=%0d\", d, gv); $finish; end
endmodule
",
        "d=6 gv=5",
    );
    pins(
        "part-select",
        "`timescale 1ns/1ps
module t;
  int gv; logic [15:0] w;
  function automatic int nx (input int a, output int o); o = a; return a + 1; endfunction
  task automatic tk (input int a, output int r); w[7:4] = nx(a, gv); r = 1; endtask
  initial begin int d; w = 0; tk(5, d); $display(\"d=%0d w=%04h gv=%0d\", d, w, gv); $finish; end
endmodule
",
        "d=1 w=0060 gv=5",
    );
    // The string element — a BYTE write, and the one that exposed the pre-existing
    // `StrPutC` routing bug (a frame-local string is slab-stored, not in `dyn_heap`).
    pins(
        "string element",
        "`timescale 1ns/1ps
module t;
  int gv;
  function automatic int nx (input int a, output int o); o = a; return 8'h41; endfunction
  task automatic tk (input int a, output int r);
    automatic string s = \"abc\";
    s[1] = nx(a, gv);
    r = 1;
    $display(\"s=%s gv=%0d\", s, gv);
  endtask
  initial begin int d; tk(5, d); $display(\"d=%0d\", d); $finish; end
endmodule
",
        "s=aAc gv=5",
    );
}

#[test]
fn over_marking_does_not_create_a_new_loud() {
    // A task newly routed to the process executor must not trip one of the
    // suspendable-subset rejects and become LOUD where it used to work. These four pair
    // an escaping copy-out with each construct that gate examines. iverilog-measured.
    pins(
        "escape + a frame-local unpacked array",
        "`timescale 1ns/1ps
module t;
  int gv;
  task automatic inner (input int i, output int o); o = i; endtask
  task automatic tk (input int a, output int r);
    int loc [0:2]; inner(a, gv); loc[0] = a; r = loc[0]; endtask
  initial begin int d; tk(5, d); $display(\"d=%0d gv=%0d\", d, gv); $finish; end
endmodule
",
        "d=5 gv=5",
    );
    pins(
        "escape inside a fork arm",
        "`timescale 1ns/1ps
module t;
  int gv;
  task automatic inner (input int i, output int o); o = i; endtask
  task automatic tk (input int a, output int r); fork inner(a, gv); join r = 1; endtask
  initial begin int d; tk(5, d); $display(\"d=%0d gv=%0d\", d, gv); $finish; end
endmodule
",
        "d=1 gv=5",
    );
    pins(
        "escape + a nonblocking assign",
        "`timescale 1ns/1ps
module t;
  int gv, nb;
  task automatic inner (input int i, output int o); o = i; endtask
  task automatic tk (input int a, output int r); inner(a, gv); nb <= a * 2; r = 1; endtask
  initial begin int d; tk(5, d); #1 $display(\"d=%0d gv=%0d nb=%0d\", d, gv, nb); $finish; end
endmodule
",
        "d=1 gv=5 nb=10",
    );
    pins(
        "escape + a named-block disable",
        "`timescale 1ns/1ps
module t;
  int gv;
  task automatic inner (input int i, output int o); o = i; endtask
  task automatic tk (input int a, output int r);
    begin : blk inner(a, gv); if (a > 3) disable blk; gv = 99; end
    r = 1; endtask
  initial begin int d; tk(5, d); $display(\"d=%0d gv=%0d\", d, gv); $finish; end
endmodule
",
        "d=1 gv=5",
    );
}

#[test]
fn elaborate_and_the_engine_agree_over_a_deferred_hierarchical_enable() {
    // `compute_suspendable_tasks` is a pure function computed independently by elaborate
    // and by the engine, and the new input — the call sites' copy-out destinations — is
    // the one table whose SIZE legitimately differs between them: elaborate runs
    // pre-resolve, so a deferred hierarchical enable (`u.set(...)`) is not in its copy.
    // A missing entry is deliberately not a signal; `FuncMeta.has_hier_call` forces those
    // callers suspendable in BOTH computes instead. If that reasoning were wrong the two
    // sets would diverge and this would be loud on one side or wrong on the other.
    // iverilog: `d=1 v=9 gv=10` / `d=10 v=9`.
    pins(
        "hier enable, escaping output actual",
        "`timescale 1ns/1ps
module sub;
  int v;
  task automatic set (input int x, output int y); v = x; y = x + 1; endtask
endmodule
module t;
  sub u(); int gv;
  task automatic go (input int a, output int r); u.set(a, gv); r = 1; endtask
  initial begin int d; go(9, d); $display(\"d=%0d v=%0d gv=%0d\", d, u.v, gv); $finish; end
endmodule
",
        "d=1 v=9 gv=10",
    );
    pins(
        "hier enable, frame-local output actual",
        "`timescale 1ns/1ps
module sub;
  int v;
  task automatic set (input int x, output int y); v = x; y = x + 1; endtask
endmodule
module t;
  sub u();
  task automatic go (input int a, output int r);
    automatic int loc; u.set(a, loc); r = loc; endtask
  initial begin int d; go(9, d); $display(\"d=%0d v=%0d\", d, u.v); $finish; end
endmodule
",
        "d=10 v=9",
    );
}

#[test]
fn the_copy_out_fires_at_the_call_not_at_frame_exit() {
    // Routing the caller to a different executor must not move WHEN the copy-out lands.
    // `gv` is 100 before the call and must read 5 on the very next statement — a copy-out
    // deferred to the frame's exit would leave `b` at 100. iverilog: `a=100 b=5 gv=5`.
    pins(
        "escaped value is visible to the next statement",
        "`timescale 1ns/1ps
module t;
  int gv, log_a, log_b;
  task automatic inner (input int i, output int o); o = i; endtask
  task automatic tk (input int a, output int r);
    gv = 100; log_a = gv; inner(a, gv); log_b = gv; r = 1; endtask
  initial begin int d; tk(5, d);
    $display(\"d=%0d a=%0d b=%0d gv=%0d\", d, log_a, log_b, gv); $finish; end
endmodule
",
        "d=1 a=100 b=5 gv=5",
    );
    // Two calls feeding the same escaping destination, the second READING what the first
    // wrote: 5*2 = 10, then 10*3 = 30. iverilog: `d=30 gv=30`.
    pins(
        "chained calls through one escaping destination",
        "`timescale 1ns/1ps
module t;
  int gv;
  task automatic p (input int i, output int o); o = i * 2; endtask
  task automatic q (input int i, output int o); o = i * 3; endtask
  task automatic tk (input int a, output int r); p(a, gv); q(gv, gv); r = gv; endtask
  initial begin int d; tk(5, d); $display(\"d=%0d gv=%0d\", d, gv); $finish; end
endmodule
",
        "d=30 gv=30",
    );
}
