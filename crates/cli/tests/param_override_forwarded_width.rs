//! IEEE 1800 §6.20.2 — an untyped, unranged parameter takes the RANGE of its FINAL
//! override value, and FORWARDING that parameter into a child's `#()` must carry that
//! range with it. ROADMAP §2 "Index sealing" row 3.
//!
//! PRE, `params.rs`'s untyped-tail literal arm answered the DEFAULT initializer's width
//! even under `declared_only` on the OVERRIDDEN lane, so `param_decl_range_opt` recorded
//! the default's width in `param_range` while `bind_one_param`'s meta chain recorded the
//! OVERRIDE's in `param_meta`. `narrow_param_bits` requires the two to agree, declined on
//! the mismatch, and both forwarding channels (`override_self_meta` for an operator top,
//! `override_bits` for a self-determined one) lost their width; `bind_one_param`'s `else`
//! then answered the LEAF's own default. `mid #(parameter Q = 8'd1)` overridden
//! `#(.Q(4'd3))` forwarding `leaf #(.P(~Q))` printed `bits=32 val=fffffffc` where both
//! oracles print `bits=4 val=c`, and the wrong width CASCADED down a three-level chain at
//! each middle module's own default width (`8/fc`, not `32/…`).
//!
//! `$bits(Q)` INSIDE the middle module was already right PRE — it reads `param_meta`, the
//! other half of the pair — which is what made this a width-only silent-wrong rather than
//! a visible one.
//!
//! ## Oracles
//!
//! Every expected value below was measured 3-way (vita / iverilog 13 + vvp / verilator
//! `--binary --timing`). The two oracles agree on every cell here EXCEPT the bound `+`
//! ones: iverilog sizes a parameter-bound `Q+1` at max+1 (33/65) while its own
//! `$bits(Q+1)` answers max, so it contradicts itself and is NON-EVIDENCE there (§4.5.466).
//! verilator arbitrates those and vita matches it. Lines that exist to prove a NON-move
//! say so.
//!
//! ## Residues pinned here, deliberately NOT repaired
//!
//! - A parent whose declared range is ASCENDING (`[0:11]`) or has a NON-ZERO LSB
//!   (`[15:4]`) still forwards at 32: `narrow_param_bits` declines `lo != 0 || ascending`
//!   outright (ROADMAP §2 🆕 H ⓑ). That decline is upstream of this slice's pair and was
//!   left alone.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_pofw_{}_{n}", std::process::id()));
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

const LEAF: &str =
    "module leaf #(parameter P = 1); initial $display(\"%m bits=%0d val=%0h\", $bits(P), P); endmodule\n";

