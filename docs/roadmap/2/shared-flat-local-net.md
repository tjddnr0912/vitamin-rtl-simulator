(§2 shared-flat-local-net · OPEN · in Scoping / imports / block-locals) (two blocks' same-named locals share one flattened net):
* Two blocks' same-named locals sharing one flattened net: a `$strobe` / `$monitor` read of one shows the other block's later write (s01c `a=bb`, s02c an extra `m=02`; iverilog, sv2v → iverilog and verilator `a=aa`, `m=01`)
* refused since §4.5.611 where every block on the net is under a timing control (each was E3010 before)
* a pair with a plain block or a process's top block keeps the old behaviour (p23 `s=99`, p24 `w=7`, p25 `a=bb`; all three `s=7`, `w=99`, `a=aa`)
* 3 oracles
* OPEN
