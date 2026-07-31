# 004 · CLI Reference

vitamin ships four command-line entry points: **`vita`** (one-shot), and the
staged trio **`vcmp`** → **`velab`** → **`vrun`**. This chapter documents what
each one actually accepts today — its inputs, outputs, flags, and exit codes.

> Platform support: vitamin builds and tests on **Linux (Ubuntu + RHEL9/UBI)
> and macOS** (3-lane CI with byte-identical outputs). See
> [Installation](001_installation.md).

> Scope note: this reference covers the flags implemented in the current
> build — including filelists (`-f`/`-F`), preprocessor defines/includes
> (`-D`/`-I` and the `+define+`/`+incdir+` spellings), work libraries
> (`--work`/`-L`/`--top`), runtime plusargs (`+NAME[=VAL]`), threads, timeouts,
> and the diagnostic gates (`-Wno-*`/`-Werror`). Run `vita --help` (or any
> applet with `--help`) for the live list; an unrecognized `-flag` still fails
> with exit 3.

---

## One binary, four names (multicall dispatch)

In a production build all four commands are the **same binary**, dispatched on
the basename of `argv[0]`:

```
vita design.sv      # argv[0] basename "vita"  → one-shot
vcmp  design.sv     # argv[0] basename "vcmp"  → compile applet
velab design.vu     # argv[0] basename "velab" → elaborate applet
vrun  design.velab  # argv[0] basename "vrun"  → simulate applet
```

You install the trio as symlinks (or hardlinks) named `vcmp`/`velab`/`vrun`
pointing at the `vita` binary. If only `vita` is on your `PATH`, every applet is
also reachable through the **explicit subcommand form**:

```
vita vcmp  design.sv
vita velab design.vu
vita vrun  design.velab
```

`vita <sub>` consumes the `vcmp`/`velab`/`vrun` token and forwards the rest of
the arguments unchanged. Any other `argv[0]` basename (or no recognized
subcommand) runs the one-shot `vita` path.

> A developer build can emit four separate executables via the dev-only
> `separate-bins` Cargo feature (`cargo build --features separate-bins` builds
> standalone `vcmp`/`velab`/`vrun` binaries — thin shims over the same multicall
> path), for debugging a single stage in isolation. The default production
> build is the single multicall binary.

---

## Exit codes

Every command returns one of three exit codes:

| code | meaning |
|------|---------|
| `0`  | clean: parse + elaborate succeeded, simulation finished with no errors |
| `1`  | user/design error: lex/parse error, empty source (no design units), elaboration failure, a stale/corrupt artifact rejected by the staleness gate, or a runtime `$fatal` |
| `3`  | CLI/usage error: no source files given, a file that cannot be read/written, an unknown flag, or a wrong argument count |

The split is deliberate: exit **1** means *your design or artifact is wrong*;
exit **3** means *you invoked the tool wrong* (bad args, missing file). A
stale-artifact rejection (schema/format mismatch) is treated as a design/data
error — exit **1**, with a rebuild hint — not a usage error.

Diagnostics go to **stderr** as `severity[CODE]: message`, prefixed with
`file:line:col` when a source location is known. The `$display`/`$write`
transcript and the run summary go to **stdout**.

---

## `vita` — one-shot

Runs the whole pipeline in memory — preprocess → lex → parse → elaborate →
simulate → VCD — with no intermediate disk artifacts.

```
vita [-o <vcd>] <source.sv> [<source2.sv> ...]
```

**Inputs.** One or more source files. Multiple files are read and concatenated
(with a newline inserted between files so a missing trailing newline cannot fuse
tokens across the boundary), then driven through the pipeline as a single unit.
With multiple files, diagnostics currently report against the **first** file's
name.

**What it emits.**

- **stdout** — the `$display`/`$write` transcript plus the run summary.
- **VCD** — only if the design itself calls the dump system tasks
  (`$dumpfile` / `$dumpvars`). vitamin does **not** force a dump: a design that
  never calls `$dumpvars` produces no VCD (this is a no-op, not an error). The
  VCD path comes from the design's `$dumpfile(...)` argument.

**Flags** (the common set below is shared by every applet):

