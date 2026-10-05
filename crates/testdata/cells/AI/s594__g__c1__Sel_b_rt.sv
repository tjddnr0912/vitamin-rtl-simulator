`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};
  initial #1 $display("RT=%0d", ((AS[0]) == -8'sd4));
  initial #5 $finish;
endmodule
