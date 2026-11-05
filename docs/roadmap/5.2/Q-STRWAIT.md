(§5.2 Q-STRWAIT · rank ① · source Q-NESTCAT r1 lens-diff PX3 (c2/X_wait, p/wait_plain)) (a `wait` on a string comparison never wakes):
* slot: before row 74: Q-NESTCAT's prerequisite, W/qnestcat/replan4/PLAN4.txt
* a `wait` on a string comparison never wakes: `fork begin wait (t == "z"); $display("X woke %0t", $time); end begin #2 t = "z"; end join` prints nothing and ends Quiescent at 2, iverilog 13.0 = sv2v `X woke 2` (verilator does not build it)
* no concatenation needed (p/wait_plain)
* IEEE 1800-2017 §9.4.3
* PRE 01cac4b3 = Q-NESTCAT r2
* on PRE 3e541d67 the same (W/qnestcat/replan4/cells/c wait_plain)
* Q-NESTCAT, parked into row 74, waits on it — its zero grant and nests reach this `wait`: K05 `wait ({t, {0{8'h41}}} == "z")` PRE E3009, Q-NESTCAT r2b never wakes (`K05 end 5`; no oracle builds K05), CS8 `wait ({{t}, "x"} == "zx")` PRE `CS8 end 5`, iverilog 13.0 `CS8 woke 2` then `CS8 end 5` (verilator does not build it), CS9 `{{t}} == "z"`, CS20 `{2{{t}}} == "zz"` the same (W/qnestcat/plan3)
