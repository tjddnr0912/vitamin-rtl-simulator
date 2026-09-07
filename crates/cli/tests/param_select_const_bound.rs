//! A parameter's BIT / PART / INDEXED-PART select folds in the elaborate-time
//! constant domain, so a width bound, dimension, replication count or array size
//! built from one stops silently collapsing to 1.
//!
//! `const_eval_in_scope` had exactly one select arm — a const-ARRAY-ELEMENT lookup
//! (`ROT[i]`) that declines for a scalar param — and no `PartSelect` / `IndexedPart`
//! arm at all. Every consumer of a constant bound therefore read `None` and clamped:
//!
//! | shape                                                   | PRE      | both oracles |
//! |---------------------------------------------------------|----------|--------------|
//! | `localparam [31:0] W=32'h34; logic [W[7:0]-1:0] v;`      | `$bits`=1| 52           |
//! | `logic [W[2]:0] v;` (bit-select)                         | 1        | 2            |
//! | `logic [W[7 -: 8]-1:0] v;`                               | 1        | 52           |
//! | `logic [39:8] B` → `logic [B[15:8]-1:0] v;`              | 1        | 52           |
//! | `logic [0:31] A` → `logic [A[24:31]-1:0] v;`             | 1        | 52           |
//!
//! ⭐ The VALUE was never in doubt: `$display("%0d", W[7:0])` already printed 52 in
//! all three tools. Only the const domain was blind, which is why the fix is an arm
//! and not an arithmetic.
//!
//! ⚠️ THE TWO DECLINE RULES ARE THE POINT, and each has a pin below.
//!
//! DIRECTION / DECLARED LSB. Extracting bits needs the base's DECLARED range, not
//! just its width — `[39:8]` and `[0:31]` both answer 52 for their low byte, and an
//! implementation that assumed `[w-1:0]` would have replaced one silent-wrong
//! (width 1) with a DIFFERENT one. `param_decl_range` grew an ascending arm for
//! this (`[0:31]` has LSB 0 like `[31:0]` and recorded nothing at all), which also
//! lifted the RUNTIME ascending select — `A[26]` was 0 against both oracles' 1.
//!
//! WRAPPING ARITHMETIC ABOVE A NARROW LEAF. `W[3:2]` is 2 bits, so `W[3:2]-4'd3`
//! wraps to 14 at 4 bits and `logic [W[3:2]-4'd3:0]` is 15 bits. The width-UNLIMITED
//! module fold answers −2 ⇒ 3, so a select now routes its enclosing arithmetic to
//! the width-honest walk. The select-free twin (`localparam [1:0] P; P-4'd3`) is a
//! pre-existing width-model gap and is deliberately NOT touched — it is pinned below
//! so this file fails if a later slice moves one without the other.
//!
//! ORACLES: iverilog 13.0 and verilator 5.050 agree on every value asserted here.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

const DECLS: &str = "\
  localparam logic [31:0] W = 32'h0000_0034;\n\
  localparam logic [39:8] B = 32'h0000_0034;\n\
  localparam logic [0:31] A = 32'h0000_0034;\n\
  localparam signed [7:0] S = -8'sd2;\n\
  localparam U = 300;\n\
  localparam logic [1:0] P = 2'd1;\n\
  localparam logic [7:0] QB = 8'h34;\n\
  localparam logic [127:0] K = 128'h0123_4567_89ab_cdef_fedc_ba98_7654_3210;\n";

/// Run `body` (statements printing `r=<n>` lines) with the shared param decls and
/// `extra` declarations, and return the joined `r=` values.
fn run(extra: &str, body: &str) -> Result<String, String> {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let src =
        format!("module t;\n{DECLS}{extra}  initial begin\n{body}    $finish;\n  end\nendmodule\n");
    let p = std::env::temp_dir().join(format!("vita_psel_{}_{n}.sv", std::process::id()));
    std::fs::write(&p, &src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&p)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&p);
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if !out.status.success() {
        return Err(all);
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.strip_prefix("r="))
        .collect::<Vec<_>>()
        .join("|"))
}

/// `$bits` of a net declared with `range`.
fn bits_of(range: &str) -> String {
    run(
        &format!("  logic {range} v;\n"),
        "    $display(\"r=%0d\", $bits(v));\n",
    )
    .unwrap_or_else(|e| panic!("expected success for `logic {range} v;`:\n{e}"))
}

