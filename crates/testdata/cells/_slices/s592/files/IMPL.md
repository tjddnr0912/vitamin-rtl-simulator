# §4.5.592 implementation report (D1) — IMPL.md
status: in progress (step 1 done; tests next)

Worktree: $S/s592/wt (branch s592 from 75f46453), CARGO_TARGET_DIR=$S/s592/target.
PRE: $S/s592/pre/vita md5 86a2d84a21d0329c57541e51ae9eb4d2 (release); staged $S/s592/pre/sep.

## Step 1 — implementation
- new crates/elaborate/src/gen_decision.rs: GenKind, GenDecision {Case(Option<usize>), If(bool), For(Vec<i64>)},
  GenInputs, ForStep; gen_decision_check (case/if), gen_for_recorded / gen_for_record / gen_for_step_agrees (for),
  gen_case_scan (PRE scan verbatim), gen_decision_mismatch (E3010 once per (prefix, kind, span)),
  forward_reference / enclosing_levels / find_construct / bare_names (message only).
- generate.rs: Case uses gen_case_scan + check; elaborate_gen_if checks each chain link; For records the iterated
  genvar values in Nets only when the loop ends on a false condition; later walks verify each step and replay
  the record after the first mismatch (init / cond / step / stall unknown with a record = mismatch).
- lib.rs fields gen_decided: BTreeMap<String, BTreeMap<(GenKind,u32,u32), GenDecision>>, gen_decision_reported;
  driver.rs init.
- Key/CU: one prefix = one instance = one module definition = one CU after the staged merge (cli/src/staged.rs
  §4 merge emits each NAMED item from one CU); the module AST for the message is map[cur_module], searched by
  (kind, span) — a miss gives the fallback sentence.

## Step 1 check — 149 cells, debug POST ($S/s592/postd/vita md5 3745136565b4772d645544a7c498a8f1)
model_cmp IEEE: same 97 · UP ACCEPT->LOUDok 27 · UP WRONG->LOUD 18 · SIDE LOUDok->LOUDok 5 · SIDE LOUD->LOUD 2 (F1 Z1g) · DOWN 0
model_cmp SVVL: same 97 · UP WRONG->LOUD 35 · DOWN OK->LOUD 7 (F3 I4 M4 V04 V24s Z1f Z5m) · SIDE 6 · n/a 4
= probe_d1 tallies exactly; value/rc per cell = probe_d1 on 149/149.
F1, Z1g: errors=1 each. Byte-identical PRE vs POST 94/149; 3 same-verdict cells gain a 2nd error after PRE's
first (M3_genchain_sh, Z2a_armerr_sh, Z2b_armerr_sh_dup).

## Step 2 — tests (cargo fmt --all applied)
- new crates/cli/tests/generate_decision_walks.rs: 10 refused pins (exact sorted error lines, exit 1, no `@` line):
  V04x (`S`, this generate block), V08 (`K`, an enclosing generate block — extra pin for the scope phrase),
  I4 (`K`, branches), F3 (`N`, iterations), F1 (one error, no cascade), M4 (fallback sentence), Z0c (`X`),
  V24 (`K`, this module), V10e (u[0], u[1]; v no error), Z1g (one error); 5 controls (exit 0, exact `@` lines,
  no error): V05a, E06, V12c, Z5f, V14. Raw iverilog / sv2v / verilator lines in each doc comment.
- generate_case_and_wildcard_prerequisites.rs: `p2_a_forward_referenced_matching_label_mixes_arms` renamed
  `p2_a_forward_referenced_matching_label_is_refused` (exit 1, one E3010 naming `K`, §26.3, no `D1P` line);
  header P2 bullet rewritten (closed by §4.5.592; the positional value is its own row).
- refactor after first pass: ForRecord bundles (record, span, inputs); generate.rs 1045 -> 1084 lines.
- `cargo nextest run -p cli --locked --no-fail-fast --test generate_decision_walks --test
  generate_case_and_wildcard_prerequisites`: Summary 27 tests run: 27 passed.
- 149 cells re-run on refactored debug (postd md5 76b2b13078ecad068ec05bdd9c7b1b84): identical tallies.

## Step 2b — two more pins (17 in the new file)
- the_message_names_the_forward_name_not_an_earlier_one (Z6a, c2/): iverilog `@one`; sv2v `@hundred`; verilator
  `@hundred`; PRE `@hundred`; POST refuses naming `B` (not the inner-earlier `A`). Kills the scope-rule mutant.
