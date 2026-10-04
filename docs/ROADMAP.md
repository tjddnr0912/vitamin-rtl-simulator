# ROADMAP — open work

Open work only, one line per item: `id — symptom (headline cell); code site; fix shape; oracles; STATUS`. Grounding re-measures every claim (LOOPROMPT §1), so a line keeps only what a re-measurement cannot recover: the id, the code site, a held design's branch or commit, a named prerequisite.
A finished slice's record is its commit message, anchored `§4.5.N` (bug, mechanism, byte-identity, review): find it with `git log --grep '§4.5.N'`; entries up to §4.5.582 are in `git show acabe991:docs/history/ROADMAP_ARCHIVE.md`. Done = delete the closed line(s), add each new find as one line in its section, recount the Summary.
Section numbers and row ids are stable, never renumbered or reused; source comments cite them (`ROADMAP §2 🆕 I ⓐ`, `§3 ⑭`, `§3.b <slug>`, `§5.1-be`). No gate or test counts in any doc: CI on 3 OSes is the record, and `format_version` is `crates/vita-artifact/src/header.rs:15`.
STATUS: OPEN = startable (two oracles, or one plus hand-IEEE, and no unmet prerequisite). Nothing else starts: BLOCKED (a named prerequisite, listed in [REMAINING_WORK.md](REMAINING_WORK.md) §D), WALL, SPLIT (oracle split, never chased), HELD (deliberate or trigger-gated), RECORD (observation), LOUD, PERF, DEFERRED, DO-NOT-START, DUP.

## Summary

Recount from the lines below in every docs step; the iteration report shows this table. `open` = the section's top-level lines (DUP not counted), `startable` = OPEN, `blocked` = the rest. `next` = where §5.2's start-order rows sit (`1` is the next slice); §5.2 orders rows counted in their own sections.

| § | track | open | startable | blocked | blocked by (top reasons) | rung | next |
|---|---|---:|---:|---:|---|---|---|
| §2 | silent-wrong start-order rows | 33 | 8 | 25 | named prerequisite 10 · oracle split 5 · loud, zero demand 3 · held 3 · performance 2 · do-not-start 2 | frozen; 🆕 AD, AB, U–W, T, S queued | 1, 2, 4, 5, 6, 7 |
| §2 | recorded defects by mechanism | 203 | 105 | 98 | oracle split / no oracle 56 · named prerequisite 19 · record only 8 · WALL 5 · held 5 · performance 4 · filed to §3 1 | frozen; two "Size cast" enum lines queued | 8 |
| §2-N | verilog-axi census | 5 | 0 | 5 | oracle split 2 · held on purpose 2 · upstream fst-writer 1 | ① | |
| §3.a | loud → correct-support, numbered | 24 | 19 | 5 | loud by design 2 · named prerequisite 2 · deferred to §5 1 | ② | |
| §3.b | loud → correct-support, small | 130 | 110 | 20 | named prerequisite 10 · held 4 · record only 2 · oracle split 2 · do-not-start 2 | ② | 3 |
| §3.c | intentionally loud | 12 | 0 | 12 | by design or oracle split 12 | — | |
| §0 | promotion queue (T2 residues) | 14 | 8 | 6 | oracle split 3 · deliberate 2 · `defparam` non-goal 1 | ③ | |
| §4 | SVA honest-loud | 6 | 0 | 6 | named prerequisite 6 | ③ | |
| §6 | G2 observability (OBS) | 6 stages + 10 | 15 | 1 | call tree: two lowering paths 1 | ④ | |
| study/03 | workload corpus | 1 | 1 | 0 | — | real-design | 9 |
| §5.b | performance / hardening | 16 | 7 | 9 | named prerequisite 5 · held or trigger-gated 4 | below the ladder | |
| §7 | conditional / long-term | 4 | 0 | 4 | trigger-gated 4 | trigger-gated | |
| §8 | non-goals | 2 | 0 | 2 | permanent 2 | permanent | |
| total | | 466 | 273 | 193 | | | |

## 0. correct-support promotion queue

Deliberately loud, not gaps: `new[]` on a fixed array; a multi-dim partial index `s[0]`; a cross-type SoA whole-element copy; a `real` scrutinee in a `generate case`.

### iverilog defects (vita is IEEE-correct) — oracle disqualifiers, regression-pinned

- ① `.len()` of a `string s[5]` element holding "abcdefg" — iverilog 5, vita 7
- ② concurrent fork activations share an `automatic` string array — iverilog `A!`, vita `A!!`
- ③ `$fmonitor` twice on one fd — iverilog accumulates, vita replaces per destination
- ④ `%s` of an empty string-array element — iverilog a blank, vita empty
- ⑤ `$clog2(4'sd7+4'sd1)` — iverilog 32, vita 3 (= verilator, §20.8.1)
- ⑥ `$itor(64'h1_0000_0008)` — iverilog 8, vita 4294967304 (= verilator)
- ⑦ `s<"ab"`, `s<"aa"`, `s<"zz"` with `s="ab"` — iverilog all 1, vita `0 0 1` (= verilator)
- ⑧ an `automatic time signed` frame local — iverilog aborts, vita `-4` (verilator alone)

### T2 residues (each its own slice)

- 8ⓐ — implicit real → `logic [R-1:0]` / `{R{1'b1}}` is loud; non-goal; SPLIT
- 8ⓑ — a real in an untyped `localparam` is loud; rounding is a withdrawn silent-wrong; HELD
- 8ⓒ — a real-valued override is loud (onto `real`, `int`, `logic`, `time` or untyped `P`); the override channel is i64; widen it to real, converting at a declared integral type; 2 oracles; OPEN
- 8ⓓ — `1.0/0.0` is loud; non-finite values refused on purpose; HELD
- 8ⓔ — `R<<1` is loud; non-goal; SPLIT (iverilog rejects, verilator 6)
- 8ⓖ — `$rtoi` in a constant-function body is loud; move to an environment-aware walk; 2 oracles; OPEN
- 8ⓗ — nested `int'(real'(R))` is loud; widen the explicit-conversion boundary; 2 oracles; OPEN
- 10ⓐ — the parser folds a `parameter` enum label before overrides (`const_locals`); move the enum-method desugar into elaborate; 2 oracles; OPEN
- 10ⓑ — a `localparam L = 8'h5` label does not fold (`const_locals` holds decimals, shared with generate indices), so enum methods are loud; 2 oracles; OPEN
- 11 — negative bound residue: `x[1:-2]` loud, ports warn and clamp; unsigned bound fold; 2 oracles; OPEN
- 14-a — `-pvalue+<name>=<val>` unimplemented (an alias of `-G`, argv only); OPEN
- 14-b — `-P<path>=<val>` unimplemented; BLOCKED (§8 `defparam` non-goal)
- 14-c — `.velab`s from different `-G` values share a header (false E9003 under `vrun --upstream`); a header field = format bump; OPEN
- 13 — `case (e) inside` where the sizing rules split (signed case expr + unsigned item; narrower signed item; narrow operator; fill item) stays E3009; `elaborate/src/case_inside.rs`; SPLIT

## 0-B. Small follow-ons (kept loud)

- `void'(getnext())` with an output formal; a frame-formal array forwarded into a nested hierarchical call; `8'(P*a)` over a param or call leaf.
- fork-in-frame: `fork_arms_self_contained`'s duplicate re-walk; an elaborate reject for a fork arm calling a forking task (F4004 guards it); zero-delay sibling visibility not differentially verified.

## 0-C. Large items — start decision table (do not re-estimate the size)

- A. file-position family (`$ftell`, `$fseek`, `$rewind`, `$ferror`) — a new `SysFuncId` = frozen-root bump, no sidecar route; payoff medium; `$feof` / `$fgetc` / `$ungetc` alone need no bump.
- B. shared literal-parsing crate — medium-large; payoff small (one two-predicate hazard); trap: `literal.rs` needs `sim_ir` (layering inversion), so split digit→bits from `ConstVal` packing.
- Order A > B.

## 1. Start priority — principle only (the live queue is §5.2)

① CRITICAL silent-wrong with an oracle (§2) > ② loud→supported with an oracle (§3) > ③ honest-loud promotion whose prerequisite holds (§0, §4, §5) > ④ G2 OBS (§6).
Performance is off the ladder. No oracle is not a reason to defer: build from the LRM, pin by hand. Inside a band a corpus witness goes first (`ibex` leads); a §2 row without one is frozen. Do not create a new queue here. T4: a function-local array element write is ~20× iverilog's cost.

## 2-N. Silent-wrong from the verilog-axi census

- 2-N-1 — verilog-axi `m_axi_*valid` is x in iverilog, 0 in vita briefly after reset (digest same); only vita raises a t0 event through a computed wire (`alias::copy_nets`, §4.5.533); SPLIT
- 2-N-2 — FST loses the `$dumpvars` snapshot; needs fst-writer's initial-value API; BLOCKED (upstream)

t0-event residues (held on purpose):

- `wire [3:0] w = r8` of `reg [7:0] r8 = 8'h12` wakes `@(w)` at t0 (iverilog not); HELD
- `wire w = 1'b1; reg r = w;`: iverilog `z`, verilator and vita `1` (§6.8); SPLIT
- `buf b1(o1, zin)` vita `x`, iverilog `z` (1364 §7.3; `oracle_split_rulings.rs`); HELD

## 2-R. Usability residue

- An unused package function is still framed; one cause is reported once per instance.
- Rule A's `Unknown` for a hierarchical callee spills E3001 onto a read-only net (the design is loud anyway).

## 2. Silent-wrong residues

Row ids are cited from source (`§2 row 7`, `§2 🆕 I`); a number is never reused.

FROZEN (owner direction): synthetic-probe rows stay out of §5.2. A row re-enters when a workload-corpus design hits it, when it lies in a corpus row's fix path, or when an external report reproduces it, and is then re-measured in full. Pre-existing finds outside a slice's fix path go to [PROBE_CATALOG.md](PROBE_CATALOG.md).

WALL(AST self-width): a tree-wide pass giving a node's self width without lowering it (today only inside a cast: `const_self_width` + `const_signed_env`).

### Start-order rows

- 🆕 B — ⓑ `case (b>>>2)` with an unsigned label: vita `eq236`, oracles `eq44`; `stmt_flow.rs` outer `$unsigned`; re-lower via `lower_size_ctx_entry(.., ext=false)`; BLOCKED (sign provenance vs a default: `expr_self_signed`)
- 3b — class-property bound normalisation has no home (`ClassField` is a heap slot); 1 oracle; BLOCKED (a field-key normalisation map)
- 7 — a parent `initial` reading a child net at t0: oracles `ee`, vita `xx` (hierarchy start order, wake-group order); SPLIT
- 10 — `wire [K[31:24]-1:0]` over a 128-bit `K` is 1 bit (oracles 221); `const_range_bound_fold`, `select_base_at_declared`; BLOCKED (declaring-scope fold, §4.5.560)
- 15 — an override's sized x/z literal loses its unknown plane (`8'b1010_010x` binds `10100100`); `params.rs` reads value bits only; BLOCKED (record 2-state-ness: `hdl-parser/src/params.rs` drops `var_kind`)
  - same plane: the narrow store's unknown plane (a declaration holding x is E3009); the >64-bit operator lane declines x/z; an x/z override as a constant event term runs at t0
- 16 — override cells where verilator sides with vita; >64-bit tops land on verilator's side, which its own `localparam` twin contradicts; SPLIT
- 17 — `#(.K(32'd0 - 32'd1))` onto 128-bit `K`: iverilog all ones, verilator 32 ones, vita 64 ones; SPLIT
- 19 — a 2-D / 3-D / packed element CA LHS is ~10× on both backends (= §5.b ARR-LHS); PERF
- 23 — `clocking cb; input a_b;` beside `cb_a; input b;` collides as `__clk_cb_a_b` (`sva_clocking.rs` mangling); LOUD (row 34)
- 24 — 24a a clocking output clobbered to x (`init_diag.rs::clocking_commit_plan`); 24b a one-cycle lag (§14.16); DO-NOT-START (row 34)
- 🆕 H — loud constant reductions: ⓑ `|P` over an ascending / lo≠0 parameter (`narrow_param_bits`); ⓔ after a const-function local assignment (`envw`); ⓕ a bitwise op over x/z (`const_wide.rs`: per-bit 4-state tables); ⓓ held; 2 oracles; OPEN
- 🆕 I — ⓐ a same-delta read of another process's write (`a5` vs `00`) held: a store-side forward breaks picorv32 / UDP / keccak parity; ⓒ same axis; ⓔ `bit` copy excluded; ⓕ interp / VM take a copy's extension sign from the slot, native from the node; ⓖ a mixed-caller callee keeps PRE; HELD
  - splits, never chased: runtime `m[k]`, negative-base index, 2-D word, genvar index, all-z driver, forced copy, array-word target, extending / truncating / concat copy, `v[7 -: 8]`, a task-only read
