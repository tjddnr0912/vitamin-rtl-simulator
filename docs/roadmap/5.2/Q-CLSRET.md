(§5.2 Q-CLSRET · rank ① S) (a class-typed function return is treated as one bit):
* slot: with the T′ rows
* a class-typed function / method return is one bit: `function C getc(int k)` records range None, `ParamType::Implicit` (`hdl-parser/src/functask.rs` `parse_function_def`, the typedef arm; `FunctionDef` has no class return type), so `$bits(getc(0))` is 1, `h = getc(1); h.v` reads X (verilator 5.052 20), `%0d` of the calls `1 0 1`, `h == b` after `h = b.self_()` is 0 (verilator 1), `a.next().v` is 0 (verilator 33)
* silent-wrong
* consumer: P-PCTU's call proxy (§4.5.630: `%u` / `%z` of a call declared without a return range is E3009, so 8 legal one-bit returns are loud until this row)
* source: P-PCTU r2 plan (W/ppctu/PLAN2.txt § 5 (ii), § 13 Q3; cells W/ppctu/plan2 pcells h_fnw hs_self hs_next)
