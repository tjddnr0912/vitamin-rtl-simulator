# bench/picorv32 — workload corpus recipe

| | |
|---|---|
| Repo | https://github.com/YosysHQ/picorv32 |
| Pinned SHA | `a473fc8fca393771d83b0ffcf0b14db3393339d8` |
| License | **ISC** (`src/COPYING`) |
| Clone | `bench/picorv32/src/` — verified byte-identical to upstream at that SHA |
| Top | `tb` in `tbd.v` (written here, not upstream) |
| Oracle | iverilog 13.0 — **not verilator** (see below) |
| Expected | `DIGEST=68d30f61bf9bf1d4` |

## Two testbenches, on purpose

`tb.v` is the ORIGINAL and is kept verbatim: the numbers published in
`docs/study/01-interpreted-vs-compiled.md` were measured with it, and rewriting it
would silently invalidate them. It prints `trap=%b addr=%h` — **final state only**,
which cannot see a divergence the core later overwrites.

`tbd.v` is the corpus testbench. It folds the whole memory bus into a rotate-xor
accumulator on **every** cycle, so the digest has cycle resolution.

Two traps are baked into that accumulator, both found by measurement:

- picorv32 drives `mem_addr`/`mem_wdata` to **x** whenever `mem_valid` is low, and
  `mem_wdata` to x whenever no strobe selects it. An ungated accumulator xors x into
  every bit within one cycle and the digest comes back `xxxxxxxxxxxxxxxx`.
- `trap`, `mem_instr`, `mem_ready` and `mem_wstrb` are x during reset, so each is
  folded through an X-proof form (`===` or a gated `&&`).

## Commands (from `bench/picorv32/`)

```sh
iverilog -g2012 -o picorv32.vvp tbd.v src/picorv32.v && vvp picorv32.vvp +N=400000
../../target/release/vita tbd.v src/picorv32.v +N=400000
```

`+N=400000` puts iverilog at ~6.6 s, mid-window. Startup is amortised: the whole run
is one 256-word memory and a 6-instruction loop, so wall time is linear in N.

## Timings (2026-08-23, macOS arm64, interleaved, first round discarded)

| tool | median | vs iverilog |
|---|---|---|
| iverilog 13.0 | 7.046 s | 1.00x |
| **vita** (one-shot) | **4.888 s** | **1.44x faster** |

Three timed samples, interleaved, first round discarded. The cross-corpus table —
and the only place the whole set is quoted together — is
`docs/study/03-workload-corpus.md`.

## The program has to STORE

The first version of this testbench ran a loop of `addi`/`add`/`beq` and nothing else.
It looked fine — deterministic, cycle-resolution, agreed with iverilog. It was also
**completely insensitive to the core**: mutating picorv32's adder
(`reg_op1 + reg_op2` -> `+ 1`) left the digest **byte-identical**, because no computed
register value ever reached the memory bus, and the bus is all the digest can see.
The digest was a program-counter trace wearing the name of a design check.

The program now stores `x5` on every lap and carries `x1` across laps, so ALU results
flow out on `mem_wdata`. The same mutation now moves the digest. Verify with:

```sh
sed -i '' "1240s/reg_op1 + reg_op2/reg_op1 + reg_op2 + 32'd1/" <copy>/picorv32.v
```

## Verilator is NOT an oracle here

`verilator --binary --timing` produces `DIGEST=d1a61ec37f37e3a8`, which is not a
disagreement about picorv32 — it is a different design. The register file starts
uninitialised and the core reads x from it, so this workload genuinely depends on
4-state semantics that verilator's 2-state model approximates away. iverilog and vita
agree exactly.
