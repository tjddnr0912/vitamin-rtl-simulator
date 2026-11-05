(§2 assoc-index-locators · OPEN · in Ranges / bounds / selects) (`item.index` and the index locators answer the position, not the key):
* `item.index` and the index locators (`find_index`, `find_first_index`, `find_last_index`, `unique_index`) over an `[int]` / `[integer]` / `[byte]` / `[shortint]` / `[longint]` associative array answer the element's 0-based position, not its key (p_locator `q='{'h1, 'h2}`, verilator `q='{'h7, 'hc8}`; `netread.rs` `array_item`)
* a typed key refuses them since §4.5.616
* verilator
* OPEN
