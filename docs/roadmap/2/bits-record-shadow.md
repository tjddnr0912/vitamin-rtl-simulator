(§2 bits-record-shadow · OPEN · in Scoping / imports / block-locals) (`$bits` of an unpacked record shadowing a packed struct answers the outer):
* `$bits(T)` of an unpacked record that shadows an outer packed struct `T` (a generate block or a function declaring `typedef struct { logic [7:0] a; logic [7:0] b; } s_t;` under a module's 8-bit `struct packed` `s_t`) returns the outer struct's width (k13 `b1=8 b2=8`, verilator `b1=16 b2=16`; k05, k05b, k09, k12 alike)
* PRE = POST
* the record writes no typedef entry, so the name's stale one answers (struct-binding-by-name's root)
* OPEN
