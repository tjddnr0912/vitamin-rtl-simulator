`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -8'sd4;
  localparam R1 = &X;
  localparam R2 = -X;
  localparam int R3 = ~X;
  initial $display("R1=%0d R2=%0d R3=%0d", R1, R2, R3);
endmodule
module top;
  sub #(.T(logic signed [7:0])) u();
  initial #100 $finish;
endmodule
