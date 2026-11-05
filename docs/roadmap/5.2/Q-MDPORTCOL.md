(§5.2 Q-MDPORTCOL · rank ① · source Q-RELDELAY r3 lens-diff open point 3 (q07 u11 u13 m17)) (a port drive and a parent assign on disjoint bits of one net are refused):
* slot: after the fold-parity rows
* symptom: a net driven partly by a child module's output port and partly by a continuous assignment in the parent is E3001 even when the two drive disjoint bits: u13 `module child(output wire [5:0] y); assign y[1:0] = r;` instantiated `child u(.y(w));` beside `assign w[5] = b;` is E3001, iverilog = sv2v `S t=1 y=0zzz11 \| S t=2 y=1zzz10` (verilator `000011 \| 100010`)
* witness: q07 (child `y[3 -: 2]`, parent `w[4]`) and u11 (child `y[3:2]`) the same, iverilog = sv2v `S t=1 y=z011zz \| S t=2 y=z110zz`
* witness: an `inout` port, m17 `module child(inout wire [5:0] y, ...); assign y[3 -: 2] = a; assign y[4] = b;` (and `y[3:2]`) is E3001 beside W3056 (`inout port approximated as one-directional`), iverilog = sv2v `S t=1 y=z011zz \| S t=2 y=z110zz`
* status: over-refusal, loud
* status: pre-existing (PRE = Q-RELDELAY)
