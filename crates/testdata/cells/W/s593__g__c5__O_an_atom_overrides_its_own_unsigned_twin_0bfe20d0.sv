`timescale 1ns/1ns
module m #(parameter type T = int) ();
  T v;
  initial begin
    v = -1;   $display("bits=%0d m1=%0d", $bits(T), v);
    v = -8;   $display("shr=%0d neg=%0d", v >>> 1, (v < 0));
    v = 'x;   $display("x=%b", v);
  end
endmodule
module top; m #(.T(int unsigned)) u (); initial #10 $finish; endmodule
