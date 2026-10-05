`timescale 1ns/1ns
module top;
  `include "r3e_decl.vh"
  reg [3:0] x, m;
  initial begin #1 x = 4'd3; case (x) inside[2:1]: m = 1; default: m = 0; endcase $display("include m=%0d", m); $finish; end
  initial #1000 $finish;
endmodule
