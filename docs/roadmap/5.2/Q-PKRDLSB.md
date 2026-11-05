(§5.2 Q-PKRDLSB · rank ① · source row 29 r3 lens-diff N1 (G_div_R2, G_div_RX, r_k11, u0r_q06, u0r_q10)) (a part-select of a non-zero-LSB packed element of an array word reads wrong bits):
* slot: PARKED 2026-11-02 behind Q-IDXVALUE
* slot: the slot is kept while parked
* slot: recommended high: a literal-index read of legal code
* slot: row 29's read half, its r1–r3 read twin answered it
* symptom: an element-then-part-select READ of a non-zero-LSB packed element of an unpacked-array word reads the wrong bits, at literal indices too: `logic [3:0][5:2] q [2]; logic [3:0][9:2] w [2];` (q[1] = 16'h4321, w[1] = 32'h44332211) `q[1][1][4:3]` `w[1][1][9:4]` print `R X x2` on PRE 719e37d2, iverilog 13.0 = sv2v = verilator 5.052 `R 1 08` (= hand)
* pins: the bare packed `p[1][4:3]` and the LSB-0 `z[1][1][3:2]` are right (W/row29/park/cells/rdlsb)
* the row 29 r3 lens-diff N1 reads are this row: G_div_R2 `rr = cR2[i32 / -2][1][1][9:4]` `R x0`, iverilog = sv2v = verilator `R 08` (its literal twin `cR2[2][1][1][9:4]` `R x2`, iverilog `R 08`; the index half is Q-IDXSIGNCTX), G_div_RX `cRX[i32 / -2][1][9:4]` on `[-8:7]` `R x8`, iverilog = sv2v `R 2a` (literal twin `R xa`, iverilog `R 2a`), r_k11 `r = cR[1][k.f][4:3]` (k.f = 1) `R X`, verilator `R 1` (literal twin `cR[1][1][4:3]` `R X`, iverilog `R 1`), u0r_q06 `cQ[k.f + 4'sd2][1][4:3]` and u0r_q10 `cQ[k.f][1][4:3]` `R X`, iverilog = verilator `R 1`
* evidence: row 29 r1–r3 read them right (r3 lens-diff n1 MOVED_VALUE 23), r3 fell back to these bytes where its twin declined
* IEEE 1800-2017 §11.5.1
* fix: the read twin of row 29 (`packed_elem_resid`, W/row29/r3/row29-r3.patch) for an index the seal leaves exact, a literal first
* status: silent-wrong, pre-existing
* status: PARKED 2026-11-02 behind Q-IDXVALUE (slot kept): three rounds, three BLOCKINGs on the select's own index: F-1 u33 on a non-zero lo
* record: r1's F-A F-B F1 F2
* r2's F-W (a `with` reduction admitted and read at the element width) + r2's F1 (an outer or word dimension whose bound is past `i32` reads modulo 2^32).
* The root is shared and pre-existing: the engine reads an offset modulo 2^32 and the normalization runs at 32 bits, so a consumer must prove exactness per read
* REPLAN's predicate held on 4.8 M model reads, D1, rr and rr3, and F-W broke premise 1 (REPLAN §9: `canonical_self_width` gives the engine's width on every producer) and r2's F1 broke E4 (REPLAN §2: both leaf bounds in `i32`, which tests the leaf and no other dimension).
* PRE keeps the raw path, W/qpkrdlsb/replan and r2's tree (W/qpkrdlsb/r2/tree-final.patch, md5 b073fcc9c965e66bc16f5b5abecb71d6) are the record
* re-entry: re-derive after Q-IDXVALUE, do not restore the patch (ER §3.6).
* The price (W/qpkrdlsb/RULINGS-lens-r2.txt): main keeps the silent-wrong literal read `q[1][1][4:3]`, and r2's 9,884 moved rows do not land.
* F-W: `sim_ir` `selfwidth.rs:402–416` types `ArrSum` .. `ArrXor` by the handle net and never reads args[1], the engine takes the with-expression's width (`sim-engine/src/eval/sysfunc.rs:176`) and `sealed_signed_index` applies `$signed` at the table's width (lens-diff a2, a6: 157 reads r>W or W>W moved, every one a `with` clause; cells W/qpkrdlsb/r2/lens-diff/p1, D7 W/main_d7/2026-11-02/qpk-fw): w_sum2 `logic [3:0][19:16] q [2]; logic signed [3:0] sar4 [4];` (7, 7, 0, 0; q[1] = 16'hA5C3) `q[1][1][sar4.sum() with (item + 1) +: 2]`, index 18, in range: PRE `R a 11`, r2 `R a xx`, verilator `R a 11` (`Y 18`), twin `q[1][1][18 +: 2]` r2 `R c 11`, hand 11 (ieee1800-2017.txt:7225–7229), iverilog `w_sum2.sv:14: syntax error`, sv2v `w_sum2.sv:13:38: Parse error: unexpected token 'with' (KW_with)`
* witness: its `R d`, `int ia [4]` (sum 2^32 + 7) `logic [3:0][9:6] r [2];` `r[1][1][ia.sum() with (longint'(item)) +: 2]`: PRE `R d xx` = hand, r2 `R d 10`, verilator `R d 10` with `Y 18 7` (it keeps 32 bits against :7229, oracle split, hand)
* witness: t21a `logic [3:0][3:-2] q [2]; q[1][1][sar4.sum() with (int'(item)) +: 2]`, index 14, out of range: PRE `R 1 xx` = hand, r2 `R 1 01`, bare twin `R 2 01` on PRE and r2, verilator `R 1 01` (2-state, not discriminating) with `Y 14` `Z 14` where vita prints `Y 14` `Z -2`, iverilog `t21a.sv:13: syntax error`, sv2v `t21a.sv:10:20: Parse error: unexpected token 'with' (KW_with)`
* note: the unsigned receiver `ar4.sum() with (int'(item))` declined at E1, unmoved
* the arm is Q-WITHREAL's width twin.
* r2's F1 (W/qpkrdlsb/r2/lens-sound/b9, D7 W/main_d7/2026-11-02/qpk-f1; hand xx, ieee1800-2017.txt:6349–6351, :6361 → §11.5.1): f1_si `logic [3:0][9:6] q [2147483649:2147483648]; int w = -2147483647; int k = 7; q[w][1][k +: 2]` PRE `R a xx`, r2 `R a 10`, iverilog `R a 10`, verilator `R a 10`
* witness: f1_cn the literal `q[-2147483647][1][k +: 2]` PRE `R a xx`, r2 `R a 10`, iverilog `R a xxxxxxxxxxxxxxxx` (warning: returning 'bx for out of bounds array access q[-2147483647].), verilator `R a 10`
* witness: f1_s4 `q [4294967295:4294967294]; logic signed [3:0] w = -1` as f1_si
* witness: f1_e `logic [2147483651:2147483648][9:6] q [2]; int i = -2147483647; q[1][i][k +: 2]` PRE `R a xx`, r2 `R a 10`, verilator `R a 10`, iverilog refuses (a variable index "is not allowed in a constant expression")
* witness: f1_em the same `q[1][i][8:7]` PRE E3009, r2 `R a 10` (loud → wrong), verilator `R a 10`, iverilog refuses
* witness: f1_eb2 `logic [4294967299:4294967296][9:6] q [2]; logic [1:0] i = 1; q[1][i][8:7]` PRE E3009, r2 `R a 10`, iverilog refuses, verilator refuses (`Value too wide for 32-bits`)
* pins: control f1_ctl `logic [31:0] w = 32'h8000_0001`, in range, r2 `R a 10` = iverilog = verilator = hand
* oracles: sv2v refuses a number past 32 bits on these cells
* scope: the flatten's half is Q-IDXSIGNCTX (c).
* G15 (r2 lens-sound S0): a select width `const_bound_u32` does not answer: W/qpkrdlsb/r2/lens-sound/s0i/w_size `logic [3:0][0:3] q [2]; logic [3:0][0:3] p; int k = 2; logic [1:0] a2; q[1][1][k +: $size(a2)]` (q[1] = p = 16'hA5C3, second token the bare twin `p[1][k +: $size(a2)]`) PRE = r2 `R a 11 00`, iverilog `R a 00 00`, hand 00.
* A packed-struct member as an element index is this row's read with no defect of its own (r2's draft Q-MEMBIDXRD; W/qpkrdlsb/park/nine; PRE 719e37d2): n11_m_pos `logic [3:0][5:2] q [4]; typedef struct packed { logic signed [3:0] m; } s_t; s_t k = '{m: 4'sd2}; q[k.m][1][4:3]` `R 0 x0`, sv2v = verilator `R 0 11` = hand, iverilog `n11_m_pos.sv:3: syntax error`
* witness: literal twin ctl_lit `q[2][1][4:3]` `R 0 x0`, iverilog = sv2v = verilator `R 0 11`
* LSB-0 twin ctl_lsb0 `q[k.m][1][2:1]` on `[3:0]` `R 0 11` = sv2v = verilator.
* Q-SELNORM's clauses are this row's other prerequisites (Q-SEALCALL E1, Q-BOUNDI32 E4)
* source: park probe rdlsb; design review F-1 (W/qpkrdlsb/RULINGS-dr1.txt); re-plan W/qpkrdlsb/replan/REPLAN.txt; r2 lens-diff F-W, lens-sound F1 and G15 (W/qpkrdlsb/RULINGS-lens-r2.txt)
