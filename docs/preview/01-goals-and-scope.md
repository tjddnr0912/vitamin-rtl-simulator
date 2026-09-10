# 01 · Goals and scope

Two goals define this project, and both are stated as commitments a test can hold. This document
gives each goal its testable form, then draws the language boundary: what the implementation accepts,
what it accepts with a stated restriction, and what it refuses. The boundary tables here are the
single authority on scope — the synthesizability markers in
[hdl-reference/](hdl-reference/01-synthesizability-legend.md) describe what a synthesis tool would
accept, which is a different question. `initial`, `#delay`, `$display` and `$finish` are marked
non-synthesizable there and are all in scope here.

## G1 — correctness at the level of the established simulators

The target is the accuracy of Icarus Verilog, Verilator, Xcelium and VCS within the scope below,
under one rule: correct or loud. A construct is either simulated to the IEEE meaning or refused with
a diagnostic. The accuracy ladder is silent-wrong ≪ loud ≪ correct-support, and movement is upward
only: making a working construct loud is a regression, and trading one silent-wrong for another is
not progress.

| Commitment | How it is held |
|---|---|
| A supported construct produces the same signal values and transition times as the differential oracle | live differential against `iverilog` + `vvp`, and against Verilator on a calibrated subset that carves out the 2-state, X-init and event-ordering differences. On a tool conflict the IEEE LRM decides ([09](09-testing-and-verification.md)) |
| A construct with no oracle is still implemented, not deferred | the assertion, class, randomisation, parameterized-class and virtual-interface areas are pinned by hand against the LRM clause, because a tool that refuses a legal construct is not evidence that the construct is unwanted |
| An unsupported construct is refused, never approximated | every refusal carries a `MsgCode`; the diagnostic corpus asserts codes, not message text, and a CI gate holds the code set 1:1 against the catalogue in [15](15-error-code-reference.md) |
| The same design gives the same bytes everywhere | byte-identical stdout and waveform across the supported platforms and across all three executors; the executor-equivalence suite compares stdout, waveform bytes, `sim_time`, finish reason and exit class |
| A change that alters the frozen IR cannot be shipped by accident | the structural schema hash of `sim_ir::SimIr` is pinned; changing a frozen type flips it and forces a `format_version` bump in the same change ([16](16-schema-hash-spec.md), [17](17-sim-ir-ir-backbone-freeze.md)) |
| A stale artifact is refused, not simulated | the header gate plus the recorded upstream digests; a rejection is exit 2 ([14](14-staged-artifacts.md)) |
| Every design change is reviewed adversarially | at least two lenses — a differential lens against a live oracle, and a soundness lens against the LRM text — and a design change re-opens the review ([ENGINEERING_RULES.md](../ENGINEERING_RULES.md)) |

Status at HEAD: residual divergences that produce a wrong value without a diagnostic are tracked as
defects in [ROADMAP §2](../ROADMAP.md) and listed with workarounds in
[manual/006](../manual/006_limitations.md). The ladder forbids adding to that set.

## G2 — an observability rail an agent can drive

An agent driving a simulation needs machine-readable answers to "what ran", "what happened" and
"what changed", without scraping a human transcript. The rail is a set of files with fixed key order
and no wall-clock content in the ledger, so two runs of the same design differ only where the design
differs. The full specification is [19](19-ai-agent-observability.md).

| Commitment | Surface at HEAD |
|---|---|
| A run reports its own identity and outcome as structured data | `--obs-dir <d>` writes `run.json`: tool and version, `format_version`, source name and digest, finish reason, exit class and code, `sim_time`, error/warning/fatal counts, PASS/FAIL status, the backend requested and the backend that ran, the codegen and native-eligibility reports, and the static subroutine census |
| A result is one machine-readable line | `results.jsonl`, exactly one record per run, with no wall-clock field |
| Value changes can be streamed | `--probe <path>` / `--probe-file` writes `trace.jsonl`, one record per change, in time order, values as 4-state binary strings |
| A design can emit its own labelled checkpoints | `$vita_stage(...)` plus the `+STAGE_TRACE` plusarg writes `stage.jsonl` |
| Functional coverage is reported as data | `coverage.json`, per instance and per coverpoint or cross |
| Structure can be read without running | `--hier-tree` and `--inst-paths` |
| Per-process and per-subroutine cost is attributable | `--obs-procs` adds the `processes`, `builtins` and `subroutine_calls` objects to `run.json` |

