`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  logic [$bits(T)*2-1:0] w;
  initial begin w = '1; #1 $display("D=%h", w); end
endmodule
module top;
  m u1(); m #(.T(logic [1:0])) u2();
  initial #5 $finish;
endmodule
