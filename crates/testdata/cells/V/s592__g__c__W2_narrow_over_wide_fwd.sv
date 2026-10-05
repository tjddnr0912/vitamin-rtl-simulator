module top;
  localparam [7:0] K = 8'd5;
  if (1) begin : gb
    initial #1 $display("@K=%h", K);
    localparam [64:0] K = 65'h1_0000_0000_0000_0007;
  end
  initial #5 $finish;
endmodule
