//! The `stmt_effect` family members whose body is SHARED between the two
//! `Kernel` implementors (A1-ii, ROADMAP §5.1-f).
//!
//! ## Why these live here and not on `Scheduler`
//!
//! Every member of this family does the same three things: read one or two
//! operands that are NOT the enclosing assignment's rhs, compute, and write a
//! REF ARGUMENT (a seed variable, a `$cast` destination, an iteration key) from
//! inside the call rather than through the statement's own lvalue. The write was
//! never the problem — it already went out through `Kernel::k_write_lvalue`, so
//! it lands in whichever store the calling kernel owns. The READS were: they
//! called `Scheduler::eval` / `SimState::read_net`, which are hard-wired to the
//! engine's nets, the one store a native run never writes.
//!
//! Moving the bodies here and taking `&mut impl Kernel` fixes that without a
//! second spelling: the engine's own seams are the same functions it called
//! before (`k_eval` → `Scheduler::eval`, `k_write_lvalue` → its write funnel), so
//! the engine path is mechanically byte-identical, and tier-3 gets the arena
//! everywhere by construction rather than by review.
//!
//! ⚠️ The alternative — decomposing each into a pure part plus a `(lvalue, value)`
//! the caller applies, as `exec::plusargs::effect` does — does not scale past one
//! member: `$dist_*` reads a VARIABLE number of parameters, so the "pure part"
//! would have to return a `Vec` and the shape argument stops paying for itself.

use sim_ir::Lvalue;

use crate::exec::Kernel;
use crate::value::Value;

/// The whole-net lvalue for a REF ARGUMENT — a `$random` seed, a `$cast`
/// destination, an assoc iteration key. Every one of them is elaborate's
/// whole-net `Signal` contract, so the chunk is offset-free and width-free and
/// the write funnel derives the destination width itself.
fn whole_net_lvalue(net: u32) -> Lvalue {
    Lvalue {
        chunks: vec![sim_ir::LvalChunk {
            net,
            word: None,
            offset: None,
            width: None,
            kind: sim_ir::SelKind::Bit,
        }],
    }
}

/// The net id behind a whole-net `Signal` ExprId, or `None` for any other shape.
/// Every caller here is defending a HAND-BUILT IR — the `kpred` probe already
/// matched the id, and elaborate only ever emits `Signal { word: None }` in these
/// argument positions — so `None` degrades rather than panics.
fn whole_net_of<K: Kernel + ?Sized>(k: &K, eid: u32) -> Option<u32> {
    match k.k_ir().exprs.get(eid as usize) {
        Some(sim_ir::Expr::Signal { net, word: None }) => Some(*net),
        _ => None,
    }
}

/// Read a 32-bit seed. X/Z reads as 0, then the IEEE 1364-2005 Annex-N
/// zero-substitution applies — the same answer an uninitialized iverilog `reg`
/// gives.
fn seed_in<K: Kernel + ?Sized>(k: &K, seed_eid: u32) -> u32 {
    let cur = k.k_eval(seed_eid);
    if cur.has_xz() {
        0
    } else {
        (cur.to_u64().unwrap_or(0) & 0xffff_ffff) as u32
    }
}

/// Write the advanced seed back through the kernel's own funnel — a resize to
/// the variable's width, exactly like any blocking assign.
fn seed_out<K: Kernel + ?Sized>(k: &mut K, net: u32, s: u32) {
    let lv = whole_net_lvalue(net);
    let sv = Value::from_i128(s as i32 as i128, 32, true);
    let off = k.k_resolve_lvalue_offsets(&lv);
    k.k_write_lvalue(&lv, sv, &off);
}

/// `r = $random(seed)` — advance the Annex-N LCG in `seed` and return the draw.
pub(crate) fn random_seeded<K: Kernel + ?Sized>(k: &mut K, rhs: u32) -> Value {
    let seed_eid = match k.k_ir().exprs.get(rhs as usize) {
        Some(sim_ir::Expr::SysFunc { args, .. }) => args.first().copied(),
        _ => None,
    };
    let Some(seed_eid) = seed_eid else {
        return Value::xs(32, true);
    };
    let Some(net) = whole_net_of(k, seed_eid) else {
        return Value::xs(32, true);
    };
    let mut s = seed_in(k, seed_eid);
    let r = crate::rng::annex_n_random(&mut s);
    seed_out(k, net, s);
    Value::from_i128(r as i128, 32, true)
}

