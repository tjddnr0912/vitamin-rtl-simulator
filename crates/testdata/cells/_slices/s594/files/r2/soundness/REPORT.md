# s594 r2 soundness REPORT (in progress)
round: 2
## (4) r1 cells + 12 new on post_b: one-shot == staged 35/35; F1 cells C1 C2 C3 C8 C9 = PRE; C3c = PRE (known-wrong, lost gain); Fa 0 = PRE; Fc/Fk U=252 B=8 (no leak); Pr/Pd E3009 all; Uu L=0 = PRE (post_a 1, oracles 1); Prr/Pdr/Uur r=1 all.
## FINDING G1: laundered AE value (param A = fl(0), never-assigned) in a CALL-FREE region:
Apg `if ((A + (X+{N{1'b0}})) == 8'hFC)`: PRE else | post_a then | post_b then (staged then) | ivl else | s2v else | vl then -> OK->WRONG (2 oracles), present since post_a.
Ap localparam int twin: PRE=post_a=post_b L=1 (ivl 0, vl/s2v X) pre-existing; Apl literal-count twin: L=1 all builds, pre-existing.
## Mutants: Ma (every Literal region releases on drop) KILLED by rule_a_self_determined_position_nested_..., rule_a_comparison_nested_...; Mb (leaf_rule via ast_contains_call, misses Concat/Replicate) SURVIVED --workspace (equivalent by R2 E3009: no fold arm for a call in a concat/count; not proven for every lane).
