`timescale 1ns/1ns
module sub;
  localparam logic signed [7:0] X = -8'sd4;
  localparam int R1 = $clog2(X + 6);
  localparam int R2 = $clog2(X);
  initial $display("R1=%0d R2=%0d", R1, R2);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
