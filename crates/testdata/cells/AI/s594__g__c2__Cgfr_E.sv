`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  for (genvar g = 0; g < (A[1][1:0] | 2'b11 + 2'd2); g = g + 1) begin : gl initial #1 $display("g=%0d", g); end
  initial #20 $finish;
endmodule
