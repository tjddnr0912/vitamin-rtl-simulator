`timescale 1ns/1ns
module t;
  for (genvar i = 0; i <= (4'bx100 ==? 4'b1?00); i++) begin : g initial #1 $display("GF_X %0d", i); end
endmodule
