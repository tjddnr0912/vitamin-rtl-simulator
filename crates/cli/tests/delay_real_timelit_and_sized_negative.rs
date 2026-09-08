//! Two structural-delay values that folded to NO DELAY, silently, at exit 0 —
//! ROADMAP §2 "Delays / events".
//!
//! * a REAL time literal (`assign #(2.5ns) y = a;`, and `#(3.0ns)` too):
//!   `delay_ticks_in_scope`'s `TimeLit` arm asked the INTEGER fold and returned
//!   `None` for the whole function when it declined, and the caller reads `None`
//!   as "no delay". Both oracles delay 3 ns and 3 ns.
//! * a negated SIZED literal (`#(-4'd1)`): a delay is a self-determined position
//!   read as UNSIGNED (§11.6), so it is 15 — `const_eval_u32`'s 32-bit
//!   `wrapping_neg` made it 4294967295 and the assign never fired at all.
//!
//! Every tick below was measured on iverilog 13.0 AND verilator 5.052; each
//! `always @(posedge …)` line is the tools' own output, copied. The rounding the
//! two agree on is at the module's TIME UNIT, not at its precision: `#(25ns)`
//! under `10ns/1ns` is 2.5 units and both delay THREE units (30 ns), which is
//! why the real lane converts to units before rounding.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(ts: &str, delays: &[&str]) -> Vec<String> {
    let mut src = format!("`timescale {ts}\nmodule top;\n  reg a;\n");
    for (i, d) in delays.iter().enumerate() {
        src.push_str(&format!("  wire y{i}; assign #({d}) y{i} = a;\n"));
    }
    src.push_str("  initial begin a = 0; #10 a = 1; #3000 $finish; end\n");
    for (i, _) in delays.iter().enumerate() {
        src.push_str(&format!(
            "  always @(posedge y{i}) $display(\"y{i}=%0t\", $time);\n"
        ));
    }
    src.push_str("endmodule\n");
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_drtl_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, &src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let all =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(0),
        "exit\n{all}\n--- src ---\n{src}"
    );
    let mut fired: Vec<String> = all
        .lines()
        .filter(|l| l.starts_with('y') && l.contains('='))
        .map(|l| l.to_string())
        .collect();
    fired.sort();
    fired
}

#[test]
fn a_real_time_literal_delays_at_the_modules_time_unit() {
    // `1ns/1ps`, edge at 10000 ps. Both oracles: 0.5→1, 1.5→2, 2.4→2, 2.5→3,
    // 2.6→3, and the integral `2.0`/`3.0` spellings 2 and 3. PRE fired every one
    // of them at 10000 (no delay at all).
    assert_eq!(
        run("1ns/1ps", &["0.5ns", "1.5ns", "2.4ns", "2.5ns", "2.6ns"]),
        ["y0=11000", "y1=12000", "y2=12000", "y3=13000", "y4=13000"]
    );
    assert_eq!(
        run("1ns/1ps", &["2.0ns", "3.0ns"]),
        ["y0=12000", "y1=13000"]
    );
    // The integer time literal and the unit-less real spelling of the same delay
    // are the controls: both were already correct and must not move.
    assert_eq!(run("1ns/1ps", &["3ns", "2.5"]), ["y0=13000", "y1=13000"]);
    // Rounding is at the module's UNIT: `2.5ns` under `10ns/1ns` is a quarter of a
    // unit and both oracles delay ZERO (the edge is at 100), while `25ns` is 2.5
    // units and both delay three.
    assert_eq!(run("10ns/1ns", &["2.5ns", "25ns"]), ["y0=100", "y1=130"]);
    // Sub-precision declines as it always did — `2.5ps` under `1ns/1ps` is 0.0025
    // units, and both oracles fire at once.
    assert_eq!(
        run("1ns/1ps", &["2.5ps", "0.4ns"]),
        ["y0=10000", "y1=10000"]
    );
}

#[test]
fn a_negated_sized_literal_delay_wraps_at_its_own_width() {
    // `1ns/1ns`, edge at 10. Both oracles: `-4'd1` = 15, `-(4'd1)` = 15,
    // `-8'sd1` = 255. PRE: none of the three ever fired.
    assert_eq!(
        run("1ns/1ns", &["-4'd1", "-(4'd1)", "-8'sd1"]),
        ["y0=25", "y1=25", "y2=265"]
    );
    // The UNSIZED spelling is 32 bits wide and keeps the value it had: neither
    // oracle fires it inside the run, and neither does vita.
    assert_eq!(run("1ns/1ns", &["-1", "5"]), ["y1=15"]);
    // Controls on the same axis, both already correct: a self-determined 4-bit sum
    // wraps to zero, and a positive sized literal is itself.
    assert_eq!(
        run("1ns/1ns", &["4'd15 + 4'd1", "4'd7"]),
        ["y0=10", "y1=17"]
    );
}
