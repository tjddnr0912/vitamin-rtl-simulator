# 19 — AI-agent observability (the OBS rail)

This is the specification for goal G2: a simulator an LLM harness can read and drive without a
round trip through a human. It defines the machine-readable rail vita writes beside a simulation —
every emitted file, its complete schema, the flag that gates it, and the guarantees a consumer may
rely on. G1 (correct-or-loud accuracy) is the companion goal; the rail inherits its rules, because a
wrong log misleads a machine exactly as a wrong value does.

The order in which open items are taken is [ROADMAP §6](../ROADMAP.md). The requirement identifiers
used below (R-L0 … R-I2) trace to the requirements document in
[docs/history/reviews](../history/reviews/2026-07-02-ai-sim-observability.md).

## 1. Contract

Five properties. Every rule further down is a consequence of one of them.

| # | Property | What it forbids |
|---|---|---|
| 1 | **Rail separation.** VCD/FST is the human waveform; the OBS rail is a separate set of JSON and JSONL artifacts for a program to read. | Making a consumer parse a waveform to answer "what happened". |
| 2 | **Determinism.** The same input with the same options produces byte-identical rail output, event order included. Wall clock is isolated to named fields listed in §11. | Any map iterated in hash order, any sort without a total tiebreak, any time-derived sort key. |
| 3 | **Single source.** Every observed value is derived from the engine value the simulation used. | A second computation path that can disagree with the run. |
| 4 | **Loud, never approximate.** An unresolved probe path, an unsupported net kind or an unsupported request is a diagnostic and a non-zero exit. | Silently skipping a request, or emitting an approximation that reads as a measurement. |
| 5 | **Token economy.** Transitions and events only; PASS is one line; FAIL is where the detail goes. | A per-cycle dump, a magic number with no schema, a run whose ledger costs more to read than the log. |

The anti-pattern contract is binding on consumers as well: a waveform is not an LLM input, a final
digest diff is not a verdict, every record carries its envelope version and its time, and tool
identity and version are always present so a transcript can be attributed.

Correctness rules that apply to this rail as they apply to the engine (see
[ENGINEERING_RULES](../ENGINEERING_RULES.md)): a wrong log ranks with a silent-wrong; the standard
teeth for an OBS change are a three-way internal differential (JSONL ≡ VCD ≡ `$display`) plus the
determinism golden; and anything that *reports* is additionally tested by an asymmetric upstream
mutation — change something the numbers must follow and confirm they move, because a repeat run of
the same input cannot see a reporting defect.

Nothing on the rail changes the simulation, stdout, the waveform or the exit code. A rail write
failure prints `error[VITA-E0001]` on stderr and leaves the exit code as the simulation set it.

Constants:

| | Value |
|---|---|
| OBS `schema_ver` | `1` |
| `tool` | `"vita"` |
| `version` | `"0.2.0"` (the workspace version) |
| `format_version` | `31` (the frozen artifact format this build emits) |
| Exit codes | `0` OK · `1` RTL or user error · `2` stale artifact · `3` CLI misuse |
| Every OBS flag diagnostic | `VITA-E0001` (`E-CLI-BAD-FLAG`) |
| `$vita_stage` elaborate errors | `VITA-E3009` (`E-ELAB-UNSUPPORTED`) |

The rail is out-of-band with respect to the frozen IR: obs settings travel on `SimOpts` and CLI
sidecars, the `SimIr` shape is unchanged, and `format_version` does not move when the rail changes.
The rail versions itself with `schema_ver`.

## 2. Requirement inventory and status

| REQ | Requirement | Served by | Status at HEAD |
|---|---|---|---|
| R-L0 | Run manifest | `run.json` (§5) | Implemented; `run_id` and `--seed` absent (§13) |
| R-L1 | Test-case ledger, PASS terse | `results.jsonl` (§6) | Implemented as one line per run; the per-test-case form is not implemented |
| R-L2 | Failure detail (`fail/*.json`, divergence value first) | — | Not implemented |
| R-L3 | State trace, transitions only | `trace.jsonl` (§8) | Implemented for scalar/vector/packed nets; `stuck_in` hang detection is not implemented |
| R-L4 | Handshake and protocol channel events | — | Not implemented |
| R-L5 | Coverage summary | `coverage.json` (§7) | Implemented for functional covergroups only |
| R-L6 | SVA log (property name plus implicated signal values) | — | Not implemented |
| R-S0 | Design-structure export (hierarchy tree, instance paths) | `--hier-tree` / `--inst-paths` (§10) | Implemented, one-shot only |
| R-S3 | Stage-trace schema shared with an emulator, plus a golden stage hook | `$vita_stage` → `stage.jsonl` (§9) | Implemented |
| R-C1 | Program control API (`peek`/`poke`/`step`/`run_until`/`finish`) | — | Not implemented |
| R-C2 | Deterministic checkpoint, replay, time travel | — | Not implemented |
| R-C3 | Delta-cycle and region-ordered events | — | Not implemented; no record carries a region or delta field |
| R-C4 | X-propagation origin (`uninit` / `multi-drv` / `arith-X`) | — | Not implemented |
| R-C5 | Dataflow backward slice | — | Not implemented |
| R-I1 | Config-driven signal introspection (auto-named dump with no hand-written `bind`) | `--probe` / `--probe-file` | Partial: a manual path list, not a config |
| R-I2 | Semantic transaction log | — | Not implemented |
| R-F1 | Format contract: JSONL, `schema_ver`, stable hierarchical path strings, explicit value encoding, byte-identical repeat runs | §1 property 2, §11 | Implemented, with the value encoding of §3 pin 4 |
| R-A | Anti-pattern contract | §1 | Adopted in full |

R-F1 costs vita nothing that a legacy simulator would have to retrofit: determinism is already a
core invariant (byte-identical output across the supported platforms, `BTreeMap`-only iteration,
seeded RNG, the SchemaHash staleness gate). The rail is a serializer on top of that invariant, not
a new guarantee.

## 3. Design pins

1. **Two rails.** VCD/FST for people, semantic JSONL for programs. Neither is derived from the other.
2. **Determinism is a gate, not an aspiration.** Byte-identical repeat output is a golden test, and
   it is the prerequisite for checkpointing, bisecting and stage diffing.
3. **The log obeys correct-or-loud.** One source (the engine), no second computation, loud on
   anything unresolved, and a three-way internal differential as the standard teeth.
4. **Record envelope.** Every JSONL record is a self-contained one-line JSON object whose first
   keys are `{"v":1,"t":<u64>,"kind":"…"}`, followed by a per-`kind` payload. Key order is fixed by
   the serializer, which is what makes byte-identity possible. Value encoding: `trace.jsonl`'s
   `old`/`new` are **full-width, unprefixed, MSB-first 4-state binary strings** (characters
   `0` `1` `x` `z`), so a value is bit-precise with no width ambiguity; `stage.jsonl`'s `vals[]` are
   **`%0d` decimal strings**, deliberately mirroring a parallel `$display("%0d", …)` so the two can
   be byte-compared as the three-way teeth. Paths are the full hierarchical string the VCD `$scope`
   rules produce (`top.u0.state_q`).
5. **Logs before capabilities.** The observation rail is small, immediately useful and low-risk; the
   control and time-travel surfaces are large. §13 states which of them exist.

Non-goals, outside the rail by decision: FSDB and UCDB, an embedded SQLite (one external loader
script is enough), a waveform GUI, UVM integration, and automatic inference of protocol channels —
R-I2 would be config-described, never guessed. VCD stays the human-facing format.

## 4. Surfaces

