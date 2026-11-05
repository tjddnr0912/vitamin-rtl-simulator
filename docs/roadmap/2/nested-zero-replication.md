(§2 nested-zero-replication · OPEN · in Ranges / bounds / selects) (a zero-count replication nested in a concatenation operand runs):
* A zero-count replication nested inside a concatenation that is itself a concatenation operand (`{8'hA5, {{(Z){1'b1}}}}`, `Z = 0`) runs at exit 0 (`r=0000014a`; `{{(Z){1'b1}}, {(Z){1'b0}}}` `r=00000000`) where iverilog refuses ("Concatenation/replication may not have zero width in this context") and verilator errors (`ZEROREPL`; sv2v reads `a5`): the zero-count rule (IEEE §11.4.12.1, `expr_main.rs`) admits a zero count whose parent is a concatenation without asking whether that concatenation is itself empty
* pre-existing (PRE = R2 = R3, §4.5.601 round-3 differential review)
* oracles: 2 oracles
* status: OPEN (filed as §5.2 Q-ZEROREPL with the run-time arm's other zero shapes)
