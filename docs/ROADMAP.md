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

| order | § | track | open items | oracle | band |
|---:|---|---|---|---|---|
| 1 | §2 | silent-wrong (correctness) | 28 rows + the mechanism lists | 2-oracle unless the row says otherwise | ① |
| 2 | §3 | loud → correct-support | 24 numbered + 66 small + 12 intentional | mostly present | ② |
| 3 | §6 | G2 observability (OBS) | 6 stages | internal 3-way differential | ④ |
| 4 | §5 | performance and hardening | 18 residues | measured | below the ladder |
| — | §0 | correct-support promotion queue | 14 rows | mixed | ③ |
| — | §4 | SVA honest-loud | 6 | mostly none; hand-IEEE when started | ③ |
| — | §7 | conditional / long-term | 4 | — | trigger-gated |
| — | §8 | non-goals | 1 | — | permanent |

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

## 2. Silent-wrong residues

Row identifiers are cited from tests (`§2 row 7/14/21/25/27/33`, `§2 🆕 I/L/M/N`); a number is never
reused. Resolved rows are listed in [history/ROADMAP_ARCHIVE.md](history/ROADMAP_ARCHIVE.md).

WALL(provenance) — rows 14, 15, 16, 25, 26, 30 and 🆕 F stop in one place: `const_wide.rs`'s
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
| 5 | LOUD | `s = string'(24'h610062);` → iverilog `len=2` and `s=="ab"`, vita E2002 | parser | belongs to §3, not §2 · the NUL-stripping shape does not parse in vita, so that axis needs another repro |
| 7 | BLOCKED (one ordering key) | a parent `initial` READING a child net at t0 sees X: `initial s = 8'hEE` in a child, read as `r = u1.s;` → oracles `ee`, vita `xx` (2-oracle). A fork arm in the child is a 3-way SPLIT (iverilog `ee` / verilator `00` / vita `xx`). An output PORT bind `child u1(.o(w))` and `assign w = u1.s;` both read `ee` at the parent's t0 immediate read, as do a constant driver, a parameter driver and a two-level chain — the only diverging spelling is the one whose value comes from the child's `initial`, i.e. process order; verilator answers `ee` there only because it constant-hoists a SINGLE-statement `initial s = <const>;`, and a second statement in that initial moves verilator to vita's answer, so that cell is an ORACLE SPLIT on order. `final` blocks run in ProcId order (`final_procs` is a `BTreeSet<ProcId>`), so any process reordering leaves vita disagreeing with itself | the rank machinery is complete (`with_rank_scope`, `init_ranks`, `RANK_MOD_INSTANCE(1) < RANK_MOD_OWN(2)`) and is applied only to declaration-initialiser processes. The root is that vita has ONE ordering key (`Activity.tie`) where the oracles use a DIFFERENT order per RESUMPTION KIND | BLOCKED BY: a per-resumption-kind ordering model. A single `proc_order` permutation seeded into the ordering key cannot express it — iverilog answers the kinds differently in ONE run of ONE design (`initial` child-first, `always_comb` t0 PARENT-first, edge PARENT-first, `#d` delay child-first, `wait` PARENT-first, fork-arm wake PARENT-first): keying only the t0 arm breaks the delay wheel, keying `Activity.tie` breaks the edge and `wait` wakes (`aa` for `cc`, a VALUE), and re-keying those two breaks `always_comb`'s t0 arm and the fork-arm wake, with both oracles against vita every time. Closes the headline plus 5 more 2-oracle cells. Costs a format bump (`proc_order` on the `StagedExtraSidecars` tail; `sim_ir::Process` is untouched) · corpus demand zero |
| 10 | OPEN | the RANGE BOUND consumer: `wire [K[31:24]-1:0] n;` on a `parameter [135:8] K` 128 bits wide is ONE BIT at exit 0 where both oracles declare 221, and the ≤64-bit twin of the same text is correct. Bare `K[31:24]`, `pk::K[31:24]`, `K[31 -: 8]`, `K[24]` and a `$bits`-sized net are all three-way identical | `const_range_bound_fold` has no wide-bit-domain fallback — the i64 select fold declines a >64-bit base (`select_base_at_declared` returns `None` for `dwidth > 64`) and both of the bound's fallbacks are i64. `$clog2` of the SAME text answers 8, so the value exists one funnel over | BLOCKED BY: `selfdet_bits_unsigned` declines the select too (only `selfdet_clog2_wide` answers it), so routing the bound at the wide domain buys nothing until that resolver reads it — an unguarded fallback moves 0 of 18 cells |
| 14 | WALL (provenance) | `localparam logic signed [7:0] NM = -8'sd2; localparam logic [63:0] X = NM ^ 64'h0;` is `fffffffffffffffe` against the oracles' `00000000000000fe`; the same expression over a FUNCTION LOCAL folds `00…fe`. The same routing also fixes `localparam logic [7:0] M = (P + 8'd100) % 8'd7` (P=200: 6 against 2), `pk::PA ^ 64'h0`, `int S = (D - C) / 2` (2147483641), the generate-scope `time NM` shadow (2c for 12c), and the 65-bit-leaf `/` and `%` | a module-scope initializer folds through the width-UNLIMITED `const_eval_in_scope` | route a DECLARED-width/sign target through `eval_const_assign`; the gate must demand provenance of every LEAF (`param_meta` is a DEFAULT for an untyped parameter and ABSENT for a `time` one), decline above the i64 lane, refuse an unsized FILL operand, answer THREE-valued in the scope walk, and NOT route the PACKAGE binder (row 26). BLOCKED BY: a width-aware walk correct on its own terms — the §11.4.10 shift count (`16'hFF01 << 3'b101` is 0; 30 correct→wrong plus 36 loud→wrong, reachable through a constant FUNCTION, so it is its own row and closes first) and an i64 bound that is not on the TARGET only (83 cells) |
| 15 | BLOCKED (2-state field) | an OVERRIDE carrying a sized x/z literal loses the unknown plane: `#(.K(8'b1010_010x))` onto `parameter logic [7:0] K` binds `10100100` at exit 0 where the oracles keep the x; `8'bzzzzz1z0` binds `11111110`. Five cells, every channel | `params.rs`'s i64-lane test reads only VALUE bits (`bp_get(..).0`); the sibling `fill` arm declines with `fill_is_unknown` | a `bp_any_unknown` test alone turns 76 CORRECT cells loud, because a 2-STATE declaration converts x and z to 0. BLOCKED BY: recording the parameter's 2-state-ness (`hdl-parser/src/params.rs` computes `var_kind` and drops it; an `hdl-ast` field plus a SchemaHash re-pin, parser-only), which also closes z→0 (1 cell today) · the separate headline (an unknown plane in the narrow store, 22 loud cells, ~40 sites, demand 0) stacks on row 14; above bit 64 what survives is z, not x |
| 16 | ORACLE-SPLIT | 12 override cells: 5 diverge from iverilog, but verilator sides with VITA on 4 (`-64'd1`, `<ones> + 64'd1`, `<ones> << 4`) and `~32'd0` is a 3-way split | that is row 17 | a fix would "correct" one side of a live split, so the context is not threaded through `override_bits` · the only 2-oracle sub-case is operands already ≥ the target width: `#(.K(~128'd0))` is vita `00…00ffffffffffffffff` against all-ones in both oracles |
| 17 | ORACLE-SPLIT | `leaf #(.K(32'd0 - 32'd1))` on `parameter logic [127:0] K`: iverilog `ffff…ffff`, verilator `0000…0000ffffffff`, vita `0000000000000000ffffffff_ffffffff` (zero-extending from 64, the i64 lane's width) | — | do not chase: vita matches NEITHER and §6.20.2 does not settle it · the neighbours are not split (`64'hFFFF_FFFF_FFFF_FFFF + 64'd0` zero-extends in all three, `-(64'sd1)` sign-extends in all three) |
| 19 | PERF | a 2-D, 3-D or packed element as a continuous-assign LHS costs ~10× on BOTH backends; against 50.0 ns for a 1-D unpacked element: 2-D `arr[0:15][0:3]` 546.7 ns native / 675.8 vm, 3-D 829.2 / 967.5, packed `logic [63:0][31:0]` 410.8 / 441.7 | not located; the native/vm ratio is 0.81–0.93, so it is SHARED PLUMBING | needs its own census; the diagnoses offered so far were refuted on the 1-D axis |
| 23 | LOUD | `clocking cb; input a_b;` beside `clocking cb_a; input b;` is legal (verilator `R1=17 R2=34`) and vita refuses it with ``net/variable `top.__clk_cb_a_b` redeclared`` at exit 1 | the `__clk_` / `__clkout_` mangling in `sva_clocking.rs:727` and `:657` | correct→loud with verilator as the accept/reject oracle, so it belongs to §3; a new sigil must be taught to the VCD and FST filters · the naming half is a one-token fix (`:745` computes the instance-qualified `alias`, `:750` re-formats it without `fq`; the `[in …]` suffix is `lvalue.rs:179`) |
| 24 | DO-NOT-START (see row 34) | 24a CLOBBER (silent-wrong, exit 0, verilator oracle): a signal merely DECLARED as a clocking output is destroyed to `x` or frozen — `vita x,171,171,171,171,171` against `verilator 170,171,172,173,174,175`; 4 cells, one across a module boundary · 24b a one-cycle LAG, 4 cells | 24a `init_diag.rs::clocking_commit_plan` (~1202), OUTPUT phase unconditional; the INPUT phase is correct · 24b the §14.16 skew `#0` in Re-NBA, a scheduler-REGION question with no anchor | 24a = a written flag produced at the write site; `out_pairs` grows a third field riding `SimOpts` out-of-band (no format bump) · corpus demand 0 |
| 25 | WALL (provenance) | `parameter P = 5` with `#(.P(32'hF0F0F0F0))` binds a SIGNED 32-bit −252645136 (`P < 0` is 1, `%0d` negative); `parameter Q = 8'sd1` with `#(.Q(32'hDEADBEEF))` is `ef` with `$bits(Q)` 8 — the oracles bind the override's own type (§6.20.2) | `params.rs::param_decl_width_opt`'s literal arm answers the DEFAULT's literal type even when `default_binds == false`; `ResolvedOverride` carries `signed` but no `width`. The operator twin of the same wall: `parameter HE = ~8'h5A` with `#(.HE(~4'h5))` binds 32 bits where verilator binds 4 — the default lane sizes by the operator and the override lane does not | a producer patch is kept out of tree (`scratchpad/r29/row25/producer.patch`, 317 lines and 5 tests; it fixes 7,982 of 122,774 cells and serv's `\|WITH_CSR`) but each shape of it opens a NEW correct→silent edge (`defparam` with a NAME rhs, a `time` parameter with a DECIMAL default, `$signed(64'h…)` resized down); the size-cast slice ships `param_type_guessed`, which DECLINES every guessed type. BLOCKED BY: a parent-side resolver that DECLINES on meta-less names and answers `$signed`/`$unsigned` by operand, carried through every channel including `defparam`; a fill onto an untyped parameter binds `(1, false)` |
| 26 | WALL (provenance) | routing the PACKAGE binder through the width-aware fold is a net loss: over 8,748 package-consumer designs it is 1,233 correct→silent-wrong against 714 fixed, plus one correct→loud | it makes `pk::X`'s stored value canonical (i64 −2 for `logic signed [7:0] PA = 8'hFE`) while every consumer still folds through the width-unlimited walk and sign-extends it | the prerequisite is on the CONSUMER side: `every_name_has_a_declared_width` does not enumerate `ExprKind::PkgScoped`, and an imported constant is in no provenance set — closing both turns those 1,233 cells correct · until then the identical text answers `00…fe` in a module and `ff…fe` in a package |
| 30 | WALL (§11.8.1 sign) | `localparam logic [127:0] C = '1 ^ 1'b0;` is vita `…00000000ffffffff` against 128 ones in both oracles; the same text as `r = …`, `assign c = …` and through a port prints 128 ones. 165 of 264 cells (22 operator forms × {32,33,64,65,96,128} × {`logic`,`logic signed`}), 0 splits, all four binder copies. The band is a property of the operands, not the width: `logic [7:0] A = '1 >> 2` is `ff` against the oracles' `3f`, 82 more cells at widths 1..31 | the wide fold's fill arm plus node-local region sign | the 4-piece shape measures 778 FIXED / 0 new-silent / 0 new-loud over 1,622 cells (`fold_bits_at`'s fill arm folding at `ctx` when `ctx>0` and not x/z; `param_i64_fill_at_declared` ahead of the i64 walk in all four binders; a LOCAL predicate with a `Cast` arm; a `fill_width_survived_the_fold` guard) but it does not stand alone: §11.6.1 evaluates at `max(ctx, every self-determined operand's width)`, so freezing the fill at the LEAF is wrong (150 cells); the ROUTING predicate cannot serve as the guard (104 NEW-LOUD, since a decline at `param_bits_at_declared` is `E3009`); and of 504 loud→value cells 215 are silently wrong on the SIGN axis (`localparam logic [7:0] B = ($signed(4'hF)+1) \| 8'h00;` is `00` against the oracles' `10`, no fill anywhere). BLOCKED BY: §11.8.1 region sign in the wide fold. ACCEPT set = "correct a value, never create one" |
| 🆕 F | WALL (§11.8.1 sign) | a narrow SIGNED operand is ZERO-extended in an unsigned context: `localparam logic [7:0] A = 8'hFF - (-1'sb1);` is `fe` and `8'hF0 \| (-1'sb1)` is `f1` against the oracles' `00` and `ff`. 24 cells at widths 2..32, both sign declarations, no fill involved | §11.8.2 reinterprets each operand at the EXPRESSION's sign; `const_wide.rs`'s bitwise and arithmetic arms compute `cs = ls && rs` then `resize_bits(.., cs)` for BOTH | the same root as row 30's prerequisite — file the fix once |
| 🆕 H | BLOCKED (§11.8.1 wall) | ⓐ `(&4'b110x)` and `(\|4'b101x)` clamp to one bit in a bound where both oracles answer 3 or 4; 8 cells (`^` and `~^` are split). The class is wider than reduction — `~&`, `~\|`, `===` and `&&` with a 0 operand are IEEE-definite and all silently become one bit (both oracles 4, 3, 4, 3) · ⓑ the `\|P` bound of an ascending `parameter [0:3] P` or a lo≠0 `parameter [7:4] P` is loud (both oracles 4) · ⓒ `localparam E = 4'hF \| 4'h0; wire [(&E)+2:0]` is loud (both oracles 4) · ⓓ `localparam R = ~(\|4'b1010);` is deliberately loud (both oracles `0`, `$bits` 1) · ⓔ a reduction after a local assignment in a constant-function body (`t = a[5:0]; return (\|t)+2;`) is loud (oracle 3) | ⓐ `fold_self_bits`'s reduction arm declines on a single unknown (`bp_any_unknown`) · ⓑ `narrow_param_bits` rejects `lo != 0 \|\| ascending` · ⓔ the body local's width is not in `envw` | ⓐ the fix site is the wide fold's ACCEPT SET, so it falls under the §5.2 do-not-start line · ⓑ a reduction-only, layout-independent resolver · ⓒ WALL(provenance) |
| 🆕 I | OPEN | ⓐ another process's read in the same delta: `initial #1 v = 8'hA5;` declared before `initial #1 $display(c);` — oracles `a5`, vita `00`. Held on purpose · ⓒ residue on the same axis, also held: a PROCEDURAL index, a DELAYED constant driver and a gate-driven one; `assign c = r + 8'd0;` with no array at all reproduces them · ⓔ `bit [7:0] c; assign c = v;` is excluded and unexercised (vita refuses `bit` copy destinations, `E-ELAB-LVALUE-KIND`) · ⓖ a callee body is ONE set of expressions, so a root is marked only when EVERY calling process writes it (3 of 4 cells); a `logic [15:0]` destination taking a sign-extending copy of a `signed [7:0]` source is `xxxx` in vita where BOTH oracles print `ffa5` (pre-existing 2-oracle). Recorded splits, not chased: the RUNTIME index `m[k]` (iverilog reads `xx` exactly like vita and only verilator reads `a5`; iverilog answers stale on an INDEX change and fresh on a SOURCE change in the same design); an UNSIGNED narrow net index into a NEGATIVE-base array (iverilog's answer depends on the ARRAY'S SIZE for a fixed index pattern and fixed `lo` — threshold 4 for `m[-2:1]`, 5 for `m[-6:1]` and for `m[-2:9]` — which no reading of §7.4.6 licenses, and verilator has no `x` for an out-of-range unpacked read at all, so its `a5` is masking; `array_word_index_domain.rs` records this, the SIGNED declaration is correct at HEAD, and vita's own `a5` at width ≥32 is the inconsistent half, a 32-bit `Add` in `dim_coord` overflowing to coordinate 0); a 2-D array word `reg [7:0] m[0:1][0:1]; assign c = m[0][1]` (iverilog `a5` / verilator `00`); `m[1][2]`; `m[32'hFFFFFFFE]` and `m[64'd0 - 64'd2]` (vita E4002 / W4029); a GENVAR index `assign cw[g] = m[g-2]` (iverilog `a5 5a`); an all-`z` driver beside a partial or delayed driver (E3001); a `force`d copy after `release`; an array-WORD-target copy `assign c[0] = v[0]` (iverilog `a5` / verilator `00`); a zero-extending, truncating or concat copy; a partial slice `v[3:0]` (iverilog `x` / verilator `5`); `v[7 -: 8]` (iverilog `0` / verilator `4294967295`, vita = verilator); an `always_comb` whose only read is inside a called task (vita and verilator `a5`, iverilog `xx` with "no sensitivities") | ⓐ a §5.4.1 race kept on the settle's value ON PURPOSE (a store-side forward breaks picorv32, UDP and keccak parity) · ⓕ the interpreter and VM take the extension sign from the slot (255), the native path from the node · ⓖ callee reads are not in the sensitivity derivation | ⓖ the copy's declared sign is re-stamped on the aliased read in `eval_core` and `read_scalar_words`; a mismatch ANYWHERE in a chain disables the tail below it |
| 🆕 J | LOUD | ⓓ `{'1, 1'b0}` is illegal (§11.4.12) and the oracles are lenient (2) where vita is loud; keep · ⓔ `v['1]` is split (iverilog 0, verilator 1) and vita follows iverilog · ⓕ the widths of `'1 * 2'd2`, `'1 + 1'b1` and `4'd8 - '1` are verilator 2/1/4 and iverilog 3/2/5 with values agreeing; vita follows verilator · ⓖ `localparam U = '1; localparam Y = U + 4'd1;` gives `$bits(Y)` 32 against the oracles' 4/5 | ⓖ is fill-INDEPENDENT (`localparam U = 1;` shows the same 32) — it is the row-14 value-inferred tail (`min_signed_bits(v).max(32)`) | ⓐ (a fill as LEFT operand under a TYPED declaration, 11 cells) is row 30 · ⓖ WALL(provenance) |
| 🆕 M | LOUD | ⓐ `m #(.P('1 ^ 1'b0)) u();` onto `parameter logic [39:0] P` is 32 bits in vita (`00ffffffff`), target-sized in iverilog (`ffffffffff`) and ONE bit in verilator (`0000000001`); 40 pre-existing plus 9 split cells · ⓑ `(\|'1)` and `{('1 ^ 1'b0)}` stay loud in a constant · ⓔ `cover property (… (a \|-> b))` is loud (`cover.rs` has its own sequence-only grammar) · ⓕ verilator prints NO failure for `a \|-> b and b \|-> a` where §16.12.8 fails at the first failing operand (vita t=35), so it is not an oracle for property-level `and` | ⓐ the parent folds the override before the target's width is known (`resolve_param_overrides` → `ovr_by_name`) | ⓐ BLOCKED BY: a target-typed override evaluation (row 17's axis) · residue: a select of an ascending or value-sized hierarchical parameter stays loud; `$bits` of a hierarchical string is loud (split 1/16); an override CARRYING past the operands' top bit (`~`, `+`, `<<`, unary minus, `?:`) stays loud; a decimal or `-(64'sd1)` override of an untyped 128-bit-default parameter keeps 128 bits (row 25's i64 half) |
| 🆕 N | OPEN | VCD `$scope` names a generate block `gi[0]` / `genblk1[0]` where iverilog writes `begin gi` / `begin genblk1`; a task declared in an unnamed block is a split (iverilog `top.genblk1.t`, verilator `top.genblk1.genblk1.t`, vita loud); a user block named `genblk1` beside an implicit one is not disambiguated (iverilog `genblk01`, verilator refuses); `%m` in a CONCURRENT `assert property` action block omits the assertion label (`top.nb` against verilator's `top.nb.ap`; iverilog refuses concurrent assertions, so this is 1-oracle) — the label dies in the parser, since `hdl_ast::Stmt::ConcurrentAssert` has no `label` field and adding one to that frozen SchemaHash type flips the root hash (an IMMEDIATE labelled assert already carries its label, which is verilator's side of a live split); `gi[0].x` on a conditional scope is accepted where the oracles reject it; a class method called from a generate-block process or a frame names its INSTANCE (a label inside the method is kept, matching verilator; iverilog drops it, a split); a `$unit` class prints `top.C.show` (split); a package class, and a class method calling a module task or `$strobe` in a class, are loud; a parameterized class prints `C__8` against verilator's `C__N8`; an ELABORATE-time diagnostic inside a class spells `[in $class$C$m]`; a package function is `top.pf` against iverilog `p::pf` and verilator `p.pf` (split); `--hier-tree` and `--inst-paths` list no generate scopes | the class table is global and its declaring INSTANCE is unknown, so the CALLING scope is prefixed | beside it (loud): an instance ARRAY of a PORTLESS module (`ch w[1:0]()`) is refused with "child has non-ANSI ports", a false reason on a valid design |
| 🆕 O | OPEN | `lookup_net_scoped` — the `symbols`-only walk — has ~90 callers, and any reader that resolves a bare name without asking `bare_ident_route` is this class. Recorded splits, unmoved: an ENUM LABEL shadowing an outer array reads the array in vita and iverilog and the label in verilator; `foreach` over a shadowed name is `i=0` in vita and iverilog and `i=31` in verilator | the guarded readers ask `bare_ident_route` the way the lowering does (`bare_name_binds_constant`) | a new instance is the same one-line guard; the splits are recorded, not chased · 2-oracle for the class, and the two recorded cases above are oracle splits |
| 🆕 Q | BLOCKED | a `localparam` declared in a procedural block is a parse error (`E-PARSE-UNEXPECTED-TOKEN: expected statement, found keyword 'localparam'`, plus a cascade of follow-on "expected statement" errors) where both oracles accept it. The class is wider than a plain named `begin : g`: an UNNAMED block, an `always_comb`, a subroutine body, the `parameter` spelling (§6.20.1 makes it a localparam) and use as a RANGE BOUND of a later block-local declaration (`localparam W = 7; logic [W-1:0] v;` — both oracles `v=127`) are the same refusal, 7 of 7 two-oracle | there is no BLOCK-SCOPED CONSTANT binding in the IR. The parser's `const_locals` is a parse-time i64 fold table read only by `try_const_index`, and the elaborator's `$blk$<span.lo>` scoping is for block-local NETS, which are not constants | BLOCKED BY: a block-scoped constant binding. A bare-name HOIST of the declaration into the enclosing container's item queue makes 6 cells correct and 5 NEW silent-wrongs, because the hoisted name has no scope: an outer literal localparam, an outer non-literal one and an outer HEADER parameter each read the block's value after the block (`out=7`, oracles `3`); two sibling blocks declaring the same name collapse to the LAST value (`a=9 b=9`, oracles `a=7 b=9`); a read AFTER the block answers 7 where both oracles reject the name. Only the outer-NET cell is loud. A parser-side rename is not cheaper — there is no single `ExprKind::Ident` funnel (52 construction sites) · 2-oracle |
| 🆕 L | LOUD | ⓑ `$bits` of a string parameter is 16 in vita and iverilog (§6.16) and 64 in verilator: vita follows the LRM, keep · ⓒ `localparam real Q = 1.5; localparam W = Q * 2;` is E3009 against the oracles' 3.0 · ⓓ a 2-state struct's `'{…}` in a constant is loud (`w'(longint'(e))` has no const-fold arm); a 4-state struct's folds · ⓔ a fill inside a `'{…}` in a constant is loud · ⓕ a string or `real` package parameter through `import p::*` is E3010 / E3009 (the scoped `p::S` works); `$bits` of a WILDCARD-IMPORTED real parameter is 32 against verilator's 64, the same name-lookup family · ⓖ `m #(.X('{1'b0, 5'd7}))` of a struct-typed header parameter is E3009 against verilator's 21 · ⓗ `p::v.a` is E2002 (the struct desugar keys on the bare first segment) · ⓙ `gather_local_decl_names` omits functions, tasks, genvars, instance names, typedef names, array parameters and generate-block contents, and a package importing a package passes an EMPTY set · ⓚ an `import` inside a generate BLOCK is loud E3009 unless redundant; per-scope application is the remaining work · ⓛ `union packed` containing an anonymous `struct packed` fails to parse, and `import` or `localparam` in a function body is loud · ⓠ `logic [1:0] i; F[i*4+3:i*4]` gives `0000` and `F[i*4 +: 0]` gives `0` in silence (verilator refuses; illegal SV) · ⓡ a block-local variable shadowing a wildcard-imported package VARIABLE is read as the local AFTER its block (`SX` → `11`, oracles `a5`) · ⓢ a header parameter redeclared in the body answers the body declaration · ⓣ `c #(.A(x), .A(y)) u();` is accepted, last wins · ⓤ an x/z WRITE into a 2-state member of a 4-state packed struct keeps the x/z — `o.q = 4'bx1z0;` is vita `xx1z0x` against iverilog `x0100x` (§7.2.1); the parser knows the member is 2-state (`StructFieldLayout.5`) and the write path does not squash · ⓦ loud residue: a package function whose body reads a package constant outside the i64 interpreter (real, string, array, enum); an INTERFACE importing a package function into a range bound (`apply_import_const_funcs` is wired for modules and packages only; both oracles 8); an `import` inside a generate block (unapplied, loud twice) · ⓧ a compilation-unit `import` after the module still applies, and a package constant used before its declaration folds (iverilog rejects both) · ⓨ the declared-range gate's span dedup reports a generate loop's bad bound once where iverilog reports it three times · ⓩ three separate roots on the negative axis: a negative select BOUND (`A[0:-2]`) false-louds because `const_bound_u32` folds it unsigned (`-2` reads `0xFFFF_FFFE`) and then trips the direction check, with the message naming the wrong fact (both oracles `p=7`); a >64-bit negative-LSB base is still positional (`logic [67:-4] A; A[7:0]` is `34` against both oracles' `33` — `const_wide.rs` reads `param_range` directly and `select_base_at_declared` refuses `dwidth > 64`); an explicit `[m:l]` PART select of a net or variable declared with a negative low bound is loud by its own gate (`packed.rs`; both oracles answer, and the message understates itself, since both INDEXED spellings and every bit-select already work on that net) · loud beside it: runtime `$size(P)` of a scalar parameter, a multi-packed ELEMENT array parameter, a >64-bit base's select (`logic [191:64] A; A[127:64]`, both oracles fold), and a whole-NAME read of a shifted or ascending parameter in a >64-bit concatenation (`logic [11:4] A; logic [79:0] L = {A, 72'h0}`; the zero-LSB twin folds, and `narrow_param_bits` declines the NAME, which is what keeps the structural select arm sound) · (aa) 324-cell residue: an UNTYPED `localparam G = C + D` stays i64 (iverilog 16 / verilator 0, split); enum labels are the same split (vita = verilator); `$clog2(C+D)` in an untyped declaration folds the 32-bit sum where the oracles fold the 4-bit 0; a `byte` operand in a bound (`[Y+Y:0]`, `Y = 100`) reads the LRM and iverilog value 57 where verilator reads 201 | ⓩ three roots: the unsigned select-bound fold, the wide lane's own `param_range` read, and the net lane's part-select gate | ⓩ each residue is its own slice, and demand is ZERO (no negative packed declaration in the 1,462 corpus RTL files), so they rank below any cell the corpus exercises · (aa) the elaborate VALUE lane is row 14's wall · also recorded: the `endpackage` export of `packed_md_params` has no `local_decl_names` filter, and `foreach` over a multi-dim packed formal is loud with a misleading enum-method diagnostic |
| 31 | PERF | all 126 pure-family cells are three-way correct in all nine positions; `assign p1 = $signed(a)*$signed(b)` measures 603 evaluations against 201 for `a*b` (3.0×), and `a >> $clog2(8)` the same; demand is 22 continuous assigns / 29 occurrences in ibex and verilog-ethernet | — | do not start as a §2 item; it is filed at §5.2 rank 4 · the STATE half is do-not-chase: of 32 wrong cells only 6 are arbitrable (26 have both oracles constant but disagreeing, iverilog `z` against verilator `0`; 4 split on constancy) and declining makes the one reachable hazard WORSE (2,406,546 settle spins) |
| 32 | LOUD | real, 6 cells, but the run ends with `F4004` and exit 1; the residue is one extra `$display` line | `frame_eval.rs::run_frame_call_with`, one funnel | belongs to the §3 tail, ~10–20 product lines · vita's own TASK path already bails without committing the caller's lvalue and matches iverilog; verilator is disqualified because it prints after a TOP-LEVEL `$finish` too |
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
  directly makes `refuse_real_size_operand` turn `int'(r)` loud. WALL(AST self-width).
