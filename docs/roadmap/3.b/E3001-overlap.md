(§3.b E3001-overlap · DEFERRED · in Nets / timing) (a part-select driver beside a whole or same-range driver is a false E3001):
* same-range part-select drivers
* witness: corpus witness row-9 O10: a wire with a part-select continuous driver beside a whole driver is E3001 (`net … ifu_axi_bresp driven by multiple overlapping continuous assignments`; EL2 `el2_veer_wrapper` ties `ifu_axi_bresp` / `ifu_axi_bid` twice; iverilog and sv2v → iverilog `w=00 v=x1`, verilator `v=01`, 2-state)
* a net declaration assignment is such a whole driver too (Q-RELDELAY r3 lens-diff m10: `wire [5:0] y = 6'b0;` beside `assign y[3 -: 2] = d0;` or `y[3:2]`, E3001, iverilog = sv2v `S t=1 y=00xx00 \| S t=2 y=00x000 \| S t=3 y=000x00`, verilator `000000`)
* row 60 builds the per-bit driver map
* witness (2026-11-16, §5.2 Q-MDRVMIX's matrix, FALSE-LOUD, W/mdrv/ground/cells/m): m_k_w8_ca_ca_ppo, `wire [7:0] v; assign v[5:2] = s[3:0]; assign v[3:0] = ~s[3:0];`, and m_k_w8_port_ca_ppo (`drvw #(4) uA (.i(s[3:0]), .o(v[5:2]));` for the first): PRE cdf0c457 rc 1 `error[VITA-E3001] E-ELAB-MULTIDRIVER: net `t.v` driven by multiple overlapping continuous assignments`
  * oracles: iverilog = sv2v → iverilog `A v=zz01xx10` `B v=zz110011`; verilator 5.054 `%Warning-MULTIDRIVEN: t.sv:4:14: Bits [3:2] of signal 't.v' have multiple combinational drivers. This can cause performance degradation.` (t.sv:5:14 on the port cell), runs `A v=00011010` `B v=00110011`; IEEE 1800-2017 §6.5 (txt:3477-3478)
* witness (m_p_innet_ca_part, an input-port net connected whole, `mi u (.v(x), ..)`, and `assign v[3:0] = ~s[3:0];` inside `mi`): PRE rc 1 `error[VITA-E3001] E-ELAB-MULTIDRIVER: net `t.u.v` driven by multiple overlapping continuous assignments`; iverilog = sv2v → iverilog `A v=00001x1x` `B v=0000xx11`; verilator `%Error-ASSIGNIN: t.sv:3:10: Assigning to input/const variable: 'v'`
* oracles: 2 oracles
* status: DEFERRED (§7 row 60)
