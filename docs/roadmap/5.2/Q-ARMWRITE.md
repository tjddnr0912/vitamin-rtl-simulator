(§5.2 Q-ARMWRITE · rank ① · source ENG-2 r4 design review F-ARM, r4 re-plan Q-ARMWRITE (ARMRD, ARMRD_h)) (fork arm admission tests touching the waited variable, not writing it):
* a frame task's fork arm admission asks whether an arm touches a variable the wait reads (`classify_one_arm`), not whether it may write it while the wait pends: a read-only arm, or an arm of a `join` that ends before the wait, cannot change it (DJ_hsel DRJ_hsel DRA_hsel DJ_h DR_h DJ_nc DR_nc DR_nc_d: right at PRE, iv = s2v `F t=1 l=1 \| W t=5 y=2 z=3` on DRJ_hsel), while a writer arm racing the wait can (FforkIf A2b N1_arm N1_sscanf: hand-IEEE / vl twin)
* fix: a writes-only arm walk with the join mode and reachability to the wait, system-task destinations included
* IEEE 1800-2017 §9.3.2
* note: prerequisite of Q-FRAMESTAR
