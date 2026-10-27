//! `simulate_capture`: run a simulation capturing RTL output as text (tests and a
//! simple CLI path). The capture keeps the BYTES and takes the lossy Rust-text view
//! once, at the end (Q-STRBYTES L3: a test boundary).

use std::cell::RefCell;
use std::rc::Rc;

use diag::{LogEvent, LogSink};
use sim_ir::SimIr;

use crate::{simulate, SimOpts, SimResult};

/// Convenience: run a simulation capturing RTL output into a `String` and the
/// VCD into the file named by `$dumpfile`/`override`. Returns (result, stdout).
/// Primarily for tests + a simple CLI path.
pub fn simulate_capture(ir: &SimIr, opts: SimOpts) -> (SimResult, String) {
    let buf = Rc::new(RefCell::new(Vec::new()));
    let sink = CaptureSink { buf: buf.clone() };
    let result = simulate(ir, &sink, opts);
    let s = String::from_utf8_lossy(&buf.borrow()).into_owned();
    (result, s)
}

struct CaptureSink {
    buf: Rc<RefCell<Vec<u8>>>,
}

impl LogSink for CaptureSink {
    fn emit(&self, event: LogEvent) {
        if let LogEvent::RtlOutput(t) = event {
            self.buf.borrow_mut().extend_from_slice(&t.text);
        }
    }
}
