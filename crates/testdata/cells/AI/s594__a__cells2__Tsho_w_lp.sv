`timescale 1ns/1ns
module t;
  localparam shortint TA [0:1] = '{-16'sd3, 16'sd2};
  localparam L = ((TA[0] + 1'b0) == 16'hFFFD);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