- 🆕 J — ⓓ `{'1, 1'b0}` illegal, loud: keep; ⓔ `v['1]` split (= iverilog); ⓕ fill widths split (= verilator); ⓖ `$bits(U + 4'd1)` over `U = '1` is 32 (`min_signed_bits(v).max(32)`, WALL); LOUD
- 🆕 M — ⓐ `#(.P('1 ^ 1'b0))` onto 40 bits: 32 vs iverilog 40, verilator 1 (`resolve_param_overrides`); ⓑ `(|'1)` loud; ⓔ `cover property (a |-> b)` loud (`cover.rs`); ⓕ verilator no oracle; BLOCKED (target-typed override evaluation)
- 🆕 N — scope naming: VCD types a generate block `module`; `genblk1` collisions; `%m` in a concurrent assert omits the label (`Stmt::ConcurrentAssert` has none); `C__8` vs `C__N8`; `--hier-tree` hides generate scopes; HELD (zero demand)
- 🆕 O — the eleven `bare_ident_route` bypasses are routed (`ident_route.rs`); left: a `foreach` shadow split and partial guards (`whole_name_net`, `packed_elem_resid`, `array_geom`); HELD
- 🆕 Q — a `localparam` in a procedural block is a parse error (both oracles accept); no block-scoped constant binding (`const_locals` is parse-time); BLOCKED (a block-scoped constant binding)
- 🆕 L — loud constant / import residues (sub-ids cited from source); LOUD
  - ⓑ string `$bits` 16 (= LRM): keep · ⓒ `Q * 2` over a real `Q` · ⓓ 2-state struct `'{…}` · ⓔ fill in `'{…}` · ⓕ string / real through `import p::*`
  - ⓖ struct-typed header override · ⓗ `p::v.a` E2002 · ⓙ `gather_local_decl_names` omissions · ⓚ generate-block `import` · ⓛ anonymous struct in `union packed`
  - ⓠ `F[i*4+3:i*4]` silent (illegal) · ⓡ block-local over a wildcard package variable read after its block · ⓣ duplicate named override accepted
  - ⓤ x/z write to a 2-state member of a 4-state struct keeps x/z (`StructFieldLayout.5`) · ⓦ package function over a non-i64 constant; `apply_import_const_funcs` · ⓧ late CU import applies · ⓨ generate bound reported once
  - ⓩ negative axis (zero demand): `A[0:-2]` (`const_bound_u32`), >64-bit negative-LSB base, `[m:l]` of a negative-LSB net (`packed.rs`) · (aa) untyped `C + D` i64 split; `$clog2(C+D)`; `byte` bound operand
- 🆕 S — (a) a constant `inside` / `==?` compares at the left width (`const_str.rs` `const_compare_special`, `const_wide.rs` `fold_region`); (b) a run-time x/z element compares with `==` (`wildcard_eq.rs` `inside_value_cmp`); BLOCKED ((a) 🆕 W, T, U · (b) a run-time `==?` primitive)
  - retry (§5.2 row 7): 🆕 W, then the i64 half (`eb9d3b69`, branch `fix/gencase-label-domain`) with every `const_wildcard_i64` decline falling back to PRE's compare; re-run round-3 cells, MC4, the 174-cell matrix, §4.5.580's harness; the wide half (cells LP, st1) after 🆕 T, U
- 🆕 T — a generate-case label the i64 fold cannot read is skipped; a read one compares untyped (T1, L06, S06, X13 take `default`; N09 inconsistent); `generate.rs` `GenItem::Case`; BLOCKED (🆕 U, 🆕 V)
  - design (§4.5.581, `fix/gencase-label-domain`, `d4dc9c26`): decide once per instance; compare PAIR and WHOLE (§12.5) in the bit domain, decide where they agree, else i64, else no match, never refuse; residue: a label no bit fold reads (`case (6'sh30) W6'(P8)`); held `generate_case_and_wildcard_prerequisites.rs`
