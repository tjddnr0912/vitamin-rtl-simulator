(§3.b unique-pkg-closure · OPEN · in Diagnostics quality) (a package call reaching a `unique` violation is refused as impure):
* the package-scoped call closure walk `elaborate/src/package.rs` `pkg_stmt_pure_orig` (fn at :144, catch-all `_ => false` at :183) treats the synthesized `$__vita_unique_violation` as impure: a `pk::f(…)` call whose callee reaches a lone `unique if` or a `unique case` with no `default` is refused E3009 `package-scoped call pk::f(...) reaches pk::g, whose body names something outside its own formals/locals …`, naming a statement the user did not write, where both oracles run (q6t_pkg_scoped_const_chain_H, q6t_pkg_scoped_const_chain_case: iverilog `Time: 0`, `Time: 2`; verilator `[0] … pk.g`)
* `import pk::*` and a bare call run (q6w_pkg_import_const_chain_H: W4031 t0 ×2, t2)
* evidence: PRE the same
* note: keeps `unique-if-chain`'s package bodies unarmed
* oracles: 2 oracles
* status: OPEN (§5.2 Q-PKGUNIQ)
