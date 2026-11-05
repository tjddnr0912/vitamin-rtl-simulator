(§5.2 Q-NETDLY · rank ① · source ENG-1 r2 (F2e2 real right-hand side, F2e4 int)) (a net delay on a declaration without an assignment is ignored silently):
* symptom: a net delay on a declaration without an assignment (`wire [7:0] #(2,5) w; assign w = a;`) is ignored silently, int too: vita `W t=0 \| t=10 \| t=20`, verilator `t=2 \| t=15 \| t=22`, iverilog `sorry: Net delays not supported` (loud)
* IEEE 1800 §10.3.3
* fix: refuse by name until lifted, or implement
* fix: census first
* a release of such a net must stay right (Q-RELDELAY E08: `wire [3:0] #3 y; assign y = a;` forced at 5, released at 7: vita = verilator `S t=8 y=0001`; Q-RELDELAY restores only a delayed continuous assignment's output, so a net delay that becomes real needs the same restore; it holds at t=8 only: L09, `a` changed while forced and released at 8, vita `S t=8imm y=0010`, verilator `0001`; L08, a `#2` net delay beside `assign #3`, vita `0011` at t=12, verilator `0001`: the delay sum is ignored)
* source: Q-RELDELAY r1 lens-diff F7 (L08 L09)
