(§2 wildcard-string-candidate · OPEN · in Scoping / imports / block-locals) (a wildcard string or real package constant is no import candidate):
* A wildcard string or real package constant is no import candidate (`apply_import_consts` writes no `str_param_raw` / `real_param_val`; §3 ⑨), so another wildcard's integral constant of that name binds without the §26.3 ambiguity (Iasn, Iarn `P=3`; c04 `sa P=5`, c05 `ra P=5`; iverilog `Ambiguous use of 'P'. It is exported by both 'pa' and by 'pb'.`, sv2v `identifier "P" ambiguously refers to the definitions in any of pa, pb`)
* record the candidate in `wc_origin`
* 2 oracles
* OPEN
