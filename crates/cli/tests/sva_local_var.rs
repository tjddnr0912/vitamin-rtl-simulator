//! SVA sequence/property LOCAL VARIABLE single-capture subset (slice N2c, IEEE
//! 1800-2017 §16.10), via a PARALLEL DATA SHIFT REGISTER. The data-tracking idiom
//! `(req, d=data) ##1 grant |-> (rdata == d)` captures a value at one term and reads
//! it at a later term/consequent within the SAME match attempt.
//!
//! iverilog 13.0 does NOT support concurrent assertions or local variables, so the
//! oracle is HAND-IEEE (clock-count + textual substitution): `clk` toggles every 5ns
//! from t=5, so posedges land at t=10, 20, 30, 40, 50. A stimulus `#10 sig=v` sets
//! `sig` at a posedge instant; the clocked checker samples the NEW value (the
//! existing flat-SVA convention — the checker's `always @(posedge clk)` runs after
//! the initial block's blocking assigns at that timestep).
//!
//! CORRECTNESS (why a single register per var, no thread table): the liveness
//! pipeline is a SHIFT register — stage k is the attempt that started k clocks ago.
//! The seed boolean fires at most once per clock, so each stage holds at most one
//! attempt. A parallel data register shifted in lockstep carries the captured value
//! with NO collision between pipelined attempts (different time-stages of one
//! register). The ONLY collision is CONVERGENCE: a RANGED delay lets two attempts
//! reach one stage via different paths → a data collision → LOUD.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_svalv_{}_{n}", std::process::id()));
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

fn fires(out: &str, err: &str, code: Option<i32>, ctx: &str) {
    assert_eq!(
        code,
        Some(1),
        "{ctx}: expected a violation (exit 1).\nstderr:\n{err}\nout:\n{out}"
    );
    assert!(
        format!("{err}{out}").to_lowercase().contains("assertion"),
        "{ctx}: a violation diagnostic was expected.\nstderr:\n{err}\nout:\n{out}"
    );
}

fn holds(out: &str, err: &str, code: Option<i32>, ctx: &str) {
    assert_eq!(
        code,
        Some(0),
        "{ctx}: expected a clean pass (exit 0).\nstderr:\n{err}\nout:\n{out}"
    );
    assert!(
        !format!("{err}{out}").to_lowercase().contains("assertion"),
        "{ctx}: no violation expected.\nstderr:\n{err}\nout:\n{out}"
    );
}

fn loud(out: &str, err: &str, code: Option<i32>, ctx: &str) {
    assert_ne!(code, Some(0), "{ctx}: must not silently pass.\n{err}{out}");
    assert!(
        format!("{err}{out}").contains("VITA-E"),
        "{ctx}: expected a loud diagnostic.\nstderr:\n{err}\nout:\n{out}"
    );
}

// ── CAPTURE+READ across a `##1` hop (the canonical idiom) ──────────────────────

#[test]
fn capture_read_fires_on_mismatch() {
    // `(req, d=data) ##1 grant |-> (rdata == d)`.
    // t10: req=1, data=5 → first attempt captures 5 (data reg `<= 5`).
    // t20: req=0, grant=1, rdata=7 → ante fires (req@t10 & grant@t20), consequent
    //      reads the captured d (=5, the data reg's prior-clock value) → `7 == 5` is
    //      FALSE → FIRE at t20.
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (req, d=data) ##1 grant |-> (rdata == d));\n\
         initial begin\n\
           #10 req=1; data=5;\n\
           #10 req=0; grant=1; rdata=7;\n\
           #10 grant=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(
        &out,
        &err,
        code,
        "(req,d=data) ##1 grant |-> rdata==d, rdata=7 != captured 5",
    );
}

#[test]
fn capture_read_holds_on_match() {
    // Same, but rdata=5 (== the captured d) at the completion clock → clean.
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (req, d=data) ##1 grant |-> (rdata == d));\n\
         initial begin\n\
           #10 req=1; data=5;\n\
           #10 req=0; grant=1; rdata=5;\n\
           #10 grant=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    holds(
        &out,
        &err,
        code,
        "(req,d=data) ##1 grant |-> rdata==d, rdata=5 == captured 5",
    );
}

