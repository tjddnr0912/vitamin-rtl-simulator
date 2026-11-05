(§5.2 Q-STRREPSHADOW · rank ① · source Q-NESTCAT r3 lens-sound q1 (B1, B1d)) (a string replication count folds a parameter that a function formal shadows):
* symptom: a string replication count folds a module parameter that a function formal shadows: `localparam int N = 2;` with `function automatic string g(int N); return {N{a}}; endfunction` (a = "a") gives `g(0)` and `g(3)` both `aa`, verilator 5.052 `[] [aaa]`
* witness: the nested `{N{{a}}}` the same
* oracles: iverilog 13.0 and sv2v refuse ("A reference to a net or variable (\`N') is not allowed in a constant expression.")
* cause: the string `Replicate` arm (`elaborate/src/expr_main.rs:939`) and `string_concat_special` (`strings.rs:867`) fold the count with `const_eval_in_scope`, whose identifier arm reads the scope's parameters (`const_fn.rs:331`) where lowering binds the formal — the hazard `const_bound.rs:203` names, on which the packed widths decline
* an indexed select's width takes the parameter too, illegal source accepted (Q-PKRDLSB r2 lens-sound N2, W/qpkrdlsb/r2/lens-sound/s0/f_formal_shadow; width probe W/qpkrdlsb/park/fshw.sv, PRE 719e37d2): `logic [7:0] v = 8'hFF; logic [31:0] k = 32'd2; parameter W = 3; function automatic logic [3:0] f(input int W, input logic [31:0] i); return v[i +: W]; endfunction` `f(2, k)` `R a 0111` (width 3), iverilog and sv2v refuse (`Indexed part select width must be an integral constants greater than zero.`), verilator refuses (`Width of :+ or :- bit slice range isn't a constant`), IEEE 1800-2017 §11.5.1 (ieee1800-2017.txt:12369–12370: a positive constant integer expression)
* fix: decline when a formal or an inline substitution binds a count name (then E3009 until Q-STRREPNC)
* oracles: IEEE 1800-2017 §11.4.12.2 (ieee1800-2017.txt:12053)
* evidence: PRE 3e541d67 = Q-NESTCAT r1–r3
* status: silent-wrong