- A cast's context width stops at an inner self-determined node (both oracles agree):
  `64'(-16'(u16))` is `000000000000fffb` against `fffffffffffffffb`, `8'(s4 * 4'(s8))` is `…f9`
  against `…09`, `16'(s8 + 4'(u8))` is `000c` against `010c`; un-nested `64'(-u16)` is correct, so
  the trigger is a nested cast or a `$signed`/`$unsigned` node. 143 of 10,368 cells.
- A widening cast cannot take an impure operand's sign correction: `extend_to`'s sign fill names the
  operand twice, so `16'(f())` and `int'(f())` keep the unsigned answer (oracles `fffd` /
  `fffffffd`). Fix = a 4-state-preserving extension that names it once, or a callee-purity predicate.
- The extension sign of a cast or inline comes from the mirror, so a signed class field cannot
  supply it even though it is pure and repeatable: `function signed [63:0] fw; fw = c.sf;` with
  `8'hAB` gives `00…ab` against hand-IEEE `ff…ab`.
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
- A 64-bit constant's unsigned value reads negative in the i64 domain (both oracles agree):
  `localparam L = (64'hFFFFFFFF00000000 > 0) ? 111 : 222;` is vita 222 against 111 in both oracles
  (the `parameter [63:0] BIG` spelling behaves the same). Root = `const_eval_i64_lit`'s 64-bit
  reinterpretation arm; the range check is not present at the comparison site. Closing it also opens
  `a_placement_that_does_not_fit_the_i64_domain_declines`.
