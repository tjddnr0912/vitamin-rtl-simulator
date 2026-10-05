`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [15:0] W = 16'h00F0;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  initial #1 $display("RT=%0d", W[(X + (A[1] - 8'd2)) - 8'd248]);
  initial #20 $finish;
endmodule
