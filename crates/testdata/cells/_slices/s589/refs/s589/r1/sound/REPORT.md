# §4.5.589 round 1 — SOUNDNESS lens REPORT (final)
round: 1 · lens verdict: FAIL (product shakes: 6 oracle-decided cells correct->wrong, staged = one-shot) · tool calls 47/60 · designed cells 33/40
binaries: PRE $S/s589/pre/vita e1e7e571… ; POST $S/s589/post_a/vita f13c0978… ; staged pre/sep, post_a/sep. cells: $S/s589/r1/sound/c

## S1 BLOCKING (new instance; root class pre-existing = "imported routine filed under the importer runs its body in the importer", §4.5.440)
Mechanism (census):
- package.rs:874-886 `funcs.entry(n).or_insert(f)`: `import q::*` / `import q::g` inside package p FILES q's functions in pkg_funcs[p].
- decl_scope.rs:288-296 `decl_probe_fn`: window half returns `pkg_funcs[w.pkg].get(f)` with `Some(w.pkg)` = (q::g, Some("p")).
- const_fn.rs eval_const_call_at: `saved_pkg = const_call_pkg.replace(pkg)` -> g's BODY runs with const_call_pkg = p -> g's bare `K` resolves to p's K.
- PRE half: bare `g` at the module site resolves through the MODULE's own import of q -> body in q -> correct. Window half "answers" -> `win.or(pre)` keeps the wrong value.
Cells (PRE | POST | iverilog / verilator / sv2v):
- c04_impfn_hdr (return range `[g():0]`, wildcard): `v=15 b=4` | `v=1023 b=10` | `v=15 b=4` x3
- c13_impfn_expl (explicit `import q::g` in p and top): `v=15 b=4` | `v=1023 b=10` | `v=15 b=4` x3
- c05_impfn_fml (formal range): `L=-1 v=15` | `L=-1 v=1023` | `L=15 v=15` x3 (L wrong on both: pre-existing)
- c14_impfn_rep (body lane `{g(){1'b1}}`): `v=7` | `v=511` | `v=7` x3
- c15b_impfn_task (task local `[g():0]`): `v=15` | `v=1023` | `v=15` x3
- staged (vcmp/velab/vrun) == one-shot on all four, PRE and POST.
- root pre-existing: c12_impfn_body_ce (p's routine BODY calls imported g): PRE = POST `L=9 v=9`, oracles `L=3 v=3` x3.
- trigger needs p to declare a same-named constant: c17_impfn_noK PRE = POST `v=15 b=4` (correct).
Claim falsified (MAJOR): "The window never writes const_call_pkg" — the window's callee probe chooses the package the interpreter then writes (filing package, not declaring).
Controls unmoved: c01/c02 module-local h shadowing wildcard p::h (`v=255 b=8` = oracles), c03 task twin, c07 for-var `i` vs package `i` (`L=3 v=3` = oracles), c11 generate routine.
Not measurable (loud on PRE and POST): c06 function-local localparam (E2002 parse), c08 class localparam (E2002), c16 `$bits` in parameter (E3009).
- c20_impfn_dflt (formal DEFAULT `= g()`, interpreter lane L1): PRE `L=3 v=9` | POST `L=9 v=9` | iverilog `L=3 v=3`; verilator build fault `Duplicate declaration of function: 'h4__Vtcwrap_1'`; sv2v `has been called with missing/empty parameters`. L: correct->wrong on iverilog + IEEE 1800-2017 §13.5.3 (default evaluated in the declaring scope; in p, `g` is q::g whose body resolves in q, §26.3). v wrong on both (pre-existing).
Lanes reached by S1: L1 default, L4/L4b/L5 return + formal, L4 task local, body lane. Fix direction: `decl_probe_fn` must answer only for a callee p DECLARES (`pkg_own_rtns`) or carry the declaring package; a decline keeps PRE.

## Other cells (batch 3)
- c18/c19 2-D packed return `[W:0][1:0]` / `[1:0][W:0]`: PRE `v=65535 b=16` -> POST `v=255 b=8` = 3 oracles (silent->correct).
- c22 2-D packed body local: PRE `L=65535 v=65535` -> POST `L=255 v=255` = 3 oracles.
- c21 forward reference (routine before `localparam W` in p): PRE `v=255 b=8` -> POST `v=15 b=4` = verilator = sv2v; iverilog `Unable to bind parameter W ... declaration after use` (split; not a descent).
- k11b/k13: POST corrects (`Q=15`, `v=31 b=5` = 3 oracles); k12 PRE = POST `v=7` = 3 oracles.

## Census answers
(3) kept `$pkg$<p>.*` bindings: `$pkg$` key builders in crates/elaborate = package.rs:490 (elaborate_package prefix), driver.rs:750 (check_decl_name_collisions, runs BEFORE elaborate_package of that package), driver.rs:436 (diag label strip), hier.rs:575 (refuses `$pkg$` prefixes), decl_scope.rs:280 (probe). Whole-map readers of the six tables: only instance_array.rs:105-151 clone/restore (verbatim). walk_scopes outward walk never forms `$pkg$p.` from a module/generate prefix. No reader outside the probe found.
(1) side effects of the double fold: `error`/`warn` take `&mut self` (driver.rs:333, :519); split closures are `Fn` over `&self`; interior-mutable Elaborator fields = const_call_fn, const_call_pkg, decl_* only (lib.rs grep). The PRE half writes only `decl_wins` (cache of declared sets, no answers). No diagnostic / net / frame side effect possible from the second fold.
(2) cache: `decl_wins` BTreeMap keyed (pkg, rtn), holds only the declared-name set; no answer cached; deterministic.
(5) arming census, unarmed folders of routine text (range_to_dims*/declared_odd_bound/record_declared_bounds_for/frame_packed_width/const_decl_wsign call sites): inline_task_locals.rs x3 (F2, known), block_local/hoist.rs x3, dynarr.rs x3 (dyn-array formal element width at the call site), const_array.rs x1, classes.rs (class, by design).
- S2 MINOR (new instance of F2's class): c23_task_blk static package task block-local `begin : b logic [W:0] x;` PRE = POST `v=255`, oracles `v=15` x3 (kept at PRE, no descent). Function twin c24_fn_blk corrected (`L=15 v=15` = oracles).

## Mutants (worktree $S/s589/rv_sound = 303703f9 + wt.diff, CARGO_TARGET_DIR $S/s589/target_rv_sound, debug)
M0 (unmutated): test file 32/32; k-cells = POST release values.
| M | mutation (decl_scope.rs) | test file | --workspace | killer cell POST -> mutant |
|---|---|---|---|---|
| M11 | decl_split never resets decl_reentered | 32/32 pass | 9080 passed, 15 skipped, rc 0 (SURVIVOR of suite) | k11_reent_then_fix `P=8 Q=15` -> `P=8 Q=255` (Q oracles 15 via k11b x3) |
| M12 | body lane drops `.filter(|s| s.owned)` | 32/32 pass | 9080 passed, 15 skipped, rc 0 (SURVIVOR of suite) | k12_imp_body_lane `v=7` -> `v=511` (oracles `v=7` x3) |
| M13 | kept snapshot drops `param_meta` (`meta: None`) | 32/32 pass | 9080 passed, 15 skipped, rc 0 (SURVIVOR of suite) | k13_bits_meta `v=31 b=5` -> `v=255 b=8` (oracles `v=31 b=5` x3) |
All three killed only by new cells; no pin covers re-entry-then-correction, the body lane's owned filter, or the kept meta table (S3 MINOR: add k11/k12/k13 as pins). Source restored (cmp identical), worktree + target removed.

(4) declared-name exclusion = formals, body enum labels, body_decls + collect_block_local_decls, own name (pkg_body_scope.rs rtn_declared_names). c07 for-init `int i` vs package `i`: PRE = POST = oracles `L=3 v=3`. Function-local / class localparam: E2002 parse on PRE and POST (not reachable). Default-arg scope pushes an empty set (pkg_body_scope.rs:252) — matches §13.5.3 (formals not visible in a default).
(6) re-entry key (pkg, rtn) by name: k11 re-entry cell keeps PRE `P=8` (no oracle: iverilog assertion, verilator internal fault, sv2v `P=0`) and the later correction `Q=15` survives in POST; flag reset only by the Idle-phase split (M11 shows nothing tests it). Interface/$unit/generate controls: c26 $unit, c11 generate, c01/c02 module = oracles on PRE and POST; c25 interface E3009 `$bits` on both.
Window lifetime: no `return`/`?` between decl_enter and decl_exit in frames_reserve.rs 801..1029 and 1243..1374 (grep); every other arming is enter/one call/exit; no catch_unwind in crates/elaborate.

## Findings table
| id | sev | class | file:line | evidence | cell PRE / POST / oracles |
|---|---|---|---|---|---|
| S1 | BLOCKING | new instance; root pre-existing (imported routine filed under importer runs body in importer, c12) | decl_scope.rs:286-295 (`Some((def, Some(w.pkg.clone())))`), const_fn.rs:1536 (`const_call_pkg.replace(pkg)`), package.rs:878/883 (`funcs.entry(..).or_insert`) | probe answers for a callee p only imports | c04 `v=15 b=4` / `v=1023 b=10` / `v=15 b=4` x3; c13, c05, c14, c15b, c20 above |
| S1m | MAJOR | claim | decl_scope.rs module doc + brief "never writes const_call_pkg" | the probe picks the package the interpreter then writes | same cells |
| S2 | MINOR | F2 class, new instance | unarmed; which of block_local/hoist.rs or inline_task_locals.rs it takes is UNVERIFIED | static task block-local | c23 `v=255` / `v=255` / `v=15` x3 |
| S3 | MINOR | test teeth | crates/cli/tests/decl_scope_pkg_text.rs | M11/M12/M13 suite survivors | k11/k12/k13 |
c12 root: no ROADMAP line found (grep importer/imported into + body/file/resolv) — filing status UNVERIFIED.
