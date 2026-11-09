//! Range rules: the declared-range record and every rule that derives a count, a width, a
//! low / high bound, a direction, an offset, a stride or an index position from a pair of
//! bounds, in one home.
//!
//! History (owner, 2026-11-06, ROADMAP §5.2 Q-VALRULES fix (d)): the range record and every
//! rule that reads a PAIR OF BOUNDS were collected into this one file, so an edit to one rule
//! is reviewed beside its siblings and no declaration kind keeps a private copy elsewhere
//! (ER §5.1). The boundary is a rule, not a token: a pair of bounds DECLARED (packed and
//! unpacked dimensions of nets, ports, parameters, struct members, typedefs, instance arrays,
//! formals) or SELECTED (part-select spans), every product over recorded dimensions, the
//! §20.7 range queries, and the struct layout over member widths. Where PRE's declaration
//! kinds computed differently, each difference is a named variant ([`Lane`], or a named
//! function) that copies PRE verbatim, so moving it here moved no value; Q-PARAM2STATE S1v
//! unifies the variants one by one, each a visible edit here. Splitting this file is left to
//! the periodic code review.
//!
//! Slice A is STRUCTURE ONLY (owner, 2026-11-07): PRE's values AND PRE's loudness. The refusals
//! first designed with it -- R1w (an unsigned 64-bit bound >= 2^63), R1x (an `x` bound), C1 (an
//! x / z literal bound) and R1c (an element count past `integer`) -- moved to Q-PARAM2STATE S1v,
//! beside R1v and the bound range rule: every refusal slice A added was returned as a `None` /
//! `Err` that a PRE composition read as a value (a missing width as unlimited, a declined call,
//! the caller scope's text), seven instances in five review rounds (F24, refusal-as-value). S1v
//! adds each as a side diagnostic raised where the bound is declared, with PRE's value going on.
//!
//! The record is built in PRE's EVALUATION ORDER (premise6, 2026-11-07): a bound fold can call a
//! constant function, recurse or cost seconds, so the lsb is folded only once the msb folded
//! ([`RangeFacts::lazy`], PRE's `?` chains, the default), both first only where PRE folded the
//! pair ([`RangeFacts::pair`]; `tests/range_rules_home.rs` PAIR_SITES pins those by function).
//!
//! OUTSIDE this file, by rule (each reads or feeds a rule here, none derives from a pair):
//! * EVALUATORS, which fold one expression to a value: hdl-parser `const_fold.rs`'s
//!   `const_bound`, `const_lit` (passed in to the builders below as a [`LitFold`]); elaborate
//!   `fold_self_at`, `const_eval_in_scope`, the `$bits` fold; the evaluators' comparison
//!   arms (`a < b` over two operand values).
//! * TABLES: hdl-parser `typedefs.rs`'s alias slot (Q-SYMMIX F1).
//! * The parser's GATES, which only decide whether a range is symbolic: `names_an_overridable`,
//!   the `lsb == 0` test, `sym_struct.rs`'s member gate.
//! * SELF-WIDTH rules over operand widths, not bounds: a replication `{n{x}}`'s width, and the
//!   larger of two operand widths (IEEE 1800-2017 §11.6.1).
//! * VALUE RANGES that are not declared or selected ranges: coverage bins, constraint
//!   ranges, SVA delay windows, a queue slice and `$urandom_range` at run time, bit-interval
//!   sets, source spans and text offsets.
//! * CRATE-EDGE and literal-text exceptions (no hdl-ast edge, or the operand is a literal's
//!   text): sim-engine `task_frames.rs` (x2, F21, a copy of [`Lane::PartSelectSub`]) and
//!   `builtins/pattern.rs` (a runtime pattern's stride); the runtime copies of a select's
//!   index rules over what sim-ir records (lens2 CD1, owner Q-SIDETABLES): sim-ir `lib.rs`
//!   `sel_written_bits` and sim-engine `state/changes.rs`, `state/frame_eval.rs` (`[off -: w]`'s
//!   low index, [`indexed_last`]'s rule), sim-ir `selfwidth.rs` `part_width` (`|m - l| + 1` in
//!   the declared direction); elaborate `string_array_route.rs` and hdl-parser
//!   `expr_primary.rs` (a literal's magnitude); elaborate `packed.rs` `norm_sub_k` (offset
//!   arithmetic on an already-derived `k`).
//!
//! ENFORCED by `crates/hdl-ast/tests/range_rules_home.rs` (its lists, verbatim), outside this
//! file, in every crate's `src`, `//` comments and string literals skipped:
//! * the tokens `abs_diff(` and `unsigned_abs(`;
//! * `.abs() + 1` (also `.abs() as T + 1`);
//! * a span `- B + 1` (also `- B) + 1`, `- B as T + 1`), `B` in LOW_NAMES = `lo` `l` `lsb`
//!   `lo_n` `c_lo` `right` `low` `min`;
//! * `.min(` / `.max(` over two names in PAIR_NAMES = `m` `l` `msb` `lsb` `left` `right`
//!   `ia` `ib` `a` `b`;
//! * a direction test `A < B` / `A > B` / `A <= B` / `A >= B` over a pair in DIR_PAIRS =
//!   (`m`, `l`) (`msb`, `lsb`) (`left`, `right`), either order, a `x.` prefix and a `*`
//!   allowed;
//! * a dimension product: `.product()`, `fold(1u32`, `fold(1u64`, `fold(1,`;
//! * a checked span: `checked_add(1)` / `saturating_add(1)` within 8 tokens of a
//!   `checked_sub(` / `saturating_sub(` on the same line;
//! * a span with its 1 first: `1 + A - B` and `+ 1 - B`, `B` in LOW_NAMES;
//! * a span over a low accessor: `- X.B() + 1` (also `- X.B() as T + 1`), `B` in LOW_NAMES;
//! * a free `min(A, B)` / `max(A, B)` (not a method call), `A` and `B` in PAIR_NAMES;
//! * an indexed part-select's low / high index: `- W + 1` (also `- W as T + 1`) and `+ 1 - W`,
//!   `W` in WIDTH_NAMES = `w` `width` `ww`.
//!
//! Its EXCEPTIONS (each with its reason there, the count exact): elaborate `packed.rs`
//! unsigned_abs x1, `string_array_route.rs` unsigned_abs x1, `cover_bins.rs` span x1,
//! `cover_synth.rs` span x1, `crv.rs` span x1, `sva_seq.rs` span x1,
//! `const_fn_width/fold.rs` min/max x1 (self-width); hdl-parser `expr_primary.rs`
//! unsigned_abs x1; sim-engine `builtins/crv_draw.rs` abs_diff x1 + span x3,
//! `state/task_frames.rs` unsigned_abs x2, `builtins/pattern.rs` product x1,
//! `builtins/queues_io.rs` span x1, `eval/sysfunc.rs` span x1, `state/changes.rs` indexed
//! low x1, `state/frame_eval.rs` indexed low x1 (crate-edge); sim-ir `selfwidth.rs` min/max
//! x1 (self-width) + checked span x1 (crate-edge), `lib.rs` indexed low x1 (crate-edge),
//! `selfwidth/edge_value_tests.rs` span x1 + direction x2 (a test's hand oracle); cli
//! `pipeline.rs` min/max x1 (text offsets).
//!
//! REVIEW-ONLY (no token test sees them): a bare `*` of a width by a count (`rem * w`), an
//! index offset `i - lo`, a bound pair under names outside the lists above, a direction test
//! over a selected pair named `a` / `b` (the evaluators' comparison arms share that
//! spelling; the routed select sites are the pattern: `!descending(a, b)` for `a <= b`), a
//! rule spelled through a helper defined outside this file, a span over a HIGH accessor
//! alone (`f.hi() - x`), a product written as a loop or a bare `*=`, a `checked_add(1)`
//! more than 8 tokens from its `checked_sub(` or on another line, and a select's
//! offset arithmetic over names outside WIDTH_NAMES (`off - n + 1`).
//!
//! The file has five parts, in this order:
//! 1. the record ([`RangeFacts`]): elaborate's one range record, its exact count;
//! 2. the bound value rule ([`bound_step`]): elaborate's router decision (F1a);
//! 3. the lane variants ([`Lane`]): every count / width rule a declaration kind spells on
//!    its own;
//! 4. direction, low / high, positions, the §20.7 queries, dimension products and select
//!    spans, each a named function copying its PRE spelling;
//! 5. the parser's respell builders: the AST a parse-time lane writes for elaborate to fold
//!    in context (multi-dimensional packed parameters, symbolic typedef and struct-member
//!    widths, the symbolic struct layout, package-constant respells), with the literal
//!    helpers they fold through.

