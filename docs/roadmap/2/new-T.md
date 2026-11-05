(§2 🆕 T · BLOCKED · in Start-order rows) (a generate-case label the i64 fold cannot read is skipped):
* a generate-case label the i64 fold cannot read is skipped
* a read one compares untyped (T1, L06, S06, X13 take `default`; N09 inconsistent)
* an unbound label, which §26.3 makes illegal, is skipped too: undeclared (Z5g `NOPE:` `def 9 bits=4`; iverilog and sv2v ``Unable to bind parameter `NOPE' in `top.gb'``, verilator `Can't find definition of variable: 'NOPE'`) or declared later in the block with no outer object (V14, V27, V30b, Z2c, Z3a `def 9 bits=4`; iverilog `Unable to bind parameter`)
* `generate.rs` `GenItem::Case`
* status: BLOCKED (🆕 U's held half; 🆕 V, whose §4.5.592 attempt was reverted)
  * design (§4.5.581, `fix/gencase-label-domain`, `d4dc9c26`): decide once per instance
  * compare PAIR and WHOLE (§12.5) in the bit domain, decide where they agree, else i64, else no match, never refuse
  * residue: a label no bit fold reads (`case (6'sh30) W6'(P8)`), and an unbound label, which must refuse, not take no match (§4.5.592's round-3 replay reached this skip: R3q, 🆕 V)
  * pins: held `generate_case_and_wildcard_prerequisites.rs`
