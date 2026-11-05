(§5.2 Q-STRREPEVAL · rank ① · source main D7 W/main_d7/2026-10-30/qn-reval/RESULT.txt) (a string replication evaluates its operands once per copy):
* slot: before row 74: Q-NESTCAT's prerequisite, W/qnestcat/replan4/PLAN4.txt
* a string replication evaluates its value list once per copy: `r = {2{f()}}` prints `F! F! R [aa]` (verilator 5.052 `F! R [aa]`) and `r = {0{f()}}` never calls `f` (`R []`, verilator `F! R []`)
* IEEE 1800-2017 §11.4.12.1 (ieee1800-2017.txt:12029–12034) "the operands shall be evaluated exactly once, even if the replication constant is zero"
* iverilog 13.0 and sv2v also evaluate n times (an oracle defect: plan3 FL7 iverilog `F! F!`)
* shared lowering `elaborate/src/expr_main.rs` (the string `Replicate` arm) and `strings.rs` (`string_concat_rhs_parts`)
* fix: evaluate the value list once into temps, then repeat
* PRE 3e541d67
* also `r = {2{a, f()}}` (f returns 8'h41) `F! F! L03 [aAaA] 4`, verilator `F! L03 [aAaA] 4` (iverilog 13.0 and sv2v abort: vthread.cc `depth<stack_str_.size()`)
* through a `let` (`let LF = f(); r = {2{a, LF}}`; `$display("[%s]", {2{a, LF}})`) two `F!`, verilator one
* `r = {0{a, f()}}` none, verilator one
* and a diagnostic per copy: `r = {2{a, nope_y}}` prints E3010 twice, each oracle once (W/qnestcat/replan4/cells/l, c)
* Q-NESTCAT, parked into row 74, waits on it: a nested replication runs the same lowering (`{2{{f()}}}` the string route `F! F! R [aa]`; Q-NESTCAT r2 q03 q09 u11b, plan3 FL7, p1/W12; r3 N1 `F! F! N1 [AaAa] 4`, verilator `F! N1 [AaAa] 4`), and excluding such nests (r3) broke pairs inside the row (§3.b string-door-pairs)
* source: Q-NESTCAT re-plan 3 (W/qnestcat/PLAN3.txt X4)
