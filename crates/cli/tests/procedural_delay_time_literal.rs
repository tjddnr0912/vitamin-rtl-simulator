//! §2 "Delays / events" ⓓ: the PROCEDURAL delay lanes fold a time literal in the
//! DELAY domain, like the structural ones.
//!
//! `lower_delay` used to hand the amount straight to `lower_expr`, which reaches
//! `const_eval_in_scope`'s `TimeLit` arm. That arm declines three separate ways —
//! a literal whose UNIT is finer than the design precision, a REAL numerator, and
//! one that is not a whole multiple of the module's time unit — so `#(2500ps);`
//! under `1ns/1ns` was E3009 where both oracles delay 3 ns, while its structural
//! twin `assign #(2500ps)` had been right since §4.5.458/459.
//!
//! Worse, on one shape the integer lane did not decline, it ANSWERED WRONG at
//! exit 0: `#(3ns / 2)` folded integer division to one module unit where both
//! oracles delay two. That is why the delay fold is asked BEFORE `lower_expr`
//! rather than as a fallback — the same ordering `delay_ticks_in_scope`'s own
//! comment records for the structural lane.
//!
//! Census: 6 timescales × 18 literals × 5 procedural lanes (statement, intra-assign
//! `=`, intra-assign `<=`, task body, `fork` arm), PRE / POST / iverilog 13.0 /
//! verilator 5.052 → **66 fixed, 0 regressed, 23 already correct, 7 oracle-split**.
//!
//! The sign axis was added after the differential lens caught a regression the first
//! census could not see: `#(1ns - 5ns)` never fires in either oracle and never fired
//! here, but `delay_ticks_in_scope` returns a `u32` and `real_delay_ticks` clamps a
//! negative to 0, so the first cut of the routing fired it at once. The sign is now
//! read in the units domain, before the clamp.
//!
//! ⚠️ Every value here is in FEMTOSECONDS, read with `$timeformat(-15,0,"",20)` and
//! `$realtime`. Bare `$time` rounds to the module TIME UNIT and reports 3 for the
//! headline cell whether it is fixed or not.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run one design and return every `P<n>` line's last field, plus the exit code.
fn run(ts: &str, delay: &str) -> (Vec<String>, Option<i32>, String) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pdtl_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    let src = format!(
        "`timescale {ts}\n\
         module m;\n\
         \x20 logic a = 0, x1, x2;\n\
         \x20 task automatic tk(); #({delay}); endtask\n\
         \x20 initial begin $timeformat(-15,0,\"\",20);\n\
         \x20   #({delay});           $display(\"P1 %t\", $realtime);\n\
         \x20   x1 = #({delay}) a;    $display(\"P2 %t\", $realtime);\n\
         \x20   x2 <= #({delay}) a; #0; $display(\"P3 %t\", $realtime);\n\
         \x20   tk();                 $display(\"P4 %t\", $realtime);\n\
         \x20   fork #({delay}); join $display(\"P5 %t\", $realtime);\n\
         \x20   $finish; end\n\
         \x20 initial #10000000 $finish;\n\
         endmodule\n"
    );
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let all =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    let vals = all
        .lines()
        .filter(|l| l.starts_with('P') && l.len() > 1 && l.as_bytes()[1].is_ascii_digit())
        .map(|l| l.split_whitespace().last().unwrap_or("").to_string())
        .collect();
    (vals, out.status.code(), all)
}

/// Each of the five procedural lanes must delay `step` fs.
///
/// The cumulative stamps are 1,2,2,3,4 — not 1..5 — because the NBA intra-assign
/// (`x2 <= #(d) a;`) schedules its update without suspending the process, so P3
/// reads the time P2 already reported. That is IEEE §10.4.2, and both oracles
/// print the identical five stamps: a property of the lane, not of the delay.
fn lanes(ts: &str, delay: &str, step: u64) {
    let (vals, code, all) = run(ts, delay);
    assert_eq!(code, Some(0), "`{ts}` `#({delay})` must run:\n{all}");
    let want: Vec<String> = [1u64, 2, 2, 3, 4]
        .iter()
        .map(|k| (k * step).to_string())
        .collect();
    assert_eq!(
        vals, want,
        "`{ts}` `#({delay})`: each procedural lane must delay {step} fs\n{all}"
    );
}

