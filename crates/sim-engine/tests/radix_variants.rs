//! P1-5: `$displayb/o/h`, `$writeb/o/h`, `$strobeb/o/h`, `$monitorb/o/h`
//! (REMAINING_WORK 2026-06-10). The b/o/h variants change the DEFAULT radix of
//! unformatted arguments (IEEE 1364-2005 §17.1.1.1) — `%` specs inside format
//! strings are untouched. Before this fix the display/write variants silently
//! printed DECIMAL and the strobe/monitor variants didn't exist at all
//! (warn+skip). The radix rides an out-of-band side table (StmtId → radix),
//! like the severity table — the frozen SysTaskId is unchanged.

use std::cell::RefCell;
use std::collections::BTreeMap;

use diag::{LogEvent, LogSink};
use sim_engine::{simulate, SimOpts};

#[derive(Default)]
struct OutSink(RefCell<String>);
impl LogSink for OutSink {
    fn emit(&self, e: LogEvent) {
        if let LogEvent::RtlOutput(t) = e {
            self.0.borrow_mut().push_str(&t.text);
        }
    }
}

fn run(src: &str) -> String {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "parse errors: {pe:?}");
    let sink = OutSink::default();
    let (ir, sc) =
        elaborate::elaborate_with_timescale(&su.expect("source unit"), &sink, &BTreeMap::new(), -9);
    let ir = ir.expect("elaborate");
    let opts = SimOpts {
        fork_modes: sc.fork_modes,
        net_names: sc.net_names,
        proc_multipliers: sc.proc_multipliers,
        severities: sc.severities,
        radixes: sc.radixes,
        ..SimOpts::default()
    };
    simulate(&ir, &sink, opts);
    sink.0.into_inner()
}

/// `$displayh`: unformatted args print as padded hex fields (like bare `%h`).
#[test]
fn displayh_renders_hex() {
    let out = run(r#"
module t;
  reg [7:0] a; reg [15:0] b;
  initial begin
    a = 8'd255; b = 16'h00ab;
    $displayh(a, b);
    $finish;
  end
endmodule
"#);
    assert_eq!(out, "ff00ab\n"); // iverilog: padded concat, no separator
}

/// `$displayb` / `$displayo`.
#[test]
fn displayb_and_displayo() {
    let out = run(r#"
module t;
  reg [3:0] a;
  initial begin
    a = 4'd5;
    $displayb(a);
    $displayo(a);
    $finish;
  end
endmodule
"#);
    assert_eq!(out, "0101\n05\n");
}

/// A format STRING inside a radix variant behaves exactly like in `$display`
/// (its `%` specs are authoritative); only the unformatted tail changes radix.
#[test]
fn displayh_format_specs_unaffected() {
    let out = run(r#"
module t;
  initial begin
    $displayh("d=%0d", 4'd5, 8'hff);
    $finish;
  end
endmodule
"#);
    assert_eq!(out, "d=5ff\n"); // iverilog parity
}

/// `$writeh` — no trailing newline, hex default radix.
#[test]
fn writeh_no_newline() {
    let out = run(r#"
module t;
  initial begin
    $writeh(8'hf0);
    $write("|");
    $finish;
  end
endmodule
"#);
    assert_eq!(out, "f0|");
}

/// `$strobeh` registers a postponed capture that renders HEX at flush.
#[test]
fn strobeh_renders_hex_at_flush() {
    let out = run(r#"
module t;
  reg [7:0] v;
  initial begin
    v = 8'd16;
    $strobeh(v);
    v = 8'd255;
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(out, "ff\n", "strobe samples settled value, renders hex");
}

/// `$monitorh` prints establishment + changes in HEX.
#[test]
fn monitorh_renders_hex() {
    let out = run(r#"
module t;
  reg [7:0] v;
  initial begin
    v = 8'd16;
    $monitorh(v);
    #1 v = 8'd255;
    #1 $finish;
  end
endmodule
"#);
    assert_eq!(out, "10\nff\n");
}
