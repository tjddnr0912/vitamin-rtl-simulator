(§5.2 Q-STRWIDTH · rank ① · source Q-RELDELAY r5 design review N-E1; Q-STRBYTES re-plan d2 (Q-EMPTYLIT)) (the empty string literal folds as 1 bit instead of 8):
* the empty string literal `""` is folded as 1 bit (`elaborate/src/literal.rs` `str_const_from_bytes` width 0, then `c.width.max(1)` in `sim_ir::selfwidth`) where IEEE 1800-2017 §11.10.3 makes it `"\0"`, 8 bits, so `"" - 4'd1` is 15 instead of 255: sE_empty_minus_solo `wire [255:0] y; assign y["" - 4'd1] = a;` vita `S y=z1` ({y[255], y[15]}), iverilog = sv2v `1z`
* sE_empty_minus_vs255 (+ `assign y[255] = b;`) vita `01`, iverilog = sv2v `xz`
* sE_empty_minus_vs15 (+ `y[15]`) vita E3001, iverilog = sv2v = verilator `10`
* sE2 `"" - 1'b1` + `y[1]` vita E3001, iverilog = sv2v = verilator `10`
* the read sE3 `x["" - 4'd1]` vita `S r=0` (bit 15), iverilog = sv2v = verilator `r=1` (bit 255)
* a context-sized use (`localparam [15:0] V = "" - 4'd1` = 65535) is right
* the multi-driver check reads the same value as the store (no false overlap of its own)
* in a packed concatenation and `$bits` (re-measured 2026-10-27, PRE 9d60771d): `$bits("")` = 1 and `$bits({"P", "", "P"})` = 17, a 32-bit `w = {"P", "", "P"}` `%h` `0000a050`, where iverilog 13.0 = verilator 5.052 read 8 and 24 bits, `00500050` (IEEE 1800-2017 §11.10.3, ieee1800-2017.txt:12849: `""` is `"\0"`, one 8-bit character)
* `r = {"P", "", "P"}` into a string gives `[\xef\xbf\xbdP] 2` (iverilog = sv2v = verilator `[PP] 2`) and `$display("[%s]", {"P","","P"})` `[ \xc2\xa0P]` (all three `[P P]`)
* a `localparam` of that concatenation is E3009 (iverilog 24 bits)
* census the uses of `""` first (a string use stays empty: IEEE 1800-2017 §6.16, ieee1800-2017.txt:4470–4471)
* row 48: the core already reads `""` as 8 bits (`const_wide.rs` `str_lit_bits`), so two of its three constant ≠ run-time parity cells are this row (`({"" - 1'b1} == 255)` folds 4, runs 2)
* the `[%s]` exit needs no `render.rs` change (a NUL byte prints a space on all four tools: convert at the integral consumer)
* a V′ code row with no lane-2 file (outline W/row48/DESIGN2.txt D5)
* pre-existing
