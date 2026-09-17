//! The per-transition delay of a delayed continuous assign — split out of
//! `scan_arm` (module-size policy). Three lanes answer one question, so they
//! live together.

use super::*;

impl Scheduler<'_, '_> {
    /// How long after `now` this cont-assign's new value lands, or `None` when
    /// it never lands at all.
    ///
    /// Three lanes, consulted in this order:
    ///
    ///   1. `ca_delay_exprs` — S1's RUNTIME lane, for a delay that is not an
    ///      elaboration constant. Its values are EVALUATED HERE, at the
    ///      scheduling point: the caller reaches this line only when the rhs
    ///      was found to have changed, which is exactly the moment IEEE 1364
    ///      §6.1.3 names. The delay expression is therefore not part of the
    ///      assign's sensitivity, and a write to the delay variable alone moves
    ///      nothing. It is consulted FIRST because on this lane the uniform
    ///      `ContAssign.delay` is a `Some(0)` routing flag, not an answer.
    ///   2. `ca_delays` — constant rise/fall/turnoff, when they are not all
    ///      equal.
    ///   3. the uniform `d` from the frozen `ContAssign.delay`.
    ///
    /// `nets` is the tier-3 seam, threaded from the caller so a delay value is
    /// read from the SAME store the rhs was read from.
    ///
    /// `None` is `delay_ticks_of`'s `u64::MAX` never-fires sentinel (a negative
    /// delay amount, integral or real) returned as a state rather than as a
    /// number: `u64::MAX` is not a tick, and enqueuing it would hand
    /// `next_delayed_ca` a time the run loop then advances to. Measured, both
    /// oracles: `int dn = -1; assign #(dn) y = a;` never lets `y` follow `a`.
    pub(crate) fn effective_ca_delay<N: crate::eval::NetReader + ?Sized>(
        &self,
        nets: Option<&N>,
        ci: usize,
        d: u32,
        old: Option<&Value>,
        new: &Value,
    ) -> Option<u64> {
        let eff = match self.st.ca_delay_exprs.get(&(ci as u32)).copied() {
            Some((r_eid, f_eid, t_eid, mult, pmult)) => {
                let rise = self.ca_delay_ticks(nets, r_eid, mult, pmult);
                let fall = self.ca_delay_ticks(nets, f_eid, mult, pmult);
                // No third value ⇒ `min(rise, fall)` of the values just read —
                // the same default `fold_ca_delay` applies to a constant
                // 2-value spec, one level later because the `min` is of
                // runtime values.
                let toff = match t_eid {
                    Some(e) => self.ca_delay_ticks(nets, e, mult, pmult),
                    None => rise.min(fall),
                };
                transition_delay(old, new, rise, fall, toff)
            }
            // When this cont-assign has differing rise/fall/turnoff delays the
            // net updates atomically at `now + max(per-changed-bit destination
            // delay)`. Absent a sidecar entry the uniform `d` is used —
            // byte-identical to the behaviour before either sidecar existed.
            None => match self.st.ca_delays.get(&(ci as u32)) {
                Some(&(rise, fall, toff)) => {
                    transition_delay(old, new, rise as u64, fall as u64, toff as u64)
                }
                None => d as u64,
            },
        };
        (eff != u64::MAX).then_some(eff)
    }

    /// One RUNTIME structural-delay value → global-precision ticks, read
    /// through the same store the cont-assign's rhs was read through.
    ///
    /// The RULE is `eval::delay_ticks_of`, shared with `Terminator::Delay` (real
    /// → two-stage round, any X/Z → 0, negative → `u64::MAX` = never fires), so
    /// a `#(dx)` on a continuous assign and a procedural `#dx` cannot answer
    /// differently. `mult`/`prec_mult` come from the SIDECAR, not from
    /// `st.cur_time_mult`: a continuous assign has no process, so the ambient
    /// multiplier is whichever process last ran.
    fn ca_delay_ticks<N: crate::eval::NetReader + ?Sized>(
        &self,
        nets: Option<&N>,
        eid: u32,
        mult: u64,
        prec_mult: u64,
    ) -> u64 {
        let v = match nets {
            Some(r) => self.st.mk_eval_ctx_with(r).eval(eid),
            None => self.st.mk_eval_ctx().eval(eid),
        };
        crate::eval::delay_ticks_of(&v, mult, prec_mult)
    }
}
