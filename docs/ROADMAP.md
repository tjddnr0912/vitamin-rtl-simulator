# ROADMAP — open work

Open work, one line per item: id, title and a link to its detail doc, and its status or slot when it has one. Format, filing, census: [roadmap/README.md](roadmap/README.md).

## Summary
> Per-track counts of open, startable and blocked items with their top blockers, ladder rung and next §5.2 rows: 637 open, 399 startable.

Recount from the lines below in every docs step; the iteration report shows this table; the census rule is in the notes ([notes](roadmap/README.md))

| § | track | open | startable | blocked | blocked by (top reasons) | rung | next |
|---|---|---:|---:|---:|---|---|---|
| §2 | silent-wrong start-order rows | 36 | 7 | 29 | named prerequisite 14 · oracle split 5 · loud, zero demand 3 | frozen; six 🆕 rows held (§5.2) | 1, 3, 4, 6, 7 ([note](roadmap/README.md)) |
| §2 | recorded defects by mechanism | 336 | 214 | 122 | oracle split or none 68 · prerequisite 21 | frozen; witness lines queued | 2, 8, 46, 51, 52, 68, 70, 71, 72b ([note](roadmap/README.md)) |
| §2-N | corpus censuses | 6 | 0 | 6 | oracle split 3 · held on purpose 2 · upstream fst-writer 1 | ① | |
| §3.a | loud → correct-support, numbered | 29 | 20 | 9 | loud by design 2 · named prerequisite 2 · loud, no census page 3 · deferred to §5 1 · deferred to §7 1 | ② | 42, 44 |
| §3.b | loud → correct-support, small | 209 | 174 | 35 | named prerequisite 10 · deferred to §7 8 | ② | 5, 23, 24, 26, 29, 33–35, 37–41, 43, 45, 49, 50, 69 ([note](roadmap/README.md)) |
| §3.c | intentionally loud | 11 | 0 | 11 | by design or oracle split 11 | — | |
| §0 | promotion queue (T2 residues) | 14 | 8 | 6 | oracle split 3 · deliberate 2 · `defparam` non-goal 1 | ③ | |
| §4 | SVA honest-loud | 6 | 0 | 6 | named prerequisite 6 | ③ | |
| §6 | G2 observability (OBS) | 6 stages + 10 | 15 | 1 | call tree: two lowering paths 1 | ④ | |
| study/03 | workload corpus | 3 | 0 | 3 | named prerequisite 3 | real-design | 31, 47, 53 |
| §5.b | performance / hardening | 21 | 11 | 10 | named prerequisite 6 · held or trigger-gated 4 | below the ladder | |
| §7 | conditional / long-term | 8 | 0 | 8 | trigger-gated 8 | trigger-gated | |
| §8 | non-goals | 2 | 0 | 2 | permanent 2 | permanent | |
| total | | 697 | 449 | 248 | | | |

## 0. correct-support promotion queue
> Honest-loud constructs waiting to become correct support: iverilog defects pinned as oracle disqualifiers and the T2 residues, 8 of 14 startable.

Deliberately loud, not gaps: `new[]` on a fixed array; a multi-dim partial index `s[0]`; a cross-type SoA whole-element copy; a `real` scrutinee in a `generate case`. ([notes](roadmap/0/README.md))

### iverilog defects (vita is IEEE-correct) — oracle disqualifiers, regression-pinned
> Ten cases where iverilog is wrong and vita follows IEEE; each is regression-pinned and disqualifies iverilog as an oracle for that construct.

- ① — [`.len()` of a string-array element](roadmap/0/1.md)
- ② — [concurrent fork activations sharing an `automatic` string array](roadmap/0/2.md)
- ③ — [`$fmonitor` called twice on one file descriptor](roadmap/0/3.md)
- ④ — [`%s` of an empty string-array element](roadmap/0/4.md)
- ⑤ — [`$clog2` of a 4-bit signed sum](roadmap/0/5.md)
- ⑥ — [`$itor` of a 64-bit value wider than 32 bits](roadmap/0/6.md)
- ⑦ — [string less-than comparisons against literals](roadmap/0/7.md)
- ⑧ — [an `automatic time signed` frame local](roadmap/0/8.md)
- ⑨ — [a function in a continuous assign evaluated at time 0](roadmap/0/9.md)
- ⑩ — [negative zero from constant and runtime real negation](roadmap/0/10.md)

### T2 residues (each its own slice)
> Fourteen loud T2 leftovers, each its own slice: real-valued parameters, enum labels, negative bounds, `-G` / `-P` options, `case inside` sizing.

- 8ⓐ — [a real parameter used as a vector width or replication count](roadmap/0/8a.md) · SPLIT
- 8ⓑ — [a real value in an untyped `localparam`](roadmap/0/8b.md) · HELD
- 8ⓒ — [real-valued parameter overrides](roadmap/0/8c.md) · OPEN
- 8ⓓ — [non-finite real constants such as `1.0/0.0`](roadmap/0/8d.md) · HELD
- 8ⓔ — [a shift of a real operand](roadmap/0/8e.md) · SPLIT
- 8ⓖ — [`$rtoi` inside a constant-function body](roadmap/0/8g.md) · OPEN
- 8ⓗ — [nested explicit real and integer casts](roadmap/0/8h.md) · OPEN
- 10ⓐ — [`parameter` enum labels folded before overrides](roadmap/0/10a.md) · OPEN
- 10ⓑ — [enum methods over a `localparam` label with a based literal](roadmap/0/10b.md) · OPEN
- 11 — [negative range bounds on vectors and ports](roadmap/0/11.md) · OPEN
- 14-a — [the `-pvalue+` parameter option](roadmap/0/14-a.md) · OPEN
- 14-b — [the `-P` hierarchical parameter option](roadmap/0/14-b.md) · BLOCKED
- 14-c — [`.velab` files from different `-G` values sharing a header](roadmap/0/14-c.md) · OPEN
- 13 — [`case inside` where the signed and unsigned sizing rules split](roadmap/0/13.md) · SPLIT

## 0-B. Small follow-ons (kept loud)
> Two small follow-ons that stay loud for now: argument forms of a frame call, and fork inside a frame.

- frame-formal-follow-ons — [frame-call follow-ons: void cast, nested array formal, sized cast](roadmap/0-B/frame-formal-follow-ons.md)
- frame-fork-rewalk — [fork inside a frame: re-walk, nested forking task, sibling visibility](roadmap/0-B/frame-fork-rewalk.md)

## 0-C. Large items — start decision table (do not re-estimate the size)
> Two large items with a fixed start decision and size: the file-position family goes before a shared literal-parsing crate.

- A — [file-position system functions (`$ftell`, `$fseek`, `$rewind`, `$ferror`)](roadmap/0-C/A.md)
- B — [a shared crate for literal parsing](roadmap/0-C/B.md)
- Order A > B.

## 1. Start priority — principle only (the live queue is §5.2)
> The start-priority principle only, no queue: silent-wrong first, then loud to supported, honest-loud promotion, observability; the live queue is §5.2.

① CRITICAL silent-wrong with an oracle (§2) > ② loud→supported with an oracle (§3) > ③ honest-loud promotion whose prerequisite holds (§0, §4, §5) > ④ G2 OBS (§6). ([notes](roadmap/1/README.md))

## 2-N. Silent-wrong from the corpus censuses
> Silent-wrong differences the corpus censuses found: three numbered rows and three t0-event residues, all split, held or blocked upstream.

- 2-N-1 — [verilog-axi valid outputs differ briefly after reset](roadmap/2-N/2-N-1.md) · SPLIT
- 2-N-2 — [FST dumps lose the `$dumpvars` initial snapshot](roadmap/2-N/2-N-2.md) · BLOCKED
- 2-N-3 — [verilog-axis: a time-0 change from x wakes an `always @*`](roadmap/2-N/2-N-3.md) · SPLIT

t0-event residues (held on purpose):

- t0-narrow-wire-wake — [a narrowing wire initialiser wakes an event control at time 0](roadmap/2-N/t0-narrow-wire-wake.md) · HELD
- reg-init-from-wire — [a reg initialised at declaration from a constant wire](roadmap/2-N/reg-init-from-wire.md) · SPLIT
- buf-z-input — [a `buf` gate driven by a high-impedance input](roadmap/2-N/buf-z-input.md) · HELD

## 2-R. Usability residue
> Two usability residues of designs that are loud anyway: an unused package function still framed, and a spurious E3001 from Rule A.

