`timescale 1ns/1ns
module t;
  if ((4'b1100 inside {4'b1?00})) begin : g initial #1 $display("GI_I then"); end else begin : h initial #1 $display("GI_I else"); end
endmodule
