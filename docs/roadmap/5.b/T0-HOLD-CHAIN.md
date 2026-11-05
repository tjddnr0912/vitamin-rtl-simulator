(§5.b T0-HOLD-CHAIN · BLOCKED) (held time-0 assign chains cost quadratic time):
* §2 🆕 AB (a): the t0 release re-runs a released held uncertified assign in every pass of every later wave, so a held chain of N is O(N²) (§4.5.590, native, PRE vs POST: r_chain, a `$random` callee, 1k/2k/4k/8k 15.3/38.9/91.4/198.4×; hu_urandom 1k/2k/4k 14.4/35.9/81.0×; a chain of certified held assigns stays ~1.0×)
* `sched/t0_hold.rs` (`held_always`, the wave lanes)
* fix = 🆕 AB's own-wave rule (round 4's build: 0.96–1.00× on both)
* status: BLOCKED (🆕 AB (d))
