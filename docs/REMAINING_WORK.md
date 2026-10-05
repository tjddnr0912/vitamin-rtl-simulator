# vitamin — remaining work

What stands between HEAD and the two goals. The open items and the queue are in [ROADMAP.md](ROADMAP.md) (start order: §5.2); a finished slice's record is its commit message (`git log --grep '§4.5.N'`).

- G1 = a correct open-source RTL simulator (correct-or-loud) at the level of icarus, verilator, xcelium and vcs.
- G2 = an AI-agent-friendly simulator (the observability rail; SPEC = [preview/19](preview/19-ai-agent-observability.md)).

## D. Prerequisites that block work from starting

One line each: prerequisite — what it means; what it blocks. None blocks a row on a corpus design's current page: they block frozen §2 rows, synthetic-origin §3 rows and the external report's residues (§3.b `unique-if-chain`'s subroutine bodies, `unique-const-fn`), and wake with them. Oracle splits are recorded, never chased.

- A declined callee's body reads in `ca_deps`' read set (§2 🆕 AB (d); `levelize/mod.rs` adds a callee's body reads only for a call it certifies) — the t0 release orders a held assign without them, so its function can run on a held net's `z` first, and an assign re-run only in its own wave stays stale past t0; blocks §2 🆕 AB's own-wave rule and its cost half (§5.b T0-HOLD-CHAIN), which §4.5.590 built and reverted on that gap.
- A constant-function arm for the synthesized no-match report (§3.b `unique-const-fn`, blocked by the declaring-scope fold below; the arm declines a run that reads a never-assigned 4-state variable), a package-scoped call closure walk that admits it (§3.b `unique-pkg-closure`) and a class-handle continuous assign evaluated once per event (§2 🆕 AB (a), the "Diagnostics" re-run line, behind the purity-certification adjudication below) — today the constant interpreter refuses a call that reaches the arm, the closure walk refuses a legal `pk::f(…)`, and an armed miss in a class method a continuous assign reaches reports three times at a step where verilator reports twice (§4.5.590 removed the t0 run on x); they block §3.b `unique-if-chain`'s residue (1), arming chains in function and task bodies.
- A 4-state result channel from the constant interpreter to module-scope folds — `const_eval_in_scope` answers `Option<i64>` and the wide walk (`const_wide.rs` `fold_region`) has no call arm, so an x a constant function returns reaches every binder as a value; with row 15's 2-state identity (below) and §2 🆕 AC's sinks refusing an x-valued call, each binder can convert, refuse or hold the x; blocks §2 🆕 AE (§4.5.588), and through it §2 🆕 U's wide→narrow half (§4.5.591, below); §2 🆕 AF, AG, AH and §3.b `const-fn-case`, `const-fn-systask`, `unique-const-fn` do not wait on it: a lane they open from loud to a value declines a run that reads a never-assigned 4-state variable.
- An enum-typed variable's 2-state identity at run time (§2 "Size cast": `typedef enum bit [7:0]` stores 4-state, and a base-less enum's never-assigned return variable reads x; §5.2 row 10) — vita's run time reads such a variable as x where a constant fold reads 0 (the parser records `enum bit [N]` as `Logic`; the return's run-time site is not found); blocks §2 🆕 AE's `repeat` cut and the "Constant domain" continuous-assign delay line.
- A constant interpreter that folds `case` bodies, system tasks and the synthesized no-match arm (§3.b `const-fn-case`, §5.2 row 5; `const-fn-systask`; `unique-const-fn`) — today each makes a call decline; blocks §2 🆕 AC's upward closure (ER §2.5) and, through `const-fn-case`, `unique-const-fn`'s `case` half.
- One current binding per key, the wide→narrow half (§2 🆕 U; §4.5.591 closed the import half) — a genvar, local enum label or explicit narrow import that replaces a >64-bit binding clears the wide entry; it hands the narrow value to the constant interpreter where the wide one was refused (Gw_cae `W=0`, oracles `W=x`), so it waits on the 4-state result channel above (§2 🆕 AE); blocks §2 🆕 T and 🆕 S (a)'s wide half.
- A generate-case arm decided once per construct instance (§2 🆕 V, §5.2 row 6) — the same arm and label bindings in every elaboration phase; blocks §2 🆕 T.
- A signed constant's width in a constant region the walk cannot size (§2 🆕 AI, §5.2 row 7: a parameter-count replication `{N{…}}`, an unpacked-array-parameter element; `const_self_width`) — blocks §2 🆕 W, which ships with 🆕 S (a)'s i64 half (§5.2 row 8) because each alone descends (§4.5.593); that half also needs every decline to fall back to PRE's own-width compare.
- A case temp for class-method bodies (§2 🆕 X) and a string-kind capture of a string-returning scrutinee (§2 🆕 Y) — block `case … inside` accepting those shapes (§3.b `case-inside-residue`).
- A per-unit parse of a design that uses `inside` as a name (§3.b `inside-name-use`) — blocks every `case … inside` in such a design.
- A declaring-scope fold beyond package routines (§2 🆕 AD; §4.5.589 binds a package routine's own text to its package wherever the caller's fold answered) — still missing: a `$unit` routine's identity (§2 🆕 AF, §5.2 row 4), generate routines in the constant table and a routine's header at its declaring prefix (§2 🆕 AG, §5.2 row 3), the instance-array prepass in the child's scope (§2 🆕 AH, §5.2 row 1), an imported routine's origin package (§2 "Scoping", §5.2 row 2), and for text the caller's fold refuses, sinks that refuse what they cannot carry (§3.b `pkg-text-open`); typedef ranges and constants outside routine text still fold at the caller's prefix (§4.5.558, §4.5.560–562 and §4.5.587 reverted on it); blocks §3.b `unique-const-fn`, `const-fn-case`, `const-fn-systask`, §2 row 10, the "Real" call override, the bare >64-bit select, the package-body constant domain, the generate `real` shadow, typedef named dims, §3.b `pkg-string-const-select-dir`, `gen-enum-uncarried`, `string-literal-condition-residue`, `cont-array-typedef-residue`'s named bounds, `md-return-select` (a).
- A generate-scope alias's recorded type — `localparam C = A;` under a generate block records A's width and sign, a guess followed through forwarding (§4.5.559); blocks the §2 "Index sealing" `parameter signed` line.
- The override binder's unknown plane and the parameter lane's 2-state identity (§2 row 15: `hdl-parser/src/params.rs` drops `var_kind`) — blocks row 15, §2 🆕 AE and §3.b `md-param-pattern-residue`'s named items and `bit` elements.
- A tree-wide AST self-width pass — WALL(AST self-width); blocks the §2 size-cast cluster.
- An exact declared-width fold for hierarchical placeholders (`env_fold` negates a narrow literal in i64) — blocks the §2 "Inline / frame binds" inexact-placeholder line.
- A declared width for array-reduction, string and placeholder cast operands (`ir_bits_of` answers `None`) — blocks the §2 fabricated-width line and the fabricated `coerce_two_state` arms.
- A binding-resolved scope (which declaration a post-block reference binds, not a name) — blocks three §2 "Scoping" lines and §3.b `blocal-inert-falseloud`; the parser's name-keyed struct bindings are the same gap (PROBE_CATALOG §4.5.571).
- Type bindings that survive a nearer rebinding of their name, and a package's complete type twins — block the three PROBE_CATALOG rows §4.5.573 built and reverted.
- A block-scoped constant binding — blocks §2 🆕 Q (a bare-name hoist makes 5 new silent-wrongs).
- A field-key normalisation map (a class field is not a net) — blocks §2 row 3b and its class-field line.
- Sign provenance told apart from a default (`expr_self_signed`'s catch-all) — blocks §2 🆕 B.
- A target-typed override evaluation — blocks §2 🆕 M ⓐ.
- Per-instance declarator arity (a symbolic arity marker on two SchemaHash roots) — blocks §3 ⑤ⓕ's dim-count axis.
- Per-instance class registration (`register_classes` is a whole-design prescan) — blocks §3 ⑤ⓕ's class property.
- An `order_walk`-grade ordering judge — blocks §3 ③ⓒ (`$feof` hoisting).
- An elaborate-time record of inline site → caller — blocks §3 ⑭'s call tree and §6's CALL TREE.
- The hierarchical-call framing gate — blocks §3.b `iface-modport-formal`.
- A per-bit driver map — blocks §3.b `E3001-overlap`.
- A tick representation that can say "never fires" — blocks §2 "Delays" ⓒ (a negative structural delay).
- An oracle for the order inside a wake group — blocks §2 "Delays"' continuous-assign hop (§4.5.539's yield settle).
- The adjudication of the diagnostic stream a purity certification moves — blocks the §2 "Performance" certification line and the two "Diagnostics" lines with its root.
- The arena — blocks §5.b 5c (a frame body in `wprog`); S1d-4c / S1d-4c-2 block §5.b MON-RENDER and QUIESCE-NBA; an unsafe-FFI ruling blocks MEM-GUARD.
- fst-writer's initial-value API (upstream) — blocks §2-N-2.
- One oracle and zero corpus demand — blocks clocking (§2 rows 23, 24, 34).
- An oracle split on the boundary itself — blocks §2 row 32 (`$finish` / `$fatal` inside a function).
- Stated on their own lines: §4's six SVA prerequisites, §5.b ARR-LHS (its own census first), §0 row 14-b (the §8 `defparam` non-goal).
