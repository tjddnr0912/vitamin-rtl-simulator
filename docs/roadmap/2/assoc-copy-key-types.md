(§2 assoc-copy-key-types · OPEN · in Ranges / bounds / selects) (a copy between associative arrays of different key types is accepted):
* A whole-array copy between two keyword-atom associative arrays of different key types is accepted and copies the keys as stored (q6_copy_leg `[int]` to `[byte]`: `A n=2 b5=1 b300=2 b44=0 ex300=1`, the `[byte]` array holding key 300)
* verilator refuses the assignment at compile (`no viable overloaded '='`), IEEE 1800-2017 §7.9.9 wants one index type
* a typed key refuses it since §4.5.616
* verilator + hand-IEEE
* OPEN
