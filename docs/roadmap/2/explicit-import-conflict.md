(§2 explicit-import-conflict · OPEN · in Scoping / imports / block-locals) (two explicit imports of one type or function name take the second):
* Two explicit imports of one type or function name from two packages take the second in silence (D19 `import pa::t; import pb::t;` `d19 b=8`, D20 `d20 f=5`; iverilog `'t' has already been imported into this scope from package 'pa'.`, sv2v `import of pb::t conflicts with prior import of pa::t`; verilator the first, `b=4`, `f=3`)
* §4.5.591 refuses only a name that carries a value (`package.rs` `offers_value`)
* 2 oracles
* OPEN
