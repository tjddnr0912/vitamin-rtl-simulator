import os
S='/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s588/g/a2'
def cell(name,body,disp,pre=''):
    open(os.path.join(S,name+'.sv'),'w').write('module top;\n'+pre+body+'\n  initial begin #1 $display('+disp+'); $finish; end\n  initial #100 $finish;\nendmodule\n')
C={}
C['a41_rmw_unbound']=('''  function automatic int f(input int a);
    int t = int'(2.5);
    t[0] = 1'b0;
    f = t;
  endfunction
  localparam int P = f(2);''','"P=%0d", P','')
C['a42_write_x_index']=('''  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    f = 4'b0000;
    f[t] = 1'b1;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P','')
C['a43_ret4_read_before']=('''  function automatic logic [3:0] f(input int a);
    f = f + 4'd1;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P','')
C['a44_ret2_read_before']=('''  function automatic int f(input int a);
    f = f + 1;
  endfunction
  localparam int P = f(2);''','"P=%0d", P','')
C['a45_retbit_never']=('''  function automatic bit [3:0] f(input int a);
    if (a == 1) f = 4'd3;
  endfunction
  localparam bit [3:0] P = f(2);''','"P=%b", P','')
C['a46_typedef_bit_local']=('''  typedef bit [3:0] bt;
  function automatic logic [3:0] f(input int a);
    bt t;
    f = t + 4'd2;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P','')
C['a47_typedef_logic_local']=('''  typedef logic [3:0] lt;
  function automatic logic [3:0] f(input int a);
    lt t;
    f = t;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P','')
C['a48_typeparam_logic']=('''  parameter type T = logic [3:0];
  function automatic logic [3:0] f(input int a);
    T t;
    f = t;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P','')
C['a49_typeparam_bit']=('''  parameter type T = bit [3:0];
  function automatic logic [3:0] f(input int a);
    T t;
    f = t + 4'd2;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P','')
C['a50_while_uninit_i']=('''  function automatic logic [3:0] f(input int a);
    integer i;
    f = 4'd0;
    while (i < 3) begin f = f + 4'd1; i = i + 1; end
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P','')
C['a51_accum_ret4']=('''  function automatic logic [3:0] f(input int a);
    integer i;
    for (i = 0; i < 4; i = i + 1) f = f + 4'd1;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P','')
C['a52_dead_self_assign']=('''  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    t = t;
    f = 4'd3;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P','')
C['a55_known_bit_after_partial']=('''  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    t[0] = 1'b1;
    f = {3'b000, t[0]};
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P','')
C['a57_x_and_call']=('''  function automatic logic [3:0] g(input int a);
    g = 4'd0;
  endfunction
  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    f = t & g(0);
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P','')
C['a59_self_inc_local']=('''  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    t = t + 4'd1;
    f = t;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P','')
C['a61_static_fn']=('''  function logic [3:0] f(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    f = t;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P','')
C['a62_ret2_bitwrite']=('''  function automatic int f(input int a);
    f[0] = 1'b1;
  endfunction
  localparam int P = f(2);''','"P=%0d", P','')
C['a63_init_x_literal']=('''  function automatic logic [3:0] f(input int a);
    logic [3:0] t = 'x;
    f = 4'd3;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P','')
C['a64_int_from_x_and0']=('''  function automatic int f(input int a);
    logic [3:0] t;
    int k;
    k = t * 0;
    f = k;
  endfunction
  localparam int P = f(2);''','"P=%0d", P','')
C['a65_if_gt5']=('''  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    if (t > 4'd5) f = 4'd1; else f = 4'd2;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P','')
C['a66_int_sub_self']=('''  function automatic int f(input int a);
    logic [3:0] t;
    f = t - t;
  endfunction
  localparam int P = f(2);''','"P=%0d", P','')
C['a67_lp_int_retinteger']=('''  function automatic integer f(input int a);
    integer t;
    f = t;
  endfunction
  localparam int P = f(2);''','"P=%0d", P','')
C['a68_byte_ret_x']=('''  function automatic byte f(input int a);
    logic [7:0] t;
    t[3:0] = 4'b0101;
    f = t;
  endfunction
  localparam int P = f(2);''','"P=%0d", P','')
for k,(b,d,pre) in C.items(): cell(k,b,d,pre)
