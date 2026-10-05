`timescale 1ns/1ns
module m #(parameter type T = logic [7:0], parameter T X = T'(300));
  T v;
  initial begin v = X; #1 $display("D=%h %h", v, X); end
endmodule
module top;
  m u1(); m #(.T(logic [3:0])) u2();
  initial #5 $finish;
endmodule
