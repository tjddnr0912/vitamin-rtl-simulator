package p;
  localparam int W = 3;
  function automatic logic [W:0] h(); h = '1; endfunction
endpackage
interface ifc;
  import p::*;
  localparam int W = 7;
  function automatic logic [W:0] h(); h = '1; endfunction
endinterface
module top;
  ifc i();
  int v, b;
  initial begin v = i.h(); b = $bits(i.h()); $display("v=%0d b=%0d", v, b); end
  initial #100 $finish;
endmodule
