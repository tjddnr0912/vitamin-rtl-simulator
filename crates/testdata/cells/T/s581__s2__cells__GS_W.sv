`timescale 1ns/1ns
module t;
  case (((4'd15 + 4'd1) ==? 5'b1?000)) 1'b1: begin : g initial #1 $display("GS_W one"); end default: begin : h initial #1 $display("GS_W default"); end endcase
endmodule
