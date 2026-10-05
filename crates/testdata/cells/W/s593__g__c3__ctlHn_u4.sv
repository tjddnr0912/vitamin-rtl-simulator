`timescale 1ns/1ns
module sub #(parameter type T = logic [3:0], parameter T X = -4);
  initial $display("X=%0d lt0=%0d b=%0d shr=%0d", X, X < 0, $bits(X), X >>> 1);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
