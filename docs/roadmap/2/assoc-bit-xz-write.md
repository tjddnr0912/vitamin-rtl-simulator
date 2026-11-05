(§2 assoc-bit-xz-write · OPEN · in Ranges / bounds / selects) (an x/z write to a 2-state associative element keeps the x/z):
* An assignment of an x / z value to an element of an associative array with a 2-state element type keeps the x / z (`bit [7:0] mem [int]; mem[0] = 8'bx;` reads `xx`, kw_bit_elem_x_assign; `changes.rs` `assoc_write` does not apply the `two_state` coercion the flat store does)
* IEEE 1800-2017 §6.11.3 reads 0
* verilator refuses the cell (`Unsupported LHS tristate construct: ASSOCSEL`)
* `$readmem*` into such an array coerces since §4.5.617
* hand-IEEE
* OPEN
