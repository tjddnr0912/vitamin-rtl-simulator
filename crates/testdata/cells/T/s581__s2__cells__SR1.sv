`timescale 1ns/1ns
module t;
  localparam logic signed [3:0] S4 = -8;
  localparam logic signed [7:0] S8 = 8'sd0;
  localparam L = (S4 + S8) ==? 4'sb1?00;
  initial #1 $display("SR1 %b", L);
endmodule
