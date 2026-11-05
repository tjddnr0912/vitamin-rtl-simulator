(§5.2 Q-HANDLERET · rank ① · source Q-NESTCAT r1 lens-sound b9/HBret, b6/H9alias; r2b lc sb3–sb6) (two routes into a class handle skip the handle check):
* slot: before row 74: Q-NESTCAT's prerequisite, W/qnestcat/replan4/PLAN4.txt
* two routes into a class handle skip the N7 handle check (`check_handle_assign`, `elaborate/src/class_lower.rs:65`, called for the generic `=` and `<=` only, `stmt_main.rs:284` / `:401`): a function return — `function automatic C f(); return {{8'h41, a}}; endfunction` then `h = f();` runs and `h == null` is 0, a handle made from string bytes, where `h = {{8'h41, a}}` is E3009 "cannot assign a non-handle value to a class handle" (iverilog 13.0 internal error) — and the string-concatenation route (`string_concat_special`, `elaborate/src/strings.rs:843`): `h = {e, 8'h01}` (e = "") makes a handle equal to a live object, `H9ad 1 99` (`h == o`, and `h.x` reads its 99; iverilog 13.0 internal error), `h = {8'h41, a}` `H1d 0` where verilator 5.052 reports "Assign RHS expects a CLASSREFDTYPE 'C', got BASICDTYPE 'string'", likewise H2derefd hc4 hc5 h12d G11 G13 G19 H1_hnd (iverilog 13.0 runs `G11 0`)
* IEEE 1800-2017 §8.4
* PRE 01cac4b3 (return) and 3e541d67 (concatenation)
* Q-NESTCAT r2 ported the check into `string_concat_special` (r2b / r3: those 10 cells → E3009, nothing else in W/qnestcat/replan4/cells/h moved)
* Q-NESTCAT needs the concatenation route checked (r1: `h = {{8'h41, a}}` E3009 → `H1 0`, H9alias E3009 → `H9a 1 99`)
* silent-wrong
