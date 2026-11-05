(§5.2 Q-NANCANON · rank ① F · source CI runs 40328999518 / 40409819684; hyg-ci probe (PROBE_CATALOG)) (real NaN sign and payload differ by platform):
* slot: a wall-clock A/B stops every track
* real arithmetic produces a platform-dependent NaN sign / payload: a NaN made from non-NaN operands is the CPU's default NaN, `7ff8000000000000` on aarch64 and `fff8000000000000` on x86-64, so `0.0/0.0`, `inf-inf`, `0*inf` and `inf/inf` print differently on Linux CI and macOS (CI ubuntu-latest and RHEL9 at 9d60771d, `q8a_nan_from_zero_by_zero_control`: `W t=2 b=fff8000000000000` where macOS prints `7ff8000000000000`), against CONTRIBUTING's Linux = macOS byte-identical guarantee
* the vendored libm (arch-disabled) makes its invalid results the same way (`(x - x) / 0.0` in `$ln` / `$log10` / `$atanh`, `(x - x) / (x - x)` in `$pow` and real `**`, `0.0 / (x - x)` in `$asin` / `$acos`, `x - x` in `$sin` / `$cos` / `$tan` of an infinity)
* `$sqrt` (libm's generic `F::NAN`) and `$acosh` below 1 (through that sqrt) are bit-exact already
* measured on aarch64 only: every producer `7ff8000000000000` on native, vm and interp = iverilog 13.0 = verilator 5.052 (PROBE_CATALOG row "hyg-ci probe"), x86-64 by the CPU's default-NaN rule
* real -> int of a NaN is not a NaN (0)
* CONTRIBUTING's libm guard `sim-engine/tests/libm_determinism.rs` pins no NaN result
* fix: one canonical quiet NaN (`7ff8000000000000`) for a NaN produced from non-NaN operands on native / vm / interp / JIT, a census of every real-producing op (the four arithmetic ops, every libm call, conversions; `$bitstoreal` stays bit-exact), NaN results added to `libm_determinism.rs`, a perf A/B
* a NaN OPERAND's propagation (sign and payload; iverilog commutes `+`, `$sqrt` of a NaN drops its payload where iverilog and verilator keep it) is censused, not changed, by this row
* restores q8a's exact pin (`delayed_assign_value.rs`, interim: bit 63 masked)
* Q-FMTNEGNAN spells the sign this row decides
