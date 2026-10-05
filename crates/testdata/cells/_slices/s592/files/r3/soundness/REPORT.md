# s592 r3 soundness lens REPORT
round: 3   status: completed   lens verdict: FAIL (1 BLOCKING)
POST $S/s592/post_d/vita md5 5119bb026cab5cde3875b0c7b2208722 ; PRE $S/s592/pre/vita md5 86a2d84a21d0329c57541e51ae9eb4d2
## BLOCKING B-1 (same root class as r2's M4/V06n/Z5m/Z5n descents; new route: param-only arms)
SN6_hierparam_chain_coinc: iv P=1 | sv P=1 | vl P=1 | PRE P=1 | post_b E3010 | post_d P=9 rc0 (staged = one-shot).
Arm K `localparam P = Q;`, default `localparam P = 1;`, block declares K=8 and Q=9 after the case, read gb.g.P.
gen_builds_in is false for both arms, so the later walks take the record silently; generate.rs `(_, ModuleItem::Param(p))`
re-binds P in every walk and reads the later Q=9 -> hier_params[top.gb.g.P]=9. Landing defect pre-existing (SN7, no
mismatch: iv 1 | vl 9 | PRE 9 | post_d 9). SN5 (default P=2): iv 1 | sv 2 | vl 2 | PRE 2 | post_d 9.
## MAJOR M-1 gen_decision.rs header: "the walk's output is the same whichever arm it walks ... (nets, parameters and
routines are built by the Nets walk alone)" false; IMPL "take the record silently (PRE's output)" false:
SN1 iv 1 | sv/vl 2 | PRE 2 | post_d 1 ; SN2 iv Unable to bind | sv/vl 2 | PRE 2 | post_d undeclared gb.g2.P ;
SN4 iv Unable to bind | sv/vl 12 | PRE 12 | post_d undeclared gb.L[2].P (these three move toward iverilog).
## MINOR test gaps: MV1 (VarInit drops scalar string) SURVIVED ws 9161/9161, killed by CS1 (MV1 @s=a rc0 | post_d E3010);
MV2 (has_init any->all) SURVIVED ws 9161/9161, killed by CM1 (MV2 @v=9 bits=4 rc0 | post_d E3010).
## (3) r1 45 cells: post_b == post_d 45/45; PRE->post_d same 34, movers 11, DOWN 0 (IEEE).
