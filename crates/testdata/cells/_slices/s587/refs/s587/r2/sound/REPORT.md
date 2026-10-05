# soundness lens r2 (delta) — s587

status: started

## Q1 ia_element_next: writer instance_array.rs:351 only (immediately before the elaborate_instance call); taker
  instance.rs:319 `mem::take` at entry (reset to false); inst_is_ia_element saved/restored per instance (:320/:331);
  readers instance.rs:592 (bind_params held) and :978 (elaborate_ports held). Nested instances take false. No path between
  set and take. CLEAN by census.
## Q4 plain path: const_site readers = const_fn.rs:1759 arm (== Required) + const_site.rs note_* (== / != Required);
  X2 rerun needs const_arm_hit (set only in the arm). NUL keys inserted only under Required, read only via contains_key of
  the NUL key; other env readers use user names (const_fn.rs:1189/1216, const_fn_width.rs:609/636/670); no env iteration.
  => plain twins cannot change. CLEAN by code reading.
## FINDING R2-F1 (BLOCKING, NEW loud->silent-wrong; same root class as r1 F1, NEW instance): $unit-scope routine text
  unitret_u ($unit f=3; $unit h returns [f(2):0]; top's own lone-unique f): PRE rc=1 E3009 x2 | POST rc=0 P=232 v=232 |
  verilator P=8 v=8 ; plain twin PRE=POST P=232 v=232 (pre-existing), iverilog P=8 v=8. text_foreign records packages and
  routine-declaring generate levels only (const_site.rs foreign_text_spans).
## x sources other than unassigned vars (xlit 4'bx, xinit ='x, xoob t[9], xdiv 8/0): vita E3009 in PRE and POST, both
  twins (interpreter declines x) -> no X2 gap there. (xoob oracles split: verilator 0001, iverilog 000x.)
## FINDING R2-F2 (NON-BLOCKING, NEW, loud): staged two-CU build declines a u-form fold the one-shot build folds
  stg/ (pkg.sv = package q + 30 localparams + `module leaf`, 812 B; top_u.sv imports q, instantiates leaf, P = f(2) at ~offset 170):
  one-shot POST_u `P=7 K=3` | staged POST_u (vcmp --work x2, velab -L) rc=1 E3009 "parameter `P` value is not a constant" ;
  PRE_u E3009 both flows; plain P=7 both flows, PRE=POST. Cause: cli/src/staged.rs:171 merges CU items without re-basing
  spans; text_foreign compares CU-local offsets (q's span [0,~812] from CU A contains top's text from CU B).
  False positives only (a package's own text is always inside its own span) -> loud, = PRE.
## genbare (bare `if (1) begin : g` generate declaring f): POST_u = PRE_u E3009 (held); plain P=7 PRE=POST (iverilog 3) -> covered.
## xshadow (outer 4-state t unassigned on the arm path, inner block re-declares t): POST_u folds P (its "P not a constant"
  error is gone) but the design stays rc=1 on the block-local gate "block-local `t` is referenced outside its begin…end" in
  PRE and POST, both twins -> latent X2 gap (a re-declaration removes the outer mark), masked by that gate -> NIT.
## pkimp2 (q imports r::g; q::h calls g; top has own lone-unique g): P=3 v=3 everywhere (PRE/POST, both twins, oracles) -> clean.
## pkbody_u (package routine BODY via interpreter, P = q::h(2)): PRE E3009 | POST P=7 (= plain/oracle semantics) -> correct,
  but CHANGELOG/006 "Also unchanged …: a package routine's declarations or body folded from a module" is false for the
  interpreter path -> NIT text.
## step edge re-run on post_c: steps_u (n=22000, plain ~88k steps) E3009; st19_u P=19000 -> const_site.rs doc "only moves a
  fold within one step of the bound" is false (one step PER reached arm) -> NIT text.
## X2 coverage cells (all POST_u = PRE_u decline; plain PRE=POST wrong vs verilator x): return var of implicit [3:0],
  typedef, reg, integer (x2r*); typedef / enum / reg locals (x2l*); `t += 2`, `t++`, select write then t[2:1] read (x2c*).
## ifarr (interface instance array): E3009 "outside the MVP" in all -> n/a. pkown (package param range calls the package's own
  f; top has own lone-unique f): E3009 "[in q]" in PRE and POST, both twins -> no leak of top's f through cur_prefix==$pkg$q.
## Q2 census: spans are offsets into ONE preprocessed buffer per compilation unit (hdl_preprocess SourceMap; include/macro
  text expanded inline) -> comparable inside a CU; NOT across CUs (R2-F2). const_call_pkg writers: const_fn.rs:1517
  (interpreter call, restored :1550 after the body closure; args and defaults evaluated before the switch) and
  const_bound.rs `const_fn_call_width_safe` (width walk in the package scope); both switch resolution to the package.
## Q3: env value reads = const_fn.rs:912 (Ident, checked), resolver ~:1221 (checked), exit read :1543 (checked); select-write
  RMW :1604 keeps the mark (conservative); args read through caller_env -> Ident arm. No env iteration/len/clone anywhere ->
  NUL key cannot reach output or digest. Flags: saved+reset only at the outermost stated fold, restored at exit; re-run is
  Held, so neither flag can be set inside it (arm and note_read both require Required).
## Q5 text (NIT): lib.rs:512-513 "set only by `const_required`, read only by the … arm" and const_site.rs:72 "Read only by the
  interpreter's unique-violation arm" are false (writers const_site.rs:142/:150; readers :283/:299/:316). const_site.rs
  text_foreign doc "the one lane that switches resolution" — two writers. const_site.rs "within one step of the bound" false.
  CHANGELOG/006 "a package routine's declarations or body folded from a module" stays E3009 — false for the interpreter
  body path (pkbody_u POST P=7); their exclusion list omits $unit routines (R2-F1).
## Q6: crates/cli/tests/unique_const_fn.rs 1460 lines vs CONTRIBUTING.md:170 "under about 1000 lines" -> split needed.
## VERDICT r2: FAIL — R2-F1 BLOCKING ($unit routine text not in text_foreign; new loud->silent-wrong).
## r1 F1 re-measured on post_c: pkret/pkloc/pkfml/pkcast/pkimp u -> E3009 (= PRE); plain twins unchanged (= PRE);
  pkrng u b=8 (= oracle); iash u E3009 (= PRE), plain W=7. r1 F1 (package text) CLOSED; its $unit instance is R2-F1.
status: complete
