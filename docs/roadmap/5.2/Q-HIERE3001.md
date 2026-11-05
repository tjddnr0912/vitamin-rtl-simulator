(§5.2 Q-HIERE3001 · rank ① · source ENG-2 design review (N01, N01c, N31)) (a hierarchical call over an initialized variable is a false multi-driver error):
* slot: after 75
* symptom: a hierarchical function call or task enable in an `always_comb` whose actual reads a declaration-initialized variable (`logic a = 0;` … `y = u.f(a)`, `u.f(a ^ 1'b0)`, `u.tk(a ^ 1'b0);`) is a false `VITA-E3001` multiple-driver error on a legal design (loud) where iverilog runs it (`C t=0 \| C t=5 y=1 \| C t=8`)
* status: pre-existing, not ENG-2's root (PRE = POST)
* IEEE 1800 §9.2.2.2
* fix: census the multidriver scan's view of the deferred call's copy-in first
