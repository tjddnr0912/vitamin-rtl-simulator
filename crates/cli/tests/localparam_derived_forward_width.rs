//! IEEE 1800 §6.20.2 / Table 11-21 — a `localparam` whose initializer is an OPERATOR, a
//! TERNARY, a name-bearing CONCAT or a bare ALIAS forwards at ITS OWN width, not at the
//! receiving parameter's default width. ROADMAP §2 "Index sealing".
//!
//! PRE, `param_decl_width_opt`'s operator / ternary / concat / alias arms were gated
//! `default_binds && !declared_only`, and a body `localparam` is always both, so
//! `param_decl_range_opt` recorded NOTHING in `param_range` while
//! `param_decl_width_unoverridden` recorded the right width in `param_meta`.
//! `narrow_param_bits` needs a `param_range` entry, so every forwarding channel declined
//! and `bind_one_param` fell back to the LEAF's own declared default — a width-only
//! silent-wrong when that default was ≥ the derived width and a width+VALUE one when it
//! was narrower. POST, those arms answer whenever `declared_override_widths` +
//! `ctx_width_names_are_evident` (`concat_width_is_declared` for the concat family) prove
//! every NAME leaf's width through `narrow_param_bits`, and decline otherwise.
//!
//! ## Oracles
//!
//! Every expected line below was measured 3-way (vita / iverilog 13 + `vvp -n` /
//! verilator 5.052 `--binary --timing`). Both oracles agree on every cell EXCEPT:
//! a bound `+` (iverilog sizes `Q+1` at max+1 while its own `$bits` answers max — it
//! contradicts itself and is NON-EVIDENCE, §4.5.466; verilator arbitrates and vita
//! matches it), and an out-of-range CONSTANT select (verilator reads real bits where
//! iverilog reads `x`; iverilog arbitrates, and the fence cells below are pinned at its
//! answer).

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ldfw_{}_{n}", std::process::id()));
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

/// The census base shape: a `leaf` whose parameter default is `1'b0` (ONE bit, so a lost
/// width also truncates the value), a `mid` declaring `localparam R = <deriv>` and
/// forwarding it, and a `t` overriding `mid`'s untyped `Q`.
fn base(deriv: &str, qdecl: &str, ovr: &str) -> String {
    format!(
        "module leaf #(parameter P = 1'b0);\n\
        \x20 initial $display(\"leaf bits=%0d val=%h\", $bits(P), P);\n\
         endmodule\n\
         module mid #({qdecl});\n\
        \x20 localparam R = {deriv};\n\
        \x20 initial $display(\"mid bitsR=%0d R=%h\", $bits(R), R);\n\
        \x20 leaf #(.P(R)) u();\n\
         endmodule\n\
         module t;\n\
        \x20 mid {ovr} m();\n\
        \x20 initial #1 $finish;\n\
         endmodule\n"
    )
}

fn check(src: &str, want: &[&str]) {
    let (o, c) = run(src);
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(lines(&o), want, "{o}");
}

/// A1 — unary `~`, the operator arm. PRE `leaf bits=1 val=0`; both oracles `4 / c`.
#[test]
fn a_bitnot_derivation_forwards_its_own_width() {
    check(
        &base("~Q", "parameter Q = 8'd1", "#(.Q(4'd3))"),
        &["mid bitsR=4 R=c", "leaf bits=4 val=c"],
    );
}

/// A2 — the bare ALIAS arm (`localparam R = Q;`), which now answers through
/// `narrow_param_bits` instead of declining as "provenance unknown". PRE `1 / 1`; both
/// oracles `4 / 3`.
#[test]
fn a_bare_alias_derivation_forwards_the_sources_width() {
    check(
        &base("Q", "parameter Q = 8'd1", "#(.Q(4'd3))"),
        &["mid bitsR=4 R=3", "leaf bits=4 val=3"],
    );
}

/// A3 — a CONTEXT-determined `+` top. PRE `leaf bits=1 val=0`. verilator: `32 / 00000004`
/// (and vita's own `$bits(R)` already said 32 PRE); iverilog says 33 here while its own
/// `$bits(Q+1)` says 32, so it is NON-EVIDENCE (§4.5.466).
#[test]
fn a_bound_add_derivation_forwards_verilators_width() {
    check(
        &base("Q + 1", "parameter Q = 8'd1", "#(.Q(4'd3))"),
        &["mid bitsR=32 R=00000004", "leaf bits=32 val=00000004"],
    );
}