#[test]
fn a_literal_finer_than_the_precision_delays_in_every_procedural_lane() {
    // The headline: E3009 in PRE, 3 ns in both oracles.
    lanes("1ns/1ns", "2500ps", 3_000_000);
    lanes("1ns/1ns", "1500ps", 2_000_000);
    // …and where the module unit can hold it, no rounding happens at all.
    lanes("1ns/1ps", "2500ps", 2_500_000);
    lanes("1ns/1ps", "1500ps", 1_500_000);
    lanes("1ns/1ps", "1250fs", 1_000);
    lanes("1ps/1ps", "1250fs", 1_000);
}

#[test]
fn a_real_numerator_delays_too() {
    // The arm's SECOND decline: `const_eval_in_scope` has no value for a real.
    lanes("1ns/1ns", "2.5ns", 3_000_000);
    lanes("1ns/1ps", "2.5ns", 2_500_000);
    lanes("1ps/1ps", "2.5ps", 3_000);
    lanes("10ns/1ns", "2.5ns", 3_000_000);
    lanes("1us/1ns", "2.5ns", 3_000_000);
}

#[test]
fn a_literal_that_is_not_a_whole_module_unit_delays_too() {
    // The arm's THIRD decline (`ticks % mult != 0`): `#(3ns)` under a 10 ns unit
    // is 0.3 units, which the integer lane has nowhere to put.
    lanes("10ns/1ns", "3ns", 3_000_000);
    lanes("1us/1ns", "3ns", 3_000_000);
    lanes("10ns/1ns", "1500ps", 2_000_000);
}

#[test]
fn arithmetic_over_time_literals_was_silent_wrong_not_loud() {
    // These ran at exit 0 with NO diagnostic and the wrong time: PRE delayed
    // 1 000 000 / 2 000 000 / 1 000 000 fs where both oracles delay what is
    // pinned here. A fallback-on-`None` patch would have left all of them.
    lanes("1ns/1ns", "3ns / 2", 2_000_000);
    lanes("1ns/1ns", "5ns / 2ns", 3_000_000);
    lanes("1ns/1ns", "7ns / 4", 2_000_000);
    lanes("1ns/1ps", "3ns / 2", 1_500_000);
    lanes("1ns/1ps", "7ns / 4", 1_750_000);
    lanes("1ns/1ps", "5ns / 2ns", 2_500_000);
}

#[test]
fn a_time_literal_inside_an_expression_delays() {
    lanes("1ns/1ns", "2500ps + 1000ps", 4_000_000);
    lanes("1ns/1ps", "2500ps + 1000ps", 3_500_000);
    lanes("1ns/1ps", "1250fs + 1250fs", 2_000);
    lanes("10ns/1ns", "2500ps + 1000ps", 4_000_000);
}

#[test]
fn a_genuinely_sub_tick_literal_still_delays_nothing() {
    // The other half of the rule: a literal below half a tick rounds to zero
    // ticks in all three tools, so `$realtime` never moves. `lower_delay` marks
    // those `Inactive`, the same region `#0` takes.
    lanes("1ns/1ns", "2ps", 0);
    lanes("1ns/1ns", "2.5ps", 0);
    lanes("1ns/1ns", "1250fs", 0);
    lanes("10ns/1ns", "1250fs", 0);
    lanes("1us/1ns", "2.5ps", 0);
}

#[test]
fn the_cells_that_were_already_right_do_not_move() {
    // Control twins. A bare real (`#2.5`) and a whole-unit literal reached the
    // integer lane before and must read the same after — this is where a
    // regression would show if the new route had replaced rather than preceded it.
    lanes("1ns/1ns", "2.5", 3_000_000);
    lanes("1ns/1ns", "3ns", 3_000_000);
    lanes("1ns/1ps", "2.5", 2_500_000);
    lanes("1ns/1ps", "3ns", 3_000_000);
    lanes("1ps/1ps", "2500ps", 2_500_000);
    lanes("1ps/1ps", "2ps", 2_000);
    lanes("1ps/1ps", "3ns / 2", 1_500_000);
    lanes("1ps/1ps", "7ns / 4", 1_750_000);
    lanes("10ns/1ns", "2.5", 25_000_000);
    lanes("1us/1ns", "2.5", 2_500_000_000);
}

