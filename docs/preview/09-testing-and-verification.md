# 09 · Testing and verification

This is the verification contract. It states the layers of testing and what each layer is
responsible for, how a differential against an external oracle is run and adjudicated, what
stands in where no external oracle exists, which gates hold the golden and determinism
guarantees, how the executors are held byte-equivalent, how mutation gives the suite teeth,
what the workload corpus does and does not gate, and the exact command set CI enforces.

The rules of method themselves — the accuracy ladder, the two adversarial review lenses,
census discipline, what makes a gate honest — are canonical in
[ENGINEERING_RULES.md](../ENGINEERING_RULES.md). This document states how those rules are
wired into runnable gates, and does not restate them.

---

## 1. The layers

| # | Layer | Instrument | Question it answers | In CI |
|---:|---|---|---|---|
| 1 | Unit | `#[test]` inside each crate's `src/` | does this function hold its invariant | yes |
| 2 | Integration / end-to-end | `crates/*/tests/*.rs`, most of them driving the real `vita` binary | does the pipeline produce the expected bytes | yes |
| 3 | External differential | `crates/sim-engine/tests/differential.rs`, plus the oracle annotation on each CLI test | does vita agree with Icarus Verilog on the same source | no — CI has no `iverilog`; this is a developer-machine gate |
| 4 | Hand-IEEE pins | literal expected values written into tests, with the reason there is no tool oracle | what does the standard require where no simulator can arbitrate | yes |
| 5 | Backend equivalence | `backend_equiv.rs`, `native_gate.rs`, `native/run_tests.rs` | do the executors produce identical bytes from the same IR | yes |
| 6 | Flip run | invert the default backend, run the whole workspace suite | is the suite still exercising more than one executor | no — a procedure, not a test |
| 7 | Codegen axis | the `jit` feature, off by default | does a third execution strategy give the same answer | no — run by hand |
| 8 | Product shape | `--no-default-features` build with one executor | is a gate refusal loud when nothing can absorb it | yes — the `build-no-oracle` job |
| 9 | Golden and determinism | schema-hash, registry, artifact, libm and thread suites | are artifacts and outputs byte-identical across OSes and runs | yes |
| 10 | Mutation | an assembled-per-task battery of single-string substitutions | would the suite notice a defect at all | no — a local procedure |
| 11 | Workload corpus | `crates/corpus-runner` over `bench/` | do ten real designs still reproduce an oracle's digest | partly — CI runs the manifest hygiene tests only |

Layers 1 and 2 ask whether vita is right about the case in front of it. Layers 3 and 4 ask
whether it agrees with the standard. Layers 5 through 8 ask whether vita agrees with itself
across its own execution strategies and build shapes. Layer 9 asks whether two runs, on two
operating systems, produce the same bytes. Layer 10 asks whether the suite would notice a
defect at all. Layer 11 asks all of these of designs nobody here wrote.

---

## 2. The suite at HEAD

The full local gate:

```bash
cargo nextest run --workspace --locked
```

7352 tests run, 7352 passed, 15 skipped, exit code 0, 35.8 s of wall clock.

### 2.1 Inventory

613 integration-test targets live directly under `crates/*/tests/`.

| Crate | `tests/*.rs` targets | `#[test]` in `tests/` | `#[test]` in `src/` | total |
|---|---:|---:|---:|---:|
| `cli` | 564 | 6333 | 25 | 6358 |
| `sim-engine` | 32 | 447 | 273 | 720 |
| `sim-ir` | 7 | 23 | 2 | 25 |
| `vita-artifact` | 3 | 10 | 0 | 10 |
| `hdl-parser` | 2 | 6 | 68 | 74 |
| `corpus-runner` | 1 | 8 | 17 | 25 |
| `diag` | 1 | 3 | 2 | 5 |
| `hdl-ast` | 1 | 2 | 1 | 3 |
| `vita-schema` | 1 | 3 | 0 | 3 |
| `vita-artifact-derive` | 1 | 3 | 0 | 3 |
| `elaborate` | 0 | 0 | 65 | 65 |
| `hdl-preprocess` | 0 | 0 | 46 | 46 |
| `vcd-writer` | 0 | 0 | 17 | 17 |
| `hdl-lexer` | 0 | 0 | 11 | 11 |
| `vita-log` | 0 | 0 | 4 | 4 |
| `hdl-builtins`, `vcd-diff` | 0 | 0 | 0 | 0 |

Every `cli` test runs the real binary through `Command::new(env!("CARGO_BIN_EXE_vita"))` and
writes its design into the system temp directory under a name carrying an atomic counter and
the process id, so the suite is safe to run in parallel. Directories inside `tests/` that
carry no `#[test]` (`cli/tests/sva_property_util/`,
`sim-engine/tests/{common,dyn_storage_util,end_to_end_util,frame_call_util}/`) are shared
helper modules, not targets.

Seventeen `cli` targets — the `*_report_gaps.rs` and `*_report.rs` families — collect gaps
reported by users of the tool. Each reported row is re-measured against a fresh probe, then
either closed or made loud; the target keeps the case so it cannot come back.

### 2.2 The 15 skipped tests

Every `#[ignore]` in the workspace is a performance probe, and no performance number is ever
a gate:

| File | `#[ignore]` | Reason string |
|---|---:|---|
| `crates/sim-engine/tests/perf_baseline.rs` | 14 | `perf baseline (DATA, not a gate); run with --ignored --nocapture` |
| `crates/cli/tests/perf_call_regime.rs` | 1 | `perf probe (DATA, not a gate); run with --ignored --nocapture` |

