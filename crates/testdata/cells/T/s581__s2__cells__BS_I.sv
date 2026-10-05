`timescale 1ns/1ns
module t;
  localparam L = (4'b1100 inside {4'b1?00});
  case (L) 1'b1: begin : g initial #1 $display("BS_I one"); end default: begin : h initial #1 $display("BS_I default"); end endcase
endmodule
