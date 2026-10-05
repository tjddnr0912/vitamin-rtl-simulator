import os
S='/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s588/g/s'
def fn(kind, name='fx'):
    sel = "    if (a == 1) t = 4'd5;\n" if kind=='p' else "    case (a) 1: t = 4'd5; endcase\n"
    return f'''  function automatic logic [3:0] {name}(input int a);
    logic [3:0] t;
{sel}    {name} = t;
  endfunction
'''
L={}
L['s01_rt_repl']=("  localparam logic [7:0] V = 8'hA5;\n  initial begin #1 $display(\"r=%b\", {fx(2){2'b10}}); end\n", '"done"')
L['s02_ca_ipsel']=("  logic [7:0] V = 8'hA5;\n  wire [3:0] y = V[0 +: fx(2)];\n", '"y=%b", y')
L['s03_gen_case_label']=("  case (4'd0) fx(2): begin : g0 initial $display(\"L0\"); end default: begin : gd initial $display(\"LD\"); end endcase\n", '"done"')
L['s04_psel_lsb']=("  localparam logic [7:0] V = 8'hA5;\n", '"r=%b", V[3:fx(2)]')
L['s05_formal_range']=("  function automatic int h(input logic [fx(2):0] b);\n    h = $bits(b);\n  endfunction\n  localparam int P = h(0);\n", '"P=%0d", P')
L['s06_lp_ipsel_base']=("  localparam logic [7:0] V = 8'hA5;\n  localparam logic [3:0] P = V[fx(2) +: 4];\n", '"P=%b", P')
L['s07_lp_bitsel']=("  localparam logic [7:0] V = 8'hA5;\n  localparam logic P = V[fx(2)];\n", '"P=%b", P')
L['s08_rt_ipsel_width']=("  logic [7:0] V = 8'hA5;\n  initial begin #1 $display(\"r=%b\", V[0 +: fx(2)]); end\n", '"done"')
L['s09_local_range']=("  function automatic int h(input int a);\n    logic [fx(2):0] b;\n    h = $bits(b);\n  endfunction\n  localparam int P = h(0);\n", '"P=%0d", P')
L['s10_unpacked_lp_count']=("  localparam int N = fx(2);\n  logic [7:0] m [N];\n", '"s=%0d", $size(m)')
L['s11_gen_if_whole_untyped']=("  localparam X = fx(2);\n  if (X) begin : g1 initial $display(\"T\"); end else begin : g2 initial $display(\"E\"); end\n", '"done"')
L['s12_delay_ca']=("  logic a = 1'b1;\n  wire #(fx(2)) w = a;\n", '"w=%b", w')
for k,(body,disp) in L.items():
    for kind in 'pd':
        open(os.path.join(S,f'{k}_{kind}.sv'),'w').write('module top;\n'+fn(kind)+body+'  initial begin #2 $display('+disp+'); $finish; end\n  initial #100 $finish;\nendmodule\n')
