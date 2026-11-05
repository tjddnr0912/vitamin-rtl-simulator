(§2 foreach-assoc-index-type · OPEN · in Ranges / bounds / selects) (`foreach` over a byte/longint/time-keyed array types its index `integer`):
* A `foreach` over a `[byte]` / `[shortint]` / `[longint]` / `[time]` associative array declares its index `integer` (`stmt_ctl.rs` foreach desugar): `%h` of a `byte` key -128 prints `ffffff80` (k_byte, verilator `80`), and a key an `integer` cannot hold stops the walk with W4020 (`[longint]` 2^40, verilator walks it)
* fix: type the index by the array's key (the parser cannot see it; elaborate re-types the synthetic declaration)
* a typed key refuses it since §4.5.616
* verilator
* OPEN
