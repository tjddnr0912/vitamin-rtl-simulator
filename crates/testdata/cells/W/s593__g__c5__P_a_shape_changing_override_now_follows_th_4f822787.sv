`timescale 1ns/1ns
module m #(parameter type T = logic signed [7:0]);
  T v;
  initial begin v = -8'sd100; #1 $display("D=%0d %0d", v, v >>> 2); end
endmodule
module top;
  m #(.T(logic [7:0])) u2();
  initial #5 $finish;
endmodule
