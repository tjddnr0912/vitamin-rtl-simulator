(§3.b deep · OPEN · in Nets / timing) (deep residues: time-0 races, `@(*)` wake, runtime `==?`, select forces):
* scope: t0 race, `@(*)` decl-init wake, runtime `==?`, modport directions, part-select force
* status: OPEN.
* symptom: Part-select force: a bit- or part-select, an array word (`force m[0]`) and a concatenation of nets (`force {y, z}` / `release {y, z}`) as a force or release target are E3009 (`force target must be a whole net/variable (a bit/part-select is not a legal force target)`), where iverilog, sv2v and verilator run them (Q-RELDELAY cells E18, E31, R31)
* note: the wording is wrong too: IEEE 1800-2017 §10.6.2 forbids a select of a variable or of a user-defined-nettype net, not a constant bit- or part-select of a vector net
