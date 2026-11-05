(§2 timed-block-local-task · OPEN · in Scoping / imports / block-locals) (a block-local under a timing control in a task or function body is E3009):
* symptom: A block-local declared under a timing control in a task or function body is E3009 since §4.5.611 (row 72's re-plan, option (b)).
* evidence: Before, the declaration was dropped and its name bound the task's or the caller's same-named variable (T27: a module `integer i` loop driving a task whose `@(posedge clk) begin integer i; …` loops: `n=4 i=5`, iverilog, sv2v → iverilog and verilator `n=12 i=3`; T04 `x=3` for `x=7`, T07 `outer=000000aa`, T19 `mc=9` for `3`, T21 `a=0b` for `ab`, k10 `cnt=1`, k20 `t=23` for `t=6`) or was E3010.
* Refused also where that binding happened to give the right value — ruled (re-plan S0) as right only by accident, each one edit from a wrong value on PRE: T03, T05, T09, T11, T14, T15, T24, T30, k01, k02, k03, t_later_m, v6 (twins T04, T06, T09b, T12, T14b, T15b, T24b, t_later_mc, s6)
* ruled after round 5's differential review: t01, t02, o16, X8_formal_auto_emu, X8_sib_auto_emu, X8_sib_static_emu (twins t01w `outer=aa`, t02w `b=4`, o16w and o16v `i=2`, X8_formal_static_emu `s=1 proc i=3`, X8_sib_auto_w and X8_sib_static_w `s=5`; all three oracles `outer=11`, `b=0`, `i=7`, `s=3 proc i=50`, `s=1`)
* and from earlier rounds pk9, pk13, pk28, au_in, fs1.
* Prerequisite: §5.2 row 72b
* four review rounds on the name axis (r1 F5, r2 G1, r3 K1/K2, r4 M1/M3/N1; W/row72/review)
* oracles: 3 oracles
* status: OPEN (§5.2 row 72b)
