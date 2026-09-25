# ROADMAP — open work

This file tracks open work only: what is wrong or missing at HEAD, where the fix site is, what
evidence exists, what prerequisite blocks it, and its priority band. Anything finished is removed
from here and lives in [history/](history/README.md).

Section numbers §0–§9 are stable — tests and CLAUDE.md cite them. Row identifiers in §2 and the
circled identifiers in §3 are never reused, and source comments cite them (`ROADMAP §2 🆕 I ⓐ`,
`ROADMAP §3 ⑭`, `ROADMAP §5.1-be`). Baseline counts (tests, `format_version`, `MsgCode`) are in
the fact table in [../README.md](../README.md).

Every row states: symptom with repro and oracle values, root cause and code site, fix shape,
prerequisite, oracle status. A row's priority band is its section's band in the Summary table below
unless the row's own status says otherwise; `BLOCKED`, `WALL`, `DO-NOT-START` and `ORACLE-SPLIT` do
not start. When a row lands, its record is appended to [history/ROADMAP_ARCHIVE.md](history/ROADMAP_ARCHIVE.md)
and the row is deleted here; a residue survives as its own row.

## Summary

Composition of the open work at HEAD (regenerate this table in the docs step of every loop
iteration; it is what the iteration report shows beside the next slice). `open` is the number of
rows or bullets in the section, not slices; `startable` counts those with two oracles or a hand-IEEE
plan and no unmet prerequisite, `blocked` the rest, with the top reasons. The `next` column marks
where §5.2's start-order rows sit: `1` is the row the next iteration takes, `2`, `3`… the ones
behind it, so the queue and the composition are read from one table.

| § | track | open | startable | blocked | blocked by (top reasons) | composition | rung | next |
|---|---|---:|---:|---:|---|---|---|---|
| §2 | silent-wrong start-order table | 27 | 0 | 27 | WALL §11.8.1 region sign / declared-width provenance 9 · named prerequisite 7 · one oracle + zero demand (clocking) 3 · oracle split, never chased 3 · residues held on purpose or zero demand 3 · performance, not a §2 correctness item 2 | LOUD 4 · BLOCKED 6 · WALL 6 · OPEN 4 · ORACLE-SPLIT 3 · PERF 2 · DO-NOT-START 2 | ① | |
| §2 | recorded defects by mechanism | 158 | 81 | 77 | oracle split / pinned / oracle disqualified 43 · named prerequisite 17 · WALL (AST self-width) size-cast cluster 6 · one oracle 3 · held on purpose 1 · pair columns not measured 1 | inline / frame binds 15 · size cast / signedness 16 · constant domain (i64) 12 · scoping / imports / block-locals 27 · delays / events 16 · real 5 · performance 6 · index sealing 11 · ranges / selects 6 · diagnostics 8 · class fields 3 · oracle splits 33 | ① | |
| §2-N | verilog-axi census | 2 + 5 | 0 | 7 | t0-event residues held on purpose 5 · needs a second oracle or a digest ruling 1 · upstream fst-writer API 1 | x-cycle promotion · FST `$dumpvars` snapshot · five t0-event residues | ① | |
| §3.a | loud → correct-support, numbered | 24 | 19 | 5 | named prerequisite 2 · loud by design 2 · deferred to §5 performance 1 | file-I/O hoisting 4 · ibex ladder ⑤ 9 · system functions in function bodies 4 · package and the rest | ② | |
| §3.b | loud → correct-support, small | 105 | 90 | 15 | named prerequisite 6 · oracle split / unmeasured 5 · by design or trigger-gated 3 | subroutine / frame 25 · constants / parameters 20 (the pkg-type-param-import row) · parser accept 15 · system tasks & file I/O 9 · nets / timing 11 · loud shapes from §4.5.493–495 7 · strings / heap 8 · diagnostics quality 7 · VCD / real conversion 3 | ② | 1 |
| §3.c | intentionally loud | 12 | 0 | 12 | by design 6 · oracle split or disqualified oracle 4 · non-goal 1 · prerequisite 1 | not gaps; each row states its reason | — | |
| §0 | correct-support promotion queue (T2 residues) | 14 | 9 | 5 | non-goal + oracle split 2 · deliberate / withdrawn fix 2 · inherits the §8 `defparam` non-goal 1 | real const-fold ⓐ–ⓗ · enum-label folding · negative bounds · `-G` aliases · `case inside` | ③ | |
| §4 | SVA honest-loud | 6 | 0 | 6 | an explicit prerequisite on every row; no oracle on 3 | mostly no oracle; hand-IEEE when started | ③ | |
| §6 | G2 observability (OBS) | 6 stages + 10 | 15 | 1 | CALL TREE: two lowering paths (doc-19 §4.9) 1 | OBS-2 → OBS-1 → R-L4 → OBS-4 control → OBS-5 snapshot → OBS-6 X-origin, plus call tree / a `void` function filed as `kind: task` / a route decided per spelling and seven more beside the track | ④ | |
| §5.b | performance / hardening | 17 | 8 | 9 | named prerequisite 5 · trigger-gated 2 · census-first 1 · on hold 1 | frame-body wprog · scratch pooling · array-LHS cliff · inline-fold exponential · memory guard · CI nextest · MSRV ceiling | below the ladder | |
| §7 | conditional / long-term | 4 | 0 | 4 | trigger-gated re-entry 4 | BACKEND · VHDL · VCD-EXT · MVP-CUT | trigger-gated | |
| §8 | non-goals | 2 | 0 | 2 | permanent 2 | IMPLICIT-NET · `defparam` beyond a direct-child constant | permanent | |
| total | | 392 | 222 | 170 | | | | |

Prerequisites that block rows from starting are listed in REMAINING_WORK §D (§11.8.1 region sign,
a wide SELECT resolver, a tree-wide AST self-width pass, an exact declared-width fold for
hierarchical placeholders, a declared width for array-reduction / string / placeholder cast operands,
out-of-range real→integer conversion in the engine, a per-resumption-kind ordering model, a block-scoped constant binding, per-instance arity / class registration, one-oracle clocking).

Priority principle (time-invariant): ① a CRITICAL silent-wrong with an oracle, then ② loud→supported
with an oracle, then ③ an honest-loud promotion whose prerequisite holds, then ④ G2 OBS. Performance
does not enter this ladder. "No oracle" is not a reason to defer: implement from the LRM and pin by
hand.

## 0. correct-support promotion queue

Deliberately loud, not gaps: `new[]` on a fixed array; a multi-dimensional partial index `s[0]`
(iverilog rejects both); a cross-type SoA whole-element copy; a `real` scrutinee in a `generate case`.

### iverilog defects (vita is IEEE-correct) — oracle disqualifiers, regression-pinned

