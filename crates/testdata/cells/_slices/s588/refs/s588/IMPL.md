# s588 IMPL — §2 🆕 AE cut "AE-rt" (implementer report)

status: STOPPED AT STEP 0. Step 0 contradicts the plan, so per the brief no code was written.
worktree: $S/s588/wt (branch s588 @ 2f2d3f2d), clean (`git status --short` empty); no cargo build ran.
PRE: $S/s588/pre/vita md5 e1e7e57148bb83ad35d2c4f68247a290 (verified before and after; not rebuilt).
Cells: $S/s588/s0 (31 census cells + 31 `_rt` twins), $S/s588/s0e (10 legal enum cells + `_rt` twins + 4 `_hs` hand-spelled desugars).
Harness: $S/s588/g/run4.sh. A `_rt` twin has `repeat (fd(a2))` with `a2 = 2` set at run time. An `_hs` twin has
`c = fd(2); repeat (c)`, which is POST's desugar written out by hand.

## Step 0 — results

### 0a. Plan cells re-measured on PRE: all 13 byte-identical to the stored .PRE
m19 n=1 · p06 n=1 · p07 n=1 · p13 n=1 · p14 n=1 · p15 n=1 · p16 n=1 · p17 n=2 · p18 n=1 N=1 · p19 n=1 t=1 ·
p20 n=1 k=1 · l23_p n=0 · l23_d n=0 (`cmp` against the stored .PRE: 13/13 the same). The stored oracle lines are as PLAN §2 gives them.

### 0b. Callers and lanes: confirmed
- `repeat_unroll_count` has three callers: stmt_flow.rs:1119 (`lower_repeat`), frames_classify.rs:816
  (`ast_has_repeat_with_timing`, `.is_some()`) and frames_reserve.rs:685 (`collect_runtime_repeat_spans`, `.is_none()`).
- Re-grepped `Stmt::Repeat` / `S::Repeat` (26 sites). The only one that folds the count is stmt_main.rs:904, which calls
  `lower_repeat`. const_fn.rs:1705 is the interpreter's own loop. The other 24 sites only walk the body or test the
  count expression's shape (package.rs:132, da/reads.rs:799).
- Env value reads are the four in the plan and no others (grep of get/insert/remove/entry over const_fn.rs,
  const_fn_width.rs, const_select.rs, const_eval.rs, const_wide.rs, inline_fn.rs, const_bound.rs): :908 R1, :1213 R2,
  :1518 R3, :1582 R4.
- The interpreter has no memo or cache, so all three callers re-run it.
- Both module and frame counters are named `$repeat_cnt$N`. A shape pin can read them from
  `elaborate::elaborate_with_sidecars(..).2` (the `NetNameTable`) in a cli test.

### 0c. Seed-kind census: CONTRADICTS THE PLAN
What the parser records (code read):
- A type parameter is stamped `Bit`/`Logic` + `shape_param`, and `shape_kind` follows the override
  (type_params.rs:260). Not under-seeded.
- A packed struct or union is `Bit` only when every member is a 2-state atom (structs/mod.rs:237); otherwise `Logic`.
- A base-less enum is `Int`. An atom base (`byte`, scalar `bit`, `integer`) keeps the atom kind.
- **`enum bit [N]` (a 2-state VECTOR base) is recorded `Logic`** (typedefs.rs:351; the TypeInfo doc says so).
  `function e_t f` with such an `e_t` gets `ret_two_state = false` (functask.rs:201).
- In elaborate, `NetVarDecl` cannot tell this enum apart from `logic [N]`: `integral_typedef` is false for enums and
  for built-in types alike.

