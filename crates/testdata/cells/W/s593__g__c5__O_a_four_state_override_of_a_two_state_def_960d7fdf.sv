`timescale 1ns/1ns
module m #(parameter type T = bit [7:0]) ();
  T v;
  initial begin
    v = -1;   $display("bits=%0d m1=%0d", $bits(T), v);
    v = -8;   $display("shr=%0d neg=%0d", v >>> 1, (v < 0));
    v = 'x;   $display("x=%b", v);
  end
endmodule
module top; m #(.T(logic [7:0])) u (); initial #10 $finish; endmodule
