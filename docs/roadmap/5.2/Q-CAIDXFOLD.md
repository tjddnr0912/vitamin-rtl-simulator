(§5.2 Q-CAIDXFOLD · rank ① · source ENG-1c r2 lens-diff N-G (D03 D04 Q12 Q13 Q15 Q17 Q18)) (a continuous assign to a non-logic array element refuses legal constant indexes):
* slot: after Q-IDXVALUE
* a continuous assignment to an element of an array that is not `logic` (`real`, `int`) at a LEGAL constant index is E3018 (``drives an element of variable `top.ra` at an index that is not a constant``) unless the index is a constant, `+` or `-` of constants: the gate's fold (`elaborate/src/const_eval.rs` `const_expr_u64`: `Const` / `Add` / `Sub`, read by `cont_var.rs`) is narrower than the store's (`sim_ir::selfwidth::offset_value`) and than IEEE 1800-2017 §11.2.1 (ieee1800-2017.txt:11299-11302: parameters, constant function calls and constant system function calls are constant): `ra[1'b1]` (D03) and `ra[64'd1]` (D04, Q18), iverilog = sv2v `S t=2 r1=0000000000000000`
* witness: `ra[-1]` of `real ra[-2:-1]` (Q12, iverilog = sv2v `S t=2 rm1=0000000000000000 rm2=3fe0000000000000`) or of `real ra[-1:0]` (q3b)
* witness: `ra[$clog2(4)-1]` (Q13), `ra[P*2-3]` (Q15, iverilog = sv2v `S t=10 bits=0000000000000000`), `ra[f(0)]` (Q17), `$clog2(2)-1` (q3d), `f(0)` (q3e), `P ? 0 : 1` (q3j), `int'(0)` (q3l)
* witness: `int ia[-1:0]; assign #2 ia[-1] = a; assign #2 ia[0] = b;` (q3m: iverilog = sv2v = verilator `S t=5 i0=3 i1=5`)
* oracles: verilator runs the `real` cells without step (c) (`8000000000000000`)
* fix: the gate folds the index as the store does (one spelling) and keeps its every-element-once count
* status: over-refusal, loud
* status: pre-existing (PRE = BASE §4.5.626 = ENG-1c)
* source: r2 lens-sound N-4 (q3b q3d q3e q3j q3k q3l q3m)
