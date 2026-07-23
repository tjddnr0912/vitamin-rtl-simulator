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
