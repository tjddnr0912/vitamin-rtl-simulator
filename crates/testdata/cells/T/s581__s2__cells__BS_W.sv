`timescale 1ns/1ns
module t;
  localparam L = ((4'd15 + 4'd1) ==? 5'b1?000);
  case (L) 1'b1: begin : g initial #1 $display("BS_W one"); end default: begin : h initial #1 $display("BS_W default"); end endcase
endmodule
