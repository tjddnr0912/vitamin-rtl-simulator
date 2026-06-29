//! C-WAITER-POOL-p2 (ROADMAP §5): the per-delta `expr_now`/`level_fire`
//! precompute scans in `propagate_changes` ran over the FULL waiter vector on
//! every non-idle delta even when ZERO `Expr`/`Level` waiters exist. Part 2
//! adds running counts (`n_expr_waiters`/`n_level_waiters`) so the
//! buffer-FILL is skipped when the matching count is 0.
//!
//! This is a pure scheduler-scratch optimization (IR-0, byte-identical): when a
//! count is 0 the corresponding `retain` match-arm cannot fire, so the skipped
//! buffer was unused. The real gate is the existing `inbody_events`/`threads`
//! suites + the full byte-identity corpus staying green UNCHANGED.
//!
//! This characterization test pins an all-`Edge`-sensitive clocked design (a
//! shift register clocked by `always @(posedge clk)` only — NO `wait(expr)` and
//! NO `always @(*)`/level sensitivity, so `n_expr_waiters == n_level_waiters ==
//! 0` for the whole run). Its output must be byte-identical before AND after
//! the guard is introduced.

use diag::{LogEvent, LogSink};
use sim_engine::{simulate_capture, SimOpts};

#[derive(Default)]
struct DiagSink(std::cell::RefCell<Vec<String>>);
impl LogSink for DiagSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::Diagnostic(d) = e {
            self.0
                .borrow_mut()
                .push(format!("{:?}: {}", d.severity, d.message));
        }
    }
}

fn run(src: &str) -> String {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = DiagSink::default();
    let ir = elaborate::elaborate(&su.expect("source unit"), &sink);
    let hard: Vec<String> = sink
        .0
        .borrow()
        .iter()
        .filter(|d| d.starts_with("Error") || d.starts_with("Fatal"))
        .cloned()
        .collect();
    assert!(hard.is_empty(), "elaborate errors: {hard:?}");
    let (_res, out) = simulate_capture(&ir.expect("ir"), SimOpts::default());
    out
}

/// All-`Edge` design: two `always @(posedge clk)` shift the data through `q`,
/// and a clock-driver `always` toggles `clk` via in-body `#5` (self-timed, no
/// static level sensitivity). There is NO `wait(expr)` and NO level-sensitive
/// `always @(*)`, so the Expr/Level waiter counts are 0 throughout — the p2
/// guard skips both precompute scans every delta. The serial-out stream is
/// derived by hand: din=1 at posedge#1 → q0=1; posedge#2 → q1=q0=1, q0=din;
/// etc. (one-cycle pipeline latency per stage).
#[test]
fn all_edge_shift_register_byte_identical() {
    let out = run(r#"
module t;
  reg clk;
  reg din;
  reg q0, q1;
  // Clock driver: self-timed via in-body delay (no static sensitivity list).
  initial clk = 0;
  always #5 clk = ~clk;

  // Two Edge-sensitive flops — the ONLY waiters are net_to_edge entries.
  always @(posedge clk) q0 <= din;
  always @(posedge clk) q1 <= q0;

  initial begin
    din = 0; q0 = 0; q1 = 0;
    @(posedge clk) din = 1; // arrive just after edge 1 (q0/q1 still 0)
    @(posedge clk) din = 0; // edge 2: q0<=1, q1<=0
    @(posedge clk);         // edge 3: q0<=0, q1<=1
    @(posedge clk);         // edge 4: q0<=0, q1<=0
    $display("q0=%b q1=%b", q0, q1);
    $finish;
  end
endmodule
"#);
    // Hand trace of the NBA samples (q0<=din, q1<=q0) at each posedge. The
    // initial block resumes in the SAME slot as the edge and updates `din`
    // AFTER the flop NBAs sample the preponed value, so `din` seen by the flop
    // at edge N is the value set at edge N-1:
    //   din at e1 sees 0 -> q0=0 ; q0 was 0 -> q1=0   (din becomes 1 after)
    //   din at e2 sees 1 -> q0=1 ; q0 was 0 -> q1=0   (din becomes 0 after)
    //   din at e3 sees 0 -> q0=0 ; q0 was 1 -> q1=1
    //   din at e4 sees 0 -> q0=0 ; q0 was 0 -> q1=0(prev cycle)... display runs
    //     in the e4 slot, observing q0=0,q1=1 (the e3-cycle pipeline state, the
    //     e4 NBAs land but the display already sampled). This is the engine's
    //     deterministic tie/region ordering — pinned as the characterization
    //     golden; it must stay byte-identical across the p2 guard.
    assert_eq!(out.trim(), "q0=0 q1=1");
}
