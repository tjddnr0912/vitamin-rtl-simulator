(§2 gen-event-shadow-param · OPEN · in Scoping / imports / block-locals) (a generate block's `event` shadowing an outer parameter runs silently):
* A generate block's `event N` shadowing an outer `localparam int N = 0` read as a constant width runs silently (t.event_blk: `case (3) P[0 +: N]:` takes `default`, `o=22`) where all three oracles refuse
* the constant fold reads the outer parameter (`lookup_scoped`)
* pre-existing (PRE = row 65)
* 3 oracles
* OPEN
