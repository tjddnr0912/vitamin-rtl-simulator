//! Assertion control over a module-item deferred assertion is refused (E3009).
//!
//! A module-item deferred assertion (`[L :] assert #0 (c) …;`, IEEE 1800-2017 §16.4)
//! is desugared by the parser onto the `always_comb` it is treated as. vita's global
//! assertion control (`$assertoff`, `$asserton`, `$assertkill`) reaches only the
//! concurrent checkers' reports, not an immediate or deferred assertion's pass or
//! fail action, so such an item would keep reporting while assertions are off. Until
//! the whole assertion branch is gated on the runtime enable (ROADMAP §5.2), a design
//! that holds both an assertion-control call and a module-item deferred assertion is
//! refused here, after the whole design is elaborated — every compilation unit,
//! whether the units came from one invocation or from a work library.

use super::*;

/// The assertion-control system tasks of IEEE 1800-2017 §20.11 and §20.12.
const ASSERT_CONTROL: &[&str] = &[
    "$assertoff",
    "$asserton",
    "$assertkill",
    "$assertcontrol",
    "$assertfailoff",
    "$assertfailon",
    "$assertpassoff",
    "$assertpasson",
    "$assertnonvacuouson",
    "$assertvacuousoff",
];

impl Elaborator<'_> {
    /// A user `always_comb` whose body starts where the block does is the parser's
    /// desugar of a module-item deferred assertion: a written `always_comb` starts
    /// at its keyword, before its body.
    pub(crate) fn note_deferred_assert_item(&mut self, p: &ast::ProceduralBlock) {
        if p.kind == ast::ProcKind::AlwaysComb && p.span.lo == p.body.span().lo {
            self.deferred_assert_items.push(p.span);
        }
    }

    /// An assertion-control call reached while lowering a statement.
    pub(crate) fn note_assert_control_call(&mut self, name: &ast::Ident, span: ast::Span) {
        if ASSERT_CONTROL.contains(&name.name.as_str()) {
            self.assert_control_calls.push((name.name.clone(), span));
        }
    }

    /// After the whole design is elaborated: refuse each module-item deferred
    /// assertion when the design also calls an assertion-control task.
    pub(crate) fn finish_assert_item_gate(&mut self) {
        let items = std::mem::take(&mut self.deferred_assert_items);
        let calls = std::mem::take(&mut self.assert_control_calls);
        let Some((call, call_span)) = calls.first().cloned() else {
            return;
        };
        let mut seen = std::collections::BTreeSet::new();
        for item in items {
            if !seen.insert((item.lo, item.hi)) {
                continue;
            }
            self.error_at(
                MsgCode::ElabUnsupported,
                item,
                &format!(
                    "assertion control over a module-item deferred assertion is not \
                     supported yet: the design calls `{call}`, which this deferred \
                     assertion item would not follow"
                ),
            );
            self.note_at(
                MsgCode::ElabUnsupported,
                call_span,
                &format!("the `{call}` call"),
            );
        }
    }
}
