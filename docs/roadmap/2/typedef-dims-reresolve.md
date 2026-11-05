(§2 typedef-dims-reresolve · OPEN · in Scoping / imports / block-locals) (a typedef's named dimensions re-resolve at each use):
* A typedef's named dims re-resolve at each use: module, `$unit`, generate and routine scopes → §5.2 row 68, which narrows the line (x.sh.gen `v=3 b=2`, all three `f/4`; x.sh.genst3, x.cat.c564ret / c570bits / c570mem, rv.gen_*, rv.unit_*, rv.coll_src, rv.gencond_shadow, r2.ds_untyped; r2.ds_shl once row 23 has added `<<`)
* a name an enclosing scope binds (r2.ds_encl, r3.encl_gfor, r3.encl_import; u01c, a `$unit` typedef over an imported constant under a module's own `D`: PRE `bits=2`, iverilog and sv2v → iverilog `7`) → §5.2 row 71
* packages → §5.2 row 46 (row-9 R11: `prim_mubi_pkg`'s `typedef enum logic [MuBi4Width-1:0] mubi4_t` as a function formal folds at `t.$func$mp::inv`, where `MuBi4Width` is unbound: E3009 `undefined name MuBi4Width is not allowed in a constant range bound`; iverilog and sv2v → iverilog `A o=0` / `A o=1`; `lc_ctrl_pkg`'s `TxWidth`; first the local alternative, a package typedef's bound folded at its declaration where the package binds every name in it)
* what stays after those rows: a bound the parser's table cannot fold (a constant-function call, `$bits(T)`, a `$unit` constant inside a module, r3.encl_unit) under a later shadow
* 2 oracles
* status: OPEN (§5.2 rows 68, 71, 46)
