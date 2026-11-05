(§3.b V2A/V5 · OPEN · in Subroutine / frame) (dynamic-array formals on automatic tasks and function `new[]`):
* dyn-array formals on automatic tasks, function `new[]`
* a with-reduction over a dyn-array function formal (`function automatic int far(real a[]); return a.sum() with (int'(item * 2));`) is E3009 "array reduction `with` applies to ..." (hand 8; PRE 01cac4b3, Q-WITHREAL r2 lens-diff FN_arg)
* oracles: iverilog
* status: OPEN
