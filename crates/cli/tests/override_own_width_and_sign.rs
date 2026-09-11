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

/// The accept set's remaining declines, pinned as the residues they are rather than left
/// to look like coverage. Both keep their pre-slice answer.
///
/// * a >64-bit tree (`~128'd0`) — `const_ctx_within_i64` refuses it, because the value
///   re-fold clamps at 64. Both oracles bind 128; vita keeps 32.
///
/// ⚠️ `localparam W8 = ~8'hCB` is NO LONGER one of them, and the move is the point: an
/// OPERATOR initializer whose every NAME leaf has a proved declared width now records its
/// own width in `param_range`, so `narrow_param_bits` certifies `W8` at 8 and
/// `#(.P(W8 + 1'b0))` binds at 8 — verilator's answer and a direct `$bits`'s, measured
/// 3-way (iverilog binds `+` at 9 while its own `$bits` says 8, the §4.5.466
/// self-contradiction, so it is NON-EVIDENCE here). It stays in this test as the control
/// that separates "the certification proved it" from "the domain refuses the width".
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
            "top.name_leaf bits=8 hex=34",
            "top.too_wide bits=32 hex=ffffffff",
        ]
    );
}

/// §2 "Index sealing" residue ⓐ: a NAME leaf with a DECLARED range takes that width, at
/// every operator top the accept set covers.
///
/// ⚠️ Where the two oracles part, the cell is annotated with WHICH tool contradicts
/// itself, by the adjudication this file's header states: iverilog binds `+ - *` at
/// max+1 while its own `$bits` of the same text says 8; verilator binds `%` at 32 while
/// its own `$bits` says 8. Every number below is the answer all three give when asked
/// `$bits(<expr>)` directly — Table 11-21.
#[test]
fn a_declared_name_leaf_binds_at_its_declared_width() {
    let (o, c) = run("module sub #(parameter P = 1) (); initial $display(\"%m bits=%0d dec=%0d hex=%h\", $bits(P), P, P); endmodule\n\
         module top;\n\
        \x20 parameter [7:0] W8 = 8'd7;\n\
        \x20 sub #(.P(W8 + 1'b0)) a_plus();\n\
        \x20 sub #(.P(W8 << 1))   b_shl();\n\
        \x20 sub #(.P(~W8))       c_not();\n\
        \x20 sub #(.P(-W8))       d_neg();\n\
        \x20 sub #(.P(W8 * 2'd2)) e_mul();\n\
        \x20 sub #(.P(W8 / 2'd2)) f_div();\n\
        \x20 sub #(.P(W8 % 3'd3)) g_mod();\n\
        \x20 sub #(.P(1'b1 ? W8 : 8'd0)) h_tern();\n\
        \x20 sub #(.P((W8)))      i_paren();\n\
        \x20 initial #10 $finish;\nendmodule\n");
    assert_eq!(c, Some(0), "{o}");
    let mut got = lines(&o);
    got.sort();
    assert_eq!(
        got,
        [
            // iverilog binds `+` at 9; verilator and a direct `$bits` say 8
            "top.a_plus bits=8 dec=7 hex=07",
            "top.b_shl bits=8 dec=14 hex=0e",  // both oracles
            "top.c_not bits=8 dec=248 hex=f8", // both oracles — WIDTH and VALUE
            "top.d_neg bits=8 dec=249 hex=f9", // both oracles
            // iverilog binds `*` at 10; verilator and a direct `$bits` say 8
            "top.e_mul bits=8 dec=14 hex=0e",
            "top.f_div bits=8 dec=3 hex=03", // both oracles
            // verilator binds `%` at 32; iverilog and a direct `$bits` say 8
            "top.g_mod bits=8 dec=1 hex=01",
            "top.h_tern bits=8 dec=7 hex=07",  // both oracles
            "top.i_paren bits=8 dec=7 hex=07", // a bare name through `override_bits`
        ]
    );
}

