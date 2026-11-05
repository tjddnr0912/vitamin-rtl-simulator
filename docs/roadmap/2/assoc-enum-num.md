(§2 assoc-enum-num · OPEN · in Ranges / bounds / selects) (`num()` of an enum-element associative array answers the label count):
* `num()` of an associative array whose element type is an enum answers the enum's label count: `e_t m [int]; m[1] = B;` then `m.num()` is 3 (`typedef enum logic [1:0] {A, B, C} e_t`; verilator `n=1`) — the enum method resolves first
* verilator
* OPEN
