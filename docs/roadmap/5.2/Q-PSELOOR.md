(§5.2 Q-PSELOOR · rank ① · source Q-RELDELAY r4 lens-diff i02 e04, r5 design review d6) (a part-select partly below the net's low bit is refused, not written in range):
* slot: after the fold-parity rows
* symptom: a constant part-select whose bounds are in the declared order but partly below the net's low bit is E3009 (`part-select bounds [msb:lsb] ascend but the net is descending`): i02 `wire [3:0] y; assign y[5:-2] = a;` and e04 `assign y[0:-1] = a;` beside `assign y[0] = b;` (PRE = R4 = r5)
* oracles: IEEE 1800-2017 §11.5.1 writes the in-range bits of such a select (iverilog and sv2v: `sorry: part selects straddling the start of signal`; the indexed twin `y[0 -: 2]` reaches the multi-driver check, E3001)
* status: pre-existing
