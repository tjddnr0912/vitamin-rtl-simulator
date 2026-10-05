`timescale 1ns/1ns
`define DECL(n) reg [3:0] n;
`define SET(n) n = 4'b0110;
module top;
  `DECL(inside)
  reg [3:0] x, m;
  initial begin `SET(inside) x = 4'd3; case (x) inside[2:1]: m = 1; default: m = 0; endcase $display("arg m=%0d", m); $finish; end
  initial #1000 $finish;
endmodule
