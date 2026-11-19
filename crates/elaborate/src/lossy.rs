//! §5.2 Q-LOSSYSPELL: the design-wide record of which lossy constant producers a design
//! SPELLS, complete at the head of `Elaborator::run` and nowhere earlier.
//!
//! The parser notes one [`ast::LossySpell`] per parsed unit (`hdl-parser` `lossy.rs`,
//! pushed as `TopItem::LossySpell` only when a bit is set). The design is more than one
//! unit's tokens: a library compose (`velab -L`, `staged.rs`) merges the items of every
//! LOADED unit, and `-G` values are in no source at all. So the design's record is the
//! OR of every unit's item in the unit `run` receives, then `param_xz` for a `-G` value
//! whose text spells an unknown digit — read by [`Elaborator::cli_value_spells_unknown`],
//! through the same raw → value fns `cli_overrides_for` reads.
//!
//! WHAT A BIT DOES NOT CLAIM (the parked class: a reader taking a value filled for another
//! purpose): a bit is "this spelling is in the text elaborate was given, or in a `-G`
//! value" ; it is not "a constant is wrong", not "reaches a read", not per module or
//! instance, and `false` is not "every constant is right" (an x made by an operator, a
//! `'0` / `'1` fill, a constant function's local, the Q-AESINK mark are in none of them).
//! Staged: a unit `velab -L` does not reach from `--top` is never decoded, so its bits are
//! absent there and present in the one-shot run, which elaborates every parsed unit
//! (W/lossy/premise/cells/b6_staged ; RULINGS-plan R9).
//!
//! WHO READS IT: nobody in production while nothing reads it (home rule H4,
//! `crates/elaborate/tests/lossy_home.rs` ; the first reader edits that rule). It is
//! handed to `Sidecars::lossy_spell`, not serialized, for tests.

use crate::{ast, Elaborator};

/// The design's record: the OR of EVERY `TopItem::LossySpell` in `unit` (never the first
/// one: a library compose carries one per loaded unit that spells one), then `param_xz`
/// for any `-G` value in `top_params` that spells an unknown. A `-G` naming no parameter,
/// a localparam or a value `cli_overrides_for` refuses still counts: the text decides.
pub(crate) fn design_lossy_spell(
    unit: &ast::SourceUnit,
    top_params: &[(String, String)],
) -> ast::LossySpell {
    let mut acc = ast::LossySpell::default();
    for it in &unit.items {
        if let ast::TopItem::LossySpell(ls) = it {
            acc.param_xz |= ls.param_xz;
            acc.unsized_xz |= ls.unsized_xz;
            acc.sign_kw_param |= ls.sign_kw_param;
            acc.let_any |= ls.let_any;
            acc.fill_xz |= ls.fill_xz;
        }
    }
    acc.param_xz |= top_params
        .iter()
        .any(|(_, raw)| Elaborator::cli_value_spells_unknown(raw));
    acc
}
