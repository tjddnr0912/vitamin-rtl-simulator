(§5.2 Q-ESCSCOPE · rank ① · source Q-STRBYTES design review z14 M3, z11 M1 M2 (N1); row 33 r3 plan F1) (`%m` keeps an escaped identifier's backslash):
* symptom: `%m` keeps an escaped identifier's backslash: `initial begin : \blk+1  $display("M3 [%m]");` prints `M3 [top.\blk+1]` (PRE 9d60771d and Q-STRBYTES §4.5.629 alike, native and vm) where iverilog 13.0, sv2v → iverilog and verilator 5.052 print `M3 [top.blk+1]`
* IEEE 1800-2017 §5.6.1 (ieee1800-2017.txt:2852–2853: the leading backslash and the terminating white space are not part of the identifier)
* witness: a non-ASCII escaped name the same (`begin : \bé` `%12m` `top.\b\xc3\xa9`, iverilog = sv2v `top.b\xc3\xa9`)
* fix: census the scope-name producers `%m` reads (the escaped spelling is kept for the source)
* row 33 r3 (PLAN4 F1): every resolver reads `\x ` and `x` as two names, so legal programs are refused (the parser keeps the backslash in `Ident.name`, `hdl-ast/src/lib.rs:115-118`): `\f ()` for `f` (r_call_ep) and `f` for `\f ` (r_call_pe) E3010 `call to undeclared function`, verilator and iverilog `R 7`
* nets, parameters and labels (r_net_ep, r_net_pe, r_param_ep, r_label_ep) E3010
* types (r_type_ep, r_type_pe) E2002
* `\p ::X` and `p::\X ` (r_pkg_ep, r_pkg_pe, r_pname_ep) E3009
* `import p::\X ` (r_imp_ep) E3009 + E3010
* a duplicate is accepted: `localparam int \X  = 5; localparam int X = 6;` (r_dup_pe) `R 6`, all three refuse the duplicate, w_escdup3c `R 14 15`
* PRE = r3
* W/row33/plan4/p1, p3
* an escaped identifier the parser does not end at white space (row 33 lens r3, lens3-diff N4; W/row33/lens3-diff/d1): `logic \"q  = 1'b1;` E1013 `unterminated string literal` (e27_quote; verilator `R ok 1`), `'{\a : 1'b1, b: 1'b0}` E2002 `expected an assignment-pattern key naming a member of this struct` (e21_patkey; verilator `R ok 2`)
* PRE = r3 = main's e0888973
* source: lens r3 diff N4
