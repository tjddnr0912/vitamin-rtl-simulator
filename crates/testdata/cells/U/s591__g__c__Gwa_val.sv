module top;
  if (1) begin : h
    localparam [64:0] i = 65'h1_0000_0000_0000_0009;
    initial #5 $display("post %0d", i);
  end
  for (genvar i = 0; i < 2; i++) begin : g
    localparam int K = i;
    initial #1 $display("val %m i=%0d K=%0d", i, K);
  end
  initial #100 $finish;
endmodule