/// `r = $dist_*(seed, params…)` — same seed contract, a distribution kernel in
/// place of the raw draw. Parameters are `integer` (signed 32-bit); X/Z reads 0.
pub(crate) fn dist_seeded<K: Kernel + ?Sized>(k: &mut K, rhs: u32) -> Value {
    let (which, seed_eid, params) = match k.k_ir().exprs.get(rhs as usize) {
        Some(sim_ir::Expr::SysFunc { which, args }) if !args.is_empty() => {
            (*which, args[0], args[1..].to_vec())
        }
        _ => return Value::xs(32, true),
    };
    let Some(net) = whole_net_of(k, seed_eid) else {
        return Value::xs(32, true);
    };
    let p: Vec<i32> = params
        .iter()
        .map(|&a| k.k_eval(a).to_u64().unwrap_or(0) as u32 as i32)
        .collect();
    // ⚠️ ORDER: the parameters are evaluated BEFORE the seed is read, which is
    // the engine's order and is observable — a parameter expression may itself
    // be impure (`$dist_uniform(s, 0, $random)`).
    let mut s = seed_in(k, seed_eid);
    let p0 = *p.first().unwrap_or(&0);
    let p1 = *p.get(1).unwrap_or(&0);
    use sim_ir::SysFuncId as F;
    let r = match which {
        F::DistUniform => crate::rng::dist_uniform(&mut s, p0, p1),
        F::DistNormal => crate::rng::dist_normal(&mut s, p0, p1),
        F::DistExponential => crate::rng::dist_exponential(&mut s, p0),
        F::DistPoisson => crate::rng::dist_poisson(&mut s, p0),
        F::DistChiSquare => crate::rng::dist_chi_square(&mut s, p0),
        F::DistT => crate::rng::dist_t(&mut s, p0),
        F::DistErlang => crate::rng::dist_erlang(&mut s, p0, p1),
        // Unreachable behind `kpred::dist_seeded_rhs`; a hand-built IR gets 0
        // rather than a panic, matching every other defensive path here.
        _ => 0,
    };
    seed_out(k, net, s);
    Value::from_i128(r as i128, 32, true)
}

/// The FUNCTION form `ok = $cast(dst, src)` — write the context-sized `src` into
/// the `dst` ref arg and return 1.
///
/// iverilog 13.0 does not support `$cast` (no oracle): hand-IEEE §6.24.2 — an
/// integral assignment always succeeds in this class-free subset, so the status
/// is always 1. A failure status of 0 would need class / strict-enum range
/// checks vita does not model.
pub(crate) fn cast<K: Kernel + ?Sized>(k: &mut K, rhs: u32) -> Value {
    let fail = Value::from_i128(0, 32, true);
    let (dst_eid, src_arg) = match k.k_ir().exprs.get(rhs as usize) {
        Some(sim_ir::Expr::SysFunc { args, .. }) if args.len() == 2 => (args[0], args[1]),
        _ => return fail,
    };
    let Some(net) = whole_net_of(k, dst_eid) else {
        return fail;
    };
    let lv = whole_net_lvalue(net);
    // context-size `src` to the dst width, then write through the funnel.
    let v = k.k_eval_for_lvalue(&lv, src_arg);
    let off = k.k_resolve_lvalue_offsets(&lv);
    k.k_write_lvalue(&lv, v, &off);
    Value::from_i128(1, 32, true)
}

/// `st = aa.first(k)` / `.next` / `.last` / `.prev` — position the iteration key
/// and return the status.
///
/// The locate half stays on `SimState` (`assoc_iter_compute`): it walks
/// `dyn_heap`, which is ONE object both backends share. What this function owns
/// is the store-DEPENDENT part — reading the CURRENT key for `next`/`prev`
/// through this kernel's store, and writing the new key back through its funnel
/// (dirty channel included, so `@(k)` sensitivity and VCD see it like any
/// blocking assign).
pub(crate) fn assoc_iter<K: Kernel + ?Sized>(k: &mut K, lhs: &Lvalue, rhs: u32) -> Value {
    let cur = k.k_assoc_iter_cur_key(rhs).map(|eid| k.k_eval(eid));
    let (key_write, status) = k.k_assoc_iter_compute(rhs, cur);
    if let Some((knet, kval)) = key_write {
        let klv = whole_net_lvalue(knet);
        let off = k.k_resolve_lvalue_offsets(&klv);
        k.k_write_lvalue(&klv, kval, &off);
    }
    // Context-size the int status exactly as `k_queue_pop` sizes its result
    // (self-width of the rhs = 32 signed via the width table).
    let mut v = Value::zeros(32, true);
    v.val[0] = (status as u32) as u64;
    let lw = k.k_lvalue_width(lhs);
    let (sw, ssigned) = k.k_self_width(rhs);
    v.resize_keep_sign(lw.max(sw), ssigned)
}
