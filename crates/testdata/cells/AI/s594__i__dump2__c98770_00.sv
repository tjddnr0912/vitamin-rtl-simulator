`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};
  localparam L = ((AS[0] / 8'd2) == 8'd126);
  initial #1 $display("L=%0d", L);
  initial #50 $finish;
endmodule
