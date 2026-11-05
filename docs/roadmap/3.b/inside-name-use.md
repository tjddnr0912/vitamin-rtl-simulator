(§3.b inside-name-use · OPEN · in Parser accept) (a design using `inside` as a name refuses every `case … inside`):
* a design using `inside` as a name refuses every `case … inside` (`api.rs::first_inside_name_use`)
* re-parse such a unit with case-inside off
* cross-unit needs a scope model
* E2002 where PRE ran: `inside == 4'd5:`, `inside.v`, `inside'(…)`
* oracles: iverilog `-g2005`
* status: OPEN (M)