The three tests in `perf_call_regime.rs` that are *not* ignored are gates on the validity of
the timed row, not on its value: the two spellings of the design must compute the same
digest and that digest must not be all-X, the body holding a user call must still be
admitted, and the callee bodies must still be framed. A timed pair that has drifted apart
still produces two numbers and still divides them, which is a result shaped exactly like a
measurement.

### 2.3 Per-test hard cap

`.config/nextest.toml` is tracked and sets one thing:

```toml
[profile.default]
slow-timeout = { period = "60s", terminate-after = 4 }
```

Four minutes per test. The cap exists because without it "the suite is still running" and
"the machine is dying" are indistinguishable: a mutation that made the `$writemem*` element
loop non-terminating grew two `vita` test subprocesses to roughly 33 GB on a 32 GB machine
and panicked the kernel. The cap sits far above every real test — the workspace is about
31 s of run-phase wall clock and the slowest single test measured is about 18 s
(`sim-engine::backend_equiv runaway_codegenable_loop_equal_and_fatal`) — so a test that
reaches the cap is hung, not slow.

### 2.4 The two runners are not interchangeable

| | `cargo nextest run --workspace --locked` | `cargo test --workspace --locked` |
|---|---|---|
| Role | the local full gate | the CI-canonical suite |
| Invoked by CI | no | yes, on all three jobs that build the workspace |
| Reads `.config/nextest.toml` | yes | no |
| Per-test timeout | 60 s × 4 = 4 minutes | none |
| Wall clock at HEAD | 35.8 s (run phase) | roughly 724 s |

Switching between them costs a full rebuild each way, so one session uses one runner. The
per-test cap therefore protects local runs, which is where mutation batteries execute.

---

## 3. The command set CI enforces

The four canonical commands, which every change must pass locally and which CI re-runs:

```bash
cargo build  --workspace --locked
cargo test   --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

`--locked` is mandatory: cross-OS byte identity is only meaningful against one resolved
dependency graph.

### 3.1 The jobs

`.github/workflows/ci.yml` is the only workflow. It triggers on push to `main` and on every
pull request, and cancels superseded runs in the same group. Three job definitions produce
four runs. The toolchain action is pinned to `dtolnay/rust-toolchain@1.85.0` in all three.

| Job | Runner(s) | Steps |
|---|---|---|
| `build-native` | matrix `ubuntu-latest`, `macos-latest`, `fail-fast: false` | `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo build --workspace --locked`; `cargo test --workspace --locked` |
| `build-no-oracle` | `ubuntu-latest` | `cargo build -p cli -p sim-engine --locked --no-default-features`; `cargo clippy -p cli -p sim-engine --locked --no-default-features -- -D warnings`; `cargo test -p sim-engine --locked --no-default-features --lib`; a shell smoke test |
| `build-rhel` | `ubuntu-latest` with the `redhat/ubi9` container, `dnf install -y gcc` for a C linker | `cargo build --workspace --locked`; `cargo test --workspace --locked` |

The platform matrix is Linux and macOS: `ubuntu-latest`, `macos-latest`, and RHEL 9 / UBI in
a container. Windows appears nowhere in `.github/workflows/`.

### 3.2 The product-shape axis

`build-no-oracle` is a separate job rather than an extra step on the workspace jobs because
Cargo unifies features: if any crate in a workspace build enables `oracle`, every crate gets
it, and `--workspace` dev-dependencies pull `sim-engine` with default features for the test
targets. That is why the job names two packages instead of the workspace, why the test step
is `--lib`, and why `crates/cli/Cargo.toml` carries
`sim-engine = { path = "../sim-engine", default-features = false }`.

The job's smoke step checks the two properties that exist only in this shape:

1. it writes a design whose `initial` block prints `q=7`, runs `./target/debug/vita
   smoke.sv | tee out.txt`, and requires `grep -q 'q=7' out.txt`;
2. it runs `./target/debug/vita --backend vm smoke.sv` and **fails the job if that
   succeeds** — with no VM compiled in, the oracle spellings must be rejected loudly, never
   silently ignored or silently downgraded.

Feature detail is in [03 · Build and portability](03-build-and-portability.md).

### 3.3 What CI does not run

| Not in CI | Why | Where it runs |
|---|---|---|
| The live differential (`differential.rs`) | CI images carry no `iverilog`/`vvp`; the suite skips gracefully and still simulates each design through vita | developer machines |
| `corpus-runner run` | the corpus RTL is not in the repository and CI does not clone it | developer machines; CI runs the 25 manifest and grading tests |
| The flip run | it is a procedure over the whole suite with a source edit, not a test | before shipping a change to executor routing |
| The `jit` axis | the feature is off by default and adds a large dependency set | by hand, on the command in §6.5 |
| Line coverage | no coverage tool is configured in the repository or in CI | not measured — see §13 |

---

## 4. Differential verification against an external oracle

The differential is the strongest evidence this project has, and it outranks argument: where
a soundness reading and a measured differential conflict, the differential wins.

### 4.1 The live harness

`crates/sim-engine/tests/differential.rs` holds 27 cases. Each one:

1. Runs the design through the real front end — `hdl_lexer::lex` → `hdl_parser::parse` →
   `elaborate::elaborate_with_timescale(&su, &sink, &BTreeMap::new(), -9)` — asserting no
   lex or parse errors and no `Error`/`Fatal` diagnostics, then `simulate_capture`. All
   sidecars (`fork_modes`, `net_names`, `proc_multipliers`, `severities`, `assign_ranks`,
   `radixes`) are threaded into `SimOpts`, so a `$displayh` or `$fatal` case exercises the
   same tables production uses rather than a stripped harness path.
2. Checks that both `iverilog` and `vvp` are on `PATH` with `sh -c "command -v <tool>"`. If
   either is absent it prints `[{name}] iverilog/vvp not on PATH — differential check
   skipped` to stderr and returns; the design still runs through vita, so a vita-side crash
   is still caught.
3. Writes the source to a temp file keyed by process id and counter, compiles with
   `iverilog -g2012 -o <vvp> <sv>`, runs `vvp <vvp>`, deletes both temp files, and filters
   the lines containing `$finish called` or `$stop called` — the `vvp` runtime banner.
4. Compares `$display` stdout, trimmed at the end, and nothing else. Not the VCD, not the
   exit code.

A differential case must be IEEE-deterministic: no output may depend on an X value, because
that is the axis on which the tools legitimately disagree.

Exit codes are outside the comparison because the two tools genuinely differ there. Both
continue the simulation after `$error`; vita records the diagnostic and makes an otherwise
clean run exit 1, while `vvp` exits 0.

```
$ vita errchk.sv
before
errchk.sv:4:5: error[VITA-E4003] E-RUN-USER-ERROR: boom [in t] [at time 0]
after
simulation ended (Finish) at time 0
errors=1 warnings=1 notes=0                       # exit 1

