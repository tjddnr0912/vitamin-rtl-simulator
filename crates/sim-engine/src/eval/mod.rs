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
pub(crate) mod binops;
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
/// `?Sized` is NOT load-bearing today — measured, the workspace compiles without
/// it, because nothing yet instantiates `EvalCtx<dyn NetReader>`. It is here for
/// S1d-4b-2, whose `Kernel::k_nets()` hands back a `&dyn NetReader`. Recorded
/// rather than left to look exercised.
pub struct EvalCtx<'a, N: NetReader + ?Sized> {
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

/// Scale an already-evaluated `#delay` amount into global-precision ticks:
/// real → two-stage round (to the module's own precision `P = M/S`, then by `S`);
/// any X/Z → 0 (iverilog parity); integral → `v × M`, u64-saturating. NEGATIVE in
/// either domain → `u64::MAX`, i.e. never fires, matching iverilog; zero (and a
/// value that ROUNDS to zero, including `-0.0`) → 0, the `#0` delta delay.
///
/// Shared for the same reason `resolve_offsets` is, and the S1d-4a gate proved
/// the need rather than assuming it: this rule was first RESTATED on the tier-3
/// side and lost two of its four clauses (the X/Z guard and the `u64::MAX`
/// sentinel), so a delay the engine treats as unbounded fired immediately. Both
/// were invisible to a value comparison — a delay amount is not a stored bit.
pub(crate) fn delay_ticks_of(v: &crate::value::Value, mult: u64, prec_mult: u64) -> u64 {
    let mult = mult.max(1);
    if v.is_real {
        let s_mult = prec_mult.max(1);
        let p_mult = (mult / s_mult).max(1);
        let x = v.to_f64().unwrap_or(0.0) * p_mult as f64;
        // ROUND FIRST, then test the sign — measured, not assumed. A negative
        // real fired IMMEDIATELY while a negative integer never fired
        // (`to_u64()` on a negative yields `None` → `u64::MAX`); iverilog fires
        // neither, so one function was giving two answers. But testing the sign
        // of the RAW product overshoots: a tiny negative like `#(-1e-9)` rounds
        // to zero and iverilog DOES fire it at 0. So the boundary is the ROUNDED
        // value, and `-0.0` (which `round` yields for any `x` in `(-0.5, 0]`)
        // compares `>= 0` and fires — the `#0` delta delay. The first fix here
        // was right for `-1.0` and wrong for `-1e-9`; the three-way sweep caught
        // it.
        let r = x.round();
        if r < 0.0 {
            return u64::MAX;
        }
        return (r as u64).saturating_mul(s_mult);
    }
    if v.has_xz() {
        return 0;
    }
    v.to_u64().unwrap_or(u64::MAX).saturating_mul(mult)
}

/// The bit position a resolved index VALUE names — the second half of
/// `resolve_offsets`'s `ev`, split out so a specialized evaluator can reuse the
/// rule instead of restating it. `resolve_offsets` itself is the only reason
/// this is a separate function: the rule (X/Z drops, the signed i32 domain, the
/// `OOR_DROP` sentinel) is exactly the kind that must have one spelling, and
/// the tier-3 kernel now reaches it from a second evaluator.
pub(crate) fn offset_of_index_value(v: &crate::value::Value) -> u32 {
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
    //
    // ⚠️ The "(iverilog parity)" above is TRUE for x/z and for the negative
    // and huge-but-i32 cases, and FALSE beyond i32 — measured, S2 slice 3:
    // `mem[64'h1_0000_0000]` on a `[0:3]` array is DROPPED here and TRUNCATED
    // to `mem[0]` by iverilog 13 (its index lane is 32 bits). IEEE 1364 §5.2.1
    // says an out-of-range index is ignored on a write and x on a read, with
    // no truncation step, so this is a vita-ahead divergence rather than a gap
    // — pinned by a vita-internal anchor because the oracle is the wrong one
    // here (`s2_xz_index_is_dropped_matching_the_oracle`, row F). Independently
    // re-measured by review: iverilog truncates a beyond-i32 ARRAY-WORD index
    // (`mem[64'h1_0000_0002]` lands on `mem[2]`) but does NOT truncate a
    // beyond-i32 BIT offset, so "its index lane is 32 bits" scopes to the array
    // word.
    //
    // The `has_xz` early return is a FAST PATH, not a semantic guard:
    // `to_i128_signed` goes through `to_u64`/`to_u128`, which already return
    // `None` for any x/z bit, so the `_ => OOR_DROP` arm answers identically.
    // Measured as an equivalent mutation rather than assumed.
    const OOR_DROP: u32 = 1 << 30;
    if v.has_xz() {
        return OOR_DROP;
    }
    match v.to_i128_signed() {
        Some(i) if (i32::MIN as i128..=i32::MAX as i128).contains(&i) => i as i32 as u32,
        _ => OOR_DROP,
    }
}

/// Evaluate each LHS chunk's bit-offset expression NOW, returning one offset per
/// chunk (0 for a whole-net `None` chunk). The `&mut self` write path has no
/// `EvalCtx`, so dynamic indices like `a[i]` are resolved here at the correct
/// sampling moment (statement time for blocking, SAMPLE time for NBA, settle
/// time for a cont-assign).
///
/// This WAS the only offset resolver. Since S2 slice 3 the tier-3 kernel has a
/// specialized one (`NativeKernel::fast_offsets`) that answers the shapes it
/// admits — measured at 2156 of 2168 corpus write sites — and falls back here
/// for the rest. What the two still share is the per-index RULE
/// (`offset_of_index_value`, just above), which is the part that must not
/// drift; this function remains generic over the reader so the engine
/// (`Scheduler::resolve_lvalue_offsets`) and the tier-3 fallback resolve an
/// index the same way. Two spellings of "what bit position does
/// this index name" would drift exactly where it hurts most — an X/Z index that
/// one side drops and the other writes is a silent wrong value.
pub(crate) fn resolve_offsets<N: NetReader + ?Sized>(
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
    let ev = |eid: u32| offset_of_index_value(&ctx.eval(eid));
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
