package p;
  localparam int W = 3;
  function automatic logic [W:0] h(); return '1; endfunction
endpackage
module top;
  import p::*;
  localparam int W = 7;
  function automatic logic [W:0] h(); return '1; endfunction
  localparam longint P = h();
  logic [31:0] v;
  initial begin #1 v = h(); $display("P=%0d v=%0d", P, v); $finish; end
  initial #100 $finish;
endmodule
