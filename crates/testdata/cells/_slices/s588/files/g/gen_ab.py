import os
S='/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s588/g'
def cell(d,name,body,disp):
    open(os.path.join(S,d,name+'.sv'),'w').write('module top;\n'+body+'\n  initial begin #1 $display('+disp+'); $finish; end\n  initial #100 $finish;\nendmodule\n')
# ---- group A: row cells (plain-if twins) and variable kinds
A={}
A['a01_x2a_ret4']=('''  function automatic logic [3:0] fx(input int a);
    if (a == 1) fx = 4'd10;
  endfunction
  localparam logic [3:0] P = fx(2);''','"P=%b", P')
A['a02_ret4_never']=('''  function automatic logic [3:0] fx(input int a);
    int k;
    k = a;
  endfunction
  localparam logic [3:0] P = fx(2);''','"P=%b", P')
A['a03_x2b_retint']=('''  function automatic int fi(input int a);
    if (a == 1) fi = 10;
  endfunction
  localparam int P = fi(2);''','"P=%0d", P')
A['a04_d10_untyped_px']=('''  function automatic logic [3:0] fx(input int a);
    if (a == 1) fx = 4'd10;
  endfunction
  parameter PX = fx(2);''','"PX=%b bx=%0d", PX, $bits(PX)')
A['a05_d10_untyped_pi']=('''  function automatic integer fi(input int a);
    if (a == 1) fi = 10;
  endfunction
  parameter PI = fi(2);''','"PI=%0d bi=%0d", PI, $bits(PI)')
A['a06_d10_ps_ctl']=('''  function automatic logic signed [5:0] fs(input int a);
    fs = -6'sd3;
    if (a == 1) fs = 6'sd10;
  endfunction
  parameter PS = fs(2);''','"PS=%0d bs=%0d neg=%0d", PS, $bits(PS), PS < 0')
