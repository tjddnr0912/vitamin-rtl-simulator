(§5.2 Q-RELEVAL · rank ① · source Q-RELDELAY design review (R30)) (a `release` re-evaluates an uncertified continuous assign's right-hand side):
* symptom: a `release` re-evaluates an uncertified continuous assignment's right-hand side (the unconditional re-dirty): `assign y = f(a);` with a `$display` in `f`, force at 5, release at 7 prints `F t=7 x=0001` on vita, none on iverilog, sv2v or verilator (R30)
* cause: a release of an unforced target re-dirties too (`cont_assign_call_deps.rs:368`)
* status: pre-existing
* note: with Q-RELIMM's register design the re-dirty is not needed