// ── BACK-TO-BACK (overlap correctness — the load-bearing teeth) ────────────────

#[test]
fn back_to_back_each_attempt_reads_own_capture() {
    // Two outstanding requests pipelined one clock apart; each grant must read its
    // OWN captured data (no clobber between concurrent attempts).
    //   t10: req=1, data=5            → attempt A captures 5 (data reg <= 5).
    //   t20: req=1, data=9, grant=1   → A completes (grant@t20), reads data reg
    //        = 5 (A's value, the prior-clock reg read); B captures 9 (data reg <= 9).
    //        rdata=5 → A HOLDS.
    //   t30: req=0,         grant=1   → B completes (req@t20 & grant@t30), reads data
    //        reg = 9 (B's value). rdata=9 → B HOLDS.
    // Both attempts read their own value → clean. (Proves the shift register pipelines
    // the captures with no collision.)
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (req, d=data) ##1 grant |-> (rdata == d));\n\
         initial begin\n\
           #10 req=1; data=5;\n\
           #10 req=1; data=9; grant=1; rdata=5;\n\
           #10 req=0;          grant=1; rdata=9;\n\
           #10 grant=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    holds(
        &out,
        &err,
        code,
        "back-to-back: A reads 5 @t20, B reads 9 @t30",
    );
}

#[test]
fn back_to_back_swapped_data_fires() {
    // The TEETH: if the data were SHARED (clobbered) rather than per-attempt, swapping
    // the expected rdata between the two completions would still pass. It must NOT:
    //   t20: rdata=9 — but A captured 5 → `9 == 5` FALSE → FIRE at t20.
    // (A real silent-wrong shift register that let B's 9 leak onto A's stage would
    // wrongly HOLD here.)
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (req, d=data) ##1 grant |-> (rdata == d));\n\
         initial begin\n\
           #10 req=1; data=5;\n\
           #10 req=1; data=9; grant=1; rdata=9;\n\
           #10 req=0;          grant=1; rdata=5;\n\
           #10 grant=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(
        &out,
        &err,
        code,
        "back-to-back swapped: A's stage must hold 5, not B's 9",
    );
}

// ── SINGLE-TERM OVERLAP (capture and read at the SAME clock, zero shifts) ──────

#[test]
fn single_term_overlap_same_clock_holds() {
    // `(req, d=data) |-> (rdata == d)`: capture and read at the same clock. The data
    // read is the COMBINATIONAL captured value (zero shift stages) → `rdata == data`
    // this clock. t10: req=1, data=5, rdata=5 → HOLD.
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (req, d=data) |-> (rdata == d));\n\
         initial begin\n\
           #10 req=1; data=5; rdata=5;\n\
           #10 req=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    holds(
        &out,
        &err,
        code,
        "(req,d=data) |-> rdata==d same-clock, rdata=5==5",
    );
}

#[test]
fn single_term_overlap_same_clock_fires() {
    // Same, but rdata=7 != data=5 at the capture clock → FIRE at t10.
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (req, d=data) |-> (rdata == d));\n\
         initial begin\n\
           #10 req=1; data=5; rdata=7;\n\
           #10 req=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(
        &out,
        &err,
        code,
        "(req,d=data) |-> rdata==d same-clock, rdata=7!=5",
    );
}

// ── NON-OVERLAP `|=>` (consequent one clock after antecedent completion) ───────

#[test]
fn nonoverlap_capture_read_holds() {
    // `(req, d=data) |=> (rdata == d)`: capture at t10, consequent checked at t20.
    // The data survives one extra clock (the `|=>` shift) → reads data@t10 = 5 at t20.
    // rdata=5 @t20 → HOLD.
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (req, d=data) |=> (rdata == d));\n\
         initial begin\n\
           #10 req=1; data=5;\n\
           #10 req=0; rdata=5;\n\
           #10 rdata=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    holds(
        &out,
        &err,
        code,
        "(req,d=data) |=> rdata==d, rdata=5 @t20 == captured 5",
    );
}

