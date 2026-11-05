(§5.2 Q-STRREPNC · rank ① · source Q-NESTCAT r2 main D7 (W/main_d7/2026-10-30/qn-zstr/rt.sv)) (a string replication with a non-constant multiplier is refused):
* symptom: a string replication with a non-constant multiplier is refused: `r = {n{t}}` and `r = {{n{t}}, "x"}` (t = "ab", int n = 0) are E3009 "a non-constant replication count on a string is unsupported" (`string_concat_special`, `elaborate/src/strings.rs`; the string `Replicate` arm, `elaborate/src/expr_main.rs`), verilator 5.052 `RT [] 0` / `RN [x] 1`
* oracles: iverilog 13.0 "A reference to a net or variable (\`n') is not allowed in a constant expression."
* oracles: IEEE 1800-2017 §11.4.12.2 (ieee1800-2017.txt:12048-12052: "In the case of string replication, a non-constant multiplier is allowed.")
* evidence: PRE 01cac4b3 = Q-NESTCAT r2b
* note: `$size` of a fixed-size array is a constant (IEEE 1800-2017 §20.7, ieee1800-2017.txt:27516–27519) that the string route does not fold: `int arr[3]; r = {$size(arr){s}}` (s = "ab") is this E3009, iverilog 13.0 = sv2v = verilator 5.052 `[ababab] 6`, where the packed route folds it (`{$size(arr){8'h41}}` `414141` on all four)
* witness: the nested `{$size(arr){{s}}}` is `[bbb] 3` on PRE 3e541d67 (Q-NESTCAT) and this E3009 on Q-NESTCAT r1 / r2b — wrong → loud, its direct twin's answer (Q-NESTCAT r3 lens c21, b22; W/qnestcat/replan4/cells/s)
