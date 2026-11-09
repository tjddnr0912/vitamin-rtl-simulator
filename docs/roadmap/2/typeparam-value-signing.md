(§2 typeparam-value-signing · OPEN · in Size cast / signedness) (a type-parameter value with a second or misplaced signing is accepted):
* symptom: a type-parameter value with two signings, or a signing after its packed range, is accepted, the first signing winning (`hdl-parser/src/type_params.rs:457` `:462` `:483`)
* IEEE 1800-2017 A.2.2.1 `data_type ::= integer_vector_type [ signing ] { packed_dimension } | integer_atom_type [ signing ]` (ieee1800-2017.txt:52141-52143): one signing, before the dims
* witness: i08_tpval_dbl `module c #(parameter type T = int signed unsigned) ();` `T v;` `v = -1;` `$display("A vlt=%0d b=%0d", v < 0, $bits(v));` PRE vita-main 8c970e9b and S1v-Q `A vlt=1 b=32` at exit 0
* witness: i09_tpval_trail `parameter type T = logic [3:0] signed` with `v = 4'hF;` PRE `A vlt=1 b=4` (W/s1v/premise1/cells)
* oracles: iverilog `t.sv:1: syntax error` `I give up.`, sv2v `sv2v: t.sv:1:42: Parse error: expected comma or end of declarations` (`:43` on i09), verilator `%Error: t.sv:1:42: syntax error, unexpected unsigned` (`t.sv:1:43: syntax error, unexpected signed` on i09) (W/s1v/a/filings.txt F2)
* note: S1v-Q refuses a leading signing only (W/s1v/RULINGS-premise1.txt N-1), and a type-parameter value does not route through `parse_param_prefix` (W/batch1/lens-sound/REPORT.txt census 1)
* status: OPEN
