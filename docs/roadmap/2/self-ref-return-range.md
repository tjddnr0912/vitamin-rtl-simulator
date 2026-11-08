(§2 self-ref-return-range · OPEN · in Ranges / bounds / selects) (a function whose return range calls itself overflows the stack):
* `function [f():0] f();` overflows the stack (`const_fn_ret_wsign`: `depth + 1`)
* witness: r_msb_self `module top; function automatic logic [h(1):0] h(input int a); h = a; endfunction localparam int K = h(5); initial begin #1 $display("A k=%0d", K); $finish; end endmodule`: PRE d59629fa `thread 'vita-main' has overflowed its stack` `fatal runtime error: stack overflow`, signal 6 (W/qvalrulesd/premise6/b4/r_msb_self)
* witness: r_lsb_self_ok the same with `logic [7 : h(1)] h`: the same two lines (W/qvalrulesd/premise6/b4/r_lsb_self_ok)
* witness: e2_decl_only the declaration alone, `h` never called (`initial begin #1 $display("A done"); $finish; end`): the same two lines (W/qvalrulesd/premise7/probe/e2_decl_only)
* witness: a `$unit` routine (u_msb_self, u_lsb_self_ok) and the gated site (g_rec_ok) the same, a package routine stays loud (pk_*, `decl_win`) (W/qvalrulesd/premise7/REPORT.txt N-3)
* cause: `const_fn_ret_wsign` (`elaborate/src/inline_fn.rs:128`) re-derives the return range on every constant call of the routine and the router starts each bound at depth 0, so the depth cap never trips; `decl_win` guards package routines only (`elaborate/src/decl_scope.rs:142`) (W/qvalrulesd/plan8/filings.txt E2, premise judge r6 J3)
* note: a crash where a loud refusal belongs: PRE refuses at once when the bound also reads a net (r_lsb_self_net, `int v;` and `[v : h(1)] h`: exit 1, `t.sv:6:22: E3009 parameter `K` value is not a constant: `h(…)` has no constant-fold arm [in top]` and `t.sv:3:29: E3009 a reference to net/variable `v` is not allowed in a constant range bound [in top]`, W/qvalrulesd/RULINGS-premise6.txt), the same short-circuit Q-VALRULES fix (d) keeps (§4.5.636)
* fix: a depth guard on that path, a loud refusal in place of the overflow
* OPEN
