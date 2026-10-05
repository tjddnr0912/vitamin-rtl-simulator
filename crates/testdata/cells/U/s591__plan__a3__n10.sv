module top;
  for (genvar i = 0; i < 2; i++) begin : g
    initial #3 $display("n10 body %m i=%0d", i);
  end
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  initial #5 $display("n10 post i=%0d b=%0d", i, $bits(i));
  initial #100 $finish;
endmodule
