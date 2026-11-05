(§5.2 Q-SVASCOPE · rank ① · source ENG-1b r2 lens-sound N6 (NP_named_prop_scope)) (names inside an SVA bind the wrong object):
* symptom: names inside an SVA bind the wrong object: a named property's body resolves in the asserting scope, not the declaring one (NP_named_prop_scope vita `F t=15\|t=25\|t=35`, vl `A end`)
* symptom: an assertion inside a for-generate does not see its genvar and reads an outer net of that name (F1_genvar_outer_net vita `p=9`, vl `p=1\|p=2`)
* symptom: a concatenation in a generate block reads an outer localparam over an inner string (STR_concat_gen vita `n=c`)
* evidence: PRE = ENG-1b
* IEEE 1800 §16.12, §27
* source: r2 lens-diff N6 (F1_genvar_outer_net), N7 (STR_concat_gen, F1_outer_lp_inner_strvar)
