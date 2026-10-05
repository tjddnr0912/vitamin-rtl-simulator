`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -8'sd4;
  localparam R1 = (X < 8'd5);
  localparam R2 = (X < 5);
  localparam int R3 = X + 8'd0;
  localparam int R4 = X + 0;
  initial $display("R1=%0d R2=%0d R3=%0d R4=%0d", R1, R2, R3, R4);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