use crate::{BinOp, Expr, ExprKind, IntLitKind, Range, Span, UnOp};

/// The literal fold the parser's builders read an operand through: the parser's evaluator
/// (`hdl-parser` `Parser::const_lit`), passed in, because an evaluator stays where its
/// state is (Q-VALRULES (d) K4-1).
pub type LitFold = fn(&Expr) -> Option<i64>;

// ───────────────────────────── 1. the record ─────────────────────────────

/// A parameter's DECLARED packed range: `(lo, width, ascending)` — the tuple the provenance
/// maps (`param_range`, `pkg_const_range`) carry, named so the save/restore lists that
/// shuttle it stay readable.
pub type DeclRange = (i64, u32, bool);

/// THE RANGE RECORD (ER §5.1): one declared range's value facts — each bound by the
/// declared-range router (elaborate's `const_range_bound_fold`, whose decision is
/// [`bound_step`]: PRE's value rule F1a), the element count `|msb - lsb| + 1`, the direction.
/// The count is EXACT (`u128`: two i64 bounds give at most 2^64) and carries no limit: each
/// consumer reduces the record to its own carrier with its declaration kind's PRE spelling over
/// the record's bounds ([`Lane`]), so every PRE cap, decline and wrap is where it was. The
/// `integer` limit on the count (R1c, IEEE 1800-2017 §20.6.2) is S1v's. msb / lsb are UNCHECKED
/// i64; a consumer that narrows them is bound-past-integer, owned with R1v by S1v.
/// Lane [`Lane::Record`]: every elaborate consumer that reads `range_facts`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangeFacts {
    pub msb: i64,
    pub lsb: i64,
    /// `|msb - lsb| + 1`, exact.
    pub count: u128,
}

