`timescale 1ns/1ns
module m #(parameter type T = logic signed [7:0]);
  T a, b;
  initial begin a = -1; b = 1; #1 $display("D=%0d", a < b); end
endmodule
module top;
  m u1(); m #(.T(logic signed [15:0])) u2();
  initial #5 $finish;
endmodule
