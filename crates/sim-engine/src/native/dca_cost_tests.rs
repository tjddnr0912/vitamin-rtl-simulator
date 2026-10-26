//! ENG-1c: the cost pin of a delayed continuous assignment's decision (ER §7.4: a
//! cost-only specialisation is pinned by an operation-mix census, not by an output).
//!
//! `Scheduler::schedule_delayed_cas` visits every delayed driver at the end of every
//! settle, change or not. A visit whose right-hand side equals the one its last
//! decision was made on returns after that one compare (`ca_slot[ci].rhs_seen`);
//! only a changed right-hand side is converted to the store image
//! (`SimState::store_image`).
//! Removing that return leaves every output unchanged — the full decision on an
//! unchanged right-hand side is no event (measured over the delayed-CA cells,
//! ENG-1c r3) — and converts on every visit, which is what this census counts.
//! No oracle: the count is vita's own operation mix.

use super::tests::{build_with_opts, NullSink};
use crate::state::STORE_IMAGE_CALLS;
use crate::Backend;

/// 30 time steps that each settle (the `c` changes wake `always @(c)`), one change of
/// the delayed driver's right-hand side among them.
const SRC: &str = "module t;\n\
    logic [7:0] a = 8'd3; int y; logic [7:0] c = 0; int n = 0;\n\
    assign #2 y = a;\n\
    always @(c) n = n + 1;\n\
    initial begin\n\
      repeat (15) begin c = c + 1; #1; end\n\
      a = 8'd9;\n\
      repeat (15) begin c = c + 1; #1; end\n\
      $finish;\n\
    end\n\
  endmodule\n";

fn calls_on(backend: Backend) -> u64 {
    let (ir, mut opts) = build_with_opts(SRC);
    opts.backend = backend;
    STORE_IMAGE_CALLS.with(|c| c.set(0));
    let res = crate::simulate(&ir, &NullSink, opts);
    assert_eq!(
        res.exit_class,
        crate::ExitClass::Ok,
        "{backend:?}: the design must run"
    );
    STORE_IMAGE_CALLS.with(|c| c.get())
}

#[test]
fn a_delayed_assign_converts_once_per_right_hand_side_change() {
    let backends = [
        Backend::Native,
        #[cfg(feature = "oracle")]
        Backend::Bytecode,
        #[cfg(feature = "oracle")]
        Backend::Interpreter,
    ];
    for be in backends {
        let n = calls_on(be);
        // Two right-hand-side values (3, then 9): one conversion each, and at most
        // one more for the first settle's decision on the default x (it converts 3
        // with or without an initializer). Every settle visits the driver (94 visits
        // on native, measured): converting per visit counts them all.
        assert!(
            (2..=3).contains(&n),
            "{be:?}: {n} store-image conversions for 2 right-hand-side values"
        );
    }
}