impl RangeFacts {
    /// The record of two folded bounds (no refusal: the count is exact).
    pub fn new(msb: i64, lsb: i64) -> Self {
        Self {
            msb,
            lsb,
            count: u128::from(msb.abs_diff(lsb)) + 1,
        }
    }
    /// THE EVALUATION ORDER, lazy (PRE's `let m = fold(msb)?; let l = fold(lsb)?;`): `lsb` is
    /// called only when `msb` answered. Folding a bound is not free -- it can call a constant
    /// function, recurse through a routine's own return range, or cost seconds -- and PRE's
    /// consumers relied on a failed msb to stop there (V/premise6 B1: a module routine whose
    /// return range calls itself, `[v : h(1)] h`, recursed without end when both were folded;
    /// B2: up to 48x the wall). The order every new consumer takes.
    pub fn lazy(
        msb: impl FnOnce() -> Option<i64>,
        lsb: impl FnOnce() -> Option<i64>,
    ) -> Result<Self, RangeUnfolded> {
        let m = msb().ok_or(RangeUnfolded)?;
        let l = lsb().ok_or(RangeUnfolded)?;
        Ok(Self::new(m, l))
    }
    /// THE EVALUATION ORDER, pair (PRE's `match (fold(msb), fold(lsb))`): both bounds were
    /// folded, msb first, before either is looked at -- the caller's argument list does the
    /// folding, so the order is visible where it is spelled. Kept only where PRE spelled the
    /// pair (elaborate `range_facts_pair`, its sites pinned in `tests/range_rules_home.rs`).
    pub fn pair(msb: Option<i64>, lsb: Option<i64>) -> Result<Self, RangeUnfolded> {
        match (msb, lsb) {
            (Some(m), Some(l)) => Ok(Self::new(m, l)),
            _ => Err(RangeUnfolded),
        }
    }
    pub fn lo(&self) -> i64 {
        low(self.msb, self.lsb)
    }
    pub fn hi(&self) -> i64 {
        self.msb.max(self.lsb)
    }
    pub fn asc(&self) -> bool {
        ascending(self.msb, self.lsb)
    }
    pub fn desc(&self) -> bool {
        self.msb > self.lsb
    }
    /// A parameter's declared packed range (elaborate `params.rs`): the width is
    /// [`Lane::ParamRange`]'s PRE spelling.
    pub fn decl_range(&self) -> DeclRange {
        (
            self.lo(),
            Lane::ParamRange.width_u32(self.msb, self.lsb),
            self.asc(),
        )
    }
}

/// Why [`RangeFacts`] was not built: a bound of the range did not fold (elaborate's router
/// answered None for it). The one reason in slice A -- every consumer handles it where PRE's own
/// `?` chain or pair match handled a None, so the reason changes no output; a refusal is never
/// one (S1v).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangeUnfolded;

// ─────────────────────────── 2. the bound value rule ───────────────────────────

/// What elaborate's declared-range router does after its self-determined first walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundStep {
    /// The first walk's value is the bound.
    Value(i64),
    /// Run the width-unlimited walk (`const_eval_in_scope`) and take its value.
    Wide,
}

/// The bound VALUE rule (PRE's, F1a): the first walk's value when it is at most `i32::MAX`,
/// else the width-unlimited walk. `first` is the first walk's value when it produced one
/// (`CFold::Val`).
pub fn bound_step(first: Option<i64>) -> BoundStep {
    match first {
        Some(v) if v <= i64::from(i32::MAX) => BoundStep::Value(v),
        _ => BoundStep::Wide,
    }
}

// ───────────────────────────── 3. the lane variants ─────────────────────────────

/// One variant per count / width spelling PRE's declaration kinds wrote differently, named
/// after the declaration kind that owns it (others that spelled it the same way are listed).
/// Each arm below is PRE's spelling verbatim, with PRE's carrier and overflow behaviour; a
/// variant is called through the method of its carrier (`count_*` when the lane declines,
/// `width_*` when it always answers) and any other method is a programming error.
/// S1v unifies variants one by one (D3 lane table: V/plan5/PLAN.txt).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    /// elaborate's record ([`RangeFacts::new`]): the exact count of every range `range_facts`
    /// builds (net, port, typedef `$bits`, md param via elaborate, fn return / local, instance
    /// array, assoc key, string array, type-param carrier (E3-b), generate-for, queries). No
    /// consumer reads it in slice A: each reduces the record's bounds with its own lane below.
    Record,
    /// parser struct member (`structs/mod.rs` `member_width`, every `.member_width(` caller);
    /// also elaborate's select span (`const_array.rs`). u32, declines past u32.
    StructMember,
    /// literal / self-determined helpers that never saw the record: elaborate `net_util.rs`
    /// (literal net), `hoist/arms.rs` (literal part-select), `expr_size_hier.rs` (env fold),
    /// `inline_ret_select.rs`, `expr_ctx.rs`, `expr_size_ctx.rs`. u32 of the u64 count.
    LiteralNet,
    /// parser parameter value-fits check (`params.rs`): u32 count, 64 when it does not fit.
    ParamFits,
    /// parser parameter width with sign (`params.rs`): u32 of the u64 count, 0 when it does
    /// not fit.
    ParamSigned,
    /// parser enum base (`typedefs.rs`, literal bounds): checked subtraction, under 64 bits.
    EnumBase,
    /// part-select / declared range spelled by subtraction, truncated to u32: parser
    /// `struct_sel.rs`, elaborate `cover_bins.rs` (x2), sim-engine `task_frames.rs` (x2).
    PartSelectSub,
    /// elaborate array formal / actual (`array_formal.rs` x2): u64 count clamped to u32; also
    /// `array_geom.rs` (unpacked extents) and `packed.rs` (packed extents).
    ArrayFormal,
    /// elaborate array formal's declared unpacked dims and element width (`array_formal.rs`
    /// x2): u64 count truncated to u32.
    FormalDims,
    /// elaborate parameter declared range (`params.rs`: value-inferred width, `decl_range`)
    /// and enum base (`net_util.rs` x2): u64 difference truncated to u32, then `+ 1`.
    ParamRange,
    /// elaborate associative-array key width (`assoc_key.rs`): u64, saturating.
    AssocKey,
    /// elaborate constant-function formal / local width (`const_fn_width.rs` x2): u32,
    /// declines past u32.
    FnWidth,
    /// elaborate constant-function return width (`inline_fn.rs`): u32 of `|m - l| + 1` by
    /// subtraction, declines past u32.
    FnReturn,
    /// u64 count, no limit: parser parameter width (`params.rs`), elaborate
    /// `expr_size_hier.rs` (fold width), `array_geom.rs` (negative-bound net, clamped net),
    /// `instance_array.rs` (x3), `net_util.rs` (`$bits` prescan).
    Wide64,
    /// parser struct member array (`member_array.rs`): `[hi:0]` with `0 <= hi <= i32::MAX`.
    MemberArray,
    /// parser typedef cast width (`typedefs.rs`): i64, no limit.
    TypedefCast,
    /// elaborate string array, checked subtraction (`string_array_route.rs` decl).
    StringArrayChecked,
    /// elaborate string array pattern (`string_array_route.rs` pattern): plain subtraction.
    StringArrayPlain,
    /// parser md param whose dims are literals (`packed_md.rs` `lit_dim_count`, u32 literals).
    MdLiteral,
    /// parser SVA local variable (`sva_seq.rs`, u32 small constants): unchecked u32.
    SvaLocal,
}

