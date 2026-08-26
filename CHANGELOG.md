# Changelog

All notable changes to **vitamin** are recorded here. The format loosely follows
[Keep a Changelog](https://keepachangelog.com/); dates are ISO-8601.

Per-slice engineering detail lives in [docs/ROADMAP_ARCHIVE.md](docs/ROADMAP_ARCHIVE.md)
(§4.5.x) and [docs/DEVLOG.md](docs/DEVLOG.md); the sections below summarise what
changed for a user of the simulator.

## [Unreleased]

### Added

- **Named and `default:` assignment patterns** (IEEE 1800 §10.9.1/§10.9.2):
  `c = '{mode: 4'h3, en: 1'b1, len: 8'd7}` for a packed struct, and
  `a = '{default: 5}` for a packed struct or a fixed-size unpacked array (any
  bounds, 1-D or multi-dimensional). They mix — `'{mode: 4'h5, default: 1'b0}`
  gives the named member its value and every other member the default. All three
  work in a procedural assignment, in a declaration initializer, and inside a
  task or function body.

  This is worth reaching for even where the positional form already worked. A
  positional pattern is coupled to the declaration order, so **inserting a member
  silently shifts every later value** — and that failure surfaces as a wrong
  result at exit 0, not as an error. The named form makes it impossible.

  `default:` is applied to each member SEPARATELY, in that member's own width, so
  on a `{logic [3:0] mode; logic en; logic [7:0] len;}` struct `'{default: 1'b1}`
  is `13'h0301` and not all-ones.

  Still loud, on purpose: an integer key (`'{0: a, 1: b}`), a type key
  (`'{int: 0}`), a replication (`'{N{e}}`), a keyed pattern whose target is a
  dynamic array / queue / packed array / subroutine argument / continuous assign,
  a member left uncovered, an unknown or duplicated member name, mixing
  positional and keyed elements in one pattern, and a call in the `default:`
  value (which would run once per member instead of once). Note that Icarus
  Verilog 13 rejects every keyed pattern outright, so a design using this form
  will not build there.

- **`--backend <interp|vm|native>`** on `vita` and `vrun` — selects which executor runs
  process bodies. **`native` is the default** and runs every design; `interp` is the
  readable reference semantics and `vm` the bytecode compiler, and both exist to bisect a
  suspected defect against a second implementation. **Output is byte-identical across all
  three**, enforced over the whole corpus by `sim-engine/tests/backend_equiv.rs`, so this
  is a wall-clock knob like `--threads`. `vcmp`/`velab` reject it: nothing in the artifact
  they write depends on the backend, and accepting it would suggest otherwise. A build
  made without the `oracle` feature has only `native`.
- **`sim_engine::codegen_coverage`** (also `run.json`'s `codegen` under `--obs-dir`)
  reports how many of a design's process templates get the compiled op-stream path — the
  rest walk the IR, on `native` as well.

  ⚠️ **Corrected 2026-08-18.** This entry used to say "Real designs measure 50–67%, the
  remainder being the `#delay`-bearing stimulus half. Writing transforms as `function`
  costs nothing: vitamin inlines them during elaborate." An external report measured
  **12–16%** on a function-heavy AES design, with **`user_call_in_expr` 87% of the
  rejections** and `delay` a rounding error — and they were right. Re-measured here, the
  discriminator is not package-vs-module.

  ⚠️ **Corrected again 2026-08-25.** The 08-18 correction replaced one wrong claim with a
  narrower one — "the discriminator is one keyword" — and a second external report
  measured that too. It is not one keyword. **Control flow in the body blocks inlining
  just as `automatic` does**, and a package-QUALIFIED call blocks it at the call site.
  Re-measured here on one call shape (`always_comb b = f(a);` + `always_ff c <= f(a);`),
  varying only the body or the spelling:

  | function form / call spelling | inlined during elaborate? | `able` |
  |---|---|---|
  | `function f(…)`, straight-line body | **yes** — no `Expr::Call` survives | 3/5 |
  | …with a local variable, or calling another function | **yes** | 3/5 |
  | …with a ternary `?:`, a concat, a part-select | **yes** | 3/5 |
  | …reading a module-level signal | **yes** (deliberate — framing would drop it from the `always_comb` sensitivity list) | 3/5 |
  | …containing `for` / `while` / `repeat` | **no** | 1/5 |
  | …containing `if` / `else` | **no** | 1/5 |
  | …containing `case` | **no** | 1/5 |
  | …containing a `$display` or any system task | **no** | 1/5 |
  | …written `return x + 1;` instead of `f = x + 1;` | **no** | 1/5 |
  | **`function int` / `function bit [7:0]` / `function byte`** | **no** — any 2-state return type | 1/5 |
  | …with an unpacked-array formal | **no** | 1/5 |
  | …whose return type is WIDER than its top-level operator | **no** — the context-width route | 1/5 |
  | `function automatic f(…)`, straight-line body | **no** — lowered to a frame body | 1/5 |
  | …with an OUTPUT formal + a control-flow body | **no** — reason `frame_call`, not `user_call_in_expr` | 1/5 |
  | `import p::*;` then `pf(a)` | **yes** | 3/5 |
  | `p::pf(a)` (qualified) | **no** — per CALL SITE | 1/5 (2/5 if only one of the two sites is qualified) |

  ⚠️ **Corrected a third time, 2026-08-26.** Two of the rows above were wrong and one
  sentence pointed the wrong way.

  * The qualified-call row said **2/5**. The refusal is per CALL SITE, not per function,
    so on this table's own two-call shape it is **1/5**; 2/5 appears only when exactly one
    of the two sites is qualified. And the route is not a resolver miss to be opened
    cheaply — `inline_pkg_function` frames it deliberately, with the reason written out
    (the inline fold evaluates the return expression at its self-determined operand width
    and resizes afterwards, so an 8-bit `a+b` bound to a 16-bit return keeps 8 bits and
    `255+255` is 254, not 510). All three spellings measure 510 today.
  * The table named 4 of the **12** shapes that frame. The biggest omission for SV RTL is
    a **2-state return type**: `function int f` frames where `function logic [31:0] f`
    with the identical body inlines.

  * ⚠️⚠️ **And `able` is ANTI-correlated with speed on exactly the body shape the report
    cites.** The closing sentence used to file *"widen the inliner to control-flow bodies"*
    as the improvement. Measured, interleaved, same design, only the body wrapped in
    `if (1'b1) … else …`:

    | body | inlined | framed | winner |
    |---|---|---|---|
    | 6 chained statements, local read **once** each | 0.19 s | 0.24 s | inline 1.27× |
    | …read **twice** each | 1.77 s | 0.31 s | **framed 5.6×** |
    | …read **three times** each | 14.39 s | ~0.35 s | **framed ~40×** |

    Digests are byte-identical in every pair, and `elab_s` stays flat at 0.35 ms across a
    0.16 s → 14.36 s spread of `sim_s` — so the arena SHARES the substituted subtree as a
    DAG and the evaluator re-walks it as a TREE. The inline fold is therefore
    kᶰ in (references per statement)^(chained statements) where the frame path is linear,
    and the shape that blows up — an accumulator re-read inside a chain — is what a
    cryptographic combinational function is made of. Widening the inliner would convert
    the reporting design's 0.31 s into 1.8 s or 14 s at unchanged output.

    ⇒ **`able` is a coverage number, not a speed proxy.** The item filed in ROADMAP §3 is
    now the DAG re-walk (memoise a shared sub-expression per activation), not the inliner.

  So on real RTL `automatic` is rarely the binding constraint: the reporting design has 24
  `function automatic` in its RTL, and removing the keyword from all 24 moved
  `user_call_in_expr` from 262 to 261 — **0.4%** — because 16 of those 24 contain `for` or
  `case`. That observation stands; what changed is what to do about it.

### Documentation

- **Several documents still described the simulator as running on an interpreter.**
  Phase B/D made `native` — a compiled op-stream over a flat arena — the default and
  the only executor a released build contains, but the user manual still said "the
  backend is a deterministic IR-walking interpreter", the CLI reference still listed
  `--backend <interp|vm>` with **"`interp` (default)"**, and three `docs/preview/`
  specs still introduced the backend as `인터프리터 방식 (IR-walking)` in the present
  tense. A reader — human or model — could reasonably conclude vitamin interprets.

  Worst of these, because it is the document an agent reads to interpret vitamin's
  output: **`docs/preview/19-ai-agent-observability.md` claimed `--backend native`
  "still falls back to the VM" and that `"native"` appears only in
  `backend_requested`.** Neither is true — an ordinary run writes `backend: native`,
  `backend_requested: native`, `native.refused: null` — so an agent following that
  spec would read the `backend` field and conclude native had not run.

  All corrected against the running binary. The manual's per-backend speedup table
  was **removed rather than refreshed**: stale numbers are what made the section
  wrong, so it now states the ordering (`native` < `vm` < `interp`) and the commands
  that reproduce it, leaving the numbers in one place. That ordering was re-measured
  for this change on `bench/picorv32` — 18.0 s / 33.9 s / 52.8 s, release, warm — on
  its own default testbench settings, so those figures are not comparable to the
  corpus runner's picorv32 row, which drives a different configuration. The 2026-05 design spec, which predates all of this,
  carries a HISTORICAL banner instead of being rewritten.

### Fixed

- **The constant domain now computes in the width the declaration states, not in i64.**
  Two external reports (round-33 and an AES IP audit) converged on one axis: every
  crypto/AXI constant idiom that mixes a NAME with a wide value was rejected, and which
  operator you wrote decided whether the parameter existed at all. `A ^ 128'h1` folded
  and `A ^ B` — same value, named — did not; `+`, `-`, `*` and the comparisons had no
  wide arm; a select of a >64-bit parameter (`A[127:64]`, `A[127]`) had none either; and
  a reduction (`^A`) or `$countones` had none at ANY width. All of them fold now, and
  every value was measured against **both** iverilog 13.0 and Verilator 5.050.

  The pieces, because they were separate causes:
  - **A NARROW named parameter is now an operand of the wide fold**, read at its
    DECLARED width. The width comes from `param_range` — the map that records only
    declared provenance — and not from `param_meta`, where widths inferred from a value
    also live. That distinction is the whole soundness argument: §4.5.373 built the
    reductions on `param_meta` and measured `localparam W = 4'hF | 4'h0;` reducing 32
    bits where both oracles hold 4, then reverted. With the range declared, the stored
    value IS canonical at that width — measured on the same counterexample.
  - **Wide arithmetic and comparison arms** (`+ - *`, `< <= > >= == != === !==`, `&&`
    `||` `!`), the **reductions** (`& | ^ ~& ~| ~^`), **bit / part / indexed-part
    selects**, the **conditional**, `>>>`, and `$countones` / `$onehot` / `$onehot0` /
    `$signed` / `$unsigned`. Division and modulus stay out.
  - **A package parameter may be wider than 64 bits.** `localparam logic [127:0] K =
    128'he1…;` inside a `package` was `E3009` while the identical declaration in a
    module, in a `#()` header, in a one-element array or inside a packed struct all
    worked — the package's scalar path simply had no fourth domain to fall into. It
    travels through wildcard and explicit imports and through `pkg::K`.
  - **An override carries its WIDTH.** IEEE §6.20.2 gives an untyped parameter the range
    of its FINAL override value, and the i64 channel carried no range, so
    `#(.M_ISSUE(M_ISSUE))` forwarding `{2{32'd4}}` arrived 35 bits wide where every
    other tool has 64 — and the per-port slice `M_ISSUE[32 +: 32]` then read past the
    end. A `-G K=128'h…` override, which could previously only be refused, now applies.
  - **A replication COUNT may be a name.** `{N{32'd2}}` and `parameter [S*32-1:0] MASK =
    {S{…}}` fold. §4.5.371 built this and reverted it over four blocking defects; all
    four are answered by folding the count through the *same* name resolver the
    surrounding fold uses rather than through a second evaluator.
  - **Package `real` reaches module-scope constants.** `localparam real Q = pk::R*2.0;`,
    `int'(pk::R*2.0)`, a `parameter real` port default and `generate if (pk::R > 2.0)`
    were `E3009` while a byte-identical MODULE-LOCAL real folded and the package
    string crossed the same boundary fine.
  - **String methods fold in a constant context.** `localparam int W = S.len();` and the
    `getc` / `compare` / `ato*` family.

- **`generate` / `endgenerate` are optional (IEEE 1800-2017 §27.3), and a `genvar` may be
  declared in the `for` header (§27.4).** `if (…) begin … end` and `for (…) begin … end`
  at module scope are the dominant modern spelling and what synthesis tools are handed;
  vitamin required the wrapper, and the error it produced pointed at the `end`/`else`
  that followed rather than at the missing keyword.

- **A package's STATIC `function` could not call its own sibling under a selective
  import.** `import p::gmul;` made `gmul`'s body's call to `xtime` an
  `E3010 call to undeclared function`, while spelling the same function `automatic`
  worked and so did importing `xtime` as well — which is the caller's business, not the
  callee's. IEEE §26.3 resolves a routine's body in its DECLARING scope; only the frame
  path carried that scope, and the inline path now carries it too (around the BODY only,
  never around the actual arguments, which belong to the caller).

- **A `pkg::`-qualified value may take a method.** `pk::S.len()` was three parse errors
  at one column, none of which contained the words "method", "package" or "::".

### Diagnostics

- **A constant-fold rejection now points at the declaration and names the cause.** The
  caret came from the elaborator's ambient span, which during module elaboration is the
  MODULE HEADER — so every rejection in one module printed the same `file:line:col`, at
  a line holding no parameter, and finding the culprit meant commenting declarations out
  one at a time. And the text stopped at *"value is not a foldable constant
  expression"*, so three declarations rejected for three unrelated reasons (a package
  `real`, a string method, a replication count) were indistinguishable — none of the
  words `real`, `string` or `replication` appeared in any of them, and when the cause
  was an UNDEFINED NAME the name was in hand and thrown away.

- **A missing package file produces one error that names the package, not seven that do
  not.** `function f(input q::mode_e m);` with `q` absent produced seven `E2002`s — the
  first at the `::`, the rest at perfectly correct declarations further down that only
  failed because the port list never closed — and the package's name appeared in none of
  them.

- **An enum method rejection describes enum methods.** `x.name()` on an enum whose label
  names a `parameter` was reported as an *"unsupported hierarchical function call … the
  callee must be a framed function with input-only scalar formals … reached through an
  instance path"*, which describes a different feature entirely, and carried no
  `file:line` at all.

- **A non-constant bound in a declaration is reported once, at the bound.** The range is
  folded by more than one pass, so the same `$clog2(n)` printed twice, both times with
  the caret on the `logic` keyword rather than on the `$clog2`.

- **An unconnected child INPUT warns.** vitamin warned about a dangling `output` and
  `inout` and was silent about a dangling `input` — which is backwards with respect to
  consequence: a dangling output discards a value, a dangling input MANUFACTURES one
  (`z` at time 0) and propagates it. `W-ELAB-FEATURE-LIMIT`, and a ROOT module's own
  ports stay silent as before.

- **A variable with BOTH a declaration initializer and an `always_comb` driver warns.**
  `logic rdy = 1'b1; always_comb rdy = …;` ran at exit 0 with nothing to say, while
  xcelium stops elaboration (`*E,MULAXX`) and Verilator errors (MULTIDRIVEN). Nothing
  about the simulated value changes; this is the warning that was missing.


- **A `real` constant could not reach an integer context, even when you wrote the
  conversion.** `logic [int'(R)-1:0] v;`, `{int'(R){1'b1}}`, `$clog2(R)` in a width, and
  `localparam int M = R*2.0;` were all rejected with `VITA-E3009` — while this manual told
  users to "convert it explicitly with `int'()` or `$rtoi()`", advice that did not work.
  All of them now fold, matching both iverilog and Verilator, at module, `generate` and
  package scope alike. `int'()` rounds half **away from zero** (`int'(-2.5)` is −3) and
  `$rtoi` truncates, per §6.24.1 and §20.10.

  The expression is evaluated **whole in the real domain** and only the converted result
  becomes an integer, so `R/2` is still `1.75` and `generate if (R/2 > 1)` still tests
  `1.75 > 1`. An earlier attempt converted at the leaf instead and silently picked the
  wrong `generate` branch; that is why an IMPLICIT conversion (a bare `R` in a width bound
  or a replication count) stays rejected. It is not a missing feature — the two reference
  tools disagree about those, in opposite directions: iverilog sizes `[R-1:0]` while
  Verilator rejects the design, and for `{R{1'b1}}` iverilog rejects while Verilator
  replicates. With no agreed answer, vitamin asks you to write the conversion.

- **`v[int'(<real param>):0]` selected one bit, silently.** A part-select whose bound was
  an explicitly converted real read a single bit at exit 0 where iverilog reads the whole
  range (`v[int'(3.5):0]` is `v[4:0]`). Same root as the next entry.

- **An unfoldable cast in a declaration bound declared one bit, silently.** `logic
  [int'(NOPE)-1:0] v;` ran at exit 0 with a 1-bit net, while the bare `[NOPE-1:0]` twin was
  already loud about that same undefined name. A bound that cannot be folded is now loud
  whatever node it is written with.

- **`$clog2` of a real literal produced an empty replication.** `{$clog2(8.0){1'b1}}`
  replicated **zero** times at exit 0; both oracles replicate 3.

- **`$bits(<expression>)` used as a packed declaration bound silently declared one bit.**
  `wire [$bits(8'h00)-1:0] c;` produced a 1-bit net, so an 8-bit assignment truncated to
  `1` at exit 0 in every backend — and on a port it truncated across the module boundary.
  The same call already answered 8 at runtime and 8 as an unpacked dimension, so one source
  line had three answers. It now folds for a literal, a concatenation and a replication;
  shapes the constant domain still cannot see are LOUD instead of silently one bit
  (previously even `$bits(<undeclared name>)` declared a 1-bit net). Reported externally.

- **Two rejection diagnostics quoted a restriction that had been removed.** The file-read
  system functions and `$value$plusargs` both said they are "supported only as the direct
  rhs of a blocking assignment", which stopped being true when those calls were opened for
  `if` conditions, `case` scrutinees, nonblocking right-hand sides and more. Following the
  message made a user revert working code. Both now state their actual — and different —
  reasons, and the caret points at the call rather than at the enclosing statement.

- **`a[7:0][3:0]` produced no portability warning** while `(a^b)[3:0]` did, even though the
  same `W2004` text says iverilog rejects every form. iverilog states the rule exactly —
  all but the final index in a chain must be a single value, not a range — so the warning
  now covers it. `mem[i][3:0]`, which is legal everywhere, stays silent.

- **Two diagnostics contradicted each other on one event.** A parameter override wider than
  the 64-bit override channel warned "is not a constant; default kept" while a companion
  check errored — it is a constant, and no default was kept. The wide-literal case is now
  named for what it is; a genuinely non-constant override still reports that its default
  was kept, because there that is true.

### Fixed

- Two doc comments (`sim-engine/src/lib.rs`, `tests/backend_equiv.rs`) still described the
  Stage-B state — "the VM is not yet built, so ALL bodies fall back" — which the dispatch
  in `sched/scan_arm.rs` has contradicted since Stage C landed.

## [0.1.0] — 2026-07-31 · initial release

The first tagged release. Everything below this heading is the development
history that led to it.

**The major version stays at `0` deliberately.** `tool_semver_major` is a hard
staleness gate (`crates/vita-artifact/src/gate.rs`), so a `0 → 1` bump would
invalidate every existing `.velab` / `.vu` artifact — and semver-0 is the honest
signal while [docs/ROADMAP.md](docs/ROADMAP.md) §2/§3 still carry open
correctness items.

Release state: **5009 tests** green on Ubuntu, macOS and RHEL 9 · `format_version`
**26** · MSRV **1.85** · 59 diagnostic codes.

### Fixed

- **A call's copy-out destination was invisible to the classifier.** `Terminator::Call`
  carries only `{target, ret_bb}`; the destinations live in a call-site side table, so a
  statement-level walk over lvalues never saw them. A bare call inside a call frame whose
  `output`/`inout` actual was a module net therefore reached an executor that cannot route
  that write and **panicked with no diagnostic and no source location** (exit 101). Three
  earlier slices had recorded the trigger as "not yet named" and reverted twice, because
  they were looking at statement *position* rather than at the classifier's blind spot.
  Fixing it also turned four neighbouring loud rejections into working support.
- **An unrelated `$display("x")` decided which executor ran a subroutine body.** The
  classifier looked at a blocking assignment's *destination* only, so an effect carried in
  the right-hand side was invisible. Functions shared the same root; a `static task`'s
  `string` local needed a third collector; and `$fatal` could lose to its own `$finish`.
- **A frame-local `string`'s byte writes went to the wrong storage.** `s[i] = c` wrote
  through the dynamic heap while a frame-local string is slab-stored in the frame slot, so
  the write silently vanished. (Pre-existing; surfaced when the loud gate above was lifted.)
- **Eleven further silent-wrongs** found while fixing the round-20 report, including a
  constant fold that was not shadow-aware and a stand-down guard whose scope was a whole
  function and whose key was module-global.

### Changed

- `E3009`'s message no longer claims that a bare call statement in that position works —
  that is precisely the form that used to panic. It now names the real boundary, and a
  rejected call terminator is no longer reported as "a timing/suspend/fork control".
- An unroutable frame write is a **runtime fatal naming the net** instead of a panic.
- `--threads` / `-j` help text now says what it is: a waveform-writer budget. Simulation
  is single-threaded, and the flag never affected it.

### Notes on performance

An external report attributed a 4.3× slowdown at combinational depth 6 to vitamin not
levelizing deep combinational cones. Measured against Icarus Verilog with total work held
fixed, **Icarus scales the same or worse** (4.15× vs 3.55× at depth 12) and vitamin's
absolute wall time is 0.80–0.99× of it. The depth cost is a property of interpreted
event-driven delta cycles, not a defect; the gap to a compiled, compile-time-levelizing
simulator is a separate axis, analysed in
[docs/preview/18-acceleration-analysis.md](docs/preview/18-acceleration-analysis.md).

## [2026-07-29 · round-19/20]

### Fixed

- **A call that RETURNS A VALUE now counts as writing its `output` actual.** The
  definite-assignment walk recognized that write only where a call can stand as a
  *statement* — a bare enable, or a whole `if`/`while` condition. The moment the call
  returns a value it can sit anywhere an expression can, and there the walk asked only
  "does this mention the name?", for which a mention is always a read. So
  `go = nxt(5, r);`, `if (nxt(5, r) == 1)` and `while (n < lim && rsp_next(fd, r) == 1)`
  — the standard table-driven `.rsp` / CAVP vector walker — were all rejected. That was
  33 of the 34 diagnostics in the round-19 report, and the whole of its `TB=sha2`
  regression.
- **A branch knows what its condition evaluated to.** `a && f(out r)` is true only when
  BOTH operands ran, so the loop body / `then` branch knows `r` is written even though
  the loop exit does not; `a || f(out r)` is false only when both ran, which is what an
  `else` branch gets. (Verified against iverilog 13 with an observable side effect in
  place of the copy-out.)
- **`void'(f(…, out r));` and the bare `f(…, out r);` statement now lower.** Discarding
  the return value is the natural way to make an out-formal call a statement — and it
  was rejected, which left a testbench no way to express the write at all. Statement
  position is in fact the easiest case: the return slot goes to a throwaway temp and the
  copy-out (including an `inout`'s copy-in) happens. The "supported positions" message
  had also gone stale by omitting statement position; it now lists it.
- **A named argument no longer ends the definite-assignment walk.** The call resolvers
  bailed on the first `.formal(v)` they saw, so a call that uses one precisely to leave
  the other defaults alone read as unanalysable and the walk blamed a local several
  arguments to its left. Positional-then-named mapping (IEEE 1800 §13.5.4) is now
  applied. Building it exposed that the output/inout-formal function path never
  performed the named-argument reorder at all, and produced two diagnostics naming
  neither cause.
- **SILENT-WRONG: a default argument value was evaluated in the CALLER's scope.**
  IEEE 1800 §13.5.4 evaluates it where the subroutine is declared. A caller that
  declared its own `g` therefore hijacked a callee default that names `g`: vita printed
  `91` where iverilog prints `6`, at exit 0 with no diagnostic. The same hazard for a
  class method's default was already closed; the plain function/task twin was not. The
  guard compares BINDINGS rather than banning names, so a default naming a module net
  still resolves — from a module process and from a generate block alike.
- **SILENT-WRONG: file reads inside a framed subroutine body returned 0.** `$fgets`,
  `$fscanf`, `$sscanf`, `$fread`, `$fgetc` and `$ungetc` do their real work as a
  statement-level effect that only the process executor performs; a frame body ran the
  same expression through the pure evaluator, which returns X and touches nothing. So
  `rc = $fgets(line, fd);` inside a `function automatic` yielded `rc=0` and an empty
  string where iverilog reads the line — exactly the walker shape the fix above just
  made reachable. It is now a runtime fatal. (An elaborate-time gate was tried and
  measured wrong: a `task automatic` is lowered both framed and inline, and the inline
  copy — the one its callers run — reads the file correctly.)


- **A callee that advances time no longer ends the caller's definite-assignment
  scan.** The deep (callee-body) reference walker had no arm for `@(posedge clk)`,
  `#1`, `wait`, or `wait fork`, and a `_ => false` in a walker keyed on a name means
  "may reference ANY name". One timing control anywhere in a task therefore made every
  block-local declared after a call to it unusable — which is the standard clocked
  driver (`preload()`, `run_scenario()`), and was 11 of the 12 diagnostics in the
  round-18 report. The same mistake had been fixed in the shallow twin a round earlier.
- **SILENT-WRONG: a shared flattened net plus a call that suspends.** Two same-named
  block-locals share one net; one block writes it, calls a one-line helper that does
  `@(posedge clk)`, and reads back — observing the sibling block's write. vita printed
  `A v=99` where iverilog prints `A v=1`, at exit 0, identically at `c8ad2b4` and
  `46b9816`. The shared-net rule read only *syntactic* timing, so a wrapper hid the
  suspend; and the top-level walk returned early once the local was assigned, so the
  rule was never consulted at all. Both are closed.
- **A struct member write that covers the whole variable is a whole write.**
  `rm.c = 5;` on a single-member struct left the walk believing `rm` was unwritten.
  Members are constant part-selects after the parser's desugar, so the rule is bit
  coverage — which also accepts field-by-field writes and hand-written
  `x[31:16] = a; x[15:0] = b;`. Partial coverage stays loud.
- **`automatic` is no longer silently dropped from an unpacked-struct block-local.**
  `automatic rec_t r;` could not be resolved by the automatic-lifetime parse helper
  (an unpacked struct is not in the typedef table), so the member fan-out parsed it
  with no lifetime at all and the declaration became static. Two same-named struct
  locals in disjoint blocks then shared one net, while the identical `automatic int`,
  enum, and typedef-alias forms each got their own scope.
- **The first block declaring a shared name is now asked the same question as the
  rest.** The coalesce guard keys on the net already existing, so it only ever saw the
  second and later declarations — an ordering artefact, not a semantic distinction.
  The decision is now a pure AST pre-pass.
- **A call actual naming an unpacked-struct record is seen as touching its member
  nets.** The SoA fan-out renames the variable but not the call arguments, so a write
  through a formal was invisible (a false-loud) and an `inout` copy-in read was too.

### Added

- **`-v` now prints what actually ran.** Driven from a Makefile or a wrapper
  script, the arguments you read and the arguments the process received are
  different texts: the shell has already substituted `$(WIDTH)`, the filelist
  expander has spliced `-f` frames away, and `VITA_THREADS` never appears in the
  command line at all. `-v` prints the resolved form as a block at the top of the
  transcript — the invocation (shell-quoted, paste-able), cwd, every filelist
  opened, the sources actually compiled, incdirs, defines, runtime plusargs,
  output/log paths, elaboration roots and libraries, timeout, and the thread count
  *with its provenance* (`--threads`, `VITA_THREADS`, or `auto`). Empty rows are
  omitted, long lists wrap at the value column, and a flag never wraps away from
  its value. Because it goes through the normal progress stream, `-l/--log`
  captures it in the same file and the same order as the diagnostics it explains.
  Every applet echoes its own stage. Pure reporting: nothing about the run changes
  and nothing is hashed into an artifact.

### Fixed

- **A flag's value inside a filelist is no longer treated as a source path.** The
  expander's list of value-taking flags had not been updated since the original
  five, so every flag added later had its value rewritten against the frame's base
  directory inside a `-F` filelist: `--top top` became `--top /abs/ip/top` and the
  run died with "top module not found", while `--hier-tree h.txt` silently wrote
  `ip/h.txt` instead of the path the caller named. Affected `--top`, `-L`,
  `--work`, `--workdir`, `--upstream`, `--obs-dir`, `--hier-tree`, `--inst-paths`,
  `--probe` and `--probe-file`. Source positionals still resolve, which is the
  whole point of `-F`.

- **A chained method call no longer ends the definite-assignment scan.** The
  expression walker had no arm for `s.substr(a, b).atoi()` (IEEE §8.13 chaining),
  so it answered "may reference" for *every* name — one chain anywhere in a block
  rejected the chain's own assignment target and every local declared after it. It
  also blamed the wrong file: a chain inside a callee body was reported against a
  local in the caller.
- **The definite-assignment walk no longer forgets that a local is already
  written.** Its catch-all rejected any unmodelled statement outright, even on a
  path where the local was definitely assigned. Once written this execution the
  per-entry reset it would have observed is already overwritten, and nothing
  un-assigns it. This is why a dummy write *before* a loop cleared the diagnostic
  while the same write *inside* the loop did not.
- **A timing-controlled first write is supported.** `#1 x = 7;`,
  `@(posedge clk) x = 7;`, `x = #1 7;`, `#1 begin x = 7; end` and
  `wait (c) x = 7;` are blocking writes — the process does not continue until the
  write has happened. A timing prefix that reads the local is still a genuine
  read-before-write and stays loud, and on a *shared* flattened net any
  time-advancing statement stays loud (the scheduler can hand the one net to the
  other block in between).
- **A local that is never written is accepted.** `automatic byte exp[];` passed
  to an `input` formal is not a read-before-write — there is no first write for the
  read to be before. With no writer the flattened static net holds the type default
  at every entry, which is exactly what `automatic` supplies. Proving "no writer"
  consults the callee resolver, so a task that pokes the flattened net through a
  hierarchical self-path still counts as one.
- **SILENT-WRONG: a method call was not seen as a read of its receiver.** The
  shared reference walker checked a call's arguments but not its path head, so
  `s.atoi()` did not count as reading `s`. The block-local scope-leak detector is
  built on that walker: a block-local referenced outside its block only through a
  method call went undetected, coalesced onto the outer binding's net, and the
  outside read returned the block's value (vita printed `1234` where iverilog
  prints `9999`).
- **SILENT-WRONG: `atoi`/`atohex`/`atooct`/`atobin` parsed like `strtol`.** They
  skipped leading whitespace, honored a sign, and stopped at an underscore. IEEE
  1800 §6.16.9 says the conversion "scans all leading digits and underscore
  characters (`_`) and stops as soon as it encounters any other character" — so
  `" 3".atoi()` is 0 (not 3), `"-7".atoi()` is 0 (not -7) and `"1_0".atoi()` is 10
  (not 1). Verified byte-identical to iverilog 13 over 17 inputs. The old code
  carried a comment asserting iverilog's stricter reading was "its bug".
- **SILENT-WRONG: a hierarchical reference could reach an `automatic` block-local.**
  IEEE 1800 §23.9 forbids naming an automatic variable hierarchically — it has no
  static address — but v1's flatten publishes one as a module net, so `tb.a = 99`
  from another module silently wrote per-entry storage. Now rejected on read,
  write and select-write, as iverilog rejects it.

### Added

- **`note:` telling you where the definite-assignment walk stopped.** E3009's
  lifetime message names two possible causes; the third — "the analyzer stopped
  here" — was the real one for most sites and was invisible, because only the
  declaration's location was printed. The note carries the construct's own
  file:line:col and one of six reasons.
- The block-local scope-leak diagnostic now has a location at all: the error
  points at the declaration, a note at the reference that makes it illegal. The
  deferred hierarchical read/write/select-write passes likewise carry the
  originating statement's span, so every diagnostic they raise is locatable.

## [2026-07-29 · round-16] 

### Fixed
- **Definite-assignment for `automatic` block-locals now understands control
  flow.** A `break`/`continue` placed before the local's first write no longer
  reads as a live path arriving at a later read unwritten — every path that
  reaches that read has already executed the write. `if`/`else` and `case` arms
  that jump drop out of the merge the standard way. A real `disable` is
  deliberately *not* treated as a jump: it may name a block that is not an
  ancestor, so the statement runs on.
- **A statement-position call no longer ends the definite-assignment scan.**
  Calls are now proven harmless instead of assumed harmful: the callee's body is
  walked (transitively, on a depth budget) for any reference to the name,
  including hierarchical self-paths such as `t.a`. A callee that can reach the
  name still makes the call a read.
- **Fixed-size unpacked arrays declared `automatic` in a block.** A `'{…}`
  declaration initializer is supported (it re-runs on each block entry, matching
  the dynamic-dimension form), and an array filled element-by-element with
  literal indices is accepted once every declared index has been written.
- **The same name reused at two nesting levels of sibling block trees.** The
  net-creation and body-lowering phases nested their per-block scopes
  differently, so with both levels scoped the nets were created under one path
  and resolved under another. They agree at any depth now.
- **A comma declaration is treated as independent declarators.** Previously a
  collision on the first name rejected all of them — with a message naming a net
  that did not exist — and dropped the whole declaration, so every later use
  reported "undeclared net/variable" one line below its own declaration.
- **`q.pop_front();` / `void'(q.pop_front());` on a queue of structs** whose
  member width comes from a named parameter. Such a queue is stored per field,
  and the per-field fan-out had no discarding-pop case.
- **`return f(arr);` where `f` takes a dynamic-array formal**, and the same call
  buried in a concatenation.

### Changed
- Diagnostics: the hierarchical-task-call rejection now carries
  `file:line:col`; the dynamic-array-formal message no longer states a
  "module-process level" restriction that no longer exists, and names the actual
  reason instead; the same-name block-local message no longer infers the other
  declaration's lifetime.

## [2026-07-17]

### Changed
- `format_version` 21 → 22: IEEE **two-stage `#delay` conversion** — in
  mixed-precision designs a delay is now rounded to the declaring module's own
  `` `timescale `` precision first, then converted to the global simulation
  precision (iverilog-verified). The artifact timescale trailers are extended
  accordingly. (The full per-version history lives in the comments of
  `crates/vita-artifact/src/header.rs`.)

### Fixed
- `**` (power): exponent no longer truncated at narrow result widths.
- Conversion of >64-bit **signed** values to `real`.
- Overflow panic on package-enum label ranges.
- u32 overflow panic in indexed part-selects.
- `W-FLIST-OVERRIDE` now flows through the diagnostic gate, so `-Werror=` and
  `-Wno-` apply to it.

### Added
- FST output: case-normalization hardening of the `.fst` extension dispatch.
- The dev-only `separate-bins` Cargo feature now really builds standalone
  `vcmp`/`velab`/`vrun` binaries (thin shims sharing the multicall path).

## [Phase-1 MVP]

vitamin's first milestone: a working, deterministic, 3-OS-reproducible RTL
simulator for the Verilog-2005 synthesizable subset plus a synthesizable
SystemVerilog subset. The full `preprocess → lex → parse → elaborate → sim-ir →
sim-engine → VCD` pipeline drives both the one-shot `vita` flow and the staged
`vcmp → velab → vrun` flow.

### Language & front-end
- Verilog-2005 synthesizable RTL: modules, ANSI/non-ANSI ports, parameters/
  localparam, `generate`/`genvar`, functions/tasks, hierarchy & instances.
- SystemVerilog subset: `logic`, `always_ff`/`always_comb`/`always_latch`, and the
  user-defined types **enum**, **typedef**, and **packed struct**.
- Multi-dimensional **packed** and **unpacked** arrays.
- Full operator precedence; `casez`/`casex`; `fork`/`join`/`join_any`/`join_none`;
  `#delay`, `@event`, `wait`.
- `` `timescale `` (doc-08 model): unit/precision scaling of `#delay`, `$time`,
  `$realtime`.

### Engine & output
- Event-driven IEEE-1364 scheduler (Active / Inactive / NBA regions), 4-state values.
- Word-parallel (u64) 4-state bitwise & reduction evaluation.
- Hierarchical VCD output with real signal names and module scopes.
- `$display`/`$write`/`$monitor`/`$strobe`; `$dumpfile`/`$dumpvars`/`$dumpon`/
  `$dumpoff`/`$dumpall`; `$finish`/`$stop`; `$time`/`$realtime`; real support
  (`$rtoi`/`$itor`/`$realtobits`/`$bitstoreal`).

### Determinism & artifacts
- SchemaHash-frozen `sim-ir` golden root → 3-OS byte-identical output.
- Staged artifacts (`.vu` / `.velab`) with `format_version` staleness gating.
- Stable diagnostic codes (`VITA-Exxxx` / `VITA-Wxxxx`), doc-15 bijection.

### Verification
- 419 workspace tests; differential harness against Icarus Verilog (`iverilog` +
  `vvp`) for representative designs (skips gracefully when not installed).

### Internal artifact-format history
- `format_version` 1 → 2: staged artifact re-rooted at `SimIr` (M3 IR-backbone freeze).
- `format_version` 2 → 3: `real` (f64) type evolution.

### Known limitations
See [docs/manual/006_limitations.md](docs/manual/006_limitations.md). In brief:
arithmetic lane is 128-bit unsigned / 64-bit signed (wider poisons to X, fail-safe);
`casez`/`casex` treat scrutinee-x as don't-care; `$dumpvars(depth,scope)` args are
ignored (full dump); VCD memory dump is word-0 only; `%t` lacks default field width.

### Platforms
Linux and macOS (CI: Ubuntu, macOS, RHEL9). Windows is not currently supported.