#[test]
fn a_param_select_sizes_a_packed_range() {
    // The headline cell, and each select operator spelling of it. `W[7:0]` = 52.
    for r in ["[W[7:0]-1:0]", "[W[7 -: 8]-1:0]", "[W[0 +: 8]-1:0]"] {
        assert_eq!(bits_of(r), "52", "for `logic {r} v;`");
    }
    // A BIT-select is one bit wide and still a select: `W[2]` = 1, so `[1:0]`.
    assert_eq!(bits_of("[W[2]:0]"), "2");
    // …and a zero bit is a real answer, not a decline: `W[3]` = 0 ⇒ `[0:0]`.
    assert_eq!(bits_of("[W[3]:0]"), "1");
}

#[test]
fn the_declared_lsb_and_direction_decide_which_bits_are_named() {
    // ⚠️ THE PIN FOR DECLINE RULE 1. Both of these name the low byte = 52, and both
    // would read 0 if the fold assumed the base were `[w-1:0]`.
    assert_eq!(bits_of("[B[15:8]-1:0]"), "52"); // non-zero declared LSB
    assert_eq!(bits_of("[A[24:31]-1:0]"), "52"); // ASCENDING declared range
                                                 // The descending zero-LSB base is the control: same value, same answer, and it
                                                 // is the shape that must stay byte-identical through the `param_range` change.
    assert_eq!(bits_of("[W[7:0]-1:0]"), "52");
}

#[test]
fn an_ascending_param_select_reads_the_right_bits_at_runtime_too() {
    // ⭐ SIDE HARVEST of the `param_decl_range` ascending arm. `A` is `[0:31]`, so
    // index 26 is internal bit 31−26 = 5, and `32'h34` has bit 5 set. PRE answered
    // the RAW internal bit 26 = 0 against both oracles' 1 — a silent-wrong that had
    // no §2 bullet of its own.
    assert_eq!(
        run(
            "",
            "    $display(\"r=%0d\", A[26]);\n    $display(\"r=%0d\", A[5]);\n"
        )
        .unwrap(),
        "1|0"
    );
    // The descending twin of the same two indices, unchanged.
    assert_eq!(
        run(
            "",
            "    $display(\"r=%0d\", W[26]);\n    $display(\"r=%0d\", W[5]);\n"
        )
        .unwrap(),
        "0|1"
    );
}

#[test]
fn a_value_inferred_base_width_declines_instead_of_inventing_bits() {
    // ⚠️⚠️ THE REGRESSION THE ADVERSARIAL REVIEW CAUGHT, pinned. An UNTYPED param with
    // an EXPRESSION initializer has no declared width, and `param_decl_width` sizes it
    // from the folded VALUE (`min_signed_bits(v).max(32)`). `~8'hCB` is an 8-bit value
    // recorded as 32 bits, so a fold that trusted that width read bits 15:8 out of thin
    // air and `logic [(W[15:8])+8-1:0] v;` declared a **263-bit** net.
    //
    // PRE declined and agreed with iverilog (width 1). Trusting the inferred width was
    // therefore correct→silent-wrong, not a residue — so the fold takes its width ONLY
    // from `param_decl_range`, which answers for a declared range or a TYPE/LITERAL
    // width and stays silent for a value inference.
    for init in ["~8'hCB", "8'h30 + 8'h04", "{4'h3, 4'h4}", "8'hFF & 8'h34"] {
        assert_eq!(
            run(
                &format!("  localparam VI = {init};\n  logic [(VI[15:8])+8-1:0] v;\n"),
                "    $display(\"r=%0d\", $bits(v));\n"
            )
            .unwrap(),
            "1",
            "a value-inferred width must decline, for `{init}`"
        );
    }
    // The control: the SAME value with a DECLARED width folds, because now the bits
    // being named actually exist.
    assert_eq!(bits_of("[(W[15:8])+8-1:0]"), "8");
}

#[test]
fn the_base_is_read_at_its_declared_width_not_its_i64_container() {
    // A signed negative param carries sign bits above its declared width in the i64
    // container; those are not part of the value. `-8'sd2` is `1111_1110`, so the
    // low nibble is 14 — masking at the DECLARED width is what produces that.
    assert_eq!(bits_of("[S[3:0]:0]"), "15");
    // An UNTYPED decimal param is 32 bits by §6.20.2, and `300 >> 4` masked to 5
    // bits is 18.
    assert_eq!(bits_of("[U[8:4]:0]"), "19");
}

