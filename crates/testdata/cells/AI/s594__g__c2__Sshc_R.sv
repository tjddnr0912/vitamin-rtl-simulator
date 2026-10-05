`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [15:0] W = 16'h00F0;
  localparam int N = 2;
  localparam L = 16'h0001 << ((X + {N{1'b0}}) - 8'd250);
  initial #1 $display("L=%0d", L);
  initial #20 $finish;
endmodule
