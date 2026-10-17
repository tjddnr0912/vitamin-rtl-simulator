//! ENG-1 heap twin: what a heap store did to a dynamic-storage handle's content —
//! nothing, a VALUE-only move, or an EVENT.
//!
//! The net stores answer this with `note_change`'s `event` flag
//! (`value::real_bits_change_is_event`). A `real d[]` keeps its elements in the heap,
//! where `note_dyn_change` stages a mark that the next sweep folds into the dirty
//! channel WITH the change sequence it took — so a store that only flipped the sign of
//! a zero in one element woke every `always_comb` reading `d` (R12: both tools never
//! re-run there; IEEE 754 equality says nothing changed value). The same rule here:
//! the mark is still staged — a `wait (expr)` over the element is re-checked when
//! the sweep runs (E51: verilator wakes, §9.4.3) — but with no change stamp, so no
//! level waiter takes it as its event.

use super::*;

/// The staged sequence of a VALUE-only heap mark. Real sequences start at 1
/// (`change_seq` is incremented before every use), so 0 is never a change stamp.
/// Both appliers — `SimState::mark_heap_dirty` and tier-3's `drain_heap_marks` —
/// ask [`heap_mark_is_event`], never this constant.
const HEAP_VALUE_ONLY: u64 = 0;

/// Does a drained heap mark carry an EVENT (a change stamp and an author tag), or
/// only a value change (dirty membership alone)? ONE spelling for both stores.
#[inline]
pub(crate) fn heap_mark_is_event(seq: u64) -> bool {
    seq != HEAP_VALUE_ONLY
}

/// What one heap store did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HeapMove {
    Unchanged,
    /// Only real elements changed, and each only the sign of a zero.
    ValueOnly,
    Event,
}

/// One element, `old` → `new`. A pair of REAL values (a `real d[]` element, the only
/// real element dynamic storage admits — a real queue / associative array is E3009)
/// whose bits differ only as `+0.0` / `-0.0` is a value move; any other difference is
/// an event. Keyed on the values being real, never on their width.
pub(crate) fn dyn_elem_move(old: &Value, new: &Value) -> HeapMove {
    if old == new {
        return HeapMove::Unchanged;
    }
    let plain_real = |v: &Value| v.is_real && v.unk.iter().all(|&u| u == 0);
    if plain_real(old)
        && plain_real(new)
        && !crate::value::real_bits_change_is_event(
            old.val.first().copied().unwrap_or(0),
            new.val.first().copied().unwrap_or(0),
        )
    {
        return HeapMove::ValueOnly;
    }
    HeapMove::Event
}

/// A whole slot, `old` → `new`, already known to differ (the caller asked its own
/// equality — strict `!=` or [`dyn_slot_eq`] — and that answer stays the caller's).
/// Two dynamic arrays of the same length whose differing elements are all value-only
/// moves are a value move; every other difference (a length, a kind, an integral or
/// string element) is an event.
pub(crate) fn dyn_changed_move(old: Option<&DynObj>, new: Option<&DynObj>) -> HeapMove {
    match (old, new) {
        (Some(DynObj::DynArray { elems: a }), Some(DynObj::DynArray { elems: b }))
            if a.len() == b.len() =>
        {
            let mut m = HeapMove::Unchanged;
            for (x, y) in a.iter().zip(b.iter()) {
                match dyn_elem_move(x, y) {
                    HeapMove::Event => return HeapMove::Event,
                    HeapMove::ValueOnly => m = HeapMove::ValueOnly,
                    HeapMove::Unchanged => {}
                }
            }
            // Equal element by element but different as slots: not ours to call a
            // value move.
            if m == HeapMove::Unchanged {
                HeapMove::Event
            } else {
                m
            }
        }
        _ => HeapMove::Event,
    }
}

impl SimState<'_> {
    /// Stage a heap mark for `m`: an event through [`SimState::note_dyn_change`]
    /// (stamped, author-tagged), a value move as a dirty mark alone, nothing for
    /// `Unchanged`. Gated by the same `dyn_wake_observable` as the event path.
    pub(crate) fn note_dyn_move(&self, net: u32, m: HeapMove) {
        match m {
            HeapMove::Unchanged => {}
            HeapMove::Event => self.note_dyn_change(net),
            HeapMove::ValueOnly => {
                if self.dyn_wake_observable(net) {
                    self.dyn_dirty
                        .borrow_mut()
                        .push((net, u32::MAX, HEAP_VALUE_ONLY));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real(bits: u64) -> Value {
        Value::from_f64(f64::from_bits(bits))
    }

    #[test]
    fn a_signed_zero_flip_is_a_value_move_and_nothing_else_is() {
        let (p, n) = (0u64, 1u64 << 63);
        assert_eq!(dyn_elem_move(&real(p), &real(n)), HeapMove::ValueOnly);
        assert_eq!(dyn_elem_move(&real(n), &real(p)), HeapMove::ValueOnly);
        assert_eq!(dyn_elem_move(&real(p), &real(p)), HeapMove::Unchanged);
        // 1.0 -> -1.0: a sign flip of a non-zero is an event.
        let one = 1.0f64.to_bits();
        assert_eq!(dyn_elem_move(&real(one), &real(one | n)), HeapMove::Event);
        // NaN payloads are an event.
        assert_eq!(
            dyn_elem_move(&real(0x7ff8_0000_0000_0000), &real(0x7ff8_0000_0000_0001)),
            HeapMove::Event
        );
        // A 64-bit INTEGRAL element whose sign bit flips over zero is an event.
        let mut a = Value::zeros(64, true);
        let mut b = Value::zeros(64, true);
        a.val[0] = 0;
        b.val[0] = n;
        assert_eq!(dyn_elem_move(&a, &b), HeapMove::Event);
    }

    #[test]
    fn a_slot_is_a_value_move_only_when_every_difference_is_one() {
        let (p, n) = (0u64, 1u64 << 63);
        let arr = |v: &[u64]| DynObj::DynArray {
            elems: v.iter().map(|&b| real(b)).collect(),
        };
        let one = 1.0f64.to_bits();
        assert_eq!(
            dyn_changed_move(Some(&arr(&[p, one])), Some(&arr(&[n, one]))),
            HeapMove::ValueOnly
        );
        assert_eq!(
            dyn_changed_move(Some(&arr(&[p, one])), Some(&arr(&[n, p]))),
            HeapMove::Event
        );
        assert_eq!(
            dyn_changed_move(Some(&arr(&[p])), Some(&arr(&[n, p]))),
            HeapMove::Event
        );
        assert_eq!(dyn_changed_move(None, Some(&arr(&[n]))), HeapMove::Event);
        assert!(heap_mark_is_event(1) && !heap_mark_is_event(HEAP_VALUE_ONLY));
    }
}