- A comparison at exactly width 64 is silently wrong (both oracles agree):
  `((64'd1 - 64'd2) > 64'd0)` is 1 in both oracles and 0 in vita — an off-by-one in
  `masking = ctx_w > 0 && ctx_w < 64`.
- A module-scope `localparam`'s `/`, `%` and `>>>` lose the sign even with a declared width:
  `% 64'd10` gives 18446744073709551615 (iverilog 5), `/ 64'd10` gives 0 (iverilog
  1844674407370955161), `>>> 4` gives 18446744073709551615 (iverilog 1152921504606846975); `>>` is
  exact. This is one item with row 14 — do not start it separately.
- Above 64 bits the decline is deliberate (only `w == 64` is unsigned): the two directions are wrong
  in opposite ways — `(64'hFFFF…FFFF + 65'd1) > 64'hFFFF…FFFF` wants the signed reading and
  `((65'd1-65'd2) > 65'd0)` wants the unsigned one (each oracle answers 1), so do not guess. Pin =
  `const_unsigned_at_sixty_four.rs::above_sixty_four_bits_keeps_the_pre_slice_answer`.
- A 64-bit unsigned `*` overflow is loud (`64'h8000…0000 * 64'd2` is 0 in both oracles and refused by
  vita's `checked_mul`). Wrap only when the context width is exactly 64.
- An untyped `localparam` with a huge `**` hangs iverilog, so there is no oracle:
  `localparam L = 3 ** (64'd0 - 64'd8);` runs 10 minutes at 100% CPU, leaving verilator as sole judge.
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
  element 255 against iverilog's `xx`, because the seal rejects a `Call` as not repeatable.
- ORACLE-SPLIT, do not chase: on a packed ELEMENT's `+:` overhang iverilog contradicts itself — in
  one design `pv[-2'sd1 +: 2]` is `1x` and `pm[1][-2'sd1 +: 2]`, holding the same bits, is `10`.
  verilator has no `x` for an out-of-range select at all (everything is `01`). vita is uniform `1x`
  across all four spellings and agrees with iverilog on the two spellings where iverilog agrees with
  itself. Pinned by self-consistency (`packed_select_signed_index.rs`).
- A >64-bit override tree (`~128'd0`) declines because `const_ctx_within_i64` refuses it — the value
  re-fold clamps at 64. Both oracles 128, vita 32.
- A `localparam` DERIVED from an overridden untyped parameter forwards at 32 (2-oracle): in
  `mid #(parameter Q = 8'd1)` overridden `#(.Q(4'd3))`, `localparam R = ~Q; leaf #(.P(R))` binds
  `32/c` where both oracles bind `4/c`, while `$bits(R)` inside `mid` is already 4 and `#(.P(~Q))`
  forwards correctly since §4.5.470. Root = `R` is not overridden, so `param_decl_range_opt(p, true)`
  reaches the operator arm, which declines under `declared_only` on purpose (the §4.5.363 263-bit
  net-provenance fence); `narrow_param_bits` then has no range to agree with. Width-only, on the
  localparam lane. Pinned at today's text in `param_override_forwarded_width.rs`.
- A `pkg::`-scoped name as the override SOURCE loses both columns (2-oracle): with
  `package pk; parameter logic [35:0] PW = 36'h8_0000_0001;`, `leaf #(.P(pk::PW))` binds
  `bits=32 val=1` where both oracles bind `36/800000001`; the wildcard-imported bare `PW` spelling
  is correct. Root = `wide_name_bits` / `narrow_param_bits` take a single-segment path and decline
  `ExprKind::PkgScoped`, so neither `ovr_bits` nor `ovr_self_meta` is produced and the value folds
  at the parent's 32-bit lane. §4.5.466 recorded the width half as "the next rung"; the value half
  is the same decline.
- Forwarding an UN-overridden untyped parameter whose default is NOT a literal (1-oracle,
  verilator; iverilog contradicts its own `$bits` on `+`): `mid #(parameter Q = 8'd1 + 8'd0)` with
  no override, `leaf #(.P(~Q))` binds `32/fffffffe` where verilator binds `8/fe` and iverilog 9.
  The default lane's literal arm answers only a literal; an operator default has no range entry and
  the forward falls to the leaf's own default. Recorded, not chased (one oracle).
- `const_expr_signed`'s `Ident` arm resolves with `self.fq()` (the current scope) and so diverges
  from `const_self_width` / `const_signed_env`'s `walk_scopes`: reading a module-scope
  `parameter signed [7:0] S8` inside `generate if(1) begin:gb` makes `localparam K = S8>>>1` 255,
  where the same text at module scope is −1 and both oracles are −1. The meta sign moved to
  `const_signed_env` and this arm did not inherit it.
- `parameter unsigned U = 1` overridden with a SIGNED value (`#(.U(-8'sd91))`) binds `-91` where
  both oracles bind `165`. The width axis is correct; only the sign column is open. Root =
  `ast::ParamDecl.signed` is `false` for both "the `unsigned` keyword" and "no keyword", a direction
  `sg || p.signed` cannot see. Fix = an `is_sign_declared: bool` on `hdl-ast`, which is a SchemaHash
  ROOT field and therefore a format bump. The same field closes the typedef-prefix item below.
- Observation only: two producers write `p.signed` from something that is NOT the parameter's own
  keyword — `hdl-parser/src/params.rs:313` (`signed = expl0.unwrap_or(info.signed)`, a typedef
  prefix) and `module_items.rs:740` (`signed: d.signed`, the NetVarDecl-shaped header entry with
  `ty: ParamType::Implicit`). The reachable typedef spellings are harmless in measurement (all four
  tools agree). The `is_sign_declared` field above closes this too.

### Inline / frame binds

- The inline path does not push the declared width into the body where the frame path does (oracles
  agree): `function [31:0] fh(input [7:0] x); fh = fld * x;` with `8'hFF` is `00000001` when static
  and `0000fe01` when `automatic`, which is iverilog's answer. `lower_ctx_or_plain(rhs, ctx_w)`
  sizes fills only.
- A frame argument bind does not apply the §11.6.1 extension sign (oracles agree): `8'shf7` becomes
  `000000f7` against iverilog's `0000fff7`. Three funnels share it (frame function, task, class
  method), so it is a CLASS — 24 of 1,920 cells. Net assignment and port connection are correct, so
  the site is the bind.
