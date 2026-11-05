(§5.2 Q-2STINVSEL · rank ① · source Q-TPCARRIER plan (W/qtpcarrier/PLAN.txt §2.4 D6, 2026-10-29)) (an invalid select of a 2-state variable reads x, not 0):
* slot: independent of Q-WITHREAL: no value dependency (W/qtpcarrier/PLAN.txt L2), and its select / eval touch set meets only Q-WITHREAL's parked r2 patch, which its resume re-grounds
* slot: before Q-TPCARRIER
* an invalid select of a 2-state VARIABLE reads x where IEEE 1800-2017 §11.5.1 (ieee1800-2017.txt:12351–12353) reads 0: `bit [15:8] vb = 8'hA5;` `vb[0]` x, `int iv = 5;` `iv[40]` x, `vb[ix]` (`ix = 3'b0x1`) x
* verilator 5.052 `0 0 0`
* iverilog 13.0 `x`, self-contradictory (the same `bit` variable assigned `8'b1x0z_x1x0` stores `10000100`, `$isunknown` 0, `b[8]` x)
* sv2v makes `bit` 4-state
* fix area: select lowering / sim-engine eval
* PRE `W/accept/vita-01cac4b3` 56e6bdf2 (W/qtpcarrier/plan/q/q2s, q3_iv)
* a door into Q-TPCARRIER (W/qtpcarrier/plan/k/k07)
* the parameter twin is Q-PARAM2STATE's 2-state half
* silent-wrong, pre-existing
