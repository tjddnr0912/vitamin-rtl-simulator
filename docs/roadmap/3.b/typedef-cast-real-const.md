(§3.b typedef-cast-real-const · BLOCKED) (a constant typedef cast of a real value does not fold):
* symptom: the constant twin of a typedef cast of a real: `{u4_t'(2.5){1'b1}}` and `localparam int PC = p8_t'(2.5)` in a package are E3009 (a size cast of a real / not a foldable constant), iverilog 13.0 = sv2v `111` and `3` (IEEE 1800-2017 §6.24.1: the cast converts as an assignment, rounding)
* evidence: PRE 01cac4b3 (Q-WITHREAL r2 lens-diff CX_rep, CX_pkgc)
* oracles: 2 oracles
* status: BLOCKED (§5.2 Q-WITHREAL, parked: its typedef cast of a real, with the resume)