Not implemented at HEAD, stated so a consumer does not plan around it:

| Item | Present fact |
|---|---|
| Seed control | no `--seed` flag exists; `run.json` reports `"seed": null` |
| Full input identity | `source.blake3` covers the source text only, so `-D`/`+define+`/`-I`/`-G` do not change it, and parameter overrides are not recorded |
| Per-testcase ledger, failure detail files, `sva.jsonl` | no producer exists; `results.jsonl` is one line per run |
| Interactive control | there is no control channel: no `peek`/`poke`/`step`/`run_until`, no snapshot, restore or rewind |
| Region and X-origin annotation | trace records carry no region, delta or cause field |
| Array-element, real, class and event probes | refused with a diagnostic at the CLI |
| The rail under the staged commands | refused with a diagnostic; `--obs-dir`, `--probe` and `$vita_stage` are one-shot `vita` only |
| An observability directory for a failed compile | a front-end or elaboration failure writes no directory |

## Language scope

Vocabulary used in every table below.

| Status | Meaning |
|---|---|
| Supported | lowered and simulated to the IEEE meaning |
| Restricted | accepted in a stated shape; outside that shape it is refused with a diagnostic |
| Refused | a diagnostic naming the construct — `E-PARSE-UNEXPECTED-TOKEN` (`VITA-E2002`) from the parser, `E-ELAB-UNSUPPORTED` (`VITA-E3009`) from elaboration |
| Absent | no grammar arm exists; the word lexes as an ordinary identifier or a reserved keyword and the construct dies at `VITA-E2002` |

### Design units

