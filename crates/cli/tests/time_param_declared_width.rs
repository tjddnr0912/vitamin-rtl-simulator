//! A `time` parameter is 64 bits UNSIGNED because it is DECLARED so (IEEE 1800 §6.11.2),
//! never because of what its initializer happens to be.
//!
//! `params.rs::param_decl_width_opt` answered `Integer` with a declared `(32, p.signed)`
//! and had no `Time` arm, so a `time` parameter fell through to the untyped/§6.20.2 tail
//! — whose SIZED-literal case answers for "any reaching param type" and hands back the
//! LITERAL's width. The tail's own comment already stated the right rule for the DECIMAL
//! case ("a `time` param's width is its declared 64-bit type, not the literal's") and
//! left the SIZED case unguarded.
//!
//! Both halves were wrong and only one of them shows in `$bits`:
//!   * width — `localparam time A = 8'd5` recorded 8 bits, both oracles 64;
//!   * sign  — `localparam time B = -8'sd2` PRINTED `-2` at 8 signed bits where both
//!     oracles print `18446744073709551614`, because `time` is unsigned.
//!
//! Every cell below was measured 3-way at the slice (iverilog 13.0 `-g2012`, verilator
//! 5.052 `--binary --timing`); the two oracles agree on all of them and the asserted text
//! IS their output. Four sibling containers already carried this rule
//! (`hdl-parser/src/params.rs`, `cover_bins.rs`, `inline_fn.rs` twice) — this function
//! was the missing container, which is the shape ENGINEERING_RULES records for a
//! re-spell pass that has as many sites as the type has containers.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_tpw_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let r = (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    );
    let _ = std::fs::remove_dir_all(&d);
    r
}

/// The design's `$display` lines, trimmed — the simulator's own epilogue
/// ("simulation ended …") is on stdout too and is not part of the pinned value.
fn lines(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter(|l| !l.starts_with("simulation ended"))
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// The four declaration spellings, plus the header-override channel.
///
/// `TD = 64'd5` is the control: its literal is ALREADY 64 bits, so it was correct before
/// the arm existed and must not move. Without it the other three could be "fixed" by any
/// rule that widens everything, rather than by the declared type.
#[test]
fn a_time_parameter_is_64_bits_whatever_its_initializer_is() {
    let (o, e, c) = run("module top;
           localparam time A = 8'd5;
           localparam time B = -8'sd2;
           localparam time D = 8'd5 + 8'd3;
           localparam time TD = 64'd5;
           parameter  time TC = 8'd5;
           initial begin
             $display(\"A bits=%0d hex=%h dec=%0d\", $bits(A), A, A);
             $display(\"B bits=%0d hex=%h dec=%0d\", $bits(B), B, B);
             $display(\"D bits=%0d hex=%h dec=%0d\", $bits(D), D, D);
             $display(\"TD bits=%0d hex=%h dec=%0d\", $bits(TD), TD, TD);
             $display(\"TC bits=%0d hex=%h dec=%0d\", $bits(TC), TC, TC);
             #1 $finish;
           end
         endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(
        lines(&o),
        [
            "A bits=64 hex=0000000000000005 dec=5",
            // the sign half: `-2` here was the silent-wrong, not a narrower `$bits`
            "B bits=64 hex=fffffffffffffffe dec=18446744073709551614",
            "D bits=64 hex=0000000000000008 dec=8",
            "TD bits=64 hex=0000000000000005 dec=5",
            "TC bits=64 hex=0000000000000005 dec=5",
        ]
    );
}

/// The width must survive an OVERRIDE — §6.20.2 gives the override's own type to an
/// UNTYPED parameter, and `time` is not untyped, so the declared 64 wins over the
/// override literal's 16 exactly as it wins over the default literal's 8.
#[test]
fn an_override_does_not_narrow_a_time_parameter() {
    let (o, e, c) = run("module sub #(parameter time TP = 8'd5) ();
           initial $display(\"TP bits=%0d hex=%h dec=%0d\", $bits(TP), TP, TP);
         endmodule
         module top; sub #(.TP(16'd9)) u(); initial #10 $finish; endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["TP bits=64 hex=0000000000000009 dec=9"]);
}

/// The discriminator: TWO instances of one module, one overridden and one not, in a
/// single run.
///
/// PRE answered `$bits(T)` = **16** for the overridden instance (the override literal's
/// width) and **32** for the un-overridden one (the untyped tail's `.max(32)`) — one
/// declaration, two widths, neither of them 64, inside one elaboration. Any pin that
/// looks at a single instance can be satisfied by a rule that is merely differently
/// wrong; this one cannot.
///
/// `Qb` is where the two oracles part, and vita lands on the self-consistent one:
/// verilator says `$bits(T + 1)` is 64, iverilog says 65 — the same `+`-grows-a-bit
/// self-contradiction ROADMAP §2 records for iverilog's parameter binding (its own
/// inline `$bits` of the identical expression is 64). Every other column is 3-way.
#[test]
fn one_time_parameter_has_one_width_whether_or_not_it_is_overridden() {
    let (o, e, c) = run("module sub #(parameter time T = 5) ();
           logic [T-1:0] w;
           localparam Q = T + 1;
           initial $display(\"T=%0d bits=%0d wbits=%0d Q=%0d Qb=%0d shr=%0d\",
                            T, $bits(T), $bits(w), Q, $bits(Q), T >>> 1);
         endmodule
         module top;
           sub #(.T(16'd9)) u();
           sub              v();
           initial #10 $finish;
         endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(
        lines(&o),
        [
            "T=9 bits=64 wbits=9 Q=10 Qb=64 shr=4",
            "T=5 bits=64 wbits=5 Q=6 Qb=64 shr=2",
        ]
    );
}

/// A FILL override re-folds at the target's declared width, so it is the cell that reads
/// the new arm's answer directly rather than through a literal.
///
/// Measured identical in both oracles for this source spelling, and in verilator's
/// `-GT='1` for the CLI twin that `implicit_param_ports.rs` pins.
#[test]
fn a_fill_override_folds_at_the_time_parameters_declared_width() {
    let (o, e, c) = run("module sub #(parameter time T = 5) ();
           initial $display(\"T=%0d bits=%0d\", T, $bits(T));
         endmodule
         module top; sub #(.T('1)) u(); initial #10 $finish; endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["T=18446744073709551615 bits=64"]);
}

/// The control that keeps the arm from being a blanket widening: `integer`/`int` stay 32
/// and untyped parameters keep taking their type from the VALUE (§6.20.2). If the `Time`
/// arm were reached for these, every one of them would read 64.
#[test]
fn the_neighbouring_param_types_are_unmoved() {
    let (o, e, c) = run("module top;
           localparam integer I = 8'd5;
           localparam int     N = 8'd5;
           localparam         U = 8'd5;
           localparam         V = 5;
           initial begin
             $display(\"I=%0d N=%0d U=%0d V=%0d\",
                      $bits(I), $bits(N), $bits(U), $bits(V));
             #1 $finish;
           end
         endmodule");
    assert_eq!(c, Some(0), "stderr: {e}");
    assert_eq!(lines(&o), ["I=32 N=32 U=8 V=32"]);
}