- a_first_walk_that_reports_records_nothing (F2): PRE's 7 errors kept (unfoldable bound + 6 undeclared-net
  cascade); iverilog refuses; sv2v/verilator L0..L2. Gives `ended` teeth.

## Step 3 — scoped gate
- `cargo nextest run -p cli --locked --no-fail-fast`: Summary [24.941s] 8108 tests run: 8108 passed, 1 skipped
  (before the 2 extra pins; new file then 17/17 pass).
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: rc 0. `cargo fmt --all -- --check`: rc 0.

## Step 4 — frozen POST ($S/s592/post_a)
- vita 881bf309c47b1a7178b2ed3a26c28a0c (release, 7437488 B); sep/ (--features separate-bins): vita
  15cca466aecb5d95e210da05d43b3090 vcmp e95ff613e23008a86dcee498341dfaea velab a04163f66c4d953821e112cd0493c3cc
  vrun 0fef675b76294a3443350d6c5cda9589.
- 149 cells one-shot: IEEE same 97 · UP 45 (27 ACCEPT->LOUDok + 18 WRONG->LOUD) · SIDE 7 · DOWN 0;
  SVVL same 97 · UP 35 · DOWN OK->LOUD 7 (F3 I4 M4 V04 V24s Z1f Z5m) · SIDE 6 · n/a 4. F1, Z1g errors=1.
  release output == debug output on 149/149. PRE vs POST byte-identical 94/149.
- staged (staged2.py): PRE 149/149 staged == one-shot; POST 149/149 staged == one-shot. Z6a staged = one-shot.
- corpus+examples .vu/.velab (corpus_velab.py): 30/30 byte-identical PRE vs POST.
- extra probes c2/Z7a-d (inner `localparam real S`), Z8a-d (inner 72-bit S) for the "later walk cannot fold
  where Nets did" lane: PRE = POST = iverilog on all 8 (the inner real/wide S never enters the int map, so every
  walk reads the outer S); that lane was not reached by any probe.

## Step 4b — corpus + elab A/B
- `cargo run -p corpus-runner --locked -- run` in wt (bench rsynced, wt/target/release/vita = post_a/vita): rc 0;
  10 `ok` + verilog-axi `ruled-split` (ROADMAP §2-N, pre-existing ruled split); 0 REGRESSION/DRIFTED/ORACLE-DRIFT;
  coverage 11/11.
- elab A/B (elab_ab.py: `vita velab <row>.vu`, PRE=A POST=B, interleaved, order alternating per rep, rep 0
  discarded, n=30): geomean B/A 0.9977; rows 0.969..1.034; loadavg start 2.70 end 4.81. serv 1.034 (both orders
  >1.03) re-measured n=80: PRE→POST 0.996; reversed (A=POST) PRE/POST 1.042 with the per-order ratios 1.062 /
  0.984 (sign flips) = noise. Verdict: no change (±3%).

## Step 6 — mutant battery: expected outcomes (written before running)
narrow = `cargo nextest run -p cli --locked --no-fail-fast --test generate_decision_walks --test
generate_case_and_wildcard_prerequisites`; survivors re-run `--workspace`.
- M1 no Nets record → every refused pin dies (12: 10 + Z6a + d1p).
- M2 mismatch never reported (replay only = cache) → every refused pin dies (exit 0).
- M3 report but keep the later decision (check returns `now`; for step reports and agrees) → F1, Z1g die
  (cascade); case/if pins and F3 still one error → survive those.
- M4 key without prefix → E06 control dies (iterations collide).
- M5 if-check skipped → I4 dies.
- M6 for record never consulted → F3, F1, Z1g die.
- M7 case/if compare Some/None only → the 9 case/if refused pins die.
- M8 no dedupe → every refused pin dies (3 errors).
- M9 case/if recorded in VarInit not Nets → the 9 case/if refused pins die.
- M10 for records on a diagnosed exit → F2 control dies.
- M11 scope rule `break`→`continue` (message) → Z6a dies.
- M12 init unfoldable in a later walk with a record → PRE return (no report) → SURVIVES (lane unreached by any probe).
- M13 replay also evaluates the step → SURVIVES (equivalent: replay rebinds the recorded value at loop top; a
  step mismatch is already deduped).

