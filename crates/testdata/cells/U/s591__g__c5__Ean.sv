package pk; localparam [64:0] E1 = 65'h1_0000_0000_0000_0000; endpackage
module top;
  import pk::*;
  enum {E0, E1} v;
  localparam int K = E1;
  initial #1 $display("ean E1=%0d K=%0d", E1, K);
  initial #100 $finish;
endmodule