| flag | meaning |
|------|---------|
| `-o, --out <path>` | Override the VCD output path, ignoring the design's `$dumpfile` argument (per-applet meaning below). |
| `-f <file>` / `-F <file>` | Expand a filelist (`-f` = paths relative to the CWD, `-F` = relative to the filelist's own directory). |
| `-D, --define <N[=V]>` | Predefine a text macro (`+define+N=V+M` also accepted). |
| `-I, --incdir <dir>` | Add an `` `include `` search directory (`+incdir+a+b` also accepted). |
| `--dump-filelist` | Print the effective post-expansion input list and exit. |
| `+NAME[=VAL]` | Runtime plusarg, visible to `$test$plusargs` / `$value$plusargs`. |
| `--threads, -j <N>` | **Waveform-writer** thread budget; `N ≥ 2` moves VCD writing off the sim thread. Simulation itself is single-threaded, so this does **not** speed up a run with no waveform dump. Output stays byte-identical for any N. |
| `--backend <interp\|vm>` | Which executor runs process bodies. `interp` (default) is the reference semantics; `vm` runs suspend-free bodies on the bytecode VM. **Output is byte-identical either way** — a wall-clock knob only. See [Choosing a backend](#choosing-a-backend). |
| `--timeout <ticks>` | Stop cleanly after TICKS of simulation time (CI killswitch). |
| `-Wno-<CODE>` / `-Werror[=<CODE>]` | Suppress a warning / promote warnings to errors (doc-15 mnemonics). |
| `-q` / `-v` / `--verbosity <0..3>` | Quiet / verbose. `-v` also prints the [effective-invocation block](#what-actually-ran--v). |
| `-l, --log <file>` [`--log-append`] | Tee the full transcript (RTL + diags + progress) to a file. |
| `-h, --help` / `-V, --version` | Help / version. |

Work-library flags: `vcmp --work <NAME[=DIR]>` (+ `--workdir`), `velab -L
<NAME[=DIR]>` and `--top <UNIT>`, `vrun --upstream <FILE>`.

**Examples.**

```
vita tb.sv                       # run; VCD only if tb.sv calls $dumpvars
vita tb.sv -o waves.vcd          # run; redirect the dump to waves.vcd
vita pkg.sv dut.sv tb.sv         # concatenate three files, then run
vita -f files.f +VERBOSE +N=42   # filelist + runtime plusargs
vita -D WIDTH=16 -I rtl/inc tb.sv
```

---

## Choosing a backend

`--backend vm` runs process bodies on vitamin's bytecode VM instead of the
tree-walking interpreter. **It cannot change your results** — every output byte,
stdout and waveform alike, is identical to `interp`. That equivalence is enforced
as a hard test over the whole deterministic corpus
(`sim-engine/tests/backend_equiv.rs`), not merely intended.

What it changes is wall-clock, and only for the bodies it can claim. A body that
suspends — `#delay`, `@(event)`, `wait`, `fork` — runs on the interpreter under
either setting, so a testbench-dominated run sees little or nothing while a
datapath-dominated one sees the full effect.

Measured (`cargo test -p sim-engine --test perf_baseline -- --ignored --nocapture`,
release, best-of-3, 2026-07-31):

| workload | bodies on the VM | speedup |
|---|---|---|
| expression-heavy (wide arithmetic / logic) | 1 of 2 | **1.9×** |
| structure-heavy (select / concat / replicate) | 1 of 2 | **2.0×** |
| SHA-256 compression round | 2 of 3 | **1.5×** |
| memory-indexed (`mem[p]` per statement) | 1 of 2 | 1.4× |
| clock/scheduler-bound | 1 of 2 | 1.1× |

The `examples/` designs sit at 50–60% coverage, which is typical: roughly the DUT
half of a design qualifies and the stimulus half does not.

Writing a transform as a `function` does **not** cost you coverage — vitamin
inlines those during elaborate, so the SHA-256 round above measures the same
either way.

**When to use it.** Reach for `vm` on long expression-bound runs (crypto
datapaths, wide ALUs, vector sweeps). Leave the default alone otherwise; there is
no correctness reason to prefer either, so the only question is whether your run
is long enough for 1.5× to matter.

## What actually ran (`-v`)

Run vitamin from a Makefile or a wrapper script and the arguments you *read*
and the arguments the process *received* stop being the same text. The shell
substitutes `$(WIDTH)` before `vita` starts; the filelist expander splices `-f`
frames away; `VITA_THREADS` never appears in the command line at all. When a
nightly job fails, the log has to answer "which `W` was compiled in?" on its
own — nobody can reconstruct it afterwards.

`-v` prints that answer as the first thing in the transcript:

```
$ make sim
VITA_THREADS=4 vita -f build.f -o out.vcd +SEED=7 -l sim.log -v

invocation: vita -f build.f -o out.vcd +SEED=7 -l sim.log -v
cwd:        /work/proj
filelists:  /work/proj/build.f
sources:    /work/proj/rtl/t.sv
incdirs:    /work/proj/inc
defines:    FAST_MODE W=32
plusargs:   +SEED=7
output:     out.vcd
threads:    4 (VITA_THREADS)
log:        sim.log
env:        VITA_THREADS=4

FAST W=32
seed=7
simulation ended (Quiescent) at time 0
```

The `build.f` behind that block reads `+define+FAST_MODE+W=$(WIDTH)` and
`$(RTL_DIR)/t.sv`. The echo shows `W=32` and the real path, because those are
what the run used.

Rows are omitted when they are empty, so a plain `vita tb.sv -v` prints four
lines, not fifteen. Long lists wrap at the value column, and a flag never wraps
away from its value (`-D` and `W=32` stay on one line). The `invocation:` row is
shell-quoted, so it can be pasted back into a terminal verbatim.

| row | what it answers |
|-----|-----------------|
| `invocation:` / `cwd:` | The command as the shell delivered it, and where relative paths point. |
| `filelists:` | Every `-f`/`-F` opened, including nested ones. |
| `sources:` | The files actually compiled, post-expansion, in order. |
| `incdirs:` / `defines:` | The preprocessor surface (`+define+` and `-D` merged). |
| `plusargs:` | Runtime `+NAME=VAL` — invisible everywhere else in the log. |
| `output:` / `log:` / `obs-dir:` / `probes:` | Where output goes. |
| `tops:` / `libs:` / `work:` / `upstream:` | Elaboration roots and library wiring. |
| `timeout:` / `threads:` | Run limits, with the thread count's **provenance** (`--threads`, `VITA_THREADS`, or `auto`). |
| `env:` | Environment variables that changed this run. |

Because the block is emitted through the normal progress stream, `-l/--log`
captures it in the same file, in the same order, as the diagnostics and
`$display` output — a `--log` transcript is a complete record of the run. Every
applet echoes its own stage: `vcmp` shows the define surface, `velab` the roots
and libraries, `vrun` the plusargs.

`-v` is pure reporting. It never changes what is compiled, simulated, or
written, and it is not hashed into any artifact.

> **Note.** Flag *values* inside a filelist are taken verbatim, exactly as on
> the command line — only source positionals are resolved against the frame's
> base directory. So `--top top` in an `-F` filelist names the unit `top`, and
> `--hier-tree h.txt` writes next to the caller, not next to the `.f`.

---

## Staged flow: `vcmp` → `velab` → `vrun`

The staged flow splits the same pipeline at two disk boundaries, so you can
recompile/re-elaborate once and re-simulate many times:

```
vcmp  source.sv ...   →  source.vu      (compile:   front-end → serialized AST)
velab source.vu       →  source.velab   (elaborate: AST → sim-ir snapshot)
vrun  source.velab    →  VCD + stdout   (simulate)
```

Each stage writes a self-describing artifact that the next stage reads. The
artifacts carry a header that **gates staleness** (see below) so a snapshot built
by an incompatible tool is refused cleanly instead of silently misparsed.

### Output paths and the clobber guard

If you do not pass `-o`, each stage derives the output name by replacing **only
the last extension** of the first input (standard `Path::with_extension`):

```
vcmp  a.sv      → a.vu       vcmp a.b.sv → a.b.vu
velab a.vu      → a.velab
```

Every applet refuses to write an output that would **overwrite one of its
inputs** (e.g. `vcmp foo.vu` whose default output is also `foo.vu`, or an
explicit `-o` naming an input). That is a usage error (exit 3).

---

### `vcmp` — compile

Reads and preprocesses/lexes/parses the source(s) into a serialized design unit,
written as a `.vu` artifact.

```
vcmp [-o <out.vu>] <source.sv> [<source2.sv> ...]
```

- **Inputs:** one or more source files (read + concatenated like `vita`).
- **Output:** a single `.vu` file (magic `VU`), default `<first-source>.vu`.
- **Exit:** `0` on success · `1` on lex/parse error or empty source · `3` on a
  missing input file, a write failure, an unknown flag, or no sources given.

The `.vu` body is the serialized front-end `SourceUnit` (the parsed AST) plus a
small resolved-timescale trailer; timescale is resolved here so `velab` scales
delays identically.

```
vcmp pkg.sv dut.sv -o build/dut.vu
```

---

### `velab` — elaborate

Reads one `.vu`, checks its staleness gate, elaborates the AST into a
language-neutral **sim-ir** snapshot, and writes a `.velab` artifact.

```
velab [-o <out.velab>] <input.vu>
```

- **Input:** exactly **one** `.vu` file. Any other count is a usage error
  (exit 3).
- **Output:** a single `.velab` file (magic `VELAB`), default `<input>.velab`.
- **Exit:** `0` on success · `1` on a gate rejection (schema/format mismatch),
  an elaboration error, or a corrupt `.vu` body · `3` on a missing input,
  a write failure, an unknown flag, or the wrong argument count.

The `.velab` body is the golden `SimIr` frame followed by non-golden trailers
(fork/join modes, hierarchical net names for VCD scoping, and the timescale
multipliers). Those trailers ride **outside** the hashed `SimIr` frame, so they
do not affect the staleness hash.

```
velab build/dut.vu -o build/dut.velab
```

---

### `vrun` — simulate

Reads one `.velab`, checks its staleness gate, and runs the simulation, emitting
the VCD (if the design dumps) and the stdout transcript.

```
vrun [-o <vcd>] <input.velab>
```

- **Input:** exactly **one** `.velab` file. Any other count is a usage error
  (exit 3).
- **Output:** stdout transcript always; a VCD only if the design calls
  `$dumpfile`/`$dumpvars`.
- **Exit:** `0` on a clean finish (`$finish` / quiescent / `$stop`) · `1` on a
  gate rejection, a corrupt body, or a runtime `$fatal` · `3` on a missing
  input file or the wrong argument count.

**Flags.** The common set (filelists, `-D`/`-I`, threads, timeout, gates,
logging, plusargs) plus:

| flag | meaning |
|------|---------|
| `-o <path>` | Override the VCD output path (same semantics as `vita -o`). Rejected if it names the input `.velab`. |
| `--backend <interp\|vm>` | (`vrun` only) Same meaning as `vita --backend`. `vcmp`/`velab` **reject** it: nothing in the artifact they write depends on the backend, so accepting it would misleadingly suggest otherwise. |
| `--upstream <file>` | Verify the `.velab`'s recorded upstream digest against a specific `.vu`. |

```
vrun build/dut.velab               # simulate; VCD if the design dumps
vrun build/dut.velab -o waves.vcd  # redirect the dump
vrun build/dut.velab +VERBOSE      # runtime plusargs reach $test$plusargs
```

---

## The `-o` / `--out` flag

All applets accept `-o` (long form `--out`), which consumes the next argument as
its value. Its meaning differs by stage:

| stage | `-o` value names |
|-------|------------------|
| `vita` | the VCD output path (overrides `$dumpfile`) |
| `vcmp` | the `.vu` artifact path |
| `velab`| the `.velab` artifact path |
| `vrun` | the VCD output path (overrides `$dumpfile`) |

Where `-o` names the waveform output (`vita`/`vrun`), the file **extension
selects the format**: a path ending in `.fst` (case-insensitive) writes an FST
waveform; any other extension writes VCD. The same dispatch applies to the
design's `$dumpfile(...)` argument.

Anything not recognized as a flag is treated as a positional input path
(tokens beginning with `+` are runtime plusargs). Any other token beginning
with `-` (e.g. `--bogus`) is an **unknown flag** and fails with exit 3.

---

## Staleness gating (SchemaHash / format_version), in plain terms

Each `.vu`/`.velab` header records two compatibility stamps:

- **`format_version`** — an integer bumped whenever the on-disk artifact layout
  changes (currently **22**).
- **`schema_hash`** — a structural hash derived from the **shape** of the
  serialized types (`SourceUnit` for `.vu`, `SimIr` for `.velab`). Adding,
  removing, reordering, or retyping a field flips this hash. It is computed
  identically across Linux and macOS, so the same source yields byte-identical
  artifacts on both.

When a stage reads an upstream artifact, it compares these stamps against the
ones the **current tool** was built with:

```
velab reads a .vu   → schema_hash must match this tool's SourceUnit shape
vrun  reads a .velab → schema_hash must match this tool's SimIr shape
                       + format_version must match
```

On a mismatch (or a bad magic / undecodable header), the stage **refuses to
proceed** rather than risk simulating a stale or misparsed snapshot. It emits an
artifact error (exit 1) telling you to rebuild — re-run `vcmp`/`velab` to
regenerate the artifact with the current tool. The policy is *refuse-and-rebuild*,
not silent migration: artifacts are always cheap to regenerate from source.

> The one-shot `vita` path never serializes anything, so it has no staleness to
> check — there is no on-disk artifact that could go stale.

---

## See also

- [Installation](001_installation.md) — building and putting the applets on your `PATH`.
- `docs/preview/14-staged-artifacts.md` — the authoritative artifact formats, hash-binding rules, and the full (planned) CLI flag surface.
- `docs/preview/13-diagnostics-and-logging.md` — diagnostics, message codes, and logging.