impl Lane {
    /// u32-carried lanes that decline (`None`): StructMember, LiteralNet, EnumBase, FnWidth,
    /// FnReturn.
    pub fn count_u32(self, m: i64, l: i64) -> Option<u32> {
        match self {
            Lane::StructMember => u32::try_from(m.abs_diff(l)).ok()?.checked_add(1),
            Lane::LiteralNet => u32::try_from(m.abs_diff(l) + 1).ok(),
            Lane::FnWidth => u32::try_from(m.abs_diff(l).checked_add(1)?).ok(),
            Lane::FnReturn => u32::try_from((m - l).unsigned_abs() + 1).ok(),
            Lane::EnumBase => match m.checked_sub(l) {
                Some(d) if d.unsigned_abs() < 64 => Some(d.unsigned_abs() as u32 + 1),
                _ => None,
            },
            _ => unreachable!("range_rules: lane {self:?} has no declining u32 count"),
        }
    }

    /// u32-carried lanes that always answer: ParamFits, ParamSigned, PartSelectSub,
    /// ArrayFormal, FormalDims, ParamRange.
    pub fn width_u32(self, m: i64, l: i64) -> u32 {
        match self {
            Lane::ParamFits => u32::try_from(m.abs_diff(l))
                .ok()
                .and_then(|d| d.checked_add(1))
                .unwrap_or(64),
            Lane::ParamSigned => u32::try_from(m.abs_diff(l) + 1).unwrap_or(0),
            Lane::PartSelectSub => (m - l).unsigned_abs() as u32 + 1,
            Lane::ArrayFormal => (m.abs_diff(l) + 1).min(u32::MAX as u64) as u32,
            Lane::FormalDims => (m.abs_diff(l) + 1) as u32,
            Lane::ParamRange => m.abs_diff(l) as u32 + 1,
            _ => unreachable!("range_rules: lane {self:?} has no total u32 width"),
        }
    }

    /// u64-carried lane that declines: MemberArray.
    pub fn count_u64(self, m: i64, l: i64) -> Option<u64> {
        match self {
            Lane::MemberArray => {
                if l != 0 || !(0..=i64::from(i32::MAX)).contains(&m) {
                    return None;
                }
                Some(m as u64 + 1)
            }
            _ => unreachable!("range_rules: lane {self:?} has no declining u64 count"),
        }
    }

    /// u64-carried lanes that always answer: Wide64, AssocKey.
    pub fn width_u64(self, m: i64, l: i64) -> u64 {
        match self {
            Lane::Wide64 => m.abs_diff(l) + 1,
            Lane::AssocKey => m.abs_diff(l).saturating_add(1),
            _ => unreachable!("range_rules: lane {self:?} has no total u64 width"),
        }
    }

    /// i64-carried lane: TypedefCast.
    pub fn width_i64(self, m: i64, l: i64) -> i64 {
        match self {
            Lane::TypedefCast => m.abs_diff(l) as i64 + 1,
            _ => unreachable!("range_rules: lane {self:?} has no i64 width"),
        }
    }

    /// usize-carried lanes: StringArrayChecked, StringArrayPlain.
    pub fn count_usize(self, m: i64, l: i64) -> Option<usize> {
        match self {
            Lane::StringArrayChecked => {
                Some(usize::try_from(m.checked_sub(l)?.unsigned_abs()).ok()? + 1)
            }
            Lane::StringArrayPlain => Some(usize::try_from((m - l).unsigned_abs()).ok()? + 1),
            _ => unreachable!("range_rules: lane {self:?} has no usize count"),
        }
    }

    /// u32-literal lane that declines: MdLiteral.
    pub fn count_small(self, m: u32, l: u32) -> Option<u32> {
        match self {
            Lane::MdLiteral => m.abs_diff(l).checked_add(1),
            _ => unreachable!("range_rules: lane {self:?} has no small count"),
        }
    }

