(§5.2 Q-CALHSEL · rank ① · source Q-RELDELAY plan / design review Q7 (E40)) (a continuous assign whose target has a non-constant select is accepted):
* symptom: a continuous assignment whose left-hand side has a non-constant select is accepted: `assign #3 y[i] = a;` on a logic variable (E40) runs (vita after Q-RELDELAY `t=9 0010`, verilator `0001`)
* IEEE 1800-2017 Table 10-1 requires a constant bit- / part-select
* oracles: iverilog and sv2v refuse (`Bit select expressions must be a constant integral value`)
* symptom: the net twin too: `wire [3:0] y; assign #3 y[k +: 2] = a;` with `k` moved 2 → 0 while `y` is forced (D7 Q2d_dynidx_k_changes_forced: PRE `S t=6imm y=0000`, Q-RELDELAY `0010`, its restore at the current `k`; verilator `1000`) and undelayed without a force (Q4_dynidx_wire_noforce: vita `S t=1 y=10zz \| S t=2 y=1010`)
* oracles: iverilog and sv2v refuse (`Indexed part select base expression must be a constant integral value`)
* witness: ENG-1c's decision on the store image moves its value (an illegal input, accepted like Q-CAREALCAT's): the image ignores the target index, so an index change whose image bits equal the old target's writes nothing new — `wire [7:0] w; assign #3 w[i] = a;` with `i` 0 → 1 and `a` 01 → 03 at t=10: vita `S t=15 w=zzzzzzz1` (BASE §4.5.626 `zzzzzz11`), verilator `00000001`, iverilog and sv2v refuse (``A reference to a net or variable (`i') is not allowed in a constant expression``, `Bit select expressions must be a constant integral value`) (Y01)
* witness: a logic variable (Y02: `xxxxxxx1`, BASE `xxxxxx11`), `w[i +: 2]` (Y03: `zzzzzz01`, BASE `zz01zz01`), a net-array word `wa[i]` (Y04: `w1=zzzz`, BASE `0001`, verilator `0000`), a logic-array word (Y05: `l1=xxxx`, BASE `0001`), a `real` right-hand side (Y07: `zzzzzzz1`, BASE `zzzzzz11`)
* fix: refuse by name
* source: r2 D7 (Q2d_dynidx_k_changes_forced, Q4_dynidx_wire_noforce); ENG-1c r2 lens-diff F-Y (Y01–Y05 Y07; control Y06)