/// The same rule reaches the override through every channel and from every scope a name
/// can be read in — all four measured identical, all matching BOTH oracles.
///
/// A `pkg::`-scoped name is deliberately NOT here, and for a narrower reason than this
/// comment once claimed: a BARE `pk::K` override source now binds its declared width
/// (`pkg_scoped_override_source.rs` — `wide_top_is_self_determined` admits `PkgScoped`,
/// and `wide_name_bits` answers it from `pkg_wide_bits`/`pkg_const_narrow_bits`, so
/// `narrow_param_bits`' single-segment guard is not on that path at all). What still
/// declines fail-closed is the OPERATOR-topped scoped source measured here: `~pk::PA`
/// keeps 32, because `declared_override_widths::names` and `ctx_width_names_are_evident`
/// drop a `PkgScoped` leaf and `ConstWidths` is keyed by the bare name. That is the next
/// rung, not this slice.
#[test]
fn every_channel_and_name_position_binds_the_same() {
    const SUB: &str = "module sub #(parameter P = 1) (); initial $display(\"%m bits=%0d dec=%0d hex=%h\", $bits(P), P, P); endmodule\n";
    for (label, src) in [
        (
            "named",
            format!("{SUB}module top; parameter [7:0] W8 = 8'd7; sub #(.P(~W8)) u(); initial #10 $finish; endmodule\n"),
        ),
        (
            "positional",
            format!("{SUB}module top; parameter [7:0] W8 = 8'd7; sub #(~W8) u(); initial #10 $finish; endmodule\n"),
        ),
        (
            "defparam",
            format!("{SUB}module top; parameter [7:0] W8 = 8'd7; sub u(); defparam u.P = ~W8; initial #10 $finish; endmodule\n"),
        ),
        (
            "wildcard-imported package parameter",
            format!("package pk; parameter [7:0] PA = 8'd7; endpackage\n{SUB}module top; import pk::*; sub #(.P(~PA)) u(); initial #10 $finish; endmodule\n"),
        ),
    ] {
        let (o, c) = run(&src);
        assert_eq!(c, Some(0), "{label}: {o}");
        assert_eq!(lines(&o), ["top.u bits=8 dec=248 hex=f8"], "channel {label}");
    }
}

/// A generate scope reads an OUTER parameter, and the sign comes from the same env the
/// width did.
///
/// ⚠️ This is why the meta's sign is taken from `const_signed_env` and not
/// `const_expr_signed`: the latter's `Ident` arm resolves through `fq()` — the current
/// scope only — where the width walk uses the scope CHAIN. Measured on the sibling
/// (localparam) lane, an outer `signed [7:0] S8` read from inside a generate block folds
/// `S8 >>> 1` to 255 while the identical text at module scope folds to −1, both oracles
/// −1. Asking one env for both answers is what keeps that pre-existing defect (ROADMAP
/// §2) out of this channel. Both cells below match both oracles.
#[test]
fn a_generate_scope_reads_the_outer_declaration_for_width_and_sign() {
    let (o, c) = run("module sub #(parameter P = 1) (); initial $display(\"%m bits=%0d dec=%0d hex=%h\", $bits(P), P, P); endmodule\n\
         module top;\n\
        \x20 parameter signed [7:0] S8 = -8'sd2;\n\
        \x20 generate if (1) begin : g\n\
        \x20   localparam [7:0] GL = 8'd7;\n\
        \x20   sub #(.P(~GL)) u();\n\
        \x20   sub #(.P(-S8)) v();\n\
        \x20 end endgenerate\n\
        \x20 initial #10 $finish;\nendmodule\n");
    assert_eq!(c, Some(0), "{o}");
    let mut got = lines(&o);
    got.sort();
    assert_eq!(
        got,
        [
            "top.g.u bits=8 dec=248 hex=f8", // both oracles
            "top.g.v bits=8 dec=2 hex=02",   // both oracles
        ]
    );
}

