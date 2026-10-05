`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  logic [3:0] mem [0:$bits(T)-1];
  initial begin mem[$bits(T)-1] = 4'ha; #1 $display("D=%h %0d", mem[$bits(T)-1], $size(mem)); end
endmodule
module top;
  m u1(); m #(.T(logic [1:0])) u2();
  initial #5 $finish;
endmodule
