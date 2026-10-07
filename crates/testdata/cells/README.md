# Oracle cells

These are the small SystemVerilog designs (cells) that the slices §4.5.580 to §4.5.594 wrote to measure vita
against the oracles. Each cell is stored with the raw output of every oracle run on it and the raw output of
each vita binary it ran under, as the slice captured them during development. `r9/` holds, in the same form, the
minimal repros of the row-9 new-design census (below).

This directory is preserved data. The cells and captures are read-only: nothing writes them. **The
harness reads them**: `MANIFEST.txt` (below) is generated from them, and a test fails if the two disagree. The
line `not read by any test` at the top of each `.expect` file is part of the preserved bytes and predates the
harness.

## Layout

| Path | Contents |
|---|---|
| `<row>/<cell>.sv` (or `.v`) | the cell source, byte for byte as the slice wrote it; the extension is the one the slice used |
| `<row>/<cell>.expect` | the cell's captured outputs, its verdict and where it came from (format below) |
| `<row>/<cell>.aux/` | other files the cell needs (an include, a file list, a second compilation unit), kept at the paths the slice used |
| `INDEX.tsv` | one row per cell, including byte-identical copies (`alias_of`), giving the row, slice, cell set, oracles present, vita binary tags, verdict and verdict source |
| `TOOLS.txt` | oracle versions: as each slice captured them, and as read on the day of preservation |
| `_slices/<slice>/META.md` | the slice's vita binaries (md5, profile, base commit), oracle versions, where its verdicts were quoted from, its directory layout and file suffixes, counts, and what could not be recovered |
| `_slices/<slice>/FILES.tsv` | every kept file with its scratch origin, its location here (`<row>/<cell>.expect#blockN` for an output) and its md5 |
| `_slices/<slice>/EXCLUDED.tsv` | what was left out and why |
| `_slices/<slice>/files/`, `refs/` | the slice's result tables, runner and generator scripts, and the slice documents that the verdict sources cite by line |

`<row>` is the ROADMAP row the slice worked on, in ASCII: a §2 row letter (`AE`, `AD`, `V`, `Z`, ...) or a §3.b
row name (`case-inside`, `unique-const-fn`, ...). When a slice names two rows (`§2 🆕 T; §2 🆕 S (a)`), the
first is used and `INDEX.tsv` keeps the full text. Cells with no recorded row are grouped by their scratch
directory name (`r3`), and the `grouping` column of `INDEX.tsv` says so.

`r9` is grouped by census, not by ROADMAP row: it holds the cells of the row-9 new-design census (§5.2 row 9;
OpenTitan, VeeR and alexforencich minimal repros, with the judge's re-runs attached as aliases), and each cell's
judged root id is its `roadmap_row`. It is not wired: `r9` is not a seed row, so admission and every test ignore
it (a later `cells pin --rows r9` would read it). `_slices/r9/META.md` says how its exit codes, verdicts and
licence screen were derived.

`<cell>` is `<slice>__<path of the cell in the slice's scratch directory, with / replaced by __>`, for example
`s588__g__b__b33_bits` for `s588/g/b/b33_bits.sv`.

## The .expect format

```
# preserved raw captures (oracle + vita); not read by any test; ...
cell: s588__g__b__b33_bits
slice: §4.5.588 (s588; details _slices/s588/)
origin: s588/g/b/b33_bits.sv
roadmap_row: §2 🆕 AE
set: g/b
oracles_present: iverilog,sv2v,verilator
vita_tags: PRE,PROBE-a
verdict: loud
verdict_source: s588/GROUNDING.md:222
vita_binary: PRE: md5 e1e7e57148bb83ad35d2c4f68247a290 commit 2f2d3f2d
blocks: 7
=== block 1 | oracle iverilog 13.0 (version captured in slice) | ./b33_bits.ivl | 53 bytes
<53 bytes, exactly as captured>
...
=== block 7 | vita [PROBE-a] | ./b33_bits.PROBE | 271 bytes | identical to block 6
=== end
```

- The header lines before `blocks:` are metadata. `verdict` is the slice's own word for the cell (for example
  `silent→correct`, `WRONG→OK`, `loud`, `real gap`, `no-oracle`, `vita-ahead`, `harness-format`,
  `KNOWN-WRONG`, `residue`), quoted from `verdict_source` (a scratch path and line, kept under
  `_slices/<slice>/`). It describes the slice's PRE and POST binaries, not the current tree.
- `vita_binary` gives the md5 and base commit of each vita binary tag used in the file. The binaries
  themselves are not kept. `_slices/<slice>/META.md` has the full table with where each md5 came from.
- Each block's content is exactly the number of bytes its header states, starting after the header line.
  A newline is added after content that does not end in one. Captured text can contain lines that begin
  with `===`, so a reader must count bytes and not search for the next header.
- `identical to block N` means the capture was byte-identical to block N of the same cell, so its content
  is not repeated.
- The file path in a block header is relative to the slice's scratch directory (`./` = beside the cell).
- A field the slice never captured is the word `unrecorded`. Nothing was inferred, re-run or regenerated.

## What is not here

- Third-party RTL. The workload corpus (ibex, verilog-axi, verilog-ethernet, serv, picorv32, darkriscv,
  biriscv, aes, sha256), excerpts of it, and vita or oracle output on it are never kept here; see
  `bench/README.md`.
- VCD files, by the rule in CONTRIBUTING.md ("Do not commit ... generated `.vcd` output"); they are listed in
  `_slices/DROPPED.tsv`.
- Cells that only vita ran, which have no oracle output to compare against; `EXCLUDED.tsv` counts them.
- Build trees, compiled iverilog images, verilator object directories, vita staged artefacts, git
  worktrees, repository snapshots and saved patches.

## Oracles

