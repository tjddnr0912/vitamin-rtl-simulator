(§2 block-local-named-function · OPEN · in Constant domain (i64)) (a block-local named like its function is the return variable):
* A block-local named like its function is the return variable to the constant interpreter (one flat env): `function automatic logic [3:0] f(); begin logic [3:0] f; f = 4'd1; end endfunction` folds `P=1` (c80_shadow_ret, review dr1 N2), and `begin int f; end` reads the same slot (s12 `P=0000`, verilator `xxxx`, iverilog and sv2v refuse): §4.5.633's `shadowed` flag marks both, the value stays
* verilator `P=x`, sv2v `P=1`, iverilog refuses
* IEEE 1800-2017 §6.21: the block's `f` is another variable, and the return variable is never written (§6.8)
* verilator + hand-IEEE.
* The same family, outside the mark, in a package routine only and on invalid text (IEEE 1800-2017 §13.4.1, the implicit variable is in that scope; a module twin is E3009): a local named `f` narrower than `logic [7:0] f`: h2_narrow_read `logic [3:0] f; f[0]=1'b1; f[1]=1'b1; f[2]=1'b1; f[3]=1'b1; return f;` `00001111`, iverilog `xxxx1111`, sv2v `00001111`, verilator `Non-ANSI I/O declaration of signal conflicts with type declaration: 'f'`
* h2_narrow_past, the same four writes, then `f[5]=1'b1;` and no return, `00101111`, iverilog `xx1x1111`, sv2v `Unable to evaluate parameter P value: top.p_f(32'sd1)`, verilator as h2_narrow_read
* a formal named like its function: h2_formal `function automatic logic [7:0] f(input int f);` and an empty body, `p::f(1)` `00000001`, iverilog `Function p.f port f is not an input port.`, sv2v `Unable to evaluate parameter P value: top.p_f(32'sd1)`, verilator `Duplicate declaration of signal: 'f'`
* h2_formal_ret, the same with `return f;`, `00000001`, sv2v `00000001`, iverilog and verilator as h2_formal
* a 2-state body local: sh_pkg_body_int (b_int) `function automatic logic [3:0] g(input int a); int g; endfunction` `00000000`, iverilog `Unable to evaluate parameter P value: <select=q.g(32'sd1)[(0)+:8] expr_type=logic>`, sv2v the same over `top.q_g`, verilator `Non-ANSI I/O declaration of signal conflicts with type declaration: 'g'`.
* §4.5.633's set is empty on all five
* renamed, h2_ren_narrow marks `f.f` and b_int_tw marks `g.g`
* OPEN
