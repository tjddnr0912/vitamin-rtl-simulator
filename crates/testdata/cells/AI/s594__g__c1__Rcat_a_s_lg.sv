`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  localparam int N = 2;
  localparam L = ((X + {{N{1'b0}}, 1'b0}) > 8'd100);
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
