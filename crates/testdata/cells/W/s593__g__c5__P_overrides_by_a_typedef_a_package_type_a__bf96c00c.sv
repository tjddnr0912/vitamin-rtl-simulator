`timescale 1ns/1ns
module top;
  typedef logic [5:0] s6;
  m #(.T(s6)) u();
  initial #5 $finish;
endmodule
module m #(parameter type T = logic [7:0]);
  T v;
  initial begin v = '1; #1 $display("D=%h %0d", v, $bits(T)); end
endmodule
