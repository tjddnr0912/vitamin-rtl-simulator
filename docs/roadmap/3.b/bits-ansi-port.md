(§3.b bits-ansi-port · OPEN · in Constants / parameters) (`$bits` of a whole ANSI port inside a `localparam`):
* symptom: `$bits(<ANSI port>)` in a `localparam` is E3009 (`parameter IW value is not a constant: undefined name tl_i`)
* since §4.5.605 a select or packed-struct member of an ANSI vector port answers (`$bits(h.src)`, the R10 repro: iverilog and sv2v `A o=120`), the whole port does not
* cause: `elaborate/src/net_util.rs:762` `prescan_net_bits` records body declarations only
* record the ANSI header's ports, read through the same `bits_of_view` resolver
* all three `A o=12`
* witness: row-9 R10, every OpenTitan page
* status: OPEN (§5.2 row 34)
