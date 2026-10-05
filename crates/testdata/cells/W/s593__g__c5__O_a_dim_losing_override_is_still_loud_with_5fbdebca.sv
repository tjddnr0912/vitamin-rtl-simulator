`timescale 1ns/1ns
typedef logic [7:0] a_t [0:2];
module m #(parameter type T = a_t) ();
  T v;
  initial $display("bits=%0d dims=%0d", $bits(v), $dimensions(v));
endmodule
module top; m #(.T(logic [15:0])) u (); initial #10 $finish; endmodule
