`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -8'sd4;
  localparam Y = X;
  initial $display("Y=%0d lt0=%0d b=%0d", Y, Y < 0, $bits(Y));
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