| Surface | Kind | Emits | Gate |
|---|---|---|---|
| `--obs-dir <DIR>` | flag with value | `run.json`, `results.jsonl`, and conditionally `coverage.json`, `trace.jsonl`, `stage.jsonl` | one-shot `vita` only |
| `--obs-procs` | boolean flag | populates `run.json`'s `processes`, `builtins`, `subroutine_calls` (counts only) | requires `--obs-dir` |
| `--obs-procs-time` | boolean flag | the same three objects plus `time_s` / `timed_calls` / `obs_overhead_est_s` | requires `--obs-dir`; implies `--obs-procs` |
| `--probe <NET>` | flag with value, repeatable | `trace.jsonl` | requires `--obs-dir`; one-shot only |
| `--probe-file <FILE>` | flag with value | merged into the `--probe` set | requires `--obs-dir`; one-shot only |
| `$vita_stage("label"[, vals…])` | vendor system task in RTL | `stage.jsonl` | requires the `+STAGE_TRACE` plusarg and `--obs-dir`; one-shot only |
| `--hier-tree <FILE>` | flag with value | a plain-text instance tree | one-shot `vita` only |
| `--inst-paths <FILE>` | flag with value | a plain-text instance-path list | one-shot `vita` only |

`--obs-dir` creates its directory, nested components included. All four applets (`vita`, `vcmp`,
`velab`, `vrun`) share one argv parser, so every spelling above is recognised everywhere and is
answered on a staged applet by the refusals in §4.2. The filelist expander knows which of them take
a value, so a filelist cannot swallow an argument. `vita --help` lists all of them under
`Observability (machine-readable run facts -- doc-19):`, and a test enumerates the literal match
arms in the parser to assert that it does.

### 4.1 Flag grammar

| Spelling | Value | Empty value | Repeat |
|---|---|---|---|
| `--obs-dir` | required | `'--obs-dir' needs a non-empty directory` | last wins, recorded as an override for the `-v` echo |
| `--obs-procs` | none | — | idempotent |
| `--obs-procs-time` | none | — | idempotent |
| `--hier-tree` | required, non-empty | `'--hier-tree' needs a non-empty path` | last wins |
| `--inst-paths` | required, non-empty | `'--inst-paths' needs a non-empty path` | last wins |
| `--probe` | required, non-empty | `'--probe' needs a non-empty net path` | accumulates |
| `--probe-file` | required | accepted, then fails at open | last wins |

`--obs-procs` and `--obs-procs-time` fold into one profile configuration: either flag turns the
profile on, and `--obs-procs-time` sets its `timed` bit. `--obs-procs-time` alone is therefore
sufficient and yields a timed profile.

### 4.2 Loud rejections

