`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};
  for (genvar g = 0; g < AS[1]; g = g + 1) begin : gl initial #1 $display("g=%0d", g); end
  initial #20 $finish;
endmodule
