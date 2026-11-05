(§5.2 Q-CAREALCTX · rank ① · source ENG-1c r1 lens-sound F-B (N01, N02; control P21)) (an integral expression driving a `real` by continuous assign uses 64-bit context):
* slot: after 74
* symptom: an integral right-hand side driving a `real` through a continuous assignment is evaluated at the destination's 64 bits, not self-determined (IEEE 1800-2017 §11.8.1): `logic [7:0] a = 0; real r, u; assign #2 r = ~a; assign u = ~a;` reads `r=18446744073709551616.000000` on every backend and `u` the same on vm and interp (native `u=255.000000`), iverilog = sv2v = verilator `r=255.000000 u=255.000000` (N01)
* witness: `a + b` with 200 and 100: vita `r=300.000000`, the three `44.000000` (N02)
* cause: the sites that miss `width::lvalue_targets_real`: `sched/scan_arm.rs` `ca_native` context width and `schedule_delayed_cas`'s native arm (`lvalue_width`)
* status: pre-existing (PRE = BASE = ENG-1c)
