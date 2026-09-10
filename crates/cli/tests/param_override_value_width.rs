//! IEEE 1800 §6.20.2 — an UNTYPED, unranged parameter takes the RANGE of its FINAL
//! override VALUE, and the value binds AT that range. ROADMAP §2 "Index sealing" row 2:
//! the self-determined (`ovr_bits`) twin of the operator lane `override_own_width_and_sign`
//! pins.
//!
//! PRE, `bind_one_param`'s meta chain already answered the override's width, so `$bits(P)`
//! was right — while the value resize twelve hundred lines below still read
//! `param_decl_width(p)`, the DEFAULT INITIALIZER's width, and overwrote the correct
//! parent-side fold with the truncation. One run reported `bits=33 val=3`. The cut is the
//! DEFAULT's width, not 32: `parameter P = 8'd1` cut at 8, `parameter P = 64'd1` did not
//! cut at all, and 32 is only what `parameter P = 1` happens to infer.
//!
//! ## Oracles
//!
//! Every expected value below was measured 3-way (vita / iverilog 13 + vvp / verilator
//! `--binary --timing`) at the slice; the two oracles agree on every cell in this file,
//! including the 65-bit one. Where a cell exists to prove a NON-move, that is said on the
//! line.
//!
//! ## What is deliberately NOT here
//!
//! - A DECLARED-width target (`parameter logic [39:0] P`) is a different lane carrying
//!   ROADMAP §2 rows 16/17's live oracle split. The consumer's `Implicit && range.is_none()`
//!   guard keeps this slice off it, and `a_declared_width_target_does_not_move` pins that.
//! - FORWARDING a parameter through a second `#()` level (ROADMAP §2 row 3) is a separate
//!   site (`params.rs`'s literal arm) and landed in the FOLLOWING slice;
//!   `param_override_forwarded_width.rs` is its census.
//!   `a_forwarded_override_value_is_repaired_and_so_is_its_width` keeps the interaction
//!   cell here, because the VALUE half of it is this slice's.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_povw_{}_{n}", std::process::id()));
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
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

fn lines(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter(|l| !l.starts_with("simulation ended") && !l.contains("VITA-W1017"))
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with("errors="))
        .collect()
}

const CHILD: &str =
    "module child #(parameter P = 1); initial $display(\"%m bits=%0d val=%0h\", $bits(P), P); endmodule\n";

/// The width ladder on `parameter P = 1`. 31 and 32 are the CONTROLS — they fit the
/// default's inferred 32 and were already right, so a rule that widens indiscriminately
/// would show up as them moving. 33/40/63/64 all printed `val=3`/`val=1` PRE.
#[test]
fn an_untyped_targets_override_value_keeps_the_overrides_own_width() {
    let (o, c) = run(&format!(
        "{CHILD}module top;\n\
        \x20 child #(.P(31'h7000_0003))           w31();\n\
        \x20 child #(.P(32'hF000_0003))           w32();\n\
        \x20 child #(.P(33'h1_0000_0003))         w33();\n\
        \x20 child #(.P(40'hAA_0000_0003))        w40();\n\
        \x20 child #(.P(63'h4000_0000_0000_0003)) w63();\n\
        \x20 child #(.P(64'hDEAD_BEEF_0000_0001)) w64();\n\
        \x20 initial #1 $finish;\nendmodule\n"
    ));
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "top.w31 bits=31 val=70000003", // CONTROL — ok PRE, must not move
            "top.w32 bits=32 val=f0000003", // CONTROL — ok PRE, must not move
            "top.w33 bits=33 val=100000003",
            "top.w40 bits=40 val=aa00000003",
            "top.w63 bits=63 val=4000000000000003",
            "top.w64 bits=64 val=deadbeef00000001",
        ]
    );
}

/// The >64-bit cell. PRE this was `bits=65 val=3` at exit 0 — silent-wrong, which refutes
/// the row's own claim that "§3.b wide-override records >64 as loud". POST, passing the
/// override's own width to `override_at_declared_width` makes `params.rs`'s
/// `(64..cv.width).any(bp_get)` test REACHABLE, so the value installs in `wide_param_bits`
/// where a >64-bit parameter already lives, and reads back whole. Both oracles agree.
#[test]
fn a_wider_than_64_bit_override_binds_its_whole_value() {
    let (o, c) = run(&format!(
        "{CHILD}module top;\n\
        \x20 child #(.P(65'h1_0000_0000_0000_0003)) w65();\n\
        \x20 initial #1 $finish;\nendmodule\n"
    ));
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(lines(&o), ["top.w65 bits=65 val=10000000000000003"]);
}

