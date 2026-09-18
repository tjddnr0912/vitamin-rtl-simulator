//! The REASON vocabulary behind every `wprog::compile` decline, and the
//! per-expression tally run.json publishes as its `wprog` object.
//!
//! ## What this is, and what it is not
//!
//! `codegen` in run.json counts process BODIES: whether the tier-2 compiler
//! could emit code for a body at all. A body can report `able 1/1` while every
//! evaluation of its right-hand sides runs the generic tree walk, because the
//! COMPILED lane is entered per EXPRESSION by [`super::compile_why`] — and until
//! this module existed that function returned a bare `None` at ~45 sites and
//! nothing counted them. This is the reporting table that closes that gap: the
//! boundary between the compiled lane and the generic walk, named.
//!
//! ⚠️ A reason is a REPORTING table only. It never panics, never changes a
//! value, and never changes which lane runs: `compile_why` returns `Err(reason)`
//! exactly where `compile` returned `None`, in the same order, having emitted
//! the same ops. The vocabulary is CLOSED and the keys are STABLE — run.json
//! consumers may pin them — so a new decline site names an existing key or adds
//! one to [`ALL`].
//!
//! ## The unit
//!
//! DISTINCT expression ids. An id is `asked` once however many contexts or call
//! sites ask about it, and a declined id is filed under the reason of its FIRST
//! decline. So `declined` is the sum of `reasons` and never exceeds `asked`.
//! It is NOT an evaluation count (a program compiles once per cache slot and
//! then runs for the rest of the simulation) and NOT a partition by site
//! (several sites share one key).

use std::collections::BTreeMap;

/// The root width gate, and every other width that must fit one 64-bit word:
/// a self-determined narrower node whose own width is 0 (a wider one files
/// `truncation`, since every context width here is already within `1..=64`),
/// the insurance width/sign check on a one-bit operator, and an operand /
/// condition / base / index / offset width outside `1..=64`.
pub const WIDTH: &str = "width";
/// A value wider than the context it lands in. `compile` emits no truncation
/// anywhere, so a self-determined node wider than `w` — and a sign seal whose
/// operand is wider than `w` — declines instead.
pub const TRUNCATION: &str = "truncation";
/// The uniform-sign gate on a node that is neither `Const` nor `Signal`, and a
/// read alias whose source net carries the other signedness.
pub const SIGN: &str = "sign";
/// An `Expr` variant this module has no arm for (`ArrayItem`, and the string /
/// heap / dynamic shapes). A `Call` or `SysFunc` never lands here — [`no_arm`]
/// files those under their own keys from BOTH places a no-arm node can decline
/// (the `CtxClass::Unknown` width branch and the final catch-all).
pub const NODE_KIND: &str = "node_kind";
/// A user function called inside an expression (`Expr::Call`).
pub const CALL: &str = "call";
/// A system function other than the admitted one-argument `$signed`/`$unsigned`
/// seal.
pub const SYSFUNC: &str = "sysfunc";
/// An operator with no compile arm: a `Unary` that is not `!`, `~` or a
/// reduction; a `Binary` with no arm (`Mul`, `Div`, `Mod`, `Pow`, `BitXnor`,
/// `AShl`, `CasezEq`, `CasexEq`); and a SIGNED `>>>`, whose sign fill is the one
/// shift whose bits differ from the logical one.
pub const OPERATOR: &str = "operator";
/// A literal outside the one-word numeric domain: a string, a real, or a
/// numeric wider than 64 bits.
pub const CONST_DOMAIN: &str = "const_domain";
/// A net whose kind is not `Wire`/`Reg`/`Logic`/`Integer`.
pub const NET_KIND: &str = "net_kind";
/// A net whose arena slot is not exactly `w` bits in one word.
pub const NET_WIDTH: &str = "net_width";
/// A frame-local net: its value lives in the activation window, not the slot
/// this arm resolves at compile time.
pub const FRAME_NET: &str = "frame_net";
/// A class-handle net, declined whole (a field read has to reach `class_heap`).
pub const CLASS_HANDLE: &str = "class_handle";
/// A whole unpacked-array read — no index expression while the slot holds more
/// than one element.
pub const ARRAY_WHOLE: &str = "array_whole";
/// A CONSTANT array index carrying an x or z plane bit.
pub const INDEX_UNKNOWN: &str = "index_unknown";
/// A CONSTANT array index at or beyond the slot's element count.
pub const INDEX_RANGE: &str = "index_range";
/// A `LoadIdx` inside the right operand of `&&`/`||` or inside a ternary branch:
/// the generic path may never evaluate it, and an out-of-range element read
/// there COUNTS a diagnostic. A report appearing is a divergence exactly as much
/// as one going missing.
pub const LAZY_INDEX: &str = "lazy_index";
/// A shift amount that is not a 2-state constant.
pub const SHIFT_AMOUNT: &str = "shift_amount";
/// A part-select whose offset is not a constant (`x[i +: 4]`).
pub const SELECT_OFFSET: &str = "select_offset";
/// A CONSTANT part-select offset that is x/z, negative or outside the u64→i64
/// lane, whose folded width disagrees with the width table, or whose range
/// leaves the base.
pub const SELECT_RANGE: &str = "select_range";
/// A concatenation whose parts do not tile the result: a zero-width part,
/// `Σ pw != w`, or a replication with `n == 0`, a zero-width operand, or
/// `n × operand width != w`.
pub const CONCAT_WIDTH: &str = "concat_width";
/// A replication count that does not fold to a constant.
pub const REPLICATE_COUNT: &str = "replicate_count";
/// A defensive out-of-range table id (`ir.exprs`, `ir.nets`, `arena.slots`).
/// Unreachable from the CLI — the ids come from the frozen IR — and kept only so
/// that no decline in this module is anonymous.
pub const MALFORMED: &str = "malformed";

