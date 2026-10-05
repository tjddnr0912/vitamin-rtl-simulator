`timescale 1ns/1ns
module sub #(parameter type T = logic [3:0]) ();
  localparam type T2 = T;
  localparam T2 X = '1;
  initial $display("X=%0d lt0=%0d b=%0d", X, X < 0, $bits(X));
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
