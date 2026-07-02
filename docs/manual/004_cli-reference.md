# 004 · CLI Reference

vitamin ships four command-line entry points: **`vita`** (one-shot), and the
staged trio **`vcmp`** → **`velab`** → **`vrun`**. This chapter documents what
each one actually accepts today — its inputs, outputs, flags, and exit codes.

> Platform support: vitamin builds and tests on **Linux, macOS, and Windows**
> (3-OS CI with byte-identical outputs). See [Installation](001_installation.md).

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
> `separate-bins` Cargo feature, for debugging a single stage in isolation. The
> default production build is the single multicall binary.

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
| `--threads, -j <N>` | Worker threads — output stays byte-identical for any N. |
| `--timeout <ticks>` | Stop cleanly after TICKS of simulation time (CI killswitch). |
| `-Wno-<CODE>` / `-Werror[=<CODE>]` | Suppress a warning / promote warnings to errors (doc-15 mnemonics). |
| `-q` / `-v` / `--verbosity <0..3>` | Quiet / verbose terminal output. |
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

Anything not recognized as a flag is treated as a positional input path
(tokens beginning with `+` are runtime plusargs). Any other token beginning
with `-` (e.g. `--bogus`) is an **unknown flag** and fails with exit 3.

---

## Staleness gating (SchemaHash / format_version), in plain terms

Each `.vu`/`.velab` header records two compatibility stamps:

- **`format_version`** — an integer bumped whenever the on-disk artifact layout
  changes (currently **3**).
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
