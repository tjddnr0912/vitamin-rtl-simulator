//! IEEE 1800 §6.20.2 — an UNTYPED, unranged parameter takes the RANGE of its FINAL
//! override value; §12.2.1 — a sign SPECIFICATION written on the declaration survives
//! that override. ROADMAP §2 "Index sealing" row 25, the operator half.
//!
//! vita had no channel carrying an operator-topped override's own type. `override_bits`
//! folds a SELF-DETERMINED top only (`wide_top_is_self_determined` admits the reductions,
//! the comparisons and the selects — not `~`, unary `-`/`+`, or the arithmetic binaries),
//! so the meta chain in `bind_one_param` fell through to `param_decl_width_opt`'s literal
//! arm, which answers the DEFAULT's type even when `default_binds == false`.
//!
//! ## Why these numbers are the target, when the two oracles disagree about some of them
//!
//! Asked DIRECTLY, all three tools agree on `$bits(<expr>)` for every cell here. Each then
//! contradicts its OWN direct answer when the identical text is BOUND to a parameter:
//! verilator on a reduction-topped override (`$bits(|4'b1010)` is 1, the binding is 32),
//! iverilog on `+` (`$bits(8'd200+8'd100)` is 8, the binding is 9 — ROADMAP §2 records
//! that one). So the width axis is not an oracle split: it is two tools each
//! self-contradicting in a different place, over one answer all three give when asked
//! directly. That answer is Table 11-21, and it is what vita already bound for a
//! reduction top before this slice. Every expected value below was measured 3-way at the
//! slice, and each is annotated with which tool it lands on where they part.
//!
//! ## What is deliberately NOT here
//!
//! A DECLARED-width target (`parameter [63:0] K`) is a different lane and carries ROADMAP
//! §2 rows 16/17's live oracle split (`~32'd0` is `ffffffffffffffff` in vita and iverilog,
//! `00000000ffffffff` in verilator). The consumer's `Implicit && p.range.is_none()` guard
//! keeps this slice off that lane, and `a_declared_width_target_is_untouched` pins it.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_oow_{}_{n}", std::process::id()));
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

/// The row's own four cells plus the CONTROL that must not move.
///
/// `|4'b1010` is the control: `override_bits` already answered it and already answered it
/// RIGHT (1 bit), so a rule that widens everything would be caught by it moving to 32.
/// PRE bound all four of the others at 32 bits.
#[test]
fn an_operator_override_binds_its_own_width() {
    let (o, c) = run("module s1 #(parameter P = 1) (); initial $display(\"neg_red bits=%0d hex=%h dec=%0d\", $bits(P), P, P); endmodule\n\
         module s2 #(parameter P = 1) (); initial $display(\"not_red bits=%0d hex=%h dec=%0d\", $bits(P), P, P); endmodule\n\
         module s3 #(parameter P = 1) (); initial $display(\"not8    bits=%0d hex=%h dec=%0d\", $bits(P), P, P); endmodule\n\
         module s4 #(parameter P = 1) (); initial $display(\"add8    bits=%0d hex=%h dec=%0d\", $bits(P), P, P); endmodule\n\
         module s5 #(parameter P = 1) (); initial $display(\"red     bits=%0d hex=%h dec=%0d\", $bits(P), P, P); endmodule\n\
         module top;\n\
        \x20 s1 #(.P(-(|4'b1010)))   u1();\n\
        \x20 s2 #(.P(~(|4'b1010)))   u2();\n\
        \x20 s3 #(.P(~8'h5A))        u3();\n\
        \x20 s4 #(.P(8'd200+8'd100)) u4();\n\
        \x20 s5 #(.P(|4'b1010))      u5();\n\
        \x20 initial #10 $finish;\nendmodule\n");
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            // iverilog agrees; verilator binds 32 here, contradicting its own `$bits` of 1
            "neg_red bits=1 hex=1 dec=1",
            "not_red bits=1 hex=0 dec=0",    // both oracles
            "not8    bits=8 hex=a5 dec=165", // both oracles
            // verilator agrees; iverilog binds 9, contradicting its own `$bits` of 8
            "add8    bits=8 hex=2c dec=44",
            "red     bits=1 hex=1 dec=1", // CONTROL — unmoved, `override_bits` owns it
        ]
    );
}

