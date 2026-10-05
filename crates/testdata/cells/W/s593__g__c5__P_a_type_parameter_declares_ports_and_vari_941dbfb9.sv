`timescale 1ns/1ns
module m #(parameter type A = int, parameter type B = logic [3:0]);
  A a; B b;
  initial begin a = -1; b = 4'hf; #1 $display("D=%0d %h %0d %0d", a, b, $bits(A), $bits(B)); end
endmodule
module top;
  m u1(); m #(.B(logic [7:0])) u2();
  initial #5 $finish;
endmodule
