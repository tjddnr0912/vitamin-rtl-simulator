(§3.b time-signed-param · OPEN · in Parser accept) (`parameter time signed` is rejected by the parser):
* symptom: `parameter time signed T` is E2002 (`hdl-parser/src/params.rs`)
* accept the qualifier and hand `kind_signedness` its bit
* oracles: 2 oracles
* status: OPEN