$ iverilog -g2012 -o errchk.vvp errchk.sv && vvp errchk.vvp
before
ERROR: errchk.sv:4: boom
       Time: 0  Scope: t
after
errchk.sv:6: $finish called at 0 (1s)             # exit 0
```

The same helper (`iverilog_out`, `on_path`) is used by
`crates/sim-engine/tests/frame_call_util/mod.rs` for the frame-call suite, and
`crates/corpus-runner/src/run.rs` invokes `iverilog` to prepare a `.vvp` before its timed
rounds. Those three are the only places any code in the repository shells out to an external
simulator.

### 4.2 The oracles and their standing

| Tool | Standing | How it is used |
|---|---|---|
| Icarus Verilog 13.0 (`iverilog -g2012` + `vvp`) | the reference oracle: event-driven and 4-state, the same class of tool as vita | invoked live by `differential.rs`, by the frame-call suite, and by `corpus-runner`; named in the oracle annotation of 534 of the 564 `cli` targets |
| Verilator | a second opinion on 2-state arithmetic only | never invoked by any code in the repository. Its answers are obtained by hand and recorded in the annotation of 182 `cli` targets and in `bench/*/RUN.md` |
| Hand-IEEE | the standard text, read and pinned | §5 |

Verilator's exclusion from live comparison is structural, not incidental: it is a 2-state
compiled simulator, so on X, Z and event ordering it is not answering the same question.
Where a workload's correctness depends on uninitialised state, Verilator's digest is a
different design's answer and the workload is pinned against Icarus alone.

### 4.3 Adjudicating a divergence

A divergence is not a verdict. Classify it before acting:

| Observation | Reading | Action |
|---|---|---|
| Icarus and vita agree, Verilator differs | 2-state or settle-order quirk on Verilator's side | record it, no change |
| Verilator and vita agree, Icarus differs | Icarus quirk, or a construct Icarus models loosely | confirm against the standard text before either side moves |
| All three agree | pass | — |
| Only vita differs | a vita defect until proven otherwise | fix, with the case added to the suite |
| The two oracles disagree with each other | no oracle for this cell | §5 — decide from the standard text, and record that the tools split |

Two rules bind the classification. Internal inconsistency inside one tool proves a bug but
never says which side to move toward. And a claim that vita leads both tools needs both
tools measured — a single-oracle basis for that claim is not evidence.

The resolution of the probe is part of the result: a divergence invisible on a 10 ns grid can
be plain at 1 ns, so a case that reports agreement must state the time step it sampled.

### 4.4 Known behaviour differences

The following differences are expected and are not standard violations. They are the
calibration a differential run is read through.

**X-propagation.** Icarus treats an uninitialised signal as X and propagates it. Verilator
treats X as 0 by default. IEEE 1800 §4 defines the uninitialised value as X, so Icarus is the
conforming reference and vita follows it.

```verilog
// a flip-flop with no reset
reg [7:0] data;
initial $display("data = %h", data);
// Icarus:    data = xx
// Verilator: data = 00
// vita:      data = xx
```

Verilator's `--x-initial unique` exposes part of this difference by randomising the initial
value.

**Repeated evaluation of a combinational block.** Verilator may evaluate a combinational
block more than once at the same simulation time for scheduling reasons, so a `$display`
inside `always @(*)` or `always_comb` can print several times at one timestamp. The standard
does not fully order these evaluations, so this is not a violation. Compare the last output
at each timestamp.

```verilog
always @(*) begin
  y = a & b;
  $display("y=%b at %0t", y, $time);
  // Icarus:    one line
  // Verilator: possibly several
end
```

**High impedance.** Icarus records Z in the VCD. Verilator converts Z to 0, so Z never
appears in its output. Icarus is the conforming reference; a comparison against Verilator
needs an explicit Z-to-0 normalisation.

```verilog
wire bus;
assign bus = en ? data : 1'bz;
// Icarus:    en=0 -> bus=z
// Verilator: en=0 -> bus=0
```

Background measurements for all three are in
[history/research-log/iverilog-verilator-behaviors-2026-05-28.md](../history/research-log/iverilog-verilator-behaviors-2026-05-28.md).

### 4.5 Oracle provenance is recorded in every test

Most oracle work is not a live invocation but a pinned value, so each test's module header
names the tool and version that produced it. This is a required part of writing a test: a
pinned expectation with no recorded provenance cannot be re-audited when a tool disagrees
later.

| Annotation | Targets in `crates/cli/tests/` carrying it |
|---|---:|
| `iverilog` | 534 of 564 |
| `verilator` | 182 of 564 |

---

## 5. Where no external oracle exists

A hand-IEEE pin is an expected value derived by reading the IEEE 1364 / 1800 text, written
into the test as a literal, with a header line stating why no tool arbitrates. It is used
where the oracle *rejects the construct*, not where the oracle merely disagrees.

The rule that bounds it: with neither an oracle nor a precondition, a construct stays loud
rather than being implemented. A hand-IEEE pin is permission to implement a construct
carefully, not permission to guess.

Areas with no external oracle at HEAD, and the reason:

| Area | Why there is no oracle |
|---|---|
| SVA sequences and properties | Icarus does not support the constructs |
| Classes and inheritance, virtual dispatch | Icarus 13 runs a subset; the rest is hand-IEEE |
| Constrained random (`rand`, `constraint`, `randomize() with`, `dist`, `randc`) | not supported by the oracle |
| Parameterised classes | not supported by the oracle |
| Virtual interfaces | Icarus 13 reports a syntax error |

There is a second, weaker kind of no-oracle case: the two tools disagree with each other. A
package-scoped enum method aborts Icarus 13 inside its elaborator while Verilator reports a
missing definition; replication counts, out-of-range selects and the return value of a halted
function body are further cells where the tools split. These are recorded per construct in
[manual/006_limitations.md](../manual/006_limitations.md). An oracle that contradicts itself
on the same question — identical operands, different answer — is disqualified for that
question rather than averaged with the other.

Worked shape of a hand-IEEE header, from `crates/cli/tests/virtual_interface.rs`:

```rust
//! iverilog 13 does NOT support virtual interfaces (syntax error), so the oracle is
//! hand-IEEE. vita models a `virtual IFACE vif;` as a STATIC ALIAS: … Dynamic/conditional
//! re-binding is a v1 loud-reject (honest, never silent).
```

Mixed provenance is spelled out per case rather than per file, as in
`crates/cli/tests/round11_report_gaps.rs`:

```rust
//! Oracles: N4/N5/N5B/R4 vs iverilog 13.0 (PASS); N1/N1B/N2 are hand-IEEE (iverilog
//! rejects function output/inout/ref formals) — vita renders the IEEE-correct value.
```

---

## 6. Backend equivalence and self-consistency

vita ships three executors behind one `--backend` flag. Their construction and division of
labour are in [04 · Architecture](04-architecture.md) and
[21 · Tier-3 native backend](21-tier3-native-backend.md); what follows is the contract that
holds them equal.

| Variant | `--backend` spelling | Feature gate | Role |
|---|---|---|---|
| `Interpreter` | `interp`, `interpreter` | `oracle` | tree-walking interpreter over `SimIr` — the reference semantics, and a test instrument. Excluded from performance work by rule |
| `Bytecode` | `vm`, `bytecode` | `oracle` | bytecode VM; a body the compiler declines falls back to the interpreter, so a mixed design is normal |
| `Native` | `native` | always compiled | the default and the shipping executor |

The invariant that makes equivalence enforceable is that the shared net-write and VCD choke
point stays shared: only process-body control flow differs between executors, so stdout and
VCD bytes cannot diverge in a backend-specific way without a defect.

### 6.1 The generated corpus

`crates/sim-engine/tests/common/mod.rs` provides `corpus(seed, n)`, which cycles ten
templates round-robin and fills each from a seeded RNG: `gen_comb_chain`, `gen_counter`,
`gen_alu`, `gen_shift_register`, `gen_memory_oob`, `gen_nba_sampling`, `gen_wide_arith`,
`gen_xz_index`, `gen_multi_write_glitch`, `gen_cont_assign_mixed`. `run_capture` gives each
`(design, backend)` pair its own temp VCD path that never appears in the file body, so the
two VCDs are byte-identical exactly when the behaviour matches and no normalisation is
needed.

`crates/sim-engine/tests/corpus.rs` tests the generator itself: the same seed produces a
byte-identical corpus, different seeds produce different corpora, a corpus of 45 spans every
template with unique names, and every generated design builds and runs to `$finish` without
a fatal exit class.

### 6.2 The equivalence gate

| Gate | Where | What it compares |
|---|---|---|
| Interpreter vs VM | `crates/sim-engine/tests/backend_equiv.rs`, 28 tests, no skip | `corpus(0x5EED_F00D, 72)`: 72 designs, each elaborated once and run on both executors concurrently through `std::thread::scope`, asserting byte-identical stdout, byte-identical VCD, and an equal `SimResult` summary (`sim_time`, `finish_reason`, `exit_class`). Plus hand-written shapes the generator cannot emit, a mixed-backend run, the timescale prologue, runaway-loop fatality, and the native arithmetic, XZ-poison, signed, bitwise, mixed-width, select/concat/replicate, wide-lane and indexed-read paths |
| Anti-vacuity on that gate | same file | `gate_actually_compares_vcd_bytes` asserts the compared VCD bytes are non-trivial; `the_default_backend_is_native` pins both spellings of the default |
| VM vs native | `crates/sim-engine/src/native/run_tests.rs` | checks `runnable()` first so a refusal is counted rather than silently passed, runs the same IR on both with per-design VCD targets, and merges output and diagnostic rows through one sink. It asserts the native result really is native before comparing anything |
| Gate teeth | `crates/sim-engine/tests/native_gate.rs`, 23 tests | every reject family actually fires, the runtime gate is exactly design plus storage, every statement-effect family is wired, and the generated corpus's eligibility count is pinned exactly |

Because `backend_equiv.rs` is a plain `#[test]` with no skip, it is a hard gate on every CI
leg that builds the workspace.

The eligibility classification has three layers, each answering a different question:

| Layer | Function | Question |
|---|---|---|
| Design | `sim_engine::native::design_eligibility` | is this feature inside the native backend's scope |
| Storage | `NetArena::buildable` | can this design's values live in the arena |
| Executor | `sim_engine::native::run::executor_rows` | can the executor walk this body |

Completeness of that classification is not a test — it is the compile-time exhaustive
destructure inside `design_eligibility`. A gate that never fires is vacuous, which is why
`native_gate.rs` proves each family fires rather than only that refusals happen.

### 6.3 A fallback changes no bytes, so anchors must name the backend

A native refusal falls back to the VM in a default build. The run succeeds, the exit code
stays 0, and the swap is announced: `W-RUN-BACKEND-FALLBACK` (`VITA-W4030`) names the
requested backend, the refusing row and the executor that actually ran. It is a warning
rather than an error on purpose — byte identity across the executors is itself a gate, so a
fallback is a slower answer, not a wrong one, and making it non-zero-exit would trade
correct-support for loud, which is a rung down the accuracy ladder. In a
`--no-default-features` build the same refusal is fatal, because there the fallback target is
not compiled and the choice is loud-or-wrong rather than loud-or-correct.

`run.json` records `backend_requested` beside the effective `backend`, and `native.refused`
names the refusing layer. Because stdout and VCD bytes are identical either way, a test that
does not assert `"backend": "native"` cannot distinguish a native run from a fallback. The
observability suite asserts both fields directly (`crates/cli/tests/obs.rs`), and any new
anchor on native behaviour must do the same.

The refusal population is zero at HEAD: no source reaches the fallback path. It is written
fail-closed so that a newly added gate row reports itself without anyone remembering to, and
its teeth come from a test that corrupts a sidecar to force the refusal
(`native_gate::b4a_a_backend_fall_back_emits_a_warning_naming_the_row`).

### 6.4 The differential is blind to shared code

A native-versus-VM differential sees nothing where the two executors delegate to the same
code: as delegation grows, they move together. Measured instances exist where both executors
produced the same wrong value and agreed perfectly. The standing consequence is that any
change that delegates requires an **absolute anchor** — a concrete expected string, pinned to
Icarus wherever Icarus can run the construct — and not only an agreement test.

### 6.5 The flip run and the codegen axis

The flip run inverts the default backend and runs the whole workspace suite. It is stronger
than the generated corpus differential — several thousand real tests against 72 generated
designs — and it has found defects a green corpus differential did not. Run it in both
directions while two executors exist: inverting toward the VM asks whether the oracle still
agrees, without which the suite silently becomes native-only and the oracle stops being
tested. Invert both spellings of the default, or only half the suite moves. The expected
outcome is that the only tests changing verdict are the ones asserting which backend is the
default; anything else is a real divergence.

The `jit` feature is a third execution strategy, off by default, and asks the same question:

```bash
VITA_JIT=1 cargo nextest run --workspace --features sim-engine/jit --locked
```

Being behind a feature is not an exemption from verification. This axis is run by hand
before any change to code generation.

The product shape completes the set. With `--no-default-features` there is one executor, so
layers 5 and 6 do not apply; what the `build-no-oracle` job holds instead is the property
unique to that shape — a gate refusal has nowhere to fall back to and is therefore loud, and
an oracle backend spelling is rejected rather than accepted (§3.2).

---

## 7. Golden and determinism gates

Determinism is a product guarantee, not a testing convenience: `.velab` and `.vu` artifacts
are byte-identical across operating systems, and the staleness gate is a structural shape
hash. The mechanism is in [16 · Schema hash](16-schema-hash-spec.md) and
[17 · IR backbone freeze](17-sim-ir-ir-backbone-freeze.md); the gates that hold it are here.

| Target | What it pins |
|---|---|
| `crates/sim-ir/tests/schema_hash.rs` | the golden `SimIr` structural root hash, a Process-cluster sub-pin, and a canonical-string diff against `crates/testdata/sim_ir_canonical.txt` |
| `crates/sim-ir/tests/reflection.rs` | traces the real serde derives with `serde-reflection` and diffs the registry against `crates/testdata/sim_ir_registry.ron` |
| `crates/sim-ir/tests/frozen_shapes.rs`, `m3_shapes.rs` | frozen types render crate-root fully-qualified schema names and their exact canonical shape strings |
| `crates/sim-ir/tests/body_refs.rs` | every cross-type field is spelled `sim_ir::Foo`; a bare, `crate::`- or alias-qualified spelling is a dangling body reference and fails |
| `crates/sim-ir/tests/no_float_usize.rs` | the canonical string contains none of `usize`, `isize`, `f32`, `f64` — the platform-variant tokens |
| `crates/sim-ir/tests/no_serde_attrs.rs` | frozen types carry no serde attributes |
| `crates/hdl-ast/tests/schema_hash.rs` | the golden hash for the `.vu` root type |
| `crates/vita-schema/tests/registry.rs` | registry determinism, dedup and collision behaviour, on hand-written impls so the test does not depend on the derive |
| `crates/vita-artifact-derive/tests/render.rs` | the derive renders the canonical grammar, and body references use the spelled path while the schema name is module-path based |
| `crates/vita-artifact/tests/gate.rs` | each `.velab` header gate fires its exact `MsgCode`; a tampered `schema_hash` fails |
| `crates/vita-artifact/tests/roundtrip.rs`, `vu_roundtrip.rs` | the artifact wire format round-trips and a header-only decode preserves the body |
| `crates/diag/tests/bijection.rs` | the `MsgCode` enum and the body of [15 · Error-code reference](15-error-code-reference.md) are one to one, by `include_str!` of the document itself |
| `crates/sim-engine/tests/libm_determinism.rs` | bit-exact `f64::to_bits()` pins on the vendored libm |
| `crates/sim-engine/tests/threads.rs` | `--threads N` changes wall clock only: VCD bytes, stdout and the run summary are identical for every N |

Regenerating a golden is deliberate and explicit: `REGEN_GOLDEN=1 cargo test -p sim-ir
--test schema_hash -- --nocapture`. A regeneration that is not accompanied by an intended
shape change is a defect.

Two parser-hardening suites bound the front end's failure modes rather than its results.
`crates/hdl-parser/tests/depth_guard.rs` requires 20 000 nested parentheses to be a clean
parse error rather than a stack overflow: the recursive-descent expression path is capped at
`MAX_EXPR_DEPTH = 128` and statement nesting at `MAX_STMT_DEPTH = 256`. The cap must sit
below the smallest stack-overflow depth across every build host, not merely at a large
number — macOS debug frames are fat enough to abort at roughly 241 deep, so a cap above that
never fires there. `crates/hdl-parser/tests/node_budget.rs` requires a multi-million-element
concatenation or replication to hit `MAX_AST_NODES = 2^21` and become a bounded parse error
rather than an out-of-memory kill.

### 7.1 Float determinism

`third_party/libm` is libm 0.2.16 consumed with `default-features = false`, so no hardware
intrinsics are used and `f64` results are bit-identical on every IEEE-754 target. Its
`build.rs` performs target-cfg detection only. `libm_determinism.rs` pins exact bit patterns,
with each reference integer taken from Icarus 13.0's `$realtobits`:

| Expression | Pinned bits | Against Icarus |
|---|---:|---|
| `acos(-1.0)` | 4_614_256_656_552_045_848 | identical |
| `sin(1.0)` | 4_605_754_516_372_524_270 | identical |
| `cos(1.0)` | 4_603_041_830_072_026_764 | identical |
| `log(2.0)` | 4_604_418_534_313_441_775 | identical |
| `sqrt(2.0)` | 4_609_047_870_845_172_685 | identical |
| `atan2(1.0, 1.0)` | 4_605_249_457_297_304_856 | identical |
| `asinh(1.0)` | 4_606_113_927_061_427_239 | identical |
| `tan(1.0)` | 4_609_692_760_021_066_662 | 1 ULP apart |
| `exp(1.0)` | 4_613_303_445_314_885_482 | 1 ULP apart |
| `pow(2,10)`, `hypot(3,4)`, `floor(2.7)`, `ceil(2.1)` | 1024.0, 5.0, 2.0, 3.0 | exact |

The two 1-ULP divergences are a deliberate trade: one vendored libm on every platform, over
matching a particular platform's last bit. The gap is far below `%g`/`%f` display precision,
so `$display` output still matches Icarus; only a full-precision `$realtobits` reveals it.

---

## 8. Mutation

A green suite proves the tests pass, not that they would notice a defect. Mutation measures
the difference: break one place in production code, run the whole suite, and record whether
anything failed.

```
for case in [A, B, C, …]:
    1. restore     git checkout -- <files the case touches>
    2. substitute  replace one exact string, requiring an exact match count
    3. build+run   cargo build --tests   then   cargo nextest run --workspace
    4. verdict     a failing test name => KILLED; none => SURVIVED
```

A battery is assembled per unit of work, typically four to eleven cases. There is no
committed battery script, no `xtask`, no `scripts/` directory and no `cargo-mutants`
configuration; the cost is dominated by the workspace relink, roughly eight minutes against
about thirty seconds of test execution.

The rules that make a verdict trustworthy:

| Rule | Failure it prevents |
|---|---|
| Restore with `git checkout --`, which requires a snapshot commit first | restoring from file copies loses uncommitted edits |
| Check the exit status of the substitution and of the build; keep `SUBST-FAIL` and `BUILD-FAIL` as verdicts distinct from `SURVIVED` | a substitution matching zero places leaves an unmutated tree, which goes green and records as `SURVIVED` |
| Run `cargo nextest run --workspace` | a narrowed filter manufactures fake `SURVIVED`s and cannot manufacture a fake `KILLED`, so any survivor from a narrowed run must be re-checked at `--workspace`. `-p A -p B --test X` is not a fix: `--test` applies to every package |
| Detect `FAIL`, `TRY 1 FAIL`, `TIMEOUT`, `SIGSEGV`, `SIGABRT`, `ABORT` and `LEAK-FAIL` | a mutation that turns a design into an infinite loop is reported as `TIMEOUT`, and a parser counting only `FAIL` records it as `SURVIVED` |
| Take a mutation that can hang or run away out of the battery and run it once by hand, under the per-test cap | a runaway mutation can take the machine down with the mutated tree still in place (§2.3) |

`SURVIVED` is a question, not a result. It means one of three things and which one must be
established: a **blind axis** (build a discriminating design and kill it), an **equivalent**
mutation (measure why, and record the reason in the code), or an **unreachable** one
(re-measure with a probe showing zero hits, and leave the code fail-closed). The procedure
and its worked examples are in [study/02 §5](../study/02-v1-native-coverage.md).

---

## 9. The workload corpus

`crates/corpus-runner` runs ten real designs — eight third-party, two first-party — and
checks each against a digest an external oracle produced. It answers a question the in-repo
suite cannot: whether designs nobody here wrote still reproduce an independent tool's answer.

The contract every workload obeys:

1. **Permissive licence** — MIT, BSD-2, BSD-3, ISC or Apache-2.0 only. The RTL is never
   redistributed; `bench/*` is gitignored and `fetch` clones each workload at a pinned SHA.
2. **An oracle ran it first.** No oracle, no admission. `bench/ibex` sits on disk unadmitted
   because Icarus 13 cannot parse it.
3. **One digest line, accumulated over the whole run** — not final state, which is blind to a
   divergence the design later overwrites.
4. **Deterministic and self-terminating** — an explicit `$finish`, fixed seeds, a watchdog.
5. **The digest must move when the design changes.** Every workload has been checked by
   mutating one line of its upstream RTL. A digest that survives a mutation of its own design
   is measuring nothing and looks exactly like one that is.

Only three strings in the runner's output mean failure — `REGRESSION`, `DRIFTED` and
`ORACLE-DRIFT` — and `PROMOTED`, despite being uppercase, is not one of them. The grading
table's `Refused` → `Mismatch` cell is pinned by name in a unit test
(`loud_becoming_silently_wrong_is_a_regression`), because loud-to-silently-wrong is the one
move the accuracy ladder forbids. When one tool produces more than one digest across rounds,
the detail column is overwritten with a non-determinism marker before the grade is
considered at all: two digests from one tool is a bigger fact than whichever the last round
happened to produce.

At HEAD no row is pinned as refused, so `list` and `run` report `coverage: 10/10`. A clean
run grades nine rows `ok` and one (`verilog-axi`) `ruled-split` — a divergence the oracle
cannot arbitrate, where both digests are pinned so that vita's own answer moving is still a
`REGRESSION` and the two agreeing again grades `PROMOTED`. That state exists for an
unarbitrable divergence and for nothing else: a digest that merely fails to match is a
finding, not a split.

What the corpus gates and what it does not:

| Scope | Detail |
|---|---|
| Gates | that ten real designs still reproduce an oracle's digest; that a loud refusal has not become a silent wrong answer; that the oracle itself still reproduces its own pin |
| Does not gate | the front end. Every corpus row is at least 99% simulation time, so an elaboration regression is arithmetically invisible in its medians. Closing that needs a front-end-bound workload with a pinned digest and an oracle, which no row is |
| CI runs | the 8 manifest hygiene tests and the 17 grading unit tests — not `corpus-runner run`, because the RTL is not in the repository |

The full manifest, the invocation, the eight grades, the exit codes and the timings are in
[study/03 · The workload corpus](../study/03-workload-corpus.md); the directory contract is
in [bench/README.md](../../bench/README.md).

---

## 10. Assertion discipline

**Assert on stable message codes, never on message text.** A test that pins the wording of a
diagnostic breaks when the wording improves and, worse, passes when the wording stays and the
meaning changes. The code system is in
[13 · Diagnostics and logging](13-diagnostics-and-logging.md) and each code's cause, example
and resolution is in [15 · Error-code reference](15-error-code-reference.md). The bijection
between the `MsgCode` enum and that document is itself a test (§7), so a new code cannot be
added without documenting it and a documented code cannot be dropped from the enum.

The corollary is that a refusal's *existence* is what a test pins, not its phrasing: pinning
the text of a refusal makes removing that refusal — a promotion up the accuracy ladder —
break tests that were never about the wording.

Exit codes are part of the contract and are asserted directly:

| Constant | Value | Meaning |
|---|---:|---|
| `EXIT_OK` | 0 | success |
| `EXIT_USER_ERROR` | 1 | RTL or user error, including `$fatal`, a runtime `$error`, and a `-Werror`-promoted warning turning an otherwise-clean run |
| `EXIT_STALE` | 2 | an artifact is stale — rebuild it. Not an RTL defect, and never scored as one |
| `EXIT_CLI_ERROR` | 3 | CLI misuse |

A Rust panic exits 101. That is a vita defect and is never scored as an RTL failure: the
corpus runner classifies it as a crash, which grades `REGRESSION`. Inside exit 1, the
distinction between a compile-time and a runtime failure comes from the message code in the
always-logged summary, not from the exit code alone.

`--backend` accepts `native` always, and `interp`/`interpreter`/`vm`/`bytecode` only in an
`oracle` build. In a `--no-default-features` build those four spellings are a loud rejection
reusing `MsgCode::CliBadFlag`.

---

## 11. VCD comparison

Where both sides of a comparison are vita, VCD outputs are compared as raw bytes: the
harness gives each run a temp path that never appears in the file body, so byte identity is
exactly behavioural identity and no normalisation is involved. That is what
`backend_equiv.rs` and the thread-invariance suite do.

Comparing a VCD against another tool's needs normalisation, because the two files describe
the same behaviour in different spellings. The rules such a comparison must implement:

1. **Identifier remapping** — identifier codes are assigned per tool; match on the signal's
   hierarchical path and name instead.
2. **Header noise ignored** — `$comment` blocks, blank lines and the date stamp.
3. **Compare value triples, not event order** — normalise to `(time, signal path, value)`.
4. **Optional Z-to-0 normalisation** — required for any comparison against a 2-state tool
   (§4.4).
5. **Scope-depth relative paths** — absorb a differing top-level module name by matching on
   depth-relative paths.

**Status at HEAD: not implemented.** `crates/vcd-diff` is a one-line stub. It has no CLI, no
`[[bin]]` target and no callers, and nothing in the repository invokes it; a workflow that
needs a normalised VCD diff has no tool for it today. The rules above are the contract it
must satisfy when it is built. The live differential compares `$display` stdout instead
(§4.1), and the corpus compares a single accumulated digest line (§9).

---

## 12. External reference suites

Four public suites are useful reference points for this project's coverage. None is wired
into the build or the test suite at HEAD; admission of any design into an automated gate is
governed by the corpus contract in §9, whose second rule — no oracle, no admission — is what
keeps a suite from being adopted merely because it is large.

| Suite | What it offers | Status at HEAD |
|---|---|---|
| [CHIPS Alliance sv-tests](https://github.com/chipsalliance/sv-tests) | 1,600+ minimal cases indexed by IEEE 1800 chapter, with a published per-tool pass-rate dashboard | not wired in; a reference for locating gaps by chapter |
| [Icarus Verilog's own testsuite](https://github.com/steveicarus/iverilog/tree/master/testsuite) | the reference oracle's regression suite, heavy on IEEE 1364-2005 | not wired in |
| [Caliptra RTL](https://github.com/chipsalliance/caliptra-rtl) | a CI-driven open RISC-V core — processor-scale RTL | not wired in |
| [OpenTitan](https://github.com/lowrisc/opentitan) | a production-grade open SoC | not wired in |

---

## 13. Coverage

"Coverage" in this tree means the fraction of the test suite's `simulate()` calls the native
backend actually executed — the denominator is calls, not designs, so one test run on three
executors contributes three. It is a routing measurement, not code coverage. Its definition
and its current value are in [study/02](../study/02-v1-native-coverage.md).

Line coverage is not measured. No coverage tool is configured in the repository or in CI, and
no per-crate line-coverage threshold gates anything. The intended targets, if the measurement
is added, are:

| Crate | Line-coverage target |
|---|---|
| `hdl-lexer` | 95% |
| `hdl-parser` | 95% |
| `elaborate` | 90% |
| `sim-engine`, core paths | 90% |
| `vcd-writer` | 90% |

**Status at HEAD: intent, not enforced.** What stands in its place is the layered evidence
above — a differential against a live oracle, byte-equality between executors, golden pins on
every serialized shape, and mutation to establish that the suite would notice. Mutation
answers the question line coverage is usually asked to answer, and answers it about detection
rather than about execution.

---

## 14. Related documents

| Document | What it carries |
|---|---|
| [ENGINEERING_RULES.md](../ENGINEERING_RULES.md) | canonical method: the accuracy ladder, the two review lenses, census discipline, gate design, [testing rules](../ENGINEERING_RULES.md#7-testing) and the [performance A/B protocol](../ENGINEERING_RULES.md#8-performance-measurement) |
| [ROADMAP.md](../ROADMAP.md) | the open queues: silent-wrong residue, loud-to-supported candidates, observability |
| [03 · Build and portability](03-build-and-portability.md) | features, build shapes, toolchain, the CI matrix in full |
| [04 · Architecture](04-architecture.md) | the pipeline and the three executors |
| [13 · Diagnostics and logging](13-diagnostics-and-logging.md) | the message-code system tests assert against |
| [15 · Error-code reference](15-error-code-reference.md) | every code, kept in bijection with `MsgCode` by test |
| [16 · Schema hash](16-schema-hash-spec.md), [17 · IR backbone freeze](17-sim-ir-ir-backbone-freeze.md) | the determinism mechanism the golden gates protect |
| [21 · Tier-3 native backend](21-tier3-native-backend.md) | the native backend the equivalence gates hold to the oracles |
| [study/01](../study/01-interpreted-vs-compiled.md) | the performance axis and the A/B protocol in practice |
| [study/02](../study/02-v1-native-coverage.md) | terminology, native coverage, the flip run, the mutation procedure |
| [study/03](../study/03-workload-corpus.md) | the workload corpus in full |
| [manual/006_limitations.md](../manual/006_limitations.md) | per-construct limits, including the cells where the two oracles split |
| [CONTRIBUTING.md](../../CONTRIBUTING.md) | what a contributor must run before opening a pull request |
| [history/research-log/iverilog-verilator-behaviors-2026-05-28.md](../history/research-log/iverilog-verilator-behaviors-2026-05-28.md) | the measured tool-behaviour differences behind §4.4 |

External references: [Icarus Verilog documentation](https://steveicarus.github.io/iverilog/usage/index.html),
[Verilator documentation](https://verilator.org/guide/latest/),
[sv-tests results dashboard](https://chipsalliance.github.io/sv-tests-results/).
