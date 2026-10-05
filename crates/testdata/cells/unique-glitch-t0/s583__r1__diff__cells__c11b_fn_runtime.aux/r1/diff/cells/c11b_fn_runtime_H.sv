package pk;
  function automatic int pf(input logic x, input logic z);
    int r; r = 0;
    if (x) r = 1; else unique if (z) r = 2;
    return r;
  endfunction
endpackage
class K;
  int r;
  function void m(input logic x, input logic z);
    if (x) r = 1; else unique if (z) r = 2;
  endfunction
endclass
module top;
  logic [1:0] y; int v; K k; logic p, q;
  function automatic void fv(input logic x, input logic z);
    if (x) y = 1; else unique if (z) y = 2;
  endfunction
  function automatic int fi(input logic x, input logic z);
    int r; r = 0;
    if (x) r = 1; else unique if (z) r = 2;
    return r;
  endfunction
  initial begin
    k = new; p = 0; q = 0;
    #1 fv(0, 0); $display("t=%0t fvoid", $time);
    #1 v = fi(0, 0); $display("t=%0t fint-literal v=%0d", $time, v);
    #1 v = fi(p, q); $display("t=%0t fint-var v=%0d", $time, v);
    #1 v = pk::pf(p, q); $display("t=%0t pkgfn v=%0d", $time, v);
    #1 k.m(p, q); $display("t=%0t classfn", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
