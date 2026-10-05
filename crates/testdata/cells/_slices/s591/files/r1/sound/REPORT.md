# §4.5.591 r1 SOUNDNESS lens — REPORT
round: 1
status: COMPLETE. lens verdict: FAIL (1 BLOCKING, 2 MAJOR, 2 MINOR)
PRE  $S/s591/pre/vita    md5 ac965d2938f4b5bbbda4291ecd64d1dc (release)
POST $S/s591/post_a/vita md5 732fb809ebad55f14fd214d72925c960 (release)
M13  $S/s591/target_rv_sound/debug/vita md5 6d6db33dc87238578a4068ba0869d840 (debug; POST tree + M13 only; deleted after use)
cells: $S/s591/r1/sound/cells/c*.sv (31 designed) ; raw lines in c*.res (PRE|POST|iverilog|sv2v), w_*/vl/vl.out (verilator)

## F1 BLOCKING — correct->loud: position-blind wildcard ambiguity (pre-existing class, unrecorded; U-a reaches 9 new shapes)
New ambiguity arms clear every map at IMPORT time; vita applies all module imports before any body item, so a
reference written BETWEEN the two wildcard imports (which bound the name from the first package) is refused.
PRE's surviving binding was that first-referenced one; iverilog, sv2v and verilator all answer it.
| cell | shape | PRE | POST | iverilog | sv2v | verilator |
| c22 | import pa::*(n); localparam int A=P; import pb::*(w) | rp A=3 P=3 | E3009 `A` not a constant; E3010 top.P | rp A=3 P=3 | rp A=3 P=3 | rp A=3 P=3 |
| c24 | import pb::*(w); localparam [64:0] A=P; import pa::*(n) | rw A=..625 P=..625 | E3009; E3010 | = PRE | = PRE | = PRE |
| c26 | import pa::*(var W); wire [7:0] A=W; import pb::*(w) | vw A=5 W=5 | E3010 top.W x2 | vw A=5 W=5 | vw A=5 W=5 | vw A=5 W=5 |
| c27 | import pa::*(w); localparam [64:0] A=W; import pb::*(var) | wv A=..625 W=..625 | E3009; E3010 | = PRE | = PRE | = PRE |
| c23 | module top import pa::*; #(parameter int X=P) (); import pb::*(w) | hp X=3 P=3 | E3010 top.P | hp X=3 P=3 | hp X=3 P=3 | hp X=3 P=3 |
| c30 | module top import pb::*; #(parameter [64:0] X=P) (); import pa::*(n); gen-if | hw big; hw X=..625 P=..625 | E3010 gen-if; E3010 top.P | = PRE | = PRE | = PRE |
| c38 | interface (iface_inst.rs) body = c22 | inw top.x A=3 P=3 | E3009; E3010 top.x.P | = PRE | = PRE | = PRE |
| c39 | instance array c u[1:0], body = c22 | anw top.u[0]/[1] A=3 P=3 | E3009; E3010 top.u[1].P | = PRE | = PRE | = PRE |
| c33 | package pc: import pb::*(w); localparam [64:0] A=P; import pa::*(n); localparam [64:0] B=P | pwn A=..625 B=..625 | E3009 `B` not foldable | = PRE | = PRE | = PRE |
Staged (vcmp/velab/vrun) = one-shot on c22 c33 c38 (PRE vrun prints the PRE line; POST velab rc=1, same diagnostics).
Arms: wides-loop amb `unbind_param` (c22 c23 c38 c39); narrow-loop amb `wide_param_bits.remove` (c24 c30);
wides-loop amb alias removal (c26); vars-loop amb `wide_param_bits.remove` (c27); package one-state hoist + narrow-loop arm (c33).
Class pre-existing: narrow-narrow twin c25 already loud on PRE, 3 oracles `rn A=3 P=3`. ROADMAP has no "ambigu" row.
wrong->loud siblings (acceptable): c07 PRE `pr A=3 B=5`, c32 PRE `pnw A=3 B=9`, oracles B=3, POST loud.

