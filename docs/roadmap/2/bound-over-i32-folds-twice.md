(§2 bound-over-i32-folds-twice · PERF · in Performance (open, recorded)) (a declared bound above `i32::MAX` runs its constant function twice):
* symptom: a declared bound whose self-determined value is above `i32::MAX` takes the router's path C, which folds it a second time, so a constant function in it runs twice
* witness: c1_call_small and c2_call_big, 40 nets each `logic [f(i) : f(i) - 3] w<i>;` over `f` looping 20000 times, returning 3 or `64'h1_0000_0003`: PRE d5ac3b05 6.78 s and 13.48 s, one run each (W/val3/plan/probe/cells)
* note: VAL-3 F-a (W/val3/plan/PLAN.txt PR18): PRE's cost, kept by VAL-3, which does not edit the router; R6.1 reads both values (§5.2 VAL-3); no oracle (cost)
* status: PERF
