(§3.b real-int-overflow · OPEN · in Constants / parameters) (a real constant too large for an `int` parameter is refused):
* symptom: `localparam int A = 1e20;` is E3009 (`real_round_to_i64`)
* oracles: 2 oracles
* status: OPEN