| Construct | Status | Notes |
|---|---|---|
| `module` / `macromodule`, ports, `parameter` / `localparam`, `generate` / `genvar` | Supported | typed parameters (`parameter int W = 32`) and `parameter type T` are both parsed; a type parameter is desugared into a width and a shape parameter, and an override that changes the shape is a loud runtime refusal rather than a silently unsigned type |
| `interface` / `endinterface`, `modport` | Restricted | an instance flattens to plain nets plus symbol aliases. Nets, continuous assigns, procedural blocks, modports, parameters, port declarations, genvars, imports and block-locals are accepted in an interface body; nested instances, generate blocks, subroutines, typedefs and `defparam` inside one are refused, as are non-ANSI or interface-typed header ports and interface instance arrays. A write through a modport `input` is refused with `E-ELAB-PORT-MISMATCH` |
| `package` / `endpackage`, `import pkg::*` and `import pkg::sym` | Supported | at compilation-unit scope, module scope and in an ANSI header. An import from an unknown package, or of an absent symbol, is refused; a name reachable through two wildcard imports is unbound and refused at its use site |
| `program` / `endprogram` | Restricted | parses and elaborates as a top-level module container; IEEE §24 Reactive-region scheduling of program processes is approximated by the Active region |
| `class` / `endclass` at top level, in a module body, in a package | Supported | see Classes below |
| `primitive` (UDP), combinational and sequential | Supported | desugared into a synthetic module |
| `bind <target> <checker> <inst> (…)` | Supported | body binds are hoisted to top level and prescanned before any module lowers |
| Compilation-unit items before the first `module` | Restricted | unit-scope `typedef`, `parameter`/`localparam`, `function` and `task` are injected into every later unit; a unit `parameter` becomes a `localparam`. The `$unit::name` spelling is absent |
| `config` / `endconfig`, `library`, `liblist`, `cell`, `design`, `use`, `incdir`, `instance` | Absent | lexed as keywords, never dispatched: `expected 'module', found keyword 'config'` |
| `export`, nested modules, `extern module`, `interface class`, `implements`, `checker`, `nettype`, `alias`, `timeunit` / `timeprecision` | Absent | `` `timescale `` is the supported channel for time units |

### Types

| Construct | Status | Notes |
|---|---|---|
| `wire`, `tri`, `uwire`, `wand`, `wor` | Supported | `tri` and `uwire` collapse to a plain wire with no strength or pull semantics; `wand`/`wor` get real wired resolution |
| `triand`, `trior`, `tri0`, `tri1`, `supply0`, `supply1`, `trireg` | Refused | parsed, then `unsupported net/var kind (v1)` at elaboration |
| `reg`, `logic`, `integer`, `time`, `real`, `realtime` | Supported | `real`/`realtime` are IEEE-754 `f64` |
| `bit`, `byte`, `shortint`, `int`, `longint` | Restricted | 2-state: default-initialised to 0, and `x`/`z` coerces to 0 on every write. Status at HEAD: the on-write coercion is applied by the one-shot flow; staged `vrun` initialises to 0 but does not coerce on write |
| `string` | Supported | heap-handle storage, with the length, character, substring, case and comparison methods and `$sformat`/`$sformatf` |
| `event` | Supported | desugars to a counter; `->` triggers, `@(ev)` waits |
| `void` | Restricted | two positions: a `function void` return, which desugars to a task, and the discard statement `void'(call);` |
| `typedef enum`, with a packed or atom base | Supported | the base's declared signedness is preserved; the `first`/`last`/`next`/`prev`/`name`/`num` methods are available |
| `typedef struct packed`, `typedef union packed` | Supported | members lay MSB-first into one flat vector, a union overlays them; member access desugars to a constant part-select. A member whose width does not fold takes a symbolic layout |
| `typedef struct` (unpacked record) | Restricted | scalar records only: members become independent nets. An array of unpacked structs, and a packed-struct member inside a record, are refused |
| `typedef union` (unpacked) | Refused | `packed` is required after `union` |
| `chandle`, `shortreal`, `tagged union` | Absent | |
| class handle, `null` | Supported | a handle is an object id; `null` is 0 |
| `virtual interface` | Restricted | static alias model: bound once (`vif = bif;`), after which every `vif.member` is symbol-aliased to that instance's net. Dynamic or conditional re-binding, an unbound handle, and a type mismatch are all refused |

### Arrays and dynamic storage

| Construct | Status | Notes |
|---|---|---|
| Packed vectors and packed arrays, multi-dimensional unpacked arrays | Supported | per-dimension bounds are enforced; an out-of-range unpacked index reads `x` and drops the write, a packed bit-space over-index reads `x` |
| Whole-array and element-wise array assignment | Supported | IEEE §7.6 positional correspondence, expanded element-wise at elaboration, including `<=` and transport `#d` |
| Dynamic arrays `[]`, queues `[$]` and `[$:N]`, associative arrays | Supported | every integral key spelling shares one signed 64-bit key domain; `[string]` keys are byte strings |
| Wildcard associative key `[*]` | Refused | the diagnostic names the accepted key spellings |
| `.size` `.num` `.exists` `.delete` `.push_back` `.push_front` `.insert` `.sort` `.rsort` `.reverse` | Supported | the mutators are statements |
| `.pop_back` `.pop_front`, `.first` `.next` `.last` `.prev` | Restricted | direct right-hand side of a blocking assignment only, because each writes through a reference |
| Reductions `.sum` `.product` `.and` `.or` `.xor`, bare and with `with (expr)` | Supported | on dynamic arrays, queues, associative arrays and 1-D fixed unpacked arrays; not on a string |
| Locators `.min` `.max` `.unique` `.unique_index` `.find*` | Restricted | statement form `dst = src.locator()` where `dst` is a queue handle; the `find*` family requires a `with (condition)` clause |
| `foreach` | Supported | over 1-D fixed unpacked arrays, dynamic arrays, queues and associative arrays, and over multiple dimensions; the index is renamed so it cannot clobber an outer name; `break`/`continue` work inside |
| Assignment patterns `'{…}` | Restricted | all-positional or all-keyed, never mixed. Named members and `default:` are supported for packed structs and fixed unpacked arrays; integer keys, type keys, replication inside a pattern, and a call as a `default:` value are refused |
| Streaming `{<<N{…}}` / `{>>N{…}}` | Restricted | a constant bit count as the slice size. A type as the slice size, and the `with [range]` form, are refused |