/// The cut width is the DEFAULT INITIALIZER's, not 32. `8'd1` cuts a 33-bit and a 64-bit
/// override at 8; `40'd1` is wide enough that the 33-bit cell was already right PRE (its
/// CONTROL role here); `64'd1` likewise for both. So the same override text bound four
/// different values depending only on the default it replaced.
#[test]
fn the_cut_width_was_the_defaults_not_thirty_two() {
    let (o, c) = run(
        "module c8  #(parameter P = 8'd1);  initial $display(\"%m bits=%0d val=%0h\", $bits(P), P); endmodule\n\
         module c40 #(parameter P = 40'd1); initial $display(\"%m bits=%0d val=%0h\", $bits(P), P); endmodule\n\
         module c64 #(parameter P = 64'd1); initial $display(\"%m bits=%0d val=%0h\", $bits(P), P); endmodule\n\
         module top;\n\
        \x20 c8  #(.P(33'h1_0000_0003))         a();\n\
        \x20 c8  #(.P(64'hDEAD_BEEF_0000_0001)) b();\n\
        \x20 c40 #(.P(33'h1_0000_0003))         c();\n\
        \x20 c64 #(.P(33'h1_0000_0003))         d();\n\
        \x20 initial #1 $finish;\nendmodule\n",
    );
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "top.a bits=33 val=100000003",
            "top.b bits=64 val=deadbeef00000001",
            "top.c bits=33 val=100000003", // CONTROL — ok PRE (40 > 33), must not move
            "top.d bits=33 val=100000003", // CONTROL — ok PRE (64 > 33), must not move
        ]
    );
}

/// The default WIDER than the override is the byte-identity argument's test case. PRE the
/// resize went UP to the default's 64 and `coerce_i64_to_width` brought it back to the meta
/// width; POST there is no resize at all. Same i64 either way — including the SIGNED
/// spelling, where `8'shFF` must still read −1. All four were ok PRE and must not move.
#[test]
fn a_narrower_override_than_the_default_does_not_move() {
    let (o, c) = run(
        "module c64 #(parameter P = 64'd1); initial $display(\"%m bits=%0d val=%0h sd=%0d\", $bits(P), P, P); endmodule\n\
         module c1  #(parameter P = 1);     initial $display(\"%m bits=%0d val=%0h sd=%0d\", $bits(P), P, P); endmodule\n\
         module top;\n\
        \x20 c64 #(.P(8'hFF))  a();\n\
        \x20 c64 #(.P(8'shFF)) b();\n\
        \x20 c1  #(.P(8'hFF))  c();\n\
        \x20 c1  #(.P(8'shFF)) d();\n\
        \x20 initial #1 $finish;\nendmodule\n",
    );
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "top.a bits=8 val=ff sd=255",
            "top.b bits=8 val=ff sd=-1",
            "top.c bits=8 val=ff sd=255",
            "top.d bits=8 val=ff sd=-1",
        ]
    );
}

/// Every SPELLING of the same 33-bit override reaches the same binding: named by literal,
/// named by a wide parameter, positional, signed, a concatenation, and a wildcard-imported
/// package parameter. `W33 + 0` is the CONTROL — the operator lane (§4.5.463/466) already
/// bound it right through `ovr_self_meta`, and it must not move.
#[test]
fn every_override_spelling_binds_the_same_value() {
    let (o, c) = run(&format!(
        "package pk; parameter logic [32:0] PW33 = 33'h1_0000_0003; endpackage\n\
         {CHILD}module top;\n\
         \x20 import pk::*;\n\
        \x20 parameter logic [32:0] W33 = 33'h1_0000_0003;\n\
        \x20 parameter logic [63:0] W64 = 64'hDEAD_BEEF_0000_0001;\n\
        \x20 child #(.P(W33))                    nmd();\n\
        \x20 child #(.P(W64))                    nmd64();\n\
        \x20 child #(.P(W33 + 0))                expr();\n\
        \x20 child #(.P({{1'b1, 32'h0000_0003}})) cat();\n\
        \x20 child #(33'h1_0000_0003)            posi();\n\
        \x20 child #(.P(33'sh1_0000_0003))       sgn();\n\
        \x20 child #(.P(PW33))                   pw();\n\
        \x20 initial #1 $finish;\nendmodule\n"
    ));
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "top.nmd bits=33 val=100000003",
            "top.nmd64 bits=64 val=deadbeef00000001",
            "top.expr bits=33 val=100000003", // CONTROL — the operator lane, ok PRE
            "top.cat bits=33 val=100000003",
            "top.posi bits=33 val=100000003",
            "top.sgn bits=33 val=100000003",
            "top.pw bits=33 val=100000003",
        ]
    );
}

