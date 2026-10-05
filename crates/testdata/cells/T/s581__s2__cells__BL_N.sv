`timescale 1ns/1ns
module t;
  localparam L = ((4'd15 + 4'd1) inside {5'b0?000});
  case (1) L: begin : g initial #1 $display("BL_N item"); end default: begin : h initial #1 $display("BL_N default"); end endcase
endmodule
