(§2 gen-typedef-shadow-param · OPEN · in Ranges / bounds / selects) (a generate-block typedef named like an outer parameter reads as it):
* A typedef declared in a generate block whose name an outer parameter carries is read as the parameter in a bracket: `parameter int N = 4;` then `begin : gb typedef int N; bit [7:0] m [N];` sizes a fixed 4-element array, where the inner typedef makes `m` associative (c7d_gen_td_shadow_fe `cnt=4 m1=22`, verilator `cnt=1`; c7c_gen_td_shadow_val `m44=xx` with E4002, verilator `m44=11`)
* before row 20 the same, and §4.5.616's file-wide name scan (`assoc_key.rs` `name_is_only_a_type`) keeps the size reading on purpose
* the fix is the scan's narrowing below
* verilator
* OPEN