/// The five forwarded expression shapes over three override widths plus the un-overridden
/// control, on `mid #(parameter Q = 8'd1)`.
///
/// - `~Q` and `Q` are the two channels — the operator top goes through `override_self_meta`
///   and the bare name through `override_bits`. Both printed 32 PRE for every override.
/// - `Q+1` is a CONTEXT-determined top: vita keeps 32 at 4 and 16 bits (verilator agrees;
///   iverilog's 33 is NON-EVIDENCE) and moves to 64 at a 64-bit override, which is
///   verilator's answer and was 32 PRE.
/// - `{Q,Q}` is self-determined: 8 at a 4-bit override (was 32), 128 at a 64-bit one (was
///   the E3009 refusal below), 16 un-overridden.
/// - `$bits(Q)` reads `param_meta` and is 32 by §20.6 whatever Q is — it must not move.
#[test]
fn forwarding_an_overridden_untyped_parameter_carries_the_overrides_width() {
    let (o, c) = run(&format!(
        "{LEAF}module mid #(parameter Q = 8'd1);\n\
        \x20 initial $display(\"%m Q bits=%0d val=%0h\", $bits(Q), Q);\n\
        \x20 leaf #(.P(~Q))       n();\n\
        \x20 leaf #(.P(Q))        b();\n\
        \x20 leaf #(.P(Q+1))      p();\n\
        \x20 leaf #(.P({{Q,Q}}))    c();\n\
        \x20 leaf #(.P($bits(Q))) s();\n\
         endmodule\n\
         module top;\n\
        \x20 mid #(.Q(4'd3))  a();\n\
        \x20 mid #(.Q(16'd3)) b();\n\
        \x20 mid #(.Q(64'd3)) c();\n\
        \x20 mid              d();\n\
        \x20 initial #1 $finish;\nendmodule\n"
    ));
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "top.a Q bits=4 val=3",  // CONTROL — `param_meta`, right PRE
            "top.a.n bits=4 val=c",  // PRE 32/fffffffc
            "top.a.b bits=4 val=3",  // PRE 32/3
            "top.a.p bits=32 val=4", // CONTROL — verilator 32; iverilog 33 NON-EVIDENCE
            "top.a.c bits=8 val=33", // PRE 32/33
            "top.a.s bits=32 val=4", // CONTROL — `$bits` is always 32 bits wide
            "top.b Q bits=16 val=3",
            "top.b.n bits=16 val=fffc",  // PRE 32/fffffffc
            "top.b.b bits=16 val=3",     // PRE 32/3
            "top.b.p bits=32 val=4",     // CONTROL — verilator 32
            "top.b.c bits=32 val=30003", // CONTROL — ok PRE, but only BY ACCIDENT (16+16)
            "top.b.s bits=32 val=10",
            "top.c Q bits=64 val=3",
            "top.c.n bits=64 val=fffffffffffffffc", // PRE 32/fffffffc
            "top.c.b bits=64 val=3",                // PRE 32/3
            "top.c.p bits=64 val=4", // PRE 32/4; verilator 64, iverilog 65 NON-EVIDENCE
            "top.c.c bits=128 val=30000000000000003", // PRE the E3009 refusal
            "top.c.s bits=32 val=40",
            "top.d Q bits=8 val=1",    // CONTROL — no override anywhere below
            "top.d.n bits=8 val=fe",   // CONTROL — ok PRE
            "top.d.b bits=8 val=1",    // CONTROL — ok PRE
            "top.d.p bits=32 val=2",   // CONTROL — ok PRE (verilator 32)
            "top.d.c bits=16 val=101", // CONTROL — ok PRE
            "top.d.s bits=32 val=8",   // CONTROL — ok PRE
        ]
    );
}

/// `{Q,Q}` alone, with the 12-bit TWIN that gives the 16-bit cell teeth.
///
/// `.Q(16'd3)` was `bits=32` PRE and 32 is also the right answer — 16+16 — so that cell
/// cannot detect anything on its own. `.Q(12'd3)` must be 24, and PRE it was 32 like every
/// other forwarded concatenation. The 64-bit cell is the delta-limiter: PRE the whole run
/// died with E3009 ("the override of parameter `P` is not a constant"), because the 128-bit
/// fold was unreachable while the width was missing; it must land on the oracles' value,
/// never on a truncated one at exit 0.
#[test]
fn a_forwarded_concatenation_is_as_wide_as_the_override_twice() {
    let (o, c) = run(&format!(
        "{LEAF}module mid #(parameter Q = 8'd1); leaf #(.P({{Q,Q}})) c(); endmodule\n\
         module top;\n\
        \x20 mid #(.Q(4'd3))  a();\n\
        \x20 mid #(.Q(12'd3)) t();\n\
        \x20 mid #(.Q(16'd3)) s();\n\
        \x20 mid #(.Q(64'd3)) w();\n\
        \x20 mid              d();\n\
        \x20 initial #1 $finish;\nendmodule\n"
    ));
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "top.a.c bits=8 val=33",                  // PRE 32/33
            "top.t.c bits=24 val=3003",               // the TWIN — PRE 32/3003
            "top.s.c bits=32 val=30003",              // ok PRE, and ok BY ACCIDENT (16+16)
            "top.w.c bits=128 val=30000000000000003", // PRE the whole run was E3009
            "top.d.c bits=16 val=101",                // CONTROL — no override
        ]
    );
}