## Step 6 — mutant battery: results (narrow; 29 tests = 17 new + 12 prerequisite file)
| mutant | result | killers |
|---|---|---|
| M1 no Nets record | KILLED | 12 refused pins (all 10 + Z6a + d1p) |
| M2 never report (cache) | KILLED | same 12 |
| M3 report, keep later decision | KILLED | F1, Z1g (cascade) |
| M4 key without prefix | KILLED | E06 control + `p1_a_genvar_under_a_wide_constant` |
| M5 if unchecked | KILLED | I4 |
| M6 for record unused | KILLED | F3, F1, Z1g |
| M7 Some/None compare | KILLED | 9 case/if refused pins |
| M8 no dedupe | KILLED | 12 refused pins |
| M9 case/if record in VarInit | KILLED | 8 (expected 9: the M4 chain moves in every walk, so a VarInit record still mismatches in Logic with the same one fallback line) |
| M10 record on diagnosed exit | KILLED | F2 control |
| M11 scope rule break->continue | KILLED | Z6a |
| M12 later-walk unfoldable init with record -> PRE | SURVIVED (expected) | — |
| M13 replay also evaluates the step | SURVIVED (expected, equivalent) | — |
tree: STATUS-SAME before/after; restore verified by cmp.
- workspace re-run of survivors (`cargo nextest run --workspace --locked --no-fail-fast`): M12 SURVIVED (Summary
  [67.007s] 9134 tests run: 9134 passed, 15 skipped); M13 SURVIVED (9134 passed). STATUS-SAME.
- M12 reachability probes c2/Z9a (`localparam S = $random`), Z9b (`= Nope`): PRE = POST (one E3009 at the inner
  declaration; the outer S stays bound). The arm is kept and documented on `ForStep::Unknown` as measured
  unreached (ER §7.4 defensive arm).

## Final POST = $S/s592/post_b (post_a + a comment-only edit on ForStep::Unknown; 82 bytes differ = panic-location
line numbers of gen_decision.rs)
- vita 020de73fcf94cad012aff0f7a3a7db62 (release 7437488 B); sep/ vita 7679961ad9f53b7e19b5e793ecf9a143 vcmp
  bdcef7174cc497b7e35c9912ebe3fe5f velab 274d80fa6b04db9b15411aa384e8344c vrun ffcdcbf6cb8107fb07cde5907dc808c5.
- 149 cells: post_b output == post_a output on 149/149; staged == one-shot 149/149; corpus 30/30 == PRE.
- elab A/B PRE vs post_b n=20 under loadavg 9.8-16.7 (another session busy): geomean 1.010; large rows ibex
  1.001, verilog-axi 1.005, biriscv 1.005, picorv32 0.991; darkriscv 1.092 (per order 0.861/1.193) and keccak
  1.040 (0.839/1.254) flip sign = noise. The low-load post_a run (geomean 0.998) is the measurement of record.
- non-vacuity (instrumented debug build of the final source, GDW_LOG; source restored, cmp-verified):
  corpus + examples (vcmp+velab): 343 REC, 810 OK (case/if verify), 1152 FOROK (for-step verify), 0 MIS;
  149 cells: 266 REC, 607 OK, 96 FOROK, 12 FORMIS, 170 MIS (mismatch entries before dedupe).

## Final gate on the final source (post_b)
- `cargo nextest run --workspace --locked --no-fail-fast`: Summary [50.836s] 9134 tests run: 9134 passed, 15 skipped.
- `cargo test --doc --workspace --locked`: rc 0 (17 crates, 0 doctests).
- `cargo fmt --all -- --check`: rc 0. clippy (workspace, all targets, -D warnings) rc 0 before the comment edit.
- corpus-runner ran on post_a; post_b's corpus .vu/.velab are byte-identical to PRE (30/30), so not re-run.

## Deliverables
- diff: $S/s592/post_b/wt.diff = $S/s592/post_a/wt.diff, md5 eef035d60206cdaa334eab13c9e41d06 (new files `git add -N`).
- post_a binary = this diff minus the 6-line ForStep::Unknown comment (mut/orig vs mut/final gen_decision.rs).

## Deviations
- d1p pin renamed `p2_a_forward_referenced_matching_label_mixes_arms` -> `..._is_refused` (no doc cites it).
- 3 pins beyond the plan: V08 (scope phrase), Z6a (message scope rule), F2 (no-record lane / `ended`).
- message wording: "declare it above the generate {case|if|for}" (plan: "generate construct"); fallback "declare
  that parameter above the generate {kind}"; scope phrase varies: this generate block / an enclosing generate
  block / this module (module-level region).
