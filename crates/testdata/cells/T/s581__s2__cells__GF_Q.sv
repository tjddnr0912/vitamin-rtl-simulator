`timescale 1ns/1ns
module t;
  for (genvar i = 0; i <= (4'b1100 ==? 4'b1?00); i++) begin : g initial #1 $display("GF_Q %0d", i); end
endmodule