iverilog 13.0 (`-g2012`), verilator 5.052 (`--binary --timing`), and sv2v v0.0.13 followed by iverilog. The
development rules treat verilator as no oracle for x/z, out-of-range and event-order questions.

## Manifest and harness

`MANIFEST.txt` is the curated set of cells the gate grades; `MANIFEST.excluded.tsv` lists every other cell of
the seed rows with the reason it is out. Both are written only by `corpus-runner cells pin`; never edit them by
hand.

```sh
cargo run -p corpus-runner --locked -- cells run [--vita PATH]          # the graded table
cargo run -p corpus-runner --locked -- cells pin --vita PATH --label TEXT # regenerate both files
```

Admission is mechanical and reads no `verdict:` word. It counts SIMULATORS, not captures: iverilog and
sv2v→iverilog share iverilog's back half and vvp (`_slices/s588/files/g/run4.sh:9`), so they are one vote, and
verilator (and xcelium) one each. A cell of the seed rows (`AD AE AI U V W`, the slices §4.5.587–594) is
admitted when at least two simulators ran it to `rc=0` with equal normalized output and no capture dissents.
Each capture is classified (`crates/corpus-runner/src/cells/admit.rs`):

| Capture | Counts as |
|---|---|
| `rc=0` | an answer |
| an error diagnostic and a non-zero exit: iverilog `error:` / `syntax error` / `Unable to bind`, sv2v `Parse error:` / `sv2v:`, verilator `%Error` (a failed build included) | a dissent: the cell is excluded (`oracle-rejects`), however many others agree |
| `sorry:` only, `%Error-UNSUPPORTED` only, an internal error or assertion, a signal, a hang | absent: not a verdict on the design |
| anything else | excluded (`oracle-failed-unclassified`) |

When iverilog and sv2v both answer and differ, the cell is `iverilog-family-split`; when they are the only
answers, `same-family-only`. Two simulators that disagree make an `oracle-split` (`oracle-split-verilator-only`
when iverilog and sv2v agree against verilator, `harness-format` when only whitespace differs). There is no
majority. A cell that needs its `.aux/` files (`needs-aux`) or whose agreed answer prints nothing is out. When
iverilog itself did not answer, a cell whose source reads a `parameter` or `localparam` on a line above its
declaration is out too (`forward-ref-without-iverilog`): ROADMAP gives that axis to iverilog and disqualifies
sv2v and verilator on it. The textual check over-reports (a legal shadow counts) by design.

Normalization (`crates/corpus-runner/src/cells/normalize.rs`) is scoped to the tool that printed the
capture. A capture is cut at its last `rc=N` line. From iverilog-family captures it drops vvp's `$finish called
at` line, `F:N: warning:` lines, their `F:N:   :` continuations and `warning: verinum::`; from verilator
captures, `%Warning-CODE: F:L:C:` build lines, the `$finish` line, the report banner and `- Verilator:` lines;
from vita's stdout, its last line when that is `simulation ended (…) at time T`. Trailing blank lines are
trimmed.

`pin` runs each admitted cell twice under the binary it is given and records what vita does:

| `expect` | vita at pin time | Pinned |
|---|---|---|
| `runs 0` | exit 0, the oracles' answer | the oracle lines |
| `known-wrong 0` | exit 0, another answer | the oracle lines and vita's lines |
| `refused` | exit 1 and an error diagnostic | `refusal`: the first error line with its file name stripped; `codes`: every error code, sorted; `at`: every error's `line:col`, sorted |

A re-pin is guarded: `pin` compares what it would write with the manifest already there and prints every
cell's move. A move in the regression direction — runs to known-wrong or refused, refused to known-wrong, a
known-wrong answer or a refusal changing, a pinned cell leaving for a vita-side reason (`vita-crash-at-pin`,
`vita-nondeterministic-at-pin`, `order-only`) — is written only when the cell is named with `--accept CELL,…` or
`--accept-file FILE`; otherwise nothing is written and the exit is 1. Improvements (known-wrong or refused to
runs, known-wrong to refused) and oracle-side changes are printed and need no name.

A cell that crashes, hangs or answers differently on its two runs is excluded, and so is a known-wrong cell
whose lines are the oracles' in another order (`order-only`: an order the oracles may share by coincidence).
`run` copies each cell into a temporary directory under its original file name, runs `vita <name>` twice with
every `VITA_*` variable removed from its environment, and grades it with the workload corpus's grades: `runs` is
`ok` or `REGRESSION`; `known-wrong` is `known-wrong` (the same wrong answer), `PROMOTED` (the oracles' answer),
`DRIFTED` (another wrong answer, or a refusal: wrong to loud) or `REGRESSION` (a crash, a hang, another exit);
`refused` is `known-gap` (the exact pin), `DRIFTED` or `PROMOTED`. Two runs that disagree are `REGRESSION`,
marked `*** NON-DETERMINISTIC ***`. Unlike the workload corpus, every move fails: only `ok`, `known-wrong` and
`known-gap` pass. A `PROMOTED` cell is re-pinned, so a fix cannot be lost silently when it is later reverted.

Known blind spots. Only stdout is compared, so a design whose `$error` or `$fatal` runs under vita is pinned
`refused` by its diagnostic (none is admitted today). The end time and end reason are not compared: the tools
count time in different units (iverilog in the global precision, sv2v's translation in `1s` because it drops
`` `timescale ``, verilator in the caller's time unit, rounded), so a change that moves only when a run ends is
invisible to the gate.

`crates/cli/tests/oracle_cells.rs` runs the whole manifest against the tree's own `vita` on every
`cargo nextest run`; `crates/corpus-runner/tests/cells.rs` checks that the manifest's oracle side is what
admission derives from the captures here and that every vita pin has its shape.
