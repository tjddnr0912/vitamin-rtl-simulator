`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -8'sd4;
  localparam int R1 = (X < 0) ? X : -X;
  localparam int R2 = 1 ? X : 8'd0;
  initial $display("R1=%0d R2=%0d", R1, R2);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
