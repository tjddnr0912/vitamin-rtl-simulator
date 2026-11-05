(§2 auto-local-input-slot · OPEN · in Scoping / imports / block-locals) (an `automatic` task's block-local named like an input shares its slot):
* symptom: An `automatic` task's block-local named like an INPUT argument shares the argument's slot, and a delay, event or `iff` expression outside the block that reads the argument sees the block's write (the no-timing twin k20c of k20 `repeat (2) #(k) begin int k; k = 20; … end`, whose PRE value is `t=23` where iverilog, sv2v → iverilog and verilator print `t=6`; k10 `iff k > 0` `cnt=1`, verilator `cnt=2`; the timing-body forms are E3009 since §4.5.611): `check_block_local_scope_leaks` walks statements, not timing-control expressions — extend it there (§5.2 row 72b, c4)
* oracles: 3 oracles
* status: OPEN
