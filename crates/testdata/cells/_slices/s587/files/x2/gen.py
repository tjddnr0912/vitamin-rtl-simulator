import os
D=os.path.dirname(os.path.abspath(__file__))
L={}
L['x2a_ret4']="""module top;
  function automatic logic [3:0] fx(input int a);
    @Q@if (a == 1) fx = 4'd10;
  endfunction
  localparam logic [3:0] P = fx(2);
  initial begin #1 $display("P=%b", P); $finish; end
endmodule
"""
L['x2b_ret2']="""module top;
  function automatic int fi(input int a);
    @Q@if (a == 1) fi = 10;
  endfunction
  localparam int P = fi(2);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
"""
L['x2c_local']="""module top;
  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    @Q@if (a == 1) t = 4'd10;
    f = t;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
endmodule
"""
L['x2d_partial']="""module top;
  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    t[1:0] = 2'b11;
    @Q@if (a == 1) t = 4'd10;
    f = t;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
endmodule
"""
L['x2e_after']="""module top;
  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    f = 4'd7;
    @Q@if (a == 1) f = 4'd10;
    t = 4'd3;
    f = f + t;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
"""
L['x2f_callee']="""module top;
  function automatic int g(input int a);
    g = 7;
    @Q@if (a == 1) g = 10;
  endfunction
  function automatic logic [3:0] f(input int a);
    if (g(a) == 3) f = 4'd1;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
endmodule
"""
L['x2g_sibling']="""module top;
  function automatic int f(input int a);
    f = 7;
    @Q@if (a == 1) f = 10;
  endfunction
  function automatic logic [3:0] g(input int a);
    if (a == 1) g = 4'd1;
  endfunction
  localparam int P = f(2) + g(3);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
"""
L['x2h_noarm']="""module top;
  function automatic int f(input int a);
    f = 7;
    @Q@if (a == 1) f = 10;
  endfunction
  function automatic logic [3:0] g(input int a);
    if (a == 1) g = 4'd1;
  endfunction
  localparam logic [3:0] P = g(3);
  localparam int Q = f(1);
  initial begin #1 $display("P=%b Q=%0d", P, Q); $finish; end
endmodule
"""
L['x2i_integer']="""module top;
  function automatic integer f(input int a);
    integer t;
    @Q@if (a == 1) t = 10;
    f = t;
  endfunction
  localparam integer P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
"""
L['x2j_bitlocal']="""module top;
  function automatic logic [3:0] f(input int a);
    bit [3:0] t;
    f = 4'd7;
    @Q@if (a == 1) f = 4'd10;
    f = f + t;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
"""
L['x2k_range']="""module top;
  function automatic logic [3:0] fx(input int a);
    @Q@if (a == 1) fx = 4'd10;
  endfunction
  logic [fx(2):0] v;
  initial begin #1 $display("b=%0d", $bits(v)); $finish; end
endmodule
"""
for n,t in L.items():
    for suf,q in (('u','unique '),('p','')):
        open(os.path.join(D,f'{n}_{suf}.sv'),'w').write(t.replace('@Q@',q))
print(len(L))
