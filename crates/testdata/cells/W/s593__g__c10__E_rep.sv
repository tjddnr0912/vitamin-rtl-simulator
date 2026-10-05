`timescale 1ns/1ns
module sub;
  localparam logic signed [7:0] X = -8'sd4;
  localparam R1 = {(X + 6){1'b1}};
  initial $display("R1=%b", R1);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