- 🆕 U — a key holds two bindings in different maps and lanes read different ones (A1D genvar prints 2^64+9, B2D, q1g); writers `bind_param_value`, `package.rs` import arms; readers `wide_name_bits`, `lookup_scoped`; 3 oracles; OPEN (§5.2 row 4)
- 🆕 V — a generate-case arm is re-decided per `GenPhase` walk (`instance.rs`), so a forward label mixes arms (d1p `k 8 bits=4`, oracles `k 200 bits=8`); decide once (key: scope prefix + span; U1 kills a span-only key) and resolve labels alike per phase; 2 oracles; OPEN (§5.2 row 5)
- 🆕 W — a constant typed by an overridden type parameter reads unsigned (PT5c `PV < 0` → 0, dF3 `TP = '1` → 15); measure §4.5.479 / §4.5.483's pins first; 3 oracles; OPEN (§5.2 row 6)
- 🆕 X — a class method's `case` re-evaluates a call scrutinee per label (f2); `class_lower.rs` / `frames_reserve.rs` reserve no case temp; reserve them, then let `lower_case_inside` take it (pin `case_inside_refused.rs` `l14_…`); 2 oracles; OPEN
- 🆕 Y — a string-returning call scrutinee compares packed (s2 `m=0`, verilator 3); `ir_expr_is_string` (`strings.rs`), `hoist_case_scrutinee`; a string-kind capture (pin `case_inside_refused.rs` `l09_…`); verilator + hand-IEEE; OPEN
- 🆕 AA — a consumer `always_comb` / `always_latch` written before the block feeding it, in a chain no time-0 process write reaches (declaration initializers, a constant block, a settled CA), reads x in its time-0 pass and reports W4031 / E4003 at t0 where both oracles are silent (q_chaind_A_B, q_chain3_A_Bc_Bb, s583_a08, x13b, tg2, tg5, t3j; PRE the same; and aa1_chain, an `if` chain, §4.5.585: vita W4031 at t0 and t5, verilator silent, iverilog on the `unique case` twin s583_a08 `Time: 5` only); the passes run one per batch in source order (§4.5.584: `sched/run_loop.rs`, `native/run.rs`); closing it needs an order among the time-0 passes, and the oracles are silent by different ones (iverilog reverse source order, which reports the same consumer-first read at the source's next change and reports forward chains at t0; verilator dependency order); every order measured moves split cells (§4.5.584 grounding: reverse 3 to iverilog, dependency order 4 to verilator); SPLIT
- 🆕 AB — a function reached from a continuous assign runs once more at t0, on x, before the `initial` that writes its inputs (k1_ca_display: vita `f t=0 x=x z=x` then `f t=0 x=0 z=1`; iverilog and verilator print `f t=0 x=0 z=1` once): a `unique` / `priority` miss in it, or in anything it calls, reports W4031 at t0 (q1caf_case_tbF/L, t0f_ca_case, t0f_ca_if, n_ca_objf2case_tbL ×4 through a class handle, q6a_fn_vfn_noformal_case through a formal-less item `function void`) and an immediate `assert` fails E4003 at t0, so a clean design exits 1 (k2b_ca_assert_clean), where both oracles are silent at t0 and exit 0; `always_comb y = f(a, b);` is the workaround (docs_cells k1w, k2bw: vita = iverilog, exit 0); same family as manual 006 §3.1's `$random` re-drawn per settle pass; documented in manual 006 §3.1; holds `unique-if-chain`'s subroutine-body residue (armed, a CA reaches a class function through a handle, a class void method through `this.`, a constructor through `new`, a formal-less item `function void`, and a task through an accepted function→task call: n_ca_objf2_tbL_H ×4, t2t_c_fg_this_H ×2, u20_c_fnew_member_H ×2, q6a_fn_vfn_noformal_H, u8_c_ft_this_nowrite_H / u23_ctor_cls_task_H ×2 W4031 at t0 on PRE); first: census where the t0 settle evaluates a continuous assign ahead of the first batch, then measure an after-the-first-batch evaluation on k1, k2b, q1caf, q6a; 2 oracles; OPEN (§5.2 row 2)
- 🆕 AC — a call the constant interpreter declines, where vita needs a constant (a count, a width, a bound, a label), takes a fallback at exit 0: an empty replication (probe p6 `r1=00000000 r2=00000000`, a `$display` body and a plain `case` body; c_rep_case_M; `{f(2){"ab"}}` into a `string` prints `s=`: e02, t16), a 1-bit `+:` (e_pswr_case_M `pw=1`), a 1-element `[m:l]` (h_prd_case_M `pr=1`, h_mdpr_case_M `m=1`, h_plsb_case_M `v[11:f(2)]` `pl=0`), the generate-case `default` (h_gcl_case_M, h_gcl10_case_N), or an unmasked value when the call is in an interpreter formal or local range (pk_ce_fml, pk_ce_loc `P=1000`: `const_fn_width.rs` `const_decl_wsign` declines a bound holding a call); iverilog and verilator `r1=0000007f r2=0000007f`, `s=abab`, `pw=7f`, `pr=ff`, `m=89abcdef`, `pl=1e`, the label, `P=8` (19 §4.5.587 grounding cells + p6); sinks `const_bound.rs:185` `lower_const_width_expr` (keeps the lowered call when `const_bound_u32` declines; the engine's shallow fold reads `unwrap_or(0)` / `unwrap_or(1)`), `packed.rs:1890` (also :2204, `packed_inner.rs:152`), `generate.rs:459` (a label that does not fold is no match, 🆕 T's sink; h_gcl_case_N is right by that accident); the same sinks take an x-valued compare (PROBE_CATALOG §4.5.580, §4.5.581); the `[m:l]` LSB lowers as a run-time call and reports W4031 at time 1 where both oracles are silent (h_plsb_case_M, `packed.rs:339`); close upward first (ER §2.5): the 19 cells and p6 fold once §3.b `const-fn-case`, `const-fn-systask` and `unique-const-fn` land (its 10 `unique if` cells are `unique-const-fn`'s; e02 / t16 only if it states a string replication count, whose multiplier IEEE lets be non-constant), pk_ce_fml / pk_ce_loc once `const_decl_wsign` folds a bound holding a call (it declines one so `bit [f()-1:0]` inside `f` cannot recurse) in the declaring scope (🆕 AD); then re-measure what still declines at each sink; re-entry: `unique-const-fn`'s fix path (§4.5.587); 2 oracles; BLOCKED (§3.b `const-fn-case`, `const-fn-systask`, `unique-const-fn`; the interpreter ranges also 🆕 AD)
- 🆕 AD — routine text folded or lowered at the caller's prefix binds a call in it to the caller's same-named function (plain twins on PRE; iverilog and verilator in parentheses): a package routine's return, formal and local ranges, a frame local, and an inlined cast, replication and `[m:l]` (pk_rt_ret / pk_rt_fml / pk_rt_loc / pk_fr_loc `v=232` (8), pk_rt_cast `v=-24` (0), pk_rt_rep `0000007f` (`00000007`), pk_rt_psel `000000cd` (`0000000d`)); the interpreter's return range and formal default (pk_ce_ret `P=232` (8); pk_ce_dflt `P=1007`, iverilog 1003, verilator refuses the design); `$bits(q::h(0))` (pk_wq `B=8` (4)); an imported routine (imp_rt_ret, imp_ce_ret, imp_two_pkgs `232` (8)); a `$unit` routine, which `inject_cu_items` (hdl-parser `module_items.rs:342`) copies into every module (r24_unit_shadow `v=232 P=232` (8)); a generate-scoped routine shadowing the module's (gen_fn `P=7 bw=8`; iverilog `P=3 bw=4`; verilator refuses `Constant function may not be declared under generate (IEEE 1800-2023 13.4.3)`); the instance-array prepass (`instance_array.rs:114`–`144`) binding a child's header default and port range with the parent's function (d01, d02 `top.u[1] … p=5`, oracles `p=a`; t11 `top.u[1] b=4 p=a`, oracles `p=5`); the interpreter's body is right (pk_ce_body, `const_call_pkg`); these lanes read constants at the caller's prefix too (PROBE_CATALOG §4.5.564; the "Scoping" package lines below), so the fix is the declaring-scope fold (REMAINING_WORK §D) over routine text and the prepass, never an exclusion by text span or enumerated scope (§4.5.558, §4.5.560–562, §4.5.587 reverted on it); a lane this row opens from loud to a value declines a run that reads a never-assigned 4-state variable (§2 🆕 AE; an over-seed stays loud, never silent; l22, the instance-array prepass over `fx(2)`: PRE E3009, both oracles `P=xxxx`, and a fold in the child reads 🆕 AE's 0); first: census every lane that folds or lowers another scope's text, then fold each where it is declared; re-entry: `unique-const-fn`'s fix path (§4.5.587); 2 oracles; OPEN (§5.2 row 1)
- 🆕 AE — the constant interpreter reads a never-assigned 4-state variable as 0 (its env is `BTreeMap<String, i64>`, its result `Option<i64>`): the return variable, seeded by `const_fn.rs:1507` `env.entry(name).or_insert(0)` (x2a `P=0000`, d10 `PX=0000 PI=0`, x2m / x2n beside a folded sibling: verilator `xxxx` / `x`; iverilog refuses `Unable to evaluate parameter P value: top.fx(32'sd2)`), and a 4-state local declared without a value (`:1335`; x2c `P=0000`, x2i `integer` `P=0`, x2l after a bit write `P=0001`: iverilog and verilator `xxxx`, `x`, `xxx1`); a bound read from it binds (x2k `b=1`; both oracles refuse the design); a known result read through it is wrong too (m19 `repeat (fd(2))`, `fd` testing `t == 4'd0`: `n=1`, iverilog and sv2v `n=2`, vita's own run time 2, m25); IEEE 1800-2017 §6.8 gives x; fix: carry an x plane to each binder, which converts it (2-state), refuses it (≤64-bit 4-state) or holds it (>64-bit); every narrower cut descends (§4.5.587, §4.5.588): a decline on every read (r08 `t & 4'b0000`, `t === t`: both oracles `0000`, `1`) or on an x-bearing result (13 correct cells loud; s01 silent through 🆕 AC's sinks); a 4-state re-run keeping only a fully known result breaks a cancellation (p01 `localparam int Q = fd(2) - fx1(2);` PRE and iverilog `Q=0`, re-run `Q=1` by construction) that escapes through a parameter (`R = P - fx1(2)`), so no opt-in contains it; sending a `repeat` count whose fold read such a variable to the run-time loop (`repeat_unroll_count`; the loop runs an x count 0 times) moves 8 cells silent→correct (m19, p06, p15, p16, p19 to `n=2`; p07, p13, p14 to `n=0`, hand-IEEE §12.7.2) but also seeds `enum bit [N]` locals and returns, which the parser records as 4-state `Logic` (`hdl-parser/src/typedefs.rs:351`, `functask.rs:201` `ret_two_state`) and vita's run time misreads (e01 `n=1`→`n=2`, e05 `n=2`→`n=0`; d11, a base-less enum return beside a seeded local, `n=2`→`n=0`; both oracles = PRE); until it lands, a lane a row opens from loud to a value declines a run that reads a never-assigned 4-state variable (an over-seed stays loud, never silent); re-entry: `unique-const-fn`'s fix path (§4.5.587); 2 oracles (locals), verilator + hand-IEEE (return variable); BLOCKED (§2 row 15's 2-state identity; 🆕 AC's sinks refusing an x-valued call; a 4-state result channel from the interpreter to module-scope folds; the `repeat` cut: the "Size cast" enum lines)
- 31 — `$signed(a)*$signed(b)` in a CA evaluates 3× (values right); PERF
- 32 — `$finish` in a function: iverilog stops, verilator runs on (`frame_eval.rs::run_frame_call_with`); SPLIT
- 34 — rows 23, 24 (clocking): 1 oracle, zero corpus demand; DO-NOT-START

### Size cast / signedness

- Width probe — runs diagnostics and serializes the dead node (E3009 ×2); WALL
- Node width — `ir_bits_of(plain)` is the node's, not the operand's (`2'((s8>>u3)*s16)`); WALL
- Unknown width — `ir_bits_of` → `None` (hier read, string): `2'(u1.mem[0] % 4)` `xx`; WALL
- `typedef enum bit [7:0]` stores 4-state (reads x, keeps `X`; the parser records `enum bit [N]` as `Logic`, `hdl-parser/src/typedefs.rs:351`, and a function returning one gets `ret_two_state` false, `functask.rs:201`: d01rt `repeat (fd(a2))` over a never-assigned `enum bit [3:0]` local `n=2`, d05rt over such a return `n=0`, iverilog and verilator `n=1`, `n=2`); prerequisite of §3 ⑤ⓕ's enum base and of §2 🆕 AE's `repeat` cut; 2 oracles; OPEN (§5.2 row 8)
- A function returning a base-less enum reads its never-assigned return variable as x at run time (d07rt `repeat (fr(a2))`, `if (fr == 4'd0) fr = B;`: `n=0`, iverilog and verilator `n=2`; the constant fold e07 `n=2` is right; d11, the same return beside a never-assigned `logic [3:0]` local, folds `n=2` and runs `n=0`); the parser records the return 2-state (`functask.rs:201` `ret_two_state`) and a base-less enum local is right (d09rt `n=1`); the run-time site is not found; prerequisite of §2 🆕 AE's `repeat` cut; iverilog + hand-IEEE (§6.19: the base is `int`); OPEN (§5.2 row 8)
- 4-state narrowing drops x (`2'(a+1)` over `8'bxxxx_0011`); iverilog; OPEN
- Call-leaf sign — `64'(f(1) - 40)` zero-extends; give `expr_self_signed`'s `_ => false` the return type; 2 oracles; OPEN
- Real × fill — `4'(RP ^ '0)` skips the funnel (`ast_ctx_signed`, `expr_is_real`); WALL
- `$signed(real)` accepted in 7 of 22 positions (iverilog refuses all); make loud; 2 oracles; OPEN
- Nested cast — context stops at an inner node (`64'(-16'(u16))`); 2 oracles; OPEN
- Fabricated width — `40'(u1.f(0))`, `longint'(q.sum())` zero-extend; BLOCKED (declared width for reduction / string / placeholder operands)
- Double call — `40'(sf(1) + 8'sd0)` calls `sf` twice; `expr_size_ctx.rs` `lower_size_leaf` → `extend_signed_once`; 2 oracles; OPEN
- Element sign — `unpacked_elem_signed` takes single-segment idents only (`pk::pm[0]` zero-extends); iverilog; OPEN

### Constant domain (i64)

- Above 64 bits the decline is deliberate (`const_unsigned_at_sixty_four.rs`); HELD
- An untyped 64-bit `*` / `+` wraps (= verilator; iverilog self-contradicts); HELD
- A huge untyped `**` hangs iverilog (verilator alone); RECORD
- Placement / cast fold residue (concat carry, concat x/z, `int'(7)`, `longint'` bit 63) is loud; route to the width-aware interpreter walk; 2 oracles; OPEN
- Size-wrapping cast `4'((4'd8+4'd8)/4'd3)` declines (`const_eval_cast`); WALL
- A 70-deep constant-function chain is loud (iverilog 71); charge depth on re-entry only; iverilog; OPEN
- `$bits("")` / untyped `P = ""` is 1 bit (oracles 8); read the literal numerically; 2 oracles; OPEN
- Declined wide walk — `int'(NM) ^ 64'h0` keeps the unlimited i64 answer; `fold_region` arms per consumer (§4.5.562's design); 2 oracles; OPEN
- Masked overflow — `((0-1)*(0-1)-2)/2` at 32 bits is 0 (oracles `7fffffff`); `const_fn_width.rs` `checked_mul` → wrapping + mask; 2 oracles; OPEN
- Generate `case` compares two i64 values; size the case and items once (`fold_bits_at`), not pairwise (§4.5.555); 2 oracles; OPEN
- A select write into a local whose initializer did not fold reads 0 for the other bits (`const_fn.rs:1582` `env.get(name).copied().unwrap_or(0)` over the unbound local `bind_const_decl` leaves): a41 `int t = int'(2.5); t[0] = 1'b0; f = t;` `P=0`, iverilog, verilator and sv2v `P=2`; declining there makes the twins that read only written bits or overwrite every bit loud (p03 `t[0] = 1'b1; f = {31'b0, t[0]};` and a 32-bit overwrite loop: `P=1 Q=0` = all three oracles); close upward: fold the initializer (the `int'(…)` placement / cast residue above) or track the written bits; 2 oracles; OPEN
- A continuous-assign or net-declaration delay over a constant-function call that reads a never-assigned 4-state variable folds 🆕 AE's 0 (`const_eval.rs` `fold_ca_delay`): `assign #(fd(2)) w = r;` and `wire #(fd(2)) w = r;` delay 1 where iverilog and verilator delay 2 (p05b, p08b); `#(fx1(2))` (`xxx1`) delays 1 where iverilog and sv2v take the x delay as 0 (p10), as vita's run-time lane does (p12, `#(dv)` with `dv = 4'bxxx1`); that lane (`ca_delay_rt.rs`) shares 🆕 AE's `repeat` cut's enum prerequisites (d12, an `enum bit [3:0]` local: folded delay 1 = iverilog and verilator, run-time twin d12rt delay 2), and routing there must count the routed assign as delayed in `demote_runtime_delay_on_resolved_nets`, which otherwise drops its delay on a resolved net where today's folded delay is E3001 ("Delays": a resolved multi-driven `wire` drops a runtime delay); 2 oracles; BLOCKED (the "Size cast" enum lines)
- `bit [65535:0][65535:0]` panics at net allocation; make it loud; OPEN
- Three declared-width models (`const_decl_wsign`, `const_bound.rs::decl_is_wide`, `ast_kind_range_width`); RECORD
- The interpreter reads an `int unsigned` return signed (`const_fn_ret_wsign`); 2 oracles; OPEN (sign, scope half BLOCKED: declaring-scope fold)
- A formal shadowing a parameter is selected as the parameter (`const_param_select_env`); 2 oracles; OPEN
- A non-zero-LSB local in a concat is read by position (`envw` has no LSB); 2 oracles; OPEN
- A negative replication count is accepted (`fold_count`, `const_eval_u32`); 2 oracles; OPEN
- `byte'(…)` folds without the type width (`const_eval_cast`); 2 oracles; OPEN
- A count over `4'sbx000 == 8'sd5` collapses to 0 (`wide_eq_with_unknowns`); 2 oracles; OPEN
- `16'd1 % ('1 + 1'b1)` divides by a wrapped zero silently; 2 oracles; OPEN

### Index sealing

- Queue / dyn index unsealed (`q[-8'sd1]` reads 255; `dynarr.rs`); iverilog; OPEN
- Call index unsealed (`arr[fneg(0)]`, `mg[u.hs(1)]`); 2 oracles; OPEN
- `gp[0][$urandom] = 1` draws twice (`array_word_index_domain.rs`), and the element write `m[f(2)] = 4'hA` over `logic [15:0][3:0]` calls `f` three times (§4.5.587 probe p5, iverilog once; e_sel2: a `unique` miss in `f` reports W4031 three times, both oracles once); iverilog; OPEN
- Packed element `+:` overhang (`packed_select_signed_index.rs`); SPLIT
- `#(.P(32'd1 << 128'd70))` stays E3009; 2 oracles; OPEN
- A declined >64-bit tree binds its low 64 bits (`const_wide_num.rs` zero-divisor arm); 2 oracles; OPEN
- A fill in an override tree keeps the default's type; region width via `declared_override_widths`; 2 oracles; OPEN
- An unreadable override leaf (element, replication, call, `$rtoi`, prim cast, `SZ_0'(B8)`) keeps the default's type; verilator; OPEN
- `parameter signed A = 4'd10` reads unsigned (`param_decl_width_opt`; §4.5.559); BLOCKED (the alias line below)
- A generate-scope alias `localparam C = A;` binds 32 bits (`param_decl_width_opt` alias arm); 2 oracles; OPEN
- `signed` on a string default is dropped (`str_param_raw`); 2 oracles; OPEN
- A fill override onto a >64-bit default binds at its width (`override_at_declared_width`); 2 oracles; OPEN
- `parameter unsigned U` + a signed override binds signed (`ParamDecl.signed`); `is_sign_declared` = format bump; 2 oracles; OPEN
- `p.signed` written from non-keyword sources (`params.rs` typedef prefix, `module_items.rs`); RECORD
- An instance-array parameter takes a sibling array's override width (`instance_array.rs`); 2 oracles; OPEN

### Inline / frame binds

- A hierarchical leaf in a §11.6.1 region declines without the child's environment (typed parameter, `defparam`, generate / array instance); a width channel (`coerce_int_width`); 2 oracles; OPEN
- An interface member through a module port keeps the old width (`p.uh`); verilator; OPEN
- Interface members the §4.5.510 fact table declines (block-local shadow, import range, generate instance); 2 oracles; OPEN
- `$signed(<string>)` and `$unsigned(a, b)` are accepted; make loud; 2 oracles; OPEN
- A time literal in an inline region folds at 64 bits; 2 oracles; OPEN
- An inline body's written formal gets no width context; a body `real` local is E3010; 2 oracles; OPEN
- A signed x/z queue element bound to a formal zero-extends; 2 oracles; OPEN
- An inexact placeholder width keeps the PRE route (`expr_size_hier.rs` `env_fold`); BLOCKED (exact declared-width fold for hierarchical placeholders)
- A real-returning hierarchical call stays raw bits inline (`u.hr(…)`); 2 oracles; OPEN
- A multi-dim hierarchical select is unrecorded inline (`u.m2[0][1]`); 2 oracles; OPEN
- A declined declaration walk keeps the leaf width (negative LSB, generate / upward instance, `u.P`); 2 oracles; OPEN
- A local stream into a wider target right-justifies (§11.4.14.3 left); a hierarchical one is E3009; verilator + hand-IEEE; OPEN
- A nested inline callee sees the outer formal's `subst` (`param_sel_range`); use the callee's `frame_base`; 2 oracles; OPEN

### Real

- A real-returning constant function body is E3009 (iverilog 0.0); LOUD (§3)
- `$signed(<real>)` in a function body is accepted; 2 oracles; OPEN
- `real unsigned r;` is accepted; 2 oracles; OPEN
- `$realtobits` / `$bitstoreal` accept a non-64-bit argument; iverilog; OPEN
- A real over a >64-bit integral (`65'd5 - 65'd7`) converts the i64 fold; 2 oracles; OPEN
- An untyped `R = 2.5` given an integral call override stays real; BLOCKED (declaring-scope fold, §4.5.558)
- A real parameter's integer view answers `R/4 > 1` in the integer domain (`params` before `real_param_val`); 2 oracles; OPEN
- A package real imported by name reads its i64 twin (`apply_import_consts`); 2 oracles; OPEN
- `$itor` / `real'()` / assignment use `to_f64`, `$realtobits` uses `integral_to_f64`; route all through the latter; 2 oracles; OPEN
- An exact-integer real parameter divides as an integer (`int'(R / 2)`); 2 oracles; OPEN
- `time'(NM) + 0.0` converts signed; iverilog + hand-IEEE; OPEN

### Ranges / bounds / selects

- `$size(da, 1)` answers the element width (§4.5.500 takes one argument only); verilator; OPEN
- A scalar shadowing a const array is skipped (`const_array_vals_of_base`); verilator; OPEN
- A bare >64-bit non-zero-LSB parameter select reads positionally (`P[79:72]`); BLOCKED (declaring-scope fold, §4.5.560)
- A parameter select as a struct member (E2002) or class property (E3009) width; 2 oracles; OPEN
- `u.A[12:19]` of an ascending >64-bit parameter reads positionally (`build_hier_param_select`); 2 oracles; OPEN
- An overridable 1-D `parameter type T = logic [8:1]` registers `[T$w-1:0]` (§4.5.514's carrier); 2 oracles; OPEN
- `function [f():0] f();` overflows the stack (`const_fn_ret_wsign`: `depth + 1`); OPEN
- A >64-bit generate-scope localparam is invisible to `param_sel_range`; 2 oracles; OPEN

### Class fields

- An ascending negative bound is clamped only on a class property; BLOCKED (row 3b)
- `x[-3]` on `logic [-3:0][1:0] x` is loud (`dim_coord` ascending arm); OPEN
- A duplicate class field is accepted, last wins (= iverilog); SPLIT
- A class field default folds at its own width and keeps x in a 2-state field (`fold_init`, `classes.rs`); 2 oracles; OPEN

### Scoping / imports / block-locals

- `pk.hf(x)` on an instance named like a package resolves to the package (`inline_fn.rs`, `expr_size_ctx.rs::pkg_call_head`); verilator; OPEN
- A package routine called bare from another package resolves defaults and names in the caller's package; resolve via `inject_pkg_callees` first; 2 oracles; OPEN
- A package body's read after a shadowing block falls to the caller; BLOCKED (binding-resolved scope)
- The constant domain in a package body is unhooked (`lookup_scoped` + nine `params` twins); BLOCKED (declaring-scope fold, §4.5.560–561)
- `$bits` of a package constant in its routine reads the caller's; BLOCKED (declaring-scope fold)
- A whole-name write to a package constant in its function is accepted; 2 oracles; OPEN
- A free name in an imported package routine binds the caller's net; 2 oracles; OPEN
- Scoped `pk::g()` skips the block-local scope-leak gate; BLOCKED (binding-resolved scope, §4.5.490)
- The inline-fold lane takes §11.6.1 context by name (`inline_fn.rs` `scope.dims`); BLOCKED (binding-resolved geometry)
- A block-local shadowing a formal is read after its block; 2 oracles; OPEN
- A static shadow pair beside an `automatic` span keeps the old flatten (`compute_scoped_block_locals`); verilator; OPEN
- The constant domain is not shadow-aware (`const_eval.rs` → `walk_scopes(params)`); 2 oracles; OPEN
- A static frame local initialised from a formal runs per call (`frame_static_init_t0_safe`); iverilog; HELD
- A static frame local initialised from a module net is not retained across calls; 2 oracles; OPEN
- A package routine's static local is copied per importing scope (`apply_import_routines`); one design-wide frame; 2 oracles; OPEN
- A static function reading its own return variable first returns 0 (§13.4.1); 2 oracles; OPEN
- A class method's same-named sibling block-locals are E3010 (`classes.rs`); 2 oracles; OPEN
- A parameter and net of one name are accepted (`block_local_shadows_param.rs`); 2 oracles; OPEN
- A part-select write into a queue element vanishes (`q[0][15:8]`); verilator; OPEN
- A typedef's named dims re-resolve at each use; BLOCKED (declaring-scope fold)
- A `modport` named like an imported symbol is accepted; SPLIT
- A call naming a nested block label that is also a routine is silent; 2 oracles; OPEN
- `parameter type T` beside `wire T` is accepted (one oracle, one diagnosis); HELD
- Package typedef / class / enum-label pairs vs non-routines are unmeasured; RECORD
- Typedef × enum-label refusals rest on verilator alone; RECORD
- A width-0 indexed part-select is accepted (iverilog rejects); RECORD
- A string-keyed assoc index skips the §6.16 funnel (`to_sv_string_bytes`); verilator; OPEN
- A function-local block `u` beside instance `u`: `u.lv` binds the instance (`census_item`); iverilog + §23.8; OPEN
- A label or genvar over a wildcard >64-bit constant leaves it visible (🆕 U's class); 2 oracles; OPEN
- Two wildcard >64-bit constants of one name bind the first; iverilog + §26.3; OPEN
- An array-method iterator is read through a same-named parameter's range (`param_sel_range`); verilator; OPEN
- A generate `real` parameter does not shadow an outer integral one; BLOCKED (declaring-scope fold, §4.5.562)

### Delays / events

- An in-body `@(*)` over a dyn handle stays stale (compares the handle word); iverilog; OPEN
- A runtime delay net driven by `#0` is read before it lands (`schedule_delayed_cas`); OPEN
- A resolved multi-driven `wire` drops a runtime delay; 2 oracles; OPEN
- `vm` loses a static task's `string` formal (native right); OPEN
- ⓐ Delay rounding (real leaf vs sub-precision unit leaf); SPLIT
- ⓒ `#(1ns - 5ns)` fires at once (`real_delay_ticks` clamps); BLOCKED (a "never fires" tick)
- ⓔ Multi-timescale rounding precision; SPLIT
- A t0 copy net takes only its own move (`alias::copy_nets`); iverilog; OPEN
- An unpacked net array is one net on the dirty channel; per-element dirt; iverilog; OPEN
- A multi-bit level term beside an edge term wakes on bit 0 only (`state::edge_mask`); 2 oracles; OPEN
- A non-constant CA LHS index is accepted (`delayed_cont_assign_heap_read.rs`); verilator; OPEN
- `$fatal` drops its step's pending NBA; HELD
- A constant select indexed through concat / sysfunc / hier name never wakes (`index_provably_live`); 2 oracles; OPEN
- Unadmitted time-0 bodies drop a constant's t0 run (`body_suspend_blocker`); 2 oracles; OPEN
- The header `Level` waiter re-runs a pending process (dS05c); 2 oracles; OPEN
- A runtime-delay call runs silently on `vm` (native loud, W4030); OPEN
- A CA in another time unit reads `$time` at the wrong scale (`cont_assign_time_read.rs`); per-assign multiplier = bump; 2 oracles; OPEN
- A CA hop lands after the whole batch; yield settle + nonblocking UDP (§4.5.539); BLOCKED (wake-group order has no oracle)
  - t0: Level / Edge processes the t0 settle wakes, read in batch 2 through a CA or port, read x where both oracles read the value (x7, x7b, x7c, x7d, lb1–lb3, b1_ca_hop_noinit; the `always_comb` twin b2 keeps `v=x`); §4.5.584's patch B (each such process alone in a batch, a settle after each) closed them but merged that process's own wakes into the parked first batch with no settle between — a two-oracle descent (b1: iverilog = verilator `R t=0 v=1 u=1`, B `v=x`) — and was dropped
- An in-body edge wait sees an earlier edge of its batch (`WaitCause::Edge`); §4.5.537's rule, then `@(cb)`; 2 oracles; OPEN

### Diagnostics / artifacts

- A Mul chain base re-lowered n times emits n E4002 copies; OPEN
- `coverpoint_domain`'s Pow arm uses `max(lw,rw)`; OPEN
- `run.json` omits `-G` (§6 OBS-1); OPEN
- A CA's user function runs more than once per event; BLOCKED (purity-certification adjudication)
- An operator over a call names it twice (`p8() >>> 1`); BLOCKED (purity-certification adjudication)
- `wprog` decline reasons relabelled by §4.5.530; RECORD
- `%h` of a 1-bit unknown expression is `x` (iverilog `X`, inconsistent); RECORD
- A string literal's NUL byte is kept raw (oracles a space); 2 oracles; OPEN
- `$dumpfile(DF)` names the file after the parameter path; iverilog; OPEN
- `V.x` under a shadowing constant gives the generic E3010 (`bare_const_shadows_net`); OPEN

### Performance (open, recorded)

- A CA with any `Call` / `SysFunc` re-evaluates 6× (`levelize.rs` `expr_is_pure_of_nets`); BLOCKED (purity-certification adjudication)
- Per-bit `coerce_two_state` survives (`inline_fn.rs` R2, `lower_prim_cast`); OPEN (R2, the rest BLOCKED: declared width)
- Coercing at the operand's width is ~10× faster where the width is declared; PERF
- The size-cast seal leaves `wprog` on three axes (`Expr::Call` has no `compile_node` arm); PERF
- Constant width / sign walk the tree three times (`eval_const_env_self`); PERF
- A left-leaning `==?` chain is 2^depth; PERF

### Oracle splits (recorded, not chased)

- select vs whole-net level waiter order — split (vita = verilator)
- deferred-assertion report maturity — no oracle (vita hand-IEEE §16.4)
- a real stored into a string — no oracle (vita the element rule)
- t0 wake / edge of a no-definite-bit driver — split (vita by value, §9.4.2)
- zero-delay CA update visibility — split (vita: first `#0` promotion)
- t0 Active order around an all-constant `always @(K)` — split
- what runs after `$finish` in its step — split
- packed dims before an unpacked typedef name — split (vita = verilator)
- init width of untyped `4'd15 + 4'd1` — split (vita = iverilog 16)
- 64-bit unsigned `%` — iverilog self-contradicts
- cross-scope t0 decl-init race; runtime `-0.0`; iverilog expression-force — do not chase
- `$stime` sign under a cast — split (vita unsigned, 1364 §17.7.2)
- cross-package const-function recursion — vita = hand-IEEE
- `#(.S("str"))` prints one spurious W3056 (value right)
- untyped `K - 20` over `bit [3:0]` — split (vita = verilator 32 bits)
- real beside a bit-vector inline region — split
- two explicit imports of one name — split
- use before declaration in an interface — loud kept (= iverilog)
- `#(.P(pk::PA + 0))` over `[0:35]` — split (`pkg_const_layout_override.rs`)
- `%s` of an unstored `string'(e)` — split (vita = verilator)
- string-cast spellings with ≤1 oracle — stay loud
- `always @*` at t0 (no pass until an input changes, = iverilog and §9.2.2.2; verilator runs it) and reading only a constant — split
- unlabelled `begin` in a generate region — split (vita = iverilog)
- unlabelled generate net beside a same-named function — split (vita = iverilog)
- repeated non-ANSI header port — split
- non-finite / out-of-range real into an integral — split
- override equal to the default: verilator is no oracle there
- `u.w[u.P*2-1:0]` — split (iverilog rejects)
- `$bits(u.r)` of a hierarchical real — split (vita = verilator 64)
- constant term beside a live one, suspending body — split (vita = verilator)
- constant term beside an edge term — split (vita = iverilog)
- in-body wait on a constant — split (vita = verilator)
- named event beside a constant — split (vita = verilator)
- constant equal to a variable's default as an event term — split
- t0 NBA / `#0` write of a live term — split (vita = iverilog)
- same-step glitch under an in-body wait — split (vita = verilator)
- net-only header level list at t0 — split
- same-time resume order (wake group, fork arms, t0 hierarchy, `wire #0`) — split
- `$strobe` before the first monitored change — split (vita = verilator)
- `-G` decimal beyond 32-bit signed — split
- sign keyword with no range beside a real — split
- `$realtobits` of an integral — split (`integral_to_f64`)
- package-body enum label named like a constant — split
- negative signed part-select bound — split; vita mixes both readings
- `always_comb` / `always_latch` t0 passes of several blocks — split (vita forward source order, one per batch with a settle after each; iverilog reverse source order; verilator dependency order): from a quiet source a forward chain reports at t0 in iverilog only (tg1, tg3, tg4, tg6, tg8, tg10, tg13, tg14), a consumer-first chain in vita only (§2 🆕 AA)
- a constant or declaration-fed `always_comb` read at t0 before its pass (an `initial`'s first slice, a reader another t0 write woke, a t0-posedge flop: q_const_comb_first, s583_v01, s583_v03, x3, r1, r2, f2, f2b) — split (vita = iverilog `xx`; verilator the value; after one `#0` vita reads the value, iverilog `xx`)
- `always_comb` run count at t0 when an input changes at t0 — split (vita = iverilog twice: the woken pass, then the implicit one; verilator once; d1b iverilog once)
- GATED-CLOCK edge dedup at t0 (`reset_edge_seen_marks` now runs at t0 in every comb design): an edge from a settle-woken comb pass is deduped, one from an implicit pass re-fires (g1) — split (vita = iverilog on the woken pass's edge, neither on the implicit pass's)
- an implicit t0 pass's wakes against a promoted `#0` resume (c3, c4: iverilog `s=x g=x`, verilator `s=1 g=1`, vita `s=1 g=x`; fj1, a fork arm's `#0`) — split; vita matches neither (a legal §4.7 order)
- comb t0 pass slots in a port chain (t3g: vita = verilator, iverilog reports at t0), a first-batch-woken checker reading an implicit-only comb (t3k: vita = iverilog's two W4031) and a named event a comb's t0 pass triggers (n1: vita = iverilog `E1 t=0 y=01`, verilator nothing) — split
- a self-timed `always` (no header) written before the `initial` that drives it, at t0 — split (vita = iverilog: source order, a `unique` miss on x reports at t0; verilator starts the `initial` first): s1s_case (iverilog `Time: 0`, `Time: 1`), s3_case (`Time: 0`, `Time: 2`), s1s_selftimed_first, s3_child_selftimed_tbL (the `if` chains, §4.5.585), s583 v04

## 3. loud → correct-support candidates (all loud = safe, additive)

The workload corpus has no refused row; the next work is the new-design census (§5.2).

### 3.a Numbered open items

- ③ⓐ — a file-read call in `&&` / `||` / `?:` cannot be hoisted (`hoist/general.rs`); `guarded_hoist` + an fd ordering proof; iverilog; OPEN
- ③ⓑ — a call in a `while` / `for` condition; rewrite into the body (`lower_shortcircuit_cond`); iverilog; OPEN
- ③ⓒ — every statement a `$feof` survives into is refused; BLOCKED (an `order_walk`-grade ordering judge)
- ③ⓓ — reads no alias names (`m.a`, `p::v`); judge overlap by net identity; iverilog; OPEN
- ⑤ — ibex's compiled-away `export "DPI-C"` and genvar-indexed path `g_pmp_csrs[i_region].x`; verilator; OPEN (DEEP)
- ⑤ⓐ — multi-packed array parameters (E2002) and two wildcard imports of `P` (§26.3); one arm per consumer; verilator; OPEN
- ⑤ⓒ — header array parameters: nested override, >64-bit element, `defparam`, interface header, `'{default: v}` (`array_param_twin`); verilator; OPEN
- ⑤ⓔ — element select: multi-packed element, `p::S[1].b`, runtime `$size`, untyped-child override; verilator; OPEN
- ⑤ⓕ — unpacked-array typedef per consumer (`!info.unpacked.is_empty()`): `string` element, body parameter, interface header, `typedefs.rs` chain, `T'(…)`, enum base, union member; 2 oracles; OPEN (S each, arity / class property / return type: §5.2 Do not start)
- ⑤ⓓ — nested struct members (`default: v`, `o.i.e.name()`, packed array member, named widths); the parser's flat layout; 2 oracles; OPEN
- ⑤ — CU scope: unit variable, unit enum label in a class, forward unit constant, `$unit::t`; 2 oracles; OPEN
- ⑤ — parser / preprocessor: md packed formal also unpacked, narrow based literal parameter, non-ANSI `<type> [dims]` port, `` `define `` formals; 2 oracles; OPEN
- ⑤ — `parameter type` with a struct / enum / union / real / string / class value; HELD (loud by design)
- ⑧ — system functions in a function body (`levelize::func_read_deps` cannot name a seed); 2 oracles; OPEN
- ⑧ — the statement after a function's `$finish` runs (`SimState::frame_end_is_loud`); iverilog; OPEN
- ⑧ — an output-formal function performs the `$finish`; unify the routing; OPEN
- ⑧ — a counter-carrying function is one evaluation off (extra t0 settle); 2 oracles; OPEN
- ⑨ — a bare imported string / real parameter is loud (`apply_import_consts`); 2 oracles; OPEN
- ⑨ — `generate if (P::R > 1.0)` is loud (`const_real.rs` `PkgScoped`); 2 oracles; OPEN
- ⑬ — a subroutine body's array access is attributed to the call; a second `cur_stmt` source; OPEN
- ⑬ — terminator / settle / t0-arm drains have no location; HELD
- ⑬ — W4022, W4028, delta limit, RunRange, W4020, W4029 / W4007 lack a location (`stmt_diag_meta`); OPEN
- ⑭ — an inlined subroutine reports 0 calls (`inline_task.rs` / `inline_fn.rs`); BLOCKED (an inline site → caller record)
- ⑭ — a reporter's 21× scheduler gap is not observability; DEFERRED (§5)

### 3.b Small residues

**Parser accept**

- non-ansi-md-port — non-ANSI `input T a;` with a md typedef is E2002; `PortDecl` packed list = format bump; iverilog; OPEN
- specify — `specify` is E2002; hoist `specparam`, discard path delays and timing checks behind a `ModuleItem` marker + an elaborate W3056 (the parser has no warning channel); iverilog; OPEN
- case-inside-residue — `case (e) inside` shapes refused by `case_inside.rs` `inside_refusal`: no-oracle (`$` bound, real, string, handle, call item), held (🆕 S (b), 🆕 X, 🆕 Y, PROBE_CATALOG P5), over-refusals N1 (`**` item), N2 (`$signed(u4) + 1`); sv2v + verilator; OPEN
- inside-name-use — a design using `inside` as a name refuses every `case … inside` (`api.rs::first_inside_name_use`); re-parse such a unit with case-inside off; cross-unit needs a scope model; E2002 where PRE ran: `inside == 4'd5:`, `inside.v`, `inside'(…)`; iverilog `-g2005`; OPEN (M)
- pkg-scoped-task — `pk::t()` as a statement is E2002; accept a scoped call target on a statement head, routed like the import; verilator; OPEN
- time-signed-param — `parameter time signed T` is E2002 (`hdl-parser/src/params.rs`); accept the qualifier and hand `kind_signedness` its bit; 2 oracles; OPEN
- based-ws — `64'sh FFFF` is a lexer reject; iverilog; OPEN
- tf-localparam — `localparam` in a task body is E2002 (§6.20); iverilog; OPEN
- blk-automatic — `automatic int x` in a block of a task body is E2002; 2 oracles; OPEN
- tf-decl-lifetime — `static int c = 0;` after `task t;` is E2002; feed `d.lifetime`; OPEN
- class-tf-port — a class-typed tf-port is E2002; accept a class name as a tf-port type; 2 oracles; OPEN
- R30-1 — a missing package gives 7 E2002 lines, unnamed; parse `IDENT::IDENT` as a type; OPEN
- enum-label — `enum bit[3:0] {A=8'hFF}` skips `enum_defs` and truncates (`const_lit`); widen `const_lit` or check at elaborate; iverilog rejects; OPEN
- md-packed-write — md packed nested part-select write: ascending / non-zero-LSB leaf, genvar index, silent OOB; OPEN
- misc-parse — negative-LSB member sub-select, generate `import`, `'{k:v}`, a write-sentinel panic; no oracle; OPEN
- edge-event — `@(edge clk)` is E2002; union of both edges (§9.4.2); 2 oracles; OPEN

**Constants / parameters**

- override-sysfn-value — a sysfunc override whose value does not fold is loud (`override_self_value`); give the value channel the concat, data-object select and dimension-query folds; 2 oracles; OPEN
- override-bits-asc — `$bits(pk::PA)` of an ascending package constant is E3009 (`const_eval_in_scope`); 2 oracles; OPEN
- override-concat-name — `#(.P({pk::PA}))` is E3009; census the concat accept set; 2 oracles; OPEN
- §3.3 — wide `localparam` part-select fold `{A[127:64], 64'h0}`; iverilog; OPEN
- real-fold — real `localparam` arithmetic is E3009 (i64-only `const_eval_in_scope`); iverilog; OPEN
- xz-fill-param — `P = 'x` and `#(.P(4'b1x00))` lose the x (`fill_to_i64`); 2 oracles; OPEN
- string-default-numeric-override — a numeric override of a string default is E3002 (`bind_one_param`); keyword on `ParamDecl` = bump; 2 oracles; OPEN
- string-const-operand — a string literal in the i64 walk is E3009; add the arm, census consumers; 2 oracles; OPEN
- real-int-overflow — `localparam int A = 1e20;` is E3009 (`real_round_to_i64`); 2 oracles; OPEN
- wide-untyped-consumer — untyped `Q = P` over >64-bit `P` is E3009; `fold_self_bits`; 2 oracles; OPEN
- override-signing-operand — `$signed(H0) / 8'sd3` as an override is E3009; 2 oracles; OPEN
- real-param-hier — `u.P` of a real parameter is E3010 (`hier_params` is i64); carry the real value to the hierarchical read; 2 oracles; OPEN
- compound-==? — `==?` fold residue (unsized x/z, negative LHS, hierarchical operand width); OPEN
- defparam-iface — `defparam` onto an interface instance matches nothing (`iface_inst.rs`); merge `defparams.remove(path)` into the canonical binder; 2 oracles; OPEN
- neg-ascending — `reg [-33:-2]` gives `$bits` 1 (`array_geom.rs` `allow_neg_lsb`); iverilog; OPEN
- neg-bound-part — negative-bound `[msb:lsb]` part select blocked (`const_bound_signed`); verilator; OPEN
- neg-elem-bound — `logic [-3:0] q[$]` clamps (`elaborate_netvar_decl_inner`); verilator; OPEN
- mdrv-partial — the multidriver check ignores partial writes (`stmt_writes_whole_ident`); xcelium unmeasured; RECORD
- mdrv-hier-actual — `always_comb u.ts(src)` is a false E3001 (Rule A `Unknown`); resolve the callee's formals through the instance's module; 2 oracles; OPEN
- frame-body-write-sites — call sites of a body-writing frame function that cannot carry `Terminator::Call` are E3009: CA rhs (no statement), a nested frame call (`frame_fn_lowering`), `pk::gw()` (`inline_fn.rs` reserves late), a class method, an outward / generate hier path (`hier_body_write_callee`), Rule A on a hier actual (`stmt_never_writes_ident`); per site; 2 oracles; OPEN
- frame-body-write-order — a hoisted body-writing call precedes a left read (vita = verilator); SPLIT
- dyn-size-spellings — `$size(c.da)`, `$size(u.da)`, a dyn formal's `$size` (`resolve_intro_net`); route all three to `DynSize`; verilator; OPEN
- dyn-bits-count — `$bits(da)` folds the element width (§20.6.2: the whole array); size × element bits, pinned by hand; no oracle; OPEN
- pkg-type-param-import — explicit `import p::PT;` of a `parameter type` is E3009; DO-NOT-START (§5.2)
- gen-rtn-edges — `u.g.f()`, `gi.f()` and a constant-expression call of a generate routine are E3009 (`frames_reserve.rs` hier gate, `const_func_table`, `frames_classify.rs`); compose the hier key, register per scope; iverilog; OPEN
- aes§2 — the inliner's discriminator is more than `automatic`; OPEN
- gen-enum-uncarried — a generate `typedef enum` binds only where §4.5.565 carries it (`gen_enum.rs`); BLOCKED (declaring-scope fold)
- string-literal-condition-residue — a string literal in a generate condition §4.5.568 cannot read (`cond_names.rs`); BLOCKED (declaring-scope fold)
- md-param-pattern-residue — a md packed parameter's non-literal `'{…}` items, keyed patterns, named bounds (`packed_md.rs`); per item, named items after row 15's carrier; verilator; OPEN
- cont-array-typedef-residue — a whole-array CA with an enum / mixed / named-bound typedef element (`cont_array::typedef_elem_type`); verilator + iverilog; OPEN
- cont-array-residue — a whole-array CA with a delay, decl assignment, sub-array, `?:` or second writer; OPEN
- packed-default-residue — packed `'{default: v}` with a typedef dim, partial target or non-integral `v` (`packed_pattern.rs`); OPEN
- const-fn-case — a constant function whose body holds any `case` / `casez` / `casex` / `case … inside` is E3009 `f(…) has no constant-fold arm`, every qualifier, reached or not, where both oracles fold (probe p2: plain `case`, `g(2)`, iverilog and verilator `P=20`; 49 §4.5.587 cells; `inside`: verilator only, iverilog `Incomprehensible case expression`); `elaborate/src/const_fn.rs` `exec_const_stmt` has no `Stmt::Case` arm (catch-all `_ => None` at :1719); the arm sizes the case expression and every item once at one width and sign (ER §2.4), never pairwise, the row's main risk; casez / casex masks, §12.5.4 for `inside`, first match wins; opening it lets the interpreter fold more functions at every consumer, the widening §4.5.587 reverted on, so it inherits §2 🆕 AD (not yet measured on case bodies), and the arm declines a run that reads a never-assigned 4-state variable (§2 🆕 AE: a `case` with no `default` that misses leaves the return variable unassigned; an over-seed stays loud, never silent); a `unique` / `priority case` miss then reaches the synthesized default (`unique-const-fn`); closes §2 🆕 AC's case-form cells; 2 oracles; BLOCKED (§2 🆕 AD)
- const-fn-systask — `$display` / `$info` in a constant function is E3009 `f(…) has no constant-fold arm` where both oracles fold silently and print nothing (§4.5.587 g4, `localparam int P = f(2)`: iverilog and verilator `P=3`); `$warning`: iverilog `P=3`, verilator `%Warning-USERWARN: "in f a=2"` at build, then `P=3`; `$error`: iverilog `P=3`, verilator `%Warning-USERERROR: "in f a=2"` and fails the build (`Expecting expression to be constant, but can't determine constant for FUNCREF 'f'`); IEEE 1800-2017 §13.4.3 ignores system task calls in a constant function (recalled); `exec_const_stmt` catch-all; an ignore arm needs `unique-const-fn`'s positive opt-in, since at a consumer that replaces a run-time call (a `repeat` count, a delay) both oracles print the task's output at run time (g4 d_display_rt), and it widens the interpreter like `const-fn-case`, so it declines a run that reads a never-assigned 4-state variable (§2 🆕 AE; an over-seed stays loud, never silent); 2 oracles (`$display`, `$info`); BLOCKED (§2 🆕 AD)
- pkg-param-own-fn — a package parameter whose value calls a function of the same package is E3009 `package parameter P value is not a foldable constant [in pk]` even for `return a + 1` (§4.5.587 plan probes pk_simple, pk_plain: iverilog and verilator `P=2`, `P=10 Q=7`); its declared range is E3009 too (pkown `parameter logic [f(2):0] P`: `a function call that does not fold to a constant is not allowed in a constant range bound`; oracle output not kept); `elaborate/src/package.rs:674` (the package fold's call lookup, not traced); 2 oracles; OPEN

**Subroutine / frame**

- md-return-select — a md packed return: (a) named bounds, (b) operator indices, (c) a select on a call (`packed_md.rs`); 2 oracles; OPEN ((a) BLOCKED: declaring-scope fold)
- scoped-call-wide-const — a scoped call whose body selects a >64-bit package constant (`pkg_wide_bits`); 2 oracles; OPEN
- pkg-string-const-select-dir — a package string select takes the caller's direction; BLOCKED (declaring-scope fold)
- pkg-task-stmt — same defect as `pkg-scoped-task`; DUP
- blocal-inert-falseloud — an inert block-local is refused (`check_block_local_scope_leaks`); BLOCKED (binding-resolved scope)
- scoped-call-comb-arg — a scoped call's initialised actual is a false E3001; DO-NOT-START (§5.2)
- blocal-collector-parity — `collect_block_local_decls_spanned` lacks timing-control recursion; latent, lands with that loud's removal; HELD
- dimquery-width — `$size` family and `$signed` loud in certified consumers (`param_decl_width_opt`); admit with their own census (`$signed` is its operand's width); 2 oracles; OPEN
- §3.11 — `function automatic` inlining names the operand twice; a purity predicate or codegen; OPEN
- static-local — a straight-line read-then-write static local is E3010; iverilog; OPEN
- frame-oob — a frame-local array OOB read lacks E4002; OPEN
- 2seg-call — `c.m()` left of an output-formal call is loud (`callee_body_cannot_touch`); iverilog; OPEN
- dyn-formal-pos — 10 dyn-formal call positions are loud (`hoist_dyn_formal_calls`); iverilog; OPEN
- pkg-default — a package default compares the importer's `tf_decl_scope`; iverilog; OPEN
- fgets-rhs — `return $fgets(line, fd);` is E3009; iverilog; OPEN
- V3/V4 — `wait(<frame-local>)`, `repeat(n) @`, frame-local NBA, `fork` in a task; iverilog; OPEN
- frame-array — multi-dim / offset / copied / `foreach` frame-local arrays; iverilog; OPEN
- V2A/V5 — dyn-array formals on automatic tasks, function `new[]`; iverilog; OPEN
- foreach-fn — function + `foreach` on a dyn formal; iverilog; OPEN
- r16-exec — recursion over dyn locals, non-bit-vector elements, unwritten outputs (§13.5.2); OPEN
- re-forward — a function re-forwarding its dyn formal (`dyn_array_actual_net`); iverilog; OPEN
- hier-task — hierarchical task output / array / string formals, static and generate cases; iverilog; OPEN
- array-formal — offset, OUTPUT / INOUT, 2-D, signed and task array formals; iverilog; OPEN
- blk-automatic — block-local `automatic` lifetime needs per-activation storage; OPEN
- hier-default-arg — a hierarchical call ignores formal defaults (`collect_callee_ports`); thread them like `with_default_arg_scope`; 2 oracles; OPEN
- iface-modport-formal — `w.mp()` on an interface port formal (`modport_call_refused`); BLOCKED (hier-call framing gate)
- hier-fn-inline-callee — a hierarchical call to an inline-lowered callee is E3009 (`hier_defer/func_call.rs` reads framed `hier_funcs` only); frame the callee or inline it in the instance scope; 2 oracles; OPEN
- genblk-fn-call — `gb.f()` into a labelled generate block is E3009; admit a generate-scope callee to the frame route; 2 oracles; OPEN
- misc-sub — `q.min()[0]`, `x.name().len()`, class-scope name defaults; OPEN

**System tasks & file I/O**

- plusargs-%0d — `$value$plusargs` rejects `%0d`; strip the width in `exec::plusargs::effect` (trap: `'0'` reads as `%s`); iverilog; OPEN
- writemem-local — `$writememh` of a task-local array is E3009; open the `read_task_net` seam with it (pin `writemem_targets_the_seam_cannot_own_are_refused_before_the_backend`); iverilog; OPEN
- filepos — `$ftell` / `$sscanf` E3009, `$fseek` skipped (§0-C A); iverilog; OPEN
- deferred-inline-action — a deferred non-print action runs at reach (`Scheduler::try_defer_with`, `prune_deferred_actions`); evaluate inputs at reach, dispatch at maturation; hand-IEEE §16.4.2; OPEN
- immediate-cover — `cover (c) stmt;`, `cover #0`, `cover final` are E2002; both oracles accept and drop the statement (§16.3 runs it); parse onto the assert lowering with a pass arm; SPLIT
- typedef-atom-cast — `ti'(…)` to an atom / `bit` typedef is E3009 (`simple_typedef_cast`); desugar to `CastTarget::Prim`, or a size cast + 2-state coercion; 2 oracles; OPEN
- $typename — enum / struct render as the base type (§20.6.1); OPEN
- %p-ⓐ — unpacked struct / `string sa[2]` are E3010 at declaration; verilator; OPEN
- %p-ⓒ — negative assoc key and real array renders are pinned divergences; HELD
- sformatf — `$sformatf` in a ternary / short-circuit / task argument (`format_args_str`); OPEN
- ext-shadow — un-isolated §4.11 items; RECORD

**Nets / timing**

- E3001-delayed — overlapping delayed tri-state drivers are E3001 (`check_whole_net_multidriver`); let `md_nets` resolve delayed drivers (probe at 1 ns); 2 oracles; OPEN
- E3001-overlap — same-range part-select drivers; BLOCKED (a per-bit driver map)
- hier-event — ``always @(`TOP.u.x)`` lacks sensitivity; a derived copy (`level_select_net`); iverilog; OPEN
- xproc-disable — a cross-process `disable` (no-op if not suspended, else loud); iverilog; OPEN
- timescale — partial-timescale diagnostics unwired (`rt.default_used`); OPEN
- nonansi-child-array — an instance array of a non-ANSI child is E3009 (`instance_array.rs`); read body `PortDecl` widths (pin `nonempty_nonansi_child_array_stays_loud`); 2 oracles; OPEN
- implicit-net-generate — an implicit net in a generate block is E3010; 2 oracles; OPEN
- modport-port-actual — `sub u(w.mp)` is E3002; verilator; OPEN
- iface-generate — a generate region in an interface is E3009 (`iface_inst.rs`); 2 oracles; OPEN
- level-select-var-index — `@(n[i])` is E3009; copy-net t0 rule for variable selects; 2 oracles; OPEN
- deep — t0 race, `@(*)` decl-init wake, runtime `==?`, modport directions, part-select force; OPEN

**Loud shapes surfaced by §4.5.493–495**

- ac-random-stmt — `$random(sd);` in `always_comb` is E3001 (§9.2.2.2); HELD
- readmem-write-table — `$readmem*` missing from `syscall_writes_arg`; HELD (until a design reaches it)
- real-cont-assign — `assign w = fr();` to a `real` is E3018; 2 oracles; OPEN
- real-fmt-hex — `%h` of a real is E3009; print the rounded value at 64 bits; 2 oracles (value); OPEN
- pkg-writer-import — importing a package whose function writes a package variable is loud; classify lazily; 2 oracles; OPEN
- fn-writes-pkg-var — a package function writing a package variable is loud; 2 oracles; OPEN
- class-real-members — real property, static method, method `for`, body enums are loud; verilator; OPEN

**Diagnostics quality**

- line-directive — `` `line `` is ignored: rail locations name the physical file; a `` `line `` table in the preprocessor, read by the span resolver; OPEN
- E3009-anchor — some E3010 / E3009 sites lack file:line (`diag::SpanResolver`); OPEN
- error_at — the anchor and `found` token differ; OPEN
- #9 — `velab -L` diagnostics have no location; OPEN
- cli-lib — `cargo test -p cli --no-default-features --lib` fails E0004 (a dev-dependency revives sim-engine's `oracle`); `default-features = false` on it; CI cannot see it; OPEN
- carrier-namespace — `T$w`-style carrier names are not reserved at declarations (`names_a_type_param_carrier` runs at override names only); refuse at every declaration site; 2 oracles; OPEN
- EXT2-DOC — stale CLI, language, system-task and explain docs; OPEN
- unique-if-chain — after §4.5.585 two residues stay silent where verilator reports `'unique if' statement violated`: (1) a chain in any function or task body (every subroutine body keeps the lone-`if` rule, `hdl-parser/src/functask.rs` `tf_body` sets `first_if_arm_only`; 111 lines in the §4.5.585 census by kind: task 44, item `function void` 30, item non-void function 17, class function 7, class void function 7, constructor 6 — e.g. lt_P lines 2 / 5 / 9, b02 fchain / tchain, m_void_rt, ifc_vfn_rt, pkg_vfn3_imp_rt, cls_task_rt, cls_fn_rt, cls_vfn_rt, cls_ctor_rt, n_ca_objf2_tbL, t2t_c_fg_this, u20_c_fnew_member, q6a_fn_vfn_noformal, q6t_pkg_scoped_const_chain); BLOCKED on §2 🆕 AB, `unique-const-fn` (itself BLOCKED on §2 🆕 AD) and `unique-pkg-closure`; (2) an outer series whose `else` is a qualified `if`: verilator continues it through `else unique0 if` and reports (vl_lines t3 / t5, b01 t1, f1c2_qual C2 / C7, q16_nested_ml t3, q16b_nested_ml2 t3, q23_prio_u0 t3, q5n_nested_spans t4), where IEEE 1800-2017 Syntax 12-2 makes the qualified `if` the final `else` statement (vita = PRE = IEEE; a split, not chased); also recorded: an inner `unique` / `priority if` after `else` reports at its own line where verilator names the outer line (vl_lines t2 / t4, q16_nested_ml, q16b_nested_ml2; PRE the same for a lone inner `if`); pins `crates/cli/tests/unique_if_chain.rs`; verilator; BLOCKED
- unique-const-fn — a constant function that reaches a synthesized no-match arm (a lone `unique if` / `priority if` with no `else`, a `unique case` with no `default`) is E3009 `… has no constant-fold arm` where both oracles fold silently (iverilog on `case`; it rejects `unique if`); `elaborate/src/const_fn.rs` `exec_const_stmt` has no arm for `$__vita_unique_violation`; §4.5.587 built a no-op arm under a positive opt-in (`ConstSite::Required`, set at IEEE constant-required consumers) and reverted it whole after two consecutive BLOCKING rounds (ER §3.6: a root returning through another door): it moved 48 of 444 cells (38 loud → oracle value; 10 silent-wrong → correct, §2 🆕 AC's `unique if` cells), but folding the `unique` form exactly like its plain-`if` twin inherits every plain-twin silent-wrong at those consumers — round 1: the instance-array prepass (d01, d02), package routine text (pkret, pkloc, pkfml, pkcast, pkimp), a 4-state default 0 (d10); round 2, past its exclusions (an instance-array element hold; a text-span rule over package text and routine-declaring generate levels): `$unit` routines, which `inject_cu_items` (hdl-parser `module_items.rs:342`) copies into every module (r24_unit_shadow, unitret), and a span key that staged compilation units do not share (`cli/src/staged.rs:171`); re-derive (ER §3.6) once §2 🆕 AD lands: a positive opt-in at constant-required consumers whose arm declines a run that reads a never-assigned 4-state variable (§2 🆕 AE, round 1's d10; an over-seed stays loud, never silent), never an exclusion by text span or enumerated scope; consumers that replace a run-time call (`repeat_unroll_count` `const_bound.rs:146`, `events.rs:1065`, `netdecl.rs:876` / `var_init.rs:127` via `ca_delay_rt.rs:71`) stay unstated, else they drop the report both oracles print at run time (c_rpt, e_trpt, h_tdly, h_cad, h_wd); 2 oracles; BLOCKED (§2 🆕 AD; the `case` half also §3.b `const-fn-case`)
- unique-pkg-closure — the package-scoped call closure walk `elaborate/src/package.rs` `pkg_stmt_pure_orig` (fn at :144, catch-all `_ => false` at :183) treats the synthesized `$__vita_unique_violation` as impure: a `pk::f(…)` call whose callee reaches a lone `unique if` or a `unique case` with no `default` is refused E3009 `package-scoped call pk::f(...) reaches pk::g, whose body names something outside its own formals/locals …`, naming a statement the user did not write, where both oracles run (q6t_pkg_scoped_const_chain_H, q6t_pkg_scoped_const_chain_case: iverilog `Time: 0`, `Time: 2`; verilator `[0] … pk.g`); `import pk::*` and a bare call run (q6w_pkg_import_const_chain_H: W4031 t0 ×2, t2); PRE the same; keeps `unique-if-chain`'s package bodies unarmed; 2 oracles; OPEN
- unique-if-text — W4031 says `… priority or unique case statement` on `if` forms too (verilator `'unique if' statement violated`); `parse_unique_priority` `warn_stmt`; pins `round29_report.rs`, `procedural_adv.rs`, `runtime_diag_location.rs`; verilator; OPEN

**Strings / heap**

- paren-select — `(p)[0]` of a string is silently 0 (`string_index_read`); OPEN
- real-part-write — `x[3:0] = …` on a scalar `real` is silently ignored; make loud; OPEN
- reduction-init — `$sformatf` over `arr.sum()` in a declaration is E3009; OPEN
- string-array — fixed string array init, runtime index, `string q[$]`, 2-D; iverilog; OPEN
- inline-string — a static task's inline string local is E3018; OPEN
- string-misc — `s[i:j]`, `s[i].len()`, string queue / assoc elements, `u.q[0]`; OPEN
- dyn-md-elem-select — a select into a md element of a dyn array is E3009; record the handle's packed extents (`netdecl.rs`), route via `lower_packed_read`; verilator; OPEN
- class-field-select — `c.u8[3:0]` is E3010; route the select's base through the class-field read; verilator; OPEN

**VCD / real conversion**

- vcd — cosmetic VCD encoding differences (widths, t0 dump, var kinds); iverilog; OPEN
- x→real — an x/z integral converts to real as 0.0 at every store and in `$itor` / `$sqrt` / `**` (`real_arg` is `to_i128_signed().unwrap_or(0)`); convert per bit (§6.12.2); iverilog; OPEN
- wide→real — an integer wider than 128 bits converts to 0.0; OPEN

### 3.c Intentionally loud (not gaps)

- §3.1 DPI-C and `export "DPI-C" function` — permanent non-goal
- `$value$plusargs` in an arbitrary expression — no statement form to keep single evaluation
- `%p` of `int a[0:0]` — the IR carries no array-ness
- a direction mismatch on an unpacked array port (`[0:3]` against `[3:0]`) — §7.6 pairs by position
- a fixed-array fill whose coverage cannot be proved (computed index, conditional write, incomplete set) — a rule
- calling the same dyn-formal function twice, or recursively, in one frame-body expression — one marker slot
- shadowing by a block that encloses a block and redeclares the same name, when NO module-scope net carries that name — two initialisations on one net
- a `$readmem*` child-versus-parent `initial` race — split
- `$readmemh` into a `wire` array — split
- a header default that names a constant from a body import — split
- two iverilog 13.0 defects (vita is IEEE-correct) — `break` after a loop local; `continue` in a `case`
- `%u` / `%z` and `%l` — documented choices

## 4. SVA / verification honest-loud residues

- `##0` fused with `##[m:$]`; BLOCKED (§16.9.2.1 discontinuity)
- N2c full sequence local variables; BLOCKED (nested-attempt data model)
- later-antecedent read and outer `|=>` skew; BLOCKED (2-cycle / cross-clock census)
- SVA-QUAD collapse default flip (`VITA_SVA_COLLAPSE`); BLOCKED (full-VCD golden audit)
- N4 clocking skew value (`output #0`); BLOCKED (hand-IEEE §14.11 / §14.16)
- class casts `Derived'(base)`, `(B'(d)).foo()`; BLOCKED (a `$cast` type guard)

## 5. Performance / hardening

Below the correctness ladder; a row resumes only when its re-entry condition fires. `§5.1-<x>` records: `git show acabe991:docs/history/ROADMAP_ARCHIVE_PHASE_A-D.md`.

### 5.a Standing verdicts

| id | verdict | reason | re-entry condition |
|---|---|---|---|
| codegen (cranelift), including the `§5.1-be` machine-code experiment | rejected | the call boundary (~38% of a run) exceeds the 8.9–11.3% ceiling; most `wprog` programs are one op | inline leaf loads and 2-state arithmetic without writing the semantics twice |
| D2-b two-state storage | rejected | a step down the accuracy ladder | a way with no correctness trade |
| cycle-based mode | rejected | event-driven already skips ~90% of combinational work | real demand with ≥1 evaluation per block per cycle |
| levelize (rank-ordered Active drain) | discarded | 1.00×; the dirty settle closed the root | none |
| process fusion (the E axis) | not adopted | a chain reader sees the propagated value (silent-wrong) | none |
| net-count reduction (flatten) | on hold | erases what `--probe`, VCD, `%m` and hierarchical names target | an owner ruling |
| S4 schedule elision · S5 NBA specialisation | stopped | below the 1.3× stop threshold | none |
| the `wprog` reject family inside settle | not started | ~2–2.5% of serv; needs `Tern` as a jump | prize exceeds the machinery |
| `drain_range_diags` early-out | rejected | zero gain | none |

### 5.b Open performance and hardening residues

- 4b-r — native does not pool scratch (`fire_waiters`, `settle_cont_assigns` `vals`); OPEN
- 5c — a frame body never enters `wprog` (`Vec<Value>` window; `arena.frame` declines); a flat word window; BLOCKED (arena)
- ARR-LHS — md element LHS ~10× cliff (= §2 row 19); BLOCKED (its own census first)
- INLINE-FOLD — the inline fold is exponential (DAG walked as a tree); per-activation memoisation; OPEN
- MEM-GUARD — no process memory guard; RSS watchdog + `--max-mem`; BLOCKED (macOS FFI is unsafe)
- EXEC-ROWS — `native::run::executor_rows` rescans per `simulate`; HELD (cache if it matters)
- DELAY-CLAMP — `#delay` above u32::MAX is clamped (frozen u32); bump or announce; OPEN
- KPRED-3RD — tier-3 lacks a "kernel can run it" layer; HELD (with dispatch)
- QUIESCE-NBA — tier-3 quiescence ignores `delayed_nba`; BLOCKED (S1d-4c-2)
- BYTE-GATE-6 — six known differences meet the S1d-4d byte gate; decide each pin; OPEN
- MON-RENDER — tier-3 refuses `$monitor` / `$strobe` (`flush_postponed`); BLOCKED (S1d-4c)
- FD-EOF + FEOF — `fd_eof` X-poison hole; `$feof` over-marked (`sysfunc_is_stmt_effect`); OPEN
- NETSLOT-PREV — `NetSlot.prev` is written but never read; OPEN
- ELAB-PHASE-BLIND — the corpus cannot see front-end cost; add a front-end-bound row; OPEN
- LOW-ROI — FMT-CACHE b, GEN-3X-STR a, QUEUE-MID-ON; HELD
- T0-SERIAL — the engine scheduler follows each serial t0 comb pass (§4.5.584) with a propagate that scans every waiter: comb5000_settle interp +9.7% (vm shares the scheduler, unmeasured; native +0.8%, ibex +0.7%); `sched/run_loop.rs`, `sched/propagate.rs`; HELD (native, the default, is flat; ER §10.2 keeps the reference interpreter out of performance work)

### 5.c Current state

Default backend `native` (tier 3); product `--no-default-features` = one executor; the corpus runs blocking in CI on 3 OSes; codegen off.

## 5.2 Queue (start order)

Canonical start order; LOOPROMPT's NEXT mirrors it and this table wins.
One iteration takes ONE row (single root; an oracle, a corpus row's pinned oracle counting; outside walls and oracle splits), plus at most one review-free hygiene item. A row that needs a format bump takes it.
Same-root rows are one slice, and their order is a measurement (a value lane lands before a guard over an unlimited fold is deleted).

Real-design first: every row has a corpus witness except rows 1–8, which an external report or its fix path reproduced. A slice that moves a corpus page re-pins that row's refusal in the same commit (`DRIFTED` otherwise) and reports the page before and after.
Incoming reports pre-empt the queue; reproduce every item at HEAD first.

The `rank` column is a row's defect class — ① a silent-wrong, ② loud→supported (§1) — not its place in the queue: the row number is the start order.
Rows 1–3, what the external report's fix path found (§4.5.583, §4.5.585, §4.5.587), go before the ① rows 4–7 because the owner's order of 2026-10-02 keeps the report's items first. §4.5.587 reverted §3.b `unique-const-fn` (now BLOCKED): folding the `unique` form like its plain-`if` twin inherits the twin's silent-wrongs; row 1 closes the prefix-binding half (§2 re-entry: that row's fix path). The 4-state half is a rule, not a row: §4.5.588 measured that the interpreter's 0 for a never-assigned 4-state variable cannot be fixed yet (§2 🆕 AE, BLOCKED), so every arm or lane a later row opens from loud to a value declines a run that reads one; row 3 widens the interpreter that way and waits on row 1; `unique-const-fn` is re-derived from its row once row 1 lands (ER §3.6). Row 2, with `unique-const-fn` and §3.b `unique-pkg-closure`, blocks §3.b `unique-if-chain`'s subroutine-body residue. Rows 4–7 are §4.5.581's prerequisites: none has a corpus witness and each redesigns shared code. Row 8 (§4.5.588) unblocks only 🆕 AE's `repeat` cut, and no report item waits on it, so it follows rows 4–7.

| # | slot | item | source | rank |
|---|---|---|---|---|
| 1 | 1 | §2 🆕 AD — routine text (package and `$unit` routines' ranges, defaults and inlined bodies, a generate-scoped routine, the instance-array prepass) folds at the caller's prefix, so a call in it binds the caller's same-named function (pk_rt_ret, r24_unit_shadow `232`, d01 `p=5`; both oracles `8`, `p=a`); first: census every lane that folds or lowers another scope's text, then fold each where it is declared (the declaring-scope fold, REMAINING_WORK §D), never an exclusion by text span or enumerated scope | §4.5.587's review (`unique-const-fn`'s fix path) | ① |
| 2 | 2 | §2 🆕 AB — a function reached from a continuous assign runs at t0 on x before the `initial`: W4031 / E4003, exit 1 where both oracles are silent; first: census the t0 CA evaluation, then an after-first-batch evaluation on 2 oracles | §4.5.585's fix path | ① |
| 3 | 3 | §3.b `const-fn-case` — every `case` form in a constant function is E3009 where both oracles fold (probe p2); after row 1: a `Stmt::Case` arm in `exec_const_stmt` that sizes the case expression and every item once (ER §2.4) and declines a run that reads a never-assigned 4-state variable (§2 🆕 AE), then census the newly folded consumers against their plain-`if` twins | §4.5.587's grounding | ② |
| 4 | 4 | §2 🆕 U — a key holds two bindings and lanes read different ones (A1D, B2D, q1g wrong; A1S, B2E, X1 held); writers `bind_param_value`, `package.rs` import arms; readers `wide_name_bits`, `lookup_scoped`; first: census every writer of `params`, `wide_param_bits`, `str_param_raw`, `real_param_val`, then one current binding per key at the binders (a genvar suspends the wide entry), never a reader-side guess | §4.5.581's review | ① |
| 5 | 5 | §2 🆕 V — a generate-case arm is re-decided per `GenPhase` walk (`generate.rs` `GenItem::Case`, `instance.rs`), so a forward label mixes arms (d1p, C1); first: decide the arm once per construct instance (key: scope prefix + span) and resolve forward labels alike in every phase (caching alone leaves d1p wrong) | §4.5.581's review | ① |
| 6 | 6 | §2 🆕 W — a constant typed by an overridden type parameter loses the override's sign (PT5c `PV < 0` → 0, dF3 `TP = '1` → 15; oracles 1, -1); first: measure §4.5.479's and §4.5.483's pins on HEAD, then the two declaration lanes | §4.5.581's review | ① |
| 7 | 7 | §2 🆕 T, then 🆕 S (a) — §4.5.581's designs (branch `fix/gencase-label-domain`): labels compared two ways in the bit domain, decided where they agree (`d4dc9c26`); then the i64 half with every decline falling back to PRE's compare (`eb9d3b69`), then the wide half; held cells `generate_case_and_wildcard_prerequisites.rs` | §4.5.580's fix path | ① |
| 8 | 8 | §2 "Size cast" — `typedef enum bit [7:0]` stores 4-state (the parser records `enum bit [N]` as `Logic`, `typedefs.rs:351`; d01rt `n=2`, d05rt `n=0`, both oracles `n=1`, `n=2`), then the base-less enum return beside it (d07rt `n=0`, both oracles `n=2`); first: census every producer of an enum declaration's and return's recorded kind (`TypeInfo`, `ret_two_state`, struct member kinds) and every run-time reader; unblocks §2 🆕 AE's `repeat` cut | §4.5.588's step 0 (🆕 AE's fix path) | ① |
| 9 | 9 | new-design census: OpenTitan IPs (Apache-2.0), VeeR EL2 / EH1 (Apache-2.0), alexforencich verilog-axis / -pcie / -uart / -i2c (MIT) — licence and oracle first, then one corpus row each (Solderpad stays out: owner ruling); each admitted page becomes queue rows | corpus | — |
| 10 | hygiene | split production files over the 1,000-line policy not on its exception list (largest: `native/kernel.rs`, `elaborate/params.rs`, `elaborate/packed.rs`); list them with `wc -l` at HEAD; precedent: a lane in a sibling module (`pkg_body_scope.rs`, `inline_bind.rs`); never inside a correctness bundle | [ENGINEERING_RULES.md](ENGINEERING_RULES.md) §10.1 | — |

Do not start:

- §2, every row — frozen (§2 preamble) until a corpus hit, a corpus fix path or an external report.
- §3.b `pkg-type-param-import`, `scoped-call-comb-arg`, the mixed-caller callee, `m #(8)` / `defparam u.T$w`, the VCD `$scope` `[0]` spelling, the `genblk<N>` collision, §2 🆕 L ⓦ / 🆕 N residues — synthetic origin, no corpus witness.
- §2 row 16 — an oracle split on the override's context.
- §2 row 34 (one oracle, zero demand) and row 31 (correct; performance).
- §2 row 32 — the `$finish`-in-a-function boundary is a split.
- §2 🆕 Q — needs a block-scoped constant binding (a bare hoist makes 5 silent-wrongs).
- §2 row 10's bound half — needs the declaring-scope fold.
- §2 row 7's remainder — hierarchy start order and wake-group order are splits.
- §3 ⑤ⓕ ARITY — no per-instance declarator dims (`decls.rs` stamps once); needs an arity marker on `Dim` / `DeclName` (SchemaHash roots).
- §3 ⑤ⓕ CLASS PROPERTY — needs per-instance class registration (`register_classes` prescans).
- §3 ⑤ⓕ function RETURN type — 1 oracle; needs unpacked dims on `FunctionDef` (hdl-ast re-pin).

Oracle-split axes (§2 "Oracle splits") are never chased.

## 6. G2 — the AI-agent observability track

SPEC = [preview/19-ai-agent-observability.md](preview/19-ai-agent-observability.md). Teeth = a 3-way internal differential (JSONL ≡ VCD ≡ `$display`), a determinism golden, and an asymmetric mutation for anything that reports. A wrong log ranks with a silent-wrong. Ranks third, behind §2 and §3.

| stage | deliverable | size |
|---|---|---|
| OBS-2 residue | `sva.jsonl` (R-L6) · per-element array probe · real / class / event probe | M |
| OBS-1 residue | staged obs · compile-fail manifest · `--seed` · `run_id` · `-G` in `run.json` · `-D` in `source.blake3` · `results.jsonl` v2 · R-L2 `fail/*.json` · R-L5 SVA / cover counts | S-M |
| R-L4 | log-channel separation | M |
| OBS-4 | `vrun --control stdio` JSON-RPC plus a poke journal | L |
| OBS-5 | snapshot / restore / rewind | L-XL |
| OBS-6 | X-origin · region-annotated events · a static backward slice | L+ |

Open items beside the staged track:

- CALL TREE (doc-19 §4.9 item 1): an inlined subroutine reports 0 calls; BLOCKED (two lowering paths, §3 ⑭)
- `subroutines` files `function void` as `"kind": "task"`; OPEN
- the frame / inline route is per `(module, name)`, so one package function shows two routes; OPEN
- per-call-site builtin rows are not emitted; OPEN
- `--hier-tree` / `--inst-paths` are dropped silently on a staged run (not in `reject_obs_dir`); OPEN
- `--hier-tree` collapses generate scopes (§2 🆕 N); OPEN
- doc-19 §3 pin 4 enum values as names: no name path; OPEN
- R-I1 introspection is partial; R-I2 transaction log has no producer; OPEN
- the `builtins` table counts lowering artifacts beside user calls; OPEN
- six `wprog` keys have no producer; `index_range` counts one spelling; OPEN

Non-goals: FSDB / UCDB, an embedded SQLite, a waveform GUI, UVM. VCD stays the human-facing format.

## 7. Conditional / long-term (promoted only when the re-entry trigger fires; orthogonal to correctness)

| id | item | trigger |
|---|---|---|
| BACKEND | 2-state mode (rejected) · PDES BSP · native-eval residual lane · cranelift JIT (rejected) | W≥64 with grain ≥200 ns · low ROI · §5.a codegen |
| VHDL | a VHDL front end (GHDL oracle, E7xxx) | an SV plateau, a value-domain decision, GHDL setup |
| VCD-EXT | `$dumpports*` | waveform-tool demand |
| MVP-CUT | string concat outside assignment · `[*]` index · package import residue · cross-frame `disable` | individual demand |

## 8. Non-goals (permanent, not gaps)

- IMPLICIT-NET (explicit `E3010`) · out of scope: synthesis, a waveform GUI, UPF / SDF / DPI-C, `shortreal`, `trireg`, UVM, `unique` / `unique0` multiple-match checking (`VITA-I2021` announces it, once per parse).
- `defparam` stays direct-child with a constant value; deeper or non-constant is a loud refusal. Tracked: §0 row 14-b, §3.b `defparam-iface`.