/// The default width the override REPLACES does not matter — 1, 8, 32 or 64, an override
/// BELOW it or ABOVE it, the forwarded width is the override's.
///
/// PRE every one of these sixteen cells printed 32, because the leaf fell back to its OWN
/// `parameter P = 1`. That is why `m32`'s cells looked right: 32 by coincidence of the
/// leaf's default, not of the parent's.
#[test]
fn the_replaced_default_width_does_not_reach_the_child() {
    let (o, c) = run(&format!(
        "{LEAF}\
         module m1  #(parameter Q = 1'd1);  leaf #(.P(~Q)) n(); leaf #(.P(Q)) b(); endmodule\n\
         module m8  #(parameter Q = 8'd1);  leaf #(.P(~Q)) n(); leaf #(.P(Q)) b(); endmodule\n\
         module m32 #(parameter Q = 32'd1); leaf #(.P(~Q)) n(); leaf #(.P(Q)) b(); endmodule\n\
         module m64 #(parameter Q = 64'd1); leaf #(.P(~Q)) n(); leaf #(.P(Q)) b(); endmodule\n\
         module mD  #(parameter Q = 1);     leaf #(.P(~Q)) n(); leaf #(.P(Q)) b(); endmodule\n\
         module top;\n\
        \x20 m1  #(.Q(4'd3))  a1(); m1  #(.Q(16'd3)) b1();\n\
        \x20 m8  #(.Q(4'd3))  a8(); m8  #(.Q(16'd3)) b8();\n\
        \x20 m32 #(.Q(4'd3))  a3(); m32 #(.Q(64'd3)) b3();\n\
        \x20 m64 #(.Q(4'd3))  a6(); m64 #(.Q(16'd3)) b6();\n\
        \x20 mD  #(.Q(4'd3))  ad();\n\
        \x20 initial #1 $finish;\nendmodule\n"
    ));
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "top.a1.n bits=4 val=c",
            "top.a1.b bits=4 val=3",
            "top.b1.n bits=16 val=fffc",
            "top.b1.b bits=16 val=3",
            "top.a8.n bits=4 val=c",
            "top.a8.b bits=4 val=3",
            "top.b8.n bits=16 val=fffc",
            "top.b8.b bits=16 val=3",
            "top.a3.n bits=4 val=c",
            "top.a3.b bits=4 val=3",
            "top.b3.n bits=64 val=fffffffffffffffc",
            "top.b3.b bits=64 val=3",
            "top.a6.n bits=4 val=c",
            "top.a6.b bits=4 val=3",
            "top.b6.n bits=16 val=fffc",
            "top.b6.b bits=16 val=3",
            // the UNSIZED-decimal default, `parameter Q = 1` — the arm's other half
            "top.ad.n bits=4 val=c",
            "top.ad.b bits=4 val=3",
        ]
    );
}

/// Three levels. PRE the wrong width did not stop at the first hop: it cascaded, and at
/// each hop it was that middle module's OWN default (`8/fc`, `8/3`), which is the signature
/// that says the leaf is answering from its own declaration rather than from anything the
/// parent sent. `mid2 #(.S(~Q))` forwards an operator top through the second level, so both
/// channels are exercised at both hops.
#[test]
fn the_overrides_width_survives_three_levels() {
    let (o, c) = run(
        "module leaf #(parameter R = 1); initial $display(\"%m bits=%0d val=%0h\", $bits(R), R); endmodule\n\
         module mid2 #(parameter S = 8'd1); leaf #(.R(~S)) n(); leaf #(.R(S)) b(); endmodule\n\
         module mid #(parameter Q = 8'd1); mid2 #(.S(Q)) f(); mid2 #(.S(~Q)) g(); endmodule\n\
         module top; mid #(.Q(4'd3)) a(); initial #1 $finish; endmodule\n",
    );
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "top.a.f.n bits=4 val=c", // PRE 8/fc — mid2's own default, one level down
            "top.a.f.b bits=4 val=3", // PRE 8/3
            "top.a.g.n bits=4 val=3", // PRE 8/3
            "top.a.g.b bits=4 val=c", // PRE 8/fc
        ]
    );
}

