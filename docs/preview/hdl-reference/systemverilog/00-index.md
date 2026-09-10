# 00 · SystemVerilog (IEEE 1800) Reference

This folder is a reference for the features SystemVerilog (IEEE 1800-2017) adds or extends —
Verilog absorbed, plus the extensions.

## Files

| # | File | Topic |
|---|---|---|
| 01 | data-types | logic/bit/int, enum/struct/union/typedef, 2-state vs 4-state |
| 02 | arrays | packed/unpacked/dynamic/associative/queue + array methods |
| 03 | procedural | always_comb/_ff/_latch, unique/priority, foreach |
| 04 | interfaces | interface/modport/clocking block |
| 05 | packages | package/import/export/$unit |
| 06 | classes-oop | class/inheritance (verification-only, not synthesizable) |
| 07 | assertions-sva | immediate/concurrent assertion, property, sequence |
| 08 | functions-tasks | cross-link to the system tasks SV adds |
| 09 | synthesizability | SV synthesizable/conditional/non-synthesizable map |

## Relationship to Verilog

SV is a superset of Verilog (IEEE 1364). The Verilog documents (`../verilog/`) cover that
subset; this folder covers what SV adds or changes.

## Sources

- This project's spec, §10
- IEEE 1800-2017