Caps enforced at elaboration: one net is at most 2²⁰ bits wide, an unpacked array at most 2²⁴
elements, the net arena at most 2¹⁷ nets, an instance array at most 4096 elements, a generate-for at
most 4096 iterations nested at most 32 deep, and at most 200 elaboration diagnostics.

### Procedural blocks, statements and timing

| Construct | Status | Notes |
|---|---|---|
| `initial`, `always`, `always_ff`, `always_comb`, `always_latch` | Supported | auto-sensitivity for the SV forms |
| `final` | Restricted | a zero-time one-shot at end of simulation; a timing control inside is refused (IEEE §9.2.3) |
| `fork` … `join` / `join_any` / `join_none`, `wait fork`, `disable fork` | Supported | a nested `fork` inside a subroutine frame body, and `disable fork` inside one, are refused |
| Blocking `=`, nonblocking `<=`, `if`, `case`, `casez`, `casex`, `for`, `while`, `repeat`, `forever`, `do`-`while`, `begin`/`end`, `break`, `continue`, `return`, statement labels | Supported | `casez` and `casex` follow the IEEE don't-care rules exactly; a statement label desugars to a named block, so `%m` and `disable L` see it |
| `unique` / `priority` on `if` and `case` | Restricted | the no-match arm is reported as `W-RUN-UNIQUE-VIOLATION`. Multi-match is not checked: the lowered cascade is first-match-wins, so an overlap is unobservable. `unique0`/`priority0` suppress the report |
| `disable` of an enclosing named block | Restricted | the target must lexically enclose the statement; a cross-frame `disable` is refused |
| Procedural `assign` / `deassign`, `force` / `release` | Restricted | whole net or variable targets only; a bit- or part-select target is refused. `force` evaluates its right-hand side once, at execution, rather than re-evaluating it when an operand changes; the forced value is then held against every other driver until `release` |
| `#delay` with a constant or a runtime expression, `@(event)`, `wait(expr)`, named events | Supported | an in-body edge wait must be a bare signal name or a constant bit-select; a multi-term in-body edge wait is refused |
| Intra-assignment `= #d`, `<= #d`, `= [repeat(n)]@(ev)`, `<= [repeat(n)]@(ev)` | Supported | the right-hand side is captured now and written later: `= #d` suspends the process for the delay and then writes, `<= #d` files a transport update for the later time. Neither cancels a pending write. `assign #d` is the inertial form: a pulse narrower than the delay is absorbed |
| `let NAME [(formals)] = expr;` | Supported | substituted at each use; arity mismatch and recursion are refused |
| `clocking` / `endclocking` | Restricted | default-skew input sampling: elaboration synthesises preponed-sampled holding nets, `@(cb)` is the clocking event and `cb.sig` reads the holding net. An explicit skew, a clocking `inout`, and a non-net-reference bind are refused. Status at HEAD: a signal declared as a clocking output is not driven correctly — the defect row is in [ROADMAP §2](../ROADMAP.md) |
| Gate primitives `and or nand nor xor xnor buf not bufif0 bufif1 notif0 notif1`, with delays | Supported | |
| `specify`/`endspecify`, drive strengths, `vectored`/`scalared`, `pullup`/`pulldown`, MOS and bidirectional switch primitives, `edge` event control, comma-separated `for` init | Absent | the keywords lex, but no parser rule accepts them; the comma-separated `for` init likewise has no arm |

### Subroutines and hierarchy

