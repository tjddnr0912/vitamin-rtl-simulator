(§2 assertoff-immediate · OPEN · in Delays / events) (`$assertoff` and its siblings reach only the concurrent checkers' reports):
* `$assertoff`, `$asserton` and `$assertkill` reach only the concurrent checkers' reports: a simple immediate, deferred or final assertion's pass and fail actions and its default `$error` keep running while assertions are off (row 19 A14 t2h procedural twins: `Fe t=5`, `Ff t=5`, the default `Assertion failed`; Verilator prints none), its condition is still evaluated (E25 `chk v=-1 t=5`, E29 an `$error` inside the condition's function; Verilator `end`), a system-read action (`$sscanf`, `$value$plusargs`, `$random(seed)`) still writes its destination (G1), and a work-library unit compiled on its own is never gated (R2-4, Q2_worklib `F t=5`)
* §4.5.606 refuses only a module-item deferred assertion beside a control call
* fix in §5.2 row 73
* status: OPEN
