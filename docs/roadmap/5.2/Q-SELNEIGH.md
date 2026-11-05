(§5.2 Q-SELNEIGH · rank ① · source Q-PKRDLSB plan R-7 (W/qpkrdlsb/PLAN.txt:71)) (neighbours of the element select: odd index kinds, generate `let`, class scope):
* slot: past the last numbered slot
* slot: starts only on a track whose next C row cannot start
* scope: neighbours of the element select that Q-PKRDLSB's plan and design review raised, no page or corpus line
* every text below is PRE 719e37d2 re-measured 2026-11-02 (W/qpkrdlsb/park/nine, cells and outputs), and a `q[..][1][m:l]` value in it is Q-PKRDLSB's raw read: Q-PARENBASE — a part-select of a parenthesized element is accepted, W2004 and a value: `logic [3:0][5:2] q [2]; (q[1][1])[4:3]` (q[1] = 16'h4321) `R 0 x0`, sv2v `R 0 01`, iverilog `f_paren_base.sv:6: syntax error`, verilator `syntax error, unexpected '[', expecting ')' or ','`
* fix: first: decide refuse or read, then the raw path's normalization
* symptom: Q-HANDLEIDX — a class handle as an index is accepted, silent on illegal source: `class C; endclass C h; q[h][1][4:3]` `R 0 x0`, iverilog `kd_handle.sv:7: Error: Class/null r-value not allowed in this context.`, sv2v a parse error, verilator `Expected numeric type, but got a 'class{}C' data type`
* fix: first: the check a real variable already meets (E3009 `a select index / bound / size must be integral, not real`, `elaborate/src/packed.rs:314`, `:339`)
* symptom: Q-REALPIDX — a real parameter as an index passes that check: `logic [3:0][9:6] q [0:15]; parameter real RP = 1.0;` `q[RP][1]` `R 0 1101`, `q[RP][1][8:7]` `R 1 xx`, verilator `R 0 1101` `R 1 10`, iverilog and sv2v crash (`Assertion failed: (0), function ivl_expr_signal, file t-dll-api.cc, line 705.`), a real VARIABLE is E3009
* fix: first: read the parameter's type in that check, then the conversion
* symptom: Q-STRPIDX — a string parameter as an index reads as its bytes: `parameter string SP = "\001";` `q[SP][1]` `R 0 1101`, `q[SP][1][8:7]` `R 1 xx`, iverilog = sv2v `R 0 1101` `R 1 10`, verilator `Expected numeric type, but got a 'string' data type`
* fix: first: decide whether a string index is legal here
* symptom: Q-LETGENIGN — a `let` declared in a generate block is ignored by name lookup where an outer variable has the name: `logic [7:0] k = 1; generate if (1) begin : g let k = 64'h1_0000_0001; initial #2 $display("R 2 %0d %b %b", k, q[k][1], q[k][1][8:7]); end endgenerate` (l_let_tw, b_let) `R 2 1 1101 xx`, hand `R 2 4294967297 xxxx xx`
* oracles: iverilog `sorry: let declarations (k) are not currently supported.`, sv2v a parse error, verilator `Found definition of 'k' as a VAR but expected a task/function`: by hand only
* note: §3's `let` over an outer parameter is the constant twin
* fix: first: the scope `let` registers in
* status: silent-wrong
* symptom: Q-CLASSMODARR — a class declared in a module cannot read the module's names, the array is one case: `class C; int i = 2; function logic [1:0] g(); return q[i][1][4:3]; endfunction endclass` E3010 `undeclared net/variable $class$C$g.q`, iverilog = verilator `R 0 11`, sv2v a parse error (n_cls_field, n_cls_formal, n_cls_field_shadow)
* pins: control cls_ctl, a scalar `v`, a word `m[1]`, an element `q[1][1]`: E3010 on each, iverilog = verilator `R 0 5 5a 2`
* status: loud
* symptom: Q-NETKINDIDX — the refusal is the declaration's, not the index's: `tri1 [3:0] k = 2; q[k][1][4:3]` and `supply0 k; q[k][1][4:3]` E3009 `unsupported net/var kind (v1)` at the declaration (`elaborate/src/netdecl.rs:120`, documented `docs/manual/006_limitations.md:52`), iverilog = sv2v = verilator `R 0 11` / `R 0 00`
* status: loud
* status: all pre-existing
* symptom: two more on the keyword md member, funnel `parse_struct_field_sel`'s stride arm: W/row41/probe/cells2/p1_md_narrow.nat.sv `logic [1:0] i=3; x=s.m[i+i]`: PRE `A s=xxx v=111`, verilator `A s=111 v=111`, sv2v `A s=xxx v=xxx`, iverilog refuses (``A reference to a net or variable (`i') is not allowed in a constant expression``), IEEE self-determined index 2, 111
* p2_md_uwrap.nat.sv `int unsigned i=32'h55555556; x=s.m[i]`: PRE `A s=001 v=xxx`, verilator `A s=001 v=001`, sv2v `A s=001 v=001`, iverilog refuses, IEEE out of range, x
* note: row 41's forms reach neither
* source: design review N-1 N-2 N-3 N-4 (W/qpkrdlsb/RULINGS-dr1.txt:15, cells W/qpkrdlsb/review-design/c)
