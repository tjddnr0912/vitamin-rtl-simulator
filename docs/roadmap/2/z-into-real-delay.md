(§2 z-into-real-delay · in Oracle splits (recorded, not chased)) (z into a `real` destination under rise, fall and turn-off delays):
* z into a `real` destination under `#(rise, fall, turnoff)` (`assign #(1,4,2) b = i;`, `i` 5 -> `'z`, ENG-1c O1) — no oracle (iverilog rises on every real transition, `t=11`; verilator never lands the z): vita stores 0.0 and takes the fall delay (`t=14`), the value rule every real destination follows
* PRE took the turn-off delay (`t=12`)
* hand-IEEE
