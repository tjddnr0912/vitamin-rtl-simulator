(§0-C B) (a shared crate for literal parsing):
* scope: shared literal-parsing crate — medium-large
* note: payoff small (one two-predicate hazard)
* note: trap: `literal.rs` needs `sim_ir` (layering inversion), so split digit→bits from `ConstVal` packing.
