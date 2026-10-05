`timescale 1ns/1ns
module t;
  localparam signed [7:0] S = 8'sd84;
  localparam signed [7:0] S2 = 8'sd12;
  localparam L = S ==? 4'sb?100;
  localparam L2 = S2 ==? 4'sb1?00;
  initial begin $display("Z05 %b %b", L, L2); #1 $finish; end
  initial #100 $finish;
endmodule
