(§2 time-key-signed · OPEN · in Ranges / bounds / selects) (a `[time]` associative key is held as a signed i64):
* A `[time]` associative key is held as a signed i64, unconverted: a key at or above 2^63, or a negative index, orders before every smaller key (c_time_key `k=18446744073709551615` then `k=5`, verilator `k=5` first)
* fix: an unsigned key domain (a biased or `u64` map), which also opens `longint unsigned` / `bit [63:0]` (refused by name since §4.5.616)
* verilator + hand-IEEE §7.8.4
* OPEN
