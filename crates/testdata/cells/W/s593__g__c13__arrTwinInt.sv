`timescale 1ns/1ns
module sub;
  localparam int A [0:1] = '{-1, -2};
  int k; initial begin for (k = 0; k < 2; k++) $display("A%0d=%0d lt0=%0d", k, A[k], A[k] < 0); end
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