| Construct | Status | Notes |
|---|---|---|
| `function`, `task`, including `function void` and `function string` | Supported | |
| `automatic` / `static`, recursion | Restricted | recursion runs on a call-frame model with a depth cap; exceeding it is a loud fatal |
| Default argument values, named arguments `.formal(v)` | Supported | |
| `ref` / `const ref` formals | Restricted | copy-in / copy-out; the spelling is recorded for diagnostics only |
| Unpacked-array formals, body-local `typedef enum` | Supported | |
| Hierarchical name reference (`tb.dut.x`) | Restricted | a whole-net read and a whole-net write are supported, resolved once the hierarchy is known. The element and part-select forms are restricted per shape, and each refusal names the supported spelling. A hierarchical name in an event control, a hierarchical `disable`, and a hierarchical reference to an `automatic` block-local are refused |
| Hierarchical function and task call (`u1.f(x)`) | Restricted | supported through deferred resolution; the named-argument form and an unresolvable path are refused |
| `defparam` | Restricted | a direct-child `instance.param` target with a constant value; a multi-level path is refused. IEEE-deprecated; `#(.param())` is the supported spelling |

### Assertions, coverage, classes, randomisation

| Area | Status | Notes |
|---|---|---|
| Immediate `assert` / `assume`, with and without action blocks | Supported | a failing assertion with no action block reports `E-RUN-ASSERT-FAIL` |
| Deferred `assert #0` and `assert final` | Supported | maturing in the Observed and Reactive regions respectively; `assert #N` with N ≠ 0 is refused |
| Concurrent `assert property`, `cover property` | Restricted | single- and multi-clock implications `\|->` / `\|=>`, `disable iff`, `default disable iff`, action blocks, named `sequence` and `property` with formal arguments, and labelled module-scope assertions. A bare sequence property with no implication is refused |
| Sequence operators `##n`, `##[m:n]`, `##[m:$]`, `[*n]`, `[*m:n]`, `[*m:$]`, `[->n]`, `[=n]`, `throughout`, `within`, `[+]`, re-clocking at a `##` boundary | Restricted | the unbounded, goto and non-consecutive forms require a boolean operand; `[->n]`/`[=n]` take a single count; `within` is a top-level antecedent only. A sequence expanding past 256 alternatives is refused |
| Property operators `and`, `or`, `not`, `implies`, `iff`, `until`, `s_until`, `s_eventually`, `nexttime`, `s_nexttime`, top-level `always` | Restricted | operands must reduce to a same-clock verdict; a nested `always`, a bounded `s_eventually`/`nexttime`, weak unbounded `eventually` and `s_always` are refused |
| Sequence and property local variables | Restricted | integral fixed-width only; a `real`, `string`, `event`, class or net-kind local, and an initialiser, are refused |
| `first_match`, `intersect` as a sequence operator, `expect`, `restrict` | Absent | |
| Sampled-value functions `$sampled` `$rose` `$fell` `$stable` `$changed` | Supported | prev-register lowering |
| `$past` | Restricted | one signal argument; no `[n]` delay and no clocking or gating argument |
| `covergroup` / `coverpoint` / `cross`, explicit `bins` / `ignore_bins` / `illegal_bins`, `default` bins, `iff` guards, `option.at_least`, `option.weight`, `sample()`, `get_coverage()` | Restricted | one 64-bit hit bitmap per item, so more than 64 explicit bins, a cross product above 64, and an unsized array bin above 64 are refused. Transition bins, wildcard bins, fixed-size array bins and a `binsof`/`intersect` cross-select body are refused |
| `class`, single inheritance, `virtual` methods with dynamic dispatch, constructors with implicit `super.new()`, `this` / `super`, `local` / `protected`, handle comparison | Supported | |
| Parameterized class `class C #(int W = 8)` | Restricted | value parameters only, monomorphized in the parser; a `type` class parameter is refused |
| `real`, `string`, array and array-of-handle class members; `static` / `const` / `pure` / `extern` members; `virtual class`, `pure virtual` | Refused | |
| Object lifetime | Restricted | the class heap is not garbage-collected. The live-object budget defaults to 1,000,000 and exceeding it is the fatal `F-RUN-CLASS-LIMIT` |
| `rand` members, `constraint` blocks with `soft` terms, `inside`, `dist`, `randomize()`, `randomize() with {…}` | Restricted | integral members only. Constraint bodies are boolean expressions terminated by `;` over rand fields and constants; `if`/`else`, `foreach` and `solve … before` constraint forms are refused. A general constraint operand is at most 63 bits, or exactly 64 bits signed; a pure single-field range constraint is unrestricted |
| `randc` | Restricted | cycles over its full type range, at most 16 bits wide, and no constraint may reference it |
| `constraint_mode()`, `rand_mode()`, `pre_randomize()`, `post_randomize()`, `std::randomize()` | Absent | |
| `semaphore`, `mailbox`, the `process` class, `randcase`, `randsequence`, `wait_order` | Absent | |

