(§2 🆕 AA · SPLIT · in Start-order rows) (a combinational block written before its feeder reads x at time 0):
* a consumer `always_comb` / `always_latch` written before the block feeding it, in a chain no time-0 process write reaches (declaration initializers, a constant block, a settled CA), reads x in its time-0 pass and reports W4031 / E4003 at t0 where both oracles are silent (q_chaind_A_B, q_chain3_A_Bc_Bb, s583_a08, x13b, tg2, tg5, t3j; PRE the same; and aa1_chain, an `if` chain, §4.5.585: vita W4031 at t0 and t5, verilator silent, iverilog on the `unique case` twin s583_a08 `Time: 5` only)
* the passes run one per batch in source order (§4.5.584: `sched/run_loop.rs`, `native/run.rs`)
* closing it needs an order among the time-0 passes, and the oracles are silent by different ones (iverilog reverse source order, which reports the same consumer-first read at the source's next change and reports forward chains at t0; verilator dependency order)
* every order measured moves split cells (§4.5.584 grounding: reverse 3 to iverilog, dependency order 4 to verilator)
* status: SPLIT
