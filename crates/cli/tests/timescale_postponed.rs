//! Regression (Stage C / C7): a `$strobe`/`$monitor` in the postponed region must
//! render `$time`/`$realtime` with the multiplier of the module that REGISTERED it —
//! NOT whatever process happened to run LAST in the timestep. `flush_postponed` used
//! the scheduler's `cur_time_mult`, which by flush time holds the last-run process's
//! module multiplier; under mixed timescales that is a DIFFERENT module's `M`.
//!
//! Repro shape: one top instantiating two submodules with DIFFERENT `` `timescale ``s,
//! both firing in the postponed region at the SAME global tick. (Two bare top modules
//! would now also both elaborate — see `multi_top.rs` — but the single-top form is the
//! tighter repro: it guarantees both fire in the SAME timestep's postponed batch.)
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Write a temp `.sv`, run `vita <file>` (oneshot, interpreter backend), capture stdout.
fn run_vita(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_tspost_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("failed to run vita");
    let _ = std::fs::remove_file(&path);
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// Both submodules fire at the same global tick (1ps precision ⇒ now == 1000):
//   sub_ns: 1ns unit, `#1`    → 1 × 1000 = tick 1000, $time = 1000/1000 = 1
//   sub_ps: 1ps unit, `#1000` → 1000 × 1  = tick 1000, $time = 1000/1    = 1000
// The postponed flush renders BOTH captures in one batch; each must use its own M.
#[test]
fn strobe_renders_registering_modules_timescale() {
    let src = r#"
`timescale 1ns/1ns
module sub_ns;
  initial #1 $strobe("ns=%0d", $time);
endmodule
`timescale 1ps/1ps
module sub_ps;
  initial #1000 $strobe("ps=%0d", $time);
endmodule
`timescale 1ns/1ns
module top;
  sub_ns u_ns();
  sub_ps u_ps();
endmodule
"#;
    let out = run_vita(src);
    assert!(
        out.lines().any(|l| l == "ns=1"),
        "ns strobe must use the 1ns module multiplier (expected 'ns=1'); got:\n{out}"
    );
    assert!(
        out.lines().any(|l| l == "ps=1000"),
        "ps strobe must use the 1ps module multiplier (expected 'ps=1000'); got:\n{out}"
    );
}

// $monitor establishment print is also a postponed render: it must use the
// monitoring module's multiplier. Here sub_ns registers the monitor and sub_ps
// (different timescale) runs afterward in the same tick, clobbering cur_time_mult.
#[test]
fn monitor_establishment_uses_registering_modules_timescale() {
    let src = r#"
`timescale 1ns/1ns
module sub_ns;
  initial #1 $monitor("mon_ns=%0d", $time);
endmodule
`timescale 1ps/1ps
module sub_ps;
  initial #1000 $strobe("ps=%0d", $time);
endmodule
`timescale 1ns/1ns
module top;
  sub_ns u_ns();
  sub_ps u_ps();
endmodule
"#;
    let out = run_vita(src);
    assert!(
        out.lines().any(|l| l == "mon_ns=1"),
        "monitor must render $time with the 1ns module multiplier (expected 'mon_ns=1'); got:\n{out}"
    );
}
