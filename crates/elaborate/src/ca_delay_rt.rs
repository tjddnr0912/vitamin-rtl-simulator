//! S1: the RUNTIME (variable) structural-delay lane — `assign #(dv) y = a;`,
//! `wire #(dv) w = a;` and the gate primitives the parser desugars into the
//! same `ContinuousAssign` (IEEE 1364-2005 §6.1.3 / §7.14).
//!
//! `fold_ca_delay` answers `uniform = None` for a delay that is not an
//! elaboration constant, and every caller consumes `None` as a SILENT default:
//! no delay at all. Measured on both oracles, `reg [7:0] dv; assign #(dv) y =
//! a; initial dv = 8'd5;` with `a` rising at 1 raises `y` at 6 — iverilog 13.0
//! and verilator 5.052 both print `RISE=6`, vita printed `RISE=2`.
//!
//! The value cannot be known at elaboration, so this lane lowers the delay
//! VALUE expressions into the same expression arena the assign's rhs uses and
//! records their ids in the `ca_delay_exprs` sidecar. The engine evaluates them
//! at the SCHEDULING point — the moment the rhs changes — which is the only
//! point the standard names. The delay expression is NOT part of the assign's
//! sensitivity, so a later write to the delay variable alone re-schedules
//! nothing; `schedule_delayed_cas` gets that for free because it re-schedules
//! only when the evaluated rhs differs from the last one.
//!
//! The multipliers ride the sidecar because a continuous assign has no process:
//! the engine's `cur_time_mult`/`cur_prec_mult` are the multipliers of whatever
//! process last ran, and a delayed assign in a `1ns/1ns` module must not be
//! scaled by a `1ps/1ps` testbench's.

use super::*;

/// One runtime structural delay as the engine consumes it: `(rise_eid,
/// fall_eid, toff_eid, time_mult, prec_mult)`.
pub(crate) type CaDelayExprs = (u32, u32, Option<u32>, u64, u64);

/// Everything one structural delay contributes: the frozen `ContAssign.delay`
/// ticks, the constant `(rise, fall, turnoff)` sidecar triple, and the runtime
/// sidecar tuple. At most one of the last two is ever `Some` — a value that did
/// not fold kills the triple and is exactly what makes the tuple exist.
pub(crate) type FoldedCaDelay = (Option<u32>, Option<(u32, u32, u32)>, Option<CaDelayExprs>);

