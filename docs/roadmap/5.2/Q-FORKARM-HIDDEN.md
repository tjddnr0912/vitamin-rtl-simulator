(§5.2 Q-FORKARM-HIDDEN · rank ① · source ENG-2 r4 re-plan step 3) (a fork arm reading a parent variable only through a hierarchical operand panics):
* slot: after 75
* symptom: a Case-A fork arm in a frame task that reads a parent variable only through a deferred hierarchical operand (`u.m[l]` in a `$display`, a branch, a delay, a system task) panics `frame_eval.rs:361` on every backend (S1br S1dl S1rd S1rd_none S1sys S1wr; iv = s2v = vl agree on S1rd S1wr S1br S1dl, the local twins run right)
* status: loud, pre-existing (PRE = POST)
* fix: re-decide the arm admission after the resolve passes
* note: prerequisite of Q-FRAMESTAR
