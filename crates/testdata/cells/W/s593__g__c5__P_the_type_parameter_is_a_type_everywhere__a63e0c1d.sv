`timescale 1ns/1ns
module m #(parameter type T = bit [3:0]);
  T v;
  initial begin #1 $display("D=%h %0d", v, $bits(T)); end
endmodule
module top;
  m u1(); m #(.T(bit [5:0])) u2();
  initial #5 $finish;
endmodule