/// A DECLARED-width target survives the override (§12.2.1) and is not this lane. `dcl` and
/// the defparam twin `h` below were both ok PRE; they pin that the new flag's
/// `Implicit && range.is_none()` guard keeps ROADMAP §2 rows 16/17 unmoved.
#[test]
fn a_declared_width_target_does_not_move() {
    let (o, c) = run(
        "module childd #(parameter logic [39:0] P = 1); initial $display(\"%m bits=%0d val=%0h\", $bits(P), P); endmodule\n\
         module top;\n\
        \x20 childd #(.P(33'h1_0000_0003)) dcl();\n\
        \x20 childd h();\n\
        \x20 defparam h.P = 33'h1_0000_0003;\n\
        \x20 initial #1 $finish;\nendmodule\n",
    );
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "top.dcl bits=40 val=100000003",
            "top.h bits=40 val=100000003",
        ]
    );
}

/// The `defparam` channel is the same defect through a different collector: `instance.rs`
/// hard-coded `bits: None` on its `ResolvedOverride`, so a defparam had NO wide channel at
/// all and lost the `$bits` column too (`bits=32 val=3` PRE, where `#()` at least reported
/// 33). POST both columns are the value both oracles print.
///
/// `g` is the CONTROL: `override_bits` declines every operator top, so `~8'h5A` still takes
/// the `self_meta` route §4.5.463 gave it and was ok PRE.
#[test]
fn a_defparam_override_carries_its_own_width_too() {
    let (o, c) = run(&format!(
        "{CHILD}module top;\n\
        \x20 child e(); defparam e.P = 33'h1_0000_0003;\n\
        \x20 child f(); defparam f.P = 64'hDEAD_BEEF_0000_0001;\n\
        \x20 child g(); defparam g.P = ~8'h5A;\n\
        \x20 initial #1 $finish;\nendmodule\n"
    ));
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "top.e bits=33 val=100000003",
            "top.f bits=64 val=deadbeef00000001",
            "top.g bits=8 val=a5", // CONTROL — the operator defparam lane, ok PRE
        ]
    );
}

/// The interaction with ROADMAP §2 row 3 (forwarding), which is a DIFFERENT site.
///
/// `Q`'s OWN value inside `m1` is this slice's: PRE it was truncated to the default
/// `1'd1`'s single bit and printed `Q=1` where both oracles print `3`. POST it is `3`, and
/// the values forwarded into the leaf follow it (`3` and `~3`).
///
/// The leaf's WIDTH was row 3's and is now repaired too: `params.rs`'s untyped-tail literal
/// arm no longer reports the DEFAULT literal's width under `declared_only` once an override
/// has reached the declaration, so `param_range[Q]` agrees with `param_meta[Q]` and
/// `narrow_param_bits` stops declining. Both oracles bind these at 4 bits.
///
/// This cell stays here — rather than moving wholesale into the forwarding census — because
/// the ORDER of the two slices is what it records: row 3 alone would have forwarded the
/// TRUNCATED `1`, i.e. traded one silent-wrong for another.
#[test]
fn a_forwarded_override_value_is_repaired_and_so_is_its_width() {
    let (o, c) = run(
        "module leaf #(parameter P = 1); initial $display(\"%m bits=%0d val=%0h\", $bits(P), P); endmodule\n\
         module m1 #(parameter Q = 1'd1);\n\
        \x20 initial $display(\"%m Qbits=%0d Q=%0h\", $bits(Q), Q);\n\
        \x20 leaf #(.P(Q))  b();\n\
        \x20 leaf #(.P(~Q)) n();\n\
         endmodule\n\
         module top; m1 #(.Q(4'd3)) a(); initial #1 $finish; endmodule\n",
    );
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "top.a Qbits=4 Q=3", // this slice: PRE `Q=1`. Both oracles `3`.
            // The forwarding slice: PRE `bits=32`. Both oracles bind 4 bits here.
            "top.a.b bits=4 val=3",
            "top.a.n bits=4 val=c",
        ]
    );
}
