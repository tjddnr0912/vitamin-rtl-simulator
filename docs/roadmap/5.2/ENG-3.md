(§5.2 ENG-3 · rank ① F) (implicit sensitivity at longest-static-prefix granularity):
* slot: after Q-IDXVALUE, Q-MDCONSTOFF, 70 and Q-PARAM2STATE
* slot: never beside row 4
* implicit sensitivity at longest-static-prefix granularity, IEEE 1800-2017 §9.2.2.2.1 / §11.5.3 — an `always_comb`, `always_latch` or static deferred assertion wakes only on a change inside the prefix of a read (C10–C13, C16–C19, C27–C29, SENS1), decided per store (C15: a same-process glitch on the read bit still wakes)
* scope: a variable index ends the prefix (C11, C14)
* scope: exception b) per region (C26b wakes on v[1] only)
* scope: `always @*` keeps whole identifiers (C01) and an explicit `@(v[0])` its holder (C15e)
* cause: vita wakes on any change of a whole read net (`sched/scan_arm.rs` `arm_sensitivity`, `native/wake.rs`)
* fix: a region on `EdgeTerm`, store-side region stamps in interp / vm / native / JIT, a format bump
* note: hot path: a census binary (activations dropped, stores into region nets, per workload) before code, then a perf A/B
* oracles: iverilog where it is element-precise (C27), hand-IEEE elsewhere (iverilog `sorry: constant selects in always_* processes`)
* note: ENG-2 landed (§4.5.627: function bodies in, task contents and action blocks out), ENG-3 is the rest of row 73's §9.2.2.2.1 prerequisite (select precision)
* source: engine plan for row 73 (P1 a; row 73 lens cells MI1, MI2, MI2b, MI4, MI8, MI10, MI12, MIA1, I2, I13, P01, P13, T3, V03)
