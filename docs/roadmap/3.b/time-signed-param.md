(§3.b time-signed-param · OPEN · in Parser accept) (`parameter time signed` is rejected by the parser):
* symptom: `parameter time signed T` is E2002 (`hdl-parser/src/params.rs`)
* accept the qualifier and hand `kind_signedness` its bit
* oracles: 2 oracles
* status: OPEN
* witness (2026-11-05, Q-DECLTYPE S12 differential lens D-N2): a typedef alias bypasses the refusal: d1_time_signed_s_alias `typedef time signed al_t;` `al_t v;` `parameter al_t PA = -1;` `v = '1;` `$display("PA d=%0d b=%b n=%0d lt=%0d", PA, PA, $bits(PA), (PA < 0));`
  * PRE `PA d=18446744073709551615 b=1111111111111111111111111111111111111111111111111111111111111111 n=64 lt=0` at exit 0 (the variable `v` reads `AL d=-1 … lt=1`), iverilog, sv2v → iverilog and verilator `PA d=-1 b=1111111111111111111111111111111111111111111111111111111111111111 n=64 lt=1`
* note: the direct spelling `parameter time signed` is E2002, so the alias is a silent bypass of it (PRE 92805863 = POST)
* source: W/qdecltype/lens1/diff/orc/d1_time_signed_s_alias
