`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};
  initial #1 $display("RT=%0d", ((2 ** AS[0]) == 0));
  initial #20 $finish;
endmodule
