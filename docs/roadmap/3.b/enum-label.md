(§3.b enum-label · OPEN · in Parser accept) (an enum label wider than its base skips its enum slot and truncates):
* `enum bit[3:0] {A=8'hFF}` skips the name's enum slot (`type_rec.rs` `enum_def`, the former `enum_defs`) and truncates (`const_lit`)
* widen `const_lit` or check at elaborate
* oracles: iverilog rejects
* status: OPEN
