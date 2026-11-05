(§2 gen-param-later-walk · OPEN · in Scoping / imports / block-locals) (a later generate walk reads a parameter declared after its reference):
* A walk after Nets reads a generate-scope parameter declared after the reference at the previous walk's binding (`generate.rs` `(_, ModuleItem::Param)`: bound at its position in every `GenPhase` walk, never unbound), where IEEE 1800-2017 §26.3 and the Nets walk read the outer object: N2 `K=8 bits=4 w=15` (the Nets-walk width is right, the later read is not), B1 `bits=4 t=15 K=8`, Q65 `P=11 bits=3`, P1c `K=8 v=8` / `top.gb.u P=8`, W2 `K=10000000000000007`, Z0b `l2 X=2`, SN7 `P=9`
* iverilog `K=4 bits=4 w=15`, `bits=4 t=15 K=4`, `P=2 bits=3`, `K=4 v=4` / `P=4`, `K=05`, `l2 X=1`, `P=1`
* with no outer object the read is accepted where iverilog refuses (Z5a `K=8`, Z5b `P=8`, Z5c `v=8`, Z5d `w=8`; iverilog `Unable to bind`), and an instance override reads the later name and steers the child's arm (DO02 `@two top.g.u.a bits=8`, iverilog `@one top.g.u.b bits=4`)
* fix: restore each generate level's own parameter keys at its entry in every walk after Nets (module-region keys only where the pre-Nets entry was empty: V24s keeps an import), which also makes every walk decide as Nets does (🆕 V)
* a decision it moves can reach 🆕 AE's 0 (Z1g), so it obeys §5.2's 4-state rule
* must stay: Z5i, Z5j hierarchical reads (all three `K=5`)
* oracles: iverilog + hand-IEEE (sv2v and verilator bind scope-wide: "Oracle splits")
* status: OPEN