| Condition | Message | Exit |
|---|---|---|
| `--obs-dir` on `vcmp`/`velab`/`vrun` | ``'--obs-dir {dir}' is a one-shot `vita` argument — '{stage}' does not emit the obs rail (a staged-run manifest is a follow-on)`` | 3 |
| `--probe`/`--probe-file` on a staged applet | ``'--probe'/'--probe-file' is a one-shot `vita` argument — '{stage}' does not emit the trace rail (staged probing is a follow-on)`` | 3 |
| `--obs-procs`/`--obs-procs-time` on a staged applet | ``'--obs-procs'/'--obs-procs-time' is a one-shot `vita` argument — '{stage}' does not emit run.json (a staged-run manifest is a follow-on)`` | 3 |
| `--obs-procs` without `--obs-dir` | ``'--obs-procs' requires '--obs-dir <D>' (the profile is published as run.json's `processes` object)`` | 3 |
| `--probe` without `--obs-dir` | `'--probe' requires '--obs-dir <D>' (trace.jsonl is written there)` | 3 |
| `--probe-file` unreadable | `cannot read --probe-file '{f}': {e}` | 3 |
| A `--probe` path that is not a net | ``--probe path '{p}' does not resolve to a net (check the hierarchical name; `--dump-filelist`-style net listing is a follow-on)`` | 3 |
| A `--probe` path of an unsupported kind | ``--probe path '{p}' is {r} — v1 can trace only a scalar/vector/packed net (real / per-element array / handle probing is a follow-on)``, where `{r}` is `a dynamic-array/queue/string handle`, `an unpacked array` or `a real/realtime net` | 3 |
| `$vita_stage` in a design fed to staged `velab` | ``` `$vita_stage` is a one-shot `vita` task — `velab` does not stage it (run one-shot: `vita <design> --obs-dir <D> +STAGE_TRACE`) ``` | 3 |
| `$vita_stage()` with no arguments | ``$vita_stage requires at least a label argument (`$vita_stage("label"[, values…])`)`` | 1 |
| `$vita_stage` as a deferred-assertion action | `$vita_stage as a deferred-assertion action is unsupported — call it as a plain statement` | 1 |

Order of operations on the one-shot path: preprocess → lex → parse → elaborate → write
`--hier-tree` → write `--inst-paths` → resolve probes → check `--obs-procs` has a directory →
simulate → finalise the exit code → write the obs directory. Two consequences a consumer must
expect: a rejected `--probe` still leaves the hierarchy files behind and writes no obs directory at
all, and a front-end or elaborate failure writes no obs directory either. Absence of `run.json` is
therefore "the design never ran", not "the run said nothing".

### 4.3 The `-v` echo

The effective-invocation block printed under `-v` carries:

```
obs-dir:    <dir>                       # only when set
obs-procs:  counts (--obs-procs)        # or: counts+time (--obs-procs-time)
probes:     top.y1 top.clk              # always present, empty when no --probe was given
```

The `probes:` row lists `--probe` values only, not the paths merged in from `--probe-file`.
`--hier-tree` and `--inst-paths` have no echo row.

## 5. `run.json`

Written on every one-shot run that reaches simulation with `--obs-dir` set — PASS or FAIL, `$fatal`
included. The JSON is hand-rolled with a fixed key order and one top-level field per line, so a
`diff` on two runs points at a field rather than at a reflowed line. String escaping covers `"`,
`\`, `\n`, `\r`, `\t` and any scalar below `0x20` as `\u00XX`; UTF-8 passes through.

### 5.1 Top-level keys, in emission order

| # | Key | Type | Meaning |
|---|---|---|---|
| 1 | `schema_ver` | int | `1`. Bumped only when the record envelope changes, never for an additive field |
| 2 | `tool` | string | `"vita"` |
| 3 | `version` | string | the crate version |
| 4 | `format_version` | int | the frozen artifact format this build emits |
| 5 | `seed` | null | always `null`; there is no `--seed` flag (§13) |
| 6 | `plusargs` | array of string | runtime plusargs in command-line order, leading `+` stripped (`+STAGE_TRACE` → `"STAGE_TRACE"`) |
| 7 | `source` | object `{name, blake3}` | `name` is the basename of the first source file only, so the same design run from two directories byte-diffs clean; `blake3` is the digest of the concatenated source text of every command-line file (a `\n` is appended to any file not ending in one) |
| 8 | `finish_reason` | string | `"finish"` · `"stop"` · `"quiescent"` · `"delta_limit"` · `"error"`. Descriptive — how the run stopped, never the verdict. `--timeout` produces `"quiescent"` |
| 9 | `exit_class` | string | `"ok"` · `"had_errors"` · `"fatal"`, derived from the final exit code and the fatal count rather than from the engine result, so a `-Werror`-promoted warning cannot report `"ok"` beside `exit_code: 1` |
| 10 | `exit_code` | int | the process exit code actually returned |
| 11 | `sim_time` | int | final simulation time in raw ticks of the global precision |
| 12 | `counts` | object `{errors, warnings, fatals}` | from the diagnostic sink. `errors` **excludes** fatals; the stderr epilogue's `errors=` token is errors plus fatals, so the two differ exactly when `fatals > 0` |
| 13 | `status` | string | `"PASS"` when `exit_code == 0`, else `"FAIL"` |
| 14 | `backend` | string | the executor that actually ran process bodies: `"native"` · `"vm"` · `"interp"` |
| 15 | `backend_requested` | string | what `--backend` asked for; the default request is `native` |
| 16 | `codegen` | object | the bytecode-VM static capability census (§5.2) |
| 17 | `native` | object | the native-backend eligibility verdict (§5.3) |
| 18 | `subroutines` | object | the static frame/inline route census (§5.6). Unconditional |
| 19 | `processes` | object or `null` | per-body activation profile (§5.4) |
| 20 | `builtins` | object or `null` | per-builtin call profile (§5.5) |
| 21 | `subroutine_calls` | object or `null` | runtime per-`FuncId` subroutine profile (§5.7) |
| 22 | `utc_unix_s` | int | wall-clock epoch seconds. Isolated non-deterministic field |
| 23 | `wall_s` | number | total wall seconds. Isolated |
| 24 | `elab_s` | number | wall seconds before simulation (preprocess, lex, parse, elaborate). Isolated |
| 25 | `sim_s` | number | wall seconds inside simulation — the only part `--backend` can move. Isolated |

Every seconds field, and every `time_s` anywhere in the file, is formatted to six decimals; a
non-finite value renders as `0.0`.

`backend` versus `backend_requested` is the only fallback signal in the file, and it is why both
fields exist. `native` is the default executor and an ordinary run reports `"backend": "native"`
with `"native": {"eligible": true, …, "refused": null}` — a consumer must not read the presence of
these fields as evidence that vita interprets. When the effective backend differs from the requested
one the engine also emits the warning `W-RUN-BACKEND-FALLBACK` (`VITA-W4030`).

### 5.2 `codegen`

```json
"codegen": {"able": 3, "total": 5, "frame_bodies": 2, "reject_reasons": {"delay": 1, "wait": 1}}
```

| Key | Meaning |
|---|---|
| `able` | process templates the VM's own compile gate accepts |
| `total` | the number of process templates in the IR |
| `frame_bodies` | the number of function and task bodies. None of them is a compile candidate, so a design whose work lives in subroutines can report `able == total` and still run none of its work on the VM |
| `reject_reasons` | cause → number of process templates exhibiting it. A template with two causes counts under both, so the column sum may exceed the number of rejected templates |

The closed key vocabulary: `class_new` · `delay` · `disable` · `force_release` · `fork` ·
`frame_call` · `nba_transport_delay` · `sformatf` · `stmt_effect_rhs` · `wait`. `sformatf` means
"the body reaches a `$sformatf`-shaped IR node", not "the source spells `$sformatf`" — elaboration
desugars string concatenation onto that node. The census comes from the same walk the compile gate
runs, which is property 3 of §1 applied to a capability claim; the map is a `BTreeMap`, so key
order is stable.

### 5.3 `native`

```json
"native": {"eligible": true, "buildable": true, "refused": null, "reject_reasons": {}}
```

| Key | Meaning |
|---|---|
| `eligible` | the scope gate accepted the design; true exactly when `reject_reasons` is empty |
| `buildable` | the storage gate accepted it (the net arena can hold this design) |
| `refused` | the reason of the first gate that refused — design, then storage, then executor — or `null` when nothing refuses |
| `reject_reasons` | reject family → count of offending items. Any non-zero row disqualifies |

The two gates are separate fields because folding them into one flag makes a scope limit read as an
implementation capability: a subroutine-heavy design can be `eligible` and not `buildable`.

`refused` carries **two vocabularies**. When the design gate refused, it is a key of
`reject_reasons` — the byte-lexicographically first when several fired, which is deterministic but
is one reason out of several. When the design gate passed, it is the prose of the next gate that
refused — storage first, then the executor gate — and that prose appears in no map. A consumer
joining `refused` back to `reject_reasons` must read a miss as the storage or executor case, not as
an error. The design gate has exactly one family key, `"stmt_effect"`. The storage refusals, from
`NetArena::buildable` and the frame admission it calls, are:

```
a call in a delayed continuous assign: S3b
a module body that names a frame-local net
a nested call to an unresolved target: S3b
a nested call with no sidecar entry: S3b
a nonblocking assign to a frame-local net: S3b
a subroutine body that suspends, forks or calls a task
a subroutine statement the frame executor drops
a subroutine that WRITES a net outside its own frame: S3b
a system task the tier-3 kernel refuses, inside a task frame
arena exceeds u32 words
arena exceeds usize
malformed frame sidecar (block id out of range)
malformed frame sidecar (frame window out of range)
malformed frame sidecar (func_table length)
malformed frame sidecar (return slot out of range)
```

The executor gate adds two of its own, published in the same field when the design and storage
gates both pass:

```
a `wait fork`, a `fork`, or a call statement whose callee forks: S3b
a system task the tier-3 kernel refuses (VCD, $monitor/$strobe, file)
```

Neither `--probe` nor `$vita_stage` is a reject axis: both are native-backend core, an instrumented
run reports `"backend": "native"`, and instrumentation therefore never changes which executor the
measurement describes. The gate itself is specified in
[21-tier3-native-backend.md](21-tier3-native-backend.md).

### 5.4 `processes` — the per-body activation profile

`--obs-procs` (counts) or `--obs-procs-time` (counts and cumulative wall clock). `null` when neither
flag was given, and `null` is deliberately distinct from an empty object: "not measured" is a
different statement from "measured, nothing ran".

Everything else in `run.json` about execution is static — `codegen` says which bodies the VM *could*
compile, `native` says whether the design is accepted. `processes` is the dynamic half: which body
actually ran, and how often. It is the half a user can act on without changing the simulator.

```json
"processes": {"timed": false, "counts": {"processes": 5, "assigns": 1, "total_evals": 57},
  "items": [
    {"domain": "process", "index": 1, "kind": "always", "scope": "tb",
     "file": "t1.sv", "line": 9, "col": 3, "evals": 21},
    {"domain": "assign", "index": 0, "kind": "assign", "scope": "tb",
     "file": "t1.sv", "line": 7, "col": 3, "evals": 12}
  ]}
```

| Field | Meaning |
|---|---|
| `timed` | whether `--obs-procs-time` was given. When false, rows carry no `time_s` at all — a `0.0` would read as "this body is free", which is a different claim from "nobody asked" |
| `counts.processes` / `.assigns` | the size of each IR domain. Rows are emitted for all of them, zero-eval ones included: "this `always_comb` never fired" is a finding too |
| `counts.total_evals` | the sum over every row, for normalising a share |
| `domain` | `"process"` or `"assign"`, stated explicitly rather than inferred from `kind`, whose vocabulary can grow |
| `index` | the index into that domain's IR vector; stable for a given design and run options |
| `kind` | the source construct (vocabulary below) |
| `scope` | the instance path the body was elaborated under, the same string `%m` renders. A module instantiated N times contributes N rows with one `file:line:col` and N different `scope` values, which is exactly what "which of them eats the cost" asks |
| `file` | the source path **as given on the command line** — the string diagnostics print, not the basename `source.name` carries. A multi-file design needs the directory to be actionable |
| `line` / `col` | 1-based position of the construct, or `0` when unknown |
| `evals` | activations (see below) |
| `time_s` | emitted only when `timed`: cumulative seconds inside that body, six decimals |

`kind` vocabulary:

| Group | Values |
|---|---|
| User-written processes | `initial` · `always` · `always_ff` · `always_comb` · `always_latch` · `final` |
| Processes vita synthesizes | `sva` · `covergroup` · `clocking` · `var_init` (the declaration-initializer flush) |
| Continuous assigns | `assign` · `net_init` (a `wire a = expr;` declaration initializer) · `port` (a synthesized port hookup) |
| Fail-safe | `synth` — a synthesized body with no more specific label. No producer reaches it; seeing it in a `run.json` means a new producer appeared unlabelled |

The synthesized labels exist so a reader is not sent hunting for an `always` that is not at the
cited line.

An **evaluation** is one activation of the body by the scheduler. A process that suspends on `#5`
and resumes counts twice — the two halves are two dispatches. A continuous assign counts one
settle-fixpoint visit that actually evaluated its right-hand side; a visit the dirty worklist skips
costs nothing and counts nothing. A fork child is charged to the process template it belongs to.

`kind: "port"` rows locate the **port connection in the parent's instantiation**: the whole
`.p(expr)` (or the `.p` shorthand) starting at the `.`, or the connection expression for a
positional list, so `col` is what distinguishes connections written on one line. A design with a
39-connection instance can put half its evaluations in `port` rows that `scope` alone cannot tell
apart. An unpacked-array port lowers to one row per element, all carrying that single connection's
position — the source holds one connection and the split is vita's.

`"", 0, 0` is an honestly unlocated row, never a stand-in for a location that exists. Two producers
reach it: a run with no span resolver installed, and a `.*` wildcard connection, which the
elaborator synthesizes per unnamed port and which has no source text of its own. Explicitly written
connections in the same instantiation still locate. An identity table shorter than its accumulator
falls back to this unlocated row rather than dropping it: a profile that silently omits the body the
user is hunting is worse than one that admits it cannot name it.

**Row order** is `evals` descending, then `domain` ascending, then `index` ascending. The tiebreak
is total, so the order is deterministic even on a timed run — which is why `evals`, never `time_s`,
is the sort key.

**Determinism.** `evals` is a function of design and run options alone and belongs inside the
determinism golden. `time_s` is wall clock and is isolated exactly like `wall_s`/`elab_s`/`sim_s`.
That is the reason timing is a second flag rather than implied: a transcript taken with
`--obs-procs` alone has a byte-reproducible `run.json`.

**Backend invariance.** Both dispatch seams bump the same counters at their own dispatch point and
their own settle fixpoint, counting the same event, so one design profiles identically under
`--backend interp`, `vm` and `native`.

**Observer effect, and it is asymmetric.** `--obs-procs-time` takes two clock reads per activation
(tens of nanoseconds). For a fat `always_ff` that is noise; for a one-bit continuous assign it can
exceed the work it measures, so a timed run's `sim_s` is longer than the same run's untimed `sim_s`
and the per-row shares are biased toward the cheap rows. Read `evals` first and reach for `time_s`
only to break a tie between rows with similar counts.

**Cost when not asked for.** The counters are a runtime test, not a compile-time one. On a real
clocked workload the difference is inside the measurement noise; on a synthetic built to be the
worst case this instrumentation can have — 64 one-line `always @(posedge clk)` blocks plus a
64-deep continuous-assign chain, where the dispatch seam and the settle visit *are* the run — it is
about 1.3%, and that residue is the per-visit test itself rather than the code around it. Removing
it would need the settle pass monomorphised over a compile-time flag, i.e. a second copy of the
simulator's hottest loop, which is not worth 1.3% on a shape no real design has.

### 5.5 `builtins` — the per-builtin call profile

Same flags and same requirements as §5.4. A **sibling** of `processes`, not a member of it: a
`processes` row is a body the author can edit, a `builtins` row is a simulator primitive that body
called. `null` without the flag, on the same "not measured" convention.

It exists because one row per user-written body has a granularity limit. A single `initial` that
drives a vector file through a stack of tasks can be 60% of a run, and every nested cost sums into
it, so the profile says *that* the testbench is expensive and not *which primitive* to attack.

```json
"builtins": {"timed": true, "attribution": "self-plus-arguments",
  "included_in_processes": true,
  "time_semantics": "ranking and UPPER BOUND on removal gain: …",
  "distinct": 8, "total_calls": 3006, "obs_overhead_est_s": 0.001204,
  "items": [
    {"name": "$fgets", "calls": 1001, "time_s": 0.004241},
    {"name": "$sscanf", "calls": 1000, "time_s": 0.000642},
    {"name": ".push_back()", "calls": 1000, "time_s": 0.000100},
    {"name": ".size()", "calls": 1, "time_s": 0.000012}
  ]}
```

| Field | Meaning |
|---|---|
| `timed` | whether `--obs-procs-time` was given; when false, rows carry no `time_s` at all |
| `attribution` | the literal string `"self-plus-arguments"`. A field rather than a documented assumption, so a consumer can refuse a value it does not understand |
| `time_semantics` | the literal sentence `"ranking and UPPER BOUND on removal gain: a row's time_s includes evaluating that call's own arguments (nested builtins are subtracted, ordinary expression work is not), so removing the call recovers at most this, usually much less"`. Emitted in the file so a consumer cannot read a row as a saving without meeting the caveat |
| `included_in_processes` | the literal `true` — this time is already inside the `processes` row of whichever body made the call. The one mistake a reader can make here is adding the two arrays |
| `distinct` | number of rows: builtins this run touched at least once |
| `total_calls` | the sum of `calls` |
| `obs_overhead_est_s` | emitted only when `timed`: a measured estimate of what the timing itself cost this run, calibrated at emit time by timing a batch of the same clock reads an invocation pays and scaling by `total_calls`. It is an estimate and the name says so |
| `items[].name` | the identity. A builtin has no declaration site — it is not declared in the user's source — so the name is the key |
| `items[].calls` | invocations; deterministic |
| `items[].time_s` | emitted only when `timed`: cumulative seconds, six decimals |

**Name vocabulary.** Two spellings by design: `$…` is the IEEE system task or function spelling the
user typed (`$display`, `$fgets`, `$sscanf`, `$finish`, …); `.name()` is a method-form builtin
(`.push_back()`, `.size()`, `.sort()`, `.randomize()`, `.substr()`, …) — printing `$qpushback` for
one of those would send a reader looking for a system task that does not exist. The name table has
no catch-all arm, so a new frozen-enum variant is a build error rather than an `"unknown"` row.
`$cast` is the one deliberate merge: the task form and the function form are one construct to the
author and both render `"$cast"`.

Six constructs share one internal display identifier and are un-folded by the label so their cost
appears on their own row rather than inside `$display`: the severity tasks
(`$info`/`$warning`/`$error`/`$fatal` and the `unique`/`priority` check), `$timeformat`,
`$vita_stage`, the assertion-control family, a whole-handle copy, and a queue slice.

**A row counts IR nodes, not source text** — the same caveat `codegen`'s `sformatf` key carries.
Elaboration desugars some constructs onto a builtin the author never typed: a string concatenation
becomes a `$sformatf` node, a `foreach` over a queue becomes associative-iteration steps, a
`unique case` violation becomes the `$warning` shape. A row whose count exceeds the call sites
visible in the source is that, and it is still the honest answer to "what is this run spending
itself on".

**The arithmetic contract.** `time_s` is the wall clock of one invocation minus the wall clock of
any builtin invoked inside it, so rows are disjoint and their sum is a true simulator-builtin
subtotal. That nesting is real: `$display("%0d", q.size())` evaluates `.size()` during `$display`'s
argument rendering, i.e. inside the outer builtin's own dispatch. What the span does **not**
exclude is the call's own argument evaluation, so `$signed(<big expression>)` charges the big
expression to `$signed`. A row therefore ranks, and bounds the prize from above; it does not
predict a saving. The useful subtraction is the other direction: *this `initial` costs 43.7 s, of
which 18.2 s is `$fgets` plus `$sscanf`, so 25.5 s is my RTL.*

**Row order** is `calls` descending, then `name` ascending — a total order over static string keys,
so the object is byte-identical across two runs including a timed one.

**Backend invariance.** Four seams see a builtin run and all four charge one interior-mutable
profile object, so a `&self` seam can reach it:

| Seam | Covers | Reached by |
|---|---|---|
| the shared builtin dispatch | every system task | interpreter, VM, JIT and native all converge here |
| statement-effect application | statement-effect system functions (`$fgets`, `$fscanf`/`$sscanf`, `$fread`, `$fgetc`, `$feof`, `$ungetc`, `$fopen`, `$sformatf`, `$value$plusargs`, seeded `$random`/`$dist_*`, `$cast`, queue pop, associative iteration, class `new()`) | interpreter and native |
| the shared expression evaluator | pure system functions in expression position (`.len()`, `.substr()`, `.size()`, `.sum()`, `$clog2`, `$countones`, the real-math family) | every executor |
| the frame executor's system-task arms | prints inside a subroutine body run by the synchronous frame executor | that executor only — it does not go through the shared dispatch |

The fourth row is the one worth knowing about: a hook only in the shared dispatch would silently
under-report every print inside a subset function body. Tests pin that a `$display` inside a
`function automatic` counts, and that the whole table is identical under
`--backend native|vm|interp`.

**Observer effect, worse than §5.4's.** Two clock reads per invocation against a `.len()` on a short
string is most of the measurement. Read `calls` first; `time_s` separates rows with comparable call
counts, and the expensive builtins (file I/O, formatting, `$readmem*`, sorting) are exactly the ones
where the overhead is negligible. Counting without timing costs under 1% even on a design that does
nothing but evaluate pure system functions, which is within the noise band a control workload that
touches none of the four seams establishes.

### 5.6 `subroutines` — the static route census

Emitted on every `--obs-dir` run; `--obs-procs` does not move it. It is not a measurement — it is
what elaboration decided — so it is deterministic and sits inside the `run.json` determinism golden
rather than beside it.

It exists because vita lowers a subroutine two ways, and nothing else in the rail says which way a
given routine went. A frame call can cost several times the same expression written inline, and a
reader cannot guess the route from the source.

```json
"subroutines": {"counts": {"total": 6, "frame": 4, "inlined": 2},
  "sites_semantics": "call sites LOWERED (after generate/instance expansion), not executions; 0 = declared and never called",
  "uncounted": "class methods and hierarchical calls",
  "items": [
    {"module": "leaf", "name": "aut", "kind": "function", "route": "frame", "sites": 2},
    {"module": "leaf", "name": "pure8", "kind": "function", "route": "inlined", "sites": 4},
    {"module": "top", "name": "p::dbl", "kind": "function", "route": "frame", "sites": 1}
  ]}
```

| Field | Meaning |
|---|---|
| `counts.total` / `.frame` / `.inlined` | rows, framed rows, and the remainder |
| `sites_semantics` | the literal sentence quoted above, so the meaning of `sites` travels with the data |
| `uncounted` | the literal string `"class methods and hierarchical calls"` — stated in the file rather than assumed |
| `items[].module` | the module whose body was being lowered. A package routine is filed under the module that **calls** it, since a package has no instance of its own. Two modules declaring a same-named function are two rows, which is why the key is a pair |
| `items[].name` | the routine key the elaborator resolved: `f`, or the scoped spelling `pkg::f` |
| `items[].kind` | `"function"` or `"task"` |
| `items[].route` | `"frame"` = a body reached through a call node; `"inlined"` = folded into the caller. Read from the same map the lowering is selected by, never re-derived — a predicate that re-answers "would this be framed?" is free to disagree with the route the design actually took, which is the failure this table exists to make visible |
| `items[].sites` | call sites **lowered**, after generate and instance expansion: a call written once inside a module instantiated twice is `2`; a call inside a `for` loop body is `1`. Never an execution count. `0` means declared and never called |

Rows are seeded for every declared subroutine from the same two sets the frame lowering reserves
from, so a dead subroutine still reports its route; the call seams then increment `sites`, and the
route a call actually took overwrites the seed. Items are sorted by `(module, name)`.

Class methods and hierarchical calls are excluded by construction: class methods are a separate
lowering with its own table, and a hierarchical call's target is not bound until the deferred
resolve, which runs after this seam. The runtime object in §5.7 covers both.

What surprises readers is the point of the table: `function int f` is framed and its
`function logic [31:0] f` twin is inlined, because `int` is 2-state and the frame return slot is
what coerces x/z to 0. No one guesses that from the source.

### 5.7 `subroutine_calls` — the runtime per-`FuncId` profile

Gated by `--obs-procs`; `null` otherwise, on the same convention.

```json
"subroutine_calls": {"timed": true, "key": "…", "time_semantics": "…",
  "distinct": 5, "total_calls": 5,
  "items": [
    {"func": 0, "name": "C.new", "decl_file": "cls.sv", "decl_line": 3, "decl_col": 12,
     "calls": 1, "timed_calls": 1, "time_s": 0.000008},
    {"func": 4, "name": "top.slow", "decl_file": "cls.sv", "decl_line": 10, "decl_col": 18,
     "calls": 1, "timed_calls": 0, "time_s": 0.000000}
  ]}
```

| Field | Meaning |
|---|---|
| `timed` | whether `--obs-procs-time` was given |
| `key` | the literal sentence ``"per-INSTANCE FuncId. INCLUDES class methods and hierarchical calls, which the static `subroutines` object does not file at all, and EXCLUDES every INLINED subroutine, which has no call node to count — so the two objects share no key and their columns must not be added. A row identifies its source by decl_file:decl_line:decl_col; the static object does not carry that yet, so read it by name and route"`` |
| `time_semantics` | the literal sentence ``"time_s is SELF time over timed_calls only (nested subroutine time subtracted); timed_calls < calls means the rest were SUSPENDABLE task frames, which are counted and never timed. total_calls is entries into FRAMED subroutines only — an inlined one contributes none, and `subroutines[].route` is what says which is which"`` |
| `distinct` | number of rows |
| `total_calls` | the sum of `calls` |
| `items[].func` | the `FuncId`, per instance |
| `items[].name` | a **label**, not a key. A module subroutine gets its `%m` path, which is per-instance (`top.u1.aut`, `top.u2.aut`); a class method gets the class-relative `C.m` fragment. Nothing in the row discriminates the two conventions |
| `items[].decl_file` / `decl_line` / `decl_col` | the declaration site, written once however many `FuncId`s it minted; this is what identifies a row's source. The span is the routine name's own, so `decl_col` points at the identifier. `""` / `0` / `0` when no span resolver was installed |
| `items[].calls` | entries into this subroutine from all three runtime seams; deterministic |
| `items[].timed_calls` | emitted only when `timed`: the subset of `calls` that `time_s` covers |
| `items[].time_s` | emitted only when `timed`: self seconds over `timed_calls`, nested subroutine time subtracted |

Three runtime seams feed it, all methods on one shared state object, so the counts are
backend-invariant by construction:

| Seam | Covers | Timed |
|---|---|---|
| frame call evaluation | every call expression — plain and package functions, class methods and constructors, virtual-dispatch targets, hierarchical `u1.f(x)` | yes |
| synchronous task call | every synchronous task call, nested calls included | yes |
| suspendable task frame entry | every suspendable task frame | no — counted only |

The third seam is why `timed_calls` is a separate column: frame entry returns as soon as the frame
is open, and the task may then sit on a `#5` for the rest of the run. Timing that wall span would
report waiting as work.

**Row order** is `calls` descending, then `FuncId` ascending.

The two subroutine objects answer different questions and do not join today:

| Axis | `subroutines` (static) | `subroutine_calls` (runtime) |
|---|---|---|
| Key | `(module, routine key)` | per-instance `FuncId` |
| Flag | none, `--obs-dir` alone | `--obs-procs` |
| Class methods | excluded | included |
| Hierarchical calls | excluded | included |
| Inlined subroutines | included, `route: "inlined"` | absent — no call node exists to count |
| Multiple instances | folded into one row | one row per instance |
| Package routine name | `p::dbl` under `module: "top"` | `top.dbl` — the `%m` path, package qualifier gone |
| Declaration site | not carried | `decl_file:decl_line:decl_col` |
| Counted quantity | `sites` = lowered call sites | `calls` = runtime entries |

The stated join key is the declaration site and the static object does not carry it, so the only
cross-read available is by name and route. The `key` string says exactly that rather than
instructing a join it cannot serve. Closing it is a ROADMAP §6 item.

## 6. `results.jsonl` — the ledger

Written alongside `run.json`. One line per run, terminated by `\n`, no wall-clock field at all, so
the whole file byte-diffs clean.

```json
{"v":1,"t":35,"kind":"result","status":"PASS","finish_reason":"finish","exit_code":0,"sim_time":35,"errors":0,"warnings":1,"fatals":0}
```

| Key | Type | Meaning |
|---|---|---|
| `v` | int | record-envelope version, always `1` |
| `t` | int | the record's time — here the final simulation time, identical to `sim_time` |
| `kind` | string | always `"result"` |
| `status` | string | `"PASS"` or `"FAIL"` |
| `finish_reason` | string | the `run.json` vocabulary |
| `exit_code` | int | the process exit code |
| `sim_time` | int | final simulation time in ticks |
| `errors` / `warnings` / `fatals` | int | the same three counts; `errors` excludes fatals |

vita's model is one run = one test case, so v1 of this file is one line. A per-test-case ledger and
a `detail_ref` pointing at failure detail are the v2 shape and are not implemented (§13).

## 7. `coverage.json` — functional coverage

Written only when the design produced a coverage summary, which requires at least one covergroup
**instance** carrying at least one coverage item. A design with no covergroup simply does not get
the file.

```json
{
  "schema_ver": 1,
  "kind": "coverage",
  "groups": [
    {"instance": "top.c", "coverage_pct": 70.833333, "coverpoints": [
      {"name": "cp_v", "kind": "coverpoint", "num_bins": 4, "covered_bins": 3, "coverage_pct": 75.000000},
      {"name": "cp_w", "kind": "coverpoint", "num_bins": 2, "covered_bins": 2, "coverage_pct": 100.000000},
      {"name": "cross_0", "kind": "cross", "num_bins": 8, "covered_bins": 3, "coverage_pct": 37.500000}]}
  ]
}
```

| Key | Type | Meaning |
|---|---|---|
| `schema_ver` | int | `1` |
| `kind` | string | always `"coverage"` |
| `groups[].instance` | string | the covergroup instance's fully qualified name |
| `groups[].coverage_pct` | number, six decimals | the weighted average, mirroring what `c.get_coverage()` returns inside the design |
| `groups[].coverpoints[]` | array | one entry per coverage item: coverpoints first, then crosses |
| `…[].name` | string | the coverpoint label, or `cp_{i}` when unlabelled. For a cross it is always `cross_{i}` — a user-written cross label is not carried |
| `…[].kind` | string | `"cross"` or `"coverpoint"` |
| `…[].num_bins` | int | the item's bin count |
| `…[].covered_bins` | int | the popcount of the final hit bitmap, excluding unknown bits |
| `…[].coverage_pct` | number, six decimals | `covered * 100 / num_bins`, or `0.0` when `num_bins == 0` |

The overall percent is computed by the same routine the RTL's own `get_coverage()` uses — property
3 of §1 — and the accumulation order is part of the contract because floating-point addition is
order-sensitive: per item `pct = covered * 100 / num_bins`, a coverpoint with no bins is excluded
from the average, a cross always counts with an implicit weight of 1, terms are accumulated
coverpoints then crosses, and the result is `sum / total_weight`, or `0.0` when the total weight is
zero. Percentages format as six decimals, matching `$display("%f", …)`; a non-finite value renders
as `0.000000`. The internal per-item weight is not serialized.

Not in this file: SVA assertion pass/fail counts, cover-property counts, and per-bin hit detail
(§13).

## 8. `trace.jsonl` — the `--probe` change stream

Written whenever at least one `--probe` or `--probe-file` path resolved, and always written in that
case even when it is empty (a probed net that never changes). `--probe` requires `--obs-dir`, so
absence of the file means no probe was given.

### 8.1 Probe grammar

1. The `--probe` values are taken in command-line order.
2. `--probe-file F` is read as UTF-8; each line is trimmed, a line that is empty or starts with `#`
   is skipped, and every other line is appended after the `--probe` values.
3. An empty path set means no probing.
4. A non-empty path set with no `--obs-dir` is a loud error, exit 3.
5. Each path is resolved by **exact string equality** against the elaborated net-name table. There
   is no glob, wildcard, prefix, regular expression or bit/element selection syntax: the path must
   be the full dotted hierarchical name as the table spells it (`top.y1`, `top.u1.y`, `top.clk`).
   An unresolved path is loud, never a silent skip.
6. Kind filter: a dynamic-array, queue or string **handle** is rejected, an unpacked **array** is
   rejected, a **real** net is rejected. v1 traces a scalar, vector or packed net.
7. Duplicates are harmless — the same net is armed once and emits once.

There is no CLI surface that lists available net names; the rejection message says so.

### 8.2 Record schema

```json
{"v":1,"t":5,"kind":"chg","path":"top.u1.y","old":"xxxx","new":"0001"}
```

| Key | Type | Meaning |
|---|---|---|
| `v` | int | envelope version, always `1` |
| `t` | int | the current simulation time in raw ticks of the global precision. There is no delta-cycle or region field (§13) |
| `kind` | string | always `"chg"` |
| `path` | string | the full dotted net path, the same string `--inst-paths` and the VCD `$scope` structure use; JSON-escaped, so an escaped SystemVerilog identifier survives |
| `old` | string | the last emitted value string |
| `new` | string | the new value string |

Value encoding is §3 pin 4: a full-width, unprefixed, MSB-first 4-state binary string, one character
per bit from `{0, 1, x, z}`. A width-1 net yields one character.

### 8.3 Sampling semantics

- **Transition-only.** A record is emitted from inside the engine's change notification — the same
  stream the VCD writer consumes — and is then deduplicated against the last emitted formatted
  string, so a same-value write or a glitch in another word emits nothing.
- **The comparison baseline is armed before the event loop**, from each probed net's construction
  value (usually all-`x`). A value first driven at time 0 is therefore logged, with `old` set to the
  construction default.
- **Independent of waveform dumping.** A `--probe` run with no `$dumpvars` still traces.
- **Whole-net only.** The value formatted is the net's full width regardless of which word changed,
  which is safe because the CLI refuses arrays.
- **One ordered stream.** All probes share one record vector, so records from different paths
  interleave in emission order rather than grouping by path.
- **Backend-invariant.** The native backend cannot reach the sink from inside its store, so its
  arena captures each `(net, value)` pair at its own store point and hands it to the same emitter,
  preserving A→B→A round-trips within one slot.

## 9. `$vita_stage` and `stage.jsonl`

### 9.1 The built-in

`$vita_stage` is the only vendor introspection built-in. Signature:

```systemverilog
$vita_stage("label" [, v0, v1, …]);
```

At least one argument, the label; the rest are arbitrary runtime expressions evaluated at execution
time. It lowers to a no-op print node plus a statement-id sidecar, so the frozen system-task
enumeration gains no variant and `format_version` is unaffected. Zero arguments and use as a
deferred-assertion action are both `VITA-E3009`.

**It never prints.** With or without the plusarg, no `$vita_stage` text reaches stdout. Capture is
armed only by a plusarg equal to `STAGE_TRACE` or starting with `STAGE_TRACE=`; a neighbouring
spelling such as `+STAGE_TRACEX` does not arm it. Without the plusarg the call returns immediately
and the record index does not advance.

`$vita_stage` does not disqualify the native backend, and its argument reads are threaded through
the alternate net reader so a native run records the values the design actually wrote. It counts as
a builtin and gets its own `builtins` row rather than hiding inside `$display`.

vita predefines no macro, so a design that must also compile under another simulator guards the call
and vita is invoked with the define:

```systemverilog
`ifdef VITA
  $vita_stage("decode", pc, opcode);
`endif
```

```
vita design.sv -D VITA --obs-dir out +STAGE_TRACE
```

### 9.2 The file

`stage.jsonl` is written whenever `+STAGE_TRACE` was given, independently of whether the design
contains any `$vita_stage` at all: with the plusarg and no call site the file is written empty.

```json
{"v":1,"t":0,"kind":"stage","label":"init","idx":0,"vals":["0"]}
{"v":1,"t":35,"kind":"stage","label":"done","idx":1,"vals":["3","3"]}
```

| Key | Type | Meaning |
|---|---|---|
| `v` | int | envelope version, always `1` |
| `t` | int | simulation time at the call, in raw ticks |
| `kind` | string | always `"stage"` |
| `label` | string | the first argument, `%s`-coerced. It may be a runtime string variable, not only a literal |
| `idx` | int | a monotonic per-run counter starting at `0`, incremented once per captured call. Not per label, not per scope |
| `vals` | array of string | the remaining arguments, each formatted with `$display("%0d", …)` semantics and emitted as a JSON string so unknown values are representable |

Decimal formatting rules, which are the mirror half of the three-way teeth: a real value rounds
half-away-from-zero to a 64-bit integer (saturating; NaN becomes 0); a value containing unknown bits
renders as a single collapsed character (`x`/`X`/`z`/`Z`); anything else is exact decimal at any
width, with signed values as `-` followed by the two's-complement magnitude. A **string** argument
renders numerically — as its packed byte value, not as text.

Records go into one ordered vector, so the file is in emission (time) order.

## 10. `--hier-tree` and `--inst-paths`

Both are written immediately after elaboration and before simulation, from the instance table the
elaborator populates in elaboration order. Both are plain UTF-8 text, one record per line,
`\n`-terminated, written by overwrite (no atomic rename). A write failure prints
`error[VITA-E0001]: cannot write --hier-tree '{path}': {e}` and does not change the exit code.

`--hier-tree` is a pre-order walk from every root; children are emitted in ascending elaboration
order, indented two spaces per level. Each line is `<leaf of the instance path> : <module name>`.

```
top : top
  m0 : mid
    u : leaf
    u : leaf
  arr[1] : leaf
  arr[0] : leaf
```

Arrayed-instance segments survive because they are part of the leaf segment; note that a `[1:0]`
range elaborates in descending order. **Generate-scope information is lost in this file**:
`top.m0.g[0].u` and `top.m0.g[1].u` both render as `u : leaf`, which is why the two `u` lines above
are identical.

`--inst-paths` writes every instance path verbatim, one per line, in elaboration order. Arrayed and
generate segments appear in full, consistent with the VCD `$scope` structure — so this is the file
to copy a scope or force target out of.

Status at HEAD: these two flags are the one accept-and-drop on the rail. On a staged applet they
parse, reach no writer, and the run exits 0 with no diagnostic and no file; every other obs surface
is loud there (§4.2).

```
top
top.m0
top.m0.g[0].u
top.m0.g[1].u
top.arr[1]
top.arr[0]
```

## 11. Determinism, per file

| File | Byte-identical across two runs of the same input | Exceptions |
|---|---|---|
| `run.json` | yes | exactly four isolated fields: `utc_unix_s`, `wall_s`, `elab_s`, `sim_s`. Under `--obs-procs-time` also every `time_s` and `obs_overhead_est_s` |
| `results.jsonl` | yes, fully — the file carries no wall-clock field | none |
| `coverage.json` | yes | none |
| `trace.jsonl` | yes | none |
| `stage.jsonl` | yes | none |
| `--hier-tree` / `--inst-paths` | yes (elaboration order) | none |

What makes it hold: hand-rolled JSON with a fixed key order; a `BTreeMap` behind every map that is
iterated (`codegen.reject_reasons`, `native.reject_reasons`, the subroutine route table, the builtin
rows keyed by static string, the subroutine-call rows keyed by `FuncId`); a total tiebreak on every
sort; and a count, never a time, as every sort key. `source.name` is a basename so the same design
run from two directories byte-diffs clean.

Row order, collected:

| Object | Primary | Tiebreak |
|---|---|---|
| `processes.items` | `evals` descending | `domain` ascending, then `index` ascending |
| `builtins.items` | `calls` descending | `name` ascending |
| `subroutine_calls.items` | `calls` descending | `FuncId` ascending |
| `subroutines.items` | `(module, name)` ascending | — |
| `coverage.groups` | elaboration order | — |
| `coverage.groups[].coverpoints` | tracker order, coverpoints then crosses | — |
| `trace.jsonl` | emission (time) order, all paths interleaved | — |
| `stage.jsonl` | emission (time) order | — |

A comparison of two `run.json` files strips exactly the four isolated fields and asserts all four
are present; that is the determinism golden, and it is the test that R-F1 rests on.

## 12. A worked example

```
vita d.sv --obs-dir out --obs-procs --probe top.y1 \
     --hier-tree hier.txt --inst-paths inst.txt +STAGE_TRACE -o d.vcd
```

on a design with one `always #5 clk`, two instances of a module holding one `always_ff` and one
`function automatic [3:0] f`, and two `$vita_stage` calls:

```json
{
  "schema_ver": 1,
  "tool": "vita",
  "version": "0.2.0",
  "format_version": 31,
  "seed": null,
  "plusargs": ["STAGE_TRACE"],
  "source": {"name": "d.sv", "blake3": "476800d8e8b05f12865dba0227b12bae7f9a83863c4472b431c3809e0d0a3c59"},
  "finish_reason": "finish",
  "exit_class": "ok",
  "exit_code": 0,
  "sim_time": 35,
  "counts": {"errors": 0, "warnings": 1, "fatals": 0},
  "status": "PASS",
  "backend": "native",
  "backend_requested": "native",
  "codegen": {"able": 3, "total": 5, "frame_bodies": 2, "reject_reasons": {"delay": 1, "wait": 1}},
  "native": {"eligible": true, "buildable": true, "refused": null, "reject_reasons": {}},
  "subroutines": {"counts": {"total": 1, "frame": 1, "inlined": 0}, "sites_semantics": "…", "uncounted": "class methods and hierarchical calls", "items": [
    {"module": "sub", "name": "f", "kind": "function", "route": "frame", "sites": 2}
  ]},
  "processes": {"timed": false, "counts": {"processes": 5, "assigns": 6, "total_evals": 56},
  "items": [
    {"domain": "assign", "index": 0, "kind": "port", "scope": "top.u1", "file": "d.sv", "line": 7, "col": 10, "evals": 9},
    {"domain": "process", "index": 1, "kind": "always", "scope": "top", "file": "d.sv", "line": 9, "col": 3, "evals": 8},
    {"domain": "process", "index": 3, "kind": "always_ff", "scope": "top.u1", "file": "d.sv", "line": 3, "col": 3, "evals": 3},
    {"domain": "process", "index": 0, "kind": "var_init", "scope": "top", "file": "d.sv", "line": 6, "col": 15, "evals": 1}
  ]},
  "builtins": {"timed": false, "attribution": "self-plus-arguments", "included_in_processes": true, "time_semantics": "…", "distinct": 3, "total_calls": 4,
  "items": [
    {"name": "$vita_stage", "calls": 2},
    {"name": "$display", "calls": 1},
    {"name": "$finish", "calls": 1}
  ]},
  "subroutine_calls": {"timed": false, "key": "…", "time_semantics": "…", "distinct": 2, "total_calls": 6,
  "items": [
    {"func": 0, "name": "top.u1.f", "decl_file": "d.sv", "decl_line": 2, "decl_col": 28, "calls": 3},
    {"func": 1, "name": "top.u2.f", "decl_file": "d.sv", "decl_line": 2, "decl_col": 28, "calls": 3}
  ]},
  "utc_unix_s": 1788921496,
  "wall_s": 0.012602,
  "elab_s": 0.011536,
  "sim_s": 0.001009
}
```

`processes.items` is abridged here — the real file lists every row, zero-eval ones included. The
three `…` strings are the fixed sentences quoted verbatim in §5.5, §5.6 and §5.7.

The sibling files from the same run:

```
results.jsonl  {"v":1,"t":35,"kind":"result","status":"PASS","finish_reason":"finish","exit_code":0,"sim_time":35,"errors":0,"warnings":1,"fatals":0}
trace.jsonl    {"v":1,"t":5,"kind":"chg","path":"top.y1","old":"xxxx","new":"0001"}
               {"v":1,"t":15,"kind":"chg","path":"top.y1","old":"0001","new":"0010"}
               {"v":1,"t":25,"kind":"chg","path":"top.y1","old":"0010","new":"0011"}
stage.jsonl    {"v":1,"t":0,"kind":"stage","label":"init","idx":0,"vals":["0"]}
               {"v":1,"t":35,"kind":"stage","label":"done","idx":1,"vals":["3","3"]}
hier.txt       top : top
                 u1 : sub
                 u2 : sub
inst.txt       top
               top.u1
               top.u2
```

Read in order: `status` and `finish_reason` say whether and how it ended; `counts` says how loudly;
`processes` says which body ran the most; `builtins` says which primitive it spent itself on;
`subroutines` says which routine is a frame call; `trace.jsonl` says what the signal did; and
`stage.jsonl` aligns the run against a reference implementation's stage log.

## 13. Not implemented at HEAD

Each row is a present fact about this build, not a schedule. The order in which they are taken is
[ROADMAP §6](../ROADMAP.md).

| Item | State at HEAD |
|---|---|
| `run_id` in `run.json` | No such key |
| `--seed` | No such flag; `"seed": null` is a constant |
| Full input identity in `source.blake3` | The digest covers the source **text** only, so two runs of one `` `ifdef ``-switched file with and without `-D FOO` behave differently and report the same digest |
| `-G` / `--param` overrides in `run.json` | No key carries them; two runs differing only in `-G` produce manifests identical outside the four isolated wall-clock fields |
| `results.jsonl` v2 (per-test-case ledger, `$vita_test_begin`/`$vita_test_end`) | Not parsed, not lowered, not dispatched. v1 is one line per run |
| `detail_ref` on a FAIL line | The single line carries no such key |
| R-L2 failure detail (`fail/*.json`) | No producer |
| R-L3 `stuck_in` / hang detection | No producer |
| Per-element array probe, real probe, class or event probe | Loud-rejected at the CLI (§4.2) |
| R-L4 handshake and protocol channel events | No producer |
| SVA assertion pass/fail and cover-property counts in `coverage.json` | The file carries covergroups only |
| Per-bin hit detail in `coverage.json` | Only `num_bins` and `covered_bins` |
| R-L6 `sva.jsonl` (property name plus support cone) | No producer |
| R-C1 `vrun --control stdio` JSON-RPC (`peek`/`poke`/`step`/`run_until`/`finish`) plus a poke journal | No control surface exists |
| R-C2 `snapshot` / `restore` / `rewind_to` | No producer |
| R-C3 region-annotated events | No record carries a region or delta field |
| R-C4 X-origin (`cause: uninit \| multi-drv \| arith-X`) | No producer |
| R-C5 dataflow backward slice | No producer |
| R-I1 config-driven signal introspection | Partial: `--probe` / `--probe-file` is a manual path list, not a config-driven auto-dump |
| R-I2 semantic transaction log | No producer |
| §3 pin 4's enum values rendered as **names** | No name path exists: `trace` values are 4-state binary and `stage` values are `%0d` decimal |
| The obs rail on the staged flow (`vcmp` / `velab` / `vrun`) | Loud-rejected (§4.2) |
| A compile-fail manifest | A front-end or elaborate failure writes no obs directory |
| The call tree (`processes` decomposed to task granularity) | `processes.items[].domain` is `"process"` or `"assign"` only. The blocker is structural: an inlined subroutine leaves no call node, so a seam-based profile reports it 0 times, and `0` reads as *free* about the very thing the user is hunting. `subroutines` (§5.6) is the minimum form of the prerequisite — the elaborate-time record of which route each routine took |
| Per-call-site builtin rows (`{"name":"$sscanf","file":…,"line":…}`) | The table is name-level aggregation. The system-task half could locate today; the system-function half cannot, because the effect carries no statement id and the pure evaluator has no statement context at all, and half a table locating is worse than none |
| A declaration site on the static `subroutines` rows | Items carry `module`, `name`, `kind`, `route`, `sites` only, so the two subroutine objects cannot be joined (§5.7) |
| `WPROG-WHY`: a per-`(reason, count)` tally of expression-level compile declines beside `codegen` | Not emitted. `codegen.reject_reasons` is a per-**process** census, so a body can report `able 1/1` while every evaluation of its right-hand side runs the generic path, and the compiled-lane boundary can only be inferred from call counts — an inference that has produced wrong causes twice |
| `--hier-tree` generate scopes | Collapsed: sibling generate instances render as identical lines (§10). `--inst-paths` carries the full path |
| `--hier-tree` / `--inst-paths` on a staged applet | Accepted and dropped: the run exits 0, prints no diagnostic and writes no file. This is the one accept-and-drop on the rail; every other obs surface is loud on a staged applet |

## 14. Tests that gate the schemas

| File | What it pins |
|---|---|
| `crates/cli/tests/obs.rs` | the `run.json` constants and the exact `results.jsonl` prefix · the determinism golden (strip exactly the four wall-clock fields, assert all four are present) · status versus process exit · plusargs and source digest · no obs output on a compile error · `exit_class` under `-Werror` · staged rejection · empty `--obs-dir` rejection · no output without the flag · the coverage schema against `get_coverage()`, including crosses, zero hits, weighting and determinism · the trace change stream, its three-way match against `$monitor`, probe typo and missing-directory loudness, determinism, the loud array/real rejections and full-width packed values · the stage capture, its three-way match against `$display`, native capture, no-plusarg no-op, determinism, zero-argument loudness and the `STAGE_TRACE=` spelling · the `codegen` claim and reason keys, backend invariance, and the `native` reject families · native-backend probe capture at the store point |
| `crates/cli/tests/obs_procs.rs` | hand-checkable counts · byte identity across runs · backend invariance · timing adds `time_s` without moving counts · one row per instance · loudness without `--obs-dir` · `null` without the flag · staged rejection · port rows locating their connection · wildcard port rows staying unlocated · array-port elements sharing one connection span |
| `crates/cli/tests/obs_builtins.rs` | hand-checkable counts · total row order · byte identity · backend invariance · timing without moving counts · `null` without the flag · a `$display` inside a function body counting |
| `crates/cli/tests/obs_subroutines.rs` | every route reported with its lowered site count · the counts header and semantics strings · emission without `--obs-procs` · the empty table · the 2-state/4-state return-type route split · output-formal calls in both spellings · class declarations not shifting module counts |
| `crates/cli/tests/obs_subroutine_calls.rs` | per-instance counts and the declaration join · backend invariance · byte identity · `null` without `--obs-procs` · an inlined subroutine having a static row and no runtime row · a suspendable task frame counted and not timed · a synchronous call timed · the object describing itself truthfully |
| `crates/cli/tests/help_covers_flags.rs` | every literal flag arm in the parser appears in `vita --help` |
| `crates/sim-engine/tests/native_gate.rs` | the stage sidecar and the probed-net set are both native-backend core — neither disqualifies |
| `crates/sim-engine/src/profile.rs` unit tests | nested time charged once · counting without timing never reads the clock |

## 15. Schema evolution

- A schema change is made in this document first, and `schema_ver` moves with it.
- `schema_ver` bumps only when the **record envelope** changes. An additive field keeps
  `schema_ver` at `1`, because a consumer that ignores unknown keys is unaffected.
- The record envelope of §3 pin 4 (`v`, `t`, `kind` first, fixed key order) is the freeze line.
- The rail is out of band with respect to the frozen IR, so no rail change moves
  `format_version` — see [16-schema-hash-spec.md](16-schema-hash-spec.md) and
  [17-sim-ir-ir-backbone-freeze.md](17-sim-ir-ir-backbone-freeze.md) for what does.
- Related specifications: [21-tier3-native-backend.md](21-tier3-native-backend.md) for the backend
  gate the `native` object reports, [06-simulation-engine.md](06-simulation-engine.md) for the
  scheduler an activation counts against, [07-vcd-format.md](07-vcd-format.md) for the waveform
  rail and its scope naming, [13-diagnostics-and-logging.md](13-diagnostics-and-logging.md) and
  [15-error-code-reference.md](15-error-code-reference.md) for the diagnostic surface, and
  [../manual/004_cli-reference.md](../manual/004_cli-reference.md) for the user-facing flag
  reference.
