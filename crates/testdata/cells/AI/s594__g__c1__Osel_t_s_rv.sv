`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  localparam W = 5 + 3;
  initial #1 $display("RV=%0d", C ? X : W[1:0]);
  initial #5 $finish;
endmodule
