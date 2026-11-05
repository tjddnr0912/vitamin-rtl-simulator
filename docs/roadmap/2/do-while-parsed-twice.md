(§2 do-while-parsed-twice · OPEN · in Scoping / imports / block-locals) (a `do … while` body is parsed twice):
* A `do … while` body is parsed twice (`hdl-parser` `parse_do_while` clones it with one span), so a body whose local shadows a module variable is E3009 `redeclared (duplicate declaration)` (dw01, dw06, dw08, and under a timing control dw05, which ran right by accident before §4.5.611 and is refused now) and another body takes a wrong value (dw09 before §4.5.611)
* 3 oracles
* OPEN