- `expr_is_repeatable` rejects an array element, so `f(mem[i])` cannot get the bind (oracles agree):
  `gs(arr[2])` is `00…f7` against iverilog's `ff…f7`. What is needed is not repeatability but
  side-effect-free duplication.
- A hierarchical reference or class-field actual cannot get the declared width: the fabricated 32
  makes `trusted_self_width` answer `None`, the bind stands down, and the result comes out at the
  actual's width (`hi.hv` into `gs={x,x}` is 8 bits). A generate-scope name behaves the same.
- `cast_operand_is_real`'s AST half sees only a bare single segment (oracles agree): `pa(f(0))` is 4
  (correct) while `pa(p::f(0))` sends an f64 payload into a 2-state formal (and so does `c.cm()`).
  Widening it touches 8 call sites.
- An inline body-local's 2-state declaration does not drop x/z (oracles agree): `bit [7:0] b; b = x;`
  is `x7` against iverilog's `07` — `fold_straight_line` has no 2-state step. Fix it with the bind,
  in one place.
- The inline bind's width decision trusts `ir_bits_of`'s fabricated width: a class field answers 32
  and inverts the truncate/extend decision — `i16(c.bu)` on an 8-bit field is `xxc3`. The window is
  `field width < formal width < 32`; the canonical answer is `canonical_self_width`.
