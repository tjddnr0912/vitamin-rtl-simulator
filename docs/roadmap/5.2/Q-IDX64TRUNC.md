(§5.2 Q-IDX64TRUNC · rank ① · source row 29 r1 lens-sound F6, r3 lens-diff N2) (an unpacked word index wider than 32 bits is truncated to its low bits):
* symptom: an unpacked word index wider than 32 bits is truncated to its low 32 bits: `logic [1:0][15:2] a [2]; logic [13:0] m [2]; logic [7:0] u [2];` with `li = 64'h1_0000_0001`, `lj = 33'h1_0000_0001`: `a[li] = '1; m[li][9:4] = 6'h3f; u[lj] = 8'hff;` print `R fffffff 0000000 03f0 0000 ff 00` (element 1 written) on PRE 719e37d2 = iverilog 13.0 = sv2v → iverilog (w1), where IEEE 1800-2017 §7.4.6 ignores an out-of-range write (oracle divergence in PROBE_CATALOG)
* witness: row 29's reach: h3a `cU[u64][1][4:3]` (u64 = 64'h1_0000_0002, PRE E3009) writes element 2 on r2 / r3 and all three oracles
* witness: the DF1 matrix's u33 / u64 rows (W/row29/r2/DF1_MATRIX.tsv, F6) are this class
* fix: compare the full-width index against the bounds before narrowing
* status: silent-wrong by hand, pre-existing
