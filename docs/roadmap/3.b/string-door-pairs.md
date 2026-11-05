(§3.b string-door-pairs · RECORD · in Strings / heap) (nested string concatenations: every spelling has to change at once):
* (NOTE, not queued) — PRE gives a string concatenation nested in a concatenation and the same text through a `?:` or a `let` the same wrong bytes, so a design comparing the two sees them equal, as the oracles do
* fixing one door alone makes them unequal (ER §2.3 row 1): `x = {"<", {a, "t"}}; y = {"<", (k ? {a, "t"} : {a, "e"})}` (a = "a", k = 1) `x == y` PRE 3e541d67 `[at] [at] eq`, verilator 5.052 `[<at] [<at] eq` (iverilog 13.0 and sv2v vvp syntax error), Q-NESTCAT r1–r3 `[<at] [at] ne`
* the same through `if`, `case`, a testbench `exp != got` (`ok` → `MISMATCH`), a three-part concatenation, a replication and a `let` (W/qnestcat/replan4/cells/x X01–X04, X06, X07, X09–X11), and through a `?:` with nested arms that PRE gets right: b07 `{"<", (k ? {{a}, "t"} : {{a}, "e"}), ">"}` PRE `[<at>] 4` = verilator, r1–r3 `[at>] 3`
* inside the row, a split keeping PRE's value for an excluded nest breaks pairs too (r3: n12 `eq` → `ne`, n13 n14 n15 t13 t14 f22 f23; r2b, which excludes nothing, = the oracles)
* the other doors form no such pair (cells/y: X13 hierarchical, wrong and unequal on PRE; X14 class member and X15 package call, loud; X16 parameter, right; X17–X20 unpacked-struct member, array element, local call, `string'()`, move with the nest)
* so row 74's concatenation, replication, `?:` and `let` arms land in one slice
* status: RECORD