- A `real` rhs skips §10.7 — `resize_inline_assign` has an `expr_is_real` early return
  (`f = r + x*x` is `013b` against iverilog's `3b`).
- Below the `!trusted_w` carve-out the bind still leaks (a deliberate trade): `fh = c.big + 1'b1;`
  on a 40-bit field is `00000000` or `0000010000000000` depending on the destination width.
- Beside it, deliberate: an actual wider than the formal is not truncated — `f(8'hFF)` is `ff`
  against iverilog's `0f`, and `{f(8'h02){1'b1}}` is 0 against `f`.
- The verbatim inline actual's mirror is wrong on its own path: an 8-bit signed frame-call actual
  into a 16-bit signed formal (`fs16_add(g(-16))` → `00f0`, oracles `fff0`), because
  `bind_formal_actual` widens by the actual's MIRROR sign (`Call ⇒ false`).
- There are nine binding sites, not four, and five are open (2-oracle; `f(300.0)` into an
  `input byte` gives 300 where the oracles give 44): ⓐ a frame function with an output formal;
  ⓑ a hierarchical task call (the argument is pre-lowered in `inline_task.rs` without the formal
  width); ⓒ a hierarchical function call; ⓓ a class method or task; ⓔ a class constructor. ⓑ and ⓒ
  are structurally different.
- `expr_is_repeatable`'s decline leaves a silent default (2-oracle): a user `Call` (`f(rfn(3))`), a
  real array or queue element, a non-whitelisted SysFunc (`$sqrt`, `$itor`, `$bitstoreal`), and
  `p::rf(...)`. Declining `$random` is correct.
- An explicit `signed` qualifier on a `time` declaration is discarded (2-oracle): with
  `input time signed k`, `k/2` is −4 in the oracles and 9223372036854775804 in vita, because
  `kind_signedness` hard-codes `time` to unsigned.
- An out-of-range real clamps wrongly on integer conversion: `real rv = 1.0e300; byte'(rv)` is 0 in
  both oracles and −1 in vita (the same for ±inf and NaN).
- `int'($random*1.0)` draws the wrong number of times (both values wrong, and the value changes):
  `lower_prim_cast` has no `expr_is_repeatable` gate, so it draws 4 times per cast against
  iverilog's 1.

### Real

- Using an `automatic` (framed) real function directly as an operand widens it (2-oracle):
  `fa(1) + (-s)` is −7 in both oracles and 9 in vita; beside it `{fa(1), 1'b0}` passes silently. The
  shared rule's `Call` arm does not reach that shape.
- Package and class functions have the same hole: `p::one() + (-s)` and `c.getr() + (-s)` are −7 in
  both oracles and 9 in vita.
- The remaining conversion boundaries are context-determined: `real r; r = (-s);` is −8.0 against
  8.0 and `r = (s+s)` is 0.0 against −16.0, so `Binary` and `Ternary` are closed and plain
  assignment is open.
- The body of a real-returning constant function belongs to §3, not §2:
  `localparam real R = f();` gives `E3009 … not a foldable constant expression` where iverilog gives
  0.000000 — honest-loud.
- `$realtobits` and `$bitstoreal` silently accept a non-64-bit argument (iverilog says "requires a
  64-bit argument"); vita answers with the low 64 bits.

### Ranges / bounds / selects

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
- A self-referential return range overflows the stack (no oracle — iverilog aborts too):
  `function [f():0] f();` — `const_fn_ret_wsign` does not carry call depth. Prescription = one line,
  `depth + 1`.

### Class fields

- `ir_bits_of` reads a class field's width from the handle net, where the real width is only in the
  `class_field_widths` sidecar, so it answers a wrong `Some(32)` (`16'(c.sb)` is `xxxd` against
  hand-IEEE `fffd`). CLASS; the canonical answer is `canonical_self_width`. Second symptom: when the
  cast width equals the fabricated 32, `Ordering::Equal` skips the resize and the cast disappears —
  `32'(c.s8)` is `fd` (should be `fffffffd`) and `32'(c.s8 + ua[0])` is `fa` (should be `000001fa`).
- An ascending negative bound is clamped only on a class property (one oracle — verilator 4 bits;
  iverilog dies on an assertion): `class C; logic [-3:0] q;` gives W3056 and exit 0 with a wrong
  value (row 3b). The cheaper half: an un-normalised class-field select is broken on `logic [7:1] q`
  (lsb ≠ 0) as well.
- A bit select of a packed dimension with a negative low bound cannot build a coordinate:
  `logic [-3:0][1:0] x; x[-3]` — `dim_coord`'s ascending arm does not build the signed subtraction
  of the correct coordinate `(lo+size-1) - idx` (loud on both builds). The whole value and `$bits`
  are correct.

### Scoping / imports / block-locals

- A block-local declaration clobbers an IMPORTED package variable of the same name (both oracles
  agree): after `import pk::*`, `begin : blk integer pv; pv = 99; end` makes `pk::pv` read 99 in vita
  and 5 in both oracles — the v1 model that flattens to a module net by bare name lands in the same
  slot as the import alias.
- A static shadow pair beside a DISJOINT `automatic` span of the same name still takes the old
  flatten (1-oracle: verilator `MOD=0`, vita `MOD=41`; iverilog rejects the lifetime override): a
  nested `int s` / `int s` pair in an `initial` plus `automatic int s` in a separate `always` makes
  `shadow_static_only` false, so the pair keeps the pre-§4.5.468 route and the outer write lands on
  the module net. No E3009 fires because the automatic span never crosses its block. Site =
  `compute_scoped_block_locals`'s per-name exemption; the fix is a per-SPAN exemption that must not
  mix a static and an automatic member inside ONE nesting pair (that mixing was measured to turn
  the loud `outer static / inner automatic` shape into a silent leak).
- Two same-named sibling block-locals inside a STATIC TASK FRAME body silently become one variable
  when they are assigned only by declaration initialisers: `o1=55 o2=55` against both oracles'
  `o1=44 o2=55`. A four-entry re-entry ladder shows the two variables are one counter
  (`a=56 b=57 c=58 d=59` against the oracles' `45 56 46 57`). Controls behave correctly: an
  `automatic` task, a function, and different names. The storage path is different
  (`reserve_frame_block_locals`) — the same class as §3.b `blocal-flatten` on the module procedural
  path, at a different site.
- Declaring a parameter and a net with the same name is accepted by vita alone (both oracles reject
  it): vita takes `localparam N = 7; logic [3:0] N;` and reads it as the parameter (`r=7`). This is
  a vita invention, so making it loud is not a step down the ladder; the shadow rule's `!params`
  clause is the only observable site, and the current behaviour is pinned
  (`block_local_shadows_param.rs`).
- A part-select WRITE into a queue or associative element vanishes silently (oracle: verilator;
  iverilog rejects the syntax): `q[0][15:8]=8'h0F;` gives verilator `ffff0fff` against vita's
  `ffffffff`; the dynamic (`q[]`) spelling is correct — a write-twin gap.
- A width-0 indexed part-select is accepted silently: `parameter P = 0; t[i +: P] = …` is rejected by
  iverilog and exits 0 in vita (§3 in character).

### Delays / events

- A runtime variable delay (2-oracle): `assign #(dv) y = a;` with dv=5 gives 5 in both oracles and 0
  in vita — the engine must evaluate at the suspension point. The partial fold of `#(D, dv)` has the
  same root (the rise folds and the fall takes the rise value). Pin =
  `structural_delay_scope_fold.rs::a_runtime_variable_delay_is_still_zero_delay_and_still_quiet`.
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
- A wire driven only by a continuous assign raises a false event at t=0 (1 oracle: iverilog;
  verilator not consulted, so a 3-oracle census is required): with `wire b; assign #5 b = a;`, b
  starts at z and the t=0 settle's z→x wakes `always @(b)` where iverilog has no t=0 event.
  `assign d = c ^ 1'b0;` behaves the same — an initial-value domain problem.

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
- `%h` prints a 1-bit unknown EXPRESSION result as `x` where iverilog prints `X`
  (`$display("%h", ^a)`); the 1-bit NET of the same value is `x` in both, so iverilog is
  inconsistent and IEEE §21.2.1.3 is on vita's side. 17 of 215 designs.

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
- `coerce_two_state` names its operand once per TARGET BIT and the engine walks that DAG as a tree:
  `byte'` 8, `int'` 32, `longint'` 64, `int'(int'(x))` 1024 against iverilog's 1. The discriminator
  is 2-state-ness, not width (`integer'` and `int'` differ by 27×). The `expr_may_be_unknown` guard
  in `lower_prim_cast` takes 1024 down to 32. Still wrong: `int'(f())` names `f` 32 times because a
  `Call` is conservatively unknown, and a WIDENING cast over a call fans out to the wider width, so
  it needs the `expr_is_repeatable` gate.
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
  located: `compile` is the only honest answer to "will `wprog` take this", which is `WPROG-WHY` in
  §5.b.
- Coercing a 4-state actual into a 2-state formal is O(declared width) at runtime (identical on all
  three backends: `byte` 12.8×, `shortint` 23.8×, `int` 46.4×). The real fix is an x/z→0 IR
  primitive (format bump) or engine memoisation; two mitigations are refuted with zero improvement
  (a per-query memo and a node budget), because the cost is in the number of binds and a persistent
  cache conflicts with in-place patching.
- Constant-domain width and sign resolution walks the tree three times: `eval_const_env_self` runs
  `const_self_width`, `const_signed_env` and then the evaluation where the i64 walk needs one. A
  `[W-1:0]` bound costs 2 extra `walk_scopes` — 20,000 declarations are 0.079 s without the bound
  and 0.115 s with it (+45%). Values are correct. Prescription ⓐ fuse the width and sign walks
  (half of it) and stop `walk_scopes` returning an owned `String` per lookup (the other half);
  ⓑ memoise the genvar-free sub-expressions of a generate-for bound. Front-end cost is invisible to
  the workload corpus (§5.b `ELAB-PHASE-BLIND`), so this needs a front-end-bound measurement of its
  own.
- A left-leaning `==?` / `!=?` chain is still 2^depth: depth 22 goes from 30 s to 79 s. Values are
  correct.

### Oracle splits (recorded, not chased)

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
| ⑤ⓕ | unpacked-array typedef residue, still loud: a function RETURN type of the typedef (1-oracle; iverilog does not merely refuse it, it SIGABRTs with `Assertion failed: (lwid == ivl_signal_width(lsig))`; it needs an unpacked slot on the FROZEN `hdl_ast::FunctionDef`, i.e. a format bump — the worst evidence-per-cost in the row) · a `string` element (`var_kind` is `None`; the explicit twin is equally loud) · a module-BODY overridable `parameter` of that type (the array gate, same as the explicit twin) · an INTERFACE or program HEADER array parameter (`module_items.rs`'s module-only gate) · `typedef <struct/enum/alias> x_t [dims];`, refused upstream at `typedefs.rs`'s chained-alias gate and unreachable from here (1-oracle) · an override that CHANGES the dim count (DO-NOT-START, see §5.2; both oracles print `bits=256 s1=2 dims=3`) · `T'(…)` (no oracle) · dims on BOTH the typedef and the declarator (`a_t y [0:1]`, a live oracle SPLIT on dimension ORDER — iverilog `$size(y,1)=4 $size(y,2)=2`, verilator `2` / `4`, and iverilog contradicts its own answer for the identical explicit type). Composition order is the trap `$bits` cannot catch: the NAME's dims come first (`localparam a_t P [0:1]` reads `P[0][1]`=2 and `P[1][2]`=6). ARITY is the half that cannot follow an override — declarators are stamped with the default's dim LIST once at parse — so `shape_flags` carries the dim COUNT and a mismatch is loud in both directions (dim-losing ⇒ the group's F4004; dim-ADDING ⇒ E3002 named on `T`, suppressed when `T$w` is equally unknown, because then `T` is simply not overridable here) | each consumer reads `TypeInfo` and has no slot for unpacked dims. The DECLARATION consumers (a variable, a port, a tf-port formal, a type-parameter default) carry the dims through the map; the rest decline on `!info.unpacked.is_empty()` rather than bind the element type | per consumer, each its own slice; the split row is do-not-start | 2-oracle except the function return type (1) and the declarator-dims split (0) | S each |
| ⑤ⓓ | nested struct members: `default: v` with a non-fill non-zero `v` · `o.i.e.name()` · a packed ARRAY member `in_t [1:0] i` · a packed struct inside an UNPACKED record · `u.c.perms.q` · `o.i[1+:2] = …` · a member width given as `1 << 3`, `8'd5`, a forward-referenced localparam, or a header `parameter` (overridable = correct-loud) | outside the source kinds the parser's flat layout table accepts | widen per consumer | 2-oracle (`default: v`: verilator whole / iverilog rejects) | — |
| ⑤ | CU scope: a unit-scope VARIABLE or net · a unit enum label in a class body · a forward reference between unit constants · `$unit::t` · an enum-typed output port driven by `assign` (E3018) | outside the parser's unit-scope clone | per item | 2-oracle (the forward reference is split; vita follows iverilog) | — |
| ⑤ | parser / preprocessor: a multi-dim packed formal that is also an unpacked array (`logic [1:0][3:0] a [2]`) · a based-literal value for a parameter narrower than 32 bits, outside the parse-time table · a non-ANSI `<type> [dims]` port · an atom typedef with dims · a SIGNED typedef element with dims · an unnamed or duplicate `` `define `` formal | the parser's flat rewrite · the `` `define `` argument parser | widen the table and the rewrite | 2-oracle / split (verilator lenient) | — |
| ⑤ | `parameter type` with a struct, enum, union, real, string or class default or override, or a multi-dimensional one | not expressible by the `T$w` / `T$s` two-value-parameter desugar | loud by design | 2-oracle | — |
| ⑧ | system functions in a function body are refused — `$random` / `$time` inside `assign m = f()` re-draw on every pass | the seed is not a net, and `levelize::func_read_deps` cannot name it | represent non-net state in the dependency set | 2-oracle (both freeze it) | — |
| ⑧ | the statement after a reached `$finish` executes (the same behaviour as `$fatal`) | `SimState::frame_end_is_loud`'s boundary is the statement | move the boundary to expression level | iverilog stops | — |
| ⑧ | a function with an output formal is routed through `Terminator::Call` and PERFORMS the `$finish` (exit 0), so the same syntax has two answers depending on formal direction | routing splits on formal direction | unify the routing | iverilog rejects the syntax itself | — |
| ⑧ | the residual mismatch for a function that carries a counter forward is one evaluation, not a rule | vita's extra t0 settle pass | prerequisite for an honest certification of this family | 2-oracle | — |
| ⑨ | after `import pk::*;` a bare string or real parameter name is loud (the fold succeeds; only the import binding is missing) | `apply_import_consts` re-binds through `params` (i64) only | give the string and real side maps the same treatment — plumbing, not routing, two call sites. Pins = `string_const_domain.rs`, `real_params.rs` | 2-oracle | — |
| ⑨ | a real condition in `generate if (P::R > 1.0)` is loud | `const_real.rs` has no `PkgScoped` arm | add the arm | 2-oracle | small |
| ⑬ | an array access inside a subroutine body is attributed to the CALL statement | the tier-3 arena only RECORDS and drains at the caller's statement boundary, so it does not know the callee StmtId | a second `cur_stmt` source or a shared `Rc<Cell>`. Trap: adding a publish makes interp report `d.sv:6` and native report no location, so backend agreement was chosen | — | — |
| ⑬ | a terminator condition (`if (mem[i])`), a continuous-assign settle, a t0 arm and a delayed-CA apply drain have no location | they are evaluated after the block's last statement, which clears `cur_stmt` to NO_STMT | no location is better than a wrong line (deliberate) | — | — |
| ⑬ | W4022, W4028, the delta limit, RunRange, W4020 and the W4029/W4007 instance path all report `location: None` | the sid-less diagnostic family has no access-statement key in the engine | `cur_stmt` plus `stmt_diag_meta` (the plumbing exists) · a `SpanResolver` plus a StmtId→span sidecar | — | — |
| ⑭ | the call tree to task granularity is not shipped: an inlined subroutine reports 0 calls and reads as "free" | calls are lowered two ways — a call-seam frame body and an elaborate-time INLINE splice (`inline_task.rs` / `inline_fn.rs`; 14.39 s inlined against 0.35 s framed) | prerequisite = an elaborate-time record of inline site → caller (`Sidecars::func_names` exists; only the declaration `file:line:col` twin is missing) | — | — |
| ⑭ | a reporter wants ~440 cycles/s and measures 20.4 — a 21× scheduler/executor gap | not an observability item | Phase D codegen plus arena — tracked in §5 | — | — |

### 3.b Small residues

**Parser accept**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| specify | a `specify … endspecify` block is E2002 (`specparam` is accepted as a module constant) | the parser does not accept it | hoist `specparam` and discard path delays and timing checks. Prerequisite: `hdl_parser::parse` has no warning channel, so discarding silently would turn `$setup` from loud into silent — needs a `ModuleItem` marker plus an elaborate `W3056` | iverilog | — |
| case-inside | `case (x) inside {…}` (§12.5.4) is E2002 | the parser does not accept it | hand-IEEE `==?` plus an internal differential | no oracle | — |
| based-ws | `64'sh FFFF` is a lexer reject | lexer | accept it | iverilog accepts | minor |
| tf-localparam | `task automatic t; localparam int K = 3;` gives `E2002 expected statement, found keyword 'localparam'` (IEEE §6.20 allows it) | the parser's statement position | accept the declaration | iverilog | small |
| R30-1 | a missing package gives 7 lines of E2002 and never names the package | the parser cannot take `IDENT::IDENT` in a tf-port as a type | take it as a type and let elaborate say "unknown package" ⇒ 1 line | — | parser |
| blocal-flatten | ⓐ two sibling blocks using the same name are refused by the read-before-assign guard even when they are assigned only by declaration initialisers (both oracles `o1=44 o2=55`) · ⓑ two mutually exclusive `if`/`else` branches declaring the same name hit the same guard, and it is 2-oracle (both `o1=44`) | `block_local/gate.rs`'s read-before-assign guard is the SYMPTOM site. The real site is the storage classifier `gather_auto_block_locals`: a static initialiser runs once at t0 rather than on block entry (measured `6,7,8,9` across three tools), so counting initialisers in the guard turns the cells into `o1=44 o2=44`, loud→silent-wrong | adding a "static with an initialiser" term to the classifier turns ⓐ, ⓑ and the widened nesting — 19 cells, all agreeing with both oracles — from loud into a value, but the candidacy pass cannot absorb the fourth admission rule: widening REMOVES candidates (an adjacent `automatic` block loses its scope), an `automatic` floor then makes 4 kinds of dynamic storage loud, and a per-rule floor makes the shadow rule's floor erase another span's loud, 18 cells loud→silent-wrong. PREREQUISITE CLOSED by §4.5.468 (the shadow mis-route; `AdmitReason` now carries WHY a span was admitted, which is the carrier the per-rule floor lacked). The per-rule floor itself was still the wrong shape at revert time — re-census the 19 cells and the 18 R3 cells on HEAD before restarting | ⓐ 2-oracle ⓑ 2-oracle | — |
| enum-label | `enum bit[3:0] {A=8'hFF}` never reaches `enum_defs`, so `.first` / `.next` / `.name` are all E3010 / E3009, and the skipped out-of-range check silently truncates | `const_lit` folds unsized decimals only | widen `const_lit` or check at elaborate time | iverilog rejects | — |
| md-packed-write | multi-dim packed nested part-select WRITE: an ascending or non-zero-lsb leaf · a genvar-indexed `x[g][m:l]` (over-rejected) · a const out-of-bounds packed index is a silent no-op | the current support is limited to a descending zero-lsb leaf | widen the leaf geometry | — | — |
| misc-parse | a negative-LSB member sub-select · `import` inside a generate · a package's own-function initializer · a SYS-READ hierarchical-element destination · a hierarchical-write sentinel panic that should be loud · `logic[1:0][7:0] PK` · `'{k:v}` | — | hand-IEEE plus an internal differential | no oracle | — |

**Constants / parameters**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| wide-override | a wide (>64-bit) parameter OVERRIDE is loud — `#(.K(128'h…))` is refused | the override channel carries i64 and string only | a wide slot on `ResolvedOverride`; since the parent folds without knowing the child's width, pass the source text and re-fold at the child's width | — | — |
| §3.3 | a wide `localparam` part-select fold — `{A[127:64], 64'h0}` | a separate arm that needs an index fold | add the arm | iverilog folds | — |
| real-fold | `+`, `*`, `/`, `-` and `**` on a `localparam` / `parameter real` are all E3009 "not foldable"; `$clog2(real-lit)` has the same root | `const_eval_in_scope` is i64-only | add real f64 arithmetic | iverilog folds | broad |
| xz-fill-param | `localparam logic [W] P = 'x` binds 0 (the x is lost), so `P==0`, `P+1` and `P ==? pat` all diverge | `fill_to_i64` / `fill_literal_const` | x/z in the constant domain | — | broad |
| compound-==? | `==?` fold residue = an unsized x/z pattern · a negative signed LHS · a non-literal RHS · a non-constant parameter override (W3056→error) · a longint MIN fold (package) · two loud-message quality items | the current fold handles sized patterns only | widen it | — | — |
| defparam-iface | `ifc a(); defparam a.D = 255;` gives `W3056 … matched no instance` and keeps the default (iverilog `d=ff`, vita `d=8`) | `defparams` is consumed only in `elaborate_instance`, and `iface_inst.rs` reads only its own `overrides` | merge `defparams.remove(path)` into the canonical binder | iverilog | small |
| neg-ascending | `reg [-33:-2]` gives `$bits` 1 in vita against iverilog's 32, plus a loud `W3056`. Descending `[-2:-33]` and mixed `[3:-2]` are correct | `array_geom.rs`'s `allow_neg_lsb` is opt-in | put that combination on the opt-in path | iverilog | — |
| neg-bound-part | a negative-bound net PART select: `q[-3 +: 2]` and `q[-1 -: 2]` are exact and only `[msb:lsb]` is blocked. Writes are asymmetric — `x[-3:-2]=…` is silently exact while `x[-1:0]=…` is loud with an "out of order" diagnostic that names the wrong fact | the bound fold is unsigned | `const_bound_signed` | verilator | — |
| neg-elem-bound | `logic [-3:0] q[$]` gives a W3056 clamp (verilator `q[0][-3]`=1) | the element net takes `elaborate_netvar_decl_inner`'s early-`continue` path and never reaches the declaration side map | make it reach the side map | verilator | — |
| §3.1(c) | no warning for a declaration initializer on a variable driven by `always_comb` — verilator gives a `MULTIDRIVEN` error, xrun `*E,MULAXX`, iverilog runs it (2:1) | the elaborate layer; sensitivity-list synthesis already knows the set | one W2004-grade warning line | split (lint) | small |
| aes§2 | the inliner's discriminator is more than `automatic` — plain 3/5, and `automatic` / `for` / `if` / `case` 1/5, `p::f()` 2/5 | the inliner's discriminator | widen the inliner | measured | — |

**Subroutine / frame**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
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
| deep | a t0 race · an `@(*)` decl-init wake · a runtime `==?` pattern · a NON-fill context width in an inline body · modport direction enforcement · a force on a part-select · an associative key or clocking array output word0 · a PART select of a negative range bound (§2) | — | — | — | deep |

**Diagnostics quality**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| E3009-anchor | `E3010` / `E3009` file:line is inconsistent — some sites attach it (`d_trunc.v:3:20`) and some print only the hierarchical path | the anchor is not passed through | `diag::SpanResolver` exists, so the scope is every call site that does not pass an anchor | — | — |
| error_at | the anchor and the `found` token differ — `g[w].u.q` anchors at `w` and the message says `found '.'` | `error_at` takes an earlier node while `found` takes the cursor token | they are separate fields, so this is correct-but-confusing; 10 sites | — | — |
| #9 | the `velab -L` (worklib merge) path has no locations | each compilation unit's spans index its own expansion buffer from 0, so the coordinate spaces overlap; a wrong CU map would give a wrong file:line, so `None` is kept | rewrite span offsets at merge time (a whole-AST walk) | — | — |
| cli-lib | `cargo test -p cli --no-default-features --lib` dies with E0004 (pre-existing) | the lib test target revives sim-engine's `oracle` through a dev-dependency link while the cli feature stays off, so two `#[cfg(feature="oracle")]` arms of `backend_name` are cut | set the cli dev-dependency to `default-features = false`, or merge the two crates' `oracle` into one. CI cannot see it, so do not add `-p cli` to that command | — | — |
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

**VCD / real conversion**

| id | gap · repro · oracle values | root cause · code site | fix shape · prerequisite | oracle | size |
|---|---|---|---|---|---|
| vcd | cosmetic encoding differences (decoding identical): ① vita writes full width (`bxxxxxxxx`) where iverilog strips (`bx`, `b0`) ② the t=0 initial dump is a `$dumpvars` pre-assign X plus a `#0` change against a settled value ③ a procedurally driven `logic` is `wire` in vita and `reg` in iverilog, and `int` is `reg` against `integer` ④ a real's size is 64 against 1 ⑤ `parameter` is not dumped | elaborate's packed-md `NetVar.lsb` is stale (the VCD helper routes around it with a flat fallback) | — | iverilog | large golden churn |
| x→real | an X-bearing integral converted to real: vita takes the whole value to `0.0` where iverilog converts per bit (`4'bxx01` → 1). Shared by `$itor`, `$sqrt`, `$pow` and real `**` | `real_arg` is `to_i128_signed().unwrap_or(0)` | convert per bit | iverilog | not silent |
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
| BYTE-GATE-6 | the S1d-4d byte-identity gate will meet 6 pre-existing oracle differences: ① pending NBA/transport at the `$finish` tick ② VCD intra-tick granularity ③ t=0 `initial` order ④ t0 arm order ⑤ an `always @(*)` with an empty read set runs at t0 in vita only ⑥ a `.velab` is not reproducible across `vcmp` runs (RULEV-MTIME) | half deliberate design, half LRM-undefined | decide which way each of the six is pinned, first | — |
| MON-RENDER | the tier-3 render path refuses `$monitor` / `$strobe` | rendering lives in `sched/run_loop.rs::flush_postponed` and that path takes no reader | wiring — one slice with S1d-4c | lifts the refusal |
| FD-EOF + FEOF | the `fd_eof` X-poison hole in `NetArena` (`fd_eof` alone is outside the "no heap/class/frame" argument; the `$feof` over-marking currently hides it) · `$feof` is over-marked in the canonical statement-effect predicate, so `e = $feof(fd);` is refused while `while (!$feof(fd))` passes | `k_feof` is a pure read while `sysfunc_is_stmt_effect` says `true`; fixing one consumer leaves two spellings | one slice · fixing the canonical predicate also widens the tier-2 gate · a byte-identity argument | removes a tier-3 over-refusal |
| NETSLOT-PREV | nothing in the workspace reads `NetSlot.prev` (only the declaration, the constructor and pass (c)'s write), so pass (c)'s two `clone_from` calls per changed net per delta are dead work | nobody reads it | remove it, plus a separate slice that verifies the obviousness itself | perf |
| WPROG-WHY | an expression falling out of the compiled lane is INVISIBLE. `codegen.reject_reasons` is a per-PROCESS census, so a body reports `able 1/1` while every evaluation of its RHS runs the generic path, and the compiled-lane boundary can only be inferred from `$signed`/`$unsigned` call counts — an inference that has produced two wrong causes (§2 Performance) | `wprog::compile` returns a bare `None` at ~20 decline sites and nothing counts them | a per-(reason, count) tally on `SimOpts`, folded into `run.json` beside `codegen` — the shape `builtins` already has. Reject reasons are a REPORTING table: never let one panic or change a value | reads as G2/OBS, not perf |
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

Canonical start order. LOOPROMPT.md's NEXT mirrors this table; when they differ this table wins. A
bundle is one track item plus two rows meeting the §1 slot rule (single root, two oracles, outside
the walls and the oracle splits, no format bump visible at selection). Two rows with the SAME ROOT
are one slot, and their ORDER is a measurement: a value lane must land before the guard over an
unlimited fold is deleted, or the deletion is 8 cells of loud→silent-wrong.

| # | slot | item | source | rank |
|---|---|---|---|---|
| 1 | 1 | §3.b `blocal-flatten` ⓐⓑ: prerequisite closed by §4.5.468. Re-census the 19 loud→value cells and §4.5.467's 18 R3 cells on HEAD first; the fourth admission term now has a reason carrier (`AdmitReason`), so the candidacy pass can tell a static-initialiser span from a shadow span without a per-rule floor | §3.b | ② |
| 2 | 2 | §2 `localparam` derived from an overridden untyped parameter forwards at 32 (`localparam R = ~Q; leaf #(.P(R))` → `32/c`, both oracles `4/c`). Root = the operator arm declining under `declared_only` (§4.5.363 fence) on a lane that has no override to fence. Width-only, pinned | §2 Index sealing | ① |
| 3 | 3 | §2 `pkg::`-scoped override SOURCE loses both columns (`leaf #(.P(pk::PW))` → `32/1`, both oracles `36/800000001`). Root = `wide_name_bits` / `narrow_param_bits` take a single-segment path. The bare imported spelling is correct, so the fix is the `PkgScoped` arm of that path, not a new channel | §2 Index sealing | ① |
| 4 | OBS | §6 follow-on: give the static `subroutines` rows a declaration site so the two subroutine objects can be joined (`subroutine_calls`'s `key` text currently says they cannot be) | §6 | ④ |
| 5 | OBS | `WPROG-WHY`: a per-(reason, count) tally of `wprog::compile`'s decline sites, folded into `run.json` beside `codegen` (the shape `builtins` already has) | §5.b | ④ |
| 6 | next | §3 ⑤ⓕ's NON-ARITY axis: `shape_flags`'s F4004 rejects signedness, 2-state kind and arity through one `!=`. The first two need no arity carrier and are 2-oracle measured (`int`→`logic [31:0]`, `int`→`int unsigned`, `logic [7:0]`→`logic signed [7:0]` all agree in both oracles while vita gives F4004; the width-only control passes three-way) · a multi-dimensional PACKED type-param default or override is E2002 at parse (both oracles run it) · a mixed-caller callee · `m #(8)` / `defparam u.T$w` · the VCD `$scope` `[0]` spelling · a `genblk<N>` label collision (split) · the §2 🆕 L ⓦ residue · the §2 🆕 N residue | §3 | ② |
| 7 | hygiene | `params.rs` is 2,116 lines against the 1,000-line policy and is not on the exception list; `param_query.rs` is the precedent for the split. NOT inside a correctness bundle — a refactor is a design nobody has reviewed | [ENGINEERING_RULES.md](ENGINEERING_RULES.md) §10.1 | — |

Do not start:

- §2 rows 16, 26, 30 and 🆕 F — the §11.8.1 region-sign wall. Rows 14 and 25's declared-width
  provenance half is NOT a wall: `param_range` carries the provenance and `narrow_param_bits`
  resolves it at the override fold; what remains there is the i64/CARRY axis.
- §2 row 34 (one oracle, zero demand) and row 31 (the pure half is correct, so it is performance).
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
- The static `subroutines` rows carry no declaration site, so a consumer cannot join them to
  `subroutine_calls`; the runtime object's `key` text says so rather than instructing a join it
  cannot serve. Giving `SubroutineRoute` a `DeclLoc` closes it — the elaborator already resolves one
  at reserve and the route census is filed in a different pass, so it is a threading slice, not a new
  mechanism. This is §5.2 queue item 4.
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
- `WPROG-WHY` (§5.b) reads as an OBS item: it is the tally that answers why an expression left the
  compiled lane.

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