    /// u32-constant lane that always answers: SvaLocal.
    pub fn width_small(self, m: u32, l: u32) -> u32 {
        match self {
            Lane::SvaLocal => m.abs_diff(l) + 1,
            _ => unreachable!("range_rules: lane {self:?} has no small width"),
        }
    }
}

// ─────────── 4. direction, positions, queries, dimension products, select spans ───────────

/// Direction: a range is ascending when its left bound is below its right bound (the
/// record's `asc`, the parser's struct-member `member_ascending`, and every `m < l` /
/// `m >= l` test over a declared or selected pair).
pub fn ascending<T: PartialOrd>(m: T, l: T) -> bool {
    m < l
}

/// Direction: strictly descending, `m > l` (a one-element range is neither).
pub fn descending<T: PartialOrd>(m: T, l: T) -> bool {
    m > l
}

/// The low bound (the record's `lo`, the parser's struct-member `member_dbase`).
pub fn low<T: Ord>(m: T, l: T) -> T {
    m.min(l)
}

/// The high bound (the record's `hi`).
pub fn high<T: Ord>(m: T, l: T) -> T {
    m.max(l)
}

/// `(low, high)` of a pair (unpacked-array slice, function-return part-select).
pub fn low_high<T: Ord + Copy>(m: T, l: T) -> (T, T) {
    (m.min(l), m.max(l))
}

/// `(low, high)` by the `<=` test (string-array dimension, pattern position).
pub fn ordered(m: i64, l: i64) -> (i64, i64) {
    if m <= l {
        (m, l)
    } else {
        (l, m)
    }
}

/// Is `v` inside the range `[m:l]`, either direction (md parameter select)?
pub fn within(m: i64, l: i64, v: i64) -> bool {
    m.min(l) <= v && v <= m.max(l)
}

/// The index step walking `[left:right]` from its left bound (string-array pattern).
pub fn step(left: i64, right: i64) -> i64 {
    if left <= right {
        1
    } else {
        -1
    }
}

/// The low bound as an array formal's / actual's u32, clamped at 0 (`array_formal.rs`).
pub fn low_u32_clamped(m: i64, l: i64) -> u32 {
    m.min(l).max(0) as u32
}

/// The low bound as a u32, unclamped (`array_formal.rs` formal dims; the caller refused a
/// negative bound first).
pub fn low_u32(m: i64, l: i64) -> u32 {
    m.min(l) as u32
}

/// The negative lsb a descending declared range normalizes against (`array_geom.rs`
/// `declared_neg_lsb`).
pub fn neg_lsb_desc(m: i64, l: i64) -> Option<i64> {
    (l < 0 && m >= l).then_some(l)
}

/// The lsb an ascending declared range with a negative msb normalizes against
/// (`array_geom.rs` `declared_asc_lsb`).
pub fn neg_lsb_asc(m: i64, l: i64) -> Option<i64> {
    (m < l && m < 0).then_some(l)
}

/// Offset of a negative low bound, as the u32 elaborate's array index adds (`array_geom.rs`).
pub fn neg_lo_offset_u32(lo: i64) -> u32 {
    lo.unsigned_abs().min(u32::MAX as u64) as u32
}

/// The `k` of `raw - lsb` for a negative declared lsb (`packed.rs` `norm_sub_k`).
pub fn neg_lsb_k(neg: i64) -> i32 {
    -(neg.unsigned_abs().min(i32::MAX as u64) as i64) as i32
}

/// The magnitude a negative bound value is respelled with (`-N`), or `None` when the value
/// is not negative or its magnitude is past the u32 the literal builder carries (package
/// constant respell, package type-parameter bound).
pub fn neg_bound_magnitude(v: i64) -> Option<u32> {
    if v < 0 {
        u32::try_from(v.unsigned_abs()).ok()
    } else {
        None
    }
}

/// The declared index of instance `k` of an instance array `[left:right]` (`instance_array.rs`).
pub fn instance_index(left: i64, right: i64, k: i64) -> i64 {
    if left >= right {
        left - k
    } else {
        left + k
    }
}

/// The `(hi, lo)` bits of chunk `k` (MSB-first, `p` bits each) of a net `[msb:lsb]` an
/// instance array slices across its instances (`instance_array.rs`).
pub fn instance_chunk(msb: i64, lsb: i64, k: i64, p: i64) -> (i64, i64) {
    if msb >= lsb {
        let hi = msb - k * p;
        (hi, hi - (p - 1))
    } else {
        // ascending [0:7]: the LEFT index is the MSB.
        let hi = msb + k * p;
        (hi, hi + (p - 1))
    }
}

/// The internal bit position (0 = LSB) of declared index `idx` in `[msb:lsb]` (a function
/// return's part-select, `inline_ret_select.rs`).
pub fn index_position(msb: i64, lsb: i64, idx: i64) -> i64 {
    if msb >= lsb {
        idx - lsb
    } else {
        lsb - idx
    }
}

/// md lane, a select's dimension direction (`packed_md.rs`): both bounds literal, or one
/// literal bound (`[N-1:0]` descending, `[0:N-1]` ascending), else unknown.
pub fn md_select_desc(m: Option<i64>, l: Option<i64>) -> Option<bool> {
    match (m, l) {
        (Some(m), Some(l)) => Some(m >= l),
        (None, Some(_)) => Some(true),
        (Some(_), None) => Some(false),
        (None, None) => None,
    }
}

