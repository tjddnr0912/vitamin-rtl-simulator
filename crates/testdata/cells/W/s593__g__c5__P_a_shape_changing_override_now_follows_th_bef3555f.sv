`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  T v;
  initial begin #1 $display("D=%h", v); end
endmodule
module top;
  m #(.T(bit [7:0])) u2();
  initial #5 $finish;
endmodule
