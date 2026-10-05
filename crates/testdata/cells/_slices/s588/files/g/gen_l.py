import os
S='/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s588/g/l'
def fn(name, decl, kind, w='logic [3:0]'):
    # kind p = if (PRE folds 0-seeded), d = case (PRE declines = POST proxy)
    sel = "    if (a == 1) t = 4'd5;\n" if kind=='p' else "    case (a) 1: t = 4'd5; endcase\n"
    return f'''  function automatic {w} {name}(input int a);
    logic [3:0] t;
{decl}{sel}    {name} = t;
  endfunction
'''
FX  = lambda k: fn('fx','',k)
FX1 = lambda k: fn('fx1',"    t[0] = 1'b1;\n",k)
L = {}
L['l01_lp4']      = ('{FX}  localparam logic [3:0] P = fx(2);\n', '"P=%b", P', '')
L['l02_untyped']  = ('{FX}  parameter P = fx(2);\n', '"P=%b b=%0d", P, $bits(P)', '')
L['l03_lpint_x1'] = ('{FX1}  localparam int P = fx1(2);\n', '"P=%0d", P', '')
L['l04_lpinteger']= ('{FX}  localparam integer P = fx(2);\n', '"P=%0d", P', '')
L['l05_lpbit_x1'] = ('{FX1}  localparam bit [3:0] P = fx1(2);\n', '"P=%b", P', '')
L['l06_packed_bound'] = ('{FX}  logic [fx(2):0] v;\n', '"b=%0d", $bits(v)', '')
L['l07_unpacked_dim'] = ('{FX}  logic v [fx(2):0];\n', '"s=%0d", $size(v)', '')
L['l08_repl_count'] = ('{FX}  localparam logic [7:0] P = {{fx(2){{1\'b1}}}};\n', '"P=%b", P', '')
L['l09_ipsel_width']= ('{FX}  localparam logic [7:0] V = 8\'hA5;\n', '"r=%b", V[0 +: fx(2)]', '')
L['l10_psel_msb']   = ('{FX}  localparam logic [7:0] V = 8\'hA5;\n', '"r=%b", V[fx(2):0]', '')
L['l11_gen_if']     = ('{FX}  if (fx(2)) begin : g1 initial $display("T"); end else begin : g2 initial $display("E"); end\n', '"done"', '')
L['l12_gen_if_eq0'] = ('{FX}  if (fx(2) == 4\'d0) begin : g1 initial $display("T"); end else begin : g2 initial $display("E"); end\n', '"done"', '')
L['l13_gen_case']   = ('{FX}  case (fx(2)) 4\'d0: begin : g0 initial $display("C0"); end default: begin : gd initial $display("CD"); end endcase\n', '"done"', '')
L['l14_gen_for']    = ('{FX}  for (genvar i = 0; i < fx(2); i = i + 1) begin : g initial $display("I%0d", i); end\n', '"done"', '')
L['l15_case_label'] = ('{FX}  initial begin case (4\'d0) fx(2): $display("M"); default: $display("D"); endcase end\n', '"done"', '')
L['l16_enum4']      = ('{FX1}  typedef enum logic [3:0] {{A = fx1(2)}} e_t;\n', '"A=%b", A', '')
L['l17_enum2']      = ('{FX1}  typedef enum bit [3:0] {{A = fx1(2)}} e_t;\n', '"A=%b", A', '')
L['l18_bits_call']  = ('{FX}  localparam int B = $bits(fx(2));\n', '"B=%0d", B', '')
L['l23_repeat']     = ('{FX}  int n;\n  initial begin n = 0; repeat (fx(2)) n = n + 1; $display("n=%0d", n); end\n', '"done"', '')
L['l24_delay']      = ('{FX}  initial begin #(fx(2)) $display("t=%0t", $time); end\n', '"done"', '')
L['l25_and0']       = ('{FX}  localparam logic [3:0] P = fx(2) & 4\'b0000;\n', '"P=%b", P', '')
L['l26_ceqx']       = ('{FX}  localparam logic P = (fx(2) === 4\'bxxxx);\n', '"P=%b", P', '')
L['l27_int_plus']   = ('{FX}  localparam int P = fx(2) + 1;\n', '"P=%0d", P', '')
L['l28_intcast_x1'] = ('{FX1}  localparam int P = int\'(fx1(2));\n', '"P=%0d", P', '')
L['l29_clog2']      = ('{FX}  localparam int P = $clog2(fx(2));\n', '"P=%0d", P', '')
L['l30_gen_lp']     = ('{FX}  if (1) begin : g localparam logic [3:0] P = fx(2); end\n', '"P=%b", g.P', '')
L['l31_var_init']   = ('{FX}  logic [3:0] w = fx(2);\n', '"w=%b", w', '')
L['l32_cont_assign']= ('{FX}  wire [3:0] w = fx(2);\n', '"w=%b", w', '')
L['l33_rt_display'] = ('{FX}', '"r=%b", fx(2)', '')
L['l34_lp4_x1']     = ('{FX1}  localparam logic [3:0] P = fx1(2);\n', '"P=%b", P', '')
L['l35_tern_x']     = ('{FX}  localparam logic [3:0] P = (fx(2) == 4\'d0) ? 4\'d1 : 4\'d2;\n', '"P=%b", P', '')
L['l36_lpint_x']    = ('{FX}  localparam int P = fx(2);\n', '"P=%0d", P', '')
L['l37_eq_cmp_int'] = ('{FX1}  localparam int P = (fx1(2) == 4\'d1);\n', '"P=%0d", P', '')
L['l38_lnot']       = ('{FX}  localparam int P = !fx(2);\n', '"P=%0d", P', '')
for k,(body,disp,_) in L.items():
    for kind in 'pd':
        b = body.replace('{FX}', FX(kind)).replace('{FX1}', FX1(kind)).replace('{{','{').replace('}}','}')
        open(os.path.join(S,f'{k}_{kind}.sv'),'w').write('module top;\n'+b+'  initial begin #1 $display('+disp+'); $finish; end\n  initial #100 $finish;\nendmodule\n')
# multi-module lanes
def mm(name, text):
    open(os.path.join(S,name+'.sv'),'w').write(text)
for kind in 'pd':
    f = FX(kind)
    mm(f'l19_hdr_default_{kind}', f'''module child #(parameter logic [3:0] P = fx(2)) ();
{f}  initial begin #1 $display("P=%b", P); end
endmodule
module top;
  child u();
  initial #100 $finish;
endmodule
''')
    mm(f'l20_override_{kind}', f'''module child #(parameter logic [3:0] P = 4'd9) ();
  initial begin #1 $display("P=%b", P); end
endmodule
module top;
{f}  child #(.P(fx(2))) u();
  initial #100 $finish;
endmodule
''')
    mm(f'l21_package_{kind}', f'''package pk;
{f}  localparam logic [3:0] P = fx(2);
endpackage
module top;
  initial begin #1 $display("P=%b", pk::P); $finish; end
  initial #100 $finish;
endmodule
''')
    mm(f'l22_inst_array_{kind}', f'''module child #(parameter logic [3:0] P = fx(2)) ();
{f}  initial begin #1 $display("%m P=%b", P); end
endmodule
module top;
  child u[1:0] ();
  initial #100 $finish;
endmodule
''')
    mm(f'l39_override_int_{kind}', f'''module child #(parameter int P = 9) ();
  initial begin #1 $display("P=%0d", P); end
endmodule
module top;
{FX1(kind)}  child #(.P(fx1(2))) u();
  initial #100 $finish;
endmodule
''')
