`begin_keywords "1364-2005"
`timescale 1ns/1ns
`define CAT(a,b) a``b
module top;
  reg [3:0] `CAT(ins,ide);
  reg [3:0] x, m;
  initial begin `CAT(ins,ide) = 4'b0110; x = 4'd3; case (x) inside[2:1]: m = 1; default: m = 0; endcase $display("paste m=%0d", m); $finish; end
  initial #1000 $finish;
endmodule
`end_keywords
