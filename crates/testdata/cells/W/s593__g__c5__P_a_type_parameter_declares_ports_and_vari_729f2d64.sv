`timescale 1ns/1ns
module m #(parameter type A = byte, B = shortint);
  A a; B b;
  initial begin a = -1; b = -2; #1 $display("D=%0d %0d %0d %0d", a, b, $bits(A), $bits(B)); end
endmodule
module top;
  m u1();
  initial #5 $finish;
endmodule
