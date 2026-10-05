`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  localparam bit C = 1;
  initial #1 $display("R=%0d lt0=%0d", AS[0] + 8'sd0, (AS[0] + 8'sd0) < 0);
  initial #20 $finish;
endmodule
