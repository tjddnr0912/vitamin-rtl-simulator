`timescale 1ns/1ns
module n #(parameter type T = logic [7:0]) ();
  T v; initial begin v = -1; $display("n m1=%0d neg=%0d bits=%0d", v, (v<0), $bits(T)); end
endmodule
module m #(parameter type T = logic [7:0]) ();
  n #(.T(T)) u ();
  T w; initial begin w = -1; $display("m m1=%0d", w); end
endmodule
module top; m #(.T(logic signed [7:0])) a (); m b (); initial #10 $finish; endmodule
