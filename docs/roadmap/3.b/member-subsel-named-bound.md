(§3.b member-subsel-named-bound · OPEN · in Parser accept) (a struct member part-select with named bounds, read or written):
* a packed-struct member part-select whose bounds name a constant is E2002 at parse (`cam[i].tag[NBLOAD_TAG_MSB:0]`, EL2 / EH1: all three `c=00110000 r=11`; `tl_i.a_address[RegAw-1:2]`, every OpenTitan page: all three `A o=fc`; the write side `h.a[P-1:0] = …`, `tl_sram_o.a_address[AccessSize-1:0] = …`: `A h=2d4`), and a member sub-select converted to an lvalue is E3009 `nested lvalue select (v1: single-level)` (`elaborate/src/lvalue.rs:572`) even with literal bounds: an output-port actual (`.dout(p.index[3:0])`, EH1: all three `p0=1011010 p1=0100101`), a task or function output actual, a gate output terminal (a split)
* cause: `hdl-parser/src/struct_sel.rs:726 / 876 / 913` `const_lit`
* fix: the read desugar marks a member's part-select and elaborate folds a named bound at the use and lowers a marked nested lvalue one level
* witness: row-9 T1, row-9 T2, row-9 R05, row-9 R13 (a symbolic layout's sub-select is ⑤ⓓ's row-9 R21)
* status: OPEN (§5.2 row 24)
