# 00 · Map of the IEEE HDL standards

## Quick reference

| Standard | Edition | Published | Main changes | Status |
|---|---|---|---|---|
| IEEE 1364 (Verilog) | 1995 | 1995 | First edition — 4-state logic, module/always/assign/initial, system tasks such as `$display` | superseded |
| IEEE 1364 | 2001 | 2001 | `generate`, signed nets, `always @*`, built-in arithmetic operators, named parameter overrides, stronger file I/O | superseded |
| IEEE 1364 | 2005 | 2005 | Last standalone Verilog standard. Ambiguous definitions repaired and the incompatibilities with 1800-2005 resolved. `uwire` added | merged into 1800-2009 |
| IEEE 1800 (SV) | 2005 | 2005 | First SV edition — published alongside 1364, not merged with it. `logic`, `always_ff/comb/latch`, interfaces, struct/union/enum, SVA, classes, randomization | superseded |
| IEEE 1800 | 2009 | 2009 | **Absorbs IEEE 1364-2005 in full** — SV becomes a superset of Verilog in a single document. Stronger dynamic arrays and queues, improved assertions | superseded |
| IEEE 1800 | 2012 | 2012 | `unique if` / `priority if` refined, errata fixed. Mostly a consistency-and-clarity edition | superseded |
| IEEE 1800 | 2017 | 2017-12-06 | Error fixes plus small refinements. **Free access through the IEEE GET Program begins** | active (widely deployed) |
| IEEE 1800 | 2023 | 2024-02-28 | `ref static` argument direction added, language extensions plus errata. **Free through the IEEE GET Program** | latest |
| IEEE 1076 (VHDL) | 1987 | 1987 | First edition — developed at the request of the DoD. Integer, real, logic, character and time types, bit_vector, string | superseded |
| IEEE 1076 | 1993 | 1993 | More consistent syntax, ISO-8859-1 character set, `xnor` added, `postponed process` introduced | superseded |
| IEEE 1076 | 2000 | 2000 | Small edition — protected types introduced (comparable to a C++ class) | superseded |
| IEEE 1076 | 2002 | 2002 | Small edition — buffer-port rules relaxed | superseded |
| IEEE 1076 | 2008 | 2009-01-26 | **Major revision** — absorbs IEEE 1164/1076.2/1076.3, integrates VHPI, adds a PSL subset and generics on packages and subprograms | active |
| IEEE 1076 | 2019 | 2019-12-23 | Integers widened to 64 bits, conditional analysis, generics on protected types, stronger PSL, extended TEXTIO. **Free through the IEEE GET Program** | latest |
| IEEE 1164 (std_logic_1164) | 1993 | 1993 | Standalone standard — the `std_logic` 9-value system, `std_logic_vector`, resolution functions | merged into 1076-2008 |

## How the languages relate

**SystemVerilog ⊃ Verilog (from 2009 on)**

From IEEE 1800-2009 on, every Verilog (IEEE 1364) construct is defined inside the SystemVerilog standard itself. 1364 no longer exists as a standalone standard. That is why one SystemVerilog front end covers Verilog RTL as well, rather than two.

**VHDL ⊃ std_logic_1164 (from 2008 on)**

IEEE 1076-2008 absorbed IEEE 1164. A `use ieee.std_logic_1164.all;` clause still works, but the definitions now originate in 1076. IEEE 1164 is superseded and receives no separate updates.

**VHDL is a language of its own, independent of SV**

Its syntax, semantics, type system and library ecosystem are entirely different. The two languages require fully separate front ends up to the point where they meet a shared IR (sim-ir).

## Which editions these notes are written against

| Language | Baseline edition | Later editions |
|---|---|---|
| SystemVerilog | IEEE 1800-2017 | 1800-2023 consulted for individual issues |
| Verilog (inside SV) | Everything 1800 absorbed (= all of 1364-2005 RTL) | — |
| VHDL | IEEE 1076-2008 | 1076-2019 consulted for individual issues |

These are the editions the notes in this folder cite; they are not a statement of what the simulator implements. A VHDL front end is conditional and not scheduled — see [01-goals-and-scope.md](../01-goals-and-scope.md) for the scope of the project, and [manual/003_language-reference.md](../../manual/003_language-reference.md) for the construct-by-construct status of what vita supports today.

## Freely available texts (IEEE GET Program)

Sponsored by Accellera, the following standards can be downloaded at no cost.

| Standard | Access |
|---|---|
| IEEE 1800-2017 (SystemVerilog) | [IEEE Xplore GET](https://ieeexplore.ieee.org/browse/standards/get-program/page/) |
| IEEE 1800-2023 (SystemVerilog) | [IEEE Xplore GET](https://ieeexplore.ieee.org/browse/standards/get-program/page/) / [Accellera](https://www.accellera.org/downloads/ieee) |
| IEEE 1076-2019 (VHDL) | [IEEE Xplore GET](https://ieeexplore.ieee.org/browse/standards/get-program/page/) / [Accellera](https://www.accellera.org/downloads/ieee) |
| IEEE 1666-2023 (SystemC) | [Accellera](https://www.accellera.org/downloads/ieee) (outside the scope of this project) |

IEEE 1364 (1995/2001/2005) and IEEE 1164 (1993) are superseded, are not part of the GET Program, and must be purchased. Most of their content can be read in the IEEE 1800-2017/2023 LRM instead.

## Sources

- This project's spec, §10 (structure)
- research-log: [hdl-standards-versions-2026-05-28.md](../../history/research-log/hdl-standards-versions-2026-05-28.md)
- IEEE 1800-2023: https://standards.ieee.org/ieee/1800/7743/
- IEEE 1364-2005 Xplore abstract: https://ieeexplore.ieee.org/document/1620780
- IEEE 1076-2019 Xplore abstract: https://ieeexplore.ieee.org/document/8938196
- Accellera GET Program downloads: https://www.accellera.org/downloads/ieee
