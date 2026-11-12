(§2 untyped-param-product-width · in Oracle splits (recorded, not chased)) (an untyped parameter over a product that overflows 64 bits):
* witness: `localparam P = 3037000500 * 3037000500;` `parameter int Q = 3037000500 * 3037000500;` with `$bits(P)`: PRE d5ac3b05 = POST `A p=145474192 q=145474192 r=-9223372036709301616 bp=33` (W/batch3/lens-sound/work/s7)
* oracles: iverilog `p=9223372037000250000 bp=65`, verilator `p=145474192 bp=32`; for the typed `Q` all three `q=145474192`
* note: batch 3 lens-sound N7 (W/batch3/RULINGS-lens-sound-r1.txt): the untyped width splits the oracles, so `value_rules.rs`'s note is narrowed to the typed case (§5.2 VAL-1); PRE = POST
