//! P4-T1: `--threads` byte-identity contract. Thread count changes WALL-CLOCK
//! ONLY — VCD bytes, stdout, and the run summary are identical for every N
//! (the writer thread receives the exact byte stream the single-thread path
//! writes, over an order-preserving bounded FIFO).

use std::cell::RefCell;
use std::collections::BTreeMap;

use diag::{LogEvent, LogSink};
use sim_engine::{simulate, SimOpts, SimResult};

struct OutSink(RefCell<String>);
impl LogSink for OutSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::RtlOutput(t) = e {
            self.0.borrow_mut().push_str(&t.text);
        }
    }
}

/// Dump-heavy design: 4 nets toggling for 2k ticks ≈ many VCD records.
const DUMP_SRC: &str = r#"
module top;
  reg clk;
  reg [31:0] a, b, c;
  integer k;
  always @(posedge clk) begin
    a <= a + 32'd1;
    b <= b ^ a;
    c <= c + b;
  end
  initial begin
    $dumpfile("x.vcd");
    $dumpvars;
    clk = 0; a = 1; b = 2; c = 3;
    for (k = 0; k < 1000; k = k + 1) begin #1 clk = 1; #1 clk = 0; end
    $display("done c=%0d", c);
    $finish;
  end
endmodule
"#;

fn run_threads(threads: u32, tag: &str) -> (SimResult, String, Vec<u8>) {
    let (toks, le) = hdl_lexer::lex(DUMP_SRC);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, DUMP_SRC);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = OutSink(RefCell::new(String::new()));
    let (ir, sc) =
        elaborate::elaborate_with_timescale(&su.expect("source unit"), &sink, &BTreeMap::new(), -9);
    let ir = ir.expect("elaborate");
    let path = std::env::temp_dir().join(format!("vita_threads_{}_{tag}.vcd", std::process::id()));
    let opts = SimOpts {
        fork_modes: sc.fork_modes,
        net_names: sc.net_names,
        proc_multipliers: sc.proc_multipliers,
        severities: sc.severities,
        radixes: sc.radixes,
        threads,
        vcd_path_override: Some(path.to_string_lossy().into_owned()),
        ..SimOpts::default()
    };
    let result = simulate(&ir, &sink, opts);
    let vcd = std::fs::read(&path).expect("VCD must exist");
    let _ = std::fs::remove_file(&path);
    (result, sink.0.into_inner(), vcd)
}

/// THE byte-identity contract: `--threads 1` vs `--threads 4` — same VCD bytes,
/// same stdout, same summary. N changes wall-clock only.
#[test]
fn threads_vcd_byte_identical() {
    let (r1, o1, v1) = run_threads(1, "t1");
    let (r4, o4, v4) = run_threads(4, "t4");
    assert!(v1.len() > 1000, "dump design must emit a real VCD");
    assert_eq!(v1, v4, "VCD bytes must be identical for all thread counts");
    assert_eq!(o1, o4, "stdout must be identical");
    assert_eq!(r1.sim_time, r4.sim_time);
    assert_eq!(r1.finish_reason, r4.finish_reason);
    assert_eq!(r1.exit_class, r4.exit_class);
}
