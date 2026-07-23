//! Frame-call model (automatic/recursive functions) — ENGINE layer (B1
//! Increment 2). The runtime lifts the v1 loud rejection of automatic/recursive
//! functions by lowering each callee body ONCE into the reserved `ir.blocks`
//! func arena and executing it against a per-invocation frame (IR-0: the
//! `Frame`/`FuncDef`/`Expr::Call` shapes were pre-frozen at PR1-B/M3, so
//! `format_version` stays 8).
//!
//! No front-end syntax exists yet (that is Increment 4, batched with the `.vu`
//! flip), so these tests HAND-BUILD a frozen `SimIr` + a populated `FuncTable`
//! and drive them through the public `simulate`/`simulate_capture` seam — exactly
//! what elaborate will emit once the syntax lands (the assoc/iface precedent).
//!
//! Oracle: iverilog 13.0 models automatic recursion (fresh per-call storage,
//! IEEE 1800 §13.4.2) AND static-lifetime corruption faithfully. The probe
//! (`acc = n*10` before the recursive call, read `acc` after) is the lifetime
//! discriminator: for `f = f(n-1) + acc`, automatic `probe(3)=60` (each frame
//! keeps its own `acc`) vs static `probe(3)=30` (the shared `acc` is clobbered
//! to the deepest frame's `10`, so every level adds 10). Oracle-verified live;
//! the REAL-pipeline differential lands at Increment 5 (the `#[ignore]`d section
//! at the bottom). The deep/runaway corpus runs on a large-stack worker thread
//! so the depth CAP — not a host stack overflow — is the guard.

#[path = "frame_call_util/mod.rs"]
mod util;
#[allow(unused_imports)]
use util::*;

#[test]
fn frame_body_loud_rejects_unsupported_constructs() {
    // Family D (r17): a genuine `$display`/`$write` in a frame FUNCTION body is now
    // RENDERED by the `&self` executors (via `frame_print_stmts` + the render arm) —
    // it used to be a deliberate B1 cut (loud) because the eval path would silently
    // drop it. iverilog also runs it, so this is now a straight differential. (A
    // severity/timeformat/control $systask stays loud — see the follow-on cases below.)
    check(
        r#"
module tb;
  function automatic integer noisy(input integer n);
    begin $display("x"); noisy = n; end
  endfunction
  initial $display("%0d", noisy(3));
endmodule
"#,
        "x\n3",
    );
    // Writing a MODULE net from a frame function: the &self eval path cannot write
    // the flat store — loud-reject, never a silent mis-route.
    assert!(
        elaborate_rejects(
            r#"
module tb;
  integer g;
  function automatic integer bad(input integer n);
    begin g = n; bad = n; end
  endfunction
  initial $display("%0d", bad(3));
endmodule
"#
        ),
        "a module-net write from a frame function body must be loud-rejected"
    );
    // A non-blocking assign inside a frame body — also outside the subset.
    assert!(
        elaborate_rejects(
            r#"
module tb;
  function automatic integer nb(input integer n);
    nb <= n;
  endfunction
  initial $display("%0d", nb(3));
endmodule
"#
        ),
        "a nonblocking assign in a frame function body must be loud-rejected"
    );
    // B3: `disable <funcname>` (self-disabling a FUNCTION) is illegal — iverilog
    // rejects it ("cannot disable functions"); a TASK self-disable is the legal
    // form. Only named-BLOCK disables (break/continue) are allowed in a function.
    assert!(
        elaborate_rejects(
            r#"
module tb;
  function automatic integer f(input integer n);
    begin f = 0; if (n < 0) disable f; f = n; end
  endfunction
  initial $display("%0d", f(3));
endmodule
"#
        ),
        "self-disabling a frame FUNCTION must be loud-rejected (iverilog: cannot disable functions)"
    );
}

#[test]
fn e2e_recursive_automatic_function_factorial() {
    let src = r#"
module tb;
  function automatic integer fact(input integer n);
    if (n <= 1) fact = 1;
    else fact = n * fact(n - 1);
  endfunction
  initial begin
    $display("fact(5)=%0d", fact(5));
    $display("fact(0)=%0d", fact(0));
    $display("fact(1)=%0d", fact(1));
    $display("fact(10)=%0d", fact(10));
  end
endmodule
"#;
    check(src, "fact(5)=120\nfact(0)=1\nfact(1)=1\nfact(10)=3628800");
}

#[test]
fn e2e_recursive_automatic_task() {
    // B2: a recursive automatic TASK with an output formal, through the REAL
    // pipeline + iverilog. factt(n,r): r=n! computed into the output. Each
    // automatic frame keeps its own local `t`.
    let src = r#"
module tb;
  task automatic factt(input integer n, output integer r);
    integer t;
    if (n <= 1) r = 1;
    else begin factt(n - 1, t); r = n * t; end
  endtask
  integer res;
  initial begin
    factt(5, res); $display("%0d", res);
    factt(0, res); $display("%0d", res);
    factt(7, res); $display("%0d", res);
  end
endmodule
"#;
    check(src, "120\n1\n5040");
}