/// A4 — a SHIFT top takes the left operand's width. PRE `1 / 0`; both oracles `4 / 6`.
#[test]
fn a_shift_derivation_forwards_the_left_operands_width() {
    check(
        &base("Q << 1", "parameter Q = 8'd1", "#(.Q(4'd3))"),
        &["mid bitsR=4 R=6", "leaf bits=4 val=6"],
    );
}

/// A7 — unary `-`. PRE `1 / 1`; both oracles `4 / d`.
#[test]
fn a_negation_derivation_forwards_its_own_width() {
    check(
        &base("-Q", "parameter Q = 8'd1", "#(.Q(4'd3))"),
        &["mid bitsR=4 R=d", "leaf bits=4 val=d"],
    );
}

/// A8 — the TERNARY arm (§11.4.11: as wide as the wider arm). PRE `leaf bits=1 val=1` —
/// WIDTH-only silent-wrong, because `1` survives a 1-bit truncation, which is why the
/// value column cannot be the detector for this axis. Both oracles `4 / 1`.
#[test]
fn a_ternary_derivation_forwards_the_wider_arms_width() {
    check(
        &base("Q ? 4'd1 : 4'd2", "parameter Q = 8'd1", "#(.Q(4'd3))"),
        &["mid bitsR=4 R=1", "leaf bits=4 val=1"],
    );
}

/// A5 — a CONCAT whose leaf is a NAME. `concat_width_is_declared` used to refuse every
/// name; it now admits one the certified environment proves. PRE `1 / 1`; both oracles
/// `8 / 33`.
#[test]
fn a_concat_of_a_certified_name_forwards_its_summed_width() {
    check(
        &base("{Q, Q}", "parameter Q = 8'd1", "#(.Q(4'd3))"),
        &["mid bitsR=8 R=33", "leaf bits=8 val=33"],
    );
}

/// C1 — a DECLARED TYPE on the parent. The parent's provenance was never the problem:
/// this cell was equally wrong PRE (`leaf bits=1 val=0`). Both oracles `8 / fc`.
#[test]
fn a_typed_parents_width_reaches_the_leaf() {
    check(
        &base("~Q", "parameter logic [7:0] Q = 8'd1", "#(.Q(4'd3))"),
        &["mid bitsR=8 R=fc", "leaf bits=8 val=fc"],
    );
}

/// D1 — NO override anywhere. The defect was never about override-ness: the default lane
/// was equally wrong PRE (`leaf bits=1 val=0`). Both oracles `8 / fe`.
#[test]
fn the_un_overridden_default_lane_forwards_too() {
    check(
        &base("~Q", "parameter Q = 8'd1", ""),
        &["mid bitsR=8 R=fe", "leaf bits=8 val=fe"],
    );
}

/// E2 — the VALUE column. The leaf's own default is `1'b0`, one bit, so PRE the forward
/// fell back to it and printed `leaf bits=1 val=0`: the derived `c` was TRUNCATED, not
/// merely mis-widened. (E1 `= 1`, E3 `= 8'd0` and E4 `= 32'd0` are the same cell with a
/// wider leaf default, where only the width was wrong.) Both oracles `4 / c`.
#[test]
fn a_one_bit_leaf_default_no_longer_truncates_the_forwarded_value() {
    let (o, c) = run(&base("~Q", "parameter Q = 8'd1", "#(.Q(4'd3))"));
    assert_eq!(c, Some(0), "{o}");
    assert_eq!(
        lines(&o),
        ["mid bitsR=4 R=c", "leaf bits=4 val=c"],
        "the value `c` must survive a 1-bit leaf default: {o}"
    );
}

/// F1 — the `defparam` channel. Both oracles `4 / c`; PRE `1 / 0`.
#[test]
fn the_defparam_channel_forwards_too() {
    check(
        "module leaf #(parameter P = 1'b0);\n\
        \x20 initial $display(\"leaf bits=%0d val=%h\", $bits(P), P);\n\
         endmodule\n\
         module mid #(parameter Q = 8'd1);\n\
        \x20 localparam R = ~Q;\n\
        \x20 initial $display(\"mid bitsR=%0d R=%h\", $bits(R), R);\n\
        \x20 leaf #(.P(R)) u();\n\
         endmodule\n\
         module t;\n\
        \x20 mid m();\n\
        \x20 defparam m.Q = 4'd3;\n\
        \x20 initial #1 $finish;\n\
         endmodule\n",
        &["mid bitsR=4 R=c", "leaf bits=4 val=c"],
    );
}

