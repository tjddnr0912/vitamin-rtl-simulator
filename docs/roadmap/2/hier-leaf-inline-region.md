(§2 hier-leaf-inline-region · OPEN · in Inline / frame binds) (a hierarchical leaf in a §11.6.1 region lacks the child's environment):
* A hierarchical leaf in a §11.6.1 region declines without the child's environment (typed parameter, `defparam`, generate / array instance)
* a width channel (`coerce_int_width`)
* 2 oracles
* OPEN
* witness (2026-11-05, Q-DECLTYPE plan p18b_envfold_untyped): `module c #(parameter P = 4'sd15) (); logic signed [P+16:0] hs; initial hs = 16'hFFFF; endmodule` `c u();` `logic signed [63:0] r1, r2;` `r1 = 64'(u.hs); r2 = u.hs;` `$display("A b=%0d r1=%0d r2=%0d", $bits(u.hs), r1, r2);`: PRE `A b=16 r1=65535 r2=-1` at exit 0, iverilog and verilator `A b=16 r1=-1 r2=-1` (sv2v → iverilog `syntax error` on the cast) -- the size cast reads the hierarchical signed leaf as 65535 where a plain assignment of the same leaf in the same run reads -1
* note: the same `r1=65535` with `parameter unsigned P = 4'sd15` (p18_envfold) and `parameter unsigned P = -4'sd1` (p18c_envfold_negu), where both oracles print 65535 (their P is 15), so r1 does not follow P on PRE
* note: silent-wrong, pre-existing (PRE 92805863)
* source: W/qdecltype/plan/cells/p18b_envfold_untyped, p18_envfold, p18c_envfold_negu
* cause: a signed child net whose width names a parameter the env fold binds inexactly or not at all (signed, ranged, an integer-unsigned slot, or a non-exact default) has no recorded shape and is read zero-extended
* evidence: W/qdecltype/premise1/cells k1_e7_mech `cs` / `cr` / `ci` (PRE 255, iverilog and verilator -1), W/qdecltype/plan2/cells e2_neglit, e3_identsrc and e4_ovr_env no-keyword twins `rt=255` (iverilog = verilator -1), e6_exactset `fr=f0` (the oracles 0), the texts copied whole from each RES at filing
* note: shared funnel: the exact gate (`elaborate/src/expr_size_hier_exact.rs:93`, `hier_leaf_shape`) and the size-cast lane (`elaborate/src/expr_size_ctx.rs:189-205`, `:397-410`, `:590-605`), fixed by neither S12 nor S1v
