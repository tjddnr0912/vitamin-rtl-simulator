`timescale 1ns/1ns
module t;
  if (((4'd15 + 4'd1) inside {5'b0?000})) begin : g initial #1 $display("GI_N then"); end else begin : h initial #1 $display("GI_N else"); end
endmodule
