(§5.2 Q-DYNPAT · rank ① · source ENG-1 r2 (Q04, Q04i)) (a dynamic-array self pattern assigns element by element with no snapshot):
* symptom: a dynamic-array self pattern `d = '{d[1], d[0]}` assigns element by element with no snapshot (`int`: vita and iverilog `0 0`, verilator `2 1`)
* the right-hand side is evaluated before the assignment
* fix: the queue pattern's snapshot on the dynamic-array path
