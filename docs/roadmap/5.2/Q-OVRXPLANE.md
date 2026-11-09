(§5.2 Q-OVRXPLANE · rank ① · source Q-MEMBSRC PLAN3 O-C1 (P8), PLAN4 (pvi, pvo)) (an override's sized x / z literal loses its unknown plane):
* slot: after Q-PARAM2STATE
* slot: R7.4 · L1, after S1v-V (R5.2), on BIND-1: an override's typed value keeps its x plane (owner, 2026-11-09, the record-first order; it was 20 · L2)
* V′, §2 start-order 15 — an override's sized x / z literal loses its unknown plane (`elaborate/src/params.rs` binds the value bits only): `sub #(.V(40'bx), .N(4'bx)) u();` into `parameter logic [39:0] V = 40'd0, parameter logic [3:0] N = 4'd0` binds `v=0000000000 n=0000` and reads `V[9:8]` as `00` and `VA[~N]` as `1` (`logic [15:8] VA = 8'hA5`), where iverilog 13.0 = sv2v → iverilog print `v=xxxxxxxxxx n=xxxx` and `vs=xx r=x` and verilator 5.052 refuses the constant x index (W/row24/membsrc/park/cells/p8)
* the declaration twin `localparam logic [3:0] N = 4'bx;` is E3009 (a ≤ 64-bit declaration holding x), so the override is the door that binds a value
* fix per row 48's design (D4, the override row: bind at the target's declared width and sign; x: a 2-state target converts it to 0, a ≤ 64-bit 4-state target refuses, a > 64-bit target holds it in `wide_param_bits`), reading Q-PARAM2STATE's record (start-order 15's BLOCKED note)
* the rest of start-order 15's plane stays in §2
* in row 24's fix path (Q-MEMBSRC PLAN3 O-C1 P8; PLAN4 pvi p8i, pvo o05)
* PRE 01cac4b3
* silent-wrong, pre-existing