/// md lane, the last index an indexed part-select `o +: w` / `o -: w` touches (`packed_md.rs`).
pub fn indexed_last(o: i64, w: i64, plus: bool) -> i64 {
    if plus {
        o + w - 1
    } else {
        o - w + 1
    }
}

/// An indexed part-select's element-index pair `(low, high)` in u64 (elaborate `packed.rs`
/// multi-dimensional packed lane, lens2 CD1): `[c+:w]` = `(c, c + w - 1)`, `[c-:w]` =
/// `(c + 1 - w, c)`, adding before subtracting; `None` for `[c-:w]` when `c + 1 < w` (the
/// low index would be below 0).
pub fn indexed_pair_u64(c: u64, w: u64, plus: bool) -> Option<(u64, u64)> {
    if plus {
        Some((c, c + w - 1))
    } else if c + 1 < w {
        None
    } else {
        Some((c + 1 - w, c))
    }
}

/// The half-open bit range `[low, end)` a select names over a u32 offset and width
/// (elaborate `read_set.rs` `const_bits`, lens2 CD1): `[o-:w]` = `[o + 1 - w, o + 1)`, every
/// other kind `[o, o + w)`; `None` on u32 overflow or underflow.
pub fn indexed_bits_u32(o: u32, w: u32, down: bool) -> Option<(u32, u32)> {
    if down {
        Some((o.checked_add(1)?.checked_sub(w)?, o.checked_add(1)?))
    } else {
        Some((o, o.checked_add(w)?))
    }
}

/// §20.7 range queries over one recorded dimension `[left:right]` (elaborate
/// `expr_special.rs` and `const_array.rs`; the md parser lane's are [`md_query`]).
pub fn dim_query(name: &str, left: i64, right: i64) -> Option<i64> {
    Some(match name {
        "$left" => left,
        "$right" => right,
        "$low" => left.min(right),
        "$high" => left.max(right),
        "$size" => (left - right).abs() + 1,
        "$increment" => {
            if left >= right {
                1
            } else {
                -1
            }
        }
        _ => return None,
    })
}

/// Product of recorded dimension sizes, each at least 1, saturating in u32 (packed width /
/// unpacked element count of nets, ports, frames, task locals, instances).
pub fn dims_width_sat<I: IntoIterator<Item = u32>>(sizes: I) -> u32 {
    sizes
        .into_iter()
        .fold(1u32, |a, w| a.saturating_mul(w.max(1)))
}

/// Product of recorded dimension sizes, saturating in u32, no floor (`array_formal.rs`).
pub fn dims_count_sat<I: IntoIterator<Item = u32>>(sizes: I) -> u32 {
    sizes.into_iter().fold(1u32, |a, s| a.saturating_mul(s))
}

/// Product of recorded dimension sizes in u64 (element widths, strides, element counts).
pub fn dims_product_u64<I: IntoIterator<Item = u64>>(sizes: I) -> u64 {
    sizes.into_iter().product()
}

/// Product of recorded dimension sizes in u64, saturating (array copy / continuous unroll).
pub fn dims_product_sat_u64<I: IntoIterator<Item = u64>>(sizes: I) -> u64 {
    sizes.into_iter().fold(1u64, |a, s| a.saturating_mul(s))
}

/// One step of a checked product over dimensions, u32 (struct member / cast widths, md
/// literal dims, a function return's extra packed dims).
pub fn dim_mul_u32(acc: u32, d: u32) -> Option<u32> {
    acc.checked_mul(d)
}

/// One step of a checked product over dimensions, u64.
pub fn dim_mul_u64(acc: u64, d: u64) -> Option<u64> {
    acc.checked_mul(d)
}

/// One step of a checked product over dimensions, usize (string-array init).
pub fn dim_mul_usize(acc: usize, d: usize) -> Option<usize> {
    acc.checked_mul(d)
}

/// A select span `[hi:lo]`'s element count, `hi - lo + 1` truncated to u32.
pub fn span_u32_wrap(lo: i64, hi: i64) -> u32 {
    (hi - lo + 1) as u32
}

/// A select span's element count, `hi - lo + 1`, `None` past u32.
pub fn span_u32_checked(lo: i64, hi: i64) -> Option<u32> {
    u32::try_from(hi - lo + 1).ok()
}

/// A select span's element count, `hi - lo + 1` with checked i64 steps, `None` past u32
/// (elaborate `const_select.rs` `const_param_select_shifted_bits`, lens2 CD1).
pub fn span_u32_checked_steps(lo: i64, hi: i64) -> Option<u32> {
    u32::try_from(hi.checked_sub(lo)?.checked_add(1)?).ok()
}

/// A select span's element count, `hi - lo + 1` as u64.
pub fn span_u64_wrap(lo: i64, hi: i64) -> u64 {
    (hi - lo + 1) as u64
}

/// A span's element count, `(hi - lo) as u64 + 1` (string-array pattern position).
pub fn span_u64_after(lo: i64, hi: i64) -> u64 {
    (hi - lo) as u64 + 1
}

/// A span's element count in i64, `hi - lo + 1` (an array's element bounds).
pub fn span_i64(lo: i64, hi: i64) -> i64 {
    hi - lo + 1
}

/// A select span over u32 bounds, `hi - lo + 1` (the caller ordered them).
pub fn span_u32(lo: u32, hi: u32) -> u32 {
    hi - lo + 1
}

