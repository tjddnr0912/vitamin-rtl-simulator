module X5;
  localparam Q = 3;
  if (1) begin : gb
    localparam [64:0] P = Q;
    wire [P[3:0]:0] w;
    initial #1 $display("X5 P=%h bits_w=%0d", P, $bits(w));
    localparam [64:0] Q = 65'h1_0000_0000_0000_0007;
  end
  initial #5 $finish;
endmodule
