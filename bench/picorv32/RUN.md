# bench/picorv32 — workload corpus recipe

The corpus row `picorv32`: the reference RISC-V workload. A 256-word memory, a short
program the core runs for N cycles, and one `DIGEST=` line folded from the memory bus
on every cycle. The cross-corpus timing table is
[docs/study/03-workload-corpus.md](../../docs/study/03-workload-corpus.md);
`cargo run -p corpus-runner -- run --filter picorv32 --compare` reproduces it.

| | |
|---|---|
| Repo | https://github.com/YosysHQ/picorv32 |
| Pinned SHA | `a473fc8fca393771d83b0ffcf0b14db3393339d8` |
| Licence | ISC (`src/COPYING`) |
| Clone path | `bench/picorv32/src/` — byte-identical to upstream at that SHA |
| Top module | `tb`, in `tbd.v` (written here, not upstream) |
| Oracle | iverilog 13.0 — not verilator, see below |
| Expected | `DIGEST=68d30f61bf9bf1d4` at `+N=400000` |

Reconstruct the clone:

```sh
cd bench/picorv32
git clone https://github.com/YosysHQ/picorv32 src
git -C src checkout a473fc8fca393771d83b0ffcf0b14db3393339d8
```

## Two testbenches, on purpose

`tb.v` is the original and is kept verbatim: the numbers published in
[docs/study/01-interpreted-vs-compiled.md](../../docs/study/01-interpreted-vs-compiled.md)
are measured with it, and rewriting it would silently invalidate them. It prints
`trap=%b addr=%h` — final state only, which is blind to a divergence the core later
overwrites.

`tbd.v` is the corpus testbench, and it is the one the manifest names. It folds the
whole memory bus into a rotate-xor accumulator on every cycle, so the digest has cycle
resolution.

Two hazards are handled inside that accumulator, both found by measurement:

- picorv32 drives `mem_addr` and `mem_wdata` to x whenever `mem_valid` is low, and
  `mem_wdata` to x whenever no strobe selects it. An ungated accumulator xors x into
  every bit within one cycle and the digest comes back `xxxxxxxxxxxxxxxx`, which
  compares equal to nothing. The bus payload is therefore folded in only while the
  signal that qualifies it holds.
- `trap`, `mem_instr`, `mem_ready` and `mem_wstrb` are x during reset, so each is
  folded through an X-proof form (`===` or a gated `&&`). The handshake bits are folded
  unconditionally, which is what keeps the digest cycle-resolution.

## Commands (from `bench/picorv32/`)

```sh
iverilog -g2012 -o picorv32.vvp tbd.v src/picorv32.v && vvp picorv32.vvp +N=400000
../../target/release/vita tbd.v src/picorv32.v +N=400000
```

`+N=400000` puts the iverilog reference in the middle of the 3–15 s band. Startup is
fully amortised: the whole run is one 256-word memory and a short loop, so wall time is
linear in N.

## The program has to store

The digest can only see the memory bus, so a program that computes without storing
produces a program-counter trace wearing the name of a design check. The program in
`tbd.v` stores `x5` on every lap and carries `x1` across laps, so ALU results flow out
on `mem_wdata` and a mutation of the core's adder moves the digest.

Verify that the workload is still gating by mutating the adder in a copy of the RTL and
re-running — `src/picorv32.v:1240` is
`alu_add_sub = instr_sub ? reg_op1 - reg_op2 : reg_op1 + reg_op2;`:

```sh
sed -i '' "1240s/reg_op1 + reg_op2/reg_op1 + reg_op2 + 32'd1/" <copy>/picorv32.v
```

The digest must change. A digest that survives a mutation of its own design is
measuring nothing and looks exactly like one that is.

## Verilator is not an oracle here

`verilator --binary --timing` produces `DIGEST=d1a61ec37f37e3a8`, which is not a
disagreement about picorv32 — it is a different design. The register file starts
uninitialised and the core reads x from it, so this workload genuinely depends on
4-state semantics that verilator's 2-state model approximates away. iverilog and vita
agree exactly, and the manifest records iverilog as the sole oracle for this row.
