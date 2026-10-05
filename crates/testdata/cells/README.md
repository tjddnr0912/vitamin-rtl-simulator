# Oracle cells

These are the small SystemVerilog designs (cells) that the slices §4.5.580 to §4.5.594 wrote to measure vita
against the oracles. Each cell is stored with the raw output of every oracle run on it and the raw output of
each vita binary it ran under, as the slice captured them during development.

This directory is preserved data. **No test, build script or harness reads it.** Nothing outside this
directory refers to it, and a harness can be added later. Until then, editing or adding a file here changes
no test result.

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
