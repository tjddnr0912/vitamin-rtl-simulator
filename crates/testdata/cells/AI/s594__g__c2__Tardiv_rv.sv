`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  initial #1 $display("RV=%0d", X / 2'b11);
  initial #20 $finish;
endmodule
