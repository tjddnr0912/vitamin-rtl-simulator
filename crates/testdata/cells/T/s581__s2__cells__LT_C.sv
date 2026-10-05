`timescale 1ns/1ns
module t;
  localparam logic [3:0] L = (4'b1100 == 4'b1100);
  initial #1 $display("LT_C %b", L);
endmodule
