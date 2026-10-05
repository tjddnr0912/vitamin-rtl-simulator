module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    initial #3 $display("n4 g %m i=%0d", i);
  end
  localparam [64:0] M = i;
  for (genvar i = 5; i < 7; i++) begin : h
    initial #3 $display("n4 h %m i=%0d", i);
  end
  initial #5 $display("n4 post i=%0d M=%0d b=%0d", i, M, $bits(i));
  initial #100 $finish;
endmodule