- unused-pkg-fn-framed — [an unused package function is still framed](roadmap/2-R/unused-pkg-fn-framed.md)
- rule-a-hier-spill — [Rule A's unknown hierarchical callee spills E3001 onto a read-only net](roadmap/2-R/rule-a-hier-spill.md)

## 2. Silent-wrong residues
> Rows where vita runs and prints a wrong value, by area; most are open with oracles, frozen out of §5.2 until a corpus design hits them

Row ids are cited from source (`§2 row 7`, `§2 🆕 I`); a number is never reused.

FROZEN (owner direction): synthetic-probe rows stay out of §5.2; the re-entry rule and the AST self-width wall are in the notes ([notes](roadmap/2/README.md))

### Start-order rows
> The numbered and 🆕 rows in start order: constant folds, generate walks, time-0 holds; 14 of 36 blocked, six behind 🆕 AE (4 direct U V W AI, 2 via S T)

- 🆕 B — [a `case` on a signed right shift with an unsigned label keeps the sign](roadmap/2/new-B.md) · BLOCKED
- 3b — [class-property bound normalisation has no home](roadmap/2/3b.md) · BLOCKED
- 7 — [a parent `initial` reads a child net at time 0 before it settles](roadmap/2/7.md) · SPLIT
- 15 — [an override's sized x/z literal loses its unknown plane](roadmap/2/15.md) · BLOCKED
- 16 — [parameter overrides on which the oracles split (>64-bit tops)](roadmap/2/16.md) · SPLIT
- 17 — [a 32-bit all-ones override onto a 128-bit parameter extends three ways](roadmap/2/17.md) · SPLIT
- 19 — [a 2-D, 3-D or packed-element continuous-assign target is ~10× slower](roadmap/2/19.md) · PERF
- 23 — [two clocking blocks whose mangled signal names collide](roadmap/2/23.md) · LOUD
- 24 — [a clocking output clobbered to x, and a one-cycle lag](roadmap/2/24.md) · DO-NOT-START
- 🆕 H — [constant reductions that stay loud: parameters, function locals, x/z bits](roadmap/2/new-H.md) · OPEN
- 🆕 I — [a same-delta read of another process's write, and copy-sign residues](roadmap/2/new-I.md) · HELD
- 🆕 J — [the unbased unsized `'1` in concat, index, fill and `$bits`](roadmap/2/new-J.md) · LOUD
- 🆕 M — [`'1` in a parameter override and other target-typed folds](roadmap/2/new-M.md) · BLOCKED
- 🆕 N — [scope naming in VCD, generate block names, `%m` and the hierarchy tree](roadmap/2/new-N.md) · HELD
- 🆕 O — [the name-routing bypasses: a `foreach` shadow split and partial guards](roadmap/2/new-O.md) · HELD
- 🆕 Q — [a `localparam` inside a procedural block does not parse](roadmap/2/new-Q.md) · BLOCKED
- 🆕 L — [loud constant and import residues](roadmap/2/new-L.md) · LOUD
- 🆕 S — [a constant `inside` / `==?` compares at the left operand's width](roadmap/2/new-S.md) · BLOCKED
- 🆕 T — [a generate-case label the i64 fold cannot read is skipped](roadmap/2/new-T.md) · BLOCKED
- 🆕 U — [a genvar, enum label or narrow import over a wide constant reads the wide value](roadmap/2/new-U.md) · BLOCKED
- 🆕 V — [a generate-scope parameter stays bound across the four generate walks](roadmap/2/new-V.md) · BLOCKED
- 🆕 W — [a constant typed by an overridden type parameter reads unsigned](roadmap/2/new-W.md) · BLOCKED
- 🆕 X — [a class method's `case` re-evaluates a call scrutinee per label](roadmap/2/new-X.md) · OPEN
- 🆕 Y — [a string-returning call as a `case` scrutinee compares as packed bits](roadmap/2/new-Y.md) · OPEN
- 🆕 AA — [a combinational block written before its feeder reads x at time 0](roadmap/2/new-AA.md) · SPLIT
- 🆕 AB — [residues of the time-0 hold of effectful and class-handle assigns](roadmap/2/new-AB.md) · BLOCKED
- 🆕 AC — [a call the constant interpreter declines where a constant is needed](roadmap/2/new-AC.md) · BLOCKED
- 🆕 AD — [routine text folded at the caller binds the caller's same-named names](roadmap/2/new-AD.md) · OPEN (part)
- 🆕 AE — [the constant interpreter reads a never-assigned 4-state variable as 0](roadmap/2/new-AE.md) · BLOCKED
- 🆕 AF — [a `$unit` routine folds at the calling module and binds its names](roadmap/2/new-AF.md) · OPEN
- 🆕 AG — [a constant call in a generate block binds the module's same-named function](roadmap/2/new-AG.md) · OPEN
- 🆕 AH — [an instance array binds a child's header at the parent's scope](roadmap/2/new-AH.md) · OPEN
- 🆕 AI — [a signed constant of unknown width sign-extends into an unsigned region](roadmap/2/new-AI.md) · BLOCKED
- 31 — [`$signed(a)*$signed(b)` in a continuous assign evaluates three times](roadmap/2/31.md) · PERF
- 32 — [`$finish` called inside a function](roadmap/2/32.md) · SPLIT
- 34 — [why the clocking rows 23 and 24 wait](roadmap/2/34.md) · DO-NOT-START

### Size cast / signedness
> Casts and sign rules that take the wrong width or sign: enum widths, call leaves, nested casts; open fixes beside four width walls

- enum-method-width — [an enum method's result is 32 bits instead of the enum's width](roadmap/2/enum-method-width.md) · OPEN
- Width probe — [the width probe runs diagnostics and serializes the dead node](roadmap/2/Width-probe.md) · WALL
- Node width — [`ir_bits_of` gives the cast node's width, not the operand's](roadmap/2/Node-width.md) · WALL
- Unknown width — [`ir_bits_of` has no width for a hierarchical or string read](roadmap/2/Unknown-width.md) · WALL
- enum-bit-four-state — [a `typedef enum bit` stores and reads 4-state values](roadmap/2/enum-bit-four-state.md) · OPEN
- baseless-enum-return — [a function returning a base-less enum reads its unset return as x](roadmap/2/baseless-enum-return.md) · OPEN
- narrowing-cast-drops-x — [a 4-state narrowing cast drops x](roadmap/2/narrowing-cast-drops-x.md) · OPEN
- Call-leaf sign — [a sized cast over a call minus a constant zero-extends](roadmap/2/Call-leaf-sign.md) · OPEN
- Real × fill — [a cast over a real and a fill literal skips the sign funnel](roadmap/2/Real---fill.md) · WALL
- signed-of-real-accepted — [`$signed` over a real is accepted in 7 of 22 positions](roadmap/2/signed-of-real-accepted.md) · OPEN
- Nested cast — [an outer cast's context stops at an inner node](roadmap/2/Nested-cast.md) · OPEN
- Fabricated width — [a cast over a method call or array reduction zero-extends](roadmap/2/Fabricated-width.md) · BLOCKED
- Double call — [a sized cast over a signed call calls it twice](roadmap/2/Double-call.md) · OPEN
- Element sign — [an unpacked element's sign is read for single-segment names only](roadmap/2/Element-sign.md) · OPEN
- type-param-array-sign — [a constant array typed by a type parameter keeps the default type's sign](roadmap/2/type-param-array-sign.md) · OPEN
- 🆕 AJ — [an ANSI port continuation keeps its type parameter's default type](roadmap/2/new-AJ.md) · OPEN
- 🆕 AK — [a non-ANSI `input signed` port redeclared as an unsigned net reads unsigned](roadmap/2/new-AK.md) · OPEN
- respell-drops-const-type — [a package md parameter's use site reads an imported constant as a 32-bit integer](roadmap/2/respell-drops-const-type.md) · OPEN
- body-typedef-sym-mix — [a function-body symbolic struct over mixed-sign parameters is laid out wrong](roadmap/2/body-typedef-sym-mix.md) · OPEN
- specparam-data-type — [a `specparam` with a data type, or outside a module, is accepted](roadmap/2/specparam-data-type.md) · OPEN
- typeparam-value-signing — [a type-parameter value with a second or misplaced signing is accepted](roadmap/2/typeparam-value-signing.md) · OPEN

### Constant domain (i64)
> The i64 constant folder's limits: wraps, masked overflow, unset variables read as 0, replication counts; mostly open, a few held by design

- wide-decline-deliberate — [the i64 constant fold declines above 64 bits on purpose](roadmap/2/wide-decline-deliberate.md) · HELD
- untyped-64bit-wrap — [an untyped 64-bit `*` / `+` wraps](roadmap/2/untyped-64bit-wrap.md) · HELD
- huge-untyped-power — [a huge untyped `**` that only one oracle runs](roadmap/2/huge-untyped-power.md) · RECORD
- placement-cast-fold-residue — [constant folds of a concat carry, concat x/z and prim casts stay loud](roadmap/2/placement-cast-fold-residue.md) · OPEN
- size-wrapping-cast — [a size-wrapping cast over a constant division declines](roadmap/2/size-wrapping-cast.md) · WALL
- deep-const-fn-chain — [a 70-deep constant-function chain is loud](roadmap/2/deep-const-fn-chain.md) · OPEN
- empty-string-width — [an empty string literal's width is 1 bit](roadmap/2/empty-string-width.md) · OPEN
- Declined wide walk — [a cast inside a wide expression keeps the unlimited i64 answer](roadmap/2/Declined-wide-walk.md) · OPEN
- Masked overflow — [a 32-bit constant overflow is masked to the wrong value](roadmap/2/Masked-overflow.md) · OPEN
- gen-case-i64-compare — [generate `case` compares two i64 values](roadmap/2/gen-case-i64-compare.md) · OPEN
- select-write-unfolded-local — [a select write into an unfolded local reads 0 for the other bits](roadmap/2/select-write-unfolded-local.md) · OPEN
- block-local-named-function — [a block-local named like its function is the return variable](roadmap/2/block-local-named-function.md) · OPEN
- ca-delay-const-call — [a continuous-assign delay over a constant call folds an unset variable as 0](roadmap/2/ca-delay-const-call.md) · BLOCKED
- huge-packed-net-panic — [a huge 2-D packed `bit` vector panics at net allocation](roadmap/2/huge-packed-net-panic.md) · OPEN
- three-width-models — [three code paths each model a declared width](roadmap/2/three-width-models.md) · RECORD
- unsigned-return-read-signed — [the interpreter reads an `int unsigned` return signed](roadmap/2/unsigned-return-read-signed.md) · OPEN (part)
- formal-shadows-parameter — [a formal shadowing a parameter is selected as the parameter](roadmap/2/formal-shadows-parameter.md) · OPEN
- nonzero-lsb-local-concat — [a non-zero-LSB local in a concat is read by position](roadmap/2/nonzero-lsb-local-concat.md) · OPEN
- negative-replication-count — [a negative replication count is accepted](roadmap/2/negative-replication-count.md) · OPEN
- call-count-replication-width — [a replication counted by a constant call has no width](roadmap/2/call-count-replication-width.md) · BLOCKED
- string-len-self-width — [`S.len()` has no self width in a constant region](roadmap/2/string-len-self-width.md) · OPEN
- wide-signed-name-count — [a >64-bit signed neighbour of a name-count replication folds wrong](roadmap/2/wide-signed-name-count.md) · OPEN
- byte-cast-type-width — [`byte'(…)` folds without the type width](roadmap/2/byte-cast-type-width.md) · OPEN
- x-compare-count-collapses — [a count over an x-holding signed compare collapses to 0](roadmap/2/x-compare-count-collapses.md) · OPEN
- wrapped-zero-divisor — [a modulo by a divisor wrapped to zero binds the unlimited answer](roadmap/2/wrapped-zero-divisor.md) · OPEN
- typed-localparam-unconverted — [a typed localparam whose range does not fold at parse time stays unconverted](roadmap/2/typed-localparam-unconverted.md) · OPEN
- dev-overflow-release-wraps — [a dev build overflows where release wraps to the right constant](roadmap/2/dev-overflow-release-wraps.md) · RECORD
- enum-base-overflow-silent — [an enum base wider than 2^64 bits runs with a wrapped width](roadmap/2/enum-base-overflow-silent.md) · OPEN
- const-atoi-underscore — [a constant-folded `atoi` stops at an underscore](roadmap/2/const-atoi-underscore.md) · OPEN
- bits-width-over-i32 — [`$bits` of a vector 2^32 bits wide prints -1](roadmap/2/bits-width-over-i32.md) · OPEN
- const-fn-signed-return-unsigned-context — [a signed constant-function return in an unsigned context keeps its sign](roadmap/2/const-fn-signed-return-unsigned-context.md) · OPEN
- const-fn-redecl-init-reads-prior — [a constant-function local's initializer that names the local itself](roadmap/2/const-fn-redecl-init-reads-prior.md) · OPEN

### Index sealing
> Parameter and index values bound at the wrong width or sign: unsealed indexes, override leaves, signed parameters; mostly open

- queue-dyn-index-unsealed — [a signed index into a queue or dynamic array is read unsigned](roadmap/2/queue-dyn-index-unsealed.md) · OPEN
- call-index-unsealed — [an index given by a function call is not sealed](roadmap/2/call-index-unsealed.md) · OPEN
- index-call-repeats — [an index holding a call or `$urandom` is evaluated more than once](roadmap/2/index-call-repeats.md) · OPEN
- packed-element-overhang — [a `+:` select that overhangs a packed element](roadmap/2/packed-element-overhang.md) · SPLIT
- wide-shift-override — [a parameter override shifting by a 128-bit amount stays E3009](roadmap/2/wide-shift-override.md) · OPEN
- wide-tree-low-bits — [a declined >64-bit tree binds its low 64 bits](roadmap/2/wide-tree-low-bits.md) · OPEN
- override-fill-default-type — [a fill in an override tree keeps the default's type](roadmap/2/override-fill-default-type.md) · OPEN
- override-leaf-default-type — [an unreadable override leaf keeps the default's type](roadmap/2/override-leaf-default-type.md) · OPEN
- signed-param-reads-unsigned — [a `parameter signed` with an unsigned value reads unsigned](roadmap/2/signed-param-reads-unsigned.md) · BLOCKED
- gen-scope-alias-width — [a generate-scope alias `localparam C = A;` binds 32 bits](roadmap/2/gen-scope-alias-width.md) · OPEN
- signed-string-default — [`signed` on a string default is dropped](roadmap/2/signed-string-default.md) · OPEN
- fill-override-wide-default — [a fill override onto a >64-bit default binds at its width](roadmap/2/fill-override-wide-default.md) · OPEN
- unsigned-param-signed-override — [a `parameter unsigned` with a signed override binds signed](roadmap/2/unsigned-param-signed-override.md) · OPEN
- instance-array-override-width — [an instance-array parameter takes a sibling array's override width](roadmap/2/instance-array-override-width.md) · OPEN

### Inline / frame binds
> Inlined calls and frames that lose the width context: hierarchical leaves, interface members, time literals, streams; open with oracles

- hier-leaf-inline-region — [a hierarchical leaf in a §11.6.1 region lacks the child's environment](roadmap/2/hier-leaf-inline-region.md) · OPEN
- interface-member-port-width — [an interface member through a module port keeps the old width](roadmap/2/interface-member-port-width.md) · OPEN
- interface-facts-declined — [interface members the §4.5.510 fact table declines](roadmap/2/interface-facts-declined.md) · OPEN
- signed-string-arg-accepted — [`$signed` of a string and two-argument `$unsigned` are accepted](roadmap/2/signed-string-arg-accepted.md) · OPEN
- inline-time-literal — [a time literal in an inline region folds at 64 bits](roadmap/2/inline-time-literal.md) · OPEN
- runtime-time-literal — [a time literal in a run-time operation keeps the integral context](roadmap/2/runtime-time-literal.md) · OPEN
- inline-formal-width — [an inline body's written formal gets no width context](roadmap/2/inline-formal-width.md) · OPEN
- signed-xz-queue-formal — [a signed x/z queue element bound to a formal zero-extends](roadmap/2/signed-xz-queue-formal.md) · OPEN
- inexact-placeholder-width — [a hierarchical placeholder with an inexact width keeps the old lowering route](roadmap/2/inexact-placeholder-width.md) · BLOCKED
- hier-real-call-inline — [a real-returning hierarchical call stays raw bits inline](roadmap/2/hier-real-call-inline.md) · OPEN
- hier-md-select-inline — [a multi-dim hierarchical select is unrecorded inline](roadmap/2/hier-md-select-inline.md) · OPEN
- declined-decl-walk — [a declined declaration walk keeps the leaf width](roadmap/2/declined-decl-walk.md) · OPEN
- local-stream-justify — [a local stream into a wider target right-justifies](roadmap/2/local-stream-justify.md) · OPEN
- nested-inline-subst — [a nested inline callee sees the outer formal's `subst`](roadmap/2/nested-inline-subst.md) · OPEN
- inline-sysfn-context-width — [a `$bits` term in a 64-bit function body loses the context width](roadmap/2/inline-sysfn-context-width.md) · OPEN

### Real
> Real values in integral places: real constant functions, `$signed` of a real, imported real twins; mostly open

- real-const-fn-body — [a real-returning constant function folds at no consumer](roadmap/2/real-const-fn-body.md) · LOUD
- signed-real-fn-body — [`$signed` over a real in a function body is accepted](roadmap/2/signed-real-fn-body.md) · OPEN
- real-unsigned-accepted — [`real unsigned r;` is accepted](roadmap/2/real-unsigned-accepted.md) · OPEN
- realtobits-width — [`$realtobits` / `$bitstoreal` accept a non-64-bit argument](roadmap/2/realtobits-width.md) · OPEN
- real-call-override — [an untyped real parameter given an integral call override stays real](roadmap/2/real-call-override.md) · BLOCKED
- real-param-int-view — [a real parameter's integer view answers in the integer domain](roadmap/2/real-param-int-view.md) · OPEN
- imported-real-twin — [a package real imported by name reads its i64 twin](roadmap/2/imported-real-twin.md) · OPEN
- exact-real-divides — [an exact-integer real parameter divides as an integer](roadmap/2/exact-real-divides.md) · OPEN

### Ranges / bounds / selects
> Select widths, replication counts and associative arrays that bind or read wrong, from late-folding widths to key types; mostly open

- gen-case-label-unfoldable — [an unfoldable generate-case item label is skipped silently](roadmap/2/gen-case-label-unfoldable.md) · OPEN
- size-dim-arg — [`$size` with a dimension argument answers the element width](roadmap/2/size-dim-arg.md) · OPEN
- scalar-shadows-const-array — [a scalar shadowing a const array is skipped](roadmap/2/scalar-shadows-const-array.md) · OPEN
- wide-param-select-position — [a bare >64-bit non-zero-LSB parameter select reads positionally](roadmap/2/wide-param-select-position.md) · BLOCKED
- V1b — [select widths and counts the constant domain declines are refused](roadmap/2/V1b.md) · OPEN
- late-sibling-operand-width — [an operand sized from a late-folding sibling takes 32 bits](roadmap/2/late-sibling-operand-width.md) · OPEN
- cast-late-width — [a cast over an operand whose width folds late takes 32 bits](roadmap/2/cast-late-width.md) · OPEN
- nested-zero-replication — [a zero-count replication nested in a concatenation operand runs](roadmap/2/nested-zero-replication.md) · OPEN
- negative-signed-count — [a negative two-bit signed replication count reads unsigned](roadmap/2/negative-signed-count.md) · OPEN
- unsized-concat-operand — [an unsized constant concatenation operand is accepted](roadmap/2/unsized-concat-operand.md) · OPEN
- fill-in-replication — [a fill in a constant replication's value list declines the concat](roadmap/2/fill-in-replication.md) · OPEN
- bits-hier-placeholder — [`$bits` of a hierarchical name reads a 32 placeholder before patching](roadmap/2/bits-hier-placeholder.md) · OPEN
- param-select-member-width — [a parameter select as a struct member or class property width is refused](roadmap/2/param-select-member-width.md) · OPEN
- ascending-wide-hier-select — [a hierarchical select of an ascending >64-bit parameter reads by position](roadmap/2/ascending-wide-hier-select.md) · OPEN
- overridable-type-param-range — [an overridable 1-D type parameter registers a stand-in range](roadmap/2/overridable-type-param-range.md) · BLOCKED
- type-param-standin — [a non-overridable type parameter keeps the stand-in range when unfolded](roadmap/2/type-param-standin.md) · BLOCKED
- self-ref-return-range — [a function whose return range calls itself overflows the stack](roadmap/2/self-ref-return-range.md) · OPEN
- gen-wide-localparam-select — [a >64-bit generate-scope localparam is invisible to `param_sel_range`](roadmap/2/gen-wide-localparam-select.md) · OPEN
- md-param-hier-element — [a multi-dimensional packed parameter read hierarchically answers a bit](roadmap/2/md-param-hier-element.md) · OPEN
- dead-arm-zero-width — [a zero-or-negative indexed width in a dead `?:` arm runs the live arm](roadmap/2/dead-arm-zero-width.md) · OPEN
- unreferenced-pkg-zero-width — [a zero indexed width in an unreferenced package's routine runs](roadmap/2/unreferenced-pkg-zero-width.md) · OPEN
- zero-width-refusal-optin — [row 65's zero-width refusal is opt-in, so text folded at use stays silent](roadmap/2/zero-width-refusal-optin.md) · BLOCKED
- signed-negative-width — [a signed parameter holding a negative indexed width reads unsigned](roadmap/2/signed-negative-width.md) · OPEN
- zero-width-span-library — [library-mode spans could fool row 65's zero-width check (unreachable today)](roadmap/2/zero-width-span-library.md) · RECORD
- assoc-index-locators — [`item.index` and the index locators answer the position, not the key](roadmap/2/assoc-index-locators.md) · OPEN
- foreach-assoc-index-type — [`foreach` over a byte/longint/time-keyed array types its index `integer`](roadmap/2/foreach-assoc-index-type.md) · OPEN
- time-key-signed — [a `[time]` associative key is held as a signed i64](roadmap/2/time-key-signed.md) · OPEN
- iter-var-type-mismatch — [an iteration variable smaller than the associative key is fit by value, not -1](roadmap/2/iter-var-type-mismatch.md) · OPEN
- gen-typedef-shadow-param — [a generate-block typedef named like an outer parameter reads as it](roadmap/2/gen-typedef-shadow-param.md) · OPEN
- typedef-key-scan-wide — [row 20's typedef-key name scan is file-wide and fails closed](roadmap/2/typedef-key-scan-wide.md) · OPEN
- assoc-struct-member-write — [a write to a packed-struct member of an associative element is dropped](roadmap/2/assoc-struct-member-write.md) · OPEN
- bracket-name-message — [a bracket name that names no type or value gets a misleading message](roadmap/2/bracket-name-message.md) · OPEN
- assoc-index-widened — [an associative index is widened to 64 bits, not cast to the key](roadmap/2/assoc-index-widened.md) · OPEN
- assoc-copy-key-types — [a copy between associative arrays of different key types is accepted](roadmap/2/assoc-copy-key-types.md) · OPEN
- compound-assign-index-twice — [a compound assignment evaluates its element index twice](roadmap/2/compound-assign-index-twice.md) · OPEN
- dyn-oob-reads-x — [an out-of-range read of a 2-state dynamic array or queue reads x](roadmap/2/dyn-oob-reads-x.md) · OPEN
- readmem-bad-token — [the fixed-memory `$readmem*` loader skips bad characters silently](roadmap/2/readmem-bad-token.md) · OPEN
- assoc-bit-xz-write — [an x/z write to a 2-state associative element keeps the x/z](roadmap/2/assoc-bit-xz-write.md) · OPEN
- assoc-enum-num — [`num()` of an enum-element associative array answers the label count](roadmap/2/assoc-enum-num.md) · OPEN
- assoc-concat-target — [an associative element as a concatenation-target chunk is not written](roadmap/2/assoc-concat-target.md) · OPEN
- nested-select-of-bit — [a nested part-select read of a bit-select is accepted](roadmap/2/nested-select-of-bit.md) · OPEN
- net-variable-select-assign — [a continuous assign to a net through a variable select runs, sticky](roadmap/2/net-variable-select-assign.md) · OPEN
- md-select-index-context — [an md packed parameter's select index is evaluated in the offset's context](roadmap/2/md-select-index-context.md) · OPEN
- xz-range-lsb — [an x / z or zero-divisor lsb of a declared range folds to 0](roadmap/2/xz-range-lsb.md) · OPEN
- xz-decimal-bound — [a malformed decimal x / z literal in a range bound is accepted](roadmap/2/xz-decimal-bound.md) · OPEN
- low-negative-formal — [`$low` of a function formal with a negative lsb reads 0](roadmap/2/low-negative-formal.md) · OPEN
- negated-min-bound-one-bit — [a range bound over a negated 64-bit minimum takes 1 bit](roadmap/2/negated-min-bound-one-bit.md) · OPEN
- unpacked-size-div0-dropped — [an unpacked size dimension that divides by zero is dropped](roadmap/2/unpacked-size-div0-dropped.md) · OPEN
- range-bound-real-operand — [a real or string operand of `!` or `?:` in a range bound takes 1 bit](roadmap/2/range-bound-real-operand.md) · OPEN

### Class fields
> Class property bounds, defaults and name binding inside a class body; open, one blocked on field-key normalisation

- ascending-negative-clamp — [an ascending negative bound is clamped only on a class property](roadmap/2/ascending-negative-clamp.md) · BLOCKED
- negative-ascending-index — [an index into an ascending negative packed dimension is loud](roadmap/2/negative-ascending-index.md) · OPEN
- duplicate-class-field — [a duplicate class field is accepted, last wins](roadmap/2/duplicate-class-field.md) · SPLIT
- class-field-default-width — [a class field default folds at its own width and keeps x in a 2-state field](roadmap/2/class-field-default-width.md) · OPEN
- class-scope forward use — [a name in a class body binds where it is parsed, not in the class scope](roadmap/2/class-scope-forward-use.md) · OPEN
- inline-constraint-caller-scope — [a name in an inline constraint binds at the call, not in the class](roadmap/2/inline-constraint-caller-scope.md) · OPEN
- md-class-property-select — [a packed multi-dimensional class property select reads the flat bit](roadmap/2/md-class-property-select.md) · OPEN
- extends-other-module — [a class extending a class declared in another module is accepted](roadmap/2/extends-other-module.md) · OPEN
- class-field-x-actual — [a class field's range over an x parameter actual is silent](roadmap/2/class-field-x-actual.md) · OPEN

### Scoping / imports / block-locals
> Names bound in the wrong scope: type names, imports, package routines, inlined bodies, block-locals; the largest area, mostly open

- struct-binding-by-name — [which declaration a type name denotes is read from name-keyed maps](roadmap/2/struct-binding-by-name.md) · OPEN
- declaration-order-accept — [a type name used before a later same-name declaration binds the earlier one](roadmap/2/declaration-order-accept.md) · OPEN
- union-member-pattern — [an untyped nested pattern for a packed-union member concatenates all members](roadmap/2/union-member-pattern.md) · OPEN
- typed-pattern-class-name — [a typed pattern over a class name shadowing a struct is accepted](roadmap/2/typed-pattern-class-name.md) · OPEN
- bits-record-shadow — [`$bits` of an unpacked record shadowing a packed struct answers the outer](roadmap/2/bits-record-shadow.md) · OPEN
- gen-event-shadow-param — [a generate block's `event` shadowing an outer parameter runs silently](roadmap/2/gen-event-shadow-param.md) · OPEN
- bits-flat-prescan — [`$bits` reads the flat prescan before the scope-aware net table](roadmap/2/bits-flat-prescan.md) · OPEN
- instance-named-like-package — [a call on an instance named like a package resolves to the package](roadmap/2/instance-named-like-package.md) · OPEN
- reimported-routine-binding — [a routine imported into another package never binds in its own package](roadmap/2/reimported-routine-binding.md) · OPEN
- pkg-fn-imported-const — [a package function reading an imported constant binds it in the caller](roadmap/2/pkg-fn-imported-const.md) · OPEN
- pkg-body-shadow-fallthrough — [a package body's read after a shadowing block falls to the caller](roadmap/2/pkg-body-shadow-fallthrough.md) · BLOCKED
- fork-declarations-flatten — [a `fork`'s own declarations flatten onto same-named module variables](roadmap/2/fork-declarations-flatten.md) · OPEN
- deferred-assert-block — [a declaring block in a deferred assertion's action is not hoisted](roadmap/2/deferred-assert-block.md) · OPEN
- pkg-task-sibling-call — [a package task's call of a sibling package task is not injected with it](roadmap/2/pkg-task-sibling-call.md) · OPEN
- timed-block-local-task — [a block-local under a timing control in a task or function body is E3009](roadmap/2/timed-block-local-task.md) · OPEN
- inlined-body-free-name — [a routine body lowered in the calling block binds a same-named local](roadmap/2/inlined-body-free-name.md) · OPEN
- static-task-arg-subst — [a static task substitutes an input over its whole body, locals included](roadmap/2/static-task-arg-subst.md) · OPEN
- static-task-auto-local — [a static task's `automatic` block-local is one copy for every call](roadmap/2/static-task-auto-local.md) · OPEN
- static-local-runtime-init — [a static task's static local from a run-time value initialises at first call](roadmap/2/static-local-runtime-init.md) · OPEN
- pkg-local-drops-const — [a package routine's block-local named like a package constant hides it](roadmap/2/pkg-local-drops-const.md) · OPEN
- pkg-task-const-nested — [a package task's constants in nested ranges and events bind the importer's](roadmap/2/pkg-task-const-nested.md) · OPEN
- auto-local-input-slot — [an `automatic` task's block-local named like an input shares its slot](roadmap/2/auto-local-input-slot.md) · OPEN
- local-hides-hier-census — [a block-local named like a module variable hides it from the width census](roadmap/2/local-hides-hier-census.md) · OPEN
- shared-flat-local-net — [two blocks' same-named locals share one flattened net](roadmap/2/shared-flat-local-net.md) · OPEN
- fork-auto-local-respawn — [an `automatic` local in a respawned `fork … join_none` arm shares one net](roadmap/2/fork-auto-local-respawn.md) · OPEN
- blk-scope-name-leak — [a shadowing block-local's internal scope name shows in waves and diagnostics](roadmap/2/blk-scope-name-leak.md) · OPEN
- do-while-parsed-twice — [a `do … while` body is parsed twice](roadmap/2/do-while-parsed-twice.md) · OPEN
- pkg-intra-assign-delay — [a package routine's intra-assignment delay binds the caller's names](roadmap/2/pkg-intra-assign-delay.md) · OPEN
- pkg-body-const-unhooked — [the constant domain in a package body is unhooked](roadmap/2/pkg-body-const-unhooked.md) · BLOCKED
- pkg-const-whole-write — [a whole-name write to a package constant in its function is accepted](roadmap/2/pkg-const-whole-write.md) · OPEN
- imported-routine-free-name — [a free name in an imported package routine binds the caller's net](roadmap/2/imported-routine-free-name.md) · OPEN
- scoped-call-leak-gate — [a scoped package call skips the block-local scope-leak gate](roadmap/2/scoped-call-leak-gate.md) · BLOCKED
- inline-fold-context-name — [the inline-fold lane takes §11.6.1 context by name](roadmap/2/inline-fold-context-name.md) · BLOCKED
- inline-fn-callsite-bind — [an inlined function body binds unbound names where the call is](roadmap/2/inline-fn-callsite-bind.md) · OPEN
- explicit-import-own-const — [an explicitly imported function's range naming its package constant is E3009](roadmap/2/explicit-import-own-const.md) · OPEN
- local-shadow-formal-after — [a block-local shadowing a formal is read after its block](roadmap/2/local-shadow-formal-after.md) · OPEN
- static-shadow-auto-span — [a static shadow pair beside an `automatic` span keeps the old flatten](roadmap/2/static-shadow-auto-span.md) · OPEN
- const-domain-shadow — [the constant domain is not shadow-aware](roadmap/2/const-domain-shadow.md) · OPEN
- static-frame-init-formal — [a static frame local initialised from a formal runs per call](roadmap/2/static-frame-init-formal.md) · HELD
- static-frame-init-net — [a static frame local initialised from a module net is not kept across calls](roadmap/2/static-frame-init-net.md) · OPEN
- pkg-static-local-copies — [a package routine's static local is copied per importing scope](roadmap/2/pkg-static-local-copies.md) · OPEN
- static-fn-return-read — [a static function reading its own return variable first returns 0](roadmap/2/static-fn-return-read.md) · OPEN
- method-sibling-locals — [a class method's same-named sibling block-locals are E3010](roadmap/2/method-sibling-locals.md) · OPEN
- param-net-same-name — [a parameter and a net of one name are accepted](roadmap/2/param-net-same-name.md) · OPEN
- queue-elem-part-write — [a part-select write into a queue element vanishes](roadmap/2/queue-elem-part-write.md) · OPEN
- typedef-dims-reresolve — [a typedef's named dimensions re-resolve at each use](roadmap/2/typedef-dims-reresolve.md) · OPEN
- modport-import-name — [a `modport` named like an imported symbol is accepted](roadmap/2/modport-import-name.md) · SPLIT
- call-block-label-routine — [a call naming a nested block label that is also a routine is silent](roadmap/2/call-block-label-routine.md) · OPEN
- type-param-beside-wire — [`parameter type T` beside `wire T` is accepted](roadmap/2/type-param-beside-wire.md) · HELD
- pkg-pairs-unmeasured — [package typedef, class and enum-label pairs against non-routines, unmeasured](roadmap/2/pkg-pairs-unmeasured.md) · RECORD
- typedef-enum-refusals — [typedef and enum-label refusals rest on one oracle](roadmap/2/typedef-enum-refusals.md) · RECORD
- zero-width-part-select — [a width-0 indexed part-select is accepted](roadmap/2/zero-width-part-select.md) · RECORD
- string-key-funnel — [a string-keyed associative index skips the §6.16 funnel](roadmap/2/string-key-funnel.md) · OPEN
- fn-block-named-instance — [a function-local block named like an instance loses to the instance](roadmap/2/fn-block-named-instance.md) · OPEN
- import-ambiguity-adjacency — [import ambiguity follows adjacency, not the reference's position](roadmap/2/import-ambiguity-adjacency.md) · OPEN
- unit-module-import-scope — [`$unit` imports and a module's imports are one import scope](roadmap/2/unit-module-import-scope.md) · OPEN
- wildcard-string-candidate — [a wildcard string or real package constant is no import candidate](roadmap/2/wildcard-string-candidate.md) · OPEN
- explicit-import-conflict — [two explicit imports of one type or function name take the second](roadmap/2/explicit-import-conflict.md) · OPEN
- iterator-param-range — [an array-method iterator is read through a same-named parameter's range](roadmap/2/iterator-param-range.md) · OPEN
- gen-real-param-shadow — [a generate `real` parameter does not shadow an outer integral one](roadmap/2/gen-real-param-shadow.md) · BLOCKED
- gen-param-later-walk — [a later generate walk reads a parameter declared after its reference](roadmap/2/gen-param-later-walk.md) · OPEN
- generate-param-overridable — [a generate-block parameter is treated as overridable](roadmap/2/generate-param-overridable.md) · RECORD
- const-fn-local-unbound-width — [a header default's constant function sizes a local by an unbound body localparam](roadmap/2/const-fn-local-unbound-width.md) · OPEN
- generated-name-collision — [a generated name a user identifier can spell collides with it](roadmap/2/generated-name-collision.md) · OPEN
- gen-param-self-ref-refold — [a generate parameter naming an outer one of its name reads its own value](roadmap/2/gen-param-self-ref-refold.md) · OPEN
- const-fn-range-formal-shadow — [a local's range naming a formal or local reads the module parameter](roadmap/2/const-fn-range-formal-shadow.md) · OPEN
- duplicate-value-name-accepted — [32 measured same-name pairs holding an enum label or a `typedef` are accepted](roadmap/2/duplicate-value-name-accepted.md) · OPEN

### Delays / events
> Event, wait and delay scheduling: assertion control, heap-container waits, time-0 order, delay rounding; mostly open, wake order blocked

- ① auto-local-fork-delay — [a loop-body `automatic` read by a `fork` arm's delay takes the last value](roadmap/2/auto-local-fork-delay.md) · OPEN
- ① assertoff-immediate — [`$assertoff` and its siblings reach only the concurrent checkers' reports](roadmap/2/assertoff-immediate.md) · OPEN
- assertoff-action-blocks — [a concurrent assertion's own action blocks run while assertions are off](roadmap/2/assertoff-action-blocks.md) · OPEN
- at-star-dyn-handle — [an in-body `@(*)` over a dynamic handle stays stale](roadmap/2/at-star-dyn-handle.md) · OPEN
- wait-heap-container — [a `wait` over a heap container's element or method never wakes](roadmap/2/wait-heap-container.md) · OPEN
- task-local-assoc-word — [an associative array declared in a static task's body is one word](roadmap/2/task-local-assoc-word.md) · OPEN
- zero-delay-net-read — [a run-time delay net driven by `#0` is read before it lands](roadmap/2/zero-delay-net-read.md) · OPEN
- multidriven-delay-dropped — [a resolved multi-driven `wire` drops a run-time delay](roadmap/2/multidriven-delay-dropped.md) · OPEN
- vm-task-string-formal — [the VM loses a static task's `string` formal](roadmap/2/vm-task-string-formal.md) · OPEN
- delay-rounding-leaf — [delay rounding of a real leaf against a sub-precision unit leaf](roadmap/2/delay-rounding-leaf.md) · SPLIT
- negative-delay-fires — [a negative real-time delay fires at once](roadmap/2/negative-delay-fires.md) · BLOCKED
- multi-timescale-rounding — [multi-timescale rounding precision](roadmap/2/multi-timescale-rounding.md) · SPLIT
- t0-copy-net-move — [a time-0 copy net takes only its own move](roadmap/2/t0-copy-net-move.md) · OPEN
- net-array-dirty-channel — [an unpacked net array is one net on the dirty channel](roadmap/2/net-array-dirty-channel.md) · OPEN
- level-edge-bit0-wake — [a multi-bit level term beside an edge term wakes on bit 0 only](roadmap/2/level-edge-bit0-wake.md) · OPEN
- ca-lhs-runtime-index — [a non-constant continuous-assign target index is accepted](roadmap/2/ca-lhs-runtime-index.md) · OPEN
- fatal-drops-nba — [`$fatal` drops its step's pending nonblocking update](roadmap/2/fatal-drops-nba.md) · HELD
- const-select-never-wakes — [a select indexed through a concat, system call or hierarchical name never wakes](roadmap/2/const-select-never-wakes.md) · OPEN
- t0-body-const-run — [unadmitted time-0 bodies drop a constant's time-0 run](roadmap/2/t0-body-const-run.md) · OPEN
- header-level-rerun — [the header `Level` waiter re-runs a pending process](roadmap/2/header-level-rerun.md) · OPEN
- vm-runtime-delay-call — [a run-time delay call runs silently on the VM](roadmap/2/vm-runtime-delay-call.md) · OPEN
- ca-time-scale — [a continuous assign in another time unit reads `$time` at the wrong scale](roadmap/2/ca-time-scale.md) · OPEN
- ca-hop-after-batch — [a continuous-assign hop lands after the whole batch](roadmap/2/ca-hop-after-batch.md) · BLOCKED
- edge-wait-earlier-edge — [an in-body edge wait sees an earlier edge of its batch](roadmap/2/edge-wait-earlier-edge.md) · OPEN

### Diagnostics / artifacts
> Wrong or missing messages and artifacts: panics, misleading refusal reasons, waveform scope names, `run.json`; open or recorded

- ① strobe-automatic-panic — [`$strobe` / `$monitor` of an `automatic` task variable panics](roadmap/2/strobe-automatic-panic.md) · OPEN
- zero-width-message-shadow — [a zero-width refusal names the width zero where a shadow hides the name](roadmap/2/zero-width-message-shadow.md) · OPEN
- ascending-hier-bounds-order — [a late-folding part-select of an ascending hierarchical net says out of order](roadmap/2/ascending-hier-bounds-order.md) · OPEN
- replication-count-cap — [a replication count above 2^24 repeats 2^24 times](roadmap/2/replication-count-cap.md) · RECORD
- mul-chain-diag-copies — [a re-lowered multiply chain base emits one E4002 per lowering](roadmap/2/mul-chain-diag-copies.md) · OPEN
- coverpoint-pow-width — [`coverpoint_domain`'s power arm uses the wider operand width](roadmap/2/coverpoint-pow-width.md) · OPEN
- run-json-omits-g — [`run.json` omits `-G`](roadmap/2/run-json-omits-g.md) · OPEN
- ca-function-reruns — [a continuous assign's user function runs more than once per event](roadmap/2/ca-function-reruns.md) · BLOCKED
- operator-call-twice — [an operator over a call names it twice](roadmap/2/operator-call-twice.md) · BLOCKED
- wprog-decline-relabel — [`wprog` decline reasons relabelled by §4.5.530](roadmap/2/wprog-decline-relabel.md) · RECORD
- hex-one-bit-unknown — [the letter case `%h` prints for a 1-bit unknown expression](roadmap/2/hex-one-bit-unknown.md) · RECORD
- string-nul-byte — [a string literal's NUL byte is kept raw](roadmap/2/string-nul-byte.md) · OPEN
- dumpfile-param-name — [`$dumpfile` of a parameter names the file after the parameter path](roadmap/2/dumpfile-param-name.md) · OPEN
- shadowed-member-message — [a member read under a shadowing constant gives the generic E3010](roadmap/2/shadowed-member-message.md) · OPEN
- velab-mtime-stamp — [a work-library `.velab` stores unit mtimes, so its bytes differ run to run](roadmap/2/velab-mtime-stamp.md) · RECORD
- velab-cu-dedupe — [`velab -L` drops a second unit's diagnostic at the same offset](roadmap/2/velab-cu-dedupe.md) · OPEN
- zero-width-gate-after-error — [an earlier error turns the zero-width line into a false cause](roadmap/2/zero-width-gate-after-error.md) · OPEN
- stale-key-refused-spelling — [a prebuilt artifact passes a spelling the parser now refuses](roadmap/2/stale-key-refused-spelling.md) · RECORD
- sized-literal-separator-read — [a sized literal's size reader keeps a separator other readers strip](roadmap/2/sized-literal-separator-read.md) · OPEN
- sysfn-zero-arg-panic — [`$clog2()` or `$signed()` with no argument panics at run time](roadmap/2/sysfn-zero-arg-panic.md) · OPEN
- pkg-param-no-cause — [a package parameter that does not fold prints no cause or position](roadmap/2/pkg-param-no-cause.md) · OPEN
- override-recovery-reread — [a refused override's recovery value is read by a second diagnostic](roadmap/2/override-recovery-reread.md) · OPEN
- param-range-net-name-text — [a localparam's range naming a net calls it an undefined name](roadmap/2/param-range-net-name-text.md) · OPEN

### Performance (open, recorded)
> Correct but slow paths: repeated continuous-assign calls, per-bit coercion, triple tree walks, `==?` chains; recorded, one blocked

- ca-call-reevaluates — [a continuous assign with any call re-evaluates six times](roadmap/2/ca-call-reevaluates.md) · BLOCKED
- per-bit-coerce — [2-state coercion still runs bit by bit (`coerce_two_state`)](roadmap/2/per-bit-coerce.md) · OPEN (part)
- operand-width-coerce — [coercing at the operand's width is ~10× faster where it is declared](roadmap/2/operand-width-coerce.md) · PERF
- size-cast-seal-wprog — [a size-cast seal falls out of `wprog` on three axes (no compile arm)](roadmap/2/size-cast-seal-wprog.md) · PERF
- const-walk-three-times — [constant width and sign walk the tree three times](roadmap/2/const-walk-three-times.md) · PERF
- wildcard-eq-chain-cost — [a left-leaning `==?` chain costs 2^depth](roadmap/2/wildcard-eq-chain-cost.md) · PERF
- nterm-sum-localparam-cost — [an n-term `1+…+1` localparam costs superlinear time](roadmap/2/nterm-sum-localparam-cost.md) · PERF
- bound-over-i32-folds-twice — [a declared bound above `i32::MAX` runs its constant function twice](roadmap/2/bound-over-i32-folds-twice.md) · PERF
- instarr-prepass-map-clone — [the instance-array prepass clones eight whole parameter maps per array](roadmap/2/instarr-prepass-map-clone.md) · PERF

### Oracle splits (recorded, not chased)
> Cases where the oracles disagree or none exists, with the side vita takes; recorded, not chased

- explicit-after-wildcard-type — [an explicit import after a wildcard import of the same type name](roadmap/2/explicit-after-wildcard-type.md)
- let-shadow-width — [a `let` declaration shadowing an outer parameter read as a constant width](roadmap/2/let-shadow-width.md)
- negative-signed-width-operand — [a negative signed width operand](roadmap/2/negative-signed-width-operand.md)
- static-local-loop-read — [a static function's local read before its write in a loop, across calls](roadmap/2/static-local-loop-read.md)
- select-whole-waiter-order — [select vs whole-net level waiter order](roadmap/2/select-whole-waiter-order.md)
- deferred-assert-maturity — [deferred-assertion report maturity](roadmap/2/deferred-assert-maturity.md)
- real-into-string — [a real stored into a string](roadmap/2/real-into-string.md)
- t0-no-definite-driver — [time-0 wake and edge of a driver with no definite bit](roadmap/2/t0-no-definite-driver.md)
- zero-delay-ca-visibility — [zero-delay continuous-assign update visibility](roadmap/2/zero-delay-ca-visibility.md)
- t0-const-always-order — [time-0 Active order around an all-constant `always @(K)`](roadmap/2/t0-const-always-order.md)
- after-finish-step — [what runs after `$finish` in its step](roadmap/2/after-finish-step.md)
- packed-dims-unpacked-typedef — [packed dims before an unpacked typedef name](roadmap/2/packed-dims-unpacked-typedef.md)
- untyped-init-width — [init width of an untyped sum that carries out](roadmap/2/untyped-init-width.md)
- unsigned-64-modulo — [64-bit unsigned `%`](roadmap/2/unsigned-64-modulo.md)
- cross-scope-init-race — [cross-scope time-0 declaration-init race](roadmap/2/cross-scope-init-race.md)
- nan-delayed-ca — [a delayed continuous assign re-evaluated with identical NaN bits](roadmap/2/nan-delayed-ca.md)
- delayed-ca-rise-fall — [an integral delayed continuous assignment's rise / fall](roadmap/2/delayed-ca-rise-fall.md)
- z-into-real-delay — [z into a `real` destination under rise, fall and turn-off delays](roadmap/2/z-into-real-delay.md)
- stime-cast-sign — [`$stime` sign under a cast](roadmap/2/stime-cast-sign.md)
- cross-pkg-const-recursion — [cross-package const-function recursion](roadmap/2/cross-pkg-const-recursion.md)
- string-override-w3056 — [a string parameter override prints one spurious W3056](roadmap/2/string-override-w3056.md)
- untyped-minus-narrow — [untyped `K - 20` over `bit [3:0]`](roadmap/2/untyped-minus-narrow.md)
- real-bitvector-inline — [real beside a bit-vector inline region](roadmap/2/real-bitvector-inline.md)
- vl-import-order — [import precedence](roadmap/2/vl-import-order.md)
- gen-forward-references — [forward references in a generate scope](roadmap/2/gen-forward-references.md)
- inline-task-enum-label — [a static inline task's body-local enum label beside a module constant](roadmap/2/inline-task-enum-label.md)
- interface-use-before-decl — [use before declaration in an interface](roadmap/2/interface-use-before-decl.md)
- pkg-const-override-ascending — [a package-constant override over an ascending range](roadmap/2/pkg-const-override-ascending.md)
- unstored-string-cast — [`%s` of an unstored `string'(e)`](roadmap/2/unstored-string-cast.md)
- string-cast-spellings — [string-cast spellings with at most one oracle](roadmap/2/string-cast-spellings.md)
- always-star-t0 — [`always @*` at time 0, and one reading only a constant](roadmap/2/always-star-t0.md)
- gen-unlabelled-begin — [unlabelled `begin` in a generate region](roadmap/2/gen-unlabelled-begin.md)
- gen-net-named-function — [unlabelled generate net beside a same-named function](roadmap/2/gen-net-named-function.md)
- repeated-nonansi-port — [repeated non-ANSI header port](roadmap/2/repeated-nonansi-port.md)
- nonfinite-real-integral — [non-finite / out-of-range real into an integral](roadmap/2/nonfinite-real-integral.md)
- override-equals-default — [an override equal to the default](roadmap/2/override-equals-default.md)
- hier-select-scaled-param — [a hierarchical select sized by a scaled hierarchical parameter](roadmap/2/hier-select-scaled-param.md)
- bits-hier-real — [`$bits` of a hierarchical real](roadmap/2/bits-hier-real.md)
- const-term-suspending — [a constant event term beside a live one, in a suspending body](roadmap/2/const-term-suspending.md)
- const-term-edge — [a constant event term beside an edge term](roadmap/2/const-term-edge.md)
- wait-on-constant — [an in-body wait on a constant](roadmap/2/wait-on-constant.md)
- named-event-constant — [a named event beside a constant](roadmap/2/named-event-constant.md)
- const-equals-default-event — [a constant equal to a variable's default as an event term](roadmap/2/const-equals-default-event.md)
- t0-nba-live-term — [a time-0 nonblocking or `#0` write of a live term](roadmap/2/t0-nba-live-term.md)
- glitch-in-body-wait — [a same-step glitch under an in-body wait](roadmap/2/glitch-in-body-wait.md)
- net-only-level-list — [a net-only header level list at time 0](roadmap/2/net-only-level-list.md)
- same-time-resume-order — [same-time resume order](roadmap/2/same-time-resume-order.md)
- strobe-before-monitor — [`$strobe` before the first monitored change](roadmap/2/strobe-before-monitor.md)
- g-decimal-overflow — [a `-G` decimal beyond 32-bit signed](roadmap/2/g-decimal-overflow.md)
- sign-keyword-real — [a sign keyword with no range beside a real](roadmap/2/sign-keyword-real.md)
- realtobits-integral — [`$realtobits` of an integral](roadmap/2/realtobits-integral.md)
- pkg-enum-label-const — [a package-body enum label named like a constant](roadmap/2/pkg-enum-label-const.md)
- negative-part-select-bound — [a negative signed part-select bound](roadmap/2/negative-part-select-bound.md)
- comb-t0-pass-order — [time-0 passes of several `always_comb` / `always_latch` blocks](roadmap/2/comb-t0-pass-order.md)
- comb-read-before-pass — [a declaration-fed `always_comb` read at time 0 before its pass](roadmap/2/comb-read-before-pass.md)
- comb-t0-run-count — [`always_comb` run count at time 0 when an input changes at time 0](roadmap/2/comb-t0-run-count.md)
- gated-clock-edge-dedup — [gated-clock edge dedup at time 0](roadmap/2/gated-clock-edge-dedup.md)
- implicit-pass-zero-resume — [an implicit time-0 pass's wakes against a promoted `#0` resume](roadmap/2/implicit-pass-zero-resume.md)
- comb-port-chain-slots — [comb time-0 pass slots in a port chain, a checker and a named event](roadmap/2/comb-port-chain-slots.md)
- self-timed-before-initial — [a self-timed `always` written before the `initial` that drives it](roadmap/2/self-timed-before-initial.md)
- held-assign-t0-read — [a time-0 read of a held continuous assign's output before release](roadmap/2/held-assign-t0-read.md)
- overcap-typedef-bits — [`$bits` of a typedef wider than the v1 cap gives a value](roadmap/2/overcap-typedef-bits.md)
- typed-signed-bound-oracle-split — [verilator and sv2v read a typed signed negative range bound as unsigned](roadmap/2/typed-signed-bound-oracle-split.md)
- const-product-range-split — [a range bound over a product that overflows 64 bits](roadmap/2/const-product-range-split.md)
- untyped-param-product-width — [an untyped parameter over a product that overflows 64 bits](roadmap/2/untyped-param-product-width.md)

## 3. loud → correct-support candidates (all loud = safe, additive)
> Loud shapes that could become correct support: numbered items (3.a), small residues by area (3.b) and shapes kept loud on purpose (3.c).

The workload corpus has no refused row; the row-9 census's roots on the new designs' pages are §5.2's real-design program. ([notes](roadmap/3/README.md))

### 3.a Numbered open items
> 20 of 29 open: file-read hoisting ③, array / type parameters ⑤, function bodies ⑧, imported constants ⑨, diagnostic locations ⑬, profiling ⑭, DPI-C ⑮

- ③ⓐ — [a file-read call inside `&&`, `||` or `?:` cannot be hoisted](roadmap/3.a/3a.md) · OPEN
- ③ⓑ — [a call in a `while` / `for` condition, to be rewritten into the loop body](roadmap/3.a/3b.md) · OPEN
- ③ⓒ — [every statement a `$feof` survives into is refused](roadmap/3.a/3c.md) · BLOCKED
- ③ⓓ — [the overlap judge does not see alias names such as `m.a` or `p::v`](roadmap/3.a/3d.md) · OPEN
- ⑤ⓜ — [ibex's genvar-indexed hierarchical path `g_pmp_csrs[i_region].x`](roadmap/3.a/5m.md) · OPEN
- ⑤ⓐ — [multi-packed array parameters and two wildcard imports of one name](roadmap/3.a/5a.md) · OPEN
- ⑤ⓒ — [header array parameters: nested override, wide element, `defparam`, interface](roadmap/3.a/5c.md) · OPEN
- ⑤ⓔ — [element select residues: multi-packed element, `p::S[1].b`, runtime `$size`](roadmap/3.a/5e.md) · OPEN
- ⑤ⓕ — [unpacked-array typedefs, one consumer at a time](roadmap/3.a/5f.md) · OPEN (part)
- ⑤ⓓ — [nested struct members: defaults, nested `.name()`, array members, named widths](roadmap/3.a/5d.md) · OPEN
- ⑤ⓝ — [compilation-unit scope: unit variables, enum labels, forward constants](roadmap/3.a/5n.md) · OPEN
- ⑤ⓞ — [parser and preprocessor gaps: md packed formals, narrow literals, macro formals](roadmap/3.a/5o.md) · OPEN
- ⑤ⓟ — [a `parameter type` given an enum type is rejected by the parser](roadmap/3.a/5p.md) · OPEN
- ⑤ⓠ — [a `parameter type` given a struct, union, real, string or class type](roadmap/3.a/5q.md) · HELD
- ⑤ⓡ — [a `parameter type` whose default is a struct or enum is rejected](roadmap/3.a/5r.md) · LOUD
- ⑤ⓢ — [a union member of a symbolic-layout or type-parameter type is rejected](roadmap/3.a/5s.md) · LOUD
- ⑤ⓣ — [a `parameter type` declared inside a routine or block is rejected at parse](roadmap/3.a/5t.md) · LOUD
- ⑧ⓐ — [system functions in a function body: the read-dependency walk names no seed](roadmap/3.a/8a.md) · OPEN
- ⑧ⓑ — [the statement after a function's `$finish` still runs](roadmap/3.a/8b.md) · OPEN
- ⑧ⓒ — [`$finish` inside an output-formal function takes a separate route](roadmap/3.a/8c.md) · OPEN
- ⑧ⓓ — [a counter-carrying function is one evaluation off (an extra time-0 settle)](roadmap/3.a/8d.md) · OPEN
- ⑨ⓐ — [a bare imported string or real parameter is refused](roadmap/3.a/9a.md) · OPEN
- ⑨ⓑ — [a real package constant in a `generate if` condition is refused](roadmap/3.a/9b.md) · OPEN
- ⑬ⓐ — [a subroutine body's array access diagnostic is attributed to the call](roadmap/3.a/13a.md) · OPEN
- ⑬ⓑ — [terminator, settle and time-0 drain diagnostics have no location](roadmap/3.a/13b.md) · HELD
- ⑬ⓒ — [several runtime warnings and the delta limit lack a source location](roadmap/3.a/13c.md) · OPEN
- ⑭ⓐ — [an inlined subroutine reports zero calls](roadmap/3.a/14a.md) · BLOCKED
- ⑭ⓑ — [a reporter's 21× scheduler gap is a speed question, not observability](roadmap/3.a/14b.md) · DEFERRED
- ⑮ — [DPI-C: `import` / `export "DPI-C"` and loading a C shared library](roadmap/3.a/15.md) · DEFERRED

### 3.b Small residues
> Named residues by area (parser, constants, multidriver, frames, system tasks, nets, diagnostics, strings); most open, some wait on §5.2 rows.

- ret-select-residue — [select writes to an inlined function's return variable outside the merged shapes](roadmap/3.b/ret-select-residue.md) · OPEN
- class-typedef — [a `typedef` inside a class and a `typedef class` forward declaration](roadmap/3.b/class-typedef.md) · OPEN
- with-outer-iterator — [a nested `with` clause cannot read the outer iterator](roadmap/3.b/with-outer-iterator.md) · OPEN
- typedef-cast-real-const — [a constant typedef cast of a real value does not fold](roadmap/3.b/typedef-cast-real-const.md) · BLOCKED

**Constant edges (§4.5.601)**

- bits-select-edge — [`$bits` of a select inside a part-select bound does not fold](roadmap/3.b/bits-select-edge.md) · OPEN
- late-target-capture — [fills and delayed captures into a target whose width folds after elaboration](roadmap/3.b/late-target-capture.md) · OPEN
- over-max-width — [a computed part-select width above 2^20 bits is refused](roadmap/3.b/over-max-width.md) · OPEN

**Parser accept**

- non-ansi-md-port — [a non-ANSI port declared with a multi-dimensional typedef is rejected](roadmap/3.b/non-ansi-md-port.md) · OPEN
- specify — [`specify` blocks are rejected by the parser](roadmap/3.b/specify.md) · OPEN
- case-inside-residue — [`case … inside` shapes the case-inside lowering still refuses](roadmap/3.b/case-inside-residue.md) · OPEN
- inside-name-use — [a design using `inside` as a name refuses every `case … inside`](roadmap/3.b/inside-name-use.md) · OPEN
- pkg-scoped-task — [a package-scoped task call `pk::t()` as a statement is rejected](roadmap/3.b/pkg-scoped-task.md) · OPEN
- time-signed-param — [`parameter time signed` is rejected by the parser](roadmap/3.b/time-signed-param.md) · OPEN
- param-time-unsigned — [a signing keyword after `time` on a parameter is refused](roadmap/3.b/param-time-unsigned.md) · OPEN
- begin-keywords-version — [a `begin_keywords` version does not narrow the keyword set](roadmap/3.b/begin-keywords-version.md) · OPEN
- based-ws — [a space inside a based literal (`64'sh FFFF`) is a lexer error](roadmap/3.b/based-ws.md) · OPEN
- tf-localparam — [`localparam` inside a task body is rejected](roadmap/3.b/tf-localparam.md) · OPEN
- blk-automatic — [an `automatic` declaration in a block of a task body is rejected](roadmap/3.b/blk-automatic.md) · OPEN
- tf-decl-lifetime — [a `static` declaration after a task header is rejected](roadmap/3.b/tf-decl-lifetime.md) · OPEN
- class-tf-port — [a class-typed task or function port is rejected](roadmap/3.b/class-tf-port.md) · OPEN
- R30-1 — [a missing package gives seven unnamed parse errors](roadmap/3.b/R30-1.md) · OPEN
- enum-label — [an enum label wider than its base skips its enum slot and truncates](roadmap/3.b/enum-label.md) · OPEN
- md-packed-write — [nested part-select writes into multi-dimensional packed arrays](roadmap/3.b/md-packed-write.md) · OPEN
- misc-parse — [parser odds: negative-LSB member select, generate `import`, `'{k:v}`, a panic](roadmap/3.b/misc-parse.md) · OPEN
- edge-event — [`@(edge clk)` is rejected by the parser](roadmap/3.b/edge-event.md) · OPEN
- eh1-assert-on — [VeeR EH1 with `ASSERT_ON` stops at its concurrent properties](roadmap/3.b/eh1-assert-on.md) · OPEN
- cover-immediate — [`cover` in a procedure is E2002 and at module level gets the assert message](roadmap/3.b/cover-immediate.md) · OPEN
- assoc-typed-key-loud — [typed associative-array keys that stay refused by name](roadmap/3.b/assoc-typed-key-loud.md) · OPEN
- parser-fold-shift-pow — [the parser's constant fold lacks `>>`, `<<` and `**`](roadmap/3.b/parser-fold-shift-pow.md) · OPEN
- member-subsel-named-bound — [a struct member part-select with named bounds, read or written](roadmap/3.b/member-subsel-named-bound.md) · OPEN
- pattern-repl — [a replication `N{…}` inside an assignment pattern is rejected](roadmap/3.b/pattern-repl.md) · OPEN
- typed-pattern-residue — [typed assignment patterns `T'{…}` beyond an unsigned packed struct](roadmap/3.b/typed-pattern-residue.md) · OPEN
- md-struct-member-select — [a member select after a two-level index into a packed struct array](roadmap/3.b/md-struct-member-select.md) · OPEN
- pkg-export — [a package `export p::name;` / `export *::*;` is rejected](roadmap/3.b/pkg-export.md) · OPEN
- typedef-packed-dims — [packed dimensions on a typedef'd type outside the composed forms](roadmap/3.b/typedef-packed-dims.md) · OPEN
- member-width-call — [a package struct member width that calls a constant function](roadmap/3.b/member-width-call.md) · OPEN
- cg-block-local — [a covergroup block local declared with an initializer is rejected](roadmap/3.b/cg-block-local.md) · DEFERRED
- empty-param-list — [an empty parameter port list `#()` is rejected](roadmap/3.b/empty-param-list.md) · DEFERRED
- anon-member-dims — [packed dimensions after an anonymous struct or union member body](roadmap/3.b/anon-member-dims.md) · OPEN
- member-width-genvar — [a struct member width over a genvar in a generate-block typedef](roadmap/3.b/member-width-genvar.md) · OPEN
- anon-enum-member — [an anonymous `enum` as a struct member is rejected](roadmap/3.b/anon-enum-member.md) · OPEN
- sym-member-user-cast — [a cast in a symbolic struct member's range is refused](roadmap/3.b/sym-member-user-cast.md) · OPEN
- empty-positional-argument — [an empty positional argument `f(1, , 3)` is a parse error](roadmap/3.b/empty-positional-argument.md) · OPEN

**Constants / parameters**

- override-sysfn-value — [a system-function override whose value does not fold is refused](roadmap/3.b/override-sysfn-value.md) · OPEN
- override-bits-asc — [`$bits` of an ascending package constant does not fold](roadmap/3.b/override-bits-asc.md) · OPEN
- override-concat-name — [a concatenation of package names as an override does not fold](roadmap/3.b/override-concat-name.md) · OPEN
- §3.3 — [a wide `localparam` part-select inside a concatenation does not fold](roadmap/3.b/-3.3.md) · OPEN
- real-fold — [real `localparam` arithmetic does not fold](roadmap/3.b/real-fold.md) · OPEN
- xz-fill-param — [x / z bits in a parameter value or override are lost](roadmap/3.b/xz-fill-param.md) · OPEN
- string-default-numeric-override — [a numeric override of a string-default parameter is refused](roadmap/3.b/string-default-numeric-override.md) · OPEN
- string-const-operand — [a string literal operand in the integer constant fold is refused](roadmap/3.b/string-const-operand.md) · OPEN
- real-int-overflow — [a real constant too large for an `int` parameter is refused](roadmap/3.b/real-int-overflow.md) · OPEN
- wide-untyped-consumer — [an untyped parameter copied from a parameter wider than 64 bits](roadmap/3.b/wide-untyped-consumer.md) · OPEN
- override-signing-operand — [a `$signed` division as a parameter override does not fold](roadmap/3.b/override-signing-operand.md) · OPEN
- real-param-hier — [a hierarchical read of a real parameter is refused](roadmap/3.b/real-param-hier.md) · OPEN
- compound-==? — [constant-fold residue of the wildcard equality `==?`](roadmap/3.b/compound----.md) · OPEN
- defparam-iface — [`defparam` onto an interface instance matches nothing](roadmap/3.b/defparam-iface.md) · OPEN
- neg-ascending — [an ascending negative range `[-33:-2]` gives `$bits` 1](roadmap/3.b/neg-ascending.md) · OPEN
- neg-bound-part — [a part select with negative bounds is blocked](roadmap/3.b/neg-bound-part.md) · OPEN
- neg-elem-bound — [a negative packed range on a queue element clamps](roadmap/3.b/neg-elem-bound.md) · OPEN
- mdrv-partial — [the multidriver check ignores partial writes](roadmap/3.b/mdrv-partial.md) · RECORD
- mdrv-ca-force — [`force` over a variable's continuous `assign` is no second driver](roadmap/3.b/mdrv-ca-force.md) · RECORD
- mdrv-ca-call — [task-body, hierarchical and system-task writes escape the multidriver rule](roadmap/3.b/mdrv-ca-call.md) · OPEN
- mdrv-ca-port — [port, generate-block and interface drivers escape the multidriver rule](roadmap/3.b/mdrv-ca-port.md) · OPEN
- mdrv-ca-array — [a whole-array `assign` beside another writer gets the wrong error code](roadmap/3.b/mdrv-ca-array.md) · RECORD
- mdrv-shadow-scope — [a same-named block local hides a whole process from the multidriver check](roadmap/3.b/mdrv-shadow-scope.md) · OPEN
- mdrv-gen-region — [a bare `generate` region is skipped by the multidriver check](roadmap/3.b/mdrv-gen-region.md) · OPEN
- decl-var-net-kw — [`wire logic`, `var` declarations, drive strengths and `alias` are rejected](roadmap/3.b/decl-var-net-kw.md) · OPEN
- mdrv-assert-local — [a local in an assertion action block is flattened onto the module variable](roadmap/3.b/mdrv-assert-local.md) · OPEN
- cont-var-partial — [a continuous `assign` to part of a non-`logic` variable is refused](roadmap/3.b/cont-var-partial.md) · OPEN
- cont-var-force — [`force` / `release` over a continuous `assign` to a non-`logic` variable](roadmap/3.b/cont-var-force.md) · OPEN
- mdrv-ca-hier — [a hierarchical continuous `assign` to a variable is never a recorded driver](roadmap/3.b/mdrv-ca-hier.md) · OPEN
- cont-var-gate — [a built-in gate driving a non-`logic` variable is refused](roadmap/3.b/cont-var-gate.md) · SPLIT
- cont-var-string — [a continuous `assign` to a `string` or class handle is refused](roadmap/3.b/cont-var-string.md) · SPLIT
- mdrv-hier-actual — [a hierarchical task call in `always_comb` is a false multidriver error](roadmap/3.b/mdrv-hier-actual.md) · OPEN
- frame-body-write-sites — [call sites of a body-writing frame function that cannot carry the call](roadmap/3.b/frame-body-write-sites.md) · OPEN
- frame-body-write-order — [a hoisted body-writing call runs before a read to its left](roadmap/3.b/frame-body-write-order.md) · SPLIT
- dyn-size-spellings — [`$size` of a dynamic array through a field, hierarchy or formal](roadmap/3.b/dyn-size-spellings.md) · OPEN
- dyn-bits-count — [`$bits` of a dynamic array folds the element width only](roadmap/3.b/dyn-bits-count.md) · OPEN
- pkg-type-param-import — [an explicit import of a package `parameter type` is refused](roadmap/3.b/pkg-type-param-import.md) · DO-NOT-START
- gen-rtn-edges — [calls into generate-block routines by hierarchy or in a constant](roadmap/3.b/gen-rtn-edges.md) · OPEN
- aes§2 — [the inliner needs a discriminator beyond `automatic`](roadmap/3.b/aes-2.md) · OPEN
- gen-enum-uncarried — [a generate-block `typedef enum` binds only where it is carried](roadmap/3.b/gen-enum-uncarried.md) · BLOCKED
- string-literal-condition-residue — [a string parameter in a generate condition is refused](roadmap/3.b/string-literal-condition-residue.md) · OPEN
- md-param-pattern-residue — [patterns into a multi-dimensional packed parameter beyond literal items](roadmap/3.b/md-param-pattern-residue.md) · OPEN
- md-packed-pattern — [an assignment pattern into a multi-dimensional packed array is refused](roadmap/3.b/md-packed-pattern.md) · OPEN
- cont-array-typedef-residue — [a whole-array continuous assign with an enum or named-bound typedef element](roadmap/3.b/cont-array-typedef-residue.md) · OPEN
- cont-array-residue — [a whole-array continuous assign with a delay, slice, `?:` or second writer](roadmap/3.b/cont-array-residue.md) · OPEN
- packed-default-residue — [packed `'{default: v}` beyond plain vector targets](roadmap/3.b/packed-default-residue.md) · OPEN
- const-fn-case — [a constant function containing any `case` statement does not fold](roadmap/3.b/const-fn-case.md) · BLOCKED
- const-fn-systask — [`$display` and other system tasks inside a constant function](roadmap/3.b/const-fn-systask.md) · BLOCKED
- pkg-param-own-fn — [a package parameter calling a function of its own package](roadmap/3.b/pkg-param-own-fn.md) · OPEN
- pkg-text-open — [package routine text naming its own constants, called from outside](roadmap/3.b/pkg-text-open.md) · BLOCKED
- bits-select-rest — [`$bits` of a select in the shapes the constant domain still declines](roadmap/3.b/bits-select-rest.md) · OPEN
- bits-ansi-port — [`$bits` of a whole ANSI port inside a `localparam`](roadmap/3.b/bits-ansi-port.md) · OPEN
- zero-repl-concat — [a zero-count replication inside a constant concatenation](roadmap/3.b/zero-repl-concat.md) · OPEN
- enum-label-domain — [enum labels that the 64-bit label fold cannot hold](roadmap/3.b/enum-label-domain.md) · OPEN
- param-fits-w63 — [a 63-bit unsigned parameter fits no value in the parser's check](roadmap/3.b/param-fits-w63.md) · OPEN
- const-fn-if-x — [a constant function's `if` on an x condition declines](roadmap/3.b/const-fn-if-x.md) · OPEN
- const-fn-wide-local-truth — [a function local over 64 bits as a `?:` condition declines](roadmap/3.b/const-fn-wide-local-truth.md) · OPEN
- const-fn-negate-min — [a constant function's minus of the 64-bit minimum declines](roadmap/3.b/const-fn-negate-min.md) · OPEN
- const-fn-signing-cast — [a signing or `int'` cast in a constant function declines](roadmap/3.b/const-fn-signing-cast.md) · OPEN
- typeparam-cast-w64 — [a type-parameter cast at 64 or 65 bits declines](roadmap/3.b/typeparam-cast-w64.md) · OPEN
- itor-const-fold — [`$itor` in a constant expression has no fold](roadmap/3.b/itor-const-fold.md) · OPEN
- ctx-guard-wide-param — [a wide parameter beside a function local declines the truth fold](roadmap/3.b/ctx-guard-wide-param.md) · OPEN
- bits-negative-msb-net — [`$bits` of a net with a negative msb in a localparam](roadmap/3.b/bits-negative-msb-net.md) · OPEN
- interface-port-parameter-const — [an interface parameter read through an interface port in a constant](roadmap/3.b/interface-port-parameter-const.md) · OPEN
- const-fn-part-select-write — [a constant `[m:l]` part-select write to a function local declines](roadmap/3.b/const-fn-part-select-write.md) · OPEN
- const-fn-local-select-read — [a select read of a constant-function local declines](roadmap/3.b/const-fn-local-select-read.md) · OPEN
- const-fn-bits-of-local — [`$bits` of a constant-function local declines](roadmap/3.b/const-fn-bits-of-local.md) · OPEN
- const-fn-break-continue — [`break` or `continue` in a constant function declines](roadmap/3.b/const-fn-break-continue.md) · OPEN
- const-fn-wide-local-return — [a constant function's local or return over 64 bits declines](roadmap/3.b/const-fn-wide-local-return.md) · OPEN
- const-fn-init-div-zero — [a loop local initialised by a division by zero declines](roadmap/3.b/const-fn-init-div-zero.md) · OPEN
- const-fn-first-init-bits — [a zero-formal constant function's first `$bits` initializer declines](roadmap/3.b/const-fn-first-init-bits.md) · OPEN

**Subroutine / frame**

- timing-return — [`return` or a self-`disable` inside a timing-control body in a task](roadmap/3.b/timing-return.md) · OPEN
- md-return-select — [selects on a multi-dimensional packed function return](roadmap/3.b/md-return-select.md) · OPEN (part)
- scoped-call-wide-const — [a scoped call whose body selects a wide package constant](roadmap/3.b/scoped-call-wide-const.md) · OPEN
- pkg-string-const-select-dir — [a package string select takes the caller's direction](roadmap/3.b/pkg-string-const-select-dir.md) · BLOCKED
- pkg-task-stmt — [a package-scoped task call as a statement (same as `pkg-scoped-task`)](roadmap/3.b/pkg-task-stmt.md) · DUP
- blocal-inert-falseloud — [an inert block-local declaration is refused](roadmap/3.b/blocal-inert-falseloud.md) · BLOCKED
- scoped-call-comb-arg — [a scoped call's initialised actual is a false multidriver error](roadmap/3.b/scoped-call-comb-arg.md) · DO-NOT-START
- blocal-collector-parity — [the block-local collector does not recurse into timing controls](roadmap/3.b/blocal-collector-parity.md) · HELD
- dimquery-width — [the `$size` family and `$signed` inside width-certified consumers](roadmap/3.b/dimquery-width.md) · OPEN
- §3.11 — [`function automatic` inlining names an operand twice](roadmap/3.b/-3.11.md) · OPEN
- static-local — [a static local read then written in straight-line code is refused](roadmap/3.b/static-local.md) · OPEN
- frame-oob — [an out-of-bounds read of a frame-local array gives no warning](roadmap/3.b/frame-oob.md) · OPEN
- 2seg-call — [a method call left of an output-formal call is refused](roadmap/3.b/2seg-call.md) · OPEN
- dyn-formal-pos — [ten call positions of dynamic-array formals are refused](roadmap/3.b/dyn-formal-pos.md) · OPEN
- pkg-default — [a package routine's default argument is checked in the importer's scope](roadmap/3.b/pkg-default.md) · OPEN
- fgets-rhs — [`return $fgets(…)` is refused](roadmap/3.b/fgets-rhs.md) · OPEN
- V3/V4 — [timing, NBA and `fork` in automatic routines entered from automatic ones](roadmap/3.b/V3-V4.md) · OPEN
- frame-array — [multi-dimensional, offset, copied and `foreach` frame-local arrays](roadmap/3.b/frame-array.md) · OPEN
- V2A/V5 — [dynamic-array formals on automatic tasks and function `new[]`](roadmap/3.b/V2A-V5.md) · OPEN
- foreach-fn — [`foreach` over a dynamic-array formal in a function](roadmap/3.b/foreach-fn.md) · OPEN
- r16-exec — [recursion over dynamic locals, non-vector elements and unwritten outputs](roadmap/3.b/r16-exec.md) · OPEN
- re-forward — [a function forwarding its dynamic-array formal to another call](roadmap/3.b/re-forward.md) · OPEN
- hier-task — [hierarchical task calls with output, array or string formals](roadmap/3.b/hier-task.md) · OPEN
- array-formal — [array formals: offset, output / inout, 2-D, signed and task forms](roadmap/3.b/array-formal.md) · OPEN
- blk-automatic-lifetime — [an `automatic` block local needs per-activation storage](roadmap/3.b/blk-automatic-lifetime.md) · OPEN
- hier-default-arg — [a hierarchical call ignores formal argument defaults](roadmap/3.b/hier-default-arg.md) · OPEN
- iface-modport-formal — [a modport routine call through an interface port formal](roadmap/3.b/iface-modport-formal.md) · BLOCKED
- hier-fn-inline-callee — [a hierarchical call to an inline-lowered function is refused](roadmap/3.b/hier-fn-inline-callee.md) · OPEN
- genblk-fn-call — [a call into a function of a labelled generate block is refused](roadmap/3.b/genblk-fn-call.md) · OPEN
- misc-sub — [method chains on call results and class-scope name defaults](roadmap/3.b/misc-sub.md) · OPEN
- frame-concat-split-residue — [concatenation targets over frame locals with a delay, call or hierarchy](roadmap/3.b/frame-concat-split-residue.md) · OPEN
- frame-local-array-default — [`'{default: v}` onto a function-local unpacked array is refused](roadmap/3.b/frame-local-array-default.md) · OPEN
- ca-fn-retvar-select — [a function in a continuous assign that select-writes its return variable](roadmap/3.b/ca-fn-retvar-select.md) · OPEN

**System tasks & file I/O**

- plusargs-%0d — [`$value$plusargs` rejects a `%0d` format](roadmap/3.b/plusargs--0d.md) · OPEN
- writemem-local — [`$writememh` of a task-local array is refused](roadmap/3.b/writemem-local.md) · OPEN
- filepos — [`$ftell` and `$sscanf` refused, `$fseek` skipped](roadmap/3.b/filepos.md) · OPEN
- deferred-inline-action — [a deferred assertion's non-print action runs at reach, not at maturation](roadmap/3.b/deferred-inline-action.md) · OPEN
- immediate-cover — [immediate `cover` forms are E2002; both oracles accept and drop them](roadmap/3.b/immediate-cover.md) · SPLIT
- typedef-atom-cast — [a cast to an atom or `bit` typedef is refused](roadmap/3.b/typedef-atom-cast.md) · OPEN
- $typename — [`$typename` renders enums and structs as their base type](roadmap/3.b/-typename.md) · OPEN
- %p-ⓐ — [`%p` of a packable unpacked struct prints a flat vector](roadmap/3.b/-p-a.md) · OPEN
- %p-ⓒ — [`%p` renders of negative assoc keys and real arrays differ by design](roadmap/3.b/-p-c.md) · HELD
- sformatf — [`$sformatf` inside a ternary, short-circuit or task argument](roadmap/3.b/sformatf.md) · OPEN
- ext-shadow — [same-named block-local items of external report round 20 §4.11, not yet isolated](roadmap/3.b/ext-shadow.md) · RECORD
- seed-read-warning — [a warning when a constant rests on a never-assigned variable](roadmap/3.b/seed-read-warning.md) · RECORD
- readmem-assoc-loud — [`$readmemh` into an associative array: addresses and targets still refused](roadmap/3.b/readmem-assoc-loud.md) · OPEN
- pattern-slice-arith-bound — [an arithmetic constant bound on an unpacked-array slice is refused](roadmap/3.b/pattern-slice-arith-bound.md) · OPEN
- pattern-slice-blocking-reads — [a blocking pattern into an array slice that reads variables is refused](roadmap/3.b/pattern-slice-blocking-reads.md) · OPEN

**Constrained random**

- constraint-if — [`if` / `else` inside a class constraint block is rejected](roadmap/3.b/constraint-if.md) · DEFERRED
- std-randomize-nested — [`std::randomize() with` used as a value expression](roadmap/3.b/std-randomize-nested.md) · DEFERRED

**Nets / timing**

- E3001-delayed — [overlapping delayed tri-state drivers are a false multidriver error](roadmap/3.b/E3001-delayed.md) · OPEN
- E3001-overlap — [a part-select driver beside a whole or same-range driver is a false E3001](roadmap/3.b/E3001-overlap.md) · DEFERRED
- hier-event — [an event control on a macro-built hierarchical name has no sensitivity](roadmap/3.b/hier-event.md) · OPEN
- xproc-disable — [a `disable` of a block running in another process](roadmap/3.b/xproc-disable.md) · OPEN
- timescale — [partial-timescale diagnostics are not wired](roadmap/3.b/timescale.md) · OPEN
- nonansi-child-array — [an instance array of a non-ANSI child module is refused](roadmap/3.b/nonansi-child-array.md) · OPEN
- implicit-net-generate — [an implicit net inside a generate block is refused](roadmap/3.b/implicit-net-generate.md) · OPEN
- modport-port-actual — [an interface modport as a port actual is refused](roadmap/3.b/modport-port-actual.md) · DEFERRED
- iface-generate — [a generate region inside an interface is refused](roadmap/3.b/iface-generate.md) · OPEN
- level-select-var-index — [an event on a variable-indexed select `@(n[i])` is refused](roadmap/3.b/level-select-var-index.md) · OPEN
- deep — [deep residues: time-0 races, `@(*)` wake, runtime `==?`, select forces](roadmap/3.b/deep.md) · OPEN
- iface-port-forward — [an interface port passed on to a child binds a 1-bit net](roadmap/3.b/iface-port-forward.md) · DEFERRED
- unpacked-port-actual — [unpacked-array port actuals: sub-arrays, patterns, one-element arrays](roadmap/3.b/unpacked-port-actual.md) · OPEN
- dead-arm-select-residue — [a reversed select in a never-taken `if` arm, by condition shape](roadmap/3.b/dead-arm-select-residue.md) · OPEN
- hier-unpacked-array-write — [a hierarchical write of a whole unpacked array is refused](roadmap/3.b/hier-unpacked-array-write.md) · DEFERRED
- nested-lval-select — [a refused nested lvalue leaves a chunk that fakes a second writer](roadmap/3.b/nested-lval-select.md) · OPEN

**Loud shapes surfaced by §4.5.493–495**

- ac-random-stmt — [`$random(seed);` in `always_comb` is a multidriver error](roadmap/3.b/ac-random-stmt.md) · HELD
- readmem-write-table — [`$readmem*` is missing from the system-call write table](roadmap/3.b/readmem-write-table.md) · HELD
- real-cont-assign — [a continuous assign of a function call to a `real` variable](roadmap/3.b/real-cont-assign.md) · OPEN
- real-fmt-hex — [`%h` of a real value is refused](roadmap/3.b/real-fmt-hex.md) · OPEN
- pkg-writer-import — [importing a package whose function writes a package variable](roadmap/3.b/pkg-writer-import.md) · OPEN
- fn-writes-pkg-var — [a package function writing a package variable is refused](roadmap/3.b/fn-writes-pkg-var.md) · OPEN
- class-real-members — [class real properties, static methods, method loops and body enums](roadmap/3.b/class-real-members.md) · OPEN

**Diagnostics quality**

- line-directive — [`` `line `` directives are ignored in diagnostic locations](roadmap/3.b/line-directive.md) · OPEN
- E3009-anchor — [some elaboration errors lack file:line or name the module header](roadmap/3.b/E3009-anchor.md) · OPEN
- error_at — [a parse error's anchor and its `found` token differ](roadmap/3.b/error_at.md) · OPEN
- #9 — [`velab -L` diagnostics have no location](roadmap/3.b/-9.md) · OPEN
- cli-lib — [the cli crate's library tests fail to build without default features](roadmap/3.b/cli-lib.md) · OPEN
- carrier-namespace — [internal type-parameter carrier names are not reserved](roadmap/3.b/carrier-namespace.md) · OPEN
- EXT2-DOC — [stale CLI, language, system-task and explain docs](roadmap/3.b/EXT2-DOC.md) · OPEN
- unique-if-chain — [`unique if` violations missed in subroutine bodies and qualified chains](roadmap/3.b/unique-if-chain.md) · BLOCKED
- unique-const-fn — [a constant function reaching a `unique` no-match arm does not fold](roadmap/3.b/unique-const-fn.md) · BLOCKED
- unique-pkg-closure — [a package call reaching a `unique` violation is refused as impure](roadmap/3.b/unique-pkg-closure.md) · OPEN
- unique-if-text — [the `unique if` warning text names a case statement](roadmap/3.b/unique-if-text.md) · OPEN

**Strings / heap**

- paren-select — [`(p)[0]` of a string silently reads 0](roadmap/3.b/paren-select.md) · OPEN
- real-part-write — [a part-select write to a scalar `real` is silently ignored](roadmap/3.b/real-part-write.md) · OPEN
- reduction-init — [`$sformatf` over an array reduction in a declaration is refused](roadmap/3.b/reduction-init.md) · OPEN
- string-array — [string arrays: fixed init, runtime index, queues, 2-D](roadmap/3.b/string-array.md) · OPEN
- inline-string — [a static task's inline string local is refused](roadmap/3.b/inline-string.md) · OPEN
- string-misc — [string slices, element `.len()`, string queue and assoc elements](roadmap/3.b/string-misc.md) · OPEN
- string-concat-unsized — [an unsized constant operand in a string concatenation is accepted](roadmap/3.b/string-concat-unsized.md) · RECORD
- string-door-pairs — [nested string concatenations: every spelling has to change at once](roadmap/3.b/string-door-pairs.md) · RECORD
- dyn-md-elem-select — [selects into a multi-dimensional element of a dynamic container](roadmap/3.b/dyn-md-elem-select.md) · OPEN
- class-field-select — [a part-select of a class field is refused](roadmap/3.b/class-field-select.md) · OPEN

**VCD / real conversion**

- vcd — [cosmetic VCD encoding differences](roadmap/3.b/vcd.md) · OPEN
- const-x-real — [a constant with x / z bits converted to real does not fold](roadmap/3.b/const-x-real.md) · OPEN

### 3.c Intentionally loud (not gaps)
> Shapes refused on purpose, each with its reason (a rule, a split, a documented choice); not gaps, not queued.

- plusargs-in-expression — [`$value$plusargs` in an arbitrary expression](roadmap/3.c/plusargs-in-expression.md)
- p-format-one-element — [`%p` of a one-element array `int a[0:0]`](roadmap/3.c/p-format-one-element.md)
- unpacked-port-direction — [a direction mismatch on an unpacked array port](roadmap/3.c/unpacked-port-direction.md)
- unproved-array-fill — [a fixed-array fill whose coverage cannot be proved](roadmap/3.c/unproved-array-fill.md)
- dyn-formal-call-twice — [the same dynamic-formal function called twice in one frame-body expression](roadmap/3.c/dyn-formal-call-twice.md)
- nested-block-redeclare — [a nested block redeclaring a name no module-scope net carries](roadmap/3.c/nested-block-redeclare.md)
- readmem-initial-race — [a `$readmem*` child-versus-parent `initial` race](roadmap/3.c/readmem-initial-race.md)
- readmem-wire-array — [`$readmemh` into a `wire` array](roadmap/3.c/readmem-wire-array.md)
- header-default-body-import — [a header default that names a constant from a body import](roadmap/3.c/header-default-body-import.md)
- iverilog-loop-defects — [two iverilog 13.0 defects around `break` and `continue`](roadmap/3.c/iverilog-loop-defects.md)
- l-format-choice — [the `%l` format specifier](roadmap/3.c/l-format-choice.md)

## 4. SVA / verification honest-loud residues
> Six SVA and verification constructs that stay honest-loud, each blocked on a named prerequisite.

- fused-zero-unbounded-delay — [`##0` fused with an unbounded cycle-delay range](roadmap/4/fused-zero-unbounded-delay.md) · BLOCKED
- n2c-seq-local-vars — [full sequence local variables (N2c)](roadmap/4/n2c-seq-local-vars.md) · BLOCKED
- later-antecedent-skew — [a later antecedent read and the outer `|=>` skew](roadmap/4/later-antecedent-skew.md) · BLOCKED
- sva-quad-collapse-flip — [flipping the SVA-QUAD collapse default](roadmap/4/sva-quad-collapse-flip.md) · BLOCKED
- n4-clocking-skew — [the clocking skew value `output #0` (N4)](roadmap/4/n4-clocking-skew.md) · BLOCKED
- class-static-cast — [class casts such as `Derived'(base)`](roadmap/4/class-static-cast.md) · BLOCKED

## 5. Performance / hardening
> Performance and hardening, below the correctness ladder: standing verdicts, open residues and the backend state in force.

Below the correctness ladder; a row resumes only when its re-entry condition fires. `§5.1-<x>` records: `git show acabe991:docs/history/ROADMAP_ARCHIVE_PHASE_A-D.md`. ([notes](roadmap/5/README.md))

### 5.a Standing verdicts
> Settled performance verdicts (rejected, discarded, stopped or on hold), each with the condition that would reopen it.

| id | verdict | reason | re-entry condition |
|---|---|---|---|
| codegen (cranelift), including the `§5.1-be` machine-code experiment | rejected | call boundary cost over the ceiling | a way without writing the semantics twice ([note](roadmap/5/README.md)) |
| D2-b two-state storage | rejected | a step down the accuracy ladder | a way with no correctness trade |
| cycle-based mode | rejected | event-driven already skips ~90% of combinational work | real demand with ≥1 evaluation per block per cycle |
| levelize (rank-ordered Active drain) | discarded | 1.00×; the dirty settle closed the root | none |
| process fusion (the E axis) | not adopted | a chain reader sees the propagated value (silent-wrong) | none |
| net-count reduction (flatten) | on hold | erases what `--probe`, VCD, `%m` and hierarchical names target | an owner ruling |
| S4 schedule elision · S5 NBA specialisation | stopped | below the 1.3× stop threshold | none |
| the `wprog` reject family inside settle | not started | ~2–2.5% of serv; needs `Tern` as a jump | prize exceeds the machinery |
| `drain_range_diags` early-out | rejected | zero gain | none |

### 5.b Open performance and hardening residues
> Twenty-one open performance and hardening residues: eleven startable, six blocked on a prerequisite, four held or trigger-gated.

- 4b-r — [the native backend does not pool its scratch buffers](roadmap/5.b/4b-r.md) · OPEN
- 5c — [frame bodies never enter `wprog`](roadmap/5.b/5c.md) · BLOCKED
- ARR-LHS — [a multi-dimensional element write on the left side is a cost cliff](roadmap/5.b/ARR-LHS.md) · BLOCKED
- INLINE-FOLD — [the inline fold walks a DAG as a tree, exponentially](roadmap/5.b/INLINE-FOLD.md) · OPEN
- MEM-GUARD — [no guard on process memory](roadmap/5.b/MEM-GUARD.md) · BLOCKED
- NBA-GROWTH — [nothing bounds the NBAs or forked processes piled up in one time step](roadmap/5.b/NBA-GROWTH.md) · OPEN
- EXEC-ROWS — [`executor_rows` rescans on every `simulate`](roadmap/5.b/EXEC-ROWS.md) · HELD
- DELAY-CLAMP — [a `#delay` above u32::MAX is clamped](roadmap/5.b/DELAY-CLAMP.md) · OPEN
- KPRED-3RD — [tier 3 lacks a kernel-can-run-it layer](roadmap/5.b/KPRED-3RD.md) · HELD
- QUIESCE-NBA — [tier-3 quiescence ignores delayed NBAs](roadmap/5.b/QUIESCE-NBA.md) · BLOCKED
- BYTE-GATE-6 — [six known differences against the S1d-4d byte gate](roadmap/5.b/BYTE-GATE-6.md) · OPEN
- MON-RENDER — [tier 3 refuses `$monitor` and `$strobe`](roadmap/5.b/MON-RENDER.md) · BLOCKED
- FD-EOF + FEOF — [end-of-file tracking: an `fd_eof` X-poison hole and an over-marked `$feof`](roadmap/5.b/FD-EOF---FEOF.md) · OPEN
- NETSLOT-PREV — [`NetSlot.prev` is written but never read](roadmap/5.b/NETSLOT-PREV.md) · OPEN
- ELAB-PHASE-BLIND — [the corpus cannot see front-end cost](roadmap/5.b/ELAB-PHASE-BLIND.md) · OPEN
- LOW-ROI — [low-return performance ideas parked together](roadmap/5.b/LOW-ROI.md) · HELD
- T0-SERIAL — [each serial time-0 comb pass is followed by a scan of every waiter](roadmap/5.b/T0-SERIAL.md) · HELD
- T0-HOLD-CHAIN — [held time-0 assign chains cost quadratic time](roadmap/5.b/T0-HOLD-CHAIN.md) · BLOCKED
- CORPUS-FETCH-GUARD — [`corpus-runner fetch` removes clean clones it misreads as stale](roadmap/5.b/CORPUS-FETCH-GUARD.md) · OPEN
- STORE-STACK-POS — [store-path speed depends on where frames sit on the stack](roadmap/5.b/STORE-STACK-POS.md) · OPEN
- Q-EXECSEM — [census of the operations each executor spells on its own, no code](roadmap/5.b/Q-EXECSEM.md) · OPEN

### 5.c Current state
> Where the backend stands today: the default executor, the product build, the CI corpus gate and codegen.

Default backend `native` (tier 3); product `--no-default-features` = one executor; the corpus runs blocking in CI on 3 OSes; codegen off. ([notes](roadmap/5/README.md))

## 5.2 Queue (start order)
> The start order of the open rows: slot = order · track, rank = defect class; C rows (the page cut) go first, and one slot runs the queue

Canonical start order; LOOPROMPT's NEXT mirrors it and this table wins.
State (2026-11-13): BIND-1 and BIND-2 landed as batch 4 (§4.5.640); record-first order, next Q-SYMMIX (C) and S1v-V; iteration and real-design-first rules ([notes](roadmap/5.2/README.md))
Incoming reports pre-empt the queue; reproduce every item at HEAD first.

The real-design program (its scope, the page-cut order, the T′ / V′ / B′ stages), how to read the slot and rank columns, and why rows 1–5 go first ([notes](roadmap/5.2/README.md))

| # | slot | item | rank |
|---|---|---|---|
| SYS-1 | R2.1 · L1 (landed) | [one record per system function, in sim-ir](roadmap/5.2/SYS-1.md) | — F |
| PARSE-1 | R2.2 · L1 (landed) | [the parser's constant fold in one file, its 64-bit kernels in hdl-ast](roadmap/5.2/PARSE-1.md) | — F |
| VAL-1 | R3.1 · L1 (landed) | [the elaborate constant evaluators' shared arms, written once](roadmap/5.2/VAL-1.md) | — F |
| VAL-2 | R3.2 · L1 (landed) | [the parameter declaration chain and the live-scope wide ask, written once](roadmap/5.2/VAL-2.md) | — F |
| VAL-3 | R3.3 · L1 (landed) | [a declared range's bounds folded, then checked, in one body](roadmap/5.2/VAL-3.md) | — F |
| BIND-1 | R4.1 · L1 (landed) | [the nine parameter maps written in one home, read through named lanes](roadmap/5.2/BIND-1.md) | — F |
| BIND-2 | R4.2 · L1 (landed) | [the constant-function frame as one type, its writers and read lanes](roadmap/5.2/BIND-2.md) | — F |
| Q-SYMMIX | R5.1 · L1 (PARKED) | [a symbolic struct over a signed and an unsigned parameter is laid out wrong](roadmap/5.2/Q-SYMMIX.md) | ① |
| Q-PARAM2STATE | R5.2 · L1 | [a parameter's declared 2-state type is not recorded](roadmap/5.2/Q-PARAM2STATE.md) | ① |
| Q-CONSTSELLSB | R7.1 · L1 | [constant interpreter select writes ignore the declared LSB and direction](roadmap/5.2/Q-CONSTSELLSB.md) | ① |
| Q-CONSTAGG | R7.2 · L1 | [constant interpreter treats packed-dim and unpacked locals as one vector](roadmap/5.2/Q-CONSTAGG.md) | ① |
| Q-OVRXPLANE | R7.4 · L1 | [an override's sized x / z literal loses its unknown plane](roadmap/5.2/Q-OVRXPLANE.md) | ① |
| 24 | R8.1 · L1 (PARKED) | [struct member part-selects with named bounds, as reads and write targets](roadmap/5.2/24.md) | ② F |
| 29 | R8.2 · L1 (PARKED) | [element selects nested in a write target are refused](roadmap/5.2/29.md) | ② F |
| 31 | R8.3 · L1 | [VeeR EH1 corpus row: the hello_world test passes on unmodified RTL](roadmap/5.2/31.md) | — C |
| 68 | R9.1 · L1 | [a typedef's bound re-folds where the type is used, not where declared](roadmap/5.2/68.md) | ① F |
| 23 | R9.1 · L1 | [the parser's constant fold lacks shift and power operators](roadmap/5.2/23.md) | ② F |
| Q-RTNTEXT | R9.2 · L1 | [one answer for the scope a package routine's text binds in](roadmap/5.2/Q-RTNTEXT.md) | — F |
| 46 | R9.3 · L1 (PARKED) | [a package typedef's named dimensions fold where the type is used](roadmap/5.2/46.md) | ② F |
| 39 | R9.4 · L1 | [package export declarations are refused at parse](roadmap/5.2/39.md) | ② F |
| 33 | C1 · L1 (PARKED) | [`'{default: v}` onto a function-local unpacked array is refused](roadmap/5.2/33.md) | ② S |
| Q-SIDETABLES | C7 · L1 | [one engine side-table record, below sim-engine](roadmap/5.2/Q-SIDETABLES.md) | — F |
| Q-NETSHAPE | C7 · L1 | [one net-shape record in elaborate, written only by add_net](roadmap/5.2/Q-NETSHAPE.md) | — F |
| 74-S | C7 · L1 | [the expression-type rule in one home beside sim-ir's self width](roadmap/5.2/74-S.md) | — F |
| Q-VARBASEWR | C8 · T3 | [a variable-base `+:` write into a multi-dimensional packed element](roadmap/5.2/Q-VARBASEWR.md) | ② F |
| 35 | C9 · L1 | [a replication inside an assignment pattern is refused at parse](roadmap/5.2/35.md) | ② S |
| 37 | C11 · L1 | [unpacked-array port actuals: sub-arrays, patterns, one-element arrays](roadmap/5.2/37.md) | ② S |
| 43 | C12 · L2 | [a package struct member width given by a constant function call](roadmap/5.2/43.md) | ② F |
| 42 | C14 · L2 | [a struct with header-parameter member widths, beyond whole-member reads](roadmap/5.2/42.md) | ② F |
| 38 | C15 · L2 | [a member select after a two-level index into a packed struct array](roadmap/5.2/38.md) | ② S |
| 70 | C16 · L2 | [hierarchical element select of an md packed parameter reads the flat bit](roadmap/5.2/70.md) | ① F |
| Q-MDPARAMIDXSELF | C16 · L2 | [an md packed parameter element select reads a 32-bit-widened index](roadmap/5.2/Q-MDPARAMIDXSELF.md) | ① |
| 69 | C17 · L2 | [assignment patterns into multi-dimensional packed arrays](roadmap/5.2/69.md) | ② F |
| 47 | C18 · any | [OpenTitan corpus row: five IP pages run under first-party TL-UL testbenches](roadmap/5.2/47.md) | — C |
| Q-WITHREAL | 1 · L1 (PARKED) | [real-or-integral type of a `with` reduction and its `item`](roadmap/5.2/Q-WITHREAL.md) | ① |
| Q-XNONINT | 1 · L1 (PARKED) | [x-condition `?:` bit-merges string/real arms](roadmap/5.2/Q-XNONINT.md) | ② S |
| Q-INLITEM | 1 · L1 (parked) | [an inlined function inside a `with` clause shares the caller's `item`](roadmap/5.2/Q-INLITEM.md) | ① |
| 48 | 3 · T3 (delivered) | [V stage: census and design of the constant-value domain, no code](roadmap/5.2/48.md) | — F |
| Q-PKCAP | 5 · L2 | [the packed dimension product has no net-width cap (memory blow-up)](roadmap/5.2/Q-PKCAP.md) | — |
| P-PCTU | 6 · T3 (PARKED, landed (part)) | [`%u` / `%z` format specifiers write nothing](roadmap/5.2/P-PCTU.md) | ① S |
| Q-PCTUREC | 12 · L2 | [`%u` / `%z` of an unpacked struct or union need its member layout](roadmap/5.2/Q-PCTUREC.md) | ① S |
| Q-CLSRET | 12 · L2 | [a class-typed function return is treated as one bit](roadmap/5.2/Q-CLSRET.md) | ① S |
| Q-DLYCAT | 7 · L1 (PARKED) | [an intra-assignment delay on a string concatenation is refused](roadmap/5.2/Q-DLYCAT.md) | ② |
| Q-RANGEMASK | any · L2 | [interpreter formals and locals with unfoldable ranges are not masked](roadmap/5.2/Q-RANGEMASK.md) | ① |
| Q-PKGBLKLEAK | any · L2 | [a package routine's block-local writes the enclosing name](roadmap/5.2/Q-PKGBLKLEAK.md) | ① |
| Q-SCALARSEL | any · L2 | [a bit select of a scalar folds instead of being an error](roadmap/5.2/Q-SCALARSEL.md) | ① |
| Q-IDXSIGNCTX | 9 · T3 | [an index is re-typed unsigned 32-bit by the flatten it is lowered into](roadmap/5.2/Q-IDXSIGNCTX.md) | ① |
| Q-STRREPEVAL | 10 · any | [a string replication evaluates its operands once per copy](roadmap/5.2/Q-STRREPEVAL.md) | ① |
| Q-HANDLERET | 10 · any | [two routes into a class handle skip the handle check](roadmap/5.2/Q-HANDLERET.md) | ① |
| Q-STRCATSEL | 10 · any | [a select on a string concatenation reads the packed vector](roadmap/5.2/Q-STRCATSEL.md) | ① |
| Q-STRWAIT | 10 · any | [a `wait` on a string comparison never wakes](roadmap/5.2/Q-STRWAIT.md) | ① |
| Q-VALRULES | 10 · L1 | [census and design of the value and width rules spelled several times, no code](roadmap/5.2/Q-VALRULES.md) | — F |
| 74 | 10 · L1 | [ET: the expression-type funnel (string, real or integral, and width)](roadmap/5.2/74.md) | ① F |
| Q-STRTERN | 10 · L1 | [a `?:` with a string arm is not typed as a string](roadmap/5.2/Q-STRTERN.md) | ② |
| Q-STRCASTLIT | 10 · L1 | [`string'(<literal>)` lowers to a bare literal, not a string](roadmap/5.2/Q-STRCASTLIT.md) | ② |
| Q-NESTCAT | 10 · L1 (PARKED) | [a string concatenation nested in a concatenation drops bytes](roadmap/5.2/Q-NESTCAT.md) | ② |
| Q-STRTERNCAT | 10 · L1 | [a string `?:` as a concatenation operand drops the arm](roadmap/5.2/Q-STRTERNCAT.md) | ① |
| Q-LETSTR | 10 · L1 | [a `let` with a string body is not typed as a string](roadmap/5.2/Q-LETSTR.md) | ① |
| 8 | 11 · L2 | [2-state enum declarations and returns are stored as 4-state](roadmap/5.2/8.md) | ① |
| Q-XCORE | 12 · T3 | [the region fold declines where IEEE gives an x result](roadmap/5.2/Q-XCORE.md) | ① F |
| Q-PRIMCAST | 12 · T3 | [the region fold has no `int'` / `byte'` cast arm](roadmap/5.2/Q-PRIMCAST.md) | ① F |
| 40 | 12 · T3 (un-parked) | [a zero-count replication declines its whole constant concatenation](roadmap/5.2/40.md) | ② S |
| Q-LOCSTALE | 13 · L1 | [an array locator whose `with` reads a variable returns every index (native)](roadmap/5.2/Q-LOCSTALE.md) | ① |
| Q-UNSZXPAD | 15 · T3 | [an unsized x / z literal is zero-extended past 32 bits](roadmap/5.2/Q-UNSZXPAD.md) | ① |
| Q-STRNUL | 16 · L1 | [strings keep interior NUL bytes that IEEE removes](roadmap/5.2/Q-STRNUL.md) | ① S |
| 26 | 16 · L1 | [`$sformatf` as a task argument and inside `?:` is refused](roadmap/5.2/26.md) | ② S |
| Q-STRWIDTH | 18 · T3 | [the empty string literal folds as 1 bit instead of 8](roadmap/5.2/Q-STRWIDTH.md) | ① |
| Q-2STINVSEL | 19 · L1 | [an invalid select of a 2-state variable reads x, not 0](roadmap/5.2/Q-2STINVSEL.md) | ① |
| 49 | 21 · T3 | [enum labels the 64-bit label fold cannot hold](roadmap/5.2/49.md) | ② F |
| Q-NANCANON | 22 · L1 | [real NaN sign and payload differ by platform](roadmap/5.2/Q-NANCANON.md) | ① F |
| 76 | 23 · L2 | [the constant interpreter's environment holds no x](roadmap/5.2/76.md) | ① F |
| Q-TIMELITCONST | 24 · T3 | [a time literal in an integral constant folds as an integer](roadmap/5.2/Q-TIMELITCONST.md) | ① |
| Q-LETCAST | 26 · L2 | [a `let` formal's declared type is dropped at parse](roadmap/5.2/Q-LETCAST.md) | ① |
| 50 | 27 · T3 | [a string-valued header parameter is not read by a generate condition](roadmap/5.2/50.md) | ② F |
| 34 | 28 · L1 | [`$bits` of an ANSI port in a `localparam` is refused](roadmap/5.2/34.md) | ② S |
| Q-TPCARRIER | 29 · L2 (PARKED) | [type parameters are carried by parse-time stand-in names](roadmap/5.2/Q-TPCARRIER.md) | ① F |
| Q-IDXVALUE | 30 · T3 | [a select offset's value is read four different ways](roadmap/5.2/Q-IDXVALUE.md) | ① |
| Q-MEMBSRC | 32 · L2 (PARKED (part)) | [carry a struct member select's source bounds from parser to elaborate](roadmap/5.2/Q-MEMBSRC.md) | ① F |
| Q-MEMBIDXNOT | 32 · L2 (parked) | [a `~` index on a non-zero-base struct member reads x](roadmap/5.2/Q-MEMBIDXNOT.md) | — |
| Q-VMSELSHIFT | 33 · T3 | [debug build panics on a select read partly below the vector](roadmap/5.2/Q-VMSELSHIFT.md) | ① |
| Q-MDCONSTOFF | 36 · T3 | [the multi-driver check skips selects with a constant offset](roadmap/5.2/Q-MDCONSTOFF.md) | ① |
| 75 | 37 · L1 | [DH: the deferred hierarchical reference funnel](roadmap/5.2/75.md) | ① F |
| Q-STRSEL | 39 · T3 | [a string literal as a constant select bound or count is refused](roadmap/5.2/Q-STRSEL.md) | ① |
| 41 | 41 · L2 | [packed dimensions on a typedef'd type: the residue after the page half](roadmap/5.2/41.md) | ② F |
| Q-BITSTYPE | 42 · T3 (PARKED) | [`$bits` of a constant expression sized from its type, not from a folded value](roadmap/5.2/Q-BITSTYPE.md) | ① F |
| Q-REALSPLIT | 42 · T3 (PARKED) | [a constant region that mentions a real folds whole as real, not typed per node](roadmap/5.2/Q-REALSPLIT.md) | ① F |
| Q-REALTWIN | 42 · T3 (PARKED) | [an exact real parameter's integer twin is read as an integral name](roadmap/5.2/Q-REALTWIN.md) | ① F |
| Q-FORKARM-HIDDEN | 43 · any | [a fork arm reading a parent variable only through a hierarchical operand panics](roadmap/5.2/Q-FORKARM-HIDDEN.md) | ① |
| Q-CAIDXFOLD | 45 · T3 | [a continuous assign to a non-logic array element refuses legal constant indexes](roadmap/5.2/Q-CAIDXFOLD.md) | ① |
| Q-RS30FOLD | 48 · T3 | [the implicit read set's exact index fold has no arm for `/`, `%`, `&`, `$clog2`](roadmap/5.2/Q-RS30FOLD.md) | ① |
| Q-ARMWRITE | 49 · any | [fork arm admission tests touching the waited variable, not writing it](roadmap/5.2/Q-ARMWRITE.md) | ① |
| Q-DAMEMBPART | 51 · T3 | [definite assignment counts a select with a named constant bound as partial](roadmap/5.2/Q-DAMEMBPART.md) | ② |
| Q-FRAMEWAKE | 52 · any | [a write of a frame task's own variable wakes no in-frame level wait](roadmap/5.2/Q-FRAMEWAKE.md) | ① F |
| Q-COVSELDOM | 54 · T3 | [a coverpoint on a constant part-select gets the wrong bin domain width](roadmap/5.2/Q-COVSELDOM.md) | ① |
| Q-FRAMESTAR | 55 · any | [ENG-2's frame-body part: the read set of an in-body `@(*)`](roadmap/5.2/Q-FRAMESTAR.md) | ① F |
| Q-CONSTXIDX | 57 · T3 | [a constant select whose index has an x or z bit is refused instead of reading x](roadmap/5.2/Q-CONSTXIDX.md) | ② |
| Q-HIERCOMPOUND | 58 · any | [a compound assign to a hierarchical lvalue in `always @*` reads its own copy](roadmap/5.2/Q-HIERCOMPOUND.md) | ① |
| Q-CONSTPSELOOR | 60 · T3 | [a constant part-select partly or wholly outside its vector cannot be folded](roadmap/5.2/Q-CONSTPSELOOR.md) | ② |
| 45 | 61 · T3 | [a package parameter whose value calls a function is refused](roadmap/5.2/45.md) | ① F |
| 2 | 61 · T3 | [a routine imported into another package binds in the importer or the caller](roadmap/5.2/2.md) | ① F |
| Q-HIERE3001 | 61 · any | [a hierarchical call over an initialized variable is a false multi-driver error](roadmap/5.2/Q-HIERE3001.md) | ① |
| 44 | 62 · L2 | [a `parameter type` overridden with an enum type is refused](roadmap/5.2/44.md) | ② F |
| 53 | 64 · any | [OpenTitan corpus row: the aon_timer and edn pages under census testbenches](roadmap/5.2/53.md) | — C |
| Q-HIERFORCE | 64 · any | [`force` / `release` on a net through a hierarchical name is refused](roadmap/5.2/Q-HIERFORCE.md) | ① |
| 71 | 65 · L2 | [a typedef bound naming an enclosing scope's constant re-folds at its use](roadmap/5.2/71.md) | ① F |
| 51 | 66 · T3 | [selects of a parameter wider than 64 bits: correct support in place of refusal](roadmap/5.2/51.md) | ② F |
| Q-CLKSHADOW | 67 · any | [clocking-name resolution misses generate and instance shadowing](roadmap/5.2/Q-CLKSHADOW.md) | ① |
| Q-CLSPARAMTYPE | 68 · L2 | [a class value-parameter override is not converted to the parameter's type](roadmap/5.2/Q-CLSPARAMTYPE.md) | ① |
| Q-DECLSELBOUND | 69 · T3 | [a range bound from an x or reversed constant select takes a silent default](roadmap/5.2/Q-DECLSELBOUND.md) | ① |
| Q-HIERSTR | 70 · any | [a hierarchical string inside a concatenation or relational prints wrong](roadmap/5.2/Q-HIERSTR.md) | ② |
| Q-CLSPROPLSB | 71 · L2 | [a class property with a non-zero LSB is selected as if declared from bit 0](roadmap/5.2/Q-CLSPROPLSB.md) | ① |
| Q-USZGEOM | 72 · T3 | [an unpacked dimension with a declined bound gets size and storage from two folds](roadmap/5.2/Q-USZGEOM.md) | ① |
| Q-STRCMPCAT | 73 · any | [a one-element concatenation over a string `?:` compares as packed, not string](roadmap/5.2/Q-STRCMPCAT.md) | ② |
| Q-MEMBIDXWR | 74 · L2 | [an indexed part-select or variable-index bit write of a struct member is refused](roadmap/5.2/Q-MEMBIDXWR.md) | ② |
| Q-CAREALCTX | 75 · any | [an integral expression driving a `real` by continuous assign uses 64-bit context](roadmap/5.2/Q-CAREALCTX.md) | ① |
| Q-RDYNELEM | 76 · any | [a write to a `real` dynamic-array element converts at the wrong width](roadmap/5.2/Q-RDYNELEM.md) | ① |
| Q-MEMBOOBWR | 77 · L2 | [a struct member range write with a bit outside the member is refused](roadmap/5.2/Q-MEMBOOBWR.md) | ② |
| Q-SVALET | 78 · any | [an SVA sampled-value register of a `let` argument is typed as one bit](roadmap/5.2/Q-SVALET.md) | ① |
| ENG-3 | 79 · any | [implicit sensitivity at longest-static-prefix granularity](roadmap/5.2/ENG-3.md) | ① F |
| Q-MEMBSYSFNBOUND | 80 · L2 | [`$high`, `$size` and the other array queries of a struct member are refused](roadmap/5.2/Q-MEMBSYSFNBOUND.md) | ② |
| Q-PSELOOR | 81 · any | [a part-select partly below the net's low bit is refused, not written in range](roadmap/5.2/Q-PSELOOR.md) | ① |
| 73 | 82 · any (parked) | [assertion control: gate the whole assertion branch on the runtime enable](roadmap/5.2/73.md) | ① F |
| 52 | 83 · L2 | [a package typedef member width over a member of a >64-bit struct parameter](roadmap/5.2/52.md) | ② F |
| Q-MIXCAPROC | 84 · any | [continuous and procedural writes to overlapping bits of a variable are accepted](roadmap/5.2/Q-MIXCAPROC.md) | ① |
| Q-MDPORTCOL | 85 · any | [a port drive and a parent assign on disjoint bits of one net are refused](roadmap/5.2/Q-MDPORTCOL.md) | ① |
| Q-PKGPMEMB | 86 · L2 | [a member of a package-scoped struct parameter is refused at parse](roadmap/5.2/Q-PKGPMEMB.md) | ② |
| Q-ARRWORDMD | 88 · any | [two continuous assigns to one net-array word are neither refused nor resolved](roadmap/5.2/Q-ARRWORDMD.md) | ① |
| 1 | 89 · L2 | [the instance-array prepass binds a child's header with the parent's tables](roadmap/5.2/1.md) | ① |
| Q-DEADARMREAD | 89 · any | [a constant-condition dead arm keeps its reads in an implicit read set](roadmap/5.2/Q-DEADARMREAD.md) | ① |
| Q-UNION | 91 · any | [a whole read after part writes covering every bit still counts as a read](roadmap/5.2/Q-UNION.md) | ① |
| 3 | 92 · L2 | [a constant call in a generate block binds the module's same-named function](roadmap/5.2/3.md) | ① |
| Q-EVSELT0 | 92 · any | [an event control on a constant-expression bit-select counts an extra event](roadmap/5.2/Q-EVSELT0.md) | ① |
| Q-RTNBODYSCOPE | 93 · L2 | [a constant call's body folds at the call's scope, not the routine's](roadmap/5.2/Q-RTNBODYSCOPE.md) | ① |
| Q-MIX | 94 · any | [in a mixed level and edge event list, the level term wakes on bit 0 only](roadmap/5.2/Q-MIX.md) | ① |
| 77 | 95 · L2 | [a generate-scope parameter read in a later walk sees the previous walk's binding](roadmap/5.2/77.md) | ① F |
| Q-CLKDRV | 95 · any | [a clocking output drives its signal before the first synchronous drive](roadmap/5.2/Q-CLKDRV.md) | ① |
| Q-CONVT0 | 96 · any | [a continuous copy converting real and integral takes a spurious time-0 event](roadmap/5.2/Q-CONVT0.md) | ① |
| Q-NETDLY | 97 · any | [a net delay on a declaration without an assignment is ignored silently](roadmap/5.2/Q-NETDLY.md) | ① |
| 4 | 98 · L2 | [a `$unit` routine's text binds the calling module's same-named names](roadmap/5.2/4.md) | ① |
| Q-FORCEDYN | 98 · any | [a `force` reading a dynamic-array element never re-evaluates on its change](roadmap/5.2/Q-FORCEDYN.md) | ① |
| 5 | 99 · T3 | [every `case` form in a constant function is refused](roadmap/5.2/5.md) | ② |
| Q-WAITDYN | 99 · any | [a `wait` on a dynamic-array element is never re-checked](roadmap/5.2/Q-WAITDYN.md) | ① |
| 6 | 100 · T3 | [a constant typed by an overridden type parameter loses the override's sign](roadmap/5.2/6.md) | ① |
| Q-CAPLIFE | 100 · any | [overlapping automatic task activations share one delayed-assign capture](roadmap/5.2/Q-CAPLIFE.md) | ① |
| 72b | 101 · L2 (held) | [block-local scoping in subroutine bodies, joined only if a page needs it](roadmap/5.2/72b.md) | ① F |
| Q-RELIMM | 101 · any | [after a release, an undelayed driver's target reads forced until the next settle](roadmap/5.2/Q-RELIMM.md) | ① |
| 7 | 102 · T3 | [generate-case label compares decided in the bit domain, with 🆕 S's wide half](roadmap/5.2/7.md) | ① |
| Q-RELEVAL | 102 · any | [a `release` re-evaluates an uncertified continuous assign's right-hand side](roadmap/5.2/Q-RELEVAL.md) | ① |
| Q-DCASETTLE | 103 · any | [a delayed continuous assign's right-hand side is evaluated on every settle](roadmap/5.2/Q-DCASETTLE.md) | ① |
| Q-FORCEPORT | 104 · any | [a `force` on a net bound to a child's output port is invisible inside the child](roadmap/5.2/Q-FORCEPORT.md) | ① |
| Q-DCAPARTX | 105 · any | [a delayed part-select driver beside another driver owes no initial x](roadmap/5.2/Q-DCAPARTX.md) | ① |
| N-R01 | 106 · any | [a delayed assign's owed initial x is sized from the right-hand side](roadmap/5.2/N-R01.md) | ① |
| Q-RELUNDRV | 107 · any | [after a release, undriven bits of a net keep the forced value instead of z](roadmap/5.2/Q-RELUNDRV.md) | ① |
| Q-RELZERODLY | 108 · any | [a `#0` driver of a forced net changing in its release step lands twice](roadmap/5.2/Q-RELZERODLY.md) | ① |
| Q-REFMD | 109 · any | [an initialized variable passed to a read-only `ref` is falsely multi-driven](roadmap/5.2/Q-REFMD.md) | ① |
| Q-RECVWRITE | 110 · any | [a method writing its receiver counts as a read, never a write, in a read set](roadmap/5.2/Q-RECVWRITE.md) | ① |
| Q-STATICFRAME-WAIT | 111 · any | [a static recursive task waiting on its own variable misses other activations](roadmap/5.2/Q-STATICFRAME-WAIT.md) | ① |
| Q-PTAIL | 112 · any | [an automatic task whose body only calls a suspending task returns at once](roadmap/5.2/Q-PTAIL.md) | ① |
| Q-DSV | 113 · any | [an SVA past-value register starts at the type default, not the declared value](roadmap/5.2/Q-DSV.md) | ① |
| Q-FMTNEGNAN | 114 · any | [a negative NaN prints without its sign under `%f`](roadmap/5.2/Q-FMTNEGNAN.md) | ① |
| Q-PCTM | 115 · any | [`%m` after a suspension in an automatic task names another instance](roadmap/5.2/Q-PCTM.md) | ① |
| Q-ESCSCOPE | 116 · any | [`%m` keeps an escaped identifier's backslash](roadmap/5.2/Q-ESCSCOPE.md) | ① |
| Q-PCTCXZ | 117 · any | [`%c` of a value with x or z bits prints a NUL byte](roadmap/5.2/Q-PCTCXZ.md) | ① |
| Q-ATOREAL-WS | 118 · any | [`atoreal()` and the constant `ato*` folds skip Unicode whitespace](roadmap/5.2/Q-ATOREAL-WS.md) | ① S |
| Q-DYNPAT | 119 · any | [a dynamic-array self pattern assigns element by element with no snapshot](roadmap/5.2/Q-DYNPAT.md) | ① |
| Q-DISTN | 120 · any | [`$dist_normal` and `$dist_chi_square` differ from the standard's algorithm](roadmap/5.2/Q-DISTN.md) | ① |
| Q-CALHSEL | 121 · any | [a continuous assign whose target has a non-constant select is accepted](roadmap/5.2/Q-CALHSEL.md) | ① |
| Q-DEASSIGNCA | 122 · any | [`deassign` of a variable a continuous assign drives is accepted](roadmap/5.2/Q-DEASSIGNCA.md) | ① |
| Q-CADYNELEM | 123 · any | [a continuous assign to a dynamic array or queue element is accepted](roadmap/5.2/Q-CADYNELEM.md) | ① |
| Q-CAREALCAT | 124 · any | [a `real` operand in a concatenation target of a continuous assign is accepted](roadmap/5.2/Q-CAREALCAT.md) | ① |
| Q-TASKOUTCAT | 125 · any | [a concatenation as a task's output actual is refused](roadmap/5.2/Q-TASKOUTCAT.md) | ① |
| Q-REFALIAS | 126 · any | [two `ref` formals bound to one variable keep only the last copy-out](roadmap/5.2/Q-REFALIAS.md) | ① |
| Q-DYNELEMSUBWR | 127 · any | [a sub-select write into an associative-array or queue element is dropped](roadmap/5.2/Q-DYNELEMSUBWR.md) | ① |
| Q-CLKHEAP | 128 · any | [clocking items over unpacked arrays, strings and queues get a heap-aware holder](roadmap/5.2/Q-CLKHEAP.md) | ② |
| Q-CAPE3009 | 129 · any | [a delayed capture into a hierarchical `real` from an integral source is refused](roadmap/5.2/Q-CAPE3009.md) | ② |
| Q-STRFMTARG | 130 · any | [a string variable as `$display`'s first argument is used as a format](roadmap/5.2/Q-STRFMTARG.md) | ② |
| P-ARGV | 131 · any | [a command-line argument that is not UTF-8 panics before the design runs](roadmap/5.2/P-ARGV.md) | ② S |
| Q-SRCBYTES | 132 · any | [a source file that is not UTF-8 is refused](roadmap/5.2/Q-SRCBYTES.md) | ② |
| Q-FLISTUTF8 | 133 · any | [a `-f` file list re-encodes a UTF-8 path as Latin-1](roadmap/5.2/Q-FLISTUTF8.md) | ② |
| Q-SVASCOPE | 134 · any | [names inside an SVA bind the wrong object](roadmap/5.2/Q-SVASCOPE.md) | ① |
| Q-QNBA | 135 · any | [a nonblocking write to a queue element is accepted](roadmap/5.2/Q-QNBA.md) | — |
| Q-REFTXT | 136 · any | [refusal text: duplicate lines, wrong columns and a wrong order](roadmap/5.2/Q-REFTXT.md) | — |
| Q-REFILLEGAL | 137 · any | [IEEE-illegal `ref` actuals are accepted and written through the copy-out](roadmap/5.2/Q-REFILLEGAL.md) | — |
| Q-W4023LOC | 138 · any | [a run-time W4023 loses its location in staged runs and under JIT](roadmap/5.2/Q-W4023LOC.md) | — |
| Q-FORCEAUTO | 139 · any | [`force` to an automatic variable is accepted](roadmap/5.2/Q-FORCEAUTO.md) | — |
| Q-FNTIMING | 140 · any | [a function body accepts a timing control or a task enable](roadmap/5.2/Q-FNTIMING.md) | — |
| Q-CVARPATEXT | 141 · any | [refusal text: a procedural assign beside a continuous one is named as a force](roadmap/5.2/Q-CVARPATEXT.md) | — |
| Q-STRPORT | 142 · any | [a `string` input port runs silently wrong where the manual says it is refused](roadmap/5.2/Q-STRPORT.md) | ① |
| Q-XAGG | 143 · any | [an unpacked struct `?:` at an x condition is merged bit by bit](roadmap/5.2/Q-XAGG.md) | ① |
| Q-SCANXZSRC | 144 · any | [a `$sscanf` source with x or z bits converts instead of returning EOF](roadmap/5.2/Q-SCANXZSRC.md) | ① |
| Q-PCTC-ENG | 145 · any | [`%C` in a scan format is not read](roadmap/5.2/Q-PCTC-ENG.md) | ① |
| Q-PKGMEMBW | 146 · any | [a member write through a package alias of a packed struct is refused](roadmap/5.2/Q-PKGMEMBW.md) | ② |
| Q-TDREDECL | 147 · any | [a type name declared twice in one package is accepted](roadmap/5.2/Q-TDREDECL.md) | ② |
| Q-ESCTDKEEP | 148 · any | [an escaped type name holding `::` escapes its module](roadmap/5.2/Q-ESCTDKEEP.md) | ② |
| Q-AMBIMPORT | 149 · any | [a type name offered by two wildcard imports binds instead of being refused](roadmap/5.2/Q-AMBIMPORT.md) | ② |
| ENG-4 | 150 · held | [an `iff` guard is evaluated at resume, not at the store that makes the event](roadmap/5.2/ENG-4.md) | ① F |
| Q-SHORTREAL | 151 · held | [`shortreal` is not supported](roadmap/5.2/Q-SHORTREAL.md) | ② |
| 10 | 152 · hygiene | [split production files over the 1,000-line policy not on its exception list](roadmap/5.2/10.md) | — |
| Q-ALWINTRA | 153 · any (PARKED) | [an `always` whose only timing is an intra-assignment delay runs once](roadmap/5.2/Q-ALWINTRA.md) | ① |
| Q-STREMPTY | 156 · any | [an empty string literal in a string concatenation is sized as one bit](roadmap/5.2/Q-STREMPTY.md) | ① |
| Q-STRREPNC | 159 · any | [a string replication with a non-constant multiplier is refused](roadmap/5.2/Q-STRREPNC.md) | ① |
| Q-ZEROREPL | 161 · L1 | [the zero-count replication rule admits illegal shapes and refuses legal ones](roadmap/5.2/Q-ZEROREPL.md) | ① |
| Q-STRREPSHADOW | 162 · any | [a string replication count folds a parameter that a function formal shadows](roadmap/5.2/Q-STRREPSHADOW.md) | ① |
| Q-PKRDLSB | 163 · any (PARKED) | [a part-select of a non-zero-LSB packed element of an array word reads wrong bits](roadmap/5.2/Q-PKRDLSB.md) | ① |
| Q-CMPIDXTWICE | 164 · any | [a compound assignment evaluates its lvalue's index expression twice](roadmap/5.2/Q-CMPIDXTWICE.md) | ① |
| Q-GENVARALIAS | 165 · any | [a genvar reusing a variable's or an imported name is accepted](roadmap/5.2/Q-GENVARALIAS.md) | ① |
| Q-IDX64TRUNC | 166 · any | [an unpacked word index wider than 32 bits is truncated to its low bits](roadmap/5.2/Q-IDX64TRUNC.md) | ① |
| Q-TASKPKIDX | 167 · any | [a packed index reading a module variable in an automatic task writes element 0](roadmap/5.2/Q-TASKPKIDX.md) | ① |
| Q-SELNORM | 168 · any | [select-index normalization: unsealed narrow calls and bounds clamped at i32](roadmap/5.2/Q-SELNORM.md) | ① |
| Q-SELNEIGH | 169 · any | [neighbours of the element select: odd index kinds, generate `let`, class scope](roadmap/5.2/Q-SELNEIGH.md) | ① |
| Q-ALWBUDGET | 170 · any | [an engine budget: a pass that never suspends ends in a fatal, not OOM or a hang](roadmap/5.2/Q-ALWBUDGET.md) | — |
| Q-ALWRESIDUE | 171 · any | [defects Q-ALWINTRA's reviews found that stand without its rule, one per clause](roadmap/5.2/Q-ALWRESIDUE.md) | ① |
| Q-PATNOREC | 172 · any | [a keyed assignment pattern with no target record is refused at two page lines](roadmap/5.2/Q-PATNOREC.md) | ② |
| Q-PKGPARPAT | 173 · any | [a package parameter with a keyed struct pattern value is not folded](roadmap/5.2/Q-PKGPARPAT.md) | ② |
| Q-PKGUNIQ | 174 · any | [a package call reaching a `unique` violation is refused, on four OpenTitan pages](roadmap/5.2/Q-PKGUNIQ.md) | ② |
| Q-NETCAP | 175 · any | [the v1 net and variable count cap stops two OpenTitan pages](roadmap/5.2/Q-NETCAP.md) | ② |
| Q-TYPEOVERVAL | 176 · any | [a cast or `$bits` of a name binds a visible typedef over the scope's own value](roadmap/5.2/Q-TYPEOVERVAL.md) | ① |
| Q-FRAMEREAL | 177 · any | [a real assigned to an element of a frame-local array is not rounded](roadmap/5.2/Q-FRAMEREAL.md) | ① |
| Q-FRAMECONCAT | 178 · any | [a concatenation assigned to a frame-local unpacked array fills it in reverse](roadmap/5.2/Q-FRAMECONCAT.md) | ① |
| Q-ENUMCONV | 179 · any | [an implicit conversion to an enum is accepted](roadmap/5.2/Q-ENUMCONV.md) | — |
| Q-PORTDEFAULT | 180 · any | [an ANSI port default is parsed and never lowered](roadmap/5.2/Q-PORTDEFAULT.md) | ① |
| Q-SVALOCALKIND | 181 · any | [an SVA local variable's declared 2-state kind is dropped](roadmap/5.2/Q-SVALOCALKIND.md) | ① |

Do not start:

- §2, every row — frozen (§2 preamble) until a corpus hit, a corpus fix path or an external report.
- §3.b `pkg-type-param-import`, `scoped-call-comb-arg` and the other synthetic-origin residues of §3.b and §2 — no corpus witness. ([note](roadmap/5.2/README.md))
- §2 row 16 — an oracle split on the override's context.
- §2 row 34 (one oracle, zero demand) and row 31 (correct; performance).
- §2 row 32 — the `$finish`-in-a-function boundary is a split.
- §2 🆕 Q — needs a block-scoped constant binding (a bare hoist makes 5 silent-wrongs).
- §2 row 7's remainder — hierarchy start order and wake-group order are splits.
- §3 ⑤ⓕ ARITY — no per-instance declarator dims (`decls.rs` stamps once); needs an arity marker on `Dim` / `DeclName` (SchemaHash roots).
- §3 ⑤ⓕ CLASS PROPERTY — needs per-instance class registration; a property typed by a module alias opens with §5.2 row 68, the rest stays here. ([note](roadmap/5.2/README.md))
- §3 ⑤ⓕ function RETURN type — 1 oracle; needs unpacked dims on `FunctionDef` (hdl-ast re-pin).

Oracle-split axes (§2 "Oracle splits") are never chased.

## 6. G2 — the AI-agent observability track
> The AI-agent observability track: six staged deliverables and ten open items beside them, ranked third behind §2 and §3.

SPEC = [preview/19-ai-agent-observability.md](preview/19-ai-agent-observability.md); its teeth and the track's rank are in the notes ([notes](roadmap/6/README.md))

| stage | deliverable | size |
|---|---|---|
| OBS-2 residue | `sva.jsonl` (R-L6) · per-element array probe · real / class / event probe | M |
| OBS-1 residue | staged obs · compile-fail manifest · `--seed` · `run_id` · `-G` in `run.json` · `-D` in `source.blake3` · `results.jsonl` v2 · R-L2 `fail/*.json` · R-L5 SVA / cover counts | S-M |
| R-L4 | log-channel separation | M |
| OBS-4 | `vrun --control stdio` JSON-RPC plus a poke journal | L |
| OBS-5 | snapshot / restore / rewind | L-XL |
| OBS-6 | X-origin · region-annotated events · a static backward slice | L+ |

Open items beside the staged track:

- call-tree-inlined — [the call tree reports no calls for an inlined subroutine](roadmap/6/call-tree-inlined.md) · BLOCKED
- subroutines-void-kind — [`subroutines` files a void function as a task](roadmap/6/subroutines-void-kind.md) · OPEN
- frame-inline-route-key — [the frame or inline route is keyed per module and name](roadmap/6/frame-inline-route-key.md) · OPEN
- builtin-call-site-rows — [per-call-site builtin rows are not emitted](roadmap/6/builtin-call-site-rows.md) · OPEN
- staged-obs-flags-dropped — [a staged run silently drops `--hier-tree` and `--inst-paths`](roadmap/6/staged-obs-flags-dropped.md) · OPEN
- hier-tree-generate — [`--hier-tree` collapses generate scopes](roadmap/6/hier-tree-generate.md) · OPEN
- enum-value-names — [enum values cannot be reported as names](roadmap/6/enum-value-names.md) · OPEN
- introspection-txn-log — [partial introspection and a transaction log with no producer](roadmap/6/introspection-txn-log.md) · OPEN
- builtins-lowering-rows — [the `builtins` table counts lowering artifacts as calls](roadmap/6/builtins-lowering-rows.md) · OPEN
- wprog-keys-no-producer — [six `wprog` keys have no producer](roadmap/6/wprog-keys-no-producer.md) · OPEN

Non-goals: FSDB / UCDB, an embedded SQLite, a waveform GUI, UVM-specific rail hooks (running a UVM testbench is §7 UVM). VCD stays the human-facing format. ([notes](roadmap/6/README.md))

## 7. Conditional / long-term (promoted only when the re-entry trigger fires; orthogonal to correctness)
> Long-term and deferred items, each held until its re-entry trigger fires; deferred real-design rows keep their row number, source and rank

| id | item | trigger |
|---|---|---|
| BACKEND | [backend options: 2-state mode, PDES BSP, a native-eval lane, a cranelift JIT](roadmap/7/BACKEND.md) | W≥64 with grain ≥200 ns · low ROI · §5.a codegen |
| VHDL | [a VHDL front end](roadmap/7/VHDL.md) | an SV plateau, a value-domain decision, GHDL setup |
| VCD-EXT | [`$dumpports*` extended VCD output](roadmap/7/VCD-EXT.md) | waveform-tool demand |
| MVP-CUT | [features cut from the MVP, each reopened on individual demand](roadmap/7/MVP-CUT.md) | individual demand |
| FF-MISSED-AT | [run a header-less `always_ff` whose single event control some pass misses](roadmap/7/FF-MISSED-AT.md) | §5.b NBA-GROWTH closed AND a real design uses the shape (corpus: 0 uses) |
| UVM | [run an IEEE 1800.2 (UVM) testbench](roadmap/7/UVM.md) | the real-design program's rows land, a UVM testbench of a permissive-licence design is a corpus candidate, and an oracle runs it |

Deferred real-design rows keep their row number, source and rank and return to §5.2 when their trigger fires ([notes](roadmap/7/README.md))

| # | item | rank | trigger |
|---|---|---|---|
| 54 | [interface port actuals: a modport expression and a passed-on interface port](roadmap/7/54.md) | ② S | the owner promotes the EL2 page |
| 55 | [a covergroup-typed block-local declaration with an initializer is refused](roadmap/7/55.md) | ② L | the owner promotes the EL2 page |
| 56 | [an empty parameter port list `module m #() ();` is refused](roadmap/7/56.md) | ② L | the owner promotes the EL2 page |
| 57 | [`if` / `else` inside a class constraint block is refused](roadmap/7/57.md) | ② S | the owner promotes the EL2 page |
| 58 | [`std::randomize(v) with {…}` as a value expression is refused](roadmap/7/58.md) | ② S | the owner promotes the EL2 page |
| 59 | [a hierarchical write of a whole unpacked array is refused](roadmap/7/59.md) | ② F | the owner promotes the EL2 page |
| 60 | [a wire with a part-select driver beside a whole driver is a false multi-driver](roadmap/7/60.md) | ② F | the owner promotes the EL2 page |
| 64 | [corpus row: VeeR EL2 on unmodified RTL under its first-party testbench](roadmap/7/64.md) | — C | the owner promotes the EL2 page |
| 61 | [DPI-C phase 1: imported functions with Annex H's basic types](roadmap/7/61.md) | ② F | a corpus design needs DPI-C |
| 62 | [DPI-C phase 2: exports, import tasks, open arrays and scopes](roadmap/7/62.md) | ② F | a corpus design needs DPI-C's phase-2 forms (an export, an import task, an open array) |
| 63 | [corpus row: verilog-pcie's `dma_if_pcie_us` under a first-party testbench](roadmap/7/63.md) | — C | the owner promotes the verilog-pcie page |

## 8. Non-goals (permanent, not gaps)
> Permanent non-goals, not gaps: implicit nets and the out-of-scope list, and `defparam` limited to a direct child with a constant value.

- non-goal-list — [implicit nets and the list of out-of-scope features](roadmap/8/non-goal-list.md)
- defparam-depth — [`defparam` only on a direct child with a constant value](roadmap/8/defparam-depth.md)
