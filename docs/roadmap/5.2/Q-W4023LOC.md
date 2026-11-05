(§5.2 Q-W4023LOC · rank — · source ENG-2 r2 D7 aside (P_readmem_assoc, P_readmem_assoc_local)) (a run-time W4023 loses its location in staged runs and under JIT):
* symptom: a run-time `VITA-W4023` loses its `t.sv:L:C:` prefix and its `[in top.t]` instance in the staged work-library flow and under JIT (one-shot native, vm and interp keep both)
* status: diagnostic text only, pre-existing (PRE identical)
* fix: carry the location the one-shot run reports through the `.velab` and the JIT's diagnostic path
