`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) ();
  initial $display("bits=%0d neg=%0d hx=%0h", $bits(T'(8'hF0)), (T'('1) - 1) < 0, T'(8'hF0));
  initial #10 $finish;
endmodule
module top; m #(.T(logic signed [79:0])) u(); endmodule