/// The width alone is not the answer: truncation does not commute with `/ % >> >>>`, so
/// the value has to be re-folded AT the type the width channel just chose.
///
/// The first two cells are the ones that prove it — PRE folded them in the unlimited lane
/// and masked once at the end, giving 31 for 15 and 150 for 22. All three tools agree on
/// every line here.
#[test]
fn the_value_is_refolded_at_the_overrides_own_width() {
    let (o, c) = run("module a #(parameter P=1) (); initial $display(\"mulshr bits=%0d hex=%h dec=%0d\", $bits(P), P, P); endmodule\n\
         module b #(parameter P=1) (); initial $display(\"addshr bits=%0d hex=%h dec=%0d\", $bits(P), P, P); endmodule\n\
         module d #(parameter P=1) (); initial $display(\"div    bits=%0d hex=%h dec=%0d\", $bits(P), P, P); endmodule\n\
         module e #(parameter P=1) (); initial $display(\"mod    bits=%0d hex=%h dec=%0d\", $bits(P), P, P); endmodule\n\
         module f #(parameter P=1) (); initial $display(\"sra    bits=%0d hex=%h dec=%0d\", $bits(P), P, P); endmodule\n\
         module top;\n\
        \x20 a #((8'hFF * 8'h02) >> 4) ua();\n\
        \x20 b #((8'd200+8'd100) >> 1) ub();\n\
        \x20 d #(8'hFF / 8'd3)         ud();\n\
        \x20 e #(8'hFF % 8'd7)         ue();\n\
        \x20 f #(8'shF0 >>> 2)         uf();\n\
        \x20 initial #10 $finish;\nendmodule\n");
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "mulshr bits=8 hex=0f dec=15", // PRE: 32 / 31
            "addshr bits=8 hex=16 dec=22", // PRE: 32 / 150
            "div    bits=8 hex=55 dec=85",
            "mod    bits=8 hex=03 dec=3",
            "sra    bits=8 hex=fc dec=-4",
        ]
    );
}

/// The width ladder, and the reason it is here: a first cut installed the width without the
/// value and read CORRECT on `$bits` while the value was still folded for the old width —
/// `-33'd1` bound `0ffffffff` for `1ffffffff` and `-64'd1` bound `00000000ffffffff` for all
/// ones. A `$bits`-only pin would have shipped it. 8/16/32 were already right and must not
/// move; 33 and 64 are the ones that moved. All three tools agree on all five.
#[test]
fn the_width_ladder_moves_only_where_it_was_wrong() {
    let (o, c) = run("module m8  #(parameter P=1) (); initial $display(\"w08 bits=%0d hex=%h\", $bits(P), P); endmodule\n\
         module m16 #(parameter P=1) (); initial $display(\"w16 bits=%0d hex=%h\", $bits(P), P); endmodule\n\
         module m32 #(parameter P=1) (); initial $display(\"w32 bits=%0d hex=%h\", $bits(P), P); endmodule\n\
         module m33 #(parameter P=1) (); initial $display(\"w33 bits=%0d hex=%h\", $bits(P), P); endmodule\n\
         module m64 #(parameter P=1) (); initial $display(\"w64 bits=%0d hex=%h\", $bits(P), P); endmodule\n\
         module top;\n\
        \x20 m8 #(-8'd1) a(); m16 #(-16'd1) b(); m32 #(-32'd1) c();\n\
        \x20 m33 #(-33'd1) d(); m64 #(-64'd1) e();\n\
        \x20 initial #10 $finish;\nendmodule\n");
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "w08 bits=8 hex=ff",
            "w16 bits=16 hex=ffff",
            "w32 bits=32 hex=ffffffff",
            "w33 bits=33 hex=1ffffffff",
            "w64 bits=64 hex=ffffffffffffffff",
        ]
    );
}

/// §12.2.1: a `signed` keyword with no range survives the override — only the RANGE comes
/// from the override value. Both arms of the meta chain must say so, or one declaration
/// reports two signs depending on which channel binds it.
///
/// `a` and `d` bind through the WIDE channel (`override_bits`), `b` and `c` through the
/// operator arm. Review round 1 caught the operator arm dropping the sign (`165`), round 2
/// caught the wide arm never having applied it. iverilog prints all seven of these lines.
/// `e`/`f` are the no-keyword control — they must stay unsigned on BOTH arms.
#[test]
fn a_declared_signed_keyword_survives_an_override_on_both_arms() {
    let (o, c) = run("module s2m #(parameter signed R = 1) (); initial $display(\"R bits=%0d hex=%h dec=%0d\", $bits(R), R, R); endmodule\n\
         module sn  #(parameter        N = 1) (); initial $display(\"N bits=%0d hex=%h dec=%0d\", $bits(N), N, N); endmodule\n\
         module ss  #(parameter signed S = 1) (); initial $display(\"S bits=%0d hex=%h dec=%0d\", $bits(S), S, S); endmodule\n\
         module top;\n\
        \x20 s2m #(.R(8'hA5))         a();\n\
        \x20 s2m #(.R(~8'h5A))        b();\n\
        \x20 s2m #(.R(-8'd91))        c();\n\
        \x20 s2m #(.R(8'hA5 & 8'hFF)) d();\n\
        \x20 sn  #(.N(8'hA5))         e();\n\
        \x20 sn  #(.N(~8'h5A))        f();\n\
        \x20 ss  #(.S(8'shA5))        g();\n\
        \x20 initial #1 $finish;\nendmodule\n");
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        [
            "R bits=8 hex=a5 dec=-91", // wide arm  — PRE said 165
            "R bits=8 hex=a5 dec=-91", // operator arm
            "R bits=8 hex=a5 dec=-91", // operator arm
            "R bits=8 hex=a5 dec=-91", // wide arm  — PRE said 165
            "N bits=8 hex=a5 dec=165", // CONTROL: no keyword, stays unsigned
            "N bits=8 hex=a5 dec=165", // CONTROL
            "S bits=8 hex=a5 dec=-91", // a signed override literal was already right
        ]
    );
}

