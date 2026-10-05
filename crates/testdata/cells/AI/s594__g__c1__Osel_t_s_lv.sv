`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  localparam W = 5 + 3;
  localparam L = C ? X : W[1:0];
  initial #1 $display("L=%0d B=%0d", L, $bits(L));
  initial #5 $finish;
endmodule
