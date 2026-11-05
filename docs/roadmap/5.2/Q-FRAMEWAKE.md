(§5.2 Q-FRAMEWAKE · rank ① F · source ENG-2 r4 re-plan (E/PLAN-r4 Q-FRAMEWAKE), r4 lens-sound N-QFW, r4 lens-diff N2) (a write of a frame task's own variable wakes no in-frame level wait):
* symptom: a write of a frame task's own variable wakes no in-frame level wait (the engine's level wait wakes on module-net writes only), so a frame task whose `@(*)` / `wait` reads a variable a fork arm writes is refused (`VITA-E3009`) when the wait's set holds it
* witness: when it reaches the wait only through a nested task call's argument or a hierarchical operand the task runs instead: Fhtask_forkarm warns `VITA-W3056` and never wakes (`A end q=x`, iv = s2v `q=7`), A2b panics `run.rs:1644` (hand `y=1`), FforkIf never wakes and does not warn (`A end y=x`, hand `y=4`; iv crashes, s2v and vl refuse)
* scope: sim-engine
* note: prerequisite of Q-FRAMESTAR
