`timescale 1ns/1ns
module inner #(parameter type T = logic [7:0]);
  T v;
  initial begin v = '1; #1 $display("D=%h %0d", v, $bits(T)); end
endmodule
module m #(parameter type T = logic [7:0]);
  inner #(.T(T)) i();
endmodule
module top;
  m #(.T(logic [5:0])) u();
  initial #5 $finish;
endmodule