## F2 MAJOR — M13 is NOT equivalent; package.rs comment "(an ambiguous one is already unbound in every map)" is false
Producer census of `params` at an ambiguous ("") key: explicit NARROW const arm (package.rs `} else if let Some(&v) = consts.get(..)`
-> `bind_param_value(key, v)`) has no wc_origin check; a `$unit` explicit import has same_scope=false, so the new
two-explicit check does not fire. Cell c01: $unit `import pa::*; import pb::*; import pn::P;`, module `import pw::P;`(w):
PRE `m13 small` `m13 P=18446744073709551625 b=4`; POST identical; M13 `m13 big` `m13 P=18446744073709551625 b=10`;
iverilog, sv2v, verilator `m13 big` `m13 P=18446744073709551625 b=10`. Staged POST = one-shot POST.
(The underlying silent-wrong is the known `$unit`+module one-import-scope class, Icue_nw_gif; PRE = POST.)

## F3 MAJOR (doc) — "`localparam int` asks `params` alone" is false
Test header (import_one_binding_per_key.rs: "the integer fold (generate-if/-case, widths, `localparam int`) asks
`params` alone") and package.rs explicit-wide comment ("the integer fold (generate-if/-case, a `localparam int`)
answered it"). c28 on PRE (params=3 stale, wide=..9): `xw small` (gen-if read params) but `xw A=9`
(`localparam int A = P` read the wide value).

## F4 MINOR residue — string/real package constants never enter the ambiguity machinery
apply_import_consts writes no str_param_raw / real_param_val (census: only package.rs:633/644/984/994 own-package,
instance.rs:692/701, params.rs, generate.rs, decl_scope.rs:407/410). c04 PRE=POST `sa P=5`, c05 PRE=POST `ra P=5`;
iverilog `Ambiguous use of 'P'. It is exported by both 'pa' and by 'pb'.`; sv2v `identifier "P" ambiguously refers ...`.

## F5 MINOR — verilator "disqualified on import precedence" is not a registered disqualifier
ROADMAP §0 lists only iverilog defects ①–⑧; RULES lists verilator non-oracle axes x/z, out-of-range, event order.
The claim rests on the slice's own self-contradiction argument. On every F1 cell verilator = iverilog = sv2v.

## Q census results (no finding)
Q1 clear scope: keys are fq() = instance prefix (`$pkg$<p>` in packages) — no outer-scope key touched. `$bits(P)`
  on an ambiguous nw pair: PRE `ba b=32` -> POST loud (stale param_meta not read). §4.5.589 snapshot re-installs only
  param_meta (no value) at an ambiguous `$pkg$pc.P`; c06a/b PRE=POST loud. unbind_param leaves hier_param_range
  (UNVERIFIED reader impact). Package import wide inserts are not in saved_wide (pre-existing, `$pkg$` keys).
Q2 hoist: BTreeMap/BTreeSet only; package imports interleave with declarations in order (c07 c32 c33 show the
  position dependence). c08 PRE `pe Z=5` -> POST `pe Z=3` = 3 oracles. c09 all `pw K=9 Z=..625`.
Q3 labels: anonymous enum decl (c10) and `S[2]` labels (c12a/b) are E2002 parse errors on PRE and POST
  (unreachable); interface typedef refused ("typedefs inside an interface are outside the MVP", c11a/b) so the
  iface_inst.rs comment holds. Span filter compares spans of ONE preprocessed buffer (Span doc "into the
  preprocessed source"; cu_items per parser, module_items.rs:567-620; staged merge injects nothing; the same filter
  pre-exists at decl_collide.rs:641).
Q4 new errors: no legal design found. c17 gen-region explicit pair: POST E3009; iverilog `'P' has already been
  imported into this scope from package 'pa'.`, sv2v `import of pb::P conflicts with prior import of pa::P`.
  c28/c29 explicit import after a referenced wildcard: iverilog/sv2v refuse; c28 PRE `xw small` -> POST `xw big`
  (silent->silent, both wrong, pre-existing undetected §26.3 conflict); c29 PRE silent -> POST loud.
Q5 M13: refuted (F2).

## Mutants
| id | mutation | cell/test | result |
| M13 (re-measured) | package.rs explicit-wide arm `is_some_and(|p| !p.is_empty())` -> `is_some()` | c01 | output differs from POST (`m13 big`, `b=10` vs `m13 small`, `b=4`) -> NOT equivalent; survives the suite (implementer's 9111-test run) = test gap |
No further new mutants built (budget spent on F1 census).