Measured (PRE, `_rt` twin on PRE, iverilog / verilator / sv2v). Every cell reads a never-assigned variable of the given type:
| cell | type | PRE | vita run time (`_rt`) | iverilog | verilator | sv2v | under the planned guard |
|---|---|---|---|---|---|---|---|
| e01 | local `enum bit [3:0]` | n=1 | n=2 (`_hs` n=2) | n=1 | n=1 | n=2 | seeded -> run time -> n=2: CORRECT -> SILENT |
| e05 | return `enum bit [3:0]` (`if (fr == 4'd0) fr = B;`) | n=2 | n=0 (`_hs` n=0) | n=2 | n=2 | rc=142 | seeded -> run time -> n=0: CORRECT -> SILENT |
| e04 | local `enum bit` (scalar) | n=1 | n=1 | n=1 | n=1 | n=2 | not seeded (Bit): keep |
| e08 | local `enum byte` | n=1 | n=1 | n=1 | n=1 | n=2 | not seeded: keep |
| e09 | local base-less enum | n=1 | n=1 | n=1 | n=1 | n=2 | not seeded (Int): keep |
| e07 | return, base-less enum | n=2 | n=0 | n=2 | n=2 | rc=142 | not seeded (ret_two_state): keep. The run-time n=0 is a separate pre-existing defect |
| e02 / e10 / e03 | local `enum logic [3:0]` / `enum logic` / `enum integer` | n=1 | n=2 | n=2 | n=1 | n=2 | silent -> correct |
| e06 | return `enum logic [3:0]` | n=2 | n=0 | rc=142 | n=2 | rc=142 | silent -> correct (hand-IEEE) |
| c01 / c14 / c18 | struct of logic / mixed struct / union of logic | n=1 | n=2 | n=2 | n=1 | n=2 | silent -> correct |
| c02 / c17 | struct of bit / union of bit | n=1 | n=1 | n=1 | n=1 | n=2 | not seeded: keep |
| c06 / c07 | typedef logic [3:0] / bit [3:0] | n=1 / n=1 | n=2 / n=1 | n=2 / n=1 | n=1 / n=1 | n=2 / n=2 | c06 moves, c07 keeps |
| c08 / c10 / c12 | type param logic / bit->logic / int->integer | n=1 | n=2 | n=2 | n=1 | n=2 | silent -> correct (seeded through shape_kind) |
| c09 / c11 / c13 | type param bit / logic->bit / integer->int | n=1 | n=1 | n=1 | n=1 | n=2 | not seeded: keep |
| r01 / r05 / r10 / r07 | return typedef logic / struct4 / type-param logic / integer | n=1 | n=0 | rc=142 x3, r07 n=0 | n=1 | rc=142, r07 n=0 | silent -> correct |
| r02 / r06 / r08 / r09 / r11 | return typedef bit / struct2 / int / bit [3:0] / type-param bit | n=1 | n=1 | n=1 | n=1 | (r08 n=0, others rc=142) | not seeded: keep |
| r12 / r13 | return type param with a shape-changing override | `fatal[VITA-F4004] … the override changes the t…` | same F4004 | rc=142 / n=1 | n=1 | rc=142 | loud on PRE, stays loud |
c03, c04, c05, c15, c16, r03, r04 (an `int` assigned to an enum) are refused by iverilog (`This assignment requires an
explicit cast`) and verilator (`%Error-ENUMVALUE`). e01–e10 are their legal respellings, using labels.

### 0d. Contradiction
PLAN §4 and §6 rest on two claims: "under-seeding is the only unsound direction; seed when unsure", and M5 "over-seed:
value-invisible". Step 0 refutes both.
- The decline sends the count to vita's run time.
- The run time carries the same misrecord as the parser: ROADMAP §2 Size cast line 151, "`typedef enum bit [7:0]`
  stores 4-state (reads x, keeps `X`); … 2 oracles; OPEN".
- So the planned seed rule (`shape_kind` + `net_kind_is_two_state`) over-seeds `enum bit [N]` locals and returns.
  e01 and e05 go from correct to silent-wrong, each against two agreeing oracles. The `_hs` twin on PRE shows the exact
  POST spelling giving n=2 and n=0.
- PLAN §3 predicts 0 descents. With this rule it is 2 measured descents, plus any packed struct or union that has an
  `enum bit [N]` member, which the parser records as `Logic` through the same path (not measured).

No narrowing exists in elaborate without an AST or parser change, because elaborate has no discriminator. Two options
for the planner (I built neither):
1. Treat ROADMAP §2 line 151 (record `enum bit [N]` as 2-state) as this cut's prerequisite. It also fixes e01_rt and
   e05_rt, which are pre-existing run-time silent-wrongs.
2. Add a parser-side carrier that marks an enum-typed declaration or return, and exclude it from seeding. hdl-ast
   types are SchemaHash, so this has a format and golden cost.

Also recorded: e07 (a base-less enum return, which is 2-state) runs n=0 at vita run time, where iverilog and verilator
print n=2. This is a pre-existing run-time defect; PRE's constant fold is right there. As far as I found, it is not on file.

## Steps 1–7
Not run: stopped by 0d, as the brief requires. No code, pins, gate, POST, mutants or diff.