| # | repro | iverilog | vita |
|---|---|---|---|
| ① | `.len()` of an element of `string s[5]; s[0]="abcdefg"` | 5 (the array size) | 7 |
| ② | concurrent fork activations share an `automatic` string array | `A!` | `A!!` |
| ③ | `$fmonitor` twice on the same fd | accumulates (contradicting its own singleton `$monitor`) | replaces per destination |
| ④ | `%s` of an empty string-array element | one blank | the empty string |
| ⑤ | `$clog2(4'sd7+4'sd1)` (§20.8.1 = the bit pattern at the argument's own width) | 32 | 3 (verilator also 3) |
| ⑥ | `$itor(64'h1_0000_0008)` (unsigned and signed `longint` both give 8, so this is not the sign axis) | 8 | 4294967304 (verilator identical) |
| ⑦ | `s<"ab"`, `s<"aa"`, `s<"zz"` with `s="ab"` | all 1 (they cannot all be true) | `0 0 1` (verilator identical) |
| ⑧ | an `automatic time signed` local inside a frame function | `Assertion failed: (index < get_max(fun_thr, val)), of_RET_VEC4, vthread.cc:5466` (abort) | `-4`, with verilator as the sole oracle |

### T2 residues (each its own slice)

| id | symptom · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle |
|---|---|---|---|---|
| 8ⓐ | implicit conversion `logic [R-1:0]` and `{R{1'b1}}` from a real, loud | — | non-goal | split (width: verilator rejects, iverilog 3; count: iverilog rejects, verilator 3) |
| 8ⓑ | a real value in an untyped `localparam`, loud | a real parameter under §6.20.2 | rounding here is a withdrawn silent-wrong | 2-oracle |
| 8ⓒ | a real override `#(.R(2.5))`, loud | the override channel is i64 | widen the channel to real | 2-oracle |
| 8ⓓ | `1.0/0.0`, loud | the real domain refuses non-finite values on purpose | deliberate | 2-oracle |
| 8ⓔ | `R<<1` (an operator undefined on real), loud | — | non-goal | split (iverilog rejects, verilator 6) |
| 8ⓖ | `$rtoi` in a constant-function body, loud | the module-scope resolver would allow a shadow | move to an environment-aware walk | 2-oracle |
| 8ⓗ | nested `int'(real'(R))`, loud | — | widen the explicit-conversion boundary | 2-oracle |
| 10ⓐ | a `parameter` label must not be folded — an override changes the label's value (`m #(.K(9))` gives iverilog 10 / `first=9`), and the parser runs before overrides | the parser's `const_locals` label fold | move the enum-method desugar into elaborate (architectural) | 2-oracle |
| 10ⓑ | a `localparam L = 8'h5` label does not fold, so the enum never enters `enum_defs` and every method is a loud "hierarchical function call" | `const_locals` records decimals only | that table is shared with generate indices, so widening the producer moves another consumer (its own item). Keep rejecting literals that need truncation (unsized `'h1FFFFFFFF`, mis-sized `4'hFF`) — both oracles reject them too | 2-oracle |
| 11 | negative range-bound residue = a PART select `x[1:-2]` (honest loud) and ports/formals (warn + clamp) | bound folding is unsigned | the port asymmetry is deliberate opt-in | 2-oracle |
| 14-a | `-pvalue+<name>=<val>` is unimplemented (`grep -rn pvalue crates/` = 0 hits) | — | it is an alias of `-G`, so argv parsing only | n/a |
| 14-b | `-P<path>=<val>` (a hierarchical path) is unimplemented | `defparam` is direct-child only | it inherits the same restriction | n/a |
| 14-c | two `.velab` files built from different `-G` values have byte-identical 128-byte headers, so the gate cannot tell them apart (values are right; the risk is provenance) | `-G` does not enter the RULE-V upstream digest; mixing them makes `vrun --upstream` report a false `E9003 digest changed` on an unchanged `.vu` | a header field of its own (doc-14 §RULE B) = `format_version` bump | n/a |
| 13 | `case (x) inside {…}` is loud | — | hand-IEEE plus an internal differential | no oracle (iverilog 13.0 rejects `case inside`, the `inside` operator and array reduction) |

## 0-B. Small follow-ons (kept loud)

- `void'(getnext())` — a void cast of a function with an output formal; forwarding a frame-formal
  array into a nested hierarchical call (OUTPUT/INOUT); a size cast on a param or call leaf,
  `8'(P*a)`.
- fork-in-frame residue (minor, safe): duplicate resolve-time re-walk in
  `fork_arms_self_contained`; the shared `enter_task_frame` arm comment; an elaborate-time reject
  for a fork arm that calls a forking task (the `F4004` tie-cap runtime guard makes it safe today,
  and a clean `E3009` would be clearer); same-instant zero-delay sibling visibility is not
  differentially verified.

## 0-C. Large items — start decision table (do not re-estimate the size)

| item | cost | payoff | prerequisite / trap |
|---|---|---|---|
| A. file-position family (`$ftell`, `$fseek`, `$rewind`, `$ferror`) | a `format_version` bump is certain | medium | a new `SysFuncId` is a frozen-root change: re-pin the SimIr schema hash, the canonical string and the RON goldens, and invalidate every `.velab`. A sidecar cannot route around it (no overlapping existing id, measured). `$feof`/`$fgetc`/`$ungetc` alone are possible without a bump |
| B. shared literal-parsing crate | medium to large (559 lines moved, an adapter, and every literal re-verified) | small — the rejected shapes are truncating literals (`4'hFF` in `[3:0]`) and unsized + `s`, and iverilog rejects truncation too; the gain is removing a two-predicate hazard | `literal.rs` depends on `sim_ir::{BitPacked,ConstRepr,ConstVal}`, so moving it makes hdl-parser see sim-ir (a layering inversion). The split is digit→bits (neutral) versus `ConstVal` packing (IR) |

Order A > B: A when the format bump is worth taking, B only when removing the two-predicate hazard
earns priority.

## 1. Start priority — principle only (the live queue is §5.2)

1. A CRITICAL silent-wrong with an oracle (§2). Correctness is this repository's top principle.
2. loud→supported with an oracle (§3; additive, therefore low risk). "There is no oracle" is not a
   reason to defer — build from the LRM and pin by hand.
3. An honest-loud promotion whose prerequisite holds (§0, §4, §5).
4. A G2 OBS slice (§6).

Performance does not climb this ladder. Do not create a new queue here.

T4 (opportunistic): a function-local array element write costs 514 ns against iverilog's 24 ns.

## 2-N. Silent-wrong from the verilog-axi census

| id | symptom · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle |
|---|---|---|---|---|
| 2-N-1 | verilog-axi is not promoted: `m_axi_awvalid` / `m_axi_wvalid` / `m_axi_arvalid` are x in iverilog and 0 in vita just after reset — 29 of 123,166 cycles (`XC=29` against `XC=0`; invariant at N=200, digest unchanged). Function matches (same completion cycle); vita is the optimistic side, which hides x-propagation bugs | the crossbar reaches a register slice through a computed wire (`int_s_axi_wready[m] = int_axi_wready[w_select_reg*S_COUNT+m] \|\| w_drop_reg`), and only vita raises a t0 event. vita's rule: a driver that COMPUTES has an initial state, a driver that MOVES bits does not (`sim_engine::alias::copy_nets`) | do not chase without a second oracle; promotion requires either a digest that does not count x-cycles or an oracle-split ruling | oracle-split: `assign w = a \| b` gives iverilog c=1 but `a & b` gives c=0 with identical operands and values; `pr & 1'b1`, `~(~pr)`, `{pr}`, `1'b1 ? pr : 1'b0` fold while `pr \| 1'b0` and `pr ^ 1'b0` do not — that is where the elaborator stops. verilator is not the tiebreak either: `a=10 b=11` against iverilog's and vita's `a=x b=x` |
| 2-N-2 | FST loses the `$dumpvars` snapshot: two designs differing only in initialization produce byte-identical 473-byte `.fst` files (every signal `x`, exit 0); 24 designs with differing VCD produce identical FST, so a waveform differential oracle is impossible | not a missing time step — opening time 0 lazily (the arm fires once) leaves `xxxxxxxx` unchanged | the value is absorbed into fst-writer's per-variable INITIAL value, so the next step is that library's initial-value API | n/a |

t0-event residue (pre-existing, held on purpose):

- `assign w = 1'bx;` — a computed driver whose value is `x` also raises a vita t0 event where
  iverilog does not; vita's driven-net default is `z`, iverilog's is effectively `x`.
- A truncating copy `wire [3:0] w; assign w = r8;` — iverilog collapses it, vita treats it as
  computed (widening fires on both sides). `assign #1 w = r;` — iverilog 0, vita 2. Multi-driver and
  two-driver `wand`/`wor` — iverilog 0, vita 1 (a single driver is resolved by the copy rule). A
  concat lvalue `assign {x,y} = …` is excluded by the single-chunk gate.
- vita's dirty channel is per NET where iverilog's collapse is per BIT, so a constant driver on
  `bus[1]` wakes a reader of `bus[0]`.
- Oracle split `wire w; assign w = 1'b1; reg r = w;` — iverilog `z`, verilator and vita `1` (§6.8
  fixes only what precedes a procedure, and a continuous assignment is not a procedure). Pinned only.
- `buf b1(o1, zin)` is vita `x` and iverilog `z` (the neighbouring `assign o2 = zin;` is `z` in
  both). The LRM table says x, and `oracle_split_rulings.rs` pins it: a `buf` is not a bit move but
  the IEEE 1364 §7.3 z→x coercion.

## 2-R. Usability residue

- An unused package function is still framed, and one cause is reported once per instance.
- Multidriver Rule A's `Unknown` verdict for an unresolvable (hierarchical) callee spills an E3001 onto a READ-ONLY net of the same `always_comb` (`t` in `n = u.w.size() + (t ? 1000 : 100)`); the design is loud for the call anyway.

## 2. Silent-wrong residues

Row identifiers are cited from tests (`§2 row 7/14/21/25/27/33`, `§2 🆕 I/L/M/N`); a number is never
reused. Resolved rows are listed in [history/ROADMAP_ARCHIVE.md](history/ROADMAP_ARCHIVE.md).

WALL(provenance) — rows 14, 15, 16, 25, 26, 30, 🆕 F and 🆕 R stop in one place: `const_wide.rs`'s
`fold_bits_at` decides an expression's sign NODE-LOCALLY (`sg = ls && rs`) where §11.8.1 makes the
whole region unsigned if ANY operand is, and a module-scope initializer folds through the
width-UNLIMITED `const_eval_in_scope` while a function local's declared width lives in `envw`.
Routing a DECLARED-width/sign target through `eval_const_assign` moves the cells (row 14: 27
divergent down to 3 of 44), but the shared walk is not correct on its own terms — §11.4.10 makes a
shift's RIGHT operand self-determined and unsigned, and the i64-lane bound is on the TARGET only.
`param_declared_width_provenance.rs` pins the current state, the prerequisites, and the cells a fix
must not move.

WALL(AST self-width) — the size-cast cluster below (the width probe, the `ir_bits_of` fallbacks,
real × fill, the prim cast) needs a tree-wide AST pass that answers a node's self width WITHOUT
lowering it. That pass already stands INSIDE a cast (`const_self_width` + `const_signed_env`).

| row | status | symptom · repro · oracle values | root cause · code site | fix shape · prerequisite / wall |
|---|---|---|---|---|
| 🆕 B | BLOCKED (sign provenance) | ⓐ `localparam [31:0] L1=(B>>>2)+8'd0` is 4294967276 where the runtime twin and both oracles are 44; net size is polluted too (`logic [((B>>>2)+8'd0)-1:0] bus` is 22 bits against 44) · ⓑ `case (b>>>2)` with an unsigned label: vita `eq236`, oracles `eq44` | ⓐ `const_fn.rs:162` `AShr => Some(a >> b)` carries no sign in its signature; the wide twin `const_wide.rs:308` uses the left-operand rule · ⓑ `stmt_flow.rs:~605` wraps the lowered scrutinee in an outer `$unsigned`, whose argument is self-determined | ⓐ the right rule is one file over at `const_fn_width.rs:427`; WALL(provenance) · ⓑ re-lower with `lower_size_ctx_entry(scrutinee, w, ext=false)` keeping the wrapper as a FALLBACK (6 `case`/`casez`/`casex` cells; `case (b/c)` 1 → 3, `b%c` 2 → 1). BLOCKED BY: sign provenance told apart from a default — `expr_self_signed`'s catch-all is not a fact for calls, non-whitelisted system functions, or constants folded from them |
| 3b | BLOCKED (field-key map) | class-property ascending/negative bound normalisation has nowhere to be recorded | class fields are not nets (`ClassField` → heap slot) and the map is keyed by NetId | BLOCKED BY: a field-key normalisation map · 1 oracle (iverilog dies on an assertion) and the minimal repro is loud for another reason (`C c = new();`) |
| 7 | BLOCKED (one ordering key) | a parent `initial` READING a child net at t0 sees X: `initial s = 8'hEE` in a child, read as `r = u1.s;` → oracles `ee`, vita `xx` (2-oracle). A fork arm in the child is a 3-way SPLIT (iverilog `ee` / verilator `00` / vita `xx`). An output PORT bind `child u1(.o(w))` and `assign w = u1.s;` both read `ee` at the parent's t0 immediate read, as do a constant driver, a parameter driver and a two-level chain — the only diverging spelling is the one whose value comes from the child's `initial`, i.e. process order; verilator answers `ee` there only because it constant-hoists a SINGLE-statement `initial s = <const>;`, and a second statement in that initial moves verilator to vita's answer, so that cell is an ORACLE SPLIT on order. `final` blocks run in ProcId order (`final_procs` is a `BTreeSet<ProcId>`), so any process reordering leaves vita disagreeing with itself | the rank machinery is complete (`with_rank_scope`, `init_ranks`, `RANK_MOD_INSTANCE(1) < RANK_MOD_OWN(2)`) and is applied only to declaration-initialiser processes. The root is that vita has ONE ordering key (`Activity.tie`) where the oracles use a DIFFERENT order per RESUMPTION KIND | BLOCKED BY: a per-resumption-kind ordering model. A single `proc_order` permutation seeded into the ordering key cannot express it — iverilog answers the kinds differently in ONE run of ONE design (`initial` child-first, `always_comb` t0 PARENT-first, edge PARENT-first, `#d` delay child-first, `wait` PARENT-first, fork-arm wake PARENT-first): keying only the t0 arm breaks the delay wheel, keying `Activity.tie` breaks the edge and `wait` wakes (`aa` for `cc`, a VALUE), and re-keying those two breaks `always_comb`'s t0 arm and the fork-arm wake, with both oracles against vita every time. Closes the headline plus 5 more 2-oracle cells. Costs a format bump (`proc_order` on the `StagedExtraSidecars` tail; `sim_ir::Process` is untouched) · corpus demand zero |
| 10 | OPEN | the RANGE BOUND consumer: `wire [K[31:24]-1:0] n;` on a `parameter [135:8] K` 128 bits wide is ONE BIT at exit 0 where both oracles declare 221, and the ≤64-bit twin of the same text is correct. Bare `K[31:24]`, `pk::K[31:24]`, `K[31 -: 8]`, `K[24]` and a `$bits`-sized net are all three-way identical | `const_range_bound_fold` has no wide-bit-domain fallback — the i64 select fold declines a >64-bit base (`select_base_at_declared` returns `None` for `dwidth > 64`) and both of the bound's fallbacks are i64. `$clog2` of the SAME text answers 8, so the value exists one funnel over | BLOCKED BY: `selfdet_bits_unsigned` declines the select too (only `selfdet_clog2_wide` answers it), so routing the bound at the wide domain buys nothing until that resolver reads it — an unguarded fallback moves 0 of 18 cells |
| 14 | WALL (provenance) | `localparam logic signed [7:0] NM = -8'sd2; localparam logic [63:0] X = NM ^ 64'h0;` is `fffffffffffffffe` against the oracles' `00000000000000fe`; the same expression over a FUNCTION LOCAL folds `00…fe`. The same routing also fixes `localparam logic [7:0] M = (P + 8'd100) % 8'd7` (P=200: 6 against 2), `pk::PA ^ 64'h0`, `int S = (D - C) / 2` (2147483641), the generate-scope `time NM` shadow (2c for 12c), and the 65-bit-leaf `/` and `%` | a module-scope initializer folds through the width-UNLIMITED `const_eval_in_scope` | route a DECLARED-width/sign target through `eval_const_assign`; the gate must demand provenance of every LEAF (`param_meta` is a DEFAULT for an untyped parameter and ABSENT for a `time` one), decline above the i64 lane, refuse an unsized FILL operand, answer THREE-valued in the scope walk, and NOT route the PACKAGE binder (row 26). BLOCKED BY: a width-aware walk correct on its own terms — the §11.4.10 shift count (`16'hFF01 << 3'b101` is 0; 30 correct→wrong plus 36 loud→wrong, reachable through a constant FUNCTION, so it is its own row and closes first) and an i64 bound that is not on the TARGET only (83 cells) |
| 15 | BLOCKED (2-state field) | an OVERRIDE carrying a sized x/z literal loses the unknown plane: `#(.K(8'b1010_010x))` onto `parameter logic [7:0] K` binds `10100100` at exit 0 where the oracles keep the x; `8'bzzzzz1z0` binds `11111110`. Five cells, every channel | `params.rs`'s i64-lane test reads only VALUE bits (`bp_get(..).0`); the sibling `fill` arm declines with `fill_is_unknown` | a `bp_any_unknown` test alone turns 76 CORRECT cells loud, because a 2-STATE declaration converts x and z to 0. BLOCKED BY: recording the parameter's 2-state-ness (`hdl-parser/src/params.rs` computes `var_kind` and drops it; an `hdl-ast` field plus a SchemaHash re-pin, parser-only), which also closes z→0 (1 cell today) · the separate headline (an unknown plane in the narrow store, 22 loud cells, ~40 sites, demand 0) stacks on row 14; above bit 64 what survives is z, not x · the >64-bit operator lane (§4.5.527) declines on any x/z bit, so `#(.P(~128'bx))`, `128'h1x << 120` and `128'hz5 >>> 2` stay E3009 where both oracles bind 128 bits with the x/z kept; a build that let the plane through lost it whenever the value bits fit i64 (`128'hx0 >> 4`, `+128'hx0` bound zeros, six loud→silent cells), so this lane waits on the same unknown-plane binder · an overridden parameter used as a constant EVENT term inherits the lost plane through §4.5.529's time-0 run: `child #(.P(4'bx))` with `always @(P or clk)` runs at time 0 printing `P=0000` (`4'bzzzz` → `P=1111`, `4'b1z0x` → `P=1100`), where iverilog prints `P=xxxx` / `P=zzzz` / `P=1z0x` and does not run an all-x or all-z constant at time 0 (it runs `4'b1z0x` there); verilator cannot compile the design (`Unsupported tristate construct: SENITEM`) (dS10c) |
| 16 | ORACLE-SPLIT | 12 override cells: 5 diverge from iverilog, but verilator sides with VITA on 4 (`-64'd1`, `<ones> + 64'd1`, `<ones> << 4`) and `~32'd0` is a 3-way split | that is row 17 | a fix would "correct" one side of a live split, so the context is not threaded through `override_bits` · the 2-oracle sub-case (operands already ≥ the target width, `#(.K(~128'd0))`) is closed by §4.5.527 · the >64-bit operator tops narrower than a typed target now land on VERILATOR's side, recorded here, not support: onto `parameter logic [127:0]`, `~65'd0` and `65'd1 - 65'd2` are `0000000000000001ffff…ff`, `~96'd0` is `00000000ffff…ff`, `~65'd0 >> 1` is `0000000000000000ffff…ff` (iverilog all ones / `7ff…ff`); onto `logic signed [127:0]`, `~96'd0` / `~72'd0` are `00000000ff…ff` / `00000000000000ff…ff` (iverilog all ones; PRE matched iverilog by an i64 sign-extension coincidence); onto `[255:0]`, `~128'h…` is zero-extended (`hier_param_select.rs`). verilator's OWN `localparam [255:0] L = ~128'd0` is all ones (= iverilog), so verilator contradicts itself between the override and the localparam twin; adjudicate before calling these pins support |
| 17 | ORACLE-SPLIT | `leaf #(.K(32'd0 - 32'd1))` on `parameter logic [127:0] K`: iverilog `ffff…ffff`, verilator `0000…0000ffffffff`, vita `0000000000000000ffffffff_ffffffff` (zero-extending from 64, the i64 lane's width) | — | do not chase: vita matches NEITHER and §6.20.2 does not settle it · the neighbours are not split (`64'hFFFF_FFFF_FFFF_FFFF + 64'd0` zero-extends in all three, `-(64'sd1)` sign-extends in all three) |
| 19 | PERF | a 2-D, 3-D or packed element as a continuous-assign LHS costs ~10× on BOTH backends; against 50.0 ns for a 1-D unpacked element: 2-D `arr[0:15][0:3]` 546.7 ns native / 675.8 vm, 3-D 829.2 / 967.5, packed `logic [63:0][31:0]` 410.8 / 441.7 | not located; the native/vm ratio is 0.81–0.93, so it is SHARED PLUMBING | needs its own census; the diagnoses offered so far were refuted on the 1-D axis |
| 23 | LOUD | `clocking cb; input a_b;` beside `clocking cb_a; input b;` is legal (verilator `R1=17 R2=34`) and vita refuses it with ``net/variable `top.__clk_cb_a_b` redeclared`` at exit 1 | the `__clk_` / `__clkout_` mangling in `sva_clocking.rs:727` and `:657` | correct→loud with verilator as the accept/reject oracle, so it belongs to §3; a new sigil must be taught to the VCD and FST filters · the naming half is a one-token fix (`:745` computes the instance-qualified `alias`, `:750` re-formats it without `fq`; the `[in …]` suffix is `lvalue.rs:179`) |
| 24 | DO-NOT-START (see row 34) | 24a CLOBBER (silent-wrong, exit 0, verilator oracle): a signal merely DECLARED as a clocking output is destroyed to `x` or frozen — `vita x,171,171,171,171,171` against `verilator 170,171,172,173,174,175`; 4 cells, one across a module boundary · 24b a one-cycle LAG, 4 cells | 24a `init_diag.rs::clocking_commit_plan` (~1202), OUTPUT phase unconditional; the INPUT phase is correct · 24b the §14.16 skew `#0` in Re-NBA, a scheduler-REGION question with no anchor | 24a = a written flag produced at the write site; `out_pairs` grows a third field riding `SimOpts` out-of-band (no format bump) · corpus demand 0 |
| 25 | WALL (provenance) | `parameter P = 5` with `#(.P(32'hF0F0F0F0))` binds a SIGNED 32-bit −252645136 (`P < 0` is 1, `%0d` negative); `parameter Q = 8'sd1` with `#(.Q(32'hDEADBEEF))` is `ef` with `$bits(Q)` 8 — the oracles bind the override's own type (§6.20.2) | `params.rs::param_decl_width_opt`'s literal arm answers the DEFAULT's literal type even when `default_binds == false`; `ResolvedOverride` carries `signed` but no `width`. The operator twin of the same wall: `parameter HE = ~8'h5A` with `#(.HE(~4'h5))` binds 32 bits where verilator binds 4 — the default lane sizes by the operator and the override lane does not | a producer patch is kept out of tree (`scratchpad/r29/row25/producer.patch`, 317 lines and 5 tests; it fixes 7,982 of 122,774 cells and serv's `\|WITH_CSR`) but each shape of it opens a NEW correct→silent edge (`defparam` with a NAME rhs, a `time` parameter with a DECIMAL default, `$signed(64'h…)` resized down); the size-cast slice ships `param_type_guessed`, which DECLINES every guessed type. BLOCKED BY: a parent-side resolver that DECLINES on meta-less names and answers `$signed`/`$unsigned` by operand, carried through every channel including `defparam`; a fill onto an untyped parameter binds `(1, false)` |
| 26 | WALL (provenance) | routing the PACKAGE binder through the width-aware fold is a net loss: over 8,748 package-consumer designs it is 1,233 correct→silent-wrong against 714 fixed, plus one correct→loud | it makes `pk::X`'s stored value canonical (i64 −2 for `logic signed [7:0] PA = 8'hFE`) while every consumer still folds through the width-unlimited walk and sign-extends it | the prerequisite is on the CONSUMER side: `every_name_has_a_declared_width` does not enumerate `ExprKind::PkgScoped`, and an imported constant is in no provenance set — closing both turns those 1,233 cells correct · until then the identical text answers `00…fe` in a module and `ff…fe` in a package |
| 30 | WALL (§11.8.1 sign) | `localparam logic [127:0] C = '1 ^ 1'b0;` is vita `…00000000ffffffff` against 128 ones in both oracles; the same text as `r = …`, `assign c = …` and through a port prints 128 ones. 165 of 264 cells (22 operator forms × {32,33,64,65,96,128} × {`logic`,`logic signed`}), 0 splits, all four binder copies. The band is a property of the operands, not the width: `logic [7:0] A = '1 >> 2` is `ff` against the oracles' `3f`, 82 more cells at widths 1..31 | the wide fold's fill arm plus node-local region sign | the 4-piece shape measures 778 FIXED / 0 new-silent / 0 new-loud over 1,622 cells (`fold_bits_at`'s fill arm folding at `ctx` when `ctx>0` and not x/z; `param_i64_fill_at_declared` ahead of the i64 walk in all four binders; a LOCAL predicate with a `Cast` arm; a `fill_width_survived_the_fold` guard) but it does not stand alone: §11.6.1 evaluates at `max(ctx, every self-determined operand's width)`, so freezing the fill at the LEAF is wrong (150 cells); the ROUTING predicate cannot serve as the guard (104 NEW-LOUD, since a decline at `param_bits_at_declared` is `E3009`); and of 504 loud→value cells 215 are silently wrong on the SIGN axis (`localparam logic [7:0] B = ($signed(4'hF)+1) \| 8'h00;` is `00` against the oracles' `10`, no fill anywhere). BLOCKED BY: §11.8.1 region sign in the wide fold. ACCEPT set = "correct a value, never create one" |
| 🆕 F | WALL (§11.8.1 sign) | a narrow SIGNED operand is ZERO-extended in an unsigned context: `localparam logic [7:0] A = 8'hFF - (-1'sb1);` is `fe` and `8'hF0 \| (-1'sb1)` is `f1` against the oracles' `00` and `ff`. 24 cells at widths 2..32, both sign declarations, no fill involved | §11.8.2 reinterprets each operand at the EXPRESSION's sign; `const_wide.rs`'s bitwise and arithmetic arms compute `cs = ls && rs` then `resize_bits(.., cs)` for BOTH | the same root as row 30's prerequisite — file the fix once |
| 🆕 R | WALL (§11.8.1 sign) | the shared wide walk `fold_bits_at` is wrong inside a self-determined position and on the sign of context-determined operands, on the `localparam` lane at PRE and POST alike (2-oracle CLASS): `localparam logic [127:0] L = $signed(~8'd1 + 128'd0);` is `0…0fe` against both oracles' `ff…fe` (the same text as a whole override, through the self-determined arm, too); `{~8'd1 + 120'd0} + 128'd0` is `0…0fe` against verilator `00ff…fe`; `~S8 + 128'd0` (`S8` signed [7:0] −3) is `0…02` against `ff…f02`; `-(8'sh80) + 128'd0` is `0…080` against `ff…f80`; `(S8 >>> 1) + 128'd0` is `ff…fe` against `0…07e`; `(8'sh80 * 8'sd1) + 128'd0` is `ff…f80` against `0…080`; `(-8'sd1 >>> 1) ^ 128'h1_0000_0000_0000_0000` is `fff…feff…ff` against `7ff…feff…ff`; `(S8n + 8'sd1) * 128'd1` (−3) is `ff…fe` against `0…0fe`; `int'(~8'd1) + 128'd0` is `0000000000000000fffffffffffffffe` and `int'(S8n) + 128'd0` `0000000000000000fffffffffffffffd` against `0…0fffffffe` / `0…0fffffffd`; `128'd0 + {(S8n + 128'sd0) \| 128'd0}` into `[255:0]` is `…ff…fd` against `…0fd` | (1) a self-determined position (signing cast, concatenation part, replicate, comparison operand, shift count, `**` exponent, reduction, select index, system-function argument) folds its inner at ctx 0 (`fold_bits_at0`), so a narrower context-determined operator inside it is computed at its own width; (2) §11.8.2 pushes the EXPRESSION's sign into every context-determined operand, but `~ - << >> >>> **` extend a leaf with the leaf's own sign, a signed binary node with its own, and `>>> / % **` read the node's sign — the node-local sign of the WALL(provenance) paragraph | two-pass at each self-determined position (pass 1 = the position's self width, pass 2 = fold its inner at that width) plus a top-down sign push-down; then §4.5.527's `const_wide_num::wide_operator_tree_is_plain` can widen to mixed-sign trees and operators inside positions, and the ≤64-bit operator tops of §2 "Index sealing" can route through the wide fold. Half (2) is 🆕 F's root — file the fix once; half (1) has no wall of its own · measure every admitted shape's `localparam` twin on PRE first |
| 🆕 H | BLOCKED (§11.8.1 wall) | ⓐ `(&4'b110x)` and `(\|4'b101x)` clamp to one bit in a bound where both oracles answer 3 or 4; 8 cells (`^` and `~^` are split). The class is wider than reduction — `~&`, `~\|`, `===` and `&&` with a 0 operand are IEEE-definite and all silently become one bit (both oracles 4, 3, 4, 3) · ⓑ the `\|P` bound of an ascending `parameter [0:3] P` or a lo≠0 `parameter [7:4] P` is loud in BOTH the module and the package lane (both oracles 4; 5 cells). §4.5.488's width-only twins do NOT reach it, measured: a reduction bound needs the operand's BITS through `wide_name_bits`, not only its width · ⓒ `localparam E = 4'hF \| 4'h0; wire [(&E)+2:0]` is loud (both oracles 4) · ⓓ `localparam R = ~(\|4'b1010);` is deliberately loud (both oracles `0`, `$bits` 1) · ⓔ a reduction after a local assignment in a constant-function body (`t = a[5:0]; return (\|t)+2;`) is loud (oracle 3) | ⓐ `fold_self_bits`'s reduction arm declines on a single unknown (`bp_any_unknown`) · ⓑ `narrow_param_bits` / `pkg_const_narrow_bits` reject `lo != 0 \|\| ascending`, and that decline is the one §4.5.488 deliberately kept · ⓔ the body local's width is not in `envw` | ⓐ the fix site is the wide fold's ACCEPT SET, so it falls under the §5.2 do-not-start line · ⓑ a reduction-only, layout-independent resolver · ⓒ WALL(provenance) |
| 🆕 I | OPEN | ⓐ another process's read in the same delta: `initial #1 v = 8'hA5;` declared before `initial #1 $display(c);` — oracles `a5`, vita `00`. Held on purpose · ⓒ residue on the same axis, also held: a PROCEDURAL index, a DELAYED constant driver and a gate-driven one; `assign c = r + 8'd0;` with no array at all reproduces them · ⓔ `bit [7:0] c; assign c = v;` is excluded and unexercised (vita refuses `bit` copy destinations, `E-ELAB-LVALUE-KIND`) · ⓖ a callee body is ONE set of expressions, so a root is marked only when EVERY calling process writes it (3 of 4 cells): a mixed-caller population keeps PRE's value in BOTH callers (§4.5.438's pin). The WIDTH-CHANGING half closed in §4.5.521 — a `logic [15:0]` destination taking a sign-extending copy of a `signed [7:0]` source reads `ffa5` on every spelling now, and the row's own recorded t0 cell answers iverilog's `ffa5 ffa5`; verilator is DISQUALIFIED for this class by self-contradiction (the same FIRST read of the copy is `ffa5` alone and `0000` when a later read exists). Zero-extending (iverilog `zzxx`, verilator `00a5`), truncating (`xx` / `5a`) and part-select-rhs copies stay on PRE's value as recorded splits, and a `force`d copy is not forwarded at all (`copy_alias` excludes a forced net). Recorded splits, not chased: the RUNTIME index `m[k]` (iverilog reads `xx` exactly like vita and only verilator reads `a5`; iverilog answers stale on an INDEX change and fresh on a SOURCE change in the same design); an UNSIGNED narrow net index into a NEGATIVE-base array (iverilog's answer depends on the ARRAY'S SIZE for a fixed index pattern and fixed `lo` — threshold 4 for `m[-2:1]`, 5 for `m[-6:1]` and for `m[-2:9]` — which no reading of §7.4.6 licenses, and verilator has no `x` for an out-of-range unpacked read at all, so its `a5` is masking; `array_word_index_domain.rs` records this, the SIGNED declaration is correct at HEAD, and vita's own `a5` at width ≥32 is the inconsistent half, a 32-bit `Add` in `dim_coord` overflowing to coordinate 0); a 2-D array word `reg [7:0] m[0:1][0:1]; assign c = m[0][1]` (iverilog `a5` / verilator `00`); `m[1][2]`; `m[32'hFFFFFFFE]` and `m[64'd0 - 64'd2]` (vita E4002 / W4029); a GENVAR index `assign cw[g] = m[g-2]` (iverilog `a5 5a`); an all-`z` driver beside a partial or delayed driver (E3001); a `force`d copy after `release`; an array-WORD-target copy `assign c[0] = v[0]` (iverilog `a5` / verilator `00`); a zero-extending, truncating or concat copy; a partial slice `v[3:0]` (iverilog `x` / verilator `5`); `v[7 -: 8]` (iverilog `0` / verilator `4294967295`, vita = verilator); an `always_comb` whose only read is inside a called task (vita and verilator `a5`, iverilog `xx` with "no sensitivities") | ⓐ a §5.4.1 race kept on the settle's value ON PURPOSE (a store-side forward breaks picorv32, UDP and keccak parity) · ⓕ the interpreter and VM take the extension sign from the slot (255), the native path from the node · ⓖ callee reads are not in the sensitivity derivation | ⓐ and ⓒ are held on purpose; ⓖ's width-changing half shipped in §4.5.521 (`alias::sign_extending_copies` admits a sign-extending driver to the READ alias only, `alias_read_needs_restamp` declines in both compiled lanes), leaving the mixed-caller population and the zero-extending / truncating splits |
| 🆕 J | LOUD | ⓓ `{'1, 1'b0}` is illegal (§11.4.12) and the oracles are lenient (2) where vita is loud; keep · ⓔ `v['1]` is split (iverilog 0, verilator 1) and vita follows iverilog · ⓕ the widths of `'1 * 2'd2`, `'1 + 1'b1` and `4'd8 - '1` are verilator 2/1/4 and iverilog 3/2/5 with values agreeing; vita follows verilator · ⓖ `localparam U = '1; localparam Y = U + 4'd1;` gives `$bits(Y)` 32 against the oracles' 4/5 | ⓖ is fill-INDEPENDENT (`localparam U = 1;` shows the same 32) — it is the row-14 value-inferred tail (`min_signed_bits(v).max(32)`) | ⓐ (a fill as LEFT operand under a TYPED declaration, 11 cells) is row 30 · ⓖ WALL(provenance) |
| 🆕 M | LOUD | ⓐ `m #(.P('1 ^ 1'b0)) u();` onto `parameter logic [39:0] P` is 32 bits in vita (`00ffffffff`), target-sized in iverilog (`ffffffffff`) and ONE bit in verilator (`0000000001`); 40 pre-existing plus 9 split cells · ⓑ `(\|'1)` and `{('1 ^ 1'b0)}` stay loud in a constant · ⓔ `cover property (… (a \|-> b))` is loud (`cover.rs` has its own sequence-only grammar) · ⓕ verilator prints NO failure for `a \|-> b and b \|-> a` where §16.12.8 fails at the first failing operand (vita t=35), so it is not an oracle for property-level `and` | ⓐ the parent folds the override before the target's width is known (`resolve_param_overrides` → `ovr_by_name`) | ⓐ BLOCKED BY: a target-typed override evaluation (row 17's axis) · residue: a select of an ascending or value-sized hierarchical parameter stays loud; `$bits` of a hierarchical string is loud (split 1/16); an override CARRYING past the operands' top bit (`~`, `+`, `<<`, unary minus, `?:`) stays loud; a decimal or `-(64'sd1)` override of an untyped 128-bit-default parameter keeps 128 bits (row 25's i64 half) |
| 🆕 N | OPEN | VCD `$scope` types a generate block `module` where iverilog writes `begin` (a SINGLETON block is spelled `gi` since §4.5.474; a loop iteration keeps `gi[0]` as iverilog does); a task declared in an unnamed block is a split (iverilog `top.genblk1.t`, verilator `top.genblk1.genblk1.t`, vita loud); a user block named `genblk1` beside an implicit one is not disambiguated (iverilog `genblk01`, verilator refuses); `%m` in a CONCURRENT `assert property` action block omits the assertion label (`top.nb` against verilator's `top.nb.ap`; iverilog refuses concurrent assertions, so this is 1-oracle) — the label dies in the parser, since `hdl_ast::Stmt::ConcurrentAssert` has no `label` field and adding one to that frozen SchemaHash type flips the root hash (an IMMEDIATE labelled assert already carries its label, which is verilator's side of a live split); a class method called from a generate-block process or a frame names its INSTANCE (a label inside the method is kept, matching verilator; iverilog drops it, a split); a `$unit` class prints `top.C.show` (split); a package class, and a class method calling a module task or `$strobe` in a class, are loud; a parameterized class prints `C__8` against verilator's `C__N8`; an ELABORATE-time diagnostic inside a class spells `[in $class$C$m]`; a package function is `top.pf` against iverilog `p::pf` and verilator `p.pf` (split); `--hier-tree` and `--inst-paths` list no generate scopes; `--inst-paths` prints a SINGLETON generate scope with its storage index (`top.gi[0].w[1]`) where `%m` prints `top.gi` (PRE-identical on the ported twin); instance-array ELEMENT ORDER is a split vita sits outside — vita elaborates the declared range left to right, so `w[1:0]` runs `w[1]` first where both oracles run `w[0]` first, and on `[0:1]` the two oracles disagree with each other (iverilog `w[1]` first, verilator `w[0]`), PRE-identical on the ANSI-ported twin; only the LEADING path segment of a hierarchical name gets the §27.4/§27.5 spelling hint §4.5.522 added, a deeper offender (`u.gi[0].x`, `u.gl.x`) keeps the generic text and `gi[0].f()` reports the call lane's own text (deliberate — `gen_spelling_hint` is pure and will not name a scope it would have to re-derive the committed walk to find) | the class table is global and its declaring INSTANCE is unknown, so the CALLING scope is prefixed | beside it (loud): an instance array whose child has a NON-EMPTY non-ANSI header is still refused as `child has non-ANSI ports` (§3.b `nonansi-child-array`); the PORTLESS case runs since §4.5.522 |
| 🆕 O | OPEN | the row's stated class — a reader that resolves a bare name through `lookup_net_scoped` without asking `bare_ident_route` — is CLOSED for the eleven unguarded readers §4.5.523 routed through `ident_route.rs` (`packed.rs` ×2, `arrays.rs`, `ports.rs`, `instance_array.rs`, `inline_task.rs` + `array_formal.rs`, `class_lower.rs` ×5, `classes.rs` ×2, `static_array_method.rs`, `dynarr.rs`, `strings.rs`), and the one over-reporting write site with it. What survives: recorded oracle splits, unmoved and never chased — an ENUM LABEL shadowing an outer array reads the array in vita and iverilog and the label in verilator (16 of 16 enum-label twins byte-identical across §4.5.523), and `foreach` over a shadowed name is `i=0` in vita and iverilog and `i=31` in verilator — plus the hand-rolled PARTIAL guards the census marked CLEAN or SPLIT rather than converting: `arrays.rs whole_name_net`, `packed_elem_resid`, `array_geom`'s ascending arm and the `foreach` lane | those four sites walk the scopes themselves instead of calling the funnel | a new instance is the same one-line guard through `ident_route.rs`; the two split axes are recorded, not chased, so nothing here is startable |
| 🆕 Q | BLOCKED | a `localparam` declared in a procedural block is a parse error (`E-PARSE-UNEXPECTED-TOKEN: expected statement, found keyword 'localparam'`, plus a cascade of follow-on "expected statement" errors) where both oracles accept it. The class is wider than a plain named `begin : g`: an UNNAMED block, an `always_comb`, a subroutine body, the `parameter` spelling (§6.20.1 makes it a localparam) and use as a RANGE BOUND of a later block-local declaration (`localparam W = 7; logic [W-1:0] v;` — both oracles `v=127`) are the same refusal, 7 of 7 two-oracle | there is no BLOCK-SCOPED CONSTANT binding in the IR. The parser's `const_locals` is a parse-time i64 fold table read only by `try_const_index`, and the elaborator's `$blk$<span.lo>` scoping is for block-local NETS, which are not constants | BLOCKED BY: a block-scoped constant binding. A bare-name HOIST of the declaration into the enclosing container's item queue makes 6 cells correct and 5 NEW silent-wrongs, because the hoisted name has no scope: an outer literal localparam, an outer non-literal one and an outer HEADER parameter each read the block's value after the block (`out=7`, oracles `3`); two sibling blocks declaring the same name collapse to the LAST value (`a=9 b=9`, oracles `a=7 b=9`); a read AFTER the block answers 7 where both oracles reject the name. Only the outer-NET cell is loud. A parser-side rename is not cheaper — there is no single `ExprKind::Ident` funnel (52 construction sites) · 2-oracle |
| 🆕 L | LOUD | ⓑ `$bits` of a string parameter is 16 in vita and iverilog (§6.16) and 64 in verilator: vita follows the LRM, keep · ⓒ `localparam real Q = 1.5; localparam W = Q * 2;` is E3009 against the oracles' 3.0 · ⓓ a 2-state struct's `'{…}` in a constant is loud (`w'(longint'(e))` has no const-fold arm); a 4-state struct's folds · ⓔ a fill inside a `'{…}` in a constant is loud · ⓕ a string or `real` package parameter through `import p::*` is E3010 / E3009 (the scoped `p::S` works); `$bits` of a WILDCARD-IMPORTED real parameter is 32 against verilator's 64, the same name-lookup family · ⓖ `m #(.X('{1'b0, 5'd7}))` of a struct-typed header parameter is E3009 against verilator's 21 · ⓗ `p::v.a` is E2002 (the struct desugar keys on the bare first segment) · ⓙ `gather_local_decl_names` omits functions, tasks, genvars, instance names, typedef names, array parameters and generate-block contents, and a package importing a package passes an EMPTY set · ⓚ an `import` inside a generate BLOCK is loud E3009 unless redundant; per-scope application is the remaining work · ⓛ `union packed` containing an anonymous `struct packed` fails to parse, and `import` or `localparam` in a function body is loud · ⓠ `logic [1:0] i; F[i*4+3:i*4]` gives `0000` and `F[i*4 +: 0]` gives `0` in silence (verilator refuses; illegal SV) · ⓡ a block-local variable shadowing a wildcard-imported package VARIABLE is read as the local AFTER its block (`SX` → `11`, oracles `a5`) · ⓣ `c #(.A(x), .A(y)) u();` is accepted, last wins · ⓤ an x/z WRITE into a 2-state member of a 4-state packed struct keeps the x/z — `o.q = 4'bx1z0;` is vita `xx1z0x` against iverilog `x0100x` (§7.2.1); the parser knows the member is 2-state (`StructFieldLayout.5`) and the write path does not squash · ⓦ loud residue: a package function whose body reads a package constant outside the i64 interpreter (real, string, array, enum); an INTERFACE importing a package function into a range bound (`apply_import_const_funcs` is wired for modules and packages only; both oracles 8); an `import` inside a generate block (unapplied, loud twice) · ⓧ a compilation-unit `import` after the module still applies, and a package constant used before its declaration folds (iverilog rejects both) · ⓨ the declared-range gate's span dedup reports a generate loop's bad bound once where iverilog reports it three times · ⓩ three separate roots on the negative axis: a negative select BOUND (`A[0:-2]`) false-louds because `const_bound_u32` folds it unsigned (`-2` reads `0xFFFF_FFFE`) and then trips the direction check, with the message naming the wrong fact (both oracles `p=7`); a >64-bit negative-LSB base is still positional (`logic [67:-4] A; A[7:0]` is `34` against both oracles' `33` — `const_wide.rs` reads `param_range` directly and `select_base_at_declared` refuses `dwidth > 64`); an explicit `[m:l]` PART select of a net or variable declared with a negative low bound is loud by its own gate (`packed.rs`; both oracles answer, and the message understates itself, since both INDEXED spellings and every bit-select already work on that net) · loud beside it: runtime `$size(P)` of a scalar parameter, a multi-packed ELEMENT array parameter, a >64-bit base's select (`logic [191:64] A; A[127:64]`, both oracles fold), and a whole-NAME read of a shifted or ascending parameter in a >64-bit concatenation (`logic [11:4] A; logic [79:0] L = {A, 72'h0}`; the zero-LSB twin folds, and `narrow_param_bits` declines the NAME, which is what keeps the structural select arm sound) · (aa) 324-cell residue: an UNTYPED `localparam G = C + D` stays i64 (iverilog 16 / verilator 0, split); enum labels are the same split (vita = verilator); `$clog2(C+D)` in an untyped declaration folds the 32-bit sum where the oracles fold the 4-bit 0; a `byte` operand in a bound (`[Y+Y:0]`, `Y = 100`) reads the LRM and iverilog value 57 where verilator reads 201 | ⓩ three roots: the unsigned select-bound fold, the wide lane's own `param_range` read, and the net lane's part-select gate | ⓩ each residue is its own slice, and demand is ZERO (no negative packed declaration in the 1,462 corpus RTL files), so they rank below any cell the corpus exercises · (aa) the elaborate VALUE lane is row 14's wall · also recorded: the `endpackage` export of `packed_md_params` has no `local_decl_names` filter, and `foreach` over a multi-dim packed formal is loud with a misleading enum-method diagnostic |
| 31 | PERF | all 126 pure-family cells are three-way correct in all nine positions; `assign p1 = $signed(a)*$signed(b)` measures 603 evaluations against 201 for `a*b` (3.0×), and `a >> $clog2(8)` the same; demand is 22 continuous assigns / 29 occurrences in ibex and verilog-ethernet | — | do not start as a §2 item; it is filed at §5.2 rank 4 · the STATE half is do-not-chase: of 32 wrong cells only 6 are arbitrable (26 have both oracles constant but disagreeing, iverilog `z` against verilator `0`; 4 split on constancy) and declining makes the one reachable hazard WORSE (2,406,546 settle spins) |
| 32 | ORACLE-SPLIT | re-measured 2026-09-21 and NOT changed. `$fatal` inside a function: all three tools print the caller's earlier line and then stop — vita ``fatal[VITA-F4004] F-RUN-FATAL: boom [in top.f] [at time 0]`` rc=1, iverilog ``FATAL: r32.sv:2: boom / Time: 0 Scope: top.f``, verilator ``[0] %Fatal: … Assertion failed in top.f: boom / %Error: Verilog $stop / Aborting...``; the residue is one extra `$display` line when ANOTHER process has a statement at the same time, and only iverilog suppresses it. `$finish` inside a function is a straight split: iverilog stops, verilator runs on, vita is loud by design | `frame_eval.rs::run_frame_call_with`, one funnel | not startable: the two oracles answer the boundary differently and neither can be followed without contradicting the other on the sibling spelling · vita's own TASK path already bails without committing the caller's lvalue and matches iverilog; verilator is disqualified because it prints after a TOP-LEVEL `$finish` too |
| 34 | DO-NOT-START | rows 23 and 24 (clocking): 36 silent-wrong cells and startable, still excluded — 1-oracle only (iverilog 13 cannot parse `clocking`) and demand is zero twice over (no corpus row; all 16 `endclocking` files sit inside interfaces, which vita refuses through an unrelated gate) | — | worse than the rows record: the signal is destroyed PERMANENTLY, and with a 2-state declaration it clobbers to `0,0,0,…`, a plausible value with no `x` anywhere |

