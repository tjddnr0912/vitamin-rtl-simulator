# ROADMAP — open work

Open work only, one line per item: `id — symptom (headline cell); code site; fix shape; oracles; STATUS`. Grounding re-measures every claim (LOOPROMPT §1), so a line keeps only what a re-measurement cannot recover: the id, the code site, a held design's branch or commit, a named prerequisite.
A finished slice's record is its commit message, anchored `§4.5.N` (bug, mechanism, byte-identity, review): find it with `git log --grep '§4.5.N'`; entries up to §4.5.582 are in `git show acabe991:docs/history/ROADMAP_ARCHIVE.md`. Done = delete the closed line(s), add each new find as one line in its section, recount the Summary.
Section numbers and row ids are stable, never renumbered or reused; source comments cite them (`ROADMAP §2 🆕 I ⓐ`, `§3 ⑭`, `§3.b <slug>`, `§5.1-be`). No gate or test counts in any doc: CI on 3 OSes is the record, and `format_version` is `crates/vita-artifact/src/header.rs:15`.
STATUS: OPEN = startable (two oracles, or one plus hand-IEEE, and no unmet prerequisite). Nothing else starts: BLOCKED (a named prerequisite, listed in [REMAINING_WORK.md](REMAINING_WORK.md) §D), WALL, SPLIT (oracle split, never chased), HELD (deliberate or trigger-gated), RECORD (observation), LOUD, PERF, DEFERRED, DO-NOT-START, DUP.

## Summary

Recount from the lines below in every docs step; the iteration report shows this table. `open` = the section's top-level lines (DUP not counted), `startable` = OPEN, `blocked` = the rest. `next` = where §5.2's start-order rows sit (`1` is the next slice); §5.2 orders rows counted in their own sections.

