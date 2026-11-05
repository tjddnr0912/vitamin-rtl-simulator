(§3.b anon-member-dims · OPEN · in Parser accept) (packed dimensions after an anonymous struct or union member body):
* symptom: packed dimensions after an anonymous struct or union member's body (`struct packed { logic q; logic e; } [1:0] a;`) are E2002 `packed dimensions on an anonymous member type are unsupported in v1`, in a package too, where the named twin `t [1:0] a;` of a packed struct now composes (outside a package the twin keeps §4.5.604's refusal)
* oracles: iverilog, sv2v → iverilog and verilator `A 10 1`
* the composition is §5.2 row 41's residue (row-9 R04)
* status: OPEN