/// A folded part-select's width `hi - lo + 1` over u32 bounds, `None` when `lo > hi` or the
/// width passes `max` (`packed.rs` `folded_part_width`).
pub fn span_folded(hi: u32, lo: u32, max: u64) -> Option<u32> {
    let w = (hi as u64).checked_sub(lo as u64)?.checked_add(1)?;
    (w <= max).then_some(w as u32)
}

// ─────────────────────────── 5. the parser's respell builders ───────────────────────────
// Every builder reads its literal operands through `lit`, the parser's evaluator.

/// A decimal integer-literal expression with the given value.
pub fn dec_lit(v: u32, span: Span) -> Expr {
    Expr {
        kind: ExprKind::IntLit {
            kind: IntLitKind::Decimal,
            raw: v.to_string(),
        },
        span,
    }
}

/// A non-negative literal or `None`.
pub fn lit_u32(lit: LitFold, e: &Expr) -> Option<u32> {
    lit(e).and_then(|v| u32::try_from(v).ok())
}

fn bin(op: BinOp, l: Expr, r: Expr, span: Span) -> Expr {
    Expr {
        kind: ExprKind::Binary {
            op,
            lhs: Box::new(l),
            rhs: Box::new(r),
        },
        span,
    }
}

/// `l op r` spanning both operands (the parser's `mk_bin`).
fn bin_spanning(op: BinOp, l: Expr, r: Expr) -> Expr {
    let span = l.span.to(r.span);
    bin(op, l, r, span)
}

/// `a - b`, folded when both are literals (and the result is non-negative); `a - 0` is `a`.
pub fn sub(lit: LitFold, a: Expr, b: Expr, span: Span) -> Expr {
    match (lit_u32(lit, &a), lit_u32(lit, &b)) {
        (Some(x), Some(y)) if x >= y => dec_lit(x - y, span),
        (_, Some(0)) => a,
        _ => bin(BinOp::Sub, a, b, span),
    }
}

/// `a + b`, folded when both are literals; `0 + b` is `b`, `a + 0` is `a`.
pub fn add(lit: LitFold, a: Expr, b: Expr, span: Span) -> Expr {
    match (lit_u32(lit, &a), lit_u32(lit, &b)) {
        (Some(x), Some(y)) => match x.checked_add(y) {
            Some(s) => dec_lit(s, span),
            None => bin(BinOp::Add, a, b, span),
        },
        (Some(0), _) => b,
        (_, Some(0)) => a,
        _ => bin(BinOp::Add, a, b, span),
    }
}

/// `a * b`, folded when both are literals; `a * 1` / `1 * b` collapse. A literal ZERO factor
/// is NOT folded away (`0 * x` is `x` in 4-state — the flat twin must keep that arithmetic).
pub fn mul(lit: LitFold, a: Expr, b: Expr, span: Span) -> Expr {
    match (lit_u32(lit, &a), lit_u32(lit, &b)) {
        (Some(x), Some(y)) => match x.checked_mul(y) {
            Some(p) => dec_lit(p, span),
            None => bin(BinOp::Mul, a, b, span),
        },
        (Some(1), _) => b,
        (_, Some(1)) => a,
        _ => bin(BinOp::Mul, a, b, span),
    }
}

/// md lane, direction: `(r.msb >= r.lsb) ? desc : asc`, or just one of them when the
/// dimension's direction is known from literal bounds.
pub fn md_by_dir(lit: LitFold, r: &Range, desc: Expr, asc: Expr, span: Span) -> Expr {
    match (lit_u32(lit, &r.msb), lit_u32(lit, &r.lsb)) {
        (Some(m), Some(l)) => {
            if m >= l {
                desc
            } else {
                asc
            }
        }
        _ => Expr {
            kind: ExprKind::Ternary {
                cond: Box::new(bin(BinOp::Ge, r.msb.clone(), r.lsb.clone(), span)),
                then_e: Box::new(desc),
                else_e: Box::new(asc),
            },
            span,
        },
    }
}

/// md lane, count: the element count of one packed dimension, `|msb - lsb| + 1`.
pub fn md_dim_width(lit: LitFold, r: &Range, span: Span) -> Expr {
    let desc = sub(lit, r.msb.clone(), r.lsb.clone(), span);
    let asc = sub(lit, r.lsb.clone(), r.msb.clone(), span);
    add(
        lit,
        md_by_dir(lit, r, desc, asc, span),
        dec_lit(1, span),
        span,
    )
}

/// md lane, width: the bit width of `dims[k..]` — the size of one element of `dims[k-1]`.
pub fn md_stride(lit: LitFold, dims: &[Range], k: usize, span: Span) -> Expr {
    let mut w = dec_lit(1, span);
    for r in &dims[k..] {
        w = mul(lit, w, md_dim_width(lit, r, span), span);
    }
    w
}

/// md lane, flat range: the `[total-1:0]` range a multi-dimensional packed parameter is
/// declared with.
pub fn md_flat_range(lit: LitFold, dims: &[Range], span: Span) -> Range {
    let total = md_stride(lit, dims, 0, span);
    Range {
        msb: sub(lit, total, dec_lit(1, span), span),
        lsb: dec_lit(0, span),
        span,
    }
}

