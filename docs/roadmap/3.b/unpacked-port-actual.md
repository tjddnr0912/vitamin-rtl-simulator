(§3.b unpacked-port-actual · OPEN · in Nets / timing) (unpacked-array port actuals: sub-arrays, patterns, one-element arrays):
* symptom: unpacked-array port actuals: a sub-array (`mt[h]`) or an assignment pattern (`'{default: 0}`, `'{a, b}`) as the actual is E3009 / E3002 (`ports.rs:441`; verilator and sv2v → iverilog `A s0=3 s1=30`), and a one-element array (`logic [7:0] m [1]`, `[N]` with N=1) is wired as a scalar, so its whole-array uses refuse (`ports.rs:745` `child_len > 1`, `netdecl.rs:792`; `A s=4 s2=4`, `A s=4 s2=1`)
* one funnel (`wire_array_port`)
* witness: row-9 R17, row-9 R27
* status: OPEN (§5.2 row 37)
