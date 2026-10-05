`timescale 1ns/1ns
module t;
  case (((4'd15 + 4'd1) inside {5'b0?000})) 1'b1: begin : g initial #1 $display("GS_N one"); end default: begin : h initial #1 $display("GS_N default"); end endcase
endmodule