/// The reason for a node this module has no arm for — ONE classification for
/// the two places such a node can decline (the width branch's
/// `CtxClass::Unknown` and the final catch-all), so a `Call` files `call` and a
/// `SysFunc` files `sysfunc` whatever the context width. `None` (an id outside
/// `ir.exprs`) is `malformed`.
pub fn no_arm(e: Option<&sim_ir::Expr>) -> &'static str {
    match e {
        Some(sim_ir::Expr::Call { .. }) => CALL,
        Some(sim_ir::Expr::SysFunc { .. }) => SYSFUNC,
        Some(_) => NODE_KIND,
        None => MALFORMED,
    }
}

/// Every key, in byte-lexicographic order. The vocabulary is CLOSED: run.json's
/// `wprog.reasons` can only carry keys from this slice, and a consumer may pin
/// it.
pub const ALL: &[&str] = &[
    ARRAY_WHOLE,
    CALL,
    CLASS_HANDLE,
    CONCAT_WIDTH,
    CONST_DOMAIN,
    FRAME_NET,
    INDEX_RANGE,
    INDEX_UNKNOWN,
    LAZY_INDEX,
    MALFORMED,
    NET_KIND,
    NET_WIDTH,
    NODE_KIND,
    OPERATOR,
    REPLICATE_COUNT,
    SELECT_OFFSET,
    SELECT_RANGE,
    SHIFT_AMOUNT,
    SIGN,
    SYSFUNC,
    TRUNCATION,
    WIDTH,
];

/// The tally behind run.json's `wprog` object. Unit: DISTINCT expression ids
/// `compile` was asked about; a declined id is filed under the reason of its
/// FIRST decline, and asked once however many contexts or sites ask.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WprogDeclines {
    /// Distinct expression ids `compile` was asked about.
    pub asked: u64,
    /// How many of those declined. Equals `reasons.values().sum()`, and is never
    /// greater than `asked`.
    pub declined: u64,
    /// Declines per reason — keys from [`ALL`] only. `BTreeMap` so the
    /// serialized key order is stable across runs and platforms.
    pub reasons: BTreeMap<&'static str, u64>,
}

/// The accumulator the native kernel carries for one run.
///
/// Two bitsets rather than a set: one slot per ExprId, the same direct-indexed
/// shape `wcache`/`icache` use, so noting an ask is a bounds check and a store.
#[derive(Debug, Default)]
pub(crate) struct WprogWhy {
    asked: Vec<bool>,
    declined: Vec<bool>,
    reasons: BTreeMap<&'static str, u64>,
}

impl WprogWhy {
    /// One slot per expression id.
    pub(crate) fn new(exprs: usize) -> Self {
        WprogWhy {
            asked: vec![false; exprs],
            declined: vec![false; exprs],
            reasons: BTreeMap::new(),
        }
    }

    /// `compile` was asked about `eid`. Idempotent — the unit is the id, not the
    /// ask, so a cache miss re-asked under a second context adds nothing.
    pub(crate) fn ask(&mut self, eid: u32) {
        if let Some(b) = self.asked.get_mut(eid as usize) {
            *b = true;
        }
    }

    /// `compile` declined `eid` for `why`. Only the FIRST decline of an id bumps
    /// the map, which is what keeps `declined == Σ reasons`: a second context
    /// can decline the same id for a different reason, and counting both would
    /// make the two numbers disagree with no way for a reader to tell which is
    /// the id count.
    pub(crate) fn decline(&mut self, eid: u32, why: &'static str) {
        match self.declined.get_mut(eid as usize) {
            Some(b) if !*b => {
                *b = true;
                *self.reasons.entry(why).or_insert(0) += 1;
            }
            _ => {}
        }
    }

    /// Popcount the two bitsets and move the map out.
    pub(crate) fn finish(self) -> WprogDeclines {
        WprogDeclines {
            asked: self.asked.iter().filter(|b| **b).count() as u64,
            declined: self.declined.iter().filter(|b| **b).count() as u64,
            reasons: self.reasons,
        }
    }
}
