`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic signed [3:0] NS = 4'sd2;
  localparam L = ((X + {NS{1'b0}}) == 8'hFC);
  initial #1 $display("L=%0d", L);
  initial #20 $finish;
endmodule
