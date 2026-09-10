# 00 · Verilog (IEEE 1364) Reference

This folder is a syntax and structure reference for Verilog (IEEE 1364-2005) — the
subset that SystemVerilog absorbed.

## Files

| # | File | Topic |
|---|---|---|
| 01 | lexical | tokens, identifiers, number literals, comments |
| 02 | data-types | net/wire/reg/integer/real/vector/parameter |
| 03 | expressions-operators | operators, precedence, signed/x/z |
| 04 | modules-hierarchy | module/port/instantiation/parameter/generate |
| 05 | behavioral | initial/always/blocking(=)/nonblocking(<=) |
| 06 | procedural-statements | if/case/for/while/repeat/forever/fork-join |
| 07 | tasks-functions | task/function/automatic |
| 08 | gate-level | primitives/UDP/drive strength |
| 09 | compiler-directives | `` `timescale/`define/`ifdef/`include `` |
| 10 | system-tasks | overview for this folder + cross-link to ../system-tasks/ |
| 11 | synthesizability | synthesizable / conditional / non-synthesizable mapping |

## Version history summary

| Standard | Main additions |
|------|-------------|
| IEEE 1364-1995 (Verilog-1995) | the base Verilog language |
| IEEE 1364-2001 (Verilog-2001) | signed types, ANSI port style, generate, the `**` operator |
| IEEE 1364-2005 (Verilog-2005) | uwire, minor corrections, the last standalone standard |
| IEEE 1800-2009 onwards (SystemVerilog) | absorbs 1364-2005 — no separate 1364 has been published since |

## Scope of this folder

Verilog is a strict subset of SystemVerilog, so a SystemVerilog front end reads
Verilog source with no separate Verilog mode. This folder is the reference for an
RTL author who deliberately stays inside the Verilog subset.

What SystemVerilog has and Verilog does not: `logic`, `always_ff/comb/latch`,
`interface`, `struct/union`, `package`, `clocking`, assertions.

For what vita itself supports, see
[docs/manual/003_language-reference.md](../../../manual/003_language-reference.md) —
these notes describe the language standards, not the implementation.

## Sources

- This spec §10 (vitamin project spec = `docs/preview/`)
- IEEE 1364-2005, IEEE 1800-2017