### System tasks and functions

Every family below is implemented; the per-name specification is
[hdl-reference/system-tasks/](hdl-reference/system-tasks/00-index.md) and the per-name status is
[manual/005](../manual/005_system-tasks.md).

| Family | Members |
|---|---|
| Display and I/O | `$display` `$write` `$monitor` `$strobe` with the `b`/`o`/`h` variants, the `$f`-prefixed forms, `$monitoron` / `$monitoroff` |
| Severity | `$info` `$warning` `$error` `$fatal`, at elaboration and at run time |
| Simulation control | `$finish` `$stop` `$exit` |
| Time | `$time` `$realtime` `$stime`; `$timeformat` with 0 or 4 arguments |
| Conversion | `$signed` `$unsigned` `$rtoi` `$itor` `$realtobits` `$bitstoreal` `$clog2` |
| Bit-vector query | `$bits` `$countones` `$onehot` `$onehot0` `$isunknown` `$countbits` |
| Real math | the 21 IEEE §20.8.2 functions, from the vendored pure-Rust libm |
| Random | `$random` (IEEE 1364 Annex N), `$urandom`, `$urandom_range`, and all seven `$dist_*` |
| Plusargs | `$test$plusargs` `$value$plusargs` |
| File I/O | `$fopen` `$fclose` `$fdisplay` `$fwrite` `$fread` `$fscanf` `$fgets` `$fgetc` `$ungetc` `$feof` `$sscanf` `$sformat` `$sformatf` |
| Memory load and dump | `$readmemb` `$readmemh` `$writememb` `$writememh` |
| Waveform dump | `$dumpfile` `$dumpvars` `$dumpon` `$dumpoff` `$dumpall` `$dumpflush` `$dumplimit` |
| Introspection | `$typename` `$cast` `$size` `$left` `$right` `$low` `$high` `$increment` `$dimensions` `$unpacked_dimensions` `$isunbounded` |
| Assertion control | `$assertoff` `$asserton` `$assertkill`, argument-free, as a global fire gate |
| Vendor observability | `$vita_stage` |

Three refusal shapes apply across the families:

- A function with a statement-level effect must be the direct right-hand side of a blocking
  assignment, so it is evaluated exactly once. This covers a seeded `$random`, every `$dist_*`, the
  function form of `$cast`, `$fopen`, `$sformatf`, `$value$plusargs`, the file-read family and a
  queue pop. Anywhere else it is refused.
- An unrecognised `$task` is a warning (`W-ELAB-FEATURE-LIMIT`) and is skipped; an unrecognised
  `$function` in an expression is refused. `$system`, `$assertcontrol`, `$printtimescale`,
  `$sdf_annotate` and `$dumpports*` fall here.
- `$fflush` is accepted and dropped with no diagnostic: file writes are unbuffered, so the warning
  would be misleading.

### Accepted with a stated simplification

Every row here is in scope, deterministic and documented. None of them is silent: where a value is
degraded, a diagnostic says so.

