# bench/ — the workload corpus

`bench/` holds the workload corpus: real RTL fetched at a pinned commit, run under
`vita`, and checked against a digest an external oracle produced. The harness is
[`crates/corpus-runner`](../crates/corpus-runner); the contract every workload obeys,
the cross-corpus timing tables and the review findings live in
[docs/study/03-workload-corpus.md](../docs/study/03-workload-corpus.md). This file
covers what is on disk under `bench/`, how to fetch it, how to run it, and how to read
the result.

## Driving the corpus

```sh
cargo run -p corpus-runner -- list           # what the corpus contains, and what is on this machine
cargo run -p corpus-runner -- fetch --run    # clone each upstream design at its pinned SHA
cargo run -p corpus-runner -- run --compare  # run everything, check every digest, time iverilog too
```

`list` is the default subcommand. `run` accepts `--filter <substring>`, `--reps <N>`
and `--compare`. `--reps N` records N timed samples over N+1 rounds — the first round
is discarded as cache warm-up — and defaults to 3. `run` needs a binary at
`target/release/vita`; it falls back to `target/debug/vita` and warns that the timings
are not comparable.

| Exit | Meaning |
|---:|---|
| 0 | every present workload matched |
| 1 | a mismatch, a crash, or a failed clone under `fetch --run` |
| 2 | nothing present (run `fetch --run` first), or `--filter` matched no workload |
| 3 | usage error, or no `vita` binary to run |

## The ten rows

Ten workloads over nine directories: `keccak` and `keccak-arr` share `bench/keccak`
and differ only in which design file they compile. Four shapes are represented, so
the corpus measures more than one kind of RTL.

| Workload | Directory | Origin | Shape | Licence |
|---|---|---|---|---|
| `sha256` | `sha256/` | secworks/sha256 | crypto | BSD-2-Clause |
| `aes` | `aes/` | secworks/aes | crypto | BSD-2-Clause |
| `picorv32` | `picorv32/` | YosysHQ/picorv32 | cpu | ISC |
| `darkriscv` | `darkriscv/` | darklife/darkriscv | cpu | BSD-3-Clause |
| `biriscv` | `biriscv/` | ultraembedded/biriscv | cpu | Apache-2.0 |
| `serv` | `serv/` | olofk/serv | cpu | ISC |
| `verilog-axi` | `verilog-axi/` | alexforencich/verilog-axi | fabric | MIT |
| `verilog-ethernet` | `verilog-ethernet/` | alexforencich/verilog-ethernet | stream | MIT |
| `keccak` | `keccak/` | first-party | crypto | this repository |
| `keccak-arr` | `keccak/` | first-party | crypto | this repository |

Each directory carries a `RUN.md` with the by-hand recipe for that one workload: the
pinned SHA, the exact file list in order, the exact command lines, the expected
output, and the reconstruction steps for anything not committed. `bench/keccak` also
carries a `README.md`, because that RTL is written here rather than fetched.

`bench/ibex` is not a corpus row and is not committed. It is kept locally for
reproduction only: iverilog 13 cannot parse it, so no oracle can grade it, and the
corpus admits no design an oracle has not run first.

## Committed here, and not

Two kinds of thing live under `bench/`, treated oppositely.

| | Committed | Why |
|---|---|---|
| Testbenches (`tb*.v`, `tb*.sv`), `files.txt`, `RUN.md`, `README.md`, `run.sh`, `prepare.sh`, `*.py` | yes | First-party work product. A pinned SHA reconstructs the upstream RTL but not the harness, and the harness is what produced the digest |
| `bench/*/src/` — the upstream clone | no | Third-party RTL under eight different licences, never redistributed by this project. `corpus-runner fetch --run` clones it at the pinned SHA |
| `bench/keccak/*.sv`, `bench/keccak/*.py` | yes | First-party RTL written in this repository |
| Firmware images (`prog.hex`) | no | Upstream content, extracted from upstream's own `test.elf`. `bench/biriscv/prepare.sh` regenerates it |
| Build products (`*.vvp`, `obj_dir/`, simulator binaries, logs) | no | — |

`.gitignore` implements this as an allow-list: everything under `bench/` is ignored
unless it matches a named pattern, so a stray binary or scratch probe cannot be
committed by accident. `bench/*/src/`, `bench/*/obj_dir*/` and `bench/ibex/` are
ignored outright.

## Fetching

`fetch` prints the clone plan; `fetch --run` performs it. Per upstream workload:

```sh
git clone --filter=blob:none --no-checkout <repo> bench/<root>/src
git -C bench/<root>/src fetch --depth 1 origin <sha>
git -C bench/<root>/src checkout --detach <sha>
sh bench/<root>/prepare.sh          # only where the workload has one
```

A workload already on disk still re-runs its `prepare.sh` under `--run`: the script
regenerates deliberately-uncommitted artifacts and is idempotent. `bench/biriscv` is
the only workload carrying one.

Presence is tested on the source files, not on the directory, because `fetch` creates
`bench/<root>/src` and that makes `bench/<root>` exist whether or not the clone
succeeded.

## Reading the run table

The table is fixed-width — workload left-18, tool left-10, grade left-11, median
right-9, two spaces, then the detail column. Parse it by column offset, not by
whitespace.

```
workload           tool       grade          median  detail
sha256             vita       ok             1.193s
verilog-axi        vita       ruled-split         -  ruled split — <the ruling>
```

A median of `-` means nothing was timed: a job that does not match is retired after
its first round rather than repeated, since repeating it cannot change the verdict.

Eight grades, of which exactly three are failures:

| Grade | Failure | Meaning |
|---|---|---|
| `ok` | no | The digest matched the pin |
| `known-gap` | no | A pinned refusal reproduced, diagnostic and all |
| `PROMOTED` | no | A row pinned as refused or split now agrees with the oracle — move its manifest row |
| `ruled-split` | no | The digest disagrees with the oracle on an axis that has been measured and ruled un-arbitrable |
| `absent` | no | Sources are missing; fetch first |
| `REGRESSION` | yes | A row that ran stopped matching, started refusing, crashed or timed out |
| `DRIFTED` | yes | A pinned refusal produced a different refusal |
| `ORACLE-DRIFT` | yes | The oracle itself stopped reproducing the pin |

`PROMOTED` is uppercase and is not a failure. Two distinct digests from one tool
overwrite the detail column with `*** NON-DETERMINISTIC ***` and both digests,
whatever the grade — a flapping tool is a bigger fact than whichever answer the last
round produced.

After the table `run` prints a phase split (one extra `--obs-dir` probe run per
workload, not the timed rounds), then a coverage line. With `--compare` it also prints
one `vita … iverilog … = N.NNx faster|SLOWER` line per workload.

At HEAD the corpus reports `coverage: 10/10`: nine rows grade `ok` and
`verilog-axi` grades `ruled-split`, which is neither a pass nor a failure and reads
that way on every run with its reason on the line.

## Gating without a simulator installed

The pinned digests are recorded in the manifest, so `run` grades a machine that has
`vita` but no iverilog. `--compare` is what needs iverilog, and a missing or failing
iverilog is reported and is not itself a corpus failure. The manifest's hygiene tests
— permissive licence, full 40-character SHA, unique names, a findable digest, an
oracle per row, a reason on every pinned refusal, more than one shape — run in the
normal test suite.
