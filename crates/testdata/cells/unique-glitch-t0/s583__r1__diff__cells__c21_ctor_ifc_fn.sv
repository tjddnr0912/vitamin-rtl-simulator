interface ifc;
  logic [1:0] y;
  function automatic int f(input logic a, input logic b);
    int r; r = 0;
    unique if (a) r = 1; else if (b) r = 2;
    return r;
  endfunction
endinterface
class K;
  int r;
  function new(input logic a, input logic b);
    unique if (a) r = 1; else if (b) r = 2;
  endfunction
endclass
module top;
  ifc i(); K k; int v;
  initial begin
    #1 v = i.f(0, 0); $display("t=%0t ifc fn v=%0d", $time, v);
    #1 k = new(0, 0); $display("t=%0t ctor", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
