//! Intra-assignment event control on a BLOCKING assignment (IEEE 1800 §9.4.5):
//! `lhs = @(event) rhs;` and `lhs = repeat(n) @(event) rhs;`. The RHS is evaluated
//! (captured) NOW, the process then waits for the event (n times for `repeat(n)`),
//! and only THEN is the captured value written to `lhs`. Lowered as a desugar:
//!   tmp = rhs;  repeat(n) @(event);  lhs = tmp;
//! reusing the existing capture-temp + event-wait machinery. The `delay`/sim-ir
//! golden is unchanged; the AST gains an `event` field on `Stmt::Blocking`, so the
//! `.vu` AST-hash re-pins once.
//!
//! iverilog 13.0 SUPPORTS this (it is a real differential oracle for this slice).
//! NON-blocking `<= @(ev)` / `<= repeat(n) @(ev)` event control stays an advisory
//! (parse-and-discard) — out of scope here.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_iae_{}_{n}", std::process::id()));
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
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

#[test]
fn repeat_event_captures_rhs_now_writes_after_n_events() {
    // b=11 captured at t=0; b changes to 99 at t=7 (during the wait). After 2
    // posedges (t=5, t=15) the captured 11 — NOT 99 — is written. Proves capture-now.
    let (out, err, code) = run("module top;\n\
         reg clk=0; reg [7:0] a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial begin\n\
           b = 8'd11;\n\
           a = repeat(2) @(posedge clk) b;\n\
           if (a === 8'd11) $display(\"OK a=%0d\", a); else $display(\"BAD a=%0d\", a);\n\
           $finish;\n\
         end\n\
         initial #7 b = 8'd99;\n\
         endmodule\n");
    assert_eq!(code, Some(0), "must run clean. stderr:\n{err}\nout:\n{out}");
    assert!(
        out.contains("OK a=11"),
        "captured RHS (11) must be written, not the later 99. out:\n{out}\nerr:\n{err}"
    );
}

