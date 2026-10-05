import os
D=os.path.dirname(os.path.abspath(__file__))
# @SH@ = shadow function in the CALLER scope (the wrong one): value 7, with `@Q@if` miss
SH = """  function automatic int f(input int a);
    f = 7;
    @Q@if (a == 1) f = 10;
  endfunction"""
L={}
# --- package function, run-time inline lane
L['pk_rt_ret']="""package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
@SH@
  int v;
  initial begin v = q::h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
"""
L['pk_rt_loc']="""package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x); logic [f(2):0] t; t = x; return t; endfunction
endpackage
module top;
@SH@
  int v;
  initial begin v = q::h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
"""
L['pk_rt_fml']="""package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input logic [f(2):0] x); return x; endfunction
endpackage
module top;
@SH@
  int v;
  initial begin v = q::h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
"""
L['pk_rt_cast']="""package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x); return f(2)'(x); endfunction
endpackage
module top;
@SH@
  int v;
  initial begin v = q::h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
"""
L['pk_rt_rep']="""package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [31:0] h(input int x); return {f(2){1'b1}}; endfunction
endpackage
module top;
@SH@
  logic [31:0] v;
  initial begin v = q::h(0); $display("v=%h", v); #1 $finish; end
endmodule
"""
L['pk_rt_psel']="""package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [31:0] h(input logic [15:0] x); return x[f(2):0]; endfunction
endpackage
module top;
@SH@
  logic [31:0] v;
  initial begin v = q::h(16'hABCD); $display("v=%h", v); #1 $finish; end
endmodule
"""
# frame lane: loop forces a frame? plus a delay-free task
L['pk_fr_loc']="""package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x);
    logic [f(2):0] t;
    t = 0;
    for (int i = 0; i < x; i++) t = t + 1;
    return t;
  endfunction
endpackage
module top;
@SH@
  int v, n = 1000;
  initial begin v = q::h(n); $display("v=%0d", v); #1 $finish; end
endmodule
"""
L['pk_task_loc']="""package q;
  function automatic int f(input int a); return 3; endfunction
  task automatic tk(input int x, output int o);
    logic [f(2):0] t;
    t = x;
    o = t;
  endtask
endpackage
module top;
@SH@
  int v;
  initial begin q::tk(1000, v); $display("v=%0d", v); #1 $finish; end
endmodule
"""
L['pk_task_susp']="""package q;
  function automatic int f(input int a); return 3; endfunction
  task automatic tk(input int x, output int o);
    logic [f(2):0] t;
    t = x;
    #1 o = t;
  endtask
endpackage
module top;
@SH@
  int v;
  initial begin q::tk(1000, v); $display("v=%0d", v); #1 $finish; end
endmodule
"""
# --- package function, constant interpreter lane
L['pk_ce_ret']="""package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
@SH@
  localparam int P = q::h(1000);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
"""
L['pk_ce_loc']="""package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x); logic [f(2):0] t; t = x; return t; endfunction
endpackage
module top;
@SH@
  localparam int P = q::h(1000);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
"""
L['pk_ce_fml']="""package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input logic [f(2):0] x); return x; endfunction
endpackage
module top;
@SH@
  localparam int P = q::h(1000);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
"""
L['pk_ce_cast']="""package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x); return f(2)'(x); endfunction
endpackage
module top;
@SH@
  localparam int P = q::h(1000);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
"""
L['pk_ce_body']="""package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x); return f(2) + x; endfunction
endpackage
module top;
@SH@
  localparam int P = q::h(1000);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
"""
L['pk_ce_dflt']="""package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x, input int k = f(2)); return x + k; endfunction
endpackage
module top;
@SH@
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); #1 $display("P=%0d v=%0d", P, v); $finish; end
endmodule
"""
L['pk_wq']="""package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
@SH@
  initial begin #1 $display("B=%0d", $bits(q::h(0))); $finish; end
endmodule
"""
# --- imported package function (import q::*), shadow f in module
L['imp_rt_ret']="""package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  import q::h;
@SH@
  int v;
  initial begin v = h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
"""
L['imp_ce_ret']="""package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  import q::h;
@SH@
  localparam int P = h(1000);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
"""
L['imp_two_pkgs']="""package a;
  function automatic int f(input int n);
    f = 7;
    @Q@if (n == 1) f = 10;
  endfunction
endpackage
package q;
  function automatic int f(input int n); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  import a::*;
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); #1 $display("P=%0d v=%0d", P, v); $finish; end
endmodule
"""
# --- interface function called hierarchically from the parent
L['if_rt_ret']="""interface ifc;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endinterface
module top;
@SH@
  ifc i();
  int v;
  initial begin v = i.h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
"""
L['if_wq']="""interface ifc;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endinterface
module top;
@SH@
  ifc i();
  initial begin #1 $display("B=%0d", $bits(i.h(0))); $finish; end
endmodule
"""
L['if_own']="""interface ifc;
  function automatic int f(input int a); return 3; endfunction
  logic [f(2):0] s;
  localparam int L = f(2);
endinterface
module top;
@SH@
  ifc i();
  initial begin #1 $display("bs=%0d L=%0d", $bits(i.s), i.L); $finish; end
endmodule
"""
L['if_port']="""interface ifc;
  function automatic int f(input int a); return 3; endfunction
  logic [f(2):0] s;
endinterface
module c (ifc i);
@SH@
  initial #1 $display("bs=%0d", $bits(i.s));
endmodule
module top;
  ifc i();
  c u (.i(i));
  initial #2 $finish;
endmodule
"""
# --- class method (class in package q)
L['cls_rt']="""package q;
  function automatic int f(input int a); return 3; endfunction
  class C;
    function logic [f(2):0] m(input int x); return x; endfunction
  endclass
endpackage
module top;
@SH@
  q::C c;
  int v;
  initial begin c = new; v = c.m(1000); $display("v=%0d", v); #1 $finish; end
endmodule
"""
# --- hierarchical call into a child instance's function
L['hier_rt']="""module child;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endmodule
module top;
@SH@
  child u ();
  int v;
  initial begin v = u.h(1000); $display("v=%0d", v); #1 $finish; end
endmodule
"""
L['hier_wq']="""module child;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endmodule
module top;
@SH@
  child u ();
  initial begin #1 $display("B=%0d", $bits(u.h(0))); $finish; end
endmodule
"""
# --- regular instance: child header default / port range with parent shadow
L['inst_hdr']="""module child #(parameter int W = f(2)) (input logic [W:0] p);
  function automatic int f(input int a); return 3; endfunction
  initial #1 $display("%m W=%0d b=%0d p=%h", W, $bits(p), p);
endmodule
module top;
@SH@
  logic [7:0] bus = 8'hA5;
  child u (.p(bus));
  initial #2 $finish;
endmodule
"""
# --- instance array element: child header default / port range, child fn plain, parent shadow
L['ia_hdr']="""module child #(parameter int W = f(2)) (input logic [W:0] p);
  function automatic int f(input int a); return 3; endfunction
  initial #1 $display("%m W=%0d b=%0d p=%h", W, $bits(p), p);
endmodule
module top;
@SH@
  logic [7:0] bus = 8'hA5;
  child u[1:0] (.p(bus));
  initial #2 $finish;
endmodule
"""
L['ia_port']="""module child (input logic [f(2):0] p);
  function automatic int f(input int a); return 3; endfunction
  initial #1 $display("%m b=%0d p=%h", $bits(p), p);
endmodule
module top;
@SH@
  logic [7:0] bus = 8'hA5;
  child u[1:0] (.p(bus));
  initial #2 $finish;
endmodule
"""
L['ia_body']="""module child (input logic [3:0] p);
  function automatic int f(input int a); return 3; endfunction
  localparam int L = f(2);
  logic [f(2):0] q;
  initial #1 $display("%m L=%0d bq=%0d", L, $bits(q));
endmodule
module top;
@SH@
  logic [7:0] bus = 8'hA5;
  child u[1:0] (.p(bus));
  initial #2 $finish;
endmodule
"""
# --- generate-scoped function shadowing the module function
L['gen_fn']="""module top;
@SH@
  if (1) begin : g
    function automatic int f(input int a); return 3; endfunction
    localparam int P = f(2);
    logic [f(2):0] w;
    initial #1 $display("P=%0d bw=%0d", P, $bits(w));
  end
  initial #2 $finish;
endmodule
"""
L['gen_fn_ret']="""module top;
@SH@
  if (1) begin : g
    function automatic int f(input int a); return 3; endfunction
    function automatic logic [f(2):0] h(input int x); return x; endfunction
    localparam int P = h(1000);
    int v;
    initial begin v = h(1000); #1 $display("P=%0d v=%0d", P, v); end
  end
  initial #3 $finish;
endmodule
"""
# --- bind: checker with header default calling its own function, target has shadow f
L['bind_hdr']="""module chk #(parameter int W = f(2)) (input logic [7:0] s);
  function automatic int f(input int a); return 3; endfunction
  initial #1 $display("%m W=%0d", W);
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
# --- package parameter range via another package's function
L['pk_param']="""package r;
  function automatic int f(input int a);
    f = 7;
    @Q@if (a == 1) f = 10;
  endfunction
endpackage
package q;
  localparam logic [r::f(2):0] K = '1;
endpackage
module top;
  initial begin #1 $display("b=%0d", $bits(q::K)); $finish; end
endmodule
"""
for n,t in L.items():
    for suf,q in (('u','unique '),('p','')):
        open(os.path.join(D,f'{n}_{suf}.sv'),'w').write(t.replace('@SH@',SH).replace('@Q@',q))
print(len(L))
