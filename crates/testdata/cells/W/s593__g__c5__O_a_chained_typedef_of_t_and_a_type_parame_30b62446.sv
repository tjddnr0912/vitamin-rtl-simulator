`timescale 1ns/1ns
module m #(parameter type T = logic [7:0], parameter type U = T) ();
  U v;
  initial begin v = -1; $display("chainp m1=%0d neg=%0d", v, (v < 0)); end
endmodule
module top; m #(.T(logic signed [7:0])) u (); initial #10 $finish; endmodule
