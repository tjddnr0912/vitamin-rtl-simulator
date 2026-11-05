(§5.2 Q-IDXSIGNCTX · rank ① · source row 29 r1 lens-diff DF1 (sx01–sx12), r2 lenses B1 / B2 / DB1 (h03b, tw1)) (an index is re-typed unsigned 32-bit by the flatten it is lowered into):
* slot: row 29's prerequisite with Q-IDXVALUE
* slot: shared flatten `elaborate/src/array_geom.rs`, `packed.rs`: D12 file check at start
* an index is re-typed by the flatten it is lowered into: `flatten_word` / `flatten_word_eids` (`elaborate/src/array_geom.rs`) put every unpacked or packed index under synthesized 32-bit UNSIGNED constants (the bound guard `dim_guard`, the coordinate `dim_coord`, the stride `Mul` / `Add`), which IEEE 1800-2017 §11.6.1 / §11.8.2 make the operands' context
* `seal_index_unsigned` (`packed.rs:748`) protects only a narrow signed REPEATABLE index (sign-extended) or a provably unsigned one (`{1'b0, e}`).
* Two halves, measured on PRE 719e37d2 (W/row29/park/cells): (a) a narrower signed index zero-extends and `/` runs unsigned — sx01 `logic [1:0][15:2] b; logic signed [3:0] s = -1; b[s + 2] = 14'h3fff;` `R 0000000`, iverilog = sv2v = verilator `R fffc000`
* sx02 `b[s + 2][5] = 1` `R 0000000`, sv2v = verilator `R 0020000`
* sx03 reads `R xxxx xx x`, sv2v = verilator `R 0f25 09 0`
* sx12 (NBA) `R 0000000`, all three `R fffc000`
* a class field h03b `logic [15:0][5:2] cP; k.f = -1; cP[k.f] = 4'h6;` `R 6000000000000000` = verilator, iverilog `R 0000000000000000` (by hand -1 is out of range)
* tw1 `logic [1:0][5:2] cX [-8:7]; cX[k.f][1] = 4'h6;` E4002, `R 00 00 00`, iverilog = verilator `R 60 00 00`
* x12_keep_cf7 `logic [1:0][7:0] u [1:16]; u[c.f + 4'sd2][1][5:2] = 4'hf;` (c.f = -4'sd1: the 4-bit sum is 1) E4002, `R 0000 0000`, iverilog `R 3c00 0000` = hand (verilator `R 0000 0000`)
* `cR2[i32 / -2][1][1]` (i32 = -4, `logic [1:0][9:2] cR2 [16][2]`) reads `R 03` (element 0), iverilog `R 23`
* (b) an UNSIGNED 32-bit index on a negative-LSB dimension wraps `idx + \|lo\|` into range (`array_geom.rs:92–98`, `:125`; the comment at `:86–90`, "exact for either signedness", holds for a signed index only) — w5_pk_ufn_wh `logic [7:-8][7:0] b; b[fu(x)] = 8'hff;` (x = 32'hFFFF_FFF9) `R 0000000000000000000000000000ff00` (element -7) = verilator, iverilog = sv2v `R 00000000000000000000000000000000`
* y04 `b[fu(x)][3] = 1'b1` `R 00000000000000000000000000000800` = verilator, sv2v `R 00000000000000000000000000000000`
* x_urand_wh `logic [7:-8][5:2] cN; cN[$urandom % 1 - 2] = 4'h6;` `R 0000000006000000` = sv2v = verilator, iverilog `R 0000000000000000` = hand (`$urandom` is unsigned)
* a module net or `$unsigned(...)` index is sealed `{1'b0, e}` and drops right
* row 29's reach (PRE E3009, every attempt wrong) is its F1, and its exclusions are this row's over-exclusion witnesses (the pair matrix W/row29/r2/DF1_MATRIX.tsv: 294 HAZARD rows of 1680, PRE value != hand-IEEE)
* (c) beside the two halves, a dimension whose bound is past `i32` (Q-PKRDLSB r2 lens-sound F1, PRE 719e37d2): the guard and the coordinate take `lo as u32` at 32 bits (`array_geom.rs:100–101`, `:154`), so an index lands modulo 2^32 on a word or an outer packed element: W/qpkrdlsb/r2/lens-sound/b8/w_si `logic [3:0][9:6] q [2147483649:2147483648]; logic [3:0][3:0] z [2147483649:2147483648]; int w = -2147483647; int k = 7;` `q[w]` `q[w][1]` `q[w][1][7]` `q[w][1][k +: 2]` `q[w][1][8:7]` `z[w][1][1 +: 2]` `z[w][1][2:1]` `R a a5c3 c 0 xx xx 10 10`, iverilog `R a a5c3 c 0 10 10 10 10`, hand x on all (ieee1800-2017.txt:6349–6351)
* W/qpkrdlsb/park/f1w `logic [15:0] m [2147483649:2147483648]; int w = -2147483647;` `m[w]` `m[w][7]` `R a a5c3 1` = iverilog = verilator, hand x, sv2v refuses
* W/qpkrdlsb/park/f1o `logic [2147483651:2147483648][3:0] z [2]; logic [2147483651:2147483648][3:0] y; int i = -2147483647;` `z[1][i]` `z[1][i][1 +: 2]` `y[i]` `y[i][2:1]` `R a c xx c 10`, verilator `R a c 10 c 10`, hand x on all, iverilog and sv2v refuse
* the literal f1_cn is the oracles' own split: iverilog reads `q[-2147483647]` x and warns out of bounds, its run-time twin f1_si reads the word
* fix: type each index under its own self-determined width and sign before the flatten and guard it against the declared bounds at a width that cannot wrap, then row 29 needs no audit for these shapes
* not Q-IDXVALUE's seal carrier
* silent-wrong, pre-existing
* source: r3 lens-sound F1 / F4 and lens-diff DF-F1 (W/row29/RULINGS-r3lens.txt)