### Size cast / signedness

- The width probe emits its diagnostics twice (values are correct): the lowering used to decide
  narrowing also runs diagnostics and side-table registration — `8'(a >> pk::nope)` gives E3009 ×2,
  `2'({u1.nope} % 4)` gives E3010 ×2 — and the dead node is serialized, so a depth-32 nesting is 5.7×
  the `.velab` (5884 B against 1035 B). WALL(AST self-width).
- `w = ir_bits_of(plain)` is the NODE's width, so it cannot see the whole cast operand's self width:
  `2'((s8>>u3)*s16)` is vita `11` against both oracles' `01`, 11 cells.
- When the width is unknown, no default is right: `2'(u1.mem[0] % 4)` and `2'(u1.k[7:0] % 4)` are
  `xx` against iverilog's `11`, and `2'(s % 4)` with `string s="A"` is `xx` against hand-IEEE `01`.
  `ir_bits_of` answers `None` for a deferred hierarchical read, a `string` net, and the `SysFunc`
  family that produces a string. `unwrap_or(u32::MAX)` is worse (`4'('0 / {u1.k})` `0000` → `xxxx`,
  `.velab` 22.4×, RSS 10 MB → 1.1 GB, 41 cells regressed). WALL(AST self-width).
- A `typedef enum bit [7:0]` stores 4-STATE (1-oracle, iverilog): an uninitialised variable of that
  type reads `u=xxxxxxxx` where iverilog reads `u=00000000` (verilator prints `0` for every 4-state
  type too, so it is not an oracle here). The enum base's 2-state kind never reaches the net's
  storage kind. This is the PREREQUISITE for §3 ⑤ⓕ's enum-base container: until it is fixed, the
  2-state axis of a type-parameter override on an enum base cannot be certified. The ASSIGNMENT
  twin has two oracles: `e_t v; v = 8'b1x00_0111;` reads `00X7` in vita on the module variable, the
  frame local and the inline body-local alike, where iverilog and verilator both read `0087`
  (§4.5.526 R1; the inline body-local's new 2-state step leaves an enum-typed local 4-state too).
- A 4-state narrowing drops x: with `a=8'bxxxx_0011`, `2'(a+1)`, `2'(a*2)`, `2'(-a)` and `2'(a-1)`
  are all known where iverilog answers `xx` (`<<` and `&` are closed even in 4-state; 4,116 cells,
  0 divergent).
- A size cast over a FUNCTION-CALL leaf evaluates at self width (2-oracle): `ast_ctx_signed` answers
  `None` for a call, so `64'(f(1) - 40)` is `00000000ffffffd0` against the oracles'
  `ffffffffffffffd0`, 16 of 720 cells. Fix = give `expr_self_signed`'s `_ => false` (21 callers) the
  declared return type. Residue: a dynamic, queue or associative element's sign is invisible to the
  classifier; a HIERARCHICAL or class-member operand keeps the older classifier; a `time` constant
  from a guessed parameter declines.
- A real inside a size cast is silent when it meets a fill (oracles agree): if the other side is a
  fill, the real source is not a plain real net (`parameter real`, a real literal, a real return,
  `$signed(r)`, `$realtime`, `$sqrt(r)`) and the operator does not propagate real (`&`, `|`, `^`,
  `<<`, `>>`, `>>>`, `%`), the expression never enters the funnel — 84 of 288 cells
  (`4'(RP ^ '0)`, `4'($sqrt(r) & '1)` at exit 0; iverilog rejects all of them). Two locks
  (`ast_ctx_signed` is `None`; `expr_is_real`'s `Binary` arm has no bitwise, shift or `%` case), so
  this is a CLASS. WALL(AST self-width).
- `$signed(real)` and `$unsigned(real)` are position-dependent: 15 positions inside a cast are
  refused and 7 exit 0 (`$signed(r)*2` → 15, `%0d`/`%0f`, int and real assignment); iverilog refuses
  all of them. Beside it, two-argument `$signed(r, u)` is accepted silently.
- A prim cast does not push the target width down to a context-determined operand (oracles agree):
  with `a=8'hFF`, `int'(a*a)` is `00000001` against iverilog's `0000fe01`, and `shortint'(a*a)` is
  `00000001` against `fffffe01`. `lower_prim_cast` uses `lower_ctx_or_plain` (fill only); wiring it
  directly makes `refuse_real_size_operand` turn `int'(r)` loud. WALL(AST self-width). Census on
  §4.5.530's cells c17 / c18 (`u4 = 4'b1010`, `u8 = 8'hf0`, both oracles agree, PRE = POST): `int'(-u4)`
  is `00000006` against `fffffff6`, `int'(~u4)` `00000005` against `fffffff5`, `int'(u4 - 4'd11)`
  `0000000f` against `ffffffff`, `int'(u8 << 4)` `00000000` against `00000f00`, `int'(u4*u4)`
  `00000004` against `00000064`, `longint'(u8*u8*u8*u8*u8)` `0000000000000000` against
  `000000b964f00000`, `shortint'(u4+u4+u4+u4+u4)` `0002` against `0032`, `integer'(-u4)` `00000006`
  against `fffffff6`, `int'({u4,u4} + 8'd255)` `000000a9` against `000001a9`, and
  `int'(u4 ? u4 + 4'd8 : 4'd0)` `00000002` against `00000012` — 13 of 19 lines wrong. The size-cast
  twins `16'(-u4)` `fff6`, `16'(u4*u4)` `0064`, `16'(u8 << 4)` `0f00` and the signed `int'(-s8)`,
  `int'(s8*s8)`, `int'(-s4)` are right.
- A cast's context width stops at an inner self-determined node (both oracles agree):
  `64'(-16'(u16))` is `000000000000fffb` against `fffffffffffffffb`, `8'(s4 * 4'(s8))` is `…f9`
  against `…09`, `16'(s8 + 4'(u8))` is `000c` against `010c`; un-nested `64'(-u16)` is correct, so
  the trigger is a nested cast or a `$signed`/`$unsigned` node. 143 of 10,368 cells.
- A signed non-repeatable operand whose width `ir_bits_of` does not know keeps the mirror sign and
  the two-mention `extend_to` (PREREQUISITE row). §4.5.530's single-mention shapes (`TwoState`, the
  `extend_signed_once` ternary) carry the operand's own width, so over a FABRICATED width they drop
  the width the cast asserts (`$bits(int'(q.sum()))` went 32 → E3009 in its first cut); those
  operands are excluded and keep the PRE shape. Cells: `40'(u1.f(0))` (a hierarchical call
  placeholder) is `00000000000000fb` against both oracles' `fffffffffffffffb`, and `16'(u1.f(0)) * 2`
  is `xxxxxxxxxxxxxxxx` against iverilog's `fffffffffffffff6`; `longint'(q.sum())` is `00…fd` against
  verilator's `ff…fd` and `16'(q8.sum())` / `shortint'(q8.sum())` are `xxfd` / `00fd` against
  verilator's `fffd` (1 oracle: iverilog rejects `sum()` on a queue); `longint'("ABCDEFGH")` is
  `0000000045464748` against the LRM's `4142434445464748` (no oracle: iverilog refuses the cast of a
  string, verilator aborts). The §4.5.528 hierarchical twin is the same class
  (`release_hier_call_for_widening_cast` withdraws the recorded shape under a widening cast:
  `16'(u.hs(3))` `xxfd`, `40'(u.hs(3))` `0000000000fd` where both oracles print `fffd`,
  `fffffffffffd`). Prerequisite = a declared width for array-reduction, string and placeholder cast
  operands.
- A size cast whose operand is an OPERATOR over a signed call leaf evaluates the call twice (both
  oracles once; values right; PRE = POST): `40'(sf(1) + 8'sd0)`, `20'(sf(3) + 8'sd0)`,
  `64'(sf(4) + 8'sd0)`, `40'(sf(5) + 16'sd0)`, `40'(sf(6) - 16'sd1)`, `40'(-sf(7))` and
  `40'(sf(8) * 16'sd2)` print the `sf` line twice. `lower_size_leaf` (`expr_size_ctx.rs`) calls
  `extend_to` with no repeatability gate; a `$random` leaf is not affected (`40'($random + 8'sd0)`
  and the next draw match iverilog). Fix = `extend_signed_once` there, behind the same declared-width
  test the cast arms use.
- Spellings where a cast cannot claim an element's sign: `unpacked_elem_signed` claims it only when
  the base is a single-segment ident, so `40'(x[0]*1)` is vita `00000000fd` against iverilog's
  `fffffffffd` for a multi-dimensional `g[i][j]`, `pk::pm[0]`, a frame-local array, a dynamic or
  queue element, and an interface-array element. The package spelling is the urgent one — `arrays.rs`
  already has a `pkg::arr[i]` arm, so the classifier disagrees with its own lowering resolver and one
  design answers `pm[0]` correctly and `pk::pm[0]` wrongly. Beside it, `16'(u1.sarr[0])` is
  `000000000000fff9` against iverilog's `fffffffffffffff9`.
- A FILL override (`'1` / `'0`, through `#()` or `-G`) binds at the default's width where both
  oracles bind ONE bit: `#(.P('1))` onto `parameter P = 5` is `-1 bits=32` in vita and `1 bits=1` in
  both oracles (§6.20.2). Fix = a fill onto an UNTYPED parameter binds `(1, false)`.