/// Every channel must bind ONE type. A `defparam` and a `#()` naming the same expression
/// disagreeing would be the "one key, several rules" shape; `DefparamOverride` carries the
/// meta for exactly that reason. All three channels print `8 / a5 / 165` in both oracles.
#[test]
fn every_override_channel_binds_the_same_type() {
    let (o, c) = run("module sub #(parameter P = 1) (); initial $display(\"%m bits=%0d hex=%h dec=%0d\", $bits(P), P, P); endmodule\n\
         module top;\n\
        \x20 sub #(.P(~8'h5A)) named();\n\
        \x20 sub #(~8'h5A)     positional();\n\
        \x20 sub               dp();\n\
        \x20 defparam dp.P = ~8'h5A;\n\
        \x20 initial #10 $finish;\nendmodule\n");
    assert_eq!(c, Some(0), "{o}");
    let mut got = lines(&o);
    got.sort();
    assert_eq!(
        got,
        [
            "top.dp bits=8 hex=a5 dec=165",
            "top.named bits=8 hex=a5 dec=165",
            "top.positional bits=8 hex=a5 dec=165",
        ]
    );
}

/// The lane guard. A DECLARED-width target never reaches the new arm, which is what keeps
/// the slice off ROADMAP §2 rows 16/17's live oracle split — on `parameter [63:0] K`,
/// `~32'd0` is `ffffffffffffffff` here and in iverilog and `00000000ffffffff` in verilator.
/// Every line below is byte-identical to PRE.
#[test]
fn a_declared_width_target_is_untouched() {
    let (o, c) = run(
        "module d64 #(parameter [63:0] K = 0) (); initial $display(\"%m hex=%h\", K); endmodule\n\
         module top;\n\
        \x20 d64 #(.K(~32'd0))                       a();\n\
        \x20 d64 #(.K(-64'd1))                       b();\n\
        \x20 d64 #(.K(64'hFFFFFFFFFFFFFFFF + 64'd1)) c();\n\
        \x20 d64 #(.K(64'hFFFFFFFFFFFFFFFF << 4))    d();\n\
        \x20 initial #10 $finish;\nendmodule\n",
    );
    assert_eq!(c, Some(0), "{o}");
    let mut got = lines(&o);
    got.sort();
    assert_eq!(
        got,
        [
            "top.a hex=ffffffffffffffff",
            "top.b hex=ffffffffffffffff",
            "top.c hex=0000000000000000",
            "top.d hex=fffffffffffffff0",
        ]
    );
}

/// The accept set's two declines, pinned as the residues they are rather than left to look
/// like coverage. Both keep their pre-slice answer.
///
/// * a NAME leaf (`W8 + 1'b0`) — `ctx_width_names_are_evident` refuses it, because
///   `const_self_width` would size the name from `param_meta`, where value-INFERRED widths
///   live. Both oracles bind 8 (iverilog 9, its `+` quirk); vita keeps 32.
/// * a >64-bit tree (`~128'd0`) — `const_ctx_within_i64` refuses it, because the value
///   re-fold clamps at 64. Both oracles bind 128; vita keeps 32.
#[test]
fn the_declined_shapes_keep_their_pre_slice_answer() {
    let (o, c) = run("module sub #(parameter P = 1) (); initial $display(\"%m bits=%0d hex=%h\", $bits(P), P); endmodule\n\
         module top;\n\
        \x20 localparam W8 = ~8'hCB;\n\
        \x20 sub #(.P(W8 + 1'b0)) name_leaf();\n\
        \x20 sub #(.P(~128'd0))   too_wide();\n\
        \x20 initial #10 $finish;\nendmodule\n");
    assert_eq!(c, Some(0), "{o}");
    let mut got = lines(&o);
    got.sort();
    assert_eq!(
        got,
        [
            "top.name_leaf bits=32 hex=00000034",
            "top.too_wide bits=32 hex=ffffffff",
        ]
    );
}
