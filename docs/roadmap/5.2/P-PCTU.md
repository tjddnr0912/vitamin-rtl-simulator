(§5.2 P-PCTU · rank ① S · source Q-STRBYTES grounding (c21), re-plan probe q_pctu, W/ppctu/PLAN2.txt) (`%u` / `%z` format specifiers write nothing):
* slot: PARKED 2026-10-29 behind Q-PCTUREC, Q-CLSRET and rows 74 / 75
* slot: section A, the loud half, landed §4.5.630
* `%u` / `%z` write nothing (`sim-engine/src/builtins/render.rs:272`): IEEE 1800-2017 §21.2.1.2 (ieee1800-2017.txt:28676–28677) writes the 2-value (`%u`, x / z → 0) and 4-value (`%z`) binary representation in native endian order, in units of 32 bits: PRE 9d60771d `$sformatf("%u", 8'h41)` has length 0 (iverilog 13.0 = sv2v `1 41`, verilator 5.052 `4 41 00 00 00`)
* `$fwrite(fd, "%u\|%z\|%u", 32'hC0937F81, 8'hC8, 8'h41)` writes `7c 7c` (verilator `81 7f 93 c0 7c c8 00 00 00 00 00 00 00 7c 41 00 00 00`; iverilog, sv2v and verilator agree on the 32-bit word and `%z`'s 8 bytes)
* the byte count follows IEEE (4 bytes per 32-bit unit, = verilator), iverilog = sv2v's 1 byte recorded as the split
* fix: the two arms over the byte buffer (Q-STRBYTES, §4.5.629), word layout and x / z encoding measured against all three, every display task that takes them
* §3.c's "`%u` / `%z` … documented choices" line and the manual's "no text is emitted" (005, 006) change with this row
* a licensed `%u` / `%z` node (row 26) answers iverilog = sv2v's `1 41` through the degenerate path today and is not routed onto the engine before this row.
* PARKED 2026-10-29 (W/ppctu/PLAN2.txt): the encoder would move 33 lines from writing nothing to a different wrong value (ER §2.2) — Q-PCTUREC (28 record lines), Q-CLSRET and rows 74 / 75 (5 handle lines) first
* the candidate is W/ppctu/plan2/patches/cb3.patch (CB3 v3: the encoder, an integral `$sformat` / `$swrite` destination receiving the bytes, a format held in a variable or a later argument writing nothing, the scan gate, the call proxy) with W/ppctu/plan2/expected.tsv
* re-derived after them (ER §3.6).
