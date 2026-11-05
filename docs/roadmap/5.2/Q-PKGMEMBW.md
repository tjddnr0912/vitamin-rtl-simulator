(§5.2 Q-PKGMEMBW · rank ②) (a member write through a package alias of a packed struct is refused):
* symptom: a member write through a package alias of a packed struct is refused where both oracles run it: q's `typedef r::out_t out_t` over the unit's `import p::*` copy of p's `out_t` (the two layouts differ only in a nested member's re-spelled name, `p::in_t` / `r::in_t`), `q::out_t v; v.z = 4'h9; v.i.a = 2'b10;` is E3010 (undeclared hierarchical write target `v.z`) (row66.m04; with a package variable of `out_t`, row66.m24) where iverilog 13.0 and verilator 5.052 print `A m04 v=89`
* witness: p's `typedef r::st2 st` equal by layout to the unit-imported `q::st`, `p::st v; v.a = 2'b11;` E3010 `v.a` (grid2.dr04), both `A dr04 v=c0`
* cause: the alias twin is compared by layout `PartialEq`, which reads a nested member by its re-spelled name, and an equal layout registers no `p::` twin of its own (`hdl-parser/src/module_items.rs:717–743` `struct_fresh`)
* status: loud, pre-existing (PRE 01cac4b3 = row 66 POST)
* source: row 66 battery M04 / M22 / M24 discriminators (W/row66/r1/bat/disc m04 m24, W/row66/grid2/dr04; pins `type_record.rs` m04 m22 m24, KNOWN-WRONG)