| Area | Behaviour |
|---|---|
| Out-of-range index or select, known index | read is all-`x`, write is dropped, and `E-RUN-RANGE` (`VITA-E4002`) is reported, rate-limited to 8 reports per kind |
| Out-of-range with an `x`/`z` index | read is all-`x`, write is a no-op, reported as `W-RUN-RANGE-UNKNOWN` on its own report budget |
| Null handle dereference | read yields `x`, write is a no-op, with `W-RUN-DYN-DEGRADE` once per net. IEEE makes this an error |
| `$stop` | ends the batch run under its own finish reason. There is no interactive mode and no breakpoint |
| `$dumpvars` | the first call fixes the file and the filter; a later call warns once and is ignored |
| Format specifiers `%u` and `%z` | consume their argument and emit nothing; `%l` is cosmetic |
| A design with no `` `timescale `` | assumes a 1ns/1ns base and says so (`W-PP-TIMESCALE-DEFAULT`) |
| Multi-word arithmetic | exact on the word grid; only `*`, `/`, `%` and `**` above 2²⁰ bits degrade to `x`, with `W-RUN-WIDE-ARITH` |
| A legal construct accepted in simplified form | `W-ELAB-FEATURE-LIMIT`, with the specifics in the message |

## Out of scope

| Item | Reason |
|---|---|
| A synthesis tool | out of scope as a product. The reference notes still record each construct's synthesizability |
| A waveform GUI viewer | VCD and FST are read in GTKWave, Surfer or another external viewer |
| Waveform formats other than VCD and FST | FSDB and the rest are not planned |
| DPI-C (`import "DPI-C"`, `export "DPI-C"`) | permanent non-goal |
| UVM and its ecosystem | permanent non-goal |
| UPF power intent, SDF timing back-annotation | permanent non-goal |
| `shortreal`, `trireg` | permanent non-goal |
| Implicit net creation | a policy decision: an undeclared name is `E-ELAB-UNRESOLVED-NAME`, not a new wire |
| In-process machine-code generation | built behind the `jit` cargo feature, measured, and rejected: it is slower than the arena backend. The feature stays off by default ([18](18-acceleration-analysis.md)) |
| A separate 2-state simulation mode, and a cycle-based mode | measured and rejected, each with its re-entry condition recorded ([20](20-cycle-mode-feasibility.md)) |
| A VHDL front end, `$dumpports*` | conditional, not scheduled: each has a recorded re-entry trigger ([05](05-strategy-and-roadmap.md)) |

## Target environments

| | |
|---|---|
| Operating systems | Linux and macOS. Windows is not a build target and is not in CI |
| Toolchain targets | `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin` |
| CI environments | `ubuntu-latest`, `macos-latest`, and a `redhat/ubi9` container on `ubuntu-latest` |
| Build philosophy | source in, `cargo build` on each platform. No prebuilt-binary distribution, and no C or C++ dependency |
| Toolchain pin | `rust-toolchain.toml` pins the channel; the MSRV is a floor with no upper bound ([02](02-implementation-language.md)) |

## What must hold

These are the conditions the test suite and CI enforce; the layer-by-layer contract is
[09](09-testing-and-verification.md).

- A representative RTL testbench differentially verified against Icarus Verilog agrees on signal
  values and transition times; Verilator agrees on the calibrated subset.
- The emitted VCD loads without error in a standard viewer and matches its golden under a normalizing
  diff that absorbs identifier-code differences.
- The same source builds and runs to the same result on every supported platform.
- Mixed `` `timescale `` modules keep one global time axis: 64-bit integer time with precision
  conversion, held by the timescale suite.
- The system-task compliance corpus passes with at least one case per family.
- A `.velab` whose upstream source has changed is refused by `vrun` with `E-ART-STALE-UPSTREAM` and
  exit 2, on a hash comparison rather than a timestamp.
- The diagnostic corpus asserts `MsgCode` values rather than message text, and every code is 1:1 with
  the catalogue under a CI gate.
- The three executors produce identical stdout, identical waveform bytes and the same finish reason
  on the equivalence corpus.
- The full local gate is `cargo nextest run --workspace --locked`: 7352 tests, 7352 passed, 15
  skipped. CI runs `cargo test --workspace --locked`, plus a product-shape job that builds and tests
  with `--no-default-features` and asserts that an absent executor is refused loudly.