#[test]
fn a_negative_delay_still_never_fires() {
    // The regression the differential lens caught, and the reason the sign is read
    // before the clamp. `#(1ns - 5ns)` never fires in iverilog, never fires in
    // verilator, and never fired here — but `delay_ticks_in_scope` returns a `u32`
    // and `real_delay_ticks` clamps a negative amount to 0, so the first cut of the
    // routing fired it at time 0. A negative delay now falls through to the path
    // that was already right, at every timescale.
    for ts in ["1ns/1ns", "1ns/1ps", "1ps/1ps", "10ns/1ns"] {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!("vita_pdtl_n_{}_{n}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("t.sv");
        std::fs::write(
            &f,
            format!(
                "`timescale {ts}\nmodule m;\n\
                 \x20 initial begin $timeformat(-15,0,\"\",20);\n\
                 \x20   #(1ns - 5ns); $display(\"FIRED %t\", $realtime); $finish; end\n\
                 \x20 initial #1000000 $finish;\nendmodule\n"
            ),
        )
        .unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_vita"))
            .arg(f.to_str().unwrap())
            .current_dir(&d)
            .output()
            .expect("run vita");
        let all = String::from_utf8_lossy(&out.stdout).into_owned()
            + &String::from_utf8_lossy(&out.stderr);
        assert!(
            !all.contains("FIRED"),
            "`{ts}`: a negative delay must not fire (both oracles reach the watchdog):\n{all}"
        );
    }
}

#[test]
fn a_delay_with_no_time_literal_is_untouched() {
    // The opt-in gate is `expr_has_time_lit`, so this is the byte-identity claim:
    // a plain integer delay, a parameter delay and a zero delay never reach the
    // new route at all, region included.
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pdtl_z_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(
        &f,
        "`timescale 1ns/1ns\nmodule m;\n  localparam Z = 0;\n  localparam D = 4;\n\
         \x20 initial begin $timeformat(-15,0,\"\",20);\n\
         \x20   #(Z); $display(\"P1 %t\", $realtime);\n\
         \x20   #(D); $display(\"P2 %t\", $realtime);\n\
         \x20   #3;   $display(\"P3 %t\", $realtime);\n\
         \x20   #(D/2); $display(\"P4 %t\", $realtime);\n\
         \x20   $finish; end\nendmodule\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let all =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{all}");
    let vals: Vec<&str> = all
        .lines()
        .filter(|l| l.starts_with('P'))
        .filter_map(|l| l.split_whitespace().last())
        .collect();
    assert_eq!(vals, ["0", "4000000", "7000000", "9000000"], "{all}");
}

#[test]
fn the_two_lanes_of_one_delay_now_agree_on_the_rounding() {
    // `5ns / 2ns` under `1ps/1ps` is 2.5 ps, and the two oracles round it apart
    // (iverilog 3 ps, verilator 2 ps) — a documented split this slice does not
    // chase. What it fixes is that vita used to take DIFFERENT sides in its own
    // two lanes: the structural `assign #(…)` rounded to 3 (measured, PRE and
    // POST alike) while the procedural `#(…)` rounded to 2. Routing inherits the
    // structural lane's answer rather than re-deriving one, so both are 3 now.
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pdtl_s_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(
        &f,
        "`timescale 1ps/1ps\nmodule m;\n  logic a = 0, y;\n\
         \x20 assign #(5ns / 2ns) y = a;\n\
         \x20 initial begin $timeformat(-15,0,\"\",20); #1 a = 1; end\n\
         \x20 initial begin @(posedge y) $display(\"S %t\", $realtime); end\n\
         \x20 initial begin #(5ns / 2ns); $display(\"P %t\", $realtime); end\n\
         \x20 initial #1000000 $finish;\nendmodule\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let all =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{all}");
    // 1 ps of stimulus + 3 ps of delay for the structural edge; 3 ps for the
    // procedural one. iverilog answers exactly this pair.
    let get = |tag: &str| {
        all.lines()
            .find(|l| l.starts_with(tag))
            .and_then(|l| l.split_whitespace().last())
            .unwrap_or("")
            .to_string()
    };
    assert_eq!(get("S"), "4000", "structural:\n{all}");
    assert_eq!(get("P"), "3000", "procedural:\n{all}");
}
