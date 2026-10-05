module top;
  if (1) begin : b
    localparam [64:0] i = 65'h1_0000_0000_0000_0009;
    for (genvar i = 0; i < 2; i++) begin : g
      localparam int K = i;
      initial #1 $display("val %m i=%0d K=%0d b=%0d", i, K, $bits(i));
    end
    initial #5 $display("post %0d", i);
  end
  initial #100 $finish;
endmodule
