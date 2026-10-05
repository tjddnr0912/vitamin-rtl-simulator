`timescale 1ns/1ns
module m #(parameter type T = logic [3:0][7:0]);
  T v;
  initial begin v = 32'hDEADBEEF; #1 $display("D=%h %h %0d %0d %0d", v[3], v[0], $bits(T), $size(v,1), $size(v,2)); end
endmodule
module top; m u(); initial #5 $finish; endmodule
