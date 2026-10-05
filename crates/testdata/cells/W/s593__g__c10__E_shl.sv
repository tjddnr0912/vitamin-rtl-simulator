`timescale 1ns/1ns
module sub;
  localparam logic signed [7:0] X = -8'sd4;
  localparam int R1 = 32'd1 << X;
  localparam int R2 = X << 1;
  localparam int R3 = X >> 1;
  initial $display("R1=%0d R2=%0d R3=%0d", R1, R2, R3);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
