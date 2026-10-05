`timescale 1ns/1ns
module t;
  localparam logic signed [64:0] X = -4;
  localparam L = ((X + 2'b00) > 65'd100);
  initial #1 $display("L=%%0d", L);
  initial #5 $finish;
endmodule
