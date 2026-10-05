`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int AI [0:1] = '{-4, 2};
  localparam L = ((AI[0] + 0) == -4);
  initial #1 $display("L=%0d", L);
  initial #20 $finish;
endmodule
