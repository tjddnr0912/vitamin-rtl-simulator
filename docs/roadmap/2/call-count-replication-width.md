(§2 call-count-replication-width · BLOCKED · in Constant domain (i64)) (a replication counted by a constant call has no width):
* cause: A replication counted by a constant call has no width (`fold_count` has no call arm by design, §4.5.371 ⓸; `const_eval_u32` folds literals only), so a signed neighbour folds sign-extended where the count is not evaluated (Rfn_t_s `(C ? X : {f2(2){1'b0}}) == 8'hFC` `L=0`, `GI=else`, `vb=4`; iverilog, sv2v, verilator `L=1`, `then`, `vb=5`), a bound takes 🆕 AC's fallback (Rfn_a_s_rb `vb=1`, 3 oracles `vb=5`) and a value position is E3009 (Rfn_a_s_lp; 3 oracles `L=1`)
* §4.5.594's cut left it
* oracles: 3 oracles
* status: BLOCKED (a call arm in `fold_count`, §4.5.371 ⓸ · 🆕 AE for a call that reads a never-assigned variable · 🆕 AC's sinks)
