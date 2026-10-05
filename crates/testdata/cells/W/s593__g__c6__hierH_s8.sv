`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0], parameter T X = -8'sd4) ();
endmodule
module top;
  sub #(.T(logic signed [7:0])) u();
  initial #100 $finish;
  initial #1 $display("ux=%0d lt0=%0d", u.X, u.X < 0);
endmodule
