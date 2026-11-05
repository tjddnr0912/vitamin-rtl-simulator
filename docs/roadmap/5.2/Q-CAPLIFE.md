(§5.2 Q-CAPLIFE · rank ①) (overlapping automatic task activations share one delayed-assign capture):
* symptom: overlapping activations of a `task automatic` share one static capture temp per `y = #d rhs` statement: the first activation lands the second's value (int `S 7 7`, iv = s2v = vl `S 5 7`; real `S 1.500000 1.500000`, vl `S 0.500000 1.500000`; string `S cd cd`, iv = vl `S ab cd`)
* IEEE 1800 §6.21, §9.4.5
* fix: a per-activation capture in an automatic body
* witness: a delayed nested string concatenation shows this row beside Q-NESTCAT: `rr2 = #d {{8'h41, x}}` in a `task automatic` called twice, overlapping, PRE 3e541d67 `[q] [q]`, Q-NESTCAT r1–r3 `[Aq] [Aq]`, verilator 5.052 `[Aq] [Ap]`
* witness: the direct `rr2 = #d x` PRE `[q] [q]`, iverilog 13.0 = sv2v = verilator `[q] [p]`
* witness: the int twin `o = #2 a + 1` vita `OI 21 21`, iverilog = sv2v = verilator `OI 11 21`
* note: Q-DLYCAT is parked behind this row
* source: ENG-1b r1 lens-diff N2 (CAP_task_auto_conc_int, _real, _str, CAP_task_auto_conc_direct_*); Q-NESTCAT r1 lens-sound F2 (b2/D3task), lens-diff PX4 (c2/O2_ovl_task, p/ovl_int); W/qnestcat/replan4/cells/c (D3task, D3taskv, ovl_int)
