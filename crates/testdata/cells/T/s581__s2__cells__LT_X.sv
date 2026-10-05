`timescale 1ns/1ns
module t;
  localparam logic [3:0] L = (4'bx100 ==? 4'b1?00);
  initial #1 $display("LT_X %b", L);
endmodule
