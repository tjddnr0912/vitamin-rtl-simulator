# 00 · VHDL (IEEE 1076) Reference

This folder is a reference for VHDL (IEEE 1076-2008) syntax, packages and synthesizability.

## Files

| # | File | Topic |
|---|---|---|
| 01 | [lexical](01-lexical.md) | tokens, identifiers, comments, literals |
| 02 | [types](02-types.md) | scalar/composite, std_logic_1164, numeric_std |
| 03 | [objects](03-objects.md) | signal/variable/constant/generic + port modes |
| 04 | [design-units](04-design-units.md) | entity/architecture/package/configuration/library |
| 05 | [concurrent](05-concurrent.md) | process, concurrent assignment, component, generate |
| 06 | [sequential](06-sequential.md) | if/case/loop, wait, variable assignment |
| 07 | [subprograms](07-subprograms.md) | function/procedure |
| 08 | [packages-libraries](08-packages-libraries.md) | ieee, std_logic_1164, numeric_std, std |
| 09 | [synthesizability](09-synthesizability.md) | the VHDL synthesizable/conditional/non-synthesizable map |

## Relationship to this project

vita has no VHDL front end; its input language is Verilog/SystemVerilog. A VHDL front end —
a separate parser, the 9-value `std_logic` value domain, GHDL as the oracle — is a
conditional, long-term item in [ROADMAP](../../../ROADMAP.md) §7, not part of the
implementation. These notes document the language itself, so the standard can be consulted
while implementing; for what vita accepts, the authority is
[manual/003_language-reference.md](../../../manual/003_language-reference.md).

## Sources

- This project's spec, §10
- IEEE 1076-2008
