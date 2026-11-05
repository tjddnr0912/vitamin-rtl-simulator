(§3.b pattern-slice-arith-bound · OPEN · in System tasks & file I/O) (an arithmetic constant bound on an unpacked-array slice is refused):
* symptom: an arithmetic constant bound on an unpacked-array slice (`v[A+1:A] = '{…}`, `v[A*2+1 -: W]`, `v[1+2:2]`, a genvar `g*4+3`) is E3009 (`is not a single constant`; `arrays/pattern.rs` `const_bound` reads one lowered constant)
* fold it as a part-select bound folds, binding each name in the lowering's order (`bare_ident_route`: an inline formal before a parameter)
* oracles: sv2v and verilator `00 00 00 00 a1 a0 00 00`
* status: OPEN
