(§5.2 Q-RELUNDRV · rank ① · source Q-RELDELAY r1 lens-diff F3 (L12 M01 M02); r3 lens-diff F5 (m20)) (after a release, undriven bits of a net keep the forced value instead of z):
* symptom: after a `release`, bits of the net that no driver drives keep the forced value, where iverilog and sv2v read z: L12 `assign #3 w[3:2] = a;` (`w[1:0]` undriven), `force w = 4'b1010`, release: vita `S t=8 w=0110`, iverilog = sv2v `01zz` (verilator `0100`, 2-state)
* witness: M01 the undelayed twin, the same
* witness: M02 a net with no driver at all, vita = verilator `1010`, iverilog = sv2v `zzzz` (split)
* witness: m20 `wire [3:-2] y; assign #3 y[1 -: 2] = d0; assign y[2] = d1;` forced to 0 and released, vita `S t=6imm y=001100` (its spelled twin `{y[1], y[0]}` byte-identical), iverilog = sv2v `z011zz` (verilator `001100`)
* IEEE 1800-2017 §10.6.2 ("the value determined by the drivers of the net")
* status: pre-existing
