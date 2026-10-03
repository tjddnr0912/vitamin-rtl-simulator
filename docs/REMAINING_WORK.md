# vitamin — remaining work

What stands between HEAD and the two goals. The open items and the queue are in [ROADMAP.md](ROADMAP.md) (start order: §5.2); a finished slice's record is its commit message (`git log --grep '§4.5.N'`).

- G1 = a correct open-source RTL simulator (correct-or-loud) at the level of icarus, verilator, xcelium and vcs.
- G2 = an AI-agent-friendly simulator (the observability rail; SPEC = [preview/19](preview/19-ai-agent-observability.md)).

## D. Prerequisites that block work from starting

One line each: prerequisite — what it means; what it blocks. None blocks a row on a corpus design's current page: they block frozen §2 rows, synthetic-origin §3 rows and one external-report residue (§3.b `unique-if-chain`'s subroutine bodies), and wake with them. Oracle splits are recorded, never chased.

- No t0 run of a continuous assign's function on x ahead of the `initial` that writes its inputs (§2 🆕 AB, §5.2 row 2), a constant-function arm for the synthesized no-match report (§3.b `unique-const-fn`, §5.2 row 1) and a package-scoped call closure walk that admits it (§3.b `unique-pkg-closure`) — today a continuous assign runs the function it calls once more at t0 on x, the constant interpreter refuses a call that reaches the arm, and the closure walk refuses a legal `pk::f(…)`; they block §3.b `unique-if-chain`'s residue (1), arming chains in function and task bodies.
- One current binding per key (§2 🆕 U, §5.2 row 3) — a genvar, local enum label or import that rebinds a key clears or suspends the other map's entry; blocks §2 🆕 T and 🆕 S (a)'s wide half.
- A generate-case arm decided once per construct instance (§2 🆕 V, §5.2 row 4) — the same arm and label bindings in every elaboration phase; blocks §2 🆕 T.
- The sign of a constant typed by an overridden type parameter (§2 🆕 W, §5.2 row 5) — blocks 🆕 S (a)'s i64 half, which also needs every decline to fall back to PRE's own-width compare.
- A case temp for class-method bodies (§2 🆕 X) and a string-kind capture of a string-returning scrutinee (§2 🆕 Y) — block `case … inside` accepting those shapes (§3.b `case-inside-residue`).
- A per-unit parse of a design that uses `inside` as a name (§3.b `inside-name-use`) — blocks every `case … inside` in such a design.
- A declaring-scope fold — a scope's return, formal, local and typedef ranges, defaults, constant-function bodies and package routines fold where they are declared, never at the caller's prefix (§4.5.558, §4.5.560–562 reverted on it); blocks §2 row 10, the "Real" call override, the bare >64-bit select, the package-body constant domain and `$bits`, the generate `real` shadow, typedef named dims, §3.b `pkg-string-const-select-dir`, `gen-enum-uncarried`, `string-literal-condition-residue`, `cont-array-typedef-residue`'s named bounds, `md-return-select` (a).
- A generate-scope alias's recorded type — `localparam C = A;` under a generate block records A's width and sign, a guess followed through forwarding (§4.5.559); blocks the §2 "Index sealing" `parameter signed` line.
- The override binder's unknown plane and the parameter lane's 2-state identity (§2 row 15: `hdl-parser/src/params.rs` drops `var_kind`) — blocks row 15 and §3.b `md-param-pattern-residue`'s named items and `bit` elements.
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