#[test]
fn arithmetic_above_a_select_wraps_at_the_select_width() {
    // ⚠️ THE PIN FOR DECLINE RULE 2. `W[3:2]` is a 2-BIT value (= 1), so `- 4'd3` is
    // 4-bit arithmetic and wraps to 14 ⇒ `[14:0]` = 15 bits. The width-unlimited
    // fold would answer −2 ⇒ 3 — a different wrong answer, not a fix.
    assert_eq!(bits_of("[W[3:2]-4'd3:0]"), "15");
    // The non-wrapping sibling: an unsized `1` is 32 bits, so nothing wraps.
    assert_eq!(bits_of("[W[7:0]-1:0]"), "52");
    // ⚠️ EVERY operator above the select, not just a Binary one. The first draft put
    // the redirect in `const_eval_in_scope`'s Binary arm, and the adversarial review
    // showed the Unary arm walked straight past it: `~W[3:0]` is a 4-bit `~4` = 11
    // ⇒ 12 bits, and the unlimited walk answered −5 ⇒ 6 — silent-wrong traded for a
    // different silent-wrong. Putting the rule on the CONSUMER covers the whole node.
    assert_eq!(bits_of("[~W[3:0]:0]"), "12");
    assert_eq!(bits_of("[(1 ? W[3:0] : 4'd0) + 4'd15:0]"), "4");
}

#[test]
fn a_parameter_value_keeps_its_assignment_context() {
    // ⚠️⚠️ THE OTHER HALF OF THAT SAME MISTAKE, and the worse one. A range bound is a
    // self-determined position, but a PARAMETER'S OWN VALUE is an ASSIGNMENT (§11.6):
    // the RHS evaluates at max(self width, target width). Redirecting inside the
    // SHARED evaluator imposed the self-determined width on both, so
    // `localparam int Q = W[7:0] + 8'd240;` folded 36 where both oracles fold 292 —
    // and it had been honest-loud before, i.e. loud→silent-wrong.
    assert_eq!(
        run(
            "  localparam int Q = W[7:0] + 8'd240;\n  logic [15:0] Q2 = W[3:0] + 4'd1;\n",
            "    $display(\"r=%0d\", Q);\n    $display(\"r=%0d\", Q2);\n"
        )
        .unwrap(),
        "292|5"
    );
}

#[test]
fn a_selects_own_index_is_self_determined_too() {
    // §11.5.1 + Table 11-21: a select's index is a constant expression in a
    // SELF-DETERMINED position, so `4'd15 + 4'd1` wraps to 0 and names bit 0. Folding
    // the index in the width-unlimited domain read bit 16 instead, which made the
    // bound below 10 bits where both oracles say 9 — while vita's OWN runtime lane
    // answered bit 0 in the same run.
    assert_eq!(
        run(
            "  localparam logic [31:0] WF = 32'hFFFF_0034;\n  logic [WF[4'd15+4'd1]+8:0] v;\n",
            "    $display(\"r=%0d\", $bits(v));\n    $display(\"r=%0d\", WF[4'd15+4'd1]);\n"
        )
        .unwrap(),
        "9|0"
    );
}

#[test]
fn an_equal_endpoint_part_select_is_legal_in_both_directions() {
    // §11.5.1: `[5:5]` is the ordinary one-bit slice and a parameterised `[K:K]`
    // degenerates to it. The first direction table asked `a <= b` for the descending
    // case, which is `a > b` — so the descending equal-endpoint form declined while
    // its ascending twin folded, from one table.
    assert_eq!(bits_of("[W[5:5]+8:0]"), "10"); // descending base
    assert_eq!(bits_of("[A[26:26]+8:0]"), "10"); // ascending base
}

#[test]
fn a_negative_declared_bound_declines_rather_than_recording_a_clamped_lie() {
    // ⚠️⚠️ `param_range` cannot hold a negative `lo`, and the old code wrote
    // `min(l).max(0)` — a lie that was inert only because the `lo == 0` early return
    // filtered every negative range out. Removing that return to admit the ascending
    // case let the lie reach both consumers: an ascending `[-2:3]` recorded
    // `(0, 6, true)` and turned `A[0]`/`A[3]` from the correct 0/0 into 1/1 against
    // both oracles. correct→silent-wrong; declining restores the pre-existing
    // behaviour of the separately-tracked negative-bound class.
    assert_eq!(
        run(
            "  localparam logic [-2:3] NA = 6'b110100;\n",
            "    $display(\"r=%0d\", NA[0]);\n    $display(\"r=%0d\", NA[3]);\n"
        )
        .unwrap(),
        "0|0"
    );
}

