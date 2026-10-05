`timescale 1ns/1ns
module n #(parameter type T = logic [7:0]) ();
  T v; initial begin v = -1; $display("n m1=%0d neg=%0d", v, (v<0)); end
endmodule
module m #(parameter type T = logic [7:0]) ();
  n #(.T(T)) u ();
endmodule
module top; m #(.T(logic signed [7:0])) a (); initial #10 $finish; endmodule
