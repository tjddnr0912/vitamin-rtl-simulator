(§2 self-ref-return-range · OPEN · in Ranges / bounds / selects) (a function whose return range calls itself overflows the stack):
* `function [f():0] f();` overflows the stack (`const_fn_ret_wsign`: `depth + 1`)
* OPEN
