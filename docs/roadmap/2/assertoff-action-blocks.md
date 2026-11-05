(§2 assertoff-action-blocks · OPEN · in Delays / events) (a concurrent assertion's own action blocks run while assertions are off):
* A concurrent assertion's own pass or fail action (`assert property (@(posedge clk) a) else $display(…)`) keeps running while assertions are off
* only the synthesized default `$error` is gated (`sva_check.rs`, `in_assert_synth`): `Fc t=25`, `Fc t=35` between `$assertoff` at 22 and `$asserton` at 42, Verilator none (IEEE 1800-2017 §20.11)
* §5.2 row 73's gate
* OPEN
