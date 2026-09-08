//! A time literal inside an EXPRESSION as a structural delay — ROADMAP §2
//! "Delays / events", the residue left by §4.5.457's bare-`TimeLit` arm.
//!
//! `assign #(2.5ns + 1ns) y = a;` fired at once, at exit 0, with no diagnostic,
//! where both oracles delay 3.5 ns. The bare arm only fires when the delay IS a
//! `TimeLit`, so a single paren already defeated it; everything else fell to the
//! integer lane, whose `TimeLit` handling declines on either a non-integral value
//! or a literal that is not a whole multiple of the module's time unit.
//!
//! ⚠️ "A REAL time literal" is the wrong name for the class and ROADMAP said so:
//! `#(2500ps + 1000ps)` has no real anywhere and was equally silent, and so was
//! `#(1ns + 1ns)` under a `10ns/1ns` module. The discriminator is a FRACTIONAL
//! module-unit value, not a real one.
//!
//! ⚠️ The integer lane does not merely decline here — it answers WRONG, which is
//! why the new lane is asked BEFORE it and not after: `#(3ns / 2)` folded integer
//! division to 1 ns where both oracles delay 1.5 ns.
//!
//! Every tick below is the tools' own output, probed with `$realtime` at the
//! design's full precision. A `$time` probe cannot see these cells: it rounds to
//! the module's TIME UNIT, which is what hid the true values from this file's
//! sibling (see `delay_real_timelit_and_sized_negative.rs`).
//!
//! Measured over 215 cells (43 delay spellings x 5 timescales), PRE vs POST vs
//! iverilog: 81 fixed, 0 regressed. Of the 19 that still differ from iverilog,
//! all three groups are recorded in ROADMAP §2 and none is caused by this file:
//!
//! * 11 are the ROUNDING SPLIT that only exists where the module's PRECISION
//!   equals its UNIT. There iverilog rounds each time literal at its own leaf
//!   (`2 * 2.5ns` = 6 ns, `2.5ns + 1.5ns` = 5 ns) and verilator rounds once on the
//!   finished value (5 ns, 4 ns). vita answers verilator's, which is the rule that
//!   is also right at every FINER precision — where both oracles agree with it.
//!   PRE fired all eleven at once, so every one moved from wrong-on-both to
//!   right-on-one; the ROADMAP row's instruction not to fold the two into one rule
//!   is what keeps them out of the assertions above.
//! * 4 are a literal whose UNIT is finer than the design's precision but whose
//!   VALUE is not: `#(2500ps)` under `1ns/1ns` is 3 ns in both oracles and no
//!   delay here, bare spelling included. That is the shipped bare arm's `e < 0`
//!   decline, pre-existing and untouched.
//! * 4 are a NEGATIVE delay (`#(1ns - 5ns)`): neither oracle ever fires it and
//!   both PRE and POST fire at once, because the tick field clamps at zero.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// One delay per wire, probed with `$realtime` in `fs` so nothing is rounded away.
/// The edge is at 10 module units; each returned string is `y<i>=<fs>`.
fn run(ts: &str, delays: &[&str]) -> Vec<String> {
    let mut src = format!("`timescale {ts}\nmodule top;\n  parameter P = 3;\n  reg a;\n");
    for (i, d) in delays.iter().enumerate() {
        src.push_str(&format!("  wire y{i}; assign #({d}) y{i} = a;\n"));
    }
    src.push_str("  initial begin a = 0; #10 a = 1; #3000 $finish; end\n");
    src.push_str("  initial $timeformat(-15, 0, \"\", 1);\n");
    for (i, _) in delays.iter().enumerate() {
        src.push_str(&format!(
            "  always @(posedge y{i}) $display(\"y{i}=%0t\", $realtime);\n"
        ));
    }
    src.push_str("endmodule\n");
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dtle_{}_{n}", std::process::id()));
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
fn a_time_literal_in_an_arithmetic_expression_is_a_delay() {
    // `1ns/1ps`, edge at 10_000_000 fs. Both oracles: 3.5 ns, 3.5 ns (operand
    // order is irrelevant), 4 ns, 1.5 ns. PRE fired every one at the edge.
    assert_eq!(
        run(
            "1ns/1ps",
            &["2.5ns + 1ns", "1ns + 2.5ns", "2.5ns + 1.5ns", "2.5ns - 1ns"]
        ),
        ["y0=13500000", "y1=13500000", "y2=14000000", "y3=11500000"]
    );
    // One paren is enough to leave the bare arm — this cell is not in the ROADMAP
    // row and was silent for the same reason.
    assert_eq!(
        run("1ns/1ps", &["(2.5ns)", "-(-2.5ns)"]),
        ["y0=12500000", "y1=12500000"]
    );
    // Rounding happens ONCE, on the finished sum. Per-leaf rounding would answer
    // 6 ns and 6 ns here; both oracles answer 5 ns and 5 ns.
    assert_eq!(
        run("1ns/1ps", &["2.5ns + 2.5ns", "2 * 2.5ns"]),
        ["y0=15000000", "y1=15000000"]
    );
    // A unit-less operand is already a count of MODULE UNITS, which is what makes
    // `2.5ns + 1` 3.5 ns and not 2.5 ns + 1 ps.
    assert_eq!(
        run("1ns/1ps", &["2.5ns + 1", "2.5ns + 0", "(2.5ns + 1ns) * 2"]),
        ["y0=13500000", "y1=12500000", "y2=17000000"]
    );
}

#[test]
fn the_class_is_a_fractional_unit_value_and_not_a_real_one() {
    // No real anywhere: two INTEGER time literals, each a whole number of
    // picoseconds, summing to 3.5 module units. Both oracles 3.5 ns; PRE fired at
    // the edge. This is the cell that renames the row.
    assert_eq!(run("1ns/1ps", &["2500ps + 1000ps"]), ["y0=13500000"]);
    // Same shape under a module whose UNIT is coarser than its precision: `1ns`
    // is a tenth of a unit, so the integer lane declined both of these too, with
    // no real and no fraction in the source text at all. Edge at 100 ns.
    assert_eq!(
        run("10ns/1ns", &["1ns + 1ns", "25ns + 25ns"]),
        ["y0=102000000", "y1=150000000"]
    );
    // And the timescale sweep of the row's own cell: 3.5 ns rounds at the module's
    // PRECISION, so it survives whole at 1 ps and becomes 4 ns where the precision
    // is 1 ns. (`10ns/1ns`: edge at 100 ns.)
    assert_eq!(run("1ns/1ns", &["2.5ns + 1ns"]), ["y0=14000000"]);
    assert_eq!(run("10ns/1ns", &["2.5ns + 1ns"]), ["y0=104000000"]);
}

#[test]
fn a_delay_divides_as_a_magnitude_and_not_as_an_integer() {
    // The reason the new lane is asked BEFORE the integer lane. These three were
    // not silent — they were WRONG, folded with integer division at exit 0. Both
    // oracles: 1.25 ns, 0.5 ns, 1.5 ns, 2.5 ns.
    assert_eq!(
        run("1ns/1ps", &["2.5ns/2", "1ns / 2", "3ns / 2", "5ns / 2ns"]),
        ["y0=11250000", "y1=10500000", "y2=11500000", "y3=12500000"]
    );
    // A ternary over a time comparison picks the branch, and both oracles delay
    // the 3 ns arm.
    assert_eq!(
        run("1ns/1ps", &["2.5ns > 1ns ? 3ns : 4ns"]),
        ["y0=13000000"]
    );
}

#[test]
fn the_lanes_that_already_answered_keep_their_answers() {
    // Whole-unit time literals in a tree were already correct through the integer
    // lane and must not move: 2 ns, 3 ns, 3 ns (a parameter operand).
    assert_eq!(
        run("1ns/1ps", &["1ns + 1ns", "1ns * 3", "P * 1ns"]),
        ["y0=12000000", "y1=13000000", "y2=13000000"]
    );
    // `%` is a remainder of two MAGNITUDES. Where both operands are whole module
    // units the integer lane already answered it and the value does not move (2 ns);
    // where neither is — `10ns/1ns`, so 5 ns is half a unit — the integer lane
    // declined to no delay and iverilog delays 2 ns. verilator refuses `%` on a
    // time, so these two cells are iverilog + §11.4.3.
    assert_eq!(
        run("1ns/1ps", &["5ns % 3ns", "5.5ns % 2ns"]),
        ["y0=12000000", "y1=11500000"]
    );
    assert_eq!(run("10ns/1ns", &["5ns % 3ns"]), ["y0=102000000"]);
    // A delay with NO time literal never reaches the new lane at all. A
    // self-determined 4-bit sum wraps to zero (fires at the edge), a negated sized
    // literal is read unsigned at its own width (15 and 255), and the unit-less
    // real spelling is unchanged.
    assert_eq!(
        run("1ns/1ns", &["4'd15 + 4'd1", "-4'd1", "-8'sd1", "2.5"]),
        ["y0=10000000", "y1=25000000", "y2=265000000", "y3=13000000"]
    );
    // A sub-precision literal still declines, and the bare-`TimeLit` arm still
    // owns the bare spelling — `2.5ps` under `1ns/1ps` is a ROUNDING-TIE SPLIT
    // (iverilog 3 ps, verilator 2 ps) and vita keeps iverilog's answer. The new
    // lane sits after that arm precisely so this cell cannot move.
    assert_eq!(
        run("1ns/1ps", &["2.5ps", "0.4ns"]),
        ["y0=10003000", "y1=10400000"]
    );
}

#[test]
fn the_other_two_structural_spellings_share_the_fix() {
    // A net-declaration delay and a gate primitive desugar into the same
    // continuous-assign funnel, so both were silent and both are 3.5 ns now.
    // (verilator is the only oracle for the net-delay spelling — iverilog says
    // "sorry: Net delays not supported"; the gate primitive is 2-oracle.)
    let src = "`timescale 1ns/1ps\nmodule top;\n  reg a;\n  \
               wire #(2.5ns + 1ns) w = a;\n  wire g;\n  \
               buf #(2.5ns + 1ns) gg(g, a);\n  \
               initial begin a = 0; #10 a = 1; #3000 $finish; end\n  \
               initial $timeformat(-15, 0, \"\", 1);\n  \
               always @(posedge w) $display(\"y0=%0t\", $realtime);\n  \
               always @(posedge g) $display(\"y1=%0t\", $realtime);\nendmodule\n";
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dtle_f_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let all =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "{all}");
    assert!(all.contains("y0=13500000"), "net delay\n{all}");
    assert!(all.contains("y1=13500000"), "gate primitive\n{all}");
}

#[test]
fn the_procedural_twin_stays_loud() {
    // `#(2.5ns + 1ns) x = 1;` is honest-loud (E3009) and must stay that way: that
    // path lowers the amount as an expression the engine evaluates at suspension
    // time, and the new lane is deliberately not wired into it. Both oracles do
    // delay it 3.5 ns, so this is a §3 row, not a silent-wrong.
    let src = "`timescale 1ns/1ps\nmodule top;\n  logic x;\n  \
               initial begin #(2.5ns + 1ns) x = 1; end\n  \
               initial #100 $finish;\nendmodule\n";
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_dtle_p_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let all =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert_ne!(out.status.code(), Some(0), "{all}");
    assert!(all.contains("VITA-E3009"), "{all}");
}