/// md lane, offset: element offset (in elements from the lowest bits) of index `i` within
/// dimension `r` = `[left:right]`: the element at the RIGHT bound is the lowest bits either
/// way, so `i - right` descending and `right - i` ascending (`Range::lsb` is the right
/// bound, whichever is numerically smaller).
pub fn md_elem_off(lit: LitFold, r: &Range, i: &Expr, span: Span) -> Expr {
    let desc = sub(lit, i.clone(), r.lsb.clone(), span);
    let asc = sub(lit, r.lsb.clone(), i.clone(), span);
    md_by_dir(lit, r, desc, asc, span)
}

/// md lane, queries: `$size` / `$left` / `$right` / `$high` / `$low` / `$increment` of one
/// dimension of a multi-dimensional packed parameter; `None` for any other name.
pub fn md_query(lit: LitFold, name: &str, r: &Range, span: Span) -> Option<Expr> {
    Some(match name {
        "$size" => md_dim_width(lit, r, span),
        "$left" => r.msb.clone(),
        "$right" => r.lsb.clone(),
        "$high" => md_by_dir(lit, r, r.msb.clone(), r.lsb.clone(), span),
        "$low" => md_by_dir(lit, r, r.lsb.clone(), r.msb.clone(), span),
        "$increment" => {
            let neg = Expr {
                kind: ExprKind::Unary {
                    op: UnOp::Minus,
                    operand: Box::new(dec_lit(1, span)),
                },
                span,
            };
            md_by_dir(lit, r, dec_lit(1, span), neg, span)
        }
        _ => return None,
    })
}

/// SymTypedef lane: the width of a packed range whose bounds name overridable parameters, as
/// the §7.4.1 expression `(msb >= lsb) ? msb - lsb + 1 : lsb - msb + 1` (elaborate folds it
/// per instance); `[X-1:0]` folds to `X`. The caller decides the range is symbolic.
pub fn sym_typedef_width(lit: LitFold, r: &Range) -> Expr {
    let span = r.span;
    if lit_u32(lit, &r.lsb) == Some(0) {
        if let ExprKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } = &r.msb.kind
        {
            if lit_u32(lit, rhs) == Some(1) {
                return (**lhs).clone();
            }
        }
    }
    let up = add(
        lit,
        sub(lit, r.msb.clone(), r.lsb.clone(), span),
        dec_lit(1, span),
        span,
    );
    let down = add(
        lit,
        sub(lit, r.lsb.clone(), r.msb.clone(), span),
        dec_lit(1, span),
        span,
    );
    Expr {
        kind: ExprKind::Ternary {
            cond: Box::new(bin(BinOp::Ge, r.msb.clone(), r.lsb.clone(), span)),
            then_e: Box::new(up),
            else_e: Box::new(down),
        },
        span,
    }
}

/// SymMember lane: the symbolic width of a struct member `[E:0]` whose `E` names an
/// overridable parameter: IEEE §7.4.1 `|E - 0| + 1` as `(E < 0) ? 1 - E : E + 1` (`[W-1:0]`
/// with `W = 0` is `[-1:0]`, TWO bits). The comparison's sign follows `E`'s own type, as the
/// oracles evaluate the range's direction. The caller decides the member is symbolic.
pub fn sym_member_width(msb: &Expr) -> Expr {
    let sp = msb.span;
    let e = msb.clone();
    let neg = bin_spanning(BinOp::Lt, e.clone(), dec_lit(0, sp));
    let up = bin_spanning(BinOp::Add, e.clone(), dec_lit(1, sp));
    let down = bin_spanning(BinOp::Sub, dec_lit(1, sp), e);
    Expr {
        kind: ExprKind::Ternary {
            cond: Box::new(neg),
            then_e: Box::new(down),
            else_e: Box::new(up),
        },
        span: sp,
    }
}

/// Symbolic struct layout: the width a member that is neither folded nor symbolic is laid
/// out with after its error (`sym_struct.rs` `register_sym_struct`): one bit.
pub fn sym_member_default_width(span: Span) -> Expr {
    dec_lit(1, span)
}

/// Symbolic struct layout: `a + b + …` over member widths `parts` — the literals folded into
/// one leading literal ([`add`]), the empty sum `0`. A member's offset is the sum of the
/// widths below it, the struct's total the sum of all (`sym_struct.rs`, `struct_sel.rs`).
pub fn sym_sum_of(lit: LitFold, parts: &[Expr], span: Span) -> Expr {
    let mut acc_lit: u32 = 0;
    let mut syms: Vec<&Expr> = Vec::new();
    for p in parts {
        match lit_u32(lit, p) {
            Some(v) => acc_lit = acc_lit.saturating_add(v),
            None => syms.push(p),
        }
    }
    let mut acc = dec_lit(acc_lit, span);
    for s in syms {
        acc = add(lit, acc, s.clone(), span);
    }
    acc
}

/// Symbolic struct layout: `e - 1`, folded when `e` is a literal or ends in `+ <literal>` (so
/// a member width `E + 1` gives the bound `E` back) — the `[total-1:0]` msb, a member's high
/// bit.
pub fn sym_sub_one(lit: LitFold, e: Expr, span: Span) -> Expr {
    if let ExprKind::Binary {
        op: BinOp::Add,
        lhs,
        rhs,
    } = &e.kind
    {
        if let Some(k) = lit_u32(lit, rhs) {
            if k >= 1 {
                return add(lit, (**lhs).clone(), dec_lit(k - 1, span), span);
            }
        }
    }
    sub(lit, e, dec_lit(1, span), span)
}
