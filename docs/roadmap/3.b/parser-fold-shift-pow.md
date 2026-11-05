(§3.b parser-fold-shift-pow · OPEN · in Parser accept) (the parser's constant fold lacks `>>`, `<<` and `**`):
* symptom: the parser's constant fold has no `>>`, `<<` or `**` arm (`hdl-parser/src/expr_primary.rs:33` `try_const_index`, :656 `cw_eval`), so a struct member width over `top_pkg::TL_DBW = (TL_DW>>3)` or `2 ** WdrAw` is E2002
* the three arms in the fold's own domain, measured with a control twin on every reader of the parser's constant table (member widths, `const_locals`, enum labels, generate indices)
* oracles: iverilog and sv2v → iverilog `A mask=a op=5 bits=8`
* witness: row-9 R01a, every OpenTitan page
* status: OPEN (§5.2 row 23)
