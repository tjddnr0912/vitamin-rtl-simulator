`timescale 1ns/1ns
module t;
  if ((4'bx100 ==? 4'b1?00)) begin : g initial #1 $display("GI_X then"); end else begin : h initial #1 $display("GI_X else"); end
endmodule
