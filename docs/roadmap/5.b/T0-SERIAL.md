(§5.b T0-SERIAL · HELD) (each serial time-0 comb pass is followed by a scan of every waiter):
* the engine scheduler follows each serial t0 comb pass (§4.5.584) with a propagate that scans every waiter: comb5000_settle interp +9.7% (vm shares the scheduler, unmeasured; native +0.8%, ibex +0.7%)
* `sched/run_loop.rs`, `sched/propagate.rs`
* status: HELD (native, the default, is flat; ER §10.2 keeps the reference interpreter out of performance work)