- a later walk that cannot fold where the Nets walk did counts as a mismatch (plan's "different => report");
  measured unreached (Z7a-d, Z8a-d, Z9a-b, 149 cells), M12 survivor.
- key map nested per prefix (BTreeMap<String, BTreeMap<(kind,lo,hi),_>>): no prefix clone on lookup.
- final POST is post_b (comment-only rebuild), post_a kept frozen.
- generate.rs 1045 -> 1084 lines (already over the ~1000 guideline before the slice).
status: completed

# Round-2 delta (coordinator: r1 differential FAIL F1-F5, soundness PASS 1 MAJOR + 3 MINOR)
status: in progress

## Rule implemented (r2 item 1)
- Record = (decision, first case label the Nets walk could not read before its match). Later-walk mismatch: take
  the record; refuse (E3010, once per construct instance) only when that label exists; otherwise silent.
- Census of "read" (gen_decision.rs module doc): case = scrutinee folds + every label the scan evaluates up to its
  match folds (const_eval or the string path); labels after the match are not evaluated and cannot change a
  first-match decision. if = condition folds (an unfoldable one is reported, recorded nowhere). for = init, every
  condition, every step fold and the loop ends on cond false (zero iterations included, generate.rs `ended`); the
  stall guard (generate.rs stall arm), unroll cap and unfoldable exits report and record nothing in the Nets walk.
  So only a case record can carry an unread label; if/for mismatches are always silent replays.
- Later-walk Unknown (init / cond / step unfoldable, stall guard) with a record = mismatch -> replay (DM07b init,
  STALL1 stall).
- For mismatch no longer reports at all (ForRecord::agrees is pure); If check ignores the refuse slot.

## Messages (r2 items 2, 4)
- advice clause removed; message = "`K` is used before it is declared in {scope} (declared at [file:]L:C), so vita's
  elaboration passes choose different case items here; IEEE 1800-2017 §26.3 binds a name only to declarations before
  the reference"; file printed when the declaration's file differs from the use's (DM10c: `declared at
  DM10c_inc.svh:1:24`); fallback "this generate case depends on a parameter declared after it, …" (FB1).
- name choice: names of the unread label first, then the case's; among forward names the first with no earlier
  declaration in an outer level or the module's own parameters wins (MSG2 names `K`, not `A`).
- ForStep::Unknown comment rewritten to what the code does (DM07b init, STALL1 stall reach it).
- simplification after the first r2 build: the record carries `read_all: bool` (no label span; the label-first
  name priority was redundant with the "no earlier declaration" preference: a label read before the unread one
  folded, so its forward names had earlier declarations). Outputs identical to the first r2 build on 149 + 85
  lens cells.

## Pins (generate_decision_walks.rs, 28 tests; prerequisite file 12; 40/40 pass)
- silent, = iverilog: V04x `@one 1 bits=6`, I4 `@else`, Z6a `@one`, Z0c `@pkg 9 bits=4`, F3 `@L0`, F1 `@L0 w=10`,
  DL08 `@bits=4`, F6 `@top.b.g[1] @top.b.g[2]`, F15 (no output, rc 0), STALL1 `g[0..2] w=0..2`, DM07b `@L1 @L501`,
  DM07c `@L0..@L2` (step), DM07d `@L0..@L2` (cond).
- KNOWN-WRONG (arm = iverilog's, a read inside it is not): M4 `@seven P=5` (iv `@seven P=7`; Scoping reads row),
  Z1g `@L0 w=10 P=0000` (iv `P=xxxx`; 🆕 AE).
- refused: V08, V24, V10e, MSG2 (names `K` not `A`), MSG3 (skips inner-earlier `A`), DM10c (`declared at
  inc.svh:1:24`), FB1 (fallback, after PRE's E3009), d1p (prerequisite file).
- controls unchanged: V05a E06 V12c Z5f V14 F2.

## Frozen POST round 2 = $S/s592/post_c
vita 1ce7d9540a91385f8ee589e6695cc5a2 (release, 7437488 B); sep/ vita 8bd58523442ed9ed288a9b58361f18da vcmp
26dcbf7a251284055ca49d3e91c29b7c velab 457bd68786340d436510129a626940ff vrun 29f40494eeebaffa635c1d02356ad21f.
Gate on that source: `cargo nextest run -p cli --locked --no-fail-fast` 8119 passed 1 skipped; workspace clippy
-D warnings rc 0; fmt rc 0. (DM07c/DM07d pins added afterwards, test-only.)

## Round-2 mutants: expected (written before running)
R1 always refuse on a case mismatch -> V04x Z6a Z0c die · R2 always silent -> V08 V24 V10e MSG2 MSG3 DM10c FB1 d1p
die · R3 unread label never noticed -> same as R2 · R4 file never cited -> DM10c · R5 no earlier preference -> MSG2 ·
M1 no record -> silent + refused pins · M3 keep later decision -> V04x I4 Z6a Z0c M4 · M4 prefix-less key -> E06
(+p1 genvar) · M5 if unchecked -> I4 · M6 for record unused -> F3 F1 F6 F15 STALL1 DM07b/c/d Z1g · M7 Some/None ->
case/if silent + refused pins · M8 no dedupe -> refused pins · M9 case/if record in VarInit -> case/if pins · M10 ->
F2 · M11 break->continue -> MSG3 · M12 init Unknown -> old exit -> DM07b · M13 equivalent -> SURVIVES · MS6 -> F6 ·
MS15 -> F15 · MSTALL -> STALL1 · MSTEP -> DM07c · MCOND -> DM07d · MUNK (Unknown agrees) -> DM07b/c/d STALL1.

## Round-2 delta tables (post_c, release; one-shot; staged == one-shot 149/149, 40/40, 45/45)
149 cells, post_b -> post_c (IEEE): same 130 · UP LOUD->OK 13 (E09 F1 F3 I2 I3 I4 V04 V04x V05c V05d Z0c Z1f Z5h) ·
  DOWN LOUD->WRONG 5 (M4 `seven P=5` iv `seven P=7`; V06n `def 9 bits=4 P=3` iv `P=5`; Z1g `L0 w=10 P=0000` iv
  `P=xxxx`; Z5m `k f=8` iv `k f=4`; Z5n `L0 one 9 bits=4 P=10 | L1 other 200 bits=8 P=11` iv `P=1 | P=2`) ·
  DOWN LOUDok->ACCEPT 1 (V24s `k`; iv "'K' has already been imported").
149 cells, PRE -> post_c (IEEE): same 97 · UP 41 (26 ACCEPT->LOUDok, 12 WRONG->OK, 2 WRONG->LOUD V06 W1, 1 LOUD->OK
  F1) · SIDE 10 (5 LOUDok->LOUDok; WRONG->WRONG M4 V06n Z5m Z5n; ACCEPT->ACCEPT V24s) · DOWN 1 (Z1g LOUD->WRONG).
differential lens 40 cells: post_b -> post_c UP LOUD->OK 12 (DI05m DL01 DL03 DL06 DL08 DL11 DM07 DM07b DM08 DM09 DM10
  DX03), same 28; PRE -> post_c UP WRONG->OK 7, same 33, DOWN 0.
soundness lens 45 cells: post_b -> post_c UP LOUD->OK 10 (F15 F3i F6 M12c M12e MSG1 NEG1 STALL1 U8 U8e), DOWN
  LOUDok->ACCEPT 1 (B1_bind_fwd: iverilog/sv2v cannot parse `bind`, so not an oracle; by hand-IEEE outer K=4 ->
  arm K, post_c `w=1 bits=4`); PRE -> post_c UP 9, SIDE 1 (B1), n/a 1 (NEG1: iv g[0..2] = post_c).
extra c2 cells: DM07c `@L0..@L2` (PRE `@L0`), DM07d `@L0..@L2` (PRE none), Z6a `@one` (PRE `@hundred`), DM10c / MSG2
  / MSG3 refused (PRE `@inner` / `@x` / `@x`), FB1 E3009 + E3010 (PRE E3009); Z7a-d, Z8a-d, Z9a-b = PRE.
corpus + examples .vu/.velab 30/30 byte-identical to PRE; corpus-runner rc 0, 10 ok + verilog-axi ruled-split,
  0 REGRESSION/DRIFTED/ORACLE-DRIFT, coverage 11/11.

## FINDING (coordinator's expectation "0 down" not met)
The 6 post_b->post_c descents share one property: the arm/iterations chosen are now iverilog's, but something
inside the chosen arm still reads a later declaration in a later walk — the pre-existing "Scoping" reads row
(M4, V06n, Z5m, Z5n: a body read / function body / body localparam chain sees the later binding; PRE printed the
same wrong read with the wrong arm) or 🆕 AE (Z1g: interpreter x read as 0; PRE's loud was the undeclared-net
cascade of the extra iterations, an accident). V24s: import + later region `K` — iverilog refuses the
redeclaration; vita accepts it (reject-vs-accept, as in PRE). PRE -> post_c: Z1g is the one ladder descent
(LOUD -> WRONG); M4 V06n Z5m Z5n are WRONG -> WRONG with the arm now right.

## Round-2 mutants: results (narrow, 40 tests)
all KILLED except M13 (expected equivalent). R1 4 (V04x Z6a Z0c M4 — the case pins) · R2 8 · R3 8 (V08 V24 V10e MSG2
MSG3 DM10c FB1 d1p) · R4 DM10c · R5 MSG2 · M1 22 · M3 5 (V04x I4 Z6a Z0c M4) · M4 E06 + p1 genvar · M5 I4 · M6 9 (all
for pins) · M7 13 · M8 8 · M9 13 · M10 F2 · M11 MSG3 · M12 DM07b · MS6 F6 · MS15 F15 · MSTALL STALL1 · MSTEP DM07c ·
MCOND DM07d · MUNK 4 (STALL1 DM07b DM07c DM07d). STATUS-SAME, restore cmp-verified.

## r2 item 5 (no code)
DO02 (`sub #(.P(K))` reading a later block `K`): iv `@one top.g.u.b bits=4`; sv2v / verilator / PRE / post_c `@two
top.g.u.a bits=8` — an override read of a later declaration, consistent in every walk, so D1 cannot see it; a sink
for the docs' "Scoping" reads row.
- M13 at `--workspace`: SURVIVED (Summary [39.575s] 9145 tests run: 9145 passed, 15 skipped) = equivalent (replay
  rebinds the recorded value at the loop top). STATUS-SAME.

## Round-2 close
- final source: workspace clippy -D warnings rc 0, fmt rc 0 (after the DM07c/DM07d test-only pins).
- diff: $S/s592/post_c/wt.diff md5 1b2678dc952bb3ef81a5af1873e6d870 (6 files, +1564 -73; new files `git add -N`).
- generate.rs 1045 -> 1088 lines; gen_decision.rs 533.
- disk 25 GiB free.
status: completed (round 2)

# Round-3 delta (coordinator: Z1g LOUD->WRONG vs PRE not acceptable; refuse unless the mismatch builds nothing)
status: in progress

## Rule
Later-walk mismatch (case / if / for): take the record (no cascade) and refuse once per construct instance (E3010,
round-1 behaviour), UNLESS neither the recorded arm nor the walk's own arm (for: the loop body, the same for both
lists) holds an item the walk builds — then take the record silently (PRE's output: DL01/03/06/08/11). The read_all
flag of round 2 is gone (with processes the d1p class is refused by this rule; a net-only d1p arm is PRE-equal).

## Census: what each later walk builds (generate.rs `lower_gen_module_item` + `elaborate_gen_item`)
| GenItem / ModuleItem | VarInit | Logic | Instances | Nets only / binding only |
|---|---|---|---|---|
| NetVar, variable or scalar string kind, with an initializer | collect_var_init_drivers | — | — | net created in Nets |
| NetVar, net kind, with an initializer | — | elaborate_net_init_drivers (implicit assign) | — | net created in Nets |
| NetVar without initializer | — | — | — | Nets only |
| ContAssign | — | elaborate_cont_assign | — | |
| Proc | — (its block-local inits were queued in Nets; the proc itself is Logic's) | lower_user_proc / try_elab_task | — | block-local hoist in Nets |
| Instance | — | — | elaborate_child_instances | |
| Param, carried Typedef enum | bind | bind | bind | every walk binds; builds nothing |
| Func, Task, PortDecl, Defparam, Import | — | — | — | Nets only (register / loud) |
| Genvar, Modport, SVA decls, Covergroup, Class, Let, Clocking, … | — | — | — | `_ => {}` in every walk |
| nested Generate region, For, If, Case, Block | through every body (positive set, conservative) | same | same | |
Implemented as `gen_builds_in(phase, items)` / `module_item_builds_in` in gen_decision.rs (positive set per walk;
precise mirror of collect_var_init_drivers / elaborate_net_init_drivers kind guards).

## Pins (generate_decision_walks.rs 40 tests + prerequisite file 12 = 52/52 pass)
re-pinned refused (each checked against PRE: PRE wrong or PRE loud): V04x (PRE `k 8 bits=6`), I4 (`then`), Z6a
(`hundred`; names `B`), Z0c (`local 8 bits=4`), F3 (`L0 L1 L2`), F1 (PRE loud cascade), F6 (`g[2] g[3]`), F15
(`g[0] g[1]`), STALL1 (`g[0]`), DM07b (none), DM07c (`L0`), DM07d (none), M4 (`five P=5`, fallback message), Z1g (PRE
loud cascade). new refused per kind: KA assign (PRE `o=200`, iv `o=9`), KN net init (PRE `w=8 bits=4`, iv `w=9
bits=4`), KV variable init (PRE `v=8 bits=4`, iv `v=9 bits=4`), KI instance (PRE `P=200`, iv `P=9`); nested
constructs NXF NXI NXC NXB NXG (PRE `@a`, iv `@b`). kept refused: V08 V24 V10e MSG2 MSG3 DM10c FB1 d1p, F2.
silent (= PRE = iverilog): DL08 `@bits=4`, DL01 `@bits=4`, DL03 `@Q=10`, DL06 `@bits=4`. controls unchanged.

## Mutants: expected (written before running)
B1 builds always true -> DL01 DL03 DL06 DL08 die · B2 always false -> every refused pin with a building arm dies ·
B3 drop Logic ContAssign -> KA · B4 drop Logic net-init NetVar -> KN · B5 drop Logic Proc -> I4 F3 NX* (and more) ·
B6 drop VarInit NetVar -> KV · B7 drop Instances Instance -> KI · B8 Logic NetVar regardless of init -> DL01 DL08 ·
B9 VarInit NetVar regardless of init -> DL06 · B10 Param counts -> DL03 · B11 For not recursed -> NXF · B12 If not
recursed -> NXI · B13 Case not recursed -> NXC · B14 Block not recursed -> NXB · B15 Generate region not recursed ->
NXG · B16 case: only the recorded arm checked (own arm ignored) -> ? (expected SURVIVE: every pinned case mismatch has
a building arm on the recorded side too) · B17 for: report without the builds test -> DL08 · plus the round-2 set
re-run on the new code (M1 M3 M4 M5 M6 M7 M8 M9 M10 M11 M12 M13 MS6 MS15 MSTALL MSTEP MCOND MUNK R4 R5).

## Frozen POST round 3 = $S/s592/post_d
vita 5119bb026cab5cde3875b0c7b2208722 (release, 7437488 B); sep/ vita 3f004355972c329791fe780a401eef1b vcmp
cba526d6af23a8c765cc1fdffa3721f6 velab 456b74400eab558253d6a3c15bda064a vrun a10b5fd37b2b8c5b049d7bd93315b637.
Gate on that source: `cargo nextest run -p cli --locked --no-fail-fast` 8133 passed 1 skipped; workspace clippy -D
warnings rc 0; fmt rc 0. (ONE / TWO pins added afterwards, test-only; pin files 54/54.)

## Round-3 delta (IEEE model; one-shot; staged == one-shot 149/149, 40/40, 45/45, 28/28)
| cell set | PRE -> post_d | post_c -> post_d |
|---|---|---|
| 149 grounding | same 97 · UP 45 (27 ACCEPT->LOUDok, 18 WRONG->LOUD) · SIDE 7 (5 LOUDok->LOUDok, 2 LOUD->LOUD) · DOWN 0 | same 130 · DOWN OK->LOUD 13 (E09 F1 F3 I2 I3 I4 V04 V04x V05c V05d Z0c Z1f Z5h) · UP WRONG->LOUD 5 (M4 V06n Z1g Z5m Z5n) · UP ACCEPT->LOUDok 1 (V24s) |
| differential lens 40 | same 33 (incl. DL01 DL03 DL06 DL08 DL11 = PRE = iv) · UP WRONG->LOUD 7 (DI05m DM07 DM07b DM08 DM09 DM10 DX03) · DOWN 0 | same 33 · DOWN OK->LOUD 7 (same 7) |
| soundness lens 45 | same 34 · UP WRONG->LOUD 7 · SIDE LOUD->LOUD 2 (M12e U8e) · UP ACCEPT->LOUDok 1 (B1; iv cannot parse bind) · n/a 1 (NEG1: PRE 200-error cascade, post_d 1 error) · DOWN 0 | same 34 · DOWN OK->LOUD 10 · UP 1 (B1) |
| new cells 28 | same 11 (FB1 +1 error; Z7a-d Z8a-d Z9a-b) · UP WRONG->LOUD 14 (DM07c DM07d KA KI KN KV NXB NXC NXF NXG NXI ONE TWO Z6a) · UP ACCEPT->LOUDok 3 (DM10c MSG2 MSG3) · DOWN 0 | same 14 · DOWN OK->LOUD 14 |
post_b -> post_d: 149 same (verdict class), sound lens 45 same, diff lens UP LOUD->OK 5 (DL01 DL03 DL06 DL08 DL11).
Every PRE -> post_d sideways (raw first error; PRE cascade -> one E3010):
- AE7: PRE `undeclared net/variable `top.gb[0].gk[0].w`` (errors=4) -> `` `K` is used before it is declared in this
  generate block (declared at 9:28) `` (errors=1); iv "Unable to evaluate parameter Q value: top.fx(32'sd2)".
- V14b, V17: PRE `undeclared net/variable` (errors=4) -> `` `K` is used before … `` (errors=1); iv "Unable to bind
  parameter `K'".
- Z1d, Z1e: PRE `undeclared net/variable `top.gb[0].gk[0].o`` (errors=2) -> `` `K` is used before … `` (errors=1).
- F1, Z1g: PRE `undeclared net/variable `top.gb[0].L[1].w`` (errors=4) -> `` `N` is used before … (declared at
  8:16 / 10:16) `` (errors=1); iv `@L0 w=10` / `@L0 w=10 P=xxxx`.
- M12e: PRE `undeclared net/variable `top.b[0].g[-1].w`` (errors=4) -> `this generate for depends on a parameter
  declared after it, …` (errors=1); U8e: same PRE -> `` `A` is used before … (declared at 8:23) ``; iv `g[1] g[2]`.
The post_c -> post_d downs are the silent-legal cells of round 2 going back to round-1 refusals (PRE was wrong on
every one; see the PRE column).
- corpus + examples .vu/.velab 30/30 byte-identical to PRE; corpus-runner (wt/target/release/vita = post_d) rc 0,
  10 ok + verilog-axi ruled-split, 0 REGRESSION/DRIFTED/ORACLE-DRIFT, coverage 11/11 (loadavg 18-47 from another
  session during this round; no timing measured).

## Round-3 mutants: results (narrow, 54 tests; then 56 after two pins)
the builds test: B1 always true KILLED (DL01 DL03 DL06 DL08) · B2 always false KILLED (33) · one kind dropped: B3
ContAssign KILLED (KA) · B4 Logic net-init KILLED (KN) · B5 Logic Proc KILLED (20) · B6 VarInit var-init KILLED
(KV) · B7 Instance KILLED (KI) · B8 Logic NetVar regardless of init KILLED (DL01 DL08) · B9 VarInit regardless of
init KILLED (DL06) · B10 Param counts KILLED (DL03) · recursion: B11 For (NXF) · B12 If (NXI) · B13 Case (NXC) · B14
Block KILLED (14; a named case arm body is a Block) · B15 Generate region (NXG) · B16 own arm ignored KILLED (ONE) ·
B17 for always reports KILLED (DL08) · B18 recorded arm ignored KILLED (TWO) · B19 for never reports KILLED (9).
carried: M1 M4 M5 M6 M7 M9 M10 M11 M12 MS6 MS15 MSTALL MSTEP MCOND MUNK R4 R5 KILLED. M3 and M8 SURVIVED the first
run (every refused pin had one building walk and same-labelled arms) -> pins V17 (different item labels: M3 would
cascade) and KVP (a variable initializer + a process: two walks report under M8) added; re-run: M3 KILLED (V17), M8
KILLED (KVP). M13 SURVIVED (equivalent).
- M13 at `--workspace`: SURVIVED (Summary [51.654s] 9161 tests run: 9161 passed, 15 skipped). STATUS-SAME.

## Round-3 close
- final source: workspace clippy -D warnings rc 0, fmt rc 0 (after the test-only ONE/TWO/V17/KVP pins); pin files
  56/56; the full workspace suite passed 9161/9161 under the M13 mutant (equivalent).
status: completed (round 3)