/// G1 — a SECOND derivation level (`localparam R2 = ~R;`). The certification chains: `R`
/// gains a `param_range` entry, which is what lets `R2` certify `R` in turn. PRE
/// `leaf bits=1 val=1`; both oracles `4 / 3`.
#[test]
fn a_second_derivation_level_chains() {
    check(
        "module leaf #(parameter P = 1'b0);\n\
        \x20 initial $display(\"leaf bits=%0d val=%h\", $bits(P), P);\n\
         endmodule\n\
         module mid #(parameter Q = 8'd1);\n\
        \x20 localparam R = ~Q;\n\
        \x20 initial $display(\"mid bitsR=%0d R=%h\", $bits(R), R);\n\
        \x20 localparam R2 = ~R;\n\
        \x20 initial $display(\"mid bitsR2=%0d R2=%h\", $bits(R2), R2);\n\
        \x20 leaf #(.P(R2)) u();\n\
         endmodule\n\
         module t;\n\
        \x20 mid #(.Q(4'd3)) m();\n\
        \x20 initial #1 $finish;\n\
         endmodule\n",
        &["mid bitsR=4 R=c", "mid bitsR2=4 R2=3", "leaf bits=4 val=3"],
    );
}

/// H1 — the GENERATE-scope binder (`generate.rs`'s twin of the module-body one, which
/// passes the same `declared_only = true`). PRE `1 / 0`; both oracles `4 / c`.
#[test]
fn a_generate_scope_localparam_forwards_too() {
    check(
        "module leaf #(parameter P = 1'b0);\n\
        \x20 initial $display(\"leaf bits=%0d val=%h\", $bits(P), P);\n\
         endmodule\n\
         module mid #(parameter Q = 8'd1);\n\
        \x20 generate if (1) begin : g\n\
        \x20   localparam R = ~Q;\n\
        \x20   initial $display(\"mid bitsR=%0d R=%h\", $bits(R), R);\n\
        \x20   leaf #(.P(R)) u();\n\
        \x20 end endgenerate\n\
         endmodule\n\
         module t;\n\
        \x20 mid #(.Q(4'd3)) m();\n\
        \x20 initial #1 $finish;\n\
         endmodule\n",
        &["mid bitsR=4 R=c", "leaf bits=4 val=c"],
    );
}

/// I1 — a SIGNED override expression. The certified environment carries the sign with the
/// width (`const_signed_env`, the same resolver `override_self_meta` uses), so the 4-bit
/// `-4'sd3` lands as `2` at 4 bits. PRE `1 / 0`; both oracles `4 / 2`.
#[test]
fn a_signed_override_forwards_at_four_bits() {
    check(
        &base("~Q", "parameter Q = 8'd1", "#(.Q(-4'sd3))"),
        &["mid bitsR=4 R=2", "leaf bits=4 val=2"],
    );
}

/// ⚠️ THE FENCE. §4.5.363's rule is that `param_range` answers only "is this width a
/// DECLARED fact?", because a consumer that EXTRACTS BITS from it invents bits that do not
/// exist — `logic [(W[15:8])+8-1:0] v;` declared a 263-bit net where iverilog declares 1.
/// These four cells are the shapes that rule protects, pinned at the text they print both
/// PRE and POST. They must NOT move.
///
/// `F1`/`G3`/`G4` certify (their leaves are sized literals, so the width recorded is the
/// same 8 `param_meta` already held and all three tools print), and the constant select
/// still reads out of range: iverilog's `x` / 1-bit net, which is vita's answer.
/// verilator reads `52` and a 60-bit net and is not the oracle here.
///
/// ⭐ `F4` was that row's RESIDUE and is CLOSED (§2 "Index sealing" ⓑ). It used to
/// declare `bitsv=1` where both oracles declare 263, because `localparam A =
/// $clog2(300)` reached `param_decl_width_opt`'s value-inferred tail and recorded no
/// `param_range`. `param_decl_width_opt` now has a `SysCall` arm for the three
/// integer-returning calls the constant domain folds, so `A` records `(0, 32, false)`,
/// the certification passes, and vita declares the same `263` as iverilog 13 and
/// verilator 5.052. Everything else on this test is unchanged — `F1`/`G3`/`G4` are the
/// value-INFERRED operands the fence is actually about and still decline.
#[test]
fn the_declared_only_fence_does_not_move() {
    check(
        "module t;\n\
        \x20 localparam W = ~8'hCB;\n\
        \x20 logic [(W[15:8])+8-1:0] v;\n\
        \x20 initial begin $display(\"F1 bitsW=%0d W=%h sel=%0d bitsv=%0d\", $bits(W), W, W[15:8], $bits(v)); #1 $finish; end\n\
         endmodule\n",
        &["F1 bitsW=8 W=34 sel=x bitsv=1"],
    );
    check(
        "module t;\n\
        \x20 localparam A = $clog2(300);\n\
        \x20 localparam W = ~A;\n\
        \x20 logic [(W[15:8])+8-1:0] v;\n\
        \x20 initial begin $display(\"F4 bitsA=%0d bitsW=%0d W=%h sel=%0d bitsv=%0d\", $bits(A), $bits(W), W, W[15:8], $bits(v)); #1 $finish; end\n\
         endmodule\n",
        &["F4 bitsA=32 bitsW=32 W=fffffff6 sel=255 bitsv=263"],
    );
    check(
        "module t;\n\
        \x20 localparam Q = 8'hCB;\n\
        \x20 localparam R = ~Q;\n\
        \x20 initial begin $display(\"G3 bitsR=%0d R=%h sel158=%0d sel30=%0d\", $bits(R), R, R[15:8], R[3:0]); #1 $finish; end\n\
         endmodule\n",
        &["G3 bitsR=8 R=34 sel158=x sel30=4"],
    );
    check(
        "module t;\n\
        \x20 localparam Q = 8'hCB;\n\
        \x20 localparam R = ~Q;\n\
        \x20 logic [(R[15:8])+8-1:0] v;\n\
        \x20 initial begin $display(\"G4 bitsR=%0d R=%h sel=%0d bitsv=%0d\", $bits(R), R, R[15:8], $bits(v)); #1 $finish; end\n\
         endmodule\n",
        &["G4 bitsR=8 R=34 sel=x bitsv=1"],
    );
}

