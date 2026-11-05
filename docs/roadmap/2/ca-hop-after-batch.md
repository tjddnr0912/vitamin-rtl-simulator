(§2 ca-hop-after-batch · BLOCKED · in Delays / events) (a continuous-assign hop lands after the whole batch):
* A CA hop lands after the whole batch
* yield settle + nonblocking UDP (§4.5.539)
* status: BLOCKED (wake-group order has no oracle)
  * t0: Level / Edge processes the t0 settle wakes, read in batch 2 through a CA or port, read x where both oracles read the value (x7, x7b, x7c, x7d, lb1–lb3, b1_ca_hop_noinit; the `always_comb` twin b2 keeps `v=x`)
  * evidence: §4.5.584's patch B (each such process alone in a batch, a settle after each) closed them but merged that process's own wakes into the parked first batch with no settle between — a two-oracle descent (b1: iverilog = verilator `R t=0 v=1 u=1`, B `v=x`) — and was dropped
