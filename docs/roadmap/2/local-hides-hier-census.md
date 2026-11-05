(§2 local-hides-hier-census · OPEN · in Scoping / imports / block-locals) (a block-local named like a module variable hides it from the width census):
* A block-local named like a module variable makes `expr_size_hier`'s census decline the name, and a parent's `16'(u.hs * q8)` takes the 32-bit fallback (hs1_ctl, hs2_ctl, hs3: `0000xx20`, all three `00000120`)
* a local under a timing control is not counted since §4.5.611 (hs2 stays `00000120`)
* 3 oracles
* OPEN
