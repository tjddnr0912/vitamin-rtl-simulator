//! value/lanes -- a core asked in a named scope, spelled once (ROADMAP §5.2 VAL-2, owner 2026-11-09). See value/mod.rs's head
//! note for the lane table.

use crate::*;

impl Elaborator<'_> {
    /// REG (`fold_self_bits`) asked in the LIVE scope, self-determined: a name resolves through
    /// [`Self::wide_name_bits`] (the module / instance / package tables ; the `is_count` flag is
    /// not read here). The one spelling of the eleven adapters' first fold ; the reading after
    /// it (a truth, an unsigned or i64 value, a width, a `$clog2`, a real) stays in each adapter.
    #[inline]
    pub(crate) fn fold_self_wide(&self, e: &ast::Expr) -> Option<WideBits> {
        fold_self_bits(e, &|n, _| self.wide_name_bits(n))
    }
}
