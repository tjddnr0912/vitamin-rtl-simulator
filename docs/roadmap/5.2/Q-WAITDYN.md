(§5.2 Q-WAITDYN · rank ① · source ENG-1a lens-diff r3 (G03b G03c G03d, W_none_*, W_monsame_*)) (a `wait` on a dynamic-array element is never re-checked):
* symptom: `wait (d[i] …)` on a dynamic-array element that no continuous reader observes is never re-checked, even for a real value change (`-1.0`): vita `A end`, iverilog and verilator `WT t=5`
* evidence: PRE = POST
* IEEE 1800 §9.4.3
* fix: a heap change re-checks every pending `wait` over that handle
