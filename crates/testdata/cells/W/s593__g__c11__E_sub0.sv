`timescale 1ns/1ns
module sub;
  localparam logic signed [7:0] X = -8'sd4;
  localparam R = ((X - 8'sd0) !=? 4'b1?00);
  initial $display("R=%0d", R);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
