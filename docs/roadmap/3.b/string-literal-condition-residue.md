(§3.b string-literal-condition-residue · OPEN · in Constants / parameters) (a string parameter in a generate condition is refused):
* a string literal in a generate condition §4.5.568 cannot read (`cond_names.rs`)
* witness: corpus witness row-9 R30: an untyped header parameter holding a string (`parameter LfsrType = "GAL_XOR"`, `prim_lfsr`) is filed as `Other` by `cond_names.rs:120–129` `param_site`, so even `64'(LfsrType) == 64'("GAL_XOR")` is E3010 (`… it does not read LfsrType`; all three `A a=1 b=2`)
* the string kind from §5.2 row 48 at the position §4.5.568 kept (top level, declared once, every override channel)
* oracles: 3 oracles
* status: OPEN (§5.2 row 50)