/// FORWARDING — a DIFFERENT root, pinned in both directions so the boundary is a measured
/// fact rather than an omission.
///
/// A parent's own untyped parameter `Q` forwarded into a child binds correctly when `Q` is
/// NOT overridden, and when the override's width happens to equal the default literal's.
/// It kept its pre-slice 32 when they DIFFERED, because `param_range` still held the
/// DEFAULT literal's width for an overridden untyped parameter while `param_meta` held the
/// override's — and `narrow_param_bits` refuses a disagreement rather than picking one.
/// That refusal is why the slice that pinned this could not regress: the stale entry was
/// declined, not believed.
///
/// The `m_wide` line was that residue and is now CLOSED: `param_decl_width_opt`'s
/// untyped-tail literal arm no longer answers under `declared_only` once an override has
/// reached the declaration, so the two maps agree at 16 and the fold binds `fff6` — which
/// is what both oracles print. `param_override_forwarded_width.rs` is that slice's census;
/// this cell stays here as its boundary marker.
#[test]
fn forwarding_binds_when_the_two_width_maps_agree_and_declines_when_they_do_not() {
    let (o, c) = run("module sub #(parameter P = 1) (); initial $display(\"%m bits=%0d dec=%0d hex=%h\", $bits(P), P, P); endmodule\n\
         module mid #(parameter Q = 8'd7) ();\n\
        \x20 sub #(.P(~Q)) f();\n\
         endmodule\n\
         module top;\n\
        \x20 mid              m_def();\n\
        \x20 mid #(.Q(8'd9))  m_same();\n\
        \x20 mid #(.Q(16'd9)) m_wide();\n\
        \x20 initial #10 $finish;\nendmodule\n");
    assert_eq!(c, Some(0), "{o}");
    let mut got = lines(&o);
    got.sort();
    assert_eq!(
        got,
        [
            "top.m_def.f bits=8 dec=248 hex=f8",  // both oracles
            "top.m_same.f bits=8 dec=246 hex=f6", // both oracles
            // WAS the residue `bits=32 dec=-10 hex=fffffff6`. Both oracles: 16 / fff6.
            "top.m_wide.f bits=16 dec=65526 hex=fff6",
        ]
    );
}

/// The shapes this slice must NOT move, measured PRE and POST: a bare name and the
/// bitwise trees already answered by `override_bits`, a reduction top (1 bit — vita is on
/// iverilog's side; verilator self-contradicts at 32), and an untyped DECIMAL default,
/// whose 32 is the correct answer in all three tools.
#[test]
fn the_already_correct_shapes_are_untouched() {
    let (o, c) = run("module sub #(parameter P = 1) (); initial $display(\"%m bits=%0d dec=%0d hex=%h\", $bits(P), P, P); endmodule\n\
         module top;\n\
        \x20 parameter [7:0] W8 = 8'd7;\n\
        \x20 parameter       WD = 7;\n\
        \x20 sub #(.P(W8))        a_alone();\n\
        \x20 sub #(.P(W8 | 1'b0)) b_or();\n\
        \x20 sub #(.P(W8 & 8'hFF))c_and();\n\
        \x20 sub #(.P(|W8))       d_red();\n\
        \x20 sub #(.P(WD | 1'b0)) e_untyped_decimal();\n\
        \x20 initial #10 $finish;\nendmodule\n");
    assert_eq!(c, Some(0), "{o}");
    let mut got = lines(&o);
    got.sort();
    assert_eq!(
        got,
        [
            "top.a_alone bits=8 dec=7 hex=07",
            "top.b_or bits=8 dec=7 hex=07",
            "top.c_and bits=8 dec=7 hex=07",
            "top.d_red bits=1 dec=1 hex=1",
            "top.e_untyped_decimal bits=32 dec=7 hex=00000007",
        ]
    );
}
