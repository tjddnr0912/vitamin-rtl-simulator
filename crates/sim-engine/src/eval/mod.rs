//! 4-state expression evaluator. Evaluates a `sim_ir::Expr` (by ExprId into
//! `SimIr.exprs`) to a [`Value`], reading nets via [`NetReader`] and consts from
//! `SimIr.consts`. IEEE-1364 4-state semantics; Z is normalized to X in every
//! operator except `===`/`!==`.
//!
//! v1 simplifications (documented, IEEE-permitted):
//! - any X/Z in an arithmetic operand poisons the whole result to X;
//! - the integer arithmetic lane is 64-bit (wider vectors truncate the numeric
//!   result — bitwise/concat/select/shift remain full-width).

use sim_ir::{BinOp, ConstRepr, Expr, SelKind, SimIr, SysFuncId, UnOp};

use crate::value::{and_w, low_mask, not1, not_w, nwords, or_w, xnor_w, xor_w, Value};

// ---- split parts (mechanical refactor) ----
mod binops;
mod eval_core;
mod mw_tail;
mod sysfunc;
pub(crate) use eval_core::*;
pub(crate) use mw_tail::*;
pub(crate) use sysfunc::*;

/// WIDE-ARITH-CAP: width above which the super-linear arithmetic kernels
/// (`*` O(n²), restoring `/`·`%` O(bits·n), `**` square-multiply) poison to X
/// instead of running. A declaration-legal net is ≤ `MAX_NET_WIDTH` (2^20), but
/// a replication concat (`{16{a}}`) can push an *operand* far past it (16M bits →
/// 34 s mul / 163 s div). Mirrors `elaborate`'s `MAX_NET_WIDTH` (same value), so
/// any operand within the declarable width regime is always computed exactly.
/// `simulate` warns once (W-RUN-WIDE-ARITH) when such a node exists.
pub(crate) const WIDE_ARITH_CAP: u32 = 1 << 20;

/// A word-parallel 4-state primitive: `(av,au, bv,bu) -> (rv,ru)`, 64 bits/op.
type WordBinOp = fn(u64, u64, u64, u64) -> (u64, u64);

/// Which reduction `reduce_word` performs (the N-forms negate the result).
#[derive(Clone, Copy)]
pub(crate) enum RedKind {
    And,
    Or,
    Xor,
}

/// Evaluation context: the IR (consts/exprs), the net table, current time, and
/// the self-width side table that drives context-determined sizing.
pub struct EvalCtx<'a, N: NetReader> {
    pub ir: &'a SimIr,
    pub nets: &'a N,
    pub now: u64,
    pub wt: &'a crate::width::WidthTable,
    /// Time multiplier `M` of the process whose expression is being evaluated
    /// (`$time = now / M`, `$realtime = now / M` real). 1 ⇒ the 1ns/1ns base.
    pub time_mult: u64,
    /// v7 RNG state (`Cell`s — eval stays `&self`; every evaluation of
    /// `$random`/`$urandom` is a fresh draw, see `SimState::rng`).
    pub rng: &'a crate::state::RngCells,
    /// v7 runtime plusargs (CLI order — the $test/$value$plusargs search set).
    pub plusargs: &'a [String],
}

pub(crate) enum Tri {
    True,
    False,
    Unknown,
}

/// Evaluate each LHS chunk's bit-offset expression NOW, returning one offset per
/// chunk (0 for a whole-net `None` chunk). The `&mut self` write path has no
/// `EvalCtx`, so dynamic indices like `a[i]` are resolved here at the correct
/// sampling moment (statement time for blocking, SAMPLE time for NBA, settle
/// time for a cont-assign).
///
/// This is THE offset resolver: it lives here, generic over the reader, so the
/// engine (`Scheduler::resolve_lvalue_offsets`) and the tier-3 arena write funnel
/// resolve an index with the SAME rule. Two spellings of "what bit position does
/// this index name" would drift exactly where it hurts most — an X/Z index that
/// one side drops and the other writes is a silent wrong value.
pub(crate) fn resolve_offsets<N: NetReader>(
    ctx: &EvalCtx<N>,
    lhs: &sim_ir::Lvalue,
) -> crate::exec::Offsets {
    use crate::exec::Offsets;
    // ── v5 ⑤: single-chunk assoc-element lvalue → i64 key side-channel ──
    // (the SIGNED key domain cannot ride the u32 pairs). Concat/offset
    // shapes fall through to the pair path, where the dyn write funnel
    // degrades them loud+ignored (outside the MVP; ⑥ rejects them).
    if let [c] = lhs.chunks.as_slice() {
        if c.offset.is_none() && c.width.is_none() {
            if let Some(weid) = c.word {
                if ctx.nets.is_assoc_str(c.net) {
                    return Offsets::AssocStrKey(ctx.assoc_str_key(weid));
                }
                if ctx.nets.is_assoc(c.net) {
                    return Offsets::AssocKey(ctx.assoc_key(weid));
                }
            }
        }
    }
    let ev = |eid: u32| {
        let v = ctx.eval(eid);
        // Resolve a runtime select index to a bit-position offset. The valid
        // domain is the SIGNED i32 range: a small negative (`a[-1+:4]`) is a
        // legitimate underflow that partial-writes the in-range bits (P0-IPU),
        // and a small/large positive writes (or lands OOB-high and drops). An
        // index OUTSIDE that domain is dropped entirely (iverilog parity):
        //   - X/Z          → the bit position is UNKNOWN
        //   - huge positive / UNSIGNED > i31 / clean beyond-u32 / > 64-bit
        //                  → out of any net's range
        // `OOR_DROP` (2^30) sits far above any net width (≤2^20) so every
        // selected bit lands out of range for bit/part/indexed-part and
        // array-word chunks alike. Signed-aware: an unsigned 0xFFFFFFFF is the
        // huge 4294967295 (drop), NOT a wrapped −1 (which would partial-write).
        const OOR_DROP: u32 = 1 << 30;
        if v.has_xz() {
            return OOR_DROP;
        }
        match v.to_i128_signed() {
            Some(i) if (i32::MIN as i128..=i32::MAX as i128).contains(&i) => i as i32 as u32,
            _ => OOR_DROP,
        }
    };
    let pair = |c: &sim_ir::LvalChunk| {
        let off = c.offset.map(ev).unwrap_or(0);
        // `word` is an ExprId array index (`mem[k] = …`); resolve NOW.
        let word = c.word.map(ev).unwrap_or(0);
        (off, word)
    };
    // Inline the ≤2-chunk case (virtually all lvalues) — no allocation.
    if lhs.chunks.len() <= 2 {
        let mut buf = [(0u32, 0u32); 2];
        for (i, c) in lhs.chunks.iter().enumerate() {
            buf[i] = pair(c);
        }
        Offsets::Inline {
            buf,
            len: lhs.chunks.len() as u8,
        }
    } else {
        Offsets::Heap(lhs.chunks.iter().map(pair).collect())
    }
}