#[test]
fn the_select_free_twin_of_the_wrapping_bound_folds_at_its_own_width() {
    // Was the anti-sweep pin `3`: the select-free bound reached the width-UNLIMITED
    // fold. §4.5.423 routes every declared-range bound through the self-determined
    // walk, so `P-4'd3` wraps at 4 bits like its select twin — 15 bits in both oracles.
    assert_eq!(bits_of("[P-4'd3:0]"), "15");
}

#[test]
fn a_select_drives_the_other_constant_bound_consumers() {
    // The bound funnel is shared, so the same fold has to reach every consumer.
    // Unpacked dimension: `W[3:2]` = 1 word.
    assert_eq!(
        run(
            "  logic [7:0] mem [W[3:2]];\n",
            "    $display(\"r=%0d\", $size(mem));\n"
        )
        .unwrap(),
        "1"
    );
    // Replication count and an indexed-part WIDTH.
    assert_eq!(
        run(
            "  logic [63:0] q = 64'h0123456789ABCDEF;\n",
            "    $display(\"r=%0d\", $bits({W[3:2]{1'b1}}));\n\
             \x20   $display(\"r=%0d\", q[0 +: W[7:4]]);\n"
        )
        .unwrap(),
        "1|7"
    );
}

#[test]
fn a_wider_than_64_bit_base_selects_the_bits_it_names() {
    // ⭐ This pin asserted a LOUD reject and explained it as "OUT OF SCOPE, PINNED: a
    // >64-bit parameter is deliberately kept out of the i64 `params` table, so a select
    // on one still declines". The premise was true and the conclusion did not follow —
    // the VALUE lives in `wide_param_bits`, which carries bits and width and sign, and
    // a select is PLACEMENT: each result bit is an operand bit at a known position, so
    // nothing about the i64 table is needed to read one. iverilog: 16.
    //
    // The diagnostic this test was named for still exists for the shapes that DO
    // decline (an ascending select, an out-of-range one) — see `every_decline_rule_has_a_pin`.
    let out = run(
        "  logic [K[7:0]-1:0] v;\n",
        "    $display(\"r=%0d\", $bits(v));\n",
    )
    .expect("a select of a wide parameter folds");
    assert_eq!(out, "16", "K[7:0] is 0x10 = 16 (iverilog agrees)");
}

#[test]
fn a_real_param_select_says_it_is_real_instead_of_calling_it_undefined() {
    // OUT OF SCOPE, PINNED. `R[7:0]` on a real param is loud in every tool (iverilog:
    // "can not select part of real parameter: R"), and it was loud here too — but with
    // the WRONG REASON: `count_reads_real_param` had no select arms, so the first gate
    // of `check_const_range_bound` missed and control fell through to "undefined name
    // `R`" about a parameter declared one line up. The plain `[R-1:0]` twin already
    // said the true thing, so this is one rule that had two answers.
    let err = run(
        "  localparam real RP = 52.0;\n  logic [RP[7:0]-1:0] v;\n",
        "    $display(\"r=%0d\", $bits(v));\n",
    )
    .expect_err("a real base must stay loud");
    assert!(
        err.contains("a real parameter is not an integral constant"),
        "{err}"
    );
    assert!(!err.contains("undefined name"), "{err}");
}

#[test]
fn a_const_array_element_read_is_not_treated_as_a_bit_select() {
    // ⚠️ THE ARM-ORDER PIN. `ROT[i]` is also an `ExprKind::BitSelect`, and it names a
    // whole 32-bit ELEMENT rather than one bit. The array lookup stays FIRST and the
    // self-width answer declines for it, so GAP-G keeps its exact behaviour: element
    // 1 is 7, and a `[ROT[1]:0]` net is 8 bits, not 2.
    assert_eq!(
        run(
            "  localparam int ROT [0:1] = '{3, 7};\n  logic [ROT[1]:0] e;\n",
            "    $display(\"r=%0d\", $bits(e));\n"
        )
        .unwrap(),
        "8"
    );
}

