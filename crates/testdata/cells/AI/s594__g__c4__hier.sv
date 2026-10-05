`timescale 1ns/1ns
module m; localparam logic [7:0] P = 8'h02; endmodule
module t;
  m u();
  localparam logic signed [7:0] X = -4;
  localparam L = ((X | u.P) == 8'hFE);
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
