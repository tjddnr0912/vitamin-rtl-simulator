package pk; localparam [64:0] i = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pk::*;
  genvar i;
  for (i = 0; i < 2; i++) begin : g
    localparam int K = i;
    initial #1 $display("val %m i=%0d K=%0d b=%0d", i, K, $bits(i));
  end
  initial #100 $finish;
endmodule
