(§2 strobe-automatic-panic · OPEN · in Diagnostics / artifacts) (`$strobe` / `$monitor` of an `automatic` task variable panics):
* `$strobe` / `$monitor` of an `automatic` task's variable panics (`frame read: no active call window`): k14b, k14c, k14d with no timing control
* k14e and k24, under one, are E3009 since §4.5.611 (they were E3010).
* A static task's inline lane and a re-spawned arm print the shared storage instead (k12 `x=11 x=11`, w04 `x=1 x=1`).
* IEEE 1800 §13.3.2 bars tracing an automatic with `$monitor` (text unverified)
* verilator internal errors, iverilog refuses the lifetime
* no oracle
* OPEN