| § | track | open | startable | blocked | blocked by (top reasons) | rung | next |
|---|---|---:|---:|---:|---|---|---|
| §2 | silent-wrong start-order rows | 37 | 7 | 30 | named prerequisite 15 · oracle split 5 · loud, zero demand 3 · held 3 · performance 2 · do-not-start 2 | frozen; 🆕 AH, AG, AF, W, T, S queued | 1, 3, 4, 6, 7 |
| §2 | recorded defects by mechanism | 212 | 111 | 101 | oracle split / no oracle 59 · named prerequisite 19 · record only 8 · WALL 5 · held 5 · performance 4 · filed to §3 1 | frozen; the "Scoping" import line and two "Size cast" enum lines queued | 2, 8 |
| §2-N | verilog-axi census | 5 | 0 | 5 | oracle split 2 · held on purpose 2 · upstream fst-writer 1 | ① | |
| §3.a | loud → correct-support, numbered | 24 | 19 | 5 | loud by design 2 · named prerequisite 2 · deferred to §5 1 | ② | |
| §3.b | loud → correct-support, small | 139 | 116 | 23 | named prerequisite 11 · held 4 · record only 4 · oracle split 2 · do-not-start 2 | ② | 5 |
| §3.c | intentionally loud | 12 | 0 | 12 | by design or oracle split 12 | — | |
| §0 | promotion queue (T2 residues) | 14 | 8 | 6 | oracle split 3 · deliberate 2 · `defparam` non-goal 1 | ③ | |
| §4 | SVA honest-loud | 6 | 0 | 6 | named prerequisite 6 | ③ | |
| §6 | G2 observability (OBS) | 6 stages + 10 | 15 | 1 | call tree: two lowering paths 1 | ④ | |
| study/03 | workload corpus | 1 | 1 | 0 | — | real-design | 9 |
| §5.b | performance / hardening | 18 | 8 | 10 | named prerequisite 6 · held or trigger-gated 4 | below the ladder | |
| §7 | conditional / long-term | 5 | 0 | 5 | trigger-gated 5 | trigger-gated | |
| §8 | non-goals | 2 | 0 | 2 | permanent 2 | permanent | |
| total | | 491 | 285 | 206 | | | |

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
- ⑨ a continuous assign's function at t0 — iverilog calls `f2(a | 1'b0)` and `f3(av[0])` on x first (`f2 t=0 x=x`, `WARNING: …: value is unhandled for priority or unique case statement` `Time: 0`), but `f1(a)` only on the written value (`f1 t=0 x=0`); vita calls all three once on the value (= verilator; `t0_call_cont_assign_hold.rs` `an_expression_argument_is_not_called_on_x`)

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
  - retry (§5.2 row 6, one slice with 🆕 W after 🆕 AI): the i64 half (`eb9d3b69`, branch `fix/gencase-label-domain`) with every `const_wildcard_i64` decline falling back to PRE's compare and a left operand holding a call kept on PRE's compare (🆕 AE: A1–A3, xADD, xTRN, xSYS); §4.5.593 ran §4.5.580's 666 variants on that cut: 0 correct→other per label; the parameter channel (A5, xDP, xPT: PRE `PA=-4`, 3 oracles `x`) is 🆕 AE's; re-run round-3 cells, MC4, the 174-cell matrix; the wide half (LP, st1, two compares under `||`: S9) after 🆕 T, U (§5.2 row 7)
- 🆕 T — a generate-case label the i64 fold cannot read is skipped; a read one compares untyped (T1, L06, S06, X13 take `default`; N09 inconsistent); an unbound label, which §26.3 makes illegal, is skipped too: undeclared (Z5g `NOPE:` `def 9 bits=4`; iverilog and sv2v ``Unable to bind parameter `NOPE' in `top.gb'``, verilator `Can't find definition of variable: 'NOPE'`) or declared later in the block with no outer object (V14, V27, V30b, Z2c, Z3a `def 9 bits=4`; iverilog `Unable to bind parameter`); `generate.rs` `GenItem::Case`; BLOCKED (🆕 U's held half; 🆕 V, whose §4.5.592 attempt was reverted)
  - design (§4.5.581, `fix/gencase-label-domain`, `d4dc9c26`): decide once per instance; compare PAIR and WHOLE (§12.5) in the bit domain, decide where they agree, else i64, else no match, never refuse; residue: a label no bit fold reads (`case (6'sh30) W6'(P8)`), and an unbound label, which must refuse, not take no match (§4.5.592's round-3 replay reached this skip: R3q, 🆕 V); held `generate_case_and_wildcard_prerequisites.rs`
- 🆕 U — the wide→narrow half (U-b; §4.5.591 closed the import half): a genvar (and its generate-routine replay), an enum label (module, package, a bare generate region) or an explicit narrow import (over a wildcard or `$unit` >64-bit constant) rebinds `params` and leaves the key's >64-bit / string entry, which `bare_ident_route` and `wide_name_bits` read first (A1D `i=18446744073709551625 K=9`, B2D `E1=18446744073709551616 K=0`, Egr, Iwn_val `P=18446744073709551625 K=9`, Icue_wn, Ghr, Gnr_fn; 3 oracles `i=0 K=0` / `i=1 K=1`, `E1=1 K=1`, `P=3 K=3`); writers `generate.rs` genvar arm, `frames_reserve.rs` `with_rtn_decl_scope`, `instance.rs` `bind_enum_labels_of`, `package.rs` explicit narrow arm; fix: take the key's wide / string / real entry (and `hier_param_range`, the replay's `param_range`) around a genvar and put it back after, labels in the wildcard skip set, an explicit narrow removing the losing wide and its `param_meta` (§4.5.591's prototype: 42 grounding cells silent→correct, 5 loud→value = oracles); pins `import_one_binding_per_key.rs` `held_*`, `must_stay_loud_*`, `generate_case_and_wildcard_prerequisites.rs` `p1_a_genvar_under_a_wide_constant`; 3 oracles; BLOCKED (🆕 AE: the corrected narrow value reaches the constant interpreter where PRE's wide one was refused, through an override or a localparam alias, so no opt-in contains it: Gw_cae `W=0` / `W=1`, Iwn_kae `W=3`, Gw_kae; 3 oracles `W=x`)
- 🆕 V — each of the four `GenPhase` walks (`instance.rs` Nets, VarInit, Logic, Instances) re-runs `generate.rs` `elaborate_gen_item` (`GenItem::Case`, `elaborate_gen_if`, `GenItem::For`), and a generate-scope parameter is bound at its position in every walk and never unbound (`generate.rs` `(_, ModuleItem::Param)`), so a walk after Nets reads the previous walk's inner binding where Nets read the outer object or none, and a forward reference mixes arms (nets from the Nets arm, processes and instances from a later one): d1p `k 8 bits=4`, V04x `k 8 bits=6`, I4 `then`, F3 `L0 L1 L2`, Z0c `local 8 bits=4`; iverilog refuses d1p, prints `one 1 bits=6`, `else`, `L0`, `pkg 9 bits=4`; §4.5.592 recorded the Nets decision per construct instance (prefix, kind, span), verified it in each later walk and was reverted whole after three rounds, every mismatch policy failing: an E3010 on any mismatch refuses legal designs PRE prints right (DL01, DL06, DL08 `@bits=4`, DL03 `@Q=10`, DL11 `@w=9 bits=4`, = iverilog; refusing only where an arm builds something still refuses R3a, R3b, R3m); a silent replay where the Nets walk read every input exposes 🆕 AE (Z1g: PRE loud, replay `P=0000`, iverilog `L0 w=10 P=xxxx`); a silent replay where neither arm builds anything misses that every walk re-binds the arm's parameters (SN6: iverilog, sv2v, verilator and PRE `P=1`, replay `P=9`) and drops the range report each walk makes (R3q: PRE and 3 oracles refuse, replay `@bits=4`); fix: bind each generate level's own parameters by position in every walk as Nets does (the "Scoping" later-walk reads line), so the walks decide alike by construction; iverilog + hand-IEEE §26.3 (sv2v and verilator are no oracle here: "Oracle splits"); BLOCKED (the "Scoping" later-walk reads line; 🆕 AE for a decision it moves: Z1g, and AE5–AE9 under a scope-wide pre-bind)
- 🆕 W — a constant typed by an overridden type parameter reads unsigned (PT5c `PV < 0` → 0, dF3 `TP = '1` → 15); measure §4.5.479 / §4.5.483's pins first; one slice with 🆕 S (a)'s i64 half, since each alone descends (§4.5.593: W alone 24 correct→loud and 9 correct→silent-wrong through `const_compare_special`'s non-negative-only masked arm and 🆕 AC's sinks; with the i64 half 0 correct→other on the 666-variant harness, but a T-typed constant W makes signed meets 🆕 AI: Tv12 PRE and 3 oracles `L=252 M=1 K=1 vb=5`, the cut `L=4294967292 M=0 K=0 vb=4`; W alone: p11 17 correct→wrong, p8 8 correct→loud and 4 correct→wrong); its array container (`parse_array_param`) also needs 🆕 AI's unpacked-element arm (W3, W6); 3 oracles; BLOCKED (🆕 AI, itself BLOCKED on 🆕 AE since §4.5.594; ships as §5.2 row 6)
- 🆕 X — a class method's `case` re-evaluates a call scrutinee per label (f2); `class_lower.rs` / `frames_reserve.rs` reserve no case temp; reserve them, then let `lower_case_inside` take it (pin `case_inside_refused.rs` `l14_…`); 2 oracles; OPEN
- 🆕 Y — a string-returning call scrutinee compares packed (s2 `m=0`, verilator 3); `ir_expr_is_string` (`strings.rs`), `hoist_case_scrutinee`; a string-kind capture (pin `case_inside_refused.rs` `l09_…`); verilator + hand-IEEE; OPEN
- 🆕 AA — a consumer `always_comb` / `always_latch` written before the block feeding it, in a chain no time-0 process write reaches (declaration initializers, a constant block, a settled CA), reads x in its time-0 pass and reports W4031 / E4003 at t0 where both oracles are silent (q_chaind_A_B, q_chain3_A_Bc_Bb, s583_a08, x13b, tg2, tg5, t3j; PRE the same; and aa1_chain, an `if` chain, §4.5.585: vita W4031 at t0 and t5, verilator silent, iverilog on the `unique case` twin s583_a08 `Time: 5` only); the passes run one per batch in source order (§4.5.584: `sched/run_loop.rs`, `native/run.rs`); closing it needs an order among the time-0 passes, and the oracles are silent by different ones (iverilog reverse source order, which reports the same consumer-first read at the source's next change and reports forward chains at t0; verilator dependency order); every order measured moves split cells (§4.5.584 grounding: reverse 3 to iverilog, dependency order 4 to verilator); SPLIT
- 🆕 AB — residue of §4.5.590's time-0 hold (`sim-engine/src/sched/t0_hold.rs`: an assign reaching an effectful callee or reading a class handle, and every assign reading its output, waits for the first t0 batch, then is released in dependency waves): (a) a held UNCERTIFIED assign (in `ca_always`: an impure callee such as `$random`, a class-handle read, a delayed or multi-driver assign) is re-run in every pass of every later wave, so a held chain of them makes extra t0 calls (f07, a 6-link `$random` chain: PRE 30, vita 42, both oracles 6, and the next printed `$random` moves with the draw count: PRE -887079274, vita -445445430, iverilog -1295874971) and repeats calls on settled values (x3 tristate 4, both oracles 1; d19 delay chain 8, iverilog 1, verilator 3; e01 5, both oracles 1); its cost is §5.b T0-HOLD-CHAIN; after t0 such an assign still re-runs on every settle pass (the "Diagnostics" re-run line: n_ca_objf2_tbL_H, t2t_c_fg_this_H, u20_c_fnew_member_H ×3 at time 2, verilator ×2), which holds §3.b `unique-if-chain`'s subroutine-body residue; (b) waves are ordered per net, so links sharing one vector or array net run as a cycle (d18, six generate instances through `o[k]`: 18 calls, both oracles 6; e02 `c[k+1] = f(c[k])`: 9, iverilog 10, verilator 13); (c) a held cycle is released whole with everything below it, so a downstream link runs on a held net's `z` (k01 `f6 t=0 v=z`; iverilog `x`, verilator `0`); (d) a declined callee's body reads are not in `ca_deps`' read set (`levelize/mod.rs`), so its wave can come before the held driver of a net that body reads (r2h2 `f2 t=0 x=1 w1=z`, then `w1=0`; both oracles once, on `0`; PRE ran it on x first); (e) a `$fatal` latched by an assign's call does not stop the rest of that t0 settle (k06; PRE the same); fix for (a): re-run a held uncertified assign only in its own wave when its read set is complete (no declined callee in its calls), per wave otherwise; §4.5.590's round 4 re-ran every one only in its own wave (f07 12 calls, r_chain = PRE) and was reverted: through a declined callee's body read the value stayed stale past t0 (r4a `W2 t=1 w2=0`, r4c, r4d, r4g, g06 `ev u=0 t=1`: events at t ≥ 1 in neither oracle); pins `t0_call_cont_assign_hold.rs` (residue and race pins); 2 oracles; BLOCKED ((d): a declined callee's body reads in `ca_deps`' read set)
- 🆕 AC — a call the constant interpreter declines, where vita needs a constant (a count, a width, a bound, a label), takes a fallback at exit 0: an empty replication (probe p6 `r1=00000000 r2=00000000`, a `$display` body and a plain `case` body; c_rep_case_M; `{f(2){"ab"}}` into a `string` prints `s=`: e02, t16), a 1-bit `+:` (e_pswr_case_M `pw=1`), a 1-element `[m:l]` (h_prd_case_M `pr=1`, h_mdpr_case_M `m=1`, h_plsb_case_M `v[11:f(2)]` `pl=0`), the generate-case `default` (h_gcl_case_M, h_gcl10_case_N), or an unmasked value when the call is in an interpreter formal or local range (pk_ce_fml, pk_ce_loc `P=1000`: `const_fn_width.rs` `const_decl_wsign` declines a bound holding a call); iverilog and verilator `r1=0000007f r2=0000007f`, `s=abab`, `pw=7f`, `pr=ff`, `m=89abcdef`, `pl=1e`, the label, `P=8` (19 §4.5.587 grounding cells + p6); sinks `const_bound.rs:185` `lower_const_width_expr` (keeps the lowered call when `const_bound_u32` declines; the engine's shallow fold reads `unwrap_or(0)` / `unwrap_or(1)`), `packed.rs:1890` (also :2204, `packed_inner.rs:152`), `generate.rs:459` (a label that does not fold is no match, 🆕 T's sink; h_gcl_case_N is right by that accident); the same sinks take an x-valued compare (PROBE_CATALOG §4.5.580, §4.5.581); the `[m:l]` LSB lowers as a run-time call and reports W4031 at time 1 where both oracles are silent (h_plsb_case_M, `packed.rs:339`); close upward first (ER §2.5): the 19 cells and p6 fold once §3.b `const-fn-case`, `const-fn-systask` and `unique-const-fn` land (its 10 `unique if` cells are `unique-const-fn`'s; e02 / t16 only if it states a string replication count, whose multiplier IEEE lets be non-constant), pk_ce_fml / pk_ce_loc once `const_decl_wsign` folds a bound holding a call (it declines one so `bit [f()-1:0]` inside `f` cannot recurse) in the declaring scope (§3.b `pkg-text-open`: §4.5.589 binds a package routine's text to its package only where the caller's fold answers); then re-measure what still declines at each sink; re-entry: `unique-const-fn`'s fix path (§4.5.587); 2 oracles; BLOCKED (§3.b `const-fn-case`, `const-fn-systask`, `unique-const-fn`; the interpreter ranges also §3.b `pkg-text-open`)
- 🆕 AD — routine text folded or lowered at the caller's prefix binds a name in it to the caller's same-named function or constant; §4.5.589 closed the package-routine half where PRE's fold answered (`elaborate/src/decl_scope.rs`: return, formal and local ranges, defaults, the frame reserve, the inline lane, `$bits(q::h())`, call typing and a constant call in a package routine's body bind the declaring package's); left here: a static package task's body-locals are not armed (`inline_task_locals.rs` `hoist_inline_task_locals`: m10_itask_par `logic [W:0] t` `v=232`, c23_task_blk, a block-local, `v=255`; iverilog, verilator, sv2v `v=8`, `v=15`); arm them as `inline_task` arms the formals; a routine-local enum label in the routine's own range keeps PRE's caller binding (`decl_scope.rs` never probes a declared name: m5_enum_lbl_rt `v=232`; verilator `v=0`, the label; iverilog `v=8`, the package's; sv2v refuses; a split, not chased); the other halves are rows: `$unit` routines 🆕 AF, generate routines 🆕 AG, the instance-array prepass 🆕 AH, an imported routine's origin (the "Scoping" import line), the text PRE refuses (§3.b `pkg-text-open`); rv558_ovr's `$bits(P)` 32 (oracles 4) is the "Index sealing" override-leaf line, w05615 / w05639's `P=1000` (oracles 8) 🆕 AC's interpreter ranges; a package in another staged compilation unit is refused (PROBE_CATALOG §4.5.589); 3 oracles; OPEN (the task half)
- 🆕 AE — the constant interpreter reads a never-assigned 4-state variable as 0 (its env is `BTreeMap<String, i64>`, its result `Option<i64>`): the return variable, seeded by `const_fn.rs:1507` `env.entry(name).or_insert(0)` (x2a `P=0000`, d10 `PX=0000 PI=0`, x2m / x2n beside a folded sibling: verilator `xxxx` / `x`; iverilog refuses `Unable to evaluate parameter P value: top.fx(32'sd2)`), and a 4-state local declared without a value (`:1335`; x2c `P=0000`, x2i `integer` `P=0`, x2l after a bit write `P=0001`: iverilog and verilator `xxxx`, `x`, `xxx1`); a bound read from it binds (x2k `b=1`; both oracles refuse the design); a known result read through it is wrong too (m19 `repeat (fd(2))`, `fd` testing `t == 4'd0`: `n=1`, iverilog and sv2v `n=2`, vita's own run time 2, m25); IEEE 1800-2017 §6.8 gives x; fix: carry an x plane to each binder, which converts it (2-state), refuses it (≤64-bit 4-state) or holds it (>64-bit); every narrower cut descends (§4.5.587, §4.5.588): a decline on every read (r08 `t & 4'b0000`, `t === t`: both oracles `0000`, `1`) or on an x-bearing result (13 correct cells loud; s01 silent through 🆕 AC's sinks); a 4-state re-run keeping only a fully known result breaks a cancellation (p01 `localparam int Q = fd(2) - fx1(2);` PRE and iverilog `Q=0`, re-run `Q=1` by construction) that escapes through a parameter (`R = P - fx1(2)`), so no opt-in contains it; sending a `repeat` count whose fold read such a variable to the run-time loop (`repeat_unroll_count`; the loop runs an x count 0 times) moves 8 cells silent→correct (m19, p06, p15, p16, p19 to `n=2`; p07, p13, p14 to `n=0`, hand-IEEE §12.7.2) but also seeds `enum bit [N]` locals and returns, which the parser records as 4-state `Logic` (`hdl-parser/src/typedefs.rs:351`, `functask.rs:201` `ret_two_state`) and vita's run time misreads (e01 `n=1`→`n=2`, e05 `n=2`→`n=0`; d11, a base-less enum return beside a seeded local, `n=2`→`n=0`; both oracles = PRE); until it lands, a lane a row opens from loud to a value declines a run that reads a never-assigned 4-state variable (an over-seed stays loud, never silent); holds 🆕 U's wide→narrow half (§4.5.591: Gw_cae, Iwn_kae, Gw_kae), 🆕 V's moved decisions (§4.5.592: Z1g) and 🆕 AI's newly sized regions (§4.5.594: C1, Nq_launder, Apg), and through 🆕 AI 🆕 W; re-entry: `unique-const-fn`'s fix path (§4.5.587); 2 oracles (locals), verilator + hand-IEEE (return variable); BLOCKED (§2 row 15's 2-state identity; 🆕 AC's sinks refusing an x-valued call; a 4-state result channel from the interpreter to module-scope folds; the `repeat` cut: the "Size cast" enum lines)
- 🆕 AF — a `$unit` routine's text folds and lowers at the calling module's prefix: `inject_cu_items` (hdl-parser `module_items.rs:342`) copies unit items into every module and drops the unit routine where the module declares the name, so a call or constant in its ranges binds the module's (u_ret_fn `v=232 P=232`, u_tdw `x=ff b=8`, r24_unit_shadow, rv558_unit `P=255`; iverilog and verilator `v=8 P=8`, `x=f b=4`, `P=15`; sv2v copies unit items like vita, not an oracle here); fix: a declaring identity for injected unit items (a `$unit` pseudo-package §4.5.589's window keys on; an AST change and a `format_version` bump), which the "Scoping" package-body constant-domain line also needs; 2 oracles; OPEN (§5.2 row 4)
- 🆕 AG — the constant lane has no generate scoping: `const_func_table` is module-level (`instance.rs:521`), so a constant call in a generate block binds the module's same-named function (x_gen_fn `P=7 bw=8`, iverilog `P=3 bw=4`; verilator refuses a generate-scoped constant function, IEEE 1800-2023 §13.4.3; staged two-CU st2 `c.g.P=5`, iverilog 3), and a module routine's header called from a generate block folds at the block's prefix (g_ret_modpar `P=232`, g_for_ret_par over a genvar `W`, rv558_gen `P=255`; both oracles `P=8`, `P=15`; rv560_genwide loud); fix: generate routines registered per scope in the constant table (the root of §3.b `gen-rtn-edges`' constant-call half: one slice) and a routine's header folded at its declaring prefix (`rtn_decl_scope` as the window key); iverilog (generate functions), 2 oracles (a module routine at a generate call site); OPEN (§5.2 row 3)
- 🆕 AH — the instance-array prepass (`instance_array.rs:18` `elaborate_instance_array`, :91–144) binds a child's header parameters and port ranges at the parent's prefix with the parent's `const_func_table` and imports: a child default or port range calling `f` binds the parent's `f` (x_ia_hdr `top.u[1] p=5, u[0] p=5`, both oracles `u[0] p=5, u[1] p=a`; x_ia_port; x_t11 `p=a,p=a`, oracles `u[0] p=a, u[1] p=5`; §4.5.587's d01, d02), a child header `import q::*` is not applied (ia_pkg), and a body localparam a port range names is unbound (ia_lp `p=5,p=5`, verilator `u[1] p=a`, iverilog refuses); fix: run the prepass with the child's routine tables and header imports, the mirror of the `wire_ports` swap (`instance.rs:1075–1097`); a fold it opens from loud to a value declines a run that reads a never-assigned 4-state variable (§2 🆕 AE; l22 over `fx(2)`: PRE E3009, both oracles `P=xxxx`); the element walk into its parent is PROBE_CATALOG §4.5.568's; 2 oracles; OPEN (§5.2 row 1)
- 🆕 AI — a signed constant in a constant region whose width the walk cannot give folds sign-extended into an unsigned region (§4.5.593): `logic signed [7:0] X = -4; int N = 2;` with `X + {N{1'b0}}` (Ev12 `L=4294967292`, `== 8'hFC` `M=0`, `> 8'd100` `K=0`, a bound `vb=4`; iverilog, sv2v, verilator `L=252 M=1 K=1 vb=5`; the literal count `{2{1'b0}}`, Ev12c, is right); beside an unpacked-array-parameter element `(X | A[1]) == 8'hFE`, `(C ? X : A[1]) == 8'hFC` (Eu7–Eu9 `L=0 GI=else GC=def vb=4`; sv2v and verilator `1 then item 5`); under `==?` E3009 / E3010 / `vb=1` / generate-case `default` (Eu1, Eu2, Eu4); a signed array element `A[0] ==? …` E3009 and `default` (arrE; verilator `L=1`); `const_fn_width.rs` `const_self_width`: `K::Replicate` sizes its count with the literal-only `const_eval_u32` while the value path folds it with `fold_count`, a whole constant-array element has no width or sign arm (`const_signed_env`, `const_expr_signed` read it unsigned), and an unknown width turns masking off in `eval_const_env_at`'s leaf arm; holds 🆕 W (its T-typed twin Tv12 is right on PRE and goes wrong under W); §4.5.594 sized both (the count by `fold_count` through the scope resolver, the element by its declared width and sign) behind one region rule (one entry per evaluation; a region holding a user call, and the constant-function interpreter as a whole, kept PRE's leaf rules): WRONG→OK 295, LOUD→OK 20 over 3082 cells, corpus byte-identical; reverted whole after two BLOCKING rounds, 🆕 AE's 0 entering a newly sized region each time (`fl` reads a never-assigned local): round 1 through a nested region root that re-decided the rule under a call-bearing root (C1 `localparam int L = fl(0) + ((X + {N{1'b0}}) == 8'hFC)` PRE `L=0` = iverilog, cut `L=1`; C2, a generate-if over the same sum, `else` → `then`; C8 E3009 → `L=2`), round 2 through a constant that stored it first, named from a call-free region (Nq_launder `localparam Q = fl(0)`, Apg `localparam logic [7:0] A = fl(0)`: PRE `GI=else` = iverilog = sv2v, cut `GI=then`), which no structural "contains a call" rule can see; fix: re-derive that sizing (ER §3.6) once 🆕 AE lands, or before it with a mark on a constant whose fold read a never-assigned 4-state variable (§4.5.588's never-assigned-read taint; an over-seed only narrows here), a region that holds a call or names a marked constant keeping PRE's leaf rules; the other `None` arms with a signed sibling are "Constant domain" lines (a call count, `S.len()`, a >64-bit neighbour); 3 oracles (Ev12), 2 (Eu7–Eu9); BLOCKED (🆕 AE)
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
- A constant array typed by a type parameter keeps the default type's sign per instance: `localparam T AT [0:1] = '{8'hFC, 8'hFC};` under `parameter type T = logic signed [7:0]`, `localparam int L = (AT[0] + 8'sd0) < 0;` in `sub #(.T(logic [7:0])) u2` gives `t.u2 L=1`, verilator `t.u2 L=0` (E1, §4.5.594; u1 and `logic signed [15:0]` u3 right; iverilog `sorry: unpacked array parameters are not supported yet.`; sv2v reads every element unsigned, u1 `L=0`); 🆕 AI's element arm reads that type (PRE = §4.5.594's cut); site not traced; verilator + hand-IEEE (§6.20.3); OPEN

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
- Generate `case` compares two i64 values; size the case and items once (`fold_bits_at`), not pairwise (§4.5.555; §4.5.594's D21: `case (X + {N{1'b0}}) 8'hFC:` over a signed `X` `GC=def`, iverilog, sv2v, verilator `GC=fc`, its literal-count twins too, and 🆕 AI's fix leaves it, so the same expression's generate-if then disagrees); 2 oracles; OPEN
- A select write into a local whose initializer did not fold reads 0 for the other bits (`const_fn.rs:1582` `env.get(name).copied().unwrap_or(0)` over the unbound local `bind_const_decl` leaves): a41 `int t = int'(2.5); t[0] = 1'b0; f = t;` `P=0`, iverilog, verilator and sv2v `P=2`; declining there makes the twins that read only written bits or overwrite every bit loud (p03 `t[0] = 1'b1; f = {31'b0, t[0]};` and a 32-bit overwrite loop: `P=1 Q=0` = all three oracles); close upward: fold the initializer (the `int'(…)` placement / cast residue above) or track the written bits; 2 oracles; OPEN
- A continuous-assign or net-declaration delay over a constant-function call that reads a never-assigned 4-state variable folds 🆕 AE's 0 (`const_eval.rs` `fold_ca_delay`): `assign #(fd(2)) w = r;` and `wire #(fd(2)) w = r;` delay 1 where iverilog and verilator delay 2 (p05b, p08b); `#(fx1(2))` (`xxx1`) delays 1 where iverilog and sv2v take the x delay as 0 (p10), as vita's run-time lane does (p12, `#(dv)` with `dv = 4'bxxx1`); that lane (`ca_delay_rt.rs`) shares 🆕 AE's `repeat` cut's enum prerequisites (d12, an `enum bit [3:0]` local: folded delay 1 = iverilog and verilator, run-time twin d12rt delay 2), and routing there must count the routed assign as delayed in `demote_runtime_delay_on_resolved_nets`, which otherwise drops its delay on a resolved net where today's folded delay is E3001 ("Delays": a resolved multi-driven `wire` drops a runtime delay); 2 oracles; BLOCKED (the "Size cast" enum lines)
- `bit [65535:0][65535:0]` panics at net allocation; make it loud; OPEN
- Three declared-width models (`const_decl_wsign`, `const_bound.rs::decl_is_wide`, `ast_kind_range_width`); RECORD
- The interpreter reads an `int unsigned` return signed (`const_fn_ret_wsign`); 2 oracles; OPEN (sign, scope half BLOCKED: declaring-scope fold)
- A formal shadowing a parameter is selected as the parameter (`const_param_select_env`); 2 oracles; OPEN
- A non-zero-LSB local in a concat is read by position (`envw` has no LSB); 2 oracles; OPEN
- A negative replication count is accepted (`fold_count`, `const_eval_u32`); 2 oracles; OPEN
- A replication counted by a constant call has no width (`fold_count` has no call arm by design, §4.5.371 ⓸; `const_eval_u32` folds literals only), so a signed neighbour folds sign-extended where the count is not evaluated (Rfn_t_s `(C ? X : {f2(2){1'b0}}) == 8'hFC` `L=0`, `GI=else`, `vb=4`; iverilog, sv2v, verilator `L=1`, `then`, `vb=5`), a bound takes 🆕 AC's fallback (Rfn_a_s_rb `vb=1`, 3 oracles `vb=5`) and a value position is E3009 (Rfn_a_s_lp; 3 oracles `L=1`); §4.5.594's cut left it; 3 oracles; BLOCKED (a call arm in `fold_count`, §4.5.371 ⓸ · 🆕 AE for a call that reads a never-assigned variable · 🆕 AC's sinks)
- `S.len()` has no self width in a constant region (`const_self_width` `K::Call`: no `const_fn_def` for a two-segment call), so a signed neighbour folds sign-extended: Oslen_o_s `((X | 8'd0) + S.len()) == 32'd254` over `localparam string S = "ab"` `L=0`, `GI=else`, `vb=4`; iverilog and verilator `L=1`, `then`, `vb=5` (sv2v refuses); the `+ 32'sd2` twin is right (§4.5.594); 2 oracles; OPEN
- A >64-bit signed neighbour beside a name-count replication folds wrong: `localparam logic signed [64:0] X = -4;` with `(X + {N{1'b0}}) > 65'd100` (K_w65) `L=0`, iverilog, sv2v, verilator `L=1`; the literal twins `X + 65'd0`, `X + 2'b00` are right, and 🆕 AI's i64 count arm does not reach it (§4.5.594); site not traced; 3 oracles; OPEN
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
- An unreadable override leaf (element, replication, call, `$rtoi`, prim cast, `SZ_0'(B8)`) keeps the default's type (Eovu_R `#(.P(X + {N{1'b0}}))` onto an untyped `P`: `P=-4 B=32`, iverilog, sv2v, verilator `P=252`); an untyped `localparam` holding a name-count replication or a whole element binds the unlimited fold the same way (Adiv_lv `X / {N{1'b1}}` `L=4294967295 B=32`, 3 oracles `L=84 B=8`; the literal twin Tardiv_lv is right): the declared-width value lane (`param_init_at_declared_width`; `ctx_width_names_are_evident`, the fill line's `declared_override_widths`) has no replication or element arm, so 🆕 AI's width fix alone gives these the right width and a wrong value (§4.5.594: 8 cells, Adiv_lv `L=255 B=8`); verilator (the `localparam` half 3 oracles); OPEN
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
- A routine imported into another package binds in the importer or the caller, never in its own package: `elaborate_package`'s import arm files `r::g` in `pkg_funcs[p]`, which `pkg_owns` does not own, so §4.5.589's window stays closed: a call to it from `p`'s routine text keeps the caller's binding (n_pkgimp `P=232 v=232`, pi_rfn_hdr `232`; both oracles `8`), its body run from `p`'s body reads `p`'s constants (c12 `L=9 v=9`; iverilog, verilator, sv2v `L=3 v=3`), and its default is evaluated with `p`'s (ifn_dflt `v=3`, kx_tdflt_imp `u=3`; iverilog `8`); record the origin at the import arm (`imp.pkg`) and key the window, the interpreter's package tag and the default lane on it; 2 oracles; OPEN (§5.2 row 2)
- A package body's read after a shadowing block falls to the caller; BLOCKED (binding-resolved scope)
- The constant domain in a package body is unhooked (`lookup_scoped` + nine `params` twins; rv561_pkc `P=0000eeee`, iverilog, verilator, sv2v `P=01230123`); §4.5.589's prototype measured a strict body lookup: it loses the caller's `$unit` copy (ib1b `P=a5` → E3009) and opens 🆕 AE's 0 (ae8 E3009 → `P=00000000`, oracles `P=xxxxxxxx`); a name count in package function text waits on it too (Pkf_cntE `pf = ((PC ? PX : {PN{1'b1}}) == 8'hFC);` `L=0`, iverilog, sv2v, verilator `L=1`): §4.5.594's count arm resolved `PN` through the calling module's scopes (Pkf_cnt, beside a module `PN = 16`: `L=1` → `L=0`); BLOCKED (🆕 AF, 🆕 AE)
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
- Import ambiguity follows adjacency, not the reference's position (§4.5.591: `package.rs` `wildcard_prior`, `imports_adjacent`, `scope_item_starts`): two wildcard imports of one name with any item of the scope between them keep their pre-§4.5.591 binding, so a reference after the second binds the first package's where iverilog and sv2v refuse (n04 `import pa::*; wire w; import pb::*;` `n04 P=3`; iverilog `Ambiguous use of 'P'. It is exported by both 'pa' and by 'pb'.`, sv2v `identifier "P" ambiguously refers to the definitions in any of pa, pb`), two ≤64-bit constants with a reference between are E3010 where all three oracles answer the first (c25, oracles `rn A=3 P=3`), and in a package body the later package wins (c07 `pr A=3 B=5`, c32 `pnw A=3 B=9`, D24 `d24 Z=3 Y=5`; 3 oracles `B=3`, `B=3`, `Y=3`); fix: bind a wildcard name at its first reference (§26.3) and refuse a reference after a second offer; pins `import_one_binding_per_key.rs` `known_wrong_an_unrelated_item_between_two_wildcard_imports`, `known_wrong_a_package_reference_between_two_wildcard_imports`; 3 oracles; OPEN
- `$unit` imports and a module's imports are one import scope (`instance.rs` import loop: one `wc_origin` / `explicit_imports` over the unit's imports and the module's; `ImportSite::Unit` is recorded, not scoped), so a module import does not shadow a `$unit` one (§26.3): a module explicit narrow under a `$unit` explicit wide reads the wide (Icue_wn `P=18446744073709551625 K=18446744073709551625 b=32`, 3 oracles `P=3 K=3 b=32`; also 🆕 U's), a module explicit variable under one keeps it (c40 `uv W=18446744073709551625 b=8`, iverilog and sv2v `uv W=5 b=8`), a `$unit` wildcard and a module wildcard offering one name leave it unbound (Icuw_nn E3010, 3 oracles `cuwnn P=5 K=5`; Icuw_nw, Icuw_wn, n11–n14 E3010 since §4.5.591, PRE bound the `$unit` package's; 3 oracles the module's, n12 `P=18446744073709551625`), and two `$unit` explicit imports of one name are not refused (Icuee `cuee P=5`; iverilog and sv2v refuse); 3 oracles; OPEN
- A wildcard string or real package constant is no import candidate (`apply_import_consts` writes no `str_param_raw` / `real_param_val`; §3 ⑨), so another wildcard's integral constant of that name binds without the §26.3 ambiguity (Iasn, Iarn `P=3`; c04 `sa P=5`, c05 `ra P=5`; iverilog `Ambiguous use of 'P'. It is exported by both 'pa' and by 'pb'.`, sv2v `identifier "P" ambiguously refers to the definitions in any of pa, pb`); record the candidate in `wc_origin`; 2 oracles; OPEN
- Two explicit imports of one type or function name from two packages take the second in silence (D19 `import pa::t; import pb::t;` `d19 b=8`, D20 `d20 f=5`; iverilog `'t' has already been imported into this scope from package 'pa'.`, sv2v `import of pb::t conflicts with prior import of pa::t`; verilator the first, `b=4`, `f=3`); §4.5.591 refuses only a name that carries a value (`package.rs` `offers_value`); 2 oracles; OPEN
- An array-method iterator is read through a same-named parameter's range (`param_sel_range`); verilator; OPEN
- A generate `real` parameter does not shadow an outer integral one; BLOCKED (declaring-scope fold, §4.5.562)
- A walk after Nets reads a generate-scope parameter declared after the reference at the previous walk's binding (`generate.rs` `(_, ModuleItem::Param)`: bound at its position in every `GenPhase` walk, never unbound), where IEEE 1800-2017 §26.3 and the Nets walk read the outer object: N2 `K=8 bits=4 w=15` (the Nets-walk width is right, the later read is not), B1 `bits=4 t=15 K=8`, Q65 `P=11 bits=3`, P1c `K=8 v=8` / `top.gb.u P=8`, W2 `K=10000000000000007`, Z0b `l2 X=2`, SN7 `P=9`; iverilog `K=4 bits=4 w=15`, `bits=4 t=15 K=4`, `P=2 bits=3`, `K=4 v=4` / `P=4`, `K=05`, `l2 X=1`, `P=1`; with no outer object the read is accepted where iverilog refuses (Z5a `K=8`, Z5b `P=8`, Z5c `v=8`, Z5d `w=8`; iverilog `Unable to bind`), and an instance override reads the later name and steers the child's arm (DO02 `@two top.g.u.a bits=8`, iverilog `@one top.g.u.b bits=4`); fix: restore each generate level's own parameter keys at its entry in every walk after Nets (module-region keys only where the pre-Nets entry was empty: V24s keeps an import), which also makes every walk decide as Nets does (🆕 V); a decision it moves can reach 🆕 AE's 0 (Z1g), so it declines a run that reads a never-assigned 4-state variable (§5.2's 4-state rule); must stay: Z5i, Z5j hierarchical reads (all three `K=5`); iverilog + hand-IEEE (sv2v and verilator bind scope-wide: "Oracle splits"); OPEN

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
- import precedence — verilator binds the first import of a name, explicit or wildcard (D21 `import pb::P; import pa::*;` the explicit `P`; D22 and X1 the wildcard's; Iab_sel, D40 the first of two wildcards); iverilog, sv2v and §26.3 let an explicit import win wherever it is written and refuse two wildcards: verilator is no oracle there
- forward references in a generate scope — not a split (§4.5.592): IEEE 1800-2017 §26.3 binds a reference other than a call only to a declaration before it, else in the next outer scope, else it is illegal (its Example 1: "line 2 initializes p::x. Line 4 initializes top.b.x."), and iverilog follows it (``Unable to bind parameter `K' in `top.gb'`` … `Check for declaration after use.`); sv2v and verilator bind scope-wide and contradict the example (Z0a, Example 1 without its illegal line: iverilog `px=1 bx=2`, sv2v `px=z bx=2`, verilator `px=0 bx=2`; parameter twin Z0b: iverilog `l2 X=1`, both others `l2 X=2`): they are disqualified on this axis, so a forward-reference cell is decided by iverilog and §26.3, never by their majority
- a static inline task's body-local enum label beside a same-named module >64-bit, string or real constant (Ef*_tk2) — split (iverilog the module constant, verilator the label `r=1`, sv2v cannot parse); vita takes the label beside a narrow constant and the module constant beside the others (`r=0`, `16706`, `3`)
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
- a t0 read of a held continuous assign's output, or of an assign below it, before the release (an `initial`'s first slice): vita `z` on a net, `x` on a variable (hand-IEEE §6.8); iverilog `z`; verilator the value from the declared defaults when it runs the assign first (a6, a7 `i0 y=01`) or 2-state `0` when it runs the `initial` first (a2 `i0 y=00`) — split; both tools order a continuous assign and an `initial` both ways in other designs (T2a, a2, T3), and an assign with only constant arguments keeps PRE's place before the processes (a4 = verilator; iverilog after) — `t0_call_cont_assign_hold.rs` race pins

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
- mdrv-partial — the multidriver check ignores partial writes (`stmt_writes_whole_ident`); Rule D too: `assign y = {a, a}; always @* y[0] = b;` runs `y=11` (iverilog "Cannot perform procedural assignment to bit select", verilator MULTIDRIVEN), two `assign y[0]` on an unpacked element run `y0=5` (iverilog multiple drivers, verilator MULTIDRIVEN); xcelium unmeasured; RECORD
- mdrv-ca-force — `force` over a variable's continuous `assign` is not a Rule D writer (`ProcWrites::ExceptForce`, IEEE §10.6.2); iverilog and verilator run it `y=1 y=0 y=1 y=0` = vita; RECORD
- mdrv-ca-call — Rule D reads lvalues only: a task-body write (`task wt; y = 0; endtask` + `initial wt();` beside `assign y = a;`, `y=1 y=0`) and a hierarchical write (`u.y = 0;` onto a child's `assign y = a;`) run; iverilog rejects both, verilator runs both; a system task's output argument is no writer either (`void'($sscanf("5", "%h", y));` beside `assign y = 4'h3;` prints `y=5`; iverilog run-time `$sscanf argument 3 (a vpiNet) is not assignable.`); carry `call_body_writes_whole`, the hierarchical lvalue and the system-task destinations into `cont_assign_var_conflict`; iverilog + IEEE §6.5; OPEN
- mdrv-ca-port — Rule D is module-scope lvalues only: an instance output bound to the variable (`m u (.o(y)); assign y = a;`), a generate-block `assign` (`if (1) begin : g assign y = ~a; end`) and an interface body with two `assign`s each print `y=x`; iverilog "Variable 'y' cannot have multiple drivers.", verilator MULTIDRIVEN; a UDP instance output (`u_and g (y, a, b); assign y = c;`, a `ModuleItem::Instance`, while a built-in gate is a desugared `ContAssign` Rule D counts) prints `y=x` (iverilog "Variable 'y' cannot have multiple drivers.", verilator `y=1`, sv2v cannot parse `primitive`); count port bindings and per-scope drivers on the elaborated net; 2 oracles; OPEN
- mdrv-ca-array — an unpacked array's whole-array `assign` beside another writer is E3009 (`cont_array.rs` sole-writer check), not E3001; Rule D skips unpacked variables; both oracles reject; RECORD
- mdrv-shadow-scope — the multidriver shadow guard skips a whole PROCESS when any block in it declares a local of the variable's name (`declares_local_named`, Rules A, B and D), so the process's real writes outside that block go uncounted: `assign y = 4'h1;` + `initial begin y = 4'h2; begin : inner logic [3:0] y; y = 4'h3; end … end` prints `y=2`, a `fork` whose sibling branch declares `y` prints `y=7`, and `always @* begin y = b; assert (…) else begin logic [7:0] y; y = '0; end end` prints `y=11` (sv2v → iverilog and verilator `y=22`); iverilog "Cannot perform procedural assignment to variable 'y' because it is also continuously assigned." on all three, verilator CONTASSINIT / MULTIDRIVEN; scope the guard to the declaring block (writes outside it count); 2 oracles; OPEN
- mdrv-gen-region — a bare `generate … endgenerate` region is skipped like a generate block by Rules A, B and D, though IEEE 1800 §27.3 gives the region no scope: `generate assign y = 4'h5; endgenerate` + `initial y = 4'h0;` prints `y=0` (iverilog "Cannot perform procedural assignment …", verilator `%Error-CONTASSINIT`), `generate assign y = 4'h1; endgenerate` + `assign y = 4'h2;` prints `y=X` (iverilog "Variable 'y' cannot have multiple drivers."); walk the region's items as module items in `check_multidriver_processes`; 2 oracles; OPEN
- decl-var-net-kw — `wire logic y;`, `var logic [3:0] y;`, `output var logic [3:0] y`, `output wire logic [3:0] y` and a drive strength on `assign` (`assign (weak0, weak1) y = b;` on a `wire`) are E2002; iverilog, verilator and sv2v → iverilog run each (`y=1`, `w=1`, `w=1`, `w=1`, `y=0`); `alias a = b;` and `assign p::v = …;` are E2002 too, run by verilator only (iverilog and sv2v syntax errors); 3 oracles; OPEN
- mdrv-assert-local — a local declared in an assertion's action block is flattened onto the module variable by bare name: `always_comb x = a;` + `a1: assert property (@(posedge clk) 1'b0) else begin logic [7:0] x; x = '0; end` is a false E3001 (Rule B; verilator and sv2v → iverilog `A1 x=11 A0 x=22`), its `always @*` twin prints `A1 x=00` (verilator `x=11`), and beside `assign x = a;` an `always @(x)` sees `[1] chg x=5a` and a deferred action's `$display` of the local prints `L x=11` (verilator none / `L x=5a`); scope the action-block local, then pass `true` from Rules A and B (`declares_local_named`); pins in `cont_assign_variable_drivers.rs`; 2 oracles; OPEN
- mdrv-hier-actual — `always_comb u.ts(src)` is a false E3001 (Rule A `Unknown`); resolve the callee's formals through the instance's module; 2 oracles; OPEN
- frame-body-write-sites — call sites of a body-writing frame function that cannot carry `Terminator::Call` are E3009: CA rhs (no statement), a nested frame call (`frame_fn_lowering`), `pk::gw()` (`inline_fn.rs` reserves late), a class method, an outward / generate hier path (`hier_body_write_callee`), Rule A on a hier actual (`stmt_never_writes_ident`); per site; 2 oracles; OPEN
- frame-body-write-order — a hoisted body-writing call precedes a left read (vita = verilator); SPLIT
- dyn-size-spellings — `$size(c.da)`, `$size(u.da)`, a dyn formal's `$size` (`resolve_intro_net`); route all three to `DynSize`; verilator; OPEN
- dyn-bits-count — `$bits(da)` folds the element width (§20.6.2: the whole array); size × element bits, pinned by hand; no oracle; OPEN
- pkg-type-param-import — explicit `import p::PT;` of a `parameter type` is E3009; DO-NOT-START (§5.2)
- gen-rtn-edges — `u.g.f()`, `gi.f()` and a constant-expression call of a generate routine are E3009 (`frames_reserve.rs` hier gate, `const_func_table`, `frames_classify.rs`); compose the hier key, register per scope (the constant-call half shares §2 🆕 AG's root: one slice); iverilog; OPEN
- aes§2 — the inliner's discriminator is more than `automatic`; OPEN
- gen-enum-uncarried — a generate `typedef enum` binds only where §4.5.565 carries it (`gen_enum.rs`); BLOCKED (declaring-scope fold)
- string-literal-condition-residue — a string literal in a generate condition §4.5.568 cannot read (`cond_names.rs`); BLOCKED (declaring-scope fold)
- md-param-pattern-residue — a md packed parameter's non-literal `'{…}` items, keyed patterns, named bounds (`packed_md.rs`); per item, named items after row 15's carrier; verilator; OPEN
- cont-array-typedef-residue — a whole-array CA with an enum / mixed / named-bound typedef element (`cont_array::typedef_elem_type`); verilator + iverilog; OPEN
- cont-array-residue — a whole-array CA with a delay, decl assignment, sub-array, `?:` or second writer; OPEN
- packed-default-residue — packed `'{default: v}` with a typedef dim, partial target or non-integral `v` (`packed_pattern.rs`); OPEN
- const-fn-case — a constant function whose body holds any `case` / `casez` / `casex` / `case … inside` is E3009 `f(…) has no constant-fold arm`, every qualifier, reached or not, where both oracles fold (probe p2: plain `case`, `g(2)`, iverilog and verilator `P=20`; 49 §4.5.587 cells; `inside`: verilator only, iverilog `Incomprehensible case expression`); `elaborate/src/const_fn.rs` `exec_const_stmt` has no `Stmt::Case` arm (catch-all `_ => None` at :1719); the arm sizes the case expression and every item once at one width and sign (ER §2.4), never pairwise, the row's main risk; casez / casex masks, §12.5.4 for `inside`, first match wins; opening it lets the interpreter fold more functions at every consumer, the widening §4.5.587 reverted on, so it inherits §2 🆕 AD's residue (🆕 AF, AG, AH and the "Scoping" import line; not yet measured on case bodies), and the arm declines a run that reads a never-assigned 4-state variable (§2 🆕 AE: a `case` with no `default` that misses leaves the return variable unassigned; an over-seed stays loud, never silent); a `unique` / `priority case` miss then reaches the synthesized default (`unique-const-fn`); closes §2 🆕 AC's case-form cells; 2 oracles; BLOCKED (§2 🆕 AF, AG, AH, the "Scoping" import line)
- const-fn-systask — `$display` / `$info` in a constant function is E3009 `f(…) has no constant-fold arm` where both oracles fold silently and print nothing (§4.5.587 g4, `localparam int P = f(2)`: iverilog and verilator `P=3`); `$warning`: iverilog `P=3`, verilator `%Warning-USERWARN: "in f a=2"` at build, then `P=3`; `$error`: iverilog `P=3`, verilator `%Warning-USERERROR: "in f a=2"` and fails the build (`Expecting expression to be constant, but can't determine constant for FUNCREF 'f'`); IEEE 1800-2017 §13.4.3 ignores system task calls in a constant function (recalled); `exec_const_stmt` catch-all; an ignore arm needs `unique-const-fn`'s positive opt-in, since at a consumer that replaces a run-time call (a `repeat` count, a delay) both oracles print the task's output at run time (g4 d_display_rt), and it widens the interpreter like `const-fn-case`, so it declines a run that reads a never-assigned 4-state variable (§2 🆕 AE; an over-seed stays loud, never silent); 2 oracles (`$display`, `$info`); BLOCKED (§2 🆕 AF, AG, AH, the "Scoping" import line)
- pkg-param-own-fn — a package parameter whose value calls a function of the same package is E3009 `package parameter P value is not a foldable constant [in pk]` even for `return a + 1` (§4.5.587 plan probes pk_simple, pk_plain: iverilog and verilator `P=2`, `P=10 Q=7`); its declared range is E3009 too (pkown `parameter logic [f(2):0] P`: `a function call that does not fold to a constant is not allowed in a constant range bound`; oracle output not kept); `elaborate/src/package.rs:674` (the package fold's call lookup, not traced); 2 oracles; OPEN
- pkg-text-open — package routine text naming its own package's constant or function is E3009 / E3010 from a caller that binds no same-named item (a_ret_ce_ctl, a_ret_rt_pctl `undefined name W`, nc1_genif; all three oracles `P=8`, `v=8`, `br=then`; 72 §4.5.589 cells), and the same unit keeps 🆕 AC's silent fallback where it lowers or holds a call (16 cells: `*_fml/_loc/_frloc_ce_ctl`, `*_rep/_psel_rt_ctl`; pk_ce_fml, pk_ce_loc `P=1000`, oracles `P=8`); §4.5.589 binds the package only where the caller's fold answered, because its prototype's opening of the rest handed values to silent sinks: 🆕 AE's 0 (ae2_loc_ce `P=0000`; iverilog, verilator, sv2v `P=xxxx`), 🆕 AC's fallbacks once the frame reserves (ac10_rep_case `w=00000000`, oracles `00000007`; ac11, ac12; ac1, ac2, ac14 where the oracles refuse an x count), an x label (ac3_gcase_x `arm=h`, PRE and the oracles `arm=def`) and the "Constant domain" select-write line (id1_a41_pk `P=0`, oracles `P=2`); fix: open per consumer once each of those sinks refuses what it cannot carry; 3 oracles; BLOCKED (🆕 AE, 🆕 AC's sinks, the "Constant domain" select-write line)

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
- E3009-anchor — some E3010 / E3009 sites lack file:line (`diag::SpanResolver`), and an import arm's E3009 names the module header (§4.5.591's §26.3 conflicts: `Ewn.sv:2:8`, `Iee_gif.sv:3:8`, `D41m.sv:1:8`; iverilog the second import, `Ewn.sv:4`, `Iee_gif.sv:5`, `D41m.sv:3`); OPEN
- error_at — the anchor and `found` token differ; OPEN
- #9 — `velab -L` diagnostics have no location; OPEN
- cli-lib — `cargo test -p cli --no-default-features --lib` fails E0004 (a dev-dependency revives sim-engine's `oracle`); `default-features = false` on it; CI cannot see it; OPEN
- carrier-namespace — `T$w`-style carrier names are not reserved at declarations (`names_a_type_param_carrier` runs at override names only); refuse at every declaration site; 2 oracles; OPEN
- EXT2-DOC — stale CLI, language, system-task and explain docs; OPEN
- unique-if-chain — after §4.5.585 two residues stay silent where verilator reports `'unique if' statement violated`: (1) a chain in any function or task body (every subroutine body keeps the lone-`if` rule, `hdl-parser/src/functask.rs` `tf_body` sets `first_if_arm_only`; 111 lines in the §4.5.585 census by kind: task 44, item `function void` 30, item non-void function 17, class function 7, class void function 7, constructor 6 — e.g. lt_P lines 2 / 5 / 9, b02 fchain / tchain, m_void_rt, ifc_vfn_rt, pkg_vfn3_imp_rt, cls_task_rt, cls_fn_rt, cls_vfn_rt, cls_ctor_rt, n_ca_objf2_tbL, t2t_c_fg_this, u20_c_fnew_member, q6a_fn_vfn_noformal, q6t_pkg_scoped_const_chain); BLOCKED on `unique-const-fn` (itself BLOCKED on §2 🆕 AF, AG, AH and the "Scoping" import line), `unique-pkg-closure` and §2 🆕 AB (a)'s per-settle re-run of a class-handle continuous assign (§4.5.590 removed the t0 run on x and its W4031 at t0; armed, n_ca_objf2_tbL_H, t2t_c_fg_this_H and u20_c_fnew_member_H report ×3 at time 2, ×2 at 3 and 4, where verilator reports twice at each; u8_c_ft_this_nowrite_H and u23_ctor_cls_task_H the same, no oracle: both refuse a function calling a task; q6a_fn_vfn_noformal_H once at 2, = iverilog's `unique case` twin); (2) an outer series whose `else` is a qualified `if`: verilator continues it through `else unique0 if` and reports (vl_lines t3 / t5, b01 t1, f1c2_qual C2 / C7, q16_nested_ml t3, q16b_nested_ml2 t3, q23_prio_u0 t3, q5n_nested_spans t4), where IEEE 1800-2017 Syntax 12-2 makes the qualified `if` the final `else` statement (vita = PRE = IEEE; a split, not chased); also recorded: an inner `unique` / `priority if` after `else` reports at its own line where verilator names the outer line (vl_lines t2 / t4, q16_nested_ml, q16b_nested_ml2; PRE the same for a lone inner `if`); pins `crates/cli/tests/unique_if_chain.rs`; verilator; BLOCKED
- unique-const-fn — a constant function that reaches a synthesized no-match arm (a lone `unique if` / `priority if` with no `else`, a `unique case` with no `default`) is E3009 `… has no constant-fold arm` where both oracles fold silently (iverilog on `case`; it rejects `unique if`); `elaborate/src/const_fn.rs` `exec_const_stmt` has no arm for `$__vita_unique_violation`; §4.5.587 built a no-op arm under a positive opt-in (`ConstSite::Required`, set at IEEE constant-required consumers) and reverted it whole after two consecutive BLOCKING rounds (ER §3.6: a root returning through another door): it moved 48 of 444 cells (38 loud → oracle value; 10 silent-wrong → correct, §2 🆕 AC's `unique if` cells), but folding the `unique` form exactly like its plain-`if` twin inherits every plain-twin silent-wrong at those consumers — round 1: the instance-array prepass (d01, d02), package routine text (pkret, pkloc, pkfml, pkcast, pkimp), a 4-state default 0 (d10); round 2, past its exclusions (an instance-array element hold; a text-span rule over package text and routine-declaring generate levels): `$unit` routines, which `inject_cu_items` (hdl-parser `module_items.rs:342`) copies into every module (r24_unit_shadow, unitret), and a span key that staged compilation units do not share (`cli/src/staged.rs:171`); re-derive (ER §3.6) once §2 🆕 AD's residue lands (§4.5.589 closed its package-routine half; left: 🆕 AF, AG, AH, the "Scoping" import line): a positive opt-in at constant-required consumers whose arm declines a run that reads a never-assigned 4-state variable (§2 🆕 AE, round 1's d10; an over-seed stays loud, never silent), never an exclusion by text span or enumerated scope; consumers that replace a run-time call (`repeat_unroll_count` `const_bound.rs:146`, `events.rs:1065`, `netdecl.rs:876` / `var_init.rs:127` via `ca_delay_rt.rs:71`) stay unstated, else they drop the report both oracles print at run time (c_rpt, e_trpt, h_tdly, h_cad, h_wd); 2 oracles; BLOCKED (§2 🆕 AF, AG, AH, the "Scoping" import line; the `case` half also §3.b `const-fn-case`)
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
- NBA-GROWTH — nothing bounds the pending NBAs or forked processes one time step can pile up; F4027 (`max_body_steps`) bounds steps, not memory, so a plain `always` with a zero-delay pass holding a non-blocking assignment grows to GB before it fires (PRE, under a 1.5 GB watchdog: `always begin if (en) @(posedge clk) q <= d; q2 <= d; end` with `en` 0, 1.53 GB in 0.69 s; four NBAs, 1.69 GB in 0.56 s; `begin repeat (cnt) @(posedge clk); q2 <= d; end` with `cnt` 0, 1.57 GB in 1.20 s; `always begin fork @(posedge clk) q <= d; join_none end`, 1.51 GB in 1.45 s; under a 6 GB cap the first fires F4027 at 4.7 GB and the four-NBA one does not before 6 GB; the §4.5.597 review's u1–u3 twins); `sched/propagate.rs::schedule_nba`, `native/kernel.rs` (`nba`), the fork spawn; F4027 or a sibling fatal also bounds pending NBAs and live processes per step; no oracle needed (a guard); OPEN
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
- T0-HOLD-CHAIN — §2 🆕 AB (a): the t0 release re-runs a released held uncertified assign in every pass of every later wave, so a held chain of N is O(N²) (§4.5.590, native, PRE vs POST: r_chain, a `$random` callee, 1k/2k/4k/8k 15.3/38.9/91.4/198.4×; hu_urandom 1k/2k/4k 14.4/35.9/81.0×; a chain of certified held assigns stays ~1.0×); `sched/t0_hold.rs` (`held_always`, the wave lanes); fix = 🆕 AB's own-wave rule (round 4's build: 0.96–1.00× on both); BLOCKED (🆕 AB (d))

### 5.c Current state

Default backend `native` (tier 3); product `--no-default-features` = one executor; the corpus runs blocking in CI on 3 OSes; codegen off.

## 5.2 Queue (start order)

Canonical start order; LOOPROMPT's NEXT mirrors it and this table wins.
One iteration takes ONE row (single root; an oracle, a corpus row's pinned oracle counting; outside walls and oracle splits), plus at most one review-free hygiene item. A row that needs a format bump takes it.
Same-root rows are one slice, and their order is a measurement (a value lane lands before a guard over an unlimited fold is deleted).

Real-design first: every row has a corpus witness except rows 1–8, which an external report or its fix path reproduced. A slice that moves a corpus page re-pins that row's refusal in the same commit (`DRIFTED` otherwise) and reports the page before and after.
Incoming reports pre-empt the queue; reproduce every item at HEAD first.

The `rank` column is a row's defect class — ① a silent-wrong, ② loud→supported (§1) — not its place in the queue: the row number is the start order.
Rows 1–5, what the external report's fix path found (§4.5.585, §4.5.587, §4.5.589), go before the ① rows 6–7 because the owner's order of 2026-10-02 keeps the report's items first. §2 🆕 AD's measured half landed (§4.5.589), and the four prerequisites that half left follow in ER §10.2's risk order: routing into an existing mechanism (rows 1–2) before new infrastructure (row 3 a per-scope constant table, row 4 an AST change and a format bump). §4.5.587 reverted §3.b `unique-const-fn` (now BLOCKED): folding the `unique` form like its plain-`if` twin inherits the twin's silent-wrongs; §4.5.589 closed the package-routine half of that prefix binding and rows 1–4 close the rest (§2 re-entry: that row's fix path). The 4-state half is a rule, not a row: §4.5.588 measured that the interpreter's 0 for a never-assigned 4-state variable cannot be fixed yet (§2 🆕 AE, BLOCKED), so every arm or lane a later row opens from loud to a value declines a run that reads one; row 5 widens the interpreter that way and waits on rows 1–4; `unique-const-fn` is re-derived from its row once rows 1–4 land (ER §3.6). `unique-const-fn`, §3.b `unique-pkg-closure` and §2 🆕 AB's residue (§4.5.590 closed its t0 run on x; the residue is BLOCKED) block §3.b `unique-if-chain`'s subroutine-body residue. Rows 6–7 are §4.5.581's prerequisites: none has a corpus witness and each redesigns shared code; row 6 ships 🆕 W with 🆕 S (a)'s i64 half (each alone descends), and both meet §2 🆕 AI (§4.5.593), which waits on 🆕 AE (BLOCKED, no row; §4.5.594 reverted it whole), so row 6 waits on it; two more of §4.5.581's wait on 🆕 AE: §2 🆕 U's wide→narrow half (§4.5.591 closed its import half) and §2 🆕 V (§4.5.592 reverted it; it also waits on the §2 "Scoping" later-walk reads line), so row 7's 🆕 T waits on both. Row 8 (§4.5.588) unblocks only 🆕 AE's `repeat` cut, and no report item waits on it, so it follows rows 6–7.

| # | slot | item | source | rank |
|---|---|---|---|---|
| 1 | 1 | §2 🆕 AH — the instance-array prepass binds a child's header default and port range with the parent's routine table and imports (x_ia_hdr `top.u[1] p=5`, x_t11; both oracles `p=a` / `p=5`); first: census what `instance_array.rs:91–144` binds and resolves (`bind_params(child)` at the parent prefix, the parent's `const_func_table`, header imports), then run it with the child's routine tables and header imports as `instance.rs` `wire_ports` does in reverse | §4.5.589's grounding (🆕 AD's prepass half; §4.5.587's d01, d02) | ① |
| 2 | 2 | §2 "Scoping" — a routine imported into another package binds in the importer or the caller (n_pkgimp `P=232`, c12 `L=9`, ifn_dflt `v=3`; oracles `P=8`, `L=3`, `v=8`); first: record the origin at `elaborate_package`'s import arm (`imp.pkg`), then key §4.5.589's window, the interpreter's package tag and the default lane on it | §4.5.589's review (round 1) | ① |
| 3 | 3 | §2 🆕 AG — a constant call in a generate block binds the module's same-named function, and a module routine's header called from a generate block folds at the block's prefix (x_gen_fn `P=7`, iverilog `P=3`; g_ret_modpar `P=232`, both oracles `P=8`); first: register generate routines per scope in the constant table (one slice with §3.b `gen-rtn-edges`' constant-call half), then fold a routine's header at its declaring prefix (`rtn_decl_scope` as the window key) | §4.5.589's grounding (🆕 AD's generate half) | ① |
| 4 | 4 | §2 🆕 AF — a `$unit` routine's text binds the calling module's same-named function or constant (`inject_cu_items` copies unit items into every module; u_ret_fn `v=232 P=232`, r24_unit_shadow; both oracles `8`); first: a declaring identity for injected unit items (a `$unit` pseudo-package §4.5.589's window keys on; an AST change and a `format_version` bump) | §4.5.589's grounding (🆕 AD's `$unit` half) | ① |
| 5 | 5 | §3.b `const-fn-case` — every `case` form in a constant function is E3009 where both oracles fold (probe p2); after rows 1–4: a `Stmt::Case` arm in `exec_const_stmt` that sizes the case expression and every item once (ER §2.4) and declines a run that reads a never-assigned 4-state variable (§2 🆕 AE), then census the newly folded consumers against their plain-`if` twins | §4.5.587's grounding | ② |
| 6 | 6 | §2 🆕 W with 🆕 S (a)'s i64 half — a constant typed by an overridden type parameter loses the override's sign (PT5c `PV < 0` → 0, dF3 `TP = '1` → 15; oracles 1, -1), and a constant `==?` / `inside` compares at the left width; after §2 🆕 AI (BLOCKED on 🆕 AE, §4.5.594), one slice (each alone descends, §4.5.593): measure §4.5.479's and §4.5.483's pins on HEAD, the two declaration lanes (`ParamDecl.shape_param`), then `eb9d3b69`'s i64 half with every decline falling back to PRE's compare and a call-bearing left operand kept on it; re-run §4.5.593's sets (666 variants, p3, p7, p8, p11) on the new PRE | §4.5.581's review; §4.5.593's retry plan | ① |
| 7 | 7 | §2 🆕 T, then 🆕 S (a)'s wide half — after 🆕 U's held half (BLOCKED on 🆕 AE, §4.5.591) and 🆕 V (BLOCKED on the "Scoping" later-walk reads line and 🆕 AE, §4.5.592); §4.5.581's design (branch `fix/gencase-label-domain`): labels compared two ways in the bit domain, decided where they agree (`d4dc9c26`); then the wide half (LP, st1, two compares under `||`: S9); held cells `generate_case_and_wildcard_prerequisites.rs` | §4.5.580's fix path | ① |
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
| FF-MISSED-AT | run a header-less `always_ff` whose one `@(…)` some pass misses (`if (en) @…`, a zero-trip loop), as the IEEE text allows, instead of `VITA-E3061` (§4.5.597) | §5.b NBA-GROWTH closed AND a real design uses the shape (corpus: 0 uses) |

## 8. Non-goals (permanent, not gaps)

- IMPLICIT-NET (explicit `E3010`) · out of scope: synthesis, a waveform GUI, UPF / SDF / DPI-C, `shortreal`, `trireg`, UVM, `unique` / `unique0` multiple-match checking (`VITA-I2021` announces it, once per parse).
- `defparam` stays direct-child with a constant value; deeper or non-constant is a loud refusal. Tracked: §0 row 14-b, §3.b `defparam-iface`.
