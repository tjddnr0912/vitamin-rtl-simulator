(§5.2 Q-HIERFORCE · rank ① · source Q-RELDELAY plan (E11 E29)) (`force` / `release` on a net through a hierarchical name is refused):
* slot: after 75
* symptom: `force` / `release` through a hierarchical name of a NET is E3018 `procedural hierarchical write to net` (`hier_defer/write.rs:298`) though IEEE 1800-2017 §10.6.2 allows a force on a net: `force u.o` (E11), `force g.y` (E29)
* oracles: iverilog = sv2v = verilator run them
* status: over-refusal, loud
