(§5.2 Q-LETSTR · rank ① · source Q-NESTCAT r3 lens-sound F2 (L2c), plan3 NI7; W/qnestcat/replan4/cells/l) (a `let` with a string body is not typed as a string):
* slot: in row 74
* a `let` whose body is a string is not string-typed in elaborate: `expr_is_string_ast` (`elaborate/src/strings.rs:557`) answers false for a `let` name (no `let` arm; `lower_let_use`, `sva_decl.rs:38`, expands the body per use), so the use drops bytes in a concatenation or replication: `let LB = {a, "x"}; r = {"<", LB};` (a = "a") `[ax] 2`, verilator 5.052 `[<ax] 3`
* `let LS = fs(); r = {2{LS}};` (fs returns "q") `F! L02 [] 1`, verilator `F! L02 [qq] 2`
* `let LF = f(); r = {2{{LF}}};` (f returns "a") `F! NI7 [] 1`, verilator `F! NI7 [aa] 2`
* iverilog 13.0 "sorry: let declarations ... are not currently supported", sv2v parse error
* IEEE 1800-2017 §11.12
* PRE 3e541d67 = Q-NESTCAT r1–r3
* silent-wrong
* the `let` door of §3.b string-door-pairs (X04, X10)
