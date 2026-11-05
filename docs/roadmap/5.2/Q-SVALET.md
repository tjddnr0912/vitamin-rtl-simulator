(§5.2 Q-SVALET · rank ①) (an SVA sampled-value register of a `let` argument is typed as one bit):
* slot: after 74
* symptom: the SVA sampled-value register of a `let` or another non-name argument is typed from the net walk (`lookup_net_scoped` None → 1-bit unsigned), not from the expression: `$past` / `$stable` / `$changed` of a multi-bit, signed or real `let` are wrong (F1_let_0arg_wide vita `p=1\|p=0\|p=1`, vl `p=201\|…\|p=203`; LT_let_real `p=1.000000`, hand 2.5), `$rose` / `$fell` exact (LSB)
* symptom: a generate-local `let` does not shadow an outer net or localparam (F1_let_gen_vs_outer_net vita -3, vl 240)
* evidence: PRE = ENG-1b
* IEEE 1800 §11.12, §16.9.3
* fix: type the register from the argument's resolved expression
* source: ENG-1b r2 lens-diff N1 = lens-sound N3 (F1_let_0arg_wide, F1_let_0arg_signed, F1_let_stable_wide, LET_wide_plain_vs_past, LT_let_wide, LT_let_signed, LT_let_real, LT_let_real_rose), r2 lens-diff N2 (F1_let_gen_vs_outer_net, F1_let_gen_vs_outer_lp)
