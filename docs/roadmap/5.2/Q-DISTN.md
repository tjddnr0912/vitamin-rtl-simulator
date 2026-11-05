(§5.2 Q-DISTN · rank ① · source ENG-1b r1 lens-diff N4 (X_int_dn, X_int_dc)) (`$dist_normal` and `$dist_chi_square` differ from the standard's algorithm):
* symptom: `$dist_normal` / `$dist_chi_square` differ from iverilog = sv2v = verilator for every destination (X_int_dn vita -1, oracles -2; X_int_dc 19 vs 1)
* evidence: PRE = ENG-1b
* IEEE 1800 §20.15.2 (the reference C code)
* fix: port the standard's algorithms bit-exactly
