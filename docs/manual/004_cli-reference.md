# 004 · CLI Reference

This chapter is the complete reference for the vitamin command line: the entry points, every flag
of every entry point, filelists, plusargs, environment variables, exit codes, diagnostic control,
backend selection, the staged flow and its artifacts, waveform naming, and the observability rail.
It describes the build in this tree. Platforms are Linux and macOS.

---

## 1. Entry points

vitamin exposes four entry points. They are one program: a single binary that selects an applet
from `argv[0]`.

| entry point | stage | input | primary output |
|---|---|---|---|
| `vita` | the whole pipeline, in memory | one or more source files | RTL transcript on stdout, waveform if the design dumps |
| `vcmp` | compile: preprocess + lex + parse | one or more source files | a `.vu` artifact, or a work-library blob |
| `velab` | elaborate | exactly one `.vu`, or `-L` libraries | a `.velab` artifact |
| `vrun` | simulate | exactly one `.velab` | RTL transcript on stdout, waveform if the design dumps |

### 1.1 Three ways to reach a stage

The applet is chosen from the **file stem** of `argv[0]` — the file name with its final extension
removed, so `vcmp.exe` selects `vcmp` while `vita-0.2` matches nothing and selects one-shot `vita`.
If the stem is not one of the three staged names, the **first** argument is examined: a token that
is exactly `vcmp`, `velab` or `vrun` is consumed and selects that applet.

```
vcmp design.sv          # stem "vcmp"  — a symlink or copy named vcmp
vita vcmp design.sv     # subcommand form, identical behaviour
```

1. **Symlink dispatch.** `install.sh` runs `cargo install --path crates/cli --locked`, which
   installs `vita` alone, then creates `vcmp`, `velab` and `vrun` in the same directory as symlinks
   to it (`ln -sf`), falling back to copies on a filesystem without links.
2. **Subcommand form.** `vita <sub> [args…]` reaches the same applet with the same arguments. The
   subcommand token must be first.
3. **Separate binaries.** The `separate-bins` Cargo feature builds standalone `vcmp`, `velab` and
   `vrun` executables. Each is a shim whose whole body calls the same driver, so the code path is
   identical to the multicall binary. The feature is off by default and exists for building one
   stage in isolation.

### 1.2 Cargo features that change the CLI surface

| feature | default | effect on the CLI |
|---|---|---|
| `oracle` | on | compiles the `interp` and `vm` executors in. Without it, `--backend interp` and `--backend vm` are usage errors and `native` has no fallback target |
| `separate-bins` | off | emits the standalone `vcmp`/`velab`/`vrun` binaries |
| `jit` | off | forwards the cranelift code-generation experiment. No CLI flag; armed by the `VITA_JIT` environment variable |

### 1.3 What happens before any flag is parsed

`run()` processes an invocation in this order:

1. Applet resolution, as above.
2. `--help` or `-h` **anywhere** in the arguments prints help and exits 0.
3. `--version` or `-V` anywhere prints `<applet> 0.2.0` and exits 0. `-V` is version; `-v` is
   verbosity. They are different flags.
4. Filelist expansion (`-f` / `-F`), at argument level, for every applet.
5. Applet dispatch. For one-shot `vita`, a first argument of `explain` routes to the diagnostic
   explainer (§12).

Everything runs on a worker thread with a 256 MiB stack, so a deep-recursion depth cap reports a
diagnostic instead of overflowing the stack. A panic in that thread exits 101.

---

## 2. Exit codes

| code | meaning |
|---|---|
| `0` | clean. Also `--help`, `--version`, `vita explain <CODE>`, and `--dump-filelist` |
| `1` | user or design error: preprocess/lex/parse error, no design units in the source, elaboration failure, runtime `$fatal` or `$error`, delta-limit non-convergence, and a `-Werror`-promoted warning on an otherwise clean run |
| `2` | artifact staleness: bad magic, `format_version` mismatch, tool major-version mismatch, `schema_hash` mismatch, an undecodable artifact body or trailer, a live upstream digest mismatch, or an invalid work-library manifest |
| `3` | usage error: unknown flag, missing or bad flag value, no source files, wrong positional count, unreadable input, unwritable output or log, an output that would overwrite an input, a flag passed to the wrong stage, a filelist error, an unknown diagnostic code |
| `101` | a panic inside the tool |
| `141` | terminated by `SIGPIPE` (`vita d.sv \| head`). The default disposition is restored deliberately, so this is a clean pipe close, not a crash |

The split between 1 and 2 is what lets a CI job react correctly: **1** means the design or the run
is wrong, **2** means an artifact must be regenerated before the design can be judged at all, and
**3** means the command line is wrong.

A run exits 0 only when the engine's exit class is clean **and** the finish reason is `$finish`,
`$stop`, or quiescence; a delta-limit or error finish exits 1. A clean engine result is still
turned into exit 1 when the diagnostic sink recorded an error or fatal, which is how a
`-Werror`-promoted warning fails a run.

---

## 3. Flag reference

One parser serves all four applets. In the table below, **yes** means parsed and acted on,
**rejected** means parsed and refused with a message naming the applet to use instead (exit 3), and
**dropped** means parsed, accepted, and silently not used.

