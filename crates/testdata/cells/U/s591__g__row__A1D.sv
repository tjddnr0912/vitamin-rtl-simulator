module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    localparam int K = i;
    initial #1 $display("A1D %m i=%0d K=%0d", i, K);
  end
  initial #100 $finish;
endmodule