/// The override SPELLINGS. All four reach the same forwarded width, and all four printed 32
/// PRE.
///
/// - `#(.Q(4'sd3))` — a SIGNED override literal. The width is the override's; `~Q` is `c`,
///   so the sign does not leak into the forwarded bits here.
/// - `#(4'd3)` — POSITIONAL.
/// - `defparam dd.Q = 4'd3` — the defparam collector writes no `ovr.bits`, so this cell is
///   the one that proves the range fallback had to be keyed on `meta` (which every override
///   channel agrees on) rather than on the wide channel alone.
/// - `#(.Q(~8'h5A))` — an OPERATOR top as the override, arriving through `ovr_self_meta`
///   with no `ovr.bits` either. It was CORRECT PRE, by the accident that the default `8'd1`
///   happened to be as wide as the override; it is the reason the `meta` fallback exists,
///   and it must not move.
#[test]
fn every_override_spelling_forwards_the_same_width() {
    let (o, c) = run(&format!(
        "{LEAF}\
         module ms #(parameter Q = 8'd1); leaf #(.P(~Q)) n(); leaf #(.P(Q)) b(); endmodule\n\
         module mp #(parameter Q = 8'd1); leaf #(.P(~Q)) n(); leaf #(.P(Q)) b(); endmodule\n\
         module md #(parameter Q = 8'd1); leaf #(.P(~Q)) n(); leaf #(.P(Q)) b(); endmodule\n\
         module mo #(parameter Q = 8'd1); leaf #(.P(~Q)) n(); leaf #(.P(Q)) b(); endmodule\n\
         module top;\n\
        \x20 ms #(.Q(4'sd3))  s();\n\
        \x20 mp #(4'd3)       p();\n\
        \x20 md               dd(); defparam dd.Q = 4'd3;\n\
        \x20 mo #(.Q(~8'h5A)) o();\n\
        \x20 initial #1 $finish;\nendmodule\n"
    ));
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "top.s.n bits=4 val=c", // signed override — PRE 32/fffffffc
            "top.s.b bits=4 val=3",
            "top.p.n bits=4 val=c", // positional — PRE 32/fffffffc
            "top.p.b bits=4 val=3",
            "top.dd.n bits=4 val=c", // defparam — PRE 32/fffffffc
            "top.dd.b bits=4 val=3",
            "top.o.n bits=8 val=5a", // CONTROL — operator-top override, ok PRE
            "top.o.b bits=8 val=a5", // CONTROL — ok PRE
        ]
    );
}

/// A FILL override (`#(.Q('1))`). §5.7.1 gives the fill one unsigned bit in a
/// self-determined position and §6.20.2 hands that to the parameter, so `$bits(Q)` is 1 —
/// which vita already reported PRE — and the forwarded `~Q` is one bit of `0`.
///
/// PRE the forwarded cells were `32/fffffffe` and `32/1`: the fill arm gave `param_meta` 1
/// while the literal arm gave `param_range` the default's 8, the widest disagreement in the
/// pair. iverilog matches exactly. verilator is NOT an oracle on this cell — it contradicts
/// itself, reporting `$bits(Q)` as 8 while giving the same `'1` one bit everywhere else.
#[test]
fn a_fill_override_forwards_its_single_bit() {
    let (o, c) = run(&format!(
        "{LEAF}module mid #(parameter Q = 8'd1);\n\
        \x20 initial $display(\"%m Q bits=%0d val=%0h\", $bits(Q), Q);\n\
        \x20 leaf #(.P(~Q)) n(); leaf #(.P(Q)) b();\n\
         endmodule\n\
         module top; mid #(.Q('1)) f(); initial #1 $finish; endmodule\n"
    ));
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "top.f Q bits=1 val=1", // CONTROL — ok PRE
            "top.f.n bits=1 val=0", // PRE 32/fffffffe
            "top.f.b bits=1 val=1", // PRE 32/1
        ]
    );
}

/// A DECLARED type or range on the middle module is not this lane and must not move: it
/// survives an override (§12.2.1), so `param_decl_range_opt` still answers from the
/// declaration and the gate — which sits in the untyped, unranged TAIL — is never reached.
/// `midi` also pins that the `integer` arm above the tail stays a declared 32.
#[test]
fn a_declared_parent_parameter_does_not_move() {
    let (o, c) = run(&format!(
        "{LEAF}\
         module midr #(parameter logic [11:0] Q = 12'd1); leaf #(.P(~Q)) n(); leaf #(.P(Q)) b(); endmodule\n\
         module midi #(parameter integer Q = 1);          leaf #(.P(~Q)) n(); leaf #(.P(Q)) b(); endmodule\n\
         module top;\n\
        \x20 midr #(.Q(4'd3)) r();\n\
        \x20 midi #(.Q(4'd3)) i();\n\
        \x20 initial #1 $finish;\nendmodule\n"
    ));
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "top.r.n bits=12 val=ffc", // CONTROL — declared range, ok PRE
            "top.r.b bits=12 val=3",
            "top.i.n bits=32 val=fffffffc", // CONTROL — `integer` is a declared 32
            "top.i.b bits=32 val=3",
        ]
    );
}