| flag | argument | default | `vita` | `vcmp` | `velab` | `vrun` | effect |
|---|---|---|---|---|---|---|---|
| `-o`, `--out` | next token | per applet (§9.1, §15.1) | yes | yes | yes | yes | `vita`/`vrun`: waveform path, overriding `$dumpfile`. `vcmp`: the `.vu` path. `velab`: the `.velab` path |
| `-f` | next token | — | yes | yes | yes | yes | expand a filelist; paths inside resolve against the invocation directory |
| `-F` | next token | — | yes | yes | yes | yes | expand a filelist; paths inside resolve against the filelist's own directory |
| `--dump-filelist` | none | off | yes | yes | yes | yes | print the effective post-expansion inputs and exit 0 without compiling |
| `-D`, `--define` | `NAME` or `NAME=VAL`, repeatable | — | yes | yes | rejected | rejected | predefine an object-like text macro. Without `=`, the value is empty and only definedness is set |
| `-I`, `--incdir` | directory, repeatable | — | yes | yes | rejected | rejected | add an `` `include `` search directory, tried in order after the current file's directory |
| `-G`, `--param` | `NAME=VALUE`, repeatable | — | yes | rejected | yes | rejected | override a top-module parameter before elaboration (§6) |
| `--top` | unit name, repeatable | auto-top | yes | rejected | yes | rejected | pin the elaboration root. Required at least once in `velab -L` library mode |
| `--work` | `NAME` or `NAME=DIR` | — | rejected | yes | rejected | rejected | record the compiled unit into a work library |
| `--workdir` | directory | — | rejected | yes | rejected | rejected | library directory when `--work` carries no `=DIR`; alone it implies the library name `work` |
| `-L` | `NAME` or `NAME=DIR`, repeatable | — | rejected | rejected | yes | rejected | bind a compiled library. Search order is `-L` order |
| `--upstream` | path to a `.vu` | — | dropped | dropped | dropped | yes | re-hash that file and compare it with the digest recorded in the `.velab` |
| `+NAME[=VAL]` | attached, repeatable | — | yes | rejected | rejected | yes | runtime plusarg (§5) |
| `--backend` | `native`, `interp`, `interpreter`, `vm`, `bytecode` | `native` | yes | rejected | rejected | yes | select the process-body executor (§13) |
| `--threads`, `-j` | integer | `VITA_THREADS`, else `min(available parallelism, 8)` | yes | dropped | dropped | yes | waveform-writer budget. A value of 2 or more moves VCD writing to its own thread; the value is floored at 1 |
| `--timeout` | integer ticks | unbounded | yes | dropped | dropped | yes | end the run cleanly once simulation time reaches the cap. The finish reason is quiescent, so the exit code stays 0 |
| `--obs-dir` | non-empty directory | — | yes | rejected | rejected | rejected | write the machine-readable run report (§14) |
| `--obs-procs` | none | off | yes | rejected | rejected | rejected | add the per-body, per-builtin and per-subroutine count objects. Requires `--obs-dir` |
| `--obs-procs-time` | none | off | yes | rejected | rejected | rejected | implies `--obs-procs` and adds wall-clock fields |
| `--probe` | net path, repeatable | — | yes | rejected | rejected | rejected | record every value change of one net to `trace.jsonl`. Requires `--obs-dir` |
| `--probe-file` | file path | — | yes | rejected | rejected | rejected | read probe paths from a file, merged with `--probe` |
| `--hier-tree` | non-empty path | — | yes | dropped | dropped | dropped | write the instance tree after elaboration |
| `--inst-paths` | non-empty path | — | yes | dropped | dropped | dropped | write the full dotted instance-path list after elaboration |
| `-Wno-<CODE>` | attached | — | yes | yes | yes | yes | suppress that diagnostic (§7) |
| `-Werror`, `-Werror=all`, `-Werror=<CODE>` | attached | — | yes | yes | yes | yes | promote warnings to errors (§7) |
| `-q`, `--quiet` | none | verbosity 1 | yes | yes | yes | yes | verbosity 0: suppress the terminal copy of progress and RTL output. Diagnostics and the `--log` copy are unaffected |
| `-v` | none | — | yes | yes | yes | yes | verbosity 2: print the resolved-invocation echo (§11) |
| `-vv` | none | — | yes | yes | yes | yes | verbosity 3. A reserved level that renders as level 2 |
| `--verbosity` | integer `0..=3` | 1 | yes | yes | yes | yes | numeric form of `-q`/`-v`/`-vv`. Last wins, with no override warning |
| `-l`, `--log` | path, or `-` for stderr | — | yes | yes | yes | yes | tee every sink event to one writer in emission order. Truncates by default |
| `--log-append` | none | truncate | yes | yes | yes | yes | append to the `--log` file instead of truncating. Ignored for `-` |
| `-h`, `--help` | none | — | yes | yes | yes | yes | print help and exit 0 |
| `-V`, `--version` | none | — | yes | yes | yes | yes | print `<applet> 0.2.0` and exit 0 |

### 3.1 Attached-value spellings

These forms are recognized for compatibility with other simulators. They are tried after every
exact-match flag and before the unknown-flag catch-all.

| spelling | meaning |
|---|---|
| `-D<NAME>[=<VAL>]` | attached define |
| `-I<dir>` | attached include directory |
| `-G<NAME>=<VALUE>` | attached parameter override; a missing `=` is a usage error |
| `+define+N=V+M[=…]` | `+`-joined defines; each non-empty segment splits on its first `=` |
| `+incdir+a+b` | `+`-joined include directories |
| `+<anything>` | a runtime plusarg, with the leading `+` stripped |

There is no attached form for `-o`, `-j`, `-l`, or `-L`. `-Lfoo` falls through to the catch-all and
reports `unknown flag '-Lfoo'`.

### 3.2 Positionals and unknown tokens

* A token beginning with `-` and longer than one character that matches no flag is offered to the
  diagnostic-gate parser (`-Wno-`, `-Werror`); if that declines too, it is
  `error[VITA-E0001]: unknown flag '<tok>'`, exit 3.
* A bare `-` is a positional, not a flag.
* A bare `+` is a positional, not a plusarg.
* Everything else is a positional input path, in command order.

There is no `--` end-of-flags separator, no response-file syntax other than `-f`/`-F`, and no
interactive mode.

### 3.3 Setting a single-value knob twice

`-o`, `--threads`/`-j`, `--backend`, `--timeout`, `--upstream`, `--work`, `--workdir`, `-l`/`--log`
and `--obs-dir` hold one value. Setting one twice keeps the last value and records a warning:

```
warning[VITA-W8009] W-FLIST-OVERRIDE: -o 'a.vcd' overridden by 'b.vcd' (last wins)
```

These events are replayed through the diagnostic gate at the start of the run, so `-Wno-` and
`-Werror` apply to them and they appear in the counts epilogue. The accumulating flags — `-L`,
`--top`, `-G`, `-D`, `-I`, `--probe` — and every verbosity flag record nothing, because repeating
them is how they are used.

### 3.4 Flags a stage refuses, and what it says

Each rejection names the applet that owns the flag, so the message is the fix. All are exit 3.

| flags | refused by | message |
|---|---|---|
| `-D`, `-I`, `+define+`, `+incdir+` | `velab`, `vrun` | `+define+/+incdir+/-D/-I are compile-stage (vcmp/vita) inputs — '<stage>' has no preprocess pass, so they would be silently meaningless` (`VITA-E8007`) |
| `+PLUSARG` | `vcmp`, `velab` | `runtime plusargs (+<first>) are vita/vrun arguments — '<stage>' compiles, it does not simulate` |
| `--backend` | `vcmp`, `velab` | `'--backend <name>' is a simulate-side argument — '<stage>' does not run process bodies` |
| `-G`, `--param` | `vcmp`, `vrun` | `'-G <n>=<v>' is an elaborate-stage argument — '<stage>' cannot apply a parameter override` |
| `--obs-dir` | `vcmp`, `velab`, `vrun` | `'--obs-dir <dir>' is a one-shot vita argument — '<stage>' does not emit the obs rail` |
| `--probe`, `--probe-file` | `vcmp`, `velab`, `vrun` | `'--probe'/'--probe-file' is a one-shot vita argument — '<stage>' does not emit the trace rail` |
| `--obs-procs`, `--obs-procs-time` | `vcmp`, `velab`, `vrun` | `'--obs-procs'/'--obs-procs-time' is a one-shot vita argument — '<stage>' does not emit run.json` |
| `--work`, `--workdir` | `vita`, `velab`, `vrun` | `--work/--workdir are vcmp flags — '<stage>' does not write libraries` |
| `-L` | `vita`, `vcmp`, `vrun` | `-L is a velab flag — '<stage>' does not read libraries` |
| `--top` | `vcmp`, `vrun` | `--top selects an elaboration root — '<stage>' has no root selection` |
| a design that calls `$vita_stage` | `velab` | `` `$vita_stage` is a one-shot `vita` task — `velab` does not stage it `` |

Four flags are accepted without effect rather than refused: `--upstream` on `vita`, `--threads`
and `--timeout` on `vcmp` and `velab`, and `--hier-tree`/`--inst-paths` on all three staged
applets. `vcmp --work` without `-o` also drops the plain `.vu` output and writes only the library
blob.

---

## 4. Filelists (`-f`, `-F`)

A filelist is a text file holding the arguments a command line would otherwise carry. Expansion
happens once, at argument level, before any per-applet parsing, so every flag legal on the command
line is legal inside a filelist.

| form | relative paths inside the file resolve against |
|---|---|
| `-f FILE` | the invocation directory |
| `-F FILE` | the directory holding that filelist, which makes a vendor tree relocatable |

The base is a property of how a frame was entered, not something a nested frame inherits. A `-f` or
`-F` target written on the command line always resolves against the invocation directory; a nested
target resolves against the enclosing frame's base. Splicing is in place and depth-first, with the
command line as the outermost frame.

### 4.1 Lexing

1. `/* … */` block comments are removed first. They do not nest, an unterminated one swallows the
   rest of the file, and each is replaced by a single space so it separates tokens instead of
   joining them.
2. Everything from `//` to end of line is removed.
3. A line whose first non-blank character is `#` is dropped.
4. A line ending in `\` is joined to the next with a space.
5. What remains is split on whitespace.

### 4.2 Environment expansion

`$NAME`, `${NAME}` and `$(NAME)` are expanded in every token, in `-f`/`-F` targets, and in the
value of a value-taking flag. A lone `$` with no identifier body stays verbatim, which keeps
escaped identifiers intact. An unterminated `${` or `$(` is an error. An **undefined** variable is a
hard error, never an empty substitution:

```
error[VITA-E8006] E-FLIST-UNDEF-ENV: undefined environment variable '$NOPE_VAR_X'
```

### 4.3 What a token becomes

| token | handling |
|---|---|
| `-f`, `-F` | recurse into that filelist |
| a value-taking flag | the flag and its next token are passed through verbatim after environment expansion. A flag **value** is never path-resolved |
| `+define+…` | passed through verbatim; the segments are macro text, not paths |
| `+incdir+a+b` | each `+`-joined segment is resolved against the frame base and re-joined |
| any other `-`-prefixed token | passed through verbatim |
| anything else | a source path: checked for wildcards, then resolved against the frame base |

Because flag values are verbatim, `--top top` inside a filelist names the unit `top`, and
`--hier-tree h.txt` inside a filelist writes next to the caller, not next to the filelist.

### 4.4 Guards

| condition | code | severity | detail |
|---|---|---|---|
| a filelist includes itself, directly or through a chain | `VITA-E8001` | Error | identity is the lexical path together with the physical file identity, so two names for one file are caught |
| nesting deeper than 256 levels | `VITA-E8002` | Error | — |
| the same source included twice under different sticky `` `timescale `` context | `VITA-E8003` | Error | see below |
| a token containing `*`, `?` or `[` | `VITA-E8004` | Error | `wildcard '<tok>' not allowed in a filelist` |
| a filelist or source that cannot be read | `VITA-E8005` | Error | — |
| an undefined environment variable | `VITA-E8006` | Error | §4.2 |
| a compile-stage flag in a filelist fed to `velab`/`vrun` | `VITA-E8007` | Error | §3.4 |
| `-f` used inside a `-F` frame | `VITA-W8008` | Warning | the target resolves against the invocation directory, not the filelist directory |
| a single-value knob set twice | `VITA-W8009` | Warning | §3.3 |

Filelist diagnostics are emitted before the diagnostic gate exists, so `-Wno-` and `-Werror` do not
apply to them and a run that dies during expansion prints no counts epilogue.

### 4.5 Duplicate sources

After expansion, the same source appearing twice is reduced to its first occurrence. Flags and
flag values are exempt; only bare positionals are deduplicated. If any duplicate existed, the
pre-deduplication stream is re-walked to compare the sticky `` `timescale `` each occurrence would
have inherited. Differing context is a hard error:

```
error[VITA-E8003] E-FLIST-DUP-CTX-CONFLICT: b.sv included twice under differing sticky context:
first (a.f) inherits `1ns/1ps`, duplicate inherits `(base 1ns/1ns)`
```

---

## 5. Runtime plusargs

Any token beginning with `+` and longer than one character, other than `+define+…` and
`+incdir+…`, is a runtime plusarg. The leading `+` is stripped and command-line order is kept.
`vita` and `vrun` accept them; `vcmp` and `velab` refuse them. Plusargs are never hashed into an
artifact.

| construct | rule |
|---|---|
| `$test$plusargs("q")` | returns 1 when some plusarg **starts with** `q`. The argument must be a string literal; a non-literal yields an unknown value |
| `$value$plusargs("prefix%C", var)` | takes the first plusarg starting with `prefix` and converts the remainder: `%d` decimal, `%h`/`%x` hex, `%o` octal, `%b` binary, `%s` raw bytes packed MSB first. A miss returns 0 and leaves `var` untouched. A format with no `%` is a pure prefix probe and writes nothing. An unconvertible value writes all-x and warns (`VITA-W4028`) with status still 1. Conversion is width-aware above 64 bits |
| `+STAGE_TRACE` | arms `$vita_stage` capture (§14.8). It matches exactly `STAGE_TRACE` or a `STAGE_TRACE=<val>` prefix, and not `STAGE_TRACEX` |

Plusargs appear in the run manifest's `plusargs` array with the leading `+` already stripped.

---

## 6. Parameter overrides (`-G`, `--param`)

`-G NAME=VALUE` overrides a `parameter` of every elaboration root, before elaboration. The command
line splits on the first `=` only. `VALUE` is not a general expression; it is one of four forms:

| form | handling |
|---|---|
| a decimal integer, e.g. `9` | signed 64-bit |
| a sized literal, e.g. `8'hFF` | keeps its own width and signedness, and may exceed 64 bits |
| a quoted string, e.g. `"fast"` | the quotes are stripped by the command-line parser; the result is a string literal |
| an unsized fill literal `'0` `'1` `'x` `'z` | re-folded at the target's declared width. `'x` and `'z` are refused at bind time |

Anything else — any operator form such as `~8'h5A` — is refused:

```
`-G W=~8'h5A`: the value must be a decimal integer, a sized literal like 8'hFF, or a quoted string
```

Two more refusals, both `VITA-E3002`: the target is a `localparam`, which cannot be overridden; and
the name matches no parameter of any top, reported once after all roots are examined. A missing `=`
is a usage error: `error[VITA-E0001]: '-G WWW' needs NAME=VALUE`.

```
vita tb.sv -G W=9 -G 'NAME="fast"' -GDEPTH=8'hFF
```

---

## 7. Diagnostic control

| spelling | effect |
|---|---|
| `-Wno-<CODE>` | suppress that diagnostic when its severity is Warning, Info or Note |
| `-Werror` | promote every warning to an error |
| `-Werror=all` | identical to bare `-Werror`; `all` is case-insensitive |
| `-Werror=<CODE>` | promote just that code |

A `<CODE>` is accepted in three spellings, case-insensitively:

| spelling | example |
|---|---|
| the mnemonic | `W-ELAB-FEATURE-LIMIT` |
| the printed number | `VITA-W3056` |
| that number bare | `W3056` |

Every diagnostic prints the first two, so either can be copied out of a transcript. An unknown code
is a usage error, never a silent no-op:

```
error[VITA-E0001]: unknown diagnostic code 'W-NOPE' in '-Wno-' — a code is its mnemonic
(`W-ELAB-FEATURE-LIMIT`), its printed number (`VITA-W3056`), or that number bare (`W3056`);
`vita explain <CODE>` describes one
```

Gate semantics:

* Error and Fatal are the always-logged spine. They are never suppressed and never altered. A
  `-Wno-` naming an error code is accepted and does nothing.
* A Warning is suppressed when listed, otherwise promoted to Error when `-Werror` or a matching
  `-Werror=<CODE>` is given. A promoted diagnostic keeps its own code number; only the severity
  changes.
* Info and Note are suppressible and never promoted.

Because the counts are taken after the gate, a promoted warning drives the exit code to 1, and
`vcmp` and `velab` additionally refuse to write their artifact. There are 68 diagnostic codes; the
catalogue is [007 · Error codes](007_error-codes.md) and
[../preview/15-error-code-reference.md](../preview/15-error-code-reference.md).

---

## 8. Output streams and message formats

| event | stream | hidden by `-q` |
|---|---|---|
| diagnostics | stderr | never |
| progress, including the `-v` echo and the run-summary line | stdout | terminal copy only |
| the `$display`/`$write` transcript | stdout | terminal copy only |

`-l FILE` tees every event to one writer in emission order, so the terminal copy and the file copy
cannot drift; `-q` affects only the terminal copy. `-l -` writes that tee to stderr.

Diagnostics rendered by the sink carry a mnemonic and, when known, a location:

```
[<file>:<line>:<col>: ]<severity>[<CODE>] <MNEMONIC>: <message>[ [in <inst path>]][ [at time <ticks>]]
f2.sv:2:13: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected '(' before port connections, found ';'
```

Command-line errors — flag parsing, wrong-stage rejections, file I/O, probe resolution — take a
shorter form written straight to stderr. This form carries no mnemonic, is not filtered by `-Wno-`
or `-Werror`, and is not counted:

```
error[VITA-E0001]: unknown flag '--bogus'
```

Every pipeline run ends with a counts line on stderr, teed to `--log` like everything else:

```
errors=0 warnings=1 notes=0
```

`errors` is the sum of Error and Fatal; `notes` is the sum of Info and Note. The line is not printed
for `--help`, `--version`, `explain`, `--dump-filelist`, a filelist-expansion failure, or a
flag-parse failure.

The engine's own closing line is a progress event on stdout:

```
simulation ended (Finish) at time 10
```

The reason in parentheses is one of `Finish`, `Stop`, `Quiescent`, `DeltaLimit`, `Error`, and the
time is in precision units. Writes to stdout and stderr use a form that drops a broken-pipe error
rather than panicking, which is what makes exit 141 clean.

---

## 9. `vita` — one-shot

```
vita [-o <out.vcd|out.fst>] <source.sv> [<source2.sv> …]
```

Runs preprocess, lex, parse, elaborate, simulate and waveform writing in memory, with no
intermediate artifact. At least one source file is required; otherwise
`error[VITA-E0001]: no source files given`, exit 3.

Multiple files are compiled as one unit. Each file keeps its own identity, so a diagnostic reports
the file it came from with that file's own line and column.

```
vita tb.sv                        # run; waveform only if tb.sv calls $dumpvars
vita tb.sv -o waves.vcd           # redirect the dump
vita tb.sv -o waves.fst           # write FST instead
vita pkg.sv dut.sv tb.sv          # three files, one unit
vita -f files.f +VERBOSE +N=42    # filelist and runtime plusargs
vita -D WIDTH=16 -I rtl/inc tb.sv
```

### 9.1 Waveform output and the extension rule

`-o` sets an override; the waveform path is resolved inside `$dumpvars`:

```
path = the -o value, else the $dumpfile argument, else "dump.vcd"
```

Consequences:

* A design that never calls `$dumpvars` produces **no waveform file at all**, even with `-o`. This
  is a no-op, not an error.
* `$dumpvars` with neither `$dumpfile` nor `-o` writes `dump.vcd` in the working directory.
* `-o` beats `$dumpfile`.
* **The extension selects the format.** A path whose lowercased form ends in `.fst` writes FST for
  GTKWave and Surfer; any other extension writes VCD. The rule applies equally to a `-o` path and
  to the design's own `$dumpfile` argument.

FST is produced by writing a VCD sidecar at `<path>.vcdtmp` during the run and transcoding it at
the end, then deleting the sidecar. A transcode failure warns (`VITA-W4019`) and leaves the sidecar
in place. An FST target forces the sidecar writer single-threaded regardless of `--threads`. A
failure to create the dump file warns (`VITA-W4018`) and the run continues without a waveform. A
second `$dumpvars` call warns once (`VITA-W4021`) and does nothing; the first call fixes both the
file and the filter.

The VCD `$timescale` unit string is derived from the resolved design-wide precision and clamped to
the range VCD can express, from `1fs` to `100s`. FST metadata carries the version string
`vitamin-sim 0.2.0`.

---

## 10. `--dump-filelist`

Checked first in every applet, so it short-circuits every stage-specific rejection. It prints the
effective inputs to stdout and exits 0 without compiling:

```
$ vita --dump-filelist t.sv -D A -D B=2 -I inc
source t.sv
define A
define B=2
incdir inc
```

Order is: sources in command order, then value-less defines and defines with values, then include
directories. Nothing is sorted or resolved beyond what expansion already did. Override warnings are
replayed first; no counts epilogue is printed, and `--log` is not honoured here.

---

## 11. Resolved-invocation echo (`-v`)

At verbosity 2 or higher, the run opens with a block reporting what the process actually received —
after shell substitution, filelist expansion and environment lookup. It is emitted as ordinary
progress events, so `--log` captures it in the same file and the same order as the diagnostics and
the RTL transcript, and `-q` hides only the terminal copy. It is pure reporting and is never hashed
into an artifact.

```
$ VITA_THREADS=4 vita -f build.f -o out.vcd +SEED=7 -l sim.log -v

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

| row | contents |
|---|---|
| `invocation:` | `argv[0]` by file name, then the arguments verbatim, shell-quoted, with each value-taking flag glued to its value as one atom so a flag never wraps away from its value |
| `cwd:` | the process working directory |
| `filelists:` | every `-f`/`-F` opened, canonical, in depth-first order |
| `sources:` | post-expansion inputs in command order — source files for `vita`/`vcmp`, the `.vu` or `.velab` for the later stages, empty in `velab -L` mode |
| `work:` / `libs:` / `upstream:` | applet-specific: the `vcmp` work library, the `velab -L` libraries, the `vrun --upstream` file |
| `incdirs:` / `defines:` | the preprocessor surface, with `-I`/`+incdir+` and `-D`/`+define+` merged |
| `plusargs:` | runtime plusargs, re-prefixed with `+` |
| `params:` | `-G` overrides as `NAME=VALUE` |
| `tops:` | `--top` values |
| `output:` | the `-o` value or the resolved artifact path |
| `obs-dir:` / `obs-procs:` / `probes:` | the observability rail. `obs-procs:` reads `counts (--obs-procs)` or `counts+time (--obs-procs-time)`; `probes:` lists `--probe` values only, not lines merged from `--probe-file` |
| `timeout:` | `<N> ticks` |
| `threads:` | `<N> (<source>)`, where source is `--threads`, `VITA_THREADS` or `auto` |
| `log:` | the `--log` path |
| `env:` | `NAME=value` for each of `VITA_THREADS` and `VITA_SVA_COLLAPSE` that is set |

A row with no values prints nothing, so a plain `vita tb.sv -v` is a few lines rather than fifteen.
Long lists wrap at the value column and continuation lines are indented to it; a single value wider
than the margin is never split. `--hier-tree` and `--inst-paths` have no row. Override warnings are
emitted before the block.

---

## 12. `vita explain <CODE>`

One-shot `vita` only, recognized when the first argument is exactly `explain`. It prints the
reference entry for one diagnostic code, in any of the three spellings of §7, and exits 0.

```
vita explain W3056
vita explain VITA-W3056
vita explain W-ELAB-FEATURE-LIMIT
```

The entry text is compiled into the binary from
[../preview/15-error-code-reference.md](../preview/15-error-code-reference.md), so it needs no
installed documentation. A missing argument or an unresolvable code is a usage error, exit 3.

---

## 13. Backend selection

`native` is the default, it runs every design, and it is the only executor a build made without the
`oracle` feature contains. `--backend` is a diagnostic knob.

| backend | spellings | what it is |
|---|---|---|
| `native` | `native` | a compiled operation stream over a flat arena. The default and the product path |
| `vm` | `vm`, `bytecode` | compiles suspend-free bodies to bytecode and interprets the rest. A second implementation to bisect a suspected defect against |
| `interp` | `interp`, `interpreter` | walks the sim-IR directly. The readable reference for what a statement means, and excluded from performance work by design |

All three are required to print identical bytes — stdout and waveform alike — and that equivalence
is enforced over the deterministic corpus by `sim-engine/tests/backend_equiv.rs`. The flag therefore
moves wall clock only. Performance order is `native` faster than `vm` faster than `interp`;
`vita --help` carries a measured triple for one design, and the numbers are reproduced by:

```
cargo test -p sim-engine --release --test perf_baseline -- --ignored --nocapture
cargo run --release -q -p corpus-runner -- run --compare
```

A bad value is a usage error whose message names all three choices. In a build without the `oracle`
feature, `interp` and `vm` are still recognized and produce a distinct message saying they are
compiled out.

If the runtime gate refuses a design, `native` falls back to the bytecode VM and says so with
`W-RUN-BACKEND-FALLBACK` (`VITA-W4030`) rather than failing or going quiet. In a build without the
`oracle` feature there is no fallback target, so a refusal is a fatal run naming the refusal reason.
Under `--obs-dir`, the run manifest records `backend` (what ran) beside `backend_requested` (what
was asked for); comparing the two is the machine-readable fallback signal. The gate itself is
described in [../preview/21-tier3-native-backend.md](../preview/21-tier3-native-backend.md).

---

## 14. The observability rail

`--obs-dir <DIR>` writes a machine-readable companion to the human-facing stdout and waveform. It
never changes the simulation, the transcript, the waveform, or the exit code. The directory is
created if it does not exist, including nested components.

**The whole rail is one-shot `vita` only.** `vcmp`, `velab` and `vrun` refuse `--obs-dir`,
`--probe`, `--probe-file`, `--obs-procs` and `--obs-procs-time` with the messages in §3.4, and
`velab` refuses a design that calls `$vita_stage`. `--hier-tree` and `--inst-paths` are the one
exception to that loudness: the staged applets accept them and write nothing.

```
vita tb.sv --obs-dir obs/
```

| file | written when | contents |
|---|---|---|
| `run.json` | every `--obs-dir` run that reaches simulation | the run manifest (§14.1) |
| `results.jsonl` | every `--obs-dir` run that reaches simulation | one result record per run (§14.6) |
| `coverage.json` | the design has at least one covergroup instance with coverage items | functional covergroup coverage (§14.7) |
| `trace.jsonl` | at least one `--probe`/`--probe-file` path resolved | one record per probed value change (§14.9) |
| `stage.jsonl` | the `+STAGE_TRACE` plusarg is given | one record per captured `$vita_stage` call (§14.8) |

A compile or elaboration failure writes no observability directory at all. A filesystem failure
while writing any of these files is loud on stderr as `error[VITA-E0001]` and does **not** change
the exit code.

### 14.1 `run.json`

Hand-written JSON with a fixed key order, one top-level field per line. `schema_ver` is `1`.

| key | type | meaning |
|---|---|---|
| `schema_ver` | int | `1`. Bumped only for a record-envelope change, never for an added field |
| `tool` | string | `"vita"` |
| `version` | string | the tool version, `"0.2.0"` |
| `format_version` | int | the artifact format this build emits, `31` |
| `seed` | null | always null; there is no `--seed` flag |
| `plusargs` | array of string | runtime plusargs in command order, leading `+` stripped |
| `source` | object | `name` is the **basename of the first source file only**, so the same design run from two directories compares clean; `blake3` is the digest of the concatenated source text of every command-line file |
| `finish_reason` | string | `"finish"`, `"stop"`, `"quiescent"`, `"delta_limit"`, `"error"`. It describes how the run stopped and is never the verdict. `--timeout` produces `"quiescent"` |
| `exit_class` | string | `"ok"`, `"had_errors"`, `"fatal"`, derived from the final exit code and the fatal count, so a `-Werror`-promoted warning cannot produce `"ok"` beside `exit_code: 1` |
| `exit_code` | int | the process exit code |
| `sim_time` | int | final simulation time in raw ticks of the global precision |
| `counts` | object | `errors`, `warnings`, `fatals`. **`errors` excludes fatals**, unlike the stderr epilogue's `errors=` token, which is their sum. The two differ exactly when `fatals` is non-zero |
| `status` | string | `"PASS"` when `exit_code` is 0, else `"FAIL"` |
| `backend` | string | the executor that actually ran process bodies |
| `backend_requested` | string | what `--backend` asked for |
| `codegen` | object | the bytecode VM's static capability census (§14.2) |
| `native` | object | the native backend's eligibility verdict (§14.3) |
| `subroutines` | object | the static frame/inline route census, written unconditionally (§14.4) |
| `processes` | object or null | per-body activation profile; null means not measured (§14.5) |
| `builtins` | object or null | per-builtin call profile (§14.5) |
| `subroutine_calls` | object or null | runtime per-subroutine profile (§14.4) |
| `utc_unix_s` | int | wall-clock epoch seconds |
| `wall_s` | number | total wall seconds |
| `elab_s` | number | wall seconds before simulation: preprocess, lex, parse, elaborate |
| `sim_s` | number | wall seconds inside simulation — the only part `--backend` can move |

Two runs of the same input produce byte-identical `run.json` **except** for the four isolated
wall-clock fields `utc_unix_s`, `wall_s`, `elab_s` and `sim_s`, plus every `time_s` under
`--obs-procs-time`. All of them are formatted to six decimals. The `elab_s`/`sim_s` split is the
one to read when chasing speed: if `elab_s` dominates, no `--backend` choice helps.

`processes`, `builtins` and `subroutine_calls` are the literal `null` when the profile flags are
absent. Null means **not measured**, which is deliberately different from an empty object, which
would mean measured and empty.

### 14.2 `codegen`

```json
"codegen": {"able": 3, "total": 5, "frame_bodies": 2, "reject_reasons": {"delay": 1, "wait": 1}}
```

| key | meaning |
|---|---|
| `able` | process templates the VM's compile gate accepts |
| `total` | process templates in the elaborated IR |
| `frame_bodies` | function and task bodies, none of which are compile candidates, so a design whose work lives in subroutines can show `able` equal to `total` and still run almost nothing on the VM |
| `reject_reasons` | cause to number of process templates exhibiting it. A template with two causes counts under both, so the column can sum to more than the rejected-template count |

The reject vocabulary is closed: `class_new`, `delay`, `disable`, `force_release`, `fork`,
`frame_call`, `nba_transport_delay`, `sformatf`, `stmt_effect_rhs`, `wait`. `sformatf` means the
body reaches a string-formatting IR node, which elaboration also produces for string concatenation,
not that the source spells `$sformatf`.

### 14.3 `native`

```json
"native": {"eligible": true, "buildable": true, "refused": null, "reject_reasons": {}}
```

| key | meaning |
|---|---|
| `eligible` | the scope gate accepted the design, which is true exactly when `reject_reasons` is empty |
| `buildable` | the storage gate accepted it |
| `refused` | the runtime refusal reason, or null when nothing refuses |
| `reject_reasons` | reject family to count of offending items. Any non-zero row disqualifies |

`refused` carries **two vocabularies**. When the design gate refused, it is a key of
`reject_reasons`. When the design gate passed and the storage gate refused, it is that refusal's own
prose, which appears in no map. A consumer joining `refused` back to `reject_reasons` must read a
miss as the storage case, not as an error. The design gate has one family key, `stmt_effect`.

### 14.4 The two subroutine objects

`subroutines` is written on **every** `--obs-dir` run, with no profile flag, because it is not a
measurement: it is what elaboration decided.

```json
"subroutines": {"counts": {"total": 3, "frame": 2, "inlined": 1},
  "sites_semantics": "call sites LOWERED (after generate/instance expansion), not executions; 0 = declared and never called",
  "uncounted": "class methods and hierarchical calls",
  "items": [
    {"module": "tb", "name": "hexdig", "kind": "function", "route": "frame",   "sites": 18},
    {"module": "tb", "name": "mask8",  "kind": "function", "route": "inlined", "sites": 4}
  ]}
```

* `route` is `"frame"` — a real call with a stack frame per invocation — or `"inlined"`, meaning
  the body was spliced into every caller during elaboration.
* `sites` counts call sites **lowered**, after generate and instance expansion. A call written once
  inside a module instantiated four times is `4`; a call inside a `for` loop body is `1`. It is
  never an execution count, and `0` means declared and never called.
* `module` is the module whose body was being lowered, so a package routine is filed under the
  module that calls it and its name carries the scope, as in `p::dbl`.
* Class methods and hierarchical calls such as `u1.f(x)` are not counted; the object says so in its
  own `uncounted` field.
* Rows are sorted by `(module, name)`.

Which route a subroutine takes is not always the guess: `function int f` is framed while its twin
`function logic [31:0] f` is inlined, because `int` is 2-state and the frame's return slot is what
forces x and z to 0. A frame call is the more expensive route, so a frame-routed subroutine called
from a hot expression is usually a bigger cost than anything inside that expression.

`subroutine_calls` is the runtime half, and requires `--obs-procs`:

```json
"subroutine_calls": {"timed": false, "key": "…", "time_semantics": "…", "distinct": 2, "total_calls": 6,
  "items": [
    {"func": 0, "name": "top.u1.f", "decl_file": "d.sv", "decl_line": 2, "decl_col": 28, "calls": 3},
    {"func": 1, "name": "top.u2.f", "decl_file": "d.sv", "decl_line": 2, "decl_col": 28, "calls": 3}
  ]}
```

| key | meaning |
|---|---|
| `func` | the function id, per instance |
| `name` | a label, not a key. A module subroutine gets its `%m` instance path (`top.u1.aut`); a class method gets the class-relative `C.m` fragment. Nothing in the row distinguishes the two conventions |
| `decl_file`, `decl_line`, `decl_col` | the declaration site, written once however many function ids it minted. The column points at the routine's own name |
| `calls` | entries into this subroutine. Deterministic |
| `timed_calls` | with `--obs-procs-time` only: the subset of `calls` that `time_s` covers |
| `time_s` | with `--obs-procs-time` only: self seconds over `timed_calls`, with nested subroutine time subtracted |

`timed_calls` is lower than `calls` when some entries were **suspendable** task frames: opening such
a frame is counted, and the task may then sit on a delay for the rest of the run, so it is never
timed. Rows are sorted by `calls` descending, then by function id.

**The two objects do not join.** They count different things under different keys:

| axis | `subroutines` | `subroutine_calls` |
|---|---|---|
| key | module and routine name | per-instance function id |
| flag | none | `--obs-procs` |
| class methods | excluded | included |
| hierarchical calls | excluded | included |
| inlined subroutines | included, with `route: "inlined"` | absent — there is no call node to count |
| several instances | folded into one row | one row per instance |
| a package routine's name | `p::dbl`, filed under the calling module | `top.dbl`, the instance path, package qualifier gone |
| declaration site | not carried | `decl_file:decl_line:decl_col` |
| quantity | `sites`, lowered call sites | `calls`, runtime entries |

The columns must never be added together, and the object states this in its own `key` field. The
static rows do not carry a declaration site, so the only cross-read is by name and route.

### 14.5 `--obs-procs` and `--obs-procs-time`

`--obs-procs` adds three objects to `run.json` and requires `--obs-dir`. `--obs-procs-time` implies
it and adds wall-clock fields.

```
vita tb.sv --obs-dir obs/ --obs-procs
```

```json
"processes": {"timed": false, "counts": {"processes": 5, "assigns": 6, "total_evals": 56},
  "items": [
    {"domain": "assign",  "index": 0, "kind": "port",      "scope": "top.u1", "file": "d.sv", "line": 7, "col": 10, "evals": 9},
    {"domain": "process", "index": 1, "kind": "always",    "scope": "top",    "file": "d.sv", "line": 9, "col": 3,  "evals": 8},
    {"domain": "process", "index": 0, "kind": "var_init",  "scope": "top",    "file": "d.sv", "line": 6, "col": 15, "evals": 1}
  ]}
```

| key | meaning |
|---|---|
| `domain` | `"process"` or `"assign"`, stated explicitly rather than inferred from `kind` |
| `index` | index into that domain's IR vector |
| `kind` | the source construct |
| `scope` | the instance path the body was elaborated under — the same string `%m` renders. A module instantiated 40 times gives 40 rows with the same location and different scopes |
| `file`, `line`, `col` | the source position as given on the command line, `0` when unlocated |
| `evals` | activations |
| `time_s` | with `--obs-procs-time` only. It is omitted rather than zeroed on an untimed run, because a `0.0` would read as "this body is free" |

`kind` is a closed vocabulary: the user-written `initial`, `always`, `always_ff`, `always_comb`,
`always_latch`, `final`; the synthesized `sva`, `covergroup`, `clocking`, `var_init`, and the
fail-safe label `synth` for an unlabelled producer; and, for continuous assigns, `assign`,
`net_init` and `port`.

An **evaluation** is one activation by the scheduler. A process that suspends on `#5` and resumes
counts twice; a continuous assign counts one settle visit that actually re-evaluated its right-hand
side, and skips driven by the dirty worklist are not counted. A fork child is charged to the process
template it belongs to. Bodies that never ran are listed with `evals: 0`, because "this block never
fired" is a finding.

Rows are sorted most-evaluated first, with a total tiebreak on `(domain, index)`, so the order is
deterministic even on a timed run and two files can be diffed directly.

Port hookups appear as `kind: "port"` — one synthesized continuous assign per port connection — and
on a structural design they can be most of the rows. Their position is the port connection in the
parent's instantiation: the `.p(expr)` or `.p` shorthand starting at its `.`, or the connection
expression for a positional list, so connections written on one line differ by column. Two shapes
are reported honestly rather than approximately: a `.*` wildcard connection has no source text of
its own and reports `"", 0, 0`, and an unpacked-array port becomes one row per element, all sharing
that single connection's position.

The `builtins` object sits beside `processes` and answers what `processes` cannot: a `processes` row
is a block the user wrote, a `builtins` row is a simulator primitive that block called.

```json
"builtins": {"timed": true, "attribution": "self-plus-arguments", "included_in_processes": true,
  "time_semantics": "…", "distinct": 4, "total_calls": 3006, "obs_overhead_est_s": 0.000412,
  "items": [
    {"name": "$fgets",      "calls": 1001, "time_s": 0.004241},
    {"name": "$sscanf",     "calls": 1000, "time_s": 0.000642},
    {"name": ".push_back()","calls": 1000, "time_s": 0.000100},
    {"name": ".size()",     "calls": 1,    "time_s": 0.000012}
  ]}
```

`name` is the spelling the design uses: `$…` for a system task or function, `.name()` for a method
form such as `q.push_back(v)` or `s.len()`. Constructs that share one internal identifier are still
told apart, so `$info`, `$warning`, `$error`, `$fatal`, `$timeformat`, `$assertcontrol` and
`$vita_stage` each get their own row instead of hiding inside `$display`. `$cast` is a deliberate
merge: its task and function forms render as one name. Rows are sorted most-called first with a
name tiebreak, so two runs of one design produce identical output.

Three fields state the arithmetic so it never has to be guessed:

* `attribution` is the literal string **`"self-plus-arguments"`**. A row's `time_s` subtracts the
  spans of builtins nested inside it, but the call's own argument evaluation is **inside** the span.
  So `$display("%0d", q.size())` charges `.size()` to `.size()` and subtracts it from `$display`,
  while the cost of evaluating ordinary expression arguments stays with the caller.
* `time_semantics` is the literal string
  `"ranking and UPPER BOUND on removal gain: a row's time_s includes evaluating that call's own arguments (nested builtins are subtracted, ordinary expression work is not), so removing the call recovers at most this, usually much less"`.
  It is a ranking tool and an upper bound, not a removal estimate.
* `included_in_processes` is the literal `true`. This subtotal is already inside the `processes`
  rows, so the two arrays are never added. The useful arithmetic is the subtraction: an `initial`
  costing 43.7 s of which 18.2 s is `$fgets` plus `$sscanf` leaves 25.5 s of RTL.

`obs_overhead_est_s` appears only on a timed run. It is an estimate of what the timing itself cost,
calibrated at write time by timing 2000 clock reads on the machine and scaling by `total_calls`.

**Counts are deterministic; times are not.** `--obs-procs` alone leaves `run.json` byte-reproducible
and costs almost nothing. `--obs-procs-time` reads the clock on both sides of every activation and
every builtin call; for a fat `always_ff` that is noise, but for a one-bit continuous assign or a
short `.len()` the reading can cost more than the work, so a timed run is slower overall and the
per-row shares tilt toward the cheap rows. Read `evals` and `calls` first, and reach for `time_s`
only to break a tie between rows with similar counts.

### 14.6 `results.jsonl`

One line per run, terminated by a newline, with no wall-clock field at all, so the whole file
compares clean between two runs of the same input.

```json
{"v":1,"t":35,"kind":"result","status":"PASS","finish_reason":"finish","exit_code":0,"sim_time":35,"errors":0,"warnings":1,"fatals":0}
```

`v` is the record-envelope version, `t` is the record's time (here the final simulation time), and
the remaining keys mirror `run.json`, including the rule that `errors` excludes fatals.

### 14.7 `coverage.json`

Written only when the design produced a coverage summary, which requires at least one covergroup
instance carrying at least one coverage item.

```json
{
  "schema_ver": 1,
  "kind": "coverage",
  "groups": [
    {"instance": "top.c", "coverage_pct": 70.833333, "coverpoints": [
      {"name": "cp_v",    "kind": "coverpoint", "num_bins": 4, "covered_bins": 3, "coverage_pct": 75.000000},
      {"name": "cp_w",    "kind": "coverpoint", "num_bins": 2, "covered_bins": 2, "coverage_pct": 100.000000},
      {"name": "cross_0", "kind": "cross",      "num_bins": 8, "covered_bins": 3, "coverage_pct": 37.500000}]}
  ]
}
```

`instance` is the covergroup instance's fully qualified name. `coverage_pct` on a group mirrors what
the design's own `get_coverage()` returns, including the accumulation order that makes the two agree
to the last digit. Items are coverpoints first, then crosses. A coverpoint with no bins is excluded
from the group average; a cross always counts, with an implicit weight of 1. `covered_bins` is the
population count of the final hit map with unknown bits excluded. An unlabelled coverpoint is named
`cp_<i>`, and a cross is always named `cross_<i>` — a user-written cross label does not reach this
file. Percentages are formatted to six decimals.

Assertion pass and fail counts, cover-property counts, and per-bin hit detail are not in this file.

### 14.8 `$vita_stage`, `stage.jsonl` and `+STAGE_TRACE`

`$vita_stage("label" [, v0, v1, …]);` is a vendor system task that marks a point in a run without
printing. It takes at least one argument, the label; the rest are arbitrary runtime expressions
evaluated at the call. It is the only `$vita_*` introspection built-in.

Capture is armed by the `+STAGE_TRACE` plusarg together with `--obs-dir`. Without the plusarg the
task is a pure no-op: nothing is captured, nothing is printed, and the record counter does not
advance. **`$vita_stage` never prints**, armed or not, which is what makes it safe to leave in RTL.
It does count as a builtin, so it earns its own `builtins` row.

```
vita tb.sv --obs-dir obs/ +STAGE_TRACE
```

```json
{"v":1,"t":0,"kind":"stage","label":"init","idx":0,"vals":["0"]}
{"v":1,"t":35,"kind":"stage","label":"done","idx":1,"vals":["3","3"]}
```

| key | meaning |
|---|---|
| `v` | envelope version, `1` |
| `t` | simulation time at the call, in raw ticks |
| `kind` | always `"stage"` |
| `label` | the first argument, coerced as `%s`. It may be a runtime string variable, not only a literal |
| `idx` | a monotonic per-run counter starting at 0, incremented once per captured call. Not per label and not per scope |
| `vals` | the remaining arguments, each formatted with `$display %0d` semantics and emitted as a JSON **string** so unknown values are representable |

Value formatting follows `%0d`: a real rounds half away from zero to an integer, a value containing
unknown bits renders as a single collapsed `x`, `X`, `z` or `Z`, and everything else is exact decimal
at any width with signed values printed with a leading `-`. A **string** argument renders
numerically, as its packed byte value, not as text.

`stage.jsonl` is written whenever `+STAGE_TRACE` is set, independently of whether the design
contains any `$vita_stage` at all, so an armed run with no calls leaves an empty file. Records are
in emission order.

The staged flow refuses this rail: `velab` rejects a design that calls `$vita_stage` and names the
one-shot command that stages it.

### 14.9 `--probe`, `--probe-file` and `trace.jsonl`

`--probe <NET>` records every value change of one hierarchical net into `trace.jsonl`. It is
repeatable and requires `--obs-dir`. `--probe-file <FILE>` reads paths from a file, one per line;
a line that is empty or starts with `#` after trimming is skipped, and the remaining lines are
appended after the `--probe` values.

A path must be the **full dotted hierarchical net name**, matched by exact string equality against
the elaborated net table — `top.clk`, `top.u1.y`. There is no glob, wildcard, prefix, regular
expression, or bit and element selection syntax, and there is no command to list available net
names. Duplicates are harmless.

Both failure modes are loud, exit 3, never a silent skip:

```
--probe path 'top.nope' does not resolve to a net (check the hierarchical name; …)
--probe path 'top.mem' is an unpacked array — v1 can trace only a scalar/vector/packed net (…)
```

The kinds refused are a dynamic-array, queue or string handle; an unpacked array; and a real or
realtime net. Probe resolution happens after elaboration and before simulation, so a bad path still
writes `--hier-tree` and `--inst-paths` and writes no observability directory at all.

```json
{"v":1,"t":5, "kind":"chg","path":"top.clk", "old":"0",   "new":"1"}
{"v":1,"t":5, "kind":"chg","path":"top.u1.y","old":"xxxx","new":"0001"}
```

| key | meaning |
|---|---|
| `v` | envelope version, `1` |
| `t` | simulation time in raw ticks. There is no delta-cycle or scheduling-region field |
| `kind` | always `"chg"` |
| `path` | the full dotted net name, the same string `--inst-paths` and the VCD `$scope` tree use, JSON-escaped so an escaped identifier survives |
| `old` | the last emitted value string for that path |
| `new` | the new value string |

Values are full-width, unprefixed, MSB-first 4-state binary strings, one character per bit from
`0`, `1`, `x`, `z`; a one-bit net is one character.

Sampling is transition-only: a record is emitted at each write the engine treats as a change, then
deduplicated against the last emitted string, so a same-value write emits nothing. The previous
value is armed before the event loop from each probed net's construction value, so a value first
driven at time 0 is logged with the construction default as `old`. Probing is independent of
waveform dumping — a probed run with no `$dumpvars` still traces — and independent of the backend,
including the native one. Several probes share one ordered record stream, so records from different
paths interleave in time order rather than grouping by path.

### 14.10 `--hier-tree` and `--inst-paths`

Both run immediately after elaboration and before simulation, on one-shot `vita` only. Both are
plain UTF-8, one record per line. A write failure is loud on stderr and does not change the exit
code.

`--hier-tree <FILE>` walks the instance tree in pre-order from every root, two spaces per level,
one line per instance as `<leaf instance name> : <module name>`:

```
top : top
  m0 : mid
    u : leaf
    u : leaf
  arr[1] : leaf
  arr[0] : leaf
```

Generate-scope information does not survive this shape: `top.m0.g[0].u` and `top.m0.g[1].u` both
render as `u : leaf`. Arrayed-instance segments do survive, because they are part of the leaf
segment.

`--inst-paths <FILE>` writes each full dotted path verbatim, in elaboration order, consistent with
the VCD `$scope` tree:

```
top
top.m0
top.m0.g[0].u
top.m0.g[1].u
top.arr[1]
top.arr[0]
```

### 14.11 What the rail does not record

* The run manifest does not record `-G` parameter overrides, so two runs differing only in `-G`
  produce manifests identical outside the wall-clock fields.
* `source.blake3` covers the source **text** only. The `-D`, `+define+` and `-I` surface is not
  folded into it, so two runs of one conditionally compiled file report the same digest.
* There is no `--seed` flag and no run identifier field.
* `processes` rows reach process and continuous-assign granularity, not a call tree, and `builtins`
  rows are aggregated by name rather than by call site.
* `coverage.json` carries covergroups only, with bin totals rather than per-bin detail.
* `trace.jsonl` records value changes with no scheduling-region annotation, and neither trace nor
  stage values render enumeration names.
* There is no interactive control channel, snapshot, or rewind surface.

The specification behind the rail, including the items above, is
[../preview/19-ai-agent-observability.md](../preview/19-ai-agent-observability.md).

---

## 15. The staged flow

The staged applets split the same pipeline at two disk boundaries, so a design can be recompiled or
re-elaborated once and simulated many times.

```
vcmp  source.sv …   →  source.vu       compile:   preprocess + lex + parse → serialized AST
velab source.vu     →  source.velab    elaborate: AST → sim-IR snapshot
vrun  source.velab  →  waveform + stdout          simulate
```

The staged chain and a one-shot run must produce the same observable output: the same transcript
byte for byte, the same waveform byte for byte, the same diagnostic lines including
`file:line:col`, and the same exit class.

### 15.1 Default output names and the clobber guard

Without `-o`, each stage derives its output by replacing **only the last extension** of the first
input.

| applet | default output |
|---|---|
| `vcmp` | `<first source>.vu`; omitted entirely when `--work` is given without `-o`, in which case only the library blob is written |
| `velab`, positional | `<input>.velab` |
| `velab -L` | `<first --top>.velab` |
| `vrun` | none; `-o` is a waveform override only |

So `a.sv` becomes `a.vu` and `a.b.sv` becomes `a.b.vu`.

Every applet refuses an output that would overwrite one of its inputs — `output '<out>' would
overwrite an input file`, exit 3. The comparison is string equality first, then canonicalization of
both paths when both exist, so `./a.sv` against `a.sv` and a symlink pair are both caught. It
applies to `vcmp` for both the default and an explicit `-o`, to `velab` in positional mode, and to
`vrun` when `-o` is given.

Artifacts are written atomically: to `<out>.tmp.<pid>`, then renamed. A crash mid-write cannot leave
a partial artifact that the staleness gate would misreport.

### 15.2 Positional counts

| applet | rule | message |
|---|---|---|
| `vita` | one or more sources | `no source files given` |
| `vcmp` | one or more sources | `vcmp: no source files` |
| `velab` | exactly one `.vu`, or zero positionals with `-L` | `velab: expected exactly one .vu input`; `velab: a positional .vu and -L libraries are mutually exclusive`; `velab: library mode needs at least one --top <unit> (a library's unrelated units must not become roots)` |
| `vrun` | exactly one `.velab` | `vrun: expected exactly one .velab input` |

### 15.3 `vcmp` — compile

```
vcmp [-o <out.vu>] [--work <name[=dir]>] <source.sv> [<source2.sv> …]
```

Preprocesses, lexes and parses the sources into a `.vu` artifact. The body is the serialized
front-end source unit, followed by a resolved-timescale tail so `velab` scales delays identically,
followed by a source-map tail carrying file names, original texts and provenance segments so
`velab` prints elaborate-time diagnostics with the same `file:line:col` a one-shot run gives.

Exit codes: 0 clean; 1 on a lex or parse error, an empty unit, or a `-Werror`-promoted warning;
2 on an invalid work-library manifest; 3 on a missing input, a write failure, an unknown flag, or no
sources.

```
vcmp pkg.sv dut.sv -o build/dut.vu
```

### 15.4 `velab` — elaborate

```
velab [-o <out.velab>] <in.vu> [--top <unit>]
velab -L <name[=dir]>… --top <unit>… [-o <out.velab>]
```

Reads one `.vu`, checks its staleness gate, elaborates the AST into a language-neutral sim-IR
snapshot, and writes a `.velab`. The body is the golden sim-IR frame followed by fifteen
append-only trailer segments — fork join modes, hierarchical net names for waveform scoping,
timescale multipliers, severities, radixes, scopes, assign ranks, queue bounds, consumed-library
records, net dimensions, final processes, deferred marks and actions, extra sidecars, and work
stamps. Those trailers ride outside the hashed frame, so they do not participate in the schema
hash; the container `format_version` is what covers them.

Exit codes: 0 clean; 1 on an elaboration error or a `-Werror`-promoted warning; 2 on a gate
rejection or an undecodable `.vu` body or trailer; 3 on a missing input, a write failure, an unknown
flag, or the wrong argument count.

```
velab build/dut.vu -o build/dut.velab
```

### 15.5 `vrun` — simulate

```
vrun [-o <out.vcd|out.fst>] <in.velab>
```

Reads one `.velab`, checks its staleness gate and its upstream freshness, and runs the simulation,
emitting the transcript and, when the design dumps, the waveform. `-o` follows the same rules as
`vita -o`, including the `.fst` extension rule, and is refused if it names the input.

Exit codes: 0 on a clean finish; 1 on a runtime `$fatal`, a `-Werror`-promoted warning, or a
truncated trailer caught by an engine guard; 2 on a gate rejection, an undecodable body or trailer,
or a stale upstream; 3 on a missing input file or the wrong argument count.

```
vrun build/dut.velab                # simulate; waveform if the design dumps
vrun build/dut.velab -o waves.vcd   # redirect the dump
vrun build/dut.velab +VERBOSE       # runtime plusargs reach $test$plusargs
vrun build/dut.velab --upstream build/dut.vu
```

### 15.6 Work libraries

```
vcmp a.sv --work mylib=build/mylib
velab -L mylib=build/mylib --top top -o build/top.velab
vrun build/top.velab
```

A library is a directory holding `lib.toml`, a machine-written manifest, and `units/cu_<hex>.vu`,
content-addressed compilation-unit blobs byte-identical to what `vcmp -o` writes.

| spelling | result |
|---|---|
| `--work NAME=DIR` | library `NAME` in `DIR` |
| `--work NAME` | library `NAME` in `./NAME`, or in `--workdir` when that is given |
| `--workdir DIR` alone | library `work` in `DIR` |
| `-L NAME=DIR`, `-L NAME` | same shape, on the reading side |

Either half of a `NAME=DIR` pair being empty is a usage error. Each recorded unit carries its kind:
module, interface, package, or class. A duplicate unit name in one library from a different source
is `E-DUP-UNIT` (`VITA-E2001`), exit 1, and names the fix: recompile that source, or rename.

In `velab -L` mode the first `-L` wins a duplicate logical name, and only the instantiation closure
of the requested `--top` units is loaded, so unrelated library units never become roots. A top that
is not found is `E-ELAB-UNSUPPORTED` (`VITA-E3009`), exit 1. Library mode does not carry the
source-map tail, because spans from different compilation units share a coordinate space; its
elaborate-time diagnostics are therefore location-less.

---

## 16. Staleness gating

Each `.vu` and `.velab` begins with an 8-byte magic and a header. Three header fields are
compatibility stamps, and the stage that reads the artifact compares them against the stamps the
running tool was built with, before a single body byte is deserialized:

| stamp | what it means |
|---|---|
| **`format_version`** | the on-disk container layout, including everything in the out-of-band trailers that the schema hash cannot see. The value in this build is `31` |
| **`tool_semver_major`** | the major version of the tool that wrote the artifact. The workspace version is `0.2.0`, so this is `0` |
| **`schema_hash`** | a structural hash of the **shape** of the serialized types: the front-end source unit for a `.vu`, the sim-IR for a `.velab`. Adding, removing, reordering or retyping a field flips it. It is computed identically on Linux and macOS, so the same source yields byte-identical artifacts on both |

The policy is refuse-and-rebuild, never silent migration: artifacts are cheap to regenerate from
source, and a misparsed snapshot is not. Every rejection below exits **2** and prints one
diagnostic with no source location, such as:

```
error[VITA-E9001] E-ART-FORMAT-MISMATCH: bad or missing velab magic
```

| rejection | code | what the message says | remedy |
|---|---|---|---|
| bad or missing magic, or a file shorter than the magic | `VITA-E9001` | `bad or missing velab\|VU magic` | the file is not a vita artifact, or is the wrong kind. Regenerate it with `vcmp`/`velab` |
| an undecodable header | `VITA-E9001` | `undecodable velab\|VU header: …` | regenerate the artifact |
| `format_version` mismatch | `VITA-E9001` | `format_version=<got> but this tool expects <want>; regenerate with velab` | re-run `velab`; for a `.vu`, re-run `vcmp` then `velab` |
| tool major-version mismatch | `VITA-E9004` | `produced by vitamin <got>.x, this tool is <want>.x; regenerate or install a matching vitamin` | regenerate with this tool, or install the matching one |
| `schema_hash` mismatch | `VITA-E9002` | `sim-ir type shape changed between builds; rerun velab` | re-run `velab`; for a `.vu`, re-run `vcmp` then `velab` |
| an undecodable body or any trailer segment | `VITA-E9001` | `undecodable .velab <segment> trailer: …` | regenerate the artifact |
| `--upstream` digest changed | `VITA-E9003` | `<file>: digest changed since the .velab snapshot (rerun velab, or drop --upstream)` | re-run `velab`, or drop the flag |
| a work library, blob, source or include file changed since the snapshot | `VITA-E9003` | `… changed since the .velab snapshot (re-run velab)`, or `… source changed since 'vcmp --work' (re-run vcmp + velab)` | run the command the message names |
| an invalid or unreadable work-library manifest | `VITA-E9005` | `E-WORK-MANIFEST` | re-run `vcmp --work` |

Two freshness checks run above the header gate:

* **`vrun --upstream <file.vu>`** re-reads that file, hashes it, and compares it with the digest the
  `.velab` recorded. This is a content hash, never a timestamp.
* **The work-library check runs on every `vrun`** whose `.velab` consumed a library, with no flag.
  It walks every recorded library manifest, compilation-unit blob, and source or include file and
  compares each against its recorded digest. A size-and-timestamp stamp lets an unchanged entry skip
  the re-hash; any stamp miss re-hashes, and the content hash is always the authority.

A missing input file is not a staleness rejection: it is `error[VITA-E8005]: cannot read '<path>':
<err>`, exit 3.

One-shot `vita` serializes nothing, so it has no staleness to check.

There is no `--rebuild` and no `--clean` flag. The remedy is always to re-run the producing stage.

---

## 17. Environment variables

| variable | effect |
|---|---|
| `VITA_THREADS` | integer fallback for `--threads` when the flag is absent. Resolution order is flag, then this variable, then automatic (available parallelism capped at 8), floored at 1. An unparsable value falls through to automatic. Reported with its provenance in the `-v` echo |
| `VITA_SVA_COLLAPSE` | presence, with any value, enables the SVA `##[m:n]` window collapse. The default is fan-out. Listed in the `-v` echo |
| `VITA_SCW_CHECK` | a development self-check whose value is a log file path. It lowers each size-cast operand a second way and appends comparison lines. It doubles diagnostics and duplicates `$random` draws while set |
| `VITA_JIT` | with the `jit` feature only: presence constructs the cranelift engine |
| `VITA_JIT_STATS` | with the `jit` feature only: presence dumps code-generation coverage at end of run |
| `REGEN_GOLDEN` | test-only golden regeneration. Not a product surface |
| any `$VAR` | expanded inside a filelist as `$VAR`, `${VAR}` or `$(VAR)`; undefined is a hard error (§4.2) |

---

## 18. See also

- [001 · Installation](001_installation.md) — building and putting the applets on a `PATH`.
- [002 · Quickstart](002_quickstart.md) — the first run, end to end.
- [005 · System tasks](005_system-tasks.md) — `$dumpfile`, `$dumpvars`, `$value$plusargs` and the rest.
- [006 · Limitations](006_limitations.md) — what the language front end refuses.
- [007 · Error codes](007_error-codes.md) — the diagnostic catalogue and exit-code summary.
- [../preview/13-diagnostics-and-logging.md](../preview/13-diagnostics-and-logging.md) — the diagnostic and logging design.
- [../preview/14-staged-artifacts.md](../preview/14-staged-artifacts.md) — artifact formats and hash binding.
- [../preview/19-ai-agent-observability.md](../preview/19-ai-agent-observability.md) — the observability specification.
- [../preview/21-tier3-native-backend.md](../preview/21-tier3-native-backend.md) — the native backend and its gate.
