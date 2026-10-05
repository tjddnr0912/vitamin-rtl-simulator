`timescale 1ns/1ns
module m #(parameter type T = int);
  T v;
  initial begin v = -5; #1 $display("D=%0d %0d", v, $bits(T)); end
endmodule
module top;
  m u1(); m #(.T(longint)) u2(); m #(.T(byte)) u3();
  initial #5 $finish;
endmodule
