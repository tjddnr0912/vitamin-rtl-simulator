(§2 Double call · OPEN · in Size cast / signedness) (a sized cast over a signed call calls it twice):
* `40'(sf(1) + 8'sd0)` calls `sf` twice
* `expr_size_ctx.rs` `lower_size_leaf` → `extend_signed_once`
* 2 oracles
* OPEN
