package pk;
  function automatic int pf(input logic x, input logic z);
    int r; r = 0;
    unique if (x) r = 1; else if (z) r = 2;
    return r;
  endfunction
endpackage
class K;
  int r;
  function void m(input logic x, input logic z);
    unique if (x) r = 1; else if (z) r = 2;
  endfunction
  static function int sm(input logic x, input logic z);
    int q; q = 0;
    priority if (x) q = 1; else if (z) q = 2;
    return q;
  endfunction
endclass
module top;
  logic [1:0] y; int v; K k;
  function automatic void fv(input logic x, input logic z);
    unique if (x) y = 1; else if (z) y = 2;
  endfunction
  function automatic int fi(input logic x, input logic z);
    unique if (x) return 1; else if (z) return 2;
    return 0;
  endfunction
  initial begin
    k = new;
    #1 fv(0, 0); $display("t=%0t fvoid", $time);
    #1 v = fi(0, 0); $display("t=%0t fint v=%0d", $time, v);
    #1 v = pk::pf(0, 0); $display("t=%0t pkgfn v=%0d", $time, v);
    #1 k.m(0, 0); $display("t=%0t classfn", $time);
    #1 v = K::sm(0, 0); $display("t=%0t class-static prio v=%0d", $time, v);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
