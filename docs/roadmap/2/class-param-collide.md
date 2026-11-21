(§2 class-param-collide · OPEN · in Class fields) (a class specialization whose actual spells `?` takes another one's value):
* symptom: two specializations of one parameterized class whose actuals differ only where one spells a `?` (or x / z) digit are read as one, so the second instance's parameter takes the first one's value at rc 0; PRE == POST (§5.2 Q-LOSSYSPELL's premise judge 1, filed 2026-11-19 at batch 6's landing, not caused by batch 6); IEEE 1800-2017 §8.25 makes each set of actual values its own specialization, and §5.7.1 reads `?` as `z` in a literal
* witness: b4_classcollide (W/lossy/premise/cells/b4_classcollide/t.sv), `class C #(parameter logic [7:0] P = 0); function logic [7:0] v(); return P; endfunction endclass`, `C #(8'b0000101) a; C #(8'b0000_1?01) b;`, `$display("A a=%b b=%b", a.v(), b.v())`: vita 54bc2791 rc 0 `A a=00000101 b=00000101`
  * oracles: verilator 5.054 --binary `A a=00000101 b=00001001` (its 2-state reading of `?`); iverilog 13.0 `t.sv:2: syntax error` / `t.sv:2: error: Invalid module item.` (no parameterized class)
* note: where the name is made (the specialization's rendered name, `hdl-parser/src/monomorph.rs`) is read, not measured (UNMEASURED); the class parameter's declared type that the same path drops is §5.2 Q-CLSPARAMTYPE's (its 2026-11-19 face)
* oracles: 1 oracle (verilator, 2-state) + hand-IEEE
* status: OPEN
