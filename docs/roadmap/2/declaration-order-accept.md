(§2 declaration-order-accept · OPEN · in Scoping / imports / block-locals) (a type name used before a later same-name declaration binds the earlier one):
* a type name used before the same scope declares or imports another type of that name binds the earlier declaration (IEEE 1800-2017 §26.3; §4.5.592 takes Icarus Verilog as the oracle on this axis), which IEEE 1800-2023 §6.18 makes illegal: verilator 5.052 refuses each (`Reference to 'st' before declaration (IEEE 1800-2023 6.18)`) and vita accepts it at exit 0 — u.mod_redecl_after `a=5` (iverilog `a=5`, sv2v → iverilog `a=16`), u.mod_redecl_after_pat `s=34` (iverilog and sv2v → iverilog `34`), u.mod_redecl_after_kpat `s=34` (hand-IEEE; sv2v → iverilog `03`, iverilog refuses the keyed pattern), rv.q3b `w=05`, rv.q3c `w=05` (iverilog `05`, sv2v → iverilog `16`), rv.q3d `q=05` (iverilog), g.cls_then_mod_redecl `ga=16 ta=d` (PRE the same) and g.cls_then_mod_redecl_derived `ga=16` (E3010 before §4.5.614; iverilog and sv2v refuse the classes)
* accept-invalid under 2023 only
* refusing it would descend under 1800-2017
* oracles: verilator
* status: OPEN
