`timescale 1ns/1ns
module t;
  if (((4'd15 + 4'd1) ==? 5'b1?000)) begin : g initial #1 $display("GI_W then"); end else begin : h initial #1 $display("GI_W else"); end
endmodule
