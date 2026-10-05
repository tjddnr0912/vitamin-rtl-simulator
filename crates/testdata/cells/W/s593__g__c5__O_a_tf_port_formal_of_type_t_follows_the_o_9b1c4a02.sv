`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) ();
  function automatic int f(T x); f = (x >>> 1); endfunction
  T v;
  initial begin v = -8; $display("tf=%0d", f(v)); end
endmodule
module top; m #(.T(logic signed [7:0])) u (); initial #10 $finish; endmodule
