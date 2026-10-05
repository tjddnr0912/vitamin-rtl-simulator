`timescale 1ns/1ns
module m #(parameter W = 8, parameter type T = logic [W-1:0]);
  T v;
  initial begin v = '1; #1 $display("D=%h %0d", v, $bits(T)); end
endmodule
module top;
  m u1(); m #(.W(3)) u2(); m #(.W(3), .T(logic [5:0])) u3();
  initial #5 $finish;
endmodule
