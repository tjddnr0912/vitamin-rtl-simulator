(§2 range-bound-real-operand · OPEN · in Ranges / bounds / selects) (a real or string operand of `!` or `?:` in a range bound takes 1 bit):
* symptom: a range bound whose `!` operand or `?:` condition is a real or a string declines silently and the range takes its 1-bit default, while a parameter of the same value binds
* witness: m_lognot_real `localparam longint A = !2.5; logic [(!2.5) + 2:0] w;` PRE d5ac3b05 `A a=0 bw=1` rc 0; m_tern_real (`2.5 ? 5 : 6`) `A a=5 bw=1`; m_lognot_str (`!""`) `A a=1 bw=1` (W/val1/plan/probe/cells)
* note: the parameter binds through the real twin, the bound's integer fold declines; VAL-1 F2 (W/val1/plan/PLAN.txt), PRE kept byte-equal by VAL-1; oracles not run
* status: OPEN
