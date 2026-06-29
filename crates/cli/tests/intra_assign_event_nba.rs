//! Intra-assignment event control on a NON-BLOCKING assignment (IEEE 1800 §9.4.5):
//! `lhs <= @(event) rhs;` and `lhs <= repeat(n) @(event) rhs;` (slice N1 — the twin
//! of the blocking form in `intra_assign_event.rs`). The RHS *and any LHS index* are
//! captured NOW; unlike the blocking form, the process does NOT block — it continues
//! to the next statement immediately while a detached helper waits for the event (n
//! times) and then performs the NBA write of the captured value.
//!
//! Lowered as a `fork … join_none` desugar — `$tmp = rhs;` (plus `$idx = <lhs
//! index>;` per dynamic index) then `fork begin repeat(n) @(ev); lhs' <= $tmp; end
//! join_none` — reusing the existing capture-temp + EventCtrl + fork machinery. The
//! sim-ir golden + `format_version` are UNCHANGED (pure IR-0); the AST gains an
//! `event` field on `Stmt::NonBlocking`, so the `.vu` AST-hash re-pins once.
//!
//! iverilog 13.0 SUPPORTS this — every expected value/timing below is a differential
//! oracle captured from iverilog. Documented DIVERGENCES (hand-IEEE pins, like prior
//! slices):
//!  1. A genuinely runtime (non-constant) count is LOUD here (iverilog accepts it).
//!     (An X/Z-bearing CONSTANT count is NOT runtime — it is 0 iterations per IEEE;
//!     see `nba_xz_constant_count_is_zero_iterations`.)
//!  2. Same-site self-overlapping in-flight captures share the per-site temp
//!     (iverilog carries an independent value per in-flight assignment).
//!  3. SAME-TICK region tie (the project's documented "동시-틱 tie = 도구-발산" zone):
//!     at the tick the helper performs its NBA write, a process triggered by the SAME
//!     edge that reads the lhs in the Active / `#0`-Inactive region sees the OLD value
//!     under vita (LRM-faithful: an NBA is not visible in the write tick's active
//!     region) but the NEW value under iverilog (which applies its native intra-NBA
//!     earlier). The committed value is identical: `$strobe`/postponed, every later
//!     tick, and the final value all match iverilog exactly. Not fixable by the IR-0
//!     fork+NBA desugar (a helper resumed by an edge schedules its NBA one region
//!     behind that edge's concurrent readers); a faithful match would need an engine
//!     event-armed-NBA mechanism. The synthetic `$ia_tmp`/`$idx` capture nets also
//!     appear in VCD (cosmetic, pre-existing — shared with the blocking form).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_iaenba_{}_{n}", std::process::id()));
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
fn nba_event_parent_does_not_block() {
    // The defining NBA property: `a <= @(posedge clk) b` must NOT block the process.
    // `done = 1` and the $display run at t=0 (a still 0, write pending); a becomes the
    // captured 11 only after the posedge. iverilog: "t=0 done=1 a=0" / "final a=11".
    let (out, err, code) = run("module top;\n\
         reg clk=0; reg [7:0] a=0,b=0; reg done=0;\n\
         always #5 clk=~clk;\n\
         initial begin\n\
           b = 8'd11;\n\
           a <= @(posedge clk) b;\n\
           done = 1;\n\
           $display(\"t=%0t done=%0d a=%0d\", $time, done, a);\n\
           #100 $display(\"t=%0t final a=%0d\", $time, a);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "must run clean. stderr:\n{err}\nout:\n{out}");
    assert!(
        out.contains("t=0 done=1 a=0"),
        "parent must NOT block: done=1 and a=0 at t=0. out:\n{out}\nerr:\n{err}"
    );
    assert!(
        out.contains("final a=11"),
        "after the posedge the captured 11 must be written. out:\n{out}\nerr:\n{err}"
    );
}

#[test]
fn nba_repeat_captures_rhs_now() {
    // b=11 captured at t=0; b changes to 99 at t=7 (during the wait). After 2 posedges
    // (t=5,15) the captured 11 — NOT 99 — is the NBA value. iverilog: "final a=11".
    let (out, err, code) = run("module top;\n\
         reg clk=0; reg [7:0] a=0,b=0;\n\
         always #5 clk=~clk;\n\
         initial begin b = 8'd11; a <= repeat(2) @(posedge clk) b; end\n\
         initial #7 b = 8'd99;\n\
         initial begin #100 $display(\"final a=%0d\", a); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "must run clean. stderr:\n{err}\nout:\n{out}");
    assert!(
        out.contains("final a=11"),
        "captured RHS (11) must be written, not the later 99. out:\n{out}\nerr:\n{err}"
    );
}

#[test]
fn nba_repeat_waits_exactly_n_events() {
    // N=2: at t=12 (after 1st posedge t=5, before 2nd t=15) a must still be 0; after
    // t=15 it becomes 7. iverilog: "t12 a=0" / "t32 a=7".
    let (out, err, code) = run("module top;\n\
         reg clk=0; reg [7:0] a=0,b=0;\n\
         always #5 clk=~clk;\n\
         initial begin b = 8'd7; a <= repeat(2) @(posedge clk) b; end\n\
         initial begin\n\
           #12 $display(\"t12 a=%0d\", a);\n\
           #20 $display(\"t32 a=%0d\", a);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "must run clean. stderr:\n{err}\nout:\n{out}");
    assert!(
        out.contains("t12 a=0") && out.contains("t32 a=7"),
        "a must stay 0 until the 2nd posedge, then become 7. out:\n{out}\nerr:\n{err}"
    );
}

#[test]
fn nba_plain_event_one_occurrence() {
    // `<= @(posedge clk) b` (no repeat) waits exactly one posedge. b captured at t=0
    // (=42); changed to 5 at t=2; written 42 at t=5.
    let (out, err, code) = run("module top;\n\
         reg clk=0; reg [7:0] a=0,b=0;\n\
         always #5 clk=~clk;\n\
         initial begin b = 8'd42; a <= @(posedge clk) b; end\n\
         initial #2 b = 8'd5;\n\
         initial begin #100 $display(\"a=%0d\", a); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "must run clean. stderr:\n{err}\nout:\n{out}");
    assert!(
        out.contains("a=42"),
        "plain @(ev) NBA must capture-now (42) and write after 1 event. out:\n{out}\nerr:\n{err}"
    );
}

#[test]
fn nba_repeat_zero_writes_via_nba_region() {
    // repeat(0) → no wait → a plain NBA: the captured value joins the NBA region of
    // the current tick. So a is still 0 in the same active region (t0), and 3 by the
    // next tick. iverilog: "t0 a=0" / "t1 a=3".
    let (out, err, code) = run("module top;\n\
         reg clk=0; reg [7:0] a=0,b=0;\n\
         always #5 clk=~clk;\n\
         initial begin b = 8'd3; a <= repeat(0) @(posedge clk) b; $display(\"t0 a=%0d\", a); end\n\
         initial begin #1 $display(\"t1 a=%0d\", a); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "must run clean. stderr:\n{err}\nout:\n{out}");
    assert!(
        out.contains("t0 a=0") && out.contains("t1 a=3"),
        "repeat(0) degenerates to a plain NBA (a=0 same tick, 3 next). out:\n{out}\nerr:\n{err}"
    );
}

#[test]
fn nba_lhs_index_sampled_now() {
    // The LHS index is sampled NOW, not at write time (asymmetric with the blocking
    // form, which samples it at write time). i=0 at the statement; changed to 2 before
    // the posedge; the captured d=170 lands in mem[0], NOT mem[2]. iverilog:
    // "mem0=170 mem2=0".
    let (out, err, code) = run("module top;\n\
         reg clk=0; reg [7:0] mem[0:3]; reg [1:0] i; reg [7:0] d; integer k;\n\
         always #5 clk=~clk;\n\
         initial begin\n\
           for (k=0;k<4;k=k+1) mem[k]=0;\n\
           i = 0; d = 8'hAA;\n\
           mem[i] <= @(posedge clk) d;\n\
           #1 i = 2;\n\
         end\n\
         initial begin #100 $display(\"mem0=%0d mem2=%0d\", mem[0], mem[2]); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "must run clean. stderr:\n{err}\nout:\n{out}");
    assert!(
        out.contains("mem0=170 mem2=0"),
        "LHS index sampled NOW: captured value lands in mem[0]. out:\n{out}\nerr:\n{err}"
    );
}

#[test]
fn nba_negedge_and_parameter_counts() {
    // negedge edge + parameter / localparam counts. N=3 posedges (t=5,15,25) and M=2
    // negedges (t=10,20) → both write 7. iverilog: "a=7 b=7".
    let (out, err, code) = run("module top;\n\
         parameter N=3; localparam M=2;\n\
         reg clk=0; reg [7:0] a=0,b=0,d=8'h7;\n\
         always #5 clk=~clk;\n\
         initial begin a <= repeat(N) @(posedge clk) d; b <= repeat(M) @(negedge clk) d; end\n\
         initial begin #100 $display(\"a=%0d b=%0d\", a, b); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "must run clean. stderr:\n{err}\nout:\n{out}");
    assert!(
        out.contains("a=7 b=7"),
        "negedge + param/localparam counts must wait and write 7. out:\n{out}\nerr:\n{err}"
    );
}

#[test]
fn nba_repeated_spawn_from_always() {
    // A clocked always re-fires the NBA-event each posedge → a fresh detached helper
    // each time (exercises the fork free-list + per-fire capture-now). The capture
    // reads d's pre-NBA active-region value. iverilog: "t=58 q=4 d=6".
    let (out, err, code) = run("module top;\n\
         reg clk=0; reg [7:0] d=0; reg [7:0] q=0;\n\
         always #5 clk=~clk;\n\
         always @(posedge clk) d <= d + 8'd1;\n\
         always @(posedge clk) q <= @(negedge clk) d;\n\
         initial begin #58 $display(\"t=%0t q=%0d d=%0d\", $time, q, d); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "must run clean. stderr:\n{err}\nout:\n{out}");
    assert!(
        out.contains("t=58 q=4 d=6"),
        "repeated spawn must reproduce the iverilog trace. out:\n{out}\nerr:\n{err}"
    );
}

#[test]
fn nba_runtime_count_is_loud() {
    // A genuinely runtime (non-constant) count cannot be unrolled → LOUD (E3009),
    // never a silent wrong-timing write. DIVERGENCE: iverilog accepts a runtime count;
    // we reject it (matching the blocking-form precedent).
    let (_o, err, code) = run("module top;\n\
         reg clk=0; reg [7:0] a=0,b=0; integer n;\n\
         always #5 clk=~clk;\n\
         initial begin n = 2; b = 8'd7; a <= repeat(n) @(posedge clk) b; $display(\"a=%0d\", a); $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(1),
        "a runtime NBA-event repeat count must be loud. {err}"
    );
    assert!(err.contains("VITA-E"), "{err}");
}

#[test]
fn nba_xz_constant_count_is_zero_iterations() {
    // REVIEW fix (count X/Z, 2026-06-17): an X/Z-bearing CONSTANT count (`2'bx1`) is a
    // compile-time constant, not a runtime value — IEEE evaluates it as 0 iterations,
    // so it degenerates to a plain same-tick NBA (NOT a loud "runtime count" error).
    // b=11 captured at t=0, written via NBA, visible at t=1 BEFORE the first posedge.
    let (out, err, code) = run("module top;\n\
         reg clk=0; reg [7:0] a=0,b=0;\n\
         always #5 clk=~clk;\n\
         initial begin b=8'd11; a <= repeat(2'bx1) @(posedge clk) b; b=8'd99; end\n\
         initial begin #1 $display(\"t1 a=%0d\", a); #100 $display(\"final a=%0d\", a); $finish; end\n\
         endmodule\n");
    assert_eq!(
        code,
        Some(0),
        "X/Z constant count must NOT be loud. stderr:\n{err}\nout:\n{out}"
    );
    assert!(
        out.contains("t1 a=11") && out.contains("final a=11"),
        "X/Z constant count ⇒ 0 iterations ⇒ same-tick NBA (a=11 by t=1). out:\n{out}\nerr:\n{err}"
    );
}

#[test]
fn nba_const_part_select_lhs_writes_full_width() {
    // REVIEW fix (part-select width, 2026-06-17): a constant part-select `a[7:4]` lvalue
    // must capture the RHS at the part-select's width (4), not 1 — it was sizing the
    // capture temp at width 1 and dropping all but the LSB. iverilog: a=11110000.
    let (out, err, code) = run("module t;\n\
         reg [7:0] a; reg clk;\n\
         initial begin clk=0; a=0; a[7:4] <= @(posedge clk) 4'hF; #5 clk=1; #5 $display(\"a=%b\", a); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "must run clean. stderr:\n{err}\nout:\n{out}");
    assert!(
        out.contains("a=11110000"),
        "constant part-select lhs must write all 4 bits, not just the LSB. out:\n{out}\nerr:\n{err}"
    );
}

#[test]
fn nba_wide_const_part_select_lhs_writes_full_width() {
    // Wider (>32-bit) variant of the part-select width fix. a[40:8] is a 33-bit slice;
    // 33'h1FFFFFFFF lands as bits 40..8. iverilog: a=000001ffffffff00.
    let (out, err, code) = run("module t;\n\
         reg [63:0] a; reg clk;\n\
         initial begin clk=0; a=0; a[40:8] <= @(posedge clk) 33'h1FFFFFFFF; #5 clk=1; #5 $display(\"a=%h\", a); $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "must run clean. stderr:\n{err}\nout:\n{out}");
    assert!(
        out.contains("a=000001ffffffff00"),
        "wide constant part-select lhs must write all 33 bits. out:\n{out}\nerr:\n{err}"
    );
}

#[test]
fn nba_event_runs_deterministically() {
    let src = "module top;\n\
         reg clk=0; reg [7:0] a=0,b=0;\n\
         always #5 clk=~clk;\n\
         initial begin b = 8'd9; a <= repeat(3) @(posedge clk) b; end\n\
         initial begin #100 $display(\"a=%0d\", a); $finish; end\n\
         endmodule\n";
    let (o1, e1, c1) = run(src);
    let (o2, e2, c2) = run(src);
    assert_eq!((o1, e1, c1), (o2, e2, c2), "must be deterministic");
}
