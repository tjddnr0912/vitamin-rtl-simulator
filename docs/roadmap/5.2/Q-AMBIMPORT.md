(§5.2 Q-AMBIMPORT · rank ②) (a type name offered by two wildcard imports binds instead of being refused):
* symptom: two wildcard imports offering one TYPE name bind it where IEEE 1800-2017 §26.3 (ieee1800-2017.txt:35875–35876) makes the reference illegal: with an enum `st` in pa and a packed struct `st` in pb, `import pa::*; import pb::*; st v;` binds pa's (w2amb.mod.en_ps `A amb 3`, verilator 5.052 `A amb 3`, iverilog 13.0 `Ambiguous use of 'st'. It is exported by both 'pa' and by 'pb'.`), and a member write on it reads x (w2amb.mem.en_ps `A mem b=3 v=7 a=x`, iverilog ambiguous, verilator `%Error-ENUMVALUE`)
* symptom: the per-kind copy mixes the two: pa's packed struct with pb's unpacked record answers the member slot from pb (w2amb.mem.ps_rc `A mem b=8 a=3 b=5`, verilator `A mem b=6 a=3 b=5` from pa, iverilog ambiguous)
* symptom: q's packed struct then p's packed union, `b_t v; v = '{4'h1, 4'h2};` is E3009 `an assignment pattern '{…} is supported only as the whole right-hand side of an unpacked array assignment` (row66.m18; iverilog ambiguous, verilator `A m18 v=12`)
* scope: the constant half is §2's adjacency line (§4.5.591, `elaborate` `package.rs`), this the parser's type half (`hdl-parser/src/imports.rs` wildcard arm, `type_rec.rs:371` `copy_wildcard_slots`, IfAbsent per kind)
* fix: refuse a reference to a type name two wildcard imports of one scope offer
* status: pre-existing (PRE 01cac4b3 = row 66 POST)
* source: row 66 lens-diff r1 A2 (W/row66/r1/lens-diff/orc/wk w2amb.mod.en_ps, w2amb.mem.en_ps, w2amb.mem.ps_rc); battery disc m18
