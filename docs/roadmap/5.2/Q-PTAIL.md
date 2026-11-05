(§5.2 Q-PTAIL · rank ① · source ENG-2 r5 lens-diff D-N5) (an automatic task whose body only calls a suspending task returns at once):
* symptom: an automatic task whose body is only a call to an automatic task that suspends returns at once: `task automatic t1(); t2(); endtask` with `t2` `#4 y = 7` (NT1_tail_delay) prints `R t=1 y=0 \| A end y=0` on every vita binary, where iv = s2v = vl `R t=5 y=7 \| A end y=7`
* witness: the same with `t2(); ;` (NT3), `begin t2(); end` (NT7), `if (..) t2();` (NT8), an output formal (NT9 `y=x`), `t2` suspending on `@(*)`, `@(a)`, `#` or `wait` (NF1-NF6 NF8), N32_ff_two_procs, and B7_ff_callee_ign (PRE = POST `A end y=x`, iv = s2v `W t=5 y=4`)
* pins: it waits when the body holds another statement (NT2 NT4 NT6 NF7, B7t_callee_ign_nontail) or the caller is static (NT5)
* status: pre-existing (PRE = POST, native vm interp)
* IEEE 1800-2017 §13.3 :14237-14241 (control returns to the enabling process after the task, and every task it enabled, has completed)
