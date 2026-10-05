`timescale 1ns/1ns
module t;
  if ((4'b1100 == 4'b1100)) begin : g initial #1 $display("GI_C then"); end else begin : h initial #1 $display("GI_C else"); end
endmodule
