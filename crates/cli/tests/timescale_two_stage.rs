//! IEEE two-stage `#delay` conversion (doc-08 §delay): a real delay first
//! rounds to the DECLARING module's own `time_precision`, then scales to the
//! global tick base. The old path rounded ONCE at the global grain, keeping
//! sub-precision digits the module declared away — silent-wrong for any
//! mixed-precision design (adversarial audit, format_version 22).
//!
//! Every value is pinned to LIVE iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ts2_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn mixed_precision_module_rounds_to_own_precision_first() {
    // top is 1us/10ns; a sibling 1ns/100ps drags the GLOBAL precision to 100ps.
    // `#3.453` (us) → stage 1: round to 10ns ⇒ 3450ns → stage 2: 34500 ticks.
    // The single-round bug gave 34530 ticks (rt=3.453). iverilog: rt=3.45,
    // $finish at 34500 (100ps).
    let out = run("`timescale 1us/10ns\n\
         module top;\n\
           initial begin #3.453; $display(\"rt=%g\", $realtime); $finish; end\n\
         endmodule\n\
         `timescale 1ns/100ps\n\
         module fine; endmodule\n");
    assert!(
        out.contains("rt=3.45\n"),
        "stage-1 rounding missing:\n{out}"
    );
    assert!(out.contains("at time 34500"), "global ticks wrong:\n{out}");
}

#[test]
fn coarse_precision_module_half_away_rounds_whole_units() {
    // top is 1ns/1ns (precision == unit); global precision is 1ps via `fine`.
    // `#2.5` → stage 1: round-half-away at 1ns ⇒ 3ns → 3000 ticks. The
    // single-round bug gave 2500 ticks (rt=2.5). iverilog: rt=3, 3000 (1ps).
    let out = run("`timescale 1ns/1ns\n\
         module top;\n\
           initial begin #2.5; $display(\"rt=%g\", $realtime); $finish; end\n\
         endmodule\n\
         `timescale 1ns/1ps\n\
         module fine; endmodule\n");
    assert!(
        out.contains("rt=3\n"),
        "half-away at module grain missing:\n{out}"
    );
    assert!(out.contains("at time 3000"), "global ticks wrong:\n{out}");
}

#[test]
fn single_timescale_design_unchanged() {
    // prec == global for every module ⇒ S=1 ⇒ the two-stage conversion is
    // byte-identical to the old single rounding (regression guard for the
    // overwhelmingly common case).
    let out = run("`timescale 1ns/1ps\n\
         module top;\n\
           initial begin #2.5; $display(\"rt=%g\", $realtime); $finish; end\n\
         endmodule\n");
    assert!(out.contains("rt=2.5\n"), "single-timescale changed:\n{out}");
    assert!(
        out.contains("at time 2500"),
        "single-timescale ticks changed:\n{out}"
    );
}
