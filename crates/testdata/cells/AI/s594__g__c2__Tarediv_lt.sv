`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [15:0] L = X / 8'h02;
  initial #1 $display("L=%0d", L);
  initial #20 $finish;
endmodule