/// A >64-bit override forwarded on. It reached the wide channel already and is unmoved —
/// the gate cannot make it worse, and it is here so that a later change to the wide fold
/// cannot silently truncate it.
#[test]
fn a_wider_than_64_bit_override_forwards_whole() {
    let (o, c) = run(&format!(
        "{LEAF}module mid #(parameter Q = 8'd1);\n\
        \x20 initial $display(\"%m Q bits=%0d\", $bits(Q));\n\
        \x20 leaf #(.P(Q)) b();\n\
         endmodule\n\
         module top; mid #(.Q(96'h1_0000_0000_0000_0003)) w(); initial #1 $finish; endmodule\n"
    ));
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "top.w Q bits=96",
            "top.w.b bits=96 val=10000000000000003", // CONTROL — ok PRE
        ]
    );
}

/// ⚠️ RESIDUES, pinned at the value vita prints TODAY so the next slice's move is visible.
/// They are silent-wrong against both oracles and they are OTHER rows, not this one.
///
/// `top.l` is no longer one of them: the derived-`localparam` forward is now certified
/// leaf-by-leaf (see `localparam_derived_forward_width.rs`), so it prints both oracles'
/// `4 / c`. It stays here as the control that separates the two roots.
///
/// `top.a` / `top.n` — an ASCENDING (`[0:11]`) and a NON-ZERO-LSB (`[15:4]`) declared range
/// on the parent. `narrow_param_bits` refuses `lo != 0 || ascending` outright, upstream of
/// the `param_range`/`param_meta` agreement this slice repairs — ROADMAP §2 🆕 H ⓑ. Both
/// oracles: `12 / ffc` and `12 / 3`. Note the contrast with
/// `a_declared_parent_parameter_does_not_move`'s `[11:0]` twin, which IS right: direction
/// and LSB are the whole difference.
#[test]
fn neighbouring_rows_are_recorded_not_widened_into() {
    let (o, c) = run(&format!(
        "{LEAF}\
         module ml #(parameter Q = 8'd1);\n\
        \x20 localparam R = ~Q;\n\
        \x20 initial $display(\"%m R bits=%0d val=%0h\", $bits(R), R);\n\
        \x20 leaf #(.P(R)) r();\n\
         endmodule\n\
         module ma #(parameter logic [0:11] Q = 12'd1); leaf #(.P(~Q)) n(); leaf #(.P(Q)) b(); endmodule\n\
         module mn #(parameter logic [15:4] Q = 12'd1); leaf #(.P(~Q)) n(); leaf #(.P(Q)) b(); endmodule\n\
         module top;\n\
        \x20 ml #(.Q(4'd3)) l();\n\
        \x20 ma #(.Q(4'd3)) a();\n\
        \x20 mn #(.Q(4'd3)) n();\n\
        \x20 initial #1 $finish;\nendmodule\n"
    ));
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "top.l R bits=4 val=c",         // right PRE and POST — `param_meta`
            "top.l.r bits=4 val=c",         // CONTROL — both oracles `bits=4 val=c`
            "top.a.n bits=32 val=fffffffc", // RESIDUE (🆕 H ⓑ): both oracles `12 / ffc`
            "top.a.b bits=32 val=3",        // RESIDUE (🆕 H ⓑ): both oracles `12 / 3`
            "top.n.n bits=32 val=fffffffc", // RESIDUE (🆕 H ⓑ): both oracles `12 / ffc`
            "top.n.b bits=32 val=3",        // RESIDUE (🆕 H ⓑ): both oracles `12 / 3`
        ]
    );
}
