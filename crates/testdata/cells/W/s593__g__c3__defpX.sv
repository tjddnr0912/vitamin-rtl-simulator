`timescale 1ns/1ns
module sub #(parameter type T = logic [3:0], parameter T X = '0);
  initial $display("X=%0d lt0=%0d b=%0d", X, X < 0, $bits(X));
endmodule
module top;
  sub #(.T(logic signed [7:0])) u();
  defparam u.X = -4;
  initial #100 $finish;
endmodule
