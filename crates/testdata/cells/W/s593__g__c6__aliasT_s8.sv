`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -8'sd4;
  localparam T Y = X + 1;
  initial $display("Y=%0d lt0=%0d", Y, Y < 0);
endmodule
module top;
  sub #(.T(logic signed [7:0])) u();
  initial #100 $finish;
endmodule