#[test]
fn e2e_b4_automatic_lifetime_override() {
    // B4 (hand-IEEE — iverilog rejects "overriding the default variable
    // lifetime"): an `automatic` local in a DEFAULT-STATIC recursive function gets
    // fresh-per-call storage. The probe `f = f(n-1) + acc` with `acc = n*10`:
    //   `automatic integer acc` → each frame keeps its own acc → probe(3) = 60
    //   plain (default-static) `integer acc` → shared/clobbered → probe(3) = 30
    // The first is a MIXED-lifetime frame (automatic acc, static n/return).
    let src = r#"
module tb;
  function integer probe_a(input integer n);
    automatic integer acc;
    begin
      acc = n * 10;
      if (n > 1) probe_a = probe_a(n - 1) + acc;
      else probe_a = acc;
    end
  endfunction
  function integer probe_s(input integer n);
    integer acc;
    begin
      acc = n * 10;
      if (n > 1) probe_s = probe_s(n - 1) + acc;
      else probe_s = acc;
    end
  endfunction
  initial begin
    $display("%0d", probe_a(3));
    $display("%0d", probe_s(3));
  end
endmodule
"#;
    // value-pin only (no iverilog oracle — it rejects the override).
    let out = vita_out(src);
    assert_eq!(
        lines_trimmed(&out),
        vec!["60", "30"],
        "automatic-local acc is per-frame (60); default-static acc is shared (30)"
    );
}

#[test]
fn e2e_frame_function_disable_break() {
    // B3: the `disable <named block>` break/continue idiom inside a frame
    // function. `disable scan` ends the current loop-body block (= continue), so
    // the LAST set bit wins (matches iverilog's block-disable semantics).
    let src = r#"
module tb;
  function automatic integer lastset(input integer mask);
    integer i;
    begin
      lastset = -1;
      for (i = 0; i < 8; i = i + 1) begin: scan
        if (mask[i]) begin lastset = i; disable scan; end
      end
    end
  endfunction
  initial begin
    $display("%0d", lastset(8'b00101000));
    $display("%0d", lastset(8'b00000000));
    $display("%0d", lastset(8'b10000001));
  end
endmodule
"#;
    check(src, "5\n-1\n7");
}

#[test]
fn e2e_task_self_disable_early_return() {
    // B3: `disable <taskname>` inside a frame task is a self-disable = early
    // return (the single-frame unwind). clampt(-3) returns with r still 0.
    let src = r#"
module tb;
  task automatic clampt(input integer n, output integer r);
    begin
      r = 0;
      if (n < 0) disable clampt;
      r = n * 2;
    end
  endtask
  integer r;
  initial begin
    clampt(5, r);  $display("%0d", r);
    clampt(-3, r); $display("%0d", r);
    clampt(0, r);  $display("%0d", r);
  end
endmodule
"#;
    check(src, "10\n0\n0");
}

#[test]
fn e2e_static_vs_automatic_corruption() {
    // The lifetime discriminator through the REAL pipeline + iverilog: automatic
    // keeps a per-frame `acc` (probe(3)=60); static shares one slot, clobbered to
    // the deepest frame's 10 (probe(3)=30).
    let src = r#"
module tb;
  function automatic integer probe_auto(input integer n);
    integer acc;
    begin
      acc = n * 10;
      if (n > 1) probe_auto = probe_auto(n - 1) + acc;
      else probe_auto = acc;
    end
  endfunction
  function integer probe_static(input integer n);
    integer acc;
    begin
      acc = n * 10;
      if (n > 1) probe_static = probe_static(n - 1) + acc;
      else probe_static = acc;
    end
  endfunction
  initial begin
    $display("auto=%0d", probe_auto(3));
    $display("static=%0d", probe_static(3));
  end
endmodule
"#;
    check(src, "auto=60\nstatic=30");
}

#[test]
fn e2e_mutual_recursion() {
    // Mutual recursion: both is_even/is_odd are reserved BEFORE either body
    // lowers, so the cross-call resolves. is_even(4)=1, is_odd(4)=0.
    let src = r#"
module tb;
  function automatic integer is_even(input integer n);
    if (n == 0) is_even = 1;
    else is_even = is_odd(n - 1);
  endfunction
  function automatic integer is_odd(input integer n);
    if (n == 0) is_odd = 0;
    else is_odd = is_even(n - 1);
  endfunction
  initial begin
    $display("even4=%0d", is_even(4));
    $display("odd4=%0d", is_odd(4));
    $display("even7=%0d", is_even(7));
  end
endmodule
"#;
    check(src, "even4=1\nodd4=0\neven7=0");
}

#[test]
fn e2e_control_flow_static_function() {
    // A non-recursive, non-automatic function with control flow — framed via the
    // `body_needs_frame` rule (the inline path can't fold an if/else). Static
    // storage is harmless without recursion.
    let src = r#"
module tb;
  function integer clamp(input integer x);
    if (x > 100) clamp = 100;
    else if (x < 0) clamp = 0;
    else clamp = x;
  endfunction
  initial begin
    $display("%0d", clamp(150));
    $display("%0d", clamp(-5));
    $display("%0d", clamp(42));
  end
endmodule
"#;
    check(src, "100\n0\n42");
}

#[test]
fn e2e_non_default_return_width() {
    // `function [15:0]` — an UNSIGNED 16-bit return truncates: fact16(8)=40320
    // fits, fact16(9)=9*40320 wraps to 16 bits. The exact wrap is iverilog's
    // (the differential pins it).
    let src = r#"
module tb;
  function automatic [15:0] fact16(input integer n);
    if (n <= 1) fact16 = 1;
    else fact16 = n * fact16(n - 1);
  endfunction
  initial begin
    $display("%0d", fact16(8));
    $display("%0d", fact16(9));
  end
endmodule
"#;
    check(src, "40320\n35200");
}
