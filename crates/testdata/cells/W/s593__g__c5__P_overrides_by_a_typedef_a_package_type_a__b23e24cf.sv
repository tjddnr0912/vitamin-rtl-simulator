`timescale 1ns/1ns
parameter type T = logic [9:0];
module top;
  T v;
  initial begin v = '1; #1 $display("D=%h %0d", v, $bits(T)); #5 $finish; end
endmodule
