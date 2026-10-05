module top;
  localparam [64:0] i = 65'h1_0000_0000_0000_0009;
  for (genvar i = 0; i < 2; i++) begin : g
    initial #1 $display("@g i=%0d", i);
  end
  wire [64:0] w = i;
  localparam [64:0] Q = i + 65'd1;
  initial #2 $display("@top i=%0d w=%0d Q=%0d sh=%0d", i, w, Q, i >> 60);
  if (i > 65'd100) begin : bg initial #3 $display("@big"); end
endmodule
