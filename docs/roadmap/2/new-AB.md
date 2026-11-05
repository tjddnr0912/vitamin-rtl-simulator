(§2 🆕 AB · BLOCKED · in Start-order rows) (residues of the time-0 hold of effectful and class-handle assigns):
* residue of §4.5.590's time-0 hold (`sim-engine/src/sched/t0_hold.rs`: an assign reaching an effectful callee or reading a class handle, and every assign reading its output, waits for the first t0 batch, then is released in dependency waves): (a) a held UNCERTIFIED assign (in `ca_always`: an impure callee such as `$random`, a class-handle read, a delayed or multi-driver assign) is re-run in every pass of every later wave, so a held chain of them makes extra t0 calls (f07, a 6-link `$random` chain: PRE 30, vita 42, both oracles 6, and the next printed `$random` moves with the draw count: PRE -887079274, vita -445445430, iverilog -1295874971) and repeats calls on settled values (x3 tristate 4, both oracles 1; d19 delay chain 8, iverilog 1, verilator 3; e01 5, both oracles 1)
* its cost is §5.b T0-HOLD-CHAIN
* after t0 such an assign still re-runs on every settle pass (the "Diagnostics" re-run line: n_ca_objf2_tbL_H, t2t_c_fg_this_H, u20_c_fnew_member_H ×3 at time 2, verilator ×2), which holds §3.b `unique-if-chain`'s subroutine-body residue
* (b) waves are ordered per net, so links sharing one vector or array net run as a cycle (d18, six generate instances through `o[k]`: 18 calls, both oracles 6; e02 `c[k+1] = f(c[k])`: 9, iverilog 10, verilator 13)
* (c) a held cycle is released whole with everything below it, so a downstream link runs on a held net's `z` (k01 `f6 t=0 v=z`; iverilog `x`, verilator `0`)
* (d) a declined callee's body reads are not in `ca_deps`' read set (`levelize/mod.rs`), so its wave can come before the held driver of a net that body reads (r2h2 `f2 t=0 x=1 w1=z`, then `w1=0`; both oracles once, on `0`; PRE ran it on x first)
* (e) a `$fatal` latched by an assign's call does not stop the rest of that t0 settle (k06; PRE the same)
* fix for (a): re-run a held uncertified assign only in its own wave when its read set is complete (no declined callee in its calls), per wave otherwise
* §4.5.590's round 4 re-ran every one only in its own wave (f07 12 calls, r_chain = PRE) and was reverted: through a declined callee's body read the value stayed stale past t0 (r4a `W2 t=1 w2=0`, r4c, r4d, r4g, g06 `ev u=0 t=1`: events at t ≥ 1 in neither oracle)
* pins `t0_call_cont_assign_hold.rs` (residue and race pins)
* oracles: 2 oracles
* status: BLOCKED ((d): a declined callee's body reads in `ca_deps`' read set)
