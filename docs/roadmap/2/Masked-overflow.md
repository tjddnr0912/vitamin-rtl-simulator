(§2 Masked overflow · OPEN · in Constant domain (i64)) (a 32-bit constant overflow is masked to the wrong value):
* `((0-1)*(0-1)-2)/2` at 32 bits is 0 (oracles `7fffffff`)
* `const_fn_width.rs` `checked_mul` → wrapping + mask
* 2 oracles
* OPEN
