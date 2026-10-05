module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    for (genvar i = 5; i < 7; i++) begin : h
      initial #2 $display("n5 inner %m i=%0d", i);
    end
    initial #3 $display("n5 outer %m i=%0d", i);
  end
  initial #5 $display("n5 post i=%0d b=%0d", i, $bits(i));
  initial #100 $finish;
endmodule
