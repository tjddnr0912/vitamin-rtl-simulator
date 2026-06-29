//! Bare, paren-free event control `@e` (IEEE 1364 `event_control ::= @
//! hierarchical_event_identifier`). vita previously parse-rejected `@e` / `@clk`
//! ("expected '(' or '*' after '@'") while accepting `@(e)`; iverilog supports
//! both. The fix parses a single NO-EDGE primary+postfix reference after a bare
//! `@`, equivalent to `@(<ref>)`.
//!
//! Cardinal property: a bare `@X` is byte-for-byte equivalent to the
//! parenthesized `@(X)` — for whole-signal/event refs it now SIMULATES (matching
//! iverilog), and for refs whose UNDERLYING feature vita does not yet support
//! (a single-bit level `@a[2]`, a hierarchical event ref `@u.s`) it routes to the
//! SAME loud reject as `@(a[2])` / `@(u.s)`. So `@X` never diverges from `@(X)`;
//! the bare form only adds syntax, never semantics. The supported cases are
//! pinned to iverilog 13.0; the loud/consistency cases compare bare vs paren.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_bevt_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

#[test]
fn bare_event_wait_matches_iverilog() {
    // `@e` on a named event wakes the waiter at the trigger time (iverilog: woke@1).
    let (out, code) = run(
        "module top; event e; initial begin #1 ->e; #5 $finish; end\n\
         initial begin @e $display(\"woke@%0t\", $time); end endmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(
        out.contains("woke@1"),
        "bare @e should wake at t=1; got:\n{out}"
    );
}

#[test]
fn always_bare_signal_matches_iverilog() {
    // `always @clk` (no parens) fires on any change of the whole signal (n=2 for
    // two edges), matching iverilog.
    let (out, code) = run("module top; reg clk=0; integer n=0; always @clk n=n+1;\n\
         initial begin #1 clk=1; #1 clk=0; #1 $display(\"n=%0d\", n); $finish; end endmodule\n");
    assert_eq!(code, Some(0));
    assert!(
        out.contains("n=2"),
        "always @clk should count 2 edges; got:\n{out}"
    );
}

#[test]
fn bare_whole_multibit_signal_wait_matches_iverilog() {
    // A bare `@bus` on a multi-bit whole net fires on any change.
    let (out, code) = run(
        "module top; reg [7:0] bus=0; reg done=0;\n\
         initial begin #1 bus=8'hAB; end\n\
         initial begin @bus done=1; $display(\"d=%0d bus=%h\", done, bus); $finish; end endmodule\n",
    );
    assert_eq!(code, Some(0));
    assert!(
        out.contains("d=1 bus=ab"),
        "bare @bus any-change; got:\n{out}"
    );
}

#[test]
fn bare_event_equals_paren_event() {
    // The cardinal equivalence: a bare `@e` produces byte-identical output to the
    // parenthesized `@(e)` (same stdout, same exit code).
    let prog = |sens: &str| {
        format!(
            "module top; event e; integer n=0;\n\
             initial begin #1 ->e; #1 ->e; #5 $finish; end\n\
             initial forever begin {sens} n=n+1; $display(\"n=%0d@%0t\", n, $time); end endmodule\n"
        )
    };
    let bare = run(&prog("@e"));
    let paren = run(&prog("@(e)"));
    assert_eq!(bare.1, Some(0));
    assert_eq!(bare, paren, "bare @e must equal paren @(e)");
    assert!(bare.0.contains("n=1@1") && bare.0.contains("n=2@2"));
}

#[test]
fn bare_edge_stays_loud() {
    // A bare edge `@posedge clk` (no parens) is illegal — edges require parens.
    // Must stay a loud error (iverilog also rejects it), never silently accepted.
    let (_o, code) = run(
        "module top; reg clk=0; integer n=0; initial begin #1 clk=1; #2 $finish; end\n\
         initial forever @posedge clk n=n+1; endmodule\n",
    );
    assert_ne!(code, Some(0), "bare @posedge must stay loud");
}

#[test]
fn bare_binary_event_stays_loud() {
    // A bare `@a+b` is not a hierarchical_identifier — the reference parse stops
    // after `a`, and the trailing `+b` is a loud statement error (iverilog rejects
    // `@a+b`). Never silently treated as `@(a+b)`.
    let (_o, code) = run(
        "module top; reg a=0,b=0,x=0; initial begin #1 a=1; #1 $finish; end\n\
         initial @a+b x=1; endmodule\n",
    );
    assert_ne!(code, Some(0), "bare @a+b must stay loud");
}

#[test]
fn bare_bitsel_event_equals_paren_bitsel() {
    // A single-bit level event `@a[2]` is a pre-existing unsupported feature (vita
    // loud-rejects it in BOTH forms). The bare form must route to the SAME loud
    // reject as the paren form — bare never diverges from paren.
    let prog = |sens: &str| {
        format!(
            "module top; reg [3:0] a=0; reg x=0; initial begin #1 a[2]=1; #1 $finish; end\n\
             initial {sens} x=1; initial begin #2 $display(\"x=%0d\", x); end endmodule\n"
        )
    };
    let bare = run(&prog("@a[2]"));
    let paren = run(&prog("@(a[2])"));
    assert_eq!(
        bare, paren,
        "bare @a[2] must equal paren @(a[2]) (both loud)"
    );
    assert_ne!(
        bare.1,
        Some(0),
        "single-bit level event is loud in both forms"
    );
}

#[test]
fn bare_call_event_equals_paren_call() {
    // `@f(x)` over-reaches at the parser (`expr_postfix` consumes the call), but
    // elaborate loud-rejects a call as an event control — exactly as `@(f(x))`
    // does. iverilog also rejects `@f(x)`. Confirms bare==paren for the call form
    // (both loud, never a silent simulation).
    let prog = |sens: &str| {
        format!(
            "module top; function automatic integer f(integer x); f=x; endfunction reg a=0;\n\
             always {sens} a=~a; initial #1 $finish; endmodule\n"
        )
    };
    let bare = run(&prog("@f(1)"));
    let paren = run(&prog("@(f(1))"));
    assert_eq!(
        bare, paren,
        "bare @f(1) must equal paren @(f(1)) (both loud)"
    );
    assert_ne!(
        bare.1,
        Some(0),
        "a call event control is loud in both forms"
    );
}

#[test]
fn bare_hier_event_equals_paren_hier() {
    // A hierarchical event ref `@u.s` is a pre-existing unsupported feature (loud
    // in BOTH forms). Bare must equal paren.
    let prog = |sens: &str| {
        format!(
            "module sub; reg s=0; initial #1 s=1; endmodule\n\
             module top; sub u(); reg x=0;\n\
             initial {sens} begin x=1; $display(\"x=%0d\", x); $finish; end endmodule\n"
        )
    };
    let bare = run(&prog("@u.s"));
    let paren = run(&prog("@(u.s)"));
    assert_eq!(bare, paren, "bare @u.s must equal paren @(u.s)");
}
