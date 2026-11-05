(§5.2 Q-PCTCXZ · rank ① · source Q-STRBYTES r3 sweep mover (the `%c` field)) (`%c` of a value with x or z bits prints a NUL byte):
* symptom: `%c` of a value with x / z bits prints the NUL byte: `b = 8'b1010_xz01; $display("[%c][%s]", b, b)` writes `[\x00][\xa1]` (PRE 9d60771d = Q-STRBYTES §4.5.629, native vm interp) where iverilog 13.0 = sv2v → iverilog read x / z as 0 for both, `[\xa1][\xa1]` (verilator 5.052 stops the time step's output at the NUL: blind)
* pins: vita's own `%s` of the same value reads x / z as 0
* pins: all-x `8'bx` is `\x00` on vita and iverilog
* IEEE 1800-2017 Table 21-2 (ieee1800-2017.txt:28681) gives `%c` no x / z rule (hand: x / z → 0, as `%s` and `%u`, :28660–28661)
* witness: sweep source `emb__sim-engine_src_native_kernel_tests__587a7dc69f` line 1
