(§3.b begin-keywords-version · OPEN · in Parser accept) (a `begin_keywords` version does not narrow the keyword set):
* symptom: vita applies the full keyword set under `` `begin_keywords "1364-2001"`` (`docs/manual/003_language-reference.md:203`), so a 1364 design that names a parameter `bit` is refused
* witness: s3a `parameter signed bit = -1` under `` `begin_keywords "1364-2001"``: PRE 7 error lines, S1v-Q one more first line naming `bit` a type keyword (W/batch1/lens-sound/REPORT.txt N4)
* oracles: iverilog, sv2v → iverilog and verilator `A bit=-1`
* status: OPEN
