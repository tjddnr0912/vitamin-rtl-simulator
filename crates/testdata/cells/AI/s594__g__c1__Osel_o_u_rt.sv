`timescale 1ns/1ns
module t;
  localparam logic [7:0] X = 8'hFC;
  localparam bit C = 1;
  localparam W = 5 + 3;
  initial #1 $display("RT=%0d", ((X | W[1:0]) == 8'hFC));
  initial #5 $finish;
endmodule
