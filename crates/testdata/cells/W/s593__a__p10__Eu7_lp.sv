`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  localparam int N = 2;
  localparam bit C = 1;
  localparam L = ((X + {N{1'b0}}) == 8'hFC);
  initial #1 $display("L=%b", L);
  initial #5 $finish;
endmodule
