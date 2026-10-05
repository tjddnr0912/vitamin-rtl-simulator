`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0], parameter T X = -8'sd4);
  localparam int R1 = X;
  localparam R2 = (X < 0);
  localparam int R3 = X >>> 1;
  initial $display("R1=%0d R2=%0d R3=%0d", R1, R2, R3);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
