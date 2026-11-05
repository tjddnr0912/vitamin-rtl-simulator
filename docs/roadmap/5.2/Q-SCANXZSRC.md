(§5.2 Q-SCANXZSRC · rank ① · source P-PCTU r3 differential lens N3 (W/ppctu/r3/lens-diff/cells/d1x_4st)) (a `$sscanf` source with x or z bits converts instead of returning EOF):
* symptom: a `$sscanf` source with x / z bits converts where IEEE 1800-2017 §21.3.4.3 (ieee1800-2017.txt:29340–29341: "If the format string or the str argument to $sscanf contains unknown bits (x or z), then the system function shall return EOF (-1)") returns -1: `sx = {8'h35, 8'h41, 32'hxxxxxxxx}; n = $sscanf(sx, "%d", a)` prints `1 00000005` (PRE 01cac4b3 = iverilog 13.0 = sv2v → iverilog), and `sz = {8'h35, 40'hzzzzzzzzzz}` the same
* oracles: verilator 5.052 `1 00000005` on x, `-1 11111111` on z
* witness: `%c` the same split (`1 00000035`; verilator on z `1 00000000`)
* pins: the `%d%*u` / `%c%*u.` twins equal their `%*u`-free formats on every lane (§4.5.630 keeps them)
* note: the format-string half of the clause is unmeasured
* oracles: hand IEEE only (both runnable oracles agree with vita on x)
* status: pre-existing
