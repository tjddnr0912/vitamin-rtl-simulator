`timescale 1ns/1ns
typedef logic [7:0] a_t [0:2];
module n #(parameter type T = a_t) ();
  T v; initial begin v[0] = -1; $display("n e0=%0d neg=%0d bits=%0d", v[0], (v[0]<0), $bits(v)); end
endmodule
module m #(parameter type T = a_t) ();
  n #(.T(T)) u ();
endmodule
module top;
  typedef logic signed [7:0] s_t [0:2];
  m #(.T(s_t)) a ();
  initial #10 $finish;
endmodule
