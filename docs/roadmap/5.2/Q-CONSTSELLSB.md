(§5.2 Q-CONSTSELLSB · rank ① · source Q-AESINK review r1 (dC1); row 29 page half r3 lens-sound C1) (constant interpreter select writes ignore the declared LSB and direction):
* slot: R7.1 · L1 (batch 7, after batch 6), on BIND-2: a local's `RangeFacts`, read by select writes and the read twin (owner, 2026-11-09, the record-first order; it was any · L2)
* the constant interpreter's select write numbers from 0 whatever the declaration says: `ConstWidths` holds a width and a sign, no LSB or direction, so `exec_const_select_write` (`elaborate/src/const_fn.rs`) stores `t[i]` at storage bit `i`: k_off_b `bit [5:2] t; t[2] = 1; t[3] = 1;` `00001100`, verilator `00000011`, iverilog and sv2v `0000xx11`
* k_asc_b `bit [0:3] t` `00001101`, all three `00001011`
* a 4-state local is the same value under Q-AESINK's `numbered` flag: v1_loop0_w1 `11111111`, all three `x1111111`
* IEEE 1800-2017 §11.5.1
* fix: record the LSB and the direction beside the width, and the read twin (§2 "A non-zero-LSB local in a concat") reads the same record
* the record's LSB is the declaring scope's, not the call's (sc_lsb_gif)
* a child-local function in a port range reaches a row 29 page-shape output actual (row 29 page half r3 lens-sound C1, W/row29/page/lens3-sound/c/f/g10_formal_fn; a source inside the child is a filing, RULINGS-plan3b O4 (a), beside y01–y05 of W/row29/page/r3/docs.patch): g10_formal_fn `module c (output logic [g()-1:0] o); function automatic int g(); bit [5:2] t; t = 0; t[2] = 1; t[3] = 1; return t; endfunction assign o = '1; endmodule` with `logic [1:0][15:2] r; c u(.o(r[1][9:6]));` (`g()` 12 in vita, 3 by hand) PRE `t.sv:7:5: error[VITA-E3009] E-ELAB-UNSUPPORTED: nested lvalue select (v1: single-level) [in top]`, r3 `R xxxxxx1111xxxxxxxxxxxxxxxxxx`, iverilog 13.0 `R zzzzzz0111zzzzzzzzzzzzzzzzzz` (`t.sv:7: warning: Port 1 (o) of module c expects 3 bit(s), given 4.`), sv2v → iverilog the same, verilator 5.052 `R 0000000111000000000000000000`
* the 1-D twin `R xxxxxx1111xxxx` on PRE
* the page shape stays loud on main (row 29's page half parked)
