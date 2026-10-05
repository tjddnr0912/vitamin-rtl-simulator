`timescale 1ns/1ns
module sub;
  localparam logic signed [3:0] A [0:1] = '{-1, -2};
  initial $display("A0=%0d A1lt0=%0d", A[0], A[1] < 0);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
