(§5.2 Q-PCTC-ENG · rank ① · source P-PCTU re-plan 3 (W/ppctu/PLAN3.txt §7.3; W/ppctu/replan3/cells/k1p_C)) (`%C` in a scan format is not read):
* symptom: `%C` in a scan format is not read: the engine has no `C` arm (`sim-engine/src/sched/scan_arm.rs:408` `_ => break`), so `$sscanf("A", "%C", a)` returns `0 11111111` and `$sscanf("AB", "%C%c", a, b)` `0 11111111 xxxxxxxx` (PRE 01cac4b3) where IEEE 1800-2017 §21.3.4.3 (ieee1800-2017.txt:29273: "%c (or %C)") reads `%c`: verilator 5.052 `1 00000041` / `2 00000041 00000042`, vita's own `%c` `1 00000041`
* oracles: iverilog 13.0 rejects it (`$sscanf() was given an invalid format code: %C`, as every upper-case letter)
* note: §4.5.630's head walk passes `%C` as a letter the engine lacks (a `%u` after it is never reached, kept)
* status: silent-wrong, pre-existing
