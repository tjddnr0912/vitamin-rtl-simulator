package pk; localparam [64:0] i = 65'h1_0000_0000_0000_0009; endpackage
module top;
  import pk::*;
  for (genvar i = 0; i < 2; i++) begin : g
    initial #3 $display("n7 body %m i=%0d pki=%0d", i, pk::i);
  end
  initial #5 $display("n7 post i=%0d pki=%0d", i, pk::i);
  initial #100 $finish;
endmodule
