`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) ();
  T v;
  initial $display("bits=%0d", $bits(T));
endmodule
module top; m #(.T$d0a(16)) u (); initial #10 $finish; endmodule
