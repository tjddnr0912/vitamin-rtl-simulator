`timescale 1ns/1ns
module t;
  localparam signed [7:0] S = 8'sd84;
  localparam L = S ==? 4'sb?100;
  initial #1 $display("SR2 %b", L);
endmodule