impl Elaborator<'_> {
    /// Does this structural delay need the RUNTIME lane?
    ///
    /// True when ANY value in the list fails to const-fold — not just the
    /// first. `values[0]` alone is the tempting predicate (it is the value the
    /// frozen `ContAssign.delay` carries, so it alone decides whether the
    /// assign reaches the delayed lane at all), and it is wrong: a spec whose
    /// rise folds and whose FALL does not still needs the runtime lane for the
    /// fall, and `fold_ca_delay`'s `rft` triple cannot carry it — it requires
    /// EVERY value to fold and drops to `None` otherwise, which silently
    /// collapses the fall onto the uniform rise. Measured on
    /// `int dv = 5; assign #(2, dv) y = a;` under `1ns/1ns`, a rising at 1 and
    /// falling at 6: both oracles hold `y` high at t=9 (fall = dv = 5, so it
    /// drops at 11); vita dropped it at 8, at exit 0. The fully constant
    /// `#(2, 5)` twin beside it was correct in both, which is what makes the
    /// cell the predicate's and not the machinery's.
    ///
    /// The same holds when the RISE is a constant ZERO and a later value is
    /// runtime (`#(0, dv)`, and `#(ZP, dv)` for a `parameter ZP = 0`): both
    /// oracles keep the fall, vita collapsed it. Such a spec goes on the
    /// runtime lane too, which puts its zero rise on the pre-existing
    /// zero-tick lag (ROADMAP §2's `#0` row, observable only through a
    /// same-time-step `#0` chain) and gets the fall right. That is not the
    /// zero-rise suppression `fold_ca_delay` applies: that rule is about a
    /// WHOLLY constant delay, where staying off the delayed lane is a free
    /// choice. Here it is not a choice — a runtime value can only be delivered
    /// by the delayed lane.
    ///
    /// `ca_delay_value` is the same `const_delay_ticks`-then-scope pair
    /// `fold_ca_delay` folds through, so "does not fold" means the same thing
    /// in both. A delay with an EMPTY value list, and no delay at all, are both
    /// `false`: there is nothing to evaluate.
    pub(crate) fn ca_delay_is_runtime(&self, delay: Option<&ast::Delay>) -> bool {
        delay.is_some_and(|d| {
            !d.values.is_empty() && d.values.iter().any(|e| self.ca_delay_value(e).is_none())
        })
    }

    /// Lower a runtime structural delay's values into the module's expression
    /// arena and return the engine sidecar tuple.
    ///
    /// The arity rule is `fold_ca_delay`'s, one level later: 1 value ⇒ the same
    /// id for rise and fall; 2 ⇒ distinct rise and fall with the turnoff left
    /// `None` so the ENGINE takes `min(rise, fall)` of the values it actually
    /// read (an elaborate-time `min` is not available — that is the whole point
    /// of this lane); 3 or more ⇒ the third is the turnoff, the rest ignored,
    /// exactly as the constant fold ignores them.
    ///
    /// ⚠️ Called ONLY under `ca_delay_is_runtime`. Lowering appends to the
    /// expression arena, so calling it for a delay that folds would move the
    /// golden IR of every design that has one.
    pub(crate) fn lower_ca_delay_exprs(&mut self, delay: &ast::Delay) -> CaDelayExprs {
        // `lower_expr` already picks the `typ` branch of a min:typ:max, which is
        // the branch `delay_ticks_in_scope` picks for the constant lane.
        let rise = self.lower_expr(&delay.values[0]);
        let fall = match delay.values.get(1) {
            Some(e) => self.lower_expr(e),
            None => rise,
        };
        let toff = delay.values.get(2).map(|e| self.lower_expr(e));
        (rise, fall, toff, self.cur_time_mult, self.cur_prec_mult)
    }

    /// Fold + runtime-lower a structural delay for one declaration, in the one
    /// order both callers need: the `(uniform, rft)` pair the frozen
    /// `ContAssign.delay` and the `ca_delays` sidecar take, plus the runtime
    /// tuple when there is one.
    ///
    /// The uniform delay becomes `Some(0)` on the runtime lane. That field is a
    /// ROUTING flag as well as a tick count — `Scheduler::delayed_ca_idx` and
    /// `delayed_sole` are built from `delay.is_some()` — so the assign has to
    /// carry one to reach `schedule_delayed_cas` at all, and `0` is the value
    /// that is never read: the sidecar answers before the uniform does.
    ///
    /// Both callers must use this rather than `fold_ca_delay` directly, so a
    /// net-declaration assignment (`wire #(dv) w = a;`) and the spelled
    /// `assign #(dv) w = a;` cannot answer differently — they are one construct
    /// (IEEE §6.1.3) and the gate primitives desugar to the second.
    pub(crate) fn fold_ca_delay_rt(&mut self, delay: Option<&ast::Delay>) -> FoldedCaDelay {
        let (uniform, rft) = self.fold_ca_delay(delay);
        if !self.ca_delay_is_runtime(delay) {
            return (uniform, rft, None);
        }
        // `ca_delay_is_runtime` is true only for `Some(d)` with a non-empty
        // value list, so this cannot panic.
        let d = delay.expect("runtime delay has a Delay node");
        let rt = self.lower_ca_delay_exprs(d);
        // `rft` is already `None` here BY CONSTRUCTION, not by assumption: it
        // is built by folding EVERY value (`folded?`), and this arm is reached
        // only when at least one of them did not fold. It is returned as-is
        // rather than forced, so the two tables have one producer each and a
        // future change to either is visible as a disagreement rather than
        // silently masked. The engine consults `ca_delay_exprs` FIRST in any
        // case.
        (Some(0), rft, Some(rt))
    }

    /// Take the runtime lane back off every assign that drives a net the engine
    /// RESOLVES, and restore its pre-slice `delay = None`.
    ///
    /// ⚠️ This is a coverage narrowing, not a semantics choice. A net driven by
    /// two or more whole-net continuous assigns is resolved by 4-state wire
    /// resolution (`sim_engine::multi_driver_groups`, mirrored by
    /// `check_whole_net_multidriver`), and BOTH spellings of that rule exclude a
    /// net with any DELAYED driver — such an overlap is `E3001` at elaborate.
    /// The `Some(0)` routing flag is a delay as far as those two predicates are
    /// concerned, so without this pass
    ///
    ///     int dv = 2;
    ///     assign #(dv) y = a;
    ///     assign #(dv) y = b;
    ///
    /// stops elaborating. Measured: iverilog prints `y=x` at t=6 and `y=1` at
    /// t=11 for exactly that design, and so did vita before this slice (through
    /// the zero-delay resolution) — so refusing it is correct-to-loud, a rung
    /// DOWN, whatever the delay is worth. Its constant twin (`assign #2 y = a;`
    /// twice) is E3001 today, which is the pre-existing row this defers to: the
    /// delayed lane cannot drive a resolved net at all, and teaching it to is
    /// that row's slice, not this one. Until then such an assign keeps its
    /// pre-slice behaviour verbatim — the delay is dropped, recorded as a
    /// residue.
    ///
    /// The eligibility test is `multi_driver_groups` transcribed, with one
    /// substitution: a runtime-lane assign counts as `delay == None`, which is
    /// what it WAS. So a net that is not resolved today — an overlap that is
    /// already E3001, a partial or array-element driver anywhere on it, a sole
    /// driver — is not touched, and keeps the runtime delay.
    ///
    /// The lowered delay expressions stay in the arena. They are unreachable
    /// once the sidecar entry is gone, which costs a few nodes in a design that
    /// has one of these; the alternative is deciding before lowering, and the
    /// driver count is not known until every module is elaborated, by which
    /// point the scope the expression must resolve in is gone.
    pub(crate) fn demote_runtime_delay_on_resolved_nets(&mut self) {
        if self.ca_delay_exprs.is_empty() {
            return;
        }
        let mut whole: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        let mut excluded: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
        for (ci, ca) in self.cont_assigns.iter().enumerate() {
            let ci = ci as u32;
            let undelayed_pre = ca.delay.is_none() || self.ca_delay_exprs.contains_key(&ci);
            let is_whole = undelayed_pre && ca.lhs.chunks.len() == 1 && {
                let c = &ca.lhs.chunks[0];
                c.word.is_none() && c.offset.is_none() && c.width.is_none()
            };
            if is_whole {
                whole.entry(ca.lhs.chunks[0].net).or_default().push(ci);
            } else {
                for c in &ca.lhs.chunks {
                    excluded.insert(c.net);
                }
            }
        }
        let mut demote: Vec<u32> = Vec::new();
        for (net, cis) in whole {
            if cis.len() < 2 || excluded.contains(&net) {
                continue;
            }
            demote.extend(
                cis.into_iter()
                    .filter(|ci| self.ca_delay_exprs.contains_key(ci)),
            );
        }
        for ci in demote {
            self.ca_delay_exprs.remove(&ci);
            self.cont_assigns[ci as usize].delay = None;
        }
    }
}
