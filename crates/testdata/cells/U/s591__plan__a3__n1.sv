module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    begin : h
      localparam [64:0] i = 65'h2_0000_0000_0000_0003;
      initial #2 $display("n1 in-h %m i=%0d", i);
    end
    initial #3 $display("n1 body %m i=%0d", i);
  end
  initial #5 $display("n1 post i=%0d b=%0d", i, $bits(i));
  initial #100 $finish;
endmodule
