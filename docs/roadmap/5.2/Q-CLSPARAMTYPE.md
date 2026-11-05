(§5.2 Q-CLSPARAMTYPE · rank ① · source Q-MEMBSRC plan probe clsk) (a class value-parameter override is not converted to the parameter's type):
* symptom: a class value-parameter override is not converted to the parameter's type and range: `class K #(parameter bit [2:0] KP = 3'd0) … K #(7) b;` prints `KP=7 bits=32 s2=9 s1=8` for `$bits(KP)`, `KP + 3'd2`, `KP + 3'd1`, where IEEE 1800-2017 §23.10 (ieee1800-2017.txt:33620–33629: an override is converted to the parameter's type and range) and verilator 5.052 give `bits=3 s2=1 s1=0` (= vita's `K #(3'd7)`)
* oracles: iverilog refuses the class
* cause: `hdl-parser/src/monomorph.rs:20` `subst_expr`
* evidence: PRE 9d60771d, every backend
* status: silent-wrong
