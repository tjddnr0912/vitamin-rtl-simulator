(§2 assoc-struct-member-write · OPEN · in Ranges / bounds / selects) (a write to a packed-struct member of an associative element is dropped):
* A write to a member of a packed-struct element of an associative array (`m[261].a = 3;`) is dropped with W4020 `unsupported dyn lvalue shape (write ignored)` at exit 0 (`changes.rs` `dyn_write`; lens q1_struct `rd5=00 a=0 b=0 n=0`, verilator `rd5=39 a=3 b=9 n=1`), on a typed key and on `[int]` before row 20 alike
* verilator
* OPEN
