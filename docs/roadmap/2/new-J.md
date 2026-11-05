(§2 🆕 J · LOUD · in Start-order rows) (the unbased unsized `'1` in concat, index, fill and `$bits`):
* ⓓ `{'1, 1'b0}` illegal, loud: keep
* ⓔ `v['1]` split (= iverilog)
* ⓕ fill widths split (= verilator)
* ⓖ `$bits(U + 4'd1)` over `U = '1` is 32 (`min_signed_bits(v).max(32)`, WALL)
* LOUD
