`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -8'sd4;
  localparam int R1 = X ** 2;
  localparam int R2 = 2 ** (X + 6);
  initial $display("R1=%0d R2=%0d", R1, R2);
endmodule
module top;
  sub #(.T(logic signed [7:0])) u();
  initial #100 $finish;
endmodule
