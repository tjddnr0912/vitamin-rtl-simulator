(§5.2 Q-CLSPARAMTYPE · rank ① · source Q-MEMBSRC plan probe clsk) (a class value-parameter override is not converted to the parameter's type):
* symptom: a class value-parameter override is not converted to the parameter's type and range: `class K #(parameter bit [2:0] KP = 3'd0) … K #(7) b;` prints `KP=7 bits=32 s2=9 s1=8` for `$bits(KP)`, `KP + 3'd2`, `KP + 3'd1`, where IEEE 1800-2017 §23.10 (ieee1800-2017.txt:33620–33629: an override is converted to the parameter's type and range) and verilator 5.052 give `bits=3 s2=1 s1=0` (= vita's `K #(3'd7)`)
* oracles: iverilog refuses the class
* cause: `hdl-parser/src/monomorph.rs:20` `subst_expr`
* evidence: PRE 9d60771d, every backend
* status: silent-wrong
* witness (2026-11-05, Q-DECLTYPE grounding g1/class_param_typed): the default is not converted either: `class C #(logic [3:0] W = 5'h1F); function int g(); return W; endfunction endclass` `C #() o;` `$display("R w=%0d", o.g());` PRE `R w=31` at exit 0, verilator `R w=15`, hand-IEEE 15 (§6.20.2, ieee1800-2017.txt:5101-5118: a typed, ranged value parameter takes its declared range), iverilog `t.sv:2: syntax error` `t.sv:2: error: Invalid module item.`, sv2v `` t.sv:2:3: Parse error: missing expected `endmodule` ``
* cause: `ClassParam` (`hdl-ast/src/lib.rs:216`) keeps a name and a default only, so the declared type sizes and signs neither the default nor an override
* note: Q-DECLTYPE's S12 leaves `ClassParam` without a type: this row records it with its first reader
* source: W/qdecltype/GROUND.txt G1, W/qdecltype/plan/PLAN.txt P6
