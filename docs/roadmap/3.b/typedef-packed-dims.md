(§3.b typedef-packed-dims · OPEN · in Parser accept) (packed dimensions on a typedef'd type outside the composed forms):
* scope: packed dimensions on a typedef'd type outside the page half (§5.2 row 41's residue): composed for a variable or an ANSI port
* inside a package also for a member of a named packed struct, an alias, a parameter of the alias and an alias ANSI port (`structs/member_array.rs`), each with a packed struct element of the same package (or `pkg::name`), named by its own declaration, with no sign keyword, whose members are `logic`, `logic [N:0]`, such a vector name, or such structs one level down, and one `[hi:0]` dimension, and for a function return and an `input` formal with a `typedef logic [D:0]` base of the same package (D a decimal literal) and literal bounds
* a vector name that a packed struct or union in `$unit` or a wildcard-imported package also names is refused (W/row41/lens3-sound/sets/n/t01_unit_struct_pkg_page, W/row41/r2c/probe/src/pw1_wild_struct: iverilog, sv2v and verilator run it)
* every other spelling is E2002 and names the form
* module and `$unit` scope keep `a simple chained typedef alias (adding packed dimensions to an aliased type is unsupported in v1)` (`hdl-parser/src/typedefs.rs:572–581`, `structs/mod.rs:45–83`) until row 68's bases, a composed md parameter outside a package until row 70 (PROBE_CATALOG §4.5.570's typedef'd-member row)
* oracles: iverilog and sv2v → iverilog `A v1.w=01 v2.r=10 bits=12`
* witness: row-9 R04 (row-9 R12, row-9 R18, row-9 R20)
* status: OPEN (§5.2 row 41)
