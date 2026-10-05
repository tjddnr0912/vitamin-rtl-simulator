`timescale 1ns/1ns
typedef logic [7:0] a_t [0:2];
module m #(parameter type T = logic [7:0]) ();
  T v;
  initial $display("bits=%0d", $bits(v));
endmodule
module top; m #(.T(a_t)) u (); initial #10 $finish; endmodule
