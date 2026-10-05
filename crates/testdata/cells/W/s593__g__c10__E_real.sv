`timescale 1ns/1ns
module sub;
  localparam logic signed [7:0] X = -8'sd4;
  localparam real R1 = X;
  localparam real R2 = X * 0.5;
  initial $display("R1=%0.2f R2=%0.2f", R1, R2);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