#[test]
fn every_decline_rule_has_a_pin() {
    // ⚠️ THE MUTATION-BATTERY PINS. Each of these guards SURVIVED the battery — the
    // suite could not tell the guard from its absence — so each one is fixed here by
    // the property it protects, not by a value that happens to fall out.
    //
    // OUT OF RANGE (§11.5.1: the outside bits read `x`, which this integer domain
    // cannot represent). `Q` is 8 bits, so `Q[15:8]` is entirely outside it: iverilog
    // sizes the net 1 and prints `x` for the read, and vita must do BOTH — the const
    // lane must not out-run its own runtime lane. Dropping the check made the const
    // lane invent eight zero bits (9) while the runtime still said `x`.
    // ⚠️ verilator is NOT the oracle here: it is 2-state, zero-fills the select to 52,
    // and contradicts iverilog — the documented out-of-range disqualification.
    assert_eq!(bits_of("[QB[15:8]+8:0]"), "1");
    assert_eq!(
        run("", "    $display(\"r=%0d\", QB[15:8]);\n").unwrap(),
        "x",
        "the const lane must not answer where the runtime lane reads x"
    );

    // OUT OF ORDER. A part-select runs in the BASE's declared direction; the other way
    // round is illegal and iverilog rejects it outright ("Part select W[3:5] is out of
    // order"). Folding it as `(min, max)` would answer for a select the language does
    // not have.
    assert_eq!(bits_of("[W[3:5]+8:0]"), "1"); // descending base, ascending select
    assert_eq!(bits_of("[A[31:24]+8:0]"), "1"); // ascending base, descending select

    // WIDER THAN THE i64 CONST DOMAIN. 64 unsigned magnitude bits do not fit, so a
    // 64-bit select declines rather than wrapping into a negative bound. (iverilog
    // itself warns here: "verinum::as_long() truncated 64 bits to 63".)
    assert_eq!(
        run(
            "  localparam logic [63:0] L64 = 64'hFFFF_FFFF_FFFF_FF34;\n  logic [L64[63:0]:0] c;\n",
            "    $display(\"r=%0d\", $bits(c));\n"
        )
        .unwrap(),
        "1"
    );

    // ZERO-WIDTH INDEXED PART-SELECT. §11.5.1 requires a positive constant width;
    // iverilog rejects `W[7 +: 0]`. Accepting it would fold a span of no bits.
    assert_eq!(bits_of("[W[7 +: 0]+8:0]"), "1");
}

#[test]
fn an_inner_scalar_shadowing_a_const_array_keeps_both_lanes_on_one_object() {
    // ⚠️⚠️ THE ARM-ORDER MIRROR, and the reason it exists. `const_eval_in_scope`'s
    // `BitSelect` arm tries the const-ARRAY lookup FIRST, so the width helper must
    // refuse every base that lookup claims — otherwise the value arm and the width
    // helper resolve the SAME base to two different objects.
    //
    // They did, in exactly one shape: an inner scalar shadowing an outer const array.
    // `lookup_scoped` found the inner 99 (width 1) while the value arm answered the
    // OUTER element 20, and masking 20 to one bit gave 0 — `$bits` went 21 → 1.
    // ⚠️ 21 is ALSO wrong (verilator, the sole oracle here because iverilog rejects
    // unpacked array parameters outright, says 2) — but trading one silent wrong for
    // a DIFFERENT silent wrong is the move the ladder forbids, so this pins PRE.
    //
    // §4.5.416 closed the root: `const_array_ref_of_base` (the one resolution the
    // value table and its geometry twin share) takes the INNERMOST binding of the name
    // over the combined set the value lookups walk and answers only when that binding
    // IS the array, so the bound is the inner scalar's bit 1 → 2 (verilator).
    //
    // §4.5.446 closed the RUNTIME half (ROADMAP §2 🆕 O): the read `ROTA[1]` printed the
    // outer array's element (20) because `expr_array_chain` resolved the bare name with
    // the `symbols`-only walk. It now asks `bare_ident_route` like the lowering does, so
    // both lanes answer the inner scalar's bit 1 and the two lines agree. verilator is
    // the sole oracle here and prints `2` then `1`.
    assert_eq!(
        run(
            "  localparam int ROTA [0:3] = '{10, 20, 30, 40};\n\
             \x20 generate if (1) begin : g\n\
             \x20   localparam int ROTA = 99;\n\
             \x20   logic [ROTA[1]:0] v;\n\
             \x20   initial begin #1; $display(\"r=%0d\", $bits(v)); $display(\"r=%0d\", ROTA[1]); end\n\
             \x20 end endgenerate\n",
            "    #2;\n"
        )
        .unwrap(),
        "2|1"
    );
}
