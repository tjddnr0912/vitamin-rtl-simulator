`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  localparam int W = $bits(T);
  logic [W-1:0] x;
  initial begin x = '1; #1 $display("D=%h %0d", x, W); end
endmodule
module top;
  m u1(); m #(.T(logic [2:0])) u2();
  initial #5 $finish;
endmodule
