`timescale 1ns/1ns
module sub #(parameter type T = logic [3:0], parameter type T2 = T, parameter T2 X = '1);
  initial $display("X=%0d lt0=%0d b=%0d", X, X < 0, $bits(X));
endmodule
module top;
  sub #(.T(logic signed [3:0])) u();
  initial #100 $finish;
endmodule
