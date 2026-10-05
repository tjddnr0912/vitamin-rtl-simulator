# §4.5.589 round 1 — lens DIFFERENTIAL — verdict FAIL (the product shakes)
status: COMPLETE. 40 designed cells, ~30 tool calls.
PRE  = $S/s589/pre/vita  md5 e1e7e57148bb83ad35d2c4f68247a290 (staged $S/s589/pre/sep)
POST = $S/s589/post_a/vita md5 f13c0978e2ae4ae9ee26979a3e0e5bd6 (staged $S/s589/post_a/sep)
harness: $S/s589/r1/diff/h.sh (PRE, POST, PRE staged, POST staged vcmp->velab->vrun, iverilog 13 -g2012,
verilator 5.052 --binary --timing, sv2v 0.0.13->iverilog); cells + raw outputs $S/s589/r1/diff/c/b{1,2,3}/<cell>.{pre,post,pres,posts,ivl,vl,s2v}

## D1 BLOCKING — correct->wrong; new instance, root class pre-existing (filed-copy seeding)
Mechanism (source): decl_scope.rs decl_probe_fn returns `self.pkg_funcs.get(&w.pkg)?.get(f)?` as `(def, Some(w.pkg.clone()))`.
package.rs Import arm files every imported function into the importer: `funcs.entry(n).or_insert(f)`.
So inside an owned routine's window, a bare call h() to r::h imported into p is tagged p, and eval_const_call_at runs
h's body with const_call_pkg = p (seeded from p's constants). PRE resolved the same call through the caller's own
import (`const_func_table` + `const_fn_pkg` = r) and was right. Trigger: p declares a constant with the same name as one
r::h's body reads. Base cell (r: K=8, h(){return K;}; p: import r::h; K=3; module: import r::*):
| cell (lane) | PRE | POST | POST staged | iverilog | verilator | sv2v |
|---|---|---|---|---|---|---|
| ifn_hdr_ce (L1 return range, localparam) | P=255 | P=7 | P=7 | P=255 | P=255 | P=255 |
| ifn_hdr_rt (L4 return range) | v=255 | v=7 | v=7 | v=255 | v=255 | v=255 |
| ifn_wc (p uses import r::*) | P=255 v=255 | P=7 v=7 | P=7 v=7 | P=255 v=255 | P=255 v=255 | P=255 v=255 |
| ifn_fml (formal range) | P=511 v=255 | P=511 v=7 | P=511 v=7 | P=255 v=255 | P=255 v=255 | P=255 v=255 |
| ifn_task_fml (L4 task formal) | v=255 | v=7 | v=7 | v=255 | v=255 | v=255 |
| ifn_loc (L4 local) | P=1 v=255 | P=1 v=7 | P=1 v=7 | P=255 v=255 | P=255 v=255 | P=255 v=255 |
| ifn_inl (L5 inline lane) | v=255 | v=7 | v=7 | v=255 | v=255 | v=255 |
| ifn_body_rep (L6 `{h(){1'b1}}`) | v=255 | v=7 | v=7 | v=255 | v=255 | v=255 |
| ifn_bits_rt (L8 `$bits(p::f())`) | b=8 | b=3 | b=3 | b=8 | b=8 | b=8 |
| ifn_dflt (interp default `a = h()`) | P=8 v=3 | P=3 v=3 | P=3 v=3 | P=8 v=8 | build: Duplicate declaration of function 'f__Vtcwrap_1' | error: called with missing/empty parameters |
4-way: real gap on all rows (ifn_dflt: iverilog + IEEE 1800-2017 13.5.3, default evaluated in the declaring scope).
P of ifn_fml / ifn_loc and v of ifn_dflt are pre-existing wrongs on both binaries (not attributed).
Attribution controls: ifn_nok (p has no own K): PRE=POST=3 oracles 255. ifn_scoped (`r::h()`): PRE=POST=3 oracles 255.
Root class on PRE: ifn_l3 (p::f BODY calls h): PRE P=3 v=3 = POST, oracles P=8 v=8 -> the seeding defect exists on PRE (L3 +
run time); POST sends the window lanes into it. The implementer's M8 killer (own8_imp_rba) calls g from q's BODY and pin
package_text_binds_its_own_import uses an r::f whose body reads no constant, so neither sees D1.
Fix shape (UNVERIFIED, not built): decl_probe_fn must not tag a filed copy with the importer — miss for a non-owned name
(falls back to PRE; costs the pinned package_text_binds_its_own_import correction, back to PRE P=232) or tag it with its
declaring package.

## D2 MAJOR — false doc claim
decl_scope.rs:284 `/// The declaring package's function for bare callee f` — for an imported function it returns the
IMPORTING package's filed copy tagged with the importer (D1). The brief/IMPL "correct->anything 0" holds on the measured sets
only; D1 refutes it as a product property.

## MINOR / residue (pre-existing, PRE identical)
- nest_body (non-owned imported body nested in an owned body, constant replication): PRE=POST v=0, oracles v=3 (🆕 AC fallback).
- ifn_fml P=511 / ifn_loc P=1 (call-holding formal/local width in the interpreter, oracles 255); ifn_dflt v=3 (run-time default).
- uns_imp_rt: vita/iverilog/verilator b=4 v=15, sv2v b=8 v=255 (sv2v split; no move).

## non-findings (measured)
- corrections: two_mod PRE a=15 b=65535 -> POST a=255 b=255 = 3 oracles; POST x3 md5 identical (c4d9b597...).
  nest_pq (p header calls q::g3, q's window nested) PRE v=65535 -> POST v=255 = 3 oracles.
- no leak onto caller text: tact_rep2 (caller actuals `{g(2){1'b1}}`, `K'(5'd31)`, output `v[g(2):0]`) all 7 runs ta=3 ts=3 v=7;
  mod_shadow_ce (module h shadows imported owned h, constant context) all 7 runs P=255 v=255; nest_body_imp all v=3.
- kept-binding fidelity: enum_imp_rt b=7 v=127, uns_imp_rt b=4 v=15 PRE=POST.
- staged = one-shot on all 40 cells, PRE and POST.
- loud on both binaries (window not reached; no move): enum_imp, ifn_bits, uns_imp, wide_imp, wide_own, real_imp, nest3
  (E3009 `$bits(...)` localparam), wide_*_rt (E3009 >64-bit), real_imp_rt (E3009 $rtoi), rec_mut (E3009), ifn_noimp (E3009),
  loc_lp/cls_m/ifn_export/tact_rep_fr/tact_rep_st (E2002 parse), tact_cast_fn (E3009).
- corpus: static census, non-DV corpus packages with import+function = 0 (only ibex DV/vendor pkgs). corpus-runner not run
  by this lens (UNVERIFIED here; IMPL reports all ok).
