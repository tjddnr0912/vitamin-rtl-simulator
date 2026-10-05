`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -4;
  initial $display("X=%0d lt0=%0d b=%0d", X, X < 0, $bits(X));
endmodule
module top;
  sub #(.T(logic signed [2:0])) u();
  initial #100 $finish;
endmodule
