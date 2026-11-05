(§5.2 Q-RS30FOLD · rank ① · source ENG-2 (§4.5.627) r1 lens-diff N1 (PPi02, PPi03, PPi04, PPi09, PPk)) (the implicit read set's exact index fold has no arm for `/`, `%`, `&`, `$clog2`):
* symptom: the exact index fold of an implicit read set (`elaborate/src/read_set.rs` `exact_const`) has no arm for `/`, `%`, `&` or `$clog2`, so a write through such a constant genvar index covers nothing and a row-30 ping-pong indexed that way (`any[k/2]`, `any[k%2]`, `any[k&1]`, `any[$clog2(k+1)]`) still stops `VITA-F4016` at time 0 where iverilog and verilator give the value (`any=cc` / `any=3c`)
* status: pre-existing (PRE = POST)
* IEEE 1800-2017 §9.2.2.2.1 exception b
* fix: fold those operators exactly (division and modulo by a non-zero constant, `$clog2` of a known constant), the N36k / N36p controls guarding that an unfoldable write still covers nothing
