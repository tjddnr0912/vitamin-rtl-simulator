(§3.b const-fn-case · BLOCKED · in Constants / parameters) (a constant function containing any `case` statement does not fold):
* a constant function whose body holds any `case` / `casez` / `casex` / `case … inside` is E3009 `f(…) has no constant-fold arm`, every qualifier, reached or not, where both oracles fold (probe p2: plain `case`, `g(2)`, iverilog and verilator `P=20`; 49 §4.5.587 cells; `inside`: verilator only, iverilog `Incomprehensible case expression`)
* cause: `elaborate/src/const_fn.rs` `exec_const_stmt` has no `Stmt::Case` arm (catch-all `_ => None` at :1719)
* fix: the arm sizes the case expression and every item once at one width and sign (ER §2.4), never pairwise, the row's main risk
* fix: casez / casex masks, §12.5.4 for `inside`, first match wins
* note: opening it lets the interpreter fold more functions at every consumer, the widening §4.5.587 reverted on, so it inherits §2 🆕 AD's residue (🆕 AF, AG, AH and the "Scoping" import line; not yet measured on case bodies), and the arm obeys §5.2's 4-state rule (§2 🆕 AE: a `case` with no `default` that misses leaves the return variable unassigned)
* a `unique` / `priority case` miss then reaches the synthesized default (`unique-const-fn`)
* closes §2 🆕 AC's case-form cells
* oracles: 2 oracles
* status: BLOCKED (§2 🆕 AF, AG, AH, the "Scoping" import line)
