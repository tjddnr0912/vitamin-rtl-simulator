`timescale 1ns/1ns
typedef logic signed [7:0] s8_t;
typedef s8_t s8b_t;
module m #(parameter type T = logic [7:0]) ();
  T v;
  initial begin
    v = -1;   $display("bits=%0d m1=%0d", $bits(T), v);
    v = -8;   $display("shr=%0d neg=%0d", v >>> 1, (v < 0));
    v = 'x;   $display("x=%b", v);
  end
endmodule
module top; m #(.T(s8b_t)) u (); initial #10 $finish; endmodule