#[test]
fn nonoverlap_capture_read_fires() {
    // Same, but rdata=7 @t20 != captured 5 → FIRE at t20.
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (req, d=data) |=> (rdata == d));\n\
         initial begin\n\
           #10 req=1; data=5;\n\
           #10 req=0; rdata=7;\n\
           #10 rdata=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(
        &out,
        &err,
        code,
        "(req,d=data) |=> rdata==d, rdata=7 @t20 != captured 5",
    );
}

// ── A LONGER fixed delay `##2` (two shift stages) ──────────────────────────────

#[test]
fn capture_read_two_hop_holds() {
    // `(req, d=data) ##2 grant |-> (rdata == d)`: capture@t10, completion@t30 (two
    // shifts). The data reg chain (2 stages) delivers data@t10 = 5 at t30.
    //   t10: req=1, data=5; t30: grant=1, rdata=5 → HOLD.
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (req, d=data) ##2 grant |-> (rdata == d));\n\
         initial begin\n\
           #10 req=1; data=5;\n\
           #10 req=0;\n\
           #10 grant=1; rdata=5;\n\
           #10 grant=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    holds(
        &out,
        &err,
        code,
        "(req,d=data) ##2 grant |-> rdata==d, rdata=5 @t30",
    );
}

#[test]
fn capture_read_two_hop_fires() {
    // Same, but rdata=7 @t30 != captured 5 → FIRE.
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (req, d=data) ##2 grant |-> (rdata == d));\n\
         initial begin\n\
           #10 req=1; data=5;\n\
           #10 req=0;\n\
           #10 grant=1; rdata=7;\n\
           #10 grant=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(
        &out,
        &err,
        code,
        "(req,d=data) ##2 grant |-> rdata==d, rdata=7 @t30 != 5",
    );
}

// ── after-clock decl placement (IEEE §16.10 property_spec) ─────────────────────

#[test]
fn decl_after_clock_parses_and_fires() {
    // `@(posedge clk) int d; (req, d=data) ##1 grant |-> …` — the local-var decl
    // FOLLOWS the clocking event (the §16.10 property_spec position). Same behavior.
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (req, d=data) ##1 grant |-> (rdata == d));\n\
         initial begin\n\
           #10 req=1; data=5;\n\
           #10 req=0; grant=1; rdata=7;\n\
           #10 grant=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    fires(&out, &err, code, "decl-after-clock placement");
}

#[test]
fn named_property_local_var_holds() {
    // A NAMED property with a body-start local var (`property p; int x; @(clk) …`).
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         property p; int d; @(posedge clk) (req, d=data) ##1 grant |-> (rdata == d); endproperty\n\
         initial assert property(p);\n\
         initial begin\n\
           #10 req=1; data=5;\n\
           #10 req=0; grant=1; rdata=5;\n\
           #10 grant=0;\n\
           #20 $finish;\n\
         end\n\
         endmodule\n");
    holds(&out, &err, code, "named property p; int d; … capture");
}

// ── LOUD (correct-or-loud convergence / out-of-subset boundary) ────────────────

#[test]
fn ranged_delay_is_loud() {
    // `##[1:2]` — a ranged delay lets two attempts CONVERGE on one completion stage
    // (a data collision). Unsupported → loud (NOT a silent guess).
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (req, d=data) ##[1:2] grant |-> (rdata == d));\n\
         initial begin #10 req=1; data=5; #10 req=0; grant=1; rdata=7; #30 $finish; end\n\
         endmodule\n");
    loud(&out, &err, code, "ranged ##[1:2] delay with a local var");
}

#[test]
fn consecutive_range_repeat_is_loud() {
    // `b[*1:2]` — a ranged repetition is the same convergence hazard. Loud.
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0, x=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (req, d=data) ##1 x[*1:2] ##1 grant |-> (rdata == d));\n\
         initial begin #10 req=1; data=5; #50 $finish; end\n\
         endmodule\n");
    loud(
        &out,
        &err,
        code,
        "ranged [*1:2] repetition with a local var",
    );
}

#[test]
fn read_before_capture_is_loud() {
    // The var is read inside an antecedent term (here the capture term itself, `req &&
    // d==0`). A read at/before the capture has no defined data stage → loud.
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (req && d==0, d=data) ##1 grant |-> (rdata == d));\n\
         initial begin #10 req=1; data=5; #10 req=0; grant=1; #30 $finish; end\n\
         endmodule\n");
    loud(
        &out,
        &err,
        code,
        "read of the local var inside an antecedent term",
    );
}

