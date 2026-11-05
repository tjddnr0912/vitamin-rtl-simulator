(§5.2 Q-XAGG · rank ① · source Q-XNONINT plan probes pe_aggs, pe_aggr) (an unpacked struct `?:` at an x condition is merged bit by bit):
* symptom: an unpacked struct `?:` at an x / z condition is merged bit by bit as a packed vector: `typedef struct { logic [7:0] a; logic [7:0] b; } s_t; c = 1'bx; sr = c ? s1 : s2;` (s1 = {11, 22}, s2 = {11, 33}) prints `11 XX` where IEEE 1800-2017 §11.4.11 (ieee1800-2017.txt:11964-11971) returns Table 7-1's unpacked-struct value (:6390-6392: each member's default, 'X for a 4-state member), `XX XX`
* pins: equal structs (`11 22`) are right
* oracles: sv2v → iverilog `11 XX` (sv2v packs the struct: not an oracle for the unpacked rule), verilator 5.052 `11 33` (2-state), iverilog `sorry: Unpacked structs not supported`
* oracles: hand only
* evidence: PRE 01cac4b3
