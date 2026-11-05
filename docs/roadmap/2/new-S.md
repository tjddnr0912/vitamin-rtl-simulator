(§2 🆕 S · BLOCKED · in Start-order rows) (a constant `inside` / `==?` compares at the left operand's width):
* (a) a constant `inside` / `==?` compares at the left width (`const_str.rs` `const_compare_special`, `const_wide.rs` `fold_region`) — as a declaration range bound PRE keeps the one-bit default (a Q-DECLSELBOUND witness; the wide fold reads an `inside` element as `==`): `[(8'h0C inside {4'b1?00}):0]` IEEE n=2, PRE 1
* right by default T/s581 q4__B2 B3 B6
* `==?` with x in the LEFT operand declines (`(4'b1x00 ==? 4'b1100)` IEEE illegal, PRE n=1; `(4'b1x10 ==? 4'b1x10) + 1`, `(4'b1x10 ==? 4'b1z10) + 1`, `(4'b1x10 !=? 4'b0x10) + 1` IEEE n=3, `(4'b1x10 ==? 4'b0x10) + 1` IEEE n=2, all PRE 1; `(4'b1x10 ==? 4'b1010) + 1` IEEE illegal, PRE 1)
* (b) a run-time x/z element compares with `==` (`wildcard_eq.rs` `inside_value_cmp`)
* status: BLOCKED ((a) 🆕 W, T, U · (b) a run-time `==?` primitive)
  * retry (§5.2 row 6, one slice with 🆕 W after 🆕 AI): the i64 half (`eb9d3b69`, branch `fix/gencase-label-domain`) with every `const_wildcard_i64` decline falling back to PRE's compare and a left operand holding a call kept on PRE's compare (🆕 AE: A1–A3, xADD, xTRN, xSYS)
  * evidence: §4.5.593 ran §4.5.580's 666 variants on that cut: 0 correct→other per label
  * the parameter channel (A5, xDP, xPT: PRE `PA=-4`, 3 oracles `x`) is 🆕 AE's
  * re-run round-3 cells, MC4, the 174-cell matrix
  * the wide half (LP, st1, two compares under `||`: S9) after 🆕 T, U (§5.2 row 7)