/// The two sources the certification must REFUSE, pinned so a later widening cannot take
/// them silently.
///
/// `W1` — a >64-BIT parent. `instance.rs` puts such a parameter in `wide_param_bits` and
/// writes no `param_meta`, so `narrow_param_bits` declines and `localparam Q = ~WBIG`
/// records no range. The >64 lane has its own resolver (`wide_param_bits`) and is correct
/// and unchanged: both oracles `72 / fffffffffffffffffe` (verilator clamps the override to
/// the declared width here and is not the oracle — §6.20.2 width-from-override).
///
/// ⭐ `W3` — a `$clog2` source, the `F4` refusal on the FORWARDING lane — is CLOSED with
/// it (§2 "Index sealing" ⓑ). vita used to print the leaf's own default (`1 / 0`); it now
/// prints the `32 / fffffff6` both oracles print. The `W1` >64-bit source above is
/// untouched and is the control that says so: it is a `param_meta` absence, not a
/// `param_range` one, and no arm added here can reach it.
#[test]
fn an_unprovable_leaf_still_declines() {
    check(
        "module leaf #(parameter P = 1'b0);\n\
        \x20 initial $display(\"leaf bits=%0d val=%h\", $bits(P), P);\n\
         endmodule\n\
         module mid #(parameter WBIG = 72'h1);\n\
        \x20 localparam Q = ~WBIG;\n\
        \x20 initial $display(\"mid bitsW=%0d bitsQ=%0d Q=%h\", $bits(WBIG), $bits(Q), Q);\n\
        \x20 leaf #(.P(Q)) u();\n\
         endmodule\n\
         module t;\n\
        \x20 mid #(.WBIG(72'h1)) m();\n\
        \x20 initial #1 $finish;\n\
         endmodule\n",
        &[
            "mid bitsW=72 bitsQ=72 Q=fffffffffffffffffe",
            "leaf bits=72 val=fffffffffffffffffe",
        ],
    );
    check(
        "module leaf #(parameter P = 1'b0);\n\
        \x20 initial $display(\"leaf bits=%0d val=%h\", $bits(P), P);\n\
         endmodule\n\
         module mid #(parameter Q0 = 8'd1);\n\
        \x20 localparam A = $clog2(300);\n\
        \x20 localparam R = ~A;\n\
        \x20 initial $display(\"mid bitsA=%0d bitsR=%0d R=%h\", $bits(A), $bits(R), R);\n\
        \x20 leaf #(.P(R)) u();\n\
         endmodule\n\
         module t;\n\
        \x20 mid #(.Q0(4'd3)) m();\n\
        \x20 initial #1 $finish;\n\
         endmodule\n",
        &[
            "mid bitsA=32 bitsR=32 R=fffffff6",
            "leaf bits=32 val=fffffff6",
        ],
    );
}
