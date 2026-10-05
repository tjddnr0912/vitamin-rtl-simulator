`timescale 1ns/1ns
module t;
  localparam logic [7:0] X = 8'hFC;
  localparam bit C = 1;
  localparam W = 5 + 3;
  localparam L = ((X | W[1:0]) == 8'hFC);
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
