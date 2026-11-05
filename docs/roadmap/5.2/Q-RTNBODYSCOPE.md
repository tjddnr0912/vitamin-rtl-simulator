(§5.2 Q-RTNBODYSCOPE · rank ① · source Q-AESINK re-plan (diff N3, W/qaesink/replan/REPLAN.txt Q2)) (a constant call's body folds at the call's scope, not the routine's):
* slot: after 3
* slot: `$unit` half after 4
* symptom: a constant call's BODY folds at the call's scope: §2 🆕 AG names a routine's header only: a body-local range: sc_widf_gif `logic [W-1:0] t; t = 8'hA5; f = t;`, home W=8, call W=4, `00000101`, all three `10100101`
* witness: a body's own name read: bd_loop_gif `for (int i = 0; i < W; i++) t[i] = 1'b1;` over `logic [7:0] t`, home 4, call 8, `11111111`, all three `xxxx1111`
* witness: bd_if_gif, bd_sel_gif E3009, all three `xxxx1111`, and their `+:` spelling gives a value: bd_if2_gif `t[0 +: 4] = 4'hF; if (W == 8) t[4 +: 4] = 4'hA;` `10101111`, bd_sel2_gif `t[0 +: W] = 8'hFF;` `11111111`, all three `xxxx1111`, each with a twin (the call's constant renamed; bd_loop_gif's, bd_loop_unit's, bd_if2_gif's, bd_sel2_gif's `00001111`, all three `xxxx1111`) that §4.5.633's set marks `f.t`
* witness: a type parameter's kind: sct_tp_gif `00000000`, iverilog `sct_tp_gif.sv:8: syntax error` `sct_tp_gif.sv:8: error: Invalid module item.`, sv2v and verilator `0000xxxx`, its twin `00000000`, all three `0000xxxx`, marked `f.t`
* IEEE 1800-2017 §23.9
* note: no seed in vita's run, so §4.5.633's mark says nothing
* fix: first: the window `decl_local_win` opens for package routines opens for module and generate routines, then the `$unit` half on row 4's identity
* source: r3 differential lens D2, D4
