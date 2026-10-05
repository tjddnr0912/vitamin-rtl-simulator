`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -8'sd4;
endmodule
module top;
  sub u();
  initial #100 $finish;
  initial #1 $display("ux=%0d lt0=%0d", u.X, u.X < 0);
endmodule
