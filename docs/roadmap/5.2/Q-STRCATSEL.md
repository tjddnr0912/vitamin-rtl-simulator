(§5.2 Q-STRCATSEL · rank ① · source Q-NESTCAT r1 lens-diff PX2 (c/D_ar_sel_d)) (a select on a string concatenation reads the packed vector):
* slot: before row 74: Q-NESTCAT's prerequisite, W/qnestcat/replan4/PLAN4.txt
* a select on a string concatenation reads the packed vector: `k = {s, "x"}[0]` (s = "ab") prints `Dar 0` with W2004, verilator 5.052 `Dar 97`
* iverilog 13.0 and sv2v syntax error
* IEEE 1800-2017 §11.4.12.2, §6.16
* PRE 01cac4b3 = Q-NESTCAT r2
* on PRE 3e541d67 `k = {a, "x"}[0]` (a = "a") `0`, verilator `97` (W/qnestcat/replan4/cells/c K02)
* Q-NESTCAT, parked into row 74, waits on it — its zero grant and nests reach this select: K01 `k = {a, {0{8'h41}}}[0]` PRE E3009, Q-NESTCAT r2b `1`, verilator `97`
* CS10 `{{a}, "x"}[0]` PRE `0` (verilator `97`), CS11 `[1]` `0` (`120`), CS19 `{2{{a}}}[0]` `1` (`97`) (W/qnestcat/plan3)
