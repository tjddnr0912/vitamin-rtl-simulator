(§5.2 Q-OVRXPLANE · rank ① · source Q-MEMBSRC PLAN3 O-C1 (P8), PLAN4 (pvi, pvo)) (an override's sized x / z literal loses its unknown plane):
* slot: after Q-PARAM2STATE
* slot: R7.4 · L1, after S1v-V (R5.2), on BIND-1: an override's typed value keeps its x plane (owner, 2026-11-09, the record-first order; it was 20 · L2)
* slot: "after S1v-V" (:3) binds only its untyped-keyword targets (an override into an untyped parameter that spells a signing keyword); its typed-target lane reads S12's record and row 48's D4 (:7) only (owner O2, 2026-11-15)
* V′, §2 start-order 15 — an override's sized x / z literal loses its unknown plane (`elaborate/src/params.rs` binds the value bits only): `sub #(.V(40'bx), .N(4'bx)) u();` into `parameter logic [39:0] V = 40'd0, parameter logic [3:0] N = 4'd0` binds `v=0000000000 n=0000` and reads `V[9:8]` as `00` and `VA[~N]` as `1` (`logic [15:8] VA = 8'hA5`), where iverilog 13.0 = sv2v → iverilog print `v=xxxxxxxxxx n=xxxx` and `vs=xx r=x` and verilator 5.052 refuses the constant x index (W/row24/membsrc/park/cells/p8)
* the declaration twin `localparam logic [3:0] N = 4'bx;` is E3009 (a ≤ 64-bit declaration holding x), so the override is the door that binds a value
* fix per row 48's design (D4, the override row: bind at the target's declared width and sign; x: a 2-state target converts it to 0, a ≤ 64-bit 4-state target refuses, a > 64-bit target holds it in `wide_param_bits`), reading Q-PARAM2STATE's record (start-order 15's BLOCKED note)
* the rest of start-order 15's plane stays in §2
* in row 24's fix path (Q-MEMBSRC PLAN3 O-C1 P8; PLAN4 pvi p8i, pvo o05)
* witness (W/param2state/ground/cells/b14_ovr_logic_x): `module c #(parameter logic [3:0] B = 0) ();` with `$display("A b=%b", B)` and `c #(.B(4'b1x01)) u();`: PRE rc 0 `A b=1001`
  * oracles: iverilog and verilator `A b=1x01`
* note: the typed lane and Q-PARAM2STATE's S1v-X (a 4-state target refusing an x at the declaration lane) are planned as one slice with one decision fn (W/param2state/plan/PLAN.txt §5, §6); where an override's x plane is lost, the field that decision reads, is NOT_MEASURED (PLAN.txt U4)
* PRE 01cac4b3
* silent-wrong, pre-existing
