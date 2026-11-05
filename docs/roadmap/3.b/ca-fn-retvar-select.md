(§3.b ca-fn-retvar-select · OPEN · in Subroutine / frame) (a function in a continuous assign that select-writes its return variable):
* symptom: a function called from a continuous assign that writes a bit- or part-select of its return variable is E3009 (`function countones body is not reducible to an expression (control flow)`; EL2 `countones`, EH1 `decode2_4`, `decode3_8`, `encode8_3`)
* the inline lowering carries a select write of the return variable as a merge into its running value
* all three `n=2`
* witness: row-9 O9
* status: OPEN (§5.2 row 25)
