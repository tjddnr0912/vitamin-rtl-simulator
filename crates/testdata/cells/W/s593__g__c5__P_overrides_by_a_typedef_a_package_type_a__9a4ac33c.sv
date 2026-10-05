`timescale 1ns/1ns
package p; typedef logic [11:0] w12_t; endpackage
module m #(parameter type T = logic [7:0]);
  T v;
  initial begin v = '1; #1 $display("D=%h %0d", v, $bits(T)); end
endmodule
module top;
  m #(.T(p::w12_t)) u();
  initial #5 $finish;
endmodule