#[test]
fn multiple_write_is_loud() {
    // Two captures (`d=data, e=data`) — multiple writes converge / alias. Loud.
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; int e; (req, d=data, e=data) ##1 grant |-> (rdata == d));\n\
         initial begin #10 req=1; data=5; #10 req=0; grant=1; #30 $finish; end\n\
         endmodule\n");
    loud(&out, &err, code, "multiple local-var writes");
}

#[test]
fn cross_clock_is_loud() {
    // A re-clocking `@(posedge c2)` boundary inside a local-var sequence. Loud.
    let (out, err, code) = run("module top;\n\
         reg clk=0, c2=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk; always #7 c2=~c2;\n\
         initial assert property(@(posedge clk) int d; (req, d=data) ##1 @(posedge c2) grant |-> (rdata == d));\n\
         initial begin #10 req=1; data=5; #10 req=0; grant=1; #30 $finish; end\n\
         endmodule\n");
    loud(&out, &err, code, "cross-clock sequence with a local var");
}

#[test]
fn capture_of_undeclared_var_is_loud() {
    // A capture target that is NOT a declared local var (and not a real net) — its
    // width/sign cannot be resolved → loud (never a silent default width).
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) (req, d=data) ##1 grant |-> (rdata == d));\n\
         initial begin #10 req=1; data=5; #10 req=0; grant=1; #30 $finish; end\n\
         endmodule\n");
    loud(&out, &err, code, "capture into an undeclared variable");
}

#[test]
fn sequence_consequent_with_local_var_is_loud() {
    // A SEQUENCE consequent (`|-> (rdata==d) ##1 done`) would need the data threaded
    // through the obligation chain — out of subset → loud.
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0, done=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (req, d=data) ##1 grant |-> (rdata==d) ##1 done);\n\
         initial begin #10 req=1; data=5; #50 $finish; end\n\
         endmodule\n");
    loud(&out, &err, code, "sequence consequent with a local var");
}

#[test]
fn disable_iff_with_local_var_is_loud() {
    // `disable iff` combined with a local-var capture would need the data reset
    // machinery threaded — out of subset → loud.
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0, rst=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) disable iff(rst) int d; (req, d=data) ##1 grant |-> (rdata == d));\n\
         initial begin #10 req=1; data=5; #50 $finish; end\n\
         endmodule\n");
    loud(&out, &err, code, "disable iff with a local var");
}

// ── NON-INTEGRAL local-var TYPE = LOUD (never a silent 1-bit truncation) ───────
// A `real`/`string`/`realtime`/`event`/class-typed SVA local var has no
// synthesizable fixed-width data-tracking register in this subset. The parser must
// flag the type and elaborate must loud-reject — otherwise the value is silently
// truncated to a 1-bit register, flipping the verdict (a silent-wrong). The
// integral idiom (`int`/`bit`/`byte`/…) is unaffected.
//
// The stimulus is deliberately the SILENT-PASS direction (data=4, rdata=0): a
// 1-bit truncation captures `4 & 1 = 0`, so `rdata=0 == 0` would spuriously HOLD
// (exit 0, NO diagnostic) while hand-IEEE `0 == 4.0` is FALSE → a missed violation.
// A bare `loud()` would be satisfied by an UNRELATED property-violation E4003 in
// the other direction, so `loud_unsupported_type` additionally requires the
// `VITA-E3009` type rejection (not a spurious verdict).
fn loud_unsupported_type(out: &str, err: &str, code: Option<i32>, ctx: &str) {
    assert_ne!(code, Some(0), "{ctx}: must not silently pass.\n{err}{out}");
    let combined = format!("{err}{out}");
    assert!(
        combined.contains("VITA-E3009"),
        "{ctx}: expected the E3009 unsupported-type rejection (not a spurious \
         verdict).\nstderr:\n{err}\nout:\n{out}"
    );
    assert!(
        combined.contains("non-integral") || combined.contains("integral"),
        "{ctx}: the diagnostic should name the non-integral type rejection.\n\
         stderr:\n{err}\nout:\n{out}"
    );
}