#[test]
fn repeat_event_waits_exactly_n_events_before_write() {
    // N=2: at t=12 (after the 1st posedge t=5, before the 2nd t=15) `a` must still be
    // its old 0; after t=15 it becomes 7. Proves it waits for N=2 events, not 1.
    let (out, err, code) = run("module top;\n\
         reg clk=0; reg [7:0] a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial begin b = 8'd7; a = repeat(2) @(posedge clk) b; end\n\
         initial begin\n\
           #12; if (a === 8'd0) $display(\"PENDING a=%0d\", a); else $display(\"EARLY a=%0d\", a);\n\
           #20; if (a === 8'd7) $display(\"DONE a=%0d\", a); else $display(\"WRONG a=%0d\", a);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "must run clean. stderr:\n{err}\nout:\n{out}");
    assert!(
        out.contains("PENDING a=0") && out.contains("DONE a=7"),
        "a must stay 0 until the 2nd posedge, then become 7. out:\n{out}\nerr:\n{err}"
    );
}

#[test]
fn plain_event_intra_assign_waits_one_event() {
    // `= @(posedge clk) b` (no repeat) waits exactly one posedge, then writes.
    let (out, err, code) = run("module top;\n\
         reg clk=0; reg [7:0] a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial begin\n\
           b = 8'd42;\n\
           a = @(posedge clk) b;\n\
           if (a === 8'd42) $display(\"OK a=%0d\", a); else $display(\"BAD a=%0d\", a);\n\
           $finish;\n\
         end\n\
         initial #2 b = 8'd5;\n\
         endmodule\n");
    assert_eq!(code, Some(0), "must run clean. stderr:\n{err}\nout:\n{out}");
    assert!(
        out.contains("OK a=42"),
        "plain @(ev) intra-assign must capture-now (42) and write after 1 event. out:\n{out}\nerr:\n{err}"
    );
}

#[test]
fn repeat_event_parameter_count_waits() {
    // REVIEW fix (count-folding, 2026-06-16): a `parameter`/`localparam` count is a
    // constant and must wait that many events — it was silently eliding the wait
    // (routed through scope-blind const_eval). N=2: a stays 0 until the 2nd posedge.
    let (out, err, code) = run("module top;\n\
         parameter N = 2;\n\
         reg clk=0; reg [7:0] a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial begin b = 8'd7; a = repeat(N) @(posedge clk) b; end\n\
         initial begin\n\
           #2;  if (a === 8'd0) $display(\"T2 a=%0d\", a); else $display(\"EARLY a=%0d\", a);\n\
           #20; if (a === 8'd7) $display(\"DONE a=%0d\", a); else $display(\"WRONG a=%0d\", a);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "must run clean. stderr:\n{err}\nout:\n{out}");
    assert!(
        out.contains("T2 a=0") && out.contains("DONE a=7"),
        "parameter count N=2 must wait 2 posedges (a=0 at t=2, 7 after t=15). out:\n{out}\nerr:\n{err}"
    );
}

#[test]
fn repeat_event_runtime_count_is_loud() {
    // A genuinely runtime (non-constant) count cannot be unrolled → loud, NOT a
    // silent 0-event write with the wrong timing.
    let (_o, err, code) = run("module top;\n\
         reg clk=0; reg [7:0] a=0, b=0; integer n;\n\
         always #5 clk=~clk;\n\
         initial begin n = 2; b = 8'd7; a = repeat(n) @(posedge clk) b; $display(\"a=%0d\", a); $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "a runtime repeat count in an intra-assign event control must be loud. {err}"
    );
    assert!(err.contains("VITA-E"), "{err}");
}

#[test]
fn repeat_event_zero_count_writes_immediately() {
    // repeat(0) → zero iterations → the captured value is written immediately
    // (IEEE; matches iverilog). b captured at t=0, a=b right away.
    let (out, err, code) = run("module top;\n\
         reg clk=0; reg [7:0] a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial begin b = 8'd3; a = repeat(0) @(posedge clk) b; $display(\"t=%0t a=%0d\", $time, a); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "must run clean. stderr:\n{err}\nout:\n{out}");
    assert!(
        out.contains("t=0 a=3"),
        "repeat(0) writes the captured value immediately (t=0, a=3). out:\n{out}\nerr:\n{err}"
    );
}

#[test]
fn repeat_event_runs_deterministically() {
    let src = "module top;\n\
         reg clk=0; reg [7:0] a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial begin b = 8'd9; a = repeat(3) @(posedge clk) b; $display(\"a=%0d\", a); $finish; end\n\
         endmodule\n";
    let (o1, e1, c1) = run(src);
    let (o2, e2, c2) = run(src);
    assert_eq!((o1, e1, c1), (o2, e2, c2), "must be deterministic");
}

// ── Regressions for two latent bugs the N1 adversarial review surfaced in the
//    SHARED capture-temp machinery (they affected this BLOCKING form too). ──

#[test]
fn xz_constant_count_writes_immediately() {
    // REVIEW fix (2026-06-17): an X/Z-bearing CONSTANT count (`2'bx1`) is a compile-time
    // constant ⇒ 0 iterations (IEEE) ⇒ immediate blocking write, NOT a loud "runtime
    // count" error. iverilog: a=11 at t=0.
    let (out, err, code) = run("module top;\n\
         reg clk=0; reg [7:0] a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial begin b = 8'd11; a = repeat(2'bx1) @(posedge clk) b; $display(\"t=%0t a=%0d\", $time, a); $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "X/Z constant count must NOT be loud. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        out.contains("t=0 a=11"),
        "X/Z constant count ⇒ 0 iterations ⇒ immediate write (a=11 at t=0). out:\n{out}\nerr:\n{err}"
    );
}

#[test]
fn const_part_select_lhs_writes_full_width() {
    // REVIEW fix (2026-06-17): a constant part-select `a[7:4]` lvalue must capture the
    // RHS at the part-select width (4), not 1. Affected both the event form (here) and
    // the `= #d` form below. iverilog: a=11110000.
    let (out, err, code) = run("module t;\n\
         reg [7:0] a; reg clk=0;\n\
         always #5 clk=~clk;\n\
         initial begin a=0; a[7:4] = @(posedge clk) 4'hF; $display(\"a=%b\", a); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "must run clean. stderr:\n{err}\nout:\n{out}");
    assert!(
        out.contains("a=11110000"),
        "constant part-select lhs must write all 4 bits, not just the LSB. out:\n{out}\nerr:\n{err}"
    );
}

#[test]
fn delay_const_part_select_lhs_writes_full_width() {
    // The `= #d` intra-assignment delay shares the same capture-temp sizing path.
    // a[5:2] = #1 4'hF ⇒ a=00111100 (was 00000100 — only the LSB). iverilog: 00111100.
    let (out, err, code) = run("module t;\n\
         reg [7:0] a;\n\
         initial begin a=0; a[5:2] = #1 4'hF; $display(\"a=%b\", a); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "must run clean. stderr:\n{err}\nout:\n{out}");
    assert!(
        out.contains("a=00111100"),
        "constant part-select lhs with `#d` must write all 4 bits. out:\n{out}\nerr:\n{err}"
    );
}
