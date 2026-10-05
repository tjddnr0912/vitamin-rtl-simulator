`timescale 1ns/1ns
module t;
  localparam L = ((4'd15 + 4'd1) inside {5'b0?000});
  case (L) 1'b1: begin : g initial #1 $display("BS_N one"); end default: begin : h initial #1 $display("BS_N default"); end endcase
endmodule
