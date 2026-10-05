import os
D=os.path.dirname(os.path.abspath(__file__))
SH = """  function automatic int f(input int a);
    f = 7;
    @Q@if (a == 1) f = 10;
  endfunction"""
L={}
L['cls_top']="""package q;
  function automatic int f(input int a); return 3; endfunction
endpackage
import q::*;
class C;
  function logic [f(2):0] m(input int x); return x; endfunction
endclass
module top;
@SH@
  C c;
  int v;
  initial begin c = new; v = c.m(1000); $display("v=%0d", v); #1 $finish; end
endmodule
"""
L['cls_mod']="""module top;
@SH@
  class C;
    function logic [f(2):0] m(input int x); return x; endfunction
  endclass
  C c;
  int v;
  initial begin c = new; v = c.m(1000); $display("v=%0d", v); #1 $finish; end
endmodule
"""
L['if_ce']="""interface ifc;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endinterface
module top;
@SH@
  ifc i();
  localparam int P = i.h(1000);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
"""
L['gen_hier']="""module top;
@SH@
  if (1) begin : g
    function automatic int f(input int a); return 3; endfunction
    function automatic logic [f(2):0] h(input int x); return x; endfunction
  end
  int v;
  initial begin v = g.h(1000); #1 $display("v=%0d", v); $finish; end
endmodule
"""
L['bind_port']="""module chk (input logic [f(2):0] s);
  function automatic int f(input int a); return 3; endfunction
  initial #1 $display("%m b=%0d s=%h", $bits(s), s);
endmodule
module tgt;
@SH@
  logic [7:0] s = 8'h5a;
endmodule
module top;
  tgt t ();
  initial #2 $finish;
endmodule
bind tgt chk c (.s(s));
"""
L['pk_rt_dflt']="""package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x, input int k = f(2)); return x + k; endfunction
endpackage
module top;
@SH@
  int v;
  initial begin v = q::h(1000); #1 $display("v=%0d", v); $finish; end
endmodule
"""
for n,t in L.items():
    for suf,q in (('u','unique '),('p','')):
        open(os.path.join(D,f'{n}_{suf}.sv'),'w').write(t.replace('@SH@',SH).replace('@Q@',q))
print(len(L))
