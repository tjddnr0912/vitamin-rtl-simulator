(§2 V1b · OPEN · in Ranges / bounds / selects) (select widths and counts the constant domain declines are refused):
* a part-select width, indexed width or replication count whose bound the constant domain declines and whose lowered tree holds more than constants, `+`, `-` and `$clog2` is E3009 (§4.5.601): arithmetic over a select of a >64-bit parameter (VeeR EL2's `f[pt.HI-1:0]`, `{(WP[7:0]+1){1'b1}}`), a select of a package constant in a package function (`x[Q16[7:0]+1:0]`), `*` over a late name (`f[gb.L*2-1:0]`, `f[$bits(u.X)*1 - 1 : 0]`), a cast or a concatenation, and the same shapes in uncalled or dead code (§4.5.601 review: diff d02–d93, g01–g16, k01–k08; sound b1, b2, b6; verilator and sv2v fold them)
* the lowered tree is bound by the lowering's own resolver, so the fix widens `sim_ir::selfwidth::edge_value`'s allow-list (each arm proven by its run-time differential test, `edge_value_differential.rs`), not the declaring-scope fold
* re-entry: measure that the lowered leaves are the declaring scope's where §2 🆕 AD/AF/AG bind routine text at the caller, and keep the non-zero-LSB parameter refusal
* 2–3 oracles
* status: OPEN (corpus witness: EL2, row-9 V1; §5.2 row 51)
