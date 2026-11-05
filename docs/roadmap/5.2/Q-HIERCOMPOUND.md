(§5.2 Q-HIERCOMPOUND · rank ① · source ENG-2 r3 re-plan OP-a (HCOMP_star, LCOMP_star)) (a compound assign to a hierarchical lvalue in `always @*` reads its own copy):
* slot: after 75
* symptom: an `always @*` compound assignment to a hierarchical lvalue (`always @* u.v[0] \|= m;`) reads the lvalue's implicit copy: the compound marker decides by net identity at lowering, which the deferred placeholder lacks
* witness: PRE = POST = sv2v = verilator `A end v=01`, iverilog and the local twin (`v[0] \|= m`) `v=00`
* status: pre-existing
* IEEE 1800-2017 §9.4.2.2 (the identifiers in the statement; an lvalue-only identifier is not read)
* fix: mark the deferred right-hand-side placeholder whose path equals the left-hand side's (the R-SITE funnel then carries the mark)
