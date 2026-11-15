(§5.2 Q-REFTXT · rank — · source ENG-1b r1 lens-sound N6, r1 lens-diff N6, N10, r2 lens-sound N4, N5) (refusal text: duplicate lines, wrong columns and a wrong order):
* symptom: refusal text: duplicate lines for one cause (CW_whole_arr_evt, CW_whole_arr_nba, CL_size_arr; a named property asserted twice prints one by-name line per assertion, RP_rose_named_prop_two_uses), a bare use inside `$display` reports the statement's column, not the use's (K02 4:48), a refused clocking `$readmem` in a task body prints the type refusal before the read-only one (CF_readmem_task), "undeclared net/variable" for a declared unpacked struct / string array / package parameter / genvar source
* witness (Q-PARAM2STATE plan pass 1, W/param2state/plan/m1/cells/a06_gen_1u): `if (1) begin : g localparam logic [15:0] CV = (16'd1 % ('1 + 1'b1)); initial #1 $display("V c=%b", CV); end`: PRE 4d1d84bb rc 1, errors=2, the refusal of `CV` and then ``t.sv:2:93: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: undeclared net/variable `top.g[0].CV` [in top.g]``
  * oracles: iverilog and verilator `V c=xxxxxxxxxxxxxxxx`
* variation: its module-body twin a01_decl_1u and its package twin a05_pkg_1u print the refusal alone, errors=1
* note: one line per select is the rule since ENG-1b (F1s_clk_real_array, F3_two_selects_one_stmt)
* source: r2 lens-diff N4
