(§2 fork-auto-local-respawn · OPEN · in Scoping / imports / block-locals) (an `automatic` local in a respawned `fork … join_none` arm shares one net):
* An `automatic` block-local in a `fork … join_none` arm that a loop or an `always` spawns again while an earlier one is live shares one net (a06c `x=2 x=2`, verilator `x=1 x=2`; iverilog refuses the lifetime)
* refused since §4.5.611 under a timing control where the arm's block can wait and no `wait fork` follows the fork in its statement list
* open for the plain block
* verilator
* OPEN