- A fill override folds at 32 bits instead of the target width in four shapes, all of them where
  `param_decl_width` is `None` (oracles agree): ⓐ >64 bits (`parameter [127:0] K` with `'1` gives
  `0000…ffffffffffffffff` against iverilog's 128 ones — a hole in the "a wide parameter OVERRIDE is
  loud" invariant); ⓑ `time` (`#(.T('1))` gives 4294967295 against 18446744073709551615); ⓒ untyped
  (§12.2.2 — `#(.K(64'hDEADBEEF))` gives −559038737 against 3735928559); ⓓ `real` (`#(.R('1))` and
  `-G R='1` give `4294967295.0` against `1.0`). One root, one slice; do not break the current
  agreement of the three channels (`#()`, `defparam`, `-G`).
- A `time` parameter with a DECIMAL default forwards as 32-bit unsigned: `parameter time T = 1 << 40`
  has no `param_meta`, so `#(.P(T))` types it `(32, unsigned)` through `const_self_width`'s
  `map_or(32)` and truncates 2^40 to 0 where the oracles bind 64 bits. Fix = a typed (`time`,
  `integer`, `int`) declaration records its type as meta even for a non-literal default.

### Constant domain (i64)

- The i64 constant domain declines on overflow where the language wraps at the context width (CLASS,
  both oracles agree): `3037000500 * 3037000500` is 145474192 in both oracles and loud in vita;
  `64'h7FFF… + 64'd1` is 0 against loud; `3 ** 40` is 689956897 against loud. Do not fold modularly
  without a context width — mod 2^64 is right only in a ≤64-bit context, and
  `localparam [127:0] P = 3 ** 41` already zero-extends an already-truncated value. Prerequisite = a
  width-aware module-scope fold; the vita runtime is exact throughout.
- A module-scope `localparam`'s `/`, `%` and `>>>` lose the sign even with a declared width:
  `% 64'd10` gives 18446744073709551615 (iverilog 5), `/ 64'd10` gives 0 (iverilog
  1844674407370955161), `>>> 4` gives 18446744073709551615 (iverilog 1152921504606846975); `>>` is
  exact. This is one item with row 14 — do not start it separately.
- Above 64 bits the decline is deliberate (only `w == 64` is unsigned): the two directions are wrong
  in opposite ways — `(64'hFFFF…FFFF + 65'd1) > 64'hFFFF…FFFF` wants the signed reading and
  `((65'd1-65'd2) > 65'd0)` wants the unsigned one (each oracle answers 1), so do not guess. Pin =
  `const_unsigned_at_sixty_four.rs::above_sixty_four_bits_keeps_the_pre_slice_answer`.
- A 64-bit `*` or `+` in an UNTYPED parameter folds at its self-determined width and WRAPS
  (`64'h8000…0000 * 64'd2` is 0 at 64 bits, `64'hFFFF…FFFF + 64'd1` is 0; verilator identical, since
  §4.5.476). iverilog is not an oracle here: it prints 2^64 for both, sizing a parameter-bound `*` at
  the doubled width and `+` at max+1 (its recorded self-contradiction). The "both oracles 0" this row
  used to claim was never measured on iverilog.
- An untyped `localparam` with a huge `**` hangs iverilog, so there is no oracle:
  `localparam L = 3 ** (64'd0 - 64'd8);` runs 10 minutes at 100% CPU, leaving verilator as sole judge.
  The override `#(.P(-128'sd1 ** 128'd3))` hangs it the same way (over 2 minutes, "expanded beyond …
  8323074 bits").
- Placement and cast fold residue (honest-loud): a concat containing a carry operation
  (`{4'd2,(4'd1+4'd1)}`, iverilog 34); x/z inside a concat; a prim or signing cast (`int'(7)`,
  iverilog 7); a replication count taken from a local variable. Do not widen the carry-free folder —
  route to the interpreter's own width-aware walk.
- A const-domain cell whose SIZE wraps declines (loud E3009 against iverilog's 1): `const_eval_cast`'s
  truncating fold is unsound on top of an unlimited operand fold (`4'((4'd8+4'd8)/4'd3)` is SV 0
  against a truncated 5). Beside it, the body width of a real-returning constant function
  (`f = 4'd15+4'd1` gives 16.0 where the self-determined 0.0 is right). WALL(AST self-width).
- A decl-init call chain hits the depth cap of 64 (correct→loud): a 70-deep constant-function chain
  is 71 in iverilog and loud in vita. The unimplemented alternative charges a level only when
  re-entering a function that is already running.
- A 4-state local's uninitialised default is 0: `integer x; g = x + 1;` is vita 1 against iverilog's
  `x` (the 2-state `int x;` is correct).
- A packed dimension product above u32 panics with no diagnostic: `bit [65535:0][65535:0] tt;` gives
  `attempt to multiply with overflow` at the net-allocation site.
- There are three declared-width models and only one sees packed dimensions: `const_decl_wsign`
  (product), `const_bound.rs::decl_is_wide` (first dimension only), `ast_kind_range_width`. Sound
  today, silently broken the moment a dimension rule that SHRINKS a width appears.
- The parameter declaration fold exists in four copies (oracles agree), and the three outside the
  canonical `params.rs::bind_one_param` each omit something different: `instance.rs` (no override),
  `generate.rs`, `package.rs`. generate and package do not fold a fill default at the declared width
  (`parameter [63:0] Q = '1` gives `00000000ffffffff`), and `package.rs` records no `param_range` (a
  part-select of `parameter [15:8] P` is `x`) and routes neither `string` nor `real`. CLASS.

### Index sealing

- Queue and dynamic-array indices have no seal, for constants or nets (oracles agree): on a
  256-entry `int q[$]`, `q[-8'sd1]` and `q[s8]` (−1) read element 255 with no diagnostic where
  iverilog gives the default `0`, and `int d[]` behaves the same. The write side is loud (W4020), so
  read and write are asymmetric, and `dynarr.rs` never calls `seal_index_unsigned`. verilator is not
  an oracle here — it masks at power-of-two sizes.
- A function-call index reaches no seal (oracles agree): `arr[fneg(0)]` with `-8'sd1` silently reads
  element 255 against iverilog's `xx`, because the seal rejects a `Call` as not repeatable. The
  hierarchical call is the same: `mg[u.hs(1)]` on `logic [7:0] mg [-3:2]` is E4002 plus `xx` (and
  `xx` through an inline function) where both oracles read `9f`; the signed hierarchical NET index
  `mg[u.k]` seals since §4.5.528.
- A packed LVALUE write through an index that draws names the index twice, no cast involved:
  `gp[0][$urandom] = 1'b1;` draws two `$urandom` values where iverilog draws one (PRE = POST), so
  every later draw is shifted. `array_word_index_domain.rs` pins vita's two-draw `NEXT 113532184`
  as this residue (vita's `$urandom` stream is not iverilog's, so the pin is a draw-count pin).
- ORACLE-SPLIT, do not chase: on a packed ELEMENT's `+:` overhang iverilog contradicts itself — in
  one design `pv[-2'sd1 +: 2]` is `1x` and `pm[1][-2'sd1 +: 2]`, holding the same bits, is `10`.
  verilator has no `x` for an out-of-range select at all (everything is `01`). vita is uniform `1x`
  across all four spellings and agrees with iverilog on the two spellings where iverilog agrees with
  itself. Pinned by self-consistency (`packed_select_signed_index.rs`).
- A ≤64-bit OPERATOR-topped override over a wide SELF-determined sub-node binds at the default
  literal's 32 bits (17 cells, PRE route kept on purpose by §4.5.527): `8'hFF + (128'd1 > 128'd0)`
  is `32 00000100` against verilator `8 00` (iverilog `9 100`); `~(128'd1 != 128'd0)` is
  `32 fffffffe` against both `1 0`; `8'd1 << 128'd2` is `32 00000004` against both `8 04`;
  `(8'hFF + 8'd1) + 16'd0 + (128'd1 > 128'd0)` is `32 00000101` against verilator `16 0101`;
  `(-8'sd8) >>> 128'sd1` is `32 fffffffc` against both `8 fc`; `32'd1 << 128'd70` is E3009
  against both `32 00000000`. Folding these through the wide walk at their self width imported
  §2 🆕 R (three typed `logic [15:0]` cells went PRE-right → wrong) and moved typed ≤64 cells across
  row 16's split. Prerequisite: §2 🆕 R.
- A >64-bit MIXED-sign operator tree declines to the PRE route (32 bits or E3009), by §4.5.527's
  plain-tree rule: `S8 + 128'd0` (`S8` signed [7:0] −3) is `32 fffffffd` against verilator
  `128 0…0fd`; `-128'sd1 + 128'd0` is `32 ffffffff` against verilator `128 ff…ff`;
  `~W + S8` is E3009 against verilator `128 ff…f000…0c`; `S96 * 128'd1` is `32 fffffffb` against verilator `128 00000000ff…fb`;
  `-W8 + 128'sd0` is `32 ffffff35` against verilator `128 ff…f35` (iverilog one bit wider on `+`, its
  §4.5.466 self-contradiction). The common spelling `W + 1` over a wide untyped `W` is one of them.
  Prerequisite: §2 🆕 R (sign push-down).
- An operator INSIDE a self-determined position of a >64-bit override declines to the PRE route:
  `$signed(~8'd1 + 128'd0) + 128'sd0` and `{~8'd1 + 120'd0} + 128'd0` are E3009 against verilator
  `128 ff…fe` / `128 00ff…fe`; `~128'd0 >> (8'd2 + 8'd2)` is E3009 against verilator `128 0fff…f`;
  `(~128'd0 == ~128'd0) + 128'd8` is `32 00000009` against verilator `128 …09`;
  `W[(8'd200 + 8'd100) % 128] + 128'd0` is `32 0` against verilator `128 0`. Only a plain leaf
  (literal, name, package name) in a position folds. Prerequisite: §2 🆕 R (two-pass positions).
- A >64-bit operator tree that the wide fold itself DECLINES still binds 32 bits silently through the
  i64 route (both oracles 128): `1 ? ~128'd0 : 128'd1 / 128'd0` and `… % 128'd0` are `32 ffffffff`
  against `128 ff…ff`; `~128'd0 & (1 ? 128'd7 : 128'd1 / 128'd0)` is `32 00000007` against
  `128 …07`; `~128'd0 + int'(2.5)` is `32 00000002` against verilator `128 …02`. Root: the divisor
  path returns `None` for a zero divisor in the UNCHOSEN ternary arm (`const_wide_num.rs`), and a
  real cast leaf has no wide arm; the i64 route then answers with the default's meta. Fix: evaluate
  only the chosen arm, or refuse (E3009) when the wide fold declines a tree whose self width is
  past 64.
- A fill inside a >64-bit operator tree stays on the i64 route: `~128'd0 | '1` is `32 ffffffff`
  against both `128 ff…ff`, `128'd0 + '1` the same (iverilog 129). §4.5.527 excludes a fill by
  design (`ast_contains_fill`); the wide fold's fill arm is row 30's wall.
- `parameter unsigned U = 1` overridden with a SIGNED value (`#(.U(-8'sd91))`) binds `-91` where
  both oracles bind `165`. The width axis is correct; only the sign column is open. Root =
  `ast::ParamDecl.signed` is `false` for both "the `unsigned` keyword" and "no keyword", a direction
  `sg || p.signed` cannot see. Fix = an `is_sign_declared: bool` on `hdl-ast`, which is a SchemaHash
  ROOT field and therefore a format bump. The same field closes the typedef-prefix item below. Since
  §4.5.527 the wide twin `#(.P(-128'sd1))` onto `parameter unsigned P` binds 128 bits (PRE 32) and
  still prints `dec=-1` against both oracles' 340282366920938463463374607431768211455.
- Observation only: two producers write `p.signed` from something that is NOT the parameter's own
  keyword — `hdl-parser/src/params.rs:313` (`signed = expl0.unwrap_or(info.signed)`, a typedef
  prefix) and `module_items.rs:740` (`signed: d.signed`, the NetVarDecl-shaped header entry with
  `ty: ParamType::Implicit`). The reachable typedef spellings are harmless in measurement (all four
  tools agree). The `is_sign_declared` field above closes this too.

### Inline / frame binds

- A hierarchical leaf inside a §11.6.1 region still DECLINES (stays x-loud) where the child's
  parameter environment cannot be built or its range does not fold (§4.5.509 folds `[W-1:0]` from
  the child's defaults, the instance's `#()` overrides and its localparams): a TYPED parameter
  (`parameter [3:0] W = 20` binds 4, both oracles `0000021c`; `integer unsigned`; a 1-bit
  `parameter logic`), a `defparam` anywhere in the design (`defparam u.W = 12`, both oracles
  `00003ee0`; no environment is built for any module), an override with no value (`#(.W())`, the
  binder keeps the default), an override the binder binds through its BITS channel
  (`#(.W(~8'hF0))` binds 15), a `**` or negative `/` in a range, arithmetic whose operands are ALL
  sized literals narrower than 32 bits (`#(.W(4'd9 + 4'd9))`, `parameter P = 3'd6` then `P + P`:
  iverilog folds 18 / 12, verilator and vita's binder wrap to 2 / 4 — ORACLE-SPLIT, so the fold
  declines rather than pick a side) — and, as before, a generate-scoped instance (`g.gu.hs`), a
  read from inside a generate body, an instance-array element (`ua[1].hs`), an upward reference
  (`t.s8`), a 2-D packed element (`u.pk2 * m8` is `…xxee` for `…11ee`), a `real` child net. The
  typed-parameter cells are the largest residue; the fix is a width channel on the environment
  (the slot's declared range folded in the same environment, then `coerce_int_width`).
- An interface member reached through a module PORT keeps the pre-slice evaluation width (ONE
  oracle: iverilog rejects `module m(ifc p)`): `16'(p.uh * sq8)` over `logic [7:0] uh` prints
  `00000080` where verilator prints `0000bb80`. §4.5.510 sized a member of a BODY instance
  (`ifc w();` then `w.uh`) through the fact table; a port formal is not in the instance map, and a
  modport path (`w.mp.uh`), an interface array and a nested interface instance are loud (E3009 /
  E3010) — not gaps of this row.
- Interface members the §4.5.510 fact table still declines (both oracles agree on each): a member
  shadowed by a same-named block-local inside the interface (`logic [7:0] sh; initial begin logic
  [3:0] sh; …` — the census counts the name twice; `16'(w.sh * sq8)` prints `00000080` for
  `0000bb80`), a range over a BARE imported package constant (`import pk::*; logic [PW-1:0] q`,
  same cell), an interface instance inside a generate block (`g.w.uh` prints `0000xx80`), a
  `parameter type` member, and `virtual ifc v = w; v.uh` (verilator only). Narrow+narrow parameter
  arithmetic in a range (`parameter W = 4'd6; logic [W+W-1:0]`) declines by the ORACLE-SPLIT rule
  although no wrap occurs there (all four tools say 12 bits) — conservative, not wrong.
- `$signed(<string>)` is accepted (vita invention; both oracles refuse the program): `$signed(sv) * q8`
  folds at 32 bits since §4.5.495 (it folded at 8 before). `$unsigned(a, b)` drops its second argument
  silently (both oracles refuse the arity).
- A time literal inside an inline body's region folds at its 64-bit self width (both oracles agree):
  `f = a8*b8 + 3ns;` is `65028` against `4`.
- A bit-vector FORMAL written inside an inline body gets no width context (`P=1` against `fe01`), and
  a `real` block-local declared in a function body is E3010.
- Deliberate: an actual wider than the formal is not truncated — `f(8'hFF)` is `ff` against
  iverilog's `0f`, and `{f(8'h02){1'b1}}` is 0 against `f`. The plain spelling (`f(8'hFF)` into
  `input [3:0]`, a `[7:0]` return; `{g(4'h2){1'b1}}` into `input [1:0]`) measured `0f` / `3` on the
  §4.5.525 and §4.5.526 binaries alike, equal to both oracles — re-measure the row's own shape
  before keeping it.
- There are nine binding sites, not four, and five are open (2-oracle; `f(300.0)` into an
  `input byte` gives 300 where the oracles give 44): ⓐ a frame function with an output formal;
  ⓑ a hierarchical task call (the argument is pre-lowered in `inline_task.rs` without the formal
  width); ⓒ a hierarchical function call; ⓓ a class method or task; ⓔ a class constructor. ⓑ and ⓒ
  are structurally different.
- An out-of-range real converts wrongly to an integer (PREREQUISITE row): `real rv = 1.0e300;
  byte'(rv)` is 0 in both oracles and −1 in vita (the same for ±inf and NaN). The engine's
  `real_to_int_round` saturates at |x| ≥ 2^127 (`r as i128`): with `r = 1.0e40`, `l = r`, `i = r`
  and `b = r` store `ffffffffffffffff ffffffff ff` and the bind `pl(rf(r))` gives `ffffffffffffffff`
  where both oracles give 0, the cast lanes `longint'(r)` / `longint'(rf(r))` give 0, and `int'(r)`
  is `ffffffff` against `00000000`, repeatable operand or not. §4.5.530's single-mention `RealToInt`
  cast path is therefore limited to targets of at most 32 bits (its first cut moved
  `longint'(rf(1e40))` from 0 to `ffffffffffffffff`), and a wider cast of a non-repeatable real
  keeps the multi-mention composition: `longint'(rf())` calls `rf` 24 times (both oracles once) and
  `longint'($random*1.0)` prints `ffffffff06d7cd0d next=47ecdb8f` against iverilog's
  `0000000012153524 next=c0895e81` (PRE-identical; cell n11). Prerequisite = out-of-range
  real→integer conversion in the engine.
- A signed queue element holding x/z bits, bound to a formal, is zero-extended in the inline and the
  frame lanes (both oracles agree; PRE = POST): with `logic signed [7:0] qx8[$] = '{8'b1x001101}`,
  `b16(qx8[0])` prints `008d` where both oracles print `ff8d`, and an `l16` formal prints
  `000000000000000000Xd` against iverilog's `ffffffffffffffffffXd`. The same element read by a cast,
  and a class field holding x bits bound to the same formals, are right.
- A hierarchical placeholder whose declared width folds INEXACTLY keeps the PRE route in the inline
  lane (2-oracle; PREREQUISITE row). §4.5.528 records a placeholder's shape only when
  `expr_size_hier_exact.rs` proves the declared range's fold exact by construction, so a range or a
  `#()` override that holds a unary operator (`-`, `~`), `/` or `%`, a narrow or signed sized
  literal, or an `integer` / `int` typed slot is unrecorded: `parameter int V` read through a `[3:0]`
  return prints `3ff` where both oracles print `f`, and `leaf #(.W(4'd8)) c;` read through a `[2:0]`
  return prints `ff` against `7`. The prerequisite is the size-cast lane's shared fold `env_fold`
  (`expr_size_hier.rs`), which negates a narrow literal in i64 (`-4'd1` gives −1 where the binder
  gives 15): with `child #(.W(-4'd1)) u;` over `logic [W+1:0] x = '1`, `m3 = 20'(u.x) + 0` prints
  `xxxxxxxx` and an inline `[31:0]` return of `u.x` prints `00000001ffff`, both oracles `0001ffff`.
  Two review rounds of blockers on this axis: a record from that fold refused right reads (E3009),
  and a value-aware minus rule turned `-(-4'd1)` from right to wrong and, by voiding the instance's
  parameter environment, its unrelated nets too. Fix = an exact declared-width fold for
  hierarchical placeholders, then widen the record to it.
- A REAL-returning hierarchical call in the inline lane stays raw bits (2-oracle): `fr = u.hr(8'd3)`
  into `[15:0]` prints `4012000000000000` where both oracles print `0005`, and `u.hr(1.25)` in an
  inline position prints `4004000000000000` against `0003`. §4.5.528 records a call's shape for
  bit-vector returns only.
- A multi-dimensional hierarchical select is unrecorded in the inline lane (2-oracle): `sm = u.m2[0][1]`
  prints `beef` where both oracles print `f`, and `u.pm[1]` on `logic [1:0][7:0] pm` prints `f0`
  against `00f0`. §4.5.528 records a select only for one index on a vector or on a one-dimensional
  array, and a part or indexed part of a vector or of such an array's element.
- The declaration walk declines, so the inline lane keeps the leaf's own width (2-oracle): a net
  declared with a negative LSB (`logic [7:-2] n` read through a `[3:0]` return prints `3ff`, both
  oracles `f`), a generate-scoped instance (`g.u.lv`: `f0 f0 000000f1` against `0 00f0 00f1`), an
  instance-array element (`ua[0].lv`), an upward reference (`top.u.lv`), a `defparam`-set width
  (`fff fff 00001000` against `f 0fff 1000`), a `parameter [3:0] W` typed slot, a `bind` instance
  (verilator only), a name declared twice in the child (port + reg, generate-local, block-local:
  `a4=f0` against `0`), and a hierarchical PARAMETER `u.P` (`p4=f0 p3=000000f1` against `0 00f1`).
- A hierarchical stream stored into a target of a DIFFERENT width inside a non-`automatic` function
  is refused with E3009 since §4.5.528 (the message names `automatic` and a same-width target as
  the working spellings), and the LOCAL twin is silently wrong (1 oracle, verilator; iverilog has no
  streaming; hand-IEEE §11.4.14.3 left-justifies a stream into a wider target): the inlined body
  right-justifies, so `function [15:0] fl; fl = {<<4{loc}};` over an 8-bit `loc = 8'b1100_0110`
  prints `0000000001101100` where verilator prints `0110110000000000`, `{>>{loc}}` prints
  `0000000011000110` against `1100011000000000`, and `l1 = {>>{loc}}` / `l2 = {<<{loc, 4'h1}}`
  print `0036 086c` against `3600 86c0`. The
  refusal also fires for an UNCALLED `automatic` function whose body calls such an inline function,
  so a file whose observable cells were right on PRE is refused (`m4 = fi({>>{u.lv}})` `0036` = verilator),
  and a `$display("%h", {>>{u.lv}})` inside an inline body printed `36` on PRE where verilator
  rejects the construct. Fix = §11.4.14.3's left-justify in the inline store, then drop the refusal.

### Real

- The body of a real-returning constant function belongs to §3, not §2:
  `localparam real R = f();` gives `E3009 … not a foldable constant expression` where iverilog gives
  0.000000 — honest-loud.
- `$signed(<real>)` in a function body is accepted silently (`F=4`, exit 0) where iverilog says "The
  argument to $signed must be a vector type" and verilator "Expected integral input to SIGNED" — the
  same family as the position-dependent cast cells above.
- `real unsigned r;` is accepted by vita alone (both oracles reject the declaration, and vita prints
  `r=-8.000000`); `kind_signedness`'s `Real | Realtime => true` arm is not what accepts it.
- `$realtobits` and `$bitstoreal` silently accept a non-64-bit argument (iverilog says "requires a
  64-bit argument"); vita answers with the low 64 bits.
- A real with |x| ≥ 2^127 stored into an integral target wider than 128 bits saturates at
  ±(2^127 − 1) where both oracles store the exact rounded integer: `3e38` into `[129:0]` is
  `…007fffffffffffffffffffffffffffffff` against `…0e1b1e5f90f9450000000000000000000`, and `-3e38`
  saturates to −2^127. Identical on the module store (`s = R`), the frame bind and, since §4.5.526,
  the inline `RealToInt` — one engine limit, `real_to_int_round`'s i128 domain.

### Ranges / bounds / selects

- `$size(da, 1)` with an EXPLICIT dimension argument still answers the element width (`D1=32 H1=31`
  for verilator's `6` / `5`; iverilog rejects the two-argument form) — §4.5.500's dyn arm takes the
  one-argument spelling only, the two-argument one keeps `net_dims_desc`'s constant path.
- A parameter PART-SELECT used as a width bound is silently one bit (oracle: iverilog):
  `localparam logic [31:0] W = 32'hdeadbeef; logic [W[7:0]-1:0] v;` gives `$bits(v)=1` against
  iverilog's 239. The whole parameter is correct, so the part-select does not reach the constant
  bound domain.
- An inner scalar shadowing a const array: the GAP-G shadow check is missing on the first branch (one
  oracle, verilator). Inside a generate, `localparam int ROT = 99;` shadowing
  `localparam int ROT [0:3]` makes `logic [ROT[1]:0] v` give vita `$bits=21` against verilator's 2,
  because `const_array_vals_of_base`'s first branch returns immediately on a `walk_scopes_key` hit
  and skips the second branch's inner-wins check. The module-scope spelling is correct.
- Loud residue where the oracles answer: a >64-bit parameter select (the bits are in
  `wide_param_bits` and not in the i64 `params`); a header parameter whose default is a select of
  another header parameter; a `#(.N(W[7:0]))` override; `defparam`; a struct member width (a parser
  gap); a class property.
- An OVERRIDABLE 1-D `parameter type T = logic [8:1]` registers its typedef as `[T$w-1:0]`, so `v[1]`
  reads bit 0 and `$low(v)` is 0 where both oracles read bit 1 and 1 (X3, §4.5.515); the same for a
  `localparam type L = logic [HI:LO]` whose bounds name overridable header parameters (`lo=0 hi=15`
  for the oracles' `1`/`16` under `#(.HI(16),.LO(1))`). A non-overridable one with foldable bounds
  is correct since §4.5.515. Fix shape = the 1-D twin of the §4.5.514 `T$p0a/b` carrier (the
  registered range becomes `[T$p0a:T$p0b]` with `T$w` still the width).
- A self-referential return range overflows the stack (no oracle — iverilog aborts too):
  `function [f():0] f();` — `const_fn_ret_wsign` does not carry call depth. Prescription = one line,
  `depth + 1`.

### Class fields

- An ascending negative bound is clamped only on a class property (one oracle — verilator 4 bits;
  iverilog dies on an assertion): `class C; logic [-3:0] q;` gives W3056 and exit 0 with a wrong
  value (row 3b). The cheaper half: an un-normalised class-field select is broken on `logic [7:1] q`
  (lsb ≠ 0) as well.
- A bit select of a packed dimension with a negative low bound cannot build a coordinate:
  `logic [-3:0][1:0] x; x[-3]` — `dim_coord`'s ascending arm does not build the signed subtraction
  of the correct coordinate `(lo+size-1) - idx` (loud on both builds). The whole value and `$bits`
  are correct.
- ORACLE-SPLIT. A duplicate FIELD in a class body (`class C; int x = 1; int x = 3; … endclass`)
  is accepted and the last declaration wins (`x=3`): iverilog accepts it and prints `x=3` too,
  verilator refuses (`Duplicate declaration of signal: 'x'`). vita matches iverilog; a class body is
  outside the §3.13 declaration walk §4.5.525 added, which judges module / interface / package
  bodies only (§4.5.525 census p26_j).

### Scoping / imports / block-locals

- The DOT path `pk.hf(x)` to a module INSTANCE named like a package resolves to the PACKAGE's `hf`
  (verilator only — iverilog rejects an instance named like a package): `HF=00000200` for
  `0000d900`. Site = `inline_fn.rs`'s `pkg_funcs.contains_key(segments[0])` never looks at the
  separator; `expr_size_ctx.rs::pkg_call_head` copies that test on purpose (§4.5.501), so both must
  learn the separator in one slice.
- A package routine called by its BARE name from ANOTHER package's body evaluates its formal default
  in the CALLING package's scope (both oracles agree): p2's body calls `g1()` (declared in p1, `input
  [15:0] a = x`), and `x` resolves in p2 then the module (`N=eeef`, `778` when p2 declares an `x`)
  against `124`; the scoped `p1::g1()` spelling is right since §4.5.496. The bare key inside another
  package's body carries no package (`rtn_key_pkg` sees the module's import), so
  `with_default_arg_scope` does not push. Fix shape = resolve the callee's declaring package through
  the injected table (the `pk::` key `inject_pkg_callees` binds) before asking `rtn_pkg`.
- A package routine body's read of a name AFTER a block that shadows it falls to the CALLER for the
  whole body (both oracles agree): `begin : bl logic [15:0] x; … end  s = s + x;` gives `SH=f3` against
  `128` — `declared` stands the §4.5.493 hook down per NAME for the whole body, the flatten class above
  supplies the rest; a sibling local of another name is unaffected. Same prerequisite as the flatten
  rows (a binding-resolved scope).
- The constant-function lane folds a package constant read from a package function body at the module
  twin's WIDTH (both oracles agree): `localparam [31:0] K = g();` where `g` returns the package's
  `localparam [15:0] C = 16'h0123` beside a module `localparam [7:0] C = 8'hEE` gives `K=23` against
  `123` — neither value; without the module twin it is 123. Site = `const_fn_pkg` / the const
  interpreter's width lookup.
- The CONSTANT domain inside a package routine body is unhooked (both oracles agree): a body-local
  `logic [C-1:0] t` with a package `C = 12` and a module `C = 4` sizes from the MODULE's `C`
  (`G=f` for `fff`), and is E3009 with no module twin; `lookup_scoped` and its nine `params` twins
  (`const_eval`, `const_array`, `const_wide`, `const_str`, `const_decl_width`, `params`,
  `param_query`, `array_geom`, `$bits`' param half) resolve `params` first and `reserve_frame_func`
  runs before the §4.5.493 push. A body-local enum LABEL in a constant range bound is the same class
  (module twin too).
- A FREE name in an imported package routine body (one the package does not declare) binds to the
  caller's same-named net at exit 0 (vita invention; BOTH oracles refuse the program: `Unable to bind
  wire/reg/memory y in pk.gy`): `Y=ee`. The scoped spelling refuses it at its gate. Making the import
  lane loud is not a step down the ladder.
- The SCOPED spelling `pk::g()` reaches no block-local scope-leak gate, so the nested scope-leak
  shape keeps the flatten's value there (`Z=14`, `Z=1`, `F2=88`, `R=88`, `ff=88` against the oracles'
  7, 7, 51, 51, 49) while the module and import twins of the identical body are LOUD — an internal
  lane split. Gating it with today's name-keyed predicate was measured to be a correct→loud
  regression (§4.5.490 round 2), so the prerequisite is the same as the §3 false-loud row: the gate
  must resolve the BINDING a post-block reference takes.
- The inline-fold lane resolves a body's §11.6.1 assignment context BY NAME over all declarations
  (`inline_fn.rs`, `scope.dims` / `non_bv`), so a same-named declaration in ANY nested block can
  supply the width or turn the widening opt-in off: `V=1` against both oracles' `fe01` with an
  8-bit inner and a 32-bit outer twin. Today it is masked by the gate in the module and import lanes.
  Needs a binding-resolved geometry — three narrowings that keyed on properties of the declaration
  were each measured to create a new defect.
- An inner block-local shadowing a FORMAL, with the formal read after the block, reads the
  block-local: `ff=88` against both oracles' 49.
- A static shadow pair beside a DISJOINT `automatic` span of the same name still takes the old
  flatten (1-oracle: verilator `MOD=0`, vita `MOD=41`; iverilog rejects the lifetime override): a
  nested `int s` / `int s` pair in an `initial` plus `automatic int s` in a separate `always` makes
  `shadow_static_only` false, so the pair keeps the pre-§4.5.468 route and the outer write lands on
  the module net. No E3009 fires because the automatic span never crosses its block. Site =
  `compute_scoped_block_locals`'s per-name exemption; the fix is a per-SPAN exemption that must not
  mix a static and an automatic member inside ONE nesting pair (that mixing was measured to turn
  the loud `outer static / inner automatic` shape into a silent leak).
- The constant domain is not shadow-aware (vita invention, both oracles REFUSE): a `logic [7:0] S8`
  declared inside `generate if (1) begin : g` that shadows a module `parameter signed [7:0] S8 = -8`
  is folded as the PARAMETER by `localparam K = S8 >>> 1` (vita `-4` at exit 0; iverilog "A reference
  to a net or variable is not allowed in a constant expression", verilator "variable isn't const").
  Site = `const_eval.rs` `Ident` → `lookup_scoped` → `walk_scopes(params)`, documented non-shadow-aware
  at `scope.rs`. Making it loud is not a step down the ladder.
- A static frame-local initializer that reads a FORMAL argument still runs per activation (1-oracle):
  `function int f(int k); int c = k; c = c + 1; return c;` on `f(5)`, `f(7)` is `f=6 f=8` in vita where
  iverilog evaluates the initializer once at t0 with the formal's default (`f=1 f=2`); verilator refuses
  the shape (`%Error-UNSUPPORTED: Static variable initializer`). Kept at the pre-§4.5.486 answer on
  purpose: `frame_static_init_t0_safe` declines a formal and, transitively, a local whose own
  initializer is declined, and a frame in which a declined initializer READS a hoisted local declines
  whole, because hoisting only the admitted half was measured to produce a third answer (`f=7 f=9`).
- A static frame-local initializer that reads a MODULE net is evaluated per activation with the LIVE
  value; both oracles evaluate it before time 0. iverilog always reads the net's default (`int n;
  initial n = 9; task t; int c = n;` → `c=0` on every call); verilator's answer follows §4.7 initial
  order (`c=9` on that design, `0` when the writer is the same `initial` that calls: `d3/c2.sv`
  `17100 27100` on BOTH oracles against vita's `17109 27109`). §4.5.486 declines such an initializer
  from the once-only prologue, so it keeps the pre-slice per-activation emission and also does not
  RETAIN across calls (both oracles retain). The retention half is 2-oracle; the value half is an
  oracle race, so the row is measured per shape before it is picked up.
- A PACKAGE task's static local is ONE variable for every importing module in both oracles
  (`int c; c++; $display(c)` called from two modules alternately prints `1 2`); vita gives each
  importing module its own flattened copy (`1 1`). PRE = POST of §4.5.485; root = package routines
  are injected per caller module (`apply_import_routines`) and lowered into the caller's nets.
  The same row for a package FUNCTION on every lane (§4.5.516 census): a static local read before it
  is written, reached from three scopes, prints `S1=1 S2=2 V=10` for both oracles' `S1=1 S2=3 V=13`
  by `import pk::g;`, by a scoped `pk::g()` whose own body holds the local, and (loud today, left on
  the inline fold) by a scoped call whose CALLEE holds it. Fix shape = ONE design-wide frame for a
  static package routine, reserved once and shared by every scope — not a decision at a call site:
  three call-site guards were built in §4.5.516 and each refused a design that printed the value.
  §4.5.517 measured the INTERFACE lane: a scoped `pk::stat()` from several interface instances of
  one parent module instance shares one frame (a sibling carry keyed on the parent's instance path,
  `iface_rtn_scope.rs`), generate copies included, while the interface's IMPORT lane and the
  parent's own scope keep per-scope copies (`L1=1 L2=3 M=10` where both oracles print `M=13`;
  `q1_3par`: one local per parent instance where the oracles keep one design-wide) — the
  design-wide frame is still the fix shape. With an import-lane instance declared between two
  scoped-call instances, which copy a scoped call joins follows the declaration order (§4.5.517
  round 3: `SC10 v=10 IM20 v=1020 SC30 v=40` for the oracles' `10 1030 60`; the other two orders
  give two other answers) — a third symptom of the same row, not a rule to add. §4.5.518 measured the interface IMPORT lane on the
  same axis: a bare `import pk::ds;` reached by two SIBLING interface instances gives each its own
  copy (`P1=10 P2=10` where both oracles print `P2=20`), the same per-scope class.
- A static function that READS its own return variable before assigning it (`function int h(input
  int a); h = h + a; endfunction`, two calls) returns 0 on every lane — module, import and scoped —
  where both oracles print `S1=1 S2=2`: the frame return slot starts at 0 on each call instead of
  the value the previous call left (IEEE §13.4.1, a static return variable persists). 2-oracle;
  §4.5.516 leaves the shape on its inline loud for the scoped lane and records the module and import
  lanes as silent here.
- A CLASS method's same-named sibling block-locals are LOUD, not silent (E3010 on
  `$class$C$m.x` plus an E3009 about writing a net outside the function), where both oracles print
  `A=44 B=55`; `classes.rs` is a separate caller of the reserve.
- Declaring a parameter and a net with the same name is accepted by vita alone (both oracles reject
  it): vita takes `localparam N = 7; logic [3:0] N;` and reads it as the parameter (`r=7`). This is
  a vita invention, so making it loud is not a step down the ladder; the shadow rule's `!params`
  clause is the only observable site, and the current behaviour is pinned
  (`block_local_shadows_param.rs`).
- A part-select WRITE into a queue or associative element vanishes silently (oracle: verilator;
  iverilog rejects the syntax): `q[0][15:8]=8'h0F;` gives verilator `ffff0fff` against vita's
  `ffffffff`; the dynamic (`q[]`) spelling is correct — a write-twin gap.
- A typedef whose dims NAME a constant is re-resolved where it is used: `localparam W = 8; typedef
  logic [W-1:0] t;` plus a generate block declaring its own `localparam W = 4` gives `$bits(t)` 4
  and a 4-bit `t v;` inside that block where both oracles keep 8 (the typedef's own scope); the
  unpacked-dim and 2-D packed spellings the same (§4.5.515 review, PRE-identical). A type
  parameter is immune when its bounds fold (literal dims) or are carried (`T$w`); the typedef
  registry stores the bound EXPRESSION and every consumer folds it in its own scope.
- ORACLE-SPLIT. A `modport` named like an IMPORTED symbol (`import pk::mp;` beside `modport mp`,
  for a routine, a parameter or a typedef) is accepted (`R=44` / `O=44`); iverilog rejects the
  declaration, verilator RUNS it, and the wildcard spelling `import pk::*;` is accepted by both
  (§4.5.525 census p22_a2 / p22_b2 / p22_d / p22_e). The CALL half of the row is closed: `w.mp(…)`
  on an interface INSTANCE is refused, which is where both oracles agree. Refusing the declaration
  would be a false loud on verilator's reading, so the split is recorded and not chased.
- A CALL that resolves to a NESTED named-block LABEL which is also the name of a module routine is
  silent: `function int f` beside `initial begin : outer begin : f … end end` prints `F04 44 1` at
  exit 0 where iverilog says "No function named 'f' found in this context (top.outer)" and
  verilator "Found definition of 'f' as a BEGIN but expected a task/function". A resolution class,
  the sibling of the modport call §4.5.525 closed — the §3.13 declaration walk does not descend
  into a named block, and nobody asks the enclosing-block question at the call site.
- A `parameter type T` declared beside a `wire T` in one module is accepted (`G05 1`): one oracle
  only — verilator "Variable has same name as type parameter: 'T'", iverilog cannot parse the
  declaration at all. §4.5.525's pair matrix leaves the cell on the route it had rather than
  refusing on a single diagnosis.
- Not measured: the PACKAGE lane's `typedef` / `class` / enum-label pairs against a non-routine
  declaration. §4.5.525's package walk collects those kinds and refuses them against a routine
  (measured), but the package guard drops a non-routine pair, so the columns are a matrix hole,
  not a decision.
- The `typedef` × enum-label and `class` × enum-label cells of §4.5.525's pair matrix are `R` on
  VERILATOR's diagnosis alone: iverilog refuses both designs with a parse error on the line
  ("Syntax error in typedef clause.") rather than a name-space judgement, while its control
  compiles. Both tools do reject the file, so the refusal stands; the FOOTING is one oracle.
- A width-0 indexed part-select is accepted silently: `parameter P = 0; t[i +: P] = …` is rejected by
  iverilog and exits 0 in vita (§3 in character).

- A string-KEYED associative array converts an INTEGRAL index to its key WITHOUT the §6.16 funnel, so
  a NUL-bearing key is a different key from the string that spells the same bytes: `int m[string];
  m["ab"]=7; m[24'h610062]=9;` leaves `n=2` (`ab` still 7) where verilator leaves `n=1` (`ab` = 9).
  One oracle — iverilog cannot declare `int m[string]` ("Type names are not valid expressions here").
  The key store is the one integral→string crossing `Value::to_sv_string_bytes` does not serve
  (§4.5.519 census; `git grep to_sv_string_bytes` lists the six that do).
- A function-local named block `begin : u … end` beside an instance `u` of the module: `census_item`
  does not count function-local block labels, so the declaration walk and `hier_resolve` both bind
  `u.lv` to the INSTANCE. iverilog binds the block (§23.8 upward search: `k3 = u.lv + 1` is `0008`,
  `k4 = {u.lv, 4'h0}` is `0070`); verilator contradicts itself (`k3=00f1`, `k4=0000`, neither the
  instance's `0f00` nor the block's `0070`); vita prints `k3=00f1 k4=0f00` (`000000f1` before
  §4.5.528). ORACLE-SPLIT on the verilator side; iverilog + §23.8 is the plan.

### Delays / events

- An in-body `@(*)` over a dynamic-storage handle stays stale (iverilog only; verilator refuses the
  shape): `initial forever begin @(*) n = w.size(); end` prints `0 / 0` for iverilog's `0 / 6` — the
  in-body `Level` waiter compares the handle net's WORD, which never moves, so §4.5.503's dirty mark
  cannot fire it. Fix shape = arm the in-body waiter on the dirty mark for a handle net.
- A runtime continuous-assign delay that evaluates to ZERO lands after the Postponed region (the same
  class as the zero-rise trade below): `int dz = 0; assign #(dz) y = a;` read by a same-time-step `#0`
  chain is iverilog `1 1 1`, verilator `0 0 1`, vita `0 0 0` (the one two-oracle cell agrees; the
  first is a split). Since §4.5.506 the runtime lane routes through `ContAssign.delay = Some(0)`, so
  the fix is the zero-tick lag itself. A runtime-delayed assign drives `x` until its first write lands
  (iverilog drives the t0 rhs for a VARIABLE delay and `x` for a constant one — self-contradiction,
  not arbitrable). A delay variable changed in the SAME step as the rhs is an oracle split (iverilog
  uses the pre-write value, verilator the post-write one). A runtime delay whose expression CALLS a
  subroutine runs on the `vm` backend (tier-3 refuses it, W4030), not natively.
- A resolved (multi-driven whole-net) `wire` keeps a runtime delay DROPPED (`assign #(dv) y = a;
  assign #(dv) y = b;` is zero-delay; both oracles delay) — the constant twin is E3001 today; giving
  the delayed lane a resolved net is one slice for both spellings.
- The Bytecode backend loses a STATIC task's `string` formal (`task ss(input string s); s.len()`
  prints `0` for 8; the automatic task and the function print 8) — found by the it11 flip run,
  pre-existing at the parent; the default `native` backend is right.
- The zero-rise sidecar trade (the fall of `#(ZERO_PARAM, F)` is discarded; `#(0,F)` is correct): the
  root is the engine — `Some(0)` sends the continuous assign into the delayed lane and the zero-tick
  write lands after the Postponed region (both oracles 1, vita 0). Held, because taking it now trades
  one silent-wrong for another; fixing the zero-tick lag opens both.
- ⓐ Rounding is an ORACLE SPLIT (do not chase). The discriminator is "the leaf is not an integer
  multiple of the module precision", and the axis splits by two rules: a REAL leaf keeps its
  fraction to the end (`2.5ns+2.5ns` is 5 in both oracles) and a sub-precision-UNIT leaf rounds at
  the leaf (`1250fs+1250fs` at `1ns/1ps` is 2 ps in both oracles; rounding once would give 3). Only
  the latter is adopted, so the real-leaf cells match verilator.
- ⓒ A negative delay `#(1ns - 5ns)`: both oracles DO NOT FIRE. The STRUCTURAL lane fires immediately
  (`real_delay_ticks` clamps to 0) and that half is open. The PROCEDURAL lane matches both oracles by
  reading the sign in the units domain and routing a negative value down the older path. Finishing
  the structural lane requires a tick representation that can express "does not fire", which u32
  cannot.
- ⓔ ORACLE-SPLIT: in a MULTI-timescale design `global_prec_exp` becomes finer and the `e < 0` case
  never triggers — iverilog rounds at the module's OWN precision and verilator at the design's GLOBAL
  precision. With a single timescale the two coincide and it does not bite.
- A wire driven only by a continuous assign raises a false event at t=0 (2 oracles since
  §4.5.532's review): with `wire b; assign #5 b = a;`, b starts at z and the t=0 settle's z→x wakes
  `always @(b)` where neither oracle has a t=0 event; `assign d = c ^ 1'b0;` behaves the same, and
  a two-hop chain `wire n1 = r + 1; wire n2 = n1 + 1; always @(n2)` prints `N2 at 0 n2=x` before
  the real `n2=4` (both oracles print the real line only). An initial-value domain problem: the
  settle delivers the z→x hop as a change. STARTABLE. The §4.5.532 admission of `always @(K)` opens
  a loud→value column onto this class — a design PRE refused now prints the extra `w=x` line — but
  the same line is printed on PRE by the `@(K or clk)` twin, by `initial #0 r = 2;` and by
  `always @(lv)` with no constant at all (soundness s02 / differential e20 controls).

- A `#0` continuous-assign or gate update (`assign #0 r = u`, `wire #0 r = u`, `buf #0`) is
  delivered only after the WHOLE procedural `#0` cascade of its tick (pre-existing, 2 oracles from
  the first hop): `always @(u) begin $display("h0 r=%0d", r); #0 $display("h1 …"); #0 …; #0 …; end`
  with `u = 7` at 5 prints `h0..h3 r=0` in vita where iverilog reads `r=7` from `h0` and verilator
  from `h1`; `always @(u) begin #0; #0; #0; s = r; end` leaves `s=0` against both oracles' `s=7`.
  Site: `delayed_ca` is keyed by absolute tick and drained by `take_due_delayed_ca` on the advance
  path, which re-enters `now` only once every Inactive round is empty. Fix shape = deliver a
  zero-delay cont-assign write as an Inactive-region event of its tick (between two `#0` hops), as
  a procedural `<= #0` already is. The `always @(r)` on such a net also fires twice at time 0
  (`R 0 r=x` / `R 0 r=0`; the t0 false-event row above).
- A deferred-assertion action block that holds a `$finish` or `$stop` (`assert #0 (0) else
  $finish;`, or an `else begin $display(…); $finish; end`) prints an empty line at maturation and
  the run continues to its next `$finish` (pre-existing; verilator ends the run at the maturation
  time with the line printed; iverilog refuses deferred assertions; hand-IEEE §16.4.3: the action
  block's statements execute in the Observed region). Site: `elaborate/stmt_main.rs` ~722 captures
  every system task of a deferred action as a rendered STRING, so the Observed / Reactive
  `Step::Finish` / `Step::Stop` arms of both kernels are unreachable for a user `$finish` and only
  `$fatal` reaches them. Fix shape = carry the terminating task as a control, not as text.
- `$fatal` ends the run at the statement, so a pending NBA of the `$fatal` step is not applied and
  a process it woke does not run (`c2 <= c2 + 1` pending on the fatal's posedge: vita `c2=1`,
  both oracles `c2=2`; `always @(u) m++` beside `u = 7; $fatal(…)`: vita `m=0`, iverilog `m=1`).
  HELD ON PURPOSE: a fatal is an error, and vita prints nothing after it (§4.5.372); the `final`
  values of an error run are not a correctness surface. `$stop` keeps the same arm (iverilog parks
  in its interactive prompt, verilator aborts rc 1 — no oracle).
- A select of a constant whose index reaches a changing net only through a concatenation, a
  replication, a system-function argument or a hierarchical name still never wakes (pre-existing, =
  PRE, 2 oracles): in-body `@(K[{a,b}])` (both `IB at 1/2/3`, vita `DONE` only), header
  `@(posedge K[$unsigned(i)] or posedge clk)` (both `KE at 1/3/4`, vita `KE at 4`), `K[{1{i}}]`,
  `K[$signed(i)]`, and `@(K[u.x])` / `@(K[g.x])` on a child or generate net (both `IB at 1/2/3`,
  vita `DONE`). `index_provably_live` answers `false` for `SysCall` and for every kind
  `const_fold_children` does not descend (Concat, Replicate, MethodCall, …), and `lookup_dotted_net`
  does not resolve the dotted name when the in-body wait is lowered, so the leaf is unknown and the
  never-wake drop stands. Fix shape = descend those operands and resolve the dotted leaf, so a live
  index takes the `K[i]` refusal (cells p07 p08 dT01 q03 q04).
- Bodies the time-0 lane does not admit keep the header lane's drop of the constant's time-0 run at
  exit 0 although both oracles run it (= PRE): a task enable of an IMPORTED package task
  (`import p::t;`, k08), a recursive automatic task chain (dR54b, `REC at 0` missing), `wait (c)`
  (i10: both oracles `W at 0` for `wait (1)`), and `fork … join_none` (i08, i14: `P at 0` missing).
  Every fork is excluded because a `disable` beside a forked child splits the oracles at time 0
  (dS13 `disable fork` with a `#2` child: iverilog `F at 0`, verilator no time-0 run) and the admitted
  `disable <label>` twin printed a child line neither oracle prints (dT20 `FCH from clk=1 done at
  2`). Fix shape = widen `body_suspend_blocker` one construct at a time, each measured beside its
  `disable` twins.
- The header `Level` waiter runs a process a SECOND time in one step when a sibling term changes
  after the process was triggered and before it ran (pre-existing, every backend, 2 oracles after
  time 0): `always @(go) p = p + 1;` beside `always @(go or p) nq++;` with `go` rising at 1 prints
  `Q at 1 p=1 nq=1` / `Q at 1 p=1 nq=2` in vita where both oracles print `Q at 1 p=1 nq=1` once
  (dS05c; the same at 2). The time-0 lane reaches it because every admitted process is pending at
  once at time 0: two lanes feeding each other (`always @(K or q) p = q + 1;` and `always @(K or p)
  if (p < 3) q = p;`) print an extra `B at 0 p=3 q=2` (dR12; verilator prints it too, iverilog does
  not). Site: the Level waiter's dirty-list re-fire. Fix shape = do not re-queue a process that is
  already pending in the step.
- Two Bytecode-backend divergences, both pre-existing at 38ef535 and found by the batch flip run
  (§4.5.524): a runtime-delay expression that CALLS a subroutine falls back and runs SILENTLY on `vm`
  where the default `native` backend is loud, and a `string` formal receiving a `real` actual on a
  STATIC task prints `STATIC=0` on `vm` against `8`. The `native` answer is the right one in both.

### Diagnostics / artifacts

- The default backend re-lowers a Mul chain's base n times and emits n copies of the diagnostic:
  `r <= m[idx] ** 16;` gives 2 E4002 for interp and native and 8 plus "further suppressed" for
  bytecode. The value is right, but it consumes the 8-report cap and erases later diagnostics.
- `coverpoint_domain`'s Pow arm disagrees with the canonical rule: it folds with `max(lw,rw)` and
  `ls && rs` where the canonical rule is the LHS width and the base's sign. The effect is limited to
  the number of coverage auto-bins.
- `run.json` under `--obs-dir` does not carry `-G`: the run.json for `-G W=9` and for `-G W=100` are
  identical outside the timestamps, so the OBS rail is blind to the one flag that changes the
  design's meaning (see §6 OBS-1).
- A user function called from a CONTINUOUS ASSIGN is evaluated more than once per event: a routine
  with a `%m` side effect prints two lines per instance (`M=top.u.f / M=top.v.f` twice) where both
  oracles print one, and 12 against verilator's 7 and iverilog's 2 on a longer chain. Every VALUE
  agrees (`W=7`, `W=13`) and the module and interface lanes are byte-identical, so the defect is
  the evaluation COUNT, not the result; same root as the re-evaluation bullet under Performance.
- An operator over a user call names the call more than once outside the cast lanes (values right,
  PRE = POST): `p8() >>> 1`, `16'(p8() >>> 1)`, `16'(p8() <<< 1)` and `16'(p8() / 2)` call `p8` twice
  where both oracles call it once, and `p8() ? p8() : p8()` three times against two. A continuous
  assign pair `w1 = 16'(p8()); w2 = int'(p8());` calls it 14 times since §4.5.530 (63 before; both
  oracles 2). Same root as the bullet above and the re-evaluation bullet under Performance.
- `run.json`'s `wprog` decline reasons are relabelled by §4.5.530's shapes, counts unchanged: a
  design with cast and bind coercions over calls moves from `{"call":7,"operator":3,"sysfunc":1}`
  to `{"operator":3,"sign":1,"sysfunc":7}` (asked 17, declined 11 on both binaries) — a coerced call
  now files under its `TwoState` node and one extension ternary under `sign`. Observation only; a
  consumer keyed on the `call` count moves.
- `%h` prints a 1-bit unknown EXPRESSION result as `x` where iverilog prints `X`
  (`$display("%h", ^a)`); the 1-bit NET of the same value is `x` in both, so iverilog is
  inconsistent and IEEE §21.2.1.3 is on vita's side. 17 of 215 designs.

- A member read on a bare name that a constant shadows (`V.x` where a generate `localparam int V`
  shadows a class-handle net) reports the generic `E3010 undeclared hierarchical name `V.x`` instead
  of the shadow sentence the other ten readers now share (§4.5.523). Loud either way, both oracles
  reject; the site is the hierarchical-name lane, which never asks `bare_const_shadows_net`.

### Performance (open, recorded)

- A continuous assign whose RHS contains ANY `Expr::Call` or `Expr::SysFunc` is re-evaluated 6.00×
  per input change instead of 1.00× (300,001 evaluations for 50,000 iterations against 50,001; the
  same call in an `always @*` is 1.00×). Root = `levelize::expr_is_pure_of_nets`
  (`levelize.rs:329`), whose `E::SysFunc{..} | E::Call{..} | E::ArrayItem{..} => false` arm sets
  `dirty_ok=false` and drops the assign into `ca_always`. `$unsigned(src) ^ …` is 1.89×,
  `… ^ 128'($bits(src))` is 4.90×; the inliner trips it (`resize_inline_assign` seals with
  `$signed`/`$unsigned`, `inline_fn.rs:631,655`), so an inlined function measures 1.98× SLOWER
  (0.158 s against 0.080 s). Fix order: (1) a per-`SysFuncId` ALLOW-list, `_`-free exhaustive (~79
  variants, ~22 impure); (2) the dep set for `Expr::Call` — `expr_nets`' Call arm
  (`levelize.rs:161`) walks only the ARGS, so the reject is SOUND today; prize 5.95×; (3) then the
  body cost (2.33× ceiling). Certification moves the DIAGNOSTIC stream (a pure RHS gives
  `errors=5`, the same RHS inside a no-op `$unsigned` gives `errors=9`) and must be adjudicated
  first. There is no `pure` flag on `FuncDef`/`SimIr` (frozen); it is computable out-of-band.
- `coerce_two_state` (per-bit: it names its operand once per target bit, and the engine walks that
  DAG as a tree) survives at three sites since §4.5.530 moved the declared-width cast arms and the
  bind lane's arms (2.5) and (3) to `SysFuncId::TwoState`: the `inline_fn.rs` R2 return coercion
  (no resize in front of it), and the FABRICATED-width Equal / Less / Greater arms of
  `lower_prim_cast` (a string, `q.sum()`, a deferred hierarchical placeholder — `TwoState` there
  drops the width the per-bit `Concat` asserts). Measured: `longint'(q.sum() with (item +
  8'($random & 1)))` over `q = {-3, 1}` prints `2f7bcbfcd6b4bcaa` (PRE `00000000000000aa`; any right
  answer is a sign extension), each high bit the sign of a different evaluation, and
  `longint'(q.sum() with (item + pk(k)))` calls `pk` 128 times against verilator's 2 (iverilog
  rejects `with` here). The fabricated arms wait on the Size cast prerequisite (a declared width for
  those operands); the R2 site is open.
- Coercing at the OPERAND's width instead of the TARGET's takes the repro from 69.6 s to 6.7 s
  (10.4×) and the ping count from 32 to 4 (the hand-written `{28'd0, nb}` control is 2.76 s). Open:
  the residue is the 4 surviving terms plus the frame call, now the LARGER half. Not shipped: a
  per-bit skip fires 0 times on 41 cast cells, and the third caller (`inline_fn.rs:396`) has no
  resize in front of it, so narrowing there would change the value. The reorder's own silent-wrong:
  `ir_bits_of` answers `None` for a deferred hierarchical reference (also a `string` net, the
  string-producing system functions, and the `pop`/array-reduction family) and the caller FABRICATES
  32 — `longint'(u1.w40)` with `logic [39:0] w40` is `0000001234567800` in iverilog and in vita, and
  an unguarded reorder prints `0000000034567800`. Take it only where the width is a DECLARED fact.
- The size-cast sign seal leaves the compiled lane on THREE independent axes, and a user function
  call is one of them. Census (operand sign × contains-user-call × destination wider than the cast;
  `--obs-procs`, 400,000 iterations per cell, `d8`/`d16` destinations, `fs`/`fu` returning
  `int`/`logic [7:0]`):

  | | dest == cast width | dest wider than cast |
  |---|---:|---:|
  | signed, no call — `8'((sv<<4)\|sv)` | 0 | `$signed` 400,000 |
  | signed, call — `8'((fs(uv)<<4)\|fs(uv))` | `$signed` 400,000 | `$signed` 400,000 |
  | unsigned, no call — `8'((uv<<4)\|uv)` | 0 | `$unsigned` 400,000 |
  | unsigned, call — `8'((fu(uv)<<4)\|fu(uv))` | `$unsigned` 400,000 | `$unsigned` 400,000 |

  ⓐ a destination wider than the cast fires on both signs — that is the seal doing its job, stopping
  the context width from leaking through (`expr_cast.rs`); ⓑ a user function call in the operand
  fires even at equal width, on both signs — the column an `$unsigned`-only census cannot see;
  ⓒ `*` (and by construction `/`, `%`, `**`) has no `wprog` compile arm (`16'(a*b)` 400,000 /
  0.153 s against `16'(a+b)` 0 / 0.054 s). One mechanism under all three: `compile_node`'s entry
  gate is `sw.width != w || sw.signed != signed`, and `Expr::Call` has no arm at all, so any program
  containing one declines whole and the seal runs interpreted. The seal is not the cost it looks
  like — it is worth about 10% (`8'((hexdig<<4)|hexdig)` 5.16 s against the same expression uncast,
  4.70 s) while the FRAME CALL is 5× (against 1.03 s with no function at all). Prerequisite for ⓒ:
  the sign gate at `wprog.rs:120` argues from the admitted set ("`Div`/`Mod`/`Mul`/`Pow` are not
  admitted"), so a `Mul` arm must re-argue it. ⓐ and ⓑ stay a census, not a fix, until the decline is
  located: `compile` is the only honest answer to "will `wprog` take this", and run.json's `wprog`
  object (§4.5.513, doc-19 §5.2.1) is where that answer is published per expression.
- Constant-domain width and sign resolution walks the tree three times: `eval_const_env_self` runs
  `const_self_width`, `const_signed_env` and then the evaluation where the i64 walk needs one. A
  `[W-1:0]` bound costs 2 extra `walk_scopes` — 20,000 declarations are 0.079 s without the bound
  and 0.115 s with it (+45%). Values are correct. Prescription ⓐ fuse the width and sign walks
  (half of it) and stop `walk_scopes` returning an owned `String` per lookup (the other half);
  demand is recorded — the external aes_top report (R3-elab, 2026-09-07 → 09-09) measures `elab_s` +12 % (`tb_aes_lat`)
  and +30 % (`tb_aes_thr`) against v0.2.0-49, flat across the 27 commits that followed, on `[W-1:0]` / `$clog2`
  derived ranges; this item needs its own PRE/POST perf slice (interleaved, both orders, release);
  ⓑ memoise the genvar-free sub-expressions of a generate-for bound. Front-end cost is invisible to
  the workload corpus (§5.b `ELAB-PHASE-BLIND`), so this needs a front-end-bound measurement of its
  own.
- A left-leaning `==?` / `!=?` chain is still 2^depth: depth 22 goes from 30 s to 79 s. Values are
  correct.

### Oracle splits (recorded, not chased)

- The ORDER of distinct processes in the time-0 Active region around an all-constant
  `always @(K)` (§4.5.532; IEEE leaves it open): with `initial -> ev;` waking an `initial @(ev)`,
  iverilog prints the `always @(K)` line first and the woken `initial` second, vita the reverse,
  and verilator never wakes the `initial` at all; with `always @(K)`, `always @(K2)`,
  `always @(K or K2)` in that order iverilog prints `A` / `AB` / `B`, vita and verilator
  `A` / `B` / `AB`; with an `always_comb` beside it iverilog runs the `always @(K)` first, vita and
  verilator the `always_comb`. The `@(K or clk)` twins order the same on PRE (the pulse fires after
  the first batch of `initial` statements, the §4.5.529 placement).
- What runs after a `$finish` in its own time step, beyond what both oracles agree on (the
  processes already woken, the pending `#0` and NBA regions, their cascades — §4.5.531 runs all of
  it): iverilog halts every OTHER thread at its first system-task call once a `$finish` is pending
  (`always @(u) begin $display("V"); v = u + 1; end` runs the display only, while the same body
  without the display runs `v = u + 1` — disqualified there by self-contradiction; vita runs the
  body = verilator); verilator drops a woken process whose body holds a timing control (`$display;
  #0 …` prints nothing) and a `#0` fork child (vita runs them to their suspension = iverilog), runs
  the statement AFTER a reached `$finish` and re-enters the body on a second edge of the same step
  (vita and iverilog end the thread), and wakes `always @(v)` once at time 0 on a declaration
  initializer (its counters read +1).
- A genvar's self-determined width: RESOLVED by self-contradiction, not chased. verilator answers
  32 for every `$bits` spelling of a genvar while binding a ONE-BIT override from the same genvar;
  iverilog answers 2 for `$bits(i)` while binding 32. vita follows IEEE 1800 §27.4 and iverilog's
  binding: a genvar is a signed 32-bit integer (§4.5.478).
- Packed dims written BEFORE the name of an UNPACKED-array typedef, in DECLARATION position
  (`typedef logic [3:0] u_t [0:1]; u_t [2:0] v;`): vita and verilator run it (`bits=24 dims=3`),
  iverilog refuses the type (`Packed array base-type u_t is not packed`, IEEE §7.4.1). The PORT
  position is refused by both tools and is loud since §4.5.514.
- The init width of an untyped localparam's integer initializer: `localparam L = 4'd15 + 4'd1` is 16
  in vita and iverilog and 0 in verilator.
- iverilog contradicts itself on 64-bit unsigned `%`: `64'hFFFFFFFFFFFFFFFF % 64'd10` is 5 while
  `(64'd0 - 64'd1) % 64'd10` is 1 — do not use the `(0-1)` spelling as an oracle on that axis.
- A cross-scope t0 decl-init race (legal under §6.8 both ways); a runtime-composed `-0.0` rendering;
  iverilog's self-admitted defects (expression-force "evaluated once" and its family).
- The sign of `$stime`: `16'($stime)` at t=0x8000 is `00008000` in vita and verilator 5.050 and
  `ffff8000` in iverilog 13.0. IEEE 1364-2005 §17.7.2 says "returns an unsigned integer that is a
  32-bit time", and vita is unsigned outside a cast too (`q = $stime` at t=2^31 gives
  `0000000080000000`).
- Mutual recursion across two packages in a constant function (`p::f(4)` ↔ `q::g`): vita 10 =
  hand-IEEE (4+3+2+1+0), verilator 8, iverilog cannot parse it.
- `#(.S("str"))` emits one W3056 before it is applied (the value is right): the parent's numeric
  fold fails first and prints "the override is not a constant; keeping the default", and then the
  string channel applies it.
- `localparam bit [3:0] K = 4'd5; localparam L = K - 20;` is 33-bit `8589934577` in iverilog and
  32-bit `4294967281` in verilator; vita = verilator, identical at module and generate scope.
- A real beside a bit-vector region in an inline function body: `a8*b8 % 7 + r` is iverilog 6 and
  verilator 2; `a8*b8 + (1 ? r : 0.0)` is iverilog 65025 and verilator 1; `(a8*b8) ** r` splits the
  same way. §4.5.491's test designs were respelled onto shapes where both oracles agree.
- Two EXPLICIT imports of one name from two packages (`import pa::g; import pb::g;`): vita binds
  the LAST (`R=42`), verilator the FIRST (`R=41`), iverilog rejects the design.
- A name used before its declaration INSIDE an interface body (IEEE §6.10): vita refuses it with
  the module lane's `E3010 … is used before it is declared` since §4.5.518, which installs the
  `decl_pos` tables in the interface window; iverilog refuses it too (`Unable to bind wire/reg/
  memory 'n' in 'top.i'`), verilator alone runs it (`interface ifc; initial #1 $display(n); int n
  = 7; endinterface` prints `N=7`, and the declaration-initializer chain `int a = b; int b = 1;`
  prints `A=1 B=1`). vita's module lane has always refused the identical text, so the strictness is
  deliberate and the loud is kept; no test-suite design is affected.
- `#(.P(pk::PA + 0))` over a package constant declared `logic [0:35]`: iverilog binds 37 bits and
  verilator 36, while iverilog answers 36 for the BARE name in the same design. vita binds 36 =
  verilator = §11.6.1's `max(36, 32)`; the cell is pinned against verilator alone
  (`pkg_const_layout_override.rs`).

- `%s` and a bare `$display` of an UNSTORED `string'(e)`: iverilog formats the PACKED operand
  (`[a b]`, and `fmt < abc> l=6` for a 32-bit operand) where verilator and vita format the string
  (`[ab]`, `fmt <abc> l=5`). vita follows verilator and §6.16; the STORED cast agrees on all three.
  The packed-ASCII surface (`%s`, `$sformatf`, `$fopen`, `$sscanf`, `str_putc`) is negatively pinned
  on that reading (§4.5.519).
- The three loud spellings of the string cast have at most ONE oracle each, so they stay loud:
  `string'(real)` (iverilog "sorry: This cast operation is not yet supported", verilator renders the
  raw f64 bytes), `string'(x).len()` (both oracles refuse the SYNTAX — vita agrees) and
  `localparam string S = string'(…)` (verilator folds it, iverilog refuses; `const_fn.rs` declines
  `is_string_cast` explicitly).
- `always @*` whose body reads ONLY a constant: vita prints `Y=x` with iverilog, which also warns
  "@* found no sensitivities so it will never trigger"; verilator prints `Y=99`. The mirror cell is
  `@(posedge V)` where an ENUM LABEL shadows a net: vita and iverilog fire, verilator does not. No
  side pinned on either.
- A free-standing unlabelled `begin … end` inside a transparent `generate` region: since §4.5.524 its
  declarations flatten into the region, so a `localparam` inside it collides with the region's and
  vita refuses — iverilog agrees (and warns "Anachronistic use of begin/end to surround generate
  schemes"), verilator treats the block as a §27.6 scope and runs it. vita follows iverilog, which is
  the reading §4.5.264 already pinned for this construct; a NEW refusal of a program verilator accepts.
- The same flatten one namespace wider: a net declared in an UNLABELLED `generate … endgenerate`
  region beside a module `function` of that name (`generate begin wire f; end` + `function int f`)
  is refused by vita since §4.5.525 and by iverilog ("'f' has already been declared in this scope"),
  while verilator scopes the unlabelled block and RUNS it (`F06 44`). vita already enforced the
  flatten on the storage×storage twin before the slice (`add_net` refuses two such regions each
  declaring `wire f`), so this is vita's existing model, not a second one.
- A repeated port name in a NON-ANSI HEADER list (`module dut(a, a); input a;`): vita runs it and
  prints `A=z`, iverilog compiles and runs it too — with a DIFFERENT value on the `T9` cell (`T9 x`
  against vita's `T9 1`) — and verilator refuses (`Duplicate declaration of port: 'a'`). §4.5.525
  pushes a non-ANSI header name into the §3.13 walk only when the body declares no `PortDecl` for
  it, so one `input a;` drops BOTH repeats and the pair never forms; the BODY spelling
  (`module dut(a); input a; input a;`) is both-reject and IS refused.
- A non-finite real into a 64-bit integral: `longint'(R)` with `R = inf` is `ffffffff00000000` while
  the assignment `L = R` (and §4.5.526's `RealToInt`) gives `ffffffffffffffff`; iverilog answers `x`
  for INF, −INF and NaN and verilator `0`, so neither side can be pinned. vita's cast and store
  disagree with each other, which proves a defect but not a direction.
- verilator binds an override at the DEFAULT's width when the override VALUE equals the default:
  `#(.P(128'd1))` onto `parameter P = 1` is 32 bits in verilator while its own `$bits` of the same
  text and its `localparam` twin are 128 (iverilog 128); `1 ? 128'd1 : 8'd2`, `128'd1 ** 8'd3` and
  `128'sd1 + 128'd1` onto a default of equal value do the same. verilator is not an oracle for a cell
  whose override equals the default — give the child a default that differs (§4.5.527 harness).
- A hierarchical PARAMETER in a constant part-select bound, `m = u.w[u.P*2-1:0]` with `w = 16'hbeef`,
  `P = 4`: iverilog rejects it ("A hierarchical reference (`u.P') is not allowed in a constant
  expression"), verilator prints `00ef`, and vita prints `0001` — identical before and after §4.5.528,
  whose select record copies the width vita already computes.
- `$bits(u.r)` of a hierarchical `real` is 1 in iverilog and 64 in verilator and vita; unchanged by
  §4.5.528.
- A constant level term beside a live one when the body can SUSPEND (`always @(K or clk) begin #2
  $display(…); end`; likewise `@(e)`, `wait fork`, an intra-assignment `#` / `@`, `fork … join` /
  `join_any`, a task holding a delay): iverilog runs the body at time 0 (`D at 2`, `D at 7`,
  `D at 11`), verilator does not (`D at 7`, `D at 11`). vita keeps the header lane's drop of the
  constant, = verilator; the all-constant spelling (`always @(K) #2 …`) is refused with the reason
  "the body can suspend at …" (§4.5.529).
- A constant level term beside an EDGE term (`@(K or posedge clk)`, `@(posedge clk or K)`,
  `@(K[0] or posedge clk)`, `@(p::C or posedge clk)`): iverilog makes no time-0 run (`MIX at 1`),
  verilator makes one (`MIX at 0`, `MIX at 1`). vita = iverilog (the constant is dropped).
- An IN-BODY level wait on a constant inside an `always` (`always begin @(K) … end`, `@(K[0])`,
  `@(p::C)`, and `@(K or clk)` / `@(K[3:0] or clk)` in the body): iverilog runs the body once at time
  0 (`AB at 0`), verilator does not. vita = verilator (an in-body constant never wakes).
- A named event beside a constant (`always @(K or e)`, `@(K or e or clk)`, `-> e` at 1 and 3):
  verilator runs the time-0 pass (`E at 0`, `E at 1`, `E at 3`), iverilog does not (`E at 1`,
  `E at 3`) — while iverilog does run `@(K or clk)` at time 0, so adding `or e` removes a run it
  otherwise makes. iverilog contradicts itself on this axis; vita = verilator (dR07b, sR09).
- A constant whose value equals a variable's default: iverilog never runs `@(R0)` or `@(R0 or clk)`
  at time 0 for `localparam real R0 = 0.0`, nor an all-x or all-z constant (`1'bx`, `4'bzzzz`, an
  override `4'bx`), while it runs `int` 0, `bit` 0, string `""` and `4'bx10z` there; verilator runs
  the real at time 0 and cannot compile an x / z event term (`Unsupported tristate construct:
  SENITEM`). vita runs `@(R0 or clk)` at time 0 (= verilator); an x / z `localparam` is refused at
  its declaration, and the override half is §2 row 15 (dR04, dR04b, dS10c).
- An NBA or `#0` write of a live term at time 0 (`initial clk <= 0;` or `initial #0 clk = 0;`
  beside `always @(K or clk)` on an uninitialised `reg clk`): iverilog runs the process twice at
  time 0 (`MIX at 0 clk=x`, `MIX at 0 clk=0`), verilator once (`clk=0`) — it is 2-state and the x→0
  change is invisible to it. vita = iverilog (x2, x3); verilator is not an oracle for an x
  transition.
- A same-step glitch under an IN-BODY level wait (`always begin @(a or b) $display(…); end`, a
  blocking `a = 0; a = 1;` at 2, an NBA `b <= 0; b <= 1;` at 3): iverilog wakes at 1, 2 and 3,
  verilator and vita at 1 only. The header twin `always @(a or b)` wakes at 1, 2 and 3 in iverilog and vita and at
  0 and 1 in verilator (dR11).
- A net-only header level list at time 0: verilator runs every header level `always` at time 0
  (`reg clk; always @(clk)`: `MIX at 0`, `MIX at 1`, `MIX at 2`; `reg clk = 0;` and a net bit select
  `@(n[0])` likewise), iverilog does not (`MIX at 1`, `MIX at 2`). vita = iverilog for a variable; a
  net driven by a gate or a computing continuous assign raises vita's own time-0 event (the §2-N
  t0-event class), and there vita = verilator (dS01 `@(w or g)` with `not (g, a)`, dR64).

## 3. loud → correct-support candidates (all loud = safe, additive)

The workload corpus is 10/10 with zero rejections, so it does not order §3 any more. §3 stands
behind the §2 correctness queue.

### 3.a Numbered open items

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| ③ⓐ | a file-read call in the right operand of `&&` / `\|\|` or in a `?:` arm cannot be hoisted | skippable evaluation (§11.4.7 / §11.4.11) · `hoist/general.rs` guard block | apply `guarded_hoist` plus an fd-state ordering proof | iverilog | — |
| ③ⓑ | a call in a `while` / `for` condition | a hoist reads once, and the condition must read every iteration | rewrite into the loop body (the `lower_shortcircuit_cond` shape) | iverilog | — |
| ③ⓒ | every statement a `$feof` survives into is refused · near EOF `x = $feof(fd)*10 + $fgetc(fd)` is vita 9 against iverilog −1 (mid-file they agree) | `$feof` reads the file position and the hoist moves the mutation ahead of it · each arm has its own ordering convention (`assign_seq` rhs→index, `Case` scrutinee→labels, a task argument list) | prerequisite = an `order_walk`-grade ordering judge | iverilog | — |
| ③ⓓ | reads that cannot be named by an alias (`m.a`, `p::v`, a `Shape::NoHoist` child) plus a fail-closed refusal when the call uses a ref | overlap is judged by root name | judge by net identity | iverilog | — |
| ⑤ | the ibex page is 19→13 = `export "DPI-C"` (12, loud by design) plus `ibex_core:2481` `cs_registers_i.g_pmp_csrs[i_region].x` (1) | the parser folds only a constant index into a segment name; a genvar is known only to elaborate | an index expression in a path segment is an AST shape ⇒ DEEP | verilator | DEEP |
| ⑤ⓐ | multi-packed parameters: `$size` / `$left` / `$dimensions`, `'{…}` as a value, an ARRAY parameter of such a type, and `import p::*; import q::*` where both export `P` | outside `packed_md.rs`'s flat rewrite | one arm per consumer | verilator (iverilog: "packed array parameters are not supported yet") | — |
| ⑤ⓒ | header array parameters: a NESTED (2-D) override pattern, an element >64 bits or a whole-array default override, `defparam`, an interface-header array parameter, `'{default: v}` as an override, and a BODY `localparam` after the header that names the element width | outside `array_param_twin` / `const_array_override_vals` | widen the channel | verilator-value | — |
| ⑤ⓔ | element select: a MULTI-PACKED element (`A[1][0]`, a whole-element read and `$size(A,2)` on `logic [1:0][3:0] A[2]`), an ascending or non-zero-LSB element inside a concat or replication count, `p::S[1].b`, runtime `$size`, a select outside the element, and an element-select override of an UNTYPED child parameter | the element capture declines | widen the domain | verilator-value | — |
| ⑤ⓕ | unpacked-array typedef residue, still loud: a function RETURN type of the typedef (1-oracle; iverilog does not merely refuse it, it SIGABRTs with `Assertion failed: (lwid == ivl_signal_width(lsig))`; it needs an unpacked slot on the FROZEN `hdl_ast::FunctionDef`, i.e. a format bump — the worst evidence-per-cost in the row) · a `string` element (`var_kind` is `None`; the explicit twin is equally loud) · a module-BODY overridable `parameter` of that type (the array gate, same as the explicit twin) · an INTERFACE or program HEADER array parameter (`module_items.rs`'s module-only gate) · `typedef <struct/enum/alias> x_t [dims];`, refused upstream at `typedefs.rs`'s chained-alias gate and unreachable from here (1-oracle) · an override that CHANGES the dim count (DO-NOT-START, see §5.2; both oracles print `bits=256 s1=2 dims=3`) · `T'(…)` (no oracle) · dims on BOTH the typedef and the declarator (`a_t y [0:1]`, a live oracle SPLIT on dimension ORDER — iverilog `$size(y,1)=4 $size(y,2)=2`, verilator `2` / `4`, and iverilog contradicts its own answer for the identical explicit type). Composition order is the trap `$bits` cannot catch: the NAME's dims come first (`localparam a_t P [0:1]` reads `P[0][1]`=2 and `P[1][2]`=6). ARITY is the half that cannot follow an override — declarators are stamped with the default's dim LIST once at parse — so `shape_flags` carries the dim COUNT and a mismatch is loud in both directions (dim-losing ⇒ the group's F4004; dim-ADDING ⇒ E3002 named on `T`, suppressed when `T$w` is equally unknown, because then `T` is simply not overridable here). The SIGN and 2-STATE axes follow an override since §4.5.479 through a `shape_param` carrier on `NetVarDecl` / `AnsiPort` / `PortDecl` / `TfPort`, and since §4.5.483 the SIGN also follows it through a `T'(e)` cast and a packed STRUCT member, which share one appended `CastTarget::SigningParam` carrier and a per-AXIS guard mask. What is still loud: the 2-STATE axis of those same two positions (a cast node has no kind field, so `shape_kind` is unreachable through the funnel — strict by design); an ENUM BASE, whose prerequisite is the §2 `typedef enum bit [7:0]` 4-state-storage silent-wrong plus a `TypedefKind::Enum` slot; a packed UNION member, which is `E2002` at parse long before the shape guard, because a union's layout is the numeric overlay table and `[T$w-1:0]` never folds there; a CLASS PROPERTY, which is DO-NOT-START — it is loud with NO override at all and on the WIDTH axis too, because `register_classes` is a whole-design prescan run before any instance exists, so its prerequisite is per-instance class registration, not a shape slot; and the function RETURN type, also do-not-start. Two more pre-existing louds in the same neighbourhood, both measured: a NESTED packed struct whose inner struct has a `T` member is `E3010` on `o.i.f` while its plain nested twin runs, and `s.f[3:0]`, a sub-select of a param-width member, is `E2002` | each consumer reads `TypeInfo` and has no slot for unpacked dims. The DECLARATION consumers (a variable, a port, a tf-port formal, a type-parameter default) carry the dims through the map; the rest decline on `!info.unpacked.is_empty()` rather than bind the element type | per consumer, each its own slice; the split row is do-not-start | 2-oracle except the function return type (1) and the declarator-dims split (0) | S each |
| ⑤ⓓ | nested struct members: `default: v` with a non-fill non-zero `v` · `o.i.e.name()` · a packed ARRAY member `in_t [1:0] i` · a packed struct inside an UNPACKED record · `u.c.perms.q` · `o.i[1+:2] = …` · a member width given as `1 << 3`, `8'd5`, a forward-referenced localparam, or a header `parameter` (overridable = correct-loud) | outside the source kinds the parser's flat layout table accepts | widen per consumer | 2-oracle (`default: v`: verilator whole / iverilog rejects) | — |
| ⑤ | CU scope: a unit-scope VARIABLE or net · a unit enum label in a class body · a forward reference between unit constants · `$unit::t` · an enum-typed output port driven by `assign` (E3018) | outside the parser's unit-scope clone | per item | 2-oracle (the forward reference is split; vita follows iverilog) | — |
| ⑤ | parser / preprocessor: a multi-dim packed formal that is also an unpacked array (`logic [1:0][3:0] a [2]`) · a based-literal value for a parameter narrower than 32 bits, outside the parse-time table · a non-ANSI `<type> [dims]` port · an atom typedef with dims · a SIGNED typedef element with dims · an unnamed or duplicate `` `define `` formal | the parser's flat rewrite · the `` `define `` argument parser | widen the table and the rewrite | 2-oracle / split (verilator lenient) | — |
| ⑤ | `parameter type` with a struct, enum, union, real, string or class default or override (a multi-dimensional PACKED one is carried since §4.5.514 through `T$p<i>a/b`; a 2-D `T` as a packed struct/union member, an enum base or a function return type stays loud) | not expressible by the `T$w` / `T$s` two-value-parameter desugar | loud by design | 2-oracle | — |
| ⑧ | system functions in a function body are refused — `$random` / `$time` inside `assign m = f()` re-draw on every pass | the seed is not a net, and `levelize::func_read_deps` cannot name it | represent non-net state in the dependency set | 2-oracle (both freeze it) | — |
| ⑧ | the statement after a reached `$finish` executes (the same behaviour as `$fatal`) | `SimState::frame_end_is_loud`'s boundary is the statement | move the boundary to expression level | iverilog stops | — |
| ⑧ | a function with an output formal is routed through `Terminator::Call` and PERFORMS the `$finish` (exit 0), so the same syntax has two answers depending on formal direction | routing splits on formal direction | unify the routing | iverilog rejects the syntax itself | — |
| ⑧ | the residual mismatch for a function that carries a counter forward is one evaluation, not a rule | vita's extra t0 settle pass | prerequisite for an honest certification of this family | 2-oracle | — |
| ⑨ | after `import pk::*;` a bare string or real parameter name is loud (the fold succeeds; only the import binding is missing) | `apply_import_consts` re-binds through `params` (i64) only | give the string and real side maps the same treatment — plumbing, not routing, two call sites. Pins = `string_const_domain.rs`, `real_params.rs` | 2-oracle | — |
| ⑨ | a real condition in `generate if (P::R > 1.0)` is loud | `const_real.rs` has no `PkgScoped` arm | add the arm | 2-oracle | small |
| ⑬ | an array access inside a subroutine body is attributed to the CALL statement | the tier-3 arena only RECORDS and drains at the caller's statement boundary, so it does not know the callee StmtId | a second `cur_stmt` source or a shared `Rc<Cell>`. Trap: adding a publish makes interp report `d.sv:6` and native report no location, so backend agreement was chosen | — | — |
| ⑬ | a terminator condition (`if (mem[i])`), a continuous-assign settle, a t0 arm and a delayed-CA apply drain have no location | they are evaluated after the block's last statement, which clears `cur_stmt` to NO_STMT | no location is better than a wrong line (deliberate) | — | — |
| ⑬ | W4022, W4028, the delta limit, RunRange, W4020 and the W4029/W4007 instance path all report `location: None` | the sid-less diagnostic family has no access-statement key in the engine | `cur_stmt` plus `stmt_diag_meta` (the plumbing exists) · a `SpanResolver` plus a StmtId→span sidecar | — | — |
| ⑭ | the call tree to task granularity is not shipped: an inlined subroutine reports 0 calls and reads as "free" | calls are lowered two ways — a call-seam frame body and an elaborate-time INLINE splice (`inline_task.rs` / `inline_fn.rs`; 14.39 s inlined against 0.35 s framed) | prerequisite = an elaborate-time record of inline site → caller (`Sidecars::func_names` and the declaration `file:line:col` exist on both subroutine objects since §4.5.512; the inline-site record is what is missing) | — | — |
| ⑭ | a reporter wants ~440 cycles/s and measures 20.4 — a 21× scheduler/executor gap | not an observability item | Phase D codegen plus arena — tracked in §5 | — | — |

### 3.b Small residues

**Parser accept**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| non-ansi-md-port | a NON-ANSI port `input T a;` / `input t_t a;` whose typedef has two or more packed dims is E2002 with a position-specific message (the ANSI spelling runs since §4.5.514, 3-way); iverilog runs the non-ANSI form | `PortDecl` has one `range` and no packed list (frozen SchemaHash type) | a packed list on `PortDecl` = format bump | iverilog | S (bump) |
| specify | a `specify … endspecify` block is E2002 (`specparam` is accepted as a module constant) | the parser does not accept it | hoist `specparam` and discard path delays and timing checks. Prerequisite: `hdl_parser::parse` has no warning channel, so discarding silently would turn `$setup` from loud into silent — needs a `ModuleItem` marker plus an elaborate `W3056` | iverilog | — |
| case-inside | `case (x) inside {…}` (§12.5.4) is E2002 | the parser does not accept it | hand-IEEE `==?` plus an internal differential | no oracle | — |
| pkg-scoped-task | a scoped TASK call `pk::t()` is `E2002 E-PARSE-UNEXPECTED-TOKEN: expected '=' or '<=' after lvalue, found '::'`; the scoped FUNCTION spelling runs | the statement position does not take a scoped name as a call target | accept it there | verilator (iverilog rejects it too) | small |
| time-signed-param | `parameter time signed T` and `localparam time signed T` are `E2002 … expected identifier, found keyword 'signed'` (twice each) where both oracles bind `-4`; every other declaration site carries the qualifier since §4.5.492 | the parameter grammar at `hdl-parser/src/params.rs:469`; `elaborate/src/params.rs:302-324` carries a comment whose premise is that refusal | accept the qualifier in the parameter position and hand the same `signed` bit `kind_signedness` now reads | 2-oracle | small |
| based-ws | `64'sh FFFF` is a lexer reject | lexer | accept it | iverilog accepts | minor |
| tf-localparam | `task automatic t; localparam int K = 3;` gives `E2002 expected statement, found keyword 'localparam'` (IEEE §6.20 allows it) | the parser's statement position | accept the declaration | iverilog | small |
| blk-automatic | `begin : A automatic int x = 44; … end` inside a task body is E2002 (`static` in the same position is accepted; both oracles run it) | the parser takes a lifetime keyword on a block-local declaration only at the subroutine's own declaration position | accept the keyword in the block-declaration position | 2-oracle | small |
| tf-decl-lifetime | a declarator-level lifetime at a subroutine body's declaration position (`static int c = 0;` / `automatic int x = 1;` right after `task t;`) is E2002 `expected '=' or '<=' after lvalue, found keyword 'int'` | the parser takes a lifetime keyword only on the subroutine header | accept it and feed `d.lifetime`, whose consumers (`frame_static_init_once`, `AdmitReason`) already read it — today it is always `None` from source, so the per-declarator half of those predicates is unreachable | unmeasured on the oracles (the block-position twin `blk-automatic` is 2-oracle) | small |
| class-tf-port | a class-typed tf-port (`function signed [63:0] fw(input C k); fw = k.sf;`) is `E2002 expected a class typedef type for a tf-port … found identifier 'C'` where both oracles run the design (`fw=fffffffffffffffd g=00c3`) | the tf-port type position does not take a class name (the parser's own message lists the typedef kinds it takes) | accept a class name as a tf-port type | 2-oracle | small |
| R30-1 | a missing package gives 7 lines of E2002 and never names the package | the parser cannot take `IDENT::IDENT` in a tf-port as a type | take it as a type and let elaborate say "unknown package" ⇒ 1 line | — | parser |
| enum-label | `enum bit[3:0] {A=8'hFF}` never reaches `enum_defs`, so `.first` / `.next` / `.name` are all E3010 / E3009, and the skipped out-of-range check silently truncates | `const_lit` folds unsized decimals only | widen `const_lit` or check at elaborate time | iverilog rejects | — |
| md-packed-write | multi-dim packed nested part-select WRITE: an ascending or non-zero-lsb leaf · a genvar-indexed `x[g][m:l]` (over-rejected) · a const out-of-bounds packed index is a silent no-op | the current support is limited to a descending zero-lsb leaf | widen the leaf geometry | — | — |
| misc-parse | a negative-LSB member sub-select · `import` inside a generate · a package's own-function initializer · a SYS-READ hierarchical-element destination · a hierarchical-write sentinel panic that should be loud · `logic[1:0][7:0] PK` · `'{k:v}` | — | hand-IEEE plus an internal differential | no oracle | — |
| edge-event | `@(edge clk)` is `E2002 expected expression, found keyword 'edge'` where both oracles print `MIX at 1` / `MIX at 2` for a clock rising at 1 and falling at 2; `@(edge K or clk)` is the same parse error | the event-expression parser does not take the `edge` keyword (IEEE 1800 §9.4.2: `edge` is `posedge` or `negedge`) | accept it as the union of the two edge terms | 2-oracle | small |

**Constants / parameters**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| override-sysfn-value | an integer system function whose VALUE the constant domain cannot fold is loud as a whole override although its WIDTH is now type-determined (§4.5.487): `#(.P({$bits(x)}))`, `#(.P($bits(x[3])))`, `#(.P($bits(x[7:0])))` and the whole `$size`/`$high`/`$low`/`$left`/`$right`/`$increment` family over an unpacked array are W3056 + E3009 where both oracles bind `32/<value>` (9 cells) | the VALUE half, `override_self_value` → `const_eval_in_scope`, not the width half | give the override's value channel the folds it is missing (a concat, a select of a data object, the dimension queries); the width channel already answers. Sibling of `dimquery-width`, a different funnel | 2-oracle | small |
| override-bits-asc | `#(.P($bits(pk::PA)))` where the package constant is declared ascending (`logic [0:35]`) or with a non-zero LSB (`logic [39:4]`) is W3056 + E3009 where both oracles bind `32/24`; the descending twin and BOTH module-lane spellings are correct, so the gap is package-lane and layout-only (4 cells) | the S1×S2 seam: §4.5.487 opened the bare-call rung and §4.5.488 widened the width/meta channel, but `const_eval_in_scope`'s bit-domain fold of an ascending package constant still declines | one fold arm; `$clog2(pk::PA)` over the same name is already correct because it folds in the i64 lane | 2-oracle | small |
| override-concat-name | `#(.P({pk::PA}))` — a concatenation as a whole override — is W3056 + E3009 where both oracles bind 36 bits; the bare name is correct since §4.5.488 | a concat top is not a constant in the override channel at all (a separate root from `override-bits-asc`) | census the override channel's accept set for a concat before widening it | 2-oracle | small |
| §3.3 | a wide `localparam` part-select fold — `{A[127:64], 64'h0}` | a separate arm that needs an index fold | add the arm | iverilog folds | — |
| real-fold | `+`, `*`, `/`, `-` and `**` on a `localparam` / `parameter real` are all E3009 "not foldable"; `$clog2(real-lit)` has the same root | `const_eval_in_scope` is i64-only | add real f64 arithmetic | iverilog folds | broad |
| xz-fill-param | `localparam logic [W] P = 'x` binds 0 (the x is lost), so `P==0`, `P+1` and `P ==? pat` all diverge | `fill_to_i64` / `fill_literal_const` | x/z in the constant domain | — | broad |
| compound-==? | `==?` fold residue = an unsized x/z pattern · a negative signed LHS · a non-literal RHS · a non-constant parameter override (W3056→error) · a longint MIN fold (package) · two loud-message quality items | the current fold handles sized patterns only | widen it | — | — |
| defparam-iface | `ifc a(); defparam a.D = 255;` gives `W3056 … matched no instance` and keeps the default (iverilog `d=ff`, vita `d=8`) | `defparams` is consumed only in `elaborate_instance`, and `iface_inst.rs` reads only its own `overrides` | merge `defparams.remove(path)` into the canonical binder · re-measured §4.5.527: `defparam i.P = 5` onto `ifc #(parameter P = 1) i()` prints `bits=32 hex=00000001` at exit 0 with only `W3056 defparam target top.i matched no instance`, any value; verilator applies it too | iverilog, verilator | small |
| neg-ascending | `reg [-33:-2]` gives `$bits` 1 in vita against iverilog's 32, plus a loud `W3056`. Descending `[-2:-33]` and mixed `[3:-2]` are correct | `array_geom.rs`'s `allow_neg_lsb` is opt-in | put that combination on the opt-in path | iverilog | — |
| neg-bound-part | a negative-bound net PART select: `q[-3 +: 2]` and `q[-1 -: 2]` are exact and only `[msb:lsb]` is blocked. Writes are asymmetric — `x[-3:-2]=…` is silently exact while `x[-1:0]=…` is loud with an "out of order" diagnostic that names the wrong fact | the bound fold is unsigned | `const_bound_signed` | verilator | — |
| neg-elem-bound | `logic [-3:0] q[$]` gives a W3056 clamp (verilator `q[0][-3]`=1) | the element net takes `elaborate_netvar_decl_inner`'s early-`continue` path and never reaches the declaration side map | make it reach the side map | verilator | — |
| mdrv-partial | the process-multidriver check (§4.5.472) is silent when either writer is a PARTIAL write (`mem[a]`, `s.x`, `w[1]`) — verilator is silent too; xcelium on that shape is UNMEASURED (0 observations) | `multidriver.rs` `stmt_writes_whole_ident` | re-measure the partial cells on an xcelium run; if it rejects, the shape joins `W3060`, not `E3001` | verilator silent · xcelium unknown | small |
| mdrv-hier-actual | `always_comb u.ts(src);` (a hierarchical callee) beside `int src = 7` is E3001 on `src` — the conservative `Unknown` verdict for a callee whose formals are not visible here (both oracles run, `H=7 2`) | Rule A's resolver answers `Unknown` for a multi-segment callee; §4.5.505 downgrades only a resolved single-segment one | resolve the hierarchical callee's formal directions through the instance's module | 2 | small |
| frame-body-write-sites | after §4.5.508 (a frame function whose body writes a module net is routed to the statement executor) the call SITES that cannot carry a `Terminator::Call` are E3009 naming the position where both oracles run them: a continuous assign / `force` rhs (`assign w = fw(src)` — `w=7 acc2=9`; verilator warns MULTIDRIVEN; the hierarchical spelling `assign w = u.fw(src)` the same, `CA w=1 acc2=3`), a call inside another frame FUNCTION body (`f2 = fw(v) + 1`, `F2 r=8 acc2=9`; hierarchical `return u.fw(v) + 1`, `INFN r=4 acc2=5`), a package-scoped call `pk::gw()` (the import spelling `import pk::gw; gw()` runs, `W=8`), a class METHOD body write (E3010 on `$class$C$m.acc2`), the non-framed inline spelling (`function logic [7:0] fw; begin acc = v+1; fw = v; end`, `OLD1 acc=8 r1=7`). The hierarchical call itself runs since §4.5.511 for a DOWNWARD path from a procedural statement; its residues: an ABSOLUTE or outward path (`tb.u.fw(7)` from a sibling, `ABS r=7 acc2=9`), a path through a generate scope or an instance array, a callee with a non-input / unpacked / string / `parameter type` formal or a `real` return, a named or defaulted argument, a formal or return width the instance environment cannot fold (each E3009 "unsupported in this position"), Rule A's conservative walk counting an actual bound to the hierarchical callee as an `always_comb` driver (`logic [7:0] src = 1; always_comb r = u.fw(src);` is E3001 on `src`; both oracles run it; the local twin resolves the callee and does not), and the cross-MODULE pair — the parent's `always_comb r = u.fw(9)` plus the child's own `always_comb loc = fw(src)` — which verilator reports MULTIDRIVEN and vita runs (value-correct in all three tools, `XPAIR r=9 acc2=3 loc=1`; the per-module scan and the per-callee hierarchical pair each see one driver) | the CA lane has no statement; `frame_fn_lowering` disables the hoist; `inline_fn.rs` reserves the scoped frame at the call site after the carrying statement; `hier_body_write_callee` walks downward only (the same walk as the width leaves); `fold_straight_line` has no write slot; `stmt_never_writes_ident` cannot resolve a hierarchical callee | per site: the pkg lane needs the frame reserved before the statement; the outward / absolute path needs the fact walk to mirror `hier_resolve`'s enclosing-scope walk; the inline spelling needs framing; Rule A needs the hier callee's ports | 2 each | small–medium |
| frame-body-write-order | `acc = 0; r1 = acc + fw(5);` — hoisting the call moves its write ahead of an operand READ to its left: vita `r1=12 acc=7` = verilator; iverilog `r1=5`. IEEE §11.4.2 leaves operand evaluation order unspecified — an oracle split, recorded. The hierarchical twin `r = u.acc2 + u.fw(3)` is the same split (vita `ORD r=8` = verilator; iverilog `r=0`), and so is an `initial` / `always @(posedge)` / task-body caller beside an `always_comb` caller of the same hierarchical callee: both tools print the same values in the time step, and at a later probe verilator has re-evaluated the comb (`PAIR late acc2=3`) while iverilog and vita keep `11` | `hoist_inout_calls` emits the call before the statement | — | split | — |
| dyn-size-spellings | `$size(c.da)` (class member) and `$size(u.da)` (hierarchical) are loud where verilator prints 3 (iverilog `x` for the class case — disqualified); `$size(arr)` of a dyn-array FORMAL is E3010 (both oracles 4) | `resolve_intro_net` yields a net for a bare Ident only, so §4.5.500's dyn arm is never entered | route the three spellings to the same `DynSize` node | 1–2 | small |
| dyn-bits-count | `$bits(da)` of a dynamic array folds the ELEMENT width (32 for `int da[]`) outside a replication count; no oracle for the value (iverilog 1, verilator "UNSUPPORTED: $bits for dynamic array"); as a count it is loud | `try_introspect_fold` has no `$bits` dyn arm; §20.6.2 says the size in bits of the whole array | decide by LRM (`size × element bits`) and pin by hand | 0 | tiny |
| pkg-type-param-import | an EXPLICIT `import p::PT;` of a package `parameter type PT` is E3009 ``package `p` has no symbol `PT` `` where both oracles run it (`N35 b=8 lo=0 v1=1`); the wildcard `import p::*`, every `p::PT` spelling and the `typedef` twin of the explicit import run since §4.5.515 | the explicit-import binding is a third package-export registry beside the `pkg::t` typedef map and the wildcard export set; §4.5.515 extended the latter two | teach the explicit-import name check the type-parameter names a package declared | 2-oracle | small |
| gen-rtn-edges | after §4.5.473: a bare call of a generate-scoped routine from OUTSIDE its block is E3010 (both oracles reject — keep) · a hierarchical `u.g.f(x)` is E3009 (iverilog runs it, `f0 fe`; no `hier_funcs` entry), and so is the single-segment `gi.f()` into a generate scope of the SAME module, where BOTH oracles answer `F=21` (§4.5.522 census, PRE-identical) · a generate-scope routine in a CONSTANT expression (`localparam W = f(3)` in the block) is E3009 (iverilog `04`; `const_func_table` is filled by the module-body prescan only) · `frames_classify.rs:1069` retains callees by BARE name, so a generate-routine → generate-routine recursion edge is missed (loud-safe by that function's doc; unmeasured) · `tf_decl_scope` stays the module prefix for a generate-scoped routine, so `default_binding_matches_decl_scope` compares a default argument against module scope (traced to a conservative reject, untested) · `%m` inside a generate task is a split (iverilog `t.u.g.show` pinned, verilator `t.u.g.g.show`) | `frames_reserve.rs` hier gate · `instance.rs:496-505` const prescan · `frames_classify.rs:1069` · `scope.rs:379` | hier: compose the hier key from the qualified name · const: register generate routines into `const_func_table` per scope · edges/default-binding: measure first | iverilog | small–medium |
| aes§2 | the inliner's discriminator is more than `automatic` — plain 3/5, and `automatic` / `for` / `if` / `case` 1/5, `p::f()` 2/5 | the inliner's discriminator | widen the inliner | measured | — |

**Subroutine / frame**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| pkg-task-stmt | a package task enabled by its scoped spelling as a STATEMENT (`p::pt(z);`) is E2002 `expected '=' or '<=' after lvalue, found '::'`; the import spelling (`import p::pt; pt(z);`) runs. iverilog rejects the same line (`Malformed statement`), verilator runs it | the statement parser takes `::` only inside an expression, not on a statement head | accept a scoped call on a statement head and route it like the imported spelling | 1 (verilator; iverilog rejects) | small |
| blocal-inert-falseloud | an inner block-local that is DECLARED, never referenced inside its own block and carries no initializer is refused with ``E3009 block-local `x` is referenced outside its `begin…end` block`` although the flatten is byte-correct; both oracles print a value on every measured cell in the module and the import lane (the scoped lane is silently wrong instead — its own §2 row) | `check_block_local_scope_leaks` keys on the NAME, not on the binding a post-block reference takes | the gate must resolve that binding. Three narrowings that keyed on properties of the DECLARATION — inertness, geometry, an outer-twin lookup — were each measured to create a new defect, and the axis was reverted whole (§4.5.490) | 2-oracle | — |
| scoped-call-comb-arg | a scoped package call inside `always_comb` whose actual is a variable WITH a declaration initializer (`int i = 21; always_comb r = pk::g(i);`) is a false E3001 ``variable `i` has a declaration initializer AND is written by `always_comb` `` where both oracles print 44; the import spelling and the module-local twin run, and no block-local is involved | Rule A's driver walk counts the scoped call's actual as a write (the same conservative walk the `frame-body-write-sites` row records for a hierarchical callee) | resolve the scoped callee's ports before the walk, as the local twin does | 2-oracle | small |
| blocal-collector-parity | `collect_block_local_decls_spanned` (`block_local/mod.rs`) omits the `Wait` / `DelayCtrl` / `EventCtrl` recursion its sibling `gather_nested_block_locals` has, so a block-local declared under a timing-controlled statement in a subroutine body reaches neither the scoped feed nor §4.5.486's static-init prologue | latent: the shape is LOUD today (`E3010 undeclared net/variable top.s.$func$t.x`, PRE = POST), which is the only thing standing between the omission and a silent drop | align the collector with its sibling in the same edit that removes the loud; measure the drop channel first | — | small |
| dimquery-width | the dimension-query family (`$size`, `$high`, `$low`, `$left`, `$right`) and `$signed` are LOUD in every certified consumer (7 + 6 cells) although both are integer-returning and already foldable, so the §4.5.478 `SysCall` arm's named list excludes them | the named list in `param_decl_width_opt` / `ctx_width_names_are_evident` | admit them with their own census: `$signed` is NOT 32 bits, it is its operand's width | 2-oracle | small |
| §3.11 | inlining `function automatic` — "a non-recursive automatic is identical to an inline" is refuted by measurement (15 suite failures, `$random` drawn twice) | the inline expansion names the operand a second time | ⓐ name it once (a callee-purity predicate) · ⓑ open codegen (`is_codegen_able`'s `Terminator::Call` reject, §5 T1/T2) | — | — |
| static-local | `function integer f(input integer x); integer s; begin s = s + x; f = s; end` gives E3010 `undeclared net/variable top.s` | block-local flatten demands definite assignment; with control flow it becomes a frame and works, so the gap is exactly "straight-line body plus a read-then-write static local" | a read-before-write slot in the flatten | iverilog runs it with X | — |
| frame-oob | a frame-local array OOB read has no E4002 (a module array gives E4002 and exit 1) | a frame-local array is a packed slot with no array-word concept | elaborate must keep the slot's original geometry | vita is internally inconsistent | — |
| 2seg-call | `c.m()`, `u.size()`, `t.size()` and `ci.get_coverage()` to the left of a call with an output formal are loud. A deliberate trade: the silent answer was wrong (`t.o` gave `q=12` where 11 is right) | `order_walk` opacity cannot be answered by `callee_body_cannot_touch` (single-segment only) | a resolver for class-method and package-function bodies | iverilog | — |
| dyn-formal-pos | positions where a dyn-formal call is impossible: the right operand of `&&` / `\|\|`, an argument of another call, a select or lvalue index, a `case` scrutinee, a `repeat` count, a cast or replicate operand. 7 supported / 10 loud (9 of which iverilog PASSes, i.e. false-loud) | the narrow hoister (`hoist_dyn_formal_calls`) and the general hoister (`shape()`) cover different position sets | absorb into the general hoister (`__t = f(arr)`). Trap: the stand-down is `frame_fn_lowering`, so only frame-function bodies remain and the hoist is fine there — split it per call kind | iverilog 9/10 | — |
| pkg-default | package-function default-argument scope — `default_binding_matches_decl_scope` compares against `tf_decl_scope`, but a package function is recorded with the importing module's prefix | the declaration scope is not recorded per symbol | record it per symbol | iverilog | — |
| fgets-rhs | `return $fgets(line, fd);` gives E3009 "…only as the direct rhs of a blocking assignment" | the routing predicate and the lowering key on the same shape | widen both together | iverilog | — |
| V3/V4 | `wait(<frame-local>)` · `repeat(<non-const>) @` (the hidden counter is a SHARED net) · an NBA to a frame local · `fork` / `disable fork` / `wait fork` inside a task | no per-activation repeat counter, no in-frame fork machinery | each is its own slice | iverilog | — |
| frame-array | a frame-local array that is multi-dim, non-zero-based, non-simple-element, whole-copied (`b=a`), `foreach`-ed, NBA-assigned per element, or `'{…}`-initialized | outside the current scope | widen | iverilog | — |
| V2A/V5 | a dyn-array formal on an automatic or recursive task (the frame formal is a scalar slot) · a FUNCTION dyn-array local (the `&self` executor cannot run `new[]`, which needs `&mut` heap) · recursion and concurrency · a multi-dim, packed or non-bit-vector element | the `&self` executor | handle-in-slot · a per-activation heap stash | iverilog | — |
| foreach-fn | FUNCTION plus `foreach` on a dyn formal is loud | a framed function dyn formal is unsupported | the function-frame dyn-formal slice | iverilog | — |
| r16-exec | ① recursion using a dyn local or formal ② a dyn formal with a string, real or class-handle element ③ an unwritten output formal = IEEE §13.5.2 empty copy-out | — | a per-activation heap stash | iverilog is by-ref, i.e. non-conforming | — |
| re-forward | a FUNCTION re-forwards its own dyn formal (`return sum(c)`) | a framed function formal is heap-resident, so `dyn_array_actual_net` cannot resolve it | a mutual-recursion soundness hole ⇒ guard when routing to a frame | iverilog | — |
| hier-task | output / inout / array / string formals (cross-boundary copy-out) · a STATIC task hierarchical call · a hierarchical enable nested in a frame body (`task_calls_func` transitivity) · a hierarchical task call inside a generate block | — | make frame and inline equivalent | iverilog | large |
| array-formal | a non-zero-base descending array formal · hierarchical-task OUTPUT/INOUT array formals · forwarding a frame-formal array into a nested hierarchical call · re-forwarding · a non-zero-LSB element · 2-D, signed and task array formals | — | — | iverilog | hard |
| blk-automatic | block-local `automatic` lifetime (`automatic int j=k*10`) | per-activation storage = deep block-local flatten | — | iverilog rejects it too | deep |
| hier-default-arg | a HIERARCHICAL call does not honour the callee's formal DEFAULT values: `top.sh()` on `task sh(input logic [7:0] a = 8'hA5);` is loud / wrong where both oracles print `A=a5` | the hierarchical-enable lowering binds actuals positionally and never consults `collect_callee_ports`' defaults, which the inline and frame lanes both read | thread the callee's default list into the hierarchical bind, the way `with_default_arg_scope` already does for the other three lanes | 2-oracle | small |
| iface-modport-formal | `w.mp(40)` where `w` is an interface PORT FORMAL (`module sub(bus w); import pk::*; …`) and `mp` a modport of that interface is ``E3009 unsupported hierarchical function call `w.mp` (the callee must be a framed function …)`` — a pre-existing hier-call gate, not the §25.5 sentence §4.5.525 added, because `modport_call_refused` walks `iface_insts` only. Verilator refuses it as a modport; iverilog cannot parse `module sub(bus w)` and its control fails the same way, so it cannot judge the cell | `modport_call_refused` keys on interface INSTANCES; an interface port formal is not one | give the hierarchical-call route the interface-port-formal receiver, then the §25.5 refusal follows. Prerequisite: the hierarchical-call framing gate | verilator | small |
| genblk-fn-call | a function DECLARED inside a LABELLED generate block cannot be called by its scoped spelling: `gb.f(…)` is ``E3009 unsupported hierarchical function call `gb.f` (the callee must be a framed function with input-only scalar formals…)`` where both oracles run it (`RD=44`). One consequence beyond the call: §4.5.525's §3.13 walk does not descend into a labelled generate block (its contents are their own region, §27.3), so a DUPLICATE routine declared inside one keeps the pre-existing warning instead of a refusal, and the cell cannot be closed until the lane runs | the hierarchical-call route has no generate-scope callee | admit a generate-scope callee to the frame route | 2-oracle | small |
| misc-sub | `q.min()[0]` · `x.name().len()` · a package TASK statement call · a method or constructor NAME default at class scope · a G4 string-returning frame call | — | — | no oracle | — |

**System tasks & file I/O**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| plusargs-%0d | `$value$plusargs` over-rejects a width-qualified `%0d` spec — iverilog accepts it (value 5), vita gives E3009 | the spec parser does not strip the width modifier | strip it — one spelling in `exec::plusargs::effect`'s conversion-character extraction (the trap when it is relaxed: `'0'` reads as `%s`) | iverilog | small |
| writemem-local | `task automatic t; reg [7:0] loc[0:1]; … $writememh("x.txt", loc);` gives E3009 "a whole unpacked-array formal has no value here" | pre-existing and identical on both backends | the opening slice must take the seam with it — `read_task_net` uses this refusal as an unreachability argument and reads the arena bare-handed; pin = `writemem_targets_the_seam_cannot_own_are_refused_before_the_backend` | iverilog writes the file | — |
| filepos | `$ftell` and `$sscanf` give E3009 "unsupported system function in expression"; `$fseek` gives a W3056 warn-and-skip | a side-effecting system function in expression context | widen the statement-form desugar | iverilog works (`A=6 B=0 C=6 D=0`, `$sscanf` → `2 12 34`) | — |
| fmonitor | `$fmonitor` and `$fstrobe` are a W3056 skip, i.e. a warned silent drop of file output | `FmtCapture` has no fd | add `fd:Option<u32>` to `FmtCapture` and route the strobe drain to `file_write`. Needs a format bump · STDIN reads are a determinism decision | — | its own slice |
| $typename | an enum or packed struct renders as its base type (`logic[1:0]`; IEEE §20.6.1 says `enum{...}`) | rendering only | widen the renderer · pin `typename_pins.rs` | no oracle | no value effect |
| %p-ⓐ | an UNPACKED STRUCT and `string sa[2]` are E3010 at DECLARATION, so there is no net to render | a declaration gap | re-file under that feature | verilator | — |
| %p-ⓒ | two recorded divergences: a NEGATIVE associative key (vita follows IEEE §7.9.4 SIGNED key order, verilator sorts hex; `-1` is 64 bits) · a `real` unpacked array (verilator prints element 0 only while rendering a QUEUE of the same shape correctly, so it self-contradicts and vita follows verilator's own recursive rule) | — | keep the pins | verilator only | — |
| sformatf | `$sformatf` in a ternary arm, a short-circuit right operand, a `$monitor` / `$strobe` argument or a task argument | `eval`'s `SysFuncId::Sformatf` arm ignores the format string | lift `format_args_str` / `render_template` to a reader-generic form so `EvalCtx` can use them; that closes the family and retires the statement-level hoist | — | — |
| ext-shadow | ① 79 un-isolated items under §4.11 ② an enclosing same-name pair is shadowing and therefore loud, and a block local that collides with a module net stays as it is | — | — | — | — |

**Nets / timing**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| E3001-delayed | `assign #(D) bus = en ? d : 1'bz;` — two or more overlapping tri-state drivers exit 1 with E3001 | `check_whole_net_multidriver` mirrors `md_nets` on the rule "if any driver is delayed it is not a 4-state wire resolution target" | let the engine's `md_nets` resolve delayed drivers. Trap: a probe at 10 ns resolution cannot see a discarded 2 ns delay | 2-oracle (at 1 ns: `t=11 bus=1` against iverilog's `bus=z`) | — |
| E3001-overlap | iverilog resolves same-range part-select pairs (`assign z8[3:0]=…` twice) and delayed+plain overlap bit by bit (`zzzz0xx1`) where vita gives E3001 | there is no per-bit driver map | a per-bit driver map is the prerequisite | iverilog | — |
| hier-event | ``always @(`TOP.a_uVDC.RTRIM_I)`` — the read already works, so only sensitivity registration is missing | the patch target is `Process.sensitivity.edges[i].net` and that process is not pushed yet | a new lane that reserves `(proc_idx, edge_idx)` and patches when the instance is fixed | iverilog | — |
| xproc-disable | a cross-process `disable` | unsupported | "a `disable` of a target that is not suspended is a no-op" alone passes that library. Boundary: ignoring a suspended target too would be silent-wrong — if it is active, be loud | iverilog | — |
| timescale | partial-timescale diagnostics (`W-PARSE-TIMESCALE-PARTIAL` / `E-PP-TIMESCALE-PARTIAL`): when only some modules declare one, there is no diagnostic and 1ns/1ns is assumed (only the none-at-all case gives W1017) | not wired | the design is in doc-08 §15 and `rt.default_used` exists — wiring only | — | small |
| nonansi-child-array | an instance array whose child declares a NON-EMPTY non-ANSI header (`module ch(a); input a;` + `ch w[1:0]();`) is ``E3009 instance array `w`: child `ch` has non-ANSI ports (v1: ANSI only)`` plus one E3010 per element, where both oracles print `Q=4 4`. The PORTLESS child (`module ch;` and `module ch();`) runs since §4.5.522 | `instance_array.rs` reads per-port widths from the ANSI header only, so a body `PortDecl` list has nowhere to come from | read the port widths from the body `PortDecl`s the way the scalar-instance lane already does; pinned meanwhile by `nonempty_nonansi_child_array_stays_loud` | 2-oracle | small |
| implicit-net-generate | an undeclared name on the LEFT of a continuous assign INSIDE a generate block is `E3010` where both oracles infer the 1-bit wire and print `I=0`; at module scope vita infers it with `W2003` as IEEE 1364 §3.5 requires | the implicit-net inference is a module-body phase and the generate lowering does not re-enter it | run the same inference over a generate block's items (this is a POSITION gap in vita's own §3.5 policy, not the §8 IMPLICIT-NET non-goal) | 2-oracle | small |
| modport-port-actual | a modport EXPRESSION as a port actual — `module sub(ifc.mp p); … ifc w(); sub u(w.mp);` — is ``E3002 interface port `p` must be connected to an interface instance name`` plus an `E3010` on `p.d`, where verilator runs it (`G12 43`); iverilog cannot parse the `ifc.mp p` port declaration | the interface-port binder takes an interface INSTANCE name only | take a `<instance>.<modport>` actual and bind the modport's view | verilator | small |
| iface-generate | a `generate … endgenerate` region inside an interface body is ``E3009 generate blocks inside an interface are outside the MVP`` where both oracles run the design (`TOP=ok`) | `iface_inst.rs`'s MVP gate for interface body items | run the region through the interface window the way the module lane runs it; the routine and import lanes were opened by §4.5.517–518 | 2-oracle | small |
| level-select-event | a LEVEL event control on a NET select — `@(n[0])`, `@(n[1:0])`, `@(n[I+:2])`, `@(a[1])` (an unpacked element), `@(a[1][0])`, `@(n[i])`, `@(a[j])`, in the header and the in-body lanes, alone or beside a live term — is E3009 (a bit or element select: "a level (non-edge) event control on a bit or element select is not supported"; a part or indexed part: "event control must be a bare signal name or a constant LSB bit-select"). After time 0 both oracles agree on every non-x cell: the process wakes only when the selected bits change (other bits, other elements and same-value writes do not wake it); a variable index follows the index, and an index change wakes it only when the selected value changes; a wire, a 2-state `bit` and a non-zero-LSB range behave the same. verilator additionally runs every header level `always` at time 0 (see "Oracle splits") and misses x transitions (2-state) | the frozen `EdgeTerm { net, kind }` and `WaitCause::Level { nets }` carry no bit field, and the level wait fires on any change of the whole net | a derived 1-bit or element net per constant select, or a per-bit level waiter (a bit field on the frozen types = format bump); a variable index also needs the selected value re-read on every index change | 2-oracle after time 0; iverilog alone for x transitions | M (bump) |
| deep | a t0 race · an `@(*)` decl-init wake · a runtime `==?` pattern · a NON-fill context width in an inline body · modport direction enforcement · a force on a part-select · an associative key or clocking array output word0 · a PART select of a negative range bound (§2) | — | — | — | deep |

**Loud shapes surfaced by §4.5.493–495 (all 2-oracle unless noted; each PRE == POST)**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| ac-random-stmt | the STATEMENT form `$random(sd);` inside an `always_comb` beside `integer sd = 7;` is E3001 since §4.5.498 (the seed write is a write now; the EXPRESSION form was E3001 before too); iverilog runs it (`Y=11`), verilator refuses the shape ("Circular logic") | §9.2.2.2 by design — two drivers on `sd`; recorded as the one value→loud cell of §4.5.498 | none unless a two-oracle cell appears | iverilog | — |
| readmem-write-table | `$readmem*`'s memory fill is not in `syscall_writes_arg`, so the never-writes walk calls a memory only `$readmemh` writes "never written"; the reject-gate direction (multidriver Rule A) is measured harmless and the accept-gate direction (a fork block-local memory filled by `$readmemh`) is unreachable in v1 | the write view of the table lists the engine's `StmtEffect` writers; `$readmem*` fills through a different funnel | add the `$readmem*` arg-1 row when a reaching design exists | — | small |
| real-cont-assign | `real w; assign w = fr();` is E3018 (continuous assign drives variable) where both oracles print `1.000000`; the `always_comb` twin runs | a `real` variable is not an admitted continuous-assign target | admit a real variable as a continuous-assign destination (the engine's real-target guard is already keyed on the net kind) | both | small |
| real-fmt-hex | `$display("%h", f())` / `$display("%h", r)` on a real is E3009 where iverilog prints the rounded integer (`3`) and verilator `0000000000000003` — value agreed, width split | the format gate refuses a real for `%b/%h/%o` | round to integer and print at 64 bits (verilator's width; iverilog prints minimal — a format-width split, value is not) | value both | small |
| pkg-writer-import | `import pk::*` from a package holding a FUNCTION that writes a package variable is loud at the import even when that function is never called (frame classifier: "assignment to a net outside the function"); both oracles run the design | `apply_import_routines` injects every wildcard sibling and the frame classifier validates each injected body at reserve | classify lazily (on first call) or make the write rule per-callsite | both | small |
| fn-writes-pkg-var | a package FUNCTION writing a package variable (`cnt = cnt + 1; gw = cnt;`) is loud in both lanes where both oracles print `W=8 9`; a package TASK doing the same is correct since §4.5.493 | the frame classifier refuses a whole write outside the frame for a function (a task performs it) | admit a function's write to a module/package net through the task path's funnel, with the §13.4.4 "no side effects in a function" warning the oracles do not give | both | medium |
| class-real-members | a `real` class property, a `static` method and `for (int i …)` inside a class method are loud on both binaries where verilator runs them; a body-local anonymous `enum {A,B} v;`, a `typedef enum` inside a nested `begin…end` or a `fork`, and a body-local `localparam` are parse-loud where verilator runs them | class member kinds and the parser's body-typedef position set | per row; the enum ones are the parser's `body_enums` collection position | verilator | medium |

**Diagnostics quality**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| line-directive | a `` `line 500 "phantom.sv" `` directive above a declaration is ignored: every location the rail reports (`subroutines[].decl_*`, `subroutine_calls[].decl_*`, diagnostics) names the physical file and line. Both objects agree with each other | the preprocessor does not consume `` `line ``; the span resolver maps byte offsets to the physical file only | a `` `line `` table in the preprocessor consulted by the span resolver | — (not a value; convention) | small |
| E3009-anchor | `E3010` / `E3009` file:line is inconsistent — some sites attach it (`d_trunc.v:3:20`) and some print only the hierarchical path | the anchor is not passed through | `diag::SpanResolver` exists, so the scope is every call site that does not pass an anchor | — | — |
| error_at | the anchor and the `found` token differ — `g[w].u.q` anchors at `w` and the message says `found '.'` | `error_at` takes an earlier node while `found` takes the cursor token | they are separate fields, so this is correct-but-confusing; 10 sites | — | — |
| #9 | the `velab -L` (worklib merge) path has no locations | each compilation unit's spans index its own expansion buffer from 0, so the coordinate spaces overlap; a wrong CU map would give a wrong file:line, so `None` is kept | rewrite span offsets at merge time (a whole-AST walk) | — | — |
| cli-lib | `cargo test -p cli --no-default-features --lib` dies with E0004 (pre-existing) | the lib test target revives sim-engine's `oracle` through a dev-dependency link while the cli feature stays off, so two `#[cfg(feature="oracle")]` arms of `backend_name` are cut | set the cli dev-dependency to `default-features = false`, or merge the two crates' `oracle` into one. CI cannot see it, so do not add `-p cli` to that command | — | — |
| carrier-namespace | the `T$w` / `T$s` / `T$d…` / `T$p…` carrier names a type parameter desugars into are not reserved at DECLARATION sites: a user `parameter int PT$w = 3` beside a type parameter `PT` in another scope is accepted (illegal in both oracles; the one LEGAL spelling that read a wrong type, `import p::*` over a unit-scope `parameter type PT`, is closed since §4.5.515 because a non-overridable parameter registers literal dims that name no carrier) | `names_a_type_param_carrier` is consulted only at instance-override names (`instances.rs`) | refuse a `$`-carrier spelling at every declaration site | both oracles reject | small |
| EXT2-DOC | stale documents (CLI reference, language reference, system tasks, explain) | — | — | — | — |

**Strings / heap**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| paren-select | a string byte select on a parenthesised base is silently 0 — `(p)[0]` is 0 against `p[0]`'s 119 (`(v)[0]` and `(v)[3:0]` are exact) | `string_index_read`'s base gate `matches!` only `Ident \| BitSelect` and does not unwrap `Paren`, so it falls through to a width-0 handle's packed bit select | unwrap `Paren` in the gate | no oracle | one gate |
| real-part-write | `real x; x[3:0] = 4'hF` leaves the value unchanged with no diagnostic | the dynamic `real` ELEMENT is loud, so the scalar is the asymmetric half | make the scalar loud too | iverilog rejects it ("can not select part of real") | — |
| reduction-init | `string s = $sformatf("%0d", arr.sum());` gives E3009 "unsupported hierarchical function call arr.sum" (`q.size()`, `.len()`, `.substr()` and `.name()` work) | a gap on the t0 pre-sweep path | add reduction to the pre-sweep path | — | — |
| string-array | T1: a FIXED string array declaration initializer (`string s[2]='{"a","b"}`) · a fixed array runtime index and `foreach` · `string q[$]` · `string s[2][2]` · a hierarchical `u.s[0]` · a frame-local string array (static task = E3018, function/automatic = E3009) · a dynamic element byte select `d[0][0]` | the fixed case is const-index-only because of the element-net representation | — | iverilog supports it | T1 |
| inline-string | a static task's inline string local (`hoist_inline_task_locals`) becomes a Wire and gives E3018 | the inline path is not a frame slot, and a naive String conversion does not get the `str_bytes` twin | its own inline-string-storage slice | — | — |
| string-misc | a substr actual `s[i]` · `s[i:j]` · `s[i].len()` · a whole-element read (`x=arr[i]`) · an array of records inside a record · string and real elements of a queue or associative array · a string queue · a block-local queue declaration · a hierarchical `u.q[0]` read | — | — | — | — |
| dyn-md-elem-select | a select INTO an element of a queue / dynamic / associative array whose element type has two or more packed dims (`typedef logic [1:0][3:0] t; t q[$]; q[0][1]`, `q[0][5:2]`, `q[0][2+:4]`, and `T q[$]` with a 2-D type parameter) is E3009 since §4.5.514; before it the trailing index silently bit-selected the flat element (verilator `a`, vita `0`). A whole-element read / `push_back` / a static array of the same element type are correct | the element read is a flat word and no packed-dim table exists for a dyn handle (`packed_dims` is recorded only in the static branch of `netdecl.rs`) | record the element's packed extents for the handle net and route the trailing chain through `lower_packed_read` on the element word; the element WRITE `q[0][1] = …` is the pre-existing `nested lvalue select` refusal | verilator (iverilog rejects the two-index form) + hand-IEEE §7.4.5 | S |
| class-field-select | a part-select of a class field (`c.u8[3:0] + 16'h109` with `bit [7:0] u8 = 8'hC3`) is `E3010 undeclared hierarchical name \`c.u8\`` | the select's base is not recognised as a class-field read and falls to the hierarchical-name resolver (the message) | route the select's base through the class-field read | verilator `010c`; iverilog prints `01cc` — the select ignored, so not an oracle here | small |

**VCD / real conversion**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| vcd | cosmetic encoding differences (decoding identical): ① vita writes full width (`bxxxxxxxx`) where iverilog strips (`bx`, `b0`) ② the t=0 initial dump is a `$dumpvars` pre-assign X plus a `#0` change against a settled value ③ a procedurally driven `logic` is `wire` in vita and `reg` in iverilog, and `int` is `reg` against `integer` ④ a real's size is 64 against 1 ⑤ `parameter` is not dumped | elaborate's packed-md `NetVar.lsb` is stale (the VCD helper routes around it with a flat fallback) | — | iverilog | large golden churn |
| x→real | an X-bearing integral converted to real: vita takes the whole value to `0.0` where iverilog converts per bit (`4'bxx01` → 1). Shared by `$itor`, `$sqrt`, `$pow` and real `**` | `real_arg` is `to_i128_signed().unwrap_or(0)` | convert per bit. The same whole-value rule answers an integral with x/z bits inside a real expression at every store: `m = R + X` with `R = -3.5`, `X = 8'b0000_00x1` is `fffc` / `Y=-3.500000` in vita against iverilog `fffd` / `-2.500000`, uniform on the module, frame and inline lanes; `pb(rf2(8'bx000_0001))` with `rf2 = k + 0.5` prints `0001` against `0002`. Since §4.5.526 four inline cells that were loud reach this rule as a value (IEEE 1800-2017 §6.12.2 zeroes each x/z bit, not the value) | iverilog (verilator refuses z) | not silent |
| wide→real | an integer wider than 128 bits converts to `0.0` (65..=128 is exact) | `to_i128_signed` reaches 128 bits | a word-grid f64 approximation | — | very rare |

### 3.c Intentionally loud (not gaps)

| id | reason |
|---|---|
| §3.1 DPI-C and `export "DPI-C" function` | permanent non-goal |
| `$value$plusargs` in an arbitrary expression | `ok = $value$plusargs(…)` and `if ($value$plusargs(…))` already work; the remaining `$display("%0d", $value$plusargs(…))` shape belongs to the side-effecting system-function family, whose design lowers to a statement form to guarantee single evaluation — there is no statement to desugar into, so loud is the right answer |
| `%p` of `int a[0:0]` | `sim_ir::NetVar` carries only `array_len` and a scalar is 1 too, and `unpacked_array_nets` does not reach the engine, so accepting it would print the element without braces at exit 0. Prerequisite = array-ness in the IR or a new sidecar plus a format bump |
| a direction mismatch on an unpacked array port (`[0:3]` against `[3:0]`) | IEEE §7.6 pairs elements by position, so a flat-index connection reverses the order (vita 4, iverilog 1); implementing it requires `wire_array_port` and array assignment to spell the position↔index mapping the same way |
| a fixed-array fill whose coverage cannot be proved (computed index, conditional write, incomplete set) | a rule, not a gap; the diagnostic states the reason directly |
| calling the same dyn-formal function twice, or recursively, in one frame-body expression | there is one marker slot |
| shadowing by a block that encloses a block and redeclares the same name, when NO module-scope net carries that name | two static block locals with initialisers under one name put two pre-arm initialisations on one flattened net, so the later overwrites the earlier (iverilog 7/9 → 9/9); stays loud. The twin WITH a module net of that name is inside the shadow regime and is correct since §4.5.468 (`INNER=9 OUTER=7 MOD=0`, both oracles) |
| a `$readmem*` child-versus-parent `initial` race | IEEE §4.7 leaves `initial` order nondeterministic and the two oracles write it opposite ways — iverilog `aa bb cc dd`, verilator `01 02 03 04`, vita = verilator ⇒ ORACLE SPLIT |
| `$readmemh` into a `wire` array | iverilog rejects, verilator accepts ⇒ split; vita accepts for local/hierarchical parity |
| a header default that names a constant from a body import | split — iverilog rejects, verilator folds |
| two iverilog 13.0 defects (vita is IEEE-correct) | ① when a loop body block declares a local, `break` behaves like `continue` ② `continue` inside a `case` item aborts on a `vthread.cc` assertion |
| `%u` / `%z` and `%l` | both are documented choices (vita prints nothing, iverilog prints raw bytes); `%l` is cosmetic |

## 4. SVA / verification honest-loud residues

- Fusing an empty-match `##0` with an unbounded `##[m:$]` — no oracle — prerequisite = the §16.9.2.1
  discontinuity.
- N2c full sequence local variables (each nested attempt has its own data, L-grade) — a single
  capture is supported — prerequisite = a nested-attempt data model.
- A later-antecedent read and the advanced form of an outer `|=>` property-reference skew — no
  oracle — prerequisite = a 2-cycle, nested and cross-clock census.
- SVA-QUAD collapse default flip — currently opt-in behind `VITA_SVA_COLLAPSE` — prerequisite = a
  full-VCD golden audit.
- N4 clocking residue = the skew VALUE itself (the block-wide `default input/output SKEW` of
  IEEE §14.3 is parsed and applied). `output #0` has no anchor — iverilog cannot parse `clocking` and
  verilator samples in the Observed region — prerequisite = hand-IEEE §14.11 / §14.16. `input #0`,
  `#N` and `##N` are a different region and stay loud.
- class: a down-cast `Derived'(base)` · a real→longint cast · a base-shadow `Base'(d).v` · a
  cast-as-receiver `(B'(d)).foo()` — prerequisite = a `$cast` type guard.

## 5. Performance / hardening

The performance axis has reached diminishing returns and ranks below the correctness ladder. A row
resumes only when its own re-entry condition becomes true. The full measurement records behind the
`§5.1-<x>` identifiers are in [history/ROADMAP_ARCHIVE_PHASE_A-D.md](history/ROADMAP_ARCHIVE_PHASE_A-D.md).

### 5.a Standing verdicts

| id | verdict | reason (one clause) | re-entry condition |
|---|---|---|---|
| codegen (cranelift), including the `§5.1-be` machine-code experiment | rejected | the boundary is ~38% of a run while the ceiling is 8.9–11.3%, and 56–86% of executed `wprog` programs are a single `Load` / `Const` op | a way to inline leaf loads and 2-state arithmetic into generated code (zero calls) without writing the semantics twice |
| D2-b two-state storage | rejected | the trap is a step DOWN the accuracy ladder | find a way with no correctness trade first |
| cycle-based mode | rejected | picorv32's ratio is 10.32 against the gate's 1.84, and a combinational block is evaluated only 0.097 times per cycle, so event-driven already skips 90.3% of the combinational work | real demand with ≥1 evaluation per block per cycle |
| levelize (rank-ordered Active drain) | discarded | building the ranks and measuring gives 1.00× across depths 1–24; the root was `settle_cont_assigns` and the dirty settle closed it | none |
| process fusion (the E axis) | not adopted | it is not semantics-preserving in a simulator whose intra-delta order is pinned to iverilog — a reader of the chain's output sees the fully propagated value, so exit 0 with no diagnostic and a different value = silent-wrong | none (counter-example: `a_comb_chain_output_is_sampled_mid_propagation`) |
| net-count reduction (flatten) | on hold | it erases the targets `--probe`, hierarchical VCD, `%m` and hierarchical references name (a G2 conflict) | an owner ruling |
| S4 schedule elision · S5 NBA specialisation | stopped | S4's target sum is ≈6% = 1.06×, below the 1.3× stop threshold; S5's `k_schedule_nba_scalar` is 3.8% | none |
| the `wprog` reject family inside settle | not started | admitting all of it is ~2–2.5% of serv overall, and it requires turning `Tern` into a conditional jump | when the prize exceeds the machinery |
| `drain_range_diags` early-out | rejected | zero gain | none |

### 5.b Open performance and hardening residues

| id | symptom · measurement | mechanism · code site | fix shape · prerequisite | expected gain (as stated) |
|---|---|---|---|---|
| 4b-r | interp pools scratch buffers and native does not | `fire_waiters`'s `Vec<bool>` · the md-group `vals` in `settle_cont_assigns` | reuse kernel scratch | not measured |
| 5c | a frame body does not enter the compiled backend · profile (keccak_f, 5,949 samples): the generic walk is 25.1% against `WProg::run`'s 2.0% | the frame window is a `Vec<Value>` (`state/mod.rs:585`), so every slot read copies 72 bytes (`frame_eval.rs:281`), while `wprog`'s `Load { vi }` needs a flat u64 pair ⇒ `arena.frame` declines (`wprog.rs:461`) | a flat word window · prerequisite = arena | 6–10 weeks · addressable ~38% · ceiling 2.33× (the keccak_f_arr row) · re-priced aes 3.13×, arr 2.52×, keccak_f 1.65× |
| ARR-LHS | a 2-D / 3-D / packed element LHS is a ~10× cliff on both backends — against 1-D's 50.0 ns: 2-D 546.7/675.8, 3-D 829.2/967.5, packed `logic [63:0][31:0]` 410.8/441.7 ns (native/vm) | not located; the 0.81–0.93 ratio makes it shared plumbing | its own census first — do not guess | not estimated |
| INLINE-FOLD | the inline fold is exponential — `elab_s` is flat at 0.35 ms while `sim_s` goes 0.16 s → 14.36 s; six statements reading a local once are 0.19 s inlined against 0.24 s framed, and three reads are 14.39 s against 0.35 s | the arena shares subtrees as a DAG and the evaluator re-walks them as a TREE | per-activation memoisation · widening the inliner is the wrong direction | not estimated |
| MEM-GUARD | there is no process-level memory guard — a runaway `vita` can take 33 GB, and two of them can drive the machine to a kernel panic | the existing guards (`max_deltas`, `max_body_steps`, `time_limit`) cannot see a loop inside a system task that advances neither a delta nor a statement (`$writemem*` is closed) | ⓑ an RSS watchdog plus a default cap and `--max-mem` · prerequisite = macOS `mach_task_basic_info` is unsafe FFI · ⓐ an allocation-counting allocator is a regression if ON by default | — |
| CI-NEXTEST | `cargo test --workspace` runs 450 targets sequentially in 724 s (per-target sum 62 s) against `cargo nextest run --workspace` at 30 s for the same 5183 tests · 4 CI jobs remain | the two runners do not share a build tree, so each switch costs ≈470 s of rebuild | replace the 4 jobs in ci.yml and pin nextest 0.9.100 (0.9.143 requires rustc 1.91) · prerequisite = temp-name collisions (368 files named `vita_<tag>_<pid>_<per-process counter>`; nextest gives every test a new process, so the counter restarts at 0 and PIDs are reused) | 24× |
| MSRV-CEIL | the "follow new Rust" policy is unverified — the toolchain file, `rust-version` and all 4 ci.yml jobs pin 1.85.0, and there is no `stable` or `beta` job | nobody stands on the ceiling | one non-blocking `stable` job | — |
| EXEC-ROWS | `native::run::executor_rows` scans every statement of every process on every `simulate`, independent of the backend | a census must be executor-independent, so unconditional is correct | cache it if the cost matters | — |
| DELAY-CLAMP | a `#delay` above u32::MAX is CLAMPED — wrong only in a run that actually reaches 4.29e9 ticks | the IR field is u32 (a frozen type) | representing it needs a format bump; announcing it needs a new W-code | completes correct-or-loud |
| KPRED-3RD | the tier-3 decision has no "can today's kernel run it" layer — `$sformatf`, `$display`, transport-delay NBA and re-arm are eligible and buildable with no kernel | run.json carries only `eligible` / `buildable`, and `kpred::rhs_routes_to_worker` is not on the gate | add the third layer when dispatch is wired | removes a misreading |
| QUIESCE-NBA | tier-3 quiescence does not consult the kernel's `delayed_nba` — if a transport is the only pending work the run reports quiescent and the update disappears | the engine's `next` is the minimum of the `Scheduler`'s `wheel` / `delayed_ca` / `delayed_nba`, and in a native run those maps are empty | S1d-4c-2 | the only remaining step down the ladder |
| BYTE-GATE-6 | the S1d-4d byte-identity gate will meet 6 pre-existing oracle differences: ① the iverilog thread halt at a system call after `$finish` (the pending NBA at the finish tick applies since §4.5.531; the split is recorded under §2 Oracle splits) ② VCD intra-tick granularity ③ t=0 `initial` order ④ t0 arm order ⑤ an `always @(*)` with an empty read set runs at t0 in vita only ⑥ a `.velab` is not reproducible across `vcmp` runs (RULEV-MTIME) | half deliberate design, half LRM-undefined | decide which way each of the six is pinned, first | — |
| MON-RENDER | the tier-3 render path refuses `$monitor` / `$strobe` | rendering lives in `sched/run_loop.rs::flush_postponed` and that path takes no reader | wiring — one slice with S1d-4c | lifts the refusal |
| FD-EOF + FEOF | the `fd_eof` X-poison hole in `NetArena` (`fd_eof` alone is outside the "no heap/class/frame" argument; the `$feof` over-marking currently hides it) · `$feof` is over-marked in the canonical statement-effect predicate, so `e = $feof(fd);` is refused while `while (!$feof(fd))` passes | `k_feof` is a pure read while `sysfunc_is_stmt_effect` says `true`; fixing one consumer leaves two spellings | one slice · fixing the canonical predicate also widens the tier-2 gate · a byte-identity argument | removes a tier-3 over-refusal |
| NETSLOT-PREV | nothing in the workspace reads `NetSlot.prev` (only the declaration, the constructor and pass (c)'s write), so pass (c)'s two `clone_from` calls per changed net per delta are dead work | nobody reads it | remove it, plus a separate slice that verifies the obviousness itself | perf |
| ELAB-PHASE-BLIND | the corpus cannot see a front-end regression: EVERY workload is ≥99% simulation (biriscv 1%, the rest 0%), so a 3× elaboration cost moves the median wall time by nothing | corpus workloads are chosen for a long accumulating digest, which is the opposite of front-end weight | `corpus-runner run` prints the phase split per row, which makes the number READABLE; a THRESHOLD needs a front-end-bound row (many declarations, short simulation) with a pinned digest and an oracle | a regression the gate can see |
| LOW-ROI | FMT-CACHE part b (`render_template` pre-segmentation) · GEN-3X-STR part a (an unroll-plan cache — byte-identity risk exceeds the gain) · QUEUE-MID-ON (O(n) is inherent to the spec, and iverilog is the same) | — | on hold · QUEUE-MID-ON is permanently monitor-only | — |

### 5.c Current state

| | |
|---|---|
| default backend | `native` (tier 3) · corpus 100.00% executed · 0 divergences |
| product shape | `--no-default-features` = one executor · a gate refusal is fatal |
| workload corpus | 10/10 · 0 rejections |
| codegen | OFF by default and rejected — the build, the wiring, the measurement and the correctness are all in place |

## 5.2 Queue (start order)

Canonical start order. LOOPROMPT.md's NEXT mirrors this table; when they differ this table wins. One
loop iteration takes ONE row (single root, two oracles, outside the walls and the oracle splits,
no format bump visible at selection), optionally plus one review-free hygiene item. Two rows with
the SAME ROOT are one slice, and their ORDER is a measurement: a value lane must land before the guard over an
unlimited fold is deleted, or the deletion is 8 cells of loud→silent-wrong.

| # | slot | item | source | rank |
|---|---|---|---|---|
| 1 | 1 | an EXPLICIT `import p::PT;` of a package `parameter type PT` is ``E3009 package `p` has no symbol `PT` `` where both oracles run it (`N35 b=8 lo=0 v1=1` for `PT v; v = 8'h23; $display("N%0d b=%0d lo=%0d v1=%0d", v, $bits(v), $low(v), v[0])`); the wildcard `import p::*`, every `p::PT` spelling and the `typedef` twin run since §4.5.515 — the explicit-import binding is a third package-export registry beside the `pkg::t` typedef map and the wildcard export set (§3.b `pkg-type-param-import`). First action: census the three registries' producers in `package.rs` on HEAD and teach the explicit-import name check the type-parameter names a package declared | §3 | ② |
| 2 | next | `scoped-call-comb-arg` · a mixed-caller callee · `m #(8)` / `defparam u.T$w` · the VCD `$scope` `[0]` spelling · a `genblk<N>` label collision (split) · the §2 🆕 L ⓦ residue · the §2 🆕 N residue | §3 | ② |
| 3 | hygiene | measured with `wc -l` at HEAD, production files only. Over the 1,000-line policy and NOT on the exception list: `elaborate/packed.rs` (2,393), `elaborate/params.rs` (2,281), `sim-engine/native/kernel.rs` (2,963), `sim-engine/backend.rs` (1,994), `sim-engine/native/wprog.rs` (1,918, its vocabulary already split into `wprog/why.rs`), `elaborate/const_eval.rs` (1,916), `elaborate/package.rs` (1,907), `sim-engine/state/frame_eval.rs` (1,814), `elaborate/lib.rs` (1,728), `elaborate/const_fn.rs` (1,685), `sim-engine/value.rs` (1,669), `elaborate/instance.rs` (1,632), `sim-engine/state/changes.rs` (1,543), `hdl-parser/module_items.rs` (1,536), `sim-engine/sched/scan_arm.rs` (1,513), `elaborate/frames_classify.rs` (1,397), `elaborate/frames_reserve.rs` (1,395), `hdl-parser/typedefs.rs` (1,389), `sim-engine/alias.rs` (1,387), `elaborate/const_wide.rs` (1,368), `sim-engine/lib.rs` (1,331), `sim-engine/jit.rs` (1,305), `sim-engine/native/run.rs` (1,280), `elaborate/stmt_flow.rs` (1,250), `sim-engine/eval/eval_core.rs` (1,241), `sim-engine/state/init_diag.rs` (1,227), `sim-engine/state/task_frames.rs` (1,217), `elaborate/expr_size_ctx.rs` (1,208), `elaborate/expr_ctx.rs` (1,219), `hdl-parser/lib.rs` (1,189), `sim-engine/builtins/dispatch.rs` (1,158), `sim-engine/state/mod.rs` (1,149), `sim-engine/builtins/queues_io.rs` (1,140), `hdl-lexer/lib.rs` (1,113), `elaborate/ports.rs` (1,094), `cli/src/frontend.rs` (1,087), `sim-engine/native/frames.rs` (1,059), `hdl-parser/functask.rs` (1,038), `cli/src/stage_args.rs` (1,037), `hdl-parser/params.rs` (1,035), `elaborate/expr_special.rs` (1,034), `elaborate/net_util.rs` (1,033), `elaborate/arrays.rs` (1,023), `elaborate/dynarr.rs` (1,021), `elaborate/strings.rs` (1,014). `param_query.rs` (854) is the precedent for the `params.rs` split; §4.5.493 put its lane in a sibling module (`pkg_body_scope.rs`, 160) rather than growing `package.rs`, as §4.5.490–491 did with `block_local_feed.rs` (105) and `inline_body_ctx.rs` (290), and §4.5.520/522/523 added `param_dup.rs`, `gen_scope_name.rs` and `ident_route.rs` the same way; §4.5.526 took `elaborate/inline_fn.rs` from 1,172 to 895 lines by moving the bind into `inline_bind.rs` (268) and the straight-line body fold into `inline_fold.rs` (165). NOT inside a correctness bundle — a refactor is a design nobody has reviewed | [ENGINEERING_RULES.md](ENGINEERING_RULES.md) §10.1 | — |

Do not start:

- §2 rows 16, 26, 30 and 🆕 F — the §11.8.1 region-sign wall. Rows 14 and 25's declared-width
  provenance half is NOT a wall: `param_range` carries the provenance and `narrow_param_bits`
  resolves it at the override fold; what remains there is the i64/CARRY axis.
- §2 row 34 (one oracle, zero demand) and row 31 (the pure half is correct, so it is performance).
- §2 row 32 — re-measured 2026-09-21 and reclassified ORACLE-SPLIT: iverilog and verilator answer the
  `$finish`-inside-a-function boundary differently and neither can be followed without contradicting the
  other on the `$fatal` spelling.
- §2 🆕 Q — a block-scoped CONSTANT binding is the prerequisite; the bare-name hoist measures 5 new
  silent-wrongs.
- Any widening of the wide fold's accept set before §11.8.1 region sign stands, which includes
  §2 🆕 H ⓐ (the fix site is `fold_self_bits`'s reduction arm, i.e. the accept set itself).
- §2 row 10's surviving bound half, whose prerequisite is a wide resolver that reads a SELECT.
- ALL of §2 row 7: one ordering key cannot express what the oracles do, and the settle-reached half
  is already correct, so the remaining cell is an oracle split on order.
- §3 ⑤ⓕ's ARITY (dim COUNT) axis: the prerequisite is measurably absent. The declarator dim list is
  stamped once per module at parse (`decls.rs:606`) and `ast::DeclName.unpacked` has no per-instance
  slot; `monomorph` cannot supply it (`hdl-parser/src/monomorph.rs`, 0 references to `ModuleDecl`,
  1 call site, class-only); `ModuleMap` holds a module as `&'a ast::ModuleDecl`, so there is no
  per-instance AST copy. A fix needs a symbolic arity marker on `Dim` / `DeclName` (both SchemaHash
  roots in hdl-ast) plus 83 elaborate readers of `.unpacked`.
- §3 ⑤ⓕ's CLASS PROPERTY shape: the prerequisite is per-instance class registration, not a shape
  slot. `NetVarDecl` already carries `shape_param` and `classes.rs` already calls the funnel, but
  `register_classes` is a whole-design prescan that runs before any instance exists, so a property of
  type `T` is loud with `E3009 undefined name T$w` with NO override at all and on the WIDTH axis
  too. It is not a shape row.
- §3 ⑤ⓕ's function RETURN type (1 oracle, iverilog SIGABRTs, frozen `FunctionDef`).

Incoming compatibility reports pre-empt the queue; reproduce every item at HEAD first. Oracle-split
axes (§2 "Oracle splits") are never chased.

## 6. G2 — the AI-agent observability track

SPEC = [preview/19-ai-agent-observability.md](preview/19-ai-agent-observability.md).

Teeth = a 3-way internal differential (JSONL ≡ VCD ≡ `$display`) plus a determinism golden, and an
ASYMMETRIC MUTATION for anything that REPORTS (change something upstream that must not move the
numbers; a same-input repeat run cannot see a reporting defect). A wrong log ranks with a
silent-wrong. Value encoding: `trace`'s `old` / `new` are full-width 4-state binary, `stage`'s
`vals[]` are `%0d` decimal (doc-19 §3 pin 4). The whole track ranks third, behind §2 and §3.

| stage | deliverable | size |
|---|---|---|
| OBS-2 residue | `sva.jsonl` (R-L6: property name plus support cone, v0) · a per-element array probe · a real, class or event probe (no oracle) — all three are loud-rejected at the CLI today | M |
| OBS-1 residue | staged `vcmp`/`velab`/`vrun` obs (`.velab` source identity) · a compile-fail manifest (a front-end or elaborate failure writes no obs directory) · `--seed` (`"seed": null` is hard-coded) · `run_id` · `-G` / `--param` overrides in `run.json` · full input identity in `source.blake3` (it covers the source TEXT only, so two runs of one `` `ifdef ``-switched file with and without `-D FOO` report the same digest) · `results.jsonl` v2 (per-testcase ledger, `detail_ref` on FAIL) · R-L2 failure detail (`fail/*.json`) · R-L5 SVA pass/fail and cover-property counts and per-bin hit detail in `coverage.json` | S-M |
| R-L4 | log-channel separation (handshake and protocol channel events) | M |
| OBS-4 | `vrun --control stdio` JSON-RPC (`peek`/`poke`/`step`/`run_until`/`finish`) plus a poke journal for replay | L |
| OBS-5 | snapshot / restore / rewind (postcard serialization of engine state) | L-XL |
| OBS-6 | X-origin (`cause: uninit \| multi-drv \| arith-X`) · region-annotated events (no record carries a region or delta field) · a static backward dataflow slice | L+ |

Open items beside the staged track:

- The CALL TREE (doc-19 §4.9 item 1) is the top outstanding request and is not shipped:
  `processes.items[].domain` is `process` / `assign` only. The blocker is stated in doc-19 §4.9 —
  vita lowers a subroutine TWO ways (a frame body behind `Terminator::Call` / `Expr::Call`, and an
  elaborate-time INLINE splice), so a profile built on the runtime seams reports 0 calls for every
  inlined subroutine, and "0 calls" reads as "free" about the very thing the user is hunting.
- The static `subroutines` object files `function void vf(...)` as `"kind": "task"`; the row set and
  the route are otherwise right, and the runtime object has no `kind` to disagree with. Pre-existing
  before §4.5.512, measured on its review.
- The frame/inline ROUTE is decided per `(module, name)` key, so one package function is `inlined`
  in the module that imports it (`import p::mix; mix(x)`) and `frame` in the module that spells it
  `p::mix(x)` — value-correct in all three tools, and since §4.5.512 visible as two static rows that
  share one declaration triple with different routes. Whether the scoped spelling should inline like
  the imported one is unmeasured (the inline eligibility is read from a different table per spelling).
- Per-CALL-SITE builtin rows (`{"name":"$sscanf","file":…,"line":…}`) are not emitted; the `builtins`
  table is name-level aggregation.
- `--hier-tree` and `--inst-paths` are parsed for every applet but reach `VitaOpts` only on the
  one-shot path and are not in `reject_obs_dir`, so a staged invocation exits 0, prints no
  diagnostic and writes no file. This is the one accept-and-drop on the rail; every other obs flag
  is loud on a staged applet.
- `--hier-tree` collapses generate scopes (§2 🆕 N).
- doc-19 §3 pin 4 asks for enum values rendered as NAMES; there is no name path — `trace` values are
  4-state binary and `stage` values are `%0d` decimal.
- R-I1 (config-driven signal introspection: an auto-named JSONL dump with no hand-written bind) is
  partial — `--probe` / `--probe-file` is a manual path list. R-I2 (a semantic transaction log) has
  no producer.
- The `--obs-procs` `builtins` table counts primitives the source never wrote: an inline bind of a
  real call into an integral formal adds a `real->int` row and a 2-state body-local a `2-state` row
  (§4.5.526), beside the existing `$unsigned` / `$signed` / `$floor` / `$rtoi` rows that inline
  seals and real conversions desugar into — one row set that mixes user calls with lowering
  artifacts (measured: a design calling only `$display`, `$finish` and `$rtoi` reports `$unsigned`
  4, `$signed` 1, `2-state` 1, `real->int` 1).
- run.json's `wprog` object (§4.5.513) is the per-expression tally of why an expression left the
  compiled lane. Six of its keys (`array_whole`, `index_unknown`, `truncation`, `net_width`,
  `replicate_count`, `malformed`) have no source producer at HEAD; a sized-literal array index
  (`mem[3'd7]`) takes the runtime-index lane and is not a decline, so `index_range` counts the
  plain-constant spelling only.

Non-goals: FSDB/UCDB, an embedded SQLite, a waveform GUI, UVM integration. VCD stays the
human-facing format.

## 7. Conditional / long-term (promoted only when the re-entry trigger fires; orthogonal to correctness)

| id | item | trigger |
|---|---|---|
| BACKEND | ① a separate 2-state mode (rejected) ② PDES BSP parallelism (Amdahl ceiling T4 ≈ 2.5×) ③ the native-eval residual lane (signed >64, >128 bits, system functions, real) ④ an in-process JIT via cranelift-jit (rejected) | ② sustained W≥64 with grain ≥200 ns ③ low ROI, deferred standing ④ the reasons and re-entry are the §5.a codegen row |
| VHDL | a VHDL front end (9-value std_logic mapping, a separate parser, GHDL as oracle, E7xxx codes) | an SV plateau plus a value-domain decision plus a GHDL setup |
| VCD-EXT | `$dumpports*` (port strength) | waveform-tool demand (FST is supported — `$dumpfile("x.fst")` / `-o x.fst`; a known-edge small time table is a loud refusal from fst-writer issue #4) |
| MVP-CUT | string concat outside an assignment · a wildcard associative index `[*]` · the package internal-import and scoped-call residue · a cross-frame `disable` | individual demand |

## 8. Non-goals (permanent, not gaps)

- IMPLICIT-NET (policy: an explicit `E3010` error) · out of scope: synthesis, a waveform GUI,
  UPF/SDF/DPI-C, `shortreal`, `trireg`, the UVM ecosystem, and unique/priority multiple-match
  checking.
- `defparam` is not extended: a direct-child `instance.param` target with a constant value works, and
  a multi-level path or a non-constant value is a loud refusal. IEEE deprecates it and
  `#(.param())` covers the need, so the refusals stay (the residues that ARE tracked are §0 row 14-b
  and §3.b `defparam-iface`).

## 9. History

Completed slices, phase execution records, review records and the per-shot narrative are in
[history/](history/README.md).
