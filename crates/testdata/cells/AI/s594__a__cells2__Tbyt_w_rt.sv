`timescale 1ns/1ns
module t;
  localparam byte TA [0:1] = '{-8'sd3, 8'sd2};
  initial #1 $display("RT=%0d", ((TA[0] + 1'b0) == 8'hFD));
  initial #40 $finish;
endmodule