A['a07_x2c_local4']=('''  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd10;
    f = t;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P')
A['a08_x2i_integer']=('''  function automatic integer f(input int a);
    integer t;
    if (a == 1) t = 10;
    f = t;
  endfunction
  localparam integer P = f(2);''','"P=%0d", P')
A['a09_x2l_bitwrite']=('''  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    t[0] = 1'b1;
    if (a == 1) t = 4'd10;
    f = t;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P')
A['a10_x2d_rangewrite']=('''  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    t[1:0] = 2'b11;
    if (a == 1) t = 4'd10;
    f = t;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P')
A['a11_ipwrite']=('''  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    t[0 +: 2] = 2'b11;
    if (a == 1) t = 4'd10;
    f = t;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P')
A['a12_ret_bitwrite']=('''  function automatic logic [3:0] f(input int a);
    f[1] = 1'b1;
    if (a == 1) f = 4'd10;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P')
A['a13_ret_allbits']=('''  function automatic logic [7:0] f(input int a);
    integer i;
    for (i = 0; i < 4; i = i + 1) f[i*2 +: 2] = i;
  endfunction
  localparam logic [7:0] P = f(2);''','"P=%b", P')
A['a14_x2k_bound']=('''  function automatic logic [3:0] fx(input int a);
    if (a == 1) fx = 4'd10;
  endfunction
  logic [fx(2):0] v;''','"b=%0d", $bits(v)')
A['a15_x2j_bitlocal']=('''  function automatic logic [3:0] f(input int a);
    bit [3:0] t;
    f = 4'd7;
    if (a == 1) f = 4'd10;
    f = f + t;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%0d", P')
A['a16_intlocal']=('''  function automatic int f(input int a);
    int t;
    f = t + 5;
  endfunction
  localparam int P = f(2);''','"P=%0d", P')
A['a17_byte_short_long']=('''  function automatic longint f(input int a);
    byte b; shortint s; longint l;
    f = b + s + l + 3;
  endfunction
  localparam longint P = f(2);''','"P=%0d", P')
A['a18_time_local']=('''  function automatic logic [63:0] f(input int a);
    time t;
    f = t;
  endfunction
  localparam logic [63:0] P = f(2);''','"P=%h", P')
A['a19_reg_local']=('''  function automatic logic [3:0] f(input int a);
    reg [3:0] t;
    f = t;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P')
A['a20_signed_local']=('''  function automatic logic signed [3:0] f(input int a);
    logic signed [3:0] t;
    f = t;
  endfunction
  localparam logic signed [3:0] P = f(2);''','"P=%b", P')
A['a21_implicit_ret']=('''  function automatic [3:0] f(input integer a);
    if (a == 1) f = 4'd10;
  endfunction
  localparam [3:0] P = f(2);''','"P=%b", P')
A['a22_integer_ret']=('''  function automatic integer f(input int a);
    if (a == 1) f = 10;
  endfunction
  localparam integer P = f(2);''','"P=%0d", P')
A['a23_else_only']=('''  function automatic logic [3:0] f(input int a);
    if (a == 2) begin end else f = 4'd3;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P')
A['a24_arg_2state_formal']=('''  function automatic int g(input int b);
    g = b + 1;
  endfunction
  function automatic int f(input int a);
    logic [3:0] t;
    f = g(t);
  endfunction
  localparam int P = f(2);''','"P=%0d", P')
A['a25_arg_4state_formal']=('''  function automatic logic [3:0] g(input logic [3:0] b);
    g = b;
  endfunction
  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    f = g(t);
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P')
A['a26_ret_2state_type']=('''  function automatic int f(input int a);
    logic [3:0] t;
    f = t;
  endfunction
  localparam int P = f(2);''','"P=%0d", P')
A['a27_2state_local_from_x']=('''  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    int k;
    k = t;
    f = k;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P')
A['a28_x2m_order']=('''  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  function automatic logic [3:0] g(input int a);
    if (a == 1) g = 4'd1;
  endfunction
  localparam int Q = f(2);
  localparam logic [3:0] P = g(3);''','"Q=%0d P=%b", Q, P')
A['a29_x2g_sibling_int']=('''  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  function automatic logic [3:0] g(input int a);
    if (a == 1) g = 4'd1;
  endfunction
  localparam int P = f(2) + g(3);''','"P=%0d", P')
A['a30_x2f_callee']=('''  function automatic int g(input int a);
    g = 7;
    if (a == 1) g = 10;
  endfunction
  function automatic logic [3:0] f(input int a);
    if (g(a) == 3) f = 4'd1;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P')
A['a31_ret_int_from_xret']=('''  function automatic logic [3:0] g(input int a);
    if (a == 1) g = 4'd1;
  endfunction
  localparam int P = g(3);''','"P=%0d", P')
A['a32_local_int_lp_xlocal']=('''  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd10;
    f = t;
  endfunction
  localparam int P = f(2);''','"P=%0d", P')
A['a33_bitlp_xlocal']=('''  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    t[0] = 1'b1;
    f = t;
  endfunction
  localparam bit [3:0] P = f(2);''','"P=%b", P')
A['a34_blocklocal']=('''  function automatic logic [3:0] f(input int a);
    begin : b
      logic [3:0] t;
      f = t;
    end
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P')
A['a35_loopvar_decl']=('''  function automatic logic [3:0] f(input int a);
    f = 0;
    for (integer i = 0; i < 3; i = i + 1) f = f + 1;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P')
A['a36_struct_local']=('''  typedef struct packed { logic [1:0] a; logic [1:0] b; } s_t;
  function automatic logic [3:0] f(input int a);
    s_t s;
    f = s;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P')
A['a37_wide_ret70']=('''  function automatic logic [69:0] f(input int a);
    if (a == 1) f = 70'd5;
  endfunction
  localparam logic [69:0] P = f(2);''','"P=%h", P')
A['a38_ret64']=('''  function automatic logic [63:0] f(input int a);
    if (a == 1) f = 64'd5;
  endfunction
  localparam logic [63:0] P = f(2);''','"P=%h", P')
A['a39_init_local_ctl']=('''  function automatic logic [3:0] f(input int a);
    logic [3:0] t = 4'd3;
    f = t;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P')
A['a40_assigned_before_read_ctl']=('''  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    t = a;
    f = t + 1;
  endfunction
  localparam logic [3:0] P = f(2);''','"P=%b", P')
for k,(b,dsp) in A.items(): cell('a',k,b,dsp)
# ---- group B: operator classes over an unassigned logic [3:0] local t
def opcell(name, expr, rtype='logic [3:0]', disp='"P=%b", P', pre='', ltype=None):
    lt = ltype or rtype
    body=f'''  function automatic {rtype} f(input int a);
    logic [3:0] t;
    logic [3:0] c;
    integer i;
    c = 4'b1010;
    if (a == 1) t = 4'd5;
{pre}    f = {expr};
  endfunction
  localparam {lt} P = f(2);'''
    cell('b',name,body,disp)
B=[
('b01_add','t + 4\'d1'),('b02_sub','t - 4\'d1'),('b03_mul0','t * 4\'d0'),('b04_div','t / 4\'d1'),('b05_mod','t % 4\'d2'),
('b06_and0','t & 4\'b0000'),('b07_and3','t & 4\'b0011'),('b08_orF','t | 4\'b1111'),('b09_or3','t | 4\'b0011'),('b10_xor0','t ^ 4\'b0000'),
('b11_not','~t'),('b12_neg','-t'),('b13_pow','t ** 2'),('b14_xnor','t ~^ 4\'b0000'),
('b15_shl1','t << 1'),('b16_shr4','t >> 4'),('b17_shl_by_x','4\'d1 << t'),('b18_ashr','t >>> 1'),('b19_shl4','t << 4'),
('b20_tern_cond','t ? 4\'d1 : 4\'d2'),('b21_tern_same','t ? 4\'d5 : 4\'d5'),('b22_tern_arm','(a == 2) ? t : 4\'d1'),
('b23_concat','{t[1:0], 2\'b01}'),('b24_concat_whole','{2\'b01, t} >> 2'),('b25_repl','{2{t}} >> 4'),
('b26_bitsel','{3\'b000, t[0]}'),('b27_partsel','{2\'b00, t[1:0]}'),('b28_index_by_x','{3\'b000, c[t]}'),
('b29_sizecast','4\'(t)'),('b30_primcast_int','int\'(t)'),('b31_unsigned','$unsigned(t)'),('b32_clog2','$clog2(t)'),('b33_bits','$bits(t)'),
('b34_self_inc','t + t'),('b35_sub_self','t - t'),('b36_xor_self','t ^ t'),
]
for n,e in B: opcell(n,e)
# 1-bit results
B1=[
('b40_redand','&t'),('b41_redor','|t'),('b42_redxor','^t'),('b43_land0','t && 1\'b0'),('b44_land1','t && 1\'b1'),('b45_lorl','t || 1\'b1'),('b46_lor0','t || 1\'b0'),('b47_lnot','!t'),
('b48_lt','t < 4\'d3'),('b49_eq_self','t == t'),('b50_eq0','t == 4\'d0'),('b51_ne0','t != 4\'d0'),('b52_ceq_self','t === t'),('b53_ceq_x','t === 4\'bxxxx'),('b54_cne0','t !== 4\'d0'),('b55_ge0','t >= 4\'d0'),
('b56_redand_masked','&(t & 4\'b0011)'),('b57_redor_masked','|(t | 4\'b0001)'),('b58_wildeq','t ==? 4\'b????'),('b59_inside','t inside {4\'d0}'),
]
for n,e in B1: opcell(n,e,rtype='logic',disp='"P=%b", P')
# control flow cells
CF={
'b60_if_eq1':"    if (t == 4'd1) f = 4'd1; else f = 4'd2;\n",
'b61_if_eq0':"    if (t == 4'd0) f = 4'd1; else f = 4'd2;\n",
'b62_if_t':"    if (t) f = 4'd1; else f = 4'd2;\n",
'b63_if_not':"    if (!t) f = 4'd1; else f = 4'd2;\n",
'b64_if_ceqx':"    if (t === 4'bxxxx) f = 4'd1; else f = 4'd2;\n",
'b65_if_ne0':"    if (t != 4'd0) f = 4'd1; else f = 4'd2;\n",
'b66_for_bound':"    f = 4'd0;\n    for (i = 0; i < t; i = i + 1) f = f + 4'd1;\n",
'b67_while_t':"    f = 4'd0;\n    while (t) begin f = f + 4'd1; t = t - 4'd1; end\n",
'b68_while_lt':"    f = 4'd0;\n    i = 0;\n    while (t < 4'd3 && i < 5) begin f = f + 4'd1; i = i + 1; end\n",
'b69_repeat_t':"    f = 4'd0;\n    repeat (t) f = f + 4'd1;\n",
'b70_return_x':"    return t;\n",
'b71_if_ceq_self':"    if (t === t) f = 4'd1; else f = 4'd2;\n",
'b72_loop_overwrite':"    for (i = 0; i < 4; i = i + 1) t[i] = 1'b1;\n    f = t;\n",
'b73_if_land':"    if (t == 4'd1 && 1'b0) f = 4'd1; else f = 4'd2;\n",
'b74_if_lor':"    if (t == 4'd1 || 1'b1) f = 4'd1; else f = 4'd2;\n",
}
for n,s in CF.items():
    body=f'''  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    integer i;
    if (a == 1) t = 4'd5;
{s}  endfunction
  localparam logic [3:0] P = f(2);'''
    if 'return' in s or 'f = t;' in s:
        pass
    cell('b',n,body,'"P=%b", P')
# 2-state target twins: int return converting the x-bearing expression
B2=[('b80_int_add','t + 4\'d1'),('b81_int_or3','t | 4\'b0011'),('b82_int_and0','t & 4\'b0000'),('b83_int_concat','{t[1:0], 2\'b01}'),('b84_int_eq0','t == 4\'d0'),('b85_int_shl1','t << 1'),('b86_int_not','~t'),('b87_int_tern','t ? 4\'d1 : 4\'d2')]
for n,e in B2: opcell(n,e,rtype='int',disp='"P=%0d", P')
