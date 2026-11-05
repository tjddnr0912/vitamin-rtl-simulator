(§3.b over-max-width · OPEN · in Constant edges (§4.5.601)) (a computed part-select width above 2^20 bits is refused):
* symptom: a part-select width above 1048576 bits from bounds that are not two literals is E3009 (§4.5.601: `f[WP[71:40]:0]`, k2, k3, k5, d67, d6) while its literal twin `f[1048577:0]` runs and matches iverilog, sv2v and verilator below 2^24 (above 2^32 the oracles wrap or truncate: h08 iverilog `2f`, d6 all three `0`)
* fix: hand the decided width to the engine as the literal twin does, up to 2^24
* oracles: 3 oracles
* status: OPEN