#[test]
fn real_typed_local_var_is_loud() {
    // 1-bit truncation captures `4 & 1 = 0` → `rdata=0 == 0` spuriously HOLDS while
    // hand-IEEE `0 == 4.0` is a violation. Must be loud (E3009), not a silent pass.
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) real d; (req, d=data) ##1 grant |-> (rdata == d));\n\
         initial begin #10 req=1; data=4; #10 req=0; grant=1; rdata=0; #30 $finish; end\n\
         endmodule\n");
    loud_unsupported_type(&out, &err, code, "real-typed SVA local var");
}

#[test]
fn string_typed_local_var_is_loud() {
    // `string d;` is a heap-handle, not a fixed-width capture register → loud (E3009).
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) string d; (req, d=data) ##1 grant |-> (rdata == d));\n\
         initial begin #10 req=1; data=4; #10 req=0; grant=1; rdata=0; #30 $finish; end\n\
         endmodule\n");
    loud_unsupported_type(&out, &err, code, "string-typed SVA local var");
}

#[test]
fn realtime_typed_local_var_is_loud() {
    // `realtime d;` (a real-valued time) is non-integral → loud (not 1-bit truncated).
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) realtime d; (req, d=data) ##1 grant |-> (rdata == d));\n\
         initial begin #10 req=1; data=4; #10 req=0; grant=1; rdata=0; #30 $finish; end\n\
         endmodule\n");
    loud_unsupported_type(&out, &err, code, "realtime-typed SVA local var");
}

#[test]
fn event_typed_local_var_is_loud() {
    // `event d;` is a 64-bit counter desugar, not a capturable value type → loud.
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) event d; (req, d=data) ##1 grant |-> (rdata == d));\n\
         initial begin #10 req=1; data=4; #10 req=0; grant=1; rdata=0; #30 $finish; end\n\
         endmodule\n");
    loud_unsupported_type(&out, &err, code, "event-typed SVA local var");
}

#[test]
fn real_typed_named_property_local_var_is_loud() {
    // The same non-integral type via a NAMED property body-start decl (the other
    // local-var parse position) must also be loud, not a silent 1-bit truncation.
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         property p; real d; @(posedge clk) (req, d=data) ##1 grant |-> (rdata == d); endproperty\n\
         initial assert property(p);\n\
         initial begin #10 req=1; data=4; #10 req=0; grant=1; rdata=0; #30 $finish; end\n\
         endmodule\n");
    loud_unsupported_type(&out, &err, code, "real-typed named-property SVA local var");
}

#[test]
fn integral_local_var_still_works_after_type_guard() {
    // Regression guard: the type rejection must NOT disturb the integral idiom. The
    // same data=4/rdata=4 with `int d;` HOLDS (a wide capture, no truncation).
    let (out, err, code) = run("module top;\n\
         reg clk=0, req=0, grant=0;\n\
         reg [7:0] data=0, rdata=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) int d; (req, d=data) ##1 grant |-> (rdata == d));\n\
         initial begin #10 req=1; data=4; #10 req=0; grant=1; rdata=4; #30 $finish; end\n\
         endmodule\n");
    holds(
        &out,
        &err,
        code,
        "int-typed capture still holds (data=4 not truncated)",
    );
}

// ── BYTE-IDENTITY: a no-local-var SVA design is unaffected ─────────────────────

#[test]
fn no_local_var_assertion_unaffected_holds() {
    // A plain `a |-> b` (no local var, no match-item) must take the byte-identical
    // flat path — the data-tracking machinery only activates on a capture.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> b);\n\
         initial begin #10 a=1; b=1; #20 $finish; end\n\
         endmodule\n");
    holds(&out, &err, code, "plain a |-> b unaffected by N2c");
}

#[test]
fn no_local_var_assertion_unaffected_fires() {
    // The same plain assertion still FIRES on a real violation.
    let (out, err, code) = run("module top;\n\
         reg clk=0, a=0, b=0;\n\
         always #5 clk=~clk;\n\
         initial assert property(@(posedge clk) a |-> b);\n\
         initial begin #10 a=1; b=1; #10 a=1; b=0; #10 $finish; end\n\
         endmodule\n");
    fires(&out, &err, code, "plain a |-> b still fires on a violation");
}
