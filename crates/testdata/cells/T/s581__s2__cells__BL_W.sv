`timescale 1ns/1ns
module t;
  localparam L = ((4'd15 + 4'd1) ==? 5'b1?000);
  case (1) L: begin : g initial #1 $display("BL_W item"); end default: begin : h initial #1 $display("BL_W default"); end endcase
endmodule
